use std::{env, fs};

use anyhow::Context;
use chrono::{Duration, NaiveDate, Utc};
use fc_domain::{AssessmentHistoryPoint, DataMode, ModelReleaseRecord, Observation, ProbabilityBundle};
use fc_storage::{PostgresStore, SqliteStore};

use crate::{
    assessment::ServingModelContext,
    demo::{
        build_app_data_from_inputs, build_demo_data, load_user_preferences, BuiltAppData,
        ScenarioBacktestContext,
    },
    history_builder::{
        build_assessment_history, load_sqlite_assessment_history, HistoryQueryWindow,
    },
    history_replay::{
        expected_prediction_snapshot_method_version, historical_output_from_replay_points,
        historical_replay_source_watermark,
    },
    AppData,
};

const EVENT_LOOKBACK_DAYS: i64 = 30;
const SCENARIO_BACKTEST_REPLAY_SCAN_LIMIT: usize = 64;

#[derive(Debug, Clone)]
pub enum AppDataSource {
    Demo,
    Sqlite { path: String },
    Postgres { database_url: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssessmentHistoryBuildMode {
    Default,
    StrictRebuild,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServingRuntimePurpose {
    Production,
    Review,
}

impl ServingRuntimePurpose {
    pub fn as_label(self) -> &'static str {
        match self {
            Self::Production => "production",
            Self::Review => "review",
        }
    }
}

pub fn source_from_env() -> anyhow::Result<AppDataSource> {
    match env::var("FC_DATA_MODE").ok().as_deref() {
        Some("postgres") => {
            let database_url = env::var("DATABASE_URL").context("DATABASE_URL is required")?;
            Ok(AppDataSource::Postgres { database_url })
        }
        Some("sqlite") => Ok(AppDataSource::Sqlite {
            path: env::var("FC_SQLITE_PATH").unwrap_or_else(|_| "data/fc-local.sqlite".to_string()),
        }),
        _ => Ok(AppDataSource::Demo),
    }
}

pub async fn load_app_data(
    source: &AppDataSource,
    max_history_points: usize,
) -> anyhow::Result<AppData> {
    load_app_data_with_runtime_options(
        source,
        max_history_points,
        AssessmentHistoryBuildMode::Default,
        ServingRuntimePurpose::Production,
    )
    .await
}

pub async fn load_app_data_with_runtime_options(
    source: &AppDataSource,
    max_history_points: usize,
    history_build_mode: AssessmentHistoryBuildMode,
    runtime_purpose: ServingRuntimePurpose,
) -> anyhow::Result<AppData> {
    match source {
        AppDataSource::Demo => Ok(build_demo_data(max_history_points)),
        AppDataSource::Sqlite { path } => {
            load_sqlite_app_data(
                path,
                max_history_points,
                history_build_mode,
                runtime_purpose,
            )
            .await
        }
        AppDataSource::Postgres { database_url } => {
            load_postgres_app_data(database_url, max_history_points).await
        }
    }
}

async fn load_postgres_app_data(
    database_url: &str,
    _max_history_points: usize,
) -> anyhow::Result<AppData> {
    let as_of_date = Utc::now().date_naive();
    let store = PostgresStore::connect(database_url).await?;
    let indicators = store.load_indicators().await?;
    if indicators.is_empty() {
        anyhow::bail!("metadata.indicators is empty");
    }
    let observations = store
        .load_observations_for_entities(&["us", "jp"], as_of_date)
        .await?;
    if observations.is_empty() {
        anyhow::bail!("ts.indicator_observations has no rows for entity us");
    }
    let user_preferences = load_user_preferences();
    let historical = build_assessment_history(
        DataMode::Postgres,
        &fc_scoring::ScoringEngine::default(),
        &indicators,
        &observations,
        Some(&[]),
        None,
        &user_preferences,
        HistoryQueryWindow {
            from: None,
            to: None,
            limit: None,
        },
    );
    Ok(build_app_data_from_inputs(
        DataMode::Postgres,
        indicators,
        observations,
        Some(Vec::new()),
        None,
        as_of_date,
        historical.history_points,
        None,
        user_preferences,
    )
    .app_data)
}

async fn load_sqlite_scenario_backtest_context(
    store: &SqliteStore,
    observations: &[Observation],
    serving_model: Option<&ServingModelContext>,
    as_of_date: NaiveDate,
    default_history: &[AssessmentHistoryPoint],
) -> anyhow::Result<Option<ScenarioBacktestContext>> {
    let default_history_start = default_history.first().map(|point| point.as_of_date);
    let default_history_end = default_history.last().map(|point| point.as_of_date);
    let default_history_point_count = default_history.len();
    let release_filter = serving_model.map(|context| context.release.manifest.release_id.as_str());
    let expected_history_cache_key = expected_prediction_snapshot_method_version(serving_model);
    let expected_source_watermark = historical_replay_source_watermark(observations);

    let Some(best_run) = store
        .list_historical_replay_runs(
            Some("financial_system"),
            release_filter,
            Some(as_of_date),
            Some(as_of_date),
            Some(SCENARIO_BACKTEST_REPLAY_SCAN_LIMIT),
        )
        .await?
        .into_iter()
        .filter(|run| run.history_cache_key == expected_history_cache_key)
        .filter(|run| run.source_watermark == expected_source_watermark)
        .filter(|run| run.point_count > default_history_point_count)
        .max_by(|left, right| {
            left.point_count
                .cmp(&right.point_count)
                .then_with(|| left.created_at.cmp(&right.created_at))
        })
    else {
        return Ok(None);
    };

    let replay_points = store
        .list_historical_assessment_points(
            Some(&best_run.replay_run_id),
            Some("financial_system"),
            release_filter,
            Some(best_run.from_date),
            Some(best_run.to_date),
            None,
        )
        .await?;
    let history = historical_output_from_replay_points(replay_points).history_points;
    if history.len() <= default_history_point_count {
        return Ok(None);
    }

    let coverage_scope_note =
        match (history.first(), history.last(), default_history_start, default_history_end) {
            (Some(history_start), Some(history_end), Some(default_start), Some(default_end)) => {
                format!(
                    "这里的“本地覆盖场景 / 模板参照场景”按场景回测历史窗口 {} 到 {} 统计；当前已优先复用本地 SQLite 中更长的 persisted replay 历史，而不是只看默认运行窗口 {} 到 {}。它回答的是危机场景目录里有多少样本能直接落在这段本地历史上，不等于上面默认历史轨迹是否已经进入 PIT 正式证据层。",
                    history_start.as_of_date,
                    history_end.as_of_date,
                    default_start,
                    default_end
                )
            }
            (Some(history_start), Some(history_end), _, _) => format!(
                "这里的“本地覆盖场景 / 模板参照场景”按场景回测历史窗口 {} 到 {} 统计；当前已优先复用本地 SQLite 中更长的 persisted replay 历史。它回答的是危机场景目录里有多少样本能直接落在这段本地历史上，不等于上面默认历史轨迹是否已经进入 PIT 正式证据层。",
                history_start.as_of_date,
                history_end.as_of_date
            ),
            _ => "这里的“本地覆盖场景 / 模板参照场景”按场景回测历史窗口统计；当前已优先复用本地 SQLite 中更长的 persisted replay 历史。它回答的是危机场景目录里有多少样本能直接落在这段本地历史上，不等于上面默认历史轨迹是否已经进入 PIT 正式证据层。".to_string(),
        };

    Ok(Some(ScenarioBacktestContext {
        history,
        coverage_scope_note,
    }))
}

async fn load_sqlite_app_data(
    sqlite_path: &str,
    max_history_points: usize,
    history_build_mode: AssessmentHistoryBuildMode,
    runtime_purpose: ServingRuntimePurpose,
) -> anyhow::Result<AppData> {
    let as_of_date = Utc::now().date_naive();
    let store = SqliteStore::connect(sqlite_path).await?;
    store.migrate().await?;
    let indicators = store.load_indicators().await?;
    if indicators.is_empty() {
        anyhow::bail!("metadata_indicators is empty; run `just db-seed` first");
    }
    let observations = store
        .load_observations_for_entities(&["us", "jp"], as_of_date)
        .await?;
    if observations.is_empty() {
        anyhow::bail!(
            "ts_indicator_observations has no rows for entity us; run at least one backfill such as `just backfill-fred`, `just backfill-treasury-yield`, or `just backfill-world-bank` first"
        );
    }
    let alerts = store
        .load_alerts_recent(as_of_date - Duration::days(EVENT_LOOKBACK_DAYS), as_of_date)
        .await?;
    let serving_model = store
        .load_active_model_release("financial_system")
        .await?
        .map(|release| build_serving_model_context(release, runtime_purpose));
    let user_preferences = load_user_preferences();
    let assessment_history = load_sqlite_assessment_history(
        &store,
        &indicators,
        &observations,
        &alerts,
        serving_model.as_ref(),
        &user_preferences,
        as_of_date,
        max_history_points,
        history_build_mode,
    )
    .await?;
    let scenario_backtest_context = load_sqlite_scenario_backtest_context(
        &store,
        &observations,
        serving_model.as_ref(),
        as_of_date,
        &assessment_history,
    )
    .await?;
    let bundle_backed_runtime_release = serving_model.as_ref().and_then(|serving_model| {
        serving_model.probability_bundle.as_ref()?;
        Some((
            serving_model.release.manifest.market_scope.clone(),
            serving_model.release.manifest.release_id.clone(),
        ))
    });
    let built: BuiltAppData = build_app_data_from_inputs(
        DataMode::Sqlite,
        indicators,
        observations,
        Some(alerts),
        serving_model,
        as_of_date,
        assessment_history,
        scenario_backtest_context,
        user_preferences,
    );
    if let Some((market_scope, release_id)) = bundle_backed_runtime_release {
        if let Err(error) = store
            .delete_prediction_snapshot_history_for_release(&market_scope, &release_id, as_of_date)
            .await
        {
            tracing::warn!(
                sqlite_path = sqlite_path,
                release_id,
                error = %format!("{error:#}"),
                "failed to prune historical prediction snapshots for bundle-backed release"
            );
        }
    }
    if let Err(error) = store
        .upsert_prediction_snapshots(&built.prediction_snapshots)
        .await
    {
        tracing::warn!(
            sqlite_path = sqlite_path,
            error = %format!("{error:#}"),
            "failed to persist assessment prediction snapshots"
        );
    }
    Ok(built.app_data)
}

fn build_serving_model_context(
    release: ModelReleaseRecord,
    runtime_purpose: ServingRuntimePurpose,
) -> ServingModelContext {
    if release.manifest.probability_mode == "heuristic_mvp" {
        return ServingModelContext {
            runtime_probability_mode: release.manifest.probability_mode.clone(),
            runtime_release_status: release.manifest.serving_status.clone(),
            probability_bundle: None,
            release,
        };
    }

    if release_requires_runtime_bundle_fallback(&release, runtime_purpose) {
        tracing::warn!(
            release_id = %release.manifest.release_id,
            release_state = %format!("{}/{}", release.manifest.status, release.manifest.serving_status),
            runtime_purpose = runtime_purpose.as_label(),
            "runtime requires a formally healthy active release; falling back to heuristic probabilities"
        );
        return degraded_serving_model_context(release);
    }

    match load_probability_bundle(&release.manifest.bundle_uri) {
        Ok(bundle)
            if bundle.market_scope == release.manifest.market_scope
                && bundle
                    .horizons
                    .iter()
                    .any(|horizon| horizon.horizon_days == 5)
                && bundle
                    .horizons
                    .iter()
                    .any(|horizon| horizon.horizon_days == 20)
                && bundle
                    .horizons
                    .iter()
                    .any(|horizon| horizon.horizon_days == 60) =>
        {
            ServingModelContext {
                runtime_probability_mode: bundle.probability_mode.clone(),
                runtime_release_status: release.manifest.serving_status.clone(),
                probability_bundle: Some(bundle),
                release,
            }
        }
        Ok(_) => {
            tracing::warn!(
                release_id = %release.manifest.release_id,
                bundle_uri = %release.manifest.bundle_uri,
                "active release bundle is missing required 5d/20d/60d horizons; falling back to heuristic probabilities"
            );
            degraded_serving_model_context(release)
        }
        Err(error) => {
            tracing::warn!(
                release_id = %release.manifest.release_id,
                bundle_uri = %release.manifest.bundle_uri,
                error = %error,
                "failed to load active release bundle; falling back to heuristic probabilities"
            );
            degraded_serving_model_context(release)
        }
    }
}

fn release_requires_runtime_bundle_fallback(
    release: &ModelReleaseRecord,
    runtime_purpose: ServingRuntimePurpose,
) -> bool {
    matches!(runtime_purpose, ServingRuntimePurpose::Production)
        && (release.manifest.status != "active" || release.manifest.serving_status != "healthy")
}

fn degraded_serving_model_context(release: ModelReleaseRecord) -> ServingModelContext {
    ServingModelContext {
        runtime_probability_mode: "heuristic_mvp".to_string(),
        runtime_release_status: "degraded".to_string(),
        probability_bundle: None,
        release,
    }
}

fn load_probability_bundle(bundle_uri: &str) -> anyhow::Result<ProbabilityBundle> {
    let raw = fs::read_to_string(bundle_uri)
        .with_context(|| format!("failed to read probability bundle from {bundle_uri}"))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse probability bundle at {bundle_uri}"))
}

#[cfg(test)]
mod tests {
    use std::{env, path::PathBuf};

    use chrono::{Duration, Utc};
    use fc_domain::{
        HistoricalAssessmentPointRecord, HistoricalReplayRunRecord, HorizonEvaluationSummary,
        LogisticProbabilityModel, ModelReleaseManifest, ModelReleaseRecord,
        PredictionSnapshotRecord, ProbabilityBundle, ProbabilityCoefficient,
        ProbabilityDiagnostics, ProbabilityFeatureStat, ProbabilityHorizonBundle,
    };
    use fc_storage::SqliteStore;
    use uuid::Uuid;

    use crate::{
        demo_seed::{indicators as demo_indicators, observations as demo_observations},
        history_replay::expected_prediction_snapshot_method_version,
    };

    use super::{
        build_serving_model_context, load_app_data_with_runtime_options, AppDataSource,
        AssessmentHistoryBuildMode, ServingRuntimePurpose,
    };

    fn temp_bundle_path() -> PathBuf {
        env::temp_dir().join(format!("fc-api-serving-bundle-{}.json", Uuid::new_v4()))
    }

    fn temp_sqlite_path() -> PathBuf {
        env::temp_dir().join(format!("fc-api-runtime-{}.sqlite", Uuid::new_v4()))
    }

    fn test_probability_bundle() -> ProbabilityBundle {
        let feature_name = "feature_a".to_string();
        ProbabilityBundle {
            bundle_id: "bundle-test".to_string(),
            market_scope: "financial_system".to_string(),
            probability_mode: "formal_bundle_v1".to_string(),
            model_family: "linear_v1".to_string(),
            feature_transform: "identity_v1".to_string(),
            created_at: Utc::now(),
            feature_names: vec![feature_name.clone()],
            monotonic_min_gap_5d_to_20d: 0.0,
            monotonic_min_gap_20d_to_60d: 0.0,
            note: String::new(),
            horizons: [5_u32, 20, 60]
                .into_iter()
                .map(|horizon_days| ProbabilityHorizonBundle {
                    horizon_days,
                    decision_threshold: None,
                    threshold_diagnostics: None,
                    raw_model: LogisticProbabilityModel {
                        intercept: 0.0,
                        feature_transform: "identity_v1".to_string(),
                        feature_stats: vec![ProbabilityFeatureStat {
                            name: feature_name.clone(),
                            mean: 0.0,
                            std_dev: 1.0,
                            fill_value: 0.0,
                        }],
                        coefficients: vec![ProbabilityCoefficient {
                            name: feature_name.clone(),
                            weight: 0.0,
                        }],
                    },
                    calibration: None,
                    evaluation: HorizonEvaluationSummary::default(),
                    family_overlays: Vec::new(),
                    family_overlay_audits: Vec::new(),
                })
                .collect(),
            evaluation: None,
            actionability: None,
        }
    }

    fn test_release(bundle_uri: &str, status: &str, serving_status: &str) -> ModelReleaseRecord {
        ModelReleaseRecord {
            manifest: ModelReleaseManifest {
                release_id: format!("release-{status}-{serving_status}"),
                market_scope: "financial_system".to_string(),
                status: status.to_string(),
                probability_mode: "formal_bundle_v1".to_string(),
                serving_status: serving_status.to_string(),
                bundle_uri: bundle_uri.to_string(),
                feature_set_version: "feature_formal_v1_main_20260531".to_string(),
                label_version: "formal_label_v1_main".to_string(),
                prob_model_version: "prob".to_string(),
                calibration_version: "calib".to_string(),
                posture_policy_version: "posture".to_string(),
                action_playbook_version: "playbook".to_string(),
                point_in_time_mode: "best_effort".to_string(),
                training_range_start: None,
                training_range_end: None,
                calibration_range_start: None,
                calibration_range_end: None,
                evaluation_range_start: None,
                evaluation_range_end: None,
                brier_score: None,
                log_loss: None,
                ece: None,
                note: String::new(),
            },
            created_at: Utc::now(),
            activated_at: None,
            retired_at: None,
        }
    }

    fn persisted_snapshot(
        as_of_date: chrono::NaiveDate,
        release_id: &str,
        method_version: &str,
    ) -> PredictionSnapshotRecord {
        PredictionSnapshotRecord {
            as_of_date,
            entity_id: "us".to_string(),
            market_scope: "financial_system".to_string(),
            release_id: Some(release_id.to_string()),
            probability_mode: "formal_bundle_v1".to_string(),
            release_status: "healthy".to_string(),
            point_in_time_mode: "best_effort".to_string(),
            overall_score: 99.0,
            external_shock_score: 99.0,
            raw_p_5d: 0.99,
            raw_p_20d: 0.99,
            raw_p_60d: 0.99,
            calibrated_p_5d: 0.99,
            calibrated_p_20d: 0.99,
            calibrated_p_60d: 0.99,
            posture: "defend".to_string(),
            time_to_risk_bucket: "now".to_string(),
            feature_set_version: "feature_formal_v1_main_20260531".to_string(),
            label_version: "formal_label_v1_main".to_string(),
            coverage_score: 1.0,
            freshness_status: "fresh".to_string(),
            method_version: method_version.to_string(),
            posture_trigger_codes: vec!["legacy_snapshot_only".to_string()],
            posture_blocker_codes: Vec::new(),
            recorded_at: Utc::now(),
        }
    }

    fn replay_run(
        replay_run_id: &str,
        release_id: &str,
        method_version: &str,
        from_date: chrono::NaiveDate,
        to_date: chrono::NaiveDate,
        source_watermark: &str,
        point_count: usize,
    ) -> HistoricalReplayRunRecord {
        HistoricalReplayRunRecord {
            replay_run_id: replay_run_id.to_string(),
            release_id: Some(release_id.to_string()),
            market_scope: "financial_system".to_string(),
            from_date,
            to_date,
            history_cache_key: method_version.to_string(),
            feature_set_version: "feature_formal_v1_main_20260531".to_string(),
            label_version: "formal_label_v1_main".to_string(),
            point_in_time_mode: "best_effort".to_string(),
            runtime_policy_version: "history_runtime_policy_v1".to_string(),
            action_playbook_version: "action_playbook_v1_20260531".to_string(),
            protected_window_catalog_id: "protected_stress_windows_v1".to_string(),
            source_watermark: source_watermark.to_string(),
            status: "success".to_string(),
            point_count,
            failure_reason: None,
            created_at: Utc::now(),
        }
    }

    fn replay_point(
        replay_run_id: &str,
        release_id: &str,
        as_of_date: chrono::NaiveDate,
        overall_score: f64,
        p_20d: f64,
        posture: &str,
    ) -> HistoricalAssessmentPointRecord {
        HistoricalAssessmentPointRecord {
            replay_run_id: replay_run_id.to_string(),
            entity_id: "us".to_string(),
            market_scope: "financial_system".to_string(),
            release_id: Some(release_id.to_string()),
            as_of_date,
            feature_snapshot_id: Some(format!("feature-snapshot-{as_of_date}")),
            point_in_time_mode: "best_effort".to_string(),
            runtime_policy_version: "history_runtime_policy_v1".to_string(),
            action_playbook_version: "action_playbook_v1_20260531".to_string(),
            overall_score,
            structural_score: overall_score,
            trigger_score: overall_score / 2.0,
            external_shock_score: overall_score / 3.0,
            raw_p_5d: 0.02,
            raw_p_20d: p_20d,
            raw_p_60d: (p_20d + 0.05).min(0.95),
            calibrated_p_5d: 0.02,
            calibrated_p_20d: p_20d,
            calibrated_p_60d: (p_20d + 0.05).min(0.95),
            posture: posture.to_string(),
            time_to_risk_bucket: "weeks".to_string(),
            actionability_prepare: p_20d,
            actionability_hedge: (p_20d / 2.0).min(0.95),
            actionability_defend: 0.0,
            probability_diagnostics: ProbabilityDiagnostics::default(),
            posture_trigger_codes: vec!["replay_history".to_string()],
            posture_blocker_codes: Vec::new(),
            coverage_score: 1.0,
            freshness_status: "fresh".to_string(),
            generated_at: Utc::now(),
        }
    }

    #[test]
    fn healthy_active_release_loads_formal_bundle() {
        let bundle_path = temp_bundle_path();
        std::fs::write(
            &bundle_path,
            serde_json::to_string(&test_probability_bundle()).unwrap(),
        )
        .unwrap();

        let context = build_serving_model_context(
            test_release(&bundle_path.to_string_lossy(), "active", "healthy"),
            ServingRuntimePurpose::Production,
        );

        assert_eq!(context.runtime_probability_mode, "formal_bundle_v1");
        assert_eq!(context.runtime_release_status, "healthy");
        assert!(context.probability_bundle.is_some());

        let _ = std::fs::remove_file(bundle_path);
    }

    #[test]
    fn shadow_active_release_falls_back_to_heuristic_runtime() {
        let bundle_path = temp_bundle_path();
        std::fs::write(
            &bundle_path,
            serde_json::to_string(&test_probability_bundle()).unwrap(),
        )
        .unwrap();

        let context = build_serving_model_context(
            test_release(&bundle_path.to_string_lossy(), "active", "shadow"),
            ServingRuntimePurpose::Production,
        );

        assert_eq!(context.runtime_probability_mode, "heuristic_mvp");
        assert_eq!(context.runtime_release_status, "degraded");
        assert!(context.probability_bundle.is_none());
        assert_eq!(context.release.manifest.serving_status, "shadow");

        let _ = std::fs::remove_file(bundle_path);
    }

    #[test]
    fn non_active_bundle_release_falls_back_to_heuristic_runtime() {
        let bundle_path = temp_bundle_path();
        std::fs::write(
            &bundle_path,
            serde_json::to_string(&test_probability_bundle()).unwrap(),
        )
        .unwrap();

        let context = build_serving_model_context(
            test_release(&bundle_path.to_string_lossy(), "candidate", "healthy"),
            ServingRuntimePurpose::Production,
        );

        assert_eq!(context.runtime_probability_mode, "heuristic_mvp");
        assert_eq!(context.runtime_release_status, "degraded");
        assert!(context.probability_bundle.is_none());
        assert_eq!(context.release.manifest.status, "candidate");

        let _ = std::fs::remove_file(bundle_path);
    }

    #[test]
    fn review_runtime_allows_shadow_release_bundle_loading() {
        let bundle_path = temp_bundle_path();
        std::fs::write(
            &bundle_path,
            serde_json::to_string(&test_probability_bundle()).unwrap(),
        )
        .unwrap();

        let context = build_serving_model_context(
            test_release(&bundle_path.to_string_lossy(), "active", "shadow"),
            ServingRuntimePurpose::Review,
        );

        assert_eq!(context.runtime_probability_mode, "formal_bundle_v1");
        assert_eq!(context.runtime_release_status, "shadow");
        assert!(context.probability_bundle.is_some());

        let _ = std::fs::remove_file(bundle_path);
    }

    #[test]
    fn review_runtime_allows_candidate_release_bundle_loading() {
        let bundle_path = temp_bundle_path();
        std::fs::write(
            &bundle_path,
            serde_json::to_string(&test_probability_bundle()).unwrap(),
        )
        .unwrap();

        let context = build_serving_model_context(
            test_release(&bundle_path.to_string_lossy(), "candidate", "healthy"),
            ServingRuntimePurpose::Review,
        );

        assert_eq!(context.runtime_probability_mode, "formal_bundle_v1");
        assert_eq!(context.runtime_release_status, "healthy");
        assert!(context.probability_bundle.is_some());

        let _ = std::fs::remove_file(bundle_path);
    }

    #[tokio::test]
    async fn production_formal_runtime_keeps_only_current_prediction_snapshot() {
        let sqlite_path = temp_sqlite_path();
        let bundle_path = temp_bundle_path();
        std::fs::write(
            &bundle_path,
            serde_json::to_string(&test_probability_bundle()).unwrap(),
        )
        .unwrap();

        let store = SqliteStore::connect(&sqlite_path).await.unwrap();
        store.migrate().await.unwrap();
        store.seed_fred_metadata().await.unwrap();

        let indicators = demo_indicators();
        for indicator in &indicators {
            store.upsert_indicator(indicator).await.unwrap();
        }
        let as_of_date = Utc::now().date_naive();
        let observations = demo_observations(as_of_date);
        store.insert_observations(&observations).await.unwrap();

        let release = test_release(&bundle_path.to_string_lossy(), "approved", "healthy");
        let release_id = release.manifest.release_id.clone();
        store.upsert_model_release(&release).await.unwrap();
        store
            .activate_model_release("financial_system", &release_id, "test")
            .await
            .unwrap();

        let serving_model =
            build_serving_model_context(release.clone(), ServingRuntimePurpose::Production);
        let method_version = expected_prediction_snapshot_method_version(Some(&serving_model));
        let old_snapshot =
            persisted_snapshot(as_of_date - Duration::days(1), &release_id, &method_version);
        let current_snapshot = persisted_snapshot(as_of_date, &release_id, &method_version);
        store
            .upsert_prediction_snapshots(&[old_snapshot, current_snapshot])
            .await
            .unwrap();

        let _ = load_app_data_with_runtime_options(
            &AppDataSource::Sqlite {
                path: sqlite_path.to_string_lossy().to_string(),
            },
            260,
            AssessmentHistoryBuildMode::Default,
            ServingRuntimePurpose::Production,
        )
        .await
        .unwrap();

        let snapshots = store
            .list_prediction_snapshots(
                Some("financial_system"),
                Some(&release_id),
                None,
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].as_of_date, as_of_date);
        assert_ne!(snapshots[0].overall_score, 99.0);

        let _ = std::fs::remove_file(sqlite_path);
        let _ = std::fs::remove_file(bundle_path);
    }

    #[tokio::test]
    async fn sqlite_runtime_prefers_long_replay_history_for_scenario_backtests() {
        let sqlite_path = temp_sqlite_path();
        let bundle_path = temp_bundle_path();
        std::fs::write(
            &bundle_path,
            serde_json::to_string(&test_probability_bundle()).unwrap(),
        )
        .unwrap();

        let store = SqliteStore::connect(&sqlite_path).await.unwrap();
        store.migrate().await.unwrap();
        store.seed_fred_metadata().await.unwrap();

        let indicators = demo_indicators();
        for indicator in &indicators {
            store.upsert_indicator(indicator).await.unwrap();
        }
        let as_of_date = Utc::now().date_naive();
        let observations = demo_observations(as_of_date);
        store.insert_observations(&observations).await.unwrap();

        let release = test_release(&bundle_path.to_string_lossy(), "active", "healthy");
        let release_id = release.manifest.release_id.clone();
        store.upsert_model_release(&release).await.unwrap();
        store
            .activate_model_release("financial_system", &release_id, "test")
            .await
            .unwrap();

        let serving_model =
            build_serving_model_context(release.clone(), ServingRuntimePurpose::Production);
        let method_version = expected_prediction_snapshot_method_version(Some(&serving_model));
        let source_watermark =
            crate::history_replay::historical_replay_source_watermark(&observations);
        let replay_run_id = "scenario-backtest-long-run";
        let replay_from = chrono::NaiveDate::from_ymd_opt(2022, 12, 15).unwrap();
        let replay_to = as_of_date;
        let replay_dates = (0_i64..100)
            .map(|offset| replay_from + Duration::days(offset))
            .chain(std::iter::once(replay_to))
            .collect::<Vec<_>>();
        store
            .upsert_historical_replay_run(&replay_run(
                replay_run_id,
                &release_id,
                &method_version,
                replay_from,
                replay_to,
                &source_watermark,
                replay_dates.len(),
            ))
            .await
            .unwrap();
        let replay_points = replay_dates
            .iter()
            .map(|date| {
                let in_regional_bank_window =
                    *date >= chrono::NaiveDate::from_ymd_opt(2023, 2, 15).unwrap()
                        && *date <= chrono::NaiveDate::from_ymd_opt(2023, 3, 20).unwrap();
                let overall_score = if in_regional_bank_window { 78.0 } else { 52.0 };
                let p_20d = if in_regional_bank_window { 0.36 } else { 0.08 };
                let posture = if in_regional_bank_window {
                    "hedge"
                } else {
                    "normal"
                };
                replay_point(
                    replay_run_id,
                    &release_id,
                    *date,
                    overall_score,
                    p_20d,
                    posture,
                )
            })
            .collect::<Vec<_>>();
        store
            .replace_historical_assessment_points(replay_run_id, &replay_points)
            .await
            .unwrap();

        let data = load_app_data_with_runtime_options(
            &AppDataSource::Sqlite {
                path: sqlite_path.to_string_lossy().to_string(),
            },
            30,
            AssessmentHistoryBuildMode::Default,
            ServingRuntimePurpose::Production,
        )
        .await
        .unwrap();

        assert!(data.assessment_history.len() <= 30);
        assert!(data.assessment_history.len() < replay_dates.len());
        assert!(
            data.backtests
                .iter()
                .any(|scenario| scenario.signal_source == fc_domain::BacktestSignalSource::RealHistory),
            "scenario backtests should reuse persisted long replay history when it exists"
        );
        assert_eq!(data.assessment.backtest_summary.history_start, Some(replay_from));
        assert_eq!(data.assessment.backtest_summary.history_end, Some(replay_to));
        assert_eq!(
            data.assessment.backtest_summary.rolling_audit.history_start,
            Some(replay_from)
        );
        assert_eq!(
            data.assessment.backtest_summary.rolling_audit.history_end,
            Some(replay_to)
        );
        assert!(
            data.assessment
                .backtest_summary
                .coverage_scope_note
                .contains("persisted replay 历史")
        );
        assert!(
            data.assessment
                .backtest_summary
                .rolling_audit
                .scope_note
                .contains("persisted replay 历史")
        );

        let _ = std::fs::remove_file(sqlite_path);
        let _ = std::fs::remove_file(bundle_path);
    }
}

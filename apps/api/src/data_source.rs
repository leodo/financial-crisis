use std::{env, fs};

use anyhow::Context;
use chrono::{Duration, Utc};
use fc_domain::{DataMode, ModelReleaseRecord, ProbabilityBundle};
use fc_storage::{PostgresStore, SqliteStore};

use crate::{
    assessment::ServingModelContext,
    demo::{build_app_data_from_inputs, build_demo_data, load_user_preferences, BuiltAppData},
    history_builder::{
        build_assessment_history, load_sqlite_assessment_history, HistoryQueryWindow,
    },
    AppData,
};

const EVENT_LOOKBACK_DAYS: i64 = 30;

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
        user_preferences,
    )
    .app_data)
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
    let built: BuiltAppData = build_app_data_from_inputs(
        DataMode::Sqlite,
        indicators,
        observations,
        Some(alerts),
        serving_model,
        as_of_date,
        assessment_history,
        user_preferences,
    );
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

    use chrono::Utc;
    use fc_domain::{
        HorizonEvaluationSummary, LogisticProbabilityModel, ModelReleaseManifest,
        ModelReleaseRecord, ProbabilityBundle, ProbabilityCoefficient, ProbabilityFeatureStat,
        ProbabilityHorizonBundle,
    };
    use uuid::Uuid;

    use super::{build_serving_model_context, ServingRuntimePurpose};

    fn temp_bundle_path() -> PathBuf {
        env::temp_dir().join(format!("fc-api-serving-bundle-{}.json", Uuid::new_v4()))
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
}

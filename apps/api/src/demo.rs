use std::{
    collections::{BTreeMap, BTreeSet},
    env,
};

use chrono::{Duration, NaiveDate, Utc};
use fc_domain::{
    load_crisis_scenario_catalog, load_protected_stress_window_catalog, AlertEvent, AlertStatus,
    AlertType, AssessmentSnapshot, BacktestRollingAudit, BacktestRollingAuditEpisode,
    BacktestScenarioSummary, BacktestSignalSource, BacktestWindowPoint, DataMode, DataSource,
    DecisionPosture, Frequency, FreshnessStatus, HistoricalAssessmentPointRecord,
    HistoricalReplayRunRecord, Indicator, Observation, PostureGuidance, PredictionSnapshotRecord,
    ProtectedStressWindow, RiskContributor, RiskDimension, RiskDirection, RiskLevel, SourceHealth,
    SourcePriority, SourceStatus, TimeToRiskBucket, UserRiskPreferences, UserRiskProfile,
};
use fc_scoring::ScoringEngine;
use fc_storage::SqliteStore;
use uuid::Uuid;

use crate::assessment::{
    build_assessment_snapshot, build_backtest_summary, history_runtime_policy_version,
    runtime_threshold_diagnostics, ServingModelContext,
};
use crate::data_source::AssessmentHistoryBuildMode;
use crate::AppData;

const EVENT_LOOKBACK_DAYS: i64 = 30;
const BACKTEST_SIGNAL_WINDOW: usize = 5;
const BACKTEST_SIGNAL_MIN_HITS: usize = 3;
const ACTIONABLE_AUDIT_HORIZON_DEFEND_DAYS: i64 = 5;
const ACTIONABLE_AUDIT_HORIZON_HEDGE_DAYS: i64 = 20;
const ACTIONABLE_AUDIT_HORIZON_PREPARE_DAYS: i64 = 60;
const ROLLING_AUDIT_EPISODE_LIMIT: usize = 12;
const ROLLING_AUDIT_MIN_DATE: (i32, u32, u32) = (1990, 1, 2);
const PREDICTION_SNAPSHOT_CACHE_VERSION: &str = "history_cache_v3_20260601";
const FORMAL_MAIN_FEATURE_SET_VERSION: &str = "feature_formal_v1_main_20260531";
const FORMAL_MAIN_LABEL_VERSION: &str = "formal_label_v1_main";

#[derive(Debug, Clone, Copy)]
pub struct HistoryQueryWindow {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub limit: Option<usize>,
}

#[derive(Debug)]
pub(crate) struct BuiltAppData {
    pub(crate) app_data: AppData,
    pub(crate) prediction_snapshots: Vec<PredictionSnapshotRecord>,
}

#[derive(Debug)]
pub(crate) struct HistoricalAssessmentOutput {
    pub(crate) history_points: Vec<fc_domain::AssessmentHistoryPoint>,
    prediction_snapshots: Vec<PredictionSnapshotRecord>,
    replay_point_drafts: Vec<HistoricalReplayPointDraft>,
}

#[derive(Debug, Clone)]
struct HistoricalReplayPointDraft {
    entity_id: String,
    market_scope: String,
    release_id: Option<String>,
    as_of_date: NaiveDate,
    feature_snapshot_id: Option<String>,
    feature_set_version: String,
    label_version: String,
    point_in_time_mode: String,
    runtime_policy_version: String,
    action_playbook_version: String,
    overall_score: f64,
    structural_score: f64,
    trigger_score: f64,
    external_shock_score: f64,
    raw_p_5d: f64,
    raw_p_20d: f64,
    raw_p_60d: f64,
    calibrated_p_5d: f64,
    calibrated_p_20d: f64,
    calibrated_p_60d: f64,
    posture: String,
    time_to_risk_bucket: String,
    actionability_prepare: f64,
    actionability_hedge: f64,
    actionability_defend: f64,
    posture_trigger_codes: Vec<String>,
    posture_blocker_codes: Vec<String>,
    coverage_score: f64,
    freshness_status: String,
    generated_at: chrono::DateTime<Utc>,
}

pub fn build_demo_data(_max_history_points: usize) -> AppData {
    let as_of_date = NaiveDate::from_ymd_opt(2026, 5, 30).expect("valid date");
    let indicators = indicators();
    let observations = observations(as_of_date);
    let user_preferences = load_user_preferences();
    let historical = build_assessment_history(
        DataMode::Demo,
        &ScoringEngine::default(),
        &indicators,
        &observations,
        None,
        None,
        &user_preferences,
        HistoryQueryWindow {
            from: None,
            to: None,
            limit: None,
        },
    );
    build_app_data_from_inputs(
        DataMode::Demo,
        indicators,
        observations,
        None,
        None,
        as_of_date,
        historical.history_points,
        user_preferences,
    )
    .app_data
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_app_data_from_inputs(
    data_mode: DataMode,
    indicators: Vec<Indicator>,
    observations: Vec<Observation>,
    stored_alerts: Option<Vec<AlertEvent>>,
    serving_model: Option<ServingModelContext>,
    as_of_date: NaiveDate,
    mut assessment_history: Vec<fc_domain::AssessmentHistoryPoint>,
    user_preferences: UserRiskPreferences,
) -> BuiltAppData {
    let use_transitional_bridge = use_transitional_actionable_bridge(serving_model.as_ref());
    let scoring = ScoringEngine::default();
    let protected_stress_window_catalog = load_protected_stress_window_catalog();
    let threshold_diagnostics = runtime_threshold_diagnostics(serving_model.as_ref());
    let output = scoring.score(
        &indicators,
        &observations,
        as_of_date,
        "us",
        "financial_system",
    );
    let backtests = build_backtests(
        &output.snapshot,
        &assessment_history,
        use_transitional_bridge,
    );
    let rolling_audit = build_rolling_backtest_audit(
        &assessment_history,
        &protected_stress_window_catalog.windows,
        use_transitional_bridge,
    );
    let alerts = stored_alerts
        .map(|alerts| select_recent_alerts_for_date(&alerts, as_of_date))
        .unwrap_or_else(|| build_alerts(&output.snapshot));
    let backtest_summary = build_backtest_summary(&backtests, Some(&rolling_audit));
    let (assessment, posture_guidance, probability_trace) = build_assessment_snapshot(
        data_mode,
        &output.snapshot,
        &output.indicator_risks,
        &observations,
        &alerts,
        &backtests,
        Some(&rolling_audit),
        serving_model.as_ref(),
        &user_preferences,
    );
    let assessment = fc_domain::AssessmentSnapshot {
        backtest_summary,
        ..assessment
    };
    let current_history_point = assessment_history_point_from_assessment(
        &assessment,
        &posture_guidance,
        &probability_trace,
    );
    match assessment_history.last_mut() {
        Some(last) if last.as_of_date == current_history_point.as_of_date => {
            *last = current_history_point;
        }
        _ => assessment_history.push(current_history_point),
    }
    let backtest_timeline = build_backtest_timeline(&assessment_history, use_transitional_bridge);
    let current_prediction_snapshot = prediction_snapshot_from_assessment(
        &assessment,
        &posture_guidance,
        &probability_trace,
        serving_model.as_ref(),
    );
    BuiltAppData {
        app_data: AppData {
            data_mode,
            user_preferences,
            overview: output.snapshot,
            indicators: output.indicator_risks,
            alerts,
            sources: if matches!(data_mode, DataMode::Demo) {
                sources_demo()
            } else {
                sources_runtime(&observations, as_of_date)
            },
            backtests,
            backtest_timeline,
            assessment,
            assessment_history,
            posture_guidance,
            protected_stress_window_catalog,
            runtime_thresholds: threshold_diagnostics,
        },
        prediction_snapshots: vec![current_prediction_snapshot],
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_assessment_history(
    data_mode: DataMode,
    scoring: &ScoringEngine,
    indicators: &[Indicator],
    observations: &[Observation],
    stored_alerts: Option<&[AlertEvent]>,
    serving_model: Option<&ServingModelContext>,
    user_preferences: &UserRiskPreferences,
    window: HistoryQueryWindow,
) -> HistoricalAssessmentOutput {
    let mut dates = observations
        .iter()
        .filter(|observation| observation.entity_id == "us")
        .map(|observation| observation.as_of_date)
        .collect::<Vec<_>>();
    dates.sort();
    dates.dedup();
    if let Some(from) = window.from {
        dates.retain(|date| *date >= from);
    }
    if let Some(to) = window.to {
        dates.retain(|date| *date <= to);
    }
    if let Some(limit) = window.limit {
        if dates.len() > limit {
            dates = dates[dates.len().saturating_sub(limit)..].to_vec();
        }
    }

    build_assessment_history_for_dates(
        data_mode,
        scoring,
        indicators,
        observations,
        stored_alerts,
        serving_model,
        user_preferences,
        &dates,
    )
}

#[allow(clippy::too_many_arguments)]
fn build_assessment_history_for_dates(
    data_mode: DataMode,
    scoring: &ScoringEngine,
    indicators: &[Indicator],
    observations: &[Observation],
    stored_alerts: Option<&[AlertEvent]>,
    serving_model: Option<&ServingModelContext>,
    user_preferences: &UserRiskPreferences,
    dates: &[NaiveDate],
) -> HistoricalAssessmentOutput {
    let mut history_points = Vec::with_capacity(dates.len());
    let mut prediction_snapshots = Vec::with_capacity(dates.len());
    let mut replay_point_drafts = Vec::with_capacity(dates.len());
    for as_of_date in dates.iter().copied() {
        let output = scoring.score(
            indicators,
            observations,
            as_of_date,
            "us",
            "financial_system",
        );
        let point_alerts = stored_alerts
            .map(|alerts| select_recent_alerts_for_date(alerts, as_of_date))
            .unwrap_or_else(|| build_alerts(&output.snapshot));
        let point_backtests = build_backtests(
            &output.snapshot,
            &[],
            use_transitional_actionable_bridge(serving_model),
        );
        let (assessment, posture_guidance, probability_trace) = build_assessment_snapshot(
            data_mode,
            &output.snapshot,
            &output.indicator_risks,
            observations,
            &point_alerts,
            &point_backtests,
            None,
            serving_model,
            user_preferences,
        );
        history_points.push(assessment_history_point_from_assessment(
            &assessment,
            &posture_guidance,
            &probability_trace,
        ));
        replay_point_drafts.push(historical_replay_point_draft_from_assessment(
            &assessment,
            &posture_guidance,
            &probability_trace,
            serving_model,
        ));
        prediction_snapshots.push(prediction_snapshot_from_assessment(
            &assessment,
            &posture_guidance,
            &probability_trace,
            serving_model,
        ));
    }

    HistoricalAssessmentOutput {
        history_points,
        prediction_snapshots,
        replay_point_drafts,
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn load_sqlite_assessment_history(
    store: &SqliteStore,
    indicators: &[Indicator],
    observations: &[Observation],
    alerts: &[AlertEvent],
    serving_model: Option<&ServingModelContext>,
    user_preferences: &UserRiskPreferences,
    as_of_date: NaiveDate,
    max_history_points: usize,
    history_build_mode: AssessmentHistoryBuildMode,
) -> anyhow::Result<Vec<fc_domain::AssessmentHistoryPoint>> {
    let release_filter = serving_model.map(|context| context.release.manifest.release_id.as_str());
    let persisted_rows = store
        .list_prediction_snapshots(Some("financial_system"), release_filter, None, None, None)
        .await?;
    let target_dates = observations
        .iter()
        .filter(|observation| observation.entity_id == "us")
        .map(|observation| observation.as_of_date)
        .chain(std::iter::once(as_of_date))
        .collect::<BTreeSet<_>>();
    let existing_dates = persisted_rows
        .iter()
        .map(|snapshot| snapshot.as_of_date)
        .collect::<BTreeSet<_>>();
    let missing_dates = target_dates
        .difference(&existing_dates)
        .copied()
        .collect::<Vec<_>>();
    let full_history_refresh = should_refresh_full_release_history(
        serving_model,
        &persisted_rows,
        !missing_dates.is_empty(),
    );

    if matches!(
        history_build_mode,
        AssessmentHistoryBuildMode::StrictRebuild
    ) {
        let rebuild_dates = target_dates.into_iter().collect::<Vec<_>>();
        tracing::info!(
            release_id = release_filter.unwrap_or("heuristic"),
            rebuild_dates = rebuild_dates.len(),
            "strictly rebuilding full release history from raw observations for current reload"
        );
        let rebuilt = build_assessment_history_for_dates(
            DataMode::Sqlite,
            &ScoringEngine::default(),
            indicators,
            observations,
            Some(alerts),
            serving_model,
            user_preferences,
            &rebuild_dates,
        );
        store
            .upsert_prediction_snapshots(&rebuilt.prediction_snapshots)
            .await?;
        persist_historical_replay_output(store, observations, serving_model, &rebuilt).await?;
        return Ok(rebuilt.history_points);
    }

    if full_history_refresh {
        let rebuild_dates = target_dates.into_iter().collect::<Vec<_>>();
        tracing::info!(
            release_id = release_filter.unwrap_or("heuristic"),
            rebuild_dates = rebuild_dates.len(),
            "rebuilding full release history from raw observations because cached prediction snapshots are stale or incomplete"
        );
        let rebuilt = build_assessment_history_for_dates(
            DataMode::Sqlite,
            &ScoringEngine::default(),
            indicators,
            observations,
            Some(alerts),
            serving_model,
            user_preferences,
            &rebuild_dates,
        );
        store
            .upsert_prediction_snapshots(&rebuilt.prediction_snapshots)
            .await?;
        persist_historical_replay_output(store, observations, serving_model, &rebuilt).await?;
        return Ok(rebuilt.history_points);
    }

    if let Some(cached_replay) =
        load_cached_historical_replay_output(store, serving_model, &target_dates).await?
    {
        return Ok(cached_replay.history_points);
    }

    let mut historical =
        historical_output_from_prediction_snapshots(persisted_rows.clone(), release_filter);

    if !missing_dates.is_empty() {
        let computed = build_assessment_history_for_dates(
            DataMode::Sqlite,
            &ScoringEngine::default(),
            indicators,
            observations,
            Some(alerts),
            serving_model,
            user_preferences,
            &missing_dates,
        );
        store
            .upsert_prediction_snapshots(&computed.prediction_snapshots)
            .await?;
        let mut combined_snapshots = persisted_rows;
        combined_snapshots.extend(computed.prediction_snapshots.clone());
        historical = merge_historical_outputs(
            historical_output_from_prediction_snapshots(combined_snapshots, release_filter),
            computed,
        );
    }

    let should_refresh_recent_formal_history = serving_model
        .and_then(|context| context.probability_bundle.as_ref())
        .is_some()
        && max_history_points > 0;
    if should_refresh_recent_formal_history {
        let recent_dates = target_dates
            .iter()
            .copied()
            .rev()
            .take(max_history_points)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>();
        let recomputed = build_assessment_history_for_dates(
            DataMode::Sqlite,
            &ScoringEngine::default(),
            indicators,
            observations,
            Some(alerts),
            serving_model,
            user_preferences,
            &recent_dates,
        );
        store
            .upsert_prediction_snapshots(&recomputed.prediction_snapshots)
            .await?;
        let mut combined_snapshots = historical.prediction_snapshots.clone();
        combined_snapshots.extend(recomputed.prediction_snapshots.clone());
        historical = merge_historical_outputs(
            historical_output_from_prediction_snapshots(combined_snapshots, release_filter),
            recomputed,
        );
    }

    Ok(historical.history_points)
}

async fn persist_historical_replay_output(
    store: &SqliteStore,
    observations: &[Observation],
    serving_model: Option<&ServingModelContext>,
    output: &HistoricalAssessmentOutput,
) -> anyhow::Result<()> {
    let Some(first_point) = output.replay_point_drafts.first() else {
        return Ok(());
    };
    let Some(last_point) = output.replay_point_drafts.last() else {
        return Ok(());
    };

    let replay_run_id = Uuid::new_v4().to_string();
    let protected_stress_window_catalog = load_protected_stress_window_catalog();
    let created_at = output
        .replay_point_drafts
        .last()
        .map(|point| point.generated_at)
        .unwrap_or_else(Utc::now);
    let run = HistoricalReplayRunRecord {
        replay_run_id: replay_run_id.clone(),
        release_id: first_point.release_id.clone(),
        market_scope: first_point.market_scope.clone(),
        from_date: first_point.as_of_date,
        to_date: last_point.as_of_date,
        history_cache_key: expected_prediction_snapshot_method_version(serving_model),
        feature_set_version: first_point.feature_set_version.clone(),
        label_version: first_point.label_version.clone(),
        point_in_time_mode: first_point.point_in_time_mode.clone(),
        runtime_policy_version: first_point.runtime_policy_version.clone(),
        action_playbook_version: first_point.action_playbook_version.clone(),
        protected_window_catalog_id: protected_stress_window_catalog.catalog_id,
        source_watermark: historical_replay_source_watermark(observations),
        status: "success".to_string(),
        point_count: output.replay_point_drafts.len(),
        failure_reason: None,
        created_at,
    };
    let points = output
        .replay_point_drafts
        .iter()
        .cloned()
        .map(|point| HistoricalAssessmentPointRecord {
            replay_run_id: replay_run_id.clone(),
            entity_id: point.entity_id,
            market_scope: point.market_scope,
            release_id: point.release_id,
            as_of_date: point.as_of_date,
            feature_snapshot_id: point.feature_snapshot_id,
            point_in_time_mode: point.point_in_time_mode,
            runtime_policy_version: point.runtime_policy_version,
            action_playbook_version: point.action_playbook_version,
            overall_score: point.overall_score,
            structural_score: point.structural_score,
            trigger_score: point.trigger_score,
            external_shock_score: point.external_shock_score,
            raw_p_5d: point.raw_p_5d,
            raw_p_20d: point.raw_p_20d,
            raw_p_60d: point.raw_p_60d,
            calibrated_p_5d: point.calibrated_p_5d,
            calibrated_p_20d: point.calibrated_p_20d,
            calibrated_p_60d: point.calibrated_p_60d,
            posture: point.posture,
            time_to_risk_bucket: point.time_to_risk_bucket,
            actionability_prepare: point.actionability_prepare,
            actionability_hedge: point.actionability_hedge,
            actionability_defend: point.actionability_defend,
            posture_trigger_codes: point.posture_trigger_codes,
            posture_blocker_codes: point.posture_blocker_codes,
            coverage_score: point.coverage_score,
            freshness_status: point.freshness_status,
            generated_at: point.generated_at,
        })
        .collect::<Vec<_>>();

    store.upsert_historical_replay_run(&run).await?;
    store
        .replace_historical_assessment_points(&replay_run_id, &points)
        .await?;
    Ok(())
}

async fn load_cached_historical_replay_output(
    store: &SqliteStore,
    serving_model: Option<&ServingModelContext>,
    target_dates: &BTreeSet<NaiveDate>,
) -> anyhow::Result<Option<HistoricalAssessmentOutput>> {
    let Some(from_date) = target_dates.first().copied() else {
        return Ok(None);
    };
    let Some(to_date) = target_dates.last().copied() else {
        return Ok(None);
    };
    let release_filter = serving_model.map(|context| context.release.manifest.release_id.as_str());
    let history_cache_key = expected_prediction_snapshot_method_version(serving_model);
    let Some(run) = store
        .load_latest_historical_replay_run(
            "financial_system",
            release_filter,
            &history_cache_key,
            from_date,
            to_date,
        )
        .await?
    else {
        return Ok(None);
    };

    let points = store
        .list_historical_assessment_points(
            Some(&run.replay_run_id),
            Some("financial_system"),
            release_filter,
            Some(from_date),
            Some(to_date),
            None,
        )
        .await?;
    let available_dates = points
        .iter()
        .map(|point| point.as_of_date)
        .collect::<BTreeSet<_>>();
    if available_dates != *target_dates {
        tracing::warn!(
            replay_run_id = run.replay_run_id,
            expected_dates = target_dates.len(),
            available_dates = available_dates.len(),
            "cached historical replay run does not fully cover target dates; falling back to legacy snapshot history"
        );
        return Ok(None);
    }

    Ok(Some(historical_output_from_replay_points(points)))
}

fn historical_replay_source_watermark(observations: &[Observation]) -> String {
    observations
        .iter()
        .filter(|observation| observation.entity_id == "us")
        .map(|observation| observation.as_of_date)
        .max()
        .map(|date| format!("us_observations={date}"))
        .unwrap_or_else(|| "us_observations=missing".to_string())
}

fn historical_output_from_prediction_snapshots(
    snapshots: Vec<PredictionSnapshotRecord>,
    release_filter: Option<&str>,
) -> HistoricalAssessmentOutput {
    let filtered = snapshots
        .into_iter()
        .filter(|snapshot| match release_filter {
            Some(release_id) => snapshot.release_id.as_deref() == Some(release_id),
            None => snapshot.release_id.is_none(),
        });

    let mut latest_by_date = BTreeMap::<NaiveDate, PredictionSnapshotRecord>::new();
    for snapshot in filtered {
        match latest_by_date.get(&snapshot.as_of_date) {
            Some(existing) if existing.recorded_at >= snapshot.recorded_at => {}
            _ => {
                latest_by_date.insert(snapshot.as_of_date, snapshot);
            }
        }
    }

    let prediction_snapshots = latest_by_date.into_values().collect::<Vec<_>>();
    let history_points = prediction_snapshots
        .iter()
        .map(assessment_history_point_from_prediction_snapshot)
        .collect::<Vec<_>>();

    HistoricalAssessmentOutput {
        history_points,
        prediction_snapshots,
        replay_point_drafts: Vec::new(),
    }
}

fn historical_output_from_replay_points(
    points: Vec<HistoricalAssessmentPointRecord>,
) -> HistoricalAssessmentOutput {
    let mut latest_by_date = BTreeMap::<NaiveDate, HistoricalAssessmentPointRecord>::new();
    for point in points {
        match latest_by_date.get(&point.as_of_date) {
            Some(existing) if existing.generated_at >= point.generated_at => {}
            _ => {
                latest_by_date.insert(point.as_of_date, point);
            }
        }
    }

    let replay_points = latest_by_date.into_values().collect::<Vec<_>>();
    let history_points = replay_points
        .iter()
        .map(assessment_history_point_from_historical_replay_point)
        .collect::<Vec<_>>();

    HistoricalAssessmentOutput {
        history_points,
        prediction_snapshots: Vec::new(),
        replay_point_drafts: Vec::new(),
    }
}

fn merge_historical_outputs(
    base: HistoricalAssessmentOutput,
    recomputed: HistoricalAssessmentOutput,
) -> HistoricalAssessmentOutput {
    let mut history_by_date = base
        .history_points
        .into_iter()
        .map(|point| (point.as_of_date, point))
        .collect::<BTreeMap<_, _>>();
    for point in recomputed.history_points {
        history_by_date.insert(point.as_of_date, point);
    }

    let mut snapshots_by_date = base
        .prediction_snapshots
        .into_iter()
        .map(|snapshot| (snapshot.as_of_date, snapshot))
        .collect::<BTreeMap<_, _>>();
    for snapshot in recomputed.prediction_snapshots {
        snapshots_by_date.insert(snapshot.as_of_date, snapshot);
    }

    let mut replay_points_by_date = base
        .replay_point_drafts
        .into_iter()
        .map(|point| (point.as_of_date, point))
        .collect::<BTreeMap<_, _>>();
    for point in recomputed.replay_point_drafts {
        replay_points_by_date.insert(point.as_of_date, point);
    }

    HistoricalAssessmentOutput {
        history_points: history_by_date.into_values().collect(),
        prediction_snapshots: snapshots_by_date.into_values().collect(),
        replay_point_drafts: replay_points_by_date.into_values().collect(),
    }
}

fn prediction_snapshot_from_assessment(
    assessment: &AssessmentSnapshot,
    posture_guidance: &PostureGuidance,
    probability_trace: &crate::assessment::ProbabilityComputationTrace,
    serving_model: Option<&ServingModelContext>,
) -> PredictionSnapshotRecord {
    PredictionSnapshotRecord {
        as_of_date: assessment.as_of_date,
        entity_id: assessment.entity_id.clone(),
        market_scope: assessment.market_scope.clone(),
        release_id: assessment.method.release_id.clone(),
        probability_mode: assessment.method.probability_mode.clone(),
        release_status: assessment.method.release_status.clone(),
        point_in_time_mode: assessment.method.point_in_time_mode.clone(),
        overall_score: assessment.scores.overall_score,
        external_shock_score: assessment.scores.external_shock_score,
        raw_p_5d: probability_trace.raw_probabilities.p_5d,
        raw_p_20d: probability_trace.raw_probabilities.p_20d,
        raw_p_60d: probability_trace.raw_probabilities.p_60d,
        calibrated_p_5d: assessment.probabilities.p_5d,
        calibrated_p_20d: assessment.probabilities.p_20d,
        calibrated_p_60d: assessment.probabilities.p_60d,
        posture: decision_posture_code(assessment.posture).to_string(),
        time_to_risk_bucket: time_to_risk_bucket_code(assessment.time_to_risk_bucket).to_string(),
        feature_set_version: assessment.method.feature_set_version.clone(),
        label_version: assessment.method.label_version.clone(),
        coverage_score: assessment.data_trust.coverage_score,
        freshness_status: worst_freshness_status(&assessment.key_indicators).to_string(),
        method_version: expected_prediction_snapshot_method_version(serving_model),
        posture_trigger_codes: posture_guidance.trigger_codes.clone(),
        posture_blocker_codes: posture_guidance.blocker_codes.clone(),
        recorded_at: assessment.runtime.generated_at,
    }
}

fn historical_replay_point_draft_from_assessment(
    assessment: &AssessmentSnapshot,
    posture_guidance: &PostureGuidance,
    probability_trace: &crate::assessment::ProbabilityComputationTrace,
    serving_model: Option<&ServingModelContext>,
) -> HistoricalReplayPointDraft {
    HistoricalReplayPointDraft {
        entity_id: assessment.entity_id.clone(),
        market_scope: assessment.market_scope.clone(),
        release_id: assessment.method.release_id.clone(),
        as_of_date: assessment.as_of_date,
        feature_snapshot_id: feature_snapshot_id_for_replay_point(assessment, serving_model),
        feature_set_version: assessment.method.feature_set_version.clone(),
        label_version: assessment.method.label_version.clone(),
        point_in_time_mode: assessment.method.point_in_time_mode.clone(),
        runtime_policy_version: history_runtime_policy_version(serving_model),
        action_playbook_version: assessment.method.action_playbook_version.clone(),
        overall_score: assessment.scores.overall_score,
        structural_score: assessment.scores.structural_score,
        trigger_score: assessment.scores.trigger_score,
        external_shock_score: assessment.scores.external_shock_score,
        raw_p_5d: probability_trace.raw_probabilities.p_5d,
        raw_p_20d: probability_trace.raw_probabilities.p_20d,
        raw_p_60d: probability_trace.raw_probabilities.p_60d,
        calibrated_p_5d: assessment.probabilities.p_5d,
        calibrated_p_20d: assessment.probabilities.p_20d,
        calibrated_p_60d: assessment.probabilities.p_60d,
        posture: decision_posture_code(assessment.posture).to_string(),
        time_to_risk_bucket: time_to_risk_bucket_code(assessment.time_to_risk_bucket).to_string(),
        actionability_prepare: assessment.actionability.prepare,
        actionability_hedge: assessment.actionability.hedge,
        actionability_defend: assessment.actionability.defend,
        posture_trigger_codes: posture_guidance.trigger_codes.clone(),
        posture_blocker_codes: posture_guidance.blocker_codes.clone(),
        coverage_score: assessment.data_trust.coverage_score,
        freshness_status: worst_freshness_status(&assessment.key_indicators).to_string(),
        generated_at: assessment.runtime.generated_at,
    }
}

fn feature_snapshot_id_for_replay_point(
    assessment: &AssessmentSnapshot,
    serving_model: Option<&ServingModelContext>,
) -> Option<String> {
    serving_model.as_ref()?;
    Some(format!(
        "{}:{}:{}:{}:{}",
        assessment.market_scope,
        assessment.entity_id,
        assessment.as_of_date,
        assessment.method.feature_set_version,
        assessment.method.point_in_time_mode
    ))
}

fn expected_prediction_snapshot_method_version(
    serving_model: Option<&ServingModelContext>,
) -> String {
    let Some(serving_model) = serving_model else {
        return history_cache_key(
            None,
            "heuristic_mvp",
            "feature_v2_20260531",
            "label_v1_20260530",
            "prob_v1_20260531",
            "calib_v1_20260531",
            "posture_v1_20260530",
            "action_playbook_v1_20260531",
            "best_effort",
            &history_runtime_policy_version(None),
        );
    };

    history_cache_key(
        Some(serving_model.release.manifest.release_id.as_str()),
        &serving_model.runtime_probability_mode,
        &serving_model.release.manifest.feature_set_version,
        &serving_model.release.manifest.label_version,
        &serving_model.release.manifest.prob_model_version,
        &serving_model.release.manifest.calibration_version,
        &serving_model.release.manifest.posture_policy_version,
        &serving_model.release.manifest.action_playbook_version,
        &serving_model.release.manifest.point_in_time_mode,
        &history_runtime_policy_version(Some(serving_model)),
    )
}

#[allow(clippy::too_many_arguments)]
fn history_cache_key(
    release_id: Option<&str>,
    probability_mode: &str,
    feature_set_version: &str,
    label_version: &str,
    prob_model_version: &str,
    calibration_version: &str,
    posture_policy_version: &str,
    action_playbook_version: &str,
    point_in_time_mode: &str,
    runtime_history_policy_version: &str,
) -> String {
    format!(
        "{PREDICTION_SNAPSHOT_CACHE_VERSION}|release={}|probability_mode={probability_mode}|feature={feature_set_version}|label={label_version}|prob={prob_model_version}|calib={calibration_version}|posture={posture_policy_version}|action={action_playbook_version}|pit={point_in_time_mode}|runtime_policy={runtime_history_policy_version}",
        release_id.unwrap_or("heuristic")
    )
}

fn should_refresh_full_release_history(
    serving_model: Option<&ServingModelContext>,
    persisted_rows: &[PredictionSnapshotRecord],
    has_missing_dates: bool,
) -> bool {
    if !uses_bundle_backed_history(serving_model) {
        return false;
    }

    if persisted_rows.is_empty() || has_missing_dates {
        return true;
    }

    let expected_method_version = expected_prediction_snapshot_method_version(serving_model);
    persisted_rows
        .iter()
        .any(|snapshot| snapshot.method_version != expected_method_version)
}

fn uses_bundle_backed_history(serving_model: Option<&ServingModelContext>) -> bool {
    serving_model.is_some_and(|context| context.probability_bundle.is_some())
}

fn is_formal_main_release(serving_model: Option<&ServingModelContext>) -> bool {
    serving_model.is_some_and(|context| {
        context.release.manifest.feature_set_version == FORMAL_MAIN_FEATURE_SET_VERSION
            && context.release.manifest.label_version == FORMAL_MAIN_LABEL_VERSION
            && context.probability_bundle.is_some()
    })
}

fn assessment_history_point_from_assessment(
    assessment: &AssessmentSnapshot,
    posture_guidance: &PostureGuidance,
    probability_trace: &crate::assessment::ProbabilityComputationTrace,
) -> fc_domain::AssessmentHistoryPoint {
    fc_domain::AssessmentHistoryPoint {
        as_of_date: assessment.as_of_date,
        overall_score: assessment.scores.overall_score,
        p_5d: assessment.probabilities.p_5d,
        p_20d: assessment.probabilities.p_20d,
        p_60d: assessment.probabilities.p_60d,
        raw_p_5d: Some(probability_trace.raw_probabilities.p_5d),
        raw_p_20d: Some(probability_trace.raw_probabilities.p_20d),
        raw_p_60d: Some(probability_trace.raw_probabilities.p_60d),
        posture: assessment.posture,
        time_to_risk_bucket: assessment.time_to_risk_bucket,
        external_shock_score: assessment.scores.external_shock_score,
        posture_trigger_codes: posture_guidance.trigger_codes.clone(),
        posture_blocker_codes: posture_guidance.blocker_codes.clone(),
    }
}

fn assessment_history_point_from_prediction_snapshot(
    snapshot: &PredictionSnapshotRecord,
) -> fc_domain::AssessmentHistoryPoint {
    fc_domain::AssessmentHistoryPoint {
        as_of_date: snapshot.as_of_date,
        overall_score: snapshot.overall_score,
        p_5d: snapshot.calibrated_p_5d,
        p_20d: snapshot.calibrated_p_20d,
        p_60d: snapshot.calibrated_p_60d,
        raw_p_5d: Some(snapshot.raw_p_5d),
        raw_p_20d: Some(snapshot.raw_p_20d),
        raw_p_60d: Some(snapshot.raw_p_60d),
        posture: parse_decision_posture_code(&snapshot.posture),
        time_to_risk_bucket: parse_time_to_risk_bucket_code(&snapshot.time_to_risk_bucket),
        external_shock_score: snapshot.external_shock_score,
        posture_trigger_codes: snapshot.posture_trigger_codes.clone(),
        posture_blocker_codes: snapshot.posture_blocker_codes.clone(),
    }
}

fn assessment_history_point_from_historical_replay_point(
    point: &HistoricalAssessmentPointRecord,
) -> fc_domain::AssessmentHistoryPoint {
    fc_domain::AssessmentHistoryPoint {
        as_of_date: point.as_of_date,
        overall_score: point.overall_score,
        p_5d: point.calibrated_p_5d,
        p_20d: point.calibrated_p_20d,
        p_60d: point.calibrated_p_60d,
        raw_p_5d: Some(point.raw_p_5d),
        raw_p_20d: Some(point.raw_p_20d),
        raw_p_60d: Some(point.raw_p_60d),
        posture: parse_decision_posture_code(&point.posture),
        time_to_risk_bucket: parse_time_to_risk_bucket_code(&point.time_to_risk_bucket),
        external_shock_score: point.external_shock_score,
        posture_trigger_codes: point.posture_trigger_codes.clone(),
        posture_blocker_codes: point.posture_blocker_codes.clone(),
    }
}

fn worst_freshness_status(key_indicators: &[fc_domain::KeyIndicatorStatus]) -> &'static str {
    let status = key_indicators
        .iter()
        .map(|item| item.status)
        .max_by_key(|status| match status {
            FreshnessStatus::Fresh => 0,
            FreshnessStatus::Delayed => 1,
            FreshnessStatus::Stale => 2,
            FreshnessStatus::Missing => 3,
        })
        .unwrap_or(FreshnessStatus::Missing);
    freshness_status_code(status)
}

fn freshness_status_code(status: FreshnessStatus) -> &'static str {
    match status {
        FreshnessStatus::Fresh => "fresh",
        FreshnessStatus::Delayed => "delayed",
        FreshnessStatus::Stale => "stale",
        FreshnessStatus::Missing => "missing",
    }
}

fn decision_posture_code(posture: DecisionPosture) -> &'static str {
    match posture {
        DecisionPosture::Normal => "normal",
        DecisionPosture::Prepare => "prepare",
        DecisionPosture::Hedge => "hedge",
        DecisionPosture::Defend => "defend",
    }
}

fn parse_decision_posture_code(value: &str) -> DecisionPosture {
    match value {
        "prepare" => DecisionPosture::Prepare,
        "hedge" => DecisionPosture::Hedge,
        "defend" => DecisionPosture::Defend,
        _ => DecisionPosture::Normal,
    }
}

fn time_to_risk_bucket_code(bucket: TimeToRiskBucket) -> &'static str {
    match bucket {
        TimeToRiskBucket::Normal => "normal",
        TimeToRiskBucket::Months => "months",
        TimeToRiskBucket::Weeks => "weeks",
        TimeToRiskBucket::Now => "now",
    }
}

fn parse_time_to_risk_bucket_code(value: &str) -> TimeToRiskBucket {
    match value {
        "months" => TimeToRiskBucket::Months,
        "weeks" => TimeToRiskBucket::Weeks,
        "now" => TimeToRiskBucket::Now,
        _ => TimeToRiskBucket::Normal,
    }
}

pub(crate) fn select_assessment_history(
    points: &[fc_domain::AssessmentHistoryPoint],
    window: HistoryQueryWindow,
) -> Vec<fc_domain::AssessmentHistoryPoint> {
    let mut filtered = points
        .iter()
        .filter(|point| window.from.is_none_or(|from| point.as_of_date >= from))
        .filter(|point| window.to.is_none_or(|to| point.as_of_date <= to))
        .cloned()
        .collect::<Vec<_>>();
    if let Some(limit) = window.limit {
        if filtered.len() > limit {
            filtered = filtered[filtered.len().saturating_sub(limit)..].to_vec();
        }
    }
    filtered
}

pub(crate) fn select_backtest_timeline(
    points: &[BacktestWindowPoint],
    window: HistoryQueryWindow,
) -> Vec<BacktestWindowPoint> {
    let mut filtered = points
        .iter()
        .filter(|point| window.from.is_none_or(|from| point.as_of_date >= from))
        .filter(|point| window.to.is_none_or(|to| point.as_of_date <= to))
        .cloned()
        .collect::<Vec<_>>();
    if let Some(limit) = window.limit {
        if filtered.len() > limit {
            filtered = filtered[filtered.len().saturating_sub(limit)..].to_vec();
        }
    }
    filtered
}

pub(crate) fn load_user_preferences() -> UserRiskPreferences {
    let profile = match env::var("FC_USER_RISK_PROFILE")
        .unwrap_or_else(|_| "neutral".to_string())
        .to_lowercase()
        .as_str()
    {
        "conservative" => UserRiskProfile::Conservative,
        "aggressive" => UserRiskProfile::Aggressive,
        _ => UserRiskProfile::Neutral,
    };
    let cash_floor_pct = env::var("FC_USER_CASH_FLOOR_PCT")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(15.0);
    let max_equity_cap_pct = env::var("FC_USER_MAX_EQUITY_CAP_PCT")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(70.0);
    let max_leverage_pct = env::var("FC_USER_MAX_LEVERAGE_PCT")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(100.0);
    let option_overlay_preference_pct = env::var("FC_USER_OPTION_OVERLAY_PCT")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(5.0);
    let allow_aggressive_reentry = env::var("FC_USER_ALLOW_AGGRESSIVE_REENTRY")
        .ok()
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "True"))
        .unwrap_or(false);

    let note = format!(
        "profile={}, cash_floor={}%, max_equity={}%, max_leverage={}%, option_overlay={}%",
        match profile {
            UserRiskProfile::Conservative => "conservative",
            UserRiskProfile::Neutral => "neutral",
            UserRiskProfile::Aggressive => "aggressive",
        },
        cash_floor_pct,
        max_equity_cap_pct,
        max_leverage_pct,
        option_overlay_preference_pct
    );

    UserRiskPreferences {
        profile,
        cash_floor_pct,
        max_equity_cap_pct,
        max_leverage_pct,
        option_overlay_preference_pct,
        allow_aggressive_reentry,
        note,
    }
}

fn indicators() -> Vec<Indicator> {
    vec![
        indicator(
            "us_market_vix_close",
            "VIX 收盘价",
            RiskDimension::MarketStress,
            "美国市场隐含波动率。",
            "index",
            Frequency::Daily,
            RiskDirection::HigherIsRiskier,
            "fred",
        ),
        indicator(
            "us_credit_high_yield_oas",
            "高收益债 OAS",
            RiskDimension::LeverageCredit,
            "美国高收益债期权调整利差。",
            "percent",
            Frequency::Daily,
            RiskDirection::HigherIsRiskier,
            "fred",
        ),
        indicator(
            "us_rates_yield_curve_10y2y",
            "10Y-2Y 期限利差",
            RiskDimension::MarketStress,
            "美国 10 年期和 2 年期国债收益率利差。",
            "percent",
            Frequency::Daily,
            RiskDirection::LowerIsRiskier,
            "fred",
        ),
        indicator(
            "us_liquidity_national_financial_conditions",
            "NFCI 金融条件指数",
            RiskDimension::LiquidityFunding,
            "Chicago Fed National Financial Conditions Index。",
            "index",
            Frequency::Weekly,
            RiskDirection::HigherIsRiskier,
            "fred",
        ),
        indicator(
            "us_liquidity_effr",
            "有效联邦基金利率",
            RiskDimension::LiquidityFunding,
            "美国有效联邦基金利率。",
            "percent",
            Frequency::Daily,
            RiskDirection::RisingFastIsRiskier,
            "fred",
        ),
        indicator(
            "us_macro_unemployment_rate",
            "失业率",
            RiskDimension::MacroFragility,
            "美国失业率。",
            "percent",
            Frequency::Monthly,
            RiskDirection::HigherIsRiskier,
            "fred",
        ),
        indicator(
            "us_banking_deposits_growth",
            "银行存款增速",
            RiskDimension::BankingSystem,
            "银行存款同比或近似增速。",
            "percent",
            Frequency::Weekly,
            RiskDirection::LowerIsRiskier,
            "fred",
        ),
        indicator(
            "us_real_estate_home_price_yoy",
            "房价同比",
            RiskDimension::RealEstate,
            "全国房价同比变化。",
            "percent",
            Frequency::Monthly,
            RiskDirection::TwoSided,
            "fred",
        ),
        indicator(
            "global_external_current_account_gdp",
            "经常账户/GDP",
            RiskDimension::ExternalSector,
            "经常账户余额占 GDP 比重。",
            "percent",
            Frequency::Annual,
            RiskDirection::LowerIsRiskier,
            "world_bank",
        ),
        indicator(
            "us_external_usdjpy_level",
            "USDJPY 汇率",
            RiskDimension::ExternalSector,
            "美元兑日元水平，用于识别日元套息平仓风险放大器。",
            "jpy_per_usd",
            Frequency::Daily,
            RiskDirection::TwoSided,
            "boj",
        ),
        indicator(
            "jp_rates_call_rate",
            "日本无担保隔夜拆借利率",
            RiskDimension::ExternalSector,
            "日本无担保隔夜拆借利率，作为日元融资成本代理。",
            "percent",
            Frequency::Daily,
            RiskDirection::RisingFastIsRiskier,
            "boj",
        ),
        indicator(
            "global_news_financial_stress_count",
            "金融压力新闻数量",
            RiskDimension::EventsSentiment,
            "金融压力相关新闻数量。",
            "count",
            Frequency::Daily,
            RiskDirection::HigherIsRiskier,
            "gdelt",
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn indicator(
    indicator_id: &str,
    display_name: &str,
    dimension: RiskDimension,
    description: &str,
    unit: &str,
    frequency: Frequency,
    risk_direction: RiskDirection,
    default_source_id: &str,
) -> Indicator {
    Indicator {
        indicator_id: indicator_id.to_string(),
        display_name: display_name.to_string(),
        dimension,
        description: description.to_string(),
        unit: unit.to_string(),
        frequency,
        risk_direction,
        default_source_id: default_source_id.to_string(),
        quality_tier: "core".to_string(),
    }
}

fn observations(as_of_date: NaiveDate) -> Vec<Observation> {
    let mut rows = Vec::new();
    rows.extend(series(
        "us_market_vix_close",
        "fred",
        Frequency::Daily,
        "index",
        as_of_date,
        &[
            18.0, 21.0, 79.0, 32.0, 20.0, 15.0, 17.0, 66.0, 28.0, 20.0, 18.0, 25.0, 24.0,
        ],
        96.0,
        &[],
    ));
    rows.extend(series(
        "us_credit_high_yield_oas",
        "fred",
        Frequency::Daily,
        "percent",
        as_of_date,
        &[
            3.1, 4.2, 10.8, 7.9, 3.8, 3.4, 4.6, 8.7, 4.1, 3.7, 4.5, 5.8, 5.2,
        ],
        95.0,
        &[],
    ));
    rows.extend(series(
        "us_rates_yield_curve_10y2y",
        "fred",
        Frequency::Daily,
        "percent",
        as_of_date,
        &[
            1.2, 0.8, -0.8, -0.2, 0.5, 0.1, -1.05, -0.6, -0.1, 0.0, -0.35, -0.55, -0.45,
        ],
        94.0,
        &[],
    ));
    rows.extend(series(
        "us_liquidity_national_financial_conditions",
        "fred",
        Frequency::Weekly,
        "index",
        as_of_date,
        &[
            -0.4, -0.2, 4.0, 1.2, -0.3, -0.4, 0.1, 1.6, 0.2, -0.1, 0.25, 0.7, 0.55,
        ],
        92.0,
        &[],
    ));
    rows.extend(series(
        "us_liquidity_effr",
        "fred",
        Frequency::Daily,
        "percent",
        as_of_date,
        &[
            0.15, 0.18, 0.12, 0.09, 4.85, 5.10, 5.30, 5.32, 5.31, 5.30, 5.28, 5.20, 5.12,
        ],
        94.0,
        &[],
    ));
    rows.extend(series(
        "us_macro_unemployment_rate",
        "fred",
        Frequency::Monthly,
        "percent",
        as_of_date,
        &[
            4.6, 5.8, 10.0, 7.8, 4.2, 3.7, 3.5, 14.7, 6.2, 4.0, 3.8, 4.3, 4.1,
        ],
        91.0,
        &[],
    ));
    rows.extend(series(
        "us_banking_deposits_growth",
        "fred",
        Frequency::Weekly,
        "percent",
        as_of_date,
        &[
            7.0, 5.5, -3.5, -1.4, 4.0, 5.2, 2.3, -2.1, 1.1, 2.0, 0.2, -1.2, -0.8,
        ],
        86.0,
        &[],
    ));
    rows.extend(series(
        "us_real_estate_home_price_yoy",
        "fred",
        Frequency::Monthly,
        "percent",
        as_of_date,
        &[
            7.0, 12.5, -8.2, -4.1, 3.2, 5.6, 6.8, 13.5, 10.1, 4.8, 3.2, 5.2, 4.5,
        ],
        87.0,
        &[],
    ));
    rows.extend(series(
        "global_external_current_account_gdp",
        "world_bank",
        Frequency::Annual,
        "percent",
        as_of_date,
        &[
            -2.0, -4.2, -6.1, -3.5, -1.5, -1.8, -2.1, -4.8, -3.2, -2.0, -1.7, -3.1, -2.7,
        ],
        82.0,
        &[],
    ));
    rows.extend(series(
        "us_external_usdjpy_level",
        "boj",
        Frequency::Daily,
        "jpy_per_usd",
        as_of_date,
        &[
            106.0, 110.0, 93.0, 101.0, 115.0, 130.0, 151.0, 141.0, 144.0, 149.0, 156.0, 151.0,
            148.0,
        ],
        92.0,
        &[],
    ));
    rows.extend(series_for_entity(
        "jp_rates_call_rate",
        "jp",
        "boj",
        Frequency::Daily,
        "percent",
        as_of_date,
        &[
            -0.08, -0.07, -0.1, -0.09, 0.03, 0.08, 0.12, 0.18, 0.22, 0.29, 0.38, 0.44, 0.48,
        ],
        97.0,
        &[],
    ));
    rows.extend(series(
        "global_news_financial_stress_count",
        "gdelt",
        Frequency::Daily,
        "count",
        as_of_date,
        &[
            40.0, 72.0, 210.0, 128.0, 52.0, 44.0, 61.0, 180.0, 82.0, 70.0, 65.0, 110.0, 96.0,
        ],
        78.0,
        &["prototype_source"],
    ));
    rows
}

#[allow(clippy::too_many_arguments)]
fn series(
    indicator_id: &str,
    source_id: &str,
    frequency: Frequency,
    unit: &str,
    as_of_date: NaiveDate,
    values: &[f64],
    quality_score: f64,
    flags: &[&str],
) -> Vec<Observation> {
    series_for_entity(
        indicator_id,
        "us",
        source_id,
        frequency,
        unit,
        as_of_date,
        values,
        quality_score,
        flags,
    )
}

#[allow(clippy::too_many_arguments)]
fn series_for_entity(
    indicator_id: &str,
    entity_id: &str,
    source_id: &str,
    frequency: Frequency,
    unit: &str,
    as_of_date: NaiveDate,
    values: &[f64],
    quality_score: f64,
    flags: &[&str],
) -> Vec<Observation> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let days_back = (values.len() - index - 1) as i64 * 30;
            let date = as_of_date - Duration::days(days_back);
            Observation {
                indicator_id: indicator_id.to_string(),
                entity_id: entity_id.to_string(),
                as_of_date: date,
                period_start: Some(date),
                period_end: Some(date),
                frequency,
                value: *value,
                unit: unit.to_string(),
                source_id: source_id.to_string(),
                dataset_id: "demo".to_string(),
                revision_time: None,
                publication_time: Some(Utc::now()),
                quality_score,
                quality_flags: flags.iter().map(|flag| (*flag).to_string()).collect(),
            }
        })
        .collect()
}

fn select_recent_alerts_for_date(alerts: &[AlertEvent], as_of_date: NaiveDate) -> Vec<AlertEvent> {
    let floor = as_of_date - Duration::days(EVENT_LOOKBACK_DAYS);
    let mut filtered = alerts
        .iter()
        .filter(|alert| alert.triggered_as_of_date >= floor)
        .filter(|alert| alert.triggered_as_of_date <= as_of_date)
        .cloned()
        .collect::<Vec<_>>();
    filtered.sort_by(|a, b| {
        b.triggered_as_of_date
            .cmp(&a.triggered_as_of_date)
            .then_with(|| b.score.total_cmp(&a.score))
    });
    filtered
}

fn sources_demo() -> Vec<DataSource> {
    vec![
        source(
            "fred",
            "FRED",
            "macro_financial_timeseries",
            SourcePriority::P0,
            SourceStatus::Healthy,
            96.0,
            true,
            "FRED graph CSV is the default no-key source; official API remains optional.",
        ),
        source(
            "treasury",
            "U.S. Treasury",
            "government_timeseries",
            SourcePriority::P0,
            SourceStatus::Healthy,
            96.0,
            true,
            "Official no-key Treasury yield curve data.",
        ),
        source(
            "sec_edgar",
            "SEC EDGAR",
            "filings_events",
            SourcePriority::P0,
            SourceStatus::Prototype,
            72.0,
            false,
            "Official SEC JSON APIs. Runtime connector is available in SQLite mode; this demo source only marks the capability shape.",
        ),
        source(
            "world_bank",
            "World Bank Indicators",
            "global_macro",
            SourcePriority::P0,
            SourceStatus::Healthy,
            90.0,
            true,
            "Official World Bank Indicators API.",
        ),
        source(
            "boj",
            "Bank of Japan",
            "fx_rates_timeseries",
            SourcePriority::P1,
            SourceStatus::Delayed,
            84.0,
            true,
            "Official BOJ FX and time-series endpoints are tracked as the preferred JPY carry enhancement source.",
        ),
        source(
            "gdelt",
            "GDELT",
            "news_events",
            SourcePriority::P1,
            SourceStatus::Prototype,
            66.0,
            false,
            "News-event prototype source. Optional runtime backfill is available, but noise filtering and production validation are still pending.",
        ),
        source(
            "yfinance",
            "yfinance",
            "market_price_prototype",
            SourcePriority::P1,
            SourceStatus::Prototype,
            62.0,
            false,
            "Development-only market data prototype; not a production dependency.",
        ),
    ]
}

fn sources_runtime(observations: &[Observation], as_of_date: NaiveDate) -> Vec<DataSource> {
    let gdelt_has_data = observations
        .iter()
        .any(|observation| observation.source_id == "gdelt");
    vec![
        live_source(
            observations,
            as_of_date,
            "fred",
            "FRED",
            "macro_financial_timeseries",
            SourcePriority::P0,
            7,
            96.0,
            true,
            "FRED graph CSV is the default no-key source; official API remains optional.",
        ),
        live_source(
            observations,
            as_of_date,
            "treasury",
            "U.S. Treasury",
            "government_timeseries",
            SourcePriority::P0,
            7,
            96.0,
            true,
            "Official no-key Treasury yield curve data.",
        ),
        live_source(
            observations,
            as_of_date,
            "sec_edgar",
            "SEC EDGAR",
            "filings_events",
            SourcePriority::P0,
            7,
            88.0,
            true,
            "Official SEC JSON filings metadata aggregated into daily event features. No paid key is required.",
        ),
        live_source(
            observations,
            as_of_date,
            "world_bank",
            "World Bank Indicators",
            "global_macro",
            SourcePriority::P0,
            730,
            90.0,
            true,
            "Official World Bank Indicators API.",
        ),
        live_source(
            observations,
            as_of_date,
            "boj",
            "Bank of Japan",
            "fx_rates_timeseries",
            SourcePriority::P1,
            3,
            84.0,
            true,
            "Official BOJ FX and money-market endpoints are used for the JPY carry monitor.",
        ),
        if gdelt_has_data {
            live_source(
                observations,
                as_of_date,
                "gdelt",
                "GDELT",
                "news_events",
                SourcePriority::P1,
                3,
                66.0,
                false,
                "GDELT 聚合新闻压力序列已支持本地回填和运行时展示，但仍属于 prototype 辅助信号。",
            )
        } else {
            source(
                "gdelt",
                "GDELT",
                "news_events",
                SourcePriority::P1,
                SourceStatus::Prototype,
                66.0,
                false,
                "GDELT 聚合新闻压力序列可选接入；当前本地库尚未回填该源。",
            )
        },
        source(
            "yfinance",
            "yfinance",
            "market_price_prototype",
            SourcePriority::P1,
            SourceStatus::Prototype,
            62.0,
            false,
            "Development-only market data prototype; not a production dependency.",
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn live_source(
    observations: &[Observation],
    as_of_date: NaiveDate,
    source_id: &str,
    display_name: &str,
    source_type: &str,
    priority: SourcePriority,
    stale_days: i64,
    fallback_quality_score: f64,
    production_allowed: bool,
    license_note: &str,
) -> DataSource {
    let latest = observations
        .iter()
        .filter(|observation| observation.source_id == source_id)
        .max_by_key(|observation| observation.as_of_date);
    match latest {
        Some(observation) => {
            let lag_days = (as_of_date - observation.as_of_date).num_days();
            let status = if lag_days > stale_days * 3 {
                SourceStatus::PartialFailure
            } else if lag_days > stale_days {
                SourceStatus::Delayed
            } else {
                SourceStatus::Healthy
            };
            let message = format!(
                "latest observation {} (lag {}d, dataset={})",
                observation.as_of_date, lag_days, observation.dataset_id
            );
            runtime_source(
                source_id,
                display_name,
                source_type,
                priority,
                status,
                if observation.quality_score > 0.0 {
                    observation.quality_score
                } else {
                    fallback_quality_score
                },
                production_allowed,
                license_note,
                observation.publication_time.or(Some(Utc::now())),
                Some(lag_days.saturating_mul(86_400)),
                message,
            )
        }
        None => runtime_source(
            source_id,
            display_name,
            source_type,
            priority,
            SourceStatus::Delayed,
            fallback_quality_score,
            production_allowed,
            license_note,
            None,
            None,
            "connector available, but no local observations are loaded yet".to_string(),
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn source(
    source_id: &str,
    display_name: &str,
    source_type: &str,
    priority: SourcePriority,
    status: SourceStatus,
    quality_score: f64,
    production_allowed: bool,
    license_note: &str,
) -> DataSource {
    DataSource {
        source_id: source_id.to_string(),
        display_name: display_name.to_string(),
        source_type: source_type.to_string(),
        priority,
        access_method: "api".to_string(),
        documentation_url: None,
        production_allowed,
        license_note: license_note.to_string(),
        health: SourceHealth {
            status,
            last_success_at: Some(Utc::now()),
            lag_seconds: Some(if status == SourceStatus::Delayed {
                14_400
            } else {
                0
            }),
            consecutive_failures: 0,
            quality_score,
            message: match status {
                SourceStatus::Healthy => "source healthy".to_string(),
                SourceStatus::Delayed => "source delayed but usable".to_string(),
                SourceStatus::Prototype => "prototype source, not for production".to_string(),
                SourceStatus::PartialFailure => "partial failure".to_string(),
                SourceStatus::Failed => "source failed".to_string(),
                SourceStatus::Disabled => "source disabled".to_string(),
            },
        },
    }
}

#[allow(clippy::too_many_arguments)]
fn runtime_source(
    source_id: &str,
    display_name: &str,
    source_type: &str,
    priority: SourcePriority,
    status: SourceStatus,
    quality_score: f64,
    production_allowed: bool,
    license_note: &str,
    last_success_at: Option<chrono::DateTime<Utc>>,
    lag_seconds: Option<i64>,
    message: String,
) -> DataSource {
    DataSource {
        source_id: source_id.to_string(),
        display_name: display_name.to_string(),
        source_type: source_type.to_string(),
        priority,
        access_method: "api".to_string(),
        documentation_url: None,
        production_allowed,
        license_note: license_note.to_string(),
        health: SourceHealth {
            status,
            last_success_at,
            lag_seconds,
            consecutive_failures: 0,
            quality_score,
            message,
        },
    }
}

fn build_alerts(snapshot: &fc_domain::RiskSnapshot) -> Vec<AlertEvent> {
    let top = snapshot.top_contributors.clone();
    let credit_alert = AlertEvent {
        alert_id: Uuid::new_v4(),
        event_type: AlertType::RiskStress,
        scope: "dimension".to_string(),
        entity_id: "us".to_string(),
        dimension: Some(RiskDimension::LeverageCredit),
        level: RiskLevel::Stress,
        status: AlertStatus::Open,
        triggered_at: Utc::now(),
        triggered_as_of_date: snapshot.as_of_date,
        resolved_at: None,
        score: snapshot
            .dimensions
            .iter()
            .find(|dimension| dimension.dimension == RiskDimension::LeverageCredit)
            .map(|dimension| dimension.score)
            .unwrap_or(snapshot.overall_score),
        previous_score: Some(48.0),
        trigger_reason: "高收益债 OAS 和期限结构信号同时恶化。".to_string(),
        top_contributors: top.iter().take(3).cloned().collect(),
        related_indicators: vec![
            "us_credit_high_yield_oas".to_string(),
            "us_rates_yield_curve_10y2y".to_string(),
        ],
        method_version: snapshot.method_version.clone(),
    };

    let source_alert = AlertEvent {
        alert_id: Uuid::new_v4(),
        event_type: AlertType::SourceHealthIssue,
        scope: "data_source".to_string(),
        entity_id: "gdelt".to_string(),
        dimension: None,
        level: RiskLevel::Watch,
        status: AlertStatus::Monitoring,
        triggered_at: Utc::now(),
        triggered_as_of_date: snapshot.as_of_date,
        resolved_at: None,
        score: 35.0,
        previous_score: Some(20.0),
        trigger_reason: "GDELT 事件源仍处于 prototype 状态，事件维度质量降级。".to_string(),
        top_contributors: Vec::new(),
        related_indicators: vec!["global_news_financial_stress_count".to_string()],
        method_version: snapshot.method_version.clone(),
    };

    vec![credit_alert, source_alert]
}

fn build_backtests(
    snapshot: &fc_domain::RiskSnapshot,
    history: &[fc_domain::AssessmentHistoryPoint],
    use_transitional_bridge: bool,
) -> Vec<BacktestScenarioSummary> {
    let history_start = history.first().map(|point| point.as_of_date);
    let history_end = history.last().map(|point| point.as_of_date);
    scenario_catalog()
        .into_iter()
        .map(|scenario| {
            scenario_summary_from_history(
                snapshot,
                history,
                &scenario,
                use_transitional_bridge,
                snapshot.top_contributors.iter().take(3).cloned().collect(),
            )
            .unwrap_or_else(|| fallback_backtest(snapshot, &scenario, history_start, history_end))
        })
        .collect()
}

#[derive(Debug, Clone)]
struct ScenarioDefinition {
    scenario_id: String,
    name: String,
    region: String,
    pre_warning_start: NaiveDate,
    crisis_start: NaiveDate,
    crisis_end: NaiveDate,
    protected_window: bool,
    fallback_first_l2_date: Option<NaiveDate>,
    fallback_first_l3_date: Option<NaiveDate>,
    fallback_max_level: RiskLevel,
    fallback_max_score: f64,
    fallback_lead_time_days: Option<i64>,
    fallback_false_positive_count: u32,
}

#[derive(Debug, Clone, Copy)]
struct ScenarioFallbackProfile {
    fallback_first_l2_date: Option<NaiveDate>,
    fallback_first_l3_date: Option<NaiveDate>,
    fallback_max_level: RiskLevel,
    fallback_max_score: f64,
    fallback_lead_time_days: Option<i64>,
    fallback_false_positive_count: u32,
}

#[derive(Debug, Clone)]
struct RollingAuditEpisodeBuilder {
    start_date: NaiveDate,
    end_date: NaiveDate,
    signal_count: u32,
    classification: &'static str,
    note: String,
}

fn scenario_catalog() -> Vec<ScenarioDefinition> {
    let catalog = load_crisis_scenario_catalog();
    catalog
        .scenarios
        .into_iter()
        .map(|scenario| {
            let fallback = scenario_fallback_profile(&scenario.scenario_id);
            ScenarioDefinition {
                scenario_id: scenario.scenario_id,
                name: scenario.label,
                region: "US".to_string(),
                pre_warning_start: scenario.pre_warning_start,
                crisis_start: scenario.crisis_start,
                crisis_end: scenario.crisis_end,
                protected_window: scenario.protected_window,
                fallback_first_l2_date: fallback.fallback_first_l2_date,
                fallback_first_l3_date: fallback.fallback_first_l3_date,
                fallback_max_level: fallback.fallback_max_level,
                fallback_max_score: fallback.fallback_max_score,
                fallback_lead_time_days: fallback.fallback_lead_time_days,
                fallback_false_positive_count: fallback.fallback_false_positive_count,
            }
        })
        .collect()
}

fn scenario_fallback_profile(scenario_id: &str) -> ScenarioFallbackProfile {
    match scenario_id {
        "us_black_monday_1987" => ScenarioFallbackProfile {
            fallback_first_l2_date: Some(NaiveDate::from_ymd_opt(1987, 10, 8).expect("valid date")),
            fallback_first_l3_date: Some(
                NaiveDate::from_ymd_opt(1987, 10, 16).expect("valid date"),
            ),
            fallback_max_level: RiskLevel::Crisis,
            fallback_max_score: 95.0,
            fallback_lead_time_days: Some(6),
            fallback_false_positive_count: 0,
        },
        "us_ltcm_1998" => ScenarioFallbackProfile {
            fallback_first_l2_date: Some(NaiveDate::from_ymd_opt(1998, 8, 10).expect("valid date")),
            fallback_first_l3_date: Some(NaiveDate::from_ymd_opt(1998, 8, 27).expect("valid date")),
            fallback_max_level: RiskLevel::Crisis,
            fallback_max_score: 84.0,
            fallback_lead_time_days: Some(7),
            fallback_false_positive_count: 1,
        },
        "us_dotcom_unwind_2000" => ScenarioFallbackProfile {
            fallback_first_l2_date: Some(NaiveDate::from_ymd_opt(2000, 2, 10).expect("valid date")),
            fallback_first_l3_date: None,
            fallback_max_level: RiskLevel::Warning,
            fallback_max_score: 68.0,
            fallback_lead_time_days: Some(29),
            fallback_false_positive_count: 1,
        },
        "us_gfc_2008" => ScenarioFallbackProfile {
            fallback_first_l2_date: Some(NaiveDate::from_ymd_opt(2007, 6, 15).expect("valid date")),
            fallback_first_l3_date: Some(NaiveDate::from_ymd_opt(2007, 8, 9).expect("valid date")),
            fallback_max_level: RiskLevel::Crisis,
            fallback_max_score: 92.0,
            fallback_lead_time_days: Some(47),
            fallback_false_positive_count: 2,
        },
        "us_funding_stress_2011" => ScenarioFallbackProfile {
            fallback_first_l2_date: Some(NaiveDate::from_ymd_opt(2011, 7, 18).expect("valid date")),
            fallback_first_l3_date: None,
            fallback_max_level: RiskLevel::Warning,
            fallback_max_score: 71.0,
            fallback_lead_time_days: Some(11),
            fallback_false_positive_count: 1,
        },
        "us_covid_liquidity_2020" => ScenarioFallbackProfile {
            fallback_first_l2_date: Some(NaiveDate::from_ymd_opt(2020, 2, 25).expect("valid date")),
            fallback_first_l3_date: Some(NaiveDate::from_ymd_opt(2020, 3, 9).expect("valid date")),
            fallback_max_level: RiskLevel::Crisis,
            fallback_max_score: 88.0,
            fallback_lead_time_days: Some(13),
            fallback_false_positive_count: 1,
        },
        "us_rate_shock_2022" => ScenarioFallbackProfile {
            fallback_first_l2_date: Some(NaiveDate::from_ymd_opt(2022, 4, 29).expect("valid date")),
            fallback_first_l3_date: Some(NaiveDate::from_ymd_opt(2022, 6, 13).expect("valid date")),
            fallback_max_level: RiskLevel::Warning,
            fallback_max_score: 74.0,
            fallback_lead_time_days: Some(35),
            fallback_false_positive_count: 1,
        },
        "us_regional_banks_2023" => ScenarioFallbackProfile {
            fallback_first_l2_date: Some(NaiveDate::from_ymd_opt(2023, 2, 15).expect("valid date")),
            fallback_first_l3_date: Some(NaiveDate::from_ymd_opt(2023, 3, 10).expect("valid date")),
            fallback_max_level: RiskLevel::Warning,
            fallback_max_score: 78.0,
            fallback_lead_time_days: Some(21),
            fallback_false_positive_count: 1,
        },
        _ => ScenarioFallbackProfile {
            fallback_first_l2_date: None,
            fallback_first_l3_date: None,
            fallback_max_level: RiskLevel::Watch,
            fallback_max_score: 60.0,
            fallback_lead_time_days: None,
            fallback_false_positive_count: 0,
        },
    }
}

fn scenario_summary_from_history(
    snapshot: &fc_domain::RiskSnapshot,
    history: &[fc_domain::AssessmentHistoryPoint],
    scenario: &ScenarioDefinition,
    use_transitional_bridge: bool,
    top_contributors: Vec<RiskContributor>,
) -> Option<BacktestScenarioSummary> {
    let crisis_points = history
        .iter()
        .filter(|point| {
            point.as_of_date >= scenario.crisis_start && point.as_of_date <= scenario.crisis_end
        })
        .cloned()
        .collect::<Vec<_>>();
    if crisis_points.is_empty() {
        return None;
    }

    let warmup_start = scenario.crisis_start - Duration::days(90);
    let warmup_points = history
        .iter()
        .filter(|point| {
            point.as_of_date >= warmup_start && point.as_of_date < scenario.crisis_start
        })
        .cloned()
        .collect::<Vec<_>>();

    let first_l2_date = first_sustained_signal_date(&warmup_points, is_structural_warning_point);
    let first_l3_date = first_sustained_signal_date(&warmup_points, |point| {
        is_actionable_warning_point(point, use_transitional_bridge)
    });

    let max_point = crisis_points
        .iter()
        .max_by(|left, right| left.overall_score.total_cmp(&right.overall_score))
        .expect("crisis_points is not empty");
    let lead_time_days = lead_time_from_date(scenario.crisis_start, first_l2_date);
    let actionable_lead_time_days = lead_time_from_date(scenario.crisis_start, first_l3_date);
    let false_positive_count =
        count_false_positive_actionable_episodes(&warmup_points, use_transitional_bridge);

    Some(BacktestScenarioSummary {
        scenario_id: scenario.scenario_id.clone(),
        name: scenario.name.clone(),
        region: scenario.region.clone(),
        signal_source: BacktestSignalSource::RealHistory,
        crisis_start: scenario.crisis_start,
        crisis_end: scenario.crisis_end,
        first_l2_date,
        first_l3_date,
        max_level: RiskLevel::from_score(max_point.overall_score),
        max_score: max_point.overall_score,
        lead_time_days,
        actionable_lead_time_days,
        false_positive_count,
        missed: actionable_lead_time_days.is_none(),
        history_start: crisis_points.first().map(|point| point.as_of_date),
        history_end: crisis_points.last().map(|point| point.as_of_date),
        history_point_count: crisis_points.len() as u32,
        note: build_real_history_backtest_note(
            lead_time_days,
            actionable_lead_time_days,
            crisis_points.len(),
        ),
        top_contributors,
        method_version: snapshot.method_version.clone(),
    })
}

fn fallback_backtest(
    snapshot: &fc_domain::RiskSnapshot,
    scenario: &ScenarioDefinition,
    history_start: Option<NaiveDate>,
    history_end: Option<NaiveDate>,
) -> BacktestScenarioSummary {
    let structural_lead_time_days = scenario
        .fallback_first_l2_date
        .and_then(|date| lead_time_from_date(scenario.crisis_start, Some(date)))
        .or(scenario.fallback_lead_time_days);
    let actionable_lead_time_days = scenario
        .fallback_first_l3_date
        .and_then(|date| lead_time_from_date(scenario.crisis_start, Some(date)));
    BacktestScenarioSummary {
        scenario_id: scenario.scenario_id.clone(),
        name: scenario.name.clone(),
        region: scenario.region.clone(),
        signal_source: BacktestSignalSource::FallbackTemplate,
        crisis_start: scenario.crisis_start,
        crisis_end: scenario.crisis_end,
        first_l2_date: scenario.fallback_first_l2_date,
        first_l3_date: scenario.fallback_first_l3_date,
        max_level: scenario.fallback_max_level,
        max_score: scenario.fallback_max_score,
        lead_time_days: structural_lead_time_days,
        actionable_lead_time_days,
        false_positive_count: scenario.fallback_false_positive_count,
        missed: actionable_lead_time_days.is_none(),
        history_start,
        history_end,
        history_point_count: 0,
        note: match (history_start, history_end) {
            (Some(start), Some(end)) => format!(
                "本地历史库当前只覆盖 {start} 到 {end}，尚未覆盖该危机窗口，当前结果来自内置参考模板。"
            ),
            _ => "本地历史库尚未覆盖该危机窗口，当前结果来自内置参考模板。".to_string(),
        },
        top_contributors: snapshot.top_contributors.iter().take(3).cloned().collect(),
        method_version: snapshot.method_version.clone(),
    }
}

fn build_backtest_timeline(
    history: &[fc_domain::AssessmentHistoryPoint],
    use_transitional_bridge: bool,
) -> Vec<BacktestWindowPoint> {
    history
        .iter()
        .map(|point| BacktestWindowPoint {
            as_of_date: point.as_of_date,
            overall_score: point.overall_score,
            p_5d: point.p_5d,
            p_20d: point.p_20d,
            p_60d: point.p_60d,
            posture: point.posture,
            crisis_window_open: is_actionable_warning_point(point, use_transitional_bridge),
        })
        .collect()
}

fn build_rolling_backtest_audit(
    history: &[fc_domain::AssessmentHistoryPoint],
    stress_windows: &[ProtectedStressWindow],
    use_transitional_bridge: bool,
) -> BacktestRollingAudit {
    let catalog_window_start = scenario_catalog()
        .iter()
        .map(|scenario| scenario.crisis_start - Duration::days(90))
        .min();
    let min_supported_date = NaiveDate::from_ymd_opt(
        ROLLING_AUDIT_MIN_DATE.0,
        ROLLING_AUDIT_MIN_DATE.1,
        ROLLING_AUDIT_MIN_DATE.2,
    )
    .expect("valid rolling audit min date");
    let audit_window_start = Some(
        catalog_window_start
            .map(|date| date.max(min_supported_date))
            .unwrap_or(min_supported_date),
    );
    let filtered_history = history
        .iter()
        .filter(|point| audit_window_start.is_none_or(|start| point.as_of_date >= start))
        .cloned()
        .collect::<Vec<_>>();

    if filtered_history.is_empty() {
        return BacktestRollingAudit {
            history_point_count: 0,
            actionable_signal_count: 0,
            pre_crisis_signal_count: 0,
            in_crisis_signal_count: 0,
            stress_window_signal_count: 0,
            false_positive_signal_count: 0,
            false_positive_episode_count: 0,
            longest_false_positive_episode_days: 0,
            actionable_precision: 0.0,
            classified_episodes: Vec::new(),
            summary: "当前没有历史评估序列，无法生成全历史滚动审计。".to_string(),
        };
    }

    let scenarios = scenario_catalog();
    let mut actionable_signal_count = 0_u32;
    let mut pre_crisis_signal_count = 0_u32;
    let mut in_crisis_signal_count = 0_u32;
    let mut stress_window_signal_count = 0_u32;
    let mut false_positive_signal_count = 0_u32;
    let mut false_positive_episode_count = 0_u32;
    let mut longest_false_positive_episode_days = 0_u32;
    let mut classified_episodes = Vec::new();
    let mut current_episode: Option<RollingAuditEpisodeBuilder> = None;

    for point in &filtered_history {
        let is_actionable = is_actionable_warning_point(point, use_transitional_bridge);
        let in_crisis = scenarios.iter().any(|scenario| {
            point.as_of_date >= scenario.crisis_start && point.as_of_date <= scenario.crisis_end
        });
        let next_crisis_lead_days = scenarios
            .iter()
            .filter_map(|scenario| {
                (scenario.crisis_start >= point.as_of_date)
                    .then_some((scenario.crisis_start - point.as_of_date).num_days())
            })
            .min();
        let actionable_horizon_days = actionable_audit_horizon_days(point);
        let within_actionable_horizon = next_crisis_lead_days
            .map(|days| days <= actionable_horizon_days)
            .unwrap_or(false);

        if is_actionable {
            actionable_signal_count += 1;
            if in_crisis {
                in_crisis_signal_count += 1;
            } else if within_actionable_horizon {
                pre_crisis_signal_count += 1;
            } else if let Some(note) =
                protected_stress_window_note(point.as_of_date, stress_windows, &scenarios)
            {
                stress_window_signal_count += 1;
                advance_classified_episode(
                    &mut current_episode,
                    Some(("stress_window", note)),
                    point.as_of_date,
                    &mut classified_episodes,
                    &mut false_positive_episode_count,
                    &mut longest_false_positive_episode_days,
                );
                continue;
            } else {
                false_positive_signal_count += 1;
                advance_classified_episode(
                    &mut current_episode,
                    Some((
                        "false_positive",
                        format!(
                            "未落入姿态对应的危机前 {actionable_horizon_days} 日窗口，也不在受保护压力窗口内。"
                        ),
                    )),
                    point.as_of_date,
                    &mut classified_episodes,
                    &mut false_positive_episode_count,
                    &mut longest_false_positive_episode_days,
                );
                continue;
            }
        }

        advance_classified_episode(
            &mut current_episode,
            None,
            point.as_of_date,
            &mut classified_episodes,
            &mut false_positive_episode_count,
            &mut longest_false_positive_episode_days,
        );
    }

    close_classified_episode(
        &mut current_episode,
        &mut classified_episodes,
        &mut false_positive_episode_count,
        &mut longest_false_positive_episode_days,
    );

    let actionable_precision_denominator =
        pre_crisis_signal_count + stress_window_signal_count + false_positive_signal_count;
    let actionable_precision = if actionable_precision_denominator == 0 {
        0.0
    } else {
        ((pre_crisis_signal_count + stress_window_signal_count) as f64
            / actionable_precision_denominator as f64)
            .clamp(0.0, 1.0)
    };
    let history_start = filtered_history.first().map(|point| point.as_of_date);
    let history_end = filtered_history.last().map(|point| point.as_of_date);
    classified_episodes.sort_by(|left, right| {
        right
            .duration_days
            .cmp(&left.duration_days)
            .then_with(|| right.signal_count.cmp(&left.signal_count))
            .then_with(|| right.start_date.cmp(&left.start_date))
    });
    classified_episodes.truncate(ROLLING_AUDIT_EPISODE_LIMIT);
    let summary = format!(
        "全历史滚动审计覆盖 {} 到 {}；动作级信号共 {} 个评估点，其中危机前 {} 个、危机中 {} 个、受保护压力窗口 {} 个、纯误报 {} 个，形成 {} 段纯误报区间，动作信号精度约为 {:.0}%。",
        history_start
            .map(|date| date.to_string())
            .unwrap_or_else(|| "未知起点".to_string()),
        history_end
            .map(|date| date.to_string())
            .unwrap_or_else(|| "未知终点".to_string()),
        actionable_signal_count,
        pre_crisis_signal_count,
        in_crisis_signal_count,
        stress_window_signal_count,
        false_positive_signal_count,
        false_positive_episode_count,
        actionable_precision * 100.0
    );

    BacktestRollingAudit {
        history_point_count: filtered_history.len() as u32,
        actionable_signal_count,
        pre_crisis_signal_count,
        in_crisis_signal_count,
        stress_window_signal_count,
        false_positive_signal_count,
        false_positive_episode_count,
        longest_false_positive_episode_days,
        actionable_precision: round3(actionable_precision),
        classified_episodes,
        summary,
    }
}

fn advance_classified_episode(
    current_episode: &mut Option<RollingAuditEpisodeBuilder>,
    next_episode: Option<(&'static str, String)>,
    as_of_date: NaiveDate,
    classified_episodes: &mut Vec<BacktestRollingAuditEpisode>,
    false_positive_episode_count: &mut u32,
    longest_false_positive_episode_days: &mut u32,
) {
    match next_episode {
        Some((classification, note)) => {
            let continue_existing = current_episode.as_ref().is_some_and(|episode| {
                episode.classification == classification && episode.note == note
            });
            if continue_existing {
                if let Some(episode) = current_episode.as_mut() {
                    episode.end_date = as_of_date;
                    episode.signal_count += 1;
                }
            } else {
                close_classified_episode(
                    current_episode,
                    classified_episodes,
                    false_positive_episode_count,
                    longest_false_positive_episode_days,
                );
                *current_episode = Some(RollingAuditEpisodeBuilder {
                    start_date: as_of_date,
                    end_date: as_of_date,
                    signal_count: 1,
                    classification,
                    note,
                });
            }
        }
        None => close_classified_episode(
            current_episode,
            classified_episodes,
            false_positive_episode_count,
            longest_false_positive_episode_days,
        ),
    }
}

fn close_classified_episode(
    current_episode: &mut Option<RollingAuditEpisodeBuilder>,
    classified_episodes: &mut Vec<BacktestRollingAuditEpisode>,
    false_positive_episode_count: &mut u32,
    longest_false_positive_episode_days: &mut u32,
) {
    let Some(episode) = current_episode.take() else {
        return;
    };

    let duration_days = (episode.end_date - episode.start_date).num_days().max(0) as u32 + 1;
    if episode.classification == "false_positive" {
        *false_positive_episode_count += 1;
        *longest_false_positive_episode_days =
            (*longest_false_positive_episode_days).max(duration_days);
    }
    classified_episodes.push(BacktestRollingAuditEpisode {
        start_date: episode.start_date,
        end_date: episode.end_date,
        duration_days,
        signal_count: episode.signal_count,
        classification: episode.classification.to_string(),
        note: episode.note,
    });
}

fn first_sustained_signal_date<F>(
    points: &[fc_domain::AssessmentHistoryPoint],
    predicate: F,
) -> Option<NaiveDate>
where
    F: Fn(&fc_domain::AssessmentHistoryPoint) -> bool,
{
    points.iter().enumerate().find_map(|(index, point)| {
        if !predicate(point) {
            return None;
        }
        let end = (index + BACKTEST_SIGNAL_WINDOW).min(points.len());
        let window = &points[index..end];
        let hit_count = window
            .iter()
            .filter(|candidate| predicate(candidate))
            .count();
        let required_hits = BACKTEST_SIGNAL_MIN_HITS.min(window.len());
        (hit_count >= required_hits).then_some(point.as_of_date)
    })
}

fn is_structural_warning_point(point: &fc_domain::AssessmentHistoryPoint) -> bool {
    ((point.p_60d >= 0.35) || !matches!(point.time_to_risk_bucket, TimeToRiskBucket::Normal))
        && point.overall_score >= 54.0
        || (point.overall_score >= 54.0
            && point.p_20d >= 0.12
            && point.external_shock_score >= 42.0)
}

fn has_strong_prepare_trigger_code(point: &fc_domain::AssessmentHistoryPoint) -> bool {
    point.posture_trigger_codes.iter().any(|code| {
        matches!(
            code.as_str(),
            "prepare_p60d_structural"
                | "prepare_structural_downgrade"
                | "prepare_carry_structural"
                | "prepare_external_structural"
        )
    })
}

fn is_actionable_warning_point(
    point: &fc_domain::AssessmentHistoryPoint,
    use_transitional_bridge: bool,
) -> bool {
    let strict_short_horizon_signal =
        matches!(
            point.posture,
            DecisionPosture::Hedge | DecisionPosture::Defend
        ) || (matches!(point.time_to_risk_bucket, TimeToRiskBucket::Now)
            && point.overall_score >= 60.0
            && point.p_5d >= 0.18)
            || (matches!(point.time_to_risk_bucket, TimeToRiskBucket::Weeks)
                && point.overall_score >= 58.0
                && point.p_20d >= 0.25
                && point.external_shock_score >= 44.0);

    let high_probability_prepare_signal = matches!(point.posture, DecisionPosture::Prepare)
        && point.p_20d >= 0.18
        && point.p_60d >= 0.45
        && ((point.overall_score >= 60.0 && point.external_shock_score >= 46.0)
            || (point.overall_score >= 53.0
                && !matches!(point.time_to_risk_bucket, TimeToRiskBucket::Normal)
                && has_strong_prepare_trigger_code(point)));
    let high_probability_months_signal =
        matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
            && point.overall_score >= 62.0
            && point.p_20d >= 0.18
            && point.p_60d >= 0.45
            && point.external_shock_score >= 48.0;

    // Persisted historical snapshots still carry a transitional posture/bucket view:
    // probabilities are often floor-bound, while overall/external stress capture the
    // elevated state. Until the raw point-in-time feature store replaces that archive,
    // rolling audit needs a bridge rule for strong prepare/months phases.
    let prepare_bridge_signal = use_transitional_bridge
        && matches!(point.posture, DecisionPosture::Prepare)
        && point.overall_score >= 58.0
        && point.external_shock_score >= 46.0;
    let months_bridge_signal = use_transitional_bridge
        && matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
        && point.overall_score >= 58.0
        && point.external_shock_score >= 42.0;

    strict_short_horizon_signal
        || high_probability_prepare_signal
        || high_probability_months_signal
        || prepare_bridge_signal
        || months_bridge_signal
}

fn use_transitional_actionable_bridge(serving_model: Option<&ServingModelContext>) -> bool {
    !is_formal_main_release(serving_model)
}

fn actionable_audit_horizon_days(point: &fc_domain::AssessmentHistoryPoint) -> i64 {
    match point.posture {
        DecisionPosture::Defend => ACTIONABLE_AUDIT_HORIZON_DEFEND_DAYS,
        DecisionPosture::Hedge => ACTIONABLE_AUDIT_HORIZON_HEDGE_DAYS,
        DecisionPosture::Prepare => ACTIONABLE_AUDIT_HORIZON_PREPARE_DAYS,
        DecisionPosture::Normal => match point.time_to_risk_bucket {
            TimeToRiskBucket::Now => ACTIONABLE_AUDIT_HORIZON_DEFEND_DAYS,
            TimeToRiskBucket::Weeks => ACTIONABLE_AUDIT_HORIZON_HEDGE_DAYS,
            TimeToRiskBucket::Months => ACTIONABLE_AUDIT_HORIZON_PREPARE_DAYS,
            TimeToRiskBucket::Normal => ACTIONABLE_AUDIT_HORIZON_HEDGE_DAYS,
        },
    }
}

fn protected_stress_window_note(
    as_of_date: NaiveDate,
    explicit_windows: &[ProtectedStressWindow],
    scenarios: &[ScenarioDefinition],
) -> Option<String> {
    if let Some(window) = explicit_windows
        .iter()
        .find(|window| as_of_date >= window.start_date && as_of_date <= window.end_date)
    {
        return Some(format!("{}：{}", window.label, window.note));
    }

    scenarios
        .iter()
        .find(|scenario| {
            scenario.protected_window
                && as_of_date >= scenario.pre_warning_start
                && as_of_date <= scenario.crisis_end
        })
        .map(|scenario| {
            format!(
                "{}：场景目录将该阶段标记为受保护压力窗口，用于 posture 审计而不是主正例。",
                scenario.name
            )
        })
}

fn lead_time_from_date(crisis_start: NaiveDate, signal_date: Option<NaiveDate>) -> Option<i64> {
    signal_date
        .map(|date| (crisis_start - date).num_days())
        .filter(|days| *days >= 0)
}

fn count_false_positive_actionable_episodes(
    points: &[fc_domain::AssessmentHistoryPoint],
    use_transitional_bridge: bool,
) -> u32 {
    let actionable_flags = points
        .iter()
        .map(|point| is_actionable_warning_point(point, use_transitional_bridge))
        .collect::<Vec<_>>();
    let mut episode_count = 0_u32;
    let mut index = 0_usize;

    while index < actionable_flags.len() {
        if !actionable_flags[index] {
            index += 1;
            continue;
        }

        let start = index;
        while index < actionable_flags.len() && actionable_flags[index] {
            index += 1;
        }

        if start + 1 < actionable_flags.len() {
            episode_count += 1;
        }
    }

    episode_count.saturating_sub(1)
}

fn build_real_history_backtest_note(
    structural_lead_time_days: Option<i64>,
    actionable_lead_time_days: Option<i64>,
    history_point_count: usize,
) -> String {
    match (structural_lead_time_days, actionable_lead_time_days) {
        (Some(structural), Some(actionable)) => format!(
            "本地真实历史共 {history_point_count} 个评估点；结构性抬升约提前 {structural} 天出现，可执行预警约提前 {actionable} 天形成。"
        ),
        (Some(structural), None) => format!(
            "本地真实历史共 {history_point_count} 个评估点；结构性抬升约提前 {structural} 天出现，但危机开始前未形成足够强的可执行预警。"
        ),
        (None, Some(actionable)) => format!(
            "本地真实历史共 {history_point_count} 个评估点；危机前未见稳定的结构抬升，但约提前 {actionable} 天进入可执行预警。"
        ),
        (None, None) => format!(
            "本地真实历史共 {history_point_count} 个评估点；危机开始前未形成稳定的结构抬升或可执行预警。"
        ),
    }
}

fn round3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, TimeZone, Utc};
    use fc_domain::{
        load_protected_stress_window_catalog, DecisionPosture, ModelReleaseManifest,
        ModelReleaseRecord, PredictionSnapshotRecord, ProbabilityBundle, TimeToRiskBucket,
    };

    use super::{
        build_rolling_backtest_audit, expected_prediction_snapshot_method_version,
        historical_output_from_prediction_snapshots, is_actionable_warning_point,
        should_refresh_full_release_history, use_transitional_actionable_bridge,
        ServingModelContext,
    };

    fn history_point(
        as_of_date: NaiveDate,
        overall_score: f64,
        posture: DecisionPosture,
        time_to_risk_bucket: TimeToRiskBucket,
        external_shock_score: f64,
    ) -> fc_domain::AssessmentHistoryPoint {
        fc_domain::AssessmentHistoryPoint {
            as_of_date,
            overall_score,
            p_5d: 0.026,
            p_20d: 0.026,
            p_60d: 0.056,
            raw_p_5d: Some(0.012),
            raw_p_20d: Some(0.028),
            raw_p_60d: Some(0.081),
            posture,
            time_to_risk_bucket,
            external_shock_score,
            posture_trigger_codes: Vec::new(),
            posture_blocker_codes: Vec::new(),
        }
    }

    fn snapshot(
        as_of_date: NaiveDate,
        release_id: Option<&str>,
        p_20d: f64,
        posture: &str,
        recorded_at_hour: u32,
    ) -> PredictionSnapshotRecord {
        PredictionSnapshotRecord {
            as_of_date,
            entity_id: "us".to_string(),
            market_scope: "financial_system".to_string(),
            release_id: release_id.map(str::to_string),
            probability_mode: "heuristic_mvp".to_string(),
            release_status: "degraded".to_string(),
            point_in_time_mode: "best_effort".to_string(),
            overall_score: 42.0,
            external_shock_score: 25.0,
            raw_p_5d: 0.01,
            raw_p_20d: p_20d,
            raw_p_60d: 0.08,
            calibrated_p_5d: 0.01,
            calibrated_p_20d: p_20d,
            calibrated_p_60d: 0.08,
            posture: posture.to_string(),
            time_to_risk_bucket: "weeks".to_string(),
            feature_set_version: "feature_v2".to_string(),
            label_version: "label_v1".to_string(),
            coverage_score: 0.95,
            freshness_status: "fresh".to_string(),
            method_version: "score_v1".to_string(),
            posture_trigger_codes: vec!["prepare_p60d_structural".to_string()],
            posture_blocker_codes: Vec::new(),
            recorded_at: Utc
                .with_ymd_and_hms(2026, 5, 31, recorded_at_hour, 0, 0)
                .single()
                .unwrap(),
        }
    }

    fn formal_serving_model_context() -> ServingModelContext {
        ServingModelContext {
            release: ModelReleaseRecord {
                manifest: ModelReleaseManifest {
                    release_id: "formal-release".to_string(),
                    market_scope: "financial_system".to_string(),
                    status: "active".to_string(),
                    probability_mode: "formal_bundle_v1".to_string(),
                    serving_status: "healthy".to_string(),
                    bundle_uri: "bundle.json".to_string(),
                    feature_set_version: super::FORMAL_MAIN_FEATURE_SET_VERSION.to_string(),
                    label_version: super::FORMAL_MAIN_LABEL_VERSION.to_string(),
                    prob_model_version: "prob_bundle_test".to_string(),
                    calibration_version: "platt_test".to_string(),
                    posture_policy_version: "posture_test".to_string(),
                    action_playbook_version: "action_test".to_string(),
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
            },
            probability_bundle: Some(ProbabilityBundle {
                bundle_id: "bundle".to_string(),
                market_scope: "financial_system".to_string(),
                probability_mode: "formal_bundle_v1".to_string(),
                model_family: "linear_v1".to_string(),
                feature_transform: "identity_v1".to_string(),
                created_at: Utc::now(),
                feature_names: Vec::new(),
                monotonic_min_gap_5d_to_20d: 0.0,
                monotonic_min_gap_20d_to_60d: 0.0,
                note: String::new(),
                horizons: Vec::new(),
                evaluation: None,
                actionability: None,
            }),
            runtime_probability_mode: "formal_bundle_v1".to_string(),
            runtime_release_status: "healthy".to_string(),
        }
    }

    #[test]
    fn prediction_history_filters_by_release_and_keeps_latest_daily_snapshot() {
        let as_of_date = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();
        let output = historical_output_from_prediction_snapshots(
            vec![
                snapshot(as_of_date, Some("release-a"), 0.12, "normal", 1),
                snapshot(as_of_date, Some("release-a"), 0.27, "hedge", 3),
                snapshot(as_of_date, Some("release-b"), 0.88, "defend", 4),
            ],
            Some("release-a"),
        );

        assert_eq!(output.history_points.len(), 1);
        assert_eq!(output.prediction_snapshots.len(), 1);
        assert_eq!(output.history_points[0].p_20d, 0.27);
        assert_eq!(
            output.history_points[0].posture,
            fc_domain::DecisionPosture::Hedge
        );
        assert_eq!(
            output.history_points[0].posture_trigger_codes,
            vec!["prepare_p60d_structural".to_string()]
        );
    }

    #[test]
    fn actionable_warning_point_accepts_prepare_bridge_for_persisted_snapshots() {
        let point = history_point(
            NaiveDate::from_ymd_opt(2008, 7, 25).unwrap(),
            58.0,
            DecisionPosture::Prepare,
            TimeToRiskBucket::Normal,
            46.0,
        );

        assert!(is_actionable_warning_point(&point, true));
    }

    #[test]
    fn actionable_warning_point_rejects_weak_prepare_bridge() {
        let point = history_point(
            NaiveDate::from_ymd_opt(2008, 7, 25).unwrap(),
            57.9,
            DecisionPosture::Prepare,
            TimeToRiskBucket::Normal,
            45.9,
        );

        assert!(!is_actionable_warning_point(&point, true));
    }

    #[test]
    fn actionable_warning_point_disables_prepare_bridge_for_formal_main() {
        let point = history_point(
            NaiveDate::from_ymd_opt(2008, 7, 25).unwrap(),
            58.0,
            DecisionPosture::Prepare,
            TimeToRiskBucket::Normal,
            46.0,
        );

        assert!(!is_actionable_warning_point(&point, false));
    }

    #[test]
    fn actionable_warning_point_accepts_strong_prepare_clause_for_formal_main() {
        let point = fc_domain::AssessmentHistoryPoint {
            as_of_date: NaiveDate::from_ymd_opt(2007, 3, 1).unwrap(),
            overall_score: 53.4,
            p_5d: 0.03,
            p_20d: 0.70,
            p_60d: 0.73,
            raw_p_5d: Some(0.02),
            raw_p_20d: Some(0.68),
            raw_p_60d: Some(0.70),
            posture: DecisionPosture::Prepare,
            time_to_risk_bucket: TimeToRiskBucket::Months,
            external_shock_score: 38.5,
            posture_trigger_codes: vec!["prepare_p60d_structural".to_string()],
            posture_blocker_codes: Vec::new(),
        };

        assert!(is_actionable_warning_point(&point, false));
    }

    #[test]
    fn actionable_warning_point_rejects_weak_prepare_clause_for_formal_main() {
        let point = fc_domain::AssessmentHistoryPoint {
            as_of_date: NaiveDate::from_ymd_opt(2007, 3, 1).unwrap(),
            overall_score: 52.9,
            p_5d: 0.03,
            p_20d: 0.70,
            p_60d: 0.73,
            raw_p_5d: Some(0.02),
            raw_p_20d: Some(0.68),
            raw_p_60d: Some(0.70),
            posture: DecisionPosture::Prepare,
            time_to_risk_bucket: TimeToRiskBucket::Months,
            external_shock_score: 38.5,
            posture_trigger_codes: vec!["prepare_p60d_structural".to_string()],
            posture_blocker_codes: Vec::new(),
        };

        assert!(!is_actionable_warning_point(&point, false));
    }

    #[test]
    fn rolling_audit_counts_catalog_protected_windows_as_stress() {
        let stress_windows = load_protected_stress_window_catalog();
        let history = vec![history_point(
            NaiveDate::from_ymd_opt(2015, 9, 1).unwrap(),
            60.0,
            DecisionPosture::Prepare,
            TimeToRiskBucket::Months,
            46.0,
        )];

        let audit = build_rolling_backtest_audit(&history, &stress_windows.windows, true);

        assert_eq!(audit.actionable_signal_count, 1);
        assert_eq!(audit.stress_window_signal_count, 1);
        assert_eq!(audit.pre_crisis_signal_count, 0);
        assert_eq!(audit.false_positive_signal_count, 0);
        assert_eq!(audit.classified_episodes.len(), 1);
        assert_eq!(audit.classified_episodes[0].classification, "stress_window");
    }

    #[test]
    fn rolling_audit_counts_prepare_signal_within_sixty_days_as_pre_crisis() {
        let history = vec![fc_domain::AssessmentHistoryPoint {
            as_of_date: NaiveDate::from_ymd_opt(2000, 1, 31).unwrap(),
            overall_score: 63.0,
            p_5d: 0.03,
            p_20d: 0.19,
            p_60d: 0.48,
            raw_p_5d: Some(0.02),
            raw_p_20d: Some(0.18),
            raw_p_60d: Some(0.45),
            posture: DecisionPosture::Prepare,
            time_to_risk_bucket: TimeToRiskBucket::Months,
            external_shock_score: 49.0,
            posture_trigger_codes: vec!["prepare_p60d_structural".to_string()],
            posture_blocker_codes: Vec::new(),
        }];

        let audit = build_rolling_backtest_audit(&history, &[], false);

        assert_eq!(audit.actionable_signal_count, 1);
        assert_eq!(audit.pre_crisis_signal_count, 1);
        assert_eq!(audit.false_positive_signal_count, 0);
    }

    #[test]
    fn bundle_backed_history_refreshes_when_cached_method_version_is_stale() {
        let serving_model = formal_serving_model_context();
        let mut persisted = vec![snapshot(
            NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
            Some("formal-release"),
            0.27,
            "hedge",
            3,
        )];
        persisted[0].method_version = "legacy-cache".to_string();

        assert!(should_refresh_full_release_history(
            Some(&serving_model),
            &persisted,
            false,
        ));
    }

    #[test]
    fn bundle_backed_history_keeps_cache_when_method_version_matches() {
        let serving_model = formal_serving_model_context();
        let expected_method_version =
            expected_prediction_snapshot_method_version(Some(&serving_model));
        let mut persisted = vec![snapshot(
            NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
            Some("formal-release"),
            0.27,
            "hedge",
            3,
        )];
        persisted[0].method_version = expected_method_version;

        assert!(!should_refresh_full_release_history(
            Some(&serving_model),
            &persisted,
            false,
        ));
    }

    #[test]
    fn formal_main_disables_transitional_actionable_bridge() {
        let serving_model = formal_serving_model_context();

        assert!(!use_transitional_actionable_bridge(Some(&serving_model)));
        assert!(use_transitional_actionable_bridge(None));
    }

    #[test]
    fn formal_main_method_version_carries_runtime_policy_cache_key() {
        let serving_model = formal_serving_model_context();
        let method_version = expected_prediction_snapshot_method_version(Some(&serving_model));

        assert!(method_version.contains("runtime_policy="));
        assert!(method_version.contains("class=formal_main"));
    }
}

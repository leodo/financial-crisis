use std::collections::{BTreeMap, BTreeSet};

use chrono::{NaiveDate, Utc};
use fc_domain::{
    AssessmentHistoryPoint, AssessmentSnapshot, DecisionPosture, FreshnessStatus,
    HistoricalAssessmentPointRecord, HistoricalReplayRunRecord, Observation, PostureGuidance,
    PredictionSnapshotRecord, TimeToRiskBucket,
};
use fc_storage::SqliteStore;
use uuid::Uuid;

use crate::{
    assessment::{
        history_runtime_policy_version, ProbabilityComputationTrace, ServingModelContext,
    },
    demo::{FORMAL_MAIN_FEATURE_SET_VERSION, FORMAL_MAIN_LABEL_VERSION},
};

const PREDICTION_SNAPSHOT_CACHE_VERSION: &str = "history_cache_v3_20260601";

#[derive(Debug)]
pub(crate) struct HistoricalAssessmentOutput {
    pub(crate) history_points: Vec<AssessmentHistoryPoint>,
    pub(crate) prediction_snapshots: Vec<PredictionSnapshotRecord>,
    pub(crate) replay_point_drafts: Vec<HistoricalReplayPointDraft>,
}

#[derive(Debug, Clone)]
pub(crate) struct HistoricalReplayPointDraft {
    pub(crate) entity_id: String,
    pub(crate) market_scope: String,
    pub(crate) release_id: Option<String>,
    pub(crate) as_of_date: NaiveDate,
    pub(crate) feature_snapshot_id: Option<String>,
    pub(crate) feature_set_version: String,
    pub(crate) label_version: String,
    pub(crate) point_in_time_mode: String,
    pub(crate) runtime_policy_version: String,
    pub(crate) action_playbook_version: String,
    pub(crate) overall_score: f64,
    pub(crate) structural_score: f64,
    pub(crate) trigger_score: f64,
    pub(crate) external_shock_score: f64,
    pub(crate) raw_p_5d: f64,
    pub(crate) raw_p_20d: f64,
    pub(crate) raw_p_60d: f64,
    pub(crate) calibrated_p_5d: f64,
    pub(crate) calibrated_p_20d: f64,
    pub(crate) calibrated_p_60d: f64,
    pub(crate) posture: String,
    pub(crate) time_to_risk_bucket: String,
    pub(crate) actionability_prepare: f64,
    pub(crate) actionability_hedge: f64,
    pub(crate) actionability_defend: f64,
    pub(crate) posture_trigger_codes: Vec<String>,
    pub(crate) posture_blocker_codes: Vec<String>,
    pub(crate) coverage_score: f64,
    pub(crate) freshness_status: String,
    pub(crate) generated_at: chrono::DateTime<Utc>,
}

pub(crate) async fn persist_historical_replay_output(
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
    let protected_stress_window_catalog = fc_domain::load_protected_stress_window_catalog();
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

pub(crate) async fn load_cached_historical_replay_output(
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

pub(crate) fn historical_output_from_prediction_snapshots(
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

pub(crate) fn merge_historical_outputs(
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

pub(crate) fn prediction_snapshot_from_assessment(
    assessment: &AssessmentSnapshot,
    posture_guidance: &PostureGuidance,
    probability_trace: &ProbabilityComputationTrace,
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

pub(crate) fn historical_replay_point_draft_from_assessment(
    assessment: &AssessmentSnapshot,
    posture_guidance: &PostureGuidance,
    probability_trace: &ProbabilityComputationTrace,
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

pub(crate) fn expected_prediction_snapshot_method_version(
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

pub(crate) fn should_refresh_full_release_history(
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

pub(crate) fn is_formal_main_release(serving_model: Option<&ServingModelContext>) -> bool {
    serving_model.is_some_and(|context| {
        context.release.manifest.feature_set_version == FORMAL_MAIN_FEATURE_SET_VERSION
            && context.release.manifest.label_version == FORMAL_MAIN_LABEL_VERSION
            && context.probability_bundle.is_some()
    })
}

pub(crate) fn assessment_history_point_from_assessment(
    assessment: &AssessmentSnapshot,
    posture_guidance: &PostureGuidance,
    probability_trace: &ProbabilityComputationTrace,
) -> AssessmentHistoryPoint {
    AssessmentHistoryPoint {
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
) -> AssessmentHistoryPoint {
    AssessmentHistoryPoint {
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
) -> AssessmentHistoryPoint {
    AssessmentHistoryPoint {
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

use std::collections::BTreeMap;

use fc_domain::{
    AssessmentHistoryPoint, AssessmentSnapshot, DecisionPosture, FreshnessStatus,
    HistoricalAssessmentPointRecord, PostureGuidance, PredictionSnapshotRecord,
    ProbabilityDiagnostics, ProbabilityHorizonOverlayDiagnostics, TimeToRiskBucket,
};

use crate::assessment::{
    history_runtime_policy_version, ProbabilityComputationTrace, ServingModelContext,
};

use super::{
    expected_prediction_snapshot_method_version, pit_feature_history_source,
    HistoricalAssessmentOutput, HistoricalReplayPointDraft, HISTORY_SOURCE_RAW_OBSERVATION_REBUILD,
    HISTORY_SOURCE_RAW_OBSERVATION_REPLAY, HISTORY_SOURCE_TRANSITIONAL_SNAPSHOT_BRIDGE,
};

pub(crate) fn historical_output_from_replay_points(
    points: Vec<HistoricalAssessmentPointRecord>,
) -> HistoricalAssessmentOutput {
    let mut latest_by_date = BTreeMap::<chrono::NaiveDate, HistoricalAssessmentPointRecord>::new();
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

    let mut latest_by_date = BTreeMap::<chrono::NaiveDate, PredictionSnapshotRecord>::new();
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
        probability_diagnostics: historical_probability_diagnostics(
            &probability_trace.probability_diagnostics,
        ),
        posture_trigger_codes: posture_guidance.trigger_codes.clone(),
        posture_blocker_codes: posture_guidance.blocker_codes.clone(),
        coverage_score: assessment.data_trust.coverage_score,
        freshness_status: worst_freshness_status(&assessment.key_indicators).to_string(),
        generated_at: assessment.runtime.generated_at,
    }
}

fn historical_probability_diagnostics(
    diagnostics: &ProbabilityDiagnostics,
) -> ProbabilityDiagnostics {
    ProbabilityDiagnostics {
        horizon_overlays: diagnostics
            .horizon_overlays
            .iter()
            .map(|horizon| ProbabilityHorizonOverlayDiagnostics {
                horizon_days: horizon.horizon_days,
                raw_probability: horizon.raw_probability,
                calibrated_probability: horizon.calibrated_probability,
                final_probability: horizon.final_probability,
                runtime_final_probability: horizon.runtime_final_probability,
                monotonic_lift: horizon.monotonic_lift,
                configured_overlay_count: horizon.configured_overlay_count,
                contributions: horizon.contributions.clone(),
                overlay_audits: Vec::new(),
            })
            .collect(),
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
        replay_run_id: None,
        feature_snapshot_id: None,
        history_source: Some(HISTORY_SOURCE_RAW_OBSERVATION_REBUILD.to_string()),
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
        replay_run_id: None,
        feature_snapshot_id: None,
        history_source: Some(HISTORY_SOURCE_TRANSITIONAL_SNAPSHOT_BRIDGE.to_string()),
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
        replay_run_id: Some(point.replay_run_id.clone()),
        feature_snapshot_id: point.feature_snapshot_id.clone(),
        history_source: Some(
            pit_feature_history_source(
                point.feature_snapshot_id.as_deref(),
                point.as_of_date,
                HISTORY_SOURCE_RAW_OBSERVATION_REPLAY,
            )
            .to_string(),
        ),
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

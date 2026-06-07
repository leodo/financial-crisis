use fc_domain::{
    AlertEvent, AssessmentMethodVersions, AssessmentScores, AssessmentSnapshot,
    BacktestRollingAudit, BacktestScenarioSummary, DataMode, IndicatorRisk, Observation,
    PostureGuidance, RiskDimension, RiskSnapshot, UserRiskPreferences,
};

mod common;
mod context;
mod market_context;
mod posture;
mod probability;
mod runtime_policy;

use common::{
    clamp_probability, format_probability_threshold, posture_label, round6, round_option,
    scaled_pressure,
};
use common::{round1, round3};
pub(crate) use context::build_backtest_summary;
use context::{
    build_event_assessment, build_historical_analogs, build_key_indicator_statuses,
    build_runtime_metadata,
};
use market_context::{
    build_conviction_score, build_data_trust, build_jpy_carry_snapshot, build_relief_drivers,
    high_risk_breadth,
};
use posture::{
    build_position_guidance, build_posture_guidance, build_summary, build_time_to_risk_bucket,
};
pub(crate) use probability::ProbabilityComputationTrace;
use probability::{build_probabilities, build_probability_trace};
pub use runtime_policy::runtime_threshold_diagnostics;
pub(crate) use runtime_policy::{
    history_runtime_policy_version, probability_action_thresholds, ProbabilityActionThresholds,
};
pub use runtime_policy::{RuntimeThresholdDiagnostics, ServingModelContext};

const PROB_MODEL_VERSION: &str = "prob_v1_20260531";
const CALIBRATION_VERSION: &str = "calib_v1_20260531";
const FEATURE_SET_VERSION: &str = "feature_v2_20260531";
const LABEL_VERSION: &str = "label_v1_20260530";
const POSTURE_POLICY_VERSION: &str = "posture_v1_20260530";
const ACTION_PLAYBOOK_VERSION: &str = "action_playbook_v1_20260531";
const PROBABILITY_MODE: &str = "heuristic_mvp";
const RELEASE_STATUS: &str = "degraded";

pub(crate) fn prepare_reference_p60d(trace: &ProbabilityComputationTrace) -> Option<f64> {
    trace
        .probability_diagnostics
        .horizon_overlays
        .iter()
        .find(|horizon| horizon.horizon_days == 60)
        .map(|horizon| {
            horizon
                .runtime_final_probability
                .unwrap_or(horizon.final_probability)
        })
}

#[allow(clippy::too_many_arguments)]
pub fn build_assessment_snapshot(
    data_mode: DataMode,
    snapshot: &RiskSnapshot,
    indicator_risks: &[IndicatorRisk],
    observations: &[Observation],
    alerts: &[AlertEvent],
    backtests: &[BacktestScenarioSummary],
    rolling_audit: Option<&BacktestRollingAudit>,
    serving_model: Option<&ServingModelContext>,
    user_preferences: &UserRiskPreferences,
) -> (
    AssessmentSnapshot,
    PostureGuidance,
    ProbabilityComputationTrace,
) {
    let jpy_carry = build_jpy_carry_snapshot(snapshot, indicator_risks, observations);
    let external_dimension_score = snapshot
        .dimensions
        .iter()
        .find(|dimension| dimension.dimension == RiskDimension::ExternalSector)
        .map(|dimension| dimension.score)
        .unwrap_or(0.0);
    let event_dimension_score = snapshot
        .dimensions
        .iter()
        .find(|dimension| dimension.dimension == RiskDimension::EventsSentiment)
        .map(|dimension| dimension.score)
        .unwrap_or(0.0);
    let external_shock_score = round1(
        (external_dimension_score * 0.45 + jpy_carry.score * 0.4 + event_dimension_score * 0.15)
            .clamp(0.0, 100.0),
    );
    let data_trust = build_data_trust(snapshot, indicator_risks, jpy_carry.usdjpy_level.is_some());
    let breadth_score = high_risk_breadth(snapshot);
    let conviction_score = build_conviction_score(snapshot, &data_trust, breadth_score);
    let heuristic_probabilities = build_probabilities(
        snapshot,
        external_shock_score,
        conviction_score,
        breadth_score,
        &data_trust,
        &jpy_carry,
    );
    let runtime = build_runtime_metadata(data_mode, snapshot, observations);
    let key_indicators = build_key_indicator_statuses(observations, snapshot.as_of_date, data_mode);
    let probability_trace = build_probability_trace(
        snapshot,
        observations,
        external_shock_score,
        &data_trust,
        &jpy_carry,
        &heuristic_probabilities,
        &key_indicators,
        serving_model,
    );
    let probabilities = probability_trace.calibrated_probabilities.clone();
    let actionability = probability_trace.actionability.clone();
    let actionability_trigger = probability_trace
        .actionability_enabled
        .then_some(&actionability);
    let actionability_support = Some(&actionability);
    let prepare_reference_p60d = prepare_reference_p60d(&probability_trace);
    let active_release = serving_model.map(|context| &context.release);
    let action_thresholds = probability_action_thresholds(serving_model);
    let time_to_risk_bucket = build_time_to_risk_bucket(
        &probabilities,
        prepare_reference_p60d,
        actionability_trigger,
        actionability_support,
        snapshot.overall_score,
        snapshot.structural_score,
        snapshot.trigger_score,
        external_shock_score,
        breadth_score,
        &jpy_carry,
        action_thresholds,
    );
    let top_risk_drivers = snapshot.top_contributors.clone();
    let top_relief_drivers = build_relief_drivers(indicator_risks);
    let historical_analogs = build_historical_analogs(
        snapshot,
        &probabilities,
        external_shock_score,
        backtests,
        action_thresholds,
    );
    let event_assessment = build_event_assessment(snapshot, alerts);
    let backtest_summary = build_backtest_summary(backtests, rolling_audit);
    let posture_guidance = build_posture_guidance(
        snapshot,
        &probabilities,
        prepare_reference_p60d,
        actionability_trigger,
        actionability_support,
        conviction_score,
        &data_trust,
        external_shock_score,
        breadth_score,
        &historical_analogs,
        &jpy_carry,
        &event_assessment,
        user_preferences,
        action_thresholds,
    );
    let position_guidance = build_position_guidance(
        &posture_guidance,
        &probabilities,
        time_to_risk_bucket,
        &data_trust,
        &jpy_carry,
        &event_assessment,
        active_release,
        user_preferences,
        action_thresholds,
    );
    let method = AssessmentMethodVersions {
        score_method_version: snapshot.method_version.clone(),
        prob_model_version: active_release
            .map(|release| release.manifest.prob_model_version.clone())
            .unwrap_or_else(|| PROB_MODEL_VERSION.to_string()),
        calibration_version: active_release
            .map(|release| release.manifest.calibration_version.clone())
            .unwrap_or_else(|| CALIBRATION_VERSION.to_string()),
        actionability_model_version: probability_trace.actionability_model_version.clone(),
        actionability_calibration_version: probability_trace
            .actionability_calibration_version
            .clone(),
        feature_set_version: active_release
            .map(|release| release.manifest.feature_set_version.clone())
            .unwrap_or_else(|| FEATURE_SET_VERSION.to_string()),
        label_version: active_release
            .map(|release| release.manifest.label_version.clone())
            .unwrap_or_else(|| LABEL_VERSION.to_string()),
        posture_policy_version: active_release
            .map(|release| release.manifest.posture_policy_version.clone())
            .unwrap_or_else(|| POSTURE_POLICY_VERSION.to_string()),
        action_playbook_version: active_release
            .map(|release| release.manifest.action_playbook_version.clone())
            .unwrap_or_else(|| ACTION_PLAYBOOK_VERSION.to_string()),
        fusion_policy_version: probability_trace.fusion_policy_version.clone(),
        actionability_enabled: probability_trace.actionability_enabled,
        probability_mode: serving_model
            .map(|context| context.runtime_probability_mode.clone())
            .unwrap_or_else(|| PROBABILITY_MODE.to_string()),
        release_status: serving_model
            .map(|context| context.runtime_release_status.clone())
            .unwrap_or_else(|| RELEASE_STATUS.to_string()),
        release_id: active_release.map(|release| release.manifest.release_id.clone()),
        point_in_time_mode: active_release
            .map(|release| release.manifest.point_in_time_mode.clone())
            .unwrap_or_else(|| "best_effort".to_string()),
    };
    let summary = build_summary(
        snapshot,
        &probabilities,
        time_to_risk_bucket,
        &posture_guidance,
    );

    (
        AssessmentSnapshot {
            as_of_date: snapshot.as_of_date,
            entity_id: snapshot.entity_id.clone(),
            market_scope: snapshot.market_scope.clone(),
            probabilities,
            actionability,
            probability_diagnostics: probability_trace.probability_diagnostics.clone(),
            time_to_risk_bucket,
            posture: posture_guidance.posture,
            conviction_score,
            scores: AssessmentScores {
                overall_score: snapshot.overall_score,
                structural_score: snapshot.structural_score,
                trigger_score: snapshot.trigger_score,
                external_shock_score,
            },
            summary,
            posture_reason: posture_guidance.summary.clone(),
            top_risk_drivers,
            top_relief_drivers,
            historical_analogs,
            data_trust,
            jpy_carry,
            position_guidance,
            runtime,
            key_indicators,
            event_assessment,
            backtest_summary,
            user_preferences: user_preferences.clone(),
            method,
        },
        posture_guidance,
        probability_trace,
    )
}

#[cfg(test)]
mod tests;

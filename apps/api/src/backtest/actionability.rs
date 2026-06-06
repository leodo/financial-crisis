use fc_domain::{AssessmentHistoryPoint, DecisionPosture, TimeToRiskBucket};

use crate::{
    assessment::{ProbabilityActionThresholds, ServingModelContext},
    history_replay::is_formal_main_release,
};

const ACTIONABLE_AUDIT_HORIZON_DEFEND_DAYS: i64 = 5;
const ACTIONABLE_AUDIT_HORIZON_HEDGE_DAYS: i64 = 20;
const ACTIONABLE_AUDIT_HORIZON_PREPARE_DAYS: i64 = 60;
const LEGACY_STRICT_PREPARE_P20D_THRESHOLD: f64 = 0.18;
const LEGACY_STRICT_PREPARE_P60D_THRESHOLD: f64 = 0.45;
const STRICT_PREPARE_P60D_THRESHOLD_BUFFER: f64 = 0.04;
const STRICT_PREPARE_P60D_THRESHOLD_LIFT: f64 = 1.10;
const STRICT_PREPARE_P60D_THRESHOLD_MIN: f64 = 0.25;
const STRICT_PREPARE_PLATEAU_P20D_THRESHOLD: f64 = 0.45;
const STRICT_PREPARE_PLATEAU_P60D_THRESHOLD: f64 = 0.70;
const STRICT_PREPARE_PLATEAU_OVERALL_FLOOR: f64 = 42.0;
const STRICT_PREPARE_PLATEAU_EXTERNAL_FLOOR: f64 = 32.0;

pub(crate) fn is_actionable_warning_point(
    point: &AssessmentHistoryPoint,
    use_transitional_bridge: bool,
    strict_thresholds: Option<ProbabilityActionThresholds>,
) -> bool {
    let strict_prepare_p20d_threshold = strict_prepare_p20d_threshold(strict_thresholds);
    let strict_prepare_p60d_threshold = strict_prepare_p60d_threshold(strict_thresholds);
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
        && point.p_20d >= strict_prepare_p20d_threshold
        && point.p_60d >= strict_prepare_p60d_threshold
        && ((point.overall_score >= 60.0 && point.external_shock_score >= 46.0)
            || (point.overall_score >= 53.0
                && !matches!(point.time_to_risk_bucket, TimeToRiskBucket::Normal)
                && has_strong_prepare_trigger_code(point)));
    let probability_plateau_prepare_signal = matches!(point.posture, DecisionPosture::Prepare)
        && matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
        && point.p_20d >= STRICT_PREPARE_PLATEAU_P20D_THRESHOLD
        && point.p_60d >= strict_prepare_p60d_threshold.max(STRICT_PREPARE_PLATEAU_P60D_THRESHOLD)
        && point.overall_score >= STRICT_PREPARE_PLATEAU_OVERALL_FLOOR
        && point.external_shock_score >= STRICT_PREPARE_PLATEAU_EXTERNAL_FLOOR
        && has_probability_plateau_trigger_code(point);
    let high_probability_months_signal =
        matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
            && point.overall_score >= 62.0
            && point.p_20d >= strict_prepare_p20d_threshold
            && point.p_60d >= strict_prepare_p60d_threshold
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
        || probability_plateau_prepare_signal
        || high_probability_months_signal
        || prepare_bridge_signal
        || months_bridge_signal
}

fn strict_prepare_p20d_threshold(_strict_thresholds: Option<ProbabilityActionThresholds>) -> f64 {
    LEGACY_STRICT_PREPARE_P20D_THRESHOLD
}

fn strict_prepare_p60d_threshold(strict_thresholds: Option<ProbabilityActionThresholds>) -> f64 {
    strict_thresholds
        .map(|thresholds| {
            (thresholds.prepare_p60d + STRICT_PREPARE_P60D_THRESHOLD_BUFFER)
                .max(thresholds.prepare_p60d * STRICT_PREPARE_P60D_THRESHOLD_LIFT)
                .clamp(
                    STRICT_PREPARE_P60D_THRESHOLD_MIN,
                    LEGACY_STRICT_PREPARE_P60D_THRESHOLD,
                )
        })
        .unwrap_or(LEGACY_STRICT_PREPARE_P60D_THRESHOLD)
}

pub(crate) fn use_transitional_actionable_bridge(
    serving_model: Option<&ServingModelContext>,
) -> bool {
    !is_formal_main_release(serving_model)
}

pub(super) fn is_structural_warning_point(point: &AssessmentHistoryPoint) -> bool {
    ((point.p_60d >= 0.35) || !matches!(point.time_to_risk_bucket, TimeToRiskBucket::Normal))
        && point.overall_score >= 54.0
        || (point.overall_score >= 54.0
            && point.p_20d >= 0.12
            && point.external_shock_score >= 42.0)
}

pub(super) fn actionable_audit_horizon_days(point: &AssessmentHistoryPoint) -> i64 {
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

fn has_strong_prepare_trigger_code(point: &AssessmentHistoryPoint) -> bool {
    point.posture_trigger_codes.iter().any(|code| {
        matches!(
            code.as_str(),
            "prepare_p60d_structural"
                | "prepare_structural_downgrade"
                | "prepare_carry_structural"
                | "prepare_external_structural"
                | "prepare_continuity_bridge"
                | "prepare_probability_plateau"
        )
    })
}

fn has_probability_plateau_trigger_code(point: &AssessmentHistoryPoint) -> bool {
    point
        .posture_trigger_codes
        .iter()
        .any(|code| code == "prepare_probability_plateau")
}

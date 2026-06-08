use fc_domain::{AssessmentHistoryPoint, DecisionPosture, TimeToRiskBucket};

use crate::{
    assessment::{ProbabilityActionThresholds, ServingModelContext},
    history_replay::is_formal_main_release,
};

const ACTIONABLE_AUDIT_HORIZON_DEFEND_DAYS: i64 = 5;
const ACTIONABLE_AUDIT_HORIZON_HEDGE_DAYS: i64 = 20;
const ACTIONABLE_AUDIT_HORIZON_PREPARE_DAYS: i64 = 60;
const LEGACY_STRICT_PREPARE_P20D_THRESHOLD: f64 = 0.18;
const STRICT_PREPARE_P20D_THRESHOLD_RATIO: f64 = 0.60;
const STRICT_PREPARE_P20D_THRESHOLD_MIN: f64 = 0.12;
const LEGACY_STRICT_PREPARE_P60D_THRESHOLD: f64 = 0.45;
const STRICT_PREPARE_P60D_THRESHOLD_BUFFER: f64 = 0.04;
const STRICT_PREPARE_P60D_THRESHOLD_LIFT: f64 = 1.10;
const STRICT_PREPARE_P60D_THRESHOLD_MIN: f64 = 0.25;
const STRICT_PREPARE_PLATEAU_P20D_BUFFER: f64 = 0.10;
const STRICT_PREPARE_PLATEAU_P20D_MIN: f64 = 0.35;
const STRICT_PREPARE_PLATEAU_P20D_MAX: f64 = 0.45;
const STRICT_PREPARE_PLATEAU_RELAXED_P20D_BUFFER: f64 = 0.10;
const STRICT_PREPARE_PLATEAU_RELAXED_P20D_FLOOR_MIN: f64 = 0.45;
const STRICT_PREPARE_PLATEAU_P60D_THRESHOLD: f64 = 0.70;
const STRICT_PREPARE_PLATEAU_RELAXED_P60D_THRESHOLD: f64 = 0.65;
const STRICT_PREPARE_PLATEAU_OVERALL_FLOOR: f64 = 42.0;
const STRICT_PREPARE_PLATEAU_EXTERNAL_FLOOR: f64 = 32.0;
const STRICT_PREPARE_PLATEAU_RELAXED_EXTERNAL_FLOOR: f64 = 40.0;
const STRICT_PREPARE_WEEKS_TRIGGER_OVERALL_FLOOR: f64 = 51.5;
const STRICT_PREPARE_WEEKS_TRIGGER_EXTERNAL_FLOOR: f64 = 33.0;
const STRICT_WEEKS_TRIGGER_DOMINANT_P20D_FLOOR: f64 = 0.25;
const STRICT_WEEKS_TRIGGER_DOMINANT_P20D_SPREAD_FLOOR: f64 = 0.15;
const STRICT_WEEKS_TRIGGER_DOMINANT_OVERALL_FLOOR: f64 = 53.0;
const STRICT_WEEKS_TRIGGER_DOMINANT_EXTERNAL_FLOOR: f64 = 35.0;
const STRICT_HISTORY_HYSTERESIS_MONTHS_P20D_FLOOR: f64 = 0.35;
const STRICT_HISTORY_HYSTERESIS_MONTHS_P60D_FLOOR: f64 = 0.65;
const STRICT_HISTORY_HYSTERESIS_MONTHS_OVERALL_FLOOR: f64 = 43.0;
const STRICT_HISTORY_HYSTERESIS_MONTHS_EXTERNAL_FLOOR: f64 = 39.0;
const STRICT_HISTORY_HYSTERESIS_MONTHS_STRUCTURAL_CARRY_P20D_FLOOR: f64 = 0.25;
const STRICT_HISTORY_HYSTERESIS_MONTHS_STRUCTURAL_CARRY_P60D_FLOOR: f64 = 0.80;
const STRICT_HISTORY_HYSTERESIS_MONTHS_STRUCTURAL_CARRY_OVERALL_FLOOR: f64 = 43.5;
const STRICT_HISTORY_HYSTERESIS_MONTHS_STRUCTURAL_CARRY_EXTERNAL_FLOOR: f64 = 30.0;

pub(crate) fn is_actionable_warning_point(
    point: &AssessmentHistoryPoint,
    use_transitional_bridge: bool,
    strict_thresholds: Option<ProbabilityActionThresholds>,
) -> bool {
    let strict_prepare_p20d_threshold = strict_prepare_p20d_threshold(strict_thresholds);
    let strict_prepare_p60d_threshold = strict_prepare_p60d_threshold(strict_thresholds);
    let strict_prepare_plateau_p20d_threshold =
        strict_prepare_plateau_p20d_threshold(strict_thresholds);
    let strict_prepare_relaxed_plateau_p20d_threshold =
        strict_prepare_relaxed_plateau_p20d_threshold(strict_thresholds);
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
        && has_probability_plateau_trigger_code(point);
    let standard_probability_plateau_prepare_signal = probability_plateau_prepare_signal
        && point.p_20d >= strict_prepare_plateau_p20d_threshold
        && point.p_60d >= strict_prepare_p60d_threshold.max(STRICT_PREPARE_PLATEAU_P60D_THRESHOLD)
        && point.overall_score >= STRICT_PREPARE_PLATEAU_OVERALL_FLOOR
        && point.external_shock_score >= STRICT_PREPARE_PLATEAU_EXTERNAL_FLOOR;
    let relaxed_probability_plateau_prepare_signal = probability_plateau_prepare_signal
        && point.p_20d >= strict_prepare_relaxed_plateau_p20d_threshold
        && point.p_60d >= STRICT_PREPARE_PLATEAU_RELAXED_P60D_THRESHOLD
        && point.overall_score >= STRICT_PREPARE_PLATEAU_OVERALL_FLOOR
        && point.external_shock_score >= STRICT_PREPARE_PLATEAU_RELAXED_EXTERNAL_FLOOR;
    let weeks_trigger_dominant_signal = is_actionable_weeks_trigger_dominant_signal(
        point,
        strict_thresholds,
        strict_prepare_p20d_threshold,
        strict_prepare_p60d_threshold,
    );
    let prepare_weeks_plateau_hysteresis_signal =
        is_actionable_prepare_weeks_plateau_hysteresis_signal(point, strict_thresholds);
    let high_probability_months_signal =
        matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
            && point.overall_score >= 62.0
            && point.p_20d >= strict_prepare_p20d_threshold
            && point.p_60d >= strict_prepare_p60d_threshold
            && point.external_shock_score >= 48.0;
    let history_hysteresis_months_signal =
        matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
            && has_history_hysteresis_trigger_code(point)
            && point.p_20d >= STRICT_HISTORY_HYSTERESIS_MONTHS_P20D_FLOOR
            && point.p_60d
                >= strict_prepare_p60d_threshold.max(STRICT_HISTORY_HYSTERESIS_MONTHS_P60D_FLOOR)
            && (point.overall_score >= STRICT_HISTORY_HYSTERESIS_MONTHS_OVERALL_FLOOR
                || point.external_shock_score >= STRICT_HISTORY_HYSTERESIS_MONTHS_EXTERNAL_FLOOR);
    let history_hysteresis_months_structural_carry_signal =
        is_actionable_history_hysteresis_months_structural_carry_signal(
            point,
            strict_thresholds,
            strict_prepare_p60d_threshold,
        );

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
        || standard_probability_plateau_prepare_signal
        || relaxed_probability_plateau_prepare_signal
        || weeks_trigger_dominant_signal
        || prepare_weeks_plateau_hysteresis_signal
        || high_probability_months_signal
        || history_hysteresis_months_signal
        || history_hysteresis_months_structural_carry_signal
        || prepare_bridge_signal
        || months_bridge_signal
}

fn strict_prepare_p20d_threshold(strict_thresholds: Option<ProbabilityActionThresholds>) -> f64 {
    strict_thresholds
        .map(|thresholds| {
            (thresholds.external_prepare_p20d() * STRICT_PREPARE_P20D_THRESHOLD_RATIO).clamp(
                STRICT_PREPARE_P20D_THRESHOLD_MIN,
                LEGACY_STRICT_PREPARE_P20D_THRESHOLD,
            )
        })
        .unwrap_or(LEGACY_STRICT_PREPARE_P20D_THRESHOLD)
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

fn strict_prepare_plateau_p20d_threshold(
    strict_thresholds: Option<ProbabilityActionThresholds>,
) -> f64 {
    strict_thresholds
        .map(|thresholds| {
            (thresholds.hedge_p20d + STRICT_PREPARE_PLATEAU_P20D_BUFFER).clamp(
                STRICT_PREPARE_PLATEAU_P20D_MIN,
                STRICT_PREPARE_PLATEAU_P20D_MAX,
            )
        })
        .unwrap_or(STRICT_PREPARE_PLATEAU_P20D_MAX)
}

fn strict_prepare_relaxed_plateau_p20d_threshold(
    strict_thresholds: Option<ProbabilityActionThresholds>,
) -> f64 {
    (strict_prepare_plateau_p20d_threshold(strict_thresholds)
        + STRICT_PREPARE_PLATEAU_RELAXED_P20D_BUFFER)
        .max(STRICT_PREPARE_PLATEAU_RELAXED_P20D_FLOOR_MIN)
}

fn has_prepare_weeks_plateau_hysteresis_setup(point: &AssessmentHistoryPoint) -> bool {
    matches!(point.posture, DecisionPosture::Prepare)
        && matches!(point.time_to_risk_bucket, TimeToRiskBucket::Weeks)
        && has_probability_plateau_trigger_code(point)
        && has_history_hysteresis_trigger_code(point)
}

fn is_actionable_prepare_weeks_plateau_hysteresis_signal(
    point: &AssessmentHistoryPoint,
    strict_thresholds: Option<ProbabilityActionThresholds>,
) -> bool {
    has_prepare_weeks_plateau_hysteresis_setup(point)
        && point.p_20d >= strict_prepare_relaxed_plateau_p20d_threshold(strict_thresholds)
        && point.p_60d >= STRICT_PREPARE_PLATEAU_RELAXED_P60D_THRESHOLD
        && point.overall_score >= STRICT_PREPARE_WEEKS_TRIGGER_OVERALL_FLOOR
        && point.external_shock_score >= STRICT_PREPARE_WEEKS_TRIGGER_EXTERNAL_FLOOR
}

fn is_actionable_weeks_trigger_dominant_signal(
    point: &AssessmentHistoryPoint,
    strict_thresholds: Option<ProbabilityActionThresholds>,
    strict_prepare_p20d_threshold: f64,
    strict_prepare_p60d_threshold: f64,
) -> bool {
    strict_thresholds.is_some()
        && matches!(point.time_to_risk_bucket, TimeToRiskBucket::Weeks)
        && point.p_20d
            >= strict_prepare_p20d_threshold.max(STRICT_WEEKS_TRIGGER_DOMINANT_P20D_FLOOR)
        && point.p_60d < strict_prepare_p60d_threshold
        && (point.p_20d - point.p_60d) >= STRICT_WEEKS_TRIGGER_DOMINANT_P20D_SPREAD_FLOOR
        && point.overall_score >= STRICT_WEEKS_TRIGGER_DOMINANT_OVERALL_FLOOR
        && point.external_shock_score >= STRICT_WEEKS_TRIGGER_DOMINANT_EXTERNAL_FLOOR
}

fn is_actionable_history_hysteresis_months_structural_carry_signal(
    point: &AssessmentHistoryPoint,
    strict_thresholds: Option<ProbabilityActionThresholds>,
    strict_prepare_p60d_threshold: f64,
) -> bool {
    strict_thresholds.is_some()
        && matches!(point.posture, DecisionPosture::Prepare)
        && matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
        && has_history_hysteresis_trigger_code(point)
        && point.p_20d >= STRICT_HISTORY_HYSTERESIS_MONTHS_STRUCTURAL_CARRY_P20D_FLOOR
        && point.p_60d
            >= strict_prepare_p60d_threshold
                .max(STRICT_HISTORY_HYSTERESIS_MONTHS_STRUCTURAL_CARRY_P60D_FLOOR)
        && point.overall_score >= STRICT_HISTORY_HYSTERESIS_MONTHS_STRUCTURAL_CARRY_OVERALL_FLOOR
        && point.external_shock_score
            >= STRICT_HISTORY_HYSTERESIS_MONTHS_STRUCTURAL_CARRY_EXTERNAL_FLOOR
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
                | "prepare_history_hysteresis"
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

fn has_history_hysteresis_trigger_code(point: &AssessmentHistoryPoint) -> bool {
    point
        .posture_trigger_codes
        .iter()
        .any(|code| code == "prepare_history_hysteresis")
}

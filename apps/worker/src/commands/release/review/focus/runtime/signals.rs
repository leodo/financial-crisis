use std::collections::BTreeMap;

use chrono::{Duration, NaiveDate};
use fc_domain::{
    AssessmentHistoryPoint, BacktestScenarioSummary, DecisionPosture, TimeToRiskBucket,
};

const RELEASE_REVIEW_SIGNAL_WINDOW: usize = 5;
const RELEASE_REVIEW_SIGNAL_MIN_HITS: usize = 3;
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

pub(crate) fn release_review_structured_signal_counts(
    backtests: &[BacktestScenarioSummary],
    history: &[AssessmentHistoryPoint],
    method: &crate::AuditMethodResponseWire,
) -> (u32, u32) {
    let use_transitional_bridge = release_review_uses_transitional_actionable_bridge(method);
    let thresholds = method.runtime_thresholds.as_ref();
    let in_any_pre_crisis_window = |point: &AssessmentHistoryPoint| {
        backtests.iter().any(|scenario| {
            let window_start = scenario.crisis_start - Duration::days(90);
            point.as_of_date >= window_start && point.as_of_date < scenario.crisis_start
        })
    };
    let strict_actionable_point_count = history
        .iter()
        .filter(|point| in_any_pre_crisis_window(point))
        .filter(|point| {
            release_review_is_actionable_warning_point(point, use_transitional_bridge, thresholds)
        })
        .count() as u32;
    let runtime_floor_hit_count = history
        .iter()
        .filter(|point| in_any_pre_crisis_window(point))
        .filter(|point| release_review_hits_runtime_floor(point, thresholds))
        .count() as u32;
    (strict_actionable_point_count, runtime_floor_hit_count)
}

pub(super) fn release_review_actionable_forward_hits_by_date(
    points: &[&AssessmentHistoryPoint],
    use_transitional_bridge: bool,
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> BTreeMap<NaiveDate, (u32, bool)> {
    points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            let end = (index + RELEASE_REVIEW_SIGNAL_WINDOW).min(points.len());
            let window = &points[index..end];
            let hit_count = window
                .iter()
                .filter(|candidate| {
                    release_review_is_actionable_warning_point(
                        candidate,
                        use_transitional_bridge,
                        thresholds,
                    )
                })
                .count();
            let required_hits = RELEASE_REVIEW_SIGNAL_MIN_HITS.min(window.len());
            let sustained = release_review_is_actionable_warning_point(
                point,
                use_transitional_bridge,
                thresholds,
            ) && hit_count >= required_hits;
            (point.as_of_date, (hit_count as u32, sustained))
        })
        .collect()
}

pub(super) fn release_review_hits_runtime_floor(
    point: &AssessmentHistoryPoint,
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> bool {
    let Some(thresholds) = thresholds else {
        return false;
    };
    point.p_60d >= thresholds.prepare_p60d
        || point.p_20d >= thresholds.hedge_p20d
        || point.p_5d >= thresholds.defend_p5d
}

pub(super) fn release_review_uses_transitional_actionable_bridge(
    method: &crate::AuditMethodResponseWire,
) -> bool {
    !(method.method.probability_mode == "formal_bundle_v1"
        && method.method.label_version == "formal_label_v1_main"
        && method
            .method
            .feature_set_version
            .starts_with("feature_formal_v1_main"))
}

pub(super) fn release_review_strict_prepare_p20d_threshold(
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> f64 {
    thresholds
        .map(|thresholds| {
            (thresholds.external_prepare_p20d * STRICT_PREPARE_P20D_THRESHOLD_RATIO).clamp(
                STRICT_PREPARE_P20D_THRESHOLD_MIN,
                LEGACY_STRICT_PREPARE_P20D_THRESHOLD,
            )
        })
        .unwrap_or(LEGACY_STRICT_PREPARE_P20D_THRESHOLD)
}

pub(super) fn release_review_strict_prepare_p60d_threshold(
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> f64 {
    thresholds
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

fn release_review_strict_prepare_plateau_p20d_threshold(
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> f64 {
    thresholds
        .map(|thresholds| {
            (thresholds.hedge_p20d + STRICT_PREPARE_PLATEAU_P20D_BUFFER).clamp(
                STRICT_PREPARE_PLATEAU_P20D_MIN,
                STRICT_PREPARE_PLATEAU_P20D_MAX,
            )
        })
        .unwrap_or(STRICT_PREPARE_PLATEAU_P20D_MAX)
}

fn release_review_strict_prepare_relaxed_plateau_p20d_threshold(
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> f64 {
    (release_review_strict_prepare_plateau_p20d_threshold(thresholds)
        + STRICT_PREPARE_PLATEAU_RELAXED_P20D_BUFFER)
        .max(STRICT_PREPARE_PLATEAU_RELAXED_P20D_FLOOR_MIN)
}

pub(super) fn release_review_is_actionable_warning_point(
    point: &AssessmentHistoryPoint,
    use_transitional_bridge: bool,
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> bool {
    let strict_prepare_p20d_threshold = release_review_strict_prepare_p20d_threshold(thresholds);
    let strict_prepare_p60d_threshold = release_review_strict_prepare_p60d_threshold(thresholds);
    let strict_prepare_plateau_p20d_threshold =
        release_review_strict_prepare_plateau_p20d_threshold(thresholds);
    let strict_prepare_relaxed_plateau_p20d_threshold =
        release_review_strict_prepare_relaxed_plateau_p20d_threshold(thresholds);
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
                && release_review_has_strong_prepare_trigger_code(point)));
    let probability_plateau_prepare_setup = matches!(point.posture, DecisionPosture::Prepare)
        && matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
        && release_review_has_probability_plateau_trigger_code(point);
    let standard_probability_plateau_prepare_signal = probability_plateau_prepare_setup
        && point.p_20d >= strict_prepare_plateau_p20d_threshold
        && point.p_60d >= strict_prepare_p60d_threshold.max(STRICT_PREPARE_PLATEAU_P60D_THRESHOLD)
        && point.overall_score >= STRICT_PREPARE_PLATEAU_OVERALL_FLOOR
        && point.external_shock_score >= STRICT_PREPARE_PLATEAU_EXTERNAL_FLOOR;
    let relaxed_probability_plateau_prepare_signal = probability_plateau_prepare_setup
        && point.p_20d >= strict_prepare_relaxed_plateau_p20d_threshold
        && point.p_60d >= STRICT_PREPARE_PLATEAU_RELAXED_P60D_THRESHOLD
        && point.overall_score >= STRICT_PREPARE_PLATEAU_OVERALL_FLOOR
        && point.external_shock_score >= STRICT_PREPARE_PLATEAU_RELAXED_EXTERNAL_FLOOR;
    let high_probability_months_signal =
        matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
            && point.overall_score >= 62.0
            && point.p_20d >= strict_prepare_p20d_threshold
            && point.p_60d >= strict_prepare_p60d_threshold
            && point.external_shock_score >= 48.0;

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
        || high_probability_months_signal
        || prepare_bridge_signal
        || months_bridge_signal
}

pub(super) fn release_review_has_strong_prepare_trigger_code(
    point: &AssessmentHistoryPoint,
) -> bool {
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

fn release_review_has_probability_plateau_trigger_code(point: &AssessmentHistoryPoint) -> bool {
    point
        .posture_trigger_codes
        .iter()
        .any(|code| code == "prepare_probability_plateau")
}

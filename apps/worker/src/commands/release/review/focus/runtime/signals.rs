use std::collections::BTreeMap;

use chrono::{Duration, NaiveDate};
use fc_domain::{
    actionable_prepare_weeks_score_confirmation_gap as domain_actionable_prepare_weeks_score_confirmation_gap,
    actionable_runtime_floor_hits as domain_actionable_runtime_floor_hits,
    actionable_runtime_floor_reached as domain_actionable_runtime_floor_reached,
    actionable_warning_point as domain_actionable_warning_point,
    strict_prepare_p20d_threshold as domain_strict_prepare_p20d_threshold,
    strict_prepare_p60d_threshold as domain_strict_prepare_p60d_threshold,
    strong_prepare_trigger_code as domain_strong_prepare_trigger_code,
    weak_defend_only_runtime_floor as domain_weak_defend_only_runtime_floor,
    ActionableGateFloorHits, ActionableGateThresholds, AssessmentHistoryPoint,
    BacktestScenarioSummary,
};

const RELEASE_REVIEW_SIGNAL_WINDOW: usize = 5;
const RELEASE_REVIEW_SIGNAL_MIN_HITS: usize = 3;

pub(super) type ReleaseReviewRuntimeFloorHits = ActionableGateFloorHits;

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
    domain_actionable_runtime_floor_reached(point, actionable_gate_thresholds(thresholds))
}

pub(super) fn release_review_runtime_floor_hits(
    point: &AssessmentHistoryPoint,
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> Option<ReleaseReviewRuntimeFloorHits> {
    domain_actionable_runtime_floor_hits(point, actionable_gate_thresholds(thresholds))
}

pub(super) fn release_review_is_weak_defend_only_runtime_floor(
    point: &AssessmentHistoryPoint,
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> bool {
    domain_weak_defend_only_runtime_floor(point, actionable_gate_thresholds(thresholds))
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
    domain_strict_prepare_p20d_threshold(actionable_gate_thresholds(thresholds))
}

pub(super) fn release_review_strict_prepare_p60d_threshold(
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> f64 {
    domain_strict_prepare_p60d_threshold(actionable_gate_thresholds(thresholds))
}

pub(super) fn release_review_is_actionable_warning_point(
    point: &AssessmentHistoryPoint,
    use_transitional_bridge: bool,
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> bool {
    domain_actionable_warning_point(
        point,
        use_transitional_bridge,
        actionable_gate_thresholds(thresholds),
    )
}

pub(super) fn release_review_has_prepare_weeks_score_confirmation_gap(
    point: &AssessmentHistoryPoint,
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> bool {
    domain_actionable_prepare_weeks_score_confirmation_gap(
        point,
        actionable_gate_thresholds(thresholds),
    )
}

pub(super) fn release_review_has_strong_prepare_trigger_code(
    point: &AssessmentHistoryPoint,
) -> bool {
    domain_strong_prepare_trigger_code(point)
}

pub(super) fn actionable_gate_thresholds(
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> Option<ActionableGateThresholds> {
    thresholds.map(|thresholds| ActionableGateThresholds {
        prepare_p60d: thresholds.prepare_p60d,
        hedge_p20d: thresholds.hedge_p20d,
        defend_p5d: thresholds.defend_p5d,
        external_prepare_p20d: thresholds.external_prepare_p20d,
    })
}

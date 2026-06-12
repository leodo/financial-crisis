use fc_domain::AssessmentHistoryPoint;

use super::super::super::backtest::backtest_warning_state;
use super::super::{
    blocks::{
        release_review_primary_failure_mode, release_review_runtime_block_counts,
        release_review_runtime_continuity_facet_counts, release_review_runtime_dominant_categories,
    },
    signals::release_review_is_actionable_warning_point,
};
use super::window::PreparedScenarioWindow;

pub(super) fn build_focus_diagnostic(
    prepared: &PreparedScenarioWindow<'_>,
    interesting_points: Vec<crate::ReleaseReviewScenarioPointComparison>,
    baseline_use_transitional_bridge: bool,
    candidate_use_transitional_bridge: bool,
    baseline_runtime_thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
    candidate_runtime_thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> crate::ReleaseReviewScenarioFocusDiagnostic {
    let runtime_block_counts = release_review_runtime_block_counts(
        &prepared.baseline_pre_crisis_points,
        baseline_use_transitional_bridge,
        baseline_runtime_thresholds,
        &prepared.candidate_pre_crisis_points,
        candidate_use_transitional_bridge,
        candidate_runtime_thresholds,
    );
    let runtime_continuity_facet_counts = release_review_runtime_continuity_facet_counts(
        &prepared.baseline_pre_crisis_points,
        baseline_use_transitional_bridge,
        baseline_runtime_thresholds,
        &prepared.candidate_pre_crisis_points,
        candidate_use_transitional_bridge,
        candidate_runtime_thresholds,
    );
    let dominant_runtime_blocks = release_review_runtime_dominant_categories(&runtime_block_counts);
    let dominant_runtime_continuity_facets =
        release_review_runtime_dominant_categories(&runtime_continuity_facet_counts);
    let baseline_actionable_point_count = actionable_point_count(
        &prepared.baseline_pre_crisis_points,
        baseline_use_transitional_bridge,
        baseline_runtime_thresholds,
    );
    let candidate_actionable_point_count = actionable_point_count(
        &prepared.candidate_pre_crisis_points,
        candidate_use_transitional_bridge,
        candidate_runtime_thresholds,
    );
    let baseline_primary_failure_mode = release_review_primary_failure_mode(
        &dominant_runtime_blocks.baseline_categories,
        dominant_runtime_blocks.baseline_count,
        &dominant_runtime_continuity_facets.baseline_categories,
        dominant_runtime_continuity_facets.baseline_count,
    );
    let candidate_primary_failure_mode = release_review_primary_failure_mode(
        &dominant_runtime_blocks.candidate_categories,
        dominant_runtime_blocks.candidate_count,
        &dominant_runtime_continuity_facets.candidate_categories,
        dominant_runtime_continuity_facets.candidate_count,
    );

    crate::ReleaseReviewScenarioFocusDiagnostic {
        baseline_primary_failure_mode,
        candidate_primary_failure_mode,
        dominant_runtime_blocks,
        dominant_runtime_continuity_facets,
        scenario_id: prepared.baseline.scenario_id.clone(),
        name: prepared.baseline.name.clone(),
        outcome: format!(
            "{}_to_{}",
            backtest_warning_state(prepared.baseline.actionable_lead_time_days),
            backtest_warning_state(
                prepared
                    .candidate
                    .and_then(|scenario| scenario.actionable_lead_time_days)
            )
        ),
        window_start: prepared.window_start,
        window_end: prepared.window_end,
        crisis_start: prepared.baseline.crisis_start,
        crisis_end: prepared.baseline.crisis_end,
        baseline_first_l2_date: prepared.baseline.first_l2_date,
        candidate_first_l2_date: prepared
            .candidate
            .and_then(|scenario| scenario.first_l2_date),
        baseline_first_l3_date: prepared.baseline.first_l3_date,
        candidate_first_l3_date: prepared
            .candidate
            .and_then(|scenario| scenario.first_l3_date),
        baseline_first_non_normal_date: prepared.baseline_first_non_normal_date,
        candidate_first_non_normal_date: prepared.candidate_first_non_normal_date,
        baseline_actionable_point_count,
        candidate_actionable_point_count,
        baseline_runtime_floor_hit_point_count: prepared.baseline_runtime_floor_hit_point_count,
        candidate_runtime_floor_hit_point_count: prepared.candidate_runtime_floor_hit_point_count,
        baseline_max_p20d: release_review_max_metric(
            &prepared.baseline_pre_crisis_points,
            |point| point.p_20d,
        ),
        candidate_max_p20d: release_review_max_metric(
            &prepared.candidate_pre_crisis_points,
            |point| point.p_20d,
        ),
        baseline_max_p60d: release_review_max_metric(
            &prepared.baseline_pre_crisis_points,
            |point| point.p_60d,
        ),
        candidate_max_p60d: release_review_max_metric(
            &prepared.candidate_pre_crisis_points,
            |point| point.p_60d,
        ),
        baseline_first_runtime_floor_hit_without_l3_date: prepared
            .baseline_first_runtime_floor_hit_without_l3
            .as_ref()
            .map(|(date, _)| *date),
        candidate_first_runtime_floor_hit_without_l3_date: prepared
            .candidate_first_runtime_floor_hit_without_l3
            .as_ref()
            .map(|(date, _)| *date),
        baseline_first_runtime_floor_hit_without_l3_reason: prepared
            .baseline_first_runtime_floor_hit_without_l3
            .as_ref()
            .map(|(_, reason)| reason.clone()),
        candidate_first_runtime_floor_hit_without_l3_reason: prepared
            .candidate_first_runtime_floor_hit_without_l3
            .as_ref()
            .map(|(_, reason)| reason.clone()),
        runtime_block_counts,
        runtime_continuity_facet_counts,
        interesting_points,
    }
}

fn actionable_point_count(
    points: &[&AssessmentHistoryPoint],
    use_transitional_bridge: bool,
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> u32 {
    points
        .iter()
        .filter(|point| {
            release_review_is_actionable_warning_point(point, use_transitional_bridge, thresholds)
        })
        .count() as u32
}

fn release_review_max_metric(
    points: &[&AssessmentHistoryPoint],
    accessor: impl Fn(&AssessmentHistoryPoint) -> f64,
) -> Option<f64> {
    points
        .iter()
        .map(|point| accessor(point))
        .max_by(|left, right| left.total_cmp(right))
}

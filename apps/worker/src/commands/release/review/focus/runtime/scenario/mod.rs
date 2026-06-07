use std::collections::BTreeMap;

use fc_domain::{AssessmentHistoryPoint, BacktestScenarioSummary};

use super::super::backtest::scenario_requires_focus_review;
use super::signals::release_review_uses_transitional_actionable_bridge;

mod points;
mod summary;
mod window;

use points::build_interesting_points;
use summary::build_focus_diagnostic;
use window::{prepare_focus_window, FocusRuntimeConfig};

pub(crate) fn build_release_review_scenario_focus_diagnostics(
    baseline_backtests: &[BacktestScenarioSummary],
    candidate_backtests: &[BacktestScenarioSummary],
    baseline_history: &[AssessmentHistoryPoint],
    candidate_history: &[AssessmentHistoryPoint],
    baseline_method: &crate::AuditMethodResponseWire,
    candidate_method: &crate::AuditMethodResponseWire,
) -> Vec<crate::ReleaseReviewScenarioFocusDiagnostic> {
    let candidate_by_id = candidate_backtests
        .iter()
        .map(|scenario| (scenario.scenario_id.as_str(), scenario))
        .collect::<BTreeMap<_, _>>();
    let baseline_points_by_date = baseline_history
        .iter()
        .map(|point| (point.as_of_date, point))
        .collect::<BTreeMap<_, _>>();
    let candidate_points_by_date = candidate_history
        .iter()
        .map(|point| (point.as_of_date, point))
        .collect::<BTreeMap<_, _>>();
    let baseline_use_transitional_bridge =
        release_review_uses_transitional_actionable_bridge(baseline_method);
    let candidate_use_transitional_bridge =
        release_review_uses_transitional_actionable_bridge(candidate_method);
    let baseline_runtime_thresholds = baseline_method.runtime_thresholds.as_ref();
    let candidate_runtime_thresholds = candidate_method.runtime_thresholds.as_ref();
    let runtime_config = FocusRuntimeConfig {
        baseline_use_transitional_bridge,
        candidate_use_transitional_bridge,
        baseline_runtime_thresholds,
        candidate_runtime_thresholds,
    };

    let mut rows = baseline_backtests
        .iter()
        .filter_map(|baseline| {
            let candidate = candidate_by_id.get(baseline.scenario_id.as_str()).copied();
            let prepared = prepare_focus_window(
                baseline,
                candidate,
                baseline_history,
                candidate_history,
                &runtime_config,
            );
            if !scenario_requires_focus_review(baseline, candidate)
                && !prepared_scenario_requires_focus_review(&prepared)
            {
                return None;
            }
            let interesting_points = build_interesting_points(
                &prepared,
                &baseline_points_by_date,
                &candidate_points_by_date,
                baseline_use_transitional_bridge,
                candidate_use_transitional_bridge,
                baseline_runtime_thresholds,
                candidate_runtime_thresholds,
            );

            let mut row = build_focus_diagnostic(
                &prepared,
                interesting_points,
                baseline_use_transitional_bridge,
                candidate_use_transitional_bridge,
                baseline_runtime_thresholds,
                candidate_runtime_thresholds,
            );
            if suppress_candidate_primary_failure_mode(&row) {
                row.candidate_primary_failure_mode = None;
            }
            Some(row)
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));
    rows
}

fn suppress_candidate_primary_failure_mode(
    row: &crate::ReleaseReviewScenarioFocusDiagnostic,
) -> bool {
    let candidate_not_worse_on_l3 = match (row.baseline_first_l3_date, row.candidate_first_l3_date)
    {
        (None, Some(_)) => true,
        (Some(baseline_date), Some(candidate_date)) => candidate_date <= baseline_date,
        _ => false,
    };

    candidate_not_worse_on_l3
        && row.candidate_actionable_point_count >= row.baseline_actionable_point_count
}

fn prepared_scenario_requires_focus_review(prepared: &window::PreparedScenarioWindow<'_>) -> bool {
    prepared.baseline_runtime_floor_hit_point_count > 0
        || prepared.candidate_runtime_floor_hit_point_count > 0
        || prepared
            .baseline_first_runtime_floor_hit_without_l3
            .is_some()
        || prepared
            .candidate_first_runtime_floor_hit_without_l3
            .is_some()
}

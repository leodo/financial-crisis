use std::collections::BTreeMap;

use chrono::{Duration, NaiveDate};
use fc_domain::{AssessmentHistoryPoint, BacktestScenarioSummary};

use super::super::{
    blocks::release_review_runtime_actionable_block_reason,
    signals::{release_review_actionable_forward_hits_by_date, release_review_hits_runtime_floor},
};
use super::points::release_review_point_is_non_normal;

pub(super) struct PreparedScenarioWindow<'a> {
    pub(super) baseline: &'a BacktestScenarioSummary,
    pub(super) candidate: Option<&'a BacktestScenarioSummary>,
    pub(super) window_start: NaiveDate,
    pub(super) window_end: NaiveDate,
    pub(super) baseline_window_points: Vec<&'a AssessmentHistoryPoint>,
    pub(super) candidate_window_points: Vec<&'a AssessmentHistoryPoint>,
    pub(super) baseline_pre_crisis_points: Vec<&'a AssessmentHistoryPoint>,
    pub(super) candidate_pre_crisis_points: Vec<&'a AssessmentHistoryPoint>,
    pub(super) baseline_first_non_normal_date: Option<NaiveDate>,
    pub(super) candidate_first_non_normal_date: Option<NaiveDate>,
    pub(super) baseline_runtime_floor_hit_point_count: u32,
    pub(super) candidate_runtime_floor_hit_point_count: u32,
    pub(super) baseline_actionable_hits: BTreeMap<NaiveDate, (u32, bool)>,
    pub(super) candidate_actionable_hits: BTreeMap<NaiveDate, (u32, bool)>,
    pub(super) baseline_first_runtime_floor_hit_without_l3: Option<(NaiveDate, String)>,
    pub(super) candidate_first_runtime_floor_hit_without_l3: Option<(NaiveDate, String)>,
}

pub(super) struct FocusRuntimeConfig<'a> {
    pub(super) baseline_use_transitional_bridge: bool,
    pub(super) candidate_use_transitional_bridge: bool,
    pub(super) baseline_runtime_thresholds: Option<&'a crate::RuntimeThresholdDiagnosticsWire>,
    pub(super) candidate_runtime_thresholds: Option<&'a crate::RuntimeThresholdDiagnosticsWire>,
}

pub(super) fn prepare_focus_window<'a>(
    baseline: &'a BacktestScenarioSummary,
    candidate: Option<&'a BacktestScenarioSummary>,
    baseline_history: &'a [AssessmentHistoryPoint],
    candidate_history: &'a [AssessmentHistoryPoint],
    config: &FocusRuntimeConfig<'a>,
) -> PreparedScenarioWindow<'a> {
    let window_start = baseline.crisis_start - Duration::days(90);
    let window_end = baseline.crisis_end;
    let baseline_window_points = select_window_points(baseline_history, window_start, window_end);
    let candidate_window_points = select_window_points(candidate_history, window_start, window_end);
    let baseline_pre_crisis_points =
        select_pre_crisis_points(&baseline_window_points, baseline.crisis_start);
    let candidate_pre_crisis_points =
        select_pre_crisis_points(&candidate_window_points, baseline.crisis_start);

    PreparedScenarioWindow {
        baseline,
        candidate,
        window_start,
        window_end,
        baseline_first_non_normal_date: release_review_first_non_normal_date(
            &baseline_window_points,
        ),
        candidate_first_non_normal_date: release_review_first_non_normal_date(
            &candidate_window_points,
        ),
        baseline_runtime_floor_hit_point_count: baseline_pre_crisis_points
            .iter()
            .filter(|point| {
                release_review_hits_runtime_floor(point, config.baseline_runtime_thresholds)
            })
            .count() as u32,
        candidate_runtime_floor_hit_point_count: candidate_pre_crisis_points
            .iter()
            .filter(|point| {
                release_review_hits_runtime_floor(point, config.candidate_runtime_thresholds)
            })
            .count() as u32,
        baseline_actionable_hits: release_review_actionable_forward_hits_by_date(
            &baseline_pre_crisis_points,
            config.baseline_use_transitional_bridge,
        ),
        candidate_actionable_hits: release_review_actionable_forward_hits_by_date(
            &candidate_pre_crisis_points,
            config.candidate_use_transitional_bridge,
        ),
        baseline_first_runtime_floor_hit_without_l3:
            release_review_first_runtime_floor_hit_without_l3(
                &baseline_pre_crisis_points,
                config.baseline_use_transitional_bridge,
                config.baseline_runtime_thresholds,
            ),
        candidate_first_runtime_floor_hit_without_l3:
            release_review_first_runtime_floor_hit_without_l3(
                &candidate_pre_crisis_points,
                config.candidate_use_transitional_bridge,
                config.candidate_runtime_thresholds,
            ),
        baseline_window_points,
        candidate_window_points,
        baseline_pre_crisis_points,
        candidate_pre_crisis_points,
    }
}

fn select_window_points(
    history: &[AssessmentHistoryPoint],
    window_start: NaiveDate,
    window_end: NaiveDate,
) -> Vec<&AssessmentHistoryPoint> {
    let mut points = history
        .iter()
        .filter(|point| point.as_of_date >= window_start && point.as_of_date <= window_end)
        .collect::<Vec<_>>();
    points.sort_by_key(|point| point.as_of_date);
    points
}

fn select_pre_crisis_points<'a>(
    points: &[&'a AssessmentHistoryPoint],
    crisis_start: NaiveDate,
) -> Vec<&'a AssessmentHistoryPoint> {
    let mut pre_crisis_points = points
        .iter()
        .copied()
        .filter(|point| point.as_of_date < crisis_start)
        .collect::<Vec<_>>();
    pre_crisis_points.sort_by_key(|point| point.as_of_date);
    pre_crisis_points
}

fn release_review_first_non_normal_date(points: &[&AssessmentHistoryPoint]) -> Option<NaiveDate> {
    points
        .iter()
        .find(|point| release_review_point_is_non_normal(point))
        .map(|point| point.as_of_date)
}

fn release_review_first_runtime_floor_hit_without_l3(
    points: &[&AssessmentHistoryPoint],
    use_transitional_bridge: bool,
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> Option<(NaiveDate, String)> {
    points.iter().find_map(|point| {
        release_review_runtime_actionable_block_reason(point, use_transitional_bridge, thresholds)
            .map(|reason| (point.as_of_date, reason))
    })
}

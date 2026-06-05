use std::collections::{BTreeMap, BTreeSet};

use chrono::{Duration, NaiveDate};
use fc_domain::{AssessmentHistoryPoint, BacktestScenarioSummary};

use super::{
    blocks::{
        release_review_actionable_diagnostic, release_review_posture_name,
        release_review_primary_failure_mode, release_review_runtime_actionable_block_category,
        release_review_runtime_actionable_block_reason, release_review_runtime_block_counts,
        release_review_runtime_continuity_facet_counts, release_review_runtime_dominant_categories,
        release_review_time_bucket_name,
    },
    signals::{
        release_review_actionable_forward_hits_by_date, release_review_hits_runtime_floor,
        release_review_is_actionable_warning_point,
        release_review_uses_transitional_actionable_bridge,
    },
};

use super::super::backtest::{backtest_warning_state, scenario_requires_focus_review};

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

    let mut rows = baseline_backtests
        .iter()
        .filter_map(|baseline| {
            let candidate = candidate_by_id.get(baseline.scenario_id.as_str()).copied();
            if !scenario_requires_focus_review(baseline, candidate) {
                return None;
            }

            let window_start = baseline.crisis_start - Duration::days(90);
            let window_end = baseline.crisis_end;
            let mut baseline_window_points = baseline_history
                .iter()
                .filter(|point| point.as_of_date >= window_start && point.as_of_date <= window_end)
                .collect::<Vec<_>>();
            let mut candidate_window_points = candidate_history
                .iter()
                .filter(|point| point.as_of_date >= window_start && point.as_of_date <= window_end)
                .collect::<Vec<_>>();
            baseline_window_points.sort_by_key(|point| point.as_of_date);
            candidate_window_points.sort_by_key(|point| point.as_of_date);
            let mut baseline_pre_crisis_points = baseline_window_points
                .iter()
                .copied()
                .filter(|point| point.as_of_date < baseline.crisis_start)
                .collect::<Vec<_>>();
            let mut candidate_pre_crisis_points = candidate_window_points
                .iter()
                .copied()
                .filter(|point| point.as_of_date < baseline.crisis_start)
                .collect::<Vec<_>>();
            baseline_pre_crisis_points.sort_by_key(|point| point.as_of_date);
            candidate_pre_crisis_points.sort_by_key(|point| point.as_of_date);
            let baseline_first_non_normal_date =
                release_review_first_non_normal_date(&baseline_window_points);
            let candidate_first_non_normal_date =
                release_review_first_non_normal_date(&candidate_window_points);
            let baseline_runtime_floor_hit_point_count = baseline_pre_crisis_points
                .iter()
                .filter(|point| {
                    release_review_hits_runtime_floor(point, baseline_runtime_thresholds)
                })
                .count() as u32;
            let candidate_runtime_floor_hit_point_count = candidate_pre_crisis_points
                .iter()
                .filter(|point| {
                    release_review_hits_runtime_floor(point, candidate_runtime_thresholds)
                })
                .count() as u32;
            let baseline_actionable_hits = release_review_actionable_forward_hits_by_date(
                &baseline_pre_crisis_points,
                baseline_use_transitional_bridge,
            );
            let candidate_actionable_hits = release_review_actionable_forward_hits_by_date(
                &candidate_pre_crisis_points,
                candidate_use_transitional_bridge,
            );
            let baseline_first_runtime_floor_hit_without_l3 =
                release_review_first_runtime_floor_hit_without_l3(
                    &baseline_pre_crisis_points,
                    baseline_use_transitional_bridge,
                    baseline_runtime_thresholds,
                );
            let candidate_first_runtime_floor_hit_without_l3 =
                release_review_first_runtime_floor_hit_without_l3(
                    &candidate_pre_crisis_points,
                    candidate_use_transitional_bridge,
                    candidate_runtime_thresholds,
                );

            let mut interesting_dates = BTreeSet::new();
            for date in [
                Some(baseline.crisis_start),
                Some(baseline.crisis_end),
                baseline.first_l2_date,
                candidate.and_then(|scenario| scenario.first_l2_date),
                baseline.first_l3_date,
                candidate.and_then(|scenario| scenario.first_l3_date),
                baseline_first_non_normal_date,
                candidate_first_non_normal_date,
                baseline_first_runtime_floor_hit_without_l3
                    .as_ref()
                    .map(|(date, _)| *date),
                candidate_first_runtime_floor_hit_without_l3
                    .as_ref()
                    .map(|(date, _)| *date),
            ]
            .into_iter()
            .flatten()
            {
                if date >= window_start && date <= window_end {
                    interesting_dates.insert(date);
                }
            }

            for date in baseline_window_points
                .iter()
                .map(|point| point.as_of_date)
                .chain(candidate_window_points.iter().map(|point| point.as_of_date))
            {
                let baseline_point = baseline_points_by_date.get(&date).copied();
                let candidate_point = candidate_points_by_date.get(&date).copied();
                if release_review_point_is_interesting(
                    baseline_point,
                    candidate_point,
                    baseline_use_transitional_bridge,
                    candidate_use_transitional_bridge,
                ) {
                    interesting_dates.insert(date);
                }
            }

            let interesting_points = interesting_dates
                .into_iter()
                .filter_map(|date| {
                    let baseline_point = baseline_points_by_date.get(&date).copied();
                    let candidate_point = candidate_points_by_date.get(&date).copied();
                    if baseline_point.is_none() && candidate_point.is_none() {
                        return None;
                    }
                    let baseline_strict_review_actionable = baseline_point.is_some_and(|point| {
                        release_review_is_actionable_warning_point(
                            point,
                            baseline_use_transitional_bridge,
                        )
                    });
                    let candidate_strict_review_actionable = candidate_point.is_some_and(|point| {
                        release_review_is_actionable_warning_point(
                            point,
                            candidate_use_transitional_bridge,
                        )
                    });
                    let baseline_runtime_floor_hit = baseline_point.is_some_and(|point| {
                        release_review_hits_runtime_floor(point, baseline_runtime_thresholds)
                    });
                    let candidate_runtime_floor_hit = candidate_point.is_some_and(|point| {
                        release_review_hits_runtime_floor(point, candidate_runtime_thresholds)
                    });
                    Some(crate::ReleaseReviewScenarioPointComparison {
                        as_of_date: date,
                        baseline_p20d: baseline_point.map(|point| point.p_20d),
                        candidate_p20d: candidate_point.map(|point| point.p_20d),
                        baseline_p60d: baseline_point.map(|point| point.p_60d),
                        candidate_p60d: candidate_point.map(|point| point.p_60d),
                        baseline_posture: baseline_point
                            .map(release_review_posture_name)
                            .map(str::to_string),
                        candidate_posture: candidate_point
                            .map(release_review_posture_name)
                            .map(str::to_string),
                        baseline_time_bucket: baseline_point
                            .map(release_review_time_bucket_name)
                            .map(str::to_string),
                        candidate_time_bucket: candidate_point
                            .map(release_review_time_bucket_name)
                            .map(str::to_string),
                        baseline_strict_review_actionable,
                        candidate_strict_review_actionable,
                        baseline_runtime_floor_hit,
                        candidate_runtime_floor_hit,
                        baseline_actionable: baseline_strict_review_actionable,
                        candidate_actionable: candidate_strict_review_actionable,
                        baseline_actionable_forward_5d_hits: baseline_actionable_hits
                            .get(&date)
                            .map(|(hit_count, _)| *hit_count),
                        candidate_actionable_forward_5d_hits: candidate_actionable_hits
                            .get(&date)
                            .map(|(hit_count, _)| *hit_count),
                        baseline_actionable_sustained: baseline_actionable_hits
                            .get(&date)
                            .map(|(_, sustained)| *sustained),
                        candidate_actionable_sustained: candidate_actionable_hits
                            .get(&date)
                            .map(|(_, sustained)| *sustained),
                        baseline_trigger_codes: baseline_point
                            .map(|point| point.posture_trigger_codes.clone())
                            .unwrap_or_default(),
                        candidate_trigger_codes: candidate_point
                            .map(|point| point.posture_trigger_codes.clone())
                            .unwrap_or_default(),
                        baseline_runtime_actionable_block_category: baseline_point.and_then(
                            |point| {
                                release_review_runtime_actionable_block_category(
                                    point,
                                    baseline_use_transitional_bridge,
                                    baseline_runtime_thresholds,
                                )
                                .map(str::to_string)
                            },
                        ),
                        candidate_runtime_actionable_block_category: candidate_point.and_then(
                            |point| {
                                release_review_runtime_actionable_block_category(
                                    point,
                                    candidate_use_transitional_bridge,
                                    candidate_runtime_thresholds,
                                )
                                .map(str::to_string)
                            },
                        ),
                        baseline_runtime_actionable_block_reason: baseline_point.and_then(
                            |point| {
                                release_review_runtime_actionable_block_reason(
                                    point,
                                    baseline_use_transitional_bridge,
                                    baseline_runtime_thresholds,
                                )
                            },
                        ),
                        candidate_runtime_actionable_block_reason: candidate_point.and_then(
                            |point| {
                                release_review_runtime_actionable_block_reason(
                                    point,
                                    candidate_use_transitional_bridge,
                                    candidate_runtime_thresholds,
                                )
                            },
                        ),
                        baseline_actionable_diagnostic: baseline_point.map(|point| {
                            release_review_actionable_diagnostic(
                                point,
                                baseline_use_transitional_bridge,
                                baseline_runtime_thresholds,
                            )
                        }),
                        candidate_actionable_diagnostic: candidate_point.map(|point| {
                            release_review_actionable_diagnostic(
                                point,
                                candidate_use_transitional_bridge,
                                candidate_runtime_thresholds,
                            )
                        }),
                    })
                })
                .collect::<Vec<_>>();

            let runtime_block_counts = release_review_runtime_block_counts(
                &baseline_pre_crisis_points,
                baseline_use_transitional_bridge,
                baseline_runtime_thresholds,
                &candidate_pre_crisis_points,
                candidate_use_transitional_bridge,
                candidate_runtime_thresholds,
            );
            let runtime_continuity_facet_counts = release_review_runtime_continuity_facet_counts(
                &baseline_pre_crisis_points,
                baseline_use_transitional_bridge,
                baseline_runtime_thresholds,
                &candidate_pre_crisis_points,
                candidate_use_transitional_bridge,
                candidate_runtime_thresholds,
            );
            let dominant_runtime_blocks =
                release_review_runtime_dominant_categories(&runtime_block_counts);
            let dominant_runtime_continuity_facets =
                release_review_runtime_dominant_categories(&runtime_continuity_facet_counts);

            Some(crate::ReleaseReviewScenarioFocusDiagnostic {
                baseline_primary_failure_mode: release_review_primary_failure_mode(
                    &dominant_runtime_blocks.baseline_categories,
                    dominant_runtime_blocks.baseline_count,
                    &dominant_runtime_continuity_facets.baseline_categories,
                    dominant_runtime_continuity_facets.baseline_count,
                ),
                candidate_primary_failure_mode: release_review_primary_failure_mode(
                    &dominant_runtime_blocks.candidate_categories,
                    dominant_runtime_blocks.candidate_count,
                    &dominant_runtime_continuity_facets.candidate_categories,
                    dominant_runtime_continuity_facets.candidate_count,
                ),
                dominant_runtime_blocks,
                dominant_runtime_continuity_facets,
                scenario_id: baseline.scenario_id.clone(),
                name: baseline.name.clone(),
                outcome: format!(
                    "{}_to_{}",
                    backtest_warning_state(baseline.actionable_lead_time_days),
                    backtest_warning_state(
                        candidate.and_then(|scenario| scenario.actionable_lead_time_days)
                    )
                ),
                window_start,
                window_end,
                crisis_start: baseline.crisis_start,
                crisis_end: baseline.crisis_end,
                baseline_first_l2_date: baseline.first_l2_date,
                candidate_first_l2_date: candidate.and_then(|scenario| scenario.first_l2_date),
                baseline_first_l3_date: baseline.first_l3_date,
                candidate_first_l3_date: candidate.and_then(|scenario| scenario.first_l3_date),
                baseline_first_non_normal_date,
                candidate_first_non_normal_date,
                baseline_actionable_point_count: baseline_pre_crisis_points
                    .iter()
                    .filter(|point| {
                        release_review_is_actionable_warning_point(
                            point,
                            baseline_use_transitional_bridge,
                        )
                    })
                    .count() as u32,
                candidate_actionable_point_count: candidate_pre_crisis_points
                    .iter()
                    .filter(|point| {
                        release_review_is_actionable_warning_point(
                            point,
                            candidate_use_transitional_bridge,
                        )
                    })
                    .count() as u32,
                baseline_runtime_floor_hit_point_count,
                candidate_runtime_floor_hit_point_count,
                baseline_max_p20d: release_review_max_metric(
                    &baseline_pre_crisis_points,
                    |point| point.p_20d,
                ),
                candidate_max_p20d: release_review_max_metric(
                    &candidate_pre_crisis_points,
                    |point| point.p_20d,
                ),
                baseline_max_p60d: release_review_max_metric(
                    &baseline_pre_crisis_points,
                    |point| point.p_60d,
                ),
                candidate_max_p60d: release_review_max_metric(
                    &candidate_pre_crisis_points,
                    |point| point.p_60d,
                ),
                baseline_first_runtime_floor_hit_without_l3_date:
                    baseline_first_runtime_floor_hit_without_l3
                        .as_ref()
                        .map(|(date, _)| *date),
                candidate_first_runtime_floor_hit_without_l3_date:
                    candidate_first_runtime_floor_hit_without_l3
                        .as_ref()
                        .map(|(date, _)| *date),
                baseline_first_runtime_floor_hit_without_l3_reason:
                    baseline_first_runtime_floor_hit_without_l3
                        .as_ref()
                        .map(|(_, reason)| reason.clone()),
                candidate_first_runtime_floor_hit_without_l3_reason:
                    candidate_first_runtime_floor_hit_without_l3
                        .as_ref()
                        .map(|(_, reason)| reason.clone()),
                runtime_block_counts,
                runtime_continuity_facet_counts,
                interesting_points,
            })
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));
    rows
}

fn release_review_first_non_normal_date(points: &[&AssessmentHistoryPoint]) -> Option<NaiveDate> {
    points
        .iter()
        .find(|point| release_review_point_is_non_normal(point))
        .map(|point| point.as_of_date)
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

fn release_review_point_is_non_normal(point: &AssessmentHistoryPoint) -> bool {
    !matches!(point.posture, fc_domain::DecisionPosture::Normal)
        || !matches!(
            point.time_to_risk_bucket,
            fc_domain::TimeToRiskBucket::Normal
        )
}

fn release_review_point_is_interesting(
    baseline_point: Option<&AssessmentHistoryPoint>,
    candidate_point: Option<&AssessmentHistoryPoint>,
    baseline_use_transitional_bridge: bool,
    candidate_use_transitional_bridge: bool,
) -> bool {
    let baseline_actionable = baseline_point.is_some_and(|point| {
        release_review_is_actionable_warning_point(point, baseline_use_transitional_bridge)
    });
    let candidate_actionable = candidate_point.is_some_and(|point| {
        release_review_is_actionable_warning_point(point, candidate_use_transitional_bridge)
    });
    if baseline_actionable
        || candidate_actionable
        || baseline_point.is_some_and(release_review_point_is_non_normal)
        || candidate_point.is_some_and(release_review_point_is_non_normal)
    {
        return true;
    }

    match (baseline_point, candidate_point) {
        (Some(baseline), Some(candidate)) => {
            baseline.posture != candidate.posture
                || baseline.time_to_risk_bucket != candidate.time_to_risk_bucket
                || baseline.posture_trigger_codes != candidate.posture_trigger_codes
                || (baseline.p_20d - candidate.p_20d).abs() >= 0.05
                || (baseline.p_60d - candidate.p_60d).abs() >= 0.05
        }
        _ => false,
    }
}

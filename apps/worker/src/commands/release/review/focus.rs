use std::collections::{BTreeMap, BTreeSet};

use chrono::{Duration, NaiveDate};
use fc_domain::{
    AssessmentHistoryPoint, BacktestScenarioSummary, DecisionPosture, TimeToRiskBucket,
};

const RELEASE_REVIEW_SIGNAL_WINDOW: usize = 5;
const RELEASE_REVIEW_SIGNAL_MIN_HITS: usize = 3;

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
        .filter(|point| release_review_is_actionable_warning_point(point, use_transitional_bridge))
        .count() as u32;
    let runtime_floor_hit_count = history
        .iter()
        .filter(|point| in_any_pre_crisis_window(point))
        .filter(|point| release_review_hits_runtime_floor(point, thresholds))
        .count() as u32;
    (strict_actionable_point_count, runtime_floor_hit_count)
}

pub(crate) fn build_release_review_backtest_scenario_comparisons(
    baseline_backtests: &[BacktestScenarioSummary],
    candidate_backtests: &[BacktestScenarioSummary],
) -> Vec<crate::ReleaseReviewBacktestScenarioComparison> {
    let candidate_by_id = candidate_backtests
        .iter()
        .map(|scenario| (scenario.scenario_id.as_str(), scenario))
        .collect::<BTreeMap<_, _>>();

    let mut rows = baseline_backtests
        .iter()
        .map(|baseline| {
            let candidate = candidate_by_id.get(baseline.scenario_id.as_str()).copied();
            let candidate_lead_time_days = candidate.and_then(|scenario| scenario.lead_time_days);
            let candidate_actionable_lead_time_days =
                candidate.and_then(|scenario| scenario.actionable_lead_time_days);
            let candidate_false_positive_count = candidate
                .map(|scenario| scenario.false_positive_count)
                .unwrap_or_default();
            let candidate_first_l2_date = candidate.and_then(|scenario| scenario.first_l2_date);
            let candidate_first_l3_date = candidate.and_then(|scenario| scenario.first_l3_date);
            crate::ReleaseReviewBacktestScenarioComparison {
                scenario_id: baseline.scenario_id.clone(),
                name: baseline.name.clone(),
                signal_source: match baseline.signal_source {
                    fc_domain::BacktestSignalSource::RealHistory => "real_history",
                    fc_domain::BacktestSignalSource::FallbackTemplate => "fallback_template",
                }
                .to_string(),
                crisis_start: baseline.crisis_start,
                crisis_end: baseline.crisis_end,
                baseline_first_l2_date: baseline.first_l2_date,
                candidate_first_l2_date,
                baseline_first_l3_date: baseline.first_l3_date,
                candidate_first_l3_date,
                baseline_lead_time_days: baseline.lead_time_days,
                candidate_lead_time_days,
                baseline_actionable_lead_time_days: baseline.actionable_lead_time_days,
                candidate_actionable_lead_time_days,
                baseline_false_positive_count: baseline.false_positive_count,
                candidate_false_positive_count,
                actionable_delta_days: match (
                    baseline.actionable_lead_time_days,
                    candidate_actionable_lead_time_days,
                ) {
                    (Some(baseline_days), Some(candidate_days)) => {
                        Some(candidate_days - baseline_days)
                    }
                    _ => None,
                },
                outcome: format!(
                    "{}_to_{}",
                    backtest_warning_state(baseline.actionable_lead_time_days),
                    backtest_warning_state(candidate_actionable_lead_time_days)
                ),
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));
    rows
}

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

fn scenario_requires_focus_review(
    baseline: &BacktestScenarioSummary,
    candidate: Option<&BacktestScenarioSummary>,
) -> bool {
    baseline.first_l2_date != candidate.and_then(|scenario| scenario.first_l2_date)
        || baseline.first_l3_date != candidate.and_then(|scenario| scenario.first_l3_date)
        || baseline.lead_time_days != candidate.and_then(|scenario| scenario.lead_time_days)
        || baseline.actionable_lead_time_days
            != candidate.and_then(|scenario| scenario.actionable_lead_time_days)
        || baseline.false_positive_count
            != candidate
                .map(|scenario| scenario.false_positive_count)
                .unwrap_or_default()
        || scenario_has_structural_warning_without_actionable(baseline)
        || candidate.is_some_and(scenario_has_structural_warning_without_actionable)
}

fn scenario_has_structural_warning_without_actionable(scenario: &BacktestScenarioSummary) -> bool {
    scenario.lead_time_days.is_some() && scenario.actionable_lead_time_days.is_none()
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

fn release_review_hits_runtime_floor(
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

fn release_review_actionable_diagnostic(
    point: &AssessmentHistoryPoint,
    use_transitional_bridge: bool,
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> String {
    if release_review_is_actionable_warning_point(point, use_transitional_bridge) {
        return "actionable".to_string();
    }

    let runtime_floor_hit = release_review_hits_runtime_floor(point, thresholds);
    let mut review_gate_gaps = Vec::new();
    if point.p_20d < 0.18 {
        review_gate_gaps.push(format!("p20d {} < 18%", crate::format_pct(point.p_20d)));
    }
    if point.p_60d < 0.45 {
        review_gate_gaps.push(format!("p60d {} < 45%", crate::format_pct(point.p_60d)));
    }
    if !review_gate_gaps.is_empty() {
        let joined = review_gate_gaps.join(", ");
        return if runtime_floor_hit {
            format!("hit runtime floor, but review gate still needs {joined}")
        } else {
            joined
        };
    }

    if matches!(point.posture, DecisionPosture::Normal)
        && matches!(point.time_to_risk_bucket, TimeToRiskBucket::Normal)
    {
        return if runtime_floor_hit {
            "hit runtime floor, but posture/bucket stayed normal".to_string()
        } else {
            "posture/bucket stayed normal".to_string()
        };
    }

    if matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
        && point.overall_score < 62.0
        && point.external_shock_score < 48.0
    {
        return "months setup lacked score confirmation".to_string();
    }

    if matches!(point.posture, DecisionPosture::Prepare)
        && point.overall_score < 60.0
        && point.external_shock_score < 46.0
        && !release_review_has_strong_prepare_trigger_code(point)
    {
        return "prepare setup lacked confirmation".to_string();
    }

    if use_transitional_bridge
        && matches!(point.posture, DecisionPosture::Prepare)
        && point.overall_score < 58.0
    {
        return "prepare bridge not armed".to_string();
    }

    if use_transitional_bridge
        && matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
        && point.overall_score < 58.0
    {
        return "months bridge not armed".to_string();
    }

    "review L3 gate not satisfied".to_string()
}

fn release_review_runtime_actionable_block_category(
    point: &AssessmentHistoryPoint,
    use_transitional_bridge: bool,
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> Option<&'static str> {
    if release_review_is_actionable_warning_point(point, use_transitional_bridge)
        || !release_review_hits_runtime_floor(point, thresholds)
    {
        return None;
    }

    if point.p_20d < 0.18 || point.p_60d < 0.45 {
        return Some("review_gate_gap");
    }

    if matches!(point.posture, DecisionPosture::Normal)
        && matches!(point.time_to_risk_bucket, TimeToRiskBucket::Normal)
    {
        return Some("posture_bucket_normal");
    }

    if matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
        && point.overall_score < 62.0
        && point.external_shock_score < 48.0
    {
        return Some("months_score_confirmation");
    }

    if matches!(point.posture, DecisionPosture::Prepare)
        && point.overall_score < 60.0
        && point.external_shock_score < 46.0
        && !release_review_has_strong_prepare_trigger_code(point)
    {
        return Some("prepare_score_confirmation");
    }

    if use_transitional_bridge
        && matches!(point.posture, DecisionPosture::Prepare)
        && point.overall_score < 58.0
    {
        return Some("prepare_bridge_not_armed");
    }

    if use_transitional_bridge
        && matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
        && point.overall_score < 58.0
    {
        return Some("months_bridge_not_armed");
    }

    Some("review_l3_gate_not_satisfied")
}

fn release_review_runtime_actionable_block_reason(
    point: &AssessmentHistoryPoint,
    use_transitional_bridge: bool,
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> Option<String> {
    release_review_runtime_actionable_block_category(point, use_transitional_bridge, thresholds)
        .map(|_| release_review_actionable_diagnostic(point, use_transitional_bridge, thresholds))
}

fn release_review_runtime_gate_gap_facet(point: &AssessmentHistoryPoint) -> &'static str {
    match (point.p_20d < 0.18, point.p_60d < 0.45) {
        (true, true) => "p20d_and_p60d",
        (true, false) => "p20d_only",
        (false, true) => "p60d_only",
        (false, false) => "none",
    }
}

fn release_review_trigger_family(point: &AssessmentHistoryPoint) -> &'static str {
    if point.posture_trigger_codes.is_empty() {
        return "none";
    }

    let mut has_prepare = false;
    let mut has_hedge = false;
    let mut has_defend = false;
    let mut has_other = false;
    for code in &point.posture_trigger_codes {
        if code.starts_with("prepare_") {
            has_prepare = true;
        } else if code.starts_with("hedge_") {
            has_hedge = true;
        } else if code.starts_with("defend_") {
            has_defend = true;
        } else {
            has_other = true;
        }
    }

    let family_count = [has_prepare, has_hedge, has_defend, has_other]
        .into_iter()
        .filter(|present| *present)
        .count();
    if family_count > 1 {
        return "mixed";
    }
    if has_prepare {
        "prepare"
    } else if has_hedge {
        "hedge"
    } else if has_defend {
        "defend"
    } else {
        "other"
    }
}

fn release_review_runtime_confirmation_facet(
    point: &AssessmentHistoryPoint,
    use_transitional_bridge: bool,
) -> &'static str {
    if matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
        && point.overall_score < 62.0
        && point.external_shock_score < 48.0
    {
        return "months_score_low";
    }

    if matches!(point.posture, DecisionPosture::Prepare)
        && point.overall_score < 60.0
        && point.external_shock_score < 46.0
        && !release_review_has_strong_prepare_trigger_code(point)
    {
        return "prepare_score_low";
    }

    if use_transitional_bridge
        && matches!(point.posture, DecisionPosture::Prepare)
        && point.overall_score < 58.0
    {
        return "prepare_bridge_low";
    }

    if use_transitional_bridge
        && matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
        && point.overall_score < 58.0
    {
        return "months_bridge_low";
    }

    "ok_or_not_needed"
}

fn release_review_runtime_continuity_facets(
    point: &AssessmentHistoryPoint,
    use_transitional_bridge: bool,
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> Vec<String> {
    if release_review_runtime_actionable_block_category(point, use_transitional_bridge, thresholds)
        .is_none()
    {
        return Vec::new();
    }

    vec![
        format!("posture:{}", release_review_posture_name(point)),
        format!("bucket:{}", release_review_time_bucket_name(point)),
        format!("trigger:{}", release_review_trigger_family(point)),
        format!("gate_gap:{}", release_review_runtime_gate_gap_facet(point)),
        format!(
            "confirmation:{}",
            release_review_runtime_confirmation_facet(point, use_transitional_bridge)
        ),
    ]
}

fn release_review_runtime_block_counts(
    baseline_points: &[&AssessmentHistoryPoint],
    baseline_use_transitional_bridge: bool,
    baseline_thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
    candidate_points: &[&AssessmentHistoryPoint],
    candidate_use_transitional_bridge: bool,
    candidate_thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> Vec<crate::ReleaseReviewRuntimeBlockCount> {
    let collect_counts =
        |points: &[&AssessmentHistoryPoint],
         use_transitional_bridge: bool,
         thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>| {
            points
                .iter()
                .fold(BTreeMap::<String, u32>::new(), |mut acc, point| {
                    if let Some(category) = release_review_runtime_actionable_block_category(
                        point,
                        use_transitional_bridge,
                        thresholds,
                    ) {
                        *acc.entry(category.to_string()).or_default() += 1;
                    }
                    acc
                })
        };

    let baseline_counts = collect_counts(
        baseline_points,
        baseline_use_transitional_bridge,
        baseline_thresholds,
    );
    let candidate_counts = collect_counts(
        candidate_points,
        candidate_use_transitional_bridge,
        candidate_thresholds,
    );
    let categories = baseline_counts
        .keys()
        .chain(candidate_counts.keys())
        .cloned()
        .collect::<BTreeSet<_>>();

    categories
        .into_iter()
        .map(|category| {
            let baseline_count = baseline_counts.get(&category).copied().unwrap_or_default();
            let candidate_count = candidate_counts.get(&category).copied().unwrap_or_default();
            crate::ReleaseReviewRuntimeBlockCount {
                category,
                baseline_count,
                candidate_count,
                delta: i64::from(candidate_count) - i64::from(baseline_count),
            }
        })
        .collect()
}

fn release_review_runtime_dominant_categories(
    counts: &[crate::ReleaseReviewRuntimeBlockCount],
) -> crate::ReleaseReviewRuntimeDominantCategories {
    let baseline_count = counts
        .iter()
        .map(|row| row.baseline_count)
        .max()
        .unwrap_or(0);
    let candidate_count = counts
        .iter()
        .map(|row| row.candidate_count)
        .max()
        .unwrap_or(0);

    crate::ReleaseReviewRuntimeDominantCategories {
        baseline_categories: if baseline_count == 0 {
            Vec::new()
        } else {
            counts
                .iter()
                .filter(|row| row.baseline_count == baseline_count)
                .map(|row| row.category.clone())
                .collect()
        },
        baseline_count,
        candidate_categories: if candidate_count == 0 {
            Vec::new()
        } else {
            counts
                .iter()
                .filter(|row| row.candidate_count == candidate_count)
                .map(|row| row.category.clone())
                .collect()
        },
        candidate_count,
    }
}

fn release_review_primary_failure_mode(
    dominant_blocks: &[String],
    dominant_block_count: u32,
    dominant_facets: &[String],
    dominant_facet_count: u32,
) -> Option<String> {
    if dominant_block_count > 0 {
        if dominant_blocks
            .iter()
            .any(|category| category == "review_gate_gap")
        {
            return Some("strict_gate_mismatch".to_string());
        }
        if dominant_blocks
            .iter()
            .any(|category| category == "posture_bucket_normal")
        {
            return Some("posture_continuity_failure".to_string());
        }
        if dominant_blocks
            .iter()
            .any(|category| category.ends_with("score_confirmation"))
        {
            return Some("score_confirmation_failure".to_string());
        }
        if dominant_blocks
            .iter()
            .any(|category| category.ends_with("bridge_not_armed"))
        {
            return Some("transitional_bridge_failure".to_string());
        }
        return Some("residual_review_l3_failure".to_string());
    }

    if dominant_facet_count > 0 {
        if dominant_facets
            .iter()
            .any(|facet| facet == "posture:normal")
            || dominant_facets.iter().any(|facet| facet == "bucket:normal")
            || dominant_facets.iter().any(|facet| facet == "trigger:none")
        {
            return Some("posture_continuity_failure".to_string());
        }
        if dominant_facets
            .iter()
            .any(|facet| facet.starts_with("gate_gap:") && facet != "gate_gap:none")
        {
            return Some("strict_gate_mismatch".to_string());
        }
        if dominant_facets.iter().any(|facet| {
            facet.starts_with("confirmation:") && facet != "confirmation:ok_or_not_needed"
        }) {
            return Some("score_confirmation_failure".to_string());
        }
        return Some("runtime_continuity_failure".to_string());
    }

    None
}

fn release_review_runtime_continuity_facet_counts(
    baseline_points: &[&AssessmentHistoryPoint],
    baseline_use_transitional_bridge: bool,
    baseline_thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
    candidate_points: &[&AssessmentHistoryPoint],
    candidate_use_transitional_bridge: bool,
    candidate_thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> Vec<crate::ReleaseReviewRuntimeBlockCount> {
    let collect_counts =
        |points: &[&AssessmentHistoryPoint],
         use_transitional_bridge: bool,
         thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>| {
            points
                .iter()
                .fold(BTreeMap::<String, u32>::new(), |mut acc, point| {
                    for facet in release_review_runtime_continuity_facets(
                        point,
                        use_transitional_bridge,
                        thresholds,
                    ) {
                        *acc.entry(facet).or_default() += 1;
                    }
                    acc
                })
        };

    let baseline_counts = collect_counts(
        baseline_points,
        baseline_use_transitional_bridge,
        baseline_thresholds,
    );
    let candidate_counts = collect_counts(
        candidate_points,
        candidate_use_transitional_bridge,
        candidate_thresholds,
    );
    let categories = baseline_counts
        .keys()
        .chain(candidate_counts.keys())
        .cloned()
        .collect::<BTreeSet<_>>();

    categories
        .into_iter()
        .map(|category| {
            let baseline_count = baseline_counts.get(&category).copied().unwrap_or_default();
            let candidate_count = candidate_counts.get(&category).copied().unwrap_or_default();
            crate::ReleaseReviewRuntimeBlockCount {
                category,
                baseline_count,
                candidate_count,
                delta: i64::from(candidate_count) - i64::from(baseline_count),
            }
        })
        .collect()
}

fn release_review_actionable_forward_hits_by_date(
    points: &[&AssessmentHistoryPoint],
    use_transitional_bridge: bool,
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
                    release_review_is_actionable_warning_point(candidate, use_transitional_bridge)
                })
                .count();
            let required_hits = RELEASE_REVIEW_SIGNAL_MIN_HITS.min(window.len());
            let sustained =
                release_review_is_actionable_warning_point(point, use_transitional_bridge)
                    && hit_count >= required_hits;
            (point.as_of_date, (hit_count as u32, sustained))
        })
        .collect()
}

fn release_review_point_is_non_normal(point: &AssessmentHistoryPoint) -> bool {
    !matches!(point.posture, DecisionPosture::Normal)
        || !matches!(point.time_to_risk_bucket, TimeToRiskBucket::Normal)
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

fn release_review_uses_transitional_actionable_bridge(
    method: &crate::AuditMethodResponseWire,
) -> bool {
    !(method.method.probability_mode == "formal_bundle_v1"
        && method.method.label_version == "formal_label_v1_main"
        && method
            .method
            .feature_set_version
            .starts_with("feature_formal_v1_main"))
}

fn release_review_has_strong_prepare_trigger_code(point: &AssessmentHistoryPoint) -> bool {
    point.posture_trigger_codes.iter().any(|code| {
        matches!(
            code.as_str(),
            "prepare_p60d_structural"
                | "prepare_structural_downgrade"
                | "prepare_carry_structural"
                | "prepare_external_structural"
        )
    })
}

fn release_review_is_actionable_warning_point(
    point: &AssessmentHistoryPoint,
    use_transitional_bridge: bool,
) -> bool {
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
        && point.p_20d >= 0.18
        && point.p_60d >= 0.45
        && ((point.overall_score >= 60.0 && point.external_shock_score >= 46.0)
            || (point.overall_score >= 53.0
                && !matches!(point.time_to_risk_bucket, TimeToRiskBucket::Normal)
                && release_review_has_strong_prepare_trigger_code(point)));
    let high_probability_months_signal =
        matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
            && point.overall_score >= 62.0
            && point.p_20d >= 0.18
            && point.p_60d >= 0.45
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
        || high_probability_months_signal
        || prepare_bridge_signal
        || months_bridge_signal
}

fn release_review_posture_name(point: &AssessmentHistoryPoint) -> &'static str {
    match point.posture {
        DecisionPosture::Normal => "normal",
        DecisionPosture::Prepare => "prepare",
        DecisionPosture::Hedge => "hedge",
        DecisionPosture::Defend => "defend",
    }
}

fn release_review_time_bucket_name(point: &AssessmentHistoryPoint) -> &'static str {
    match point.time_to_risk_bucket {
        TimeToRiskBucket::Normal => "normal",
        TimeToRiskBucket::Months => "months",
        TimeToRiskBucket::Weeks => "weeks",
        TimeToRiskBucket::Now => "now",
    }
}

fn backtest_warning_state(actionable_lead_time_days: Option<i64>) -> &'static str {
    match actionable_lead_time_days {
        Some(days) if days >= 7 => "timely",
        Some(_) => "late_only",
        None => "missed",
    }
}

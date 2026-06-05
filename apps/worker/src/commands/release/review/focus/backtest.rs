use std::collections::BTreeMap;

use fc_domain::BacktestScenarioSummary;

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

pub(super) fn scenario_requires_focus_review(
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

pub(super) fn backtest_warning_state(actionable_lead_time_days: Option<i64>) -> &'static str {
    match actionable_lead_time_days {
        Some(days) if days >= 7 => "timely",
        Some(_) => "late_only",
        None => "missed",
    }
}

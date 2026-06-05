use fc_domain::{
    BacktestScenarioSummary, BacktestSignalSource, HistoricalAnalog, ProbabilityBlock, RiskSnapshot,
};

use super::super::{round1, ProbabilityActionThresholds};

pub(in super::super) fn build_historical_analogs(
    snapshot: &RiskSnapshot,
    probabilities: &ProbabilityBlock,
    external_shock_score: f64,
    backtests: &[BacktestScenarioSummary],
    thresholds: ProbabilityActionThresholds,
) -> Vec<HistoricalAnalog> {
    let mut analogs = backtests
        .iter()
        .map(|scenario| {
            let score_distance = (snapshot.overall_score - scenario.max_score).abs();
            let lead_reference = if probabilities.p_5d >= thresholds.defend_p5d
                || probabilities.p_20d >= thresholds.hedge_p20d
            {
                scenario.actionable_lead_time_days.or(scenario.lead_time_days)
            } else {
                scenario.lead_time_days.or(scenario.actionable_lead_time_days)
            };
            let lead_distance = scenario
                .actionable_lead_time_days
                .or(lead_reference)
                .map(|days| ((probabilities.p_20d * 100.0) - days as f64).abs())
                .unwrap_or(35.0);
            let fallback_penalty = match scenario.signal_source {
                BacktestSignalSource::RealHistory => 0.0,
                BacktestSignalSource::FallbackTemplate => 8.0,
            };
            let similarity_score = (100.0 - score_distance * 1.2 - lead_distance * 0.35
                + external_shock_score * 0.08
                - fallback_penalty)
                .clamp(18.0, 96.0);
            HistoricalAnalog {
                scenario_id: scenario.scenario_id.clone(),
                name: scenario.name.clone(),
                similarity_score: round1(similarity_score),
                reference_phase: if probabilities.p_5d >= thresholds.defend_p5d {
                    "acute_window".to_string()
                } else if probabilities.p_20d >= thresholds.hedge_p20d {
                    "pre_break".to_string()
                } else {
                    "fragile_build_up".to_string()
                },
                note: match scenario.signal_source {
                    BacktestSignalSource::RealHistory => match (
                        scenario.lead_time_days,
                        scenario.actionable_lead_time_days,
                    ) {
                        (Some(structural), Some(actionable)) => format!(
                            "{} 的真实历史里，结构性抬升约领先 {} 天，可执行预警约领先 {} 天。",
                            scenario.name, structural, actionable
                        ),
                        (Some(structural), None) => format!(
                            "{} 的真实历史里，结构性抬升约领先 {} 天，但危机前未形成足够强的可执行预警。",
                            scenario.name, structural
                        ),
                        (None, Some(actionable)) => format!(
                            "{} 的真实历史里，约领先 {} 天进入可执行预警，但没有更早的稳定结构抬升。",
                            scenario.name, actionable
                        ),
                        (None, None) => format!(
                            "{} 的真实历史里，危机前没有形成稳定的结构或动作级预警。",
                            scenario.name
                        ),
                    },
                    BacktestSignalSource::FallbackTemplate => {
                        format!("当前分数与 {} 的参考模板较接近；该样本尚未由本地历史库完整覆盖。", scenario.name)
                    }
                },
                peak_score: scenario.max_score,
                lead_time_days: scenario.lead_time_days,
                actionable_lead_time_days: scenario.actionable_lead_time_days,
            }
        })
        .collect::<Vec<_>>();
    analogs.sort_by(|left, right| right.similarity_score.total_cmp(&left.similarity_score));
    analogs.truncate(3);
    analogs
}

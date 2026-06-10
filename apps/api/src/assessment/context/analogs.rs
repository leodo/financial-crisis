use fc_domain::{
    BacktestScenarioSummary, BacktestSignalSource, HistoricalAnalog, ProbabilityBlock, RiskSnapshot,
};

use super::super::{round1, ProbabilityActionThresholds};

const CORE_HISTORICAL_ANALOG_IDS: [&str; 7] = [
    "us_black_monday_1987",
    "us_dotcom_unwind_2000",
    "us_gfc_2008",
    "us_funding_stress_2011",
    "us_covid_liquidity_2020",
    "us_rate_shock_2022",
    "us_regional_banks_2023",
];

pub(in super::super) fn build_historical_analogs(
    snapshot: &RiskSnapshot,
    probabilities: &ProbabilityBlock,
    external_shock_score: f64,
    backtests: &[BacktestScenarioSummary],
    thresholds: ProbabilityActionThresholds,
) -> Vec<HistoricalAnalog> {
    let analogs = backtests
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
    prioritize_historical_analogs(analogs)
}

fn prioritize_historical_analogs(mut analogs: Vec<HistoricalAnalog>) -> Vec<HistoricalAnalog> {
    analogs.sort_by(|left, right| right.similarity_score.total_cmp(&left.similarity_score));

    let core_analogs = analogs
        .iter()
        .filter(|analog| CORE_HISTORICAL_ANALOG_IDS.contains(&analog.scenario_id.as_str()))
        .cloned()
        .collect::<Vec<_>>();

    if core_analogs.is_empty() {
        analogs.truncate(3);
        return analogs;
    }

    core_analogs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analog(scenario_id: &str, similarity_score: f64) -> HistoricalAnalog {
        HistoricalAnalog {
            scenario_id: scenario_id.to_string(),
            name: scenario_id.to_string(),
            similarity_score,
            reference_phase: "fragile_build_up".to_string(),
            note: "test".to_string(),
            peak_score: 50.0,
            lead_time_days: None,
            actionable_lead_time_days: None,
        }
    }

    #[test]
    fn prioritized_historical_analogs_keeps_core_crisis_set_in_similarity_order() {
        let rows = prioritize_historical_analogs(vec![
            analog("us_bond_massacre_1994", 95.0),
            analog("us_black_monday_1987", 74.0),
            analog("us_dotcom_unwind_2000", 63.0),
            analog("us_gfc_2008", 42.0),
            analog("us_funding_stress_2011", 66.0),
            analog("us_covid_liquidity_2020", 38.0),
            analog("us_rate_shock_2022", 58.0),
            analog("us_regional_banks_2023", 52.0),
        ]);

        assert_eq!(rows.len(), 7);
        assert!(!rows.iter().any(|row| row.scenario_id == "us_bond_massacre_1994"));
        assert_eq!(rows[0].scenario_id, "us_black_monday_1987");
        assert_eq!(rows[1].scenario_id, "us_funding_stress_2011");
        assert_eq!(rows[6].scenario_id, "us_covid_liquidity_2020");
    }

    #[test]
    fn prioritized_historical_analogs_falls_back_to_top_three_when_core_set_missing() {
        let rows = prioritize_historical_analogs(vec![
            analog("scenario_a", 20.0),
            analog("scenario_b", 90.0),
            analog("scenario_c", 40.0),
            analog("scenario_d", 80.0),
        ]);

        assert_eq!(
            rows.iter()
                .map(|row| row.scenario_id.as_str())
                .collect::<Vec<_>>(),
            vec!["scenario_b", "scenario_d", "scenario_c"]
        );
    }
}

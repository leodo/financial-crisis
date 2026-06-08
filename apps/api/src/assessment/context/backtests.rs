use fc_domain::{
    BacktestPerformanceSummary, BacktestRollingAudit, BacktestScenarioSummary, BacktestSignalSource,
};

use super::super::{round1, round3};

pub(crate) fn build_backtest_summary(
    backtests: &[BacktestScenarioSummary],
    rolling_audit: Option<&BacktestRollingAudit>,
) -> BacktestPerformanceSummary {
    let rolling_audit = rolling_audit.cloned().unwrap_or_else(empty_rolling_audit);
    if backtests.is_empty() {
        return BacktestPerformanceSummary {
            scenario_count: 0,
            real_scenario_count: 0,
            fallback_scenario_count: 0,
            coverage_scope_note:
                "这里的危机场景覆盖按当前默认运行历史窗口统计，不等于上面默认历史轨迹的 PIT 证据层。"
                    .to_string(),
            structural_warning_rate: 0.0,
            timely_warning_rate: 0.0,
            missed_rate: 1.0,
            avg_structural_lead_time_days: None,
            avg_lead_time_days: None,
            median_lead_time_days: None,
            total_false_positive_count: 0,
            history_start: None,
            history_end: None,
            rolling_audit,
            summary: "当前没有可用回测场景，不能据此评估 posture 的历史可靠性。".to_string(),
        };
    }

    let scenario_count = backtests.len() as u32;
    let real_scenario_count = backtests
        .iter()
        .filter(|scenario| scenario.signal_source == BacktestSignalSource::RealHistory)
        .count() as u32;
    let fallback_scenario_count = scenario_count.saturating_sub(real_scenario_count);
    let structural_warning_count = backtests
        .iter()
        .filter(|scenario| scenario.lead_time_days.unwrap_or_default() >= 7)
        .count() as u32;
    let timely_count = backtests
        .iter()
        .filter(|scenario| {
            !scenario.missed && scenario.actionable_lead_time_days.unwrap_or_default() >= 7
        })
        .count() as u32;
    let missed_count = backtests.iter().filter(|scenario| scenario.missed).count() as u32;
    let mut structural_lead_times = backtests
        .iter()
        .filter_map(|scenario| scenario.lead_time_days.map(|days| days as f64))
        .collect::<Vec<_>>();
    structural_lead_times.sort_by(|left, right| left.total_cmp(right));
    let mut lead_times = backtests
        .iter()
        .filter_map(|scenario| scenario.actionable_lead_time_days.map(|days| days as f64))
        .collect::<Vec<_>>();
    lead_times.sort_by(|left, right| left.total_cmp(right));
    let avg_structural_lead_time_days = (!structural_lead_times.is_empty()).then(|| {
        round1(structural_lead_times.iter().sum::<f64>() / structural_lead_times.len() as f64)
    });
    let avg_lead_time_days = (!lead_times.is_empty())
        .then(|| round1(lead_times.iter().sum::<f64>() / lead_times.len() as f64));
    let median_lead_time_days = if lead_times.is_empty() {
        None
    } else {
        Some(round1(lead_times[lead_times.len() / 2]))
    };
    let total_false_positive_count = backtests
        .iter()
        .map(|scenario| scenario.false_positive_count)
        .sum();
    let structural_warning_rate = round3(structural_warning_count as f64 / scenario_count as f64);
    let timely_warning_rate = round3(timely_count as f64 / scenario_count as f64);
    let missed_rate = round3(missed_count as f64 / scenario_count as f64);
    let history_start = backtests
        .iter()
        .filter_map(|scenario| scenario.history_start)
        .min();
    let history_end = backtests
        .iter()
        .filter_map(|scenario| scenario.history_end)
        .max();
    let coverage_scope_note = match (history_start, history_end) {
        (Some(start), Some(end)) => format!(
            "这里的“本地覆盖场景 / 模板参照场景”按默认运行历史窗口 {start} 到 {end} 统计；它回答的是危机场景目录里有多少样本能直接落在这段本地历史上，不等于上面默认历史轨迹是否已经进入 PIT 正式证据层。"
        ),
        _ => "这里的“本地覆盖场景 / 模板参照场景”按当前默认运行历史窗口统计；它回答的是危机场景目录里有多少样本能直接落在本地历史上，不等于上面默认历史轨迹是否已经进入 PIT 正式证据层。".to_string(),
    };
    let summary = if fallback_scenario_count > 0 {
        format!(
            "当前危机场景目录共 {} 个样本，其中 {} 个已被当前本地历史窗口直接覆盖，{} 个仍是模板参照；结构性抬升至少提前 7 天出现的比例约为 {:.0}%，可执行预警至少提前 7 天出现的比例约为 {:.0}%。",
            scenario_count,
            real_scenario_count,
            fallback_scenario_count,
            structural_warning_rate * 100.0,
            timely_warning_rate * 100.0
        )
    } else {
        format!(
            "当前回测覆盖 {} 个真实危机样本；结构性抬升至少提前 7 天出现的比例约为 {:.0}%，可执行预警至少提前 7 天出现的比例约为 {:.0}%。",
            scenario_count,
            structural_warning_rate * 100.0,
            timely_warning_rate * 100.0
        )
    };

    BacktestPerformanceSummary {
        scenario_count,
        real_scenario_count,
        fallback_scenario_count,
        coverage_scope_note,
        structural_warning_rate,
        timely_warning_rate,
        missed_rate,
        avg_structural_lead_time_days,
        avg_lead_time_days,
        median_lead_time_days,
        total_false_positive_count,
        history_start,
        history_end,
        rolling_audit,
        summary,
    }
}

fn empty_rolling_audit() -> BacktestRollingAudit {
    BacktestRollingAudit {
        history_point_count: 0,
        actionable_signal_count: 0,
        pre_crisis_signal_count: 0,
        in_crisis_signal_count: 0,
        stress_window_signal_count: 0,
        false_positive_signal_count: 0,
        false_positive_episode_count: 0,
        longest_false_positive_episode_days: 0,
        actionable_precision: 0.0,
        classified_episodes: Vec::new(),
        summary: "当前尚未生成全历史滚动审计结果。".to_string(),
    }
}

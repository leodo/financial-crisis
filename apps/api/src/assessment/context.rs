use chrono::{NaiveDate, Utc};
use fc_domain::{
    AlertEvent, BacktestPerformanceSummary, BacktestRollingAudit, BacktestScenarioSummary,
    BacktestSignalSource, DataMode, EventAssessment, EventConfirmationState, EventSignalSummary,
    FreshnessStatus, HistoricalAnalog, KeyIndicatorStatus, Observation, ProbabilityBlock,
    RiskDimension, RiskSnapshot, RuntimeMetadata,
};

use super::{round1, round3, ProbabilityActionThresholds};

pub(super) fn build_runtime_metadata(
    data_mode: DataMode,
    snapshot: &RiskSnapshot,
    observations: &[Observation],
) -> RuntimeMetadata {
    let latest_observation_at = observations
        .iter()
        .filter(|observation| {
            !observation
                .quality_flags
                .iter()
                .any(|flag| flag == "synthetic_zero_fill")
        })
        .map(|observation| observation.as_of_date)
        .max()
        .or_else(|| {
            observations
                .iter()
                .map(|observation| observation.as_of_date)
                .max()
        });
    let latest_observation_lag_days =
        latest_observation_at.map(|date| (snapshot.as_of_date - date).num_days());
    let demo_mode = matches!(data_mode, DataMode::Demo);
    let stale_warning = if demo_mode {
        Some("当前页面运行在 demo 模式，关键指标值是示例数据，不代表真实市场最新状态。".to_string())
    } else if let Some(lag) = latest_observation_lag_days {
        (lag > 5).then(|| format!("当前评估使用的最新观测值滞后 {lag} 天，短期判断需要保守解释。"))
    } else {
        Some("当前缺少最新观测值，不能把面板数字当成实时市场状态。".to_string())
    };

    RuntimeMetadata {
        data_mode,
        generated_at: Utc::now(),
        requested_as_of_date: snapshot.as_of_date,
        latest_observation_at,
        latest_observation_lag_days,
        demo_mode,
        stale_warning,
    }
}

pub(super) fn build_key_indicator_statuses(
    observations: &[Observation],
    requested_as_of_date: NaiveDate,
    data_mode: DataMode,
) -> Vec<KeyIndicatorStatus> {
    [
        (
            "us_external_usdjpy_level",
            "USDJPY",
            "us",
            "jpy_per_usd",
            3_i64,
        ),
        (
            "jp_rates_call_rate",
            "日本无担保隔夜拆借利率",
            "jp",
            "percent",
            5_i64,
        ),
        (
            "us_liquidity_effr",
            "有效联邦基金利率",
            "us",
            "percent",
            5_i64,
        ),
        ("us_market_vix_close", "VIX 收盘价", "us", "index", 3_i64),
    ]
    .into_iter()
    .map(
        |(indicator_id, display_name, entity_id, unit, stale_threshold_days)| {
            let latest = observations
                .iter()
                .filter(|observation| observation.indicator_id == indicator_id)
                .filter(|observation| observation.entity_id == entity_id)
                .filter(|observation| observation.as_of_date <= requested_as_of_date)
                .max_by_key(|observation| observation.as_of_date);

            let latest_as_of_date = latest.map(|observation| observation.as_of_date);
            let lag_days = latest_as_of_date.map(|date| (requested_as_of_date - date).num_days());
            let status = if matches!(data_mode, DataMode::Demo) {
                FreshnessStatus::Stale
            } else if latest.is_none() {
                FreshnessStatus::Missing
            } else if lag_days.unwrap_or_default() > stale_threshold_days * 3 {
                FreshnessStatus::Stale
            } else if lag_days.unwrap_or_default() > stale_threshold_days {
                FreshnessStatus::Delayed
            } else {
                FreshnessStatus::Fresh
            };

            let note = if matches!(data_mode, DataMode::Demo) {
                "demo 示例数据，不代表真实市场最新值。".to_string()
            } else {
                match status {
                    FreshnessStatus::Fresh => "关键指标处于可接受的新鲜度范围。".to_string(),
                    FreshnessStatus::Delayed => {
                        "指标有一定滞后，近端风险判断要结合其他证据。".to_string()
                    }
                    FreshnessStatus::Stale => {
                        "指标明显陈旧，不能把当前显示值当成实时市场状态。".to_string()
                    }
                    FreshnessStatus::Missing => "缺少该指标最新值。".to_string(),
                }
            };

            KeyIndicatorStatus {
                indicator_id: indicator_id.to_string(),
                display_name: display_name.to_string(),
                entity_id: entity_id.to_string(),
                source_id: latest.map(|observation| observation.source_id.clone()),
                dataset_id: latest.map(|observation| observation.dataset_id.clone()),
                unit: unit.to_string(),
                latest_value: latest.map(|observation| observation.value),
                latest_as_of_date,
                lag_days,
                stale_threshold_days,
                status,
                note,
            }
        },
    )
    .collect()
}

pub(super) fn build_event_assessment(
    snapshot: &RiskSnapshot,
    alerts: &[AlertEvent],
) -> EventAssessment {
    let recent_event_count = alerts.len() as u32;
    let recent_events = alerts
        .iter()
        .take(4)
        .map(|alert| EventSignalSummary {
            event_type: alert.event_type,
            level: alert.level,
            triggered_as_of_date: alert.triggered_as_of_date,
            trigger_reason: alert.trigger_reason.clone(),
            related_indicators: alert.related_indicators.clone(),
        })
        .collect::<Vec<_>>();
    let confirmation_score = round1(
        (snapshot
            .dimensions
            .iter()
            .find(|dimension| dimension.dimension == RiskDimension::EventsSentiment)
            .map(|dimension| dimension.score)
            .unwrap_or(0.0)
            * 0.7
            + recent_event_count as f64 * 9.0)
            .clamp(0.0, 100.0),
    );
    let state = if confirmation_score >= 70.0 {
        EventConfirmationState::Escalating
    } else if confirmation_score >= 55.0 {
        EventConfirmationState::Confirmed
    } else if confirmation_score >= 30.0 {
        EventConfirmationState::Watching
    } else {
        EventConfirmationState::Quiet
    };

    let confirmed_signals = alerts
        .iter()
        .map(|alert| alert.trigger_reason.clone())
        .take(3)
        .collect::<Vec<_>>();
    let mut pending_gaps = Vec::new();
    if recent_event_count == 0 {
        pending_gaps.push("事件层还没有给出足够确认，当前更多依赖价格和宏观层信号。".to_string());
    }
    if snapshot.trigger_score >= 60.0 && recent_event_count < 2 {
        pending_gaps.push("触发层已抬升，但银行/公告/新闻事件还没有形成更强共振。".to_string());
    }

    let summary = match state {
        EventConfirmationState::Quiet => {
            "事件层暂时安静，当前风险判断主要来自价格和融资信号。".to_string()
        }
        EventConfirmationState::Watching => {
            "事件层开始出现支持证据，但还不足以单独驱动强结论。".to_string()
        }
        EventConfirmationState::Confirmed => {
            "事件层已经提供了实质性确认，当前风险判断不再只是市场噪声。".to_string()
        }
        EventConfirmationState::Escalating => {
            "事件层与市场层正在同步升级，需优先防范短期风险压缩。".to_string()
        }
    };

    EventAssessment {
        state,
        confirmation_score,
        recent_event_count,
        summary,
        confirmed_signals,
        pending_gaps,
        recent_events,
    }
}

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
    let summary = if fallback_scenario_count > 0 {
        format!(
            "当前回测共列出 {} 个危机样本，其中 {} 个来自本地真实历史，{} 个仍是模板参考；结构性抬升至少提前 7 天出现的比例约为 {:.0}%，可执行预警至少提前 7 天出现的比例约为 {:.0}%。",
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

pub(super) fn build_historical_analogs(
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

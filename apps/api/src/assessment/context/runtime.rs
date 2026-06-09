use chrono::Utc;
use fc_domain::{
    DataMode, FreshnessStatus, KeyIndicatorStatus, Observation, RiskSnapshot, RuntimeMetadata,
};

use super::freshness::business_lag_days;

fn freshness_rank(status: FreshnessStatus) -> u8 {
    match status {
        FreshnessStatus::Fresh => 0,
        FreshnessStatus::Delayed => 1,
        FreshnessStatus::Stale => 2,
        FreshnessStatus::Missing => 3,
    }
}

pub(in super::super) fn build_runtime_metadata(
    data_mode: DataMode,
    snapshot: &RiskSnapshot,
    observations: &[Observation],
    key_indicators: &[KeyIndicatorStatus],
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
    let latest_key_indicator_at = key_indicators
        .iter()
        .filter_map(|indicator| indicator.latest_as_of_date);
    let latest_key_indicator_at = latest_key_indicator_at.max();
    let latest_observation_lag_days =
        latest_observation_at.map(|date| (snapshot.as_of_date - date).num_days());
    let latest_observation_lag_business_days =
        latest_observation_at.map(|date| business_lag_days(date, snapshot.as_of_date));
    let latest_key_indicator_lag_days =
        latest_key_indicator_at.map(|date| (snapshot.as_of_date - date).num_days());
    let latest_key_indicator_lag_business_days =
        latest_key_indicator_at.map(|date| business_lag_days(date, snapshot.as_of_date));
    let demo_mode = matches!(data_mode, DataMode::Demo);
    let worst_key_indicator = key_indicators
        .iter()
        .max_by_key(|indicator| freshness_rank(indicator.status));
    let delayed_or_stale_indicators = key_indicators
        .iter()
        .filter(|indicator| {
            matches!(
                indicator.status,
                FreshnessStatus::Delayed | FreshnessStatus::Stale | FreshnessStatus::Missing
            )
        })
        .collect::<Vec<_>>();
    let stale_warning = if demo_mode {
        Some("当前页面运行在 demo 模式，关键指标值是示例数据，不代表真实市场最新状态。".to_string())
    } else if !delayed_or_stale_indicators.is_empty() {
        let labels = delayed_or_stale_indicators
            .iter()
            .take(3)
            .map(|indicator| indicator.display_name.as_str())
            .collect::<Vec<_>>()
            .join("、");
        let label_tail = if delayed_or_stale_indicators.len() > 3 {
            format!(" 等 {} 个关键指标", delayed_or_stale_indicators.len())
        } else {
            String::new()
        };
        match worst_key_indicator {
            Some(indicator)
                if matches!(
                    indicator.status,
                    FreshnessStatus::Delayed | FreshnessStatus::Stale | FreshnessStatus::Missing
                ) =>
            {
                match (indicator.lag_days, indicator.lag_business_days) {
                    (Some(calendar_lag), Some(business_lag)) => Some(format!(
                        "{labels}{label_tail} 当前不是实时盘中值；最旧一档自然日约 {calendar_lag} 天，按工作日口径约 {business_lag} 天。像 USDJPY、VIX、短端利率这类近端信号需要结合日期解释。"
                    )),
                    _ => Some(format!(
                        "{labels}{label_tail} 当前不是实时盘中值；近端风险判断需要结合日期解释。"
                    )),
                }
            }
            _ => None,
        }
    } else if let Some(overall_lag) = latest_observation_lag_business_days {
        if overall_lag > 5 {
            Some(format!(
                "当前评估使用的整体最新观测值按工作日口径约滞后 {overall_lag} 天，短期判断需要保守解释。"
            ))
        } else {
            None
        }
    } else if let Some(lag) = latest_observation_lag_days {
        (lag > 5)
            .then(|| format!("当前评估使用的整体最新观测值滞后 {lag} 天，短期判断需要保守解释。"))
    } else {
        Some("当前缺少最新观测值，不能把面板数字当成实时市场状态。".to_string())
    };

    RuntimeMetadata {
        data_mode,
        generated_at: Utc::now(),
        requested_as_of_date: snapshot.as_of_date,
        latest_observation_at,
        latest_observation_lag_days,
        latest_observation_lag_business_days,
        latest_key_indicator_at,
        latest_key_indicator_lag_days,
        latest_key_indicator_lag_business_days,
        demo_mode,
        stale_warning,
    }
}

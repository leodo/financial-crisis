use chrono::Utc;
use fc_domain::{DataMode, KeyIndicatorStatus, Observation, RiskSnapshot, RuntimeMetadata};

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
    let latest_key_indicator_lag_days =
        latest_key_indicator_at.map(|date| (snapshot.as_of_date - date).num_days());
    let demo_mode = matches!(data_mode, DataMode::Demo);
    let stale_warning = if demo_mode {
        Some("当前页面运行在 demo 模式，关键指标值是示例数据，不代表真实市场最新状态。".to_string())
    } else if let Some(lag) = latest_key_indicator_lag_days {
        if lag > 3 {
            Some(format!(
                "当前关键市场指标最新日期滞后 {lag} 天；像 USDJPY、VIX、短端利率这类近端信号需要保守解释。"
            ))
        } else {
            latest_observation_lag_days.and_then(|overall_lag| {
                (overall_lag > 5).then(|| {
                    format!(
                        "当前评估使用的整体最新观测值滞后 {overall_lag} 天，短期判断需要保守解释。"
                    )
                })
            })
        }
    } else if let Some(lag) = latest_observation_lag_days {
        (lag > 5).then(|| {
            format!("当前评估使用的整体最新观测值滞后 {lag} 天，短期判断需要保守解释。")
        })
    } else {
        Some("当前缺少最新观测值，不能把面板数字当成实时市场状态。".to_string())
    };

    RuntimeMetadata {
        data_mode,
        generated_at: Utc::now(),
        requested_as_of_date: snapshot.as_of_date,
        latest_observation_at,
        latest_observation_lag_days,
        latest_key_indicator_at,
        latest_key_indicator_lag_days,
        demo_mode,
        stale_warning,
    }
}

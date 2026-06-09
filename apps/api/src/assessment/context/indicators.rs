use chrono::NaiveDate;
use fc_domain::{DataMode, FreshnessStatus, KeyIndicatorStatus, Observation};

use super::freshness::business_lag_days;

pub(in super::super) fn build_key_indicator_statuses(
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
            let lag_business_days =
                latest_as_of_date.map(|date| business_lag_days(date, requested_as_of_date));
            let freshness_lag_days = lag_business_days.unwrap_or_else(|| lag_days.unwrap_or_default());
            let status = if matches!(data_mode, DataMode::Demo) {
                FreshnessStatus::Stale
            } else if latest.is_none() {
                FreshnessStatus::Missing
            } else if freshness_lag_days > stale_threshold_days * 3 {
                FreshnessStatus::Stale
            } else if freshness_lag_days > stale_threshold_days {
                FreshnessStatus::Delayed
            } else {
                FreshnessStatus::Fresh
            };

            let note = if matches!(data_mode, DataMode::Demo) {
                "demo 示例数据，不代表真实市场最新值。".to_string()
            } else {
                match status {
                    FreshnessStatus::Fresh => {
                        if let (Some(calendar_lag), Some(business_lag)) =
                            (lag_days, lag_business_days)
                        {
                            if calendar_lag >= 3 && business_lag <= stale_threshold_days {
                                format!(
                                    "显示日期跨了 {calendar_lag} 个自然日，但按工作日口径约 {business_lag} 天，主要受周末或非工作日影响。"
                                )
                            } else {
                                "关键指标处于可接受的新鲜度范围。".to_string()
                            }
                        } else {
                            "关键指标处于可接受的新鲜度范围。".to_string()
                        }
                    }
                    FreshnessStatus::Delayed => {
                        match (lag_days, lag_business_days) {
                            (Some(calendar_lag), Some(business_lag)) => format!(
                                "指标有一定滞后（自然日 {calendar_lag} 天 / 工作日约 {business_lag} 天），近端风险判断要结合其他证据。"
                            ),
                            _ => "指标有一定滞后，近端风险判断要结合其他证据。".to_string(),
                        }
                    }
                    FreshnessStatus::Stale => {
                        match (lag_days, lag_business_days) {
                            (Some(calendar_lag), Some(business_lag)) => format!(
                                "指标明显陈旧（自然日 {calendar_lag} 天 / 工作日约 {business_lag} 天），不能把当前显示值当成实时市场状态。"
                            ),
                            _ => {
                                "指标明显陈旧，不能把当前显示值当成实时市场状态。".to_string()
                            }
                        }
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
                lag_business_days,
                stale_threshold_days,
                status,
                note,
                lineage: None,
            }
        },
    )
    .collect()
}

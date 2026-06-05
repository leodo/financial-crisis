use fc_domain::{Frequency, RiskDimension, RiskDirection};

use super::super::super::{BOJ_FX_DATASET_ID, BOJ_MONEY_MARKET_DATASET_ID};
use super::BojIndicatorSeed;

pub(super) fn boj_indicator_seeds() -> Vec<BojIndicatorSeed> {
    vec![
        BojIndicatorSeed {
            indicator_id: "us_external_usdjpy_level",
            display_name: "USDJPY 汇率",
            dimension: RiskDimension::ExternalSector,
            description: "BOJ 官方美元兑日元汇率水平，用于识别日元套息交易的潜在平仓压力。",
            unit: "jpy_per_usd",
            frequency: Frequency::Daily,
            risk_direction: RiskDirection::TwoSided,
            dataset_id: BOJ_FX_DATASET_ID,
            external_code: "FXERD01",
            default_source_id: "boj",
            quality_tier: "core",
            priority: 10,
        },
        BojIndicatorSeed {
            indicator_id: "jp_rates_call_rate",
            display_name: "日本无担保隔夜拆借利率",
            dimension: RiskDimension::ExternalSector,
            description: "BOJ 官方无担保隔夜拆借利率，可作为日元融资成本与 BOJ 政策变化代理。",
            unit: "percent",
            frequency: Frequency::Daily,
            risk_direction: RiskDirection::RisingFastIsRiskier,
            dataset_id: BOJ_MONEY_MARKET_DATASET_ID,
            external_code: "STRDCLUCON",
            default_source_id: "boj",
            quality_tier: "extended",
            priority: 20,
        },
    ]
}

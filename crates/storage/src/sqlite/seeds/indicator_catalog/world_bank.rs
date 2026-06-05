use fc_domain::{Frequency, RiskDimension, RiskDirection};

use super::WorldBankIndicatorSeed;

pub(super) fn world_bank_indicator_seeds() -> Vec<WorldBankIndicatorSeed> {
    vec![
        WorldBankIndicatorSeed {
            indicator_id: "global_macro_gdp_growth",
            display_name: "GDP 实际增速",
            dimension: RiskDimension::MacroFragility,
            description: "World Bank 年频 GDP 实际增速，当前默认抓取美国。",
            unit: "percent",
            frequency: Frequency::Annual,
            risk_direction: RiskDirection::LowerIsRiskier,
            external_code: "US__NY.GDP.MKTP.KD.ZG",
        },
        WorldBankIndicatorSeed {
            indicator_id: "global_macro_inflation_yoy",
            display_name: "CPI 通胀",
            dimension: RiskDimension::MacroFragility,
            description: "World Bank 年频 CPI 通胀，当前默认抓取美国。",
            unit: "percent",
            frequency: Frequency::Annual,
            risk_direction: RiskDirection::TwoSided,
            external_code: "US__FP.CPI.TOTL.ZG",
        },
        WorldBankIndicatorSeed {
            indicator_id: "global_external_current_account_gdp",
            display_name: "经常账户/GDP",
            dimension: RiskDimension::ExternalSector,
            description: "World Bank 年频经常账户余额占 GDP 比重，当前默认抓取美国。",
            unit: "percent",
            frequency: Frequency::Annual,
            risk_direction: RiskDirection::LowerIsRiskier,
            external_code: "US__BN.CAB.XOKA.GD.ZS",
        },
    ]
}

use fc_domain::{Frequency, Indicator, RiskDimension, RiskDirection};
use uuid::Uuid;

use crate::StorageError;

use super::{SqliteStore, BOJ_FX_DATASET_ID, BOJ_MONEY_MARKET_DATASET_ID, FRED_DATASET_ID};

#[derive(Debug, Clone, Copy)]
pub(super) struct FredIndicatorSeed {
    pub(super) indicator_id: &'static str,
    pub(super) display_name: &'static str,
    pub(super) dimension: RiskDimension,
    pub(super) description: &'static str,
    pub(super) unit: &'static str,
    pub(super) frequency: Frequency,
    pub(super) risk_direction: RiskDirection,
    pub(super) external_code: &'static str,
    pub(super) priority: i64,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct BojIndicatorSeed {
    pub(super) indicator_id: &'static str,
    pub(super) display_name: &'static str,
    pub(super) dimension: RiskDimension,
    pub(super) description: &'static str,
    pub(super) unit: &'static str,
    pub(super) frequency: Frequency,
    pub(super) risk_direction: RiskDirection,
    pub(super) dataset_id: &'static str,
    pub(super) external_code: &'static str,
    pub(super) default_source_id: &'static str,
    pub(super) quality_tier: &'static str,
    pub(super) priority: i64,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct WorldBankIndicatorSeed {
    pub(super) indicator_id: &'static str,
    pub(super) display_name: &'static str,
    pub(super) dimension: RiskDimension,
    pub(super) description: &'static str,
    pub(super) unit: &'static str,
    pub(super) frequency: Frequency,
    pub(super) risk_direction: RiskDirection,
    pub(super) external_code: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct SecEventIndicatorSeed {
    pub(super) indicator_id: &'static str,
    pub(super) display_name: &'static str,
    pub(super) description: &'static str,
    pub(super) unit: &'static str,
    pub(super) risk_direction: RiskDirection,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct GdeltIndicatorSeed {
    pub(super) indicator_id: &'static str,
    pub(super) display_name: &'static str,
    pub(super) description: &'static str,
    pub(super) unit: &'static str,
    pub(super) risk_direction: RiskDirection,
}

impl FredIndicatorSeed {
    pub(super) fn indicator(&self) -> Indicator {
        Indicator {
            indicator_id: self.indicator_id.to_string(),
            display_name: self.display_name.to_string(),
            dimension: self.dimension,
            description: self.description.to_string(),
            unit: self.unit.to_string(),
            frequency: self.frequency,
            risk_direction: self.risk_direction,
            default_source_id: "fred".to_string(),
            quality_tier: "core".to_string(),
        }
    }
}

impl BojIndicatorSeed {
    pub(super) fn indicator(&self) -> Indicator {
        Indicator {
            indicator_id: self.indicator_id.to_string(),
            display_name: self.display_name.to_string(),
            dimension: self.dimension,
            description: self.description.to_string(),
            unit: self.unit.to_string(),
            frequency: self.frequency,
            risk_direction: self.risk_direction,
            default_source_id: self.default_source_id.to_string(),
            quality_tier: self.quality_tier.to_string(),
        }
    }
}

impl WorldBankIndicatorSeed {
    pub(super) fn indicator(&self) -> Indicator {
        Indicator {
            indicator_id: self.indicator_id.to_string(),
            display_name: self.display_name.to_string(),
            dimension: self.dimension,
            description: self.description.to_string(),
            unit: self.unit.to_string(),
            frequency: self.frequency,
            risk_direction: self.risk_direction,
            default_source_id: "world_bank".to_string(),
            quality_tier: "core".to_string(),
        }
    }
}

impl SecEventIndicatorSeed {
    pub(super) fn indicator(&self) -> Indicator {
        Indicator {
            indicator_id: self.indicator_id.to_string(),
            display_name: self.display_name.to_string(),
            dimension: RiskDimension::EventsSentiment,
            description: self.description.to_string(),
            unit: self.unit.to_string(),
            frequency: Frequency::Daily,
            risk_direction: self.risk_direction,
            default_source_id: "sec_edgar".to_string(),
            quality_tier: "supplemental".to_string(),
        }
    }
}

impl GdeltIndicatorSeed {
    pub(super) fn indicator(&self) -> Indicator {
        Indicator {
            indicator_id: self.indicator_id.to_string(),
            display_name: self.display_name.to_string(),
            dimension: RiskDimension::EventsSentiment,
            description: self.description.to_string(),
            unit: self.unit.to_string(),
            frequency: Frequency::Daily,
            risk_direction: self.risk_direction,
            default_source_id: "gdelt".to_string(),
            quality_tier: "supplemental".to_string(),
        }
    }
}

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

pub(super) fn sec_event_indicator_seeds() -> Vec<SecEventIndicatorSeed> {
    vec![
        SecEventIndicatorSeed {
            indicator_id: "us_event_bank_8k_count",
            display_name: "白名单银行 8-K 数量",
            description: "Daily count of 8-K filings from the SEC EDGAR bank watchlist.",
            unit: "count",
            risk_direction: RiskDirection::ManualRule,
        },
        SecEventIndicatorSeed {
            indicator_id: "us_event_risk_keyword_count",
            display_name: "SEC 风险关键词/规则命中数",
            description:
                "Daily count of SEC filing metadata keyword hits plus high-risk 8-K item rule matches.",
            unit: "count",
            risk_direction: RiskDirection::ManualRule,
        },
        SecEventIndicatorSeed {
            indicator_id: "us_banking_filing_stress_count",
            display_name: "银行 filing 压力计数",
            description:
                "Daily count of filings whose rule-based severity passes the stress threshold.",
            unit: "count",
            risk_direction: RiskDirection::ManualRule,
        },
        SecEventIndicatorSeed {
            indicator_id: "us_event_official_filing_severity",
            display_name: "SEC 官方公告严重度",
            description:
                "Daily severity index aggregated from SEC filing form types, items, and watchlist breadth.",
            unit: "score",
            risk_direction: RiskDirection::ManualRule,
        },
    ]
}

pub(super) fn gdelt_indicator_seeds() -> Vec<GdeltIndicatorSeed> {
    vec![GdeltIndicatorSeed {
        indicator_id: "global_news_financial_stress_count",
        display_name: "金融压力新闻数量",
        description:
            "Daily GDELT DOC API count for banking, liquidity, funding, and credit-stress coverage.",
        unit: "count",
        risk_direction: RiskDirection::HigherIsRiskier,
    }]
}

pub(super) fn fred_indicator_seeds() -> Vec<FredIndicatorSeed> {
    vec![
        FredIndicatorSeed {
            indicator_id: "us_external_usdjpy_level",
            display_name: "USDJPY 汇率",
            dimension: RiskDimension::ExternalSector,
            description: "美元兑日元汇率水平，用于识别日元套息交易的潜在平仓压力。",
            unit: "jpy_per_usd",
            frequency: Frequency::Daily,
            risk_direction: RiskDirection::TwoSided,
            external_code: "DEXJPUS",
            priority: 100,
        },
        FredIndicatorSeed {
            indicator_id: "us_market_vix_close",
            display_name: "VIX 收盘价",
            dimension: RiskDimension::MarketStress,
            description: "美国市场隐含波动率。",
            unit: "index",
            frequency: Frequency::Daily,
            risk_direction: RiskDirection::HigherIsRiskier,
            external_code: "VIXCLS",
            priority: 100,
        },
        FredIndicatorSeed {
            indicator_id: "us_credit_high_yield_oas",
            display_name: "高收益债 OAS",
            dimension: RiskDimension::LeverageCredit,
            description: "美国高收益债期权调整利差。",
            unit: "percent",
            frequency: Frequency::Daily,
            risk_direction: RiskDirection::HigherIsRiskier,
            external_code: "BAMLH0A0HYM2",
            priority: 100,
        },
        FredIndicatorSeed {
            indicator_id: "us_credit_baa_10y_spread",
            display_name: "Baa-10Y 信用利差",
            dimension: RiskDimension::LeverageCredit,
            description: "Baa 企业债与 10 年期美国国债利差。",
            unit: "percent",
            frequency: Frequency::Daily,
            risk_direction: RiskDirection::HigherIsRiskier,
            external_code: "BAA10Y",
            priority: 100,
        },
        FredIndicatorSeed {
            indicator_id: "us_rates_yield_curve_10y2y",
            display_name: "10Y-2Y 期限利差",
            dimension: RiskDimension::MarketStress,
            description: "美国 10 年期和 2 年期国债收益率利差。",
            unit: "percent",
            frequency: Frequency::Daily,
            risk_direction: RiskDirection::LowerIsRiskier,
            external_code: "T10Y2Y",
            priority: 100,
        },
        FredIndicatorSeed {
            indicator_id: "us_liquidity_financial_stress_stl",
            display_name: "圣路易斯金融压力指数",
            dimension: RiskDimension::LiquidityFunding,
            description: "St. Louis Fed Financial Stress Index。",
            unit: "index",
            frequency: Frequency::Weekly,
            risk_direction: RiskDirection::HigherIsRiskier,
            external_code: "STLFSI4",
            priority: 100,
        },
        FredIndicatorSeed {
            indicator_id: "us_liquidity_national_financial_conditions",
            display_name: "NFCI 金融条件指数",
            dimension: RiskDimension::LiquidityFunding,
            description: "Chicago Fed National Financial Conditions Index。",
            unit: "index",
            frequency: Frequency::Weekly,
            risk_direction: RiskDirection::HigherIsRiskier,
            external_code: "NFCI",
            priority: 100,
        },
        FredIndicatorSeed {
            indicator_id: "us_macro_unemployment_rate",
            display_name: "失业率",
            dimension: RiskDimension::MacroFragility,
            description: "美国失业率。",
            unit: "percent",
            frequency: Frequency::Monthly,
            risk_direction: RiskDirection::HigherIsRiskier,
            external_code: "UNRATE",
            priority: 100,
        },
        FredIndicatorSeed {
            indicator_id: "us_liquidity_sofr",
            display_name: "SOFR",
            dimension: RiskDimension::LiquidityFunding,
            description: "Secured Overnight Financing Rate。",
            unit: "percent",
            frequency: Frequency::Daily,
            risk_direction: RiskDirection::RisingFastIsRiskier,
            external_code: "SOFR",
            priority: 100,
        },
        FredIndicatorSeed {
            indicator_id: "us_liquidity_effr",
            display_name: "有效联邦基金利率",
            dimension: RiskDimension::LiquidityFunding,
            description:
                "Daily Effective Federal Funds Rate (legacy DFF fallback for pre-EFFR history).",
            unit: "percent",
            frequency: Frequency::Daily,
            risk_direction: RiskDirection::RisingFastIsRiskier,
            external_code: "DFF",
            priority: 80,
        },
        FredIndicatorSeed {
            indicator_id: "us_liquidity_effr",
            display_name: "有效联邦基金利率",
            dimension: RiskDimension::LiquidityFunding,
            description: "Effective Federal Funds Rate。",
            unit: "percent",
            frequency: Frequency::Daily,
            risk_direction: RiskDirection::RisingFastIsRiskier,
            external_code: "EFFR",
            priority: 100,
        },
        FredIndicatorSeed {
            indicator_id: "us_real_estate_housing_starts",
            display_name: "新屋开工",
            dimension: RiskDimension::RealEstate,
            description: "美国新屋开工总数。",
            unit: "thousands",
            frequency: Frequency::Monthly,
            risk_direction: RiskDirection::LowerIsRiskier,
            external_code: "HOUST",
            priority: 100,
        },
        FredIndicatorSeed {
            indicator_id: "us_real_estate_home_price",
            display_name: "Case-Shiller 房价指数",
            dimension: RiskDimension::RealEstate,
            description: "美国全国 Case-Shiller 房价指数。",
            unit: "index",
            frequency: Frequency::Monthly,
            risk_direction: RiskDirection::TwoSided,
            external_code: "CSUSHPISA",
            priority: 100,
        },
        FredIndicatorSeed {
            indicator_id: "us_liquidity_money_supply_m2",
            display_name: "M2 货币供应",
            dimension: RiskDimension::LiquidityFunding,
            description: "美国 M2 货币供应量。",
            unit: "billions",
            frequency: Frequency::Monthly,
            risk_direction: RiskDirection::FallingFastIsRiskier,
            external_code: "M2SL",
            priority: 100,
        },
    ]
}

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

impl SqliteStore {
    pub(super) async fn upsert_fred_mapping(
        &self,
        indicator_id: &str,
        external_code: &str,
        priority: i64,
    ) -> Result<(), StorageError> {
        self.upsert_external_mapping(
            indicator_id,
            "fred",
            FRED_DATASET_ID,
            external_code,
            priority,
        )
        .await
    }

    pub(super) async fn upsert_external_mapping(
        &self,
        indicator_id: &str,
        source_id: &str,
        dataset_id: &str,
        external_code: &str,
        priority: i64,
    ) -> Result<(), StorageError> {
        sqlx::query(
            r#"
            INSERT INTO metadata_external_indicator_mappings (
                mapping_id,
                indicator_id,
                source_id,
                dataset_id,
                external_code,
                external_params_json,
                priority
            )
            VALUES (?1, ?2, ?3, ?4, ?5, '{}', ?6)
            ON CONFLICT(indicator_id, source_id, dataset_id, external_code) DO UPDATE SET
                external_params_json = excluded.external_params_json,
                priority = excluded.priority
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(indicator_id)
        .bind(source_id)
        .bind(dataset_id)
        .bind(external_code)
        .bind(priority)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskDimension {
    MacroFragility,
    LeverageCredit,
    MarketStress,
    LiquidityFunding,
    BankingSystem,
    RealEstate,
    ExternalSector,
    EventsSentiment,
}

impl RiskDimension {
    pub fn label(self) -> &'static str {
        match self {
            Self::MacroFragility => "宏观脆弱性",
            Self::LeverageCredit => "杠杆与信用",
            Self::MarketStress => "市场压力",
            Self::LiquidityFunding => "流动性与融资",
            Self::BankingSystem => "银行体系",
            Self::RealEstate => "房地产与资产泡沫",
            Self::ExternalSector => "外部部门与汇率",
            Self::EventsSentiment => "事件与情绪",
        }
    }

    pub fn is_structural(self) -> bool {
        matches!(
            self,
            Self::MacroFragility
                | Self::LeverageCredit
                | Self::BankingSystem
                | Self::RealEstate
                | Self::ExternalSector
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Frequency {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Annual,
    Event,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskDirection {
    HigherIsRiskier,
    LowerIsRiskier,
    TwoSided,
    FallingFastIsRiskier,
    RisingFastIsRiskier,
    ManualRule,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Indicator {
    pub indicator_id: String,
    pub display_name: String,
    pub dimension: RiskDimension,
    pub description: String,
    pub unit: String,
    pub frequency: Frequency,
    pub risk_direction: RiskDirection,
    pub default_source_id: String,
    pub quality_tier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub indicator_id: String,
    pub entity_id: String,
    pub as_of_date: NaiveDate,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub frequency: Frequency,
    pub value: f64,
    pub unit: String,
    pub source_id: String,
    pub dataset_id: String,
    pub revision_time: Option<DateTime<Utc>>,
    pub publication_time: Option<DateTime<Utc>>,
    pub quality_score: f64,
    pub quality_flags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndicatorRisk {
    pub indicator: Indicator,
    pub latest_observation: Option<Observation>,
    pub score: f64,
    pub level: crate::RiskLevel,
    pub percentile: Option<f64>,
    pub change_30d: Option<f64>,
    pub score_basis: String,
    pub score_input_value: Option<f64>,
    pub score_input_unit: Option<String>,
    pub quality_grade: crate::QualityGrade,
    pub contribution: f64,
}

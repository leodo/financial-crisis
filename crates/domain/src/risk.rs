use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::{DataQualitySummary, RiskDimension};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Normal,
    Watch,
    Stress,
    Warning,
    Crisis,
}

impl RiskLevel {
    pub fn from_score(score: f64) -> Self {
        if score >= 85.0 {
            Self::Crisis
        } else if score >= 70.0 {
            Self::Warning
        } else if score >= 50.0 {
            Self::Stress
        } else if score >= 30.0 {
            Self::Watch
        } else {
            Self::Normal
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::Normal => "L0",
            Self::Watch => "L1",
            Self::Stress => "L2",
            Self::Warning => "L3",
            Self::Crisis => "L4",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Watch => "Watch",
            Self::Stress => "Stress",
            Self::Warning => "Warning",
            Self::Crisis => "Crisis",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskContributor {
    pub indicator_id: String,
    pub display_name: String,
    pub dimension: RiskDimension,
    pub score: f64,
    pub contribution: f64,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionScore {
    pub dimension: RiskDimension,
    pub label: String,
    pub score: f64,
    pub level: RiskLevel,
    pub change_30d: Option<f64>,
    pub quality_score: f64,
    pub top_contributors: Vec<RiskContributor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskSnapshot {
    pub as_of_date: NaiveDate,
    pub entity_id: String,
    pub market_scope: String,
    pub overall_score: f64,
    pub overall_level: RiskLevel,
    pub structural_score: f64,
    pub trigger_score: f64,
    pub level_reason: String,
    pub dimensions: Vec<DimensionScore>,
    pub top_contributors: Vec<RiskContributor>,
    pub data_quality_summary: DataQualitySummary,
    pub generated_at: DateTime<Utc>,
    pub method_version: String,
}

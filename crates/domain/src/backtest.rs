use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::{RiskContributor, RiskLevel};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestScenarioSummary {
    pub scenario_id: String,
    pub name: String,
    pub region: String,
    pub crisis_start: NaiveDate,
    pub crisis_end: NaiveDate,
    pub first_l2_date: Option<NaiveDate>,
    pub first_l3_date: Option<NaiveDate>,
    pub max_level: RiskLevel,
    pub max_score: f64,
    pub lead_time_days: Option<i64>,
    pub false_positive_count: u32,
    pub missed: bool,
    pub top_contributors: Vec<RiskContributor>,
    pub method_version: String,
}

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::{DataMode, RiskContributor, RiskLevel};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BacktestSignalSource {
    RealHistory,
    FallbackTemplate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestRunRecord {
    pub run_id: String,
    pub entity_id: String,
    pub market_scope: String,
    pub data_mode: DataMode,
    pub point_in_time_mode: String,
    pub status: String,
    pub scenario_scope: Option<String>,
    pub from_date: NaiveDate,
    pub to_date: NaiveDate,
    pub history_points: u32,
    pub scenario_summary_count: u32,
    pub method_version: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestScenarioSummary {
    pub scenario_id: String,
    pub name: String,
    pub region: String,
    pub signal_source: BacktestSignalSource,
    pub crisis_start: NaiveDate,
    pub crisis_end: NaiveDate,
    pub first_l2_date: Option<NaiveDate>,
    pub first_l3_date: Option<NaiveDate>,
    pub max_level: RiskLevel,
    pub max_score: f64,
    pub lead_time_days: Option<i64>,
    pub false_positive_count: u32,
    pub missed: bool,
    pub history_start: Option<NaiveDate>,
    pub history_end: Option<NaiveDate>,
    pub history_point_count: u32,
    pub note: String,
    pub top_contributors: Vec<RiskContributor>,
    pub method_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestWindowPoint {
    pub as_of_date: NaiveDate,
    pub overall_score: f64,
    pub p_5d: f64,
    pub p_20d: f64,
    pub p_60d: f64,
    pub posture: crate::DecisionPosture,
    pub crisis_window_open: bool,
}

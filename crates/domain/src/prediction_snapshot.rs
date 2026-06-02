use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionSnapshotRecord {
    pub as_of_date: NaiveDate,
    pub entity_id: String,
    pub market_scope: String,
    pub release_id: Option<String>,
    pub probability_mode: String,
    pub release_status: String,
    pub point_in_time_mode: String,
    pub overall_score: f64,
    pub external_shock_score: f64,
    pub raw_p_5d: f64,
    pub raw_p_20d: f64,
    pub raw_p_60d: f64,
    pub calibrated_p_5d: f64,
    pub calibrated_p_20d: f64,
    pub calibrated_p_60d: f64,
    pub posture: String,
    pub time_to_risk_bucket: String,
    pub feature_set_version: String,
    pub label_version: String,
    pub coverage_score: f64,
    pub freshness_status: String,
    pub method_version: String,
    #[serde(default)]
    pub posture_trigger_codes: Vec<String>,
    #[serde(default)]
    pub posture_blocker_codes: Vec<String>,
    pub recorded_at: DateTime<Utc>,
}

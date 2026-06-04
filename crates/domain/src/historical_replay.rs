use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::ProbabilityDiagnostics;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalReplayRunRecord {
    pub replay_run_id: String,
    pub release_id: Option<String>,
    pub market_scope: String,
    pub from_date: NaiveDate,
    pub to_date: NaiveDate,
    pub history_cache_key: String,
    pub feature_set_version: String,
    pub label_version: String,
    pub point_in_time_mode: String,
    pub runtime_policy_version: String,
    pub action_playbook_version: String,
    pub protected_window_catalog_id: String,
    pub source_watermark: String,
    pub status: String,
    pub point_count: usize,
    pub failure_reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalAssessmentPointRecord {
    pub replay_run_id: String,
    pub entity_id: String,
    pub market_scope: String,
    pub release_id: Option<String>,
    pub as_of_date: NaiveDate,
    pub feature_snapshot_id: Option<String>,
    pub point_in_time_mode: String,
    pub runtime_policy_version: String,
    pub action_playbook_version: String,
    pub overall_score: f64,
    pub structural_score: f64,
    pub trigger_score: f64,
    pub external_shock_score: f64,
    pub raw_p_5d: f64,
    pub raw_p_20d: f64,
    pub raw_p_60d: f64,
    pub calibrated_p_5d: f64,
    pub calibrated_p_20d: f64,
    pub calibrated_p_60d: f64,
    pub posture: String,
    pub time_to_risk_bucket: String,
    pub actionability_prepare: f64,
    pub actionability_hedge: f64,
    pub actionability_defend: f64,
    #[serde(default)]
    pub probability_diagnostics: ProbabilityDiagnostics,
    #[serde(default)]
    pub posture_trigger_codes: Vec<String>,
    #[serde(default)]
    pub posture_blocker_codes: Vec<String>,
    pub coverage_score: f64,
    pub freshness_status: String,
    pub generated_at: DateTime<Utc>,
}

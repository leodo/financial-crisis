use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelReleaseManifest {
    pub release_id: String,
    pub market_scope: String,
    pub status: String,
    pub probability_mode: String,
    pub serving_status: String,
    pub bundle_uri: String,
    pub feature_set_version: String,
    pub label_version: String,
    pub prob_model_version: String,
    pub calibration_version: String,
    pub posture_policy_version: String,
    pub action_playbook_version: String,
    pub point_in_time_mode: String,
    pub training_range_start: Option<NaiveDate>,
    pub training_range_end: Option<NaiveDate>,
    pub calibration_range_start: Option<NaiveDate>,
    pub calibration_range_end: Option<NaiveDate>,
    pub evaluation_range_start: Option<NaiveDate>,
    pub evaluation_range_end: Option<NaiveDate>,
    pub brier_score: Option<f64>,
    pub log_loss: Option<f64>,
    pub ece: Option<f64>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelReleaseRecord {
    #[serde(flatten)]
    pub manifest: ModelReleaseManifest,
    pub created_at: DateTime<Utc>,
    pub activated_at: Option<DateTime<Utc>>,
    pub retired_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveModelPointer {
    pub market_scope: String,
    pub release_id: String,
    pub updated_at: DateTime<Utc>,
    pub updated_by: String,
}

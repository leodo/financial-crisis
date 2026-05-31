use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormalDatasetManifest {
    pub dataset_id: String,
    pub dataset_version: String,
    pub market_scope: String,
    pub feature_set_version: String,
    pub label_version: String,
    pub scenario_set_version: String,
    pub point_in_time_mode: String,
    pub from_date: Option<NaiveDate>,
    pub to_date: Option<NaiveDate>,
    pub train_end_date: Option<NaiveDate>,
    pub calibration_end_date: Option<NaiveDate>,
    pub evaluation_start_date: Option<NaiveDate>,
    pub row_count: usize,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormalDatasetRecord {
    #[serde(flatten)]
    pub manifest: FormalDatasetManifest,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormalDatasetRowRecord {
    pub dataset_key: String,
    pub split_name: String,
    pub entity_id: String,
    pub market_scope: String,
    pub as_of_date: NaiveDate,
    pub point_in_time_mode: String,
    pub latest_visible_at: Option<DateTime<Utc>>,
    pub coverage_score: f64,
    pub core_feature_coverage: f64,
    pub trigger_feature_coverage: f64,
    pub external_feature_coverage: f64,
    pub sample_quality_grade: String,
    pub primary_scenario_id: Option<String>,
    pub scenario_family: Option<String>,
    pub label_5d: u8,
    pub label_20d: u8,
    pub label_60d: u8,
    pub action_label_5d: u8,
    pub action_label_20d: u8,
    pub action_label_60d: u8,
    pub features: BTreeMap<String, f64>,
    pub created_at: DateTime<Utc>,
}

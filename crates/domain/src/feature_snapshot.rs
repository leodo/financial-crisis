use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureSnapshotRecord {
    pub as_of_date: NaiveDate,
    pub entity_id: String,
    pub market_scope: String,
    pub feature_set_version: String,
    pub point_in_time_mode: String,
    pub visibility_status: String,
    pub latest_visible_at: Option<DateTime<Utc>>,
    pub coverage_score: f64,
    pub core_feature_coverage: f64,
    pub trigger_feature_coverage: f64,
    pub external_feature_coverage: f64,
    pub feature_count: usize,
    pub features: BTreeMap<String, f64>,
    pub created_at: DateTime<Utc>,
}

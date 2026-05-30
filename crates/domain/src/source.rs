use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourcePriority {
    P0,
    P1,
    P2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceStatus {
    Healthy,
    Delayed,
    PartialFailure,
    Failed,
    Prototype,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceHealth {
    pub status: SourceStatus,
    pub last_success_at: Option<DateTime<Utc>>,
    pub lag_seconds: Option<i64>,
    pub consecutive_failures: u32,
    pub quality_score: f64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSource {
    pub source_id: String,
    pub display_name: String,
    pub source_type: String,
    pub priority: SourcePriority,
    pub access_method: String,
    pub documentation_url: Option<String>,
    pub production_allowed: bool,
    pub license_note: String,
    pub health: SourceHealth,
}

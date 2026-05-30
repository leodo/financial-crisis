use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{RiskContributor, RiskDimension, RiskLevel};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertType {
    RiskWatch,
    RiskStress,
    RiskWarning,
    RiskCrisis,
    DataQualityIssue,
    SourceHealthIssue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertStatus {
    Open,
    Acknowledged,
    Monitoring,
    Escalated,
    Deescalated,
    Resolved,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertEvent {
    pub alert_id: Uuid,
    pub event_type: AlertType,
    pub scope: String,
    pub entity_id: String,
    pub dimension: Option<RiskDimension>,
    pub level: RiskLevel,
    pub status: AlertStatus,
    pub triggered_at: DateTime<Utc>,
    pub triggered_as_of_date: NaiveDate,
    pub resolved_at: Option<DateTime<Utc>>,
    pub score: f64,
    pub previous_score: Option<f64>,
    pub trigger_reason: String,
    pub top_contributors: Vec<RiskContributor>,
    pub related_indicators: Vec<String>,
    pub method_version: String,
}

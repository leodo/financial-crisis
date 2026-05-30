use chrono::{DateTime, NaiveDate, Utc};
use fc_domain::{Frequency, Observation};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceDescriptor {
    pub source_id: String,
    pub display_name: String,
    pub capabilities: Vec<ConnectorCapability>,
    pub production_allowed: bool,
    pub license_note: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorCapability {
    Discover,
    Backfill,
    Incremental,
    RefreshMetadata,
    ParseRaw,
    Normalize,
    Validate,
    SupportsVintage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunMode {
    Discover,
    Backfill,
    Incremental,
    Repair,
    MetadataRefresh,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchPlan {
    pub source_id: String,
    pub dataset_id: String,
    pub target_id: String,
    pub run_mode: RunMode,
    pub requested_start: Option<NaiveDate>,
    pub requested_end: Option<NaiveDate>,
    pub frequency: Frequency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawPayload {
    pub raw_payload_id: Uuid,
    pub source_id: String,
    pub dataset_id: String,
    pub request_url: String,
    pub response_hash: String,
    pub content_type: String,
    pub body: String,
    pub fetched_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedBatch {
    pub raw_payload_id: Uuid,
    pub source_id: String,
    pub dataset_id: String,
    pub observations: Vec<Observation>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Error)]
pub enum ConnectorError {
    #[error("rate limited by source")]
    RateLimited,
    #[error("temporary network error: {0}")]
    TemporaryNetwork(String),
    #[error("source unavailable: {0}")]
    SourceUnavailable(String),
    #[error("authentication failed")]
    AuthFailed,
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("schema changed: {0}")]
    SchemaChanged(String),
    #[error("quality gate failed: {0}")]
    QualityFailed(String),
    #[error("license blocked: {0}")]
    LicenseBlocked(String),
    #[error("parse error: {0}")]
    Parse(String),
}

#[async_trait::async_trait]
pub trait Connector: Send + Sync {
    fn describe(&self) -> SourceDescriptor;

    async fn fetch(&self, plan: &FetchPlan) -> Result<RawPayload, ConnectorError>;

    fn parse(
        &self,
        plan: &FetchPlan,
        payload: &RawPayload,
    ) -> Result<NormalizedBatch, ConnectorError>;
}

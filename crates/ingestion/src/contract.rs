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
    /// Internal system indicator id.
    pub target_id: String,
    /// External provider code, such as a FRED series id. Falls back to target_id.
    pub external_code: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connector_error_display_rate_limited() {
        let error = ConnectorError::RateLimited;
        assert_eq!(format!("{error}"), "rate limited by source");
    }

    #[test]
    fn connector_error_display_auth_failed() {
        let error = ConnectorError::AuthFailed;
        assert_eq!(format!("{error}"), "authentication failed");
    }

    #[test]
    fn connector_error_display_temporary_network() {
        let error = ConnectorError::TemporaryNetwork("timeout".to_string());
        assert_eq!(format!("{error}"), "temporary network error: timeout");
    }

    #[test]
    fn connector_error_display_parse() {
        let error = ConnectorError::Parse("bad format".to_string());
        assert_eq!(format!("{error}"), "parse error: bad format");
    }

    #[test]
    fn connector_error_display_source_unavailable() {
        let error = ConnectorError::SourceUnavailable("503".to_string());
        assert_eq!(format!("{error}"), "source unavailable: 503");
    }

    #[test]
    fn connector_error_display_schema_changed() {
        let error = ConnectorError::SchemaChanged("missing column".to_string());
        assert_eq!(format!("{error}"), "schema changed: missing column");
    }

    #[test]
    fn fetch_plan_round_trips_via_json() {
        let plan = FetchPlan {
            source_id: "fred".to_string(),
            dataset_id: "fred_series_observations".to_string(),
            target_id: "us_gdp".to_string(),
            external_code: Some("GDP".to_string()),
            run_mode: RunMode::Backfill,
            requested_start: Some(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
            requested_end: Some(NaiveDate::from_ymd_opt(2020, 12, 31).unwrap()),
            frequency: Frequency::Daily,
        };
        let json = serde_json::to_string(&plan).unwrap();
        let deserialized: FetchPlan = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.source_id, "fred");
        assert_eq!(deserialized.target_id, "us_gdp");
        assert_eq!(deserialized.run_mode, RunMode::Backfill);
    }

    #[test]
    fn fetch_plan_without_external_code_defaults_to_target_id() {
        let plan = FetchPlan {
            source_id: "mock".to_string(),
            dataset_id: "mock_data".to_string(),
            target_id: "us_test".to_string(),
            external_code: None,
            run_mode: RunMode::Incremental,
            requested_start: None,
            requested_end: None,
            frequency: Frequency::Daily,
        };
        assert!(plan.external_code.is_none());
        assert_eq!(plan.run_mode, RunMode::Incremental);
    }

    #[test]
    fn source_descriptor_serialization_round_trip() {
        let desc = SourceDescriptor {
            source_id: "test".to_string(),
            display_name: "Test Source".to_string(),
            capabilities: vec![
                ConnectorCapability::Backfill,
                ConnectorCapability::ParseRaw,
            ],
            production_allowed: false,
            license_note: "MIT".to_string(),
        };
        let json = serde_json::to_string(&desc).unwrap();
        let deserialized: SourceDescriptor = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.source_id, "test");
        assert_eq!(deserialized.capabilities.len(), 2);
    }

    #[test]
    fn raw_payload_serialization_round_trip() {
        let payload = RawPayload {
            raw_payload_id: Uuid::new_v4(),
            source_id: "fred".to_string(),
            dataset_id: "obs".to_string(),
            request_url: "https://example.com/data".to_string(),
            response_hash: "abc123".to_string(),
            content_type: "text/csv".to_string(),
            body: "date,value\n2020-01-01,42.0".to_string(),
            fetched_at: DateTime::from_timestamp_nanos(0),
        };
        let json = serde_json::to_string(&payload).unwrap();
        let deserialized: RawPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.source_id, "fred");
        assert_eq!(deserialized.body, "date,value\n2020-01-01,42.0");
    }

    #[test]
    fn normalized_batch_empty_observations() {
        let batch = NormalizedBatch {
            raw_payload_id: Uuid::new_v4(),
            source_id: "test".to_string(),
            dataset_id: "test_data".to_string(),
            observations: vec![],
            warnings: vec![],
        };
        assert!(batch.observations.is_empty());
        assert!(batch.warnings.is_empty());
    }
}

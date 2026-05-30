use chrono::{NaiveDate, Utc};
use fc_domain::{Frequency, Observation};
use uuid::Uuid;

use crate::{
    Connector, ConnectorCapability, ConnectorError, FetchPlan, NormalizedBatch, RawPayload,
    SourceDescriptor,
};

#[derive(Debug, Clone, Default)]
pub struct MockConnector;

#[async_trait::async_trait]
impl Connector for MockConnector {
    fn describe(&self) -> SourceDescriptor {
        SourceDescriptor {
            source_id: "mock".to_string(),
            display_name: "Mock Connector".to_string(),
            capabilities: vec![
                ConnectorCapability::Backfill,
                ConnectorCapability::Incremental,
                ConnectorCapability::ParseRaw,
                ConnectorCapability::Normalize,
            ],
            production_allowed: false,
            license_note: "Development-only synthetic data.".to_string(),
        }
    }

    async fn fetch(&self, plan: &FetchPlan) -> Result<RawPayload, ConnectorError> {
        let body = serde_json::json!({
            "indicator_id": plan.target_id,
            "value": 42.0,
            "as_of_date": plan.requested_end.unwrap_or_else(|| NaiveDate::from_ymd_opt(2026, 5, 30).expect("valid date")),
        })
        .to_string();
        Ok(RawPayload {
            raw_payload_id: Uuid::new_v4(),
            source_id: plan.source_id.clone(),
            dataset_id: plan.dataset_id.clone(),
            request_url: "mock://local".to_string(),
            response_hash: format!("mock-{}", body.len()),
            content_type: "application/json".to_string(),
            body,
            fetched_at: Utc::now(),
        })
    }

    fn parse(
        &self,
        plan: &FetchPlan,
        payload: &RawPayload,
    ) -> Result<NormalizedBatch, ConnectorError> {
        let value: serde_json::Value = serde_json::from_str(&payload.body)
            .map_err(|error| ConnectorError::Parse(error.to_string()))?;
        let as_of_date = value
            .get("as_of_date")
            .and_then(|value| value.as_str())
            .and_then(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d").ok())
            .ok_or_else(|| ConnectorError::SchemaChanged("missing as_of_date".to_string()))?;
        let metric_value = value
            .get("value")
            .and_then(|value| value.as_f64())
            .ok_or_else(|| ConnectorError::SchemaChanged("missing value".to_string()))?;
        Ok(NormalizedBatch {
            raw_payload_id: payload.raw_payload_id,
            source_id: payload.source_id.clone(),
            dataset_id: payload.dataset_id.clone(),
            observations: vec![Observation {
                indicator_id: plan.target_id.clone(),
                entity_id: "us".to_string(),
                as_of_date,
                period_start: Some(as_of_date),
                period_end: Some(as_of_date),
                frequency: Frequency::Daily,
                value: metric_value,
                unit: "index".to_string(),
                source_id: payload.source_id.clone(),
                dataset_id: payload.dataset_id.clone(),
                revision_time: None,
                publication_time: Some(payload.fetched_at),
                quality_score: 70.0,
                quality_flags: vec!["prototype_source".to_string()],
            }],
            warnings: vec!["mock connector emits synthetic data".to_string()],
        })
    }
}

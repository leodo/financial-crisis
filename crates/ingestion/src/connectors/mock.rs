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

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use fc_domain::Frequency;
    use uuid::Uuid;

    use crate::{Connector, ConnectorError, FetchPlan, RunMode};

    use super::MockConnector;

    fn test_plan() -> FetchPlan {
        FetchPlan {
            source_id: "mock".to_string(),
            dataset_id: "mock_data".to_string(),
            target_id: "us_test".to_string(),
            external_code: None,
            run_mode: RunMode::Incremental,
            requested_start: None,
            requested_end: Some(NaiveDate::from_ymd_opt(2026, 5, 30).unwrap()),
            frequency: Frequency::Daily,
        }
    }

    #[tokio::test]
    async fn mock_connector_fetch_returns_valid_payload() {
        let connector = MockConnector;
        let plan = test_plan();
        let payload = connector.fetch(&plan).await.unwrap();
        assert_eq!(payload.source_id, "mock");
        assert!(payload.body.contains("value"));
    }

    #[tokio::test]
    async fn mock_connector_parse_returns_valid_batch() {
        let connector = MockConnector;
        let plan = test_plan();
        let payload = connector.fetch(&plan).await.unwrap();
        let batch = connector.parse(&plan, &payload).unwrap();
        assert_eq!(batch.observations.len(), 1);
        assert_eq!(batch.observations[0].indicator_id, "us_test");
    }

    #[test]
    fn mock_connector_parse_rejects_empty_json() {
        let connector = MockConnector;
        let payload = crate::RawPayload {
            raw_payload_id: Uuid::new_v4(),
            source_id: "mock".to_string(),
            dataset_id: "test".to_string(),
            request_url: "mock://local".to_string(),
            response_hash: "empty".to_string(),
            content_type: "application/json".to_string(),
            body: "{}".to_string(),
            fetched_at: chrono::Utc::now(),
        };
        let plan = test_plan();
        let result = connector.parse(&plan, &payload);
        assert!(result.is_err());
        match result.unwrap_err() {
            ConnectorError::SchemaChanged(_) => {} // expected: missing as_of_date
            other => panic!("expected SchemaChanged, got {other}"),
        }
    }

    #[test]
    fn mock_connector_parse_rejects_invalid_json() {
        let connector = MockConnector;
        let payload = crate::RawPayload {
            raw_payload_id: Uuid::new_v4(),
            source_id: "mock".to_string(),
            dataset_id: "test".to_string(),
            request_url: "mock://local".to_string(),
            response_hash: "bad".to_string(),
            content_type: "application/json".to_string(),
            body: "{invalid json}".to_string(),
            fetched_at: chrono::Utc::now(),
        };
        let plan = test_plan();
        let result = connector.parse(&plan, &payload);
        assert!(result.is_err());
        match result.unwrap_err() {
            ConnectorError::Parse(_) => {} // expected: parse error
            other => panic!("expected Parse, got {other}"),
        }
    }

    #[test]
    fn mock_connector_parse_rejects_missing_value() {
        let connector = MockConnector;
        let payload = crate::RawPayload {
            raw_payload_id: Uuid::new_v4(),
            source_id: "mock".to_string(),
            dataset_id: "test".to_string(),
            request_url: "mock://local".to_string(),
            response_hash: "no-val".to_string(),
            content_type: "application/json".to_string(),
            body: r#"{"as_of_date": "2026-05-30"}"#.to_string(),
            fetched_at: chrono::Utc::now(),
        };
        let plan = test_plan();
        let result = connector.parse(&plan, &payload);
        assert!(result.is_err());
        match result.unwrap_err() {
            ConnectorError::SchemaChanged(_) => {} // expected: missing value
            other => panic!("expected SchemaChanged, got {other}"),
        }
    }

    #[test]
    fn mock_connector_describe_returns_prototype() {
        let connector = MockConnector;
        let desc = connector.describe();
        assert_eq!(desc.source_id, "mock");
        assert!(!desc.production_allowed);
    }
}

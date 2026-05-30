use chrono::{NaiveDate, Utc};
use fc_domain::Observation;
use serde::Deserialize;
use url::Url;
use uuid::Uuid;

use crate::{
    Connector, ConnectorCapability, ConnectorError, FetchPlan, NormalizedBatch, RawPayload,
    SourceDescriptor,
};

#[derive(Debug, Clone)]
pub struct FredConnector {
    client: reqwest::Client,
    base_url: Url,
    api_key: Option<String>,
}

impl FredConnector {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: Url::parse("https://api.stlouisfed.org/fred/").expect("valid FRED base URL"),
            api_key,
        }
    }

    pub fn build_series_observations_url(
        &self,
        series_id: &str,
        start: Option<NaiveDate>,
        end: Option<NaiveDate>,
    ) -> Result<Url, ConnectorError> {
        let mut url = self
            .base_url
            .join("series/observations")
            .map_err(|error| ConnectorError::InvalidRequest(error.to_string()))?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("series_id", series_id);
            query.append_pair("file_type", "json");
            if let Some(api_key) = &self.api_key {
                query.append_pair("api_key", api_key);
            }
            if let Some(start) = start {
                query.append_pair("observation_start", &start.to_string());
            }
            if let Some(end) = end {
                query.append_pair("observation_end", &end.to_string());
            }
        }
        Ok(url)
    }
}

#[async_trait::async_trait]
impl Connector for FredConnector {
    fn describe(&self) -> SourceDescriptor {
        SourceDescriptor {
            source_id: "fred".to_string(),
            display_name: "FRED".to_string(),
            capabilities: vec![
                ConnectorCapability::Backfill,
                ConnectorCapability::Incremental,
                ConnectorCapability::ParseRaw,
                ConnectorCapability::Normalize,
                ConnectorCapability::SupportsVintage,
            ],
            production_allowed: true,
            license_note: "Use according to FRED API terms and source-specific notes.".to_string(),
        }
    }

    async fn fetch(&self, plan: &FetchPlan) -> Result<RawPayload, ConnectorError> {
        let url = self.build_series_observations_url(
            &plan.target_id,
            plan.requested_start,
            plan.requested_end,
        )?;
        let response = self
            .client
            .get(url.clone())
            .send()
            .await
            .map_err(|error| ConnectorError::TemporaryNetwork(error.to_string()))?;
        let status = response.status();
        if status.as_u16() == 429 {
            return Err(ConnectorError::RateLimited);
        }
        if status.as_u16() == 401 || status.as_u16() == 403 {
            return Err(ConnectorError::AuthFailed);
        }
        if status.is_server_error() {
            return Err(ConnectorError::SourceUnavailable(status.to_string()));
        }
        if !status.is_success() {
            return Err(ConnectorError::InvalidRequest(status.to_string()));
        }
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("application/json")
            .to_string();
        let body = response
            .text()
            .await
            .map_err(|error| ConnectorError::TemporaryNetwork(error.to_string()))?;
        Ok(RawPayload {
            raw_payload_id: Uuid::new_v4(),
            source_id: plan.source_id.clone(),
            dataset_id: plan.dataset_id.clone(),
            request_url: url.to_string(),
            response_hash: simple_hash(&body),
            content_type,
            body,
            fetched_at: Utc::now(),
        })
    }

    fn parse(
        &self,
        plan: &FetchPlan,
        payload: &RawPayload,
    ) -> Result<NormalizedBatch, ConnectorError> {
        let parsed: FredObservationsResponse = serde_json::from_str(&payload.body)
            .map_err(|error| ConnectorError::Parse(error.to_string()))?;
        let mut observations = Vec::with_capacity(parsed.observations.len());
        let mut warnings = Vec::new();
        for item in parsed.observations {
            if item.value == "." {
                warnings.push(format!("missing FRED value for {}", item.date));
                continue;
            }
            let value = item
                .value
                .parse::<f64>()
                .map_err(|error| ConnectorError::Parse(error.to_string()))?;
            let as_of_date = NaiveDate::parse_from_str(&item.date, "%Y-%m-%d")
                .map_err(|error| ConnectorError::Parse(error.to_string()))?;
            observations.push(Observation {
                indicator_id: plan.target_id.clone(),
                entity_id: "us".to_string(),
                as_of_date,
                period_start: Some(as_of_date),
                period_end: Some(as_of_date),
                frequency: plan.frequency,
                value,
                unit: "source_unit".to_string(),
                source_id: payload.source_id.clone(),
                dataset_id: payload.dataset_id.clone(),
                revision_time: parse_fred_date(&item.realtime_end),
                publication_time: Some(payload.fetched_at),
                quality_score: 95.0,
                quality_flags: Vec::new(),
            });
        }
        Ok(NormalizedBatch {
            raw_payload_id: payload.raw_payload_id,
            source_id: payload.source_id.clone(),
            dataset_id: payload.dataset_id.clone(),
            observations,
            warnings,
        })
    }
}

#[derive(Debug, Deserialize)]
struct FredObservationsResponse {
    observations: Vec<FredObservation>,
}

#[derive(Debug, Deserialize)]
struct FredObservation {
    realtime_end: String,
    date: String,
    value: String,
}

fn parse_fred_date(value: &str) -> Option<chrono::DateTime<Utc>> {
    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()?;
    date.and_hms_opt(0, 0, 0)
        .map(|datetime| chrono::DateTime::<Utc>::from_naive_utc_and_offset(datetime, Utc))
}

fn simple_hash(input: &str) -> String {
    let hash = input.as_bytes().iter().fold(0_u64, |acc, byte| {
        acc.wrapping_mul(31).wrapping_add(*byte as u64)
    });
    format!("{hash:x}")
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::FredConnector;

    #[test]
    fn builds_observations_url() {
        let connector = FredConnector::new(Some("secret".to_string()));
        let url = connector
            .build_series_observations_url(
                "VIXCLS",
                Some(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
                Some(NaiveDate::from_ymd_opt(2020, 1, 31).unwrap()),
            )
            .unwrap();
        let url = url.as_str();
        assert!(url.contains("series_id=VIXCLS"));
        assert!(url.contains("file_type=json"));
        assert!(url.contains("observation_start=2020-01-01"));
        assert!(url.contains("observation_end=2020-01-31"));
    }
}

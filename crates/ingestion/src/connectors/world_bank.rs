use std::time::Duration;

use chrono::{Datelike, NaiveDate, Utc};
use fc_domain::Observation;
use serde::Deserialize;
use url::Url;
use uuid::Uuid;

use crate::{
    http_client, Connector, ConnectorCapability, ConnectorError, FetchPlan, NormalizedBatch,
    RawPayload, SourceDescriptor,
};

#[derive(Debug, Clone)]
pub struct WorldBankConnector {
    client: reqwest::Client,
    base_url: Url,
}

impl WorldBankConnector {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent(http_client::user_agent())
                .http1_only()
                .no_proxy()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("valid World Bank reqwest client"),
            base_url: Url::parse("https://api.worldbank.org/v2/")
                .expect("valid World Bank API base URL"),
        }
    }

    pub fn build_indicator_url(
        &self,
        country_code: &str,
        indicator_code: &str,
        start: Option<NaiveDate>,
        end: Option<NaiveDate>,
    ) -> Result<Url, ConnectorError> {
        let mut url = self
            .base_url
            .join(&format!(
                "country/{country_code}/indicator/{indicator_code}"
            ))
            .map_err(|error| ConnectorError::InvalidRequest(error.to_string()))?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("format", "json");
            query.append_pair("per_page", "20000");
            if let (Some(start), Some(end)) = (start, end) {
                query.append_pair("date", &format!("{}:{}", start.year(), end.year()));
            }
        }
        Ok(url)
    }
}

impl Default for WorldBankConnector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Connector for WorldBankConnector {
    fn describe(&self) -> SourceDescriptor {
        SourceDescriptor {
            source_id: "world_bank".to_string(),
            display_name: "World Bank Indicators".to_string(),
            capabilities: vec![
                ConnectorCapability::Backfill,
                ConnectorCapability::Incremental,
                ConnectorCapability::ParseRaw,
                ConnectorCapability::Normalize,
            ],
            production_allowed: true,
            license_note: "Official World Bank Indicators API; no API key required.".to_string(),
        }
    }

    async fn fetch(&self, plan: &FetchPlan) -> Result<RawPayload, ConnectorError> {
        let code = plan.external_code.as_deref().unwrap_or(&plan.target_id);
        let (country_code, indicator_code) = split_world_bank_code(code)?;
        let url = self.build_indicator_url(
            country_code,
            indicator_code,
            plan.requested_start,
            plan.requested_end,
        )?;
        let response = self
            .client
            .get(url.clone())
            .send()
            .await
            .map_err(|error| ConnectorError::TemporaryNetwork(format!("{error:?}")));
        let (content_type, body) = match response {
            Ok(response) => {
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
                    .map_err(|error| ConnectorError::TemporaryNetwork(format!("{error:?}")))?;
                (content_type, body)
            }
            Err(error) => {
                tracing::warn!(%error, "reqwest failed; falling back to curl");
                (
                    "application/json".to_string(),
                    http_client::curl_get_text(&url, 60)?,
                )
            }
        };
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
        let code = plan.external_code.as_deref().unwrap_or(&plan.target_id);
        let (country_code, _) = split_world_bank_code(code)?;
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&payload.body)
            .map_err(|error| ConnectorError::Parse(error.to_string()))?;
        if parsed.len() < 2 {
            return Err(ConnectorError::SchemaChanged(
                "World Bank payload missing data array".to_string(),
            ));
        }

        let rows: Vec<WorldBankObservation> = serde_json::from_value(parsed[1].clone())
            .map_err(|error| ConnectorError::Parse(error.to_string()))?;
        let mut observations = Vec::new();
        let mut warnings = Vec::new();
        for row in rows {
            let Some(value) = row.value else {
                warnings.push(format!("missing World Bank value for {}", row.date));
                continue;
            };
            let year = row
                .date
                .parse::<i32>()
                .map_err(|error| ConnectorError::Parse(error.to_string()))?;
            let Some(as_of_date) = NaiveDate::from_ymd_opt(year, 12, 31) else {
                return Err(ConnectorError::Parse(format!(
                    "invalid World Bank year {}",
                    row.date
                )));
            };
            if plan.requested_start.is_some_and(|start| as_of_date < start)
                || plan.requested_end.is_some_and(|end| as_of_date > end)
            {
                continue;
            }
            observations.push(Observation {
                indicator_id: plan.target_id.clone(),
                entity_id: world_bank_entity(country_code),
                as_of_date,
                period_start: Some(as_of_date),
                period_end: Some(as_of_date),
                frequency: plan.frequency,
                value,
                unit: "source_unit".to_string(),
                source_id: payload.source_id.clone(),
                dataset_id: payload.dataset_id.clone(),
                revision_time: None,
                publication_time: Some(payload.fetched_at),
                quality_score: 90.0,
                quality_flags: vec!["world_bank_annual_series".to_string()],
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
struct WorldBankObservation {
    date: String,
    value: Option<f64>,
}

fn split_world_bank_code(code: &str) -> Result<(&str, &str), ConnectorError> {
    let Some((country_code, indicator_code)) = code.split_once("__") else {
        return Err(ConnectorError::InvalidRequest(format!(
            "World Bank external code must be COUNTRY__INDICATOR, got {code}"
        )));
    };
    Ok((country_code, indicator_code))
}

fn world_bank_entity(country_code: &str) -> String {
    if country_code.eq_ignore_ascii_case("US") {
        "us".to_string()
    } else {
        country_code.to_ascii_lowercase()
    }
}

fn simple_hash(input: &str) -> String {
    let hash = input.as_bytes().iter().fold(0_u64, |acc, byte| {
        acc.wrapping_mul(31).wrapping_add(*byte as u64)
    });
    format!("{hash:x}")
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, Utc};
    use fc_domain::Frequency;
    use uuid::Uuid;

    use crate::{Connector, FetchPlan, RawPayload, RunMode};

    use super::WorldBankConnector;

    #[test]
    fn builds_world_bank_indicator_url() {
        let connector = WorldBankConnector::new();
        let url = connector
            .build_indicator_url(
                "US",
                "NY.GDP.MKTP.KD.ZG",
                Some(NaiveDate::from_ymd_opt(1960, 1, 1).unwrap()),
                Some(NaiveDate::from_ymd_opt(2024, 12, 31).unwrap()),
            )
            .unwrap();
        let text = url.as_str();
        assert!(text.contains("country/US/indicator/NY.GDP.MKTP.KD.ZG"));
        assert!(text.contains("format=json"));
        assert!(text.contains("date=1960%3A2024"));
    }

    #[test]
    fn parses_world_bank_annual_values() {
        let connector = WorldBankConnector::new();
        let plan = FetchPlan {
            source_id: "world_bank".to_string(),
            dataset_id: "world_bank_country_indicators".to_string(),
            target_id: "global_macro_gdp_growth".to_string(),
            external_code: Some("US__NY.GDP.MKTP.KD.ZG".to_string()),
            run_mode: RunMode::Backfill,
            requested_start: Some(NaiveDate::from_ymd_opt(2022, 1, 1).unwrap()),
            requested_end: Some(NaiveDate::from_ymd_opt(2024, 12, 31).unwrap()),
            frequency: Frequency::Annual,
        };
        let payload = RawPayload {
            raw_payload_id: Uuid::new_v4(),
            source_id: "world_bank".to_string(),
            dataset_id: "world_bank_country_indicators".to_string(),
            request_url: "https://api.worldbank.org/v2/country/US/indicator/NY.GDP.MKTP.KD.ZG?format=json"
                .to_string(),
            response_hash: "hash".to_string(),
            content_type: "application/json".to_string(),
            body: r#"[{"page":1,"pages":1},[{"date":"2024","value":2.8},{"date":"2023","value":2.5},{"date":"2022","value":null}]]"#.to_string(),
            fetched_at: Utc::now(),
        };

        let batch = connector.parse(&plan, &payload).unwrap();

        assert_eq!(batch.observations.len(), 2);
        assert_eq!(batch.observations[0].entity_id, "us");
        assert_eq!(batch.warnings.len(), 1);
    }
}

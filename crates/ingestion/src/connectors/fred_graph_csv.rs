use std::time::Duration;

use chrono::{NaiveDate, Utc};
use fc_domain::Observation;
use url::Url;
use uuid::Uuid;

use crate::{
    http_client, Connector, ConnectorCapability, ConnectorError, FetchPlan, NormalizedBatch,
    RawPayload, SourceDescriptor,
};

#[derive(Debug, Clone)]
pub struct FredGraphCsvConnector {
    client: reqwest::Client,
    base_url: Url,
}

impl FredGraphCsvConnector {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent(http_client::user_agent())
                .http1_only()
                .no_proxy()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("valid FRED graph CSV reqwest client"),
            base_url: Url::parse("https://fred.stlouisfed.org/graph/")
                .expect("valid FRED graph base URL"),
        }
    }

    pub fn build_graph_csv_url(&self, series_id: &str) -> Result<Url, ConnectorError> {
        let mut url = self
            .base_url
            .join("fredgraph.csv")
            .map_err(|error| ConnectorError::InvalidRequest(error.to_string()))?;
        url.query_pairs_mut().append_pair("id", series_id);
        Ok(url)
    }
}

impl Default for FredGraphCsvConnector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Connector for FredGraphCsvConnector {
    fn describe(&self) -> SourceDescriptor {
        SourceDescriptor {
            source_id: "fred".to_string(),
            display_name: "FRED Graph CSV".to_string(),
            capabilities: vec![
                ConnectorCapability::Backfill,
                ConnectorCapability::Incremental,
                ConnectorCapability::ParseRaw,
                ConnectorCapability::Normalize,
            ],
            production_allowed: true,
            license_note: "Uses FRED public graph CSV downloads; observe source-specific notes and cache locally."
                .to_string(),
        }
    }

    async fn fetch(&self, plan: &FetchPlan) -> Result<RawPayload, ConnectorError> {
        let series_id = plan.external_code.as_deref().unwrap_or(&plan.target_id);
        let url = self.build_graph_csv_url(series_id)?;
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
                    .unwrap_or("text/csv")
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
                    "text/csv".to_string(),
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
        let series_id = plan.external_code.as_deref().unwrap_or(&plan.target_id);
        let mut reader = csv::ReaderBuilder::new()
            .flexible(true)
            .from_reader(payload.body.as_bytes());
        let headers = reader
            .headers()
            .map_err(|error| ConnectorError::Parse(error.to_string()))?
            .clone();
        let date_index = headers
            .iter()
            .position(|header| header == "observation_date")
            .ok_or_else(|| {
                ConnectorError::SchemaChanged("missing observation_date column".to_string())
            })?;
        let value_index = headers
            .iter()
            .position(|header| header.eq_ignore_ascii_case(series_id))
            .or_else(|| (headers.len() == 2).then_some(1))
            .ok_or_else(|| {
                ConnectorError::SchemaChanged(format!("missing value column for {series_id}"))
            })?;

        let mut observations = Vec::new();
        let mut warnings = Vec::new();
        for record in reader.records() {
            let record = record.map_err(|error| ConnectorError::Parse(error.to_string()))?;
            let date_text = record
                .get(date_index)
                .ok_or_else(|| ConnectorError::Parse("missing date value".to_string()))?;
            let as_of_date = NaiveDate::parse_from_str(date_text, "%Y-%m-%d")
                .map_err(|error| ConnectorError::Parse(error.to_string()))?;
            if plan.requested_start.is_some_and(|start| as_of_date < start)
                || plan.requested_end.is_some_and(|end| as_of_date > end)
            {
                continue;
            }

            let raw_value = record.get(value_index).unwrap_or("").trim();
            if raw_value.is_empty() || raw_value == "." {
                warnings.push(format!(
                    "missing FRED CSV value for {series_id} on {as_of_date}"
                ));
                continue;
            }
            let value = raw_value
                .parse::<f64>()
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
                revision_time: None,
                publication_time: Some(payload.fetched_at),
                quality_score: 92.0,
                quality_flags: vec!["fred_graph_csv_no_vintage".to_string()],
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

    use super::FredGraphCsvConnector;

    #[test]
    fn builds_graph_csv_url_without_api_key() {
        let connector = FredGraphCsvConnector::new();
        let url = connector.build_graph_csv_url("VIXCLS").unwrap();
        assert_eq!(
            url.as_str(),
            "https://fred.stlouisfed.org/graph/fredgraph.csv?id=VIXCLS"
        );
    }

    #[test]
    fn parses_graph_csv_and_filters_requested_dates() {
        let connector = FredGraphCsvConnector::new();
        let plan = FetchPlan {
            source_id: "fred".to_string(),
            dataset_id: "fred_series_observations".to_string(),
            target_id: "us_market_vix_close".to_string(),
            external_code: Some("VIXCLS".to_string()),
            run_mode: RunMode::Backfill,
            requested_start: Some(NaiveDate::from_ymd_opt(2020, 1, 2).unwrap()),
            requested_end: Some(NaiveDate::from_ymd_opt(2020, 1, 3).unwrap()),
            frequency: Frequency::Daily,
        };
        let payload = RawPayload {
            raw_payload_id: Uuid::new_v4(),
            source_id: "fred".to_string(),
            dataset_id: "fred_series_observations".to_string(),
            request_url: "https://fred.stlouisfed.org/graph/fredgraph.csv?id=VIXCLS".to_string(),
            response_hash: "hash".to_string(),
            content_type: "text/csv".to_string(),
            body: "observation_date,VIXCLS\n2020-01-01,12.47\n2020-01-02,12.85\n2020-01-03,.\n"
                .to_string(),
            fetched_at: Utc::now(),
        };

        let batch = connector.parse(&plan, &payload).unwrap();

        assert_eq!(batch.observations.len(), 1);
        assert_eq!(batch.observations[0].value, 12.85);
        assert_eq!(batch.warnings.len(), 1);
    }
}

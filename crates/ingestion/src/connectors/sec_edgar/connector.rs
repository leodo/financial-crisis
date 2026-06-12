use std::{collections::HashSet, time::Duration};

use chrono::{NaiveDate, Utc};
use url::Url;
use uuid::Uuid;

use crate::{http_client, ConnectorError, RawPayload};

use super::aggregate::{build_alerts, build_daily_aggregates, build_observations};
use super::parse::{archive_overlaps, parse_filing_arrays};
use super::types::{
    CompanyBackfill, FilingArrays, SecEdgarBackfill, SecInstitution, SubmissionsEnvelope,
    SEC_INSTITUTIONS,
};
use super::{SEC_REQUEST_DELAY_MILLIS, SEC_SUBMISSIONS_DATASET_ID, SEC_USER_AGENT};

#[derive(Debug, Clone)]
pub struct SecEdgarConnector {
    client: reqwest::Client,
    base_url: Url,
}

impl SecEdgarConnector {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent(SEC_USER_AGENT)
                .http1_only()
                .no_proxy()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("valid SEC reqwest client"),
            base_url: Url::parse("https://data.sec.gov/submissions/")
                .expect("valid SEC submissions base URL"),
        }
    }

    pub fn institutions(&self) -> &'static [SecInstitution] {
        SEC_INSTITUTIONS
    }

    pub async fn backfill_range(
        &self,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<SecEdgarBackfill, ConnectorError> {
        if start > end {
            return Err(ConnectorError::InvalidRequest(
                "SEC backfill start must be on or before end".to_string(),
            ));
        }

        let mut payloads = Vec::new();
        let mut filings = Vec::new();
        for institution in self.institutions() {
            let company = self.fetch_company_filings(*institution, start, end).await?;
            payloads.extend(company.payloads);
            filings.extend(company.filings);
        }

        let daily = build_daily_aggregates(start, end, &filings);
        let fetched_at = Utc::now();
        let observations = build_observations(&daily, fetched_at);
        let alerts = build_alerts(&daily, end);
        let latest_filing_date = filings.iter().map(|filing| filing.filing_date).max();

        Ok(SecEdgarBackfill {
            payloads,
            observations,
            alerts,
            latest_filing_date,
            company_count: self.institutions().len(),
            filing_count: filings.len(),
        })
    }

    async fn fetch_company_filings(
        &self,
        institution: SecInstitution,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<CompanyBackfill, ConnectorError> {
        let main_url = self
            .base_url
            .join(&format!("CIK{}.json", institution.cik))
            .map_err(|error| ConnectorError::InvalidRequest(error.to_string()))?;
        let main_payload = self
            .fetch_json(&main_url, SEC_SUBMISSIONS_DATASET_ID)
            .await?;
        let envelope: SubmissionsEnvelope = serde_json::from_str(&main_payload.body)
            .map_err(|error| ConnectorError::Parse(error.to_string()))?;

        let mut payloads = vec![main_payload];
        let mut filings = parse_filing_arrays(institution, &envelope.filings.recent, start, end)?;

        let mut archive_files = envelope
            .filings
            .files
            .into_iter()
            .filter(|file| archive_overlaps(file, start, end))
            .collect::<Vec<_>>();
        archive_files.sort_by(|a, b| a.filing_from.cmp(&b.filing_from));

        for file in archive_files {
            tokio::time::sleep(Duration::from_millis(SEC_REQUEST_DELAY_MILLIS)).await;
            let archive_url = self
                .base_url
                .join(&file.name)
                .map_err(|error| ConnectorError::InvalidRequest(error.to_string()))?;
            let archive_payload = self
                .fetch_json(&archive_url, SEC_SUBMISSIONS_DATASET_ID)
                .await?;
            let archived_arrays: FilingArrays = serde_json::from_str(&archive_payload.body)
                .map_err(|error| ConnectorError::Parse(error.to_string()))?;
            filings.extend(parse_filing_arrays(
                institution,
                &archived_arrays,
                start,
                end,
            )?);
            payloads.push(archive_payload);
        }

        let mut seen = HashSet::new();
        filings.retain(|filing| seen.insert(filing.accession_number.clone()));
        filings.sort_by_key(|filing| (filing.filing_date, filing.acceptance_time));

        Ok(CompanyBackfill { payloads, filings })
    }

    async fn fetch_json(&self, url: &Url, dataset_id: &str) -> Result<RawPayload, ConnectorError> {
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
                if status.as_u16() == 401 {
                    return Err(ConnectorError::AuthFailed);
                }
                if status.as_u16() == 403 {
                    return Err(ConnectorError::SourceUnavailable(status.to_string()));
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
                let body = match response.text().await {
                    Ok(body) => body,
                    Err(error) => {
                        tracing::warn!(
                            %error,
                            "SEC response body read failed; falling back to curl"
                        );
                        http_client::curl_get_text(url, 45)?
                    }
                };
                (content_type, body)
            }
            Err(error) => {
                tracing::warn!(%error, "SEC reqwest failed; falling back to curl");
                (
                    "application/json".to_string(),
                    http_client::curl_get_text(url, 45)?,
                )
            }
        };

        Ok(RawPayload {
            raw_payload_id: Uuid::new_v4(),
            source_id: "sec_edgar".to_string(),
            dataset_id: dataset_id.to_string(),
            request_url: url.to_string(),
            response_hash: simple_hash(&body),
            content_type,
            body,
            fetched_at: Utc::now(),
        })
    }
}

impl Default for SecEdgarConnector {
    fn default() -> Self {
        Self::new()
    }
}

fn simple_hash(input: &str) -> String {
    let hash = input.as_bytes().iter().fold(0_u64, |acc, byte| {
        acc.wrapping_mul(31).wrapping_add(*byte as u64)
    });
    format!("{hash:x}")
}

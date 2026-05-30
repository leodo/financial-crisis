use std::time::Duration;

use chrono::{Datelike, NaiveDate, Utc};
use fc_domain::Observation;
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

use crate::{
    http_client, Connector, ConnectorCapability, ConnectorError, FetchPlan, NormalizedBatch,
    RawPayload, SourceDescriptor,
};

#[derive(Debug, Clone)]
pub struct TreasuryYieldCurveConnector {
    client: reqwest::Client,
    base_url: Url,
}

impl TreasuryYieldCurveConnector {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent(http_client::user_agent())
                .http1_only()
                .no_proxy()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("valid Treasury reqwest client"),
            base_url: Url::parse(
                "https://home.treasury.gov/resource-center/data-chart-center/interest-rates/pages/xml",
            )
            .expect("valid Treasury yield curve URL"),
        }
    }

    pub fn build_month_url(&self, month: YearMonth) -> Url {
        let mut url = self.base_url.clone();
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("data", "daily_treasury_yield_curve");
            query.append_pair("field_tdr_date_value_month", &month.to_query_value());
        }
        url
    }
}

impl Default for TreasuryYieldCurveConnector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Connector for TreasuryYieldCurveConnector {
    fn describe(&self) -> SourceDescriptor {
        SourceDescriptor {
            source_id: "treasury".to_string(),
            display_name: "U.S. Treasury Yield Curve".to_string(),
            capabilities: vec![
                ConnectorCapability::Backfill,
                ConnectorCapability::Incremental,
                ConnectorCapability::ParseRaw,
                ConnectorCapability::Normalize,
            ],
            production_allowed: true,
            license_note:
                "Official U.S. Treasury daily yield curve XML feed; cache responses locally."
                    .to_string(),
        }
    }

    async fn fetch(&self, plan: &FetchPlan) -> Result<RawPayload, ConnectorError> {
        let start = plan.requested_start.unwrap_or_else(|| {
            NaiveDate::from_ymd_opt(Utc::now().year(), Utc::now().month(), 1)
                .expect("valid current month start")
        });
        let end = plan
            .requested_end
            .unwrap_or_else(|| Utc::now().date_naive());
        if start > end {
            return Err(ConnectorError::InvalidRequest(
                "requested_start must be on or before requested_end".to_string(),
            ));
        }

        let mut responses = Vec::new();
        for month in YearMonth::range_inclusive(start, end) {
            let url = self.build_month_url(month);
            let response = self
                .client
                .get(url.clone())
                .send()
                .await
                .map_err(|error| ConnectorError::TemporaryNetwork(format!("{error:?}")));
            let body = match response {
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
                    response
                        .text()
                        .await
                        .map_err(|error| ConnectorError::TemporaryNetwork(format!("{error:?}")))?
                }
                Err(error) => {
                    tracing::warn!(%error, "reqwest failed; falling back to curl");
                    http_client::curl_get_text(&url, 60)?
                }
            };
            responses.push(TreasuryMonthPayload {
                month: month.to_query_value(),
                request_url: url.to_string(),
                body,
            });
        }
        let body = serde_json::to_string(&TreasuryYieldPayload { responses })
            .map_err(|error| ConnectorError::Parse(error.to_string()))?;
        Ok(RawPayload {
            raw_payload_id: Uuid::new_v4(),
            source_id: plan.source_id.clone(),
            dataset_id: plan.dataset_id.clone(),
            request_url: format!(
                "treasury_yield_curve?start={start}&end={end}&months={}",
                YearMonth::range_inclusive(start, end).len()
            ),
            response_hash: simple_hash(&body),
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
        let parsed: TreasuryYieldPayload = serde_json::from_str(&payload.body)
            .map_err(|error| ConnectorError::Parse(error.to_string()))?;
        let code = plan.external_code.as_deref().unwrap_or(&plan.target_id);
        let mut observations = Vec::new();
        let mut warnings = Vec::new();

        for month in parsed.responses {
            let document = roxmltree::Document::parse(&month.body)
                .map_err(|error| ConnectorError::Parse(error.to_string()))?;
            for properties in document
                .descendants()
                .filter(|node| node.has_tag_name("properties"))
            {
                let Some(date_text) = child_text(properties, "NEW_DATE") else {
                    continue;
                };
                let date_text = date_text
                    .get(0..10)
                    .ok_or_else(|| ConnectorError::Parse("invalid Treasury date".to_string()))?;
                let as_of_date = NaiveDate::parse_from_str(date_text, "%Y-%m-%d")
                    .map_err(|error| ConnectorError::Parse(error.to_string()))?;
                if plan.requested_start.is_some_and(|start| as_of_date < start)
                    || plan.requested_end.is_some_and(|end| as_of_date > end)
                {
                    continue;
                }
                let value = match treasury_value(properties, code) {
                    Ok(value) => value,
                    Err(error) => {
                        warnings.push(format!("{error} on {as_of_date}"));
                        continue;
                    }
                };
                observations.push(Observation {
                    indicator_id: plan.target_id.clone(),
                    entity_id: "us".to_string(),
                    as_of_date,
                    period_start: Some(as_of_date),
                    period_end: Some(as_of_date),
                    frequency: plan.frequency,
                    value,
                    unit: "percent".to_string(),
                    source_id: payload.source_id.clone(),
                    dataset_id: payload.dataset_id.clone(),
                    revision_time: None,
                    publication_time: Some(payload.fetched_at),
                    quality_score: 96.0,
                    quality_flags: vec!["official_treasury_yield_curve".to_string()],
                });
            }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YearMonth {
    year: i32,
    month: u32,
}

impl YearMonth {
    fn range_inclusive(start: NaiveDate, end: NaiveDate) -> Vec<Self> {
        let mut current = Self {
            year: start.year(),
            month: start.month(),
        };
        let last = Self {
            year: end.year(),
            month: end.month(),
        };
        let mut months = Vec::new();
        while current <= last {
            months.push(current);
            current = current.next();
        }
        months
    }

    fn next(self) -> Self {
        if self.month == 12 {
            Self {
                year: self.year + 1,
                month: 1,
            }
        } else {
            Self {
                year: self.year,
                month: self.month + 1,
            }
        }
    }

    fn to_query_value(self) -> String {
        format!("{:04}{:02}", self.year, self.month)
    }
}

impl PartialOrd for YearMonth {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for YearMonth {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.year, self.month).cmp(&(other.year, other.month))
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct TreasuryYieldPayload {
    responses: Vec<TreasuryMonthPayload>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TreasuryMonthPayload {
    month: String,
    request_url: String,
    body: String,
}

fn treasury_value(properties: roxmltree::Node<'_, '_>, code: &str) -> Result<f64, String> {
    match code {
        "T10Y2Y" => {
            Ok(parse_child(properties, "BC_10YEAR")? - parse_child(properties, "BC_2YEAR")?)
        }
        "DGS10" | "BC_10YEAR" => parse_child(properties, "BC_10YEAR"),
        "DGS2" | "BC_2YEAR" => parse_child(properties, "BC_2YEAR"),
        other => Err(format!("unsupported Treasury yield code {other}")),
    }
}

fn parse_child(properties: roxmltree::Node<'_, '_>, tag_name: &str) -> Result<f64, String> {
    let value = child_text(properties, tag_name)
        .ok_or_else(|| format!("missing Treasury field {tag_name}"))?;
    value
        .parse::<f64>()
        .map_err(|error| format!("invalid Treasury field {tag_name}: {error}"))
}

fn child_text<'a>(properties: roxmltree::Node<'a, 'a>, tag_name: &str) -> Option<&'a str> {
    properties
        .children()
        .find(|node| node.is_element() && node.tag_name().name() == tag_name)
        .and_then(|node| node.text())
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

    use super::{
        TreasuryMonthPayload, TreasuryYieldCurveConnector, TreasuryYieldPayload, YearMonth,
    };

    #[test]
    fn builds_month_url() {
        let connector = TreasuryYieldCurveConnector::new();
        let url = connector.build_month_url(YearMonth {
            year: 2026,
            month: 5,
        });
        assert!(url.as_str().contains("data=daily_treasury_yield_curve"));
        assert!(url.as_str().contains("field_tdr_date_value_month=202605"));
    }

    #[test]
    fn parses_ten_year_two_year_spread_from_treasury_xml() {
        let connector = TreasuryYieldCurveConnector::new();
        let plan = FetchPlan {
            source_id: "treasury".to_string(),
            dataset_id: "treasury_daily_yield_curve".to_string(),
            target_id: "us_rates_yield_curve_10y2y".to_string(),
            external_code: Some("T10Y2Y".to_string()),
            run_mode: RunMode::Backfill,
            requested_start: Some(NaiveDate::from_ymd_opt(2026, 5, 1).unwrap()),
            requested_end: Some(NaiveDate::from_ymd_opt(2026, 5, 1).unwrap()),
            frequency: Frequency::Daily,
        };
        let xml = r#"
            <feed xmlns:d="http://schemas.microsoft.com/ado/2007/08/dataservices" xmlns:m="http://schemas.microsoft.com/ado/2007/08/dataservices/metadata">
              <entry><content><m:properties>
                <d:NEW_DATE m:type="Edm.DateTime">2026-05-01T00:00:00</d:NEW_DATE>
                <d:BC_2YEAR m:type="Edm.Double">3.88</d:BC_2YEAR>
                <d:BC_10YEAR m:type="Edm.Double">4.39</d:BC_10YEAR>
              </m:properties></content></entry>
            </feed>
        "#;
        let body = serde_json::to_string(&TreasuryYieldPayload {
            responses: vec![TreasuryMonthPayload {
                month: "202605".to_string(),
                request_url: "https://example.invalid".to_string(),
                body: xml.to_string(),
            }],
        })
        .unwrap();
        let payload = RawPayload {
            raw_payload_id: Uuid::new_v4(),
            source_id: "treasury".to_string(),
            dataset_id: "treasury_daily_yield_curve".to_string(),
            request_url: "treasury_yield_curve?start=2026-05-01&end=2026-05-01&months=1"
                .to_string(),
            response_hash: "hash".to_string(),
            content_type: "application/json".to_string(),
            body,
            fetched_at: Utc::now(),
        };

        let batch = connector.parse(&plan, &payload).unwrap();

        assert_eq!(batch.observations.len(), 1);
        assert!((batch.observations[0].value - 0.51).abs() < 0.000001);
    }
}

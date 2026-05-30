use std::time::Duration;

use chrono::{Datelike, NaiveDate, Utc};
use fc_domain::Observation;
use url::Url;
use uuid::Uuid;

use crate::{
    http_client, Connector, ConnectorCapability, ConnectorError, FetchPlan, NormalizedBatch,
    RawPayload, SourceDescriptor,
};

const BOJ_BASE_URL: &str = "https://www.stat-search.boj.or.jp/api/v1/getDataCode";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BojDataset {
    FxDaily,
    MoneyMarketRates,
}

impl BojDataset {
    pub fn dataset_id(self) -> &'static str {
        match self {
            Self::FxDaily => "boj_fx_daily",
            Self::MoneyMarketRates => "boj_money_market_rates",
        }
    }

    fn db_code(self) -> &'static str {
        match self {
            Self::FxDaily => "FM08",
            Self::MoneyMarketRates => "FM01",
        }
    }

    fn quality_flag(self) -> &'static str {
        match self {
            Self::FxDaily => "official_boj_fx_daily",
            Self::MoneyMarketRates => "official_boj_money_market",
        }
    }
}

#[derive(Debug, Clone)]
pub struct BojConnector {
    client: reqwest::Client,
    base_url: Url,
    dataset: BojDataset,
}

impl BojConnector {
    pub fn new(dataset: BojDataset) -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent(http_client::user_agent())
                .http1_only()
                .no_proxy()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("valid BOJ reqwest client"),
            base_url: Url::parse(BOJ_BASE_URL).expect("valid BOJ API URL"),
            dataset,
        }
    }

    pub fn fx_daily() -> Self {
        Self::new(BojDataset::FxDaily)
    }

    pub fn money_market_rates() -> Self {
        Self::new(BojDataset::MoneyMarketRates)
    }

    pub fn build_series_url(
        &self,
        series_code: &str,
        start: Option<NaiveDate>,
        end: Option<NaiveDate>,
    ) -> Result<Url, ConnectorError> {
        let mut url = self.base_url.clone();
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("format", "csv");
            query.append_pair("lang", "en");
            query.append_pair("db", self.dataset.db_code());
            query.append_pair("code", series_code);
            if let Some(start) = start {
                query.append_pair("startDate", &format_period(start));
            }
            if let Some(end) = end {
                query.append_pair("endDate", &format_period(end));
            }
        }
        Ok(url)
    }
}

impl Default for BojConnector {
    fn default() -> Self {
        Self::fx_daily()
    }
}

#[async_trait::async_trait]
impl Connector for BojConnector {
    fn describe(&self) -> SourceDescriptor {
        SourceDescriptor {
            source_id: "boj".to_string(),
            display_name: match self.dataset {
                BojDataset::FxDaily => "BOJ Foreign Exchange Rates (Daily)".to_string(),
                BojDataset::MoneyMarketRates => {
                    "BOJ Money Market and Call Rates".to_string()
                }
            },
            capabilities: vec![
                ConnectorCapability::Backfill,
                ConnectorCapability::Incremental,
                ConnectorCapability::ParseRaw,
                ConnectorCapability::Normalize,
            ],
            production_allowed: true,
            license_note:
                "Official BOJ public statistics API; no API key required and responses should be cached locally."
                    .to_string(),
        }
    }

    async fn fetch(&self, plan: &FetchPlan) -> Result<RawPayload, ConnectorError> {
        let series_code = plan.external_code.as_deref().unwrap_or(&plan.target_id);
        let url = self.build_series_url(series_code, plan.requested_start, plan.requested_end)?;
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
        let csv_body = extract_csv_section(&payload.body)?;
        let mut reader = csv::ReaderBuilder::new()
            .flexible(true)
            .from_reader(csv_body.as_bytes());
        let headers = reader
            .headers()
            .map_err(|error| ConnectorError::Parse(error.to_string()))?
            .clone();
        let date_index = headers
            .iter()
            .position(|header| header == "SURVEY_DATES")
            .ok_or_else(|| {
                ConnectorError::SchemaChanged("missing SURVEY_DATES column".to_string())
            })?;
        let value_index = headers
            .iter()
            .position(|header| header == "VALUES")
            .ok_or_else(|| ConnectorError::SchemaChanged("missing VALUES column".to_string()))?;
        let unit_index = headers
            .iter()
            .position(|header| header == "UNIT")
            .ok_or_else(|| ConnectorError::SchemaChanged("missing UNIT column".to_string()))?;

        let mut observations = Vec::new();
        let mut skipped_null_rows = 0_usize;
        for record in reader.records() {
            let record = record.map_err(|error| ConnectorError::Parse(error.to_string()))?;
            let date_text = record
                .get(date_index)
                .ok_or_else(|| ConnectorError::Parse("missing SURVEY_DATES value".to_string()))?;
            let as_of_date = parse_boj_date(date_text)?;
            if plan.requested_start.is_some_and(|start| as_of_date < start)
                || plan.requested_end.is_some_and(|end| as_of_date > end)
            {
                continue;
            }

            let raw_value = record.get(value_index).unwrap_or("").trim();
            if raw_value.is_empty() || raw_value.eq_ignore_ascii_case("null") {
                skipped_null_rows += 1;
                continue;
            }
            let value = raw_value
                .parse::<f64>()
                .map_err(|error| ConnectorError::Parse(error.to_string()))?;
            let source_unit = record.get(unit_index).unwrap_or("").trim();
            observations.push(Observation {
                indicator_id: plan.target_id.clone(),
                entity_id: observation_entity_id(&plan.target_id).to_string(),
                as_of_date,
                period_start: Some(as_of_date),
                period_end: Some(as_of_date),
                frequency: plan.frequency,
                value,
                unit: normalize_unit(&plan.target_id, source_unit),
                source_id: payload.source_id.clone(),
                dataset_id: payload.dataset_id.clone(),
                revision_time: None,
                publication_time: Some(payload.fetched_at),
                quality_score: 97.0,
                quality_flags: vec![self.dataset.quality_flag().to_string()],
            });
        }

        let mut warnings = Vec::new();
        if skipped_null_rows > 0 {
            warnings.push(format!(
                "skipped {skipped_null_rows} BOJ rows with null values, typically weekends or holidays"
            ));
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

fn format_period(date: NaiveDate) -> String {
    format!("{:04}{:02}", date.year(), date.month())
}

fn parse_boj_date(value: &str) -> Result<NaiveDate, ConnectorError> {
    NaiveDate::parse_from_str(value, "%Y%m%d")
        .map_err(|error| ConnectorError::Parse(error.to_string()))
}

fn extract_csv_section(body: &str) -> Result<&str, ConnectorError> {
    let start = body
        .find("SERIES_CODE,")
        .ok_or_else(|| ConnectorError::SchemaChanged("missing BOJ CSV header".to_string()))?;
    Ok(&body[start..])
}

fn observation_entity_id(indicator_id: &str) -> &str {
    if indicator_id.starts_with("jp_") {
        "jp"
    } else {
        "us"
    }
}

fn normalize_unit(indicator_id: &str, source_unit: &str) -> String {
    match indicator_id {
        "us_external_usdjpy_level" => "jpy_per_usd".to_string(),
        "jp_rates_call_rate" => "percent".to_string(),
        _ if source_unit.is_empty() => "source_unit".to_string(),
        _ => source_unit.to_ascii_lowercase().replace(' ', "_"),
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

    use super::{BojConnector, BojDataset};

    #[test]
    fn builds_boj_fx_url() {
        let connector = BojConnector::fx_daily();
        let url = connector
            .build_series_url(
                "FXERD01",
                Some(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
                Some(NaiveDate::from_ymd_opt(2020, 12, 31).unwrap()),
            )
            .unwrap();

        assert_eq!(
            url.as_str(),
            "https://www.stat-search.boj.or.jp/api/v1/getDataCode?format=csv&lang=en&db=FM08&code=FXERD01&startDate=202001&endDate=202012"
        );
    }

    #[test]
    fn parses_boj_fx_csv_and_skips_null_days() {
        let connector = BojConnector::fx_daily();
        let plan = FetchPlan {
            source_id: "boj".to_string(),
            dataset_id: BojDataset::FxDaily.dataset_id().to_string(),
            target_id: "us_external_usdjpy_level".to_string(),
            external_code: Some("FXERD01".to_string()),
            run_mode: RunMode::Backfill,
            requested_start: Some(NaiveDate::from_ymd_opt(2025, 5, 1).unwrap()),
            requested_end: Some(NaiveDate::from_ymd_opt(2025, 5, 7).unwrap()),
            frequency: Frequency::Daily,
        };
        let payload = RawPayload {
            raw_payload_id: Uuid::new_v4(),
            source_id: "boj".to_string(),
            dataset_id: BojDataset::FxDaily.dataset_id().to_string(),
            request_url: "https://www.stat-search.boj.or.jp/api/v1/getDataCode".to_string(),
            response_hash: "hash".to_string(),
            content_type: "text/csv".to_string(),
            body: r#"STATUS,200
MESSAGEID,M181000I
MESSAGE,Successfully completed
SERIES_CODE,NAME_OF_TIME_SERIES,UNIT,FREQUENCY,CATEGORY,LAST_UPDATE,SURVEY_DATES,VALUES
FXERD01,"US.Dollar/Yen Spot Rate at 9:00 in JST, Tokyo Market",Yen per U.S. Dollar,DAILY,Foreign Exchange Rates,20260529,20250501,143.02
FXERD01,"US.Dollar/Yen Spot Rate at 9:00 in JST, Tokyo Market",Yen per U.S. Dollar,DAILY,Foreign Exchange Rates,20260529,20250502,145.41
FXERD01,"US.Dollar/Yen Spot Rate at 9:00 in JST, Tokyo Market",Yen per U.S. Dollar,DAILY,Foreign Exchange Rates,20260529,20250503,null
FXERD01,"US.Dollar/Yen Spot Rate at 9:00 in JST, Tokyo Market",Yen per U.S. Dollar,DAILY,Foreign Exchange Rates,20260529,20250507,143.09
"#
            .to_string(),
            fetched_at: Utc::now(),
        };

        let batch = connector.parse(&plan, &payload).unwrap();

        assert_eq!(batch.observations.len(), 3);
        assert_eq!(batch.observations[0].entity_id, "us");
        assert_eq!(batch.observations[0].unit, "jpy_per_usd");
        assert_eq!(batch.observations[1].value, 145.41);
        assert_eq!(batch.warnings.len(), 1);
    }

    #[test]
    fn parses_boj_money_market_rows_for_japan_entity() {
        let connector = BojConnector::money_market_rates();
        let plan = FetchPlan {
            source_id: "boj".to_string(),
            dataset_id: BojDataset::MoneyMarketRates.dataset_id().to_string(),
            target_id: "jp_rates_call_rate".to_string(),
            external_code: Some("STRDCLUCON".to_string()),
            run_mode: RunMode::Backfill,
            requested_start: Some(NaiveDate::from_ymd_opt(2025, 5, 1).unwrap()),
            requested_end: Some(NaiveDate::from_ymd_opt(2025, 5, 2).unwrap()),
            frequency: Frequency::Daily,
        };
        let payload = RawPayload {
            raw_payload_id: Uuid::new_v4(),
            source_id: "boj".to_string(),
            dataset_id: BojDataset::MoneyMarketRates.dataset_id().to_string(),
            request_url: "https://www.stat-search.boj.or.jp/api/v1/getDataCode".to_string(),
            response_hash: "hash".to_string(),
            content_type: "text/csv".to_string(),
            body: r#"STATUS,200
MESSAGEID,M181000I
MESSAGE,Successfully completed
SERIES_CODE,NAME_OF_TIME_SERIES,UNIT,FREQUENCY,CATEGORY,LAST_UPDATE,SURVEY_DATES,VALUES
STRDCLUCON,"Call Rate, Uncollateralized Overnight, Average (Daily)",percent per annum,DAILY,Call Rate,20260529,20250501,0.477
STRDCLUCON,"Call Rate, Uncollateralized Overnight, Average (Daily)",percent per annum,DAILY,Call Rate,20260529,20250502,0.476
"#
            .to_string(),
            fetched_at: Utc::now(),
        };

        let batch = connector.parse(&plan, &payload).unwrap();

        assert_eq!(batch.observations.len(), 2);
        assert_eq!(batch.observations[0].entity_id, "jp");
        assert_eq!(batch.observations[0].unit, "percent");
    }
}

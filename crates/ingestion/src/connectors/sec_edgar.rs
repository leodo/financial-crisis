use std::{
    collections::{BTreeMap, HashMap, HashSet},
    time::Duration,
};

use chrono::{DateTime, NaiveDate, NaiveTime, TimeZone, Utc};
use fc_domain::{
    AlertEvent, AlertStatus, AlertType, Observation, RiskContributor, RiskDimension, RiskLevel,
};
use serde::Deserialize;
use url::Url;
use uuid::Uuid;

use crate::{http_client, ConnectorError, RawPayload};

const SEC_SUBMISSIONS_DATASET_ID: &str = "sec_company_submissions";
const SEC_EVENTS_DATASET_ID: &str = "sec_filing_events";
const SEC_SCOPE: &str = "sec_edgar_daily";
const SEC_METHOD_VERSION: &str = "sec_edgar_rules_v1_20260531";
const SEC_USER_AGENT: &str = "financial-crisis-research/0.1";
const SEC_REQUEST_DELAY_MILLIS: u64 = 250;

const TEXT_KEYWORDS: &[&str] = &[
    "liquidity",
    "funding",
    "deposit",
    "capital",
    "downgrade",
    "restructuring",
    "bankruptcy",
    "supervisory",
    "material weakness",
    "going concern",
];

const RISKY_ITEM_CODES: &[(&str, f64)] = &[
    ("1.03", 28.0),
    ("2.03", 18.0),
    ("2.04", 24.0),
    ("2.05", 12.0),
    ("2.06", 14.0),
    ("3.01", 15.0),
    ("3.03", 6.0),
    ("4.02", 22.0),
    ("8.01", 4.0),
];

#[derive(Debug, Clone, Copy)]
pub struct SecInstitution {
    pub cik: &'static str,
    pub ticker: &'static str,
    pub display_name: &'static str,
    pub importance: u8,
}

const SEC_INSTITUTIONS: &[SecInstitution] = &[
    SecInstitution {
        cik: "0000019617",
        ticker: "JPM",
        display_name: "JPMorgan Chase",
        importance: 3,
    },
    SecInstitution {
        cik: "0000070858",
        ticker: "BAC",
        display_name: "Bank of America",
        importance: 3,
    },
    SecInstitution {
        cik: "0000831001",
        ticker: "C",
        display_name: "Citigroup",
        importance: 3,
    },
    SecInstitution {
        cik: "0000072971",
        ticker: "WFC",
        display_name: "Wells Fargo",
        importance: 3,
    },
    SecInstitution {
        cik: "0000886982",
        ticker: "GS",
        display_name: "Goldman Sachs",
        importance: 3,
    },
    SecInstitution {
        cik: "0000895421",
        ticker: "MS",
        display_name: "Morgan Stanley",
        importance: 3,
    },
    SecInstitution {
        cik: "0000036104",
        ticker: "USB",
        display_name: "U.S. Bancorp",
        importance: 2,
    },
    SecInstitution {
        cik: "0000713676",
        ticker: "PNC",
        display_name: "PNC Financial",
        importance: 2,
    },
    SecInstitution {
        cik: "0000092230",
        ticker: "TFC",
        display_name: "Truist Financial",
        importance: 2,
    },
    SecInstitution {
        cik: "0001390777",
        ticker: "BK",
        display_name: "BNY Mellon",
        importance: 2,
    },
    SecInstitution {
        cik: "0000093751",
        ticker: "STT",
        display_name: "State Street",
        importance: 2,
    },
    SecInstitution {
        cik: "0000316709",
        ticker: "SCHW",
        display_name: "Charles Schwab",
        importance: 2,
    },
    SecInstitution {
        cik: "0002012383",
        ticker: "BLK",
        display_name: "BlackRock",
        importance: 1,
    },
    SecInstitution {
        cik: "0001571949",
        ticker: "ICE",
        display_name: "Intercontinental Exchange",
        importance: 1,
    },
    SecInstitution {
        cik: "0000005272",
        ticker: "AIG",
        display_name: "AIG",
        importance: 1,
    },
    SecInstitution {
        cik: "0000004962",
        ticker: "AXP",
        display_name: "American Express",
        importance: 1,
    },
];

#[derive(Debug, Clone)]
pub struct SecEdgarBackfill {
    pub payloads: Vec<RawPayload>,
    pub observations: Vec<Observation>,
    pub alerts: Vec<AlertEvent>,
    pub latest_filing_date: Option<NaiveDate>,
    pub company_count: usize,
    pub filing_count: usize,
}

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
                let body = response
                    .text()
                    .await
                    .map_err(|error| ConnectorError::TemporaryNetwork(format!("{error:?}")))?;
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

#[derive(Debug, Clone)]
struct CompanyBackfill {
    payloads: Vec<RawPayload>,
    filings: Vec<SecFilingRecord>,
}

#[derive(Debug, Clone)]
struct SecFilingRecord {
    institution: SecInstitution,
    accession_number: String,
    filing_date: NaiveDate,
    acceptance_time: Option<DateTime<Utc>>,
    form_type: String,
    keyword_hits: Vec<String>,
    rule_hits: Vec<String>,
    severity: f64,
}

#[derive(Debug, Clone, Default)]
struct DailyAggregate {
    as_of_date: NaiveDate,
    bank_8k_count: u32,
    rule_hit_count: u32,
    stress_count: u32,
    severity_index: f64,
    filing_count: u32,
    latest_acceptance_time: Option<DateTime<Utc>>,
    entity_scores: Vec<(String, f64)>,
}

#[derive(Debug, Deserialize, Default)]
struct SubmissionsEnvelope {
    #[serde(default)]
    filings: SubmissionsFilings,
}

#[derive(Debug, Deserialize, Default)]
struct SubmissionsFilings {
    #[serde(default)]
    recent: FilingArrays,
    #[serde(default)]
    files: Vec<ArchiveFileEntry>,
}

#[derive(Debug, Deserialize, Default)]
struct ArchiveFileEntry {
    name: String,
    #[serde(rename = "filingFrom")]
    filing_from: String,
    #[serde(rename = "filingTo")]
    filing_to: String,
}

#[derive(Debug, Deserialize, Default)]
struct FilingArrays {
    #[serde(rename = "accessionNumber", default)]
    accession_numbers: Vec<String>,
    #[serde(rename = "filingDate", default)]
    filing_dates: Vec<String>,
    #[serde(rename = "acceptanceDateTime", default)]
    acceptance_datetimes: Vec<String>,
    #[serde(rename = "form", default)]
    forms: Vec<String>,
    #[serde(rename = "items", default)]
    items: Vec<String>,
    #[serde(rename = "primaryDocument", default)]
    primary_documents: Vec<String>,
    #[serde(rename = "primaryDocDescription", default)]
    primary_doc_descriptions: Vec<String>,
}

fn parse_filing_arrays(
    institution: SecInstitution,
    arrays: &FilingArrays,
    start: NaiveDate,
    end: NaiveDate,
) -> Result<Vec<SecFilingRecord>, ConnectorError> {
    let mut filings = Vec::new();
    for (index, accession_number) in arrays.accession_numbers.iter().enumerate() {
        if accession_number.trim().is_empty() {
            continue;
        }
        let Some(form_bucket) = relevant_form_bucket(field_at(&arrays.forms, index)) else {
            continue;
        };
        let filing_date = parse_date(field_at(&arrays.filing_dates, index))?;
        if filing_date < start || filing_date > end {
            continue;
        }

        let items_text = field_at(&arrays.items, index);
        let item_codes = split_codes(items_text);
        let description = field_at(&arrays.primary_doc_descriptions, index);
        let document = field_at(&arrays.primary_documents, index);
        let text_blob =
            format!("{form_bucket} {description} {document} {items_text}").to_ascii_lowercase();
        let keyword_hits = keyword_hits(&text_blob);
        let rule_hits = item_rule_hits(&item_codes);
        let severity = filing_severity(
            institution.importance,
            form_bucket,
            &keyword_hits,
            &rule_hits,
        );
        filings.push(SecFilingRecord {
            institution,
            accession_number: accession_number.to_string(),
            filing_date,
            acceptance_time: parse_datetime_opt(field_at(&arrays.acceptance_datetimes, index))?,
            form_type: form_bucket.to_string(),
            keyword_hits,
            rule_hits,
            severity,
        });
    }
    Ok(filings)
}

fn build_daily_aggregates(
    start: NaiveDate,
    end: NaiveDate,
    filings: &[SecFilingRecord],
) -> Vec<DailyAggregate> {
    let mut raw_days: HashMap<NaiveDate, DailyAccumulator> = HashMap::new();
    for filing in filings {
        let day = raw_days.entry(filing.filing_date).or_default();
        day.filing_count += 1;
        if filing.form_type == "8-K" {
            day.bank_8k_count += 1;
        }
        day.rule_hit_count += (filing.keyword_hits.len() + filing.rule_hits.len()) as u32;
        if filing.severity >= 40.0
            || !filing.keyword_hits.is_empty()
            || !filing.rule_hits.is_empty()
        {
            day.stress_count += 1;
        }
        day.max_filing_severity = day.max_filing_severity.max(filing.severity);
        day.latest_acceptance_time = match (day.latest_acceptance_time, filing.acceptance_time) {
            (Some(current), Some(candidate)) => Some(current.max(candidate)),
            (None, Some(candidate)) => Some(candidate),
            (current, None) => current,
        };
        *day.entity_score_by_ticker
            .entry(filing.institution.ticker.to_string())
            .or_insert(0.0) += filing.severity;
    }

    let mut result = Vec::new();
    let mut cursor = start;
    while cursor <= end {
        let aggregate = raw_days.remove(&cursor).unwrap_or_default();
        let breadth_boost =
            (aggregate.entity_score_by_ticker.len().saturating_sub(1) as f64 * 4.0).min(12.0);
        let stress_boost = (aggregate.stress_count.saturating_sub(1) as f64 * 6.0).min(18.0);
        let rule_boost = (aggregate.rule_hit_count as f64 * 3.0).min(18.0);
        let severity_index = if aggregate.filing_count == 0 {
            0.0
        } else {
            (aggregate.max_filing_severity + breadth_boost + stress_boost + rule_boost)
                .clamp(0.0, 100.0)
        };

        let mut entity_scores = aggregate
            .entity_score_by_ticker
            .into_iter()
            .collect::<Vec<_>>();
        entity_scores.sort_by(|a, b| b.1.total_cmp(&a.1));
        result.push(DailyAggregate {
            as_of_date: cursor,
            bank_8k_count: aggregate.bank_8k_count,
            rule_hit_count: aggregate.rule_hit_count,
            stress_count: aggregate.stress_count,
            severity_index: round1(severity_index),
            filing_count: aggregate.filing_count,
            latest_acceptance_time: aggregate.latest_acceptance_time,
            entity_scores,
        });
        cursor += chrono::Duration::days(1);
    }

    result
}

fn build_observations(daily: &[DailyAggregate], fetched_at: DateTime<Utc>) -> Vec<Observation> {
    let mut observations = Vec::with_capacity(daily.len() * 4);
    for aggregate in daily {
        let mut flags = vec![
            "official_sec_filing_metadata".to_string(),
            "sec_rule_aggregate".to_string(),
        ];
        if aggregate.filing_count == 0 {
            flags.push("synthetic_zero_fill".to_string());
        }
        observations.push(build_observation(
            "us_event_bank_8k_count",
            aggregate.as_of_date,
            aggregate.bank_8k_count as f64,
            "count",
            aggregate.latest_acceptance_time.unwrap_or(fetched_at),
            &flags,
        ));
        observations.push(build_observation(
            "us_event_risk_keyword_count",
            aggregate.as_of_date,
            aggregate.rule_hit_count as f64,
            "count",
            aggregate.latest_acceptance_time.unwrap_or(fetched_at),
            &flags,
        ));
        observations.push(build_observation(
            "us_banking_filing_stress_count",
            aggregate.as_of_date,
            aggregate.stress_count as f64,
            "count",
            aggregate.latest_acceptance_time.unwrap_or(fetched_at),
            &flags,
        ));
        observations.push(build_observation(
            "us_event_official_filing_severity",
            aggregate.as_of_date,
            aggregate.severity_index,
            "score",
            aggregate.latest_acceptance_time.unwrap_or(fetched_at),
            &flags,
        ));
    }
    observations
}

fn build_observation(
    indicator_id: &str,
    as_of_date: NaiveDate,
    value: f64,
    unit: &str,
    fetched_at: DateTime<Utc>,
    flags: &[String],
) -> Observation {
    Observation {
        indicator_id: indicator_id.to_string(),
        entity_id: "us".to_string(),
        as_of_date,
        period_start: Some(as_of_date),
        period_end: Some(as_of_date),
        frequency: fc_domain::Frequency::Daily,
        value,
        unit: unit.to_string(),
        source_id: "sec_edgar".to_string(),
        dataset_id: SEC_EVENTS_DATASET_ID.to_string(),
        revision_time: None,
        publication_time: Some(fetched_at),
        quality_score: 88.0,
        quality_flags: flags.to_vec(),
    }
}

fn build_alerts(daily: &[DailyAggregate], end: NaiveDate) -> Vec<AlertEvent> {
    let mut alerts = Vec::new();
    let severity_by_date = daily
        .iter()
        .map(|aggregate| (aggregate.as_of_date, aggregate.severity_index))
        .collect::<BTreeMap<_, _>>();
    for aggregate in daily {
        if aggregate.severity_index < 30.0
            && aggregate.stress_count < 2
            && aggregate.rule_hit_count < 3
        {
            continue;
        }
        let level = RiskLevel::from_score(aggregate.severity_index);
        let event_type = match level {
            RiskLevel::Crisis => AlertType::RiskCrisis,
            RiskLevel::Warning => AlertType::RiskWarning,
            RiskLevel::Stress => AlertType::RiskStress,
            RiskLevel::Watch | RiskLevel::Normal => AlertType::RiskWatch,
        };
        let previous_score = aggregate
            .as_of_date
            .checked_sub_signed(chrono::Duration::days(1))
            .and_then(|date| severity_by_date.get(&date).copied())
            .filter(|score| *score > 0.0);
        let entity_total = aggregate
            .entity_scores
            .iter()
            .map(|(_, score)| *score)
            .sum::<f64>()
            .max(1.0);
        let top_contributors = aggregate
            .entity_scores
            .iter()
            .take(3)
            .map(|(ticker, score)| RiskContributor {
                indicator_id: "us_event_official_filing_severity".to_string(),
                display_name: format!("{ticker} SEC filings"),
                dimension: RiskDimension::EventsSentiment,
                score: round1(*score),
                contribution: round1((*score / entity_total) * aggregate.severity_index),
                explanation: format!(
                    "{ticker} filings contributed {:.1} to the SEC event cluster.",
                    *score
                ),
            })
            .collect::<Vec<_>>();
        let major_tickers = aggregate
            .entity_scores
            .iter()
            .take(3)
            .map(|(ticker, _)| ticker.as_str())
            .collect::<Vec<_>>();
        let trigger_reason = format!(
            "SEC 白名单机构公告出现聚集：{} 个银行 8-K、{} 个压力 filing、{} 个风险规则命中。主要机构：{}。",
            aggregate.bank_8k_count,
            aggregate.stress_count,
            aggregate.rule_hit_count,
            if major_tickers.is_empty() {
                "无".to_string()
            } else {
                major_tickers.join("、")
            }
        );

        alerts.push(AlertEvent {
            alert_id: Uuid::new_v5(
                &Uuid::NAMESPACE_URL,
                format!("{SEC_SCOPE}:{}", aggregate.as_of_date).as_bytes(),
            ),
            event_type,
            scope: SEC_SCOPE.to_string(),
            entity_id: "us".to_string(),
            dimension: Some(RiskDimension::EventsSentiment),
            level,
            status: if (end - aggregate.as_of_date).num_days() <= 7 {
                AlertStatus::Open
            } else {
                AlertStatus::Monitoring
            },
            triggered_at: aggregate.latest_acceptance_time.unwrap_or_else(|| {
                Utc.from_utc_datetime(
                    &aggregate
                        .as_of_date
                        .and_time(NaiveTime::from_hms_opt(23, 59, 59).expect("valid time")),
                )
            }),
            triggered_as_of_date: aggregate.as_of_date,
            resolved_at: None,
            score: aggregate.severity_index,
            previous_score,
            trigger_reason,
            top_contributors,
            related_indicators: vec![
                "us_event_bank_8k_count".to_string(),
                "us_event_risk_keyword_count".to_string(),
                "us_banking_filing_stress_count".to_string(),
                "us_event_official_filing_severity".to_string(),
            ],
            method_version: SEC_METHOD_VERSION.to_string(),
        });
    }

    alerts.sort_by(|a, b| {
        b.triggered_as_of_date
            .cmp(&a.triggered_as_of_date)
            .then_with(|| b.score.total_cmp(&a.score))
    });
    alerts
}

fn archive_overlaps(file: &ArchiveFileEntry, start: NaiveDate, end: NaiveDate) -> bool {
    let from = NaiveDate::parse_from_str(&file.filing_from, "%Y-%m-%d").ok();
    let to = NaiveDate::parse_from_str(&file.filing_to, "%Y-%m-%d").ok();
    match (from, to) {
        (Some(from), Some(to)) => !(to < start || from > end),
        _ => true,
    }
}

fn relevant_form_bucket(form: &str) -> Option<&'static str> {
    let upper = form.to_ascii_uppercase();
    if upper.starts_with("8-K") {
        Some("8-K")
    } else if upper.starts_with("10-Q") {
        Some("10-Q")
    } else if upper.starts_with("10-K") {
        Some("10-K")
    } else {
        None
    }
}

fn filing_severity(
    importance: u8,
    form_bucket: &str,
    keyword_hits: &[String],
    rule_hits: &[String],
) -> f64 {
    let base = match form_bucket {
        "8-K" => 12.0,
        "10-Q" => 6.0,
        "10-K" => 5.0,
        _ => 0.0,
    };
    let importance_boost = match importance {
        3 => 8.0,
        2 => 5.0,
        _ => 3.0,
    };
    let keyword_boost = if keyword_hits.is_empty() {
        0.0
    } else {
        (12.0 + (keyword_hits.len().saturating_sub(1) as f64 * 4.0)).min(20.0)
    };
    let rule_boost = rule_hits
        .iter()
        .filter_map(|label| {
            label
                .strip_prefix("item_")
                .and_then(|code| {
                    RISKY_ITEM_CODES
                        .iter()
                        .find(|(candidate, _)| *candidate == code)
                })
                .map(|(_, boost)| *boost)
        })
        .fold(0.0_f64, f64::max);

    round1((base + importance_boost + keyword_boost + rule_boost).clamp(0.0, 100.0))
}

fn keyword_hits(text: &str) -> Vec<String> {
    TEXT_KEYWORDS
        .iter()
        .filter(|keyword| text.contains(**keyword))
        .map(|keyword| (*keyword).to_string())
        .collect()
}

fn item_rule_hits(items: &[String]) -> Vec<String> {
    items
        .iter()
        .filter_map(|item| {
            RISKY_ITEM_CODES
                .iter()
                .find(|(code, _)| *code == item.as_str())
                .map(|(code, _)| format!("item_{code}"))
        })
        .collect()
}

fn split_codes(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| item.to_string())
        .collect()
}

fn field_at(values: &[String], index: usize) -> &str {
    values.get(index).map(String::as_str).unwrap_or("")
}

fn parse_date(value: &str) -> Result<NaiveDate, ConnectorError> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|error| ConnectorError::Parse(error.to_string()))
}

fn parse_datetime_opt(value: &str) -> Result<Option<DateTime<Utc>>, ConnectorError> {
    if value.trim().is_empty() {
        return Ok(None);
    }
    DateTime::parse_from_rfc3339(value)
        .map(|datetime| Some(datetime.with_timezone(&Utc)))
        .map_err(|error| ConnectorError::Parse(error.to_string()))
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

fn simple_hash(input: &str) -> String {
    let hash = input.as_bytes().iter().fold(0_u64, |acc, byte| {
        acc.wrapping_mul(31).wrapping_add(*byte as u64)
    });
    format!("{hash:x}")
}

#[derive(Debug, Default)]
struct DailyAccumulator {
    bank_8k_count: u32,
    rule_hit_count: u32,
    stress_count: u32,
    filing_count: u32,
    max_filing_severity: f64,
    latest_acceptance_time: Option<DateTime<Utc>>,
    entity_score_by_ticker: HashMap<String, f64>,
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::{
        build_alerts, build_daily_aggregates, build_observations, filing_severity, item_rule_hits,
        keyword_hits, parse_filing_arrays, FilingArrays, SecInstitution,
    };

    fn sample_institution() -> SecInstitution {
        SecInstitution {
            cik: "0000019617",
            ticker: "JPM",
            display_name: "JPMorgan Chase",
            importance: 3,
        }
    }

    #[test]
    fn parses_relevant_sec_filing_rows() {
        let arrays = FilingArrays {
            accession_numbers: vec!["a".to_string(), "b".to_string(), "c".to_string()],
            filing_dates: vec![
                "2026-05-27".to_string(),
                "2026-05-01".to_string(),
                "2026-05-02".to_string(),
            ],
            acceptance_datetimes: vec![
                "2026-05-27T14:00:00.000Z".to_string(),
                "2026-05-01T13:00:00.000Z".to_string(),
                "2026-05-02T13:00:00.000Z".to_string(),
            ],
            forms: vec!["8-K".to_string(), "10-Q".to_string(), "424B2".to_string()],
            items: vec!["2.04,9.01".to_string(), "".to_string(), "".to_string()],
            primary_documents: vec![
                "a.htm".to_string(),
                "b.htm".to_string(),
                "c.htm".to_string(),
            ],
            primary_doc_descriptions: vec![
                "liquidity support update".to_string(),
                "quarterly report".to_string(),
                "ignored".to_string(),
            ],
        };

        let filings = parse_filing_arrays(
            sample_institution(),
            &arrays,
            NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 5, 31).unwrap(),
        )
        .unwrap();

        assert_eq!(filings.len(), 2);
        assert_eq!(filings[0].form_type, "8-K");
        assert!(filings[0].severity > filings[1].severity);
        assert!(filings[0]
            .rule_hits
            .iter()
            .any(|value| value == "item_2.04"));
    }

    #[test]
    fn aggregates_sparse_events_without_turning_zero_into_alerts() {
        let arrays = FilingArrays {
            accession_numbers: vec!["a".to_string(), "b".to_string()],
            filing_dates: vec!["2026-05-27".to_string(), "2026-05-27".to_string()],
            acceptance_datetimes: vec![
                "2026-05-27T14:00:00.000Z".to_string(),
                "2026-05-27T15:00:00.000Z".to_string(),
            ],
            forms: vec!["8-K".to_string(), "10-Q".to_string()],
            items: vec!["2.04,9.01".to_string(), "".to_string()],
            primary_documents: vec!["a.htm".to_string(), "b.htm".to_string()],
            primary_doc_descriptions: vec![
                "liquidity support update".to_string(),
                "quarterly report".to_string(),
            ],
        };
        let filings = parse_filing_arrays(
            sample_institution(),
            &arrays,
            NaiveDate::from_ymd_opt(2026, 5, 26).unwrap(),
            NaiveDate::from_ymd_opt(2026, 5, 28).unwrap(),
        )
        .unwrap();
        let daily = build_daily_aggregates(
            NaiveDate::from_ymd_opt(2026, 5, 26).unwrap(),
            NaiveDate::from_ymd_opt(2026, 5, 28).unwrap(),
            &filings,
        );
        let observations = build_observations(&daily, chrono::Utc::now());
        let alerts = build_alerts(&daily, NaiveDate::from_ymd_opt(2026, 5, 28).unwrap());

        assert_eq!(daily.len(), 3);
        assert_eq!(daily[0].severity_index, 0.0);
        assert!(daily[1].severity_index >= 40.0);
        assert_eq!(
            observations
                .iter()
                .filter(|observation| observation.as_of_date
                    == NaiveDate::from_ymd_opt(2026, 5, 26).unwrap())
                .filter(|observation| observation
                    .quality_flags
                    .iter()
                    .any(|flag| flag == "synthetic_zero_fill"))
                .count(),
            4
        );
        assert_eq!(alerts.len(), 1);
    }

    #[test]
    fn keyword_and_item_rules_drive_severity() {
        let keyword_hits = keyword_hits("liquidity capital material weakness");
        let rule_hits = item_rule_hits(&["4.02".to_string(), "9.01".to_string()]);
        let severity = filing_severity(3, "8-K", &keyword_hits, &rule_hits);

        assert_eq!(keyword_hits.len(), 3);
        assert!(rule_hits.iter().any(|hit| hit == "item_4.02"));
        assert!(severity >= 40.0);
    }
}

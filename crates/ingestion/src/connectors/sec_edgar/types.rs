use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use fc_domain::{AlertEvent, Observation};
use serde::Deserialize;

use crate::RawPayload;

#[derive(Debug, Clone, Copy)]
pub struct SecInstitution {
    pub cik: &'static str,
    pub ticker: &'static str,
    pub display_name: &'static str,
    pub importance: u8,
}

pub(super) const SEC_INSTITUTIONS: &[SecInstitution] = &[
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
pub(super) struct CompanyBackfill {
    pub(super) payloads: Vec<RawPayload>,
    pub(super) filings: Vec<SecFilingRecord>,
}

#[derive(Debug, Clone)]
pub(super) struct SecFilingRecord {
    pub(super) institution: SecInstitution,
    pub(super) accession_number: String,
    pub(super) filing_date: NaiveDate,
    pub(super) acceptance_time: Option<DateTime<Utc>>,
    pub(super) form_type: String,
    pub(super) keyword_hits: Vec<String>,
    pub(super) rule_hits: Vec<String>,
    pub(super) severity: f64,
}

#[derive(Debug, Clone, Default)]
pub(super) struct DailyAggregate {
    pub(super) as_of_date: NaiveDate,
    pub(super) bank_8k_count: u32,
    pub(super) rule_hit_count: u32,
    pub(super) stress_count: u32,
    pub(super) severity_index: f64,
    pub(super) filing_count: u32,
    pub(super) latest_acceptance_time: Option<DateTime<Utc>>,
    pub(super) entity_scores: Vec<(String, f64)>,
}

#[derive(Debug, Deserialize, Default)]
pub(super) struct SubmissionsEnvelope {
    #[serde(default)]
    pub(super) filings: SubmissionsFilings,
}

#[derive(Debug, Deserialize, Default)]
pub(super) struct SubmissionsFilings {
    #[serde(default)]
    pub(super) recent: FilingArrays,
    #[serde(default)]
    pub(super) files: Vec<ArchiveFileEntry>,
}

#[derive(Debug, Deserialize, Default)]
pub(super) struct ArchiveFileEntry {
    pub(super) name: String,
    #[serde(rename = "filingFrom")]
    pub(super) filing_from: String,
    #[serde(rename = "filingTo")]
    pub(super) filing_to: String,
}

#[derive(Debug, Deserialize, Default)]
pub(super) struct FilingArrays {
    #[serde(rename = "accessionNumber", default)]
    pub(super) accession_numbers: Vec<String>,
    #[serde(rename = "filingDate", default)]
    pub(super) filing_dates: Vec<String>,
    #[serde(rename = "acceptanceDateTime", default)]
    pub(super) acceptance_datetimes: Vec<String>,
    #[serde(rename = "form", default)]
    pub(super) forms: Vec<String>,
    #[serde(rename = "items", default)]
    pub(super) items: Vec<String>,
    #[serde(rename = "primaryDocument", default)]
    pub(super) primary_documents: Vec<String>,
    #[serde(rename = "primaryDocDescription", default)]
    pub(super) primary_doc_descriptions: Vec<String>,
}

#[derive(Debug, Default)]
pub(super) struct DailyAccumulator {
    pub(super) bank_8k_count: u32,
    pub(super) rule_hit_count: u32,
    pub(super) stress_count: u32,
    pub(super) filing_count: u32,
    pub(super) max_filing_severity: f64,
    pub(super) latest_acceptance_time: Option<DateTime<Utc>>,
    pub(super) entity_score_by_ticker: HashMap<String, f64>,
}

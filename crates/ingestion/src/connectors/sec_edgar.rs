mod aggregate;
mod connector;
mod parse;
mod rules;
#[cfg(test)]
mod tests;
mod types;

pub use connector::SecEdgarConnector;
pub use types::{SecEdgarBackfill, SecInstitution};

const SEC_SUBMISSIONS_DATASET_ID: &str = "sec_company_submissions";
const SEC_EVENTS_DATASET_ID: &str = "sec_filing_events";
const SEC_SCOPE: &str = "sec_edgar_daily";
const SEC_METHOD_VERSION: &str = "sec_edgar_rules_v1_20260531";
const SEC_USER_AGENT: &str = "financial-crisis-research/0.1";
const SEC_REQUEST_DELAY_MILLIS: u64 = 250;

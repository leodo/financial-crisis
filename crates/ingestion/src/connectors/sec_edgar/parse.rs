use chrono::{DateTime, NaiveDate, Utc};

use crate::ConnectorError;

use super::rules::{
    filing_severity, item_rule_hits, keyword_hits, relevant_form_bucket, split_codes,
};
use super::types::{ArchiveFileEntry, FilingArrays, SecFilingRecord, SecInstitution};

pub(super) fn parse_filing_arrays(
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

pub(super) fn archive_overlaps(file: &ArchiveFileEntry, start: NaiveDate, end: NaiveDate) -> bool {
    let from = NaiveDate::parse_from_str(&file.filing_from, "%Y-%m-%d").ok();
    let to = NaiveDate::parse_from_str(&file.filing_to, "%Y-%m-%d").ok();
    match (from, to) {
        (Some(from), Some(to)) => !(to < start || from > end),
        _ => true,
    }
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

use chrono::NaiveDate;

use super::aggregate::{build_alerts, build_daily_aggregates, build_observations};
use super::parse::parse_filing_arrays;
use super::rules::{filing_severity, item_rule_hits, keyword_hits};
use super::types::FilingArrays;
use super::SecInstitution;

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

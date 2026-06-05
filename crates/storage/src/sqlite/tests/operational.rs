use chrono::{NaiveDate, Utc};
use fc_domain::{AlertEvent, AlertStatus, AlertType, RiskContributor, RiskDimension, RiskLevel};
use uuid::Uuid;

use crate::sqlite::tests::in_memory_store;

#[tokio::test]
async fn sqlite_store_round_trips_alerts() {
    let store = in_memory_store().await;

    let alert = AlertEvent {
        alert_id: Uuid::new_v4(),
        event_type: AlertType::RiskStress,
        scope: "sec_edgar_daily".to_string(),
        entity_id: "us".to_string(),
        dimension: Some(RiskDimension::EventsSentiment),
        level: RiskLevel::Stress,
        status: AlertStatus::Open,
        triggered_at: Utc::now(),
        triggered_as_of_date: NaiveDate::from_ymd_opt(2026, 5, 30).unwrap(),
        resolved_at: None,
        score: 61.0,
        previous_score: Some(28.0),
        trigger_reason: "SEC filing stress cluster".to_string(),
        top_contributors: vec![RiskContributor {
            indicator_id: "us_event_official_filing_severity".to_string(),
            display_name: "SEC 官方公告严重度".to_string(),
            dimension: RiskDimension::EventsSentiment,
            score: 61.0,
            contribution: 61.0,
            explanation: "bank filing spike".to_string(),
        }],
        related_indicators: vec![
            "us_event_bank_8k_count".to_string(),
            "us_event_official_filing_severity".to_string(),
        ],
        method_version: "sec_rules_v1".to_string(),
    };

    store
        .replace_alerts_for_scope(
            "sec_edgar_daily",
            NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 5, 31).unwrap(),
            std::slice::from_ref(&alert),
        )
        .await
        .unwrap();

    let alerts = store
        .load_alerts_recent(
            NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 5, 31).unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].alert_id, alert.alert_id);
    assert_eq!(alerts[0].related_indicators.len(), 2);
    assert_eq!(
        alerts[0].top_contributors[0].dimension,
        RiskDimension::EventsSentiment
    );
}

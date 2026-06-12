use chrono::{NaiveDate, Utc};
use fc_domain::{AlertEvent, AlertStatus, AlertType, RiskContributor, RiskDimension, RiskLevel};
use sqlx::Row;
use uuid::Uuid;

use crate::sqlite::tests::in_memory_store;
use crate::sqlite::{IngestionRunRecord, RawResponseRecord};

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

#[tokio::test]
async fn sqlite_store_links_ingestion_run_to_raw_response() {
    let store = in_memory_store().await;
    let run_id = "run-fred-vix-20260609".to_string();
    let finished_at = Utc::now();

    store
        .insert_ingestion_run(&IngestionRunRecord {
            run_id: run_id.clone(),
            job_id: Some("backfill:fred:VIXCLS".to_string()),
            source_id: "fred".to_string(),
            dataset_id: "fred_series_observations".to_string(),
            target_id: Some("us_market_vix_close".to_string()),
            run_mode: "backfill".to_string(),
            status: "success".to_string(),
            started_at: finished_at,
            finished_at: Some(finished_at),
            attempt: 1,
            watermark_before_json: Some(r#"{"last_successful_period":"2026-06-04"}"#.to_string()),
            watermark_after_json: Some(r#"{"last_successful_period":"2026-06-05"}"#.to_string()),
            records_read: 1,
            records_written: 1,
            error_type: None,
            error_message: None,
        })
        .await
        .unwrap();

    store
        .insert_raw_response(&RawResponseRecord {
            raw_payload_id: Uuid::new_v4(),
            run_id: Some(run_id.clone()),
            source_id: "fred".to_string(),
            dataset_id: "fred_series_observations".to_string(),
            request_url: "https://fred.stlouisfed.org/graph/fredgraph.csv?id=VIXCLS".to_string(),
            request_params_hash: Some("hash".to_string()),
            response_hash: "response".to_string(),
            content_type: "text/csv".to_string(),
            content_length: 42,
            raw_file_path: "data/raw/fred/VIXCLS/test.csv".to_string(),
            fetched_at: finished_at,
        })
        .await
        .unwrap();

    let run_row = sqlx::query("SELECT status, records_written FROM ingest_runs WHERE run_id = ?1")
        .bind(&run_id)
        .fetch_one(&store.pool)
        .await
        .unwrap();
    assert_eq!(run_row.try_get::<String, _>("status").unwrap(), "success");
    assert_eq!(run_row.try_get::<i64, _>("records_written").unwrap(), 1);

    let raw_row =
        sqlx::query("SELECT run_id FROM raw_responses WHERE request_params_hash = 'hash'")
            .fetch_one(&store.pool)
            .await
            .unwrap();
    assert_eq!(raw_row.try_get::<String, _>("run_id").unwrap(), run_id);
}

#[tokio::test]
async fn sqlite_store_summarizes_ingestion_source_health() {
    let store = in_memory_store().await;
    let first_finished_at = Utc::now() - chrono::Duration::minutes(30);
    let latest_finished_at = Utc::now();

    store
        .insert_ingestion_run(&IngestionRunRecord {
            run_id: "run-fred-success".to_string(),
            job_id: Some("backfill:fred:fred_series_observations:VIXCLS".to_string()),
            source_id: "fred".to_string(),
            dataset_id: "fred_series_observations".to_string(),
            target_id: Some("us_market_vix_close".to_string()),
            run_mode: "backfill".to_string(),
            status: "success".to_string(),
            started_at: first_finished_at,
            finished_at: Some(first_finished_at),
            attempt: 1,
            watermark_before_json: None,
            watermark_after_json: Some(r#"{"last_successful_period":"2026-06-05"}"#.to_string()),
            records_read: 1,
            records_written: 1,
            error_type: None,
            error_message: None,
        })
        .await
        .unwrap();
    store
        .upsert_watermark(
            "fred",
            "fred_series_observations",
            "us_market_vix_close",
            NaiveDate::from_ymd_opt(2026, 6, 5).unwrap(),
        )
        .await
        .unwrap();
    store
        .insert_ingestion_run(&IngestionRunRecord {
            run_id: "run-fred-failed".to_string(),
            job_id: Some("backfill:fred:fred_series_observations:BAMLH0A0HYM2".to_string()),
            source_id: "fred".to_string(),
            dataset_id: "fred_series_observations".to_string(),
            target_id: Some("us_credit_high_yield_spread".to_string()),
            run_mode: "backfill".to_string(),
            status: "failed".to_string(),
            started_at: latest_finished_at,
            finished_at: Some(latest_finished_at),
            attempt: 1,
            watermark_before_json: None,
            watermark_after_json: None,
            records_read: 0,
            records_written: 0,
            error_type: Some("backfill_chunk_failed".to_string()),
            error_message: Some("temporary upstream error".to_string()),
        })
        .await
        .unwrap();

    let summaries = store
        .load_ingestion_source_health_summaries()
        .await
        .unwrap();
    let fred = summaries
        .iter()
        .find(|summary| summary.source_id == "fred")
        .expect("fred source health summary");

    assert_eq!(fred.total_run_count, 2);
    assert_eq!(fred.successful_run_count, 1);
    assert_eq!(fred.failed_run_count, 1);
    assert_eq!(fred.failures_after_last_success, 1);
    assert_eq!(fred.latest_status.as_deref(), Some("failed"));
    assert_eq!(
        fred.last_successful_period,
        Some(NaiveDate::from_ymd_opt(2026, 6, 5).unwrap())
    );
    assert_eq!(
        fred.latest_error_message.as_deref(),
        Some("temporary upstream error")
    );
}

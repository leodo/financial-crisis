use chrono::{NaiveDate, Utc};
use uuid::Uuid;

use crate::sqlite::tests::in_memory_store;
use crate::sqlite::{IngestionRunRecord, RawResponseRecord, FRED_DATASET_ID};

#[tokio::test]
async fn sqlite_store_round_trips_seeded_observations() {
    let store = in_memory_store().await;
    store.seed_fred_metadata().await.unwrap();

    let indicators = store.load_indicators().await.unwrap();
    assert!(indicators.len() >= 10);

    let indicator = indicators
        .iter()
        .find(|indicator| indicator.indicator_id == "us_market_vix_close")
        .unwrap()
        .clone();
    let observation = fc_domain::Observation {
        indicator_id: indicator.indicator_id,
        entity_id: "us".to_string(),
        as_of_date: NaiveDate::from_ymd_opt(2020, 3, 16).unwrap(),
        period_start: Some(NaiveDate::from_ymd_opt(2020, 3, 16).unwrap()),
        period_end: Some(NaiveDate::from_ymd_opt(2020, 3, 16).unwrap()),
        frequency: indicator.frequency,
        value: 82.69,
        unit: indicator.unit,
        source_id: "fred".to_string(),
        dataset_id: FRED_DATASET_ID.to_string(),
        revision_time: None,
        publication_time: None,
        quality_score: 95.0,
        quality_flags: Vec::new(),
    };
    store.insert_observations(&[observation]).await.unwrap();
    let observations = store
        .load_observations("us", NaiveDate::from_ymd_opt(2020, 3, 17).unwrap())
        .await
        .unwrap();

    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].value, 82.69);
}

#[tokio::test]
async fn sqlite_store_loads_latest_observation_lineage() {
    let store = in_memory_store().await;
    store.seed_fred_metadata().await.unwrap();

    let fetched_at = Utc::now();
    let raw_payload_id = Uuid::new_v4();
    let run_id = "run-fred-vix-lineage".to_string();
    store
        .insert_ingestion_run(&IngestionRunRecord {
            run_id: run_id.clone(),
            job_id: Some("backfill:fred:VIXCLS".to_string()),
            source_id: "fred".to_string(),
            dataset_id: FRED_DATASET_ID.to_string(),
            target_id: Some("us_market_vix_close".to_string()),
            run_mode: "backfill".to_string(),
            status: "success".to_string(),
            started_at: fetched_at,
            finished_at: Some(fetched_at),
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
        .insert_raw_response(&RawResponseRecord {
            raw_payload_id,
            run_id: Some(run_id.clone()),
            source_id: "fred".to_string(),
            dataset_id: FRED_DATASET_ID.to_string(),
            request_url: "https://fred.stlouisfed.org/graph/fredgraph.csv?id=VIXCLS".to_string(),
            request_params_hash: Some("hash".to_string()),
            response_hash: "response-hash".to_string(),
            content_type: "text/csv".to_string(),
            content_length: 42,
            raw_file_path: "data/raw/fred/VIXCLS/test.csv".to_string(),
            fetched_at,
        })
        .await
        .unwrap();

    let observation = fc_domain::Observation {
        indicator_id: "us_market_vix_close".to_string(),
        entity_id: "us".to_string(),
        as_of_date: NaiveDate::from_ymd_opt(2026, 6, 5).unwrap(),
        period_start: Some(NaiveDate::from_ymd_opt(2026, 6, 5).unwrap()),
        period_end: Some(NaiveDate::from_ymd_opt(2026, 6, 5).unwrap()),
        frequency: fc_domain::Frequency::Daily,
        value: 21.51,
        unit: "index".to_string(),
        source_id: "fred".to_string(),
        dataset_id: FRED_DATASET_ID.to_string(),
        revision_time: None,
        publication_time: None,
        quality_score: 95.0,
        quality_flags: Vec::new(),
    };
    store
        .insert_observations_with_raw_payload(&[observation], Some(raw_payload_id))
        .await
        .unwrap();

    let lineage = store
        .load_latest_observation_lineage(
            "us_market_vix_close",
            "us",
            NaiveDate::from_ymd_opt(2026, 6, 9).unwrap(),
        )
        .await
        .unwrap()
        .unwrap();

    assert_eq!(lineage.indicator_id, "us_market_vix_close");
    assert_eq!(
        lineage.as_of_date,
        NaiveDate::from_ymd_opt(2026, 6, 5).unwrap()
    );
    let raw_payload_id_string = raw_payload_id.to_string();
    assert_eq!(
        lineage.raw_payload_id.as_deref(),
        Some(raw_payload_id_string.as_str())
    );
    assert_eq!(lineage.run_id.as_deref(), Some(run_id.as_str()));
    assert_eq!(lineage.run_status.as_deref(), Some("success"));
    assert_eq!(lineage.records_written, Some(1));
    assert_eq!(lineage.response_hash.as_deref(), Some("response-hash"));
}

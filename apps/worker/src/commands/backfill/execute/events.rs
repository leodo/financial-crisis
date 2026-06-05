use anyhow::Result;
use chrono::Utc;
use fc_ingestion::{GdeltConnector, SecEdgarConnector};
use fc_storage::{
    RawResponseRecord, GDELT_DOC_DATASET_ID, SEC_EVENTS_DATASET_ID, SEC_SUBMISSIONS_DATASET_ID,
};
use uuid::Uuid;

use super::super::options::BackfillOptions;
use super::shared::open_seeded_store;

pub(crate) async fn backfill_gdelt_with_options(options: BackfillOptions) -> Result<()> {
    let store = open_seeded_store().await?;
    let connector = GdeltConnector::new();
    let effective_start = if let Some(overlap_days) = options.watermark_overlap_days {
        let watermark = store
            .load_watermark_date(
                "gdelt",
                GDELT_DOC_DATASET_ID,
                "global_news_financial_stress_count",
            )
            .await?;
        options.effective_start(watermark, overlap_days)
    } else {
        options.start
    };
    if effective_start > options.end {
        println!("GDELT backfill skipped: watermark is already beyond requested range.");
        return Ok(());
    }

    println!(
        "Backfilling GDELT timeline aggregates into {} [{}..{}]",
        crate::sqlite_path(),
        effective_start,
        options.end
    );
    let output = connector
        .backfill_range(effective_start, options.end)
        .await?;
    let raw_root = crate::raw_data_dir();
    let raw_path = crate::write_raw_payload(
        &raw_root,
        "gdelt",
        "global_news_financial_stress_count",
        "json",
        &output.payload_body,
    )?;
    let raw_payload_id = Uuid::new_v4();
    store
        .insert_raw_response(&RawResponseRecord {
            raw_payload_id,
            source_id: "gdelt".to_string(),
            dataset_id: GDELT_DOC_DATASET_ID.to_string(),
            request_url: output.payload_url.clone(),
            request_params_hash: Some(crate::simple_hash(&output.payload_url)),
            response_hash: crate::simple_hash(&output.payload_body),
            content_type: "application/json".to_string(),
            content_length: output.payload_body.len() as i64,
            raw_file_path: crate::path_to_string(&raw_path),
            fetched_at: Utc::now(),
        })
        .await?;
    store.insert_observations(&output.observations).await?;
    store
        .replace_alerts_for_scope("gdelt_daily", effective_start, options.end, &output.alerts)
        .await?;
    if let Some(latest_date) = output.latest_date {
        store
            .upsert_watermark(
                "gdelt",
                GDELT_DOC_DATASET_ID,
                "global_news_financial_stress_count",
                latest_date,
            )
            .await?;
    }

    println!(
        "GDELT backfill completed: {} observations, {} alerts",
        output.observations.len(),
        output.alerts.len()
    );
    Ok(())
}

pub(crate) async fn backfill_sec_edgar_with_options(options: BackfillOptions) -> Result<()> {
    let store = open_seeded_store().await?;

    println!(
        "Backfilling SEC EDGAR filing events into {} [{}..{}]",
        crate::sqlite_path(),
        options.start,
        options.end
    );

    let connector = SecEdgarConnector::new();
    let output = connector.backfill_range(options.start, options.end).await?;
    let raw_root = crate::raw_data_dir();

    for payload in &output.payloads {
        let raw_path = crate::write_raw_payload(
            &raw_root,
            &payload.source_id,
            SEC_SUBMISSIONS_DATASET_ID,
            crate::raw_file_extension(&payload.content_type),
            &payload.body,
        )?;
        store
            .insert_raw_response(&RawResponseRecord {
                raw_payload_id: payload.raw_payload_id,
                source_id: payload.source_id.clone(),
                dataset_id: payload.dataset_id.clone(),
                request_url: payload.request_url.clone(),
                request_params_hash: Some(crate::simple_hash(&payload.request_url)),
                response_hash: payload.response_hash.clone(),
                content_type: payload.content_type.clone(),
                content_length: payload.body.len() as i64,
                raw_file_path: crate::path_to_string(&raw_path),
                fetched_at: payload.fetched_at,
            })
            .await?;
    }

    store.insert_observations(&output.observations).await?;
    store
        .replace_alerts_for_scope(
            "sec_edgar_daily",
            options.start,
            options.end,
            &output.alerts,
        )
        .await?;
    if let Some(latest_filing_date) = output.latest_filing_date {
        store
            .upsert_watermark("sec_edgar", SEC_EVENTS_DATASET_ID, "us", latest_filing_date)
            .await?;
    }

    println!(
        "SEC EDGAR backfill completed: {} payloads, {} filings, {} observations, {} alerts",
        output.payloads.len(),
        output.filing_count,
        output.observations.len(),
        output.alerts.len()
    );
    Ok(())
}

use anyhow::Result;
use chrono::Utc;
use fc_ingestion::{GdeltConnector, RunMode, SecEdgarConnector};
use fc_storage::{
    IngestionRunRecord, RawResponseRecord, GDELT_DOC_DATASET_ID, SEC_EVENTS_DATASET_ID,
    SEC_SUBMISSIONS_DATASET_ID,
};
use uuid::Uuid;

use super::super::options::BackfillOptions;
use super::shared::{
    open_seeded_store, run_mode_code, truncate_error_message, watermark_state_json,
};

pub(crate) async fn backfill_gdelt_with_options(options: BackfillOptions) -> Result<()> {
    let store = open_seeded_store().await?;
    let connector = GdeltConnector::new();
    let watermark = if options.watermark_overlap_days.is_some() {
        store
            .load_watermark_date(
                "gdelt",
                GDELT_DOC_DATASET_ID,
                "global_news_financial_stress_count",
            )
            .await?
    } else {
        None
    };
    let effective_start = if let Some(overlap_days) = options.watermark_overlap_days {
        options.effective_start(watermark, overlap_days)
    } else {
        options.start
    };
    let run_id = Uuid::new_v4().to_string();
    let started_at = Utc::now();
    let watermark_before_json = watermark_state_json(watermark);
    if effective_start > options.end {
        store
            .insert_ingestion_run(&IngestionRunRecord {
                run_id,
                job_id: Some("backfill:gdelt:global_news_financial_stress_count".to_string()),
                source_id: "gdelt".to_string(),
                dataset_id: GDELT_DOC_DATASET_ID.to_string(),
                target_id: Some("global_news_financial_stress_count".to_string()),
                run_mode: run_mode_code(RunMode::Backfill).to_string(),
                status: "skipped".to_string(),
                started_at,
                finished_at: Some(Utc::now()),
                attempt: 1,
                watermark_before_json,
                watermark_after_json: watermark_state_json(watermark),
                records_read: 0,
                records_written: 0,
                error_type: None,
                error_message: None,
            })
            .await?;
        println!("GDELT backfill skipped: watermark is already beyond requested range.");
        return Ok(());
    }

    println!(
        "Backfilling GDELT timeline aggregates into {} [{}..{}]",
        crate::sqlite_path(),
        effective_start,
        options.end
    );
    store
        .insert_ingestion_run(&IngestionRunRecord {
            run_id: run_id.clone(),
            job_id: Some("backfill:gdelt:global_news_financial_stress_count".to_string()),
            source_id: "gdelt".to_string(),
            dataset_id: GDELT_DOC_DATASET_ID.to_string(),
            target_id: Some("global_news_financial_stress_count".to_string()),
            run_mode: run_mode_code(RunMode::Backfill).to_string(),
            status: "running".to_string(),
            started_at,
            finished_at: None,
            attempt: 1,
            watermark_before_json: watermark_before_json.clone(),
            watermark_after_json: None,
            records_read: 0,
            records_written: 0,
            error_type: None,
            error_message: None,
        })
        .await?;
    let result: Result<(usize, usize, Option<chrono::NaiveDate>)> = async {
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
                run_id: Some(run_id.clone()),
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
        store
            .insert_observations_with_raw_payload(&output.observations, Some(raw_payload_id))
            .await?;
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
        Ok((
            output.observations.len(),
            output.alerts.len(),
            output.latest_date,
        ))
    }
    .await;

    let (observation_count, alert_count, latest_date) = match result {
        Ok(summary) => {
            store
                .insert_ingestion_run(&IngestionRunRecord {
                    run_id,
                    job_id: Some("backfill:gdelt:global_news_financial_stress_count".to_string()),
                    source_id: "gdelt".to_string(),
                    dataset_id: GDELT_DOC_DATASET_ID.to_string(),
                    target_id: Some("global_news_financial_stress_count".to_string()),
                    run_mode: run_mode_code(RunMode::Backfill).to_string(),
                    status: "success".to_string(),
                    started_at,
                    finished_at: Some(Utc::now()),
                    attempt: 1,
                    watermark_before_json,
                    watermark_after_json: watermark_state_json(summary.2),
                    records_read: summary.0 as i64,
                    records_written: summary.0 as i64,
                    error_type: None,
                    error_message: None,
                })
                .await?;
            summary
        }
        Err(error) => {
            let error_message = format!("{error:#}");
            store
                .insert_ingestion_run(&IngestionRunRecord {
                    run_id,
                    job_id: Some("backfill:gdelt:global_news_financial_stress_count".to_string()),
                    source_id: "gdelt".to_string(),
                    dataset_id: GDELT_DOC_DATASET_ID.to_string(),
                    target_id: Some("global_news_financial_stress_count".to_string()),
                    run_mode: run_mode_code(RunMode::Backfill).to_string(),
                    status: "failed".to_string(),
                    started_at,
                    finished_at: Some(Utc::now()),
                    attempt: 1,
                    watermark_before_json,
                    watermark_after_json: None,
                    records_read: 0,
                    records_written: 0,
                    error_type: Some("backfill_failed".to_string()),
                    error_message: Some(truncate_error_message(&error_message)),
                })
                .await?;
            return Err(error);
        }
    };

    println!("GDELT backfill completed: {observation_count} observations, {alert_count} alerts");
    if let Some(latest_date) = latest_date {
        tracing::info!(%latest_date, "GDELT watermark advanced");
    }
    Ok(())
}

pub(crate) async fn backfill_sec_edgar_with_options(options: BackfillOptions) -> Result<()> {
    let store = open_seeded_store().await?;
    let run_id = Uuid::new_v4().to_string();
    let started_at = Utc::now();
    let watermark = store
        .load_watermark_date("sec_edgar", SEC_EVENTS_DATASET_ID, "us")
        .await?;
    let watermark_before_json = watermark_state_json(watermark);

    println!(
        "Backfilling SEC EDGAR filing events into {} [{}..{}]",
        crate::sqlite_path(),
        options.start,
        options.end
    );

    let connector = SecEdgarConnector::new();
    store
        .insert_ingestion_run(&IngestionRunRecord {
            run_id: run_id.clone(),
            job_id: Some("backfill:sec_edgar:us".to_string()),
            source_id: "sec_edgar".to_string(),
            dataset_id: SEC_EVENTS_DATASET_ID.to_string(),
            target_id: Some("us".to_string()),
            run_mode: run_mode_code(RunMode::Backfill).to_string(),
            status: "running".to_string(),
            started_at,
            finished_at: None,
            attempt: 1,
            watermark_before_json: watermark_before_json.clone(),
            watermark_after_json: None,
            records_read: 0,
            records_written: 0,
            error_type: None,
            error_message: None,
        })
        .await?;
    let result: Result<(usize, usize, usize, usize, Option<chrono::NaiveDate>)> = async {
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
                    run_id: Some(run_id.clone()),
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
        Ok((
            output.payloads.len(),
            output.filing_count,
            output.observations.len(),
            output.alerts.len(),
            output.latest_filing_date,
        ))
    }
    .await;

    let (payload_count, filing_count, observation_count, alert_count, latest_filing_date) =
        match result {
            Ok(summary) => {
                store
                    .insert_ingestion_run(&IngestionRunRecord {
                        run_id,
                        job_id: Some("backfill:sec_edgar:us".to_string()),
                        source_id: "sec_edgar".to_string(),
                        dataset_id: SEC_EVENTS_DATASET_ID.to_string(),
                        target_id: Some("us".to_string()),
                        run_mode: run_mode_code(RunMode::Backfill).to_string(),
                        status: "success".to_string(),
                        started_at,
                        finished_at: Some(Utc::now()),
                        attempt: 1,
                        watermark_before_json,
                        watermark_after_json: watermark_state_json(summary.4),
                        records_read: summary.1 as i64,
                        records_written: summary.2 as i64,
                        error_type: None,
                        error_message: None,
                    })
                    .await?;
                summary
            }
            Err(error) => {
                let error_message = format!("{error:#}");
                store
                    .insert_ingestion_run(&IngestionRunRecord {
                        run_id,
                        job_id: Some("backfill:sec_edgar:us".to_string()),
                        source_id: "sec_edgar".to_string(),
                        dataset_id: SEC_EVENTS_DATASET_ID.to_string(),
                        target_id: Some("us".to_string()),
                        run_mode: run_mode_code(RunMode::Backfill).to_string(),
                        status: "failed".to_string(),
                        started_at,
                        finished_at: Some(Utc::now()),
                        attempt: 1,
                        watermark_before_json,
                        watermark_after_json: None,
                        records_read: 0,
                        records_written: 0,
                        error_type: Some("backfill_failed".to_string()),
                        error_message: Some(truncate_error_message(&error_message)),
                    })
                    .await?;
                return Err(error);
            }
        };

    println!(
        "SEC EDGAR backfill completed: {payload_count} payloads, {filing_count} filings, {observation_count} observations, {alert_count} alerts"
    );
    if let Some(latest_filing_date) = latest_filing_date {
        tracing::info!(%latest_filing_date, "SEC EDGAR watermark advanced");
    }
    Ok(())
}

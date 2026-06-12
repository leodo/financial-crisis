use anyhow::{bail, Result};
use chrono::{NaiveDate, Utc};
use fc_ingestion::{Connector, FetchPlan, RunMode};
use fc_storage::{ExternalIndicatorMapping, IngestionRunRecord, RawResponseRecord, SqliteStore};
use uuid::Uuid;

use super::super::options::BackfillOptions;

pub(super) async fn open_seeded_store() -> Result<SqliteStore> {
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;
    store.seed_fred_metadata().await?;
    Ok(store)
}

pub(super) async fn backfill_mappings(
    connector: &dyn Connector,
    mappings: Vec<ExternalIndicatorMapping>,
    dataset_id: &str,
    options: BackfillOptions,
    label: &str,
) -> Result<()> {
    let store = open_seeded_store().await?;
    let raw_root = crate::raw_data_dir();
    let mappings = options.filter_mappings(mappings);
    if mappings.is_empty() {
        bail!("{label} backfill found no mappings matching the requested filters");
    }
    let mut total_written = 0_usize;
    let mut failures = Vec::new();
    let mut skipped_mappings = 0_usize;
    let chunks = options.chunks();
    let chunk_count = chunks.len();
    let source_id = connector.describe().source_id;
    for mapping in mappings {
        let watermark = if options.respect_frequency_watermark {
            store
                .load_watermark_date(&source_id, dataset_id, &mapping.indicator_id)
                .await?
        } else {
            None
        };
        if options.should_skip_due_to_frequency_watermark(mapping.frequency, watermark) {
            skipped_mappings += 1;
            println!(
                "skipped {} ({}) from {}: {:?} series watermark {:?} is still within refresh cadence",
                mapping.indicator_id,
                mapping.external_code,
                source_id,
                mapping.frequency,
                watermark
            );
            continue;
        }

        for (chunk_index, (chunk_start, chunk_end)) in chunks.iter().copied().enumerate() {
            let run_id = Uuid::new_v4().to_string();
            let started_at = Utc::now();
            let watermark_before_json = watermark_state_json(watermark);
            let job_id = format!(
                "backfill:{}:{}:{}:{}",
                source_id, dataset_id, mapping.indicator_id, mapping.external_code
            );
            let plan = FetchPlan {
                source_id: source_id.clone(),
                dataset_id: dataset_id.to_string(),
                target_id: mapping.indicator_id.clone(),
                external_code: Some(mapping.external_code.clone()),
                run_mode: RunMode::Backfill,
                requested_start: Some(chunk_start),
                requested_end: Some(chunk_end),
                frequency: mapping.frequency,
            };
            tracing::info!(
                indicator_id = %plan.target_id,
                external_code = %mapping.external_code,
                source_id = %plan.source_id,
                chunk = chunk_index + 1,
                chunks = chunk_count,
                start = %chunk_start,
                end = %chunk_end,
                "fetching observations"
            );
            store
                .insert_ingestion_run(&IngestionRunRecord {
                    run_id: run_id.clone(),
                    job_id: Some(job_id.clone()),
                    source_id: source_id.clone(),
                    dataset_id: dataset_id.to_string(),
                    target_id: Some(mapping.indicator_id.clone()),
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

            let result: Result<(usize, Option<NaiveDate>)> = async {
                let payload = connector.fetch(&plan).await?;
                let raw_path = crate::write_raw_payload(
                    &raw_root,
                    &payload.source_id,
                    &mapping.external_code,
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
                let batch = connector.parse(&plan, &payload)?;
                let latest_date = batch
                    .observations
                    .iter()
                    .map(|observation| observation.as_of_date)
                    .max();
                let written = batch.observations.len();
                store
                    .insert_observations_with_raw_payload(
                        &batch.observations,
                        Some(payload.raw_payload_id),
                    )
                    .await?;
                if let Some(latest_date) = latest_date {
                    store
                        .upsert_watermark(
                            &payload.source_id,
                            &payload.dataset_id,
                            &mapping.indicator_id,
                            latest_date,
                        )
                        .await?;
                }
                if written == 0 {
                    tracing::warn!(
                        indicator_id = %mapping.indicator_id,
                        external_code = %mapping.external_code,
                        start = %chunk_start,
                        end = %chunk_end,
                        "no observations were written for requested range"
                    );
                }
                println!(
                    "backfilled {} ({}) from {} with {} observations [{}..{}]",
                    mapping.indicator_id,
                    mapping.external_code,
                    payload.source_id,
                    written,
                    chunk_start,
                    chunk_end
                );
                for warning in batch.warnings.iter().take(3) {
                    tracing::warn!(%warning, indicator_id = %mapping.indicator_id, "parse warning");
                }
                Ok((written, latest_date))
            }
            .await;

            match result {
                Ok((written, latest_date)) => {
                    store
                        .insert_ingestion_run(&IngestionRunRecord {
                            run_id,
                            job_id: Some(job_id),
                            source_id: source_id.clone(),
                            dataset_id: dataset_id.to_string(),
                            target_id: Some(mapping.indicator_id.clone()),
                            run_mode: run_mode_code(RunMode::Backfill).to_string(),
                            status: "success".to_string(),
                            started_at,
                            finished_at: Some(Utc::now()),
                            attempt: 1,
                            watermark_before_json,
                            watermark_after_json: watermark_state_json(latest_date),
                            records_read: written as i64,
                            records_written: written as i64,
                            error_type: None,
                            error_message: None,
                        })
                        .await?;
                    total_written += written;
                }
                Err(error) => {
                    let error_message = format!("{error:#}");
                    store
                        .insert_ingestion_run(&IngestionRunRecord {
                            run_id,
                            job_id: Some(job_id),
                            source_id: source_id.clone(),
                            dataset_id: dataset_id.to_string(),
                            target_id: Some(mapping.indicator_id.clone()),
                            run_mode: run_mode_code(RunMode::Backfill).to_string(),
                            status: "failed".to_string(),
                            started_at,
                            finished_at: Some(Utc::now()),
                            attempt: 1,
                            watermark_before_json,
                            watermark_after_json: None,
                            records_read: 0,
                            records_written: 0,
                            error_type: Some("backfill_chunk_failed".to_string()),
                            error_message: Some(truncate_error_message(&error_message)),
                        })
                        .await?;
                    let failure = format!(
                        "{} ({}) [{}..{}]: {error_message}",
                        mapping.indicator_id, mapping.external_code, chunk_start, chunk_end
                    );
                    tracing::warn!(%failure, "backfill chunk failed");
                    failures.push(failure);
                }
            }
        }
    }

    if failures.is_empty() {
        println!(
            "{} backfill completed: {} observations written to {}{}",
            label,
            total_written,
            crate::sqlite_path(),
            if skipped_mappings > 0 {
                format!(", {skipped_mappings} mapping(s) skipped by refresh cadence")
            } else {
                String::new()
            }
        );
        Ok(())
    } else {
        println!(
            "{} backfill partially completed: {} observations written to {}, {} chunk(s) failed",
            label,
            total_written,
            crate::sqlite_path(),
            failures.len()
        );
        for failure in failures.iter().take(5) {
            println!("  failed: {failure}");
        }
        bail!(
            "{} backfill had {} failed chunk(s); retry the command to fill missing gaps",
            label,
            failures.len()
        )
    }
}

pub(super) fn watermark_state_json(date: Option<NaiveDate>) -> Option<String> {
    date.map(|date| serde_json::json!({ "last_successful_period": date.to_string() }).to_string())
}

pub(super) fn run_mode_code(run_mode: RunMode) -> &'static str {
    match run_mode {
        RunMode::Discover => "discover",
        RunMode::Backfill => "backfill",
        RunMode::Incremental => "incremental",
        RunMode::Repair => "repair",
        RunMode::MetadataRefresh => "metadata_refresh",
    }
}

pub(super) fn truncate_error_message(message: &str) -> String {
    const MAX_ERROR_MESSAGE_CHARS: usize = 1000;
    message.chars().take(MAX_ERROR_MESSAGE_CHARS).collect()
}

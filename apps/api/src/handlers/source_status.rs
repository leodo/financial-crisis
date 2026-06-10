use std::{collections::HashMap, sync::Arc};

use axum::{extract::State, Json};
use fc_domain::{DataSource, SourceStatus};
use fc_storage::{IngestionSourceHealthSummary, SqliteStore};
use serde_json::json;

use crate::{data_source::AppDataSource, AppState};

pub async fn sources(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let data = state.data().await;
    let mut sources = data.sources;

    if let AppDataSource::Sqlite { path } = state.source() {
        match load_sqlite_ingestion_health(path).await {
            Ok(ingestion_health) => {
                enrich_sources_with_ingestion_health(&mut sources, &ingestion_health);
            }
            Err(error) => {
                tracing::warn!(
                    sqlite_path = path,
                    error = %format!("{error:#}"),
                    "failed to enrich source health from ingestion runs"
                );
                mark_source_health_unverified(&mut sources);
            }
        }
    }

    Json(json!(sources))
}

async fn load_sqlite_ingestion_health(
    sqlite_path: &str,
) -> anyhow::Result<Vec<IngestionSourceHealthSummary>> {
    let store = SqliteStore::connect(sqlite_path).await?;
    store.migrate().await?;
    Ok(store.load_ingestion_source_health_summaries().await?)
}

fn enrich_sources_with_ingestion_health(
    sources: &mut [DataSource],
    ingestion_health: &[IngestionSourceHealthSummary],
) {
    let health_by_source = ingestion_health
        .iter()
        .map(|summary| (summary.source_id.as_str(), summary))
        .collect::<HashMap<_, _>>();

    for source in sources {
        match health_by_source.get(source.source_id.as_str()).copied() {
            Some(summary) => apply_ingestion_summary(source, summary),
            None => mark_source_without_run_evidence(source),
        }
    }
}

fn apply_ingestion_summary(source: &mut DataSource, summary: &IngestionSourceHealthSummary) {
    source.health.last_success_at = summary.last_success_at;
    source.health.consecutive_failures = summary.failures_after_last_success.max(0) as u32;
    if summary.failures_after_last_success > 0 || summary.latest_status.as_deref() == Some("failed")
    {
        source.health.status = SourceStatus::PartialFailure;
    }

    source.health.message = format!(
        "{}; refresh_status={}; last_success={}; data_period={}; failures_after_success={}",
        source.health.message,
        summary.latest_status.as_deref().unwrap_or("unknown"),
        summary
            .last_success_at
            .map(|timestamp| timestamp.to_rfc3339())
            .unwrap_or_else(|| "never".to_string()),
        summary
            .last_successful_period
            .map(|period| period.to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        summary.failures_after_last_success
    );
}

fn mark_source_without_run_evidence(source: &mut DataSource) {
    source.health.last_success_at = None;
    source.health.consecutive_failures = 0;
    source.health.message = format!(
        "{}; no ingest_runs evidence is available for this source yet",
        source.health.message
    );
}

fn mark_source_health_unverified(sources: &mut [DataSource]) {
    for source in sources {
        if source.health.status == SourceStatus::Prototype
            || source.health.status == SourceStatus::Disabled
        {
            continue;
        }
        source.health.message = format!(
            "{}; ingestion run health could not be verified",
            source.health.message
        );
    }
}

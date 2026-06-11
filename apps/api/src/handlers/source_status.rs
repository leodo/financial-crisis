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
        "{}; 刷新状态={}; 最近成功刷新={}; 抓取水位={}; 成功后失败次数={}",
        source.health.message,
        refresh_status_label(summary.latest_status.as_deref()),
        summary
            .last_success_at
            .map(|timestamp| timestamp.to_rfc3339())
            .unwrap_or_else(|| "暂无".to_string()),
        summary
            .last_successful_period
            .map(|period| period.to_string())
            .unwrap_or_else(|| "未知".to_string()),
        summary.failures_after_last_success
    );
}

fn refresh_status_label(status: Option<&str>) -> &'static str {
    match status {
        Some("success") => "成功",
        Some("failed") => "失败",
        Some("running") => "运行中",
        Some("skipped") => "已跳过",
        _ => "未知",
    }
}

fn mark_source_without_run_evidence(source: &mut DataSource) {
    source.health.last_success_at = None;
    source.health.consecutive_failures = 0;
    if source.production_allowed
        && matches!(
            source.health.status,
            SourceStatus::Healthy | SourceStatus::Delayed
        )
    {
        source.health.status = SourceStatus::Delayed;
        source.health.quality_score = source.health.quality_score.min(75.0);
    }
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

#[cfg(test)]
mod tests {
    use fc_domain::{SourceHealth, SourcePriority};

    use super::*;

    fn test_source(status: SourceStatus, production_allowed: bool) -> DataSource {
        DataSource {
            source_id: "test_source".to_string(),
            display_name: "Test Source".to_string(),
            source_type: "macro".to_string(),
            priority: SourcePriority::P1,
            access_method: "csv".to_string(),
            documentation_url: None,
            production_allowed,
            license_note: "test".to_string(),
            health: SourceHealth {
                status,
                last_success_at: Some(chrono::Utc::now()),
                lag_seconds: Some(0),
                consecutive_failures: 0,
                quality_score: 92.0,
                message: "source healthy".to_string(),
            },
        }
    }

    #[test]
    fn production_source_without_ingestion_run_evidence_is_not_healthy() {
        let mut source = test_source(SourceStatus::Healthy, true);

        mark_source_without_run_evidence(&mut source);

        assert_eq!(source.health.status, SourceStatus::Delayed);
        assert!(source.health.last_success_at.is_none());
        assert_eq!(source.health.quality_score, 75.0);
        assert!(source
            .health
            .message
            .contains("no ingest_runs evidence is available"));
    }

    #[test]
    fn prototype_source_without_ingestion_run_evidence_keeps_prototype_status() {
        let mut source = test_source(SourceStatus::Prototype, false);

        mark_source_without_run_evidence(&mut source);

        assert_eq!(source.health.status, SourceStatus::Prototype);
        assert!(source.health.last_success_at.is_none());
        assert_eq!(source.health.quality_score, 92.0);
    }
}

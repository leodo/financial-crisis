use std::collections::BTreeMap;

use anyhow::Context;
use fc_domain::{AssessmentHistoryPoint, ModelReleaseRecord};

use super::ReleaseReviewOptions;

#[derive(Debug, Clone)]
pub(crate) struct ReleaseReviewRuntimeSnapshot {
    pub(crate) assessment: fc_domain::AssessmentSnapshot,
    pub(crate) backtests: Vec<fc_domain::BacktestScenarioSummary>,
    pub(crate) method: crate::AuditMethodResponseWire,
    pub(crate) history: Vec<AssessmentHistoryPoint>,
}

pub(super) async fn fetch_release_review_runtime_snapshot(
    api_reload_url: &str,
    history_limit: usize,
) -> anyhow::Result<ReleaseReviewRuntimeSnapshot> {
    let api_base_url = api_reload_url
        .strip_suffix("/api/system/reload")
        .with_context(|| {
            format!(
                "cannot derive API base URL from reload URL {api_reload_url}; expected it to end with /api/system/reload"
            )
        })?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()?;
    let assessment: fc_domain::AssessmentSnapshot =
        crate::fetch_api_json(&client, api_base_url, "/api/assessment/current").await?;
    let backtests: Vec<fc_domain::BacktestScenarioSummary> =
        crate::fetch_api_json(&client, api_base_url, "/api/backtests").await?;
    let method: crate::AuditMethodResponseWire =
        crate::fetch_api_json(&client, api_base_url, "/api/assessment/method").await?;
    let history_path = format!("/api/assessment/history?limit={history_limit}");
    let history: Vec<AssessmentHistoryPoint> =
        crate::fetch_api_json(&client, api_base_url, &history_path).await?;
    Ok(ReleaseReviewRuntimeSnapshot {
        assessment,
        backtests,
        method,
        history,
    })
}

pub(crate) async fn activate_release_for_review(
    store: &fc_storage::SqliteStore,
    market_scope: &str,
    release_id: &str,
    options: &ReleaseReviewOptions,
    stage: &str,
) -> anyhow::Result<()> {
    store
        .activate_model_release(market_scope, release_id, &options.updated_by)
        .await?;
    println!("Review step {stage}: activated {release_id}.");
    println!(
        "Review step {stage}: reloading API runtime via {api_reload_url} (history_mode={} history_limit={}).",
        options.history_mode.as_label(),
        options.history_limit,
        api_reload_url = options.api_reload_url
    );
    crate::reload_api_runtime_with_history_options(
        &options.api_reload_url,
        options.history_mode,
        Some(options.history_limit),
    )
    .await?;
    println!("Review step {stage}: runtime ready.");
    Ok(())
}

pub(crate) async fn restore_release_review_state(
    store: &fc_storage::SqliteStore,
    market_scope: &str,
    original_active_release_id: &str,
    original_records: &BTreeMap<String, ModelReleaseRecord>,
    api_reload_url: &str,
    updated_by: &str,
) -> anyhow::Result<()> {
    store
        .activate_model_release(market_scope, original_active_release_id, updated_by)
        .await?;
    crate::reload_api_runtime(api_reload_url).await?;
    for record in original_records.values() {
        store.upsert_model_release(record).await?;
    }
    Ok(())
}

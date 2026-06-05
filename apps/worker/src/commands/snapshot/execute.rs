use anyhow::{bail, Context};
use fc_domain::PredictionSnapshotRecord;
use fc_storage::SqliteStore;

use super::options::{
    PredictionSnapshotExportOptions, PredictionSnapshotQueryOptions, SnapshotDatasetExportOptions,
};
use super::render::{write_dataset_export, write_snapshot_export};

pub(crate) async fn research_prediction_snapshot_list(args: &[String]) -> anyhow::Result<()> {
    let options = PredictionSnapshotQueryOptions::parse(args)?;
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;
    let snapshots = load_prediction_snapshots(&store, &options).await?;
    if snapshots.is_empty() {
        println!("No prediction snapshots found.");
        return Ok(());
    }
    println!(
        "{:<12} {:<18} {:<16} {:<12} {:<10} {:<8} {:<10}",
        "as_of_date", "market_scope", "release_id", "prob_mode", "p20d", "posture", "freshness"
    );
    for snapshot in snapshots {
        println!(
            "{:<12} {:<18} {:<16} {:<12} {:<10} {:<8} {:<10}",
            snapshot.as_of_date,
            crate::truncate_text(&snapshot.market_scope, 18),
            crate::truncate_text(snapshot.release_id.as_deref().unwrap_or("inline"), 16),
            crate::truncate_text(&snapshot.probability_mode, 12),
            crate::format_pct(snapshot.calibrated_p_20d),
            crate::truncate_text(&snapshot.posture, 8),
            crate::truncate_text(&snapshot.freshness_status, 10),
        );
    }
    Ok(())
}

pub(crate) async fn research_prediction_snapshot_export(args: &[String]) -> anyhow::Result<()> {
    let options = PredictionSnapshotExportOptions::parse(args)?;
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;
    let snapshots = load_prediction_snapshots(&store, &options.query).await?;
    write_snapshot_export(&snapshots, options.format, options.output_path.as_deref())?;
    Ok(())
}

pub(crate) async fn research_prediction_snapshot_dataset(args: &[String]) -> anyhow::Result<()> {
    let options = SnapshotDatasetExportOptions::parse(args)?;
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;
    let snapshots = load_training_snapshots(&store, &options.query).await?;
    let dataset = super::super::pipeline::build_pipeline_dataset_rows(&snapshots);
    write_dataset_export(
        &dataset,
        &super::super::pipeline::transitional_feature_names(),
        options.format,
        options.output_path.as_deref(),
    )?;
    Ok(())
}

pub(crate) async fn load_prediction_snapshots(
    store: &SqliteStore,
    options: &PredictionSnapshotQueryOptions,
) -> anyhow::Result<Vec<PredictionSnapshotRecord>> {
    Ok(store
        .list_prediction_snapshots(
            options.market_scope.as_deref(),
            options.release_id.as_deref(),
            options.from,
            options.to,
            options.limit,
        )
        .await?)
}

pub(crate) async fn load_training_snapshots(
    store: &SqliteStore,
    options: &PredictionSnapshotQueryOptions,
) -> anyhow::Result<Vec<PredictionSnapshotRecord>> {
    let market_scope = options
        .market_scope
        .clone()
        .unwrap_or_else(|| "financial_system".to_string());
    let release_id = match options.release_id.clone() {
        Some(release_id) => Some(release_id),
        None => Some(resolve_default_training_release_id(store, &market_scope).await?),
    };
    let snapshots = store
        .list_prediction_snapshots(
            Some(&market_scope),
            release_id.as_deref(),
            options.from,
            options.to,
            options.limit,
        )
        .await?;
    if snapshots.is_empty() {
        bail!("no training snapshots found for market scope {market_scope}");
    }
    Ok(snapshots)
}

async fn resolve_default_training_release_id(
    store: &SqliteStore,
    market_scope: &str,
) -> anyhow::Result<String> {
    if let Some(active_release) = store.load_active_model_release(market_scope).await? {
        if active_release.manifest.probability_mode == "heuristic_mvp" {
            return Ok(active_release.manifest.release_id);
        }
    }

    let heuristic_release = store
        .list_model_releases(Some(market_scope))
        .await?
        .into_iter()
        .find(|release| release.manifest.probability_mode == "heuristic_mvp");

    heuristic_release
        .map(|release| release.manifest.release_id)
        .with_context(|| {
            format!(
                "no heuristic training release found for market scope {market_scope}; pass --release-id explicitly or bootstrap a heuristic release first"
            )
        })
}

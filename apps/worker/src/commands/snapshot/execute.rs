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
    if let Some(release_id) = release_id.as_deref() {
        ensure_snapshot_training_release_supported(store, &market_scope, release_id).await?;
    }
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

async fn ensure_snapshot_training_release_supported(
    store: &SqliteStore,
    market_scope: &str,
    release_id: &str,
) -> anyhow::Result<()> {
    if let Some(release) = store.load_model_release(release_id).await? {
        if release.manifest.probability_mode != "heuristic_mvp" {
            bail!(
                "release {release_id} uses probability_mode={} and is not eligible for snapshot dataset export; snapshot datasets are restricted to heuristic/transitional research snapshots",
                release.manifest.probability_mode
            );
        }
        return Ok(());
    }

    let snapshots = store
        .list_prediction_snapshots(Some(market_scope), Some(release_id), None, None, Some(20))
        .await?;
    if snapshots.is_empty() {
        bail!(
            "release {release_id} has no persisted prediction snapshots in market scope {market_scope}"
        );
    }
    if snapshots
        .iter()
        .any(|snapshot| snapshot.probability_mode != "heuristic_mvp")
    {
        bail!(
            "release {release_id} is not eligible for snapshot dataset export because persisted prediction snapshots are not heuristic_mvp"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, Utc};
    use fc_domain::{ModelReleaseManifest, ModelReleaseRecord, PredictionSnapshotRecord};
    use fc_storage::SqliteStore;

    use super::{load_training_snapshots, PredictionSnapshotQueryOptions};

    async fn in_memory_store() -> SqliteStore {
        let store = SqliteStore::connect_url("sqlite::memory:").await.unwrap();
        store.migrate().await.unwrap();
        store
    }

    fn release(release_id: &str, probability_mode: &str) -> ModelReleaseRecord {
        ModelReleaseRecord {
            manifest: ModelReleaseManifest {
                release_id: release_id.to_string(),
                market_scope: "financial_system".to_string(),
                status: "active".to_string(),
                probability_mode: probability_mode.to_string(),
                serving_status: "healthy".to_string(),
                bundle_uri: "bundle.json".to_string(),
                feature_set_version: "feature_v2".to_string(),
                label_version: "label_v1".to_string(),
                prob_model_version: "prob_v1".to_string(),
                calibration_version: "calib_v1".to_string(),
                posture_policy_version: "posture_v1".to_string(),
                action_playbook_version: "playbook_v1".to_string(),
                point_in_time_mode: "best_effort".to_string(),
                training_range_start: None,
                training_range_end: None,
                calibration_range_start: None,
                calibration_range_end: None,
                evaluation_range_start: None,
                evaluation_range_end: None,
                brier_score: None,
                log_loss: None,
                ece: None,
                note: String::new(),
            },
            created_at: Utc::now(),
            activated_at: None,
            retired_at: None,
        }
    }

    fn snapshot(release_id: &str, probability_mode: &str) -> PredictionSnapshotRecord {
        PredictionSnapshotRecord {
            as_of_date: NaiveDate::from_ymd_opt(2026, 6, 6).unwrap(),
            entity_id: "us".to_string(),
            market_scope: "financial_system".to_string(),
            release_id: Some(release_id.to_string()),
            probability_mode: probability_mode.to_string(),
            release_status: "healthy".to_string(),
            point_in_time_mode: "best_effort".to_string(),
            overall_score: 50.0,
            external_shock_score: 40.0,
            raw_p_5d: 0.1,
            raw_p_20d: 0.2,
            raw_p_60d: 0.3,
            calibrated_p_5d: 0.1,
            calibrated_p_20d: 0.2,
            calibrated_p_60d: 0.3,
            posture: "normal".to_string(),
            time_to_risk_bucket: "normal".to_string(),
            feature_set_version: "feature_v2".to_string(),
            label_version: "label_v1".to_string(),
            coverage_score: 1.0,
            freshness_status: "fresh".to_string(),
            method_version: "method_v1".to_string(),
            posture_trigger_codes: Vec::new(),
            posture_blocker_codes: Vec::new(),
            recorded_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn load_training_snapshots_accepts_heuristic_release() {
        let store = in_memory_store().await;
        store
            .upsert_model_release(&release("heuristic_release", "heuristic_mvp"))
            .await
            .unwrap();
        store
            .upsert_prediction_snapshots(&[snapshot("heuristic_release", "heuristic_mvp")])
            .await
            .unwrap();

        let snapshots = load_training_snapshots(
            &store,
            &PredictionSnapshotQueryOptions {
                market_scope: Some("financial_system".to_string()),
                release_id: Some("heuristic_release".to_string()),
                from: None,
                to: None,
                limit: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].probability_mode, "heuristic_mvp");
    }

    #[tokio::test]
    async fn load_training_snapshots_rejects_formal_release() {
        let store = in_memory_store().await;
        store
            .upsert_model_release(&release("formal_release", "formal_bundle_v1"))
            .await
            .unwrap();
        store
            .upsert_prediction_snapshots(&[snapshot("formal_release", "formal_bundle_v1")])
            .await
            .unwrap();

        let error = load_training_snapshots(
            &store,
            &PredictionSnapshotQueryOptions {
                market_scope: Some("financial_system".to_string()),
                release_id: Some("formal_release".to_string()),
                from: None,
                to: None,
                limit: None,
            },
        )
        .await
        .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("not eligible for snapshot dataset export"),
            "unexpected error: {error:#}"
        );
    }

    #[tokio::test]
    async fn load_training_snapshots_accepts_legacy_heuristic_snapshots_without_release_manifest() {
        let store = in_memory_store().await;
        store
            .upsert_prediction_snapshots(&[snapshot("legacy_heuristic", "heuristic_mvp")])
            .await
            .unwrap();

        let snapshots = load_training_snapshots(
            &store,
            &PredictionSnapshotQueryOptions {
                market_scope: Some("financial_system".to_string()),
                release_id: Some("legacy_heuristic".to_string()),
                from: None,
                to: None,
                limit: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].release_id.as_deref(), Some("legacy_heuristic"));
        assert_eq!(snapshots[0].probability_mode, "heuristic_mvp");
    }

    #[tokio::test]
    async fn load_training_snapshots_rejects_legacy_formal_snapshots_without_release_manifest() {
        let store = in_memory_store().await;
        store
            .upsert_prediction_snapshots(&[snapshot("legacy_formal", "formal_bundle_v1")])
            .await
            .unwrap();

        let error = load_training_snapshots(
            &store,
            &PredictionSnapshotQueryOptions {
                market_scope: Some("financial_system".to_string()),
                release_id: Some("legacy_formal".to_string()),
                from: None,
                to: None,
                limit: None,
            },
        )
        .await
        .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("persisted prediction snapshots are not heuristic_mvp"),
            "unexpected error: {error:#}"
        );
    }
}

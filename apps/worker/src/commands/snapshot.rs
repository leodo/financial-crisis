use std::path::PathBuf;

use anyhow::{bail, Context};
use chrono::NaiveDate;

#[derive(Debug, Clone)]
pub(crate) struct PredictionSnapshotQueryOptions {
    pub(crate) market_scope: Option<String>,
    pub(crate) release_id: Option<String>,
    pub(crate) from: Option<NaiveDate>,
    pub(crate) to: Option<NaiveDate>,
    pub(crate) limit: Option<usize>,
}

impl PredictionSnapshotQueryOptions {
    pub(crate) fn parse(args: &[String]) -> anyhow::Result<Self> {
        Self::parse_with_default_limit(args, Some(20))
    }

    pub(crate) fn parse_with_default_limit(
        args: &[String],
        default_limit: Option<usize>,
    ) -> anyhow::Result<Self> {
        let mut market_scope = None;
        let mut release_id = None;
        let mut from = None;
        let mut to = None;
        let mut limit = default_limit;
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--market-scope" => {
                    index += 1;
                    market_scope = Some(
                        args.get(index)
                            .with_context(|| "--market-scope requires a value")?
                            .clone(),
                    );
                }
                "--release-id" => {
                    index += 1;
                    release_id = Some(
                        args.get(index)
                            .with_context(|| "--release-id requires a value")?
                            .clone(),
                    );
                }
                "--from" => {
                    index += 1;
                    from = Some(crate::parse_date_arg(args.get(index), "--from")?);
                }
                "--to" => {
                    index += 1;
                    to = Some(crate::parse_date_arg(args.get(index), "--to")?);
                }
                "--limit" => {
                    index += 1;
                    limit = Some(
                        args.get(index)
                            .with_context(|| "--limit requires a number")?
                            .parse::<usize>()
                            .context("--limit must be an integer")?,
                    );
                }
                other => bail!("unknown prediction snapshot query option: {other}"),
            }
            index += 1;
        }
        Ok(Self {
            market_scope,
            release_id,
            from,
            to,
            limit,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ExportFormat {
    Json,
    Csv,
}

impl ExportFormat {
    pub(crate) fn parse(raw: &str) -> anyhow::Result<Self> {
        match raw {
            "json" => Ok(Self::Json),
            "csv" => Ok(Self::Csv),
            other => bail!("unsupported format: {other}"),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PredictionSnapshotExportOptions {
    pub(crate) query: PredictionSnapshotQueryOptions,
    pub(crate) format: ExportFormat,
    pub(crate) output_path: Option<PathBuf>,
}

impl PredictionSnapshotExportOptions {
    pub(crate) fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut output_path = None;
        let mut format = ExportFormat::Json;
        let mut query_args = Vec::new();
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--output-path" => {
                    index += 1;
                    output_path = Some(PathBuf::from(
                        args.get(index)
                            .with_context(|| "--output-path requires a path")?,
                    ));
                }
                "--format" => {
                    index += 1;
                    format = ExportFormat::parse(
                        args.get(index)
                            .with_context(|| "--format requires json or csv")?,
                    )?;
                }
                other => query_args.push(other.to_string()),
            }
            index += 1;
        }

        Ok(Self {
            query: PredictionSnapshotQueryOptions::parse(&query_args)?,
            format,
            output_path,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SnapshotDatasetExportOptions {
    pub(crate) query: PredictionSnapshotQueryOptions,
    pub(crate) format: ExportFormat,
    pub(crate) output_path: Option<PathBuf>,
}

impl SnapshotDatasetExportOptions {
    pub(crate) fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut output_path = None;
        let mut format = ExportFormat::Json;
        let mut query_args = Vec::new();
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--output-path" => {
                    index += 1;
                    output_path = Some(PathBuf::from(
                        args.get(index)
                            .with_context(|| "--output-path requires a path")?,
                    ));
                }
                "--format" => {
                    index += 1;
                    format = ExportFormat::parse(
                        args.get(index)
                            .with_context(|| "--format requires json or csv")?,
                    )?;
                }
                other => query_args.push(other.to_string()),
            }
            index += 1;
        }

        Ok(Self {
            query: PredictionSnapshotQueryOptions::parse_with_default_limit(&query_args, None)?,
            format,
            output_path,
        })
    }
}

pub(crate) async fn research_prediction_snapshot_list(args: &[String]) -> anyhow::Result<()> {
    let options = PredictionSnapshotQueryOptions::parse(args)?;
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;
    let snapshots = crate::load_prediction_snapshots(&store, &options).await?;
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
    let snapshots = crate::load_prediction_snapshots(&store, &options.query).await?;
    crate::write_snapshot_export(&snapshots, options.format, options.output_path.as_deref())?;
    Ok(())
}

pub(crate) async fn research_prediction_snapshot_dataset(args: &[String]) -> anyhow::Result<()> {
    let options = SnapshotDatasetExportOptions::parse(args)?;
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;
    let snapshots = crate::load_training_snapshots(&store, &options.query).await?;
    let dataset = crate::build_pipeline_dataset_rows(&snapshots);
    crate::write_dataset_export(
        &dataset,
        &crate::transitional_feature_names(),
        options.format,
        options.output_path.as_deref(),
    )?;
    Ok(())
}

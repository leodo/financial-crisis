use std::{
    fmt::Write,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use chrono::NaiveDate;
use fc_domain::PredictionSnapshotRecord;
use fc_storage::SqliteStore;

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
    let dataset = super::pipeline::build_pipeline_dataset_rows(&snapshots);
    write_dataset_export(
        &dataset,
        &super::pipeline::transitional_feature_names(),
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

pub(crate) fn write_snapshot_export(
    snapshots: &[PredictionSnapshotRecord],
    format: ExportFormat,
    output_path: Option<&Path>,
) -> anyhow::Result<()> {
    let content = match format {
        ExportFormat::Json => serde_json::to_string_pretty(snapshots)?,
        ExportFormat::Csv => render_snapshot_csv(snapshots),
    };
    write_or_print_export(content, output_path)
}

pub(crate) fn write_dataset_export(
    dataset: &[crate::ProbabilityTrainingRow],
    feature_names: &[String],
    format: ExportFormat,
    output_path: Option<&Path>,
) -> anyhow::Result<()> {
    let content = match format {
        ExportFormat::Json => serde_json::to_string_pretty(dataset)?,
        ExportFormat::Csv => render_dataset_csv(dataset, feature_names),
    };
    write_or_print_export(content, output_path)
}

fn write_or_print_export(content: String, output_path: Option<&Path>) -> anyhow::Result<()> {
    if let Some(path) = output_path {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(path, content)?;
        println!("Exported {}", path.display());
    } else {
        println!("{content}");
    }
    Ok(())
}

fn render_snapshot_csv(snapshots: &[PredictionSnapshotRecord]) -> String {
    let mut csv = String::from(
        "as_of_date,market_scope,release_id,probability_mode,release_status,point_in_time_mode,overall_score,external_shock_score,raw_p_5d,raw_p_20d,raw_p_60d,calibrated_p_5d,calibrated_p_20d,calibrated_p_60d,posture,time_to_risk_bucket,coverage_score,freshness_status,method_version,posture_trigger_codes,posture_blocker_codes,recorded_at\n",
    );
    for snapshot in snapshots {
        let _ = writeln!(
            csv,
            "{},{},{},{},{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{},{},{:.6},{},{},{},{},{}",
            snapshot.as_of_date,
            snapshot.market_scope,
            snapshot.release_id.as_deref().unwrap_or(""),
            snapshot.probability_mode,
            snapshot.release_status,
            snapshot.point_in_time_mode,
            snapshot.overall_score,
            snapshot.external_shock_score,
            snapshot.raw_p_5d,
            snapshot.raw_p_20d,
            snapshot.raw_p_60d,
            snapshot.calibrated_p_5d,
            snapshot.calibrated_p_20d,
            snapshot.calibrated_p_60d,
            snapshot.posture,
            snapshot.time_to_risk_bucket,
            snapshot.coverage_score,
            snapshot.freshness_status,
            snapshot.method_version,
            snapshot.posture_trigger_codes.join("|"),
            snapshot.posture_blocker_codes.join("|"),
            snapshot.recorded_at.to_rfc3339()
        );
    }
    csv
}

pub(crate) fn render_dataset_csv(
    dataset: &[crate::ProbabilityTrainingRow],
    feature_names: &[String],
) -> String {
    let mut header = String::from(
        "as_of_date,market_scope,release_id,probability_mode,freshness_status,time_to_risk_bucket,split_name,primary_scenario_id,scenario_family,scenario_training_role,label_5d,label_20d,label_60d,action_label_5d,action_label_20d,action_label_60d,prepare_episode_label,hedge_episode_label,defend_episode_label,primary_action_level,action_episode_id,action_episode_phase,protected_action_window",
    );
    for feature in feature_names {
        header.push(',');
        header.push_str(feature);
    }
    header.push('\n');

    let mut csv = header;
    for row in dataset {
        let columns = [
            row.as_of_date.to_string(),
            row.market_scope.clone(),
            row.release_id.clone().unwrap_or_default(),
            row.probability_mode.clone().unwrap_or_default(),
            row.freshness_status.clone().unwrap_or_default(),
            row.time_to_risk_bucket.clone().unwrap_or_default(),
            row.split_name.clone().unwrap_or_default(),
            row.primary_scenario_id.clone().unwrap_or_default(),
            row.scenario_family.clone().unwrap_or_default(),
            row.scenario_training_role.clone().unwrap_or_default(),
            row.label_5d.to_string(),
            row.label_20d.to_string(),
            row.label_60d.to_string(),
            row.action_label_5d.to_string(),
            row.action_label_20d.to_string(),
            row.action_label_60d.to_string(),
            row.prepare_episode_label.to_string(),
            row.hedge_episode_label.to_string(),
            row.defend_episode_label.to_string(),
            row.primary_action_level.clone().unwrap_or_default(),
            row.action_episode_id.clone().unwrap_or_default(),
            row.action_episode_phase.clone(),
            (row.protected_action_window as u8).to_string(),
        ];
        csv.push_str(&columns.join(","));
        for feature in feature_names {
            let value = row.features.get(feature).copied().unwrap_or_default();
            let _ = write!(csv, ",{value:.6}");
        }
        csv.push('\n');
    }
    csv
}

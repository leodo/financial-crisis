use std::path::PathBuf;

use anyhow::{bail, Context};

use super::snapshot::PredictionSnapshotQueryOptions;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PipelineDatasetSource {
    Formal,
    Snapshot,
}

impl PipelineDatasetSource {
    pub(crate) fn parse(value: &str) -> anyhow::Result<Self> {
        match value {
            "formal" => Ok(Self::Formal),
            "snapshot" => Ok(Self::Snapshot),
            other => bail!("unsupported --dataset-source value: {other}"),
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Formal => "formal",
            Self::Snapshot => "snapshot",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProbabilityModelShape {
    LinearV1,
    InteractionTailV1,
}

impl ProbabilityModelShape {
    pub(crate) fn parse(value: &str) -> anyhow::Result<Self> {
        match value {
            "linear_v1" => Ok(Self::LinearV1),
            "interaction_tail_v1" => Ok(Self::InteractionTailV1),
            other => bail!("unsupported --model-shape value: {other}"),
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::LinearV1 => crate::PROBABILITY_MODEL_FAMILY_LINEAR_V1,
            Self::InteractionTailV1 => crate::PROBABILITY_MODEL_FAMILY_INTERACTION_TAIL_V1,
        }
    }

    pub(crate) fn feature_transform(self) -> &'static str {
        match self {
            Self::LinearV1 => crate::PROBABILITY_FEATURE_TRANSFORM_IDENTITY_V1,
            Self::InteractionTailV1 => crate::PROBABILITY_FEATURE_TRANSFORM_INTERACTION_TAIL_V1,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PipelineTrainOptions {
    pub(crate) dataset_source: PipelineDatasetSource,
    pub(crate) model_shape: ProbabilityModelShape,
    pub(crate) dataset_id: String,
    pub(crate) dataset_version: Option<String>,
    pub(crate) dataset_key: Option<String>,
    pub(crate) aux_dataset_keys: Vec<String>,
    pub(crate) query: PredictionSnapshotQueryOptions,
    pub(crate) output_dir: PathBuf,
    pub(crate) manifest_dir: PathBuf,
    pub(crate) release_prefix: String,
}

impl PipelineTrainOptions {
    pub(crate) fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut output_dir = PathBuf::from(crate::DEFAULT_PIPELINE_BUNDLE_OUTPUT_DIR);
        let mut manifest_dir = PathBuf::from(crate::DEFAULT_PIPELINE_MANIFEST_OUTPUT_DIR);
        let mut release_prefix = None;
        let mut dataset_source = PipelineDatasetSource::Formal;
        let mut model_shape = ProbabilityModelShape::LinearV1;
        let mut dataset_id = crate::DEFAULT_FORMAL_DATASET_ID.to_string();
        let mut dataset_version = None;
        let mut dataset_key = None;
        let mut aux_dataset_keys = Vec::new();
        let mut query_args = Vec::new();
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--dataset-source" => {
                    index += 1;
                    dataset_source = PipelineDatasetSource::parse(
                        args.get(index)
                            .with_context(|| "--dataset-source requires a value")?,
                    )?;
                }
                "--model-shape" => {
                    index += 1;
                    model_shape = ProbabilityModelShape::parse(
                        args.get(index)
                            .with_context(|| "--model-shape requires a value")?,
                    )?;
                }
                "--dataset-id" => {
                    index += 1;
                    dataset_id = args
                        .get(index)
                        .with_context(|| "--dataset-id requires a value")?
                        .clone();
                }
                "--dataset-version" => {
                    index += 1;
                    dataset_version = Some(
                        args.get(index)
                            .with_context(|| "--dataset-version requires a value")?
                            .clone(),
                    );
                }
                "--dataset-key" => {
                    index += 1;
                    dataset_key = Some(
                        args.get(index)
                            .with_context(|| "--dataset-key requires a value")?
                            .clone(),
                    );
                }
                "--aux-dataset-key" => {
                    index += 1;
                    aux_dataset_keys.push(
                        args.get(index)
                            .with_context(|| "--aux-dataset-key requires a value")?
                            .clone(),
                    );
                }
                "--output-dir" => {
                    index += 1;
                    output_dir = PathBuf::from(
                        args.get(index)
                            .with_context(|| "--output-dir requires a path")?,
                    );
                }
                "--manifest-dir" => {
                    index += 1;
                    manifest_dir = PathBuf::from(
                        args.get(index)
                            .with_context(|| "--manifest-dir requires a path")?,
                    );
                }
                "--release-prefix" => {
                    index += 1;
                    release_prefix = Some(
                        args.get(index)
                            .with_context(|| "--release-prefix requires a value")?
                            .clone(),
                    );
                }
                other => query_args.push(other.to_string()),
            }
            index += 1;
        }

        let release_prefix =
            release_prefix.unwrap_or_else(|| match (dataset_source, model_shape) {
                (PipelineDatasetSource::Formal, ProbabilityModelShape::LinearV1) => {
                    "us_formal_main".to_string()
                }
                (PipelineDatasetSource::Formal, ProbabilityModelShape::InteractionTailV1) => {
                    "us_formal_interaction_tail".to_string()
                }
                (PipelineDatasetSource::Snapshot, ProbabilityModelShape::LinearV1) => {
                    "us_formal_transitional".to_string()
                }
                (PipelineDatasetSource::Snapshot, ProbabilityModelShape::InteractionTailV1) => {
                    "us_formal_transitional_interaction_tail".to_string()
                }
            });

        Ok(Self {
            dataset_source,
            model_shape,
            dataset_id,
            dataset_version,
            dataset_key,
            aux_dataset_keys,
            query: PredictionSnapshotQueryOptions::parse_with_default_limit(&query_args, None)?,
            output_dir,
            manifest_dir,
            release_prefix,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PipelineBootstrapOptions {
    pub(crate) train: PipelineTrainOptions,
    pub(crate) activate: bool,
    pub(crate) reload_api: bool,
    pub(crate) api_reload_url: String,
    pub(crate) skip_operational_guard: bool,
    pub(crate) updated_by: String,
}

impl PipelineBootstrapOptions {
    pub(crate) fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut activate = true;
        let mut reload_api = true;
        let mut api_reload_url = crate::DEFAULT_API_RELOAD_URL.to_string();
        let mut skip_operational_guard = false;
        let mut updated_by = "fc-worker".to_string();
        let mut train_args = Vec::new();
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--no-activate" => activate = false,
                "--no-reload-api" => reload_api = false,
                "--skip-operational-guard" => skip_operational_guard = true,
                "--api-reload-url" => {
                    index += 1;
                    api_reload_url = args
                        .get(index)
                        .with_context(|| "--api-reload-url requires a URL")?
                        .clone();
                }
                "--updated-by" => {
                    index += 1;
                    updated_by = args
                        .get(index)
                        .with_context(|| "--updated-by requires a value")?
                        .clone();
                }
                other => train_args.push(other.to_string()),
            }
            index += 1;
        }

        Ok(Self {
            train: PipelineTrainOptions::parse(&train_args)?,
            activate,
            reload_api,
            api_reload_url,
            skip_operational_guard,
            updated_by,
        })
    }
}

pub(crate) async fn research_pipeline_train_probability(args: &[String]) -> anyhow::Result<()> {
    let options = PipelineTrainOptions::parse(args)?;
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;
    let artifacts = crate::train_probability_pipeline(&store, &options).await?;
    println!("Formal probability bundle generated.");
    println!("  dataset_source   {}", artifacts.dataset_source);
    println!("  dataset_label    {}", artifacts.dataset_label);
    println!("  model_shape      {}", options.model_shape.as_str());
    println!(
        "  release_id       {}",
        artifacts.release.manifest.release_id
    );
    println!("  bundle_path      {}", artifacts.bundle_path.display());
    println!("  manifest_path    {}", artifacts.manifest_path.display());
    println!("  evaluation_path  {}", artifacts.evaluation_path.display());
    if let Some(summary) = artifacts.bundle.evaluation.as_ref() {
        println!(
            "  eval             brier={:.4} log_loss={:.4} ece={:.4}",
            summary.brier_score, summary.log_loss, summary.ece
        );
        println!(
            "  regime_eval      usable_early_warning_horizons={} insufficient_early_warning_horizons={}",
            summary.usable_early_warning_horizon_count,
            summary.insufficient_early_warning_horizon_count,
        );
        for regime in &summary.regime_separation_summaries {
            println!(
                "  regime_horizon   {:>2}d early={} positive_window={} cooldown={} in_crisis={} diagnosis={}",
                regime.horizon_days,
                regime.early_warning_regime,
                crate::format_optional_multiplier(regime.positive_window_lift_vs_normal),
                crate::format_optional_multiplier(regime.post_crisis_cooldown_lift_vs_normal),
                crate::format_optional_multiplier(regime.in_crisis_lift_vs_normal),
                regime.diagnosis,
            );
        }
    }
    for horizon in &artifacts.bundle.horizons {
        if let Some(diag) = horizon.threshold_diagnostics.as_ref() {
            println!(
                "  threshold_diag   {:>2}d base={:.3} final={:.3} repair={} reason={} selected_rows={} early_rows={}",
                horizon.horizon_days,
                diag.base_threshold,
                diag.final_threshold,
                diag.repair_applied,
                diag.repair_reason,
                diag.selected_row_count,
                diag.base_summary.early_warning_row_count,
            );
            println!(
                "                   base_hits early={}/{} normal={}/{} final_hits early={}/{} normal={}/{}",
                diag.base_summary.early_warning_hit_count,
                diag.base_summary.early_warning_row_count,
                diag.base_summary.normal_hit_count,
                diag.base_summary.normal_row_count,
                diag.final_summary.early_warning_hit_count,
                diag.final_summary.early_warning_row_count,
                diag.final_summary.normal_hit_count,
                diag.final_summary.normal_row_count,
            );
        }
    }
    if let Some(actionability) = artifacts.bundle.actionability.as_ref() {
        for level in &actionability.levels {
            if let Some(summary) = level.evaluation.actionability.as_ref() {
                println!(
                    "  actionability    {:>7} scenarios={} on_time={} late_only={} missed={}",
                    crate::actionability_level_text(level.level),
                    summary.scenario_count,
                    crate::format_pct(summary.advance_warning_rate.unwrap_or(0.0)),
                    crate::format_pct(summary.late_confirmation_rate.unwrap_or(0.0)),
                    crate::format_pct(summary.missed_rate.unwrap_or(0.0)),
                );
            }
        }
    }
    Ok(())
}

pub(crate) async fn research_pipeline_bootstrap_formal_release(
    args: &[String],
) -> anyhow::Result<()> {
    let options = PipelineBootstrapOptions::parse(args)?;
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;
    let artifacts = crate::train_probability_pipeline(&store, &options.train).await?;
    store.upsert_model_release(&artifacts.release).await?;
    println!(
        "Published formal release {}.",
        artifacts.release.manifest.release_id
    );
    println!("  manifest {}", artifacts.manifest_path.display());
    println!("  bundle   {}", artifacts.bundle_path.display());

    if options.activate {
        super::release::activate_release_with_runtime_guard(
            &store,
            &artifacts.release.manifest.market_scope,
            &artifacts.release.manifest.release_id,
            options.reload_api,
            &options.api_reload_url,
            options.skip_operational_guard,
            &options.updated_by,
        )
        .await?;
    }

    Ok(())
}

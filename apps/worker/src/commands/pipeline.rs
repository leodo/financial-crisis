use std::{collections::BTreeMap, path::PathBuf};

use anyhow::{bail, Context};
use fc_domain::{FormalDatasetRowRecord, PredictionSnapshotRecord};
use fc_storage::SqliteStore;

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
    FamilyConditionalV1,
    FamilyHybridV1,
}

impl ProbabilityModelShape {
    pub(crate) fn parse(value: &str) -> anyhow::Result<Self> {
        match value {
            "linear_v1" => Ok(Self::LinearV1),
            "interaction_tail_v1" => Ok(Self::InteractionTailV1),
            "family_conditional_v1" => Ok(Self::FamilyConditionalV1),
            "family_hybrid_v1" => Ok(Self::FamilyHybridV1),
            other => bail!("unsupported --model-shape value: {other}"),
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::LinearV1 => crate::PROBABILITY_MODEL_FAMILY_LINEAR_V1,
            Self::InteractionTailV1 => crate::PROBABILITY_MODEL_FAMILY_INTERACTION_TAIL_V1,
            Self::FamilyConditionalV1 => crate::PROBABILITY_MODEL_FAMILY_FAMILY_CONDITIONAL_V1,
            Self::FamilyHybridV1 => crate::PROBABILITY_MODEL_FAMILY_FAMILY_HYBRID_V1,
        }
    }

    pub(crate) fn feature_transform(self) -> &'static str {
        match self {
            Self::LinearV1 => crate::PROBABILITY_FEATURE_TRANSFORM_IDENTITY_V1,
            Self::InteractionTailV1 => crate::PROBABILITY_FEATURE_TRANSFORM_INTERACTION_TAIL_V1,
            Self::FamilyConditionalV1 => crate::PROBABILITY_FEATURE_TRANSFORM_FAMILY_CONDITIONAL_V1,
            Self::FamilyHybridV1 => crate::PROBABILITY_FEATURE_TRANSFORM_FAMILY_HYBRID_V1,
        }
    }

    pub(crate) fn base_feature_transform_for_horizon(self, horizon_days: u32) -> &'static str {
        match self {
            Self::FamilyHybridV1 if horizon_days == 60 => {
                crate::PROBABILITY_FEATURE_TRANSFORM_INTERACTION_TAIL_V1
            }
            Self::FamilyHybridV1 => crate::PROBABILITY_FEATURE_TRANSFORM_FAMILY_CONDITIONAL_V1,
            _ => self.feature_transform(),
        }
    }

    pub(crate) fn overlay_feature_transform_for_horizon(self, horizon_days: u32) -> &'static str {
        match self {
            Self::FamilyHybridV1 if horizon_days == 60 => {
                crate::PROBABILITY_FEATURE_TRANSFORM_INTERACTION_TAIL_V1
            }
            Self::FamilyHybridV1 => crate::PROBABILITY_FEATURE_TRANSFORM_FAMILY_CONDITIONAL_V1,
            _ => self.feature_transform(),
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
                (PipelineDatasetSource::Formal, ProbabilityModelShape::FamilyConditionalV1) => {
                    "us_formal_family_conditional".to_string()
                }
                (PipelineDatasetSource::Formal, ProbabilityModelShape::FamilyHybridV1) => {
                    "us_formal_family_hybrid".to_string()
                }
                (PipelineDatasetSource::Snapshot, ProbabilityModelShape::LinearV1) => {
                    "us_formal_transitional".to_string()
                }
                (PipelineDatasetSource::Snapshot, ProbabilityModelShape::InteractionTailV1) => {
                    "us_formal_transitional_interaction_tail".to_string()
                }
                (PipelineDatasetSource::Snapshot, ProbabilityModelShape::FamilyConditionalV1) => {
                    "us_formal_transitional_family_conditional".to_string()
                }
                (PipelineDatasetSource::Snapshot, ProbabilityModelShape::FamilyHybridV1) => {
                    "us_formal_transitional_family_hybrid".to_string()
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
            if let Some(early_evidence) = diag
                .calibration_regime_evidence
                .iter()
                .find(|row| row.regime == diag.early_warning_regime)
            {
                println!(
                    "                   calib_evidence early={} rows={}({}) eligible={} used={} selected={} hard={} soft={} weight={}",
                    early_evidence.regime,
                    early_evidence.full_row_count,
                    crate::format_pct(early_evidence.full_row_rate),
                    early_evidence.calibration_eligible_row_count,
                    early_evidence.calibration_used_row_count,
                    early_evidence.threshold_selected_row_count,
                    crate::format_pct(early_evidence.avg_hard_label),
                    crate::format_pct(early_evidence.avg_training_target),
                    early_evidence.avg_objective_weight,
                );
            }
        }
        let configured_overlay_ids = horizon
            .family_overlays
            .iter()
            .map(|overlay| overlay.family_id.as_str())
            .collect::<Vec<_>>();
        println!(
            "  overlay_diag     {:>2}d configured={} audits={} families={}",
            horizon.horizon_days,
            horizon.family_overlays.len(),
            horizon.family_overlay_audits.len(),
            if configured_overlay_ids.is_empty() {
                "none".to_string()
            } else {
                configured_overlay_ids.join(",")
            }
        );
        for audit in &horizon.family_overlay_audits {
            println!(
                "                   audit {} scenarios={} positives={} rows={}/{}/{} gate_active={}/{}/{} note={}",
                audit.family_id,
                audit.scenario_count,
                audit.positive_label_count,
                audit.train_row_count,
                audit.calibration_row_count,
                audit.evaluation_row_count,
                audit.train_gate_active_row_count,
                audit.calibration_gate_active_row_count,
                audit.evaluation_gate_active_row_count,
                audit.note,
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

pub(crate) fn transitional_feature_names() -> Vec<String> {
    crate::TRANSITIONAL_PROBABILITY_BUNDLE_FEATURES
        .iter()
        .map(|feature| (*feature).to_string())
        .collect()
}

fn formal_feature_names() -> Vec<String> {
    crate::FORMAL_PROBABILITY_BUNDLE_FEATURES
        .iter()
        .map(|feature| (*feature).to_string())
        .collect()
}

pub(crate) async fn resolve_formal_dataset_key(
    store: &SqliteStore,
    dataset_key: Option<&str>,
    dataset_id: &str,
    dataset_version: Option<&str>,
    market_scope: Option<&str>,
) -> anyhow::Result<String> {
    if let Some(dataset_key) = dataset_key {
        return Ok(dataset_key.to_string());
    }
    if let Some(dataset_version) = dataset_version {
        return Ok(crate::formal_dataset_key(dataset_id, dataset_version));
    }

    let market_scope = market_scope.unwrap_or("financial_system");
    let latest = store
        .list_formal_datasets(Some(market_scope), Some(dataset_id), Some(1))
        .await?
        .into_iter()
        .next()
        .with_context(|| {
            format!(
                "no persisted formal dataset found for market scope {market_scope} and dataset id {dataset_id}"
            )
        })?;
    Ok(crate::formal_dataset_key(
        &latest.manifest.dataset_id,
        &latest.manifest.dataset_version,
    ))
}

async fn resolve_formal_training_dataset_key(
    store: &SqliteStore,
    options: &PipelineTrainOptions,
) -> anyhow::Result<String> {
    resolve_formal_dataset_key(
        store,
        options.dataset_key.as_deref(),
        &options.dataset_id,
        options.dataset_version.as_deref(),
        options.query.market_scope.as_deref(),
    )
    .await
}

async fn load_formal_training_dataset(
    store: &SqliteStore,
    options: &PipelineTrainOptions,
) -> anyhow::Result<crate::ProbabilityTrainingInput> {
    let primary_dataset_key = resolve_formal_training_dataset_key(store, options).await?;
    let mut dataset_keys = vec![primary_dataset_key.clone()];
    for dataset_key in &options.aux_dataset_keys {
        if !dataset_keys.contains(dataset_key) {
            dataset_keys.push(dataset_key.clone());
        }
    }

    let primary_dataset = store
        .load_formal_dataset(&primary_dataset_key)
        .await?
        .with_context(|| format!("formal dataset {primary_dataset_key} was not found in SQLite"))?;

    let mut combined_rows = Vec::<FormalDatasetRowRecord>::new();
    let mut positive_by_id = BTreeMap::<String, crate::CrisisScenario>::new();
    let mut context_by_id = BTreeMap::<String, crate::CrisisScenario>::new();

    for dataset_key in &dataset_keys {
        let dataset = store
            .load_formal_dataset(dataset_key)
            .await?
            .with_context(|| format!("formal dataset {dataset_key} was not found in SQLite"))?;
        if dataset.manifest.market_scope != primary_dataset.manifest.market_scope {
            bail!(
                "auxiliary formal dataset {dataset_key} has market scope {} but primary dataset {} uses {}; mixed-market training is not supported",
                dataset.manifest.market_scope,
                primary_dataset_key,
                primary_dataset.manifest.market_scope
            );
        }
        if dataset.manifest.point_in_time_mode != primary_dataset.manifest.point_in_time_mode {
            bail!(
                "auxiliary formal dataset {dataset_key} has point_in_time_mode {} but primary dataset {} uses {}; mixed PIT modes are not supported",
                dataset.manifest.point_in_time_mode,
                primary_dataset_key,
                primary_dataset.manifest.point_in_time_mode
            );
        }
        if dataset.manifest.feature_set_version != primary_dataset.manifest.feature_set_version {
            bail!(
                "auxiliary formal dataset {dataset_key} has feature_set_version {} but primary dataset {} uses {}; mixed feature sets are not supported",
                dataset.manifest.feature_set_version,
                primary_dataset_key,
                primary_dataset.manifest.feature_set_version
            );
        }

        let mut rows = store
            .list_formal_dataset_rows(dataset_key, None, None)
            .await?;
        if let Some(from) = options.query.from {
            rows.retain(|row| row.as_of_date >= from);
        }
        if let Some(to) = options.query.to {
            rows.retain(|row| row.as_of_date <= to);
        }
        if rows.is_empty() {
            bail!(
                "formal dataset {dataset_key} has no rows after the requested date filters; widen --from/--to or choose a different auxiliary dataset"
            );
        }
        combined_rows.extend(rows);

        let scenario_sets = crate::load_formal_dataset_scenario_sets(
            &dataset.manifest.scenario_set_version,
            &dataset.manifest.label_version,
        )?;
        for scenario in scenario_sets.positive_scenarios {
            positive_by_id.insert(scenario.scenario_id.clone(), scenario);
        }
        for scenario in scenario_sets.context_scenarios {
            context_by_id.insert(scenario.scenario_id.clone(), scenario);
        }
    }

    if combined_rows.len() < 90 {
        bail!(
            "formal dataset {} is too small after filters: {} rows found across {} dataset(s), at least 90 are required; backfill more free historical observations and rebuild the formal dataset, or use --dataset-source snapshot as a temporary fallback",
            primary_dataset_key,
            combined_rows.len(),
            dataset_keys.len()
        );
    }

    let positive_scenarios = positive_by_id.into_values().collect::<Vec<_>>();
    let context_scenarios = context_by_id.into_values().collect::<Vec<_>>();
    let scenario_by_id = context_scenarios
        .iter()
        .cloned()
        .map(|scenario| (scenario.scenario_id.clone(), scenario))
        .collect::<BTreeMap<_, _>>();

    let to_training_row = |row: &FormalDatasetRowRecord| {
        let primary_scenario = row
            .primary_scenario_id
            .as_ref()
            .and_then(|scenario_id| scenario_by_id.get(scenario_id));
        crate::ProbabilityTrainingRow {
            as_of_date: row.as_of_date,
            market_scope: row.market_scope.clone(),
            release_id: None,
            probability_mode: Some("formal_bundle_v1".to_string()),
            freshness_status: Some(row.sample_quality_grade.clone()),
            time_to_risk_bucket: row.primary_scenario_id.clone(),
            split_name: Some(row.split_name.clone()),
            features: row.features.clone(),
            primary_scenario_id: row.primary_scenario_id.clone(),
            scenario_family: row.scenario_family.clone(),
            scenario_training_role: row
                .scenario_training_role
                .clone()
                .or_else(|| primary_scenario.map(|scenario| scenario.training_role.clone())),
            days_to_primary_crisis_start: primary_scenario
                .map(|scenario| (scenario.crisis_start - row.as_of_date).num_days()),
            primary_scenario_supports_5d: primary_scenario
                .is_some_and(|scenario| crate::scenario_supports_horizon(scenario, 5)),
            primary_scenario_supports_20d: primary_scenario
                .is_some_and(|scenario| crate::scenario_supports_horizon(scenario, 20)),
            primary_scenario_supports_60d: primary_scenario
                .is_some_and(|scenario| crate::scenario_supports_horizon(scenario, 60)),
            label_5d: row.label_5d,
            label_20d: row.label_20d,
            label_60d: row.label_60d,
            regime_5d: crate::forward_crisis_training_regime_with_context(
                row.as_of_date,
                &positive_scenarios,
                &context_scenarios,
                5,
            ),
            regime_20d: crate::forward_crisis_training_regime_with_context(
                row.as_of_date,
                &positive_scenarios,
                &context_scenarios,
                20,
            ),
            regime_60d: crate::forward_crisis_training_regime_with_context(
                row.as_of_date,
                &positive_scenarios,
                &context_scenarios,
                60,
            ),
            action_label_5d: row.action_label_5d,
            action_label_20d: row.action_label_20d,
            action_label_60d: row.action_label_60d,
            prepare_episode_label: row.prepare_episode_label,
            hedge_episode_label: row.hedge_episode_label,
            defend_episode_label: row.defend_episode_label,
            primary_action_level: row.primary_action_level.clone(),
            action_episode_id: row.action_episode_id.clone(),
            action_episode_phase: row.action_episode_phase.clone(),
            protected_action_window: row.protected_action_window,
        }
    };

    let mut train_rows = combined_rows
        .iter()
        .filter(|row| row.split_name == "train")
        .map(to_training_row)
        .collect::<Vec<_>>();
    let mut calibration_rows = combined_rows
        .iter()
        .filter(|row| row.split_name == "calibration")
        .map(to_training_row)
        .collect::<Vec<_>>();
    let mut evaluation_rows = combined_rows
        .iter()
        .filter(|row| row.split_name == "evaluation")
        .map(to_training_row)
        .collect::<Vec<_>>();

    train_rows.sort_by_key(|row| row.as_of_date);
    calibration_rows.sort_by_key(|row| row.as_of_date);
    evaluation_rows.sort_by_key(|row| row.as_of_date);

    if train_rows.is_empty() || calibration_rows.is_empty() || evaluation_rows.is_empty() {
        bail!(
            "formal dataset {} is missing one or more required splits after filters (train={}, calibration={}, evaluation={}); rebuild it from a broader historical range before training the formal bundle",
            primary_dataset_key,
            train_rows.len(),
            calibration_rows.len(),
            evaluation_rows.len()
        );
    }

    let dataset_label = if dataset_keys.len() == 1 {
        primary_dataset_key.clone()
    } else {
        format!(
            "{} + aux({})",
            primary_dataset_key,
            dataset_keys[1..].join(", ")
        )
    };

    Ok(crate::ProbabilityTrainingInput {
        dataset_source: PipelineDatasetSource::Formal,
        dataset_label,
        market_scope: primary_dataset.manifest.market_scope.clone(),
        point_in_time_mode: primary_dataset.manifest.point_in_time_mode.clone(),
        feature_set_version: primary_dataset.manifest.feature_set_version.clone(),
        label_version: primary_dataset.manifest.label_version.clone(),
        feature_names: formal_feature_names(),
        train_rows,
        calibration_rows,
        evaluation_rows,
    })
}

async fn load_snapshot_training_dataset(
    store: &SqliteStore,
    options: &PipelineTrainOptions,
) -> anyhow::Result<crate::ProbabilityTrainingInput> {
    let snapshots = super::snapshot::load_training_snapshots(store, &options.query).await?;
    let dataset = build_pipeline_dataset_rows(&snapshots);
    if dataset.len() < 90 {
        bail!(
            "training dataset is too small: {} rows found, at least 90 are required",
            dataset.len()
        );
    }

    let (train_rows, calibration_rows, evaluation_rows) = crate::chronological_split(&dataset)?;
    let market_scope = train_rows
        .first()
        .map(|row| row.market_scope.clone())
        .unwrap_or_else(|| "financial_system".to_string());
    let dataset_label = train_rows
        .first()
        .and_then(|row| row.release_id.clone())
        .unwrap_or_else(|| "heuristic_prediction_snapshots".to_string());

    Ok(crate::ProbabilityTrainingInput {
        dataset_source: PipelineDatasetSource::Snapshot,
        dataset_label,
        market_scope,
        point_in_time_mode: "best_effort".to_string(),
        feature_set_version: "feature_prob_meta_v1".to_string(),
        label_version: "label_forward_crisis_v1".to_string(),
        feature_names: transitional_feature_names(),
        train_rows,
        calibration_rows,
        evaluation_rows,
    })
}

pub(crate) async fn load_probability_training_input(
    store: &SqliteStore,
    options: &PipelineTrainOptions,
) -> anyhow::Result<crate::ProbabilityTrainingInput> {
    match options.dataset_source {
        PipelineDatasetSource::Formal => load_formal_training_dataset(store, options).await,
        PipelineDatasetSource::Snapshot => load_snapshot_training_dataset(store, options).await,
    }
}

pub(crate) fn build_pipeline_dataset_rows(
    snapshots: &[PredictionSnapshotRecord],
) -> Vec<crate::ProbabilityTrainingRow> {
    let scenario_sets = crate::load_formal_dataset_scenario_sets(
        crate::DEFAULT_FORMAL_SCENARIO_SET_VERSION,
        crate::DEFAULT_FORMAL_LABEL_VERSION,
    )
    .expect("default scenario catalog must contain the main training label set");
    let positive_scenarios = scenario_sets.positive_scenarios;
    let context_scenarios = scenario_sets.context_scenarios;
    let mut rows = snapshots
        .iter()
        .map(|snapshot| {
            let features = pipeline_features_from_snapshot(snapshot);
            let scenario_labels = crate::derive_scenario_label_snapshot(
                snapshot.as_of_date,
                &positive_scenarios,
                &context_scenarios,
            );
            crate::ProbabilityTrainingRow {
                as_of_date: snapshot.as_of_date,
                market_scope: snapshot.market_scope.clone(),
                release_id: snapshot.release_id.clone(),
                probability_mode: Some(snapshot.probability_mode.clone()),
                freshness_status: Some(snapshot.freshness_status.clone()),
                time_to_risk_bucket: Some(snapshot.time_to_risk_bucket.clone()),
                split_name: None,
                features,
                primary_scenario_id: scenario_labels.primary_scenario_id,
                scenario_family: scenario_labels.scenario_family,
                scenario_training_role: scenario_labels.scenario_training_role,
                days_to_primary_crisis_start: scenario_labels.days_to_primary_crisis_start,
                primary_scenario_supports_5d: scenario_labels.primary_scenario_supports_5d,
                primary_scenario_supports_20d: scenario_labels.primary_scenario_supports_20d,
                primary_scenario_supports_60d: scenario_labels.primary_scenario_supports_60d,
                label_5d: scenario_labels.label_5d,
                label_20d: scenario_labels.label_20d,
                label_60d: scenario_labels.label_60d,
                regime_5d: scenario_labels.regime_5d,
                regime_20d: scenario_labels.regime_20d,
                regime_60d: scenario_labels.regime_60d,
                action_label_5d: scenario_labels.action_label_5d,
                action_label_20d: scenario_labels.action_label_20d,
                action_label_60d: scenario_labels.action_label_60d,
                prepare_episode_label: scenario_labels.prepare_episode_label,
                hedge_episode_label: scenario_labels.hedge_episode_label,
                defend_episode_label: scenario_labels.defend_episode_label,
                primary_action_level: scenario_labels.primary_action_level,
                action_episode_id: scenario_labels.action_episode_id,
                action_episode_phase: scenario_labels.action_episode_phase,
                protected_action_window: scenario_labels.protected_action_window,
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| row.as_of_date);
    rows
}

fn pipeline_features_from_snapshot(snapshot: &PredictionSnapshotRecord) -> BTreeMap<String, f64> {
    BTreeMap::from([
        (
            crate::FEATURE_OVERALL_SCORE.to_string(),
            (snapshot.overall_score / 100.0).clamp(0.0, 1.0),
        ),
        (
            crate::FEATURE_EXTERNAL_SHOCK_SCORE.to_string(),
            (snapshot.external_shock_score / 100.0).clamp(0.0, 1.0),
        ),
        (
            crate::FEATURE_HEURISTIC_P_5D.to_string(),
            snapshot.raw_p_5d.clamp(0.0, 0.99),
        ),
        (
            crate::FEATURE_HEURISTIC_P_20D.to_string(),
            snapshot.raw_p_20d.clamp(0.0, 0.99),
        ),
        (
            crate::FEATURE_HEURISTIC_P_60D.to_string(),
            snapshot.raw_p_60d.clamp(0.0, 0.99),
        ),
        (
            crate::FEATURE_COVERAGE_SCORE.to_string(),
            snapshot.coverage_score.clamp(0.0, 1.0),
        ),
        (
            crate::FEATURE_BUCKET_MONTHS_OR_HIGHER.to_string(),
            matches!(
                snapshot.time_to_risk_bucket.as_str(),
                "months" | "weeks" | "now"
            ) as u8 as f64,
        ),
        (
            crate::FEATURE_BUCKET_WEEKS_OR_HIGHER.to_string(),
            matches!(snapshot.time_to_risk_bucket.as_str(), "weeks" | "now") as u8 as f64,
        ),
        (
            crate::FEATURE_BUCKET_NOW.to_string(),
            matches!(snapshot.time_to_risk_bucket.as_str(), "now") as u8 as f64,
        ),
        (
            crate::FEATURE_FRESHNESS_DELAYED_OR_WORSE.to_string(),
            matches!(
                snapshot.freshness_status.as_str(),
                "delayed" | "stale" | "missing"
            ) as u8 as f64,
        ),
        (
            crate::FEATURE_FRESHNESS_STALE_OR_MISSING.to_string(),
            matches!(snapshot.freshness_status.as_str(), "stale" | "missing") as u8 as f64,
        ),
    ])
}

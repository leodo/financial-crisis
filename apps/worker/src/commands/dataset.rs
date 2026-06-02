use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
    path::PathBuf,
};

use anyhow::{bail, Context};
use chrono::{NaiveDate, Utc};
use fc_domain::{
    ActionabilityLevel, FeatureSnapshotRecord, FormalDatasetManifest, FormalDatasetRecord,
    FormalDatasetRowRecord,
};
use serde::Serialize;

use super::feature::FeatureSnapshotBuildOptions;

#[derive(Debug, Clone)]
pub(crate) struct FormalDatasetBuildOptions {
    pub(crate) feature: FeatureSnapshotBuildOptions,
    pub(crate) dataset_id: String,
    pub(crate) dataset_version: Option<String>,
    pub(crate) label_version: String,
    pub(crate) scenario_set_version: String,
}

impl FormalDatasetBuildOptions {
    pub(crate) fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut dataset_id = crate::DEFAULT_FORMAL_DATASET_ID.to_string();
        let mut dataset_version = None;
        let mut label_version = crate::DEFAULT_FORMAL_LABEL_VERSION.to_string();
        let mut scenario_set_version = crate::DEFAULT_FORMAL_SCENARIO_SET_VERSION.to_string();
        let mut feature_args = Vec::new();
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
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
                "--label-version" => {
                    index += 1;
                    label_version = args
                        .get(index)
                        .with_context(|| "--label-version requires a value")?
                        .clone();
                }
                "--scenario-set-version" => {
                    index += 1;
                    scenario_set_version = args
                        .get(index)
                        .with_context(|| "--scenario-set-version requires a value")?
                        .clone();
                }
                other => feature_args.push(other.to_string()),
            }
            index += 1;
        }
        Ok(Self {
            feature: FeatureSnapshotBuildOptions::parse(&feature_args)?,
            dataset_id,
            dataset_version,
            label_version,
            scenario_set_version,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FormalDatasetListOptions {
    pub(crate) market_scope: Option<String>,
    pub(crate) dataset_id: Option<String>,
    pub(crate) limit: Option<usize>,
}

impl FormalDatasetListOptions {
    pub(crate) fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut market_scope = None;
        let mut dataset_id = None;
        let mut limit = Some(10_usize);
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
                "--dataset-id" => {
                    index += 1;
                    dataset_id = Some(
                        args.get(index)
                            .with_context(|| "--dataset-id requires a value")?
                            .clone(),
                    );
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
                other => bail!("unknown formal dataset list option: {other}"),
            }
            index += 1;
        }
        Ok(Self {
            market_scope,
            dataset_id,
            limit,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FormalDatasetSummaryOptions {
    pub(crate) market_scope: Option<String>,
    pub(crate) dataset_id: String,
    pub(crate) dataset_version: Option<String>,
    pub(crate) dataset_key: Option<String>,
    pub(crate) output_dir: PathBuf,
}

impl FormalDatasetSummaryOptions {
    pub(crate) fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut market_scope = None;
        let mut dataset_id = crate::DEFAULT_FORMAL_DATASET_ID.to_string();
        let mut dataset_version = None;
        let mut dataset_key = None;
        let mut output_dir = PathBuf::from(crate::DEFAULT_FORMAL_DATASET_SUMMARY_OUTPUT_DIR);
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
                "--output-dir" => {
                    index += 1;
                    output_dir = PathBuf::from(
                        args.get(index)
                            .with_context(|| "--output-dir requires a directory path")?,
                    );
                }
                other => bail!("unknown formal dataset summary option: {other}"),
            }
            index += 1;
        }
        Ok(Self {
            market_scope,
            dataset_id,
            dataset_version,
            dataset_key,
            output_dir,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct FormalDatasetSplitSummary {
    split_name: String,
    row_count: usize,
    positive_5d_count: usize,
    positive_5d_rate: f64,
    positive_20d_count: usize,
    positive_20d_rate: f64,
    positive_60d_count: usize,
    positive_60d_rate: f64,
    prepare_primary_count: usize,
    prepare_primary_rate: f64,
    hedge_primary_count: usize,
    hedge_primary_rate: f64,
    defend_primary_count: usize,
    defend_primary_rate: f64,
    late_validation_row_count: usize,
    late_validation_row_rate: f64,
    protected_row_count: usize,
    protected_row_rate: f64,
    avg_coverage_score: f64,
    avg_core_feature_coverage: f64,
    avg_trigger_feature_coverage: f64,
    avg_external_feature_coverage: f64,
    scenario_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct FormalDatasetScenarioSummary {
    scenario_id: String,
    label: Option<String>,
    row_count: usize,
    split_count: usize,
    first_as_of_date: NaiveDate,
    last_as_of_date: NaiveDate,
    family: Option<String>,
    training_role: Option<String>,
    protected_window: Option<bool>,
    episode_template_id: Option<String>,
    default_horizon_roles: Vec<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct FormalDatasetFamilySummary {
    family: String,
    row_count: usize,
    scenario_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct FormalDatasetQualitySummary {
    grade: String,
    row_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct FormalDatasetRegimeSummary {
    split_name: String,
    horizon_days: u32,
    regime: String,
    row_count: usize,
    row_rate: f64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct FormalDatasetSummaryEnvelope {
    pub(crate) generated_at: String,
    pub(crate) dataset_key: String,
    pub(crate) dataset: FormalDatasetRecord,
    pub(crate) split_summaries: Vec<FormalDatasetSplitSummary>,
    pub(crate) scenario_summaries: Vec<FormalDatasetScenarioSummary>,
    pub(crate) family_summaries: Vec<FormalDatasetFamilySummary>,
    pub(crate) quality_summaries: Vec<FormalDatasetQualitySummary>,
    pub(crate) regime_summaries: Vec<FormalDatasetRegimeSummary>,
    pub(crate) recommendation: String,
}

#[derive(Debug, Clone)]
struct ScenarioSummaryMetadata {
    label: String,
    family: String,
    training_role: String,
    protected_window: bool,
    episode_template_id: String,
    default_horizon_roles: Vec<u32>,
}

#[derive(Debug, Clone)]
pub(crate) struct FormalDatasetScenarioSets {
    pub(crate) positive_scenarios: Vec<crate::CrisisScenario>,
    pub(crate) context_scenarios: Vec<crate::CrisisScenario>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FormalDatasetSplitProfile {
    Main,
    ExtensionAcute,
    ExtensionStress,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct FormalDatasetSplitRequirements {
    minimum_scenario_ranges: usize,
    minimum_calibration_scenarios: usize,
    minimum_evaluation_scenarios: usize,
    require_forward_5d: bool,
    require_forward_20d: bool,
    require_forward_60d: bool,
    require_prepare: bool,
    require_hedge: bool,
    require_defend: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ScenarioRowRange {
    pub(crate) scenario_id: String,
    pub(crate) family: String,
    pub(crate) start_index: usize,
    pub(crate) end_index: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct FormalSplitLabelSupport {
    forward_5d: Vec<usize>,
    forward_20d: Vec<usize>,
    forward_60d: Vec<usize>,
    prepare_primary: Vec<usize>,
    hedge_primary: Vec<usize>,
    defend_primary: Vec<usize>,
}

pub(crate) async fn research_formal_dataset_build_main(args: &[String]) -> anyhow::Result<()> {
    let options = FormalDatasetBuildOptions::parse(args)?;
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;
    let (indicators, observations) =
        super::feature::load_formal_feature_inputs(&store, options.feature.to).await?;
    let snapshot_build = super::feature::build_or_load_feature_snapshots(
        &store,
        &indicators,
        &observations,
        &options.feature,
    )
    .await?;
    let snapshots = snapshot_build.snapshots;
    if snapshots.is_empty() {
        bail!("no feature snapshots were generated for the requested range");
    }
    store.upsert_feature_snapshots(&snapshots).await?;

    let generated_at = Utc::now();
    let dataset_version = options
        .dataset_version
        .clone()
        .unwrap_or_else(|| format!("{}", generated_at.format("%Y%m%dT%H%M%S")));
    let dataset_key = crate::formal_dataset_key(&options.dataset_id, &dataset_version);
    let rows = build_main_formal_dataset_rows_with_catalog(
        &dataset_key,
        &snapshots,
        &options.feature.point_in_time_mode,
        &options.label_version,
        &options.scenario_set_version,
    )?;
    if rows.is_empty() {
        let ready_count = snapshots
            .iter()
            .filter(|snapshot| snapshot.visibility_status == crate::FEATURE_SNAPSHOT_STATUS_READY)
            .count();
        bail!(
            "no formal dataset rows passed the minimum coverage / visibility thresholds (pit_mode={}, ready_snapshots={}, total_snapshots={})",
            options.feature.point_in_time_mode,
            ready_count,
            snapshots.len()
        );
    }

    let train_count = rows.iter().filter(|row| row.split_name == "train").count();
    let calibration_count = rows
        .iter()
        .filter(|row| row.split_name == "calibration")
        .count();
    let evaluation_count = rows
        .iter()
        .filter(|row| row.split_name == "evaluation")
        .count();
    if train_count == 0 || calibration_count == 0 || evaluation_count == 0 {
        bail!(
            "formal dataset range is too short to produce train/calibration/evaluation splits (rows={}, train={}, calibration={}, evaluation={}); expand the date range before persisting this dataset",
            rows.len(),
            train_count,
            calibration_count,
            evaluation_count
        );
    }

    let dataset = FormalDatasetRecord {
        manifest: FormalDatasetManifest {
            dataset_id: options.dataset_id.clone(),
            dataset_version: dataset_version.clone(),
            market_scope: options.feature.market_scope.clone(),
            feature_set_version: options.feature.feature_set_version.clone(),
            label_version: options.label_version.clone(),
            scenario_set_version: options.scenario_set_version.clone(),
            point_in_time_mode: options.feature.point_in_time_mode.clone(),
            from_date: rows.first().map(|row| row.as_of_date),
            to_date: rows.last().map(|row| row.as_of_date),
            train_end_date: rows
                .iter()
                .rev()
                .find(|row| row.split_name == "train")
                .map(|row| row.as_of_date),
            calibration_end_date: rows
                .iter()
                .rev()
                .find(|row| row.split_name == "calibration")
                .map(|row| row.as_of_date),
            evaluation_start_date: rows
                .iter()
                .find(|row| row.split_name == "evaluation")
                .map(|row| row.as_of_date),
            row_count: rows.len(),
            note: "Built from raw observations and point-in-time feature snapshots; persists forward crisis labels, bounded action-window proxy labels, and episode-native prepare/hedge/defend labels so formal training can optimize for earlier executable warnings without losing the original crisis-start reference.".to_string(),
        },
        created_at: generated_at,
    };
    store.upsert_formal_dataset(&dataset).await?;
    store
        .replace_formal_dataset_rows(&dataset_key, &rows)
        .await?;

    println!("Built formal dataset {dataset_key}.");
    println!(
        "  rows={} train={} calibration={} evaluation={}",
        rows.len(),
        train_count,
        calibration_count,
        evaluation_count
    );
    println!(
        "  range={} -> {} feature_set_version={} point_in_time_mode={}",
        dataset
            .manifest
            .from_date
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string()),
        dataset
            .manifest
            .to_date
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string()),
        dataset.manifest.feature_set_version,
        dataset.manifest.point_in_time_mode
    );
    println!(
        "  snapshots reused={} recomputed={}",
        snapshot_build.reused_count, snapshot_build.recomputed_count
    );
    Ok(())
}

fn build_main_formal_dataset_rows_with_catalog(
    dataset_key: &str,
    snapshots: &[FeatureSnapshotRecord],
    point_in_time_mode: &str,
    label_version: &str,
    scenario_set_version: &str,
) -> anyhow::Result<Vec<FormalDatasetRowRecord>> {
    let scenario_sets = load_formal_dataset_scenario_sets(scenario_set_version, label_version)?;
    let positive_scenarios = scenario_sets.positive_scenarios;
    let context_scenarios = scenario_sets.context_scenarios;
    let min_date = formal_dataset_min_date(label_version);
    let mut rows = snapshots
        .iter()
        .filter(|snapshot| snapshot.as_of_date >= min_date)
        .filter(|snapshot| formal_dataset_snapshot_is_usable(snapshot, label_version))
        .map(|snapshot| {
            let scenario_labels = crate::derive_scenario_label_snapshot(
                snapshot.as_of_date,
                &positive_scenarios,
                &context_scenarios,
            );
            FormalDatasetRowRecord {
                dataset_key: dataset_key.to_string(),
                split_name: String::new(),
                entity_id: snapshot.entity_id.clone(),
                market_scope: snapshot.market_scope.clone(),
                as_of_date: snapshot.as_of_date,
                point_in_time_mode: point_in_time_mode.to_string(),
                latest_visible_at: snapshot.latest_visible_at,
                coverage_score: snapshot.coverage_score,
                core_feature_coverage: snapshot.core_feature_coverage,
                trigger_feature_coverage: snapshot.trigger_feature_coverage,
                external_feature_coverage: snapshot.external_feature_coverage,
                sample_quality_grade: crate::feature_quality_grade(snapshot.coverage_score)
                    .to_string(),
                primary_scenario_id: scenario_labels.primary_scenario_id,
                scenario_family: scenario_labels.scenario_family,
                scenario_training_role: scenario_labels.scenario_training_role,
                label_5d: scenario_labels.label_5d,
                label_20d: scenario_labels.label_20d,
                label_60d: scenario_labels.label_60d,
                regime_5d: crate::probability_training_regime_name(scenario_labels.regime_5d)
                    .to_string(),
                regime_20d: crate::probability_training_regime_name(scenario_labels.regime_20d)
                    .to_string(),
                regime_60d: crate::probability_training_regime_name(scenario_labels.regime_60d)
                    .to_string(),
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
                features: snapshot.features.clone(),
                created_at: Utc::now(),
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| row.as_of_date);
    assign_formal_dataset_splits(&mut rows, &context_scenarios, label_version);
    Ok(rows)
}

pub(crate) async fn research_formal_dataset_list_main(args: &[String]) -> anyhow::Result<()> {
    let options = FormalDatasetListOptions::parse(args)?;
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;
    let datasets = store
        .list_formal_datasets(
            options.market_scope.as_deref(),
            options.dataset_id.as_deref(),
            options.limit,
        )
        .await?;
    if datasets.is_empty() {
        println!("No formal datasets found.");
        return Ok(());
    }

    for dataset in datasets {
        let dataset_key = crate::formal_dataset_key(
            &dataset.manifest.dataset_id,
            &dataset.manifest.dataset_version,
        );
        println!(
            "[{}] {} rows={} feature_set={} label={} pit={} range={} -> {}",
            dataset_key,
            dataset.manifest.market_scope,
            dataset.manifest.row_count,
            dataset.manifest.feature_set_version,
            dataset.manifest.label_version,
            dataset.manifest.point_in_time_mode,
            dataset
                .manifest
                .from_date
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
            dataset
                .manifest
                .to_date
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string())
        );
    }
    Ok(())
}

pub(crate) async fn research_formal_dataset_summarize_main(args: &[String]) -> anyhow::Result<()> {
    let options = FormalDatasetSummaryOptions::parse(args)?;
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;
    let dataset_key = super::pipeline::resolve_formal_dataset_key(
        &store,
        options.dataset_key.as_deref(),
        &options.dataset_id,
        options.dataset_version.as_deref(),
        options.market_scope.as_deref(),
    )
    .await?;
    let dataset = store
        .load_formal_dataset(&dataset_key)
        .await?
        .with_context(|| format!("formal dataset {dataset_key} was not found in SQLite"))?;
    let rows = store
        .list_formal_dataset_rows(&dataset_key, None, None)
        .await?;
    if rows.is_empty() {
        bail!("formal dataset {dataset_key} has no persisted rows");
    }
    let summary = crate::build_formal_dataset_summary(&dataset_key, dataset, &rows)?;
    crate::write_formal_dataset_summary_report(&options.output_dir, &summary)?;
    crate::print_formal_dataset_summary(&summary);
    Ok(())
}

pub(crate) fn formal_dataset_min_date(label_version: &str) -> NaiveDate {
    match label_version {
        "formal_label_v1_ext_acute" => NaiveDate::from_ymd_opt(1987, 1, 1).expect("valid date"),
        _ => NaiveDate::from_ymd_opt(1990, 1, 2).expect("valid date"),
    }
}

fn scenario_family_code(family: fc_domain::CrisisScenarioFamily) -> &'static str {
    match family {
        fc_domain::CrisisScenarioFamily::AcuteMarketLiquidityCrash => {
            "acute_market_liquidity_crash"
        }
        fc_domain::CrisisScenarioFamily::SystemicCreditBankingCrisis => {
            "systemic_credit_banking_crisis"
        }
        fc_domain::CrisisScenarioFamily::MixedSystemicStress => "mixed_systemic_stress",
        fc_domain::CrisisScenarioFamily::RateShockOrPolicyDislocation => {
            "rate_shock_or_policy_dislocation"
        }
    }
}

fn scenario_training_role_code(role: fc_domain::CrisisScenarioTrainingRole) -> &'static str {
    match role {
        fc_domain::CrisisScenarioTrainingRole::Mandatory => "mandatory",
        fc_domain::CrisisScenarioTrainingRole::CandidateOptional => "candidate_optional",
        fc_domain::CrisisScenarioTrainingRole::ExtensionOnly => "extension_only",
        fc_domain::CrisisScenarioTrainingRole::NoPositiveMain => "no_positive_main",
    }
}

fn action_episode_template_code(template: fc_domain::ActionEpisodeTemplateId) -> &'static str {
    match template {
        fc_domain::ActionEpisodeTemplateId::AcuteMarketLiquidityCrash => {
            "acute_market_liquidity_crash"
        }
        fc_domain::ActionEpisodeTemplateId::SystemicCreditBankingCrisis => {
            "systemic_credit_banking_crisis"
        }
        fc_domain::ActionEpisodeTemplateId::MixedSystemicStress => "mixed_systemic_stress",
        fc_domain::ActionEpisodeTemplateId::RateShockOrPolicyDislocation => {
            "rate_shock_or_policy_dislocation"
        }
    }
}

pub(crate) fn formal_dataset_snapshot_is_usable(
    snapshot: &FeatureSnapshotRecord,
    label_version: &str,
) -> bool {
    match label_version {
        "formal_label_v1_ext_stress" => {
            snapshot.visibility_status == crate::FEATURE_SNAPSHOT_STATUS_READY
                && snapshot.coverage_score >= 0.75
                && snapshot.core_feature_coverage >= 0.85
                && snapshot.trigger_feature_coverage >= 0.80
                && snapshot.external_feature_coverage >= 0.50
                && crate::has_main_dataset_core_features(&snapshot.features)
        }
        "formal_label_v1_ext_acute" => {
            matches!(
                snapshot.visibility_status.as_str(),
                crate::FEATURE_SNAPSHOT_STATUS_READY
                    | crate::FEATURE_SNAPSHOT_STATUS_COVERAGE_OR_VISIBILITY_FAILED
            ) && snapshot.coverage_score >= 0.55
                && snapshot.core_feature_coverage >= 0.60
                && snapshot.trigger_feature_coverage >= 0.50
                && snapshot.external_feature_coverage >= 0.50
                && crate::has_extension_acute_core_features(&snapshot.features)
        }
        _ => {
            snapshot.visibility_status == crate::FEATURE_SNAPSHOT_STATUS_READY
                && snapshot.coverage_score >= 0.85
                && snapshot.core_feature_coverage >= 0.90
                && snapshot.trigger_feature_coverage >= 0.75
                && snapshot.external_feature_coverage >= 0.70
                && crate::has_main_dataset_core_features(&snapshot.features)
        }
    }
}

pub(crate) fn formal_dataset_split_profile(label_version: &str) -> FormalDatasetSplitProfile {
    match label_version {
        "formal_label_v1_ext_acute" => FormalDatasetSplitProfile::ExtensionAcute,
        "formal_label_v1_ext_stress" => FormalDatasetSplitProfile::ExtensionStress,
        _ => FormalDatasetSplitProfile::Main,
    }
}

pub(crate) fn formal_dataset_split_requirements(
    label_version: &str,
) -> FormalDatasetSplitRequirements {
    match formal_dataset_split_profile(label_version) {
        FormalDatasetSplitProfile::Main => FormalDatasetSplitRequirements {
            minimum_scenario_ranges: 3,
            minimum_calibration_scenarios: 2,
            minimum_evaluation_scenarios: 2,
            require_forward_5d: true,
            require_forward_20d: true,
            require_forward_60d: true,
            require_prepare: true,
            require_hedge: true,
            require_defend: true,
        },
        FormalDatasetSplitProfile::ExtensionAcute => FormalDatasetSplitRequirements {
            minimum_scenario_ranges: 2,
            minimum_calibration_scenarios: 2,
            minimum_evaluation_scenarios: 1,
            require_forward_5d: true,
            require_forward_20d: true,
            require_forward_60d: false,
            require_prepare: false,
            require_hedge: false,
            require_defend: true,
        },
        FormalDatasetSplitProfile::ExtensionStress => FormalDatasetSplitRequirements {
            minimum_scenario_ranges: 3,
            minimum_calibration_scenarios: 2,
            minimum_evaluation_scenarios: 2,
            require_forward_5d: false,
            require_forward_20d: true,
            require_forward_60d: true,
            require_prepare: true,
            require_hedge: true,
            require_defend: false,
        },
    }
}

pub(crate) fn assign_formal_dataset_splits(
    rows: &mut [FormalDatasetRowRecord],
    scenarios: &[crate::CrisisScenario],
    label_version: &str,
) {
    let ranges = collect_formal_dataset_scenario_ranges(rows, scenarios);
    let split_requirements = formal_dataset_split_requirements(label_version);
    let Ok((train_end, calibration_end)) =
        scenario_aware_formal_split_bounds(rows, &ranges, split_requirements)
            .or_else(|_| crate::chronological_split_bounds(rows.len()))
    else {
        return;
    };
    for (index, row) in rows.iter_mut().enumerate() {
        row.split_name = split_name_for_index(index, train_end, calibration_end).to_string();
    }
}

pub(crate) fn scenario_aware_formal_split_bounds(
    rows: &[FormalDatasetRowRecord],
    ranges: &[ScenarioRowRange],
    split_requirements: FormalDatasetSplitRequirements,
) -> anyhow::Result<(usize, usize)> {
    if ranges.len() < split_requirements.minimum_scenario_ranges {
        bail!(
            "fewer than {} scenario ranges available for scenario-aware split",
            split_requirements.minimum_scenario_ranges
        );
    }
    let (baseline_train_end, baseline_calibration_end) =
        crate::chronological_split_bounds(rows.len())?;
    let label_support = FormalSplitLabelSupport::from_rows(rows);
    let mut best_candidate = None::<(usize, usize, usize, usize, usize)>;

    for first_boundary_scenario in 0..ranges.len().saturating_sub(1) {
        let train_candidates = split_boundaries_within_scenario(&ranges[first_boundary_scenario]);
        for second_boundary_scenario in (first_boundary_scenario + 1)..ranges.len() {
            let calibration_candidates =
                split_boundaries_within_scenario(&ranges[second_boundary_scenario]);
            for &train_end in &train_candidates {
                for &calibration_end in &calibration_candidates {
                    if crate::validate_split_bounds(rows.len(), train_end, calibration_end).is_err()
                    {
                        continue;
                    }

                    let calibration_scenario_count =
                        scenario_count_for_split_range(ranges, train_end, calibration_end);
                    let evaluation_scenario_count =
                        scenario_count_for_split_range(ranges, calibration_end, rows.len());
                    if calibration_scenario_count < split_requirements.minimum_calibration_scenarios
                        || evaluation_scenario_count
                            < split_requirements.minimum_evaluation_scenarios
                    {
                        continue;
                    }

                    if !label_support.split_has_required_label_support(
                        0,
                        train_end,
                        split_requirements,
                    ) || !label_support.split_has_required_label_support(
                        train_end,
                        calibration_end,
                        split_requirements,
                    ) || !label_support.split_has_required_label_support(
                        calibration_end,
                        rows.len(),
                        split_requirements,
                    ) {
                        continue;
                    }

                    let scenario_coverage =
                        calibration_scenario_count.saturating_add(evaluation_scenario_count);
                    let evaluation_actionability_support_score =
                        split_actionability_scenario_support_score(
                            rows,
                            ranges,
                            calibration_end,
                            rows.len(),
                            split_requirements,
                        );
                    let deviation_from_baseline = train_end.abs_diff(baseline_train_end)
                        + calibration_end.abs_diff(baseline_calibration_end);
                    let replace_candidate = match best_candidate {
                        None => true,
                        Some((
                            best_train_end,
                            best_calibration_end,
                            best_coverage,
                            best_actionability_support_score,
                            best_deviation,
                        )) => {
                            scenario_coverage > best_coverage
                                || (scenario_coverage == best_coverage
                                    && evaluation_actionability_support_score
                                        > best_actionability_support_score)
                                || (scenario_coverage == best_coverage
                                    && evaluation_actionability_support_score
                                        == best_actionability_support_score
                                    && deviation_from_baseline < best_deviation)
                                || (scenario_coverage == best_coverage
                                    && evaluation_actionability_support_score
                                        == best_actionability_support_score
                                    && deviation_from_baseline == best_deviation
                                    && (train_end > best_train_end
                                        || (train_end == best_train_end
                                            && calibration_end > best_calibration_end)))
                        }
                    };
                    if replace_candidate {
                        best_candidate = Some((
                            train_end,
                            calibration_end,
                            scenario_coverage,
                            evaluation_actionability_support_score,
                            deviation_from_baseline,
                        ));
                    }
                }
            }
        }
    }

    best_candidate
        .map(|(train_end, calibration_end, _, _, _)| (train_end, calibration_end))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "no scenario-aware split preserves multi-scenario calibration/evaluation coverage together with forward/action-episode label support"
            )
        })
}

pub(crate) fn collect_formal_dataset_scenario_ranges(
    rows: &[FormalDatasetRowRecord],
    scenarios: &[crate::CrisisScenario],
) -> Vec<ScenarioRowRange> {
    let family_by_id = scenarios
        .iter()
        .map(|scenario| (scenario.scenario_id.as_str(), scenario.family.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut ranges = BTreeMap::<String, (usize, usize)>::new();
    for (index, row) in rows.iter().enumerate() {
        let Some(scenario_id) = row.primary_scenario_id.as_ref() else {
            continue;
        };
        ranges
            .entry(scenario_id.clone())
            .and_modify(|range| range.1 = index)
            .or_insert((index, index));
    }

    let mut summaries = ranges
        .into_iter()
        .map(|(scenario_id, (start_index, end_index))| ScenarioRowRange {
            family: family_by_id
                .get(scenario_id.as_str())
                .cloned()
                .or_else(|| rows[start_index].scenario_family.clone())
                .unwrap_or_else(|| "unknown".to_string()),
            scenario_id,
            start_index,
            end_index,
        })
        .collect::<Vec<_>>();
    summaries.sort_by_key(|range| range.start_index);
    summaries
}

impl FormalSplitLabelSupport {
    pub(crate) fn from_rows(rows: &[FormalDatasetRowRecord]) -> Self {
        let mut support = Self {
            forward_5d: Vec::with_capacity(rows.len() + 1),
            forward_20d: Vec::with_capacity(rows.len() + 1),
            forward_60d: Vec::with_capacity(rows.len() + 1),
            prepare_primary: Vec::with_capacity(rows.len() + 1),
            hedge_primary: Vec::with_capacity(rows.len() + 1),
            defend_primary: Vec::with_capacity(rows.len() + 1),
        };
        support.forward_5d.push(0);
        support.forward_20d.push(0);
        support.forward_60d.push(0);
        support.prepare_primary.push(0);
        support.hedge_primary.push(0);
        support.defend_primary.push(0);
        for row in rows {
            support.forward_5d.push(
                support.forward_5d.last().copied().unwrap_or_default()
                    + usize::from(row.label_5d > 0),
            );
            support.forward_20d.push(
                support.forward_20d.last().copied().unwrap_or_default()
                    + usize::from(row.label_20d > 0),
            );
            support.forward_60d.push(
                support.forward_60d.last().copied().unwrap_or_default()
                    + usize::from(row.label_60d > 0),
            );
            support.prepare_primary.push(
                support.prepare_primary.last().copied().unwrap_or_default()
                    + usize::from(row.prepare_episode_label > 0),
            );
            support.hedge_primary.push(
                support.hedge_primary.last().copied().unwrap_or_default()
                    + usize::from(row.hedge_episode_label > 0),
            );
            support.defend_primary.push(
                support.defend_primary.last().copied().unwrap_or_default()
                    + usize::from(row.defend_episode_label > 0),
            );
        }
        support
    }

    pub(crate) fn split_has_required_label_support(
        &self,
        start: usize,
        end: usize,
        split_requirements: FormalDatasetSplitRequirements,
    ) -> bool {
        end > start
            && (!split_requirements.require_forward_5d
                || self.has_positive(&self.forward_5d, start, end))
            && (!split_requirements.require_forward_20d
                || self.has_positive(&self.forward_20d, start, end))
            && (!split_requirements.require_forward_60d
                || self.has_positive(&self.forward_60d, start, end))
            && (!split_requirements.require_prepare
                || self.has_positive(&self.prepare_primary, start, end))
            && (!split_requirements.require_hedge
                || self.has_positive(&self.hedge_primary, start, end))
            && (!split_requirements.require_defend
                || self.has_positive(&self.defend_primary, start, end))
    }

    fn has_positive(&self, prefix: &[usize], start: usize, end: usize) -> bool {
        prefix[end] > prefix[start]
    }
}

fn split_boundaries_within_scenario(range: &ScenarioRowRange) -> Vec<usize> {
    ((range.start_index + 1)..=range.end_index.saturating_add(1)).collect()
}

pub(crate) fn scenario_count_for_split_range(
    ranges: &[ScenarioRowRange],
    start: usize,
    end: usize,
) -> usize {
    ranges
        .iter()
        .filter(|range| start <= range.end_index && end > range.start_index)
        .count()
}

fn split_actionability_scenario_support_score(
    rows: &[FormalDatasetRowRecord],
    ranges: &[ScenarioRowRange],
    start: usize,
    end: usize,
    split_requirements: FormalDatasetSplitRequirements,
) -> usize {
    let mut score = 0;
    if split_requirements.require_prepare {
        score += actionability_positive_scenario_count_for_split_range(
            rows,
            ranges,
            start,
            end,
            ActionabilityLevel::Prepare,
        )
        .min(2);
    }
    if split_requirements.require_hedge {
        score += actionability_positive_scenario_count_for_split_range(
            rows,
            ranges,
            start,
            end,
            ActionabilityLevel::Hedge,
        )
        .min(2);
    }
    if split_requirements.require_defend {
        score += actionability_positive_scenario_count_for_split_range(
            rows,
            ranges,
            start,
            end,
            ActionabilityLevel::Defend,
        )
        .min(2);
    }
    score
}

fn actionability_positive_scenario_count_for_split_range(
    rows: &[FormalDatasetRowRecord],
    ranges: &[ScenarioRowRange],
    start: usize,
    end: usize,
    level: ActionabilityLevel,
) -> usize {
    ranges
        .iter()
        .filter(|range| {
            let overlap_start = start.max(range.start_index);
            let overlap_end = end.min(range.end_index.saturating_add(1));
            overlap_start < overlap_end
                && rows[overlap_start..overlap_end].iter().any(|row| {
                    row.primary_scenario_id.as_deref() == Some(range.scenario_id.as_str())
                        && row_has_action_episode_label(row, level)
                })
        })
        .count()
}

pub(crate) fn row_has_action_episode_label(
    row: &FormalDatasetRowRecord,
    level: ActionabilityLevel,
) -> bool {
    match level {
        ActionabilityLevel::Prepare => row.prepare_episode_label > 0,
        ActionabilityLevel::Hedge => row.hedge_episode_label > 0,
        ActionabilityLevel::Defend => row.defend_episode_label > 0,
    }
}

pub(crate) fn scenario_count_for_index_range(
    rows: &[FormalDatasetRowRecord],
    start: usize,
    end: usize,
) -> usize {
    rows[start.min(rows.len())..end.min(rows.len())]
        .iter()
        .filter_map(|row| row.primary_scenario_id.as_ref())
        .collect::<BTreeSet<_>>()
        .len()
}

fn split_name_for_index(index: usize, train_end: usize, calibration_end: usize) -> &'static str {
    if index < train_end {
        "train"
    } else if index < calibration_end {
        "calibration"
    } else {
        "evaluation"
    }
}

pub(crate) fn load_label_set_crisis_scenarios(
    scenario_set_version: &str,
    label_set_id: &str,
) -> anyhow::Result<Vec<crate::CrisisScenario>> {
    let catalog = crate::load_crisis_scenario_catalog();
    if catalog.catalog_id != scenario_set_version {
        bail!(
            "scenario set version {} is not available in the active catalog {}; set FC_SCENARIO_CATALOG_PATH to another catalog or use {}",
            scenario_set_version,
            catalog.catalog_id,
            catalog.catalog_id
        );
    }

    load_label_set_crisis_scenarios_from_catalog(&catalog, label_set_id)
}

pub(crate) fn load_formal_dataset_scenario_sets(
    scenario_set_version: &str,
    label_set_id: &str,
) -> anyhow::Result<FormalDatasetScenarioSets> {
    let catalog = crate::load_crisis_scenario_catalog();
    if catalog.catalog_id != scenario_set_version {
        bail!(
            "scenario set version {} is not available in the active catalog {}; set FC_SCENARIO_CATALOG_PATH to another catalog or use {}",
            scenario_set_version,
            catalog.catalog_id,
            catalog.catalog_id
        );
    }

    let positive_scenarios = load_label_set_crisis_scenarios_from_catalog(&catalog, label_set_id)?;
    let mut context_scenarios = positive_scenarios.clone();
    if label_set_id == crate::DEFAULT_FORMAL_LABEL_VERSION {
        let protected_context_scenarios = load_window_set_crisis_scenarios_from_catalog(
            &catalog,
            crate::DEFAULT_FORMAL_MAIN_CONTEXT_WINDOW_SET_ID,
        )?;
        for scenario in protected_context_scenarios {
            if context_scenarios
                .iter()
                .any(|existing| existing.scenario_id == scenario.scenario_id)
            {
                continue;
            }
            context_scenarios.push(scenario);
        }
        context_scenarios.sort_by_key(|scenario| scenario.crisis_start);
    }

    Ok(FormalDatasetScenarioSets {
        positive_scenarios,
        context_scenarios,
    })
}

fn load_label_set_crisis_scenarios_from_catalog(
    catalog: &fc_domain::CrisisScenarioCatalog,
    label_set_id: &str,
) -> anyhow::Result<Vec<crate::CrisisScenario>> {
    let scenarios = catalog
        .scenarios_for_label_set(label_set_id)
        .with_context(|| format!("label set {label_set_id} was not found in scenario catalog"))?;
    Ok(scenarios
        .into_iter()
        .map(crisis_scenario_from_definition)
        .collect())
}

fn load_window_set_crisis_scenarios_from_catalog(
    catalog: &fc_domain::CrisisScenarioCatalog,
    window_set_id: &str,
) -> anyhow::Result<Vec<crate::CrisisScenario>> {
    let scenario_ids = catalog
        .scenario_ids_for_window_set(window_set_id)
        .with_context(|| format!("window set {window_set_id} was not found in scenario catalog"))?;
    let mut scenarios = Vec::with_capacity(scenario_ids.len());
    for scenario_id in scenario_ids {
        let scenario = catalog
            .scenarios
            .iter()
            .find(|scenario| scenario.scenario_id == *scenario_id)
            .with_context(|| {
                format!("window set {window_set_id} references unknown scenario {scenario_id}")
            })?;
        scenarios.push(crisis_scenario_from_definition(scenario));
    }
    Ok(scenarios)
}

fn crisis_scenario_from_definition(
    scenario: &fc_domain::CrisisScenarioDefinition,
) -> crate::CrisisScenario {
    crate::CrisisScenario {
        scenario_id: scenario.scenario_id.clone(),
        family: scenario_family_code(scenario.family).to_string(),
        training_role: scenario_training_role_code(scenario.training_role).to_string(),
        pre_warning_start: scenario.pre_warning_start,
        crisis_start: scenario.crisis_start,
        acute_start: scenario.acute_start,
        crisis_end: scenario.crisis_end,
        default_horizon_roles: scenario.default_horizon_roles.clone(),
        protected_window: scenario.protected_window,
        protected_action_levels: scenario.protected_action_levels.clone(),
        episode_template_id: scenario
            .episode_template_id
            .expect("validated scenario catalog must include episode_template_id"),
        action_episode_overrides: scenario.action_episode_overrides.clone(),
    }
}

pub(crate) fn build_formal_dataset_summary(
    dataset_key: &str,
    dataset: FormalDatasetRecord,
    rows: &[FormalDatasetRowRecord],
) -> anyhow::Result<FormalDatasetSummaryEnvelope> {
    let scenarios = crate::load_label_set_crisis_scenarios(
        &dataset.manifest.scenario_set_version,
        &dataset.manifest.label_version,
    )?;
    let scenario_metadata =
        load_formal_dataset_scenario_metadata(&dataset.manifest.scenario_set_version)?;
    let scenario_ranges = crate::collect_formal_dataset_scenario_ranges(rows, &scenarios);
    let split_summaries = summarize_formal_dataset_splits(rows, &scenario_ranges);
    let scenario_summaries =
        summarize_formal_dataset_scenarios(rows, &scenario_ranges, &scenario_metadata);
    let family_summaries = summarize_formal_dataset_families(rows);
    let quality_summaries = summarize_formal_dataset_quality(rows);
    let regime_summaries = summarize_formal_dataset_regimes(rows, &scenarios);
    let recommendation = build_formal_dataset_recommendation(
        &dataset.manifest.label_version,
        &split_summaries,
        rows.len(),
    );

    Ok(FormalDatasetSummaryEnvelope {
        generated_at: Utc::now().to_rfc3339(),
        dataset_key: dataset_key.to_string(),
        dataset,
        split_summaries,
        scenario_summaries,
        family_summaries,
        quality_summaries,
        regime_summaries,
        recommendation,
    })
}

fn summarize_formal_dataset_splits(
    rows: &[FormalDatasetRowRecord],
    scenario_ranges: &[crate::ScenarioRowRange],
) -> Vec<FormalDatasetSplitSummary> {
    ["train", "calibration", "evaluation"]
        .into_iter()
        .filter_map(|split_name| {
            let split_rows = rows
                .iter()
                .filter(|row| row.split_name == split_name)
                .collect::<Vec<_>>();
            let split_start = rows
                .iter()
                .position(|row| row.split_name == split_name)
                .unwrap_or_default();
            let split_end = rows
                .iter()
                .rposition(|row| row.split_name == split_name)
                .map(|index| index + 1)
                .unwrap_or_default();
            (!split_rows.is_empty()).then(|| FormalDatasetSplitSummary {
                split_name: split_name.to_string(),
                row_count: split_rows.len(),
                positive_5d_count: split_rows.iter().filter(|row| row.label_5d > 0).count(),
                positive_5d_rate: crate::round6(forward_label_rate(&split_rows, 5)),
                positive_20d_count: split_rows.iter().filter(|row| row.label_20d > 0).count(),
                positive_20d_rate: crate::round6(forward_label_rate(&split_rows, 20)),
                positive_60d_count: split_rows.iter().filter(|row| row.label_60d > 0).count(),
                positive_60d_rate: crate::round6(forward_label_rate(&split_rows, 60)),
                prepare_primary_count: split_rows
                    .iter()
                    .filter(|row| row.prepare_episode_label > 0)
                    .count(),
                prepare_primary_rate: crate::round6(action_episode_primary_rate(
                    &split_rows,
                    ActionabilityLevel::Prepare,
                )),
                hedge_primary_count: split_rows
                    .iter()
                    .filter(|row| row.hedge_episode_label > 0)
                    .count(),
                hedge_primary_rate: crate::round6(action_episode_primary_rate(
                    &split_rows,
                    ActionabilityLevel::Hedge,
                )),
                defend_primary_count: split_rows
                    .iter()
                    .filter(|row| row.defend_episode_label > 0)
                    .count(),
                defend_primary_rate: crate::round6(action_episode_primary_rate(
                    &split_rows,
                    ActionabilityLevel::Defend,
                )),
                late_validation_row_count: split_rows
                    .iter()
                    .filter(|row| row.action_episode_phase == "late_validation")
                    .count(),
                late_validation_row_rate: crate::round6(late_validation_row_rate(&split_rows)),
                protected_row_count: split_rows
                    .iter()
                    .filter(|row| row.protected_action_window)
                    .count(),
                protected_row_rate: crate::round6(protected_action_window_rate(&split_rows)),
                avg_coverage_score: crate::round3(avg_metric(&split_rows, |row| {
                    row.coverage_score
                })),
                avg_core_feature_coverage: crate::round3(avg_metric(&split_rows, |row| {
                    row.core_feature_coverage
                })),
                avg_trigger_feature_coverage: crate::round3(avg_metric(&split_rows, |row| {
                    row.trigger_feature_coverage
                })),
                avg_external_feature_coverage: crate::round3(avg_metric(&split_rows, |row| {
                    row.external_feature_coverage
                })),
                scenario_count: crate::scenario_count_for_split_range(
                    scenario_ranges,
                    split_start,
                    split_end,
                ),
            })
        })
        .collect()
}

fn summarize_formal_dataset_scenarios(
    rows: &[FormalDatasetRowRecord],
    scenario_ranges: &[crate::ScenarioRowRange],
    scenario_metadata: &BTreeMap<String, ScenarioSummaryMetadata>,
) -> Vec<FormalDatasetScenarioSummary> {
    scenario_ranges
        .iter()
        .map(|range| {
            let metadata = scenario_metadata.get(&range.scenario_id);
            FormalDatasetScenarioSummary {
                scenario_id: range.scenario_id.clone(),
                label: metadata.map(|item| item.label.clone()),
                row_count: range.end_index.saturating_sub(range.start_index) + 1,
                split_count: rows[range.start_index..=range.end_index]
                    .iter()
                    .map(|row| row.split_name.as_str())
                    .collect::<BTreeSet<_>>()
                    .len(),
                first_as_of_date: rows[range.start_index].as_of_date,
                last_as_of_date: rows[range.end_index].as_of_date,
                family: metadata
                    .map(|item| item.family.clone())
                    .or_else(|| Some(range.family.clone())),
                training_role: metadata.map(|item| item.training_role.clone()),
                protected_window: metadata.map(|item| item.protected_window),
                episode_template_id: metadata.map(|item| item.episode_template_id.clone()),
                default_horizon_roles: metadata
                    .map(|item| item.default_horizon_roles.clone())
                    .unwrap_or_default(),
            }
        })
        .collect()
}

fn summarize_formal_dataset_families(
    rows: &[FormalDatasetRowRecord],
) -> Vec<FormalDatasetFamilySummary> {
    let mut buckets = BTreeMap::<String, Vec<&FormalDatasetRowRecord>>::new();
    for row in rows.iter().filter(|row| row.scenario_family.is_some()) {
        let family = row.scenario_family.clone().unwrap_or_default();
        buckets.entry(family).or_default().push(row);
    }

    buckets
        .into_iter()
        .map(|(family, family_rows)| FormalDatasetFamilySummary {
            row_count: family_rows.len(),
            scenario_count: family_rows
                .iter()
                .filter_map(|row| row.primary_scenario_id.as_ref())
                .collect::<BTreeSet<_>>()
                .len(),
            family,
        })
        .collect()
}

fn summarize_formal_dataset_quality(
    rows: &[FormalDatasetRowRecord],
) -> Vec<FormalDatasetQualitySummary> {
    let mut buckets = BTreeMap::<String, usize>::new();
    for row in rows {
        *buckets.entry(row.sample_quality_grade.clone()).or_default() += 1;
    }
    buckets
        .into_iter()
        .map(|(grade, row_count)| FormalDatasetQualitySummary { grade, row_count })
        .collect()
}

fn summarize_formal_dataset_regimes(
    rows: &[FormalDatasetRowRecord],
    scenarios: &[crate::CrisisScenario],
) -> Vec<FormalDatasetRegimeSummary> {
    let split_totals = rows
        .iter()
        .fold(BTreeMap::<String, usize>::new(), |mut acc, row| {
            *acc.entry(row.split_name.clone()).or_default() += 1;
            acc
        });
    let mut buckets = BTreeMap::<(String, u32, String), usize>::new();
    for row in rows {
        for horizon_days in [5_u32, 20_u32, 60_u32] {
            let regime = crate::probability_training_regime_name(
                crate::forward_crisis_training_regime(row.as_of_date, scenarios, horizon_days),
            );
            *buckets
                .entry((row.split_name.clone(), horizon_days, regime.to_string()))
                .or_default() += 1;
        }
    }

    buckets
        .into_iter()
        .map(|((split_name, horizon_days, regime), row_count)| {
            let split_total = split_totals.get(&split_name).copied().unwrap_or_default();
            FormalDatasetRegimeSummary {
                split_name,
                horizon_days,
                regime,
                row_count,
                row_rate: crate::round6(crate::safe_ratio(row_count, split_total)),
            }
        })
        .collect()
}

fn build_formal_dataset_recommendation(
    label_version: &str,
    split_summaries: &[FormalDatasetSplitSummary],
    total_rows: usize,
) -> String {
    let evaluation = split_summaries
        .iter()
        .find(|split| split.split_name == "evaluation");
    if total_rows < 5_000 {
        return "样本量仍偏小，先继续补历史数据，再用这版数据集训练正式候选版。".to_string();
    }
    match crate::formal_dataset_split_profile(label_version) {
        crate::FormalDatasetSplitProfile::ExtensionAcute => {
            let Some(evaluation) = evaluation else {
                return "缺少 evaluation split，当前还不能稳定比较 1987/1998 的急性冲击表现。"
                    .to_string();
            };
            if evaluation.scenario_count < 1 || evaluation.defend_primary_count == 0 {
                return "evaluation 仍未覆盖足够的 acute 尾段主正例，先继续重做 split 或补齐 1987/1998 proxy 覆盖。".to_string();
            }
            if evaluation.prepare_primary_count == 0 || evaluation.hedge_primary_count == 0 {
                return "这套扩展 acute 数据集已经能用于 1987/1998 的 5d/20d 与急性尾段类比，但 evaluation 还不足以单独评估完整的 prepare/hedge/defend episode 头。".to_string();
            }
            return "这套扩展 acute 数据集已经可以用于 1987/1998 的 5d/20d 历史类比与短窗研究；它是研究包，不应用作正式主模型上线判断。".to_string();
        }
        crate::FormalDatasetSplitProfile::ExtensionStress => {
            let Some(evaluation) = evaluation else {
                return "缺少 evaluation split，当前还不能稳定比较 protected stress 扩展场景。"
                    .to_string();
            };
            if evaluation.scenario_count < 1
                || evaluation.prepare_primary_count == 0
                || evaluation.hedge_primary_count == 0
            {
                return "evaluation 的 protected stress / extension 主正例仍偏少，先继续重做 split，再把它用于扩展研究和 posture 对照。".to_string();
            }
            if evaluation.protected_row_count < 1 {
                return "evaluation 还没有 protected stress 尾段样本，当前不适合拿它判断受保护压力窗口是否稳定。".to_string();
            }
            return "这套扩展 stress 数据集已经可以用于 protected stress、历史对照和扩展训练研究；它不是正式主模型 go/no-go 的单独依据。".to_string();
        }
        crate::FormalDatasetSplitProfile::Main => {}
    }
    if let Some(evaluation) = evaluation {
        if evaluation.hedge_primary_count < 10 || evaluation.prepare_primary_count < 10 {
            return "evaluation 的 episode-native 主正例仍偏少，当前更适合作研究版比较，不适合直接给正式模型做上线判断。".to_string();
        }
        if evaluation.late_validation_row_count < 5 {
            return "evaluation 的 late-validation 样本仍偏少，动作头很难判断“过晚确认”到底是偶然还是系统性问题。".to_string();
        }
        if evaluation.protected_row_count < 5 {
            return "evaluation 的 protected stress 样本仍偏少，当前还不适合把 protected/cooldown 行为当成稳定结论。".to_string();
        }
        if evaluation.scenario_count < 2 {
            return format!(
                "evaluation split 的 episode-native 动作标签目前只覆盖 {} 个场景，动作头评估很不稳；应先扩历史场景或重做 split，再用它判断 formal 候选版优劣。",
                evaluation.scenario_count
            );
        }
        if evaluation.avg_coverage_score < 0.85 {
            return "evaluation 覆盖率偏低，应先补可见性/覆盖率，再看训练结果。".to_string();
        }
    }
    "样本量、split 和覆盖率已具备基础研究条件，可以进入正式训练与 release review。".to_string()
}

fn forward_label_rate(rows: &[&FormalDatasetRowRecord], horizon_days: u32) -> f64 {
    let positives = rows
        .iter()
        .filter(|row| match horizon_days {
            5 => row.label_5d > 0,
            20 => row.label_20d > 0,
            60 => row.label_60d > 0,
            _ => false,
        })
        .count();
    positives as f64 / rows.len() as f64
}

fn action_episode_primary_rate(rows: &[&FormalDatasetRowRecord], level: ActionabilityLevel) -> f64 {
    let positives = rows
        .iter()
        .filter(|row| crate::row_has_action_episode_label(row, level))
        .count();
    positives as f64 / rows.len() as f64
}

fn late_validation_row_rate(rows: &[&FormalDatasetRowRecord]) -> f64 {
    let positives = rows
        .iter()
        .filter(|row| row.action_episode_phase == "late_validation")
        .count();
    positives as f64 / rows.len() as f64
}

fn protected_action_window_rate(rows: &[&FormalDatasetRowRecord]) -> f64 {
    let positives = rows
        .iter()
        .filter(|row| row.protected_action_window)
        .count();
    positives as f64 / rows.len() as f64
}

fn avg_metric<F>(rows: &[&FormalDatasetRowRecord], accessor: F) -> f64
where
    F: Fn(&FormalDatasetRowRecord) -> f64,
{
    rows.iter().map(|row| accessor(row)).sum::<f64>() / rows.len() as f64
}

pub(crate) fn render_formal_dataset_summary_markdown(
    summary: &FormalDatasetSummaryEnvelope,
) -> String {
    let mut markdown = String::new();
    let manifest = &summary.dataset.manifest;
    let _ = writeln!(markdown, "# Formal Dataset Summary");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "- Generated at: {}", summary.generated_at);
    let _ = writeln!(markdown, "- Dataset key: {}", summary.dataset_key);
    let _ = writeln!(markdown, "- Market scope: {}", manifest.market_scope);
    let _ = writeln!(markdown, "- Feature set: {}", manifest.feature_set_version);
    let _ = writeln!(markdown, "- Label version: {}", manifest.label_version);
    let _ = writeln!(
        markdown,
        "- Scenario set: {}",
        manifest.scenario_set_version
    );
    let _ = writeln!(markdown, "- PIT mode: {}", manifest.point_in_time_mode);
    let _ = writeln!(markdown, "- Rows: {}", manifest.row_count);
    let _ = writeln!(
        markdown,
        "- Range: {} -> {}",
        manifest
            .from_date
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string()),
        manifest
            .to_date
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string())
    );
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Split Summary");
    let _ = writeln!(markdown);
    let _ = writeln!(
        markdown,
        "| Split | Rows | Forward 5d+ | Forward 20d+ | Forward 60d+ | Prepare Primary | Hedge Primary | Defend Primary | Late Validation | Protected | Avg Coverage | Core | Trigger | External | Scenarios |"
    );
    let _ = writeln!(
        markdown,
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
    );
    for split in &summary.split_summaries {
        let _ = writeln!(
            markdown,
            "| {} | {} | {} ({}) | {} ({}) | {} ({}) | {} ({}) | {} ({}) | {} ({}) | {} ({}) | {} ({}) | {:.1}% | {:.1}% | {:.1}% | {:.1}% | {} |",
            split.split_name,
            split.row_count,
            split.positive_5d_count,
            crate::format_pct(split.positive_5d_rate),
            split.positive_20d_count,
            crate::format_pct(split.positive_20d_rate),
            split.positive_60d_count,
            crate::format_pct(split.positive_60d_rate),
            split.prepare_primary_count,
            crate::format_pct(split.prepare_primary_rate),
            split.hedge_primary_count,
            crate::format_pct(split.hedge_primary_rate),
            split.defend_primary_count,
            crate::format_pct(split.defend_primary_rate),
            split.late_validation_row_count,
            crate::format_pct(split.late_validation_row_rate),
            split.protected_row_count,
            crate::format_pct(split.protected_row_rate),
            split.avg_coverage_score * 100.0,
            split.avg_core_feature_coverage * 100.0,
            split.avg_trigger_feature_coverage * 100.0,
            split.avg_external_feature_coverage * 100.0,
            split.scenario_count
        );
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Scenario Coverage");
    let _ = writeln!(markdown);
    let _ = writeln!(
        markdown,
        "| Scenario | Label | Family | Role | Protected | Horizons | Template | Rows | Splits | Range |"
    );
    let _ = writeln!(
        markdown,
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
    );
    for scenario in &summary.scenario_summaries {
        let default_horizon_roles = if scenario.default_horizon_roles.is_empty() {
            "-".to_string()
        } else {
            scenario
                .default_horizon_roles
                .iter()
                .map(|value| format!("{value}d"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let _ = writeln!(
            markdown,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} -> {} |",
            scenario.scenario_id,
            scenario.label.as_deref().unwrap_or("-"),
            scenario.family.as_deref().unwrap_or("-"),
            scenario.training_role.as_deref().unwrap_or("-"),
            scenario
                .protected_window
                .map(|value| if value { "yes" } else { "no" })
                .unwrap_or("-"),
            default_horizon_roles,
            scenario.episode_template_id.as_deref().unwrap_or("-"),
            scenario.row_count,
            scenario.split_count,
            scenario.first_as_of_date,
            scenario.last_as_of_date
        );
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Quality Mix");
    let _ = writeln!(markdown);
    for quality in &summary.quality_summaries {
        let _ = writeln!(
            markdown,
            "- grade {}: {} rows",
            quality.grade, quality.row_count
        );
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Regime Mix");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "| Split | Horizon | Regime | Rows | Share |");
    let _ = writeln!(markdown, "| --- | --- | --- | --- | --- |");
    for regime in &summary.regime_summaries {
        let _ = writeln!(
            markdown,
            "| {} | {}d | {} | {} | {} |",
            regime.split_name,
            regime.horizon_days,
            regime.regime,
            regime.row_count,
            crate::format_pct(regime.row_rate),
        );
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Recommendation");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "{}", summary.recommendation);
    markdown
}

pub(crate) fn print_formal_dataset_summary(summary: &FormalDatasetSummaryEnvelope) {
    println!(
        "Formal dataset {} rows={} pit={} feature_set={}",
        summary.dataset_key,
        summary.dataset.manifest.row_count,
        summary.dataset.manifest.point_in_time_mode,
        summary.dataset.manifest.feature_set_version
    );
    for split in &summary.split_summaries {
        println!(
            "  split={} rows={} forward[5d={}({}) 20d={}({}) 60d={}({})] action[prepare={}({}) hedge={}({}) defend={}({}) late_validation={}({}) protected={}({})] avg_coverage={:.1}%",
            split.split_name,
            split.row_count,
            split.positive_5d_count,
            crate::format_pct(split.positive_5d_rate),
            split.positive_20d_count,
            crate::format_pct(split.positive_20d_rate),
            split.positive_60d_count,
            crate::format_pct(split.positive_60d_rate),
            split.prepare_primary_count,
            crate::format_pct(split.prepare_primary_rate),
            split.hedge_primary_count,
            crate::format_pct(split.hedge_primary_rate),
            split.defend_primary_count,
            crate::format_pct(split.defend_primary_rate),
            split.late_validation_row_count,
            crate::format_pct(split.late_validation_row_rate),
            split.protected_row_count,
            crate::format_pct(split.protected_row_rate),
            split.avg_coverage_score * 100.0
        );
    }
    println!("  recommendation {}", summary.recommendation);
}

fn load_formal_dataset_scenario_metadata(
    scenario_set_version: &str,
) -> anyhow::Result<BTreeMap<String, ScenarioSummaryMetadata>> {
    let catalog = crate::load_crisis_scenario_catalog();
    if catalog.catalog_id != scenario_set_version {
        bail!(
            "scenario set version {} is not available in the active catalog {}; set FC_SCENARIO_CATALOG_PATH to another catalog or use {}",
            scenario_set_version,
            catalog.catalog_id,
            catalog.catalog_id
        );
    }

    Ok(catalog
        .scenarios
        .into_iter()
        .map(|scenario| {
            (
                scenario.scenario_id.clone(),
                ScenarioSummaryMetadata {
                    label: scenario.label,
                    family: scenario_family_code(scenario.family).to_string(),
                    training_role: scenario_training_role_code(scenario.training_role).to_string(),
                    protected_window: scenario.protected_window,
                    episode_template_id: action_episode_template_code(
                        scenario
                            .episode_template_id
                            .expect("validated scenario catalog must include episode_template_id"),
                    )
                    .to_string(),
                    default_horizon_roles: scenario.default_horizon_roles,
                },
            )
        })
        .collect())
}

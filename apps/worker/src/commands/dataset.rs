use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
    path::PathBuf,
};

use anyhow::{bail, Context};
use chrono::{NaiveDate, Utc};
use fc_domain::{
    ActionabilityLevel, FormalDatasetManifest, FormalDatasetRecord, FormalDatasetRowRecord,
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
    let rows = crate::build_main_formal_dataset_rows_with_catalog(
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
        crate::load_formal_dataset_scenario_metadata(&dataset.manifest.scenario_set_version)?;
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
    scenario_metadata: &BTreeMap<String, crate::ScenarioSummaryMetadata>,
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

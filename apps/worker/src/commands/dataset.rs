use std::path::PathBuf;

use anyhow::{bail, Context};
use chrono::{NaiveDate, Utc};
use fc_domain::{
    FeatureSnapshotRecord, FormalDatasetManifest, FormalDatasetRecord, FormalDatasetRowRecord,
};

use super::feature::FeatureSnapshotBuildOptions;
mod report;
mod scenarios;
mod split;

use report::{
    build_formal_dataset_slice_export, build_formal_dataset_summary,
    print_formal_dataset_slice_summary, print_formal_dataset_summary,
    write_formal_dataset_slice_report,
};
#[cfg(test)]
pub(crate) use report::{render_formal_dataset_slice_csv, sanitize_filename_component};
pub(crate) use report::{render_formal_dataset_summary_markdown, FormalDatasetSummaryEnvelope};
pub(crate) use scenarios::{
    action_episode_template_code, load_formal_dataset_scenario_sets,
    load_label_set_crisis_scenarios, scenario_family_code, scenario_training_role_code,
};
use split::assign_formal_dataset_splits;
#[cfg(test)]
pub(crate) use split::scenario_count_for_index_range;
pub(crate) use split::{
    collect_formal_dataset_scenario_ranges, formal_dataset_split_profile,
    row_has_action_episode_label, scenario_count_for_split_range, FormalDatasetSplitProfile,
    ScenarioRowRange,
};
#[cfg(test)]
pub(crate) use split::{
    formal_dataset_split_requirements, scenario_aware_formal_split_bounds, FormalSplitLabelSupport,
};

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

#[derive(Debug, Clone)]
pub(crate) struct FormalDatasetSliceOptions {
    pub(crate) market_scope: Option<String>,
    pub(crate) dataset_id: String,
    pub(crate) dataset_version: Option<String>,
    pub(crate) dataset_key: Option<String>,
    pub(crate) scenario_id: String,
    pub(crate) split_name: Option<String>,
    pub(crate) from_date: Option<NaiveDate>,
    pub(crate) to_date: Option<NaiveDate>,
    pub(crate) limit: Option<usize>,
    pub(crate) output_dir: PathBuf,
}

impl FormalDatasetSliceOptions {
    pub(crate) fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut market_scope = None;
        let mut dataset_id = crate::DEFAULT_FORMAL_DATASET_ID.to_string();
        let mut dataset_version = None;
        let mut dataset_key = None;
        let mut scenario_id = None;
        let mut split_name = None;
        let mut from_date = None;
        let mut to_date = None;
        let mut limit = None;
        let mut output_dir = PathBuf::from(crate::DEFAULT_FORMAL_DATASET_SLICE_OUTPUT_DIR);
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
                "--scenario-id" => {
                    index += 1;
                    scenario_id = Some(
                        args.get(index)
                            .with_context(|| "--scenario-id requires a value")?
                            .clone(),
                    );
                }
                "--split-name" => {
                    index += 1;
                    split_name = Some(
                        args.get(index)
                            .with_context(|| "--split-name requires a value")?
                            .clone(),
                    );
                }
                "--from" => {
                    index += 1;
                    from_date = Some(
                        NaiveDate::parse_from_str(
                            args.get(index)
                                .with_context(|| "--from requires a YYYY-MM-DD value")?,
                            "%Y-%m-%d",
                        )
                        .context("--from must use YYYY-MM-DD")?,
                    );
                }
                "--to" => {
                    index += 1;
                    to_date = Some(
                        NaiveDate::parse_from_str(
                            args.get(index)
                                .with_context(|| "--to requires a YYYY-MM-DD value")?,
                            "%Y-%m-%d",
                        )
                        .context("--to must use YYYY-MM-DD")?,
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
                "--output-dir" => {
                    index += 1;
                    output_dir = PathBuf::from(
                        args.get(index)
                            .with_context(|| "--output-dir requires a directory path")?,
                    );
                }
                other => bail!("unknown formal dataset slice option: {other}"),
            }
            index += 1;
        }
        let scenario_id = scenario_id.with_context(|| "--scenario-id is required")?;
        if let (Some(from_date), Some(to_date)) = (from_date, to_date) {
            if from_date > to_date {
                bail!("--from must be earlier than or equal to --to");
            }
        }
        Ok(Self {
            market_scope,
            dataset_id,
            dataset_version,
            dataset_key,
            scenario_id,
            split_name,
            from_date,
            to_date,
            limit,
            output_dir,
        })
    }
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
    let summary = build_formal_dataset_summary(&dataset_key, dataset, &rows)?;
    crate::write_formal_dataset_summary_report(&options.output_dir, &summary)?;
    print_formal_dataset_summary(&summary);
    Ok(())
}

pub(crate) async fn research_formal_dataset_slice_main(args: &[String]) -> anyhow::Result<()> {
    let options = FormalDatasetSliceOptions::parse(args)?;
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
        .list_formal_dataset_rows(&dataset_key, options.split_name.as_deref(), None)
        .await?;
    if rows.is_empty() {
        bail!("formal dataset {dataset_key} has no persisted rows for the requested split filter");
    }

    let export = build_formal_dataset_slice_export(dataset_key.clone(), dataset, rows, &options)?;
    write_formal_dataset_slice_report(&options.output_dir, &export)?;
    print_formal_dataset_slice_summary(&export);
    Ok(())
}

pub(crate) fn formal_dataset_min_date(label_version: &str) -> NaiveDate {
    match label_version {
        "formal_label_v1_ext_acute" => NaiveDate::from_ymd_opt(1987, 1, 1).expect("valid date"),
        _ => NaiveDate::from_ymd_opt(1990, 1, 2).expect("valid date"),
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

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
};

use anyhow::{bail, Context};
use chrono::{NaiveDate, Utc};
use fc_domain::{
    HistoricalAssessmentPointRecord, LogisticProbabilityModelScoreDiagnostics, ModelReleaseRecord,
    ProbabilityDiagnostics, ProbabilityHorizonOverlayDiagnostics, ProbabilityOverlayContribution,
};
use serde::Serialize;

use super::ReleaseReviewOptions;

#[derive(Debug, Clone)]
pub(crate) struct ReleaseProbabilitySliceOptions {
    pub(crate) release_id: String,
    pub(crate) market_scope: Option<String>,
    pub(crate) api_reload_url: String,
    pub(crate) output_dir: PathBuf,
    pub(crate) history_mode: crate::ApiReloadHistoryMode,
    pub(crate) history_limit: usize,
    pub(crate) from_date: NaiveDate,
    pub(crate) to_date: NaiveDate,
    pub(crate) updated_by: String,
}

impl ReleaseProbabilitySliceOptions {
    pub(crate) fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut release_id = None;
        let mut market_scope = None;
        let mut api_reload_url = crate::DEFAULT_API_RELOAD_URL.to_string();
        let mut output_dir = PathBuf::from(crate::DEFAULT_RELEASE_PROBABILITY_SLICE_OUTPUT_DIR);
        let mut history_mode = crate::ApiReloadHistoryMode::StrictRebuild;
        let mut history_limit = 20_000_usize;
        let mut from_date = None;
        let mut to_date = None;
        let mut updated_by = "fc-worker-probability-slice".to_string();
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--release-id" => {
                    index += 1;
                    release_id = Some(
                        args.get(index)
                            .with_context(|| "--release-id requires a value")?
                            .clone(),
                    );
                }
                "--market-scope" => {
                    index += 1;
                    market_scope = Some(
                        args.get(index)
                            .with_context(|| "--market-scope requires a value")?
                            .clone(),
                    );
                }
                "--api-reload-url" => {
                    index += 1;
                    api_reload_url = args
                        .get(index)
                        .with_context(|| "--api-reload-url requires a URL")?
                        .clone();
                }
                "--output-dir" => {
                    index += 1;
                    output_dir = PathBuf::from(
                        args.get(index)
                            .with_context(|| "--output-dir requires a directory path")?,
                    );
                }
                "--history-mode" => {
                    index += 1;
                    history_mode = crate::ApiReloadHistoryMode::parse(
                        args.get(index)
                            .with_context(|| "--history-mode requires default|strict_rebuild")?,
                    )?;
                }
                "--history-limit" => {
                    index += 1;
                    history_limit = args
                        .get(index)
                        .with_context(|| "--history-limit requires a positive integer")?
                        .parse::<usize>()
                        .with_context(|| "--history-limit requires a positive integer")?;
                    if history_limit == 0 {
                        bail!("--history-limit requires a positive integer");
                    }
                }
                "--from" => {
                    index += 1;
                    from_date = Some(crate::parse_date_arg(args.get(index), "--from")?);
                }
                "--to" => {
                    index += 1;
                    to_date = Some(crate::parse_date_arg(args.get(index), "--to")?);
                }
                "--updated-by" => {
                    index += 1;
                    updated_by = args
                        .get(index)
                        .with_context(|| "--updated-by requires a value")?
                        .clone();
                }
                other => bail!("unknown release probability-slice option: {other}"),
            }
            index += 1;
        }
        let from_date = from_date.with_context(|| "--from is required")?;
        let to_date = to_date.with_context(|| "--to is required")?;
        if from_date > to_date {
            bail!("--from must be earlier than or equal to --to");
        }
        Ok(Self {
            release_id: release_id.with_context(|| "--release-id is required")?,
            market_scope,
            api_reload_url,
            output_dir,
            history_mode,
            history_limit,
            from_date,
            to_date,
            updated_by,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ReleaseFormalProbabilitySliceOptions {
    pub(crate) release_id: String,
    pub(crate) market_scope: Option<String>,
    pub(crate) dataset_id: String,
    pub(crate) dataset_version: Option<String>,
    pub(crate) dataset_key: Option<String>,
    pub(crate) scenario_id: Option<String>,
    pub(crate) from_date: NaiveDate,
    pub(crate) to_date: NaiveDate,
    pub(crate) output_dir: PathBuf,
}

impl ReleaseFormalProbabilitySliceOptions {
    pub(crate) fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut release_id = None;
        let mut market_scope = None;
        let mut dataset_id = crate::DEFAULT_FORMAL_DATASET_ID.to_string();
        let mut dataset_version = None;
        let mut dataset_key = None;
        let mut scenario_id = None;
        let mut from_date = None;
        let mut to_date = None;
        let mut output_dir = PathBuf::from(crate::DEFAULT_FORMAL_DATASET_SLICE_OUTPUT_DIR);
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--release-id" => {
                    index += 1;
                    release_id = Some(
                        args.get(index)
                            .with_context(|| "--release-id requires a value")?
                            .clone(),
                    );
                }
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
                "--from" => {
                    index += 1;
                    from_date = Some(crate::parse_date_arg(args.get(index), "--from")?);
                }
                "--to" => {
                    index += 1;
                    to_date = Some(crate::parse_date_arg(args.get(index), "--to")?);
                }
                "--output-dir" => {
                    index += 1;
                    output_dir = PathBuf::from(
                        args.get(index)
                            .with_context(|| "--output-dir requires a directory path")?,
                    );
                }
                other => bail!("unknown release formal probability-slice option: {other}"),
            }
            index += 1;
        }
        let from_date = from_date.with_context(|| "--from is required")?;
        let to_date = to_date.with_context(|| "--to is required")?;
        if from_date > to_date {
            bail!("--from must be earlier than or equal to --to");
        }
        Ok(Self {
            release_id: release_id.with_context(|| "--release-id is required")?,
            market_scope,
            dataset_id,
            dataset_version,
            dataset_key,
            scenario_id,
            from_date,
            to_date,
            output_dir,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ReleaseFormalProbabilityCompareOptions {
    pub(crate) baseline_release_id: String,
    pub(crate) candidate_release_id: String,
    pub(crate) market_scope: Option<String>,
    pub(crate) dataset_id: String,
    pub(crate) dataset_version: Option<String>,
    pub(crate) dataset_key: Option<String>,
    pub(crate) scenario_id: Option<String>,
    pub(crate) from_date: NaiveDate,
    pub(crate) to_date: NaiveDate,
    pub(crate) output_dir: PathBuf,
}

impl ReleaseFormalProbabilityCompareOptions {
    pub(crate) fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut baseline_release_id = None;
        let mut candidate_release_id = None;
        let mut market_scope = None;
        let mut dataset_id = crate::DEFAULT_FORMAL_DATASET_ID.to_string();
        let mut dataset_version = None;
        let mut dataset_key = None;
        let mut scenario_id = None;
        let mut from_date = None;
        let mut to_date = None;
        let mut output_dir = PathBuf::from(crate::DEFAULT_FORMAL_PROBABILITY_COMPARE_OUTPUT_DIR);
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--baseline-release-id" => {
                    index += 1;
                    baseline_release_id = Some(
                        args.get(index)
                            .with_context(|| "--baseline-release-id requires a value")?
                            .clone(),
                    );
                }
                "--candidate-release-id" => {
                    index += 1;
                    candidate_release_id = Some(
                        args.get(index)
                            .with_context(|| "--candidate-release-id requires a value")?
                            .clone(),
                    );
                }
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
                "--from" => {
                    index += 1;
                    from_date = Some(crate::parse_date_arg(args.get(index), "--from")?);
                }
                "--to" => {
                    index += 1;
                    to_date = Some(crate::parse_date_arg(args.get(index), "--to")?);
                }
                "--output-dir" => {
                    index += 1;
                    output_dir = PathBuf::from(
                        args.get(index)
                            .with_context(|| "--output-dir requires a directory path")?,
                    );
                }
                other => bail!("unknown release formal probability-compare option: {other}"),
            }
            index += 1;
        }
        let from_date = from_date.with_context(|| "--from is required")?;
        let to_date = to_date.with_context(|| "--to is required")?;
        if from_date > to_date {
            bail!("--from must be earlier than or equal to --to");
        }
        Ok(Self {
            baseline_release_id: baseline_release_id
                .with_context(|| "--baseline-release-id is required")?,
            candidate_release_id: candidate_release_id
                .with_context(|| "--candidate-release-id is required")?,
            market_scope,
            dataset_id,
            dataset_version,
            dataset_key,
            scenario_id,
            from_date,
            to_date,
            output_dir,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseProbabilitySliceExport {
    exported_at: String,
    market_scope: String,
    release_id: String,
    replay_run_id: String,
    history_mode: String,
    history_limit: usize,
    from_date: NaiveDate,
    to_date: NaiveDate,
    row_count: usize,
    rows: Vec<ReleaseProbabilitySlicePoint>,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseProbabilitySlicePoint {
    as_of_date: NaiveDate,
    overall_score: f64,
    structural_score: f64,
    trigger_score: f64,
    external_shock_score: f64,
    raw_p_5d: f64,
    raw_p_20d: f64,
    raw_p_60d: f64,
    calibrated_p_5d: f64,
    calibrated_p_20d: f64,
    calibrated_p_60d: f64,
    posture: String,
    time_to_risk_bucket: String,
    actionability_prepare: f64,
    actionability_hedge: f64,
    actionability_defend: f64,
    coverage_score: f64,
    freshness_status: String,
    posture_trigger_codes: Vec<String>,
    posture_blocker_codes: Vec<String>,
    probability_diagnostics: ProbabilityDiagnostics,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseFormalProbabilitySliceExport {
    exported_at: String,
    market_scope: String,
    release_id: String,
    dataset_key: String,
    scenario_id: Option<String>,
    from_date: NaiveDate,
    to_date: NaiveDate,
    row_count: usize,
    rows: Vec<ReleaseFormalProbabilitySlicePoint>,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseFormalProbabilitySlicePoint {
    as_of_date: NaiveDate,
    split_name: String,
    primary_scenario_id: Option<String>,
    scenario_family: Option<String>,
    regime_20d: String,
    regime_60d: String,
    prepare_episode_label: u8,
    hedge_episode_label: u8,
    defend_episode_label: u8,
    primary_action_level: Option<String>,
    coverage_score: f64,
    probability_diagnostics: ProbabilityDiagnostics,
    base_model_diagnostics: Vec<ReleaseFormalProbabilityBaseModelDiagnostics>,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseFormalProbabilityBaseModelDiagnostics {
    horizon_days: u32,
    base_model: LogisticProbabilityModelScoreDiagnostics,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseFormalProbabilityCompareExport {
    exported_at: String,
    market_scope: String,
    baseline_release_id: String,
    candidate_release_id: String,
    dataset_key: String,
    scenario_id: Option<String>,
    from_date: NaiveDate,
    to_date: NaiveDate,
    row_count: usize,
    baseline_thresholds: Vec<ReleaseFormalProbabilityThresholdSummary>,
    candidate_thresholds: Vec<ReleaseFormalProbabilityThresholdSummary>,
    summary: ReleaseFormalProbabilityCompareSummary,
    rows: Vec<ReleaseFormalProbabilityComparePoint>,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseFormalProbabilityThresholdSummary {
    horizon_days: u32,
    decision_threshold: Option<f64>,
    overlay_count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseFormalProbabilityCompareSummary {
    baseline_hit_count_20d: usize,
    candidate_hit_count_20d: usize,
    baseline_hit_count_60d: usize,
    candidate_hit_count_60d: usize,
    baseline_max_p_20d: f64,
    baseline_max_p_20d_date: Option<NaiveDate>,
    candidate_max_p_20d: f64,
    candidate_max_p_20d_date: Option<NaiveDate>,
    baseline_max_p_60d: f64,
    baseline_max_p_60d_date: Option<NaiveDate>,
    candidate_max_p_60d: f64,
    candidate_max_p_60d_date: Option<NaiveDate>,
    overall_window: ReleaseFormalProbabilityWindowAggregateSummary,
    hedge_window: ReleaseFormalProbabilityWindowAggregateSummary,
    positive_window_20d: ReleaseFormalProbabilityWindowAggregateSummary,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseFormalProbabilityWindowAggregateSummary {
    row_count: usize,
    avg_delta_p_20d: f64,
    avg_abs_delta_p_20d: f64,
    avg_delta_p_60d: f64,
    avg_abs_delta_p_60d: f64,
    baseline_hit_rate_20d: f64,
    candidate_hit_rate_20d: f64,
    baseline_hit_rate_60d: f64,
    candidate_hit_rate_60d: f64,
    top_feature_deltas_20d: Vec<ReleaseFormalProbabilityFeatureDeltaAggregate>,
    top_feature_deltas_60d: Vec<ReleaseFormalProbabilityFeatureDeltaAggregate>,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseFormalProbabilityFeatureDeltaAggregate {
    name: String,
    sum_delta_contribution: f64,
    abs_sum_delta_contribution: f64,
    mean_delta_contribution: f64,
    count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseFormalProbabilityComparePoint {
    as_of_date: NaiveDate,
    split_name: String,
    primary_scenario_id: Option<String>,
    scenario_family: Option<String>,
    regime_20d: String,
    regime_60d: String,
    prepare_episode_label: u8,
    hedge_episode_label: u8,
    defend_episode_label: u8,
    primary_action_level: Option<String>,
    coverage_score: f64,
    baseline_raw_p_20d: f64,
    candidate_raw_p_20d: f64,
    baseline_base_linear_20d: f64,
    candidate_base_linear_20d: f64,
    baseline_final_p_20d: f64,
    candidate_final_p_20d: f64,
    delta_final_p_20d: f64,
    baseline_hit_20d: bool,
    candidate_hit_20d: bool,
    baseline_raw_p_60d: f64,
    candidate_raw_p_60d: f64,
    baseline_base_linear_60d: f64,
    candidate_base_linear_60d: f64,
    baseline_final_p_60d: f64,
    candidate_final_p_60d: f64,
    delta_final_p_60d: f64,
    baseline_hit_60d: bool,
    candidate_hit_60d: bool,
    top_feature_deltas_20d: Vec<ReleaseFormalProbabilityFeatureDelta>,
    top_feature_deltas_60d: Vec<ReleaseFormalProbabilityFeatureDelta>,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseFormalProbabilityFeatureDelta {
    name: String,
    baseline_raw_value: f64,
    candidate_raw_value: f64,
    baseline_normalized_value: f64,
    candidate_normalized_value: f64,
    baseline_weight: f64,
    candidate_weight: f64,
    baseline_contribution: f64,
    candidate_contribution: f64,
    delta_contribution: f64,
}

struct ReleaseFormalProbabilityCompareBuildInput<'a> {
    market_scope: &'a str,
    dataset_key: &'a str,
    scenario_id: Option<String>,
    from_date: NaiveDate,
    to_date: NaiveDate,
    baseline_release_id: &'a str,
    candidate_release_id: &'a str,
    baseline_bundle: &'a fc_domain::ProbabilityBundle,
    candidate_bundle: &'a fc_domain::ProbabilityBundle,
    baseline_rows: Vec<ReleaseFormalProbabilitySlicePoint>,
    candidate_rows: Vec<ReleaseFormalProbabilitySlicePoint>,
}

pub(crate) async fn research_release_probability_slice(args: &[String]) -> anyhow::Result<()> {
    let options = ReleaseProbabilitySliceOptions::parse(args)?;
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;

    let target_release = store
        .load_model_release(&options.release_id)
        .await?
        .with_context(|| format!("release {} not found", options.release_id))?;
    let market_scope = options
        .market_scope
        .clone()
        .unwrap_or_else(|| target_release.manifest.market_scope.clone());
    if target_release.manifest.market_scope != market_scope {
        bail!(
            "release {} belongs to {}, not {}",
            target_release.manifest.release_id,
            target_release.manifest.market_scope,
            market_scope
        );
    }

    let original_active = store
        .load_active_model_release(&market_scope)
        .await?
        .with_context(|| format!("no active release found for market scope {market_scope}"))?;

    let review_options = ReleaseReviewOptions {
        candidate_release_id: target_release.manifest.release_id.clone(),
        baseline_release_id: None,
        market_scope: Some(market_scope.clone()),
        api_reload_url: options.api_reload_url.clone(),
        output_dir: PathBuf::from(crate::DEFAULT_RELEASE_REVIEW_OUTPUT_DIR),
        history_mode: options.history_mode,
        history_limit: options.history_limit,
        updated_by: options.updated_by.clone(),
    };

    let mut original_records = BTreeMap::<String, ModelReleaseRecord>::new();
    for release in [original_active.clone(), target_release.clone()] {
        original_records.insert(release.manifest.release_id.clone(), release);
    }

    let export_result = async {
        super::activate_release_for_review(
            &store,
            &market_scope,
            &target_release.manifest.release_id,
            &review_options,
            "probability-slice",
        )
        .await?;
        let (run, points) = load_release_probability_slice_points(
            &store,
            &market_scope,
            &target_release.manifest.release_id,
            options.from_date,
            options.to_date,
        )
        .await?;
        let export = ReleaseProbabilitySliceExport {
            exported_at: Utc::now().to_rfc3339(),
            market_scope: market_scope.clone(),
            release_id: target_release.manifest.release_id.clone(),
            replay_run_id: run.replay_run_id,
            history_mode: options.history_mode.as_label().to_string(),
            history_limit: options.history_limit,
            from_date: options.from_date,
            to_date: options.to_date,
            row_count: points.len(),
            rows: points
                .into_iter()
                .map(|point| ReleaseProbabilitySlicePoint {
                    as_of_date: point.as_of_date,
                    overall_score: point.overall_score,
                    structural_score: point.structural_score,
                    trigger_score: point.trigger_score,
                    external_shock_score: point.external_shock_score,
                    raw_p_5d: point.raw_p_5d,
                    raw_p_20d: point.raw_p_20d,
                    raw_p_60d: point.raw_p_60d,
                    calibrated_p_5d: point.calibrated_p_5d,
                    calibrated_p_20d: point.calibrated_p_20d,
                    calibrated_p_60d: point.calibrated_p_60d,
                    posture: point.posture,
                    time_to_risk_bucket: point.time_to_risk_bucket,
                    actionability_prepare: point.actionability_prepare,
                    actionability_hedge: point.actionability_hedge,
                    actionability_defend: point.actionability_defend,
                    coverage_score: point.coverage_score,
                    freshness_status: point.freshness_status,
                    posture_trigger_codes: point.posture_trigger_codes,
                    posture_blocker_codes: point.posture_blocker_codes,
                    probability_diagnostics: point.probability_diagnostics,
                })
                .collect(),
        };
        write_release_probability_slice_report(&options.output_dir, &export)?;
        print_release_probability_slice_summary(&export);
        Ok::<(), anyhow::Error>(())
    }
    .await;

    let restore_result = super::restore_release_review_state(
        &store,
        &market_scope,
        &original_active.manifest.release_id,
        &original_records,
        &options.api_reload_url,
        &options.updated_by,
    )
    .await;

    if let Err(restore_error) = restore_result {
        if let Err(export_error) = export_result {
            bail!(
                "release probability slice export failed and restore also failed:\nexport: {export_error:#}\nrestore: {restore_error:#}"
            );
        }
        bail!("release probability slice export completed but restore failed: {restore_error:#}");
    }

    export_result?;
    println!(
        "Release probability slice restored original active release {}.",
        original_active.manifest.release_id
    );
    Ok(())
}

pub(crate) async fn research_release_formal_probability_slice(
    args: &[String],
) -> anyhow::Result<()> {
    let options = ReleaseFormalProbabilitySliceOptions::parse(args)?;
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;

    let release = store
        .load_model_release(&options.release_id)
        .await?
        .with_context(|| format!("release {} not found", options.release_id))?;
    let market_scope = options
        .market_scope
        .clone()
        .unwrap_or_else(|| release.manifest.market_scope.clone());
    if release.manifest.market_scope != market_scope {
        bail!(
            "release {} belongs to {}, not {}",
            release.manifest.release_id,
            release.manifest.market_scope,
            market_scope
        );
    }

    let dataset_key = super::super::pipeline::resolve_formal_dataset_key(
        &store,
        options.dataset_key.as_deref(),
        &options.dataset_id,
        options.dataset_version.as_deref(),
        Some(&market_scope),
    )
    .await?;
    let rows = store
        .list_formal_dataset_rows(&dataset_key, None, None)
        .await?
        .into_iter()
        .filter(|row| row.as_of_date >= options.from_date && row.as_of_date <= options.to_date)
        .filter(|row| {
            options
                .scenario_id
                .as_deref()
                .map(|scenario_id| row.primary_scenario_id.as_deref() == Some(scenario_id))
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();
    if rows.is_empty() {
        bail!(
            "formal dataset {} has no rows in {} -> {} for scenario {}",
            dataset_key,
            options.from_date,
            options.to_date,
            options.scenario_id.as_deref().unwrap_or("all")
        );
    }

    let bundle = read_release_probability_bundle(&release)?;
    let export = ReleaseFormalProbabilitySliceExport {
        exported_at: Utc::now().to_rfc3339(),
        market_scope,
        release_id: release.manifest.release_id.clone(),
        dataset_key,
        scenario_id: options.scenario_id.clone(),
        from_date: options.from_date,
        to_date: options.to_date,
        row_count: rows.len(),
        rows: score_release_formal_probability_slice_rows(&bundle, rows),
    };
    write_release_formal_probability_slice_report(&options.output_dir, &export)?;
    print_release_formal_probability_slice_summary(&export);
    Ok(())
}

pub(crate) async fn research_release_formal_probability_compare(
    args: &[String],
) -> anyhow::Result<()> {
    let options = ReleaseFormalProbabilityCompareOptions::parse(args)?;
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;

    let baseline_release = store
        .load_model_release(&options.baseline_release_id)
        .await?
        .with_context(|| format!("release {} not found", options.baseline_release_id))?;
    let candidate_release = store
        .load_model_release(&options.candidate_release_id)
        .await?
        .with_context(|| format!("release {} not found", options.candidate_release_id))?;
    let market_scope = options
        .market_scope
        .clone()
        .unwrap_or_else(|| baseline_release.manifest.market_scope.clone());
    for release in [&baseline_release, &candidate_release] {
        if release.manifest.market_scope != market_scope {
            bail!(
                "release {} belongs to {}, not {}",
                release.manifest.release_id,
                release.manifest.market_scope,
                market_scope
            );
        }
    }

    let dataset_key = super::super::pipeline::resolve_formal_dataset_key(
        &store,
        options.dataset_key.as_deref(),
        &options.dataset_id,
        options.dataset_version.as_deref(),
        Some(&market_scope),
    )
    .await?;
    let rows = store
        .list_formal_dataset_rows(&dataset_key, None, None)
        .await?
        .into_iter()
        .filter(|row| row.as_of_date >= options.from_date && row.as_of_date <= options.to_date)
        .filter(|row| {
            options
                .scenario_id
                .as_deref()
                .map(|scenario_id| row.primary_scenario_id.as_deref() == Some(scenario_id))
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();
    if rows.is_empty() {
        bail!(
            "formal dataset {} has no rows in {} -> {} for scenario {}",
            dataset_key,
            options.from_date,
            options.to_date,
            options.scenario_id.as_deref().unwrap_or("all")
        );
    }

    let baseline_bundle = read_release_probability_bundle(&baseline_release)?;
    let candidate_bundle = read_release_probability_bundle(&candidate_release)?;
    let baseline_rows = score_release_formal_probability_slice_rows(&baseline_bundle, rows.clone());
    let candidate_rows = score_release_formal_probability_slice_rows(&candidate_bundle, rows);
    let export = build_release_formal_probability_compare_export(
        ReleaseFormalProbabilityCompareBuildInput {
            market_scope: &market_scope,
            dataset_key: &dataset_key,
            scenario_id: options.scenario_id.clone(),
            from_date: options.from_date,
            to_date: options.to_date,
            baseline_release_id: &baseline_release.manifest.release_id,
            candidate_release_id: &candidate_release.manifest.release_id,
            baseline_bundle: &baseline_bundle,
            candidate_bundle: &candidate_bundle,
            baseline_rows,
            candidate_rows,
        },
    )?;
    write_release_formal_probability_compare_report(&options.output_dir, &export)?;
    print_release_formal_probability_compare_summary(&export);
    Ok(())
}

async fn load_release_probability_slice_points(
    store: &fc_storage::SqliteStore,
    market_scope: &str,
    release_id: &str,
    from_date: NaiveDate,
    to_date: NaiveDate,
) -> anyhow::Result<(
    fc_domain::HistoricalReplayRunRecord,
    Vec<HistoricalAssessmentPointRecord>,
)> {
    let run = store
        .list_historical_replay_runs(
            Some(market_scope),
            Some(release_id),
            Some(from_date),
            Some(to_date),
            Some(20),
        )
        .await?
        .into_iter()
        .find(|run| run.from_date <= from_date && run.to_date >= to_date)
        .with_context(|| {
            format!(
                "no historical replay run covering {from_date} -> {to_date} was found for release {release_id} in {market_scope}; reload the API with strict_rebuild first"
            )
        })?;
    let points = store
        .list_historical_assessment_points(
            Some(&run.replay_run_id),
            Some(market_scope),
            Some(release_id),
            Some(from_date),
            Some(to_date),
            None,
        )
        .await?;
    let mut latest_by_date = BTreeMap::<NaiveDate, HistoricalAssessmentPointRecord>::new();
    for point in points {
        latest_by_date
            .entry(point.as_of_date)
            .and_modify(|existing| {
                if point.generated_at > existing.generated_at {
                    *existing = point.clone();
                }
            })
            .or_insert(point);
    }
    let points = latest_by_date.into_values().collect::<Vec<_>>();
    if points.is_empty() {
        bail!(
            "historical replay run {} exists but produced no points in {} -> {} for release {}",
            run.replay_run_id,
            from_date,
            to_date,
            release_id
        );
    }
    Ok((run, points))
}

fn sanitize_release_probability_slice_component(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => ch,
            _ => '_',
        })
        .collect()
}

fn write_release_probability_slice_report(
    output_dir: &PathBuf,
    export: &ReleaseProbabilitySliceExport,
) -> anyhow::Result<()> {
    fs::create_dir_all(output_dir)?;
    let stem = format!(
        "{}-{}-{}-{}-probability-slice",
        sanitize_release_probability_slice_component(&export.release_id),
        export.from_date,
        export.to_date,
        sanitize_release_probability_slice_component(&export.history_mode),
    );
    let json_path = output_dir.join(format!("{stem}.json"));
    let csv_path = output_dir.join(format!("{stem}.csv"));
    fs::write(&json_path, serde_json::to_string_pretty(export)?)?;
    fs::write(&csv_path, render_release_probability_slice_csv(export)?)?;
    println!("Release probability slice exported.");
    println!("  JSON {}", json_path.display());
    println!("  CSV  {}", csv_path.display());
    Ok(())
}

fn render_release_probability_slice_csv(
    export: &ReleaseProbabilitySliceExport,
) -> anyhow::Result<String> {
    let mut csv = String::from(
        "as_of_date,overall_score,structural_score,trigger_score,external_shock_score,posture,time_to_risk_bucket,actionability_prepare,actionability_hedge,actionability_defend,coverage_score,freshness_status,raw_p_5d,calibrated_p_5d,final_p_5d,overlay_delta_5d,monotonic_lift_5d,contributions_5d_json,raw_p_20d,calibrated_p_20d,final_p_20d,overlay_delta_20d,monotonic_lift_20d,contributions_20d_json,raw_p_60d,calibrated_p_60d,final_p_60d,overlay_delta_60d,monotonic_lift_60d,contributions_60d_json,posture_trigger_codes_json,posture_blocker_codes_json\n",
    );
    for row in &export.rows {
        let horizon_5d = release_probability_horizon(row, 5);
        let horizon_20d = release_probability_horizon(row, 20);
        let horizon_60d = release_probability_horizon(row, 60);
        let columns = [
            row.as_of_date.to_string(),
            format!("{:.6}", row.overall_score),
            format!("{:.6}", row.structural_score),
            format!("{:.6}", row.trigger_score),
            format!("{:.6}", row.external_shock_score),
            row.posture.clone(),
            row.time_to_risk_bucket.clone(),
            format!("{:.6}", row.actionability_prepare),
            format!("{:.6}", row.actionability_hedge),
            format!("{:.6}", row.actionability_defend),
            format!("{:.6}", row.coverage_score),
            row.freshness_status.clone(),
            format!("{:.6}", release_raw_probability(row, 5)),
            format!("{:.6}", release_calibrated_probability(row, 5)),
            format!("{:.6}", release_final_probability(row, 5)),
            format!("{:.6}", release_overlay_delta(row, 5)),
            format!("{:.6}", release_monotonic_lift(row, 5)),
            serde_json::to_string(&release_probability_contributions(horizon_5d))?,
            format!("{:.6}", release_raw_probability(row, 20)),
            format!("{:.6}", release_calibrated_probability(row, 20)),
            format!("{:.6}", release_final_probability(row, 20)),
            format!("{:.6}", release_overlay_delta(row, 20)),
            format!("{:.6}", release_monotonic_lift(row, 20)),
            serde_json::to_string(&release_probability_contributions(horizon_20d))?,
            format!("{:.6}", release_raw_probability(row, 60)),
            format!("{:.6}", release_calibrated_probability(row, 60)),
            format!("{:.6}", release_final_probability(row, 60)),
            format!("{:.6}", release_overlay_delta(row, 60)),
            format!("{:.6}", release_monotonic_lift(row, 60)),
            serde_json::to_string(&release_probability_contributions(horizon_60d))?,
            serde_json::to_string(&row.posture_trigger_codes)?,
            serde_json::to_string(&row.posture_blocker_codes)?,
        ];
        csv.push_str(
            &columns
                .into_iter()
                .map(|value| release_probability_csv_escape(&value))
                .collect::<Vec<_>>()
                .join(","),
        );
        csv.push('\n');
    }
    Ok(csv)
}

fn release_probability_csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn print_release_probability_slice_summary(export: &ReleaseProbabilitySliceExport) {
    let first_date = export
        .rows
        .first()
        .map(|row| row.as_of_date.to_string())
        .unwrap_or_else(|| "-".to_string());
    let last_date = export
        .rows
        .last()
        .map(|row| row.as_of_date.to_string())
        .unwrap_or_else(|| "-".to_string());
    println!(
        "Release probability slice release={} replay_run={} rows={} range={} -> {} history_mode={} history_limit={}",
        export.release_id,
        export.replay_run_id,
        export.row_count,
        first_date,
        last_date,
        export.history_mode,
        export.history_limit
    );
}

fn release_probability_horizon(
    row: &ReleaseProbabilitySlicePoint,
    horizon_days: u32,
) -> Option<&ProbabilityHorizonOverlayDiagnostics> {
    row.probability_diagnostics
        .horizon_overlays
        .iter()
        .find(|horizon| horizon.horizon_days == horizon_days)
}

fn release_probability_contributions(
    horizon: Option<&ProbabilityHorizonOverlayDiagnostics>,
) -> Vec<ProbabilityOverlayContribution> {
    horizon
        .map(|horizon| horizon.contributions.clone())
        .unwrap_or_default()
}

fn release_raw_probability(row: &ReleaseProbabilitySlicePoint, horizon_days: u32) -> f64 {
    release_probability_horizon(row, horizon_days)
        .map(|horizon| horizon.raw_probability)
        .unwrap_or_else(|| match horizon_days {
            5 => row.raw_p_5d,
            20 => row.raw_p_20d,
            60 => row.raw_p_60d,
            _ => 0.0,
        })
}

fn release_calibrated_probability(row: &ReleaseProbabilitySlicePoint, horizon_days: u32) -> f64 {
    release_probability_horizon(row, horizon_days)
        .map(|horizon| horizon.calibrated_probability)
        .unwrap_or_else(|| match horizon_days {
            5 => row.calibrated_p_5d,
            20 => row.calibrated_p_20d,
            60 => row.calibrated_p_60d,
            _ => 0.0,
        })
}

fn release_final_probability(row: &ReleaseProbabilitySlicePoint, horizon_days: u32) -> f64 {
    release_probability_horizon(row, horizon_days)
        .and_then(|horizon| horizon.runtime_final_probability)
        .or_else(|| {
            release_probability_horizon(row, horizon_days).map(|horizon| horizon.final_probability)
        })
        .unwrap_or_else(|| release_calibrated_probability(row, horizon_days))
}

fn release_overlay_delta(row: &ReleaseProbabilitySlicePoint, horizon_days: u32) -> f64 {
    release_probability_horizon(row, horizon_days)
        .map(|horizon| horizon.final_probability - horizon.calibrated_probability)
        .unwrap_or(0.0)
}

fn release_monotonic_lift(row: &ReleaseProbabilitySlicePoint, horizon_days: u32) -> f64 {
    release_probability_horizon(row, horizon_days)
        .map(|horizon| horizon.monotonic_lift)
        .unwrap_or(0.0)
}

fn read_release_probability_bundle(
    release: &ModelReleaseRecord,
) -> anyhow::Result<fc_domain::ProbabilityBundle> {
    let bundle_path = release
        .manifest
        .bundle_uri
        .strip_prefix("file://")
        .unwrap_or(&release.manifest.bundle_uri);
    crate::read_probability_bundle(std::path::Path::new(bundle_path))
}

fn score_release_formal_probability_slice_rows(
    bundle: &fc_domain::ProbabilityBundle,
    mut rows: Vec<fc_domain::FormalDatasetRowRecord>,
) -> Vec<ReleaseFormalProbabilitySlicePoint> {
    rows.sort_by(|left, right| left.as_of_date.cmp(&right.as_of_date));
    rows.into_iter()
        .map(|row| {
            let base_model_diagnostics = bundle
                .horizons
                .iter()
                .map(|horizon| {
                    let mut base_model =
                        fc_domain::score_logistic_probability_model_with_diagnostics(
                            &horizon.raw_model,
                            &row.features,
                        );
                    base_model.feature_contributions.sort_by(|left, right| {
                        right.contribution.abs().total_cmp(&left.contribution.abs())
                    });
                    ReleaseFormalProbabilityBaseModelDiagnostics {
                        horizon_days: horizon.horizon_days,
                        base_model,
                    }
                })
                .collect();
            let probability_diagnostics = ProbabilityDiagnostics {
                horizon_overlays: bundle
                    .horizons
                    .iter()
                    .map(|horizon| {
                        let score =
                            fc_domain::score_probability_horizon_bundle(horizon, &row.features);
                        ProbabilityHorizonOverlayDiagnostics {
                            horizon_days: horizon.horizon_days,
                            raw_probability: score.raw_probability,
                            calibrated_probability: score.calibrated_probability,
                            final_probability: score.final_probability,
                            runtime_final_probability: Some(score.final_probability),
                            monotonic_lift: 0.0,
                            configured_overlay_count: horizon.family_overlays.len() as u32,
                            contributions: score.overlay_contributions,
                            overlay_audits: Vec::new(),
                        }
                    })
                    .collect(),
            };
            ReleaseFormalProbabilitySlicePoint {
                as_of_date: row.as_of_date,
                split_name: row.split_name,
                primary_scenario_id: row.primary_scenario_id,
                scenario_family: row.scenario_family,
                regime_20d: row.regime_20d,
                regime_60d: row.regime_60d,
                prepare_episode_label: row.prepare_episode_label,
                hedge_episode_label: row.hedge_episode_label,
                defend_episode_label: row.defend_episode_label,
                primary_action_level: row.primary_action_level,
                coverage_score: row.coverage_score,
                probability_diagnostics,
                base_model_diagnostics,
            }
        })
        .collect()
}

fn release_formal_probability_base_model(
    row: &ReleaseFormalProbabilitySlicePoint,
    horizon_days: u32,
) -> Option<&ReleaseFormalProbabilityBaseModelDiagnostics> {
    row.base_model_diagnostics
        .iter()
        .find(|item| item.horizon_days == horizon_days)
}

fn release_formal_probability_horizon(
    row: &ReleaseFormalProbabilitySlicePoint,
    horizon_days: u32,
) -> Option<&ProbabilityHorizonOverlayDiagnostics> {
    row.probability_diagnostics
        .horizon_overlays
        .iter()
        .find(|item| item.horizon_days == horizon_days)
}

fn write_release_formal_probability_slice_report(
    output_dir: &PathBuf,
    export: &ReleaseFormalProbabilitySliceExport,
) -> anyhow::Result<()> {
    fs::create_dir_all(output_dir)?;
    let mut stem = format!(
        "{}-{}-{}-formal-probability-slice",
        sanitize_release_probability_slice_component(&export.release_id),
        export.from_date,
        export.to_date
    );
    if let Some(scenario_id) = export.scenario_id.as_deref() {
        stem.push('-');
        stem.push_str(&sanitize_release_probability_slice_component(scenario_id));
    }
    let json_path = output_dir.join(format!("{stem}.json"));
    let csv_path = output_dir.join(format!("{stem}.csv"));
    fs::write(&json_path, serde_json::to_string_pretty(export)?)?;
    fs::write(
        &csv_path,
        render_release_formal_probability_slice_csv(export)?,
    )?;
    println!("Release formal probability slice exported.");
    println!("  JSON {}", json_path.display());
    println!("  CSV  {}", csv_path.display());
    Ok(())
}

fn render_release_formal_probability_slice_csv(
    export: &ReleaseFormalProbabilitySliceExport,
) -> anyhow::Result<String> {
    let mut csv = String::from(
        "as_of_date,split_name,primary_scenario_id,scenario_family,regime_20d,regime_60d,prepare_episode_label,hedge_episode_label,defend_episode_label,primary_action_level,coverage_score,raw_p_5d,base_linear_5d,calibrated_p_5d,final_p_5d,overlay_delta_5d,base_contributions_5d_json,contributions_5d_json,raw_p_20d,base_linear_20d,calibrated_p_20d,final_p_20d,overlay_delta_20d,base_contributions_20d_json,contributions_20d_json,raw_p_60d,base_linear_60d,calibrated_p_60d,final_p_60d,overlay_delta_60d,base_contributions_60d_json,contributions_60d_json\n",
    );
    for row in &export.rows {
        let base_horizon_5d = release_formal_probability_base_model(row, 5)
            .with_context(|| "bundle scoring did not produce 5d base diagnostics")?;
        let base_horizon_20d = release_formal_probability_base_model(row, 20)
            .with_context(|| "bundle scoring did not produce 20d base diagnostics")?;
        let base_horizon_60d = release_formal_probability_base_model(row, 60)
            .with_context(|| "bundle scoring did not produce 60d base diagnostics")?;
        let horizon_5d = row
            .probability_diagnostics
            .horizon_overlays
            .iter()
            .find(|item| item.horizon_days == 5)
            .cloned()
            .with_context(|| "bundle scoring did not produce 5d horizon diagnostics")?;
        let horizon_20d = row
            .probability_diagnostics
            .horizon_overlays
            .iter()
            .find(|item| item.horizon_days == 20)
            .cloned()
            .with_context(|| "bundle scoring did not produce 20d horizon diagnostics")?;
        let horizon_60d = row
            .probability_diagnostics
            .horizon_overlays
            .iter()
            .find(|item| item.horizon_days == 60)
            .cloned()
            .with_context(|| "bundle scoring did not produce 60d horizon diagnostics")?;
        let columns = [
            row.as_of_date.to_string(),
            row.split_name.clone(),
            row.primary_scenario_id.clone().unwrap_or_default(),
            row.scenario_family.clone().unwrap_or_default(),
            row.regime_20d.clone(),
            row.regime_60d.clone(),
            row.prepare_episode_label.to_string(),
            row.hedge_episode_label.to_string(),
            row.defend_episode_label.to_string(),
            row.primary_action_level.clone().unwrap_or_default(),
            format!("{:.6}", row.coverage_score),
            format!("{:.6}", horizon_5d.raw_probability),
            format!("{:.6}", base_horizon_5d.base_model.linear_score),
            format!("{:.6}", horizon_5d.calibrated_probability),
            format!("{:.6}", horizon_5d.final_probability),
            format!(
                "{:.6}",
                horizon_5d.final_probability - horizon_5d.calibrated_probability
            ),
            serde_json::to_string(&base_horizon_5d.base_model.feature_contributions)?,
            serde_json::to_string(&horizon_5d.contributions)?,
            format!("{:.6}", horizon_20d.raw_probability),
            format!("{:.6}", base_horizon_20d.base_model.linear_score),
            format!("{:.6}", horizon_20d.calibrated_probability),
            format!("{:.6}", horizon_20d.final_probability),
            format!(
                "{:.6}",
                horizon_20d.final_probability - horizon_20d.calibrated_probability
            ),
            serde_json::to_string(&base_horizon_20d.base_model.feature_contributions)?,
            serde_json::to_string(&horizon_20d.contributions)?,
            format!("{:.6}", horizon_60d.raw_probability),
            format!("{:.6}", base_horizon_60d.base_model.linear_score),
            format!("{:.6}", horizon_60d.calibrated_probability),
            format!("{:.6}", horizon_60d.final_probability),
            format!(
                "{:.6}",
                horizon_60d.final_probability - horizon_60d.calibrated_probability
            ),
            serde_json::to_string(&base_horizon_60d.base_model.feature_contributions)?,
            serde_json::to_string(&horizon_60d.contributions)?,
        ];
        csv.push_str(
            &columns
                .into_iter()
                .map(|value| release_probability_csv_escape(&value))
                .collect::<Vec<_>>()
                .join(","),
        );
        csv.push('\n');
    }
    Ok(csv)
}

fn print_release_formal_probability_slice_summary(export: &ReleaseFormalProbabilitySliceExport) {
    let first_date = export
        .rows
        .first()
        .map(|row| row.as_of_date.to_string())
        .unwrap_or_else(|| "-".to_string());
    let last_date = export
        .rows
        .last()
        .map(|row| row.as_of_date.to_string())
        .unwrap_or_else(|| "-".to_string());
    println!(
        "Release formal probability slice release={} dataset_key={} rows={} range={} -> {} scenario={}",
        export.release_id,
        export.dataset_key,
        export.row_count,
        first_date,
        last_date,
        export.scenario_id.as_deref().unwrap_or("all"),
    );
}

fn build_release_formal_probability_compare_export(
    input: ReleaseFormalProbabilityCompareBuildInput<'_>,
) -> anyhow::Result<ReleaseFormalProbabilityCompareExport> {
    let ReleaseFormalProbabilityCompareBuildInput {
        market_scope,
        dataset_key,
        scenario_id,
        from_date,
        to_date,
        baseline_release_id,
        candidate_release_id,
        baseline_bundle,
        candidate_bundle,
        baseline_rows,
        candidate_rows,
    } = input;
    let baseline_thresholds = release_formal_probability_threshold_summaries(baseline_bundle);
    let candidate_thresholds = release_formal_probability_threshold_summaries(candidate_bundle);
    let baseline_threshold_20d = release_formal_probability_threshold(baseline_bundle, 20);
    let candidate_threshold_20d = release_formal_probability_threshold(candidate_bundle, 20);
    let baseline_threshold_60d = release_formal_probability_threshold(baseline_bundle, 60);
    let candidate_threshold_60d = release_formal_probability_threshold(candidate_bundle, 60);
    let candidate_by_date = candidate_rows
        .into_iter()
        .map(|row| (row.as_of_date, row))
        .collect::<BTreeMap<_, _>>();
    let mut rows = Vec::new();
    let mut baseline_hit_count_20d = 0_usize;
    let mut candidate_hit_count_20d = 0_usize;
    let mut baseline_hit_count_60d = 0_usize;
    let mut candidate_hit_count_60d = 0_usize;
    let mut baseline_max_p_20d = f64::NEG_INFINITY;
    let mut baseline_max_p_20d_date = None;
    let mut candidate_max_p_20d = f64::NEG_INFINITY;
    let mut candidate_max_p_20d_date = None;
    let mut baseline_max_p_60d = f64::NEG_INFINITY;
    let mut baseline_max_p_60d_date = None;
    let mut candidate_max_p_60d = f64::NEG_INFINITY;
    let mut candidate_max_p_60d_date = None;

    for baseline_row in baseline_rows {
        let Some(candidate_row) = candidate_by_date.get(&baseline_row.as_of_date) else {
            continue;
        };
        let baseline_horizon_20d = release_formal_probability_horizon(&baseline_row, 20)
            .with_context(|| "baseline slice missing 20d diagnostics")?;
        let candidate_horizon_20d = release_formal_probability_horizon(candidate_row, 20)
            .with_context(|| "candidate slice missing 20d diagnostics")?;
        let baseline_horizon_60d = release_formal_probability_horizon(&baseline_row, 60)
            .with_context(|| "baseline slice missing 60d diagnostics")?;
        let candidate_horizon_60d = release_formal_probability_horizon(candidate_row, 60)
            .with_context(|| "candidate slice missing 60d diagnostics")?;
        let baseline_base_20d = release_formal_probability_base_model(&baseline_row, 20)
            .with_context(|| "baseline slice missing 20d base diagnostics")?;
        let candidate_base_20d = release_formal_probability_base_model(candidate_row, 20)
            .with_context(|| "candidate slice missing 20d base diagnostics")?;
        let baseline_base_60d = release_formal_probability_base_model(&baseline_row, 60)
            .with_context(|| "baseline slice missing 60d base diagnostics")?;
        let candidate_base_60d = release_formal_probability_base_model(candidate_row, 60)
            .with_context(|| "candidate slice missing 60d base diagnostics")?;

        let baseline_hit_20d = baseline_threshold_20d
            .map(|threshold| baseline_horizon_20d.final_probability >= threshold)
            .unwrap_or(false);
        let candidate_hit_20d = candidate_threshold_20d
            .map(|threshold| candidate_horizon_20d.final_probability >= threshold)
            .unwrap_or(false);
        let baseline_hit_60d = baseline_threshold_60d
            .map(|threshold| baseline_horizon_60d.final_probability >= threshold)
            .unwrap_or(false);
        let candidate_hit_60d = candidate_threshold_60d
            .map(|threshold| candidate_horizon_60d.final_probability >= threshold)
            .unwrap_or(false);

        baseline_hit_count_20d += usize::from(baseline_hit_20d);
        candidate_hit_count_20d += usize::from(candidate_hit_20d);
        baseline_hit_count_60d += usize::from(baseline_hit_60d);
        candidate_hit_count_60d += usize::from(candidate_hit_60d);

        if baseline_horizon_20d.final_probability > baseline_max_p_20d {
            baseline_max_p_20d = baseline_horizon_20d.final_probability;
            baseline_max_p_20d_date = Some(baseline_row.as_of_date);
        }
        if candidate_horizon_20d.final_probability > candidate_max_p_20d {
            candidate_max_p_20d = candidate_horizon_20d.final_probability;
            candidate_max_p_20d_date = Some(candidate_row.as_of_date);
        }
        if baseline_horizon_60d.final_probability > baseline_max_p_60d {
            baseline_max_p_60d = baseline_horizon_60d.final_probability;
            baseline_max_p_60d_date = Some(baseline_row.as_of_date);
        }
        if candidate_horizon_60d.final_probability > candidate_max_p_60d {
            candidate_max_p_60d = candidate_horizon_60d.final_probability;
            candidate_max_p_60d_date = Some(candidate_row.as_of_date);
        }

        rows.push(ReleaseFormalProbabilityComparePoint {
            as_of_date: baseline_row.as_of_date,
            split_name: baseline_row.split_name.clone(),
            primary_scenario_id: baseline_row.primary_scenario_id.clone(),
            scenario_family: baseline_row.scenario_family.clone(),
            regime_20d: baseline_row.regime_20d.clone(),
            regime_60d: baseline_row.regime_60d.clone(),
            prepare_episode_label: baseline_row.prepare_episode_label,
            hedge_episode_label: baseline_row.hedge_episode_label,
            defend_episode_label: baseline_row.defend_episode_label,
            primary_action_level: baseline_row.primary_action_level.clone(),
            coverage_score: baseline_row.coverage_score,
            baseline_raw_p_20d: baseline_horizon_20d.raw_probability,
            candidate_raw_p_20d: candidate_horizon_20d.raw_probability,
            baseline_base_linear_20d: baseline_base_20d.base_model.linear_score,
            candidate_base_linear_20d: candidate_base_20d.base_model.linear_score,
            baseline_final_p_20d: baseline_horizon_20d.final_probability,
            candidate_final_p_20d: candidate_horizon_20d.final_probability,
            delta_final_p_20d: candidate_horizon_20d.final_probability
                - baseline_horizon_20d.final_probability,
            baseline_hit_20d,
            candidate_hit_20d,
            baseline_raw_p_60d: baseline_horizon_60d.raw_probability,
            candidate_raw_p_60d: candidate_horizon_60d.raw_probability,
            baseline_base_linear_60d: baseline_base_60d.base_model.linear_score,
            candidate_base_linear_60d: candidate_base_60d.base_model.linear_score,
            baseline_final_p_60d: baseline_horizon_60d.final_probability,
            candidate_final_p_60d: candidate_horizon_60d.final_probability,
            delta_final_p_60d: candidate_horizon_60d.final_probability
                - baseline_horizon_60d.final_probability,
            baseline_hit_60d,
            candidate_hit_60d,
            top_feature_deltas_20d: release_formal_probability_feature_deltas(
                &baseline_base_20d.base_model,
                &candidate_base_20d.base_model,
                8,
            ),
            top_feature_deltas_60d: release_formal_probability_feature_deltas(
                &baseline_base_60d.base_model,
                &candidate_base_60d.base_model,
                8,
            ),
        });
    }

    if rows.is_empty() {
        bail!(
            "no overlapping rows found between baseline {baseline_release_id} and candidate {candidate_release_id} in the selected window"
        );
    }

    let overall_window = build_release_formal_probability_window_aggregate_summary(&rows, |_| true);
    let hedge_window = build_release_formal_probability_window_aggregate_summary(&rows, |row| {
        row.hedge_episode_label == 1
    });
    let positive_window_20d =
        build_release_formal_probability_window_aggregate_summary(&rows, |row| {
            row.regime_20d == "positive_window"
        });

    Ok(ReleaseFormalProbabilityCompareExport {
        exported_at: Utc::now().to_rfc3339(),
        market_scope: market_scope.to_string(),
        baseline_release_id: baseline_release_id.to_string(),
        candidate_release_id: candidate_release_id.to_string(),
        dataset_key: dataset_key.to_string(),
        scenario_id,
        from_date,
        to_date,
        row_count: rows.len(),
        baseline_thresholds,
        candidate_thresholds,
        summary: ReleaseFormalProbabilityCompareSummary {
            baseline_hit_count_20d,
            candidate_hit_count_20d,
            baseline_hit_count_60d,
            candidate_hit_count_60d,
            baseline_max_p_20d: baseline_max_p_20d.max(0.0),
            baseline_max_p_20d_date,
            candidate_max_p_20d: candidate_max_p_20d.max(0.0),
            candidate_max_p_20d_date,
            baseline_max_p_60d: baseline_max_p_60d.max(0.0),
            baseline_max_p_60d_date,
            candidate_max_p_60d: candidate_max_p_60d.max(0.0),
            candidate_max_p_60d_date,
            overall_window,
            hedge_window,
            positive_window_20d,
        },
        rows,
    })
}

fn release_formal_probability_threshold_summaries(
    bundle: &fc_domain::ProbabilityBundle,
) -> Vec<ReleaseFormalProbabilityThresholdSummary> {
    bundle
        .horizons
        .iter()
        .map(|horizon| ReleaseFormalProbabilityThresholdSummary {
            horizon_days: horizon.horizon_days,
            decision_threshold: horizon.decision_threshold,
            overlay_count: horizon.family_overlays.len(),
        })
        .collect()
}

fn release_formal_probability_threshold(
    bundle: &fc_domain::ProbabilityBundle,
    horizon_days: u32,
) -> Option<f64> {
    bundle
        .horizons
        .iter()
        .find(|horizon| horizon.horizon_days == horizon_days)
        .and_then(|horizon| horizon.decision_threshold)
}

fn release_formal_probability_feature_deltas(
    baseline: &LogisticProbabilityModelScoreDiagnostics,
    candidate: &LogisticProbabilityModelScoreDiagnostics,
    limit: usize,
) -> Vec<ReleaseFormalProbabilityFeatureDelta> {
    let baseline_by_name = baseline
        .feature_contributions
        .iter()
        .map(|item| (item.name.clone(), item.clone()))
        .collect::<BTreeMap<_, _>>();
    let candidate_by_name = candidate
        .feature_contributions
        .iter()
        .map(|item| (item.name.clone(), item.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut names = baseline_by_name.keys().cloned().collect::<BTreeSet<_>>();
    names.extend(candidate_by_name.keys().cloned());
    let mut deltas = names
        .into_iter()
        .map(|name| {
            let baseline_item = baseline_by_name.get(&name);
            let candidate_item = candidate_by_name.get(&name);
            let baseline_raw_value = baseline_item.map(|item| item.raw_value).unwrap_or(0.0);
            let candidate_raw_value = candidate_item.map(|item| item.raw_value).unwrap_or(0.0);
            let baseline_normalized_value = baseline_item
                .map(|item| item.normalized_value)
                .unwrap_or(0.0);
            let candidate_normalized_value = candidate_item
                .map(|item| item.normalized_value)
                .unwrap_or(0.0);
            let baseline_weight = baseline_item.map(|item| item.weight).unwrap_or(0.0);
            let candidate_weight = candidate_item.map(|item| item.weight).unwrap_or(0.0);
            let baseline_contribution = baseline_item.map(|item| item.contribution).unwrap_or(0.0);
            let candidate_contribution =
                candidate_item.map(|item| item.contribution).unwrap_or(0.0);
            ReleaseFormalProbabilityFeatureDelta {
                name,
                baseline_raw_value,
                candidate_raw_value,
                baseline_normalized_value,
                candidate_normalized_value,
                baseline_weight,
                candidate_weight,
                baseline_contribution,
                candidate_contribution,
                delta_contribution: candidate_contribution - baseline_contribution,
            }
        })
        .filter(|item| item.delta_contribution.abs() >= 1e-9)
        .collect::<Vec<_>>();
    deltas.sort_by(|left, right| {
        right
            .delta_contribution
            .abs()
            .total_cmp(&left.delta_contribution.abs())
    });
    deltas.truncate(limit);
    deltas
}

fn build_release_formal_probability_window_aggregate_summary<F>(
    rows: &[ReleaseFormalProbabilityComparePoint],
    filter: F,
) -> ReleaseFormalProbabilityWindowAggregateSummary
where
    F: Fn(&ReleaseFormalProbabilityComparePoint) -> bool,
{
    let selected = rows.iter().filter(|row| filter(row)).collect::<Vec<_>>();
    if selected.is_empty() {
        return ReleaseFormalProbabilityWindowAggregateSummary {
            row_count: 0,
            avg_delta_p_20d: 0.0,
            avg_abs_delta_p_20d: 0.0,
            avg_delta_p_60d: 0.0,
            avg_abs_delta_p_60d: 0.0,
            baseline_hit_rate_20d: 0.0,
            candidate_hit_rate_20d: 0.0,
            baseline_hit_rate_60d: 0.0,
            candidate_hit_rate_60d: 0.0,
            top_feature_deltas_20d: Vec::new(),
            top_feature_deltas_60d: Vec::new(),
        };
    }

    let row_count = selected.len();
    let avg_delta_p_20d = selected
        .iter()
        .map(|row| row.delta_final_p_20d)
        .sum::<f64>()
        / row_count as f64;
    let avg_abs_delta_p_20d = selected
        .iter()
        .map(|row| row.delta_final_p_20d.abs())
        .sum::<f64>()
        / row_count as f64;
    let avg_delta_p_60d = selected
        .iter()
        .map(|row| row.delta_final_p_60d)
        .sum::<f64>()
        / row_count as f64;
    let avg_abs_delta_p_60d = selected
        .iter()
        .map(|row| row.delta_final_p_60d.abs())
        .sum::<f64>()
        / row_count as f64;
    let baseline_hit_rate_20d =
        selected.iter().filter(|row| row.baseline_hit_20d).count() as f64 / row_count as f64;
    let candidate_hit_rate_20d =
        selected.iter().filter(|row| row.candidate_hit_20d).count() as f64 / row_count as f64;
    let baseline_hit_rate_60d =
        selected.iter().filter(|row| row.baseline_hit_60d).count() as f64 / row_count as f64;
    let candidate_hit_rate_60d =
        selected.iter().filter(|row| row.candidate_hit_60d).count() as f64 / row_count as f64;

    ReleaseFormalProbabilityWindowAggregateSummary {
        row_count,
        avg_delta_p_20d,
        avg_abs_delta_p_20d,
        avg_delta_p_60d,
        avg_abs_delta_p_60d,
        baseline_hit_rate_20d,
        candidate_hit_rate_20d,
        baseline_hit_rate_60d,
        candidate_hit_rate_60d,
        top_feature_deltas_20d: aggregate_release_formal_probability_feature_deltas(
            selected
                .iter()
                .map(|row| row.top_feature_deltas_20d.as_slice()),
            10,
        ),
        top_feature_deltas_60d: aggregate_release_formal_probability_feature_deltas(
            selected
                .iter()
                .map(|row| row.top_feature_deltas_60d.as_slice()),
            10,
        ),
    }
}

fn aggregate_release_formal_probability_feature_deltas<'a, I>(
    feature_sets: I,
    limit: usize,
) -> Vec<ReleaseFormalProbabilityFeatureDeltaAggregate>
where
    I: IntoIterator<Item = &'a [ReleaseFormalProbabilityFeatureDelta]>,
{
    let mut aggregates = BTreeMap::<String, (f64, f64, usize)>::new();
    for feature_set in feature_sets {
        for item in feature_set {
            let entry = aggregates
                .entry(item.name.clone())
                .or_insert((0.0_f64, 0.0_f64, 0_usize));
            entry.0 += item.delta_contribution;
            entry.1 += item.delta_contribution.abs();
            entry.2 += 1;
        }
    }
    let mut rows = aggregates
        .into_iter()
        .map(
            |(name, (sum_delta_contribution, abs_sum_delta_contribution, count))| {
                ReleaseFormalProbabilityFeatureDeltaAggregate {
                    name,
                    sum_delta_contribution,
                    abs_sum_delta_contribution,
                    mean_delta_contribution: sum_delta_contribution / count as f64,
                    count,
                }
            },
        )
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .abs_sum_delta_contribution
            .total_cmp(&left.abs_sum_delta_contribution)
    });
    rows.truncate(limit);
    rows
}

fn write_release_formal_probability_compare_report(
    output_dir: &PathBuf,
    export: &ReleaseFormalProbabilityCompareExport,
) -> anyhow::Result<()> {
    fs::create_dir_all(output_dir)?;
    let mut stem = format!(
        "{}-vs-{}-{}-{}-formal-probability-compare",
        sanitize_release_probability_slice_component(&export.baseline_release_id),
        sanitize_release_probability_slice_component(&export.candidate_release_id),
        export.from_date,
        export.to_date
    );
    if let Some(scenario_id) = export.scenario_id.as_deref() {
        stem.push('-');
        stem.push_str(&sanitize_release_probability_slice_component(scenario_id));
    }
    let json_path = output_dir.join(format!("{stem}.json"));
    let csv_path = output_dir.join(format!("{stem}.csv"));
    fs::write(&json_path, serde_json::to_string_pretty(export)?)?;
    fs::write(
        &csv_path,
        render_release_formal_probability_compare_csv(export)?,
    )?;
    println!("Release formal probability compare exported.");
    println!("  JSON {}", json_path.display());
    println!("  CSV  {}", csv_path.display());
    Ok(())
}

fn render_release_formal_probability_compare_csv(
    export: &ReleaseFormalProbabilityCompareExport,
) -> anyhow::Result<String> {
    let mut csv = String::from(
        "as_of_date,split_name,primary_scenario_id,scenario_family,regime_20d,regime_60d,prepare_episode_label,hedge_episode_label,defend_episode_label,primary_action_level,coverage_score,baseline_raw_p_20d,candidate_raw_p_20d,baseline_base_linear_20d,candidate_base_linear_20d,baseline_final_p_20d,candidate_final_p_20d,delta_final_p_20d,baseline_hit_20d,candidate_hit_20d,top_feature_deltas_20d_json,baseline_raw_p_60d,candidate_raw_p_60d,baseline_base_linear_60d,candidate_base_linear_60d,baseline_final_p_60d,candidate_final_p_60d,delta_final_p_60d,baseline_hit_60d,candidate_hit_60d,top_feature_deltas_60d_json\n",
    );
    for row in &export.rows {
        let columns = [
            row.as_of_date.to_string(),
            row.split_name.clone(),
            row.primary_scenario_id.clone().unwrap_or_default(),
            row.scenario_family.clone().unwrap_or_default(),
            row.regime_20d.clone(),
            row.regime_60d.clone(),
            row.prepare_episode_label.to_string(),
            row.hedge_episode_label.to_string(),
            row.defend_episode_label.to_string(),
            row.primary_action_level.clone().unwrap_or_default(),
            format!("{:.6}", row.coverage_score),
            format!("{:.6}", row.baseline_raw_p_20d),
            format!("{:.6}", row.candidate_raw_p_20d),
            format!("{:.6}", row.baseline_base_linear_20d),
            format!("{:.6}", row.candidate_base_linear_20d),
            format!("{:.6}", row.baseline_final_p_20d),
            format!("{:.6}", row.candidate_final_p_20d),
            format!("{:.6}", row.delta_final_p_20d),
            row.baseline_hit_20d.to_string(),
            row.candidate_hit_20d.to_string(),
            serde_json::to_string(&row.top_feature_deltas_20d)?,
            format!("{:.6}", row.baseline_raw_p_60d),
            format!("{:.6}", row.candidate_raw_p_60d),
            format!("{:.6}", row.baseline_base_linear_60d),
            format!("{:.6}", row.candidate_base_linear_60d),
            format!("{:.6}", row.baseline_final_p_60d),
            format!("{:.6}", row.candidate_final_p_60d),
            format!("{:.6}", row.delta_final_p_60d),
            row.baseline_hit_60d.to_string(),
            row.candidate_hit_60d.to_string(),
            serde_json::to_string(&row.top_feature_deltas_60d)?,
        ];
        csv.push_str(
            &columns
                .into_iter()
                .map(|value| release_probability_csv_escape(&value))
                .collect::<Vec<_>>()
                .join(","),
        );
        csv.push('\n');
    }
    Ok(csv)
}

fn print_release_formal_probability_compare_summary(
    export: &ReleaseFormalProbabilityCompareExport,
) {
    let first_date = export
        .rows
        .first()
        .map(|row| row.as_of_date.to_string())
        .unwrap_or_else(|| "-".to_string());
    let last_date = export
        .rows
        .last()
        .map(|row| row.as_of_date.to_string())
        .unwrap_or_else(|| "-".to_string());
    println!(
        "Release formal probability compare baseline={} candidate={} rows={} range={} -> {} scenario={}",
        export.baseline_release_id,
        export.candidate_release_id,
        export.row_count,
        first_date,
        last_date,
        export.scenario_id.as_deref().unwrap_or("all"),
    );
    println!(
        "  20d hits baseline={} candidate={} max baseline={:.3} ({}) candidate={:.3} ({})",
        export.summary.baseline_hit_count_20d,
        export.summary.candidate_hit_count_20d,
        export.summary.baseline_max_p_20d,
        export
            .summary
            .baseline_max_p_20d_date
            .map(|date| date.to_string())
            .unwrap_or_else(|| "-".to_string()),
        export.summary.candidate_max_p_20d,
        export
            .summary
            .candidate_max_p_20d_date
            .map(|date| date.to_string())
            .unwrap_or_else(|| "-".to_string()),
    );
    println!(
        "  60d hits baseline={} candidate={} max baseline={:.3} ({}) candidate={:.3} ({})",
        export.summary.baseline_hit_count_60d,
        export.summary.candidate_hit_count_60d,
        export.summary.baseline_max_p_60d,
        export
            .summary
            .baseline_max_p_60d_date
            .map(|date| date.to_string())
            .unwrap_or_else(|| "-".to_string()),
        export.summary.candidate_max_p_60d,
        export
            .summary
            .candidate_max_p_60d_date
            .map(|date| date.to_string())
            .unwrap_or_else(|| "-".to_string()),
    );
    println!(
        "  avg delta 20d overall={:.3} hedge={:.3} positive_window={:.3}",
        export.summary.overall_window.avg_delta_p_20d,
        export.summary.hedge_window.avg_delta_p_20d,
        export.summary.positive_window_20d.avg_delta_p_20d,
    );
    println!(
        "  20d hit rate positive_window baseline={:.3} candidate={:.3}",
        export.summary.positive_window_20d.baseline_hit_rate_20d,
        export.summary.positive_window_20d.candidate_hit_rate_20d,
    );
    let top_overall_features = export
        .summary
        .overall_window
        .top_feature_deltas_20d
        .iter()
        .take(3)
        .map(|item| format!("{}:{:.2}", item.name, item.sum_delta_contribution))
        .collect::<Vec<_>>()
        .join(", ");
    let top_hedge_features = export
        .summary
        .hedge_window
        .top_feature_deltas_20d
        .iter()
        .take(3)
        .map(|item| format!("{}:{:.2}", item.name, item.sum_delta_contribution))
        .collect::<Vec<_>>()
        .join(", ");
    println!("  top 20d feature deltas overall={top_overall_features}");
    println!("  top 20d feature deltas hedge={top_hedge_features}");
}

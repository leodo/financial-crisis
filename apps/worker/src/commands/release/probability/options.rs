use std::path::PathBuf;

use anyhow::{bail, Context};
use chrono::NaiveDate;

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

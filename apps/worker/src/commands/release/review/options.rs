use std::path::PathBuf;

use anyhow::{bail, Context};

#[derive(Debug, Clone)]
pub(crate) struct ReleaseReviewOptions {
    pub(crate) candidate_release_id: String,
    pub(crate) baseline_release_id: Option<String>,
    pub(crate) market_scope: Option<String>,
    pub(crate) api_reload_url: String,
    pub(crate) output_dir: PathBuf,
    pub(crate) history_mode: crate::ApiReloadHistoryMode,
    pub(crate) history_limit: usize,
    pub(crate) updated_by: String,
}

impl ReleaseReviewOptions {
    pub(crate) fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut candidate_release_id = None;
        let mut baseline_release_id = None;
        let mut market_scope = None;
        let mut api_reload_url = crate::DEFAULT_API_RELOAD_URL.to_string();
        let mut output_dir = PathBuf::from(crate::DEFAULT_RELEASE_REVIEW_OUTPUT_DIR);
        let mut history_mode = crate::ApiReloadHistoryMode::StrictRebuild;
        let mut history_limit = 20_000_usize;
        let mut updated_by = "fc-worker-review".to_string();
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--candidate-release-id" => {
                    index += 1;
                    candidate_release_id = Some(
                        args.get(index)
                            .with_context(|| "--candidate-release-id requires a value")?
                            .clone(),
                    );
                }
                "--baseline-release-id" => {
                    index += 1;
                    baseline_release_id = Some(
                        args.get(index)
                            .with_context(|| "--baseline-release-id requires a value")?
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
                "--updated-by" => {
                    index += 1;
                    updated_by = args
                        .get(index)
                        .with_context(|| "--updated-by requires a value")?
                        .clone();
                }
                other => bail!("unknown release review option: {other}"),
            }
            index += 1;
        }
        Ok(Self {
            candidate_release_id: candidate_release_id
                .with_context(|| "--candidate-release-id is required")?,
            baseline_release_id,
            market_scope,
            api_reload_url,
            output_dir,
            history_mode,
            history_limit,
            updated_by,
        })
    }
}

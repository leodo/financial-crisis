use std::{collections::BTreeMap, path::PathBuf};

use anyhow::{bail, Context};
use chrono::Utc;
use fc_domain::ModelReleaseRecord;

#[derive(Debug, Clone)]
pub(crate) struct ReleasePublishOptions {
    pub(crate) manifest_path: PathBuf,
    pub(crate) activate: bool,
    pub(crate) reload_api: bool,
    pub(crate) api_reload_url: String,
    pub(crate) skip_operational_guard: bool,
    pub(crate) updated_by: String,
}

impl ReleasePublishOptions {
    pub(crate) fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut manifest_path = None;
        let mut activate = false;
        let mut reload_api = false;
        let mut api_reload_url = crate::DEFAULT_API_RELOAD_URL.to_string();
        let mut skip_operational_guard = false;
        let mut updated_by = "fc-worker".to_string();
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--manifest" => {
                    index += 1;
                    manifest_path = Some(PathBuf::from(
                        args.get(index)
                            .with_context(|| "--manifest requires a file path")?,
                    ));
                }
                "--activate" => activate = true,
                "--reload-api" => reload_api = true,
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
                other => bail!("unknown release publish option: {other}"),
            }
            index += 1;
        }
        Ok(Self {
            manifest_path: manifest_path.with_context(|| "--manifest is required")?,
            activate,
            reload_api,
            api_reload_url,
            skip_operational_guard,
            updated_by,
        })
    }
}

#[derive(Debug, Clone)]
struct ReleaseListOptions {
    market_scope: Option<String>,
}

impl ReleaseListOptions {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut market_scope = None;
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
                other => bail!("unknown release list option: {other}"),
            }
            index += 1;
        }
        Ok(Self { market_scope })
    }
}

#[derive(Debug, Clone)]
struct ReleaseShowOptions {
    release_id: String,
}

impl ReleaseShowOptions {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut release_id = None;
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
                other => bail!("unknown release show option: {other}"),
            }
            index += 1;
        }
        Ok(Self {
            release_id: release_id.with_context(|| "--release-id is required")?,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ReleaseSwitchOptions {
    pub(crate) release_id: String,
    pub(crate) market_scope: Option<String>,
    pub(crate) reload_api: bool,
    pub(crate) api_reload_url: String,
    pub(crate) skip_operational_guard: bool,
    pub(crate) updated_by: String,
}

impl ReleaseSwitchOptions {
    pub(crate) fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut release_id = None;
        let mut market_scope = None;
        let mut reload_api = false;
        let mut api_reload_url = crate::DEFAULT_API_RELOAD_URL.to_string();
        let mut skip_operational_guard = false;
        let mut updated_by = "fc-worker".to_string();
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--release-id" | "--to-release-id" => {
                    index += 1;
                    release_id = Some(
                        args.get(index)
                            .with_context(|| "--release-id/--to-release-id requires a value")?
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
                "--reload-api" => reload_api = true,
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
                other => bail!("unknown release switch option: {other}"),
            }
            index += 1;
        }
        Ok(Self {
            release_id: release_id.with_context(|| "--release-id/--to-release-id is required")?,
            market_scope,
            reload_api,
            api_reload_url,
            skip_operational_guard,
            updated_by,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ReleaseReviewOptions {
    pub(crate) candidate_release_id: String,
    pub(crate) baseline_release_id: Option<String>,
    pub(crate) market_scope: Option<String>,
    pub(crate) api_reload_url: String,
    pub(crate) output_dir: PathBuf,
    pub(crate) updated_by: String,
}

impl ReleaseReviewOptions {
    pub(crate) fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut candidate_release_id = None;
        let mut baseline_release_id = None;
        let mut market_scope = None;
        let mut api_reload_url = crate::DEFAULT_API_RELOAD_URL.to_string();
        let mut output_dir = PathBuf::from(crate::DEFAULT_RELEASE_REVIEW_OUTPUT_DIR);
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
            updated_by,
        })
    }
}

pub(crate) async fn research_release_publish(args: &[String]) -> anyhow::Result<()> {
    let options = ReleasePublishOptions::parse(args)?;
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;
    let manifest = crate::read_release_manifest(&options.manifest_path)?;
    let record = ModelReleaseRecord {
        manifest,
        created_at: Utc::now(),
        activated_at: None,
        retired_at: None,
    };
    store.upsert_model_release(&record).await?;
    println!(
        "Saved release {} for market scope {}.",
        record.manifest.release_id, record.manifest.market_scope
    );
    println!("  Bundle     {}", record.manifest.bundle_uri);
    println!("  Prob mode  {}", record.manifest.probability_mode);
    println!("  PIT mode   {}", record.manifest.point_in_time_mode);

    if options.activate {
        activate_release_with_runtime_guard(
            &store,
            &record.manifest.market_scope,
            &record.manifest.release_id,
            options.reload_api,
            &options.api_reload_url,
            options.skip_operational_guard,
            &options.updated_by,
        )
        .await?;
    }

    Ok(())
}

pub(crate) async fn research_release_list(args: &[String]) -> anyhow::Result<()> {
    let options = ReleaseListOptions::parse(args)?;
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;
    let releases = store
        .list_model_releases(options.market_scope.as_deref())
        .await?;
    if releases.is_empty() {
        println!("No model releases found.");
        return Ok(());
    }
    println!(
        "{:<32} {:<18} {:<12} {:<12} {:<16} {:<24}",
        "release_id", "market_scope", "status", "serving", "prob_mode", "created_at"
    );
    for release in releases {
        println!(
            "{:<32} {:<18} {:<12} {:<12} {:<16} {:<24}",
            crate::truncate_text(&release.manifest.release_id, 32),
            crate::truncate_text(&release.manifest.market_scope, 18),
            crate::truncate_text(&release.manifest.status, 12),
            crate::truncate_text(&release.manifest.serving_status, 12),
            crate::truncate_text(&release.manifest.probability_mode, 16),
            release.created_at.to_rfc3339()
        );
    }
    Ok(())
}

pub(crate) async fn research_release_show(args: &[String]) -> anyhow::Result<()> {
    let options = ReleaseShowOptions::parse(args)?;
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;
    let release = store
        .load_model_release(&options.release_id)
        .await?
        .with_context(|| format!("release {} not found", options.release_id))?;
    println!("{}", serde_json::to_string_pretty(&release)?);
    Ok(())
}

pub(crate) async fn research_release_activate(args: &[String]) -> anyhow::Result<()> {
    let options = ReleaseSwitchOptions::parse(args)?;
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;
    let market_scope =
        resolve_release_market_scope(&store, &options.release_id, options.market_scope.as_deref())
            .await?;
    activate_release_with_runtime_guard(
        &store,
        &market_scope,
        &options.release_id,
        options.reload_api,
        &options.api_reload_url,
        options.skip_operational_guard,
        &options.updated_by,
    )
    .await?;
    Ok(())
}

pub(crate) async fn research_release_rollback(args: &[String]) -> anyhow::Result<()> {
    let options = ReleaseSwitchOptions::parse(args)?;
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;
    let market_scope =
        resolve_release_market_scope(&store, &options.release_id, options.market_scope.as_deref())
            .await?;
    let activated = store
        .rollback_model_release(&market_scope, &options.release_id, &options.updated_by)
        .await?;
    println!(
        "Rolled back {} to release {}.",
        market_scope, activated.manifest.release_id
    );
    println!(
        "  mode={} serving={} pit={}",
        activated.manifest.probability_mode,
        activated.manifest.serving_status,
        activated.manifest.point_in_time_mode
    );
    if options.reload_api {
        crate::reload_api_runtime(&options.api_reload_url).await?;
        println!("Reloaded API runtime via {}.", options.api_reload_url);
    }
    Ok(())
}

pub(crate) async fn research_release_review(args: &[String]) -> anyhow::Result<()> {
    let options = ReleaseReviewOptions::parse(args)?;
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;

    let candidate_release = store
        .load_model_release(&options.candidate_release_id)
        .await?
        .with_context(|| {
            format!(
                "candidate release {} not found",
                options.candidate_release_id
            )
        })?;
    let market_scope = options
        .market_scope
        .clone()
        .unwrap_or_else(|| candidate_release.manifest.market_scope.clone());
    if candidate_release.manifest.market_scope != market_scope {
        bail!(
            "candidate release {} belongs to {}, not {}",
            candidate_release.manifest.release_id,
            candidate_release.manifest.market_scope,
            market_scope
        );
    }

    let original_active = store
        .load_active_model_release(&market_scope)
        .await?
        .with_context(|| format!("no active release found for market scope {market_scope}"))?;
    let baseline_release = if let Some(baseline_release_id) = options.baseline_release_id.as_deref()
    {
        let release = store
            .load_model_release(baseline_release_id)
            .await?
            .with_context(|| format!("baseline release {baseline_release_id} not found"))?;
        if release.manifest.market_scope != market_scope {
            bail!(
                "baseline release {} belongs to {}, not {}",
                release.manifest.release_id,
                release.manifest.market_scope,
                market_scope
            );
        }
        release
    } else {
        original_active.clone()
    };

    if baseline_release.manifest.release_id == candidate_release.manifest.release_id {
        bail!("baseline release and candidate release must be different");
    }

    let mut original_records = BTreeMap::<String, ModelReleaseRecord>::new();
    for release in [
        original_active.clone(),
        baseline_release.clone(),
        candidate_release.clone(),
    ] {
        original_records.insert(release.manifest.release_id.clone(), release);
    }

    let review_result = crate::run_release_review(
        &store,
        &market_scope,
        &options,
        &original_active,
        &baseline_release,
        &candidate_release,
    )
    .await;
    let restore_result = restore_release_review_state(
        &store,
        &market_scope,
        &original_active.manifest.release_id,
        &original_records,
        &options.api_reload_url,
        &options.updated_by,
    )
    .await;

    if let Err(restore_error) = restore_result {
        if let Err(review_error) = review_result {
            bail!(
                "release review failed and restore also failed:\nreview: {review_error:#}\nrestore: {restore_error:#}"
            );
        }
        bail!("release review completed but restore failed: {restore_error:#}");
    }

    review_result?;
    println!(
        "Release review restored original active release {}.",
        original_active.manifest.release_id
    );
    Ok(())
}

pub(crate) async fn activate_release_for_review(
    store: &fc_storage::SqliteStore,
    market_scope: &str,
    release_id: &str,
    api_reload_url: &str,
    history_mode: crate::ApiReloadHistoryMode,
    updated_by: &str,
    stage: &str,
) -> anyhow::Result<()> {
    store
        .activate_model_release(market_scope, release_id, updated_by)
        .await?;
    println!("Review step {stage}: activated {release_id}.");
    println!(
        "Review step {stage}: reloading API runtime via {api_reload_url} (history_mode={}).",
        history_mode.as_label()
    );
    crate::reload_api_runtime_with_history_mode(api_reload_url, history_mode).await?;
    println!("Review step {stage}: runtime ready.");
    Ok(())
}

pub(crate) async fn restore_release_review_state(
    store: &fc_storage::SqliteStore,
    market_scope: &str,
    original_active_release_id: &str,
    original_records: &BTreeMap<String, ModelReleaseRecord>,
    api_reload_url: &str,
    updated_by: &str,
) -> anyhow::Result<()> {
    store
        .activate_model_release(market_scope, original_active_release_id, updated_by)
        .await?;
    crate::reload_api_runtime(api_reload_url).await?;
    for record in original_records.values() {
        store.upsert_model_release(record).await?;
    }
    Ok(())
}

pub(crate) async fn activate_release_with_runtime_guard(
    store: &fc_storage::SqliteStore,
    market_scope: &str,
    release_id: &str,
    reload_api: bool,
    api_reload_url: &str,
    skip_operational_guard: bool,
    updated_by: &str,
) -> anyhow::Result<ModelReleaseRecord> {
    let previous_active = store.load_active_model_release(market_scope).await?;
    let previous_release_id = previous_active
        .as_ref()
        .map(|release| release.manifest.release_id.clone());
    let should_check_guard =
        reload_api && !skip_operational_guard && previous_release_id.as_deref() != Some(release_id);
    let baseline_assessment = if should_check_guard {
        Some(crate::fetch_assessment_snapshot_for_guard(api_reload_url).await?)
    } else {
        None
    };

    let activated = store
        .activate_model_release(market_scope, release_id, updated_by)
        .await?;
    println!(
        "Activated release {} for {}.",
        activated.manifest.release_id, activated.manifest.market_scope
    );
    println!(
        "  mode={} serving={} pit={}",
        activated.manifest.probability_mode,
        activated.manifest.serving_status,
        activated.manifest.point_in_time_mode
    );

    if reload_api {
        println!(
            "Reloading API runtime via {api_reload_url}. First load for a new release may take several minutes while history snapshots are materialized."
        );
        crate::reload_api_runtime(api_reload_url).await?;
        println!("Reloaded API runtime via {api_reload_url}.");
    }

    if let Some(baseline_assessment) = baseline_assessment {
        let candidate_assessment =
            crate::fetch_assessment_snapshot_for_guard(api_reload_url).await?;
        let regressions =
            crate::compare_operational_guardrails(&baseline_assessment, &candidate_assessment);
        if regressions.is_empty() {
            crate::print_operational_guardrail_summary(&baseline_assessment, &candidate_assessment);
            return Ok(activated);
        }

        if let Some(previous_release_id) = previous_release_id
            .as_deref()
            .filter(|previous_release_id| *previous_release_id != release_id)
        {
            println!(
                "Operational guard failed after activating {release_id}. Rolling back to {previous_release_id}."
            );
            let rolled_back = store
                .rollback_model_release(market_scope, previous_release_id, updated_by)
                .await?;
            if reload_api {
                println!(
                    "Reloading API runtime after rollback via {api_reload_url}. This may also take several minutes."
                );
                crate::reload_api_runtime(api_reload_url).await?;
                println!("Reloaded API runtime after rollback.");
            }
            bail!(
                "release {} regressed against baseline release {} and was rolled back to {}:\n  - {}",
                release_id,
                baseline_assessment
                    .method
                    .release_id
                    .as_deref()
                    .unwrap_or("unknown"),
                rolled_back.manifest.release_id,
                regressions.join("\n  - ")
            );
        }

        bail!(
            "release {} regressed against baseline but no previous active release was available for automatic rollback:\n  - {}",
            release_id,
            regressions.join("\n  - ")
        );
    }

    if !reload_api && !skip_operational_guard {
        println!(
            "Operational guard skipped because --reload-api was not enabled; use --reload-api to compare the new runtime against the current baseline."
        );
    } else if skip_operational_guard {
        println!("Operational guard explicitly skipped.");
    }

    Ok(activated)
}

pub(crate) async fn resolve_release_market_scope(
    store: &fc_storage::SqliteStore,
    release_id: &str,
    override_market_scope: Option<&str>,
) -> anyhow::Result<String> {
    if let Some(market_scope) = override_market_scope {
        return Ok(market_scope.to_string());
    }
    let release = store
        .load_model_release(release_id)
        .await?
        .with_context(|| format!("release {release_id} not found"))?;
    Ok(release.manifest.market_scope)
}

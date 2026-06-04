use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use anyhow::{bail, Context};
use chrono::{Duration, NaiveDate, Utc};
use fc_domain::{
    AssessmentHistoryPoint, BacktestScenarioSummary, DecisionPosture, ModelReleaseRecord,
    TimeToRiskBucket,
};

mod probability;

pub(crate) use probability::{
    research_release_formal_probability_compare, research_release_formal_probability_slice,
    research_release_probability_slice,
};
#[cfg(test)]
pub(crate) use probability::{
    ReleaseFormalProbabilityCompareOptions, ReleaseFormalProbabilitySliceOptions,
    ReleaseProbabilitySliceOptions,
};

const RELEASE_REVIEW_SIGNAL_WINDOW: usize = 5;
const RELEASE_REVIEW_SIGNAL_MIN_HITS: usize = 3;

struct ReleaseReviewComparisonInput<'a> {
    assessment: &'a fc_domain::AssessmentSnapshot,
    backtests: &'a [fc_domain::BacktestScenarioSummary],
    history: &'a [AssessmentHistoryPoint],
    method: &'a crate::AuditMethodResponseWire,
}

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

    let review_result = run_release_review(
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

#[derive(Debug, Clone)]
struct ReleaseReviewRuntimeSnapshot {
    assessment: fc_domain::AssessmentSnapshot,
    backtests: Vec<fc_domain::BacktestScenarioSummary>,
    method: crate::AuditMethodResponseWire,
    history: Vec<fc_domain::AssessmentHistoryPoint>,
}

async fn fetch_release_review_runtime_snapshot(
    api_reload_url: &str,
    history_limit: usize,
) -> anyhow::Result<ReleaseReviewRuntimeSnapshot> {
    let api_base_url = api_reload_url
        .strip_suffix("/api/system/reload")
        .with_context(|| {
            format!(
                "cannot derive API base URL from reload URL {api_reload_url}; expected it to end with /api/system/reload"
            )
        })?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()?;
    let assessment: fc_domain::AssessmentSnapshot =
        crate::fetch_api_json(&client, api_base_url, "/api/assessment/current").await?;
    let backtests: Vec<fc_domain::BacktestScenarioSummary> =
        crate::fetch_api_json(&client, api_base_url, "/api/backtests").await?;
    let method: crate::AuditMethodResponseWire =
        crate::fetch_api_json(&client, api_base_url, "/api/assessment/method").await?;
    let history_path = format!("/api/assessment/history?limit={history_limit}");
    let history: Vec<fc_domain::AssessmentHistoryPoint> =
        crate::fetch_api_json(&client, api_base_url, &history_path).await?;
    Ok(ReleaseReviewRuntimeSnapshot {
        assessment,
        backtests,
        method,
        history,
    })
}

async fn run_release_review(
    store: &fc_storage::SqliteStore,
    market_scope: &str,
    options: &ReleaseReviewOptions,
    original_active: &ModelReleaseRecord,
    baseline_release: &ModelReleaseRecord,
    candidate_release: &ModelReleaseRecord,
) -> anyhow::Result<()> {
    println!(
        "Review baseline={} candidate={} market_scope={market_scope}.",
        baseline_release.manifest.release_id, candidate_release.manifest.release_id
    );

    activate_release_for_review(
        store,
        market_scope,
        &baseline_release.manifest.release_id,
        options,
        "baseline",
    )
    .await?;
    let baseline_runtime_snapshot =
        fetch_release_review_runtime_snapshot(&options.api_reload_url, options.history_limit)
            .await?;

    activate_release_for_review(
        store,
        market_scope,
        &candidate_release.manifest.release_id,
        options,
        "candidate",
    )
    .await?;
    let candidate_runtime_snapshot =
        fetch_release_review_runtime_snapshot(&options.api_reload_url, options.history_limit)
            .await?;

    let baseline_assessment = baseline_runtime_snapshot.assessment;
    let candidate_assessment = candidate_runtime_snapshot.assessment;
    let baseline_runtime_review = crate::build_release_runtime_review_diagnostics(
        &baseline_release.manifest.release_id,
        &baseline_release.manifest.label_version,
        &baseline_runtime_snapshot.method,
        &baseline_runtime_snapshot.history,
    );
    let candidate_runtime_review = crate::build_release_runtime_review_diagnostics(
        &candidate_release.manifest.release_id,
        &candidate_release.manifest.label_version,
        &candidate_runtime_snapshot.method,
        &candidate_runtime_snapshot.history,
    );

    let baseline_actionability_review = build_release_actionability_review(baseline_release)?;
    let candidate_actionability_review = build_release_actionability_review(candidate_release)?;
    let probability_regressions = compare_probability_guardrails(candidate_release)?;
    let candidate_has_actionability = candidate_actionability_review.enabled;
    let operational_regressions =
        compare_operational_guardrails(&baseline_assessment, &candidate_assessment);
    let actionability_regressions =
        compare_actionability_guardrails(&candidate_actionability_review);
    let runtime_sanity_regressions =
        compare_runtime_sanity_guardrails(&baseline_runtime_review, &candidate_runtime_review);
    let mut overall_regressions = operational_regressions.clone();
    overall_regressions.extend(probability_regressions.iter().cloned());
    overall_regressions.extend(actionability_regressions.iter().cloned());
    overall_regressions.extend(runtime_sanity_regressions.iter().cloned());
    let scenario_focus = build_release_review_scenario_focus_diagnostics(
        &baseline_runtime_snapshot.backtests,
        &candidate_runtime_snapshot.backtests,
        &baseline_runtime_snapshot.history,
        &candidate_runtime_snapshot.history,
        &baseline_runtime_snapshot.method,
        &candidate_runtime_snapshot.method,
    );
    let historical_audit_priorities =
        crate::summarize_release_review_historical_audit_priorities(&scenario_focus);
    let historical_audit_attribution =
        crate::summarize_release_review_historical_audit_attribution(&historical_audit_priorities);
    let historical_audit_actions =
        crate::summarize_release_review_historical_audit_actions(&historical_audit_attribution);
    let historical_audit_workstreams =
        crate::summarize_release_review_historical_audit_workstreams(&historical_audit_priorities);

    let report = crate::ReleaseReviewEnvelope {
        reviewed_at: Utc::now().to_rfc3339(),
        market_scope: market_scope.to_string(),
        api_reload_url: options.api_reload_url.clone(),
        history_mode: options.history_mode.as_label().to_string(),
        history_limit: options.history_limit,
        original_active_release_id: original_active.manifest.release_id.clone(),
        restored_release_id: original_active.manifest.release_id.clone(),
        baseline_release: baseline_release.clone(),
        candidate_release: candidate_release.clone(),
        comparison: build_release_review_comparison(
            ReleaseReviewComparisonInput {
                assessment: &baseline_assessment,
                backtests: &baseline_runtime_snapshot.backtests,
                history: &baseline_runtime_snapshot.history,
                method: &baseline_runtime_snapshot.method,
            },
            ReleaseReviewComparisonInput {
                assessment: &candidate_assessment,
                backtests: &candidate_runtime_snapshot.backtests,
                history: &candidate_runtime_snapshot.history,
                method: &candidate_runtime_snapshot.method,
            },
            &baseline_runtime_review,
            &candidate_runtime_review,
        ),
        baseline_assessment,
        candidate_assessment,
        baseline_runtime_review,
        candidate_runtime_review,
        baseline_actionability_review,
        candidate_actionability_review,
        scenario_focus,
        historical_audit_workstreams,
        historical_audit_priorities,
        historical_audit_attribution,
        historical_audit_actions: historical_audit_actions.clone(),
        probability_guard_passed: probability_regressions.is_empty(),
        operational_guard_passed: operational_regressions.is_empty(),
        actionability_guard_passed: actionability_regressions.is_empty(),
        runtime_sanity_passed: runtime_sanity_regressions.is_empty(),
        overall_guard_passed: overall_regressions.is_empty(),
        recommendation: build_release_review_recommendation(
            &overall_regressions,
            candidate_has_actionability,
            &historical_audit_actions,
        ),
        operational_guard_regressions: operational_regressions,
        probability_guard_regressions: probability_regressions,
        actionability_guard_regressions: actionability_regressions,
        runtime_sanity_regressions,
        overall_guard_regressions: overall_regressions,
    };
    crate::reporting::write_release_review_report(&options.output_dir, &report)?;

    println!(
        "Release review complete: guard_passed={} baseline={} candidate={}.",
        report.overall_guard_passed,
        report.baseline_release.manifest.release_id,
        report.candidate_release.manifest.release_id
    );
    print_release_review_summary(&report);

    Ok(())
}

pub(crate) async fn activate_release_for_review(
    store: &fc_storage::SqliteStore,
    market_scope: &str,
    release_id: &str,
    options: &ReleaseReviewOptions,
    stage: &str,
) -> anyhow::Result<()> {
    store
        .activate_model_release(market_scope, release_id, &options.updated_by)
        .await?;
    println!("Review step {stage}: activated {release_id}.");
    println!(
        "Review step {stage}: reloading API runtime via {api_reload_url} (history_mode={} history_limit={}).",
        options.history_mode.as_label(),
        options.history_limit,
        api_reload_url = options.api_reload_url
    );
    crate::reload_api_runtime_with_history_options(
        &options.api_reload_url,
        options.history_mode,
        Some(options.history_limit),
    )
    .await?;
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
            compare_operational_guardrails(&baseline_assessment, &candidate_assessment);
        if regressions.is_empty() {
            print_operational_guardrail_summary(&baseline_assessment, &candidate_assessment);
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

fn build_release_actionability_review(
    release: &ModelReleaseRecord,
) -> anyhow::Result<crate::ReleaseActionabilityReview> {
    let bundle =
        crate::read_probability_bundle(std::path::Path::new(&release.manifest.bundle_uri))?;
    let Some(actionability) = bundle.actionability.as_ref() else {
        return Ok(crate::ReleaseActionabilityReview {
            release_id: release.manifest.release_id.clone(),
            enabled: false,
            model_version: None,
            calibration_version: None,
            fusion_policy_version: None,
            levels: Vec::new(),
            guard_regressions: Vec::new(),
            guard_passed: true,
            note: "This release has no independent actionability head; release review only applies runtime guardrails.".to_string(),
        });
    };

    let levels = actionability
        .levels
        .iter()
        .map(|level| {
            let evaluation = level
                .evaluation
                .actionability
                .as_ref()
                .cloned()
                .unwrap_or_default();
            crate::ReleaseActionabilityLevelReview {
                level: level.level,
                proxy_horizon_days: level.proxy_horizon_days,
                sample_count: level.evaluation.sample_count,
                positive_rate: level.evaluation.positive_rate,
                threshold: evaluation.threshold,
                predicted_positive_count: evaluation.predicted_positive_count,
                primary_positive_count: evaluation.actual_positive_count,
                late_validation_row_count: evaluation.post_start_positive_count,
                protected_row_count: evaluation.unclassified_positive_count,
                primary_hit_count: evaluation.pre_start_hit_count,
                late_validation_hit_count: evaluation.post_start_hit_count,
                protected_hit_count: evaluation.unclassified_hit_count,
                false_positive_count: evaluation.false_positive_count,
                scenario_count: evaluation.scenario_count,
                on_time_scenario_count: evaluation.advance_warning_scenario_count,
                late_only_scenario_count: evaluation.late_confirmation_scenario_count,
                missed_scenario_count: evaluation.missed_scenario_count,
                precision_at_threshold: evaluation.precision_at_threshold,
                primary_recall_at_threshold: evaluation.pre_start_recall_at_threshold,
                late_validation_capture_rate: evaluation.post_start_recall_at_threshold,
                on_time_rate: evaluation.advance_warning_rate,
                late_only_rate: evaluation.late_confirmation_rate,
                missed_rate: evaluation.missed_rate,
                note: evaluation.note,
            }
        })
        .collect::<Vec<_>>();

    let mut review = crate::ReleaseActionabilityReview {
        release_id: release.manifest.release_id.clone(),
        enabled: true,
        model_version: Some(actionability.model_version.clone()),
        calibration_version: Some(actionability.calibration_version.clone()),
        fusion_policy_version: Some(actionability.fusion_policy_version.clone()),
        levels,
        guard_regressions: Vec::new(),
        guard_passed: true,
        note: actionability.note.clone(),
    };
    review.guard_regressions = compare_actionability_guardrails(&review);
    review.guard_passed = review.guard_regressions.is_empty();
    Ok(review)
}

pub(crate) fn compare_actionability_guardrails(
    review: &crate::ReleaseActionabilityReview,
) -> Vec<String> {
    if !review.enabled {
        return Vec::new();
    }

    let mut regressions = Vec::new();
    for level in &review.levels {
        let level_name = crate::actionability_level_text(level.level);
        let policy = crate::actionability_guardrail_policy(level.level, level.proxy_horizon_days);

        if level.scenario_count < policy.min_scenario_count {
            regressions.push(format!(
                "actionability {level_name} scenario_count is {} (<{}), so the evaluation slice is too narrow for go/no-go",
                level.scenario_count, policy.min_scenario_count
            ));
        }

        let precision_score =
            (level.precision_at_threshold.unwrap_or_default() * 1_000.0).round() as i64;
        if precision_score < policy.min_precision_score {
            regressions.push(format!(
                "actionability {level_name} precision {:.1}% is below required {:.1}%",
                precision_score as f64 / 10.0,
                policy.min_precision_score as f64 / 10.0
            ));
        }

        let prediction_ceiling =
            crate::actionability_prediction_count_ceiling_from_actual_positive_count(
                level.primary_positive_count,
                level.proxy_horizon_days,
            );
        if level.predicted_positive_count > prediction_ceiling {
            regressions.push(format!(
                "actionability {level_name} predicted positives {} exceed ceiling {} for {} primary episode rows",
                level.predicted_positive_count,
                prediction_ceiling,
                level.primary_positive_count
            ));
        }

        if level.primary_positive_count > 0
            && level.primary_hit_count == 0
            && level.late_validation_hit_count == 0
        {
            regressions.push(format!(
                "actionability {level_name} produced no primary or late-validation hits across {} primary episode rows",
                level.primary_positive_count
            ));
        }

        if level.primary_positive_count > 0 {
            if let Some(min_advance_warning_rate_score) = policy.min_advance_warning_rate_score {
                let on_time_rate_score =
                    crate::percentage_score(level.on_time_rate).unwrap_or_default();
                if on_time_rate_score < min_advance_warning_rate_score {
                    regressions.push(format!(
                        "actionability {level_name} on_time_rate {:.1}% is below required {:.1}%",
                        on_time_rate_score as f64 / 10.0,
                        min_advance_warning_rate_score as f64 / 10.0
                    ));
                }
            }

            if let Some(max_late_confirmation_rate_score) = policy.max_late_confirmation_rate_score
            {
                let late_only_rate_score =
                    crate::percentage_score(level.late_only_rate).unwrap_or_default();
                if late_only_rate_score > max_late_confirmation_rate_score {
                    regressions.push(format!(
                        "actionability {level_name} late_only_rate {:.1}% exceeds ceiling {:.1}%",
                        late_only_rate_score as f64 / 10.0,
                        max_late_confirmation_rate_score as f64 / 10.0
                    ));
                }
            }

            let missed_rate_score = crate::percentage_score(level.missed_rate).unwrap_or_default();
            if missed_rate_score > policy.max_missed_rate_score {
                regressions.push(format!(
                    "actionability {level_name} missed_rate {:.1}% exceeds ceiling {:.1}%",
                    missed_rate_score as f64 / 10.0,
                    policy.max_missed_rate_score as f64 / 10.0
                ));
            }
        }
    }
    regressions
}

pub(crate) fn compare_probability_guardrails(
    release: &ModelReleaseRecord,
) -> anyhow::Result<Vec<String>> {
    if release.manifest.probability_mode == "heuristic_mvp" {
        return Ok(vec![format!(
            "release {} has no formal probability bundle evaluation, so it cannot satisfy formal promotion guard",
            release.manifest.release_id
        )]);
    }

    let bundle =
        crate::read_probability_bundle(std::path::Path::new(&release.manifest.bundle_uri))?;
    let Some(summary) = bundle.evaluation.as_ref() else {
        return Ok(vec![format!(
            "release {} bundle is missing aggregate probability evaluation summary",
            release.manifest.release_id
        )]);
    };

    let mut regressions = Vec::new();
    if summary.usable_early_warning_horizon_count == 0 {
        regressions.push(
            "probability head has zero usable early-warning horizons in bundle evaluation"
                .to_string(),
        );
    }

    for horizon in &summary.regime_separation_summaries {
        if horizon.horizon_days == 20
            && horizon.positive_window_avg_probability <= horizon.normal_avg_probability
        {
            regressions.push(format!(
                "20d positive_window avg {} is at or below normal {} in bundle evaluation",
                crate::format_pct(horizon.positive_window_avg_probability),
                crate::format_pct(horizon.normal_avg_probability),
            ));
        }
        if matches!(horizon.horizon_days, 20 | 60)
            && matches!(
                horizon.diagnosis.as_str(),
                "cooldown_bleed" | "cold_across_all_regimes"
            )
        {
            regressions.push(format!(
                "{}d regime diagnosis is {} in bundle evaluation",
                horizon.horizon_days, horizon.diagnosis
            ));
        }
    }

    Ok(regressions)
}

fn compare_operational_guardrails(
    baseline: &fc_domain::AssessmentSnapshot,
    candidate: &fc_domain::AssessmentSnapshot,
) -> Vec<String> {
    let mut regressions = Vec::new();
    let baseline_summary = &baseline.backtest_summary;
    let candidate_summary = &candidate.backtest_summary;
    let baseline_rolling = &baseline_summary.rolling_audit;
    let candidate_rolling = &candidate_summary.rolling_audit;

    if candidate_summary.timely_warning_rate + 0.05 < baseline_summary.timely_warning_rate {
        regressions.push(format!(
            "timely_warning_rate dropped from {:.1}% to {:.1}%",
            baseline_summary.timely_warning_rate * 100.0,
            candidate_summary.timely_warning_rate * 100.0
        ));
    }

    if candidate_rolling.actionable_precision + 0.05 < baseline_rolling.actionable_precision {
        regressions.push(format!(
            "actionable_precision dropped from {:.1}% to {:.1}%",
            baseline_rolling.actionable_precision * 100.0,
            candidate_rolling.actionable_precision * 100.0
        ));
    }

    if candidate_rolling.longest_false_positive_episode_days
        > baseline_rolling.longest_false_positive_episode_days + 7
    {
        regressions.push(format!(
            "longest_false_positive_episode_days increased from {} to {}",
            baseline_rolling.longest_false_positive_episode_days,
            candidate_rolling.longest_false_positive_episode_days
        ));
    }

    regressions
}

fn compare_runtime_sanity_guardrails(
    baseline: &crate::ReleaseRuntimeReviewDiagnostics,
    candidate: &crate::ReleaseRuntimeReviewDiagnostics,
) -> Vec<String> {
    let mut regressions = Vec::new();
    let usable_early_warning_horizon_count = candidate
        .regime_separation_summaries
        .iter()
        .filter(|summary| summary.diagnosis == "usable_early_warning_separation")
        .count();

    if usable_early_warning_horizon_count == 0 {
        regressions.push(format!(
            "candidate {} has zero usable early-warning horizons in runtime regime audit",
            candidate.release_id
        ));
    }

    for summary in &candidate.regime_separation_summaries {
        if summary.horizon_days == 20
            && summary.positive_window_avg_probability <= summary.normal_avg_probability
        {
            regressions.push(format!(
                "candidate {} keeps 20d positive_window avg {} at or below normal {} in runtime history",
                candidate.release_id,
                crate::format_pct(summary.positive_window_avg_probability),
                crate::format_pct(summary.normal_avg_probability),
            ));
        }
        if matches!(summary.horizon_days, 20 | 60) && summary.diagnosis == "cooldown_bleed" {
            regressions.push(format!(
                "candidate {} shows cooldown_bleed on {}d runtime regime audit: cooldown {} vs positive_window {}",
                candidate.release_id,
                summary.horizon_days,
                crate::format_pct(summary.post_crisis_cooldown_avg_probability),
                crate::format_pct(summary.positive_window_avg_probability),
            ));
        }
    }

    if release_has_cold_runtime_history(candidate) {
        regressions.push(format!(
            "candidate {} stayed all-normal across {} history points, hit zero runtime probability floors, and still showed no usable early-warning regime separation",
            candidate.release_id, candidate.history_point_count
        ));
    }

    if release_has_cold_runtime_history(baseline) {
        regressions.push(format!(
            "baseline {} is also all-normal / zero-floor-hit, so relative guardrails alone are not a sufficient promotion test",
            baseline.release_id
        ));
    }

    regressions
}

fn release_has_cold_runtime_history(diagnostics: &crate::ReleaseRuntimeReviewDiagnostics) -> bool {
    let all_normal = diagnostics.posture_distribution.len() == 1
        && diagnostics.posture_distribution.first().is_some_and(|row| {
            row.name == "normal" && row.count == diagnostics.history_point_count
        });
    let zero_floor_hits = diagnostics.runtime_thresholds.is_some()
        && [
            diagnostics
                .points_at_or_above_prepare_p60d
                .unwrap_or_default(),
            diagnostics
                .points_at_or_above_hedge_p20d
                .unwrap_or_default(),
            diagnostics
                .points_at_or_above_defend_p5d
                .unwrap_or_default(),
        ]
        .into_iter()
        .all(|count| count == 0);
    let no_usable_early_warning = !diagnostics
        .regime_separation_summaries
        .iter()
        .any(|summary| {
            matches!(
                summary.diagnosis.as_str(),
                "usable_early_warning_separation" | "separated_but_below_runtime_floor"
            )
        });

    all_normal && zero_floor_hits && no_usable_early_warning
}

fn print_operational_guardrail_summary(
    baseline: &fc_domain::AssessmentSnapshot,
    candidate: &fc_domain::AssessmentSnapshot,
) {
    println!("Operational guard summary:");
    println!(
        "  timely_warning_rate   {} -> {}",
        crate::format_pct(baseline.backtest_summary.timely_warning_rate),
        crate::format_pct(candidate.backtest_summary.timely_warning_rate)
    );
    println!(
        "  actionable_precision  {} -> {}",
        crate::format_pct(baseline.backtest_summary.rolling_audit.actionable_precision),
        crate::format_pct(
            candidate
                .backtest_summary
                .rolling_audit
                .actionable_precision
        )
    );
    println!(
        "  longest_false_positive_episode_days  {} -> {}",
        baseline
            .backtest_summary
            .rolling_audit
            .longest_false_positive_episode_days,
        candidate
            .backtest_summary
            .rolling_audit
            .longest_false_positive_episode_days
    );
}

fn build_release_review_comparison(
    baseline: ReleaseReviewComparisonInput<'_>,
    candidate: ReleaseReviewComparisonInput<'_>,
    baseline_runtime_review: &crate::ReleaseRuntimeReviewDiagnostics,
    candidate_runtime_review: &crate::ReleaseRuntimeReviewDiagnostics,
) -> crate::ReleaseReviewComparisonSummary {
    let (baseline_strict_actionable_point_count, baseline_runtime_floor_hit_count) =
        release_review_structured_signal_counts(
            baseline.backtests,
            baseline.history,
            baseline.method,
        );
    let (candidate_strict_actionable_point_count, candidate_runtime_floor_hit_count) =
        release_review_structured_signal_counts(
            candidate.backtests,
            candidate.history,
            candidate.method,
        );
    crate::ReleaseReviewComparisonSummary {
        timely_warning_rate: scalar_metric(
            baseline.assessment.backtest_summary.timely_warning_rate,
            candidate.assessment.backtest_summary.timely_warning_rate,
        ),
        strict_actionable_point_count: count_metric(
            baseline_strict_actionable_point_count,
            candidate_strict_actionable_point_count,
        ),
        runtime_floor_hit_count: count_metric(
            baseline_runtime_floor_hit_count,
            candidate_runtime_floor_hit_count,
        ),
        actionable_precision: scalar_metric(
            baseline
                .assessment
                .backtest_summary
                .rolling_audit
                .actionable_precision,
            candidate
                .assessment
                .backtest_summary
                .rolling_audit
                .actionable_precision,
        ),
        longest_false_positive_episode_days: count_metric(
            baseline
                .assessment
                .backtest_summary
                .rolling_audit
                .longest_false_positive_episode_days,
            candidate
                .assessment
                .backtest_summary
                .rolling_audit
                .longest_false_positive_episode_days,
        ),
        current_p_5d: scalar_metric(
            baseline.assessment.probabilities.p_5d,
            candidate.assessment.probabilities.p_5d,
        ),
        current_p_20d: scalar_metric(
            baseline.assessment.probabilities.p_20d,
            candidate.assessment.probabilities.p_20d,
        ),
        current_p_60d: scalar_metric(
            baseline.assessment.probabilities.p_60d,
            candidate.assessment.probabilities.p_60d,
        ),
        runtime_separation_summary: build_release_review_runtime_separation_comparisons(
            baseline_runtime_review,
            candidate_runtime_review,
        ),
        backtest_scenarios: build_release_review_backtest_scenario_comparisons(
            baseline.backtests,
            candidate.backtests,
        ),
    }
}

pub(crate) fn build_release_review_runtime_separation_comparisons(
    baseline: &crate::ReleaseRuntimeReviewDiagnostics,
    candidate: &crate::ReleaseRuntimeReviewDiagnostics,
) -> Vec<crate::ReleaseReviewRuntimeSeparationComparison> {
    let baseline_by_horizon = baseline
        .regime_separation_summaries
        .iter()
        .map(|summary| (summary.horizon_days, summary))
        .collect::<BTreeMap<_, _>>();
    let candidate_by_horizon = candidate
        .regime_separation_summaries
        .iter()
        .map(|summary| (summary.horizon_days, summary))
        .collect::<BTreeMap<_, _>>();
    let horizons = baseline_by_horizon
        .keys()
        .chain(candidate_by_horizon.keys())
        .copied()
        .collect::<BTreeSet<_>>();

    horizons
        .into_iter()
        .map(|horizon_days| {
            let baseline_summary = baseline_by_horizon.get(&horizon_days).copied();
            let candidate_summary = candidate_by_horizon.get(&horizon_days).copied();
            let baseline_threshold = release_review_runtime_threshold_for_horizon(
                baseline.runtime_thresholds.as_ref(),
                horizon_days,
            );
            let candidate_threshold = release_review_runtime_threshold_for_horizon(
                candidate.runtime_thresholds.as_ref(),
                horizon_days,
            );
            let baseline_early_warning_avg_probability =
                baseline_summary.and_then(release_review_early_warning_avg_probability);
            let candidate_early_warning_avg_probability =
                candidate_summary.and_then(release_review_early_warning_avg_probability);
            let baseline_normal_avg_probability =
                baseline_summary.map(|summary| summary.normal_avg_probability);
            let candidate_normal_avg_probability =
                candidate_summary.map(|summary| summary.normal_avg_probability);

            crate::ReleaseReviewRuntimeSeparationComparison {
                horizon_days,
                baseline_diagnosis: baseline_summary
                    .map(|summary| summary.diagnosis.clone())
                    .unwrap_or_else(|| "missing".to_string()),
                candidate_diagnosis: candidate_summary
                    .map(|summary| summary.diagnosis.clone())
                    .unwrap_or_else(|| "missing".to_string()),
                baseline_threshold,
                candidate_threshold,
                baseline_early_warning_regime: baseline_summary
                    .map(|summary| summary.early_warning_regime.clone())
                    .unwrap_or_else(|| "—".to_string()),
                candidate_early_warning_regime: candidate_summary
                    .map(|summary| summary.early_warning_regime.clone())
                    .unwrap_or_else(|| "—".to_string()),
                baseline_early_warning_avg_probability,
                candidate_early_warning_avg_probability,
                baseline_normal_avg_probability,
                candidate_normal_avg_probability,
                baseline_early_warning_gap_vs_normal: baseline_summary
                    .and_then(release_review_early_warning_gap_vs_normal),
                candidate_early_warning_gap_vs_normal: candidate_summary
                    .and_then(release_review_early_warning_gap_vs_normal),
                baseline_floor_gap: release_review_runtime_floor_gap(
                    baseline_early_warning_avg_probability,
                    baseline_threshold,
                ),
                candidate_floor_gap: release_review_runtime_floor_gap(
                    candidate_early_warning_avg_probability,
                    candidate_threshold,
                ),
                baseline_early_warning_lift_vs_normal: baseline_summary
                    .and_then(|summary| summary.early_warning_calibrated_lift_vs_normal),
                candidate_early_warning_lift_vs_normal: candidate_summary
                    .and_then(|summary| summary.early_warning_calibrated_lift_vs_normal),
                baseline_threshold_hit_rate: baseline_summary
                    .and_then(|summary| summary.max_non_normal_threshold_hit_rate),
                candidate_threshold_hit_rate: candidate_summary
                    .and_then(|summary| summary.max_non_normal_threshold_hit_rate),
            }
        })
        .collect()
}

fn release_review_runtime_threshold_for_horizon(
    runtime_thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
    horizon_days: u32,
) -> Option<f64> {
    runtime_thresholds.map(|thresholds| match horizon_days {
        5 => thresholds.defend_p5d,
        20 => thresholds.hedge_p20d,
        60 => thresholds.prepare_p60d,
        _ => 1.0,
    })
}

fn release_review_early_warning_avg_probability(
    summary: &crate::ReleaseRuntimeSeparationSummary,
) -> Option<f64> {
    match summary.early_warning_regime.as_str() {
        "positive_window" => Some(summary.positive_window_avg_probability),
        "pre_warning_buffer" => Some(summary.pre_warning_buffer_avg_probability),
        _ => None,
    }
}

fn release_review_early_warning_gap_vs_normal(
    summary: &crate::ReleaseRuntimeSeparationSummary,
) -> Option<f64> {
    release_review_early_warning_avg_probability(summary)
        .map(|value| crate::round6(value - summary.normal_avg_probability))
}

fn release_review_runtime_floor_gap(
    early_warning_avg_probability: Option<f64>,
    threshold: Option<f64>,
) -> Option<f64> {
    match (early_warning_avg_probability, threshold) {
        (Some(early_warning_avg_probability), Some(threshold)) => {
            Some(crate::round6(early_warning_avg_probability - threshold))
        }
        _ => None,
    }
}

pub(crate) fn release_review_structured_signal_counts(
    backtests: &[BacktestScenarioSummary],
    history: &[AssessmentHistoryPoint],
    method: &crate::AuditMethodResponseWire,
) -> (u32, u32) {
    let use_transitional_bridge = release_review_uses_transitional_actionable_bridge(method);
    let thresholds = method.runtime_thresholds.as_ref();
    let in_any_pre_crisis_window = |point: &AssessmentHistoryPoint| {
        backtests.iter().any(|scenario| {
            let window_start = scenario.crisis_start - Duration::days(90);
            point.as_of_date >= window_start && point.as_of_date < scenario.crisis_start
        })
    };
    let strict_actionable_point_count = history
        .iter()
        .filter(|point| in_any_pre_crisis_window(point))
        .filter(|point| release_review_is_actionable_warning_point(point, use_transitional_bridge))
        .count() as u32;
    let runtime_floor_hit_count = history
        .iter()
        .filter(|point| in_any_pre_crisis_window(point))
        .filter(|point| release_review_hits_runtime_floor(point, thresholds))
        .count() as u32;
    (strict_actionable_point_count, runtime_floor_hit_count)
}

pub(crate) fn build_release_review_backtest_scenario_comparisons(
    baseline_backtests: &[BacktestScenarioSummary],
    candidate_backtests: &[BacktestScenarioSummary],
) -> Vec<crate::ReleaseReviewBacktestScenarioComparison> {
    let candidate_by_id = candidate_backtests
        .iter()
        .map(|scenario| (scenario.scenario_id.as_str(), scenario))
        .collect::<std::collections::BTreeMap<_, _>>();

    let mut rows = baseline_backtests
        .iter()
        .map(|baseline| {
            let candidate = candidate_by_id.get(baseline.scenario_id.as_str()).copied();
            let candidate_lead_time_days = candidate.and_then(|scenario| scenario.lead_time_days);
            let candidate_actionable_lead_time_days =
                candidate.and_then(|scenario| scenario.actionable_lead_time_days);
            let candidate_false_positive_count = candidate
                .map(|scenario| scenario.false_positive_count)
                .unwrap_or_default();
            let candidate_first_l2_date = candidate.and_then(|scenario| scenario.first_l2_date);
            let candidate_first_l3_date = candidate.and_then(|scenario| scenario.first_l3_date);
            crate::ReleaseReviewBacktestScenarioComparison {
                scenario_id: baseline.scenario_id.clone(),
                name: baseline.name.clone(),
                signal_source: match baseline.signal_source {
                    fc_domain::BacktestSignalSource::RealHistory => "real_history",
                    fc_domain::BacktestSignalSource::FallbackTemplate => "fallback_template",
                }
                .to_string(),
                crisis_start: baseline.crisis_start,
                crisis_end: baseline.crisis_end,
                baseline_first_l2_date: baseline.first_l2_date,
                candidate_first_l2_date,
                baseline_first_l3_date: baseline.first_l3_date,
                candidate_first_l3_date,
                baseline_lead_time_days: baseline.lead_time_days,
                candidate_lead_time_days,
                baseline_actionable_lead_time_days: baseline.actionable_lead_time_days,
                candidate_actionable_lead_time_days,
                baseline_false_positive_count: baseline.false_positive_count,
                candidate_false_positive_count,
                actionable_delta_days: match (
                    baseline.actionable_lead_time_days,
                    candidate_actionable_lead_time_days,
                ) {
                    (Some(baseline_days), Some(candidate_days)) => {
                        Some(candidate_days - baseline_days)
                    }
                    _ => None,
                },
                outcome: format!(
                    "{}_to_{}",
                    backtest_warning_state(baseline.actionable_lead_time_days),
                    backtest_warning_state(candidate_actionable_lead_time_days)
                ),
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));
    rows
}

pub(crate) fn build_release_review_scenario_focus_diagnostics(
    baseline_backtests: &[BacktestScenarioSummary],
    candidate_backtests: &[BacktestScenarioSummary],
    baseline_history: &[AssessmentHistoryPoint],
    candidate_history: &[AssessmentHistoryPoint],
    baseline_method: &crate::AuditMethodResponseWire,
    candidate_method: &crate::AuditMethodResponseWire,
) -> Vec<crate::ReleaseReviewScenarioFocusDiagnostic> {
    let candidate_by_id = candidate_backtests
        .iter()
        .map(|scenario| (scenario.scenario_id.as_str(), scenario))
        .collect::<BTreeMap<_, _>>();
    let baseline_points_by_date = baseline_history
        .iter()
        .map(|point| (point.as_of_date, point))
        .collect::<BTreeMap<_, _>>();
    let candidate_points_by_date = candidate_history
        .iter()
        .map(|point| (point.as_of_date, point))
        .collect::<BTreeMap<_, _>>();
    let baseline_use_transitional_bridge =
        release_review_uses_transitional_actionable_bridge(baseline_method);
    let candidate_use_transitional_bridge =
        release_review_uses_transitional_actionable_bridge(candidate_method);
    let baseline_runtime_thresholds = baseline_method.runtime_thresholds.as_ref();
    let candidate_runtime_thresholds = candidate_method.runtime_thresholds.as_ref();

    let mut rows = baseline_backtests
        .iter()
        .filter_map(|baseline| {
            let candidate = candidate_by_id.get(baseline.scenario_id.as_str()).copied();
            if !scenario_requires_focus_review(baseline, candidate) {
                return None;
            }

            let window_start = baseline.crisis_start - Duration::days(90);
            let window_end = baseline.crisis_end;
            let mut baseline_window_points = baseline_history
                .iter()
                .filter(|point| point.as_of_date >= window_start && point.as_of_date <= window_end)
                .collect::<Vec<_>>();
            let mut candidate_window_points = candidate_history
                .iter()
                .filter(|point| point.as_of_date >= window_start && point.as_of_date <= window_end)
                .collect::<Vec<_>>();
            baseline_window_points.sort_by_key(|point| point.as_of_date);
            candidate_window_points.sort_by_key(|point| point.as_of_date);
            let mut baseline_pre_crisis_points = baseline_window_points
                .iter()
                .copied()
                .filter(|point| point.as_of_date < baseline.crisis_start)
                .collect::<Vec<_>>();
            let mut candidate_pre_crisis_points = candidate_window_points
                .iter()
                .copied()
                .filter(|point| point.as_of_date < baseline.crisis_start)
                .collect::<Vec<_>>();
            baseline_pre_crisis_points.sort_by_key(|point| point.as_of_date);
            candidate_pre_crisis_points.sort_by_key(|point| point.as_of_date);
            let baseline_first_non_normal_date =
                release_review_first_non_normal_date(&baseline_window_points);
            let candidate_first_non_normal_date =
                release_review_first_non_normal_date(&candidate_window_points);
            let baseline_runtime_floor_hit_point_count = baseline_pre_crisis_points
                .iter()
                .filter(|point| {
                    release_review_hits_runtime_floor(point, baseline_runtime_thresholds)
                })
                .count() as u32;
            let candidate_runtime_floor_hit_point_count = candidate_pre_crisis_points
                .iter()
                .filter(|point| {
                    release_review_hits_runtime_floor(point, candidate_runtime_thresholds)
                })
                .count() as u32;
            let baseline_actionable_hits = release_review_actionable_forward_hits_by_date(
                &baseline_pre_crisis_points,
                baseline_use_transitional_bridge,
            );
            let candidate_actionable_hits = release_review_actionable_forward_hits_by_date(
                &candidate_pre_crisis_points,
                candidate_use_transitional_bridge,
            );
            let baseline_first_runtime_floor_hit_without_l3 =
                release_review_first_runtime_floor_hit_without_l3(
                    &baseline_pre_crisis_points,
                    baseline_use_transitional_bridge,
                    baseline_runtime_thresholds,
                );
            let candidate_first_runtime_floor_hit_without_l3 =
                release_review_first_runtime_floor_hit_without_l3(
                    &candidate_pre_crisis_points,
                    candidate_use_transitional_bridge,
                    candidate_runtime_thresholds,
                );

            let mut interesting_dates = BTreeSet::new();
            for date in [
                Some(baseline.crisis_start),
                Some(baseline.crisis_end),
                baseline.first_l2_date,
                candidate.and_then(|scenario| scenario.first_l2_date),
                baseline.first_l3_date,
                candidate.and_then(|scenario| scenario.first_l3_date),
                baseline_first_non_normal_date,
                candidate_first_non_normal_date,
                baseline_first_runtime_floor_hit_without_l3
                    .as_ref()
                    .map(|(date, _)| *date),
                candidate_first_runtime_floor_hit_without_l3
                    .as_ref()
                    .map(|(date, _)| *date),
            ]
            .into_iter()
            .flatten()
            {
                if date >= window_start && date <= window_end {
                    interesting_dates.insert(date);
                }
            }

            for date in baseline_window_points
                .iter()
                .map(|point| point.as_of_date)
                .chain(candidate_window_points.iter().map(|point| point.as_of_date))
            {
                let baseline_point = baseline_points_by_date.get(&date).copied();
                let candidate_point = candidate_points_by_date.get(&date).copied();
                if release_review_point_is_interesting(
                    baseline_point,
                    candidate_point,
                    baseline_use_transitional_bridge,
                    candidate_use_transitional_bridge,
                ) {
                    interesting_dates.insert(date);
                }
            }

            let interesting_points = interesting_dates
                .into_iter()
                .filter_map(|date| {
                    let baseline_point = baseline_points_by_date.get(&date).copied();
                    let candidate_point = candidate_points_by_date.get(&date).copied();
                    if baseline_point.is_none() && candidate_point.is_none() {
                        return None;
                    }
                    let baseline_strict_review_actionable = baseline_point.is_some_and(|point| {
                        release_review_is_actionable_warning_point(
                            point,
                            baseline_use_transitional_bridge,
                        )
                    });
                    let candidate_strict_review_actionable = candidate_point.is_some_and(|point| {
                        release_review_is_actionable_warning_point(
                            point,
                            candidate_use_transitional_bridge,
                        )
                    });
                    let baseline_runtime_floor_hit = baseline_point.is_some_and(|point| {
                        release_review_hits_runtime_floor(point, baseline_runtime_thresholds)
                    });
                    let candidate_runtime_floor_hit = candidate_point.is_some_and(|point| {
                        release_review_hits_runtime_floor(point, candidate_runtime_thresholds)
                    });
                    Some(crate::ReleaseReviewScenarioPointComparison {
                        as_of_date: date,
                        baseline_p20d: baseline_point.map(|point| point.p_20d),
                        candidate_p20d: candidate_point.map(|point| point.p_20d),
                        baseline_p60d: baseline_point.map(|point| point.p_60d),
                        candidate_p60d: candidate_point.map(|point| point.p_60d),
                        baseline_posture: baseline_point
                            .map(release_review_posture_name)
                            .map(str::to_string),
                        candidate_posture: candidate_point
                            .map(release_review_posture_name)
                            .map(str::to_string),
                        baseline_time_bucket: baseline_point
                            .map(release_review_time_bucket_name)
                            .map(str::to_string),
                        candidate_time_bucket: candidate_point
                            .map(release_review_time_bucket_name)
                            .map(str::to_string),
                        baseline_strict_review_actionable,
                        candidate_strict_review_actionable,
                        baseline_runtime_floor_hit,
                        candidate_runtime_floor_hit,
                        baseline_actionable: baseline_strict_review_actionable,
                        candidate_actionable: candidate_strict_review_actionable,
                        baseline_actionable_forward_5d_hits: baseline_actionable_hits
                            .get(&date)
                            .map(|(hit_count, _)| *hit_count),
                        candidate_actionable_forward_5d_hits: candidate_actionable_hits
                            .get(&date)
                            .map(|(hit_count, _)| *hit_count),
                        baseline_actionable_sustained: baseline_actionable_hits
                            .get(&date)
                            .map(|(_, sustained)| *sustained),
                        candidate_actionable_sustained: candidate_actionable_hits
                            .get(&date)
                            .map(|(_, sustained)| *sustained),
                        baseline_trigger_codes: baseline_point
                            .map(|point| point.posture_trigger_codes.clone())
                            .unwrap_or_default(),
                        candidate_trigger_codes: candidate_point
                            .map(|point| point.posture_trigger_codes.clone())
                            .unwrap_or_default(),
                        baseline_runtime_actionable_block_category: baseline_point.and_then(
                            |point| {
                                release_review_runtime_actionable_block_category(
                                    point,
                                    baseline_use_transitional_bridge,
                                    baseline_runtime_thresholds,
                                )
                                .map(str::to_string)
                            },
                        ),
                        candidate_runtime_actionable_block_category: candidate_point.and_then(
                            |point| {
                                release_review_runtime_actionable_block_category(
                                    point,
                                    candidate_use_transitional_bridge,
                                    candidate_runtime_thresholds,
                                )
                                .map(str::to_string)
                            },
                        ),
                        baseline_runtime_actionable_block_reason: baseline_point.and_then(
                            |point| {
                                release_review_runtime_actionable_block_reason(
                                    point,
                                    baseline_use_transitional_bridge,
                                    baseline_runtime_thresholds,
                                )
                            },
                        ),
                        candidate_runtime_actionable_block_reason: candidate_point.and_then(
                            |point| {
                                release_review_runtime_actionable_block_reason(
                                    point,
                                    candidate_use_transitional_bridge,
                                    candidate_runtime_thresholds,
                                )
                            },
                        ),
                        baseline_actionable_diagnostic: baseline_point.map(|point| {
                            release_review_actionable_diagnostic(
                                point,
                                baseline_use_transitional_bridge,
                                baseline_runtime_thresholds,
                            )
                        }),
                        candidate_actionable_diagnostic: candidate_point.map(|point| {
                            release_review_actionable_diagnostic(
                                point,
                                candidate_use_transitional_bridge,
                                candidate_runtime_thresholds,
                            )
                        }),
                    })
                })
                .collect::<Vec<_>>();

            let runtime_block_counts = release_review_runtime_block_counts(
                &baseline_pre_crisis_points,
                baseline_use_transitional_bridge,
                baseline_runtime_thresholds,
                &candidate_pre_crisis_points,
                candidate_use_transitional_bridge,
                candidate_runtime_thresholds,
            );
            let runtime_continuity_facet_counts = release_review_runtime_continuity_facet_counts(
                &baseline_pre_crisis_points,
                baseline_use_transitional_bridge,
                baseline_runtime_thresholds,
                &candidate_pre_crisis_points,
                candidate_use_transitional_bridge,
                candidate_runtime_thresholds,
            );
            let dominant_runtime_blocks =
                release_review_runtime_dominant_categories(&runtime_block_counts);
            let dominant_runtime_continuity_facets =
                release_review_runtime_dominant_categories(&runtime_continuity_facet_counts);

            Some(crate::ReleaseReviewScenarioFocusDiagnostic {
                baseline_primary_failure_mode: release_review_primary_failure_mode(
                    &dominant_runtime_blocks.baseline_categories,
                    dominant_runtime_blocks.baseline_count,
                    &dominant_runtime_continuity_facets.baseline_categories,
                    dominant_runtime_continuity_facets.baseline_count,
                ),
                candidate_primary_failure_mode: release_review_primary_failure_mode(
                    &dominant_runtime_blocks.candidate_categories,
                    dominant_runtime_blocks.candidate_count,
                    &dominant_runtime_continuity_facets.candidate_categories,
                    dominant_runtime_continuity_facets.candidate_count,
                ),
                dominant_runtime_blocks,
                dominant_runtime_continuity_facets,
                scenario_id: baseline.scenario_id.clone(),
                name: baseline.name.clone(),
                outcome: format!(
                    "{}_to_{}",
                    backtest_warning_state(baseline.actionable_lead_time_days),
                    backtest_warning_state(
                        candidate.and_then(|scenario| scenario.actionable_lead_time_days)
                    )
                ),
                window_start,
                window_end,
                crisis_start: baseline.crisis_start,
                crisis_end: baseline.crisis_end,
                baseline_first_l2_date: baseline.first_l2_date,
                candidate_first_l2_date: candidate.and_then(|scenario| scenario.first_l2_date),
                baseline_first_l3_date: baseline.first_l3_date,
                candidate_first_l3_date: candidate.and_then(|scenario| scenario.first_l3_date),
                baseline_first_non_normal_date,
                candidate_first_non_normal_date,
                baseline_actionable_point_count: baseline_pre_crisis_points
                    .iter()
                    .filter(|point| {
                        release_review_is_actionable_warning_point(
                            point,
                            baseline_use_transitional_bridge,
                        )
                    })
                    .count() as u32,
                candidate_actionable_point_count: candidate_pre_crisis_points
                    .iter()
                    .filter(|point| {
                        release_review_is_actionable_warning_point(
                            point,
                            candidate_use_transitional_bridge,
                        )
                    })
                    .count() as u32,
                baseline_runtime_floor_hit_point_count,
                candidate_runtime_floor_hit_point_count,
                baseline_max_p20d: release_review_max_metric(
                    &baseline_pre_crisis_points,
                    |point| point.p_20d,
                ),
                candidate_max_p20d: release_review_max_metric(
                    &candidate_pre_crisis_points,
                    |point| point.p_20d,
                ),
                baseline_max_p60d: release_review_max_metric(
                    &baseline_pre_crisis_points,
                    |point| point.p_60d,
                ),
                candidate_max_p60d: release_review_max_metric(
                    &candidate_pre_crisis_points,
                    |point| point.p_60d,
                ),
                baseline_first_runtime_floor_hit_without_l3_date:
                    baseline_first_runtime_floor_hit_without_l3
                        .as_ref()
                        .map(|(date, _)| *date),
                candidate_first_runtime_floor_hit_without_l3_date:
                    candidate_first_runtime_floor_hit_without_l3
                        .as_ref()
                        .map(|(date, _)| *date),
                baseline_first_runtime_floor_hit_without_l3_reason:
                    baseline_first_runtime_floor_hit_without_l3
                        .as_ref()
                        .map(|(_, reason)| reason.clone()),
                candidate_first_runtime_floor_hit_without_l3_reason:
                    candidate_first_runtime_floor_hit_without_l3
                        .as_ref()
                        .map(|(_, reason)| reason.clone()),
                runtime_block_counts,
                runtime_continuity_facet_counts,
                interesting_points,
            })
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));
    rows
}

fn scenario_requires_focus_review(
    baseline: &BacktestScenarioSummary,
    candidate: Option<&BacktestScenarioSummary>,
) -> bool {
    baseline.first_l2_date != candidate.and_then(|scenario| scenario.first_l2_date)
        || baseline.first_l3_date != candidate.and_then(|scenario| scenario.first_l3_date)
        || baseline.lead_time_days != candidate.and_then(|scenario| scenario.lead_time_days)
        || baseline.actionable_lead_time_days
            != candidate.and_then(|scenario| scenario.actionable_lead_time_days)
        || baseline.false_positive_count
            != candidate
                .map(|scenario| scenario.false_positive_count)
                .unwrap_or_default()
        || scenario_has_structural_warning_without_actionable(baseline)
        || candidate.is_some_and(scenario_has_structural_warning_without_actionable)
}

fn scenario_has_structural_warning_without_actionable(scenario: &BacktestScenarioSummary) -> bool {
    scenario.lead_time_days.is_some() && scenario.actionable_lead_time_days.is_none()
}

fn release_review_first_non_normal_date(points: &[&AssessmentHistoryPoint]) -> Option<NaiveDate> {
    points
        .iter()
        .find(|point| release_review_point_is_non_normal(point))
        .map(|point| point.as_of_date)
}

fn release_review_max_metric(
    points: &[&AssessmentHistoryPoint],
    accessor: impl Fn(&AssessmentHistoryPoint) -> f64,
) -> Option<f64> {
    points
        .iter()
        .map(|point| accessor(point))
        .max_by(|left, right| left.total_cmp(right))
}

fn release_review_first_runtime_floor_hit_without_l3(
    points: &[&AssessmentHistoryPoint],
    use_transitional_bridge: bool,
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> Option<(NaiveDate, String)> {
    points.iter().find_map(|point| {
        release_review_runtime_actionable_block_reason(point, use_transitional_bridge, thresholds)
            .map(|reason| (point.as_of_date, reason))
    })
}

fn release_review_hits_runtime_floor(
    point: &AssessmentHistoryPoint,
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> bool {
    let Some(thresholds) = thresholds else {
        return false;
    };
    point.p_60d >= thresholds.prepare_p60d
        || point.p_20d >= thresholds.hedge_p20d
        || point.p_5d >= thresholds.defend_p5d
}

fn release_review_actionable_diagnostic(
    point: &AssessmentHistoryPoint,
    use_transitional_bridge: bool,
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> String {
    if release_review_is_actionable_warning_point(point, use_transitional_bridge) {
        return "actionable".to_string();
    }

    let runtime_floor_hit = release_review_hits_runtime_floor(point, thresholds);
    let mut review_gate_gaps = Vec::new();
    if point.p_20d < 0.18 {
        review_gate_gaps.push(format!("p20d {} < 18%", crate::format_pct(point.p_20d)));
    }
    if point.p_60d < 0.45 {
        review_gate_gaps.push(format!("p60d {} < 45%", crate::format_pct(point.p_60d)));
    }
    if !review_gate_gaps.is_empty() {
        let joined = review_gate_gaps.join(", ");
        return if runtime_floor_hit {
            format!("hit runtime floor, but review gate still needs {joined}")
        } else {
            joined
        };
    }

    if matches!(point.posture, DecisionPosture::Normal)
        && matches!(point.time_to_risk_bucket, TimeToRiskBucket::Normal)
    {
        return if runtime_floor_hit {
            "hit runtime floor, but posture/bucket stayed normal".to_string()
        } else {
            "posture/bucket stayed normal".to_string()
        };
    }

    if matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
        && point.overall_score < 62.0
        && point.external_shock_score < 48.0
    {
        return "months setup lacked score confirmation".to_string();
    }

    if matches!(point.posture, DecisionPosture::Prepare)
        && point.overall_score < 60.0
        && point.external_shock_score < 46.0
        && !release_review_has_strong_prepare_trigger_code(point)
    {
        return "prepare setup lacked confirmation".to_string();
    }

    if use_transitional_bridge
        && matches!(point.posture, DecisionPosture::Prepare)
        && point.overall_score < 58.0
    {
        return "prepare bridge not armed".to_string();
    }

    if use_transitional_bridge
        && matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
        && point.overall_score < 58.0
    {
        return "months bridge not armed".to_string();
    }

    "review L3 gate not satisfied".to_string()
}

fn release_review_runtime_actionable_block_category(
    point: &AssessmentHistoryPoint,
    use_transitional_bridge: bool,
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> Option<&'static str> {
    if release_review_is_actionable_warning_point(point, use_transitional_bridge)
        || !release_review_hits_runtime_floor(point, thresholds)
    {
        return None;
    }

    if point.p_20d < 0.18 || point.p_60d < 0.45 {
        return Some("review_gate_gap");
    }

    if matches!(point.posture, DecisionPosture::Normal)
        && matches!(point.time_to_risk_bucket, TimeToRiskBucket::Normal)
    {
        return Some("posture_bucket_normal");
    }

    if matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
        && point.overall_score < 62.0
        && point.external_shock_score < 48.0
    {
        return Some("months_score_confirmation");
    }

    if matches!(point.posture, DecisionPosture::Prepare)
        && point.overall_score < 60.0
        && point.external_shock_score < 46.0
        && !release_review_has_strong_prepare_trigger_code(point)
    {
        return Some("prepare_score_confirmation");
    }

    if use_transitional_bridge
        && matches!(point.posture, DecisionPosture::Prepare)
        && point.overall_score < 58.0
    {
        return Some("prepare_bridge_not_armed");
    }

    if use_transitional_bridge
        && matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
        && point.overall_score < 58.0
    {
        return Some("months_bridge_not_armed");
    }

    Some("review_l3_gate_not_satisfied")
}

fn release_review_runtime_actionable_block_reason(
    point: &AssessmentHistoryPoint,
    use_transitional_bridge: bool,
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> Option<String> {
    release_review_runtime_actionable_block_category(point, use_transitional_bridge, thresholds)
        .map(|_| release_review_actionable_diagnostic(point, use_transitional_bridge, thresholds))
}

fn release_review_runtime_gate_gap_facet(point: &AssessmentHistoryPoint) -> &'static str {
    match (point.p_20d < 0.18, point.p_60d < 0.45) {
        (true, true) => "p20d_and_p60d",
        (true, false) => "p20d_only",
        (false, true) => "p60d_only",
        (false, false) => "none",
    }
}

fn release_review_trigger_family(point: &AssessmentHistoryPoint) -> &'static str {
    if point.posture_trigger_codes.is_empty() {
        return "none";
    }

    let mut has_prepare = false;
    let mut has_hedge = false;
    let mut has_defend = false;
    let mut has_other = false;
    for code in &point.posture_trigger_codes {
        if code.starts_with("prepare_") {
            has_prepare = true;
        } else if code.starts_with("hedge_") {
            has_hedge = true;
        } else if code.starts_with("defend_") {
            has_defend = true;
        } else {
            has_other = true;
        }
    }

    let family_count = [has_prepare, has_hedge, has_defend, has_other]
        .into_iter()
        .filter(|present| *present)
        .count();
    if family_count > 1 {
        return "mixed";
    }
    if has_prepare {
        "prepare"
    } else if has_hedge {
        "hedge"
    } else if has_defend {
        "defend"
    } else {
        "other"
    }
}

fn release_review_runtime_confirmation_facet(
    point: &AssessmentHistoryPoint,
    use_transitional_bridge: bool,
) -> &'static str {
    if matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
        && point.overall_score < 62.0
        && point.external_shock_score < 48.0
    {
        return "months_score_low";
    }

    if matches!(point.posture, DecisionPosture::Prepare)
        && point.overall_score < 60.0
        && point.external_shock_score < 46.0
        && !release_review_has_strong_prepare_trigger_code(point)
    {
        return "prepare_score_low";
    }

    if use_transitional_bridge
        && matches!(point.posture, DecisionPosture::Prepare)
        && point.overall_score < 58.0
    {
        return "prepare_bridge_low";
    }

    if use_transitional_bridge
        && matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
        && point.overall_score < 58.0
    {
        return "months_bridge_low";
    }

    "ok_or_not_needed"
}

fn release_review_runtime_continuity_facets(
    point: &AssessmentHistoryPoint,
    use_transitional_bridge: bool,
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> Vec<String> {
    if release_review_runtime_actionable_block_category(point, use_transitional_bridge, thresholds)
        .is_none()
    {
        return Vec::new();
    }

    vec![
        format!("posture:{}", release_review_posture_name(point)),
        format!("bucket:{}", release_review_time_bucket_name(point)),
        format!("trigger:{}", release_review_trigger_family(point)),
        format!("gate_gap:{}", release_review_runtime_gate_gap_facet(point)),
        format!(
            "confirmation:{}",
            release_review_runtime_confirmation_facet(point, use_transitional_bridge)
        ),
    ]
}

fn release_review_runtime_block_counts(
    baseline_points: &[&AssessmentHistoryPoint],
    baseline_use_transitional_bridge: bool,
    baseline_thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
    candidate_points: &[&AssessmentHistoryPoint],
    candidate_use_transitional_bridge: bool,
    candidate_thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> Vec<crate::ReleaseReviewRuntimeBlockCount> {
    let collect_counts =
        |points: &[&AssessmentHistoryPoint],
         use_transitional_bridge: bool,
         thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>| {
            points
                .iter()
                .fold(BTreeMap::<String, u32>::new(), |mut acc, point| {
                    if let Some(category) = release_review_runtime_actionable_block_category(
                        point,
                        use_transitional_bridge,
                        thresholds,
                    ) {
                        *acc.entry(category.to_string()).or_default() += 1;
                    }
                    acc
                })
        };

    let baseline_counts = collect_counts(
        baseline_points,
        baseline_use_transitional_bridge,
        baseline_thresholds,
    );
    let candidate_counts = collect_counts(
        candidate_points,
        candidate_use_transitional_bridge,
        candidate_thresholds,
    );
    let categories = baseline_counts
        .keys()
        .chain(candidate_counts.keys())
        .cloned()
        .collect::<BTreeSet<_>>();

    categories
        .into_iter()
        .map(|category| {
            let baseline_count = baseline_counts.get(&category).copied().unwrap_or_default();
            let candidate_count = candidate_counts.get(&category).copied().unwrap_or_default();
            crate::ReleaseReviewRuntimeBlockCount {
                category,
                baseline_count,
                candidate_count,
                delta: i64::from(candidate_count) - i64::from(baseline_count),
            }
        })
        .collect()
}

fn release_review_runtime_dominant_categories(
    counts: &[crate::ReleaseReviewRuntimeBlockCount],
) -> crate::ReleaseReviewRuntimeDominantCategories {
    let baseline_count = counts
        .iter()
        .map(|row| row.baseline_count)
        .max()
        .unwrap_or(0);
    let candidate_count = counts
        .iter()
        .map(|row| row.candidate_count)
        .max()
        .unwrap_or(0);

    crate::ReleaseReviewRuntimeDominantCategories {
        baseline_categories: if baseline_count == 0 {
            Vec::new()
        } else {
            counts
                .iter()
                .filter(|row| row.baseline_count == baseline_count)
                .map(|row| row.category.clone())
                .collect()
        },
        baseline_count,
        candidate_categories: if candidate_count == 0 {
            Vec::new()
        } else {
            counts
                .iter()
                .filter(|row| row.candidate_count == candidate_count)
                .map(|row| row.category.clone())
                .collect()
        },
        candidate_count,
    }
}

fn release_review_primary_failure_mode(
    dominant_blocks: &[String],
    dominant_block_count: u32,
    dominant_facets: &[String],
    dominant_facet_count: u32,
) -> Option<String> {
    if dominant_block_count > 0 {
        if dominant_blocks
            .iter()
            .any(|category| category == "review_gate_gap")
        {
            return Some("strict_gate_mismatch".to_string());
        }
        if dominant_blocks
            .iter()
            .any(|category| category == "posture_bucket_normal")
        {
            return Some("posture_continuity_failure".to_string());
        }
        if dominant_blocks
            .iter()
            .any(|category| category.ends_with("score_confirmation"))
        {
            return Some("score_confirmation_failure".to_string());
        }
        if dominant_blocks
            .iter()
            .any(|category| category.ends_with("bridge_not_armed"))
        {
            return Some("transitional_bridge_failure".to_string());
        }
        return Some("residual_review_l3_failure".to_string());
    }

    if dominant_facet_count > 0 {
        if dominant_facets
            .iter()
            .any(|facet| facet == "posture:normal")
            || dominant_facets.iter().any(|facet| facet == "bucket:normal")
            || dominant_facets.iter().any(|facet| facet == "trigger:none")
        {
            return Some("posture_continuity_failure".to_string());
        }
        if dominant_facets
            .iter()
            .any(|facet| facet.starts_with("gate_gap:") && facet != "gate_gap:none")
        {
            return Some("strict_gate_mismatch".to_string());
        }
        if dominant_facets.iter().any(|facet| {
            facet.starts_with("confirmation:") && facet != "confirmation:ok_or_not_needed"
        }) {
            return Some("score_confirmation_failure".to_string());
        }
        return Some("runtime_continuity_failure".to_string());
    }

    None
}

fn release_review_runtime_continuity_facet_counts(
    baseline_points: &[&AssessmentHistoryPoint],
    baseline_use_transitional_bridge: bool,
    baseline_thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
    candidate_points: &[&AssessmentHistoryPoint],
    candidate_use_transitional_bridge: bool,
    candidate_thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> Vec<crate::ReleaseReviewRuntimeBlockCount> {
    let collect_counts =
        |points: &[&AssessmentHistoryPoint],
         use_transitional_bridge: bool,
         thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>| {
            points
                .iter()
                .fold(BTreeMap::<String, u32>::new(), |mut acc, point| {
                    for facet in release_review_runtime_continuity_facets(
                        point,
                        use_transitional_bridge,
                        thresholds,
                    ) {
                        *acc.entry(facet).or_default() += 1;
                    }
                    acc
                })
        };

    let baseline_counts = collect_counts(
        baseline_points,
        baseline_use_transitional_bridge,
        baseline_thresholds,
    );
    let candidate_counts = collect_counts(
        candidate_points,
        candidate_use_transitional_bridge,
        candidate_thresholds,
    );
    let categories = baseline_counts
        .keys()
        .chain(candidate_counts.keys())
        .cloned()
        .collect::<BTreeSet<_>>();

    categories
        .into_iter()
        .map(|category| {
            let baseline_count = baseline_counts.get(&category).copied().unwrap_or_default();
            let candidate_count = candidate_counts.get(&category).copied().unwrap_or_default();
            crate::ReleaseReviewRuntimeBlockCount {
                category,
                baseline_count,
                candidate_count,
                delta: i64::from(candidate_count) - i64::from(baseline_count),
            }
        })
        .collect()
}

fn release_review_actionable_forward_hits_by_date(
    points: &[&AssessmentHistoryPoint],
    use_transitional_bridge: bool,
) -> BTreeMap<NaiveDate, (u32, bool)> {
    points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            let end = (index + RELEASE_REVIEW_SIGNAL_WINDOW).min(points.len());
            let window = &points[index..end];
            let hit_count = window
                .iter()
                .filter(|candidate| {
                    release_review_is_actionable_warning_point(candidate, use_transitional_bridge)
                })
                .count();
            let required_hits = RELEASE_REVIEW_SIGNAL_MIN_HITS.min(window.len());
            let sustained =
                release_review_is_actionable_warning_point(point, use_transitional_bridge)
                    && hit_count >= required_hits;
            (point.as_of_date, (hit_count as u32, sustained))
        })
        .collect()
}

fn release_review_point_is_non_normal(point: &AssessmentHistoryPoint) -> bool {
    !matches!(point.posture, DecisionPosture::Normal)
        || !matches!(point.time_to_risk_bucket, TimeToRiskBucket::Normal)
}

fn release_review_point_is_interesting(
    baseline_point: Option<&AssessmentHistoryPoint>,
    candidate_point: Option<&AssessmentHistoryPoint>,
    baseline_use_transitional_bridge: bool,
    candidate_use_transitional_bridge: bool,
) -> bool {
    let baseline_actionable = baseline_point.is_some_and(|point| {
        release_review_is_actionable_warning_point(point, baseline_use_transitional_bridge)
    });
    let candidate_actionable = candidate_point.is_some_and(|point| {
        release_review_is_actionable_warning_point(point, candidate_use_transitional_bridge)
    });
    if baseline_actionable
        || candidate_actionable
        || baseline_point.is_some_and(release_review_point_is_non_normal)
        || candidate_point.is_some_and(release_review_point_is_non_normal)
    {
        return true;
    }

    match (baseline_point, candidate_point) {
        (Some(baseline), Some(candidate)) => {
            baseline.posture != candidate.posture
                || baseline.time_to_risk_bucket != candidate.time_to_risk_bucket
                || baseline.posture_trigger_codes != candidate.posture_trigger_codes
                || (baseline.p_20d - candidate.p_20d).abs() >= 0.05
                || (baseline.p_60d - candidate.p_60d).abs() >= 0.05
        }
        _ => false,
    }
}

fn release_review_uses_transitional_actionable_bridge(
    method: &crate::AuditMethodResponseWire,
) -> bool {
    !(method.method.probability_mode == "formal_bundle_v1"
        && method.method.label_version == "formal_label_v1_main"
        && method
            .method
            .feature_set_version
            .starts_with("feature_formal_v1_main"))
}

fn release_review_has_strong_prepare_trigger_code(point: &AssessmentHistoryPoint) -> bool {
    point.posture_trigger_codes.iter().any(|code| {
        matches!(
            code.as_str(),
            "prepare_p60d_structural"
                | "prepare_structural_downgrade"
                | "prepare_carry_structural"
                | "prepare_external_structural"
        )
    })
}

fn release_review_is_actionable_warning_point(
    point: &AssessmentHistoryPoint,
    use_transitional_bridge: bool,
) -> bool {
    let strict_short_horizon_signal =
        matches!(
            point.posture,
            DecisionPosture::Hedge | DecisionPosture::Defend
        ) || (matches!(point.time_to_risk_bucket, TimeToRiskBucket::Now)
            && point.overall_score >= 60.0
            && point.p_5d >= 0.18)
            || (matches!(point.time_to_risk_bucket, TimeToRiskBucket::Weeks)
                && point.overall_score >= 58.0
                && point.p_20d >= 0.25
                && point.external_shock_score >= 44.0);

    let high_probability_prepare_signal = matches!(point.posture, DecisionPosture::Prepare)
        && point.p_20d >= 0.18
        && point.p_60d >= 0.45
        && ((point.overall_score >= 60.0 && point.external_shock_score >= 46.0)
            || (point.overall_score >= 53.0
                && !matches!(point.time_to_risk_bucket, TimeToRiskBucket::Normal)
                && release_review_has_strong_prepare_trigger_code(point)));
    let high_probability_months_signal =
        matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
            && point.overall_score >= 62.0
            && point.p_20d >= 0.18
            && point.p_60d >= 0.45
            && point.external_shock_score >= 48.0;

    let prepare_bridge_signal = use_transitional_bridge
        && matches!(point.posture, DecisionPosture::Prepare)
        && point.overall_score >= 58.0
        && point.external_shock_score >= 46.0;
    let months_bridge_signal = use_transitional_bridge
        && matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
        && point.overall_score >= 58.0
        && point.external_shock_score >= 42.0;

    strict_short_horizon_signal
        || high_probability_prepare_signal
        || high_probability_months_signal
        || prepare_bridge_signal
        || months_bridge_signal
}

fn release_review_posture_name(point: &AssessmentHistoryPoint) -> &'static str {
    match point.posture {
        DecisionPosture::Normal => "normal",
        DecisionPosture::Prepare => "prepare",
        DecisionPosture::Hedge => "hedge",
        DecisionPosture::Defend => "defend",
    }
}

fn release_review_time_bucket_name(point: &AssessmentHistoryPoint) -> &'static str {
    match point.time_to_risk_bucket {
        TimeToRiskBucket::Normal => "normal",
        TimeToRiskBucket::Months => "months",
        TimeToRiskBucket::Weeks => "weeks",
        TimeToRiskBucket::Now => "now",
    }
}

fn backtest_warning_state(actionable_lead_time_days: Option<i64>) -> &'static str {
    match actionable_lead_time_days {
        Some(days) if days >= 7 => "timely",
        Some(_) => "late_only",
        None => "missed",
    }
}

fn scalar_metric(baseline: f64, candidate: f64) -> crate::ReleaseReviewScalarMetric {
    crate::ReleaseReviewScalarMetric {
        baseline,
        candidate,
        delta: candidate - baseline,
    }
}

fn count_metric(baseline: u32, candidate: u32) -> crate::ReleaseReviewCountMetric {
    crate::ReleaseReviewCountMetric {
        baseline,
        candidate,
        delta: i64::from(candidate) - i64::from(baseline),
    }
}

fn build_release_review_recommendation(
    regressions: &[String],
    candidate_has_actionability: bool,
    historical_audit_actions: &[crate::ReleaseReviewHistoricalAuditActionSummary],
) -> String {
    let baseline_cold_only = regressions.len() == 1
        && regressions[0].contains("relative guardrails alone are not a sufficient promotion test");
    let candidate_regression_workstreams = release_review_action_workstream_labels(
        historical_audit_actions,
        "candidate_reject_or_retrain",
    );
    let shared_blocker_workstreams = release_review_action_workstream_labels(
        historical_audit_actions,
        "shared_blocker_fix_before_promotion",
    );
    let baseline_fix_workstreams =
        release_review_action_workstream_labels(historical_audit_actions, "baseline_research_fix");
    if regressions.is_empty() {
        if !candidate_regression_workstreams.is_empty() {
            format!(
                "候选版虽然通过了当前护栏，但历史审计显示它在 {} 上出现新增退化。当前不应直接晋升，应先回到训练目标、阈值或 runtime policy 改动复核。",
                candidate_regression_workstreams.join(", ")
            )
        } else if !shared_blocker_workstreams.is_empty() {
            format!(
                "候选版虽然通过了当前护栏，但历史审计显示 {} 仍是 baseline 共性短板和 candidate 未修复的共享 blocker。当前更合适的动作是先修这条主线，再决定是否晋升。",
                shared_blocker_workstreams.join(", ")
            )
        } else if !baseline_fix_workstreams.is_empty() && candidate_has_actionability {
            format!(
                "候选版通过当前概率头、运行时与动作层护栏，并且没有继续继承 baseline 在 {} 上的历史短板，可进入下一轮人工复核。formal main 仍需继续补这条长期结构修复线。",
                baseline_fix_workstreams.join(", ")
            )
        } else if candidate_has_actionability {
            "候选版通过当前概率头、运行时与动作层护栏，可进入下一轮人工复核。仍需结合标签质量、样本覆盖和前端解释能力决定是否晋升。".to_string()
        } else {
            "候选版通过当前概率头与运行时护栏，可进入下一轮人工复核。仍需结合标签质量、样本覆盖和前端解释能力决定是否晋升。".to_string()
        }
    } else if baseline_cold_only {
        "候选版已经通过当前概率头、相对运行时护栏与动作精度约束，当前唯一阻塞是 baseline 仍属于全程 normal 的冷模型，因此这次 review 还不能直接支持“替代默认正式版”。更合适的结论是：该候选版可以视为新的 active_experimental 研究基线，但要晋升为默认正式版，仍需补足绝对提前量门槛与样本/标签治理证据。".to_string()
    } else if !candidate_regression_workstreams.is_empty() {
        format!(
            "候选版未通过当前 review，且历史审计显示它在 {} 上出现新增退化，不应替代当前默认线上版本。应先回到训练目标、标签口径、阈值或 runtime policy 改动复核。",
            candidate_regression_workstreams.join(", ")
        )
    } else if !shared_blocker_workstreams.is_empty() {
        format!(
            "候选版未通过当前 review，关键阻塞仍集中在 {}。这些同时是 baseline 共性短板和 candidate 未修复问题，因此当前不应晋升，需先把共享 blocker 作为前置修复项。",
            shared_blocker_workstreams.join(", ")
        )
    } else if candidate_has_actionability {
        "候选版未通过当前概率头 / 运行时 / 动作层护栏，不应替代当前默认线上版本。应先修正训练目标、标签口径、样本切分或样本治理，再重新训练复核。".to_string()
    } else {
        "候选版未通过当前概率头 / 运行时护栏，不应替代当前默认线上版本。应先修正训练目标、标签口径或样本治理，再重新训练复核。".to_string()
    }
}

fn release_review_action_workstream_labels(
    actions: &[crate::ReleaseReviewHistoricalAuditActionSummary],
    action_type: &str,
) -> Vec<String> {
    let mut labels = Vec::new();
    for action in actions.iter().filter(|row| row.action_type == action_type) {
        let label = match action.workstream.as_str() {
            "strict_review_vs_runtime_mapping" => "strict gate vs runtime floor",
            "posture_continuity" => "posture continuity",
            "score_confirmation" => "score confirmation",
            "transitional_bridge" => "transitional bridge",
            _ => "residual release-review audit",
        }
        .to_string();
        if !labels.contains(&label) {
            labels.push(label);
        }
    }
    labels
}

fn print_release_review_summary(report: &crate::ReleaseReviewEnvelope) {
    println!("Review comparison:");
    println!(
        "  timely_warning_rate   {} -> {}",
        crate::format_pct(report.comparison.timely_warning_rate.baseline),
        crate::format_pct(report.comparison.timely_warning_rate.candidate)
    );
    println!(
        "  strict_actionable_point_count  {} -> {}",
        report.comparison.strict_actionable_point_count.baseline,
        report.comparison.strict_actionable_point_count.candidate
    );
    println!(
        "  runtime_floor_hit_count       {} -> {}",
        report.comparison.runtime_floor_hit_count.baseline,
        report.comparison.runtime_floor_hit_count.candidate
    );
    println!(
        "  actionable_precision  {} -> {}",
        crate::format_pct(report.comparison.actionable_precision.baseline),
        crate::format_pct(report.comparison.actionable_precision.candidate)
    );
    println!(
        "  longest_false_positive_episode_days  {} -> {}",
        report
            .comparison
            .longest_false_positive_episode_days
            .baseline,
        report
            .comparison
            .longest_false_positive_episode_days
            .candidate
    );
    let runtime_takeaways = crate::release_review_runtime_separation_takeaways(
        &report.comparison.runtime_separation_summary,
    );
    if !runtime_takeaways.is_empty() {
        println!("Runtime separation takeaways:");
        for takeaway in runtime_takeaways {
            println!("  - {takeaway}");
        }
    }
    let failure_mode_summary =
        crate::summarize_release_review_failure_modes(&report.scenario_focus);
    if !failure_mode_summary.is_empty() {
        println!("Failure mode summary:");
        for row in failure_mode_summary {
            println!(
                "  - {} baseline={} ({}) candidate={} ({})",
                row.failure_mode,
                row.baseline_count,
                if row.baseline_scenarios.is_empty() {
                    "—".to_string()
                } else {
                    row.baseline_scenarios.join(", ")
                },
                row.candidate_count,
                if row.candidate_scenarios.is_empty() {
                    "—".to_string()
                } else {
                    row.candidate_scenarios.join(", ")
                }
            );
        }
    }
    if !report.historical_audit_workstreams.is_empty() {
        println!("Historical audit workstream summary:");
        let takeaways =
            crate::release_review_historical_audit_takeaways(&report.historical_audit_workstreams);
        if !takeaways.is_empty() {
            println!("Historical audit takeaways:");
            for takeaway in takeaways {
                println!("  - {takeaway}");
            }
        }
        for row in &report.historical_audit_workstreams {
            println!(
                "  - {} scenarios={} ({}) protected={} families={} roles={} review={}",
                row.workstream,
                row.scenario_count,
                row.scenarios.join(", "),
                row.protected_count,
                row.scenario_families.join(", "),
                row.training_roles.join(", "),
                row.suggested_review
            );
        }
    }
    if !report.historical_audit_attribution.is_empty() {
        println!("Historical audit attribution:");
        for row in &report.historical_audit_attribution {
            println!(
                "  - {} attribution={} scenarios={} protected={} baseline={} ({}) candidate={} ({})",
                row.workstream,
                row.attribution,
                row.scenario_count,
                row.protected_count,
                row.baseline_count,
                if row.baseline_scenarios.is_empty() {
                    "—".to_string()
                } else {
                    row.baseline_scenarios.join(", ")
                },
                row.candidate_count,
                if row.candidate_scenarios.is_empty() {
                    "—".to_string()
                } else {
                    row.candidate_scenarios.join(", ")
                }
            );
            println!("    {}", row.explanation);
        }
    }
    if !report.historical_audit_actions.is_empty() {
        println!("Historical audit actions:");
        for row in &report.historical_audit_actions {
            println!(
                "  - {} attribution={} action={} scenarios={} protected={}",
                row.workstream,
                row.attribution,
                row.action_type,
                row.scenario_count,
                row.protected_count,
            );
            println!("    {}", row.recommendation);
        }
    }
    if !report.historical_audit_priorities.is_empty() {
        println!("Historical audit priorities:");
        for row in &report.historical_audit_priorities {
            println!(
                "  - {} [{}] workstream={} baseline={} candidate={} protected={} review={}",
                row.scenario_name,
                row.training_role,
                row.primary_workstream,
                row.baseline_failure_mode,
                row.candidate_failure_mode,
                if row.protected_window { "yes" } else { "no" },
                row.suggested_review
            );
        }
    }
    if report.probability_guard_regressions.is_empty() {
        println!("Probability guard summary:");
        println!("  no bundle-level probability guard regressions");
    } else {
        println!("Probability guard summary:");
        for regression in &report.probability_guard_regressions {
            println!("  - {regression}");
        }
    }
    if report.candidate_actionability_review.enabled {
        println!("Actionability guard summary:");
        for level in &report.candidate_actionability_review.levels {
            println!(
                "  {:>7} scenarios={} on_time={} late_only={} missed={}",
                crate::actionability_level_text(level.level),
                level.scenario_count,
                crate::format_optional_pct(level.on_time_rate),
                crate::format_optional_pct(level.late_only_rate),
                crate::format_optional_pct(level.missed_rate),
            );
        }
    }
    println!("  recommendation        {}", report.recommendation);
}

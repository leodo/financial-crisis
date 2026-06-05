use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use anyhow::{bail, Context};
use chrono::Utc;
use fc_domain::{AssessmentHistoryPoint, ModelReleaseRecord};

use super::{
    build_release_actionability_review, compare_actionability_guardrails,
    compare_operational_guardrails, compare_probability_guardrails,
    compare_runtime_sanity_guardrails,
};
mod focus;

pub(crate) use focus::{
    build_release_review_backtest_scenario_comparisons,
    build_release_review_scenario_focus_diagnostics, release_review_structured_signal_counts,
};

struct ReleaseReviewComparisonInput<'a> {
    assessment: &'a fc_domain::AssessmentSnapshot,
    backtests: &'a [fc_domain::BacktestScenarioSummary],
    history: &'a [AssessmentHistoryPoint],
    method: &'a crate::AuditMethodResponseWire,
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

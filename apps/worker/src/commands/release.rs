use std::path::PathBuf;

use anyhow::{bail, Context};
use chrono::Utc;
use fc_domain::ModelReleaseRecord;

mod probability;
mod review;

pub(crate) use probability::{
    research_release_formal_probability_compare, research_release_formal_probability_slice,
    research_release_probability_slice,
};
#[cfg(test)]
pub(crate) use probability::{
    ReleaseFormalProbabilityCompareOptions, ReleaseFormalProbabilitySliceOptions,
    ReleaseProbabilitySliceOptions,
};
pub(crate) use review::{
    activate_release_for_review, research_release_review, restore_release_review_state,
    ReleaseReviewOptions,
};
#[cfg(test)]
pub(crate) use review::{
    build_release_review_backtest_scenario_comparisons,
    build_release_review_runtime_separation_comparisons,
    build_release_review_scenario_focus_diagnostics, release_review_structured_signal_counts,
};

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

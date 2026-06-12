use std::collections::BTreeMap;

use anyhow::Context;
use chrono::Utc;
use fc_domain::ModelReleaseRecord;

use super::{
    build_release_actionability_review, compare_actionability_guardrails,
    compare_operational_guardrails, compare_probability_guardrails,
    compare_release_review_count_guardrails, compare_runtime_sanity_guardrails,
};
mod comparison;
mod focus;
mod options;
mod snapshot;
mod summary;

#[cfg(test)]
pub(crate) use comparison::build_release_review_runtime_separation_comparisons;
pub(crate) use focus::{
    build_release_review_backtest_scenario_comparisons,
    build_release_review_scenario_focus_diagnostics, release_review_structured_signal_counts,
};
pub(crate) use options::ReleaseReviewOptions;
pub(crate) use snapshot::{activate_release_for_review, restore_release_review_state};

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
        anyhow::bail!(
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
            anyhow::bail!(
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
        anyhow::bail!("baseline release and candidate release must be different");
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
            anyhow::bail!(
                "release review failed and restore also failed:\nreview: {review_error:#}\nrestore: {restore_error:#}"
            );
        }
        anyhow::bail!("release review completed but restore failed: {restore_error:#}");
    }

    review_result?;
    println!(
        "Release review restored original active release {}.",
        original_active.manifest.release_id
    );
    Ok(())
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
    let baseline_runtime_snapshot = snapshot::fetch_release_review_runtime_snapshot(
        &options.api_reload_url,
        options.history_limit,
    )
    .await?;

    activate_release_for_review(
        store,
        market_scope,
        &candidate_release.manifest.release_id,
        options,
        "candidate",
    )
    .await?;
    let candidate_runtime_snapshot = snapshot::fetch_release_review_runtime_snapshot(
        &options.api_reload_url,
        options.history_limit,
    )
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
    let release_review_comparison = comparison::build_release_review_comparison(
        comparison::ReleaseReviewComparisonInput {
            assessment: &baseline_assessment,
            backtests: &baseline_runtime_snapshot.backtests,
            history: &baseline_runtime_snapshot.history,
            method: &baseline_runtime_snapshot.method,
        },
        comparison::ReleaseReviewComparisonInput {
            assessment: &candidate_assessment,
            backtests: &candidate_runtime_snapshot.backtests,
            history: &candidate_runtime_snapshot.history,
            method: &candidate_runtime_snapshot.method,
        },
        &baseline_runtime_review,
        &candidate_runtime_review,
    );
    let operational_regressions =
        compare_operational_guardrails(&baseline_assessment, &candidate_assessment);
    let actionability_regressions =
        compare_actionability_guardrails(&candidate_actionability_review);
    let runtime_sanity_regressions =
        compare_runtime_sanity_guardrails(&baseline_runtime_review, &candidate_runtime_review);
    let release_review_count_regressions =
        compare_release_review_count_guardrails(&release_review_comparison);
    let mut overall_regressions = operational_regressions.clone();
    overall_regressions.extend(probability_regressions.iter().cloned());
    overall_regressions.extend(actionability_regressions.iter().cloned());
    overall_regressions.extend(runtime_sanity_regressions.iter().cloned());
    overall_regressions.extend(release_review_count_regressions.iter().cloned());
    let scenario_focus = build_release_review_scenario_focus_diagnostics(
        &baseline_runtime_snapshot.backtests,
        &candidate_runtime_snapshot.backtests,
        &baseline_runtime_snapshot.history,
        &candidate_runtime_snapshot.history,
        &baseline_runtime_snapshot.method,
        &candidate_runtime_snapshot.method,
    );
    let (scenario_coverage_catalog, scenario_coverages) =
        crate::build_release_review_scenario_coverage(
            &build_release_review_backtest_scenario_comparisons(
                &baseline_runtime_snapshot.backtests,
                &candidate_runtime_snapshot.backtests,
            ),
            &scenario_focus,
        );
    let historical_audit_priorities = enrich_historical_audit_priorities_with_coverage(
        crate::summarize_release_review_historical_audit_priorities(&scenario_focus),
        &scenario_coverages,
    );
    let historical_audit_attribution =
        crate::summarize_release_review_historical_audit_attribution(&historical_audit_priorities);
    let historical_audit_actions =
        crate::summarize_release_review_historical_audit_actions(&historical_audit_attribution);
    let historical_audit_workstreams =
        crate::summarize_release_review_historical_audit_workstreams_with_focus(
            &historical_audit_priorities,
            &scenario_focus,
        );

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
        comparison: release_review_comparison,
        baseline_assessment,
        candidate_assessment,
        baseline_runtime_review,
        candidate_runtime_review,
        baseline_actionability_review,
        candidate_actionability_review,
        scenario_coverage_catalog,
        scenario_coverages,
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
        recommendation: summary::build_release_review_recommendation(
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
    summary::print_release_review_summary(&report);

    Ok(())
}

fn enrich_historical_audit_priorities_with_coverage(
    priorities: Vec<crate::ReleaseReviewHistoricalAuditPriority>,
    scenario_coverages: &[crate::ReleaseReviewScenarioCoverage],
) -> Vec<crate::ReleaseReviewHistoricalAuditPriority> {
    let coverage_by_id = scenario_coverages
        .iter()
        .map(|coverage| (coverage.scenario_id.as_str(), coverage))
        .collect::<BTreeMap<_, _>>();

    priorities
        .into_iter()
        .map(|mut priority| {
            if let Some(coverage) = coverage_by_id.get(priority.scenario_id.as_str()).copied() {
                priority.coverage_recommended_role = Some(coverage.recommended_role.clone());
                priority.coverage_grade = Some(coverage.coverage_grade.clone());
                priority.coverage_point_in_time_mode = Some(coverage.point_in_time_mode.clone());
                priority.coverage_current_status = Some(coverage.current_status.clone());
                priority.coverage_blocking_gaps = coverage.blocking_gaps.clone();
            }
            priority
        })
        .collect()
}

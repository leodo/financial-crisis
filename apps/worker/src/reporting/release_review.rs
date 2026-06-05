use std::fmt::Write;

use crate::ReleaseReviewEnvelope;

pub(crate) fn render_release_review_markdown_impl(report: &ReleaseReviewEnvelope) -> String {
    use crate::{
        format_bool_flag, format_optional_bool_flag, format_optional_count, format_optional_date,
        format_optional_date_with_lead, format_optional_date_with_reason, format_optional_days,
        format_optional_multiplier, format_optional_pct, format_pct, format_runtime_category_list,
        format_signed_count_delta, format_signed_pct_delta, format_trigger_codes, posture_text,
        release_review_historical_audit_takeaways, release_review_runtime_separation_takeaways,
        summarize_release_review_failure_modes, time_bucket_text,
    };

    let mut markdown = String::new();
    let verdict = if report.overall_guard_passed {
        "PASS"
    } else {
        "FAIL"
    };
    let _ = writeln!(markdown, "# Release Review");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "- Reviewed at: {}", report.reviewed_at);
    let _ = writeln!(markdown, "- Market scope: {}", report.market_scope);
    let _ = writeln!(
        markdown,
        "- History mode: {} (limit {})",
        report.history_mode, report.history_limit
    );
    let _ = writeln!(markdown, "- Verdict: {verdict}");
    let _ = writeln!(
        markdown,
        "- Original active release: {}",
        report.original_active_release_id
    );
    let _ = writeln!(
        markdown,
        "- Restored release after review: {}",
        report.restored_release_id
    );
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Releases");
    let _ = writeln!(markdown);
    let _ = writeln!(
        markdown,
        "| Role | Release ID | Prob Mode | PIT | Feature | Label | Status |"
    );
    let _ = writeln!(markdown, "| --- | --- | --- | --- | --- | --- | --- |");
    for (role, release) in [
        ("baseline", &report.baseline_release),
        ("candidate", &report.candidate_release),
    ] {
        let _ = writeln!(
            markdown,
            "| {} | {} | {} | {} | {} | {} | {} |",
            role,
            release.manifest.release_id,
            release.manifest.probability_mode,
            release.manifest.point_in_time_mode,
            release.manifest.feature_set_version,
            release.manifest.label_version,
            release.manifest.status
        );
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Current Runtime Snapshot");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "| Metric | Baseline | Candidate | Delta |");
    let _ = writeln!(markdown, "| --- | --- | --- | --- |");
    let _ = writeln!(
        markdown,
        "| p_5d | {} | {} | {} |",
        format_pct(report.comparison.current_p_5d.baseline),
        format_pct(report.comparison.current_p_5d.candidate),
        format_signed_pct_delta(report.comparison.current_p_5d.delta)
    );
    let _ = writeln!(
        markdown,
        "| p_20d | {} | {} | {} |",
        format_pct(report.comparison.current_p_20d.baseline),
        format_pct(report.comparison.current_p_20d.candidate),
        format_signed_pct_delta(report.comparison.current_p_20d.delta)
    );
    let _ = writeln!(
        markdown,
        "| p_60d | {} | {} | {} |",
        format_pct(report.comparison.current_p_60d.baseline),
        format_pct(report.comparison.current_p_60d.candidate),
        format_signed_pct_delta(report.comparison.current_p_60d.delta)
    );
    let _ = writeln!(
        markdown,
        "| Posture | {} | {} | — |",
        posture_text(report.baseline_assessment.posture),
        posture_text(report.candidate_assessment.posture)
    );
    let _ = writeln!(
        markdown,
        "| Time bucket | {} | {} | — |",
        time_bucket_text(report.baseline_assessment.time_to_risk_bucket),
        time_bucket_text(report.candidate_assessment.time_to_risk_bucket)
    );
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Runtime Diagnostics");
    let _ = writeln!(markdown);
    render_release_runtime_review_markdown(
        &mut markdown,
        "baseline",
        &report.baseline_runtime_review,
    );
    let _ = writeln!(markdown);
    render_release_runtime_review_markdown(
        &mut markdown,
        "candidate",
        &report.candidate_runtime_review,
    );
    if !report.comparison.runtime_separation_summary.is_empty() {
        let _ = writeln!(markdown);
        let _ = writeln!(markdown, "## Runtime Separation Comparison");
        let _ = writeln!(markdown);
        let _ = writeln!(
            markdown,
            "| Horizon | Baseline diagnosis | Candidate diagnosis | Baseline floor | Candidate floor | Baseline early regime | Candidate early regime | Baseline early P | Candidate early P | Baseline normal P | Candidate normal P | Baseline EW gap | Candidate EW gap | Baseline floor gap | Candidate floor gap | Baseline EW lift | Candidate EW lift | Baseline hit rate | Candidate hit rate |"
        );
        let _ = writeln!(
            markdown,
            "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
        );
        for row in &report.comparison.runtime_separation_summary {
            let _ = writeln!(
                markdown,
                "| {}d | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                row.horizon_days,
                row.baseline_diagnosis,
                row.candidate_diagnosis,
                format_optional_pct(row.baseline_threshold),
                format_optional_pct(row.candidate_threshold),
                row.baseline_early_warning_regime,
                row.candidate_early_warning_regime,
                format_optional_pct(row.baseline_early_warning_avg_probability),
                format_optional_pct(row.candidate_early_warning_avg_probability),
                format_optional_pct(row.baseline_normal_avg_probability),
                format_optional_pct(row.candidate_normal_avg_probability),
                format_optional_pct(row.baseline_early_warning_gap_vs_normal),
                format_optional_pct(row.candidate_early_warning_gap_vs_normal),
                format_optional_pct(row.baseline_floor_gap),
                format_optional_pct(row.candidate_floor_gap),
                format_optional_multiplier(row.baseline_early_warning_lift_vs_normal),
                format_optional_multiplier(row.candidate_early_warning_lift_vs_normal),
                format_optional_pct(row.baseline_threshold_hit_rate),
                format_optional_pct(row.candidate_threshold_hit_rate),
            );
        }
        let takeaways = release_review_runtime_separation_takeaways(
            &report.comparison.runtime_separation_summary,
        );
        if !takeaways.is_empty() {
            let _ = writeln!(markdown);
            let _ = writeln!(markdown, "### Runtime Interpretation");
            let _ = writeln!(markdown);
            for takeaway in takeaways {
                let _ = writeln!(markdown, "- {takeaway}");
            }
        }
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Backtest Guardrails");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "| Metric | Baseline | Candidate | Delta |");
    let _ = writeln!(markdown, "| --- | --- | --- | --- |");
    let _ = writeln!(
        markdown,
        "| timely_warning_rate | {} | {} | {} |",
        format_pct(report.comparison.timely_warning_rate.baseline),
        format_pct(report.comparison.timely_warning_rate.candidate),
        format_signed_pct_delta(report.comparison.timely_warning_rate.delta)
    );
    let _ = writeln!(
        markdown,
        "| strict_actionable_point_count | {} | {} | {} |",
        report.comparison.strict_actionable_point_count.baseline,
        report.comparison.strict_actionable_point_count.candidate,
        format_signed_count_delta(report.comparison.strict_actionable_point_count.delta)
    );
    let _ = writeln!(
        markdown,
        "| runtime_floor_hit_count | {} | {} | {} |",
        report.comparison.runtime_floor_hit_count.baseline,
        report.comparison.runtime_floor_hit_count.candidate,
        format_signed_count_delta(report.comparison.runtime_floor_hit_count.delta)
    );
    let _ = writeln!(
        markdown,
        "| actionable_precision | {} | {} | {} |",
        format_pct(report.comparison.actionable_precision.baseline),
        format_pct(report.comparison.actionable_precision.candidate),
        format_signed_pct_delta(report.comparison.actionable_precision.delta)
    );
    let _ = writeln!(
        markdown,
        "| longest_false_positive_episode_days | {} | {} | {} |",
        report
            .comparison
            .longest_false_positive_episode_days
            .baseline,
        report
            .comparison
            .longest_false_positive_episode_days
            .candidate,
        format_signed_count_delta(report.comparison.longest_false_positive_episode_days.delta)
    );
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Scenario-Level Backtests");
    let _ = writeln!(markdown);
    let _ = writeln!(
        markdown,
        "| Scenario | Source | Baseline L2 | Candidate L2 | Baseline L3 | Candidate L3 | Baseline FP | Candidate FP | Outcome |"
    );
    let _ = writeln!(
        markdown,
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- |"
    );
    for scenario in &report.comparison.backtest_scenarios {
        let _ = writeln!(
            markdown,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            scenario.name,
            scenario.signal_source,
            format_optional_days(scenario.baseline_lead_time_days),
            format_optional_days(scenario.candidate_lead_time_days),
            format_optional_days(scenario.baseline_actionable_lead_time_days),
            format_optional_days(scenario.candidate_actionable_lead_time_days),
            scenario.baseline_false_positive_count,
            scenario.candidate_false_positive_count,
            scenario.outcome
        );
    }
    let _ = writeln!(markdown);
    let failure_mode_summary = summarize_release_review_failure_modes(&report.scenario_focus);
    if !failure_mode_summary.is_empty() {
        let _ = writeln!(markdown, "## Failure Mode Summary");
        let _ = writeln!(markdown);
        let _ = writeln!(
            markdown,
            "| Failure mode | Baseline scenarios | Candidate scenarios | Baseline count | Candidate count |"
        );
        let _ = writeln!(markdown, "| --- | --- | --- | --- | --- |");
        for row in &failure_mode_summary {
            let _ = writeln!(
                markdown,
                "| {} | {} | {} | {} | {} |",
                row.failure_mode,
                format_runtime_category_list(&row.baseline_scenarios),
                format_runtime_category_list(&row.candidate_scenarios),
                row.baseline_count,
                row.candidate_count
            );
        }
        let _ = writeln!(markdown);
    }
    if !report.historical_audit_priorities.is_empty() {
        if !report.historical_audit_workstreams.is_empty() {
            let _ = writeln!(markdown, "## Historical Audit Workstream Summary");
            let _ = writeln!(markdown);
            let workstream_takeaways =
                release_review_historical_audit_takeaways(&report.historical_audit_workstreams);
            if !workstream_takeaways.is_empty() {
                let _ = writeln!(markdown, "### Historical Audit Takeaways");
                let _ = writeln!(markdown);
                for takeaway in workstream_takeaways {
                    let _ = writeln!(markdown, "- {takeaway}");
                }
                let _ = writeln!(markdown);
            }
            let _ = writeln!(
                markdown,
                "| Workstream | Scenarios | Protected | Families | Roles | Suggested review |"
            );
            let _ = writeln!(markdown, "| --- | --- | --- | --- | --- | --- |");
            for row in &report.historical_audit_workstreams {
                let _ = writeln!(
                    markdown,
                    "| {} | {} ({}) | {} | {} | {} | {} |",
                    row.workstream,
                    row.scenario_count,
                    format_runtime_category_list(&row.scenarios),
                    row.protected_count,
                    format_runtime_category_list(&row.scenario_families),
                    format_runtime_category_list(&row.training_roles),
                    row.suggested_review,
                );
            }
            let _ = writeln!(markdown);
        }
        if !report.historical_audit_attribution.is_empty() {
            let _ = writeln!(markdown, "## Historical Audit Attribution");
            let _ = writeln!(markdown);
            let _ = writeln!(
                markdown,
                "| Workstream | Attribution | Scenarios | Protected | Baseline count | Candidate count | Explanation |"
            );
            let _ = writeln!(markdown, "| --- | --- | --- | --- | --- | --- | --- |");
            for row in &report.historical_audit_attribution {
                let _ = writeln!(
                    markdown,
                    "| {} | {} | {} | {} | {} ({}) | {} ({}) | {} |",
                    row.workstream,
                    row.attribution,
                    row.scenario_count,
                    row.protected_count,
                    row.baseline_count,
                    format_runtime_category_list(&row.baseline_scenarios),
                    row.candidate_count,
                    format_runtime_category_list(&row.candidate_scenarios),
                    row.explanation,
                );
            }
            let _ = writeln!(markdown);
        }
        if !report.historical_audit_actions.is_empty() {
            let _ = writeln!(markdown, "## Historical Audit Actions");
            let _ = writeln!(markdown);
            let _ = writeln!(
                markdown,
                "| Workstream | Attribution | Action | Scenarios | Protected | Recommendation |"
            );
            let _ = writeln!(markdown, "| --- | --- | --- | --- | --- | --- |");
            for row in &report.historical_audit_actions {
                let _ = writeln!(
                    markdown,
                    "| {} | {} | {} | {} | {} | {} |",
                    row.workstream,
                    row.attribution,
                    row.action_type,
                    row.scenario_count,
                    row.protected_count,
                    row.recommendation,
                );
            }
            let _ = writeln!(markdown);
        }
        let _ = writeln!(markdown, "## Historical Audit Priorities");
        let _ = writeln!(markdown);
        let _ = writeln!(
            markdown,
            "| Scenario | Family | Role | Protected | Baseline mode | Candidate mode | Workstream | Suggested review |"
        );
        let _ = writeln!(
            markdown,
            "| --- | --- | --- | --- | --- | --- | --- | --- |"
        );
        for row in &report.historical_audit_priorities {
            let _ = writeln!(
                markdown,
                "| {} | {} | {} | {} | {} | {} | {} | {} |",
                row.scenario_name,
                row.scenario_family,
                row.training_role,
                if row.protected_window { "yes" } else { "no" },
                row.baseline_failure_mode,
                row.candidate_failure_mode,
                row.primary_workstream,
                row.suggested_review,
            );
        }
        let _ = writeln!(markdown);
    }
    if !report.scenario_focus.is_empty() {
        let _ = writeln!(markdown, "## Focus Scenarios");
        let _ = writeln!(markdown);
        for scenario in &report.scenario_focus {
            let _ = writeln!(markdown, "### {} ({})", scenario.name, scenario.outcome);
            let _ = writeln!(markdown);
            let _ = writeln!(
                markdown,
                "- Window: {} -> {}",
                scenario.window_start, scenario.window_end
            );
            let _ = writeln!(
                markdown,
                "- Crisis window: {} -> {}",
                scenario.crisis_start, scenario.crisis_end
            );
            let _ = writeln!(
                markdown,
                "- First L2: baseline={} | candidate={}",
                format_optional_date_with_lead(
                    scenario.baseline_first_l2_date,
                    scenario.crisis_start
                ),
                format_optional_date_with_lead(
                    scenario.candidate_first_l2_date,
                    scenario.crisis_start
                )
            );
            let _ = writeln!(
                markdown,
                "- First L3: baseline={} | candidate={}",
                format_optional_date_with_lead(
                    scenario.baseline_first_l3_date,
                    scenario.crisis_start
                ),
                format_optional_date_with_lead(
                    scenario.candidate_first_l3_date,
                    scenario.crisis_start
                )
            );
            let _ = writeln!(
                markdown,
                "- First non-normal point: baseline={} | candidate={}",
                format_optional_date(scenario.baseline_first_non_normal_date),
                format_optional_date(scenario.candidate_first_non_normal_date)
            );
            let _ = writeln!(
                markdown,
                "- Pre-crisis actionable points: baseline={} | candidate={}",
                scenario.baseline_actionable_point_count, scenario.candidate_actionable_point_count
            );
            let _ = writeln!(
                markdown,
                "- Pre-crisis runtime-floor hits: baseline={} | candidate={}",
                scenario.baseline_runtime_floor_hit_point_count,
                scenario.candidate_runtime_floor_hit_point_count
            );
            let _ = writeln!(
                markdown,
                "- Pre-crisis max p_20d: baseline={} | candidate={}",
                format_optional_pct(scenario.baseline_max_p20d),
                format_optional_pct(scenario.candidate_max_p20d)
            );
            let _ = writeln!(
                markdown,
                "- Pre-crisis max p_60d: baseline={} | candidate={}",
                format_optional_pct(scenario.baseline_max_p60d),
                format_optional_pct(scenario.candidate_max_p60d)
            );
            let _ = writeln!(
                markdown,
                "- First runtime-floor hit without L3: baseline={} | candidate={}",
                format_optional_date_with_reason(
                    scenario.baseline_first_runtime_floor_hit_without_l3_date,
                    scenario
                        .baseline_first_runtime_floor_hit_without_l3_reason
                        .as_deref()
                ),
                format_optional_date_with_reason(
                    scenario.candidate_first_runtime_floor_hit_without_l3_date,
                    scenario
                        .candidate_first_runtime_floor_hit_without_l3_reason
                        .as_deref()
                )
            );
            let _ = writeln!(
                markdown,
                "- Primary failure mode: baseline={} | candidate={}",
                scenario
                    .baseline_primary_failure_mode
                    .as_deref()
                    .unwrap_or("—"),
                scenario
                    .candidate_primary_failure_mode
                    .as_deref()
                    .unwrap_or("—")
            );
            let _ = writeln!(
                markdown,
                "- Dominant runtime block: baseline={} ({}) | candidate={} ({})",
                format_runtime_category_list(&scenario.dominant_runtime_blocks.baseline_categories),
                scenario.dominant_runtime_blocks.baseline_count,
                format_runtime_category_list(
                    &scenario.dominant_runtime_blocks.candidate_categories
                ),
                scenario.dominant_runtime_blocks.candidate_count
            );
            let _ = writeln!(
                markdown,
                "- Dominant continuity facet: baseline={} ({}) | candidate={} ({})",
                format_runtime_category_list(
                    &scenario
                        .dominant_runtime_continuity_facets
                        .baseline_categories
                ),
                scenario.dominant_runtime_continuity_facets.baseline_count,
                format_runtime_category_list(
                    &scenario
                        .dominant_runtime_continuity_facets
                        .candidate_categories
                ),
                scenario.dominant_runtime_continuity_facets.candidate_count
            );
            if !scenario.runtime_block_counts.is_empty() {
                let _ = writeln!(markdown, "- Runtime block mix:");
                for block in &scenario.runtime_block_counts {
                    let _ = writeln!(
                        markdown,
                        "  - {}: baseline={} | candidate={} | delta={}",
                        block.category,
                        block.baseline_count,
                        block.candidate_count,
                        format_signed_count_delta(block.delta)
                    );
                }
            }
            if !scenario.runtime_continuity_facet_counts.is_empty() {
                let _ = writeln!(markdown, "- Runtime continuity facets:");
                for block in &scenario.runtime_continuity_facet_counts {
                    let _ = writeln!(
                        markdown,
                        "  - {}: baseline={} | candidate={} | delta={}",
                        block.category,
                        block.baseline_count,
                        block.candidate_count,
                        format_signed_count_delta(block.delta)
                    );
                }
            }
            let _ = writeln!(markdown);
            if scenario.interesting_points.is_empty() {
                let _ = writeln!(
                    markdown,
                    "- No loaded runtime history points matched this scenario window. Fast review history_limit may be too small for this sample."
                );
                let _ = writeln!(markdown);
                continue;
            }
            let _ = writeln!(
                markdown,
                "| Date | Base p_20d | Cand p_20d | Base p_60d | Cand p_60d | Base posture | Cand posture | Base bucket | Cand bucket | Base strict L3 | Cand strict L3 | Base runtime floor | Cand runtime floor | Base 5d hits | Cand 5d hits | Base sustained | Cand sustained | Base triggers | Cand triggers | Base block cat | Cand block cat | Base runtime block | Cand runtime block | Base diag | Cand diag |"
            );
            let _ = writeln!(
                markdown,
                "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
            );
            for point in &scenario.interesting_points {
                let _ = writeln!(
                    markdown,
                    "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                    point.as_of_date,
                    format_optional_pct(point.baseline_p20d),
                    format_optional_pct(point.candidate_p20d),
                    format_optional_pct(point.baseline_p60d),
                    format_optional_pct(point.candidate_p60d),
                    point.baseline_posture.as_deref().unwrap_or("—"),
                    point.candidate_posture.as_deref().unwrap_or("—"),
                    point.baseline_time_bucket.as_deref().unwrap_or("—"),
                    point.candidate_time_bucket.as_deref().unwrap_or("—"),
                    format_bool_flag(point.baseline_strict_review_actionable),
                    format_bool_flag(point.candidate_strict_review_actionable),
                    format_bool_flag(point.baseline_runtime_floor_hit),
                    format_bool_flag(point.candidate_runtime_floor_hit),
                    format_optional_count(point.baseline_actionable_forward_5d_hits),
                    format_optional_count(point.candidate_actionable_forward_5d_hits),
                    format_optional_bool_flag(point.baseline_actionable_sustained),
                    format_optional_bool_flag(point.candidate_actionable_sustained),
                    format_trigger_codes(&point.baseline_trigger_codes),
                    format_trigger_codes(&point.candidate_trigger_codes),
                    point
                        .baseline_runtime_actionable_block_category
                        .as_deref()
                        .unwrap_or("—"),
                    point
                        .candidate_runtime_actionable_block_category
                        .as_deref()
                        .unwrap_or("—"),
                    point
                        .baseline_runtime_actionable_block_reason
                        .as_deref()
                        .unwrap_or("—"),
                    point
                        .candidate_runtime_actionable_block_reason
                        .as_deref()
                        .unwrap_or("—"),
                    point
                        .baseline_actionable_diagnostic
                        .as_deref()
                        .unwrap_or("—"),
                    point
                        .candidate_actionable_diagnostic
                        .as_deref()
                        .unwrap_or("—")
                );
            }
            let _ = writeln!(markdown);
        }
    }
    let _ = writeln!(markdown, "## Actionability Diagnostics");
    let _ = writeln!(markdown);
    render_release_actionability_review_markdown(
        &mut markdown,
        "baseline",
        &report.baseline_actionability_review,
    );
    let _ = writeln!(markdown);
    render_release_actionability_review_markdown(
        &mut markdown,
        "candidate",
        &report.candidate_actionability_review,
    );
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Guardrail Result");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "### Runtime Guard");
    let _ = writeln!(markdown);
    if report.operational_guard_regressions.is_empty() {
        let _ = writeln!(markdown, "- No runtime guard regressions detected.");
    } else {
        for regression in &report.operational_guard_regressions {
            let _ = writeln!(markdown, "- {regression}");
        }
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "### Probability Guard");
    let _ = writeln!(markdown);
    if report.probability_guard_regressions.is_empty() {
        let _ = writeln!(
            markdown,
            "- No probability-head guard regressions detected."
        );
    } else {
        for regression in &report.probability_guard_regressions {
            let _ = writeln!(markdown, "- {regression}");
        }
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "### Actionability Guard");
    let _ = writeln!(markdown);
    if report.actionability_guard_regressions.is_empty() {
        let _ = writeln!(markdown, "- No actionability guard regressions detected.");
    } else {
        for regression in &report.actionability_guard_regressions {
            let _ = writeln!(markdown, "- {regression}");
        }
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "### Runtime Sanity Guard");
    let _ = writeln!(markdown);
    if report.runtime_sanity_regressions.is_empty() {
        let _ = writeln!(markdown, "- No runtime sanity regressions detected.");
    } else {
        for regression in &report.runtime_sanity_regressions {
            let _ = writeln!(markdown, "- {regression}");
        }
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "### Overall");
    let _ = writeln!(markdown);
    if report.overall_guard_regressions.is_empty() {
        let _ = writeln!(markdown, "- No combined guard regressions detected.");
    } else {
        for regression in &report.overall_guard_regressions {
            let _ = writeln!(markdown, "- {regression}");
        }
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Recommendation");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "{}", report.recommendation);
    markdown
}

fn render_release_actionability_review_markdown(
    markdown: &mut String,
    role: &str,
    review: &crate::ReleaseActionabilityReview,
) {
    let _ = writeln!(markdown, "### {role} Actionability");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "- Enabled: {}", review.enabled);
    let _ = writeln!(markdown, "- Note: {}", review.note);
    if !review.enabled {
        return;
    }
    let _ = writeln!(
        markdown,
        "- Versions: model={} calib={} fusion={}",
        review.model_version.as_deref().unwrap_or("n/a"),
        review.calibration_version.as_deref().unwrap_or("n/a"),
        review.fusion_policy_version.as_deref().unwrap_or("n/a")
    );
    let _ = writeln!(markdown);
    let _ = writeln!(
        markdown,
        "| Level | Scenarios | On Time | Late Only | Missed | Primary Recall | Late Validation | Precision | Pred+ | Primary+ | Protected Hits | FP |"
    );
    let _ = writeln!(
        markdown,
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
    );
    for level in &review.levels {
        let _ = writeln!(
            markdown,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            crate::actionability_level_text(level.level),
            level.scenario_count,
            crate::format_optional_pct(level.on_time_rate),
            crate::format_optional_pct(level.late_only_rate),
            crate::format_optional_pct(level.missed_rate),
            crate::format_optional_pct(level.primary_recall_at_threshold),
            crate::format_optional_pct(level.late_validation_capture_rate),
            crate::format_optional_pct(level.precision_at_threshold),
            level.predicted_positive_count,
            level.primary_positive_count,
            level.protected_hit_count,
            level.false_positive_count
        );
    }
}

fn render_release_runtime_review_markdown(
    markdown: &mut String,
    role: &str,
    diagnostics: &crate::ReleaseRuntimeReviewDiagnostics,
) {
    let _ = writeln!(markdown, "### {role} Runtime");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "- Release: {}", diagnostics.release_id);
    let _ = writeln!(
        markdown,
        "- History points: {}",
        diagnostics.history_point_count
    );
    let _ = writeln!(markdown, "- Note: {}", diagnostics.note);
    if let Some(thresholds) = diagnostics.runtime_thresholds.as_ref() {
        let _ = writeln!(
            markdown,
            "- Thresholds: prepare_p60d={}, hedge_p20d={}, defend_p5d={}",
            crate::format_pct(thresholds.prepare_p60d),
            crate::format_pct(thresholds.hedge_p20d),
            crate::format_pct(thresholds.defend_p5d),
        );
        let _ = writeln!(
            markdown,
            "- Runtime policy version: {}",
            thresholds.history_runtime_policy_version
        );
        let _ = writeln!(
            markdown,
            "- Probability floor hits: p_60d>=prepare {} / p_20d>=hedge {} / p_5d>=defend {}",
            diagnostics
                .points_at_or_above_prepare_p60d
                .unwrap_or_default(),
            diagnostics
                .points_at_or_above_hedge_p20d
                .unwrap_or_default(),
            diagnostics
                .points_at_or_above_defend_p5d
                .unwrap_or_default(),
        );
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "| Posture | Count |");
    let _ = writeln!(markdown, "| --- | --- |");
    for row in &diagnostics.posture_distribution {
        let _ = writeln!(markdown, "| {} | {} |", row.name, row.count);
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "| Time bucket | Count |");
    let _ = writeln!(markdown, "| --- | --- |");
    for row in &diagnostics.time_bucket_distribution {
        let _ = writeln!(markdown, "| {} | {} |", row.name, row.count);
    }
    if !diagnostics.posture_trigger_distribution.is_empty() {
        let _ = writeln!(markdown);
        let _ = writeln!(
            markdown,
            "| Posture | Trigger clause | Count | Share of posture |"
        );
        let _ = writeln!(markdown, "| --- | --- | --- | --- |");
        for row in &diagnostics.posture_trigger_distribution {
            let _ = writeln!(
                markdown,
                "| {} | {} | {} | {} |",
                row.posture,
                row.clause,
                row.count,
                crate::format_pct(row.share_of_posture),
            );
        }
    }
    if !diagnostics.posture_blocker_distribution.is_empty() {
        let _ = writeln!(markdown);
        let _ = writeln!(
            markdown,
            "| Posture | Blocker clause | Count | Share of posture |"
        );
        let _ = writeln!(markdown, "| --- | --- | --- | --- |");
        for row in &diagnostics.posture_blocker_distribution {
            let _ = writeln!(
                markdown,
                "| {} | {} | {} | {} |",
                row.posture,
                row.clause,
                row.count,
                crate::format_pct(row.share_of_posture),
            );
        }
    }
    if !diagnostics.regime_separation_summaries.is_empty() {
        let _ = writeln!(markdown);
        let _ = writeln!(
            markdown,
            "| Horizon | Early regime | Normal P | Positive-window P | Cooldown P | Early raw lift | Early calibrated lift | Positive-window lift | Cooldown lift | Gap retention | Diagnosis |"
        );
        let _ = writeln!(
            markdown,
            "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
        );
        for row in &diagnostics.regime_separation_summaries {
            let _ = writeln!(
                markdown,
                "| {}d | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                row.horizon_days,
                row.early_warning_regime,
                crate::format_pct(row.normal_avg_probability),
                crate::format_pct(row.positive_window_avg_probability),
                crate::format_pct(row.post_crisis_cooldown_avg_probability),
                crate::format_optional_multiplier(row.early_warning_raw_lift_vs_normal),
                crate::format_optional_multiplier(row.early_warning_calibrated_lift_vs_normal),
                crate::format_optional_multiplier(row.positive_window_calibrated_lift_vs_normal),
                crate::format_optional_multiplier(
                    row.post_crisis_cooldown_calibrated_lift_vs_normal
                ),
                crate::format_optional_ratio(row.early_warning_gap_retention),
                row.diagnosis,
            );
        }
    }
    if !diagnostics.regime_probability_summaries.is_empty() {
        let _ = writeln!(markdown);
        let _ = writeln!(
            markdown,
            "| Horizon | Regime | Rows | Share | Avg raw P | Max raw P | Avg calibrated P | Max calibrated P | Raw lift vs normal | Calibrated lift vs normal | Gap retention | Floor hits |"
        );
        let _ = writeln!(
            markdown,
            "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
        );
        for row in &diagnostics.regime_probability_summaries {
            let _ = writeln!(
                markdown,
                "| {}d | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                row.horizon_days,
                row.regime,
                row.row_count,
                crate::format_pct(row.row_rate),
                crate::format_pct(row.avg_raw_probability),
                crate::format_pct(row.max_raw_probability),
                crate::format_pct(row.avg_probability),
                crate::format_pct(row.max_probability),
                crate::format_optional_multiplier(row.raw_lift_vs_normal),
                crate::format_optional_multiplier(row.calibrated_lift_vs_normal),
                crate::format_optional_ratio(row.calibration_gap_retention),
                row.threshold_hit_count
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string()),
            );
        }
    }
}

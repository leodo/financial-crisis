use std::fmt::Write;

use crate::ReleaseReviewEnvelope;

pub(super) fn render_release_review_overview_markdown(
    markdown: &mut String,
    report: &ReleaseReviewEnvelope,
) {
    render_release_rows_markdown(markdown, report);
    render_current_runtime_snapshot_markdown(markdown, report);
    render_runtime_separation_comparison_markdown(markdown, report);
    render_backtest_guardrails_markdown(markdown, report);
    render_scenario_level_backtests_markdown(markdown, report);
    render_failure_mode_summary_markdown(markdown, report);
}

fn render_release_rows_markdown(markdown: &mut String, report: &ReleaseReviewEnvelope) {
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
}

fn render_current_runtime_snapshot_markdown(markdown: &mut String, report: &ReleaseReviewEnvelope) {
    let _ = writeln!(markdown, "## Current Runtime Snapshot");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "| Metric | Baseline | Candidate | Delta |");
    let _ = writeln!(markdown, "| --- | --- | --- | --- |");
    let _ = writeln!(
        markdown,
        "| p_5d | {} | {} | {} |",
        crate::format_pct(report.comparison.current_p_5d.baseline),
        crate::format_pct(report.comparison.current_p_5d.candidate),
        crate::format_signed_pct_delta(report.comparison.current_p_5d.delta)
    );
    let _ = writeln!(
        markdown,
        "| p_20d | {} | {} | {} |",
        crate::format_pct(report.comparison.current_p_20d.baseline),
        crate::format_pct(report.comparison.current_p_20d.candidate),
        crate::format_signed_pct_delta(report.comparison.current_p_20d.delta)
    );
    let _ = writeln!(
        markdown,
        "| p_60d | {} | {} | {} |",
        crate::format_pct(report.comparison.current_p_60d.baseline),
        crate::format_pct(report.comparison.current_p_60d.candidate),
        crate::format_signed_pct_delta(report.comparison.current_p_60d.delta)
    );
    let _ = writeln!(
        markdown,
        "| Posture | {} | {} | — |",
        crate::posture_text(report.baseline_assessment.posture),
        crate::posture_text(report.candidate_assessment.posture)
    );
    let _ = writeln!(
        markdown,
        "| Time bucket | {} | {} | — |",
        crate::time_bucket_text(report.baseline_assessment.time_to_risk_bucket),
        crate::time_bucket_text(report.candidate_assessment.time_to_risk_bucket)
    );
    let _ = writeln!(markdown);
}

fn render_runtime_separation_comparison_markdown(
    markdown: &mut String,
    report: &ReleaseReviewEnvelope,
) {
    if report.comparison.runtime_separation_summary.is_empty() {
        return;
    }

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
            crate::format_optional_pct(row.baseline_threshold),
            crate::format_optional_pct(row.candidate_threshold),
            row.baseline_early_warning_regime,
            row.candidate_early_warning_regime,
            crate::format_optional_pct(row.baseline_early_warning_avg_probability),
            crate::format_optional_pct(row.candidate_early_warning_avg_probability),
            crate::format_optional_pct(row.baseline_normal_avg_probability),
            crate::format_optional_pct(row.candidate_normal_avg_probability),
            crate::format_optional_pct(row.baseline_early_warning_gap_vs_normal),
            crate::format_optional_pct(row.candidate_early_warning_gap_vs_normal),
            crate::format_optional_pct(row.baseline_floor_gap),
            crate::format_optional_pct(row.candidate_floor_gap),
            crate::format_optional_multiplier(row.baseline_early_warning_lift_vs_normal),
            crate::format_optional_multiplier(row.candidate_early_warning_lift_vs_normal),
            crate::format_optional_pct(row.baseline_threshold_hit_rate),
            crate::format_optional_pct(row.candidate_threshold_hit_rate),
        );
    }
    let takeaways = crate::release_review_runtime_separation_takeaways(
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
    let _ = writeln!(markdown);
}

fn render_backtest_guardrails_markdown(markdown: &mut String, report: &ReleaseReviewEnvelope) {
    let _ = writeln!(markdown, "## Backtest Guardrails");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "| Metric | Baseline | Candidate | Delta |");
    let _ = writeln!(markdown, "| --- | --- | --- | --- |");
    let _ = writeln!(
        markdown,
        "| timely_warning_rate | {} | {} | {} |",
        crate::format_pct(report.comparison.timely_warning_rate.baseline),
        crate::format_pct(report.comparison.timely_warning_rate.candidate),
        crate::format_signed_pct_delta(report.comparison.timely_warning_rate.delta)
    );
    let _ = writeln!(
        markdown,
        "| strict_actionable_point_count | {} | {} | {} |",
        report.comparison.strict_actionable_point_count.baseline,
        report.comparison.strict_actionable_point_count.candidate,
        crate::format_signed_count_delta(report.comparison.strict_actionable_point_count.delta)
    );
    let _ = writeln!(
        markdown,
        "| runtime_floor_hit_count | {} | {} | {} |",
        report.comparison.runtime_floor_hit_count.baseline,
        report.comparison.runtime_floor_hit_count.candidate,
        crate::format_signed_count_delta(report.comparison.runtime_floor_hit_count.delta)
    );
    let _ = writeln!(
        markdown,
        "| actionable_precision | {} | {} | {} |",
        crate::format_pct(report.comparison.actionable_precision.baseline),
        crate::format_pct(report.comparison.actionable_precision.candidate),
        crate::format_signed_pct_delta(report.comparison.actionable_precision.delta)
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
        crate::format_signed_count_delta(
            report.comparison.longest_false_positive_episode_days.delta
        )
    );
    let _ = writeln!(markdown);
}

fn render_scenario_level_backtests_markdown(markdown: &mut String, report: &ReleaseReviewEnvelope) {
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
            crate::format_optional_days(scenario.baseline_lead_time_days),
            crate::format_optional_days(scenario.candidate_lead_time_days),
            crate::format_optional_days(scenario.baseline_actionable_lead_time_days),
            crate::format_optional_days(scenario.candidate_actionable_lead_time_days),
            scenario.baseline_false_positive_count,
            scenario.candidate_false_positive_count,
            scenario.outcome
        );
    }
    let _ = writeln!(markdown);
}

fn render_failure_mode_summary_markdown(markdown: &mut String, report: &ReleaseReviewEnvelope) {
    let failure_mode_summary =
        crate::summarize_release_review_failure_modes(&report.scenario_focus);
    if failure_mode_summary.is_empty() {
        return;
    }

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
            crate::format_runtime_category_list(&row.baseline_scenarios),
            crate::format_runtime_category_list(&row.candidate_scenarios),
            row.baseline_count,
            row.candidate_count
        );
    }
    let _ = writeln!(markdown);
}

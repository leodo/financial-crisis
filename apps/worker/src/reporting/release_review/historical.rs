use std::fmt::Write;

use crate::ReleaseReviewEnvelope;

fn gate_gap_point_label(category: &str) -> &str {
    match category {
        "p20d_only" => "p20d only",
        "p60d_only" => "p60d only",
        "p20d_and_p60d" => "p20d + p60d",
        _ => category,
    }
}

fn format_gate_gap_point_counts(
    counts: &[crate::ReleaseReviewRuntimeBlockCount],
    for_candidate: bool,
) -> String {
    let rendered = counts
        .iter()
        .filter_map(|count| {
            let value = if for_candidate {
                count.candidate_count
            } else {
                count.baseline_count
            };
            (value > 0).then(|| format!("{}={}", gate_gap_point_label(&count.category), value))
        })
        .collect::<Vec<_>>();
    if rendered.is_empty() {
        "—".to_string()
    } else {
        rendered.join(", ")
    }
}

pub(super) fn render_release_historical_audit_markdown(
    markdown: &mut String,
    report: &ReleaseReviewEnvelope,
) {
    if !report.scenario_coverages.is_empty() {
        let _ = writeln!(markdown, "## Scenario Coverage Context");
        let _ = writeln!(markdown);
        let _ = writeln!(
            markdown,
            "- Coverage source: {}",
            report.scenario_coverage_catalog.source
        );
        let _ = writeln!(
            markdown,
            "- Backtest scenarios covered: {}/{}",
            report
                .scenario_coverage_catalog
                .covered_backtest_scenario_count,
            report.scenario_coverage_catalog.backtest_scenario_count
        );
        let _ = writeln!(
            markdown,
            "- Focus scenarios covered: {}/{}",
            report
                .scenario_coverage_catalog
                .covered_focus_scenario_count,
            report.scenario_coverage_catalog.focus_scenario_count
        );
        let _ = writeln!(
            markdown,
            "- Eligibility mix: main={} extension={} protected={} analog={}",
            report
                .scenario_coverage_catalog
                .main_training_eligible_count,
            report
                .scenario_coverage_catalog
                .extension_training_eligible_count,
            report
                .scenario_coverage_catalog
                .protected_stress_eligible_count,
            report
                .scenario_coverage_catalog
                .historical_analog_eligible_count
        );
        if let Some(warning) = &report.scenario_coverage_catalog.warning {
            let _ = writeln!(markdown, "- Coverage warning: {}", warning);
        }
        let _ = writeln!(markdown);
        let _ = writeln!(
            markdown,
            "| Scenario | Family | Dataset role | Focus | Coverage role | Grade | Coverage PIT | Allowed | Status | Gaps |"
        );
        let _ = writeln!(
            markdown,
            "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
        );
        for row in &report.scenario_coverages {
            let gaps = if row.blocking_gaps.is_empty() {
                "—".to_string()
            } else {
                row.blocking_gaps.join("; ")
            };
            let _ = writeln!(
                markdown,
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                row.scenario_name,
                row.scenario_family,
                row.training_role,
                if row.in_focus_review { "yes" } else { "no" },
                row.recommended_role,
                row.coverage_grade,
                row.point_in_time_mode,
                format_allowed_roles(
                    row.usable_for_main_training,
                    row.usable_for_extension_training,
                    row.usable_for_protected_stress,
                    row.usable_for_historical_analog
                ),
                row.current_status,
                gaps,
            );
        }
        let _ = writeln!(markdown);
    }

    if report.historical_audit_priorities.is_empty() {
        return;
    }

    if !report.historical_audit_workstreams.is_empty() {
        let _ = writeln!(markdown, "## Historical Audit Workstream Summary");
        let _ = writeln!(markdown);
        let workstream_takeaways =
            crate::release_review_historical_audit_takeaways(&report.historical_audit_workstreams);
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
            "| Workstream | Scenarios | Protected | Families | Roles | Baseline gate gap | Candidate gate gap | Baseline gate-gap points | Candidate gate-gap points | Suggested review |"
        );
        let _ = writeln!(
            markdown,
            "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
        );
        for row in &report.historical_audit_workstreams {
            let _ = writeln!(
                markdown,
                "| {} | {} ({}) | {} | {} | {} | {} | {} | {} | {} | {} |",
                row.workstream,
                row.scenario_count,
                crate::format_runtime_category_list(&row.scenarios),
                row.protected_count,
                crate::format_runtime_category_list(&row.scenario_families),
                crate::format_runtime_category_list(&row.training_roles),
                crate::format_runtime_category_list(&row.baseline_gate_gap_profiles),
                crate::format_runtime_category_list(&row.candidate_gate_gap_profiles),
                format_gate_gap_point_counts(&row.gate_gap_point_counts, false),
                format_gate_gap_point_counts(&row.gate_gap_point_counts, true),
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
                crate::format_runtime_category_list(&row.baseline_scenarios),
                row.candidate_count,
                crate::format_runtime_category_list(&row.candidate_scenarios),
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
        "| Scenario | Family | Role | Protected | Coverage role | Grade | Coverage PIT | Baseline mode | Candidate mode | Baseline gate gap | Candidate gate gap | Workstream | Suggested review |"
    );
    let _ = writeln!(
        markdown,
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
    );
    for row in &report.historical_audit_priorities {
        let _ = writeln!(
            markdown,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            row.scenario_name,
            row.scenario_family,
            row.training_role,
            if row.protected_window { "yes" } else { "no" },
            row.coverage_recommended_role.as_deref().unwrap_or("—"),
            row.coverage_grade.as_deref().unwrap_or("—"),
            row.coverage_point_in_time_mode.as_deref().unwrap_or("—"),
            row.baseline_failure_mode,
            row.candidate_failure_mode,
            row.baseline_gate_gap_profile.as_deref().unwrap_or("—"),
            row.candidate_gate_gap_profile.as_deref().unwrap_or("—"),
            row.primary_workstream,
            row.suggested_review,
        );
    }
    let _ = writeln!(markdown);
}

fn format_allowed_roles(
    usable_for_main_training: bool,
    usable_for_extension_training: bool,
    usable_for_protected_stress: bool,
    usable_for_historical_analog: bool,
) -> String {
    let mut roles = Vec::new();
    if usable_for_main_training {
        roles.push("main");
    }
    if usable_for_extension_training {
        roles.push("ext");
    }
    if usable_for_protected_stress {
        roles.push("protected");
    }
    if usable_for_historical_analog {
        roles.push("analog");
    }
    if roles.is_empty() {
        "—".to_string()
    } else {
        roles.join(", ")
    }
}

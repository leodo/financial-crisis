use std::fmt::Write;

use crate::ReleaseReviewEnvelope;

pub(super) fn render_release_historical_audit_markdown(
    markdown: &mut String,
    report: &ReleaseReviewEnvelope,
) {
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
            "| Workstream | Scenarios | Protected | Families | Roles | Suggested review |"
        );
        let _ = writeln!(markdown, "| --- | --- | --- | --- | --- | --- |");
        for row in &report.historical_audit_workstreams {
            let _ = writeln!(
                markdown,
                "| {} | {} ({}) | {} | {} | {} | {} |",
                row.workstream,
                row.scenario_count,
                crate::format_runtime_category_list(&row.scenarios),
                row.protected_count,
                crate::format_runtime_category_list(&row.scenario_families),
                crate::format_runtime_category_list(&row.training_roles),
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

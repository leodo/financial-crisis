use std::fmt::Write;

use crate::{
    release_review::{ReleaseReviewHistoricalAuditPriority, ReleaseReviewScenarioCoverage},
    ReleaseReviewEnvelope, ReleaseReviewScenarioFocusDiagnostic,
};

pub(super) fn render_release_focus_scenarios_markdown(
    markdown: &mut String,
    report: &ReleaseReviewEnvelope,
) {
    if report.scenario_focus.is_empty() {
        return;
    }

    let _ = writeln!(markdown, "## Focus Scenarios");
    let _ = writeln!(markdown);
    for scenario in &report.scenario_focus {
        render_release_focus_scenario_markdown(
            markdown,
            scenario,
            historical_audit_priority_for_scenario(report, scenario.scenario_id.as_str()),
            scenario_coverage_for_scenario(report, scenario.scenario_id.as_str()),
        );
    }
}

fn historical_audit_priority_for_scenario<'a>(
    report: &'a ReleaseReviewEnvelope,
    scenario_id: &str,
) -> Option<&'a ReleaseReviewHistoricalAuditPriority> {
    report
        .historical_audit_priorities
        .iter()
        .find(|row| row.scenario_id == scenario_id)
}

fn scenario_coverage_for_scenario<'a>(
    report: &'a ReleaseReviewEnvelope,
    scenario_id: &str,
) -> Option<&'a ReleaseReviewScenarioCoverage> {
    report
        .scenario_coverages
        .iter()
        .find(|row| row.scenario_id == scenario_id)
}

fn historical_audit_workstream_label(workstream: &str) -> &str {
    match workstream {
        "strict_review_vs_runtime_mapping" => "strict gate vs runtime floor",
        "posture_continuity" => "posture continuity",
        "score_confirmation" => "score confirmation",
        "transitional_bridge" => "transitional bridge",
        "prewarning_signal_gap" => "pre-warning signal gap",
        "weak_signal_continuity" => "weak signal continuity",
        _ => "residual release-review audit",
    }
}

fn render_release_focus_scenario_markdown(
    markdown: &mut String,
    scenario: &ReleaseReviewScenarioFocusDiagnostic,
    historical_priority: Option<&ReleaseReviewHistoricalAuditPriority>,
    scenario_coverage: Option<&ReleaseReviewScenarioCoverage>,
) {
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
        crate::format_optional_date_with_lead(
            scenario.baseline_first_l2_date,
            scenario.crisis_start
        ),
        crate::format_optional_date_with_lead(
            scenario.candidate_first_l2_date,
            scenario.crisis_start
        )
    );
    let _ = writeln!(
        markdown,
        "- First L3: baseline={} | candidate={}",
        crate::format_optional_date_with_lead(
            scenario.baseline_first_l3_date,
            scenario.crisis_start
        ),
        crate::format_optional_date_with_lead(
            scenario.candidate_first_l3_date,
            scenario.crisis_start
        )
    );
    let _ = writeln!(
        markdown,
        "- First non-normal point: baseline={} | candidate={}",
        crate::format_optional_date(scenario.baseline_first_non_normal_date),
        crate::format_optional_date(scenario.candidate_first_non_normal_date)
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
        crate::format_optional_pct(scenario.baseline_max_p20d),
        crate::format_optional_pct(scenario.candidate_max_p20d)
    );
    let _ = writeln!(
        markdown,
        "- Pre-crisis max p_60d: baseline={} | candidate={}",
        crate::format_optional_pct(scenario.baseline_max_p60d),
        crate::format_optional_pct(scenario.candidate_max_p60d)
    );
    let _ = writeln!(
        markdown,
        "- First runtime-floor hit without L3: baseline={} | candidate={}",
        crate::format_optional_date_with_reason(
            scenario.baseline_first_runtime_floor_hit_without_l3_date,
            scenario
                .baseline_first_runtime_floor_hit_without_l3_reason
                .as_deref()
        ),
        crate::format_optional_date_with_reason(
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
    if let Some(priority) = historical_priority {
        let _ = writeln!(
            markdown,
            "- Historical audit refinement: workstream={} | protected={} | role={} | suggested review={}",
            historical_audit_workstream_label(&priority.primary_workstream),
            if priority.protected_window { "yes" } else { "no" },
            priority.training_role,
            priority.suggested_review
        );
    }
    if let Some(coverage) = scenario_coverage {
        let _ = writeln!(
            markdown,
            "- Coverage context: role={} | grade={} | PIT={} | allowed={} | status={}",
            coverage.recommended_role,
            coverage.coverage_grade,
            coverage.point_in_time_mode,
            format_allowed_roles(coverage),
            coverage.current_status
        );
        if !coverage.blocking_gaps.is_empty() {
            let _ = writeln!(
                markdown,
                "- Coverage gaps: {}",
                coverage.blocking_gaps.join("; ")
            );
        }
        if !coverage.free_sources.is_empty() {
            let _ = writeln!(
                markdown,
                "- Free sources: {}",
                coverage.free_sources.join(", ")
            );
        }
    }
    let _ = writeln!(
        markdown,
        "- Dominant runtime block: baseline={} ({}) | candidate={} ({})",
        crate::format_runtime_category_list(&scenario.dominant_runtime_blocks.baseline_categories),
        scenario.dominant_runtime_blocks.baseline_count,
        crate::format_runtime_category_list(&scenario.dominant_runtime_blocks.candidate_categories),
        scenario.dominant_runtime_blocks.candidate_count
    );
    let _ = writeln!(
        markdown,
        "- Dominant continuity facet: baseline={} ({}) | candidate={} ({})",
        crate::format_runtime_category_list(
            &scenario
                .dominant_runtime_continuity_facets
                .baseline_categories
        ),
        scenario.dominant_runtime_continuity_facets.baseline_count,
        crate::format_runtime_category_list(
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
                crate::format_signed_count_delta(block.delta)
            );
        }
    }

    fn format_allowed_roles(coverage: &ReleaseReviewScenarioCoverage) -> String {
        let mut roles = Vec::new();
        if coverage.usable_for_main_training {
            roles.push("main");
        }
        if coverage.usable_for_extension_training {
            roles.push("ext");
        }
        if coverage.usable_for_protected_stress {
            roles.push("protected");
        }
        if coverage.usable_for_historical_analog {
            roles.push("analog");
        }
        if roles.is_empty() {
            "—".to_string()
        } else {
            roles.join(", ")
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
                crate::format_signed_count_delta(block.delta)
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
        return;
    }

    let _ = writeln!(
        markdown,
        "| Date | Base p_20d | Cand p_20d | Base p_60d | Cand p_60d | Base overall | Cand overall | Base external | Cand external | Base posture | Cand posture | Base bucket | Cand bucket | Base strict L3 | Cand strict L3 | Base runtime floor | Cand runtime floor | Base 5d hits | Cand 5d hits | Base sustained | Cand sustained | Base triggers | Cand triggers | Base block cat | Cand block cat | Base runtime block | Cand runtime block | Base diag | Cand diag |"
    );
    let _ = writeln!(
        markdown,
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
    );
    for point in &scenario.interesting_points {
        let _ = writeln!(
            markdown,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            point.as_of_date,
            crate::format_optional_pct(point.baseline_p20d),
            crate::format_optional_pct(point.candidate_p20d),
            crate::format_optional_pct(point.baseline_p60d),
            crate::format_optional_pct(point.candidate_p60d),
            crate::format_optional_score(point.baseline_overall_score),
            crate::format_optional_score(point.candidate_overall_score),
            crate::format_optional_score(point.baseline_external_shock_score),
            crate::format_optional_score(point.candidate_external_shock_score),
            point.baseline_posture.as_deref().unwrap_or("—"),
            point.candidate_posture.as_deref().unwrap_or("—"),
            point.baseline_time_bucket.as_deref().unwrap_or("—"),
            point.candidate_time_bucket.as_deref().unwrap_or("—"),
            crate::format_bool_flag(point.baseline_strict_review_actionable),
            crate::format_bool_flag(point.candidate_strict_review_actionable),
            crate::format_bool_flag(point.baseline_runtime_floor_hit),
            crate::format_bool_flag(point.candidate_runtime_floor_hit),
            crate::format_optional_count(point.baseline_actionable_forward_5d_hits),
            crate::format_optional_count(point.candidate_actionable_forward_5d_hits),
            crate::format_optional_bool_flag(point.baseline_actionable_sustained),
            crate::format_optional_bool_flag(point.candidate_actionable_sustained),
            crate::format_trigger_codes(&point.baseline_trigger_codes),
            crate::format_trigger_codes(&point.candidate_trigger_codes),
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

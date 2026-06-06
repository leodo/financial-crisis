pub(super) fn build_release_review_recommendation(
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

fn gate_gap_point_label(category: &str) -> &str {
    match category {
        "p20d_only" => "p20d only",
        "p60d_only" => "p60d only",
        "p20d_and_p60d" => "p20d + p60d",
        _ => category,
    }
}

fn format_workstream_gate_gap_point_counts(
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

pub(super) fn print_release_review_summary(report: &crate::ReleaseReviewEnvelope) {
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
                "  - {} scenarios={} ({}) protected={} families={} roles={} baseline_gate_gap={} candidate_gate_gap={} baseline_gate_gap_points={} candidate_gate_gap_points={} review={}",
                row.workstream,
                row.scenario_count,
                row.scenarios.join(", "),
                row.protected_count,
                row.scenario_families.join(", "),
                row.training_roles.join(", "),
                if row.baseline_gate_gap_profiles.is_empty() {
                    "—".to_string()
                } else {
                    row.baseline_gate_gap_profiles.join(", ")
                },
                if row.candidate_gate_gap_profiles.is_empty() {
                    "—".to_string()
                } else {
                    row.candidate_gate_gap_profiles.join(", ")
                },
                format_workstream_gate_gap_point_counts(&row.gate_gap_point_counts, false),
                format_workstream_gate_gap_point_counts(&row.gate_gap_point_counts, true),
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
                "  - {} [{}] workstream={} baseline={} candidate={} baseline_gate_gap={} candidate_gate_gap={} protected={} review={}",
                row.scenario_name,
                row.training_role,
                row.primary_workstream,
                row.baseline_failure_mode,
                row.candidate_failure_mode,
                row.baseline_gate_gap_profile.as_deref().unwrap_or("—"),
                row.candidate_gate_gap_profile.as_deref().unwrap_or("—"),
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

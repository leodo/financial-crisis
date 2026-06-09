use std::fmt::Write;

use crate::{ReleaseActionabilityReview, ReleaseReviewEnvelope, ReleaseRuntimeReviewDiagnostics};

pub(super) fn render_release_runtime_diagnostics_markdown(
    markdown: &mut String,
    report: &ReleaseReviewEnvelope,
) {
    let _ = writeln!(markdown, "## Runtime Diagnostics");
    let _ = writeln!(markdown);
    render_release_runtime_review_markdown(markdown, "baseline", &report.baseline_runtime_review);
    let _ = writeln!(markdown);
    render_release_runtime_review_markdown(markdown, "candidate", &report.candidate_runtime_review);
}

pub(super) fn render_release_actionability_diagnostics_markdown(
    markdown: &mut String,
    report: &ReleaseReviewEnvelope,
) {
    let _ = writeln!(markdown, "## Actionability Diagnostics");
    let _ = writeln!(markdown);
    render_release_actionability_review_markdown(
        markdown,
        "baseline",
        &report.baseline_actionability_review,
    );
    let _ = writeln!(markdown);
    render_release_actionability_review_markdown(
        markdown,
        "candidate",
        &report.candidate_actionability_review,
    );
    let _ = writeln!(markdown);
}

pub(super) fn render_release_guardrail_result_markdown(
    markdown: &mut String,
    report: &ReleaseReviewEnvelope,
) {
    let _ = writeln!(markdown, "## Guardrail Result");
    let _ = writeln!(markdown);
    render_guardrail_section(
        markdown,
        "Runtime Guard",
        &report.operational_guard_regressions,
        "No runtime guard regressions detected.",
    );
    render_guardrail_section(
        markdown,
        "Probability Guard",
        &report.probability_guard_regressions,
        "No probability-head guard regressions detected.",
    );
    render_guardrail_section(
        markdown,
        "Actionability Guard",
        &report.actionability_guard_regressions,
        "No actionability guard regressions detected.",
    );
    render_guardrail_section(
        markdown,
        "Runtime Sanity Guard",
        &report.runtime_sanity_regressions,
        "No runtime sanity regressions detected.",
    );
    render_guardrail_section(
        markdown,
        "Overall",
        &report.overall_guard_regressions,
        "No combined guard regressions detected.",
    );
}

pub(super) fn render_release_recommendation_markdown(
    markdown: &mut String,
    report: &ReleaseReviewEnvelope,
) {
    let _ = writeln!(markdown, "## Recommendation");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "{}", report.recommendation);
}

fn render_guardrail_section(
    markdown: &mut String,
    title: &str,
    regressions: &[String],
    empty_message: &str,
) {
    let _ = writeln!(markdown, "### {title}");
    let _ = writeln!(markdown);
    if regressions.is_empty() {
        let _ = writeln!(markdown, "- {empty_message}");
    } else {
        for regression in regressions {
            let _ = writeln!(markdown, "- {regression}");
        }
    }
    let _ = writeln!(markdown);
}

fn render_release_actionability_review_markdown(
    markdown: &mut String,
    role: &str,
    review: &ReleaseActionabilityReview,
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
    diagnostics: &ReleaseRuntimeReviewDiagnostics,
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
    if let Some(latest) = diagnostics.latest_probability_snapshot.as_ref() {
        let _ = writeln!(
            markdown,
            "- Latest probabilities ({}): p5d={} / p20d={} / p60d={} / p20d_vs_p5d={} / p20d_vs_p60d={}",
            latest.as_of_date,
            format_runtime_probability(latest.p_5d),
            format_runtime_probability(latest.p_20d),
            format_runtime_probability(latest.p_60d),
            crate::format_optional_ratio(latest.p20d_vs_p5d_ratio),
            crate::format_optional_ratio(latest.p20d_vs_p60d_ratio),
        );
    }
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

fn format_runtime_probability(value: f64) -> String {
    let percent = value * 100.0;
    let absolute = percent.abs();
    if absolute == 0.0 {
        return "0%".to_string();
    }
    if absolute < 0.0001 {
        return format!("{percent:.6}%");
    }
    if absolute < 0.01 {
        return format!("{percent:.4}%");
    }
    if absolute < 0.1 {
        return format!("{percent:.3}%");
    }
    format!("{percent:.2}%")
}

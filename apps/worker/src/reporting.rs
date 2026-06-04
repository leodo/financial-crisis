use std::{fmt::Write, fs, path::Path};

use anyhow::Result;
use serde::Serialize;

use crate::{AuditExportEnvelope, FormalDatasetSummaryEnvelope, ReleaseReviewEnvelope};

pub(crate) fn write_release_review_report(
    output_dir: &Path,
    report: &ReleaseReviewEnvelope,
) -> Result<()> {
    let stem = format!(
        "{}-{}-vs-{}-{}-release-review",
        report.candidate_assessment.as_of_date,
        report.baseline_release.manifest.release_id,
        report.candidate_release.manifest.release_id,
        report.history_mode,
    );
    write_json_markdown_report(
        output_dir,
        &stem,
        report,
        crate::render_release_review_markdown(report),
        "Release review",
    )
}

pub(crate) fn write_rolling_audit_report(
    output_dir: &Path,
    report: &AuditExportEnvelope,
) -> Result<()> {
    let stem = format!("{}-rolling-audit", report.assessment.as_of_date);
    write_json_markdown_report(
        output_dir,
        &stem,
        report,
        render_audit_report_markdown(report),
        "Rolling audit report",
    )
}

pub(crate) fn write_formal_dataset_summary_report(
    output_dir: &Path,
    summary: &FormalDatasetSummaryEnvelope,
) -> Result<()> {
    let stem = format!(
        "{}-{}-formal-dataset-summary",
        summary.dataset.manifest.dataset_id, summary.dataset.manifest.dataset_version
    );
    write_json_markdown_report(
        output_dir,
        &stem,
        summary,
        crate::render_formal_dataset_summary_markdown(summary),
        "Formal dataset summary",
    )
}

fn write_json_markdown_report<T: Serialize>(
    output_dir: &Path,
    stem: &str,
    payload: &T,
    markdown: String,
    label: &str,
) -> Result<()> {
    fs::create_dir_all(output_dir)?;
    let json_path = output_dir.join(format!("{stem}.json"));
    let markdown_path = output_dir.join(format!("{stem}.md"));
    fs::write(&json_path, serde_json::to_string_pretty(payload)?)?;
    fs::write(&markdown_path, markdown)?;
    println!("{label} exported.");
    println!("  JSON     {}", json_path.display());
    println!("  Markdown {}", markdown_path.display());
    Ok(())
}

fn render_audit_report_markdown(report: &AuditExportEnvelope) -> String {
    let rolling_audit = &report.assessment.backtest_summary.rolling_audit;
    let mut markdown = String::new();
    let _ = writeln!(markdown, "# Rolling Audit Report");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "- Exported at: {}", report.exported_at);
    let _ = writeln!(markdown, "- API base: {}", report.api_base_url);
    let _ = writeln!(markdown, "- As of: {}", report.assessment.as_of_date);
    let _ = writeln!(
        markdown,
        "- Data mode: {}",
        crate::data_mode_text(report.assessment.runtime.data_mode)
    );
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Current Assessment");
    let _ = writeln!(markdown);
    let _ = writeln!(
        markdown,
        "- Overall score: {:.1}",
        report.assessment.scores.overall_score
    );
    let _ = writeln!(
        markdown,
        "- Posture: {}",
        crate::posture_text(report.assessment.posture)
    );
    let _ = writeln!(
        markdown,
        "- Time bucket: {}",
        crate::time_bucket_text(report.assessment.time_to_risk_bucket)
    );
    let _ = writeln!(
        markdown,
        "- Probability 5d / 20d / 60d: {} / {} / {}",
        crate::format_pct(report.assessment.probabilities.p_5d),
        crate::format_pct(report.assessment.probabilities.p_20d),
        crate::format_pct(report.assessment.probabilities.p_60d)
    );
    let _ = writeln!(markdown, "- Summary: {}", report.assessment.summary);
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Serving Method");
    let _ = writeln!(markdown);
    let _ = writeln!(
        markdown,
        "- Release ID: {}",
        report
            .method
            .method
            .release_id
            .as_deref()
            .unwrap_or("inline")
    );
    let _ = writeln!(
        markdown,
        "- Probability mode: {}",
        report.method.method.probability_mode
    );
    let _ = writeln!(
        markdown,
        "- Release status: {}",
        report.method.method.release_status
    );
    let _ = writeln!(
        markdown,
        "- Point-in-time mode: {}",
        report.method.method.point_in_time_mode
    );
    let _ = writeln!(
        markdown,
        "- Versions: score={} prob={} calib={} feature={} label={} playbook={}",
        report.method.method.score_method_version,
        report.method.method.prob_model_version,
        report.method.method.calibration_version,
        report.method.method.feature_set_version,
        report.method.method.label_version,
        report.method.method.action_playbook_version
    );
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Rolling Audit");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "{}", rolling_audit.summary);
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "| Metric | Value |");
    let _ = writeln!(markdown, "| --- | --- |");
    let _ = writeln!(
        markdown,
        "| Actionable precision | {} |",
        crate::format_pct(rolling_audit.actionable_precision)
    );
    let _ = writeln!(
        markdown,
        "| Actionable signal count | {} |",
        rolling_audit.actionable_signal_count
    );
    let _ = writeln!(
        markdown,
        "| Pre-crisis hit count | {} |",
        rolling_audit.pre_crisis_signal_count
    );
    let _ = writeln!(
        markdown,
        "| In-crisis signal count | {} |",
        rolling_audit.in_crisis_signal_count
    );
    let _ = writeln!(
        markdown,
        "| Protected stress count | {} |",
        rolling_audit.stress_window_signal_count
    );
    let _ = writeln!(
        markdown,
        "| Pure false-positive count | {} |",
        rolling_audit.false_positive_signal_count
    );
    let _ = writeln!(
        markdown,
        "| Pure false-positive episodes | {} |",
        rolling_audit.false_positive_episode_count
    );
    let _ = writeln!(
        markdown,
        "| Longest pure false-positive episode | {}d |",
        rolling_audit.longest_false_positive_episode_days
    );
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Largest Non-crisis Action Episodes");
    let _ = writeln!(markdown);
    let _ = writeln!(
        markdown,
        "| Classification | Window | Duration | Signals | Note |"
    );
    let _ = writeln!(markdown, "| --- | --- | --- | --- | --- |");
    for episode in &rolling_audit.classified_episodes {
        let _ = writeln!(
            markdown,
            "| {} | {} .. {} | {}d | {} | {} |",
            episode.classification,
            episode.start_date,
            episode.end_date,
            episode.duration_days,
            episode.signal_count,
            episode.note.replace('|', "/")
        );
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Scenario Backtests");
    let _ = writeln!(markdown);
    let _ = writeln!(
        markdown,
        "| Scenario | Source | Crisis Window | Structural Lead | Actionable Lead | Max Score | Foldbacks | Note |"
    );
    let _ = writeln!(
        markdown,
        "| --- | --- | --- | --- | --- | --- | --- | --- |"
    );
    for scenario in &report.backtests {
        let _ = writeln!(
            markdown,
            "| {} | {} | {} .. {} | {} | {} | {:.1} | {} | {} |",
            scenario.name,
            crate::backtest_signal_source_text(scenario.signal_source),
            scenario.crisis_start,
            scenario.crisis_end,
            crate::format_optional_days(scenario.lead_time_days),
            crate::format_optional_days(scenario.actionable_lead_time_days),
            scenario.max_score,
            scenario.false_positive_count,
            scenario.note.replace('|', "/")
        );
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Protected Stress Window Catalog");
    let _ = writeln!(markdown);
    let _ = writeln!(
        markdown,
        "- Catalog: {}",
        report.method.protected_stress_window_catalog.catalog_id
    );
    let _ = writeln!(
        markdown,
        "- Source: {}",
        report.method.protected_stress_window_catalog.source
    );
    let _ = writeln!(
        markdown,
        "- Note: {}",
        report.method.protected_stress_window_catalog.note
    );
    if let Some(warning) = &report.method.protected_stress_window_catalog.warning {
        let _ = writeln!(markdown, "- Warning: {warning}");
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "| Window | Range | Note |");
    let _ = writeln!(markdown, "| --- | --- | --- |");
    for window in &report.method.protected_stress_window_catalog.windows {
        let _ = writeln!(
            markdown,
            "| {} | {} .. {} | {} |",
            window.label,
            window.start_date,
            window.end_date,
            window.note.replace('|', "/")
        );
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Method Note");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "{}", report.method.note);
    markdown
}

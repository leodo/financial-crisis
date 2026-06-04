use std::{fs, path::Path};

use anyhow::Result;
use serde::Serialize;

use crate::{FormalDatasetSummaryEnvelope, ReleaseReviewEnvelope};

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

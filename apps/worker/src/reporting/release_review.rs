use std::fmt::Write;

use crate::ReleaseReviewEnvelope;

mod diagnostics;
mod focus;
mod historical;
mod overview;

use diagnostics::{
    render_release_actionability_diagnostics_markdown, render_release_guardrail_result_markdown,
    render_release_recommendation_markdown, render_release_runtime_diagnostics_markdown,
};
use focus::render_release_focus_scenarios_markdown;
use historical::render_release_historical_audit_markdown;
use overview::render_release_review_overview_markdown;

pub(crate) fn render_release_review_markdown_impl(report: &ReleaseReviewEnvelope) -> String {
    let mut markdown = String::new();
    render_release_review_header(&mut markdown, report);
    render_release_review_overview_markdown(&mut markdown, report);
    render_release_runtime_diagnostics_markdown(&mut markdown, report);
    render_release_historical_audit_markdown(&mut markdown, report);
    render_release_focus_scenarios_markdown(&mut markdown, report);
    render_release_actionability_diagnostics_markdown(&mut markdown, report);
    render_release_guardrail_result_markdown(&mut markdown, report);
    render_release_recommendation_markdown(&mut markdown, report);
    markdown
}

fn render_release_review_header(markdown: &mut String, report: &ReleaseReviewEnvelope) {
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
}

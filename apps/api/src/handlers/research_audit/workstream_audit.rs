use std::{fs, path::Path as FsPath};

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

use super::{read_json_artifact, ReleaseReviewArtifactSummary};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct WorkstreamAuditSummary {
    pub(super) workstream: String,
    pub(super) scenario_count: usize,
    #[serde(default)]
    pub(super) scenarios: Vec<String>,
    pub(super) covered_scenario_count: usize,
    pub(super) missing_scenario_count: usize,
    #[serde(default)]
    pub(super) missing_scenarios: Vec<String>,
    #[serde(default)]
    pub(super) training_roles: Vec<String>,
    #[serde(default)]
    pub(super) scenario_families: Vec<String>,
    pub(super) total_rows: usize,
    pub(super) total_positive_label_5d_count: usize,
    pub(super) total_positive_label_20d_count: usize,
    pub(super) total_positive_label_60d_count: usize,
    pub(super) total_prepare_primary_count: usize,
    pub(super) total_hedge_primary_count: usize,
    pub(super) total_defend_primary_count: usize,
    pub(super) total_protected_row_count: usize,
    pub(super) avg_coverage_score: Option<f64>,
    pub(super) avg_core_feature_coverage: Option<f64>,
    pub(super) avg_trigger_feature_coverage: Option<f64>,
    pub(super) avg_external_feature_coverage: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct WorkstreamAuditScenarioSummary {
    pub(super) scenario_id: String,
    pub(super) scenario_name: String,
    pub(super) workstream: String,
    pub(super) scenario_family: String,
    pub(super) training_role: String,
    pub(super) protected_window: bool,
    pub(super) suggested_review: Option<String>,
    pub(super) window_start: String,
    pub(super) window_end: String,
    pub(super) crisis_start: Option<String>,
    pub(super) crisis_end: Option<String>,
    pub(super) slice_status: String,
    pub(super) slice_selector_reason: String,
    #[serde(default)]
    pub(super) attempted_datasets: Vec<String>,
    pub(super) dataset_key: String,
    pub(super) feature_set_version: String,
    pub(super) label_version: String,
    pub(super) row_count: usize,
    #[serde(default)]
    pub(super) split_counts: Vec<String>,
    #[serde(default)]
    pub(super) quality_counts: Vec<String>,
    #[serde(default)]
    pub(super) regime20_counts: Vec<String>,
    #[serde(default)]
    pub(super) regime60_counts: Vec<String>,
    #[serde(default)]
    pub(super) action_phase_counts: Vec<String>,
    #[serde(default)]
    pub(super) primary_action_level_counts: Vec<String>,
    pub(super) avg_coverage_score: Option<f64>,
    pub(super) avg_core_feature_coverage: Option<f64>,
    pub(super) avg_trigger_feature_coverage: Option<f64>,
    pub(super) avg_external_feature_coverage: Option<f64>,
    pub(super) positive_label_5d_count: usize,
    pub(super) positive_label_20d_count: usize,
    pub(super) positive_label_60d_count: usize,
    pub(super) prepare_primary_count: usize,
    pub(super) hedge_primary_count: usize,
    pub(super) defend_primary_count: usize,
    pub(super) protected_row_count: usize,
    pub(super) feature_name_count: usize,
    #[serde(default)]
    pub(super) feature_names: Vec<String>,
    pub(super) slice_json_path: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct WorkstreamAuditArtifactWire {
    generated_at: String,
    review_report_path: String,
    baseline_release_id: String,
    candidate_release_id: String,
    history_mode: String,
    market_scope: String,
    dataset_key: String,
    dataset_id: String,
    dataset_version: String,
    #[serde(default)]
    requested_workstreams: Vec<String>,
    #[serde(default)]
    workstream_summaries: Vec<WorkstreamAuditSummary>,
    #[serde(default)]
    scenario_summaries: Vec<WorkstreamAuditScenarioSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct WorkstreamAuditArtifactSummary {
    pub(super) generated_at: String,
    pub(super) source: String,
    pub(super) review_report_path: String,
    pub(super) baseline_release_id: String,
    pub(super) candidate_release_id: String,
    pub(super) history_mode: String,
    pub(super) market_scope: String,
    pub(super) dataset_key: String,
    pub(super) dataset_id: String,
    pub(super) dataset_version: String,
    pub(super) requested_workstreams: Vec<String>,
    pub(super) workstream_summaries: Vec<WorkstreamAuditSummary>,
    pub(super) scenario_summaries: Vec<WorkstreamAuditScenarioSummary>,
}

pub(super) fn load_latest_workstream_audit_summary(
    market_scope: &str,
    active_release_id: Option<&str>,
    release_review: Option<&ReleaseReviewArtifactSummary>,
) -> Option<WorkstreamAuditArtifactSummary> {
    let review_releases = release_review.map(|review| {
        [
            review.original_active_release_id.as_str(),
            review.restored_release_id.as_str(),
            review.baseline_release_id.as_str(),
            review.candidate_release_id.as_str(),
        ]
    });
    let review_pair = release_review.map(|review| {
        (
            review.baseline_release_id.as_str(),
            review.candidate_release_id.as_str(),
            review.history_mode.as_str(),
        )
    });
    let mut candidates = Vec::<(
        bool,
        usize,
        bool,
        usize,
        Option<DateTime<FixedOffset>>,
        WorkstreamAuditArtifactSummary,
    )>::new();
    for directory in ["artifacts/research/workstream-audit"] {
        let path = FsPath::new(directory);
        let Ok(entries) = fs::read_dir(path) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                continue;
            }
            let Some(body) = read_json_artifact(&path) else {
                continue;
            };
            let wire = match serde_json::from_str::<WorkstreamAuditArtifactWire>(&body) {
                Ok(wire) => wire,
                Err(error) => {
                    tracing::warn!(
                        path = %path.to_string_lossy(),
                        %error,
                        "failed to parse workstream audit artifact"
                    );
                    continue;
                }
            };
            if wire.market_scope != market_scope {
                continue;
            }
            let active_match = active_release_id.is_some_and(|release_id| {
                wire.baseline_release_id == release_id || wire.candidate_release_id == release_id
            }) || review_releases.is_some_and(|release_ids| {
                release_ids.contains(&wire.baseline_release_id.as_str())
                    || release_ids.contains(&wire.candidate_release_id.as_str())
            });
            let review_pair_match =
                review_pair.is_some_and(|(baseline, candidate, history_mode)| {
                    wire.baseline_release_id == baseline
                        && wire.candidate_release_id == candidate
                        && wire.history_mode == history_mode
                });
            candidates.push((
                active_match,
                wire.workstream_summaries.len(),
                review_pair_match,
                wire.scenario_summaries.len(),
                DateTime::parse_from_rfc3339(&wire.generated_at).ok(),
                WorkstreamAuditArtifactSummary {
                    generated_at: wire.generated_at,
                    source: path.to_string_lossy().into_owned(),
                    review_report_path: wire.review_report_path,
                    baseline_release_id: wire.baseline_release_id,
                    candidate_release_id: wire.candidate_release_id,
                    history_mode: wire.history_mode,
                    market_scope: wire.market_scope,
                    dataset_key: wire.dataset_key,
                    dataset_id: wire.dataset_id,
                    dataset_version: wire.dataset_version,
                    requested_workstreams: wire.requested_workstreams,
                    workstream_summaries: wire.workstream_summaries,
                    scenario_summaries: wire.scenario_summaries,
                },
            ));
        }
    }

    candidates.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| right.1.cmp(&left.1))
            .then_with(|| right.2.cmp(&left.2))
            .then_with(|| right.3.cmp(&left.3))
            .then_with(|| right.4.cmp(&left.4))
            .then_with(|| right.5.generated_at.cmp(&left.5.generated_at))
    });
    candidates
        .into_iter()
        .next()
        .map(|(_, _, _, _, _, summary)| summary)
}

#[cfg(test)]
mod tests {
    use super::WorkstreamAuditArtifactWire;

    #[test]
    fn workstream_wire_allows_missing_optional_arrays() {
        let body = r#"
        {
          "generated_at": "2026-06-09T00:00:00+00:00",
          "review_report_path": "review.json",
          "baseline_release_id": "baseline_release",
          "candidate_release_id": "candidate_release",
          "history_mode": "default",
          "market_scope": "financial_system",
          "dataset_key": "",
          "dataset_id": "",
          "dataset_version": ""
        }
        "#;

        let wire: WorkstreamAuditArtifactWire =
            serde_json::from_str(body).expect("wire should deserialize");
        assert!(wire.requested_workstreams.is_empty());
        assert!(wire.workstream_summaries.is_empty());
        assert!(wire.scenario_summaries.is_empty());
    }
}

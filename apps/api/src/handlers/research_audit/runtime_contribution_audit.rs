use std::{fs, path::Path as FsPath};

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

use super::{read_json_artifact, ReleaseReviewArtifactSummary};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct RuntimeContributionBaseContributionSummary {
    pub(super) name: String,
    pub(super) observed_count: usize,
    pub(super) row_coverage_ratio: f64,
    pub(super) mean_contribution: f64,
    pub(super) mean_weight: f64,
    pub(super) mean_raw_value: f64,
    pub(super) mean_normalized_value: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct RuntimeContributionDeltaContributionSummary {
    pub(super) name: String,
    pub(super) baseline_mean_contribution: Option<f64>,
    pub(super) candidate_mean_contribution: Option<f64>,
    pub(super) delta_mean_contribution: Option<f64>,
    pub(super) baseline_observed_count: usize,
    pub(super) candidate_observed_count: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct RuntimeContributionSemanticAnomaly {
    pub(super) code: String,
    pub(super) horizon_days: u32,
    pub(super) feature: String,
    pub(super) mean_raw_value: f64,
    pub(super) mean_contribution: f64,
    pub(super) message: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct RuntimeContributionDateRow {
    pub(super) as_of_date: String,
    pub(super) baseline_runtime_probability: f64,
    pub(super) candidate_runtime_probability: f64,
    pub(super) baseline_touchline_ratio: f64,
    pub(super) candidate_touchline_ratio: f64,
    pub(super) baseline_time_to_risk_bucket: String,
    pub(super) candidate_time_to_risk_bucket: String,
    pub(super) baseline_posture: String,
    pub(super) candidate_posture: String,
    pub(super) candidate_runtime_group: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct RuntimeContributionGroupSummary {
    pub(super) group: String,
    pub(super) label: String,
    pub(super) date_count: usize,
    pub(super) baseline_decision_threshold: f64,
    pub(super) candidate_decision_threshold: f64,
    pub(super) baseline_avg_runtime_probability: f64,
    pub(super) candidate_avg_runtime_probability: f64,
    pub(super) delta_avg_runtime_probability: f64,
    pub(super) baseline_touchline_ratio: f64,
    pub(super) candidate_touchline_ratio: f64,
    pub(super) baseline_rows_with_base_contributions: usize,
    pub(super) candidate_rows_with_base_contributions: usize,
    #[serde(default)]
    pub(super) baseline_top_negative_base_contributions:
        Vec<RuntimeContributionBaseContributionSummary>,
    #[serde(default)]
    pub(super) candidate_top_negative_base_contributions:
        Vec<RuntimeContributionBaseContributionSummary>,
    #[serde(default)]
    pub(super) top_abs_delta_base_contributions: Vec<RuntimeContributionDeltaContributionSummary>,
    #[serde(default)]
    pub(super) baseline_semantic_anomalies: Vec<RuntimeContributionSemanticAnomaly>,
    #[serde(default)]
    pub(super) candidate_semantic_anomalies: Vec<RuntimeContributionSemanticAnomaly>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct RuntimeContributionHorizonSummary {
    pub(super) label: String,
    pub(super) date_count: usize,
    pub(super) baseline_decision_threshold: f64,
    pub(super) candidate_decision_threshold: f64,
    pub(super) baseline_avg_runtime_probability: f64,
    pub(super) candidate_avg_runtime_probability: f64,
    pub(super) delta_avg_runtime_probability: f64,
    pub(super) baseline_touchline_ratio: f64,
    pub(super) candidate_touchline_ratio: f64,
    pub(super) baseline_rows_with_base_contributions: usize,
    pub(super) candidate_rows_with_base_contributions: usize,
    #[serde(default)]
    pub(super) baseline_top_negative_base_contributions:
        Vec<RuntimeContributionBaseContributionSummary>,
    #[serde(default)]
    pub(super) candidate_top_negative_base_contributions:
        Vec<RuntimeContributionBaseContributionSummary>,
    #[serde(default)]
    pub(super) top_abs_delta_base_contributions: Vec<RuntimeContributionDeltaContributionSummary>,
    #[serde(default)]
    pub(super) baseline_semantic_anomalies: Vec<RuntimeContributionSemanticAnomaly>,
    #[serde(default)]
    pub(super) candidate_semantic_anomalies: Vec<RuntimeContributionSemanticAnomaly>,
    pub(super) horizon_days: u32,
    #[serde(default)]
    pub(super) date_rows: Vec<RuntimeContributionDateRow>,
    #[serde(default)]
    pub(super) runtime_group_summaries: Vec<RuntimeContributionGroupSummary>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct RuntimeContributionAuditArtifactWire {
    pub(super) generated_at: String,
    pub(super) market_scope: String,
    pub(super) history_mode: String,
    pub(super) from_date: String,
    pub(super) to_date: String,
    pub(super) baseline_release_id: String,
    pub(super) candidate_release_id: String,
    pub(super) baseline_slice_path: String,
    pub(super) candidate_slice_path: String,
    pub(super) baseline_replay_run_id: String,
    pub(super) candidate_replay_run_id: String,
    pub(super) baseline_threshold_source: String,
    pub(super) candidate_threshold_source: String,
    pub(super) runtime_threshold_source: String,
    pub(super) common_date_count: usize,
    #[serde(default)]
    pub(super) methodology_limitations: Vec<String>,
    #[serde(default)]
    pub(super) horizons: Vec<RuntimeContributionHorizonSummary>,
    #[serde(default)]
    pub(super) takeaways: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct RuntimeContributionAuditArtifactSummary {
    pub(super) generated_at: String,
    pub(super) source: String,
    pub(super) market_scope: String,
    pub(super) history_mode: String,
    pub(super) from_date: String,
    pub(super) to_date: String,
    pub(super) baseline_release_id: String,
    pub(super) candidate_release_id: String,
    pub(super) baseline_slice_path: String,
    pub(super) candidate_slice_path: String,
    pub(super) baseline_replay_run_id: String,
    pub(super) candidate_replay_run_id: String,
    pub(super) baseline_threshold_source: String,
    pub(super) candidate_threshold_source: String,
    pub(super) runtime_threshold_source: String,
    pub(super) common_date_count: usize,
    pub(super) methodology_limitations: Vec<String>,
    pub(super) horizons: Vec<RuntimeContributionHorizonSummary>,
    pub(super) takeaways: Vec<String>,
}

pub(super) fn load_latest_runtime_contribution_audit_summary(
    market_scope: &str,
    release_review: Option<&ReleaseReviewArtifactSummary>,
) -> Option<RuntimeContributionAuditArtifactSummary> {
    let release_review = release_review?;
    let mut candidates = Vec::<(
        Option<DateTime<FixedOffset>>,
        RuntimeContributionAuditArtifactSummary,
    )>::new();

    for directory in ["artifacts/research/runtime-contribution-audit"] {
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
            let wire = match serde_json::from_str::<RuntimeContributionAuditArtifactWire>(&body) {
                Ok(wire) => wire,
                Err(error) => {
                    tracing::warn!(
                        path = %path.to_string_lossy(),
                        %error,
                        "failed to parse runtime contribution audit artifact"
                    );
                    continue;
                }
            };
            if wire.market_scope != market_scope
                || wire.baseline_release_id != release_review.baseline_release_id
                || wire.candidate_release_id != release_review.candidate_release_id
                || wire.history_mode != release_review.history_mode
            {
                continue;
            }
            candidates.push((
                DateTime::parse_from_rfc3339(&wire.generated_at).ok(),
                RuntimeContributionAuditArtifactSummary {
                    generated_at: wire.generated_at,
                    source: path.to_string_lossy().into_owned(),
                    market_scope: wire.market_scope,
                    history_mode: wire.history_mode,
                    from_date: wire.from_date,
                    to_date: wire.to_date,
                    baseline_release_id: wire.baseline_release_id,
                    candidate_release_id: wire.candidate_release_id,
                    baseline_slice_path: wire.baseline_slice_path,
                    candidate_slice_path: wire.candidate_slice_path,
                    baseline_replay_run_id: wire.baseline_replay_run_id,
                    candidate_replay_run_id: wire.candidate_replay_run_id,
                    baseline_threshold_source: wire.baseline_threshold_source,
                    candidate_threshold_source: wire.candidate_threshold_source,
                    runtime_threshold_source: wire.runtime_threshold_source,
                    common_date_count: wire.common_date_count,
                    methodology_limitations: wire.methodology_limitations,
                    horizons: wire.horizons,
                    takeaways: wire.takeaways,
                },
            ));
        }
    }

    candidates.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| right.1.generated_at.cmp(&left.1.generated_at))
    });
    candidates.into_iter().next().map(|(_, summary)| summary)
}

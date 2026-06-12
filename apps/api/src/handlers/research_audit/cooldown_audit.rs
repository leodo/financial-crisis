use std::{fs, path::Path as FsPath};

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{read_json_artifact, ReleaseReviewArtifactSummary};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct CooldownAuditNoGoReason {
    pub(super) code: String,
    pub(super) summary: String,
    #[serde(default)]
    pub(super) evidence: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct CooldownAuditRuntimeRow {
    pub(super) horizon_days: u32,
    pub(super) baseline_diagnosis: Option<String>,
    pub(super) candidate_diagnosis: Option<String>,
    pub(super) candidate_cooldown_minus_positive: Option<f64>,
    pub(super) candidate_cooldown_minus_normal: Option<f64>,
    #[serde(default)]
    pub(super) comparison: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct CooldownAuditFalsePositiveEpisode {
    pub(super) start_date: String,
    pub(super) end_date: String,
    pub(super) duration_days: usize,
    pub(super) signal_count: usize,
    pub(super) classification: String,
    pub(super) note: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct CooldownAuditEpisodeRegression {
    pub(super) kind: String,
    pub(super) episode: CooldownAuditFalsePositiveEpisode,
    #[serde(default)]
    pub(super) overlapping_baseline_episodes: Vec<CooldownAuditFalsePositiveEpisode>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct CooldownAuditFalsePositiveEpisodes {
    #[serde(default)]
    pub(super) baseline_top: Vec<CooldownAuditFalsePositiveEpisode>,
    #[serde(default)]
    pub(super) candidate_top: Vec<CooldownAuditFalsePositiveEpisode>,
    #[serde(default)]
    pub(super) candidate_regressions: Vec<CooldownAuditEpisodeRegression>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct CooldownAuditScenarioFalsePositiveDelta {
    pub(super) scenario_id: String,
    pub(super) name: String,
    pub(super) baseline_false_positive_count: usize,
    pub(super) candidate_false_positive_count: usize,
    pub(super) delta: i64,
    pub(super) outcome: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct CooldownAuditArtifactWire {
    pub(super) generated_at: String,
    pub(super) baseline_release_id: String,
    pub(super) candidate_release_id: String,
    pub(super) history_mode: String,
    pub(super) release_review_artifact: String,
    pub(super) reviewed_at: Option<String>,
    pub(super) recommendation: String,
    #[serde(default)]
    pub(super) comparison_metrics: Value,
    #[serde(default)]
    pub(super) runtime_cooldown_rows: Vec<CooldownAuditRuntimeRow>,
    #[serde(default)]
    pub(super) false_positive_episodes: CooldownAuditFalsePositiveEpisodes,
    #[serde(default)]
    pub(super) scenario_false_positive_deltas: Vec<CooldownAuditScenarioFalsePositiveDelta>,
    #[serde(default)]
    pub(super) no_go_reasons: Vec<CooldownAuditNoGoReason>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct CooldownAuditArtifactSummary {
    pub(super) generated_at: String,
    pub(super) source: String,
    pub(super) baseline_release_id: String,
    pub(super) candidate_release_id: String,
    pub(super) history_mode: String,
    pub(super) release_review_artifact: String,
    pub(super) reviewed_at: Option<String>,
    pub(super) recommendation: String,
    pub(super) comparison_metrics: Value,
    pub(super) runtime_cooldown_rows: Vec<CooldownAuditRuntimeRow>,
    pub(super) false_positive_episodes: CooldownAuditFalsePositiveEpisodes,
    pub(super) scenario_false_positive_deltas: Vec<CooldownAuditScenarioFalsePositiveDelta>,
    pub(super) no_go_reasons: Vec<CooldownAuditNoGoReason>,
}

pub(super) fn load_latest_cooldown_audit_summary(
    release_review: Option<&ReleaseReviewArtifactSummary>,
) -> Option<CooldownAuditArtifactSummary> {
    let release_review = release_review?;
    let mut candidates =
        Vec::<(Option<DateTime<FixedOffset>>, CooldownAuditArtifactSummary)>::new();

    for directory in ["artifacts/research/cooldown-audit"] {
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
            let wire = match serde_json::from_str::<CooldownAuditArtifactWire>(&body) {
                Ok(wire) => wire,
                Err(error) => {
                    tracing::warn!(
                        path = %path.to_string_lossy(),
                        %error,
                        "failed to parse cooldown audit artifact"
                    );
                    continue;
                }
            };
            if wire.baseline_release_id != release_review.baseline_release_id
                || wire.candidate_release_id != release_review.candidate_release_id
                || wire.history_mode != release_review.history_mode
            {
                continue;
            }
            candidates.push((
                DateTime::parse_from_rfc3339(&wire.generated_at).ok(),
                CooldownAuditArtifactSummary {
                    generated_at: wire.generated_at,
                    source: path.to_string_lossy().into_owned(),
                    baseline_release_id: wire.baseline_release_id,
                    candidate_release_id: wire.candidate_release_id,
                    history_mode: wire.history_mode,
                    release_review_artifact: wire.release_review_artifact,
                    reviewed_at: wire.reviewed_at,
                    recommendation: wire.recommendation,
                    comparison_metrics: wire.comparison_metrics,
                    runtime_cooldown_rows: wire.runtime_cooldown_rows,
                    false_positive_episodes: wire.false_positive_episodes,
                    scenario_false_positive_deltas: wire.scenario_false_positive_deltas,
                    no_go_reasons: wire.no_go_reasons,
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

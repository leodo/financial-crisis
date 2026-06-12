use std::{fs, path::Path as FsPath};

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

use super::{read_json_artifact, ReleaseReviewArtifactSummary};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct PrewarningGapHitSummary {
    pub(super) hit_count: usize,
    pub(super) segment_count: usize,
    pub(super) max_streak: usize,
    pub(super) first_hit_date: Option<String>,
    pub(super) last_hit_date: Option<String>,
    pub(super) max_streak_start: Option<String>,
    pub(super) max_streak_end: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct PrewarningGapDatasetEvidence {
    pub(super) dataset_key: Option<String>,
    pub(super) row_count: usize,
    #[serde(default)]
    pub(super) split_counts: Vec<String>,
    #[serde(default)]
    pub(super) regime20_counts: Vec<String>,
    #[serde(default)]
    pub(super) regime60_counts: Vec<String>,
    #[serde(default)]
    pub(super) action_phase_counts: Vec<String>,
    #[serde(default)]
    pub(super) primary_action_level_counts: Vec<String>,
    #[serde(default)]
    pub(super) label_20d_count: usize,
    #[serde(default)]
    pub(super) label_60d_count: usize,
    #[serde(default)]
    pub(super) prepare_primary_count: usize,
    #[serde(default)]
    pub(super) hedge_primary_count: usize,
    #[serde(default)]
    pub(super) protected_row_count: usize,
    #[serde(default)]
    pub(super) avg_coverage_score: Option<f64>,
    #[serde(default)]
    pub(super) feature_name_count: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct PrewarningGapProbabilityEvidence {
    pub(super) compare_status: Option<String>,
    #[serde(default)]
    pub(super) compare_row_count: usize,
    #[serde(default)]
    pub(super) baseline_hit_20d: PrewarningGapHitSummary,
    #[serde(default)]
    pub(super) candidate_hit_20d: PrewarningGapHitSummary,
    #[serde(default)]
    pub(super) baseline_hit_60d: PrewarningGapHitSummary,
    #[serde(default)]
    pub(super) candidate_hit_60d: PrewarningGapHitSummary,
    #[serde(default)]
    pub(super) candidate_near_threshold_20d_5pp_count: usize,
    #[serde(default)]
    pub(super) candidate_near_threshold_60d_5pp_count: usize,
    #[serde(default)]
    pub(super) baseline_max_p_20d: Option<f64>,
    #[serde(default)]
    pub(super) candidate_max_p_20d: Option<f64>,
    #[serde(default)]
    pub(super) baseline_max_p_60d: Option<f64>,
    #[serde(default)]
    pub(super) candidate_max_p_60d: Option<f64>,
    #[serde(default)]
    pub(super) candidate_avg_p_20d: Option<f64>,
    #[serde(default)]
    pub(super) candidate_avg_p_60d: Option<f64>,
    #[serde(default)]
    pub(super) baseline_avg_p_20d: Option<f64>,
    #[serde(default)]
    pub(super) baseline_avg_p_60d: Option<f64>,
    #[serde(default)]
    pub(super) avg_delta_p_20d: Option<f64>,
    #[serde(default)]
    pub(super) avg_delta_p_60d: Option<f64>,
    #[serde(default)]
    pub(super) positive_window_candidate_hit_rate_20d: Option<f64>,
    #[serde(default)]
    pub(super) hedge_window_candidate_hit_rate_20d: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct PrewarningGapDiagnosis {
    pub(super) gap_class: String,
    #[serde(default)]
    pub(super) reasons: Vec<String>,
    pub(super) next_action: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct PrewarningGapScenarioSummary {
    pub(super) scenario_id: String,
    pub(super) scenario_label: String,
    pub(super) family: String,
    pub(super) training_role: String,
    pub(super) protected_window: bool,
    pub(super) pre_warning_start: String,
    pub(super) crisis_start: String,
    pub(super) crisis_end: String,
    pub(super) coverage_grade: String,
    pub(super) coverage_role: String,
    pub(super) coverage_pit_mode: String,
    #[serde(default)]
    pub(super) free_sources: Vec<String>,
    #[serde(default)]
    pub(super) blocking_gaps: Vec<String>,
    #[serde(default)]
    pub(super) dataset_evidence: PrewarningGapDatasetEvidence,
    #[serde(default)]
    pub(super) probability_evidence: PrewarningGapProbabilityEvidence,
    #[serde(default)]
    pub(super) diagnosis: PrewarningGapDiagnosis,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct PrewarningGapAuditArtifactWire {
    pub(super) generated_at: String,
    pub(super) baseline_release_id: String,
    pub(super) candidate_release_id: String,
    pub(super) market_scope: String,
    pub(super) scenario_count: usize,
    #[serde(default)]
    pub(super) gap_counts: Vec<String>,
    #[serde(default)]
    pub(super) scenario_summaries: Vec<PrewarningGapScenarioSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct PrewarningGapAuditArtifactSummary {
    pub(super) generated_at: String,
    pub(super) source: String,
    pub(super) baseline_release_id: String,
    pub(super) candidate_release_id: String,
    pub(super) market_scope: String,
    pub(super) scenario_count: usize,
    pub(super) gap_counts: Vec<String>,
    pub(super) scenario_summaries: Vec<PrewarningGapScenarioSummary>,
}

pub(super) fn load_latest_prewarning_gap_audit_summary(
    market_scope: &str,
    release_review: Option<&ReleaseReviewArtifactSummary>,
) -> Option<PrewarningGapAuditArtifactSummary> {
    let release_review = release_review?;
    let mut candidates = Vec::<(
        Option<DateTime<FixedOffset>>,
        PrewarningGapAuditArtifactSummary,
    )>::new();

    for directory in ["artifacts/research/prewarning-gap-audit"] {
        let path = FsPath::new(directory);
        let Ok(entries) = fs::read_dir(path) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                continue;
            }
            let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if !file_name.ends_with("-prewarning-gap-audit.json") {
                continue;
            }
            let Some(body) = read_json_artifact(&path) else {
                continue;
            };
            let wire = match serde_json::from_str::<PrewarningGapAuditArtifactWire>(&body) {
                Ok(wire) => wire,
                Err(error) => {
                    tracing::warn!(
                        path = %path.to_string_lossy(),
                        %error,
                        "failed to parse prewarning gap audit artifact"
                    );
                    continue;
                }
            };
            if wire.market_scope != market_scope
                || wire.baseline_release_id != release_review.baseline_release_id
                || wire.candidate_release_id != release_review.candidate_release_id
            {
                continue;
            }
            candidates.push((
                DateTime::parse_from_rfc3339(&wire.generated_at).ok(),
                PrewarningGapAuditArtifactSummary {
                    generated_at: wire.generated_at,
                    source: path.to_string_lossy().into_owned(),
                    baseline_release_id: wire.baseline_release_id,
                    candidate_release_id: wire.candidate_release_id,
                    market_scope: wire.market_scope,
                    scenario_count: wire.scenario_count,
                    gap_counts: wire.gap_counts,
                    scenario_summaries: wire.scenario_summaries,
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

#[cfg(test)]
mod tests {
    use super::PrewarningGapAuditArtifactWire;

    #[test]
    fn prewarning_gap_wire_allows_missing_optional_arrays() {
        let wire: PrewarningGapAuditArtifactWire = serde_json::from_str(
            r#"{
              "generated_at": "2026-06-09T10:35:07Z",
              "baseline_release_id": "baseline",
              "candidate_release_id": "candidate",
              "market_scope": "financial_system",
              "scenario_count": 1,
              "scenario_summaries": [{
                "scenario_id": "us_funding_stress_2011",
                "scenario_label": "2011 美欧融资压力",
                "family": "mixed_systemic",
                "training_role": "extension_only",
                "protected_window": true,
                "pre_warning_start": "2011-07-01",
                "crisis_start": "2011-08-01",
                "crisis_end": "2011-10-31",
                "coverage_grade": "A-",
                "coverage_role": "protected_stress + extension_training",
                "coverage_pit_mode": "best_effort",
                "diagnosis": {
                  "gap_class": "no_runtime_floor_signal"
                }
              }]
            }"#,
        )
        .expect("wire should decode");

        assert!(wire.gap_counts.is_empty());
        assert_eq!(wire.scenario_summaries[0].dataset_evidence.row_count, 0);
        assert_eq!(
            wire.scenario_summaries[0].diagnosis.gap_class,
            "no_runtime_floor_signal"
        );
    }
}

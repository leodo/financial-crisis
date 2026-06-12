use std::{collections::BTreeMap, fs, path::Path as FsPath};

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

use super::{read_json_artifact, ReleaseReviewArtifactSummary};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct FundingStressCountSummary {
    pub(super) value: String,
    pub(super) count: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct FundingStressMaxSummary {
    pub(super) value: Option<f64>,
    pub(super) date: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct FundingStressHitSummary {
    pub(super) hit_count: usize,
    pub(super) segment_count: usize,
    pub(super) max_streak: usize,
    pub(super) first_hit_date: Option<String>,
    pub(super) last_hit_date: Option<String>,
    pub(super) max_streak_start: Option<String>,
    pub(super) max_streak_end: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct FundingStressNearThresholdSummary {
    pub(super) count: usize,
    pub(super) first_date: Option<String>,
    pub(super) last_date: Option<String>,
    pub(super) max_value: Option<f64>,
    pub(super) min_gap_to_threshold: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct FundingStressThresholdSummary {
    pub(super) baseline_20d: Option<f64>,
    pub(super) candidate_20d: Option<f64>,
    pub(super) baseline_60d: Option<f64>,
    pub(super) candidate_60d: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct FundingStressGroupSummary {
    pub(super) label: String,
    pub(super) row_count: usize,
    pub(super) avg_baseline_p20d: Option<f64>,
    pub(super) avg_candidate_p20d: Option<f64>,
    pub(super) avg_delta_p20d: Option<f64>,
    pub(super) avg_candidate_p60d: Option<f64>,
    #[serde(default)]
    pub(super) candidate_max_p20d: FundingStressMaxSummary,
    #[serde(default)]
    pub(super) candidate_max_p60d: FundingStressMaxSummary,
    #[serde(default)]
    pub(super) candidate_hit_20d: FundingStressHitSummary,
    #[serde(default)]
    pub(super) candidate_hit_60d: FundingStressHitSummary,
    #[serde(default)]
    pub(super) near_candidate_20d_5pp: FundingStressNearThresholdSummary,
    #[serde(default)]
    pub(super) near_candidate_60d_5pp: FundingStressNearThresholdSummary,
    #[serde(default)]
    pub(super) split_counts: Vec<FundingStressCountSummary>,
    #[serde(default)]
    pub(super) phase_counts: Vec<FundingStressCountSummary>,
    #[serde(default)]
    pub(super) action_level_counts: Vec<FundingStressCountSummary>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct FundingStressDatasetEvidence {
    #[serde(default)]
    pub(super) split_counts: Vec<FundingStressCountSummary>,
    #[serde(default)]
    pub(super) regime_20d_counts: Vec<FundingStressCountSummary>,
    #[serde(default)]
    pub(super) regime_60d_counts: Vec<FundingStressCountSummary>,
    #[serde(default)]
    pub(super) action_phase_counts: Vec<FundingStressCountSummary>,
    #[serde(default)]
    pub(super) action_level_counts: Vec<FundingStressCountSummary>,
    pub(super) protected_row_count: usize,
    pub(super) label_20d_count: usize,
    pub(super) label_60d_count: usize,
    pub(super) prepare_episode_count: usize,
    pub(super) hedge_episode_count: usize,
    pub(super) avg_coverage_score: Option<f64>,
    pub(super) feature_name_count: usize,
    #[serde(default)]
    pub(super) raw_feature_name_count: usize,
    #[serde(default)]
    pub(super) resolved_feature_name_count: usize,
    #[serde(default)]
    pub(super) available_relevant_features: Vec<String>,
    #[serde(default)]
    pub(super) missing_relevant_features: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct FundingStressProbabilityEvidence {
    #[serde(default)]
    pub(super) full_window: FundingStressGroupSummary,
    #[serde(default)]
    pub(super) primary_phase: FundingStressGroupSummary,
    #[serde(default)]
    pub(super) prepare_primary: FundingStressGroupSummary,
    #[serde(default)]
    pub(super) hedge_primary: FundingStressGroupSummary,
    #[serde(default)]
    pub(super) late_validation: FundingStressGroupSummary,
    #[serde(default)]
    pub(super) positive_window_20d: FundingStressGroupSummary,
    #[serde(default)]
    pub(super) pre_warning_buffer_20d: FundingStressGroupSummary,
    #[serde(default)]
    pub(super) normal_20d: FundingStressGroupSummary,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct FundingStressFeatureGap {
    pub(super) feature: String,
    pub(super) left_group: String,
    pub(super) right_group: String,
    pub(super) left_mean: Option<f64>,
    pub(super) right_mean: Option<f64>,
    pub(super) mean_delta: Option<f64>,
    pub(super) standardized_gap: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct FundingStressBaseContribution {
    pub(super) name: String,
    pub(super) mean_raw_value: Option<f64>,
    pub(super) mean_normalized_value: Option<f64>,
    pub(super) mean_weight: Option<f64>,
    pub(super) mean_contribution: Option<f64>,
    pub(super) sum_contribution: Option<f64>,
    pub(super) count: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct FundingStressOverlayContribution {
    pub(super) family_id: String,
    pub(super) gate_feature: String,
    pub(super) mean_gate_value: Option<f64>,
    pub(super) mean_gate: Option<f64>,
    pub(super) mean_blend: Option<f64>,
    pub(super) mean_overlay_probability: Option<f64>,
    pub(super) mean_contribution: Option<f64>,
    pub(super) sum_contribution: Option<f64>,
    pub(super) count: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct FundingStressContributionGroup {
    pub(super) label: String,
    pub(super) horizon_days: u32,
    pub(super) row_count: usize,
    #[serde(default)]
    pub(super) top_positive_base: Vec<FundingStressBaseContribution>,
    #[serde(default)]
    pub(super) top_negative_base: Vec<FundingStressBaseContribution>,
    #[serde(default)]
    pub(super) top_absolute_base: Vec<FundingStressBaseContribution>,
    #[serde(default)]
    pub(super) overlay_contributions: Vec<FundingStressOverlayContribution>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct FundingStressFeatureContext {
    #[serde(default)]
    pub(super) separation: BTreeMap<String, Vec<FundingStressFeatureGap>>,
    #[serde(default)]
    pub(super) candidate_resolved_relevant_features: Vec<FundingStressBaseContribution>,
    #[serde(default)]
    pub(super) candidate_absolute_contributions: BTreeMap<String, FundingStressContributionGroup>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct FundingStressScenarioSummary {
    pub(super) scenario_id: String,
    pub(super) label: String,
    pub(super) family: String,
    pub(super) pre_warning_start: String,
    pub(super) crisis_start: String,
    pub(super) acute_start: Option<String>,
    pub(super) crisis_end: String,
    pub(super) training_role: String,
    pub(super) protected_window: bool,
    #[serde(default)]
    pub(super) protected_action_levels: Vec<String>,
    #[serde(default)]
    pub(super) default_horizon_roles: Vec<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct FundingStressCoverageSummary {
    pub(super) coverage_grade: String,
    pub(super) recommended_role: String,
    pub(super) point_in_time_mode: String,
    #[serde(default)]
    pub(super) free_sources: Vec<String>,
    #[serde(default)]
    pub(super) blocking_gaps: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct FundingStressDiagnosis {
    pub(super) primary_class: String,
    pub(super) trainability_class: String,
    pub(super) family_context_class: String,
    pub(super) candidate_margin_class: String,
    #[serde(default)]
    pub(super) reasons: Vec<String>,
    #[serde(default)]
    pub(super) next_actions: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct FundingStressAuditArtifactWire {
    pub(super) generated_at: String,
    pub(super) compare_path: String,
    pub(super) slice_path: String,
    #[serde(default)]
    pub(super) candidate_scored_slice_path: Option<String>,
    pub(super) baseline_release_id: String,
    pub(super) candidate_release_id: String,
    pub(super) market_scope: String,
    #[serde(default)]
    pub(super) scenario: FundingStressScenarioSummary,
    #[serde(default)]
    pub(super) coverage: FundingStressCoverageSummary,
    pub(super) dataset_key: String,
    pub(super) from_date: String,
    pub(super) to_date: String,
    pub(super) row_count: usize,
    #[serde(default)]
    pub(super) thresholds: FundingStressThresholdSummary,
    #[serde(default)]
    pub(super) dataset_evidence: FundingStressDatasetEvidence,
    #[serde(default)]
    pub(super) probability_evidence: FundingStressProbabilityEvidence,
    #[serde(default)]
    pub(super) feature_context: FundingStressFeatureContext,
    #[serde(default)]
    pub(super) diagnosis: FundingStressDiagnosis,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct FundingStressAuditArtifactSummary {
    pub(super) generated_at: String,
    pub(super) source: String,
    pub(super) compare_path: String,
    pub(super) slice_path: String,
    pub(super) candidate_scored_slice_path: Option<String>,
    pub(super) baseline_release_id: String,
    pub(super) candidate_release_id: String,
    pub(super) market_scope: String,
    pub(super) scenario: FundingStressScenarioSummary,
    pub(super) coverage: FundingStressCoverageSummary,
    pub(super) dataset_key: String,
    pub(super) from_date: String,
    pub(super) to_date: String,
    pub(super) row_count: usize,
    pub(super) thresholds: FundingStressThresholdSummary,
    pub(super) dataset_evidence: FundingStressDatasetEvidence,
    pub(super) probability_evidence: FundingStressProbabilityEvidence,
    pub(super) feature_context: FundingStressFeatureContext,
    pub(super) diagnosis: FundingStressDiagnosis,
}

pub(super) fn load_latest_funding_stress_audit_summary(
    market_scope: &str,
    release_review: Option<&ReleaseReviewArtifactSummary>,
) -> Option<FundingStressAuditArtifactSummary> {
    let release_review = release_review?;
    let mut candidates = Vec::<(
        Option<DateTime<FixedOffset>>,
        FundingStressAuditArtifactSummary,
    )>::new();
    for directory in ["artifacts/research/funding-stress-audit"] {
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
            if !file_name.ends_with("-funding-stress-audit.json") {
                continue;
            }
            let Some(body) = read_json_artifact(&path) else {
                continue;
            };
            let wire = match serde_json::from_str::<FundingStressAuditArtifactWire>(&body) {
                Ok(wire) => wire,
                Err(error) => {
                    tracing::warn!(
                        path = %path.to_string_lossy(),
                        %error,
                        "failed to parse funding-stress audit artifact"
                    );
                    continue;
                }
            };
            if wire.market_scope != market_scope
                || wire.baseline_release_id != release_review.baseline_release_id
                || wire.candidate_release_id != release_review.candidate_release_id
                || wire.scenario.scenario_id != "us_funding_stress_2011"
            {
                continue;
            }
            candidates.push((
                DateTime::parse_from_rfc3339(&wire.generated_at).ok(),
                FundingStressAuditArtifactSummary {
                    generated_at: wire.generated_at,
                    source: path.to_string_lossy().into_owned(),
                    compare_path: wire.compare_path,
                    slice_path: wire.slice_path,
                    candidate_scored_slice_path: wire.candidate_scored_slice_path,
                    baseline_release_id: wire.baseline_release_id,
                    candidate_release_id: wire.candidate_release_id,
                    market_scope: wire.market_scope,
                    scenario: wire.scenario,
                    coverage: wire.coverage,
                    dataset_key: wire.dataset_key,
                    from_date: wire.from_date,
                    to_date: wire.to_date,
                    row_count: wire.row_count,
                    thresholds: wire.thresholds,
                    dataset_evidence: wire.dataset_evidence,
                    probability_evidence: wire.probability_evidence,
                    feature_context: wire.feature_context,
                    diagnosis: wire.diagnosis,
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
    use super::FundingStressAuditArtifactWire;

    #[test]
    fn funding_stress_wire_allows_missing_optional_arrays() {
        let wire: FundingStressAuditArtifactWire = serde_json::from_str(
            r#"{
              "generated_at": "2026-06-09T12:00:00Z",
              "compare_path": "compare.json",
              "slice_path": "slice.json",
              "baseline_release_id": "baseline",
              "candidate_release_id": "candidate",
              "market_scope": "financial_system",
              "scenario": {
                "scenario_id": "us_funding_stress_2011",
                "label": "2011 美欧融资压力",
                "family": "mixed_systemic_stress",
                "pre_warning_start": "2011-01-01",
                "crisis_start": "2011-07-29",
                "crisis_end": "2011-10-31",
                "training_role": "extension_only",
                "protected_window": true
              },
              "dataset_key": "formal_v1_ext_stress_1990_daily:test",
              "from_date": "2011-01-01",
              "to_date": "2011-10-31",
              "row_count": 213,
              "diagnosis": {
                "primary_class": "no_runtime_floor_signal",
                "trainability_class": "evaluation_only_window",
                "family_context_class": "mixed_systemic_proxy_missing",
                "candidate_margin_class": "candidate_margin_erosion"
              }
            }"#,
        )
        .expect("wire should decode");

        assert_eq!(wire.row_count, 213);
        assert_eq!(wire.diagnosis.primary_class, "no_runtime_floor_signal");
        assert!(wire.dataset_evidence.split_counts.is_empty());
    }
}

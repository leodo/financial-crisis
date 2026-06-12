use std::{fs, path::Path as FsPath};

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

use super::{read_json_artifact, ReleaseReviewArtifactSummary};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct LeadtimeMetricRow {
    pub(super) metric: String,
    pub(super) baseline: Option<f64>,
    pub(super) candidate: Option<f64>,
    pub(super) delta: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct LeadtimeRuntimeRow {
    pub(super) horizon_days: u32,
    pub(super) baseline_diagnosis: Option<String>,
    pub(super) candidate_diagnosis: Option<String>,
    pub(super) baseline_threshold: Option<f64>,
    pub(super) candidate_threshold: Option<f64>,
    pub(super) baseline_early_warning_regime: Option<String>,
    pub(super) candidate_early_warning_regime: Option<String>,
    pub(super) baseline_early_warning_avg_probability: Option<f64>,
    pub(super) candidate_early_warning_avg_probability: Option<f64>,
    pub(super) baseline_normal_avg_probability: Option<f64>,
    pub(super) candidate_normal_avg_probability: Option<f64>,
    pub(super) baseline_early_warning_gap_vs_normal: Option<f64>,
    pub(super) candidate_early_warning_gap_vs_normal: Option<f64>,
    pub(super) baseline_floor_gap: Option<f64>,
    pub(super) candidate_floor_gap: Option<f64>,
    pub(super) baseline_threshold_hit_rate: Option<f64>,
    pub(super) candidate_threshold_hit_rate: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct LeadtimeGapRow {
    pub(super) scenario_id: String,
    pub(super) name: String,
    pub(super) outcome: Option<String>,
    pub(super) signal_source: Option<String>,
    pub(super) baseline_lead_time_days: Option<i64>,
    pub(super) candidate_lead_time_days: Option<i64>,
    pub(super) baseline_actionable_lead_time_days: Option<i64>,
    pub(super) candidate_actionable_lead_time_days: Option<i64>,
    pub(super) actionable_delta_days: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct LeadtimeFocusRow {
    pub(super) scenario_id: String,
    pub(super) name: String,
    pub(super) outcome: Option<String>,
    pub(super) baseline_primary_failure_mode: Option<String>,
    pub(super) candidate_primary_failure_mode: Option<String>,
    pub(super) baseline_actionable_point_count: Option<u32>,
    pub(super) candidate_actionable_point_count: Option<u32>,
    pub(super) baseline_runtime_floor_hit_point_count: Option<u32>,
    pub(super) candidate_runtime_floor_hit_point_count: Option<u32>,
    pub(super) baseline_dominant_runtime_block: Option<String>,
    pub(super) baseline_dominant_runtime_block_count: Option<u32>,
    pub(super) candidate_dominant_runtime_block: Option<String>,
    pub(super) candidate_dominant_runtime_block_count: Option<u32>,
    pub(super) baseline_dominant_continuity_facet: Option<String>,
    pub(super) baseline_dominant_continuity_facet_count: Option<u32>,
    pub(super) candidate_dominant_continuity_facet: Option<String>,
    pub(super) candidate_dominant_continuity_facet_count: Option<u32>,
    pub(super) baseline_first_runtime_floor_hit_without_l3_reason: Option<String>,
    pub(super) candidate_first_runtime_floor_hit_without_l3_reason: Option<String>,
    pub(super) first_block_date: Option<String>,
    pub(super) first_baseline_block_category: Option<String>,
    pub(super) first_candidate_block_category: Option<String>,
    pub(super) first_baseline_block_reason: Option<String>,
    pub(super) first_candidate_block_reason: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct LeadtimeCountRow {
    pub(super) scenario_id: String,
    pub(super) name: String,
    pub(super) category: String,
    pub(super) baseline_count: u32,
    pub(super) candidate_count: u32,
    pub(super) delta: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct LeadtimeWorkstreamRow {
    pub(super) workstream: String,
    pub(super) scenario_count: u32,
    pub(super) protected_count: u32,
    pub(super) scenarios: Option<String>,
    pub(super) scenario_families: Option<String>,
    pub(super) training_roles: Option<String>,
    pub(super) baseline_gate_gap_profiles: Option<String>,
    pub(super) candidate_gate_gap_profiles: Option<String>,
    pub(super) baseline_gate_gap_points: Option<String>,
    pub(super) candidate_gate_gap_points: Option<String>,
    pub(super) suggested_review: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct LeadtimeAttributionRow {
    pub(super) workstream: String,
    pub(super) attribution: String,
    pub(super) scenario_count: u32,
    pub(super) protected_count: u32,
    pub(super) explanation: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct LeadtimeActionRow {
    pub(super) workstream: String,
    pub(super) attribution: String,
    pub(super) action_type: String,
    pub(super) scenario_count: u32,
    pub(super) protected_count: u32,
    pub(super) recommendation: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct LeadtimeAuditArtifactWire {
    pub(super) generated_at: String,
    pub(super) release_review_artifact: String,
    pub(super) baseline_release_id: String,
    pub(super) candidate_release_id: String,
    pub(super) market_scope: String,
    pub(super) history_mode: String,
    pub(super) reviewed_at: Option<String>,
    #[serde(default)]
    pub(super) metric_rows: Vec<LeadtimeMetricRow>,
    #[serde(default)]
    pub(super) runtime_rows: Vec<LeadtimeRuntimeRow>,
    #[serde(default)]
    pub(super) leadtime_gap_rows: Vec<LeadtimeGapRow>,
    #[serde(default)]
    pub(super) focus_rows: Vec<LeadtimeFocusRow>,
    #[serde(default)]
    pub(super) block_mix_rows: Vec<LeadtimeCountRow>,
    #[serde(default)]
    pub(super) continuity_facet_rows: Vec<LeadtimeCountRow>,
    #[serde(default)]
    pub(super) workstream_rows: Vec<LeadtimeWorkstreamRow>,
    #[serde(default)]
    pub(super) attribution_rows: Vec<LeadtimeAttributionRow>,
    #[serde(default)]
    pub(super) action_rows: Vec<LeadtimeActionRow>,
    #[serde(default)]
    pub(super) takeaways: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct LeadtimeAuditArtifactSummary {
    pub(super) generated_at: String,
    pub(super) source: String,
    pub(super) release_review_artifact: String,
    pub(super) baseline_release_id: String,
    pub(super) candidate_release_id: String,
    pub(super) market_scope: String,
    pub(super) history_mode: String,
    pub(super) reviewed_at: Option<String>,
    pub(super) metric_rows: Vec<LeadtimeMetricRow>,
    pub(super) runtime_rows: Vec<LeadtimeRuntimeRow>,
    pub(super) leadtime_gap_rows: Vec<LeadtimeGapRow>,
    pub(super) focus_rows: Vec<LeadtimeFocusRow>,
    pub(super) block_mix_rows: Vec<LeadtimeCountRow>,
    pub(super) continuity_facet_rows: Vec<LeadtimeCountRow>,
    pub(super) workstream_rows: Vec<LeadtimeWorkstreamRow>,
    pub(super) attribution_rows: Vec<LeadtimeAttributionRow>,
    pub(super) action_rows: Vec<LeadtimeActionRow>,
    pub(super) takeaways: Vec<String>,
}

pub(super) fn load_latest_leadtime_audit_summary(
    market_scope: &str,
    release_review: Option<&ReleaseReviewArtifactSummary>,
) -> Option<LeadtimeAuditArtifactSummary> {
    let release_review = release_review?;
    let mut candidates =
        Vec::<(Option<DateTime<FixedOffset>>, LeadtimeAuditArtifactSummary)>::new();
    let path = FsPath::new("artifacts/research/leadtime-audit");
    let Ok(entries) = fs::read_dir(path) else {
        return None;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !file_name.ends_with("-leadtime-audit.json") {
            continue;
        }
        let Some(body) = read_json_artifact(&path) else {
            continue;
        };
        let wire = match serde_json::from_str::<LeadtimeAuditArtifactWire>(&body) {
            Ok(wire) => wire,
            Err(error) => {
                tracing::warn!(
                    path = %path.to_string_lossy(),
                    %error,
                    "failed to parse leadtime audit artifact"
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
            LeadtimeAuditArtifactSummary {
                generated_at: wire.generated_at,
                source: path.to_string_lossy().into_owned(),
                release_review_artifact: wire.release_review_artifact,
                baseline_release_id: wire.baseline_release_id,
                candidate_release_id: wire.candidate_release_id,
                market_scope: wire.market_scope,
                history_mode: wire.history_mode,
                reviewed_at: wire.reviewed_at,
                metric_rows: wire.metric_rows,
                runtime_rows: wire.runtime_rows,
                leadtime_gap_rows: wire.leadtime_gap_rows,
                focus_rows: wire.focus_rows,
                block_mix_rows: wire.block_mix_rows,
                continuity_facet_rows: wire.continuity_facet_rows,
                workstream_rows: wire.workstream_rows,
                attribution_rows: wire.attribution_rows,
                action_rows: wire.action_rows,
                takeaways: wire.takeaways,
            },
        ));
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
    use super::LeadtimeAuditArtifactWire;

    #[test]
    fn leadtime_wire_allows_missing_optional_arrays() {
        let wire: LeadtimeAuditArtifactWire = serde_json::from_str(
            r#"{
              "generated_at": "2026-06-09T13:02:21Z",
              "release_review_artifact": "review.json",
              "baseline_release_id": "baseline",
              "candidate_release_id": "candidate",
              "market_scope": "financial_system",
              "history_mode": "default"
            }"#,
        )
        .expect("wire should decode");

        assert_eq!(wire.history_mode, "default");
        assert!(wire.takeaways.is_empty());
        assert!(wire.runtime_rows.is_empty());
    }
}

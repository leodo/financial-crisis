mod cache;
mod transform;

use chrono::{NaiveDate, Utc};
use fc_domain::{AssessmentHistoryPoint, ProbabilityDiagnostics};

#[cfg(test)]
pub(crate) use cache::should_refresh_full_release_history;
pub(crate) use cache::{
    expected_prediction_snapshot_method_version, is_formal_main_release,
    load_cached_historical_replay_output, persist_historical_replay_output,
};
pub(crate) use transform::{
    assessment_history_point_from_assessment, historical_output_from_prediction_snapshots,
    historical_output_from_replay_points, historical_replay_point_draft_from_assessment,
    merge_historical_outputs, prediction_snapshot_from_assessment,
};

pub(crate) const HISTORY_SOURCE_TRANSITIONAL_SNAPSHOT_BRIDGE: &str = "transitional_snapshot_bridge";
pub(crate) const HISTORY_SOURCE_RAW_OBSERVATION_REBUILD: &str = "raw_observation_rebuild";
pub(crate) const HISTORY_SOURCE_RAW_OBSERVATION_REPLAY: &str = "raw_observation_replay";
pub(crate) const HISTORY_SOURCE_RAW_PIT_FEATURE_REPLAY: &str = "raw_pit_feature_replay";
pub(crate) const HISTORY_SOURCE_RAW_PIT_FEATURE_REUSE: &str = "raw_pit_feature_reuse";

pub(crate) fn pit_feature_history_source(
    feature_snapshot_id: Option<&str>,
    as_of_date: NaiveDate,
    raw_source_if_none: &'static str,
) -> &'static str {
    match feature_snapshot_id {
        Some(snapshot_id) => {
            if feature_snapshot_matches_as_of_date(snapshot_id, as_of_date) {
                HISTORY_SOURCE_RAW_PIT_FEATURE_REPLAY
            } else {
                HISTORY_SOURCE_RAW_PIT_FEATURE_REUSE
            }
        }
        None => raw_source_if_none,
    }
}

fn feature_snapshot_matches_as_of_date(feature_snapshot_id: &str, as_of_date: NaiveDate) -> bool {
    feature_snapshot_id
        .split(':')
        .nth(2)
        .and_then(|raw| NaiveDate::parse_from_str(raw, "%Y-%m-%d").ok())
        .is_none_or(|snapshot_date| snapshot_date == as_of_date)
}

#[derive(Debug)]
pub(crate) struct HistoricalAssessmentOutput {
    pub(crate) history_points: Vec<AssessmentHistoryPoint>,
    pub(crate) prediction_snapshots: Vec<fc_domain::PredictionSnapshotRecord>,
    pub(crate) replay_point_drafts: Vec<HistoricalReplayPointDraft>,
}

#[derive(Debug, Clone)]
pub(crate) struct HistoricalReplayPointDraft {
    pub(crate) entity_id: String,
    pub(crate) market_scope: String,
    pub(crate) release_id: Option<String>,
    pub(crate) as_of_date: NaiveDate,
    pub(crate) feature_snapshot_id: Option<String>,
    pub(crate) feature_set_version: String,
    pub(crate) label_version: String,
    pub(crate) point_in_time_mode: String,
    pub(crate) runtime_policy_version: String,
    pub(crate) action_playbook_version: String,
    pub(crate) overall_score: f64,
    pub(crate) structural_score: f64,
    pub(crate) trigger_score: f64,
    pub(crate) external_shock_score: f64,
    pub(crate) raw_p_5d: f64,
    pub(crate) raw_p_20d: f64,
    pub(crate) raw_p_60d: f64,
    pub(crate) calibrated_p_5d: f64,
    pub(crate) calibrated_p_20d: f64,
    pub(crate) calibrated_p_60d: f64,
    pub(crate) posture: String,
    pub(crate) time_to_risk_bucket: String,
    pub(crate) actionability_prepare: f64,
    pub(crate) actionability_hedge: f64,
    pub(crate) actionability_defend: f64,
    pub(crate) probability_diagnostics: ProbabilityDiagnostics,
    pub(crate) posture_trigger_codes: Vec<String>,
    pub(crate) posture_blocker_codes: Vec<String>,
    pub(crate) coverage_score: f64,
    pub(crate) freshness_status: String,
    pub(crate) generated_at: chrono::DateTime<Utc>,
}

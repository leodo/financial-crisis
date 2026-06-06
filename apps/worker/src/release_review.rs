use chrono::NaiveDate;
use fc_domain::{ActionabilityLevel, AssessmentSnapshot, ModelReleaseRecord};
use serde::Serialize;

use crate::{reporting, RuntimeThresholdDiagnosticsWire};

mod historical;
mod runtime;

#[cfg(test)]
pub(crate) use historical::summarize_release_review_historical_audit_workstreams;
pub(crate) use historical::{
    release_review_historical_audit_takeaways, summarize_release_review_failure_modes,
    summarize_release_review_historical_audit_actions,
    summarize_release_review_historical_audit_attribution,
    summarize_release_review_historical_audit_priorities,
    summarize_release_review_historical_audit_workstreams_with_focus,
};
pub(crate) use runtime::{
    build_release_runtime_review_diagnostics, lift_vs_baseline,
    release_review_runtime_separation_takeaways,
};
#[cfg(test)]
pub(crate) use runtime::{
    classify_regime_separation, summarize_release_runtime_regime_probabilities,
    summarize_release_runtime_regime_separation,
};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseReviewScalarMetric {
    pub(crate) baseline: f64,
    pub(crate) candidate: f64,
    pub(crate) delta: f64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseReviewCountMetric {
    pub(crate) baseline: u32,
    pub(crate) candidate: u32,
    pub(crate) delta: i64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseReviewBacktestScenarioComparison {
    pub(crate) scenario_id: String,
    pub(crate) name: String,
    pub(crate) signal_source: String,
    pub(crate) crisis_start: NaiveDate,
    pub(crate) crisis_end: NaiveDate,
    pub(crate) baseline_first_l2_date: Option<NaiveDate>,
    pub(crate) candidate_first_l2_date: Option<NaiveDate>,
    pub(crate) baseline_first_l3_date: Option<NaiveDate>,
    pub(crate) candidate_first_l3_date: Option<NaiveDate>,
    pub(crate) baseline_lead_time_days: Option<i64>,
    pub(crate) candidate_lead_time_days: Option<i64>,
    pub(crate) baseline_actionable_lead_time_days: Option<i64>,
    pub(crate) candidate_actionable_lead_time_days: Option<i64>,
    pub(crate) baseline_false_positive_count: u32,
    pub(crate) candidate_false_positive_count: u32,
    pub(crate) actionable_delta_days: Option<i64>,
    pub(crate) outcome: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseReviewScenarioPointComparison {
    pub(crate) as_of_date: NaiveDate,
    pub(crate) baseline_p20d: Option<f64>,
    pub(crate) candidate_p20d: Option<f64>,
    pub(crate) baseline_p60d: Option<f64>,
    pub(crate) candidate_p60d: Option<f64>,
    pub(crate) baseline_posture: Option<String>,
    pub(crate) candidate_posture: Option<String>,
    pub(crate) baseline_time_bucket: Option<String>,
    pub(crate) candidate_time_bucket: Option<String>,
    pub(crate) baseline_strict_review_actionable: bool,
    pub(crate) candidate_strict_review_actionable: bool,
    pub(crate) baseline_runtime_floor_hit: bool,
    pub(crate) candidate_runtime_floor_hit: bool,
    pub(crate) baseline_actionable: bool,
    pub(crate) candidate_actionable: bool,
    pub(crate) baseline_actionable_forward_5d_hits: Option<u32>,
    pub(crate) candidate_actionable_forward_5d_hits: Option<u32>,
    pub(crate) baseline_actionable_sustained: Option<bool>,
    pub(crate) candidate_actionable_sustained: Option<bool>,
    pub(crate) baseline_trigger_codes: Vec<String>,
    pub(crate) candidate_trigger_codes: Vec<String>,
    pub(crate) baseline_runtime_actionable_block_category: Option<String>,
    pub(crate) candidate_runtime_actionable_block_category: Option<String>,
    pub(crate) baseline_runtime_actionable_block_reason: Option<String>,
    pub(crate) candidate_runtime_actionable_block_reason: Option<String>,
    pub(crate) baseline_actionable_diagnostic: Option<String>,
    pub(crate) candidate_actionable_diagnostic: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseReviewRuntimeBlockCount {
    pub(crate) category: String,
    pub(crate) baseline_count: u32,
    pub(crate) candidate_count: u32,
    pub(crate) delta: i64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseReviewRuntimeDominantCategories {
    pub(crate) baseline_categories: Vec<String>,
    pub(crate) baseline_count: u32,
    pub(crate) candidate_categories: Vec<String>,
    pub(crate) candidate_count: u32,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseReviewScenarioFocusDiagnostic {
    pub(crate) scenario_id: String,
    pub(crate) name: String,
    pub(crate) outcome: String,
    pub(crate) window_start: NaiveDate,
    pub(crate) window_end: NaiveDate,
    pub(crate) crisis_start: NaiveDate,
    pub(crate) crisis_end: NaiveDate,
    pub(crate) baseline_first_l2_date: Option<NaiveDate>,
    pub(crate) candidate_first_l2_date: Option<NaiveDate>,
    pub(crate) baseline_first_l3_date: Option<NaiveDate>,
    pub(crate) candidate_first_l3_date: Option<NaiveDate>,
    pub(crate) baseline_first_non_normal_date: Option<NaiveDate>,
    pub(crate) candidate_first_non_normal_date: Option<NaiveDate>,
    pub(crate) baseline_actionable_point_count: u32,
    pub(crate) candidate_actionable_point_count: u32,
    pub(crate) baseline_runtime_floor_hit_point_count: u32,
    pub(crate) candidate_runtime_floor_hit_point_count: u32,
    pub(crate) baseline_max_p20d: Option<f64>,
    pub(crate) candidate_max_p20d: Option<f64>,
    pub(crate) baseline_max_p60d: Option<f64>,
    pub(crate) candidate_max_p60d: Option<f64>,
    pub(crate) baseline_first_runtime_floor_hit_without_l3_date: Option<NaiveDate>,
    pub(crate) candidate_first_runtime_floor_hit_without_l3_date: Option<NaiveDate>,
    pub(crate) baseline_first_runtime_floor_hit_without_l3_reason: Option<String>,
    pub(crate) candidate_first_runtime_floor_hit_without_l3_reason: Option<String>,
    pub(crate) baseline_primary_failure_mode: Option<String>,
    pub(crate) candidate_primary_failure_mode: Option<String>,
    pub(crate) dominant_runtime_blocks: ReleaseReviewRuntimeDominantCategories,
    pub(crate) dominant_runtime_continuity_facets: ReleaseReviewRuntimeDominantCategories,
    pub(crate) runtime_block_counts: Vec<ReleaseReviewRuntimeBlockCount>,
    pub(crate) runtime_continuity_facet_counts: Vec<ReleaseReviewRuntimeBlockCount>,
    pub(crate) interesting_points: Vec<ReleaseReviewScenarioPointComparison>,
}

#[derive(Debug, Clone)]
pub(crate) struct ReleaseReviewFailureModeSummary {
    pub(crate) failure_mode: String,
    pub(crate) baseline_count: u32,
    pub(crate) candidate_count: u32,
    pub(crate) baseline_scenarios: Vec<String>,
    pub(crate) candidate_scenarios: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseReviewHistoricalAuditPriority {
    pub(crate) scenario_id: String,
    pub(crate) scenario_name: String,
    pub(crate) outcome: String,
    pub(crate) scenario_family: String,
    pub(crate) training_role: String,
    pub(crate) protected_window: bool,
    pub(crate) baseline_failure_mode: String,
    pub(crate) candidate_failure_mode: String,
    pub(crate) baseline_actionable_point_count: u32,
    pub(crate) candidate_actionable_point_count: u32,
    pub(crate) baseline_runtime_floor_hit_point_count: u32,
    pub(crate) candidate_runtime_floor_hit_point_count: u32,
    pub(crate) baseline_gate_gap_profile: Option<String>,
    pub(crate) candidate_gate_gap_profile: Option<String>,
    pub(crate) primary_workstream: String,
    pub(crate) suggested_review: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseReviewHistoricalAuditWorkstreamSummary {
    pub(crate) workstream: String,
    pub(crate) scenario_count: u32,
    pub(crate) protected_count: u32,
    pub(crate) scenarios: Vec<String>,
    pub(crate) scenario_families: Vec<String>,
    pub(crate) training_roles: Vec<String>,
    pub(crate) baseline_gate_gap_profiles: Vec<String>,
    pub(crate) candidate_gate_gap_profiles: Vec<String>,
    pub(crate) gate_gap_point_counts: Vec<ReleaseReviewRuntimeBlockCount>,
    pub(crate) suggested_review: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseReviewHistoricalAuditAttributionSummary {
    pub(crate) workstream: String,
    pub(crate) attribution: String,
    pub(crate) scenario_count: u32,
    pub(crate) protected_count: u32,
    pub(crate) baseline_count: u32,
    pub(crate) candidate_count: u32,
    pub(crate) baseline_scenarios: Vec<String>,
    pub(crate) candidate_scenarios: Vec<String>,
    pub(crate) explanation: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseReviewHistoricalAuditActionSummary {
    pub(crate) workstream: String,
    pub(crate) attribution: String,
    pub(crate) action_type: String,
    pub(crate) scenario_count: u32,
    pub(crate) protected_count: u32,
    pub(crate) recommendation: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseReviewComparisonSummary {
    pub(crate) timely_warning_rate: ReleaseReviewScalarMetric,
    pub(crate) strict_actionable_point_count: ReleaseReviewCountMetric,
    pub(crate) runtime_floor_hit_count: ReleaseReviewCountMetric,
    pub(crate) actionable_precision: ReleaseReviewScalarMetric,
    pub(crate) longest_false_positive_episode_days: ReleaseReviewCountMetric,
    pub(crate) current_p_5d: ReleaseReviewScalarMetric,
    pub(crate) current_p_20d: ReleaseReviewScalarMetric,
    pub(crate) current_p_60d: ReleaseReviewScalarMetric,
    pub(crate) runtime_separation_summary: Vec<ReleaseReviewRuntimeSeparationComparison>,
    pub(crate) backtest_scenarios: Vec<ReleaseReviewBacktestScenarioComparison>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseReviewRuntimeSeparationComparison {
    pub(crate) horizon_days: u32,
    pub(crate) baseline_diagnosis: String,
    pub(crate) candidate_diagnosis: String,
    pub(crate) baseline_threshold: Option<f64>,
    pub(crate) candidate_threshold: Option<f64>,
    pub(crate) baseline_early_warning_regime: String,
    pub(crate) candidate_early_warning_regime: String,
    pub(crate) baseline_early_warning_avg_probability: Option<f64>,
    pub(crate) candidate_early_warning_avg_probability: Option<f64>,
    pub(crate) baseline_normal_avg_probability: Option<f64>,
    pub(crate) candidate_normal_avg_probability: Option<f64>,
    pub(crate) baseline_early_warning_gap_vs_normal: Option<f64>,
    pub(crate) candidate_early_warning_gap_vs_normal: Option<f64>,
    pub(crate) baseline_floor_gap: Option<f64>,
    pub(crate) candidate_floor_gap: Option<f64>,
    pub(crate) baseline_early_warning_lift_vs_normal: Option<f64>,
    pub(crate) candidate_early_warning_lift_vs_normal: Option<f64>,
    pub(crate) baseline_threshold_hit_rate: Option<f64>,
    pub(crate) candidate_threshold_hit_rate: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseActionabilityLevelReview {
    pub(crate) level: ActionabilityLevel,
    pub(crate) proxy_horizon_days: u32,
    pub(crate) sample_count: u32,
    pub(crate) positive_rate: f64,
    pub(crate) threshold: f64,
    pub(crate) predicted_positive_count: u32,
    pub(crate) primary_positive_count: u32,
    pub(crate) late_validation_row_count: u32,
    pub(crate) protected_row_count: u32,
    pub(crate) primary_hit_count: u32,
    pub(crate) late_validation_hit_count: u32,
    pub(crate) protected_hit_count: u32,
    pub(crate) false_positive_count: u32,
    pub(crate) scenario_count: u32,
    pub(crate) on_time_scenario_count: u32,
    pub(crate) late_only_scenario_count: u32,
    pub(crate) missed_scenario_count: u32,
    pub(crate) precision_at_threshold: Option<f64>,
    pub(crate) primary_recall_at_threshold: Option<f64>,
    pub(crate) late_validation_capture_rate: Option<f64>,
    pub(crate) on_time_rate: Option<f64>,
    pub(crate) late_only_rate: Option<f64>,
    pub(crate) missed_rate: Option<f64>,
    pub(crate) note: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseActionabilityReview {
    pub(crate) release_id: String,
    pub(crate) enabled: bool,
    pub(crate) model_version: Option<String>,
    pub(crate) calibration_version: Option<String>,
    pub(crate) fusion_policy_version: Option<String>,
    pub(crate) levels: Vec<ReleaseActionabilityLevelReview>,
    pub(crate) guard_regressions: Vec<String>,
    pub(crate) guard_passed: bool,
    pub(crate) note: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseRuntimeCount {
    pub(crate) name: String,
    pub(crate) count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseRuntimeReviewDiagnostics {
    pub(crate) release_id: String,
    pub(crate) history_point_count: usize,
    pub(crate) posture_distribution: Vec<ReleaseRuntimeCount>,
    pub(crate) time_bucket_distribution: Vec<ReleaseRuntimeCount>,
    pub(crate) posture_trigger_distribution: Vec<ReleaseRuntimeClauseCount>,
    pub(crate) posture_blocker_distribution: Vec<ReleaseRuntimeClauseCount>,
    pub(crate) regime_probability_summaries: Vec<ReleaseRuntimeRegimeProbabilitySummary>,
    pub(crate) regime_separation_summaries: Vec<ReleaseRuntimeSeparationSummary>,
    pub(crate) runtime_thresholds: Option<RuntimeThresholdDiagnosticsWire>,
    pub(crate) points_at_or_above_prepare_p60d: Option<usize>,
    pub(crate) points_at_or_above_hedge_p20d: Option<usize>,
    pub(crate) points_at_or_above_defend_p5d: Option<usize>,
    pub(crate) note: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseRuntimeClauseCount {
    pub(crate) posture: String,
    pub(crate) clause: String,
    pub(crate) count: usize,
    pub(crate) share_of_posture: f64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseRuntimeRegimeProbabilitySummary {
    pub(crate) horizon_days: u32,
    pub(crate) regime: String,
    pub(crate) row_count: usize,
    pub(crate) row_rate: f64,
    pub(crate) avg_raw_probability: f64,
    pub(crate) max_raw_probability: f64,
    pub(crate) avg_probability: f64,
    pub(crate) max_probability: f64,
    pub(crate) raw_lift_vs_normal: Option<f64>,
    pub(crate) calibrated_lift_vs_normal: Option<f64>,
    pub(crate) raw_gap_vs_normal: Option<f64>,
    pub(crate) calibrated_gap_vs_normal: Option<f64>,
    pub(crate) calibration_gap_retention: Option<f64>,
    pub(crate) threshold_hit_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseRuntimeSeparationSummary {
    pub(crate) horizon_days: u32,
    pub(crate) early_warning_regime: String,
    pub(crate) normal_avg_probability: f64,
    pub(crate) pre_warning_buffer_avg_probability: f64,
    pub(crate) positive_window_avg_probability: f64,
    pub(crate) in_crisis_avg_probability: f64,
    pub(crate) post_crisis_cooldown_avg_probability: f64,
    pub(crate) early_warning_raw_lift_vs_normal: Option<f64>,
    pub(crate) early_warning_calibrated_lift_vs_normal: Option<f64>,
    pub(crate) early_warning_gap_retention: Option<f64>,
    pub(crate) positive_window_calibrated_lift_vs_normal: Option<f64>,
    pub(crate) positive_window_gap_vs_normal: Option<f64>,
    pub(crate) in_crisis_raw_lift_vs_normal: Option<f64>,
    pub(crate) in_crisis_calibrated_lift_vs_normal: Option<f64>,
    pub(crate) post_crisis_cooldown_calibrated_lift_vs_normal: Option<f64>,
    pub(crate) post_crisis_cooldown_gap_vs_normal: Option<f64>,
    pub(crate) max_non_normal_calibrated_lift_vs_normal: Option<f64>,
    pub(crate) max_non_normal_threshold_hit_rate: Option<f64>,
    pub(crate) diagnosis: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseReviewEnvelope {
    pub(crate) reviewed_at: String,
    pub(crate) market_scope: String,
    pub(crate) api_reload_url: String,
    pub(crate) history_mode: String,
    pub(crate) history_limit: usize,
    pub(crate) original_active_release_id: String,
    pub(crate) restored_release_id: String,
    pub(crate) baseline_release: ModelReleaseRecord,
    pub(crate) candidate_release: ModelReleaseRecord,
    pub(crate) baseline_assessment: AssessmentSnapshot,
    pub(crate) candidate_assessment: AssessmentSnapshot,
    pub(crate) baseline_runtime_review: ReleaseRuntimeReviewDiagnostics,
    pub(crate) candidate_runtime_review: ReleaseRuntimeReviewDiagnostics,
    pub(crate) baseline_actionability_review: ReleaseActionabilityReview,
    pub(crate) candidate_actionability_review: ReleaseActionabilityReview,
    pub(crate) scenario_focus: Vec<ReleaseReviewScenarioFocusDiagnostic>,
    pub(crate) historical_audit_workstreams: Vec<ReleaseReviewHistoricalAuditWorkstreamSummary>,
    pub(crate) historical_audit_priorities: Vec<ReleaseReviewHistoricalAuditPriority>,
    pub(crate) historical_audit_attribution: Vec<ReleaseReviewHistoricalAuditAttributionSummary>,
    pub(crate) historical_audit_actions: Vec<ReleaseReviewHistoricalAuditActionSummary>,
    pub(crate) comparison: ReleaseReviewComparisonSummary,
    pub(crate) probability_guard_regressions: Vec<String>,
    pub(crate) probability_guard_passed: bool,
    pub(crate) operational_guard_regressions: Vec<String>,
    pub(crate) operational_guard_passed: bool,
    pub(crate) actionability_guard_regressions: Vec<String>,
    pub(crate) actionability_guard_passed: bool,
    pub(crate) runtime_sanity_regressions: Vec<String>,
    pub(crate) runtime_sanity_passed: bool,
    pub(crate) overall_guard_regressions: Vec<String>,
    pub(crate) overall_guard_passed: bool,
    pub(crate) recommendation: String,
}

pub(crate) fn format_runtime_category_list(categories: &[String]) -> String {
    if categories.is_empty() {
        "—".to_string()
    } else {
        categories.join(", ")
    }
}

pub(crate) fn render_release_review_markdown(report: &ReleaseReviewEnvelope) -> String {
    reporting::render_release_review_markdown_impl(report)
}

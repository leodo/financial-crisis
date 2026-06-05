use chrono::NaiveDate;
use serde::Serialize;

mod build;
mod render;

pub(super) use build::build_release_formal_probability_compare_export;
pub(super) use render::{
    print_release_formal_probability_compare_summary,
    write_release_formal_probability_compare_report,
};

#[derive(Debug, Clone, Serialize)]
pub(super) struct ReleaseFormalProbabilityCompareExport {
    exported_at: String,
    market_scope: String,
    baseline_release_id: String,
    candidate_release_id: String,
    dataset_key: String,
    scenario_id: Option<String>,
    from_date: NaiveDate,
    to_date: NaiveDate,
    row_count: usize,
    baseline_thresholds: Vec<ReleaseFormalProbabilityThresholdSummary>,
    candidate_thresholds: Vec<ReleaseFormalProbabilityThresholdSummary>,
    summary: ReleaseFormalProbabilityCompareSummary,
    rows: Vec<ReleaseFormalProbabilityComparePoint>,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseFormalProbabilityThresholdSummary {
    horizon_days: u32,
    decision_threshold: Option<f64>,
    overlay_count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseFormalProbabilityCompareSummary {
    baseline_hit_count_20d: usize,
    candidate_hit_count_20d: usize,
    baseline_hit_count_60d: usize,
    candidate_hit_count_60d: usize,
    baseline_max_p_20d: f64,
    baseline_max_p_20d_date: Option<NaiveDate>,
    candidate_max_p_20d: f64,
    candidate_max_p_20d_date: Option<NaiveDate>,
    baseline_max_p_60d: f64,
    baseline_max_p_60d_date: Option<NaiveDate>,
    candidate_max_p_60d: f64,
    candidate_max_p_60d_date: Option<NaiveDate>,
    overall_window: ReleaseFormalProbabilityWindowAggregateSummary,
    hedge_window: ReleaseFormalProbabilityWindowAggregateSummary,
    positive_window_20d: ReleaseFormalProbabilityWindowAggregateSummary,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseFormalProbabilityWindowAggregateSummary {
    row_count: usize,
    avg_delta_p_20d: f64,
    avg_abs_delta_p_20d: f64,
    avg_delta_p_60d: f64,
    avg_abs_delta_p_60d: f64,
    baseline_hit_rate_20d: f64,
    candidate_hit_rate_20d: f64,
    baseline_hit_rate_60d: f64,
    candidate_hit_rate_60d: f64,
    top_feature_deltas_20d: Vec<ReleaseFormalProbabilityFeatureDeltaAggregate>,
    top_feature_deltas_60d: Vec<ReleaseFormalProbabilityFeatureDeltaAggregate>,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseFormalProbabilityFeatureDeltaAggregate {
    name: String,
    sum_delta_contribution: f64,
    abs_sum_delta_contribution: f64,
    mean_delta_contribution: f64,
    count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseFormalProbabilityComparePoint {
    as_of_date: NaiveDate,
    split_name: String,
    primary_scenario_id: Option<String>,
    scenario_family: Option<String>,
    regime_20d: String,
    regime_60d: String,
    prepare_episode_label: u8,
    hedge_episode_label: u8,
    defend_episode_label: u8,
    primary_action_level: Option<String>,
    coverage_score: f64,
    baseline_raw_p_20d: f64,
    candidate_raw_p_20d: f64,
    baseline_base_linear_20d: f64,
    candidate_base_linear_20d: f64,
    baseline_final_p_20d: f64,
    candidate_final_p_20d: f64,
    delta_final_p_20d: f64,
    baseline_hit_20d: bool,
    candidate_hit_20d: bool,
    baseline_raw_p_60d: f64,
    candidate_raw_p_60d: f64,
    baseline_base_linear_60d: f64,
    candidate_base_linear_60d: f64,
    baseline_final_p_60d: f64,
    candidate_final_p_60d: f64,
    delta_final_p_60d: f64,
    baseline_hit_60d: bool,
    candidate_hit_60d: bool,
    top_feature_deltas_20d: Vec<ReleaseFormalProbabilityFeatureDelta>,
    top_feature_deltas_60d: Vec<ReleaseFormalProbabilityFeatureDelta>,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseFormalProbabilityFeatureDelta {
    name: String,
    baseline_raw_value: f64,
    candidate_raw_value: f64,
    baseline_normalized_value: f64,
    candidate_normalized_value: f64,
    baseline_weight: f64,
    candidate_weight: f64,
    baseline_contribution: f64,
    candidate_contribution: f64,
    delta_contribution: f64,
}

pub(super) struct ReleaseFormalProbabilityCompareBuildInput<'a> {
    pub(super) market_scope: &'a str,
    pub(super) dataset_key: &'a str,
    pub(super) scenario_id: Option<String>,
    pub(super) from_date: NaiveDate,
    pub(super) to_date: NaiveDate,
    pub(super) baseline_release_id: &'a str,
    pub(super) candidate_release_id: &'a str,
    pub(super) baseline_bundle: &'a fc_domain::ProbabilityBundle,
    pub(super) candidate_bundle: &'a fc_domain::ProbabilityBundle,
    pub(super) baseline_rows: Vec<super::formal::ReleaseFormalProbabilitySlicePoint>,
    pub(super) candidate_rows: Vec<super::formal::ReleaseFormalProbabilitySlicePoint>,
}

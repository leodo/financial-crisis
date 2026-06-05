mod actionability;
mod commands;
mod formal;
mod model;
mod output_paths;
mod probability;
mod release_review;
mod reporting;
mod scenario;
mod support;
mod training;

pub(crate) use actionability::{
    actionability_bundle_quality_regressions, actionability_guardrail_policy,
    actionability_prediction_count_ceiling_from_actual_positive_count, percentage_score,
    train_actionability_bundle,
};
#[cfg(test)]
pub(crate) use actionability::{
    evaluate_actionability_summary, select_actionability_calibration_strategy,
    select_actionability_decision_threshold,
};
pub(crate) use commands::{
    feature_quality_grade, has_extension_acute_core_features, has_main_dataset_core_features,
    load_formal_dataset_scenario_sets, load_label_set_crisis_scenarios,
    render_formal_dataset_summary_markdown, scenario_count_for_split_range, AuditExportEnvelope,
    AuditMethodResponseWire, FormalDatasetSummaryEnvelope, RuntimeThresholdDiagnosticsWire,
    ScenarioRowRange,
};
#[cfg(test)]
pub(crate) use commands::{
    formal_dataset_min_date, formal_dataset_snapshot_is_usable, formal_dataset_split_requirements,
    observation_is_visible_for_date, scenario_aware_formal_split_bounds,
    scenario_count_for_index_range, AuditExportOptions, FormalSplitLabelSupport,
};
#[cfg(test)]
use commands::{FeatureSnapshotBuildOptions, PointInTimeMode};
#[cfg(test)]
use commands::{
    FormalDatasetBuildOptions, FormalDatasetSummaryOptions, PipelineDatasetSource,
    PipelineTrainOptions, PredictionSnapshotQueryOptions, ProbabilityModelShape,
    RefreshLatestOptions,
};
pub(crate) use fc_domain::load_crisis_scenario_catalog;
#[cfg(test)]
pub(crate) use fc_domain::ActionabilityLevel;
use fc_domain::{
    resolve_probability_feature_value, HorizonEvaluationSummary, LogisticProbabilityModel,
    PlattCalibrationArtifact, ProbabilityCoefficient, ProbabilityFeatureStat,
    FEATURE_BUCKET_MONTHS_OR_HIGHER, FEATURE_BUCKET_NOW, FEATURE_BUCKET_WEEKS_OR_HIGHER,
    FEATURE_COVERAGE_SCORE, FEATURE_EXTERNAL_SHOCK_SCORE, FEATURE_FRESHNESS_DELAYED_OR_WORSE,
    FEATURE_FRESHNESS_STALE_OR_MISSING, FEATURE_HEURISTIC_P_20D, FEATURE_HEURISTIC_P_5D,
    FEATURE_HEURISTIC_P_60D, FEATURE_OVERALL_SCORE, FORMAL_PROBABILITY_BUNDLE_FEATURES,
    PROBABILITY_FEATURE_TRANSFORM_FAMILY_CONDITIONAL_V1,
    PROBABILITY_FEATURE_TRANSFORM_FAMILY_HYBRID_V1, PROBABILITY_FEATURE_TRANSFORM_IDENTITY_V1,
    PROBABILITY_FEATURE_TRANSFORM_INTERACTION_TAIL_V1,
    PROBABILITY_MODEL_FAMILY_FAMILY_CONDITIONAL_V1, PROBABILITY_MODEL_FAMILY_FAMILY_HYBRID_V1,
    PROBABILITY_MODEL_FAMILY_INTERACTION_TAIL_V1, PROBABILITY_MODEL_FAMILY_LINEAR_V1,
    TRANSITIONAL_PROBABILITY_BUNDLE_FEATURES,
};
pub(crate) use formal::derive_scenario_label_snapshot;
#[cfg(test)]
pub(crate) use model::{
    apply_forward_crisis_coefficient_bound_gradient, apply_forward_crisis_sign_gradient,
    apply_regime_pairwise_gradient, build_feature_stat, forward_crisis_positive_sample_weight,
    forward_crisis_regime_pairwise_targets, forward_crisis_regime_sample_weight,
    negative_sample_weight, positive_sample_action_weight, probability_training_target_label,
    project_forward_crisis_sign_constraints,
};
pub(crate) use model::{
    evaluate_probabilities, fit_logistic_model, fit_platt_calibration,
    score_logistic_model_for_dataset,
};
use output_paths::{
    DEFAULT_FORMAL_DATASET_SLICE_OUTPUT_DIR, DEFAULT_FORMAL_DATASET_SUMMARY_OUTPUT_DIR,
    DEFAULT_FORMAL_PROBABILITY_COMPARE_OUTPUT_DIR, DEFAULT_PIPELINE_BUNDLE_OUTPUT_DIR,
    DEFAULT_PIPELINE_MANIFEST_OUTPUT_DIR, DEFAULT_RELEASE_PROBABILITY_SLICE_OUTPUT_DIR,
    DEFAULT_RELEASE_REVIEW_OUTPUT_DIR,
};
#[cfg(test)]
pub(crate) use probability::{
    adjust_probability_decision_threshold_for_regime_support,
    build_probability_threshold_diagnostics, classify_probability_regime_separation,
    probability_calibration_selection_rows, probability_decision_threshold_selection,
    select_probability_calibration_strategy, select_probability_decision_threshold,
    ProbabilityCalibrationSelection, ProbabilityThresholdDiagnosticsInput,
    ProbabilityThresholdSelection,
};
pub(crate) use probability::{
    regime_positive_window_gap_floor, summarize_bundle_evaluation, train_horizon_bundle,
};
pub(crate) use release_review::{
    build_release_runtime_review_diagnostics, format_runtime_category_list, lift_vs_baseline,
    release_review_historical_audit_takeaways, release_review_runtime_separation_takeaways,
    render_release_review_markdown, summarize_release_review_failure_modes,
    summarize_release_review_historical_audit_actions,
    summarize_release_review_historical_audit_attribution,
    summarize_release_review_historical_audit_priorities,
    summarize_release_review_historical_audit_workstreams, ReleaseActionabilityLevelReview,
    ReleaseActionabilityReview, ReleaseReviewBacktestScenarioComparison,
    ReleaseReviewComparisonSummary, ReleaseReviewCountMetric, ReleaseReviewEnvelope,
    ReleaseReviewHistoricalAuditActionSummary, ReleaseReviewRuntimeBlockCount,
    ReleaseReviewRuntimeDominantCategories, ReleaseReviewRuntimeSeparationComparison,
    ReleaseReviewScalarMetric, ReleaseReviewScenarioFocusDiagnostic,
    ReleaseReviewScenarioPointComparison, ReleaseRuntimeReviewDiagnostics,
    ReleaseRuntimeSeparationSummary,
};
#[cfg(test)]
pub(crate) use release_review::{
    classify_regime_separation, summarize_release_runtime_regime_probabilities,
    summarize_release_runtime_regime_separation, ReleaseReviewHistoricalAuditAttributionSummary,
    ReleaseReviewHistoricalAuditPriority,
};
pub(crate) use reporting::write_formal_dataset_summary_report;
pub(crate) use scenario::{
    action_episode_label_for_level, action_window_label, action_window_start_date,
    actionability_level_for_proxy_horizon, dominant_action_episode_for_date, label_anchor_date,
    primary_scenario_for_date, protected_context_phase_for_date, scenario_supports_horizon,
    ActionEpisodePhase, CrisisScenario,
};
pub(crate) use support::{
    actionability_level_text, backtest_signal_source_text, data_mode_text, fetch_api_json,
    fetch_assessment_snapshot_for_guard, formal_dataset_key, format_bool_flag,
    format_optional_bool_flag, format_optional_count, format_optional_date,
    format_optional_date_with_lead, format_optional_date_with_reason, format_optional_days,
    format_optional_multiplier, format_optional_pct, format_optional_ratio, format_pct,
    format_signed_count_delta, format_signed_pct_delta, format_trigger_codes, open_sqlite_store,
    parse_date_arg, parse_positive_i64, path_to_string, posture_text, raw_data_dir,
    raw_file_extension, read_probability_bundle, read_release_manifest, reload_api_runtime,
    reload_api_runtime_with_history_options, round3, round6, run_demo_ingestion, safe_divide,
    safe_ratio, simple_hash, sqlite_path, time_bucket_text, truncate_text, write_raw_payload,
    ApiReloadHistoryMode,
};
pub(crate) use training::{
    chronological_split, chronological_split_bounds, ensure_positive_labels, forward_crisis_label,
    forward_crisis_training_regime, forward_crisis_training_regime_with_context,
    probability_training_regime_name, train_probability_pipeline, validate_split_bounds,
    ProbabilityTargetLabelMode, ProbabilityTrainingInput, ProbabilityTrainingRegime,
    ProbabilityTrainingRow,
};

pub(crate) const DEFAULT_SQLITE_PATH: &str = "data/fc-local.sqlite";
pub(crate) const DEFAULT_RAW_DATA_DIR: &str = "data/raw";
pub(crate) const DEFAULT_API_RELOAD_URL: &str = "http://127.0.0.1:18080/api/system/reload";
pub(crate) const DEFAULT_AUDIT_API_BASE_URL: &str = "http://127.0.0.1:18080";
pub(crate) const DEFAULT_AUDIT_OUTPUT_DIR: &str = "reports/rolling-audit";
pub(crate) const DEFAULT_FORMAL_FEATURE_SET_VERSION: &str =
    "feature_formal_v1_main_20260606_gatefix";
pub(crate) const DEFAULT_FORMAL_DATASET_ID: &str = "formal_v1_main_1990_daily";
pub(crate) const DEFAULT_FORMAL_LABEL_VERSION: &str = "formal_label_v1_main";
pub(crate) const DEFAULT_FORMAL_SCENARIO_SET_VERSION: &str = "scenario_v1_main";
pub(crate) const DEFAULT_FORMAL_MAIN_CONTEXT_WINDOW_SET_ID: &str = "protected_stress_windows_v1";
pub(crate) const FEATURE_SNAPSHOT_STATUS_READY: &str = "ready";
pub(crate) const FEATURE_SNAPSHOT_STATUS_COVERAGE_OR_VISIBILITY_FAILED: &str =
    "coverage_or_visibility_failed";

pub async fn run_from_args(args: Vec<String>) -> anyhow::Result<()> {
    commands::run_from_args(args).await
}

#[cfg(test)]
mod tests;

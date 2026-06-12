use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use chrono::{NaiveDate, TimeZone, Utc};
use fc_domain::{
    ActionEpisodeTemplateId, ActionabilityBundle, ActionabilityEvaluationSummary,
    ActionabilityLevelBundle, DecisionPosture, FeatureSnapshotRecord, FormalDatasetRowRecord,
    Frequency, HorizonEvaluationSummary, LogisticProbabilityModel, PlattCalibrationArtifact,
    ProbabilityBundle, ProbabilityBundleEvaluation, RegimeSeparationEvaluationSummary,
    TimeToRiskBucket, PROBABILITY_FEATURE_TRANSFORM_IDENTITY_V1,
    PROBABILITY_MODEL_FAMILY_LINEAR_V1,
};

mod fixtures;

use super::commands::release::{
    build_release_review_backtest_scenario_comparisons,
    build_release_review_runtime_separation_comparisons,
    build_release_review_scenario_focus_diagnostics, compare_actionability_guardrails,
    compare_probability_guardrails, release_review_structured_signal_counts,
    ReleaseFormalProbabilityCompareOptions, ReleaseFormalProbabilitySliceOptions,
    ReleaseProbabilitySliceOptions, ReleasePublishOptions, ReleaseReviewOptions,
    ReleaseSwitchOptions,
};
use super::commands::{
    render_formal_dataset_slice_csv, sanitize_filename_component, FormalDatasetSliceOptions,
};
use super::{
    action_window_label, actionability_bundle_quality_regressions,
    adjust_probability_decision_threshold_for_regime_support,
    build_probability_threshold_diagnostics, classify_probability_regime_separation,
    classify_regime_separation, evaluate_actionability_summary, fit_platt_calibration,
    formal_dataset_split_requirements, forward_crisis_label,
    forward_crisis_regime_pairwise_targets, forward_crisis_regime_sample_weight,
    forward_crisis_training_regime, forward_crisis_training_regime_with_context,
    negative_sample_weight, observation_is_visible_for_date, positive_sample_action_weight,
    probability_calibration_selection_rows, probability_decision_threshold_selection,
    release_review_historical_audit_takeaways, release_review_runtime_separation_takeaways, round3,
    scenario_aware_formal_split_bounds, scenario_count_for_index_range,
    select_actionability_calibration_strategy, select_actionability_decision_threshold,
    select_probability_calibration_strategy, select_probability_decision_threshold,
    summarize_release_review_failure_modes, summarize_release_review_historical_audit_actions,
    summarize_release_review_historical_audit_attribution,
    summarize_release_review_historical_audit_priorities,
    summarize_release_review_historical_audit_workstreams,
    summarize_release_review_historical_audit_workstreams_with_focus,
    summarize_release_runtime_regime_probabilities, summarize_release_runtime_regime_separation,
    ActionabilityLevel, AuditExportOptions, CrisisScenario, FeatureSnapshotBuildOptions,
    FormalDatasetBuildOptions, FormalDatasetSummaryOptions, FormalSplitLabelSupport,
    PipelineDatasetSource, PipelineReleaseManifestMode, PipelineTrainOptions, PointInTimeMode,
    PredictionSnapshotQueryOptions, ProbabilityCalibrationSelection,
    ProbabilityCalibrationStrategyInput, ProbabilityModelShape, ProbabilityTargetLabelMode,
    ProbabilityThresholdDiagnosticsInput, ProbabilityThresholdSelection, ProbabilityTrainingRegime,
    ProbabilityTrainingRow, RefreshLatestOptions, ReleaseActionabilityLevelReview,
    ReleaseActionabilityReview, ReleaseReviewHistoricalAuditAttributionSummary,
    ReleaseReviewHistoricalAuditPriority, ReleaseReviewHistoricalAuditWorkstreamSummary,
    ReleaseReviewRuntimeBlockCount, ReleaseReviewRuntimeDominantCategories,
    ReleaseReviewRuntimeSeparationComparison, ReleaseReviewScenarioFocusDiagnostic,
    ReleaseRuntimeReviewDiagnostics, ReleaseRuntimeSeparationSummary,
    RuntimeThresholdDiagnosticsWire, ScenarioRowRange,
};
use fixtures::{
    formal_main_audit_method_wire, forward_crisis_row, observation, runtime_history_point,
    runtime_history_point_with_state, synthetic_backtest_summary,
    synthetic_backtest_summary_with_dates, synthetic_backtest_summary_with_window,
    synthetic_runtime_scenarios, test_release_with_bundle,
};

mod options;
mod quality;
mod review;
mod split_requirements;
mod training;

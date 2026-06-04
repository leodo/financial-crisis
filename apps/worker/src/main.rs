use std::env;

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
    build_formal_dataset_summary, collect_formal_dataset_scenario_ranges, feature_quality_grade,
    formal_dataset_split_profile, has_extension_acute_core_features,
    has_main_dataset_core_features, load_formal_dataset_scenario_sets,
    load_label_set_crisis_scenarios, print_formal_dataset_summary,
    render_formal_dataset_summary_markdown, row_has_action_episode_label,
    scenario_count_for_split_range, AuditExportEnvelope, AuditMethodResponseWire,
    FormalDatasetSplitProfile, FormalDatasetSummaryEnvelope, RuntimeThresholdDiagnosticsWire,
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
    FormalDatasetBuildOptions, FormalDatasetSummaryOptions, PredictionSnapshotQueryOptions,
    ProbabilityModelShape, RefreshLatestOptions,
};
#[cfg(test)]
use commands::{PipelineDatasetSource, PipelineTrainOptions};
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
    ReleaseReviewFailureModeSummary, ReleaseReviewHistoricalAuditActionSummary,
    ReleaseReviewHistoricalAuditAttributionSummary, ReleaseReviewHistoricalAuditPriority,
    ReleaseReviewHistoricalAuditWorkstreamSummary, ReleaseReviewRuntimeBlockCount,
    ReleaseReviewRuntimeDominantCategories, ReleaseReviewRuntimeSeparationComparison,
    ReleaseReviewScalarMetric, ReleaseReviewScenarioFocusDiagnostic,
    ReleaseReviewScenarioPointComparison, ReleaseRuntimeReviewDiagnostics,
    ReleaseRuntimeSeparationSummary,
};
#[cfg(test)]
pub(crate) use release_review::{
    classify_regime_separation, summarize_release_runtime_regime_probabilities,
    summarize_release_runtime_regime_separation,
};
use reporting::write_formal_dataset_summary_report;
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

const DEFAULT_SQLITE_PATH: &str = "data/fc-local.sqlite";
const DEFAULT_RAW_DATA_DIR: &str = "data/raw";
const DEFAULT_API_RELOAD_URL: &str = "http://127.0.0.1:18080/api/system/reload";
const DEFAULT_AUDIT_API_BASE_URL: &str = "http://127.0.0.1:18080";
const DEFAULT_AUDIT_OUTPUT_DIR: &str = "reports/rolling-audit";
const DEFAULT_FORMAL_FEATURE_SET_VERSION: &str = "feature_formal_v1_main_20260531";
const DEFAULT_FORMAL_DATASET_ID: &str = "formal_v1_main_1990_daily";
const DEFAULT_FORMAL_LABEL_VERSION: &str = "formal_label_v1_main";
const DEFAULT_FORMAL_SCENARIO_SET_VERSION: &str = "scenario_v1_main";
const DEFAULT_FORMAL_MAIN_CONTEXT_WINDOW_SET_ID: &str = "protected_stress_windows_v1";
const FEATURE_SNAPSHOT_STATUS_READY: &str = "ready";
const FEATURE_SNAPSHOT_STATUS_COVERAGE_OR_VISIBILITY_FAILED: &str = "coverage_or_visibility_failed";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_target(false).init();

    let args = env::args().skip(1).collect::<Vec<_>>();
    commands::run_from_args(args).await
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    use chrono::{NaiveDate, TimeZone, Utc};
    use fc_domain::{
        ActionEpisodeTemplateId, ActionabilityBundle, ActionabilityEvaluationSummary,
        ActionabilityLevelBundle, AssessmentHistoryPoint, AssessmentMethodVersions,
        BacktestScenarioSummary, BacktestSignalSource, DecisionPosture, FeatureSnapshotRecord,
        FormalDatasetRowRecord, Frequency, HorizonEvaluationSummary, LogisticProbabilityModel,
        ModelReleaseManifest, ModelReleaseRecord, Observation, PlattCalibrationArtifact,
        ProbabilityBundle, ProbabilityBundleEvaluation, RegimeSeparationEvaluationSummary,
        RiskLevel, TimeToRiskBucket, PROBABILITY_FEATURE_TRANSFORM_IDENTITY_V1,
        PROBABILITY_MODEL_FAMILY_LINEAR_V1,
    };

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
        release_review_historical_audit_takeaways, release_review_runtime_separation_takeaways,
        round3, scenario_aware_formal_split_bounds, scenario_count_for_index_range,
        select_actionability_calibration_strategy, select_actionability_decision_threshold,
        select_probability_calibration_strategy, select_probability_decision_threshold,
        summarize_release_review_failure_modes, summarize_release_review_historical_audit_actions,
        summarize_release_review_historical_audit_attribution,
        summarize_release_review_historical_audit_priorities,
        summarize_release_review_historical_audit_workstreams,
        summarize_release_runtime_regime_probabilities,
        summarize_release_runtime_regime_separation, ActionabilityLevel, AuditExportOptions,
        CrisisScenario, FeatureSnapshotBuildOptions, FormalDatasetBuildOptions,
        FormalDatasetSummaryOptions, FormalSplitLabelSupport, PipelineDatasetSource,
        PipelineTrainOptions, PointInTimeMode, PredictionSnapshotQueryOptions,
        ProbabilityCalibrationSelection, ProbabilityModelShape, ProbabilityTargetLabelMode,
        ProbabilityThresholdDiagnosticsInput, ProbabilityThresholdSelection,
        ProbabilityTrainingRegime, ProbabilityTrainingRow, RefreshLatestOptions,
        ReleaseActionabilityLevelReview, ReleaseActionabilityReview,
        ReleaseReviewHistoricalAuditAttributionSummary, ReleaseReviewHistoricalAuditPriority,
        ReleaseReviewRuntimeDominantCategories, ReleaseReviewRuntimeSeparationComparison,
        ReleaseReviewScenarioFocusDiagnostic, ReleaseRuntimeReviewDiagnostics,
        ReleaseRuntimeSeparationSummary, RuntimeThresholdDiagnosticsWire, ScenarioRowRange,
    };

    fn observation(
        source_id: &str,
        frequency: Frequency,
        as_of_date: NaiveDate,
        publication_time: Option<chrono::DateTime<Utc>>,
    ) -> Observation {
        Observation {
            indicator_id: "test_indicator".to_string(),
            entity_id: "us".to_string(),
            as_of_date,
            period_start: Some(as_of_date),
            period_end: Some(as_of_date),
            frequency,
            value: 1.0,
            unit: "value".to_string(),
            source_id: source_id.to_string(),
            dataset_id: "test_dataset".to_string(),
            revision_time: None,
            publication_time,
            quality_score: 90.0,
            quality_flags: Vec::new(),
        }
    }

    fn test_release_with_bundle(bundle: &ProbabilityBundle) -> ModelReleaseRecord {
        let bundle_path = std::env::temp_dir().join(format!(
            "fc-probability-guard-{}.json",
            Utc::now()
                .timestamp_nanos_opt()
                .unwrap_or_default()
                .unsigned_abs()
        ));
        std::fs::write(&bundle_path, serde_json::to_string_pretty(bundle).unwrap()).unwrap();
        ModelReleaseRecord {
            manifest: ModelReleaseManifest {
                release_id: bundle.bundle_id.clone(),
                market_scope: bundle.market_scope.clone(),
                status: "approved".to_string(),
                probability_mode: bundle.probability_mode.clone(),
                serving_status: "healthy".to_string(),
                bundle_uri: bundle_path.to_string_lossy().to_string(),
                feature_set_version: "feature_formal_v1_main_20260531".to_string(),
                label_version: "formal_label_v1_main".to_string(),
                prob_model_version: "prob".to_string(),
                calibration_version: "calib".to_string(),
                posture_policy_version: "posture".to_string(),
                action_playbook_version: "playbook".to_string(),
                point_in_time_mode: "best_effort".to_string(),
                training_range_start: None,
                training_range_end: None,
                calibration_range_start: None,
                calibration_range_end: None,
                evaluation_range_start: None,
                evaluation_range_end: None,
                brier_score: Some(0.1),
                log_loss: Some(0.2),
                ece: Some(0.1),
                note: "test".to_string(),
            },
            created_at: Utc::now(),
            activated_at: None,
            retired_at: None,
        }
    }

    fn synthetic_runtime_scenarios() -> Vec<CrisisScenario> {
        vec![CrisisScenario {
            scenario_id: "synthetic".to_string(),
            family: "systemic_credit_banking_crisis".to_string(),
            training_role: "mandatory".to_string(),
            pre_warning_start: NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
            crisis_start: NaiveDate::from_ymd_opt(2000, 2, 1).unwrap(),
            acute_start: None,
            crisis_end: NaiveDate::from_ymd_opt(2000, 2, 20).unwrap(),
            default_horizon_roles: vec![20, 60],
            protected_window: false,
            protected_action_levels: Vec::new(),
            episode_template_id: ActionEpisodeTemplateId::SystemicCreditBankingCrisis,
            action_episode_overrides: None,
        }]
    }

    fn synthetic_backtest_summary(
        scenario_id: &str,
        name: &str,
        lead_time_days: Option<i64>,
        actionable_lead_time_days: Option<i64>,
        false_positive_count: u32,
    ) -> BacktestScenarioSummary {
        BacktestScenarioSummary {
            scenario_id: scenario_id.to_string(),
            name: name.to_string(),
            region: "us".to_string(),
            signal_source: BacktestSignalSource::RealHistory,
            crisis_start: NaiveDate::from_ymd_opt(2023, 3, 10).unwrap(),
            crisis_end: NaiveDate::from_ymd_opt(2023, 3, 20).unwrap(),
            first_l2_date: None,
            first_l3_date: None,
            max_level: RiskLevel::Crisis,
            max_score: 72.0,
            lead_time_days,
            actionable_lead_time_days,
            false_positive_count,
            missed: actionable_lead_time_days.is_none(),
            history_start: Some(NaiveDate::from_ymd_opt(2023, 1, 1).unwrap()),
            history_end: Some(NaiveDate::from_ymd_opt(2023, 3, 20).unwrap()),
            history_point_count: 50,
            note: "test".to_string(),
            top_contributors: Vec::new(),
            method_version: "test".to_string(),
        }
    }

    fn synthetic_backtest_summary_with_dates(
        scenario_id: &str,
        name: &str,
        first_l2_date: Option<NaiveDate>,
        first_l3_date: Option<NaiveDate>,
        lead_time_days: Option<i64>,
        actionable_lead_time_days: Option<i64>,
        false_positive_count: u32,
    ) -> BacktestScenarioSummary {
        let mut summary = synthetic_backtest_summary(
            scenario_id,
            name,
            lead_time_days,
            actionable_lead_time_days,
            false_positive_count,
        );
        summary.first_l2_date = first_l2_date;
        summary.first_l3_date = first_l3_date;
        summary
    }

    fn runtime_history_point(
        as_of_date: NaiveDate,
        raw_probability: f64,
        calibrated_probability: f64,
    ) -> AssessmentHistoryPoint {
        AssessmentHistoryPoint {
            as_of_date,
            overall_score: 50.0,
            p_5d: calibrated_probability,
            p_20d: calibrated_probability,
            p_60d: calibrated_probability,
            raw_p_5d: Some(raw_probability),
            raw_p_20d: Some(raw_probability),
            raw_p_60d: Some(raw_probability),
            posture: DecisionPosture::Normal,
            time_to_risk_bucket: TimeToRiskBucket::Normal,
            external_shock_score: 20.0,
            posture_trigger_codes: Vec::new(),
            posture_blocker_codes: Vec::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn runtime_history_point_with_state(
        as_of_date: NaiveDate,
        overall_score: f64,
        p_5d: f64,
        p_20d: f64,
        p_60d: f64,
        posture: DecisionPosture,
        time_to_risk_bucket: TimeToRiskBucket,
        external_shock_score: f64,
        posture_trigger_codes: &[&str],
    ) -> AssessmentHistoryPoint {
        AssessmentHistoryPoint {
            as_of_date,
            overall_score,
            p_5d,
            p_20d,
            p_60d,
            raw_p_5d: Some(p_5d),
            raw_p_20d: Some(p_20d),
            raw_p_60d: Some(p_60d),
            posture,
            time_to_risk_bucket,
            external_shock_score,
            posture_trigger_codes: posture_trigger_codes
                .iter()
                .map(|code| (*code).to_string())
                .collect(),
            posture_blocker_codes: Vec::new(),
        }
    }

    fn formal_main_audit_method_wire() -> super::AuditMethodResponseWire {
        super::AuditMethodResponseWire {
            method: AssessmentMethodVersions {
                score_method_version: "score_v1".to_string(),
                prob_model_version: "prob_v1".to_string(),
                calibration_version: "calib_v1".to_string(),
                actionability_model_version: None,
                actionability_calibration_version: None,
                feature_set_version: "feature_formal_v1_main_20260531".to_string(),
                label_version: "formal_label_v1_main".to_string(),
                posture_policy_version: "posture_v1".to_string(),
                action_playbook_version: "playbook_v1".to_string(),
                fusion_policy_version: None,
                actionability_enabled: false,
                probability_mode: "formal_bundle_v1".to_string(),
                release_status: "active_formal".to_string(),
                release_id: Some("test_release".to_string()),
                point_in_time_mode: "raw_feature_replay".to_string(),
            },
            note: "test".to_string(),
            protected_stress_window_catalog: None,
            runtime_thresholds: Some(super::RuntimeThresholdDiagnosticsWire {
                prepare_p60d: 0.10,
                hedge_p20d: 0.07,
                defend_p5d: 0.03,
                severe_now_p20d: 0.27,
                elevated_weeks_p60d: 0.20,
                external_prepare_p20d: 0.05,
                carry_prepare_p60d: 0.08,
                downgrade_prepare_p60d: 0.075,
                downgrade_hedge_p20d: 0.053,
                downgrade_defend_p5d: 0.02,
                history_runtime_policy_version: "runtime_history_test".to_string(),
            }),
        }
    }

    fn forward_crisis_row(
        as_of_date: NaiveDate,
        label_20d: u8,
        regime_20d: ProbabilityTrainingRegime,
    ) -> ProbabilityTrainingRow {
        ProbabilityTrainingRow {
            as_of_date,
            market_scope: "financial_system".to_string(),
            release_id: None,
            probability_mode: Some("formal_bundle_v1".to_string()),
            freshness_status: Some("fresh".to_string()),
            time_to_risk_bucket: Some("test".to_string()),
            split_name: Some("calibration".to_string()),
            features: BTreeMap::new(),
            primary_scenario_id: Some("scenario_a".to_string()),
            scenario_family: Some("systemic_credit_banking_crisis".to_string()),
            scenario_training_role: None,
            days_to_primary_crisis_start: Some(10),
            primary_scenario_supports_5d: true,
            primary_scenario_supports_20d: true,
            primary_scenario_supports_60d: true,
            label_5d: 0,
            label_20d,
            label_60d: 0,
            regime_5d: ProbabilityTrainingRegime::Normal,
            regime_20d,
            regime_60d: ProbabilityTrainingRegime::Normal,
            action_label_5d: 0,
            action_label_20d: 0,
            action_label_60d: 0,
            prepare_episode_label: 0,
            hedge_episode_label: 0,
            defend_episode_label: 0,
            primary_action_level: None,
            action_episode_id: None,
            action_episode_phase: "outside".to_string(),
            protected_action_window: false,
        }
    }

    #[test]
    fn parses_refresh_latest_defaults() {
        let options = RefreshLatestOptions::parse(&[]).unwrap();
        assert_eq!(options.fast_lookback_days, 45);
        assert_eq!(options.slow_lookback_years, 15);
        assert_eq!(options.fred_chunk_days, 45);
        assert!(!options.skip_world_bank);
        assert!(!options.include_gdelt);
        assert!(options.reload_api);
    }

    #[test]
    fn parses_refresh_latest_overrides() {
        let args = vec![
            "--fast-lookback-days".to_string(),
            "90".to_string(),
            "--skip-world-bank".to_string(),
            "--include-gdelt".to_string(),
            "--no-reload-api".to_string(),
        ];
        let options = RefreshLatestOptions::parse(&args).unwrap();
        assert_eq!(options.fast_lookback_days, 90);
        assert!(options.skip_world_bank);
        assert!(options.include_gdelt);
        assert!(!options.reload_api);
    }

    #[test]
    fn parses_audit_export_overrides() {
        let args = vec![
            "--api-base-url".to_string(),
            "http://127.0.0.1:18081".to_string(),
            "--output-dir".to_string(),
            "tmp/audit".to_string(),
        ];
        let options = AuditExportOptions::parse(&args).unwrap();
        assert_eq!(options.api_base_url, "http://127.0.0.1:18081");
        assert_eq!(options.output_dir, PathBuf::from("tmp/audit"));
    }

    #[test]
    fn parses_release_publish_options() {
        let args = vec![
            "--manifest".to_string(),
            "config/model-releases/us-heuristic-bootstrap.json".to_string(),
            "--activate".to_string(),
            "--reload-api".to_string(),
            "--skip-operational-guard".to_string(),
            "--updated-by".to_string(),
            "tester".to_string(),
        ];
        let options = ReleasePublishOptions::parse(&args).unwrap();
        assert!(options.activate);
        assert!(options.reload_api);
        assert!(options.skip_operational_guard);
        assert_eq!(options.updated_by, "tester");
        assert_eq!(
            options.manifest_path,
            PathBuf::from("config/model-releases/us-heuristic-bootstrap.json")
        );
    }

    #[test]
    fn parses_release_switch_options() {
        let args = vec![
            "--release-id".to_string(),
            "release-123".to_string(),
            "--market-scope".to_string(),
            "financial_system".to_string(),
            "--reload-api".to_string(),
            "--skip-operational-guard".to_string(),
        ];
        let options = ReleaseSwitchOptions::parse(&args).unwrap();
        assert_eq!(options.release_id, "release-123");
        assert_eq!(options.market_scope.as_deref(), Some("financial_system"));
        assert!(options.reload_api);
        assert!(options.skip_operational_guard);
    }

    #[test]
    fn parses_release_review_options() {
        let args = vec![
            "--candidate-release-id".to_string(),
            "candidate-123".to_string(),
            "--baseline-release-id".to_string(),
            "baseline-456".to_string(),
            "--market-scope".to_string(),
            "financial_system".to_string(),
            "--output-dir".to_string(),
            "reports/release-review".to_string(),
            "--history-mode".to_string(),
            "default".to_string(),
            "--history-limit".to_string(),
            "5000".to_string(),
        ];
        let options = ReleaseReviewOptions::parse(&args).unwrap();
        assert_eq!(options.candidate_release_id, "candidate-123");
        assert_eq!(options.baseline_release_id.as_deref(), Some("baseline-456"));
        assert_eq!(options.market_scope.as_deref(), Some("financial_system"));
        assert_eq!(options.output_dir, PathBuf::from("reports/release-review"));
        assert_eq!(options.history_mode, super::ApiReloadHistoryMode::Default);
        assert_eq!(options.history_limit, 5000);
    }

    #[test]
    fn release_review_defaults_to_ignored_artifact_dir() {
        let args = vec![
            "--candidate-release-id".to_string(),
            "candidate-123".to_string(),
        ];
        let options = ReleaseReviewOptions::parse(&args).unwrap();
        assert_eq!(
            options.output_dir,
            PathBuf::from("artifacts/research/release-review")
        );
        assert_eq!(
            options.history_mode,
            super::ApiReloadHistoryMode::StrictRebuild
        );
        assert_eq!(options.history_limit, 20_000);
    }

    #[test]
    fn parses_release_probability_slice_options() {
        let args = vec![
            "--release-id".to_string(),
            "us_formal_family_hybrid_20260603T144814".to_string(),
            "--from".to_string(),
            "2022-12-01".to_string(),
            "--to".to_string(),
            "2023-03-15".to_string(),
            "--output-dir".to_string(),
            "reports/release-probability-slices".to_string(),
            "--history-mode".to_string(),
            "default".to_string(),
            "--history-limit".to_string(),
            "5000".to_string(),
        ];
        let options = ReleaseProbabilitySliceOptions::parse(&args).unwrap();
        assert_eq!(
            options.release_id,
            "us_formal_family_hybrid_20260603T144814"
        );
        assert_eq!(
            options.from_date,
            NaiveDate::from_ymd_opt(2022, 12, 1).unwrap()
        );
        assert_eq!(
            options.to_date,
            NaiveDate::from_ymd_opt(2023, 3, 15).unwrap()
        );
        assert_eq!(
            options.output_dir,
            PathBuf::from("reports/release-probability-slices")
        );
        assert_eq!(options.history_mode, super::ApiReloadHistoryMode::Default);
        assert_eq!(options.history_limit, 5000);
    }

    #[test]
    fn release_probability_slice_defaults_to_ignored_artifact_dir() {
        let args = vec![
            "--release-id".to_string(),
            "candidate-123".to_string(),
            "--from".to_string(),
            "2023-01-01".to_string(),
            "--to".to_string(),
            "2023-01-31".to_string(),
        ];
        let options = ReleaseProbabilitySliceOptions::parse(&args).unwrap();
        assert_eq!(
            options.output_dir,
            PathBuf::from("artifacts/research/release-probability-slices")
        );
        assert_eq!(
            options.history_mode,
            super::ApiReloadHistoryMode::StrictRebuild
        );
        assert_eq!(options.history_limit, 20_000);
    }

    #[test]
    fn parses_release_formal_probability_slice_options() {
        let args = vec![
            "--release-id".to_string(),
            "us_formal_family_hybrid_20260603T144814".to_string(),
            "--dataset-id".to_string(),
            "formal_v1_main_1990_daily".to_string(),
            "--dataset-version".to_string(),
            "20260601T172759".to_string(),
            "--scenario-id".to_string(),
            "us_regional_banks_2023".to_string(),
            "--from".to_string(),
            "2022-12-01".to_string(),
            "--to".to_string(),
            "2023-03-15".to_string(),
            "--output-dir".to_string(),
            "reports/formal-dataset-slices".to_string(),
        ];
        let options = ReleaseFormalProbabilitySliceOptions::parse(&args).unwrap();
        assert_eq!(
            options.release_id,
            "us_formal_family_hybrid_20260603T144814"
        );
        assert_eq!(options.dataset_id, "formal_v1_main_1990_daily");
        assert_eq!(options.dataset_version.as_deref(), Some("20260601T172759"));
        assert_eq!(
            options.scenario_id.as_deref(),
            Some("us_regional_banks_2023")
        );
        assert_eq!(
            options.from_date,
            NaiveDate::from_ymd_opt(2022, 12, 1).unwrap()
        );
        assert_eq!(
            options.to_date,
            NaiveDate::from_ymd_opt(2023, 3, 15).unwrap()
        );
        assert_eq!(
            options.output_dir,
            PathBuf::from("reports/formal-dataset-slices")
        );
    }

    #[test]
    fn release_formal_probability_slice_defaults_to_formal_dataset_artifact_dir() {
        let args = vec![
            "--release-id".to_string(),
            "candidate-123".to_string(),
            "--from".to_string(),
            "2023-01-01".to_string(),
            "--to".to_string(),
            "2023-01-31".to_string(),
        ];
        let options = ReleaseFormalProbabilitySliceOptions::parse(&args).unwrap();
        assert_eq!(
            options.output_dir,
            PathBuf::from("artifacts/research/formal-dataset-slices")
        );
        assert_eq!(options.dataset_id, "formal_v1_main_1990_daily");
        assert_eq!(options.dataset_version, None);
        assert_eq!(options.dataset_key, None);
        assert_eq!(options.scenario_id, None);
    }

    #[test]
    fn parses_release_formal_probability_compare_options() {
        let args = vec![
            "--baseline-release-id".to_string(),
            "baseline-123".to_string(),
            "--candidate-release-id".to_string(),
            "candidate-456".to_string(),
            "--dataset-id".to_string(),
            "formal_v1_main_1990_daily".to_string(),
            "--scenario-id".to_string(),
            "us_regional_banks_2023".to_string(),
            "--from".to_string(),
            "2022-12-01".to_string(),
            "--to".to_string(),
            "2023-03-15".to_string(),
            "--output-dir".to_string(),
            "reports/formal-probability-compares".to_string(),
        ];
        let options = ReleaseFormalProbabilityCompareOptions::parse(&args).unwrap();
        assert_eq!(options.baseline_release_id, "baseline-123");
        assert_eq!(options.candidate_release_id, "candidate-456");
        assert_eq!(options.dataset_id, "formal_v1_main_1990_daily");
        assert_eq!(
            options.scenario_id.as_deref(),
            Some("us_regional_banks_2023")
        );
        assert_eq!(
            options.from_date,
            NaiveDate::from_ymd_opt(2022, 12, 1).unwrap()
        );
        assert_eq!(
            options.to_date,
            NaiveDate::from_ymd_opt(2023, 3, 15).unwrap()
        );
        assert_eq!(
            options.output_dir,
            PathBuf::from("reports/formal-probability-compares")
        );
    }

    #[test]
    fn release_formal_probability_compare_defaults_to_compare_artifact_dir() {
        let args = vec![
            "--baseline-release-id".to_string(),
            "baseline-123".to_string(),
            "--candidate-release-id".to_string(),
            "candidate-456".to_string(),
            "--from".to_string(),
            "2023-01-01".to_string(),
            "--to".to_string(),
            "2023-01-31".to_string(),
        ];
        let options = ReleaseFormalProbabilityCompareOptions::parse(&args).unwrap();
        assert_eq!(
            options.output_dir,
            PathBuf::from("artifacts/research/formal-probability-compares")
        );
        assert_eq!(options.dataset_id, "formal_v1_main_1990_daily");
        assert_eq!(options.dataset_version, None);
        assert_eq!(options.dataset_key, None);
        assert_eq!(options.scenario_id, None);
    }

    #[test]
    fn parses_prediction_snapshot_query_options() {
        let args = vec![
            "--market-scope".to_string(),
            "financial_system".to_string(),
            "--from".to_string(),
            "2026-05-01".to_string(),
            "--to".to_string(),
            "2026-05-31".to_string(),
            "--limit".to_string(),
            "50".to_string(),
        ];
        let options = PredictionSnapshotQueryOptions::parse(&args).unwrap();
        assert_eq!(options.market_scope.as_deref(), Some("financial_system"));
        assert_eq!(
            options.from,
            Some(NaiveDate::from_ymd_opt(2026, 5, 1).unwrap())
        );
        assert_eq!(
            options.to,
            Some(NaiveDate::from_ymd_opt(2026, 5, 31).unwrap())
        );
        assert_eq!(options.limit, Some(50));
    }

    #[test]
    fn parses_feature_snapshot_build_options() {
        let args = vec![
            "--market-scope".to_string(),
            "financial_system".to_string(),
            "--from".to_string(),
            "2020-01-01".to_string(),
            "--to".to_string(),
            "2020-12-31".to_string(),
            "--feature-set-version".to_string(),
            "feature_formal_v1_test".to_string(),
        ];
        let options = FeatureSnapshotBuildOptions::parse(&args).unwrap();
        assert_eq!(options.market_scope, "financial_system");
        assert_eq!(
            options.from,
            Some(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap())
        );
        assert_eq!(
            options.to,
            Some(NaiveDate::from_ymd_opt(2020, 12, 31).unwrap())
        );
        assert_eq!(options.feature_set_version, "feature_formal_v1_test");
        assert_eq!(options.point_in_time_mode, "best_effort");
        assert!(!options.force_rebuild);
    }

    #[test]
    fn parses_feature_snapshot_force_rebuild_option() {
        let args = vec!["--force-rebuild".to_string()];
        let options = FeatureSnapshotBuildOptions::parse(&args).unwrap();
        assert!(options.force_rebuild);
    }

    #[test]
    fn parses_formal_dataset_build_options() {
        let args = vec![
            "--market-scope".to_string(),
            "financial_system".to_string(),
            "--dataset-id".to_string(),
            "formal_v1_main_1990_daily".to_string(),
            "--dataset-version".to_string(),
            "20260531T120000".to_string(),
            "--label-version".to_string(),
            "formal_label_v1_main".to_string(),
        ];
        let options = FormalDatasetBuildOptions::parse(&args).unwrap();
        assert_eq!(options.feature.market_scope, "financial_system");
        assert_eq!(options.dataset_id, "formal_v1_main_1990_daily");
        assert_eq!(options.dataset_version.as_deref(), Some("20260531T120000"));
        assert_eq!(options.label_version, "formal_label_v1_main");
    }

    #[test]
    fn extension_acute_dataset_min_date_allows_pre1990_history() {
        assert_eq!(
            super::formal_dataset_min_date("formal_label_v1_ext_acute"),
            NaiveDate::from_ymd_opt(1987, 1, 1).unwrap()
        );
        assert_eq!(
            super::formal_dataset_min_date("formal_label_v1_main"),
            NaiveDate::from_ymd_opt(1990, 1, 2).unwrap()
        );
    }

    #[test]
    fn extension_acute_dataset_allows_proxy_feature_gate_without_vix() {
        let snapshot = FeatureSnapshotRecord {
            as_of_date: NaiveDate::from_ymd_opt(1987, 10, 19).unwrap(),
            entity_id: "us".to_string(),
            market_scope: "financial_system".to_string(),
            feature_set_version: "feature_formal_v1_main_20260531".to_string(),
            point_in_time_mode: "best_effort".to_string(),
            visibility_status: "coverage_or_visibility_failed".to_string(),
            latest_visible_at: Some(Utc::now()),
            coverage_score: 0.56,
            core_feature_coverage: 0.625,
            trigger_feature_coverage: 0.50,
            external_feature_coverage: 0.50,
            feature_count: 4,
            features: [
                ("us_curve_10y2y_level".to_string(), -0.2),
                ("us_baa_10y_spread_level".to_string(), 2.8),
                ("us_fed_funds_level".to_string(), 6.5),
                ("us_usdjpy_level".to_string(), 0.0068),
            ]
            .into_iter()
            .collect(),
            created_at: Utc::now(),
        };

        assert!(super::formal_dataset_snapshot_is_usable(
            &snapshot,
            "formal_label_v1_ext_acute"
        ));
        assert!(!super::formal_dataset_snapshot_is_usable(
            &snapshot,
            "formal_label_v1_main"
        ));
    }

    #[test]
    fn extension_stress_dataset_allows_1990s_partial_coverage_gate() {
        let snapshot = FeatureSnapshotRecord {
            as_of_date: NaiveDate::from_ymd_opt(1993, 1, 5).unwrap(),
            entity_id: "us".to_string(),
            market_scope: "financial_system".to_string(),
            feature_set_version: "feature_formal_v1_main_20260531".to_string(),
            point_in_time_mode: "best_effort".to_string(),
            visibility_status: "ready".to_string(),
            latest_visible_at: Some(Utc::now()),
            coverage_score: 0.785,
            core_feature_coverage: 0.875,
            trigger_feature_coverage: 0.833,
            external_feature_coverage: 0.50,
            feature_count: 4,
            features: [
                ("us_vix_level".to_string(), 12.0),
                ("us_curve_10y2y_level".to_string(), 1.2),
                ("us_baa_10y_spread_level".to_string(), 2.1),
                ("us_fed_funds_level".to_string(), 3.0),
            ]
            .into_iter()
            .collect(),
            created_at: Utc::now(),
        };

        assert!(super::formal_dataset_snapshot_is_usable(
            &snapshot,
            "formal_label_v1_ext_stress"
        ));
        assert!(!super::formal_dataset_snapshot_is_usable(
            &snapshot,
            "formal_label_v1_main"
        ));
    }

    #[test]
    fn parses_formal_dataset_summary_options() {
        let args = vec![
            "--dataset-id".to_string(),
            "formal_v1_main_1990_daily".to_string(),
            "--dataset-version".to_string(),
            "20260531Tpitmain".to_string(),
            "--output-dir".to_string(),
            "reports/formal-dataset".to_string(),
        ];
        let options = FormalDatasetSummaryOptions::parse(&args).unwrap();
        assert_eq!(options.dataset_id, "formal_v1_main_1990_daily");
        assert_eq!(options.dataset_version.as_deref(), Some("20260531Tpitmain"));
        assert_eq!(options.output_dir, PathBuf::from("reports/formal-dataset"));
    }

    #[test]
    fn formal_dataset_summary_defaults_to_ignored_artifact_dir() {
        let options = FormalDatasetSummaryOptions::parse(&[]).unwrap();
        assert_eq!(
            options.output_dir,
            PathBuf::from("artifacts/research/formal-dataset")
        );
    }

    #[test]
    fn parses_formal_dataset_slice_options() {
        let args = vec![
            "--dataset-key".to_string(),
            "formal_v1_main_1990_daily:20260531Tpitmain".to_string(),
            "--scenario-id".to_string(),
            "us_regional_banks_2023".to_string(),
            "--from".to_string(),
            "2022-12-01".to_string(),
            "--to".to_string(),
            "2023-03-15".to_string(),
            "--split-name".to_string(),
            "evaluation".to_string(),
            "--limit".to_string(),
            "120".to_string(),
            "--output-dir".to_string(),
            "reports/formal-dataset-slices".to_string(),
        ];
        let options = FormalDatasetSliceOptions::parse(&args).unwrap();
        assert_eq!(
            options.dataset_key.as_deref(),
            Some("formal_v1_main_1990_daily:20260531Tpitmain")
        );
        assert_eq!(options.scenario_id, "us_regional_banks_2023");
        assert_eq!(
            options.from_date,
            Some(NaiveDate::from_ymd_opt(2022, 12, 1).unwrap())
        );
        assert_eq!(
            options.to_date,
            Some(NaiveDate::from_ymd_opt(2023, 3, 15).unwrap())
        );
        assert_eq!(options.split_name.as_deref(), Some("evaluation"));
        assert_eq!(options.limit, Some(120));
        assert_eq!(
            options.output_dir,
            PathBuf::from("reports/formal-dataset-slices")
        );
    }

    #[test]
    fn formal_dataset_slice_defaults_to_ignored_artifact_dir() {
        let args = vec![
            "--scenario-id".to_string(),
            "us_regional_banks_2023".to_string(),
        ];
        let options = FormalDatasetSliceOptions::parse(&args).unwrap();
        assert_eq!(
            options.output_dir,
            PathBuf::from("artifacts/research/formal-dataset-slices")
        );
    }

    #[test]
    fn sanitize_filename_component_replaces_windows_reserved_characters() {
        assert_eq!(
            sanitize_filename_component("formal_v1_main_1990_daily:20260601T172759"),
            "formal_v1_main_1990_daily_20260601T172759"
        );
    }

    #[test]
    fn parses_pipeline_train_defaults_to_formal_dataset() {
        let options = PipelineTrainOptions::parse(&[]).unwrap();
        assert_eq!(options.dataset_source, PipelineDatasetSource::Formal);
        assert_eq!(options.model_shape, ProbabilityModelShape::LinearV1);
        assert_eq!(options.dataset_id, "formal_v1_main_1990_daily");
        assert_eq!(options.dataset_version, None);
        assert_eq!(options.dataset_key, None);
        assert!(options.aux_dataset_keys.is_empty());
        assert_eq!(
            options.output_dir,
            PathBuf::from("artifacts/research/model-bundles/generated")
        );
        assert_eq!(
            options.manifest_dir,
            PathBuf::from("artifacts/research/model-releases/generated")
        );
        assert_eq!(options.release_prefix, "us_formal_main");
    }

    #[test]
    fn parses_pipeline_train_snapshot_override() {
        let args = vec![
            "--dataset-source".to_string(),
            "snapshot".to_string(),
            "--release-prefix".to_string(),
            "custom_prefix".to_string(),
            "--market-scope".to_string(),
            "financial_system".to_string(),
        ];
        let options = PipelineTrainOptions::parse(&args).unwrap();
        assert_eq!(options.dataset_source, PipelineDatasetSource::Snapshot);
        assert_eq!(options.model_shape, ProbabilityModelShape::LinearV1);
        assert_eq!(options.release_prefix, "custom_prefix");
        assert_eq!(
            options.query.market_scope.as_deref(),
            Some("financial_system")
        );
    }

    #[test]
    fn parses_pipeline_train_interaction_tail_shape() {
        let args = vec![
            "--model-shape".to_string(),
            "interaction_tail_v1".to_string(),
        ];
        let options = PipelineTrainOptions::parse(&args).unwrap();

        assert_eq!(
            options.model_shape,
            ProbabilityModelShape::InteractionTailV1
        );
        assert_eq!(options.release_prefix, "us_formal_interaction_tail");
    }

    #[test]
    fn parses_pipeline_train_family_conditional_shape() {
        let args = vec![
            "--model-shape".to_string(),
            "family_conditional_v1".to_string(),
        ];
        let options = PipelineTrainOptions::parse(&args).unwrap();

        assert_eq!(
            options.model_shape,
            ProbabilityModelShape::FamilyConditionalV1
        );
        assert_eq!(options.release_prefix, "us_formal_family_conditional");
    }

    #[test]
    fn parses_pipeline_train_family_hybrid_shape() {
        let args = vec!["--model-shape".to_string(), "family_hybrid_v1".to_string()];
        let options = PipelineTrainOptions::parse(&args).unwrap();

        assert_eq!(options.model_shape, ProbabilityModelShape::FamilyHybridV1);
        assert_eq!(options.release_prefix, "us_formal_family_hybrid");
    }

    #[test]
    fn parses_pipeline_train_aux_dataset_keys() {
        let args = vec![
            "--dataset-key".to_string(),
            "formal_v1_main_1990_daily:20260601T172759".to_string(),
            "--aux-dataset-key".to_string(),
            "formal_v1_ext_stress_1990_daily:20260601T162655".to_string(),
            "--aux-dataset-key".to_string(),
            "formal_v1_ext_acute_pre1990:20260601T163102".to_string(),
        ];
        let options = PipelineTrainOptions::parse(&args).unwrap();
        assert_eq!(
            options.dataset_key.as_deref(),
            Some("formal_v1_main_1990_daily:20260601T172759")
        );
        assert_eq!(
            options.aux_dataset_keys,
            vec![
                "formal_v1_ext_stress_1990_daily:20260601T162655".to_string(),
                "formal_v1_ext_acute_pre1990:20260601T163102".to_string()
            ]
        );
    }

    #[test]
    fn best_effort_visibility_uses_release_rule_not_backfill_fetch_time_for_fred() {
        let observation = observation(
            "fred",
            Frequency::Monthly,
            NaiveDate::from_ymd_opt(2020, 1, 31).unwrap(),
            Some(Utc.with_ymd_and_hms(2026, 5, 31, 0, 0, 0).single().unwrap()),
        );

        assert!(!observation_is_visible_for_date(
            &observation,
            NaiveDate::from_ymd_opt(2020, 2, 14).unwrap(),
            PointInTimeMode::BestEffort
        ));
        assert!(observation_is_visible_for_date(
            &observation,
            NaiveDate::from_ymd_opt(2020, 2, 15).unwrap(),
            PointInTimeMode::BestEffort
        ));
    }

    #[test]
    fn strict_visibility_requires_timestamp_to_arrive_before_cutoff() {
        let observation = observation(
            "sec_edgar",
            Frequency::Daily,
            NaiveDate::from_ymd_opt(2020, 1, 2).unwrap(),
            Some(Utc.with_ymd_and_hms(2020, 1, 2, 23, 0, 0).single().unwrap()),
        );

        assert!(!observation_is_visible_for_date(
            &observation,
            NaiveDate::from_ymd_opt(2020, 1, 2).unwrap(),
            PointInTimeMode::Strict
        ));
        assert!(observation_is_visible_for_date(
            &observation,
            NaiveDate::from_ymd_opt(2020, 1, 3).unwrap(),
            PointInTimeMode::Strict
        ));
    }

    #[test]
    fn forward_crisis_label_uses_acute_anchor_for_5d_without_dropping_other_crisis_starts() {
        let acute_only = CrisisScenario {
            scenario_id: "acute".to_string(),
            family: "acute_market_liquidity_crash".to_string(),
            training_role: "mandatory".to_string(),
            pre_warning_start: NaiveDate::from_ymd_opt(2020, 1, 24).unwrap(),
            crisis_start: NaiveDate::from_ymd_opt(2020, 2, 24).unwrap(),
            acute_start: Some(NaiveDate::from_ymd_opt(2020, 3, 9).unwrap()),
            crisis_end: NaiveDate::from_ymd_opt(2020, 4, 30).unwrap(),
            default_horizon_roles: vec![5, 20],
            protected_window: false,
            protected_action_levels: Vec::new(),
            episode_template_id: ActionEpisodeTemplateId::AcuteMarketLiquidityCrash,
            action_episode_overrides: None,
        };
        let systemic_only = CrisisScenario {
            scenario_id: "systemic".to_string(),
            family: "systemic_credit_banking_crisis".to_string(),
            training_role: "mandatory".to_string(),
            pre_warning_start: NaiveDate::from_ymd_opt(2023, 2, 1).unwrap(),
            crisis_start: NaiveDate::from_ymd_opt(2023, 3, 8).unwrap(),
            acute_start: Some(NaiveDate::from_ymd_opt(2023, 3, 10).unwrap()),
            crisis_end: NaiveDate::from_ymd_opt(2023, 5, 15).unwrap(),
            default_horizon_roles: vec![20, 60],
            protected_window: false,
            protected_action_levels: Vec::new(),
            episode_template_id: ActionEpisodeTemplateId::SystemicCreditBankingCrisis,
            action_episode_overrides: None,
        };

        assert_eq!(
            forward_crisis_label(
                NaiveDate::from_ymd_opt(2020, 3, 4).unwrap(),
                &[acute_only.clone(), systemic_only.clone()],
                5,
            ),
            1
        );
        assert_eq!(
            forward_crisis_label(
                NaiveDate::from_ymd_opt(2020, 2, 20).unwrap(),
                &[acute_only.clone(), systemic_only.clone()],
                5,
            ),
            0
        );
        assert_eq!(
            forward_crisis_label(
                NaiveDate::from_ymd_opt(2023, 3, 4).unwrap(),
                &[acute_only.clone(), systemic_only.clone()],
                5,
            ),
            1
        );
        assert_eq!(
            forward_crisis_label(
                NaiveDate::from_ymd_opt(2023, 2, 20).unwrap(),
                &[acute_only, systemic_only],
                20,
            ),
            1
        );
    }

    #[test]
    fn action_window_label_extends_before_crisis_start_and_stays_near_onset() {
        let systemic = CrisisScenario {
            scenario_id: "systemic".to_string(),
            family: "systemic_credit_banking_crisis".to_string(),
            training_role: "mandatory".to_string(),
            pre_warning_start: NaiveDate::from_ymd_opt(2007, 2, 27).unwrap(),
            crisis_start: NaiveDate::from_ymd_opt(2007, 8, 1).unwrap(),
            acute_start: Some(NaiveDate::from_ymd_opt(2008, 9, 15).unwrap()),
            crisis_end: NaiveDate::from_ymd_opt(2009, 6, 30).unwrap(),
            default_horizon_roles: vec![20, 60],
            protected_window: false,
            protected_action_levels: Vec::new(),
            episode_template_id: ActionEpisodeTemplateId::SystemicCreditBankingCrisis,
            action_episode_overrides: None,
        };
        let acute = CrisisScenario {
            scenario_id: "acute".to_string(),
            family: "acute_market_liquidity_crash".to_string(),
            training_role: "mandatory".to_string(),
            pre_warning_start: NaiveDate::from_ymd_opt(2020, 1, 24).unwrap(),
            crisis_start: NaiveDate::from_ymd_opt(2020, 2, 24).unwrap(),
            acute_start: Some(NaiveDate::from_ymd_opt(2020, 3, 9).unwrap()),
            crisis_end: NaiveDate::from_ymd_opt(2020, 4, 30).unwrap(),
            default_horizon_roles: vec![5, 20],
            protected_window: false,
            protected_action_levels: Vec::new(),
            episode_template_id: ActionEpisodeTemplateId::AcuteMarketLiquidityCrash,
            action_episode_overrides: None,
        };

        assert_eq!(
            action_window_label(
                NaiveDate::from_ymd_opt(2007, 5, 10).unwrap(),
                std::slice::from_ref(&systemic),
                60,
            ),
            1
        );
        assert_eq!(
            action_window_label(
                NaiveDate::from_ymd_opt(2020, 2, 28).unwrap(),
                std::slice::from_ref(&acute),
                5,
            ),
            1
        );
        assert_eq!(
            action_window_label(
                NaiveDate::from_ymd_opt(2007, 8, 15).unwrap(),
                std::slice::from_ref(&systemic),
                20,
            ),
            1
        );
        assert_eq!(
            action_window_label(
                NaiveDate::from_ymd_opt(2007, 9, 15).unwrap(),
                std::slice::from_ref(&systemic),
                20,
            ),
            0
        );
    }

    #[test]
    fn forward_crisis_training_regime_marks_buffer_crisis_and_cooldown() {
        let systemic = CrisisScenario {
            scenario_id: "systemic".to_string(),
            family: "systemic_credit_banking_crisis".to_string(),
            training_role: "mandatory".to_string(),
            pre_warning_start: NaiveDate::from_ymd_opt(2007, 2, 27).unwrap(),
            crisis_start: NaiveDate::from_ymd_opt(2007, 8, 1).unwrap(),
            acute_start: Some(NaiveDate::from_ymd_opt(2008, 9, 15).unwrap()),
            crisis_end: NaiveDate::from_ymd_opt(2009, 6, 30).unwrap(),
            default_horizon_roles: vec![20, 60],
            protected_window: false,
            protected_action_levels: Vec::new(),
            episode_template_id: ActionEpisodeTemplateId::SystemicCreditBankingCrisis,
            action_episode_overrides: None,
        };

        assert_eq!(
            forward_crisis_training_regime(
                NaiveDate::from_ymd_opt(2007, 5, 10).unwrap(),
                std::slice::from_ref(&systemic),
                60,
            ),
            ProbabilityTrainingRegime::PreWarningBuffer
        );
        assert_eq!(
            forward_crisis_training_regime(
                NaiveDate::from_ymd_opt(2007, 6, 15).unwrap(),
                std::slice::from_ref(&systemic),
                60,
            ),
            ProbabilityTrainingRegime::PositiveWindow
        );
        assert_eq!(
            forward_crisis_training_regime(
                NaiveDate::from_ymd_opt(2008, 10, 1).unwrap(),
                std::slice::from_ref(&systemic),
                20,
            ),
            ProbabilityTrainingRegime::InCrisis
        );
        assert_eq!(
            forward_crisis_training_regime(
                NaiveDate::from_ymd_opt(2009, 7, 20).unwrap(),
                std::slice::from_ref(&systemic),
                20,
            ),
            ProbabilityTrainingRegime::PostCrisisCooldown
        );
        assert_eq!(
            forward_crisis_training_regime(
                NaiveDate::from_ymd_opt(2010, 1, 20).unwrap(),
                std::slice::from_ref(&systemic),
                20,
            ),
            ProbabilityTrainingRegime::Normal
        );
    }

    #[test]
    fn protected_context_promotes_main_regime_buffer_without_changing_positive_labels() {
        let scenario_sets = super::load_formal_dataset_scenario_sets(
            super::DEFAULT_FORMAL_SCENARIO_SET_VERSION,
            super::DEFAULT_FORMAL_LABEL_VERSION,
        )
        .unwrap();
        let protected_date = NaiveDate::from_ymd_opt(2021, 11, 15).unwrap();
        let cooldown_date = NaiveDate::from_ymd_opt(2022, 11, 10).unwrap();

        assert_eq!(
            forward_crisis_label(protected_date, &scenario_sets.positive_scenarios, 20),
            0
        );
        assert_eq!(
            forward_crisis_training_regime_with_context(
                protected_date,
                &scenario_sets.positive_scenarios,
                &scenario_sets.context_scenarios,
                20,
            ),
            ProbabilityTrainingRegime::PreWarningBuffer
        );
        assert_eq!(
            forward_crisis_training_regime_with_context(
                cooldown_date,
                &scenario_sets.positive_scenarios,
                &scenario_sets.context_scenarios,
                20,
            ),
            ProbabilityTrainingRegime::PostCrisisCooldown
        );
    }

    #[test]
    fn forward_crisis_negative_weights_and_calibration_scope_follow_regime() {
        let positive_row = ProbabilityTrainingRow {
            as_of_date: NaiveDate::from_ymd_opt(2020, 2, 20).unwrap(),
            market_scope: "financial_system".to_string(),
            release_id: None,
            probability_mode: Some("formal_bundle_v1".to_string()),
            freshness_status: Some("a".to_string()),
            time_to_risk_bucket: Some("weeks".to_string()),
            split_name: Some("calibration".to_string()),
            features: BTreeMap::new(),
            primary_scenario_id: Some("scenario_a".to_string()),
            scenario_family: Some("systemic_credit_banking_crisis".to_string()),
            scenario_training_role: None,
            days_to_primary_crisis_start: Some(10),
            primary_scenario_supports_5d: true,
            primary_scenario_supports_20d: true,
            primary_scenario_supports_60d: true,
            label_5d: 0,
            label_20d: 1,
            label_60d: 0,
            regime_5d: ProbabilityTrainingRegime::Normal,
            regime_20d: ProbabilityTrainingRegime::PositiveWindow,
            regime_60d: ProbabilityTrainingRegime::Normal,
            action_label_5d: 0,
            action_label_20d: 1,
            action_label_60d: 0,
            prepare_episode_label: 0,
            hedge_episode_label: 1,
            defend_episode_label: 0,
            primary_action_level: Some("hedge".to_string()),
            action_episode_id: Some("scenario_a:hedge".to_string()),
            action_episode_phase: "primary".to_string(),
            protected_action_window: false,
        };
        let mut normal_negative = positive_row.clone();
        normal_negative.label_20d = 0;
        normal_negative.regime_20d = ProbabilityTrainingRegime::Normal;
        normal_negative.primary_scenario_id = None;
        normal_negative.scenario_family = None;
        normal_negative.days_to_primary_crisis_start = None;
        let mut buffer_negative = normal_negative.clone();
        buffer_negative.primary_scenario_id = Some("scenario_a".to_string());
        buffer_negative.scenario_family = Some("systemic_credit_banking_crisis".to_string());
        buffer_negative.days_to_primary_crisis_start = Some(28);
        buffer_negative.regime_20d = ProbabilityTrainingRegime::PreWarningBuffer;
        buffer_negative.regime_60d = ProbabilityTrainingRegime::PreWarningBuffer;
        let mut crisis_negative = normal_negative.clone();
        crisis_negative.primary_scenario_id = Some("scenario_a".to_string());
        crisis_negative.scenario_family = Some("systemic_credit_banking_crisis".to_string());
        crisis_negative.days_to_primary_crisis_start = Some(-5);
        crisis_negative.regime_20d = ProbabilityTrainingRegime::InCrisis;
        crisis_negative.regime_60d = ProbabilityTrainingRegime::InCrisis;
        let mut cooldown_negative = normal_negative.clone();
        cooldown_negative.primary_scenario_id = Some("scenario_a".to_string());
        cooldown_negative.scenario_family = Some("systemic_credit_banking_crisis".to_string());
        cooldown_negative.days_to_primary_crisis_start = Some(-35);
        cooldown_negative.regime_20d = ProbabilityTrainingRegime::PostCrisisCooldown;
        cooldown_negative.regime_60d = ProbabilityTrainingRegime::PostCrisisCooldown;

        assert_eq!(
            negative_sample_weight(
                &normal_negative,
                20,
                ProbabilityTargetLabelMode::ForwardCrisis,
            ),
            1.10
        );
        assert_eq!(
            negative_sample_weight(
                &buffer_negative,
                20,
                ProbabilityTargetLabelMode::ForwardCrisis,
            ),
            0.70
        );
        assert_eq!(
            negative_sample_weight(
                &buffer_negative,
                60,
                ProbabilityTargetLabelMode::ForwardCrisis,
            ),
            0.60
        );
        assert_eq!(
            negative_sample_weight(
                &crisis_negative,
                20,
                ProbabilityTargetLabelMode::ForwardCrisis,
            ),
            1.25
        );
        assert_eq!(
            negative_sample_weight(
                &cooldown_negative,
                20,
                ProbabilityTargetLabelMode::ForwardCrisis,
            ),
            1.45
        );
        let mut protected_negative = normal_negative.clone();
        protected_negative.primary_scenario_id = Some("scenario_protected".to_string());
        protected_negative.scenario_family = Some("mixed_systemic_stress".to_string());
        protected_negative.protected_action_window = true;
        assert_eq!(
            negative_sample_weight(
                &protected_negative,
                20,
                ProbabilityTargetLabelMode::ForwardCrisis,
            ),
            0.55
        );
        assert_eq!(
            negative_sample_weight(
                &protected_negative,
                60,
                ProbabilityTargetLabelMode::ForwardCrisis,
            ),
            0.65
        );
        let mut protected_cooldown_negative = protected_negative.clone();
        protected_cooldown_negative.action_episode_phase = "cooldown".to_string();
        assert_eq!(
            negative_sample_weight(
                &protected_cooldown_negative,
                20,
                ProbabilityTargetLabelMode::ForwardCrisis,
            ),
            1.20
        );
        assert_eq!(
            negative_sample_weight(
                &protected_cooldown_negative,
                60,
                ProbabilityTargetLabelMode::ForwardCrisis,
            ),
            1.35
        );
        assert_eq!(
            negative_sample_weight(
                &buffer_negative,
                20,
                ProbabilityTargetLabelMode::ActionWindow,
            ),
            0.75
        );
        assert_eq!(
            negative_sample_weight(
                &crisis_negative,
                20,
                ProbabilityTargetLabelMode::ActionWindow,
            ),
            1.70
        );
        assert_eq!(
            negative_sample_weight(
                &cooldown_negative,
                20,
                ProbabilityTargetLabelMode::ActionWindow,
            ),
            1.45
        );

        let calibration_rows = vec![
            positive_row.clone(),
            normal_negative.clone(),
            buffer_negative.clone(),
            crisis_negative.clone(),
        ];
        let selection = probability_calibration_selection_rows(
            &calibration_rows,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );
        assert_eq!(selection.rows.len(), 4);
        assert!(!selection.used_full_split_fallback);
        assert_eq!(
            selection
                .rows
                .iter()
                .filter(|row| {
                    row.label_20d > 0
                        || matches!(
                            row.regime_20d,
                            ProbabilityTrainingRegime::Normal
                                | ProbabilityTrainingRegime::PreWarningBuffer
                                | ProbabilityTrainingRegime::InCrisis
                        )
                })
                .count(),
            4
        );
        assert_eq!(
            forward_crisis_regime_sample_weight(20, ProbabilityTrainingRegime::PositiveWindow),
            2.2
        );
    }

    #[test]
    fn formal_main_context_scenarios_include_protected_window_set() {
        let scenario_sets = super::load_formal_dataset_scenario_sets(
            super::DEFAULT_FORMAL_SCENARIO_SET_VERSION,
            super::DEFAULT_FORMAL_LABEL_VERSION,
        )
        .unwrap();
        let positive_ids = scenario_sets
            .positive_scenarios
            .iter()
            .map(|scenario| scenario.scenario_id.as_str())
            .collect::<BTreeSet<_>>();
        let context_ids = scenario_sets
            .context_scenarios
            .iter()
            .map(|scenario| scenario.scenario_id.as_str())
            .collect::<BTreeSet<_>>();

        assert_eq!(positive_ids.len(), 3);
        assert!(positive_ids.contains("us_gfc_2008"));
        assert!(positive_ids.contains("us_covid_liquidity_2020"));
        assert!(positive_ids.contains("us_regional_banks_2023"));

        assert!(context_ids.len() > positive_ids.len());
        assert!(context_ids.contains("us_dotcom_unwind_2000"));
        assert!(context_ids.contains("us_funding_stress_2011"));
        assert!(context_ids.contains("us_rate_shock_2022"));
    }

    #[test]
    fn forward_crisis_pairwise_targets_push_buffer_centroid_above_normal() {
        let make_row =
            |feature_value: f64, regime_20d: ProbabilityTrainingRegime, label_20d: u8| {
                let mut features = BTreeMap::new();
                features.insert("stress".to_string(), feature_value);
                ProbabilityTrainingRow {
                    as_of_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
                    market_scope: "financial_system".to_string(),
                    release_id: None,
                    probability_mode: Some("formal_bundle_v1".to_string()),
                    freshness_status: Some("a".to_string()),
                    time_to_risk_bucket: Some("weeks".to_string()),
                    split_name: Some("train".to_string()),
                    features,
                    primary_scenario_id: Some("scenario".to_string()),
                    scenario_family: Some("systemic_credit_banking_crisis".to_string()),
                    scenario_training_role: None,
                    days_to_primary_crisis_start: Some(10),
                    primary_scenario_supports_5d: false,
                    primary_scenario_supports_20d: true,
                    primary_scenario_supports_60d: true,
                    label_5d: 0,
                    label_20d,
                    label_60d: 0,
                    regime_5d: ProbabilityTrainingRegime::Normal,
                    regime_20d,
                    regime_60d: ProbabilityTrainingRegime::Normal,
                    action_label_5d: 0,
                    action_label_20d: label_20d,
                    action_label_60d: 0,
                    prepare_episode_label: 0,
                    hedge_episode_label: 0,
                    defend_episode_label: 0,
                    primary_action_level: None,
                    action_episode_id: None,
                    action_episode_phase: "outside".to_string(),
                    protected_action_window: false,
                }
            };

        let rows = vec![
            make_row(0.0, ProbabilityTrainingRegime::Normal, 0),
            make_row(0.1, ProbabilityTrainingRegime::Normal, 0),
            make_row(0.8, ProbabilityTrainingRegime::PreWarningBuffer, 0),
            make_row(0.9, ProbabilityTrainingRegime::PreWarningBuffer, 0),
            make_row(1.2, ProbabilityTrainingRegime::PositiveWindow, 1),
            make_row(1.1, ProbabilityTrainingRegime::PositiveWindow, 1),
            make_row(1.0, ProbabilityTrainingRegime::PostCrisisCooldown, 0),
        ];
        let feature_stats = vec![super::build_feature_stat(&rows, "stress")];
        let targets = forward_crisis_regime_pairwise_targets(
            &rows,
            &feature_stats,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
            false,
        );
        assert!(!targets.is_empty());
        assert!(targets.len() >= 5);
        let mut gradients = vec![0.0];
        super::apply_regime_pairwise_gradient(&mut gradients, &[0.0], &targets, 100.0, 20, false);
        assert!(gradients[0] < 0.0);
    }

    #[test]
    fn forward_crisis_sign_gradient_pushes_wrong_direction_coefficients_toward_zero() {
        let feature_names = vec![
            "us_baa_10y_spread_level".to_string(),
            "us_curve_10y2y_level".to_string(),
            "us_stlfsi_level".to_string(),
            "tail_neg__us_curve_10y2y_level__0".to_string(),
            "tail_pos__us_baa_10y_spread_level__2".to_string(),
        ];
        let weights = vec![-0.8, 0.5, -0.4, -0.6, -0.3];
        let mut gradients = vec![0.0; weights.len()];

        super::apply_forward_crisis_sign_gradient(
            &mut gradients,
            &weights,
            &feature_names,
            100.0,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );

        assert!(gradients[0] < 0.0);
        assert!(gradients[1] > 0.0);
        assert!(gradients[2] < 0.0);
        assert_eq!(gradients[3], 0.0);
        assert!(gradients[4] < 0.0);
    }

    #[test]
    fn forward_crisis_sign_projection_clips_wrong_direction_coefficients() {
        let feature_names = vec![
            "us_baa_10y_spread_level".to_string(),
            "us_curve_10y2y_level".to_string(),
            "structural_score".to_string(),
            "us_usdjpy_change_20d".to_string(),
        ];
        let mut weights = vec![-0.8, 0.5, -0.2, -0.7];

        super::project_forward_crisis_sign_constraints(
            &mut weights,
            &feature_names,
            60,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );

        assert_eq!(weights[0], 0.0);
        assert_eq!(weights[1], 0.0);
        assert_eq!(weights[2], 0.0);
        assert_eq!(weights[3], -0.7);
    }

    #[test]
    fn forward_crisis_sign_projection_clips_wrong_direction_monotonic_interactions() {
        let feature_names = vec![
            "interaction__overall_score__us_vix_level".to_string(),
            "interaction__us_baa_10y_spread_level__us_vix_level".to_string(),
            "interaction__external_dimension_score__us_usdjpy_level".to_string(),
        ];
        let mut weights = vec![-0.2, -0.6, -0.4];

        super::project_forward_crisis_sign_constraints(
            &mut weights,
            &feature_names,
            60,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );

        assert_eq!(weights[0], 0.0);
        assert_eq!(weights[1], 0.0);
        assert_eq!(weights[2], 0.0);
    }

    #[test]
    fn forward_crisis_tail_sign_projection_applies_on_20d_only() {
        let feature_names = vec![
            "tail_neg__us_curve_10y2y_level__0".to_string(),
            "tail_pos__us_baa_10y_spread_level__2".to_string(),
        ];
        let mut weights_20d = vec![-0.4, -0.1];
        super::project_forward_crisis_sign_constraints(
            &mut weights_20d,
            &feature_names,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );
        assert_eq!(weights_20d[0], 0.0);
        assert_eq!(weights_20d[1], 0.0);

        let mut weights_60d = vec![-0.4, -0.1];
        super::project_forward_crisis_sign_constraints(
            &mut weights_60d,
            &feature_names,
            60,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );
        assert_eq!(weights_60d[0], -0.4);
        assert_eq!(weights_60d[1], -0.1);
    }

    #[test]
    fn forward_crisis_curve_tail_bound_gradient_pushes_too_negative_weight_up() {
        let feature_names = vec!["tail_neg__us_curve_10y2y_level__0".to_string()];
        let weights = vec![-0.30];
        let mut gradients = vec![0.0; weights.len()];

        super::apply_forward_crisis_coefficient_bound_gradient(
            &mut gradients,
            &weights,
            &feature_names,
            100.0,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );

        assert!(gradients[0] < 0.0);
    }

    #[test]
    fn forward_crisis_rate_shock_family_caps_apply_on_20d_only() {
        let feature_names = vec![
            "family_context__rate_shock__external_dimension_score".to_string(),
            "family_proxy__rate_shock".to_string(),
        ];
        let mut weights_20d = vec![0.32, 0.14];
        super::project_forward_crisis_sign_constraints(
            &mut weights_20d,
            &feature_names,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );
        assert_eq!(weights_20d[0], 0.12);
        assert_eq!(weights_20d[1], 0.06);

        let mut weights_60d = vec![0.32, 0.14];
        super::project_forward_crisis_sign_constraints(
            &mut weights_60d,
            &feature_names,
            60,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );
        assert_eq!(weights_60d[0], 0.32);
        assert_eq!(weights_60d[1], 0.14);
    }

    #[test]
    fn forward_crisis_jpy_carry_caps_apply_on_20d_only() {
        let feature_names = vec![
            "family_context__jpy_carry__external_dimension_score".to_string(),
            "family_proxy__jpy_carry".to_string(),
        ];
        let mut weights_20d = vec![0.24, 0.11];
        super::project_forward_crisis_sign_constraints(
            &mut weights_20d,
            &feature_names,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );
        assert_eq!(weights_20d[0], 0.10);
        assert_eq!(weights_20d[1], 0.06);

        let mut weights_60d = vec![0.24, 0.11];
        super::project_forward_crisis_sign_constraints(
            &mut weights_60d,
            &feature_names,
            60,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );
        assert_eq!(weights_60d[0], 0.24);
        assert_eq!(weights_60d[1], 0.11);
    }

    #[test]
    fn forward_crisis_curve_family_caps_only_apply_when_family_context_exists() {
        let family_feature_names = vec![
            "us_curve_10y2y_level".to_string(),
            "interaction__us_curve_10y2y_level__us_fed_funds_level".to_string(),
            "family_proxy__rate_shock".to_string(),
        ];
        let mut family_weights_20d = vec![-0.90, 0.60, 0.05];
        super::project_forward_crisis_sign_constraints(
            &mut family_weights_20d,
            &family_feature_names,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );
        assert_eq!(family_weights_20d[0], -0.72);
        assert_eq!(family_weights_20d[1], 0.46);
        assert_eq!(family_weights_20d[2], 0.05);

        let mut family_weights_60d = vec![-0.90, 0.60, 0.05];
        super::project_forward_crisis_sign_constraints(
            &mut family_weights_60d,
            &family_feature_names,
            60,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );
        assert_eq!(family_weights_60d[0], -0.90);
        assert_eq!(family_weights_60d[1], 0.60);
        assert_eq!(family_weights_60d[2], 0.05);

        let plain_feature_names = vec![
            "us_curve_10y2y_level".to_string(),
            "interaction__us_curve_10y2y_level__us_fed_funds_level".to_string(),
        ];
        let mut plain_weights_20d = vec![-0.90, 0.60];
        super::project_forward_crisis_sign_constraints(
            &mut plain_weights_20d,
            &plain_feature_names,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );
        assert_eq!(plain_weights_20d[0], -0.90);
        assert_eq!(plain_weights_20d[1], 0.60);
    }

    #[test]
    fn forward_crisis_usdjpy_level_family_cap_only_applies_when_family_context_exists() {
        let family_feature_names = vec![
            "us_usdjpy_level".to_string(),
            "family_proxy__rate_shock".to_string(),
        ];
        let mut family_weights_20d = vec![0.20, 0.05];
        super::project_forward_crisis_sign_constraints(
            &mut family_weights_20d,
            &family_feature_names,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );
        assert_eq!(family_weights_20d[0], 0.30);
        assert_eq!(family_weights_20d[1], 0.05);

        let mut family_weights_60d = vec![0.20, 0.05];
        super::project_forward_crisis_sign_constraints(
            &mut family_weights_60d,
            &family_feature_names,
            60,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );
        assert_eq!(family_weights_60d[0], 0.20);
        assert_eq!(family_weights_60d[1], 0.05);

        let plain_feature_names = vec!["us_usdjpy_level".to_string()];
        let mut plain_weights_20d = vec![0.20];
        super::project_forward_crisis_sign_constraints(
            &mut plain_weights_20d,
            &plain_feature_names,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );
        assert_eq!(plain_weights_20d[0], 0.20);
    }

    #[test]
    fn forward_crisis_usdjpy_interaction_family_cap_only_applies_when_family_context_exists() {
        let family_feature_names = vec![
            "interaction__external_dimension_score__us_usdjpy_level".to_string(),
            "family_proxy__jpy_carry".to_string(),
        ];
        let mut family_weights_20d = vec![0.72, 0.03];
        super::project_forward_crisis_sign_constraints(
            &mut family_weights_20d,
            &family_feature_names,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );
        assert_eq!(family_weights_20d[0], 0.58);
        assert_eq!(family_weights_20d[1], 0.03);

        let mut family_weights_60d = vec![0.72, 0.03];
        super::project_forward_crisis_sign_constraints(
            &mut family_weights_60d,
            &family_feature_names,
            60,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );
        assert_eq!(family_weights_60d[0], 0.72);
        assert_eq!(family_weights_60d[1], 0.03);

        let plain_feature_names =
            vec!["interaction__external_dimension_score__us_usdjpy_level".to_string()];
        let mut plain_weights_20d = vec![0.72];
        super::project_forward_crisis_sign_constraints(
            &mut plain_weights_20d,
            &plain_feature_names,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );
        assert_eq!(plain_weights_20d[0], 0.72);
    }

    #[test]
    fn forward_crisis_rate_shock_family_cap_gradient_pushes_excess_weight_down() {
        let feature_names = vec![
            "family_context__rate_shock__external_dimension_score".to_string(),
            "family_proxy__rate_shock".to_string(),
        ];
        let weights = vec![0.30, 0.12];
        let mut gradients = vec![0.0; weights.len()];

        super::apply_forward_crisis_coefficient_bound_gradient(
            &mut gradients,
            &weights,
            &feature_names,
            100.0,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );

        assert!(gradients[0] > 0.0);
        assert!(gradients[1] > 0.0);
    }

    #[test]
    fn forward_crisis_jpy_carry_family_cap_gradient_pushes_excess_weight_down() {
        let feature_names = vec![
            "family_context__jpy_carry__external_dimension_score".to_string(),
            "family_proxy__jpy_carry".to_string(),
        ];
        let weights = vec![0.22, 0.09];
        let mut gradients = vec![0.0; weights.len()];

        super::apply_forward_crisis_coefficient_bound_gradient(
            &mut gradients,
            &weights,
            &feature_names,
            100.0,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );

        assert!(gradients[0] > 0.0);
        assert!(gradients[1] > 0.0);
    }

    #[test]
    fn forward_crisis_monotonic_interaction_sign_gradient_pushes_wrong_direction_up() {
        let feature_names = vec![
            "interaction__overall_score__us_vix_level".to_string(),
            "interaction__us_baa_10y_spread_level__us_vix_level".to_string(),
        ];
        let weights = vec![-0.20, -0.60];
        let mut gradients = vec![0.0; weights.len()];

        super::apply_forward_crisis_sign_gradient(
            &mut gradients,
            &weights,
            &feature_names,
            100.0,
            60,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );

        assert!(gradients[0] < 0.0);
        assert!(gradients[1] < 0.0);
    }

    #[test]
    fn forward_crisis_curve_family_cap_gradient_only_activates_for_family_context_sets() {
        let family_feature_names = vec![
            "us_curve_10y2y_level".to_string(),
            "interaction__us_curve_10y2y_level__us_fed_funds_level".to_string(),
            "family_proxy__rate_shock".to_string(),
        ];
        let family_weights = vec![-0.90, 0.60, 0.05];
        let mut family_gradients = vec![0.0; family_weights.len()];

        super::apply_forward_crisis_coefficient_bound_gradient(
            &mut family_gradients,
            &family_weights,
            &family_feature_names,
            100.0,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );

        assert!(family_gradients[0] < 0.0);
        assert!(family_gradients[1] > 0.0);
        assert_eq!(family_gradients[2], 0.0);

        let plain_feature_names = vec![
            "us_curve_10y2y_level".to_string(),
            "interaction__us_curve_10y2y_level__us_fed_funds_level".to_string(),
        ];
        let plain_weights = vec![-0.90, 0.60];
        let mut plain_gradients = vec![0.0; plain_weights.len()];

        super::apply_forward_crisis_coefficient_bound_gradient(
            &mut plain_gradients,
            &plain_weights,
            &plain_feature_names,
            100.0,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );

        assert_eq!(plain_gradients[0], 0.0);
        assert_eq!(plain_gradients[1], 0.0);
    }

    #[test]
    fn forward_crisis_usdjpy_level_family_cap_gradient_only_activates_for_family_context_sets() {
        let family_feature_names = vec![
            "us_usdjpy_level".to_string(),
            "family_proxy__rate_shock".to_string(),
        ];
        let family_weights = vec![0.48, 0.05];
        let mut family_gradients = vec![0.0; family_weights.len()];

        super::apply_forward_crisis_coefficient_bound_gradient(
            &mut family_gradients,
            &family_weights,
            &family_feature_names,
            100.0,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );

        assert!(family_gradients[0] > 0.0);
        assert_eq!(family_gradients[1], 0.0);

        let plain_feature_names = vec!["us_usdjpy_level".to_string()];
        let plain_weights = vec![0.38];
        let mut plain_gradients = vec![0.0; plain_weights.len()];

        super::apply_forward_crisis_coefficient_bound_gradient(
            &mut plain_gradients,
            &plain_weights,
            &plain_feature_names,
            100.0,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );

        assert_eq!(plain_gradients[0], 0.0);
    }

    #[test]
    fn forward_crisis_usdjpy_interaction_family_cap_gradient_only_activates_for_family_context_sets(
    ) {
        let family_feature_names = vec![
            "interaction__external_dimension_score__us_usdjpy_level".to_string(),
            "family_proxy__jpy_carry".to_string(),
        ];
        let family_weights = vec![0.72, 0.03];
        let mut family_gradients = vec![0.0; family_weights.len()];

        super::apply_forward_crisis_coefficient_bound_gradient(
            &mut family_gradients,
            &family_weights,
            &family_feature_names,
            100.0,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );

        assert!(family_gradients[0] > 0.0);
        assert_eq!(family_gradients[1], 0.0);

        let plain_feature_names =
            vec!["interaction__external_dimension_score__us_usdjpy_level".to_string()];
        let plain_weights = vec![0.72];
        let mut plain_gradients = vec![0.0; plain_weights.len()];

        super::apply_forward_crisis_coefficient_bound_gradient(
            &mut plain_gradients,
            &plain_weights,
            &plain_feature_names,
            100.0,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );

        assert_eq!(plain_gradients[0], 0.0);
    }

    #[test]
    fn forward_crisis_training_target_softens_buffer_and_cooldown_negatives() {
        let build_row = |regime_20d: ProbabilityTrainingRegime,
                         regime_60d: ProbabilityTrainingRegime,
                         label_20d: u8,
                         label_60d: u8| ProbabilityTrainingRow {
            as_of_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
            market_scope: "financial_system".to_string(),
            release_id: None,
            probability_mode: Some("formal_bundle_v1".to_string()),
            freshness_status: Some("a".to_string()),
            time_to_risk_bucket: Some("weeks".to_string()),
            split_name: Some("train".to_string()),
            features: BTreeMap::new(),
            primary_scenario_id: Some("scenario".to_string()),
            scenario_family: Some("systemic_credit_banking_crisis".to_string()),
            scenario_training_role: None,
            days_to_primary_crisis_start: Some(25),
            primary_scenario_supports_5d: false,
            primary_scenario_supports_20d: true,
            primary_scenario_supports_60d: true,
            label_5d: 0,
            label_20d,
            label_60d,
            regime_5d: ProbabilityTrainingRegime::Normal,
            regime_20d,
            regime_60d,
            action_label_5d: 0,
            action_label_20d: 0,
            action_label_60d: 0,
            prepare_episode_label: 0,
            hedge_episode_label: 0,
            defend_episode_label: 0,
            primary_action_level: None,
            action_episode_id: None,
            action_episode_phase: "outside".to_string(),
            protected_action_window: false,
        };

        let buffer_row = build_row(
            ProbabilityTrainingRegime::PreWarningBuffer,
            ProbabilityTrainingRegime::PreWarningBuffer,
            0,
            0,
        );
        let cooldown_row = build_row(
            ProbabilityTrainingRegime::PostCrisisCooldown,
            ProbabilityTrainingRegime::PostCrisisCooldown,
            0,
            0,
        );
        let positive_row = build_row(
            ProbabilityTrainingRegime::PositiveWindow,
            ProbabilityTrainingRegime::PositiveWindow,
            1,
            1,
        );

        assert_eq!(
            super::probability_training_target_label(
                &buffer_row,
                20,
                ProbabilityTargetLabelMode::ForwardCrisis
            ),
            0.18
        );
        assert_eq!(
            super::probability_training_target_label(
                &buffer_row,
                60,
                ProbabilityTargetLabelMode::ForwardCrisis
            ),
            0.26
        );
        assert_eq!(
            super::probability_training_target_label(
                &cooldown_row,
                20,
                ProbabilityTargetLabelMode::ForwardCrisis
            ),
            0.01
        );
        assert_eq!(
            super::probability_training_target_label(
                &cooldown_row,
                60,
                ProbabilityTargetLabelMode::ForwardCrisis
            ),
            0.02
        );
        assert_eq!(
            super::probability_training_target_label(
                &positive_row,
                20,
                ProbabilityTargetLabelMode::ForwardCrisis
            ),
            1.0
        );
    }

    #[test]
    fn forward_crisis_60d_prepare_buffer_uses_episode_native_objective() {
        let build_row = |prepare_episode_label: u8,
                         scenario_training_role: Option<&str>,
                         scenario_family: &str,
                         supports_60d: bool,
                         lead_days: Option<i64>,
                         protected_action_window: bool| {
            ProbabilityTrainingRow {
                as_of_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
                market_scope: "financial_system".to_string(),
                release_id: None,
                probability_mode: Some("formal_bundle_v1".to_string()),
                freshness_status: Some("a".to_string()),
                time_to_risk_bucket: Some("weeks".to_string()),
                split_name: Some("train".to_string()),
                features: BTreeMap::new(),
                primary_scenario_id: Some("scenario".to_string()),
                scenario_family: Some(scenario_family.to_string()),
                scenario_training_role: scenario_training_role.map(str::to_string),
                days_to_primary_crisis_start: lead_days,
                primary_scenario_supports_5d: false,
                primary_scenario_supports_20d: true,
                primary_scenario_supports_60d: supports_60d,
                label_5d: 0,
                label_20d: 0,
                label_60d: 0,
                regime_5d: ProbabilityTrainingRegime::Normal,
                regime_20d: ProbabilityTrainingRegime::Normal,
                regime_60d: ProbabilityTrainingRegime::PreWarningBuffer,
                action_label_5d: 0,
                action_label_20d: 0,
                action_label_60d: prepare_episode_label,
                prepare_episode_label,
                hedge_episode_label: 0,
                defend_episode_label: 0,
                primary_action_level: (prepare_episode_label > 0).then_some("prepare".to_string()),
                action_episode_id: (prepare_episode_label > 0)
                    .then_some("scenario:prepare".to_string()),
                action_episode_phase: if prepare_episode_label > 0 {
                    "primary".to_string()
                } else {
                    "outside".to_string()
                },
                protected_action_window,
            }
        };

        let mandatory_prepare = build_row(
            1,
            Some("mandatory"),
            "systemic_credit_banking_crisis",
            true,
            Some(75),
            false,
        );
        assert_eq!(
            super::probability_training_target_label(
                &mandatory_prepare,
                60,
                ProbabilityTargetLabelMode::ForwardCrisis
            ),
            0.64
        );
        assert_eq!(
            negative_sample_weight(
                &mandatory_prepare,
                60,
                ProbabilityTargetLabelMode::ForwardCrisis
            ),
            1.35
        );

        let protected_extension = build_row(
            1,
            Some("extension_only"),
            "mixed_systemic_stress",
            true,
            Some(82),
            true,
        );
        assert_eq!(
            super::probability_training_target_label(
                &protected_extension,
                60,
                ProbabilityTargetLabelMode::ForwardCrisis
            ),
            0.58
        );
        assert_eq!(
            negative_sample_weight(
                &protected_extension,
                60,
                ProbabilityTargetLabelMode::ForwardCrisis
            ),
            1.10
        );

        let outside_prepare = build_row(
            0,
            Some("mandatory"),
            "systemic_credit_banking_crisis",
            true,
            Some(75),
            false,
        );
        assert_eq!(
            super::probability_training_target_label(
                &outside_prepare,
                60,
                ProbabilityTargetLabelMode::ForwardCrisis
            ),
            0.26
        );

        let acute_prepare = build_row(
            1,
            Some("mandatory"),
            "acute_market_liquidity_crash",
            true,
            Some(75),
            false,
        );
        assert_eq!(
            super::probability_training_target_label(
                &acute_prepare,
                60,
                ProbabilityTargetLabelMode::ForwardCrisis
            ),
            0.26
        );

        let unsupported_prepare = build_row(
            1,
            Some("mandatory"),
            "systemic_credit_banking_crisis",
            false,
            Some(75),
            false,
        );
        assert_eq!(
            super::probability_training_target_label(
                &unsupported_prepare,
                60,
                ProbabilityTargetLabelMode::ForwardCrisis
            ),
            0.26
        );
    }

    #[test]
    fn positive_sample_action_weight_prefers_early_role_aligned_systemic_samples_for_60d() {
        let aligned = ProbabilityTrainingRow {
            as_of_date: NaiveDate::from_ymd_opt(2007, 6, 5).unwrap(),
            market_scope: "financial_system".to_string(),
            release_id: None,
            probability_mode: Some("formal_bundle_v1".to_string()),
            freshness_status: Some("a".to_string()),
            time_to_risk_bucket: Some("us_gfc_2008".to_string()),
            split_name: Some("train".to_string()),
            features: BTreeMap::new(),
            primary_scenario_id: Some("us_gfc_2008".to_string()),
            scenario_family: Some("systemic_credit_banking_crisis".to_string()),
            scenario_training_role: Some("mandatory".to_string()),
            days_to_primary_crisis_start: Some(57),
            primary_scenario_supports_5d: false,
            primary_scenario_supports_20d: true,
            primary_scenario_supports_60d: true,
            label_5d: 0,
            label_20d: 0,
            label_60d: 1,
            regime_5d: ProbabilityTrainingRegime::Normal,
            regime_20d: ProbabilityTrainingRegime::PreWarningBuffer,
            regime_60d: ProbabilityTrainingRegime::PositiveWindow,
            action_label_5d: 0,
            action_label_20d: 1,
            action_label_60d: 1,
            prepare_episode_label: 1,
            hedge_episode_label: 1,
            defend_episode_label: 0,
            primary_action_level: Some("hedge".to_string()),
            action_episode_id: Some("us_gfc_2008:hedge".to_string()),
            action_episode_phase: "primary".to_string(),
            protected_action_window: false,
        };
        let misaligned = ProbabilityTrainingRow {
            as_of_date: NaiveDate::from_ymd_opt(2020, 2, 20).unwrap(),
            market_scope: "financial_system".to_string(),
            release_id: None,
            probability_mode: Some("formal_bundle_v1".to_string()),
            freshness_status: Some("a".to_string()),
            time_to_risk_bucket: Some("us_covid_liquidity_2020".to_string()),
            split_name: Some("train".to_string()),
            features: BTreeMap::new(),
            primary_scenario_id: Some("us_covid_liquidity_2020".to_string()),
            scenario_family: Some("acute_market_liquidity_crash".to_string()),
            scenario_training_role: Some("mandatory".to_string()),
            days_to_primary_crisis_start: Some(4),
            primary_scenario_supports_5d: true,
            primary_scenario_supports_20d: true,
            primary_scenario_supports_60d: false,
            label_5d: 1,
            label_20d: 1,
            label_60d: 1,
            regime_5d: ProbabilityTrainingRegime::PositiveWindow,
            regime_20d: ProbabilityTrainingRegime::PositiveWindow,
            regime_60d: ProbabilityTrainingRegime::PositiveWindow,
            action_label_5d: 1,
            action_label_20d: 1,
            action_label_60d: 0,
            prepare_episode_label: 0,
            hedge_episode_label: 1,
            defend_episode_label: 1,
            primary_action_level: Some("defend".to_string()),
            action_episode_id: Some("us_covid_liquidity_2020:defend".to_string()),
            action_episode_phase: "primary".to_string(),
            protected_action_window: false,
        };

        assert!(
            positive_sample_action_weight(&aligned, 60)
                > positive_sample_action_weight(&misaligned, 60)
        );
    }

    #[test]
    fn forward_crisis_positive_weight_boosts_extension_role_on_supported_horizon() {
        let mandatory = ProbabilityTrainingRow {
            as_of_date: NaiveDate::from_ymd_opt(2007, 6, 5).unwrap(),
            market_scope: "financial_system".to_string(),
            release_id: None,
            probability_mode: Some("formal_bundle_v1".to_string()),
            freshness_status: Some("a".to_string()),
            time_to_risk_bucket: Some("us_gfc_2008".to_string()),
            split_name: Some("train".to_string()),
            features: BTreeMap::new(),
            primary_scenario_id: Some("us_gfc_2008".to_string()),
            scenario_family: Some("systemic_credit_banking_crisis".to_string()),
            scenario_training_role: Some("mandatory".to_string()),
            days_to_primary_crisis_start: Some(48),
            primary_scenario_supports_5d: false,
            primary_scenario_supports_20d: true,
            primary_scenario_supports_60d: true,
            label_5d: 0,
            label_20d: 0,
            label_60d: 1,
            regime_5d: ProbabilityTrainingRegime::Normal,
            regime_20d: ProbabilityTrainingRegime::PreWarningBuffer,
            regime_60d: ProbabilityTrainingRegime::PositiveWindow,
            action_label_5d: 0,
            action_label_20d: 1,
            action_label_60d: 1,
            prepare_episode_label: 1,
            hedge_episode_label: 1,
            defend_episode_label: 0,
            primary_action_level: Some("hedge".to_string()),
            action_episode_id: Some("us_gfc_2008:hedge".to_string()),
            action_episode_phase: "primary".to_string(),
            protected_action_window: false,
        };
        let extension = ProbabilityTrainingRow {
            as_of_date: NaiveDate::from_ymd_opt(2011, 6, 20).unwrap(),
            market_scope: "financial_system".to_string(),
            release_id: None,
            probability_mode: Some("formal_bundle_v1".to_string()),
            freshness_status: Some("a".to_string()),
            time_to_risk_bucket: Some("us_funding_stress_2011".to_string()),
            split_name: Some("train".to_string()),
            features: BTreeMap::new(),
            primary_scenario_id: Some("us_funding_stress_2011".to_string()),
            scenario_family: Some("mixed_systemic_stress".to_string()),
            scenario_training_role: Some("extension_only".to_string()),
            days_to_primary_crisis_start: Some(39),
            primary_scenario_supports_5d: false,
            primary_scenario_supports_20d: true,
            primary_scenario_supports_60d: true,
            label_5d: 0,
            label_20d: 0,
            label_60d: 1,
            regime_5d: ProbabilityTrainingRegime::Normal,
            regime_20d: ProbabilityTrainingRegime::PreWarningBuffer,
            regime_60d: ProbabilityTrainingRegime::PositiveWindow,
            action_label_5d: 0,
            action_label_20d: 1,
            action_label_60d: 1,
            prepare_episode_label: 1,
            hedge_episode_label: 1,
            defend_episode_label: 0,
            primary_action_level: Some("hedge".to_string()),
            action_episode_id: Some("us_funding_stress_2011:hedge".to_string()),
            action_episode_phase: "primary".to_string(),
            protected_action_window: true,
        };

        assert!(
            super::forward_crisis_positive_sample_weight(&extension, 60)
                > super::forward_crisis_positive_sample_weight(&mandatory, 60)
        );
    }

    #[test]
    fn render_dataset_csv_includes_scenario_role_columns() {
        let mut row = forward_crisis_row(
            NaiveDate::from_ymd_opt(2020, 2, 20).unwrap(),
            1,
            ProbabilityTrainingRegime::PositiveWindow,
        );
        row.scenario_training_role = Some("mandatory".to_string());
        row.features.insert("stress".to_string(), 0.42);

        let csv = super::commands::snapshot::render_dataset_csv(&[row], &[String::from("stress")]);
        let mut lines = csv.lines();
        let header = lines.next().unwrap_or_default();
        let first_row = lines.next().unwrap_or_default();

        assert!(header.contains("primary_scenario_id"));
        assert!(header.contains("scenario_family"));
        assert!(header.contains("scenario_training_role"));
        assert!(first_row.contains(",scenario_a,systemic_credit_banking_crisis,mandatory,"));
    }

    #[test]
    fn render_formal_dataset_slice_csv_includes_feature_columns() {
        let mut row = FormalDatasetRowRecord {
            dataset_key: "dataset".to_string(),
            split_name: "evaluation".to_string(),
            entity_id: "us".to_string(),
            market_scope: "financial_system".to_string(),
            as_of_date: NaiveDate::from_ymd_opt(2023, 2, 20).unwrap(),
            point_in_time_mode: "best_effort".to_string(),
            latest_visible_at: None,
            coverage_score: 0.92,
            core_feature_coverage: 0.95,
            trigger_feature_coverage: 0.88,
            external_feature_coverage: 0.84,
            sample_quality_grade: "a".to_string(),
            primary_scenario_id: Some("us_regional_banks_2023".to_string()),
            scenario_family: Some("systemic_credit_banking_crisis".to_string()),
            scenario_training_role: Some("mandatory".to_string()),
            label_5d: 0,
            label_20d: 1,
            label_60d: 1,
            regime_5d: "normal".to_string(),
            regime_20d: "pre_warning_buffer".to_string(),
            regime_60d: "positive_window".to_string(),
            action_label_5d: 0,
            action_label_20d: 1,
            action_label_60d: 1,
            prepare_episode_label: 1,
            hedge_episode_label: 1,
            defend_episode_label: 0,
            primary_action_level: Some("hedge".to_string()),
            action_episode_id: Some("us_regional_banks_2023:hedge".to_string()),
            action_episode_phase: "primary".to_string(),
            protected_action_window: false,
            features: BTreeMap::new(),
            created_at: Utc::now(),
        };
        row.features.insert("feature_a".to_string(), 0.42);

        let csv = render_formal_dataset_slice_csv(&[row], &[String::from("feature_a")]);
        let mut lines = csv.lines();
        let header = lines.next().unwrap_or_default();
        let first_row = lines.next().unwrap_or_default();

        assert!(header.contains("primary_scenario_id"));
        assert!(header.contains("feature_a"));
        assert!(first_row.contains("us_regional_banks_2023"));
        assert!(first_row.ends_with(",0.420000"));
    }

    #[test]
    fn actionability_summary_distinguishes_advance_late_and_missed_scenarios() {
        let build_row =
            |scenario_id: &str, as_of_date: (i32, u32, u32), lead_days: i64, predicted: bool| {
                let action_label = 1_u8;
                let mut features = BTreeMap::new();
                if predicted {
                    features.insert("predicted".to_string(), 1.0);
                }
                ProbabilityTrainingRow {
                    as_of_date: NaiveDate::from_ymd_opt(as_of_date.0, as_of_date.1, as_of_date.2)
                        .unwrap(),
                    market_scope: "financial_system".to_string(),
                    release_id: None,
                    probability_mode: Some("formal_bundle_v1".to_string()),
                    freshness_status: Some("a".to_string()),
                    time_to_risk_bucket: Some("weeks".to_string()),
                    split_name: Some("evaluation".to_string()),
                    features,
                    primary_scenario_id: Some(scenario_id.to_string()),
                    scenario_family: Some("systemic_credit_banking_crisis".to_string()),
                    scenario_training_role: None,
                    days_to_primary_crisis_start: Some(lead_days),
                    primary_scenario_supports_5d: true,
                    primary_scenario_supports_20d: true,
                    primary_scenario_supports_60d: true,
                    label_5d: 0,
                    label_20d: 0,
                    label_60d: 0,
                    regime_5d: ProbabilityTrainingRegime::Normal,
                    regime_20d: if lead_days > 0 {
                        ProbabilityTrainingRegime::PositiveWindow
                    } else {
                        ProbabilityTrainingRegime::InCrisis
                    },
                    regime_60d: ProbabilityTrainingRegime::Normal,
                    action_label_5d: 0,
                    action_label_20d: action_label,
                    action_label_60d: 0,
                    prepare_episode_label: 0,
                    hedge_episode_label: u8::from(lead_days > 0),
                    defend_episode_label: 0,
                    primary_action_level: (lead_days > 0).then_some("hedge".to_string()),
                    action_episode_id: Some(format!("{scenario_id}:hedge")),
                    action_episode_phase: if lead_days > 0 {
                        "primary".to_string()
                    } else {
                        "late_validation".to_string()
                    },
                    protected_action_window: false,
                }
            };

        let false_positive_row = ProbabilityTrainingRow {
            as_of_date: NaiveDate::from_ymd_opt(2024, 1, 10).unwrap(),
            market_scope: "financial_system".to_string(),
            release_id: None,
            probability_mode: Some("formal_bundle_v1".to_string()),
            freshness_status: Some("a".to_string()),
            time_to_risk_bucket: Some("normal".to_string()),
            split_name: Some("evaluation".to_string()),
            features: BTreeMap::new(),
            primary_scenario_id: None,
            scenario_family: None,
            scenario_training_role: None,
            days_to_primary_crisis_start: None,
            primary_scenario_supports_5d: false,
            primary_scenario_supports_20d: false,
            primary_scenario_supports_60d: false,
            label_5d: 0,
            label_20d: 0,
            label_60d: 0,
            regime_5d: ProbabilityTrainingRegime::Normal,
            regime_20d: ProbabilityTrainingRegime::Normal,
            regime_60d: ProbabilityTrainingRegime::Normal,
            action_label_5d: 0,
            action_label_20d: 0,
            action_label_60d: 0,
            prepare_episode_label: 0,
            hedge_episode_label: 0,
            defend_episode_label: 0,
            primary_action_level: None,
            action_episode_id: None,
            action_episode_phase: "outside".to_string(),
            protected_action_window: false,
        };

        let rows = vec![
            build_row("scenario_a", (2007, 8, 20), 10, true),
            build_row("scenario_a", (2007, 9, 5), -2, true),
            build_row("scenario_b", (2020, 2, 20), 8, false),
            build_row("scenario_b", (2020, 3, 18), -3, true),
            build_row("scenario_c", (2011, 7, 20), 6, false),
            build_row("scenario_c", (2011, 8, 10), -1, false),
            false_positive_row,
        ];
        let probabilities = vec![0.82, 0.61, 0.12, 0.42, 0.18, 0.07, 0.77];

        let summary = evaluate_actionability_summary(&probabilities, &rows, 20, 0.3);

        assert_eq!(summary.predicted_positive_count, 4);
        assert_eq!(summary.actual_positive_count, 3);
        assert_eq!(summary.pre_start_positive_count, 3);
        assert_eq!(summary.post_start_positive_count, 3);
        assert_eq!(summary.pre_start_hit_count, 1);
        assert_eq!(summary.post_start_hit_count, 2);
        assert_eq!(summary.false_positive_count, 1);
        assert_eq!(summary.scenario_count, 3);
        assert_eq!(summary.advance_warning_scenario_count, 1);
        assert_eq!(summary.late_confirmation_scenario_count, 1);
        assert_eq!(summary.missed_scenario_count, 1);
        assert_eq!(summary.precision_at_threshold, Some(0.75));
        assert_eq!(
            summary.pre_start_recall_at_threshold,
            Some(round3(1.0 / 3.0))
        );
        assert_eq!(
            summary.post_start_recall_at_threshold,
            Some(round3(2.0 / 3.0))
        );
        assert_eq!(summary.advance_warning_rate, Some(round3(1.0 / 3.0)));
        assert_eq!(summary.late_confirmation_rate, Some(round3(1.0 / 3.0)));
        assert_eq!(summary.missed_rate, Some(round3(1.0 / 3.0)));
    }

    #[test]
    fn actionability_threshold_selection_avoids_zero_hit_fixed_cutoff() {
        let build_row = |scenario_id: &str, lead_days: i64| ProbabilityTrainingRow {
            as_of_date: NaiveDate::from_ymd_opt(2020, 3, 1).unwrap(),
            market_scope: "financial_system".to_string(),
            release_id: None,
            probability_mode: Some("formal_bundle_v1".to_string()),
            freshness_status: Some("a".to_string()),
            time_to_risk_bucket: Some("evaluation".to_string()),
            split_name: Some("calibration".to_string()),
            features: BTreeMap::new(),
            primary_scenario_id: Some(scenario_id.to_string()),
            scenario_family: Some("systemic_credit_banking_crisis".to_string()),
            scenario_training_role: None,
            days_to_primary_crisis_start: Some(lead_days),
            primary_scenario_supports_5d: true,
            primary_scenario_supports_20d: true,
            primary_scenario_supports_60d: true,
            label_5d: 0,
            label_20d: 0,
            label_60d: 0,
            regime_5d: ProbabilityTrainingRegime::Normal,
            regime_20d: ProbabilityTrainingRegime::PreWarningBuffer,
            regime_60d: ProbabilityTrainingRegime::Normal,
            action_label_5d: 0,
            action_label_20d: 1,
            action_label_60d: 0,
            prepare_episode_label: 0,
            hedge_episode_label: u8::from(lead_days > 0),
            defend_episode_label: 0,
            primary_action_level: (lead_days > 0).then_some("hedge".to_string()),
            action_episode_id: Some(format!("{scenario_id}:hedge")),
            action_episode_phase: if lead_days > 0 {
                "primary".to_string()
            } else {
                "late_validation".to_string()
            },
            protected_action_window: false,
        };
        let rows = vec![
            build_row("scenario_a", 8),
            build_row("scenario_a", -2),
            build_row("scenario_b", 10),
            build_row("scenario_b", -1),
        ];
        let probabilities = vec![0.24, 0.18, 0.22, 0.07];

        let threshold = select_actionability_decision_threshold(&probabilities, &rows, 20);
        let summary = evaluate_actionability_summary(&probabilities, &rows, 20, threshold);

        assert!(threshold < 0.3);
        assert!(summary.predicted_positive_count > 0);
        assert!(
            summary.advance_warning_scenario_count + summary.late_confirmation_scenario_count > 0
        );
    }

    #[test]
    fn actionability_threshold_selection_raises_cutoff_when_low_threshold_is_overbroad() {
        let build_positive_row = |scenario_id: &str, lead_days: i64| ProbabilityTrainingRow {
            as_of_date: NaiveDate::from_ymd_opt(2020, 3, 1).unwrap(),
            market_scope: "financial_system".to_string(),
            release_id: None,
            probability_mode: Some("formal_bundle_v1".to_string()),
            freshness_status: Some("a".to_string()),
            time_to_risk_bucket: Some("calibration".to_string()),
            split_name: Some("calibration".to_string()),
            features: BTreeMap::new(),
            primary_scenario_id: Some(scenario_id.to_string()),
            scenario_family: Some("systemic_credit_banking_crisis".to_string()),
            scenario_training_role: None,
            days_to_primary_crisis_start: Some(lead_days),
            primary_scenario_supports_5d: true,
            primary_scenario_supports_20d: true,
            primary_scenario_supports_60d: true,
            label_5d: 0,
            label_20d: 0,
            label_60d: 0,
            regime_5d: ProbabilityTrainingRegime::Normal,
            regime_20d: ProbabilityTrainingRegime::PreWarningBuffer,
            regime_60d: ProbabilityTrainingRegime::Normal,
            action_label_5d: 0,
            action_label_20d: 1,
            action_label_60d: 0,
            prepare_episode_label: 0,
            hedge_episode_label: u8::from(lead_days > 0),
            defend_episode_label: 0,
            primary_action_level: (lead_days > 0).then_some("hedge".to_string()),
            action_episode_id: Some(format!("{scenario_id}:hedge")),
            action_episode_phase: if lead_days > 0 {
                "primary".to_string()
            } else {
                "late_validation".to_string()
            },
            protected_action_window: false,
        };
        let build_false_positive_row = |day: u32| ProbabilityTrainingRow {
            as_of_date: NaiveDate::from_ymd_opt(2020, 4, day).unwrap(),
            market_scope: "financial_system".to_string(),
            release_id: None,
            probability_mode: Some("formal_bundle_v1".to_string()),
            freshness_status: Some("a".to_string()),
            time_to_risk_bucket: Some("normal".to_string()),
            split_name: Some("calibration".to_string()),
            features: BTreeMap::new(),
            primary_scenario_id: None,
            scenario_family: None,
            scenario_training_role: None,
            days_to_primary_crisis_start: None,
            primary_scenario_supports_5d: false,
            primary_scenario_supports_20d: false,
            primary_scenario_supports_60d: false,
            label_5d: 0,
            label_20d: 0,
            label_60d: 0,
            regime_5d: ProbabilityTrainingRegime::Normal,
            regime_20d: ProbabilityTrainingRegime::Normal,
            regime_60d: ProbabilityTrainingRegime::Normal,
            action_label_5d: 0,
            action_label_20d: 0,
            action_label_60d: 0,
            prepare_episode_label: 0,
            hedge_episode_label: 0,
            defend_episode_label: 0,
            primary_action_level: None,
            action_episode_id: None,
            action_episode_phase: "outside".to_string(),
            protected_action_window: false,
        };

        let mut rows = vec![
            build_positive_row("scenario_a", 8),
            build_positive_row("scenario_b", 10),
        ];
        rows.extend((1..=20).map(build_false_positive_row));

        let mut probabilities = vec![0.82, 0.18];
        probabilities.extend(std::iter::repeat_n(0.18, 20));

        let threshold = select_actionability_decision_threshold(&probabilities, &rows, 20);
        let summary = evaluate_actionability_summary(&probabilities, &rows, 20, threshold);

        assert!(threshold > 0.18);
        assert_eq!(summary.false_positive_count, 0);
        assert_eq!(summary.advance_warning_scenario_count, 1);
    }

    #[test]
    fn actionability_bundle_quality_gate_rejects_overbroad_low_precision_levels() {
        let bundle = ActionabilityBundle {
            model_version: "actionability_bundle_test".to_string(),
            calibration_version: "actionability_platt_test".to_string(),
            fusion_policy_version: "fusion_policy_test".to_string(),
            note: "test".to_string(),
            levels: vec![ActionabilityLevelBundle {
                level: ActionabilityLevel::Prepare,
                proxy_horizon_days: 60,
                target_label_mode: "action_window".to_string(),
                decision_threshold: 0.05,
                raw_model: LogisticProbabilityModel {
                    intercept: 0.0,
                    feature_transform: PROBABILITY_FEATURE_TRANSFORM_IDENTITY_V1.to_string(),
                    feature_stats: Vec::new(),
                    coefficients: Vec::new(),
                },
                calibration: None,
                evaluation: HorizonEvaluationSummary {
                    sample_count: 2269,
                    positive_rate: 0.033,
                    brier_score: 0.0,
                    log_loss: 0.0,
                    ece: 0.0,
                    precision_at_30pct: None,
                    recall_at_30pct: None,
                    regime_separation: None,
                    actionability: Some(ActionabilityEvaluationSummary {
                        threshold: 0.05,
                        predicted_positive_count: 1751,
                        actual_positive_count: 77,
                        advance_warning_scenario_count: 1,
                        precision_at_threshold: Some(0.038),
                        ..Default::default()
                    }),
                },
            }],
        };

        let regressions = actionability_bundle_quality_regressions(&bundle);

        assert!(!regressions.is_empty());
        assert!(regressions
            .iter()
            .any(|item| item.contains("precision") || item.contains("predicted positives")));
    }

    #[test]
    fn actionability_calibration_strategy_rejects_inverting_fit() {
        let build_row = |scenario_id: &str, lead_days: i64| ProbabilityTrainingRow {
            as_of_date: NaiveDate::from_ymd_opt(2020, 3, 1).unwrap(),
            market_scope: "financial_system".to_string(),
            release_id: None,
            probability_mode: Some("formal_bundle_v1".to_string()),
            freshness_status: Some("a".to_string()),
            time_to_risk_bucket: Some("calibration".to_string()),
            split_name: Some("calibration".to_string()),
            features: BTreeMap::new(),
            primary_scenario_id: Some(scenario_id.to_string()),
            scenario_family: Some("systemic_credit_banking_crisis".to_string()),
            scenario_training_role: None,
            days_to_primary_crisis_start: Some(lead_days),
            primary_scenario_supports_5d: true,
            primary_scenario_supports_20d: true,
            primary_scenario_supports_60d: true,
            label_5d: 0,
            label_20d: 0,
            label_60d: 0,
            regime_5d: ProbabilityTrainingRegime::Normal,
            regime_20d: ProbabilityTrainingRegime::PreWarningBuffer,
            regime_60d: ProbabilityTrainingRegime::Normal,
            action_label_5d: 0,
            action_label_20d: 1,
            action_label_60d: 0,
            prepare_episode_label: 0,
            hedge_episode_label: u8::from(lead_days > 0),
            defend_episode_label: 0,
            primary_action_level: (lead_days > 0).then_some("hedge".to_string()),
            action_episode_id: Some(format!("{scenario_id}:hedge")),
            action_episode_phase: if lead_days > 0 {
                "primary".to_string()
            } else {
                "late_validation".to_string()
            },
            protected_action_window: false,
        };
        let rows = vec![
            build_row("scenario_a", 8),
            build_row("scenario_a", -2),
            build_row("scenario_b", 10),
            build_row("scenario_b", -1),
        ];
        let raw_probabilities = vec![0.31, 0.28, 0.27, 0.24];
        let calibration_candidate = PlattCalibrationArtifact {
            alpha: -1.2,
            beta: -3.5,
            min_input: 0.24,
            max_input: 0.31,
        };

        let (calibration, evaluation_probabilities, threshold) =
            select_actionability_calibration_strategy(
                &raw_probabilities,
                &rows,
                &raw_probabilities,
                20,
                calibration_candidate,
            );

        assert!(calibration.is_none());
        assert_eq!(evaluation_probabilities, raw_probabilities);
        assert!(threshold >= 0.24);
    }

    #[test]
    fn probability_calibration_strategy_rejects_inverting_fit() {
        let raw_probabilities = vec![0.82, 0.71, 0.24, 0.11];
        let labels = vec![1.0, 1.0, 0.0, 0.0];
        let calibration_rows = vec![
            forward_crisis_row(
                NaiveDate::from_ymd_opt(2020, 1, 5).unwrap(),
                1,
                ProbabilityTrainingRegime::PositiveWindow,
            ),
            forward_crisis_row(
                NaiveDate::from_ymd_opt(2020, 1, 6).unwrap(),
                1,
                ProbabilityTrainingRegime::PositiveWindow,
            ),
            forward_crisis_row(
                NaiveDate::from_ymd_opt(2020, 1, 20).unwrap(),
                0,
                ProbabilityTrainingRegime::Normal,
            ),
            forward_crisis_row(
                NaiveDate::from_ymd_opt(2020, 1, 21).unwrap(),
                0,
                ProbabilityTrainingRegime::Normal,
            ),
        ];
        let calibration_row_refs = calibration_rows.iter().collect::<Vec<_>>();
        let calibration_candidate = PlattCalibrationArtifact {
            alpha: -1.4,
            beta: -3.0,
            min_input: 0.11,
            max_input: 0.82,
        };

        let (calibration, evaluation_probabilities) = select_probability_calibration_strategy(
            &raw_probabilities,
            &labels,
            &calibration_row_refs,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
            &raw_probabilities,
            calibration_candidate,
        );

        assert!(calibration.is_none());
        assert_eq!(evaluation_probabilities, raw_probabilities);
    }

    #[test]
    fn probability_calibration_strategy_keeps_inverting_fit_for_reversed_raw_ranking() {
        let raw_probabilities = vec![0.11, 0.24, 0.71, 0.82];
        let labels = vec![1.0, 1.0, 0.0, 0.0];
        let calibration_rows = vec![
            forward_crisis_row(
                NaiveDate::from_ymd_opt(2020, 1, 5).unwrap(),
                1,
                ProbabilityTrainingRegime::PositiveWindow,
            ),
            forward_crisis_row(
                NaiveDate::from_ymd_opt(2020, 1, 6).unwrap(),
                1,
                ProbabilityTrainingRegime::PositiveWindow,
            ),
            forward_crisis_row(
                NaiveDate::from_ymd_opt(2020, 1, 20).unwrap(),
                0,
                ProbabilityTrainingRegime::Normal,
            ),
            forward_crisis_row(
                NaiveDate::from_ymd_opt(2020, 1, 21).unwrap(),
                0,
                ProbabilityTrainingRegime::Normal,
            ),
        ];
        let calibration_row_refs = calibration_rows.iter().collect::<Vec<_>>();
        let calibration_candidate = fit_platt_calibration(&raw_probabilities, &labels);
        assert!(calibration_candidate.alpha < 0.0);

        let (calibration, evaluation_probabilities) = select_probability_calibration_strategy(
            &raw_probabilities,
            &labels,
            &calibration_row_refs,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
            &raw_probabilities,
            calibration_candidate.clone(),
        );

        assert_eq!(
            calibration.as_ref().map(|artifact| artifact.alpha),
            Some(calibration_candidate.alpha)
        );
        assert_ne!(evaluation_probabilities, raw_probabilities);
        assert!(evaluation_probabilities[0] > evaluation_probabilities[2]);
    }

    #[test]
    fn probability_calibration_strategy_keeps_raw_when_calibration_flattens_early_warning() {
        let calibration_rows = vec![
            forward_crisis_row(
                NaiveDate::from_ymd_opt(2020, 1, 5).unwrap(),
                1,
                ProbabilityTrainingRegime::PositiveWindow,
            ),
            forward_crisis_row(
                NaiveDate::from_ymd_opt(2020, 1, 7).unwrap(),
                1,
                ProbabilityTrainingRegime::PositiveWindow,
            ),
            forward_crisis_row(
                NaiveDate::from_ymd_opt(2020, 1, 12).unwrap(),
                0,
                ProbabilityTrainingRegime::PreWarningBuffer,
            ),
            forward_crisis_row(
                NaiveDate::from_ymd_opt(2020, 1, 20).unwrap(),
                0,
                ProbabilityTrainingRegime::Normal,
            ),
            forward_crisis_row(
                NaiveDate::from_ymd_opt(2020, 2, 3).unwrap(),
                0,
                ProbabilityTrainingRegime::InCrisis,
            ),
        ];
        let calibration_row_refs = calibration_rows.iter().collect::<Vec<_>>();
        let raw_probabilities = vec![0.72, 0.68, 0.44, 0.12, 0.61];
        let labels = calibration_rows
            .iter()
            .map(|row| row.label_20d as f64)
            .collect::<Vec<_>>();
        let flattening_calibration = PlattCalibrationArtifact {
            alpha: 0.02,
            beta: -4.2,
            min_input: 0.12,
            max_input: 0.72,
        };

        let (calibration, evaluation_probabilities) = select_probability_calibration_strategy(
            &raw_probabilities,
            &labels,
            &calibration_row_refs,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
            &raw_probabilities,
            flattening_calibration,
        );

        assert!(calibration.is_none());
        assert_eq!(evaluation_probabilities, raw_probabilities);
    }

    #[test]
    fn probability_decision_threshold_prefers_precision_over_low_cutoff_noise() {
        let probabilities = vec![0.82, 0.71, 0.24, 0.11, 0.09, 0.08];
        let labels = vec![1.0, 1.0, 0.0, 0.0, 0.0, 0.0];

        let threshold = select_probability_decision_threshold(&probabilities, &labels, 5);

        assert!(threshold >= 0.71);
    }

    #[test]
    fn probability_decision_threshold_allows_low_calibrated_ranges() {
        let probabilities = vec![0.024, 0.021, 0.018, 0.007, 0.006, 0.005];
        let labels = vec![1.0, 1.0, 1.0, 0.0, 0.0, 0.0];

        let threshold = select_probability_decision_threshold(&probabilities, &labels, 60);

        assert!(threshold < 0.05);
        assert!(threshold >= 0.018);
    }

    #[test]
    fn probability_decision_threshold_can_drop_below_one_percent() {
        let probabilities = vec![0.0086, 0.0082, 0.0079, 0.0034, 0.0028, 0.0021];
        let labels = vec![1.0, 1.0, 1.0, 0.0, 0.0, 0.0];

        let threshold = select_probability_decision_threshold(&probabilities, &labels, 20);

        assert!(threshold < 0.01);
        assert!(threshold >= 0.007);
    }

    #[test]
    fn probability_decision_threshold_raises_cutoff_when_low_threshold_is_overbroad() {
        let probabilities = vec![0.38, 0.36, 0.35, 0.34, 0.33, 0.32, 0.31, 0.30, 0.29, 0.28];
        let labels = vec![1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];

        let threshold = select_probability_decision_threshold(&probabilities, &labels, 20);

        assert!(threshold >= 0.35);
        assert!(threshold > 0.30);
    }

    #[test]
    fn probability_decision_threshold_keeps_more_recall_for_60d_when_precision_tradeoff_is_small() {
        let probabilities = vec![0.45, 0.40, 0.35, 0.30, 0.25, 0.34, 0.28, 0.22, 0.18, 0.12];
        let labels = vec![1.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0];

        let threshold = select_probability_decision_threshold(&probabilities, &labels, 60);

        assert!(threshold <= 0.30);
        assert!(threshold >= 0.25);
    }

    #[test]
    fn regime_support_adjustment_lowers_60d_threshold_when_base_misses_prewarning_buffer() {
        let build_row =
            |regime_60d: ProbabilityTrainingRegime, label_60d: u8| ProbabilityTrainingRow {
                as_of_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
                market_scope: "financial_system".to_string(),
                release_id: None,
                probability_mode: Some("formal_bundle_v1".to_string()),
                freshness_status: Some("fresh".to_string()),
                time_to_risk_bucket: Some("test".to_string()),
                split_name: Some("calibration".to_string()),
                features: BTreeMap::new(),
                primary_scenario_id: Some("scenario".to_string()),
                scenario_family: Some("systemic_credit_banking_crisis".to_string()),
                scenario_training_role: None,
                days_to_primary_crisis_start: Some(20),
                primary_scenario_supports_5d: true,
                primary_scenario_supports_20d: true,
                primary_scenario_supports_60d: true,
                label_5d: 0,
                label_20d: 0,
                label_60d,
                regime_5d: ProbabilityTrainingRegime::Normal,
                regime_20d: ProbabilityTrainingRegime::Normal,
                regime_60d,
                action_label_5d: 0,
                action_label_20d: 0,
                action_label_60d: 0,
                prepare_episode_label: 0,
                hedge_episode_label: 0,
                defend_episode_label: 0,
                primary_action_level: None,
                action_episode_id: None,
                action_episode_phase: "outside".to_string(),
                protected_action_window: false,
            };

        let rows = vec![
            build_row(ProbabilityTrainingRegime::PositiveWindow, 1),
            build_row(ProbabilityTrainingRegime::PositiveWindow, 1),
            build_row(ProbabilityTrainingRegime::InCrisis, 1),
            build_row(ProbabilityTrainingRegime::InCrisis, 1),
            build_row(ProbabilityTrainingRegime::PreWarningBuffer, 0),
            build_row(ProbabilityTrainingRegime::PreWarningBuffer, 0),
            build_row(ProbabilityTrainingRegime::Normal, 0),
            build_row(ProbabilityTrainingRegime::Normal, 0),
            build_row(ProbabilityTrainingRegime::Normal, 0),
            build_row(ProbabilityTrainingRegime::PostCrisisCooldown, 0),
        ];
        let row_refs = rows.iter().collect::<Vec<_>>();
        let probabilities = vec![0.95, 0.91, 0.84, 0.80, 0.58, 0.56, 0.22, 0.18, 0.14, 0.10];
        let labels = rows
            .iter()
            .map(|row| row.label_60d as f64)
            .collect::<Vec<_>>();
        let calibration_selection = ProbabilityCalibrationSelection {
            rows: row_refs.clone(),
            eligible_row_count: row_refs.len(),
            eligible_positive_count: labels.iter().filter(|label| **label >= 0.5).count(),
            eligible_negative_count: labels.iter().filter(|label| **label < 0.5).count(),
            used_full_split_fallback: false,
        };
        let threshold_selection = ProbabilityThresholdSelection {
            rows: row_refs.clone(),
            probabilities: probabilities.clone(),
            labels: labels.clone(),
            used_full_split_fallback: false,
        };

        let base_threshold = select_probability_decision_threshold(&probabilities, &labels, 60);
        let adjusted_threshold = adjust_probability_decision_threshold_for_regime_support(
            base_threshold,
            &probabilities,
            &labels,
            &row_refs,
            60,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );
        let diagnostics =
            build_probability_threshold_diagnostics(ProbabilityThresholdDiagnosticsInput {
                full_calibration_rows: &rows,
                calibration_selection: &calibration_selection,
                threshold_selection: &threshold_selection,
                horizon_days: 60,
                label_mode: ProbabilityTargetLabelMode::ForwardCrisis,
                base_threshold,
                final_threshold: adjusted_threshold,
            });

        assert!(base_threshold > 0.58);
        assert!(adjusted_threshold <= base_threshold);
        assert!(diagnostics.repair_applied);
        assert!(matches!(
            diagnostics.repair_reason.as_str(),
            "repaired_to_early_warning_cap" | "repaired_to_regime_support_candidate"
        ));
        assert_eq!(diagnostics.base_summary.early_warning_hit_count, 0);
        assert!(diagnostics.final_summary.early_warning_hit_count > 0);

        let prewarning_evidence = diagnostics
            .calibration_regime_evidence
            .iter()
            .find(|row| row.regime == "pre_warning_buffer")
            .expect("pre-warning calibration evidence");
        assert_eq!(prewarning_evidence.full_row_count, 2);
        assert_eq!(prewarning_evidence.calibration_eligible_row_count, 2);
        assert_eq!(prewarning_evidence.calibration_used_row_count, 2);
        assert_eq!(prewarning_evidence.threshold_selected_row_count, 2);
        assert_eq!(prewarning_evidence.positive_label_count, 0);
        assert_eq!(prewarning_evidence.avg_hard_label, 0.0);
        assert_eq!(prewarning_evidence.avg_training_target, 0.26);
        assert_eq!(prewarning_evidence.avg_objective_weight, 0.6);
    }

    #[test]
    fn threshold_selection_excludes_in_crisis_negatives_for_60d_forward_crisis() {
        let build_row =
            |regime_60d: ProbabilityTrainingRegime, label_60d: u8| ProbabilityTrainingRow {
                as_of_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
                market_scope: "financial_system".to_string(),
                release_id: None,
                probability_mode: Some("formal_bundle_v1".to_string()),
                freshness_status: Some("fresh".to_string()),
                time_to_risk_bucket: Some("test".to_string()),
                split_name: Some("calibration".to_string()),
                features: BTreeMap::new(),
                primary_scenario_id: Some("scenario".to_string()),
                scenario_family: Some("systemic_credit_banking_crisis".to_string()),
                scenario_training_role: None,
                days_to_primary_crisis_start: Some(20),
                primary_scenario_supports_5d: true,
                primary_scenario_supports_20d: true,
                primary_scenario_supports_60d: true,
                label_5d: 0,
                label_20d: 0,
                label_60d,
                regime_5d: ProbabilityTrainingRegime::Normal,
                regime_20d: ProbabilityTrainingRegime::Normal,
                regime_60d,
                action_label_5d: 0,
                action_label_20d: 0,
                action_label_60d: 0,
                prepare_episode_label: 0,
                hedge_episode_label: 0,
                defend_episode_label: 0,
                primary_action_level: None,
                action_episode_id: None,
                action_episode_phase: "outside".to_string(),
                protected_action_window: false,
            };

        let rows = vec![
            build_row(ProbabilityTrainingRegime::PositiveWindow, 1),
            build_row(ProbabilityTrainingRegime::PreWarningBuffer, 0),
            build_row(ProbabilityTrainingRegime::Normal, 0),
            build_row(ProbabilityTrainingRegime::PostCrisisCooldown, 0),
            build_row(ProbabilityTrainingRegime::InCrisis, 0),
        ];
        let row_refs = rows.iter().collect::<Vec<_>>();
        let probabilities = vec![0.9, 0.55, 0.20, 0.10, 0.88];
        let labels = rows
            .iter()
            .map(|row| row.label_60d as f64)
            .collect::<Vec<_>>();

        let selection = probability_decision_threshold_selection(
            &probabilities,
            &labels,
            &row_refs,
            60,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );

        assert!(!selection.used_full_split_fallback);
        assert_eq!(selection.rows.len(), 4);
        assert_eq!(
            selection
                .labels
                .iter()
                .filter(|label| **label >= 0.5)
                .count(),
            1
        );
        assert_eq!(
            selection
                .labels
                .iter()
                .filter(|label| **label < 0.5)
                .count(),
            3
        );
        assert!(selection
            .rows
            .iter()
            .all(|row| row.regime_60d != ProbabilityTrainingRegime::InCrisis));
    }

    #[test]
    fn runtime_regime_separation_detects_calibration_crush() {
        let scenarios = synthetic_runtime_scenarios();
        let history = vec![
            runtime_history_point(NaiveDate::from_ymd_opt(1999, 12, 20).unwrap(), 0.10, 0.020),
            runtime_history_point(NaiveDate::from_ymd_opt(2000, 1, 5).unwrap(), 0.30, 0.021),
            runtime_history_point(NaiveDate::from_ymd_opt(2000, 1, 20).unwrap(), 0.45, 0.022),
            runtime_history_point(NaiveDate::from_ymd_opt(2000, 2, 5).unwrap(), 0.55, 0.023),
        ];

        let summaries = summarize_release_runtime_regime_probabilities(&history, &scenarios, None);
        let separation = summarize_release_runtime_regime_separation(&summaries);
        let row20 = separation
            .iter()
            .find(|row| row.horizon_days == 20)
            .expect("20d summary");

        assert_eq!(row20.early_warning_regime, "pre_warning_buffer");
        assert_eq!(row20.diagnosis, "calibration_crushed_early_warning");
        assert!(
            row20
                .early_warning_raw_lift_vs_normal
                .expect("raw lift should exist")
                >= 2.9
        );
        assert!(
            row20
                .early_warning_calibrated_lift_vs_normal
                .expect("calibrated lift should exist")
                < 1.1
        );
        assert!(
            row20
                .early_warning_gap_retention
                .expect("gap retention should exist")
                < 0.1
        );
    }

    #[test]
    fn runtime_regime_classifier_flags_cooldown_bleed() {
        let diagnosis = classify_regime_separation(
            20,
            1.7,
            1.6,
            Some(0.9),
            1.55,
            0.014,
            1.3,
            1.58,
            0.015,
            1.58,
            0.05,
        );

        assert_eq!(diagnosis, "cooldown_bleed");
    }

    #[test]
    fn offline_regime_classifier_uses_positive_window_gap_not_only_buffer_lift() {
        let diagnosis =
            classify_probability_regime_separation(20, 1.6, 1.52, 1.6, 1.2, 1.1, 0.012, 0.004, 1.6);

        assert_eq!(diagnosis, "usable_early_warning_separation");
    }

    #[test]
    fn probability_guardrails_reject_zero_usable_early_warning_horizons() {
        let bundle = ProbabilityBundle {
            bundle_id: "candidate_guard_zero".to_string(),
            market_scope: "financial_system".to_string(),
            probability_mode: "formal_bundle_v1".to_string(),
            model_family: PROBABILITY_MODEL_FAMILY_LINEAR_V1.to_string(),
            feature_transform: PROBABILITY_FEATURE_TRANSFORM_IDENTITY_V1.to_string(),
            created_at: Utc::now(),
            feature_names: Vec::new(),
            monotonic_min_gap_5d_to_20d: 0.0,
            monotonic_min_gap_20d_to_60d: 0.0,
            note: "test".to_string(),
            horizons: Vec::new(),
            evaluation: Some(ProbabilityBundleEvaluation {
                sample_count: 100,
                brier_score: 0.1,
                log_loss: 0.2,
                ece: 0.1,
                regime_separation_summaries: vec![RegimeSeparationEvaluationSummary {
                    horizon_days: 20,
                    early_warning_regime: "pre_warning_buffer".to_string(),
                    normal_sample_count: 50,
                    pre_warning_buffer_sample_count: 10,
                    positive_window_sample_count: 10,
                    early_warning_sample_count: 10,
                    in_crisis_sample_count: 10,
                    post_crisis_cooldown_sample_count: 10,
                    normal_avg_probability: 0.02,
                    pre_warning_buffer_avg_probability: 0.025,
                    positive_window_avg_probability: 0.019,
                    early_warning_avg_probability: 0.025,
                    in_crisis_avg_probability: 0.04,
                    post_crisis_cooldown_avg_probability: 0.03,
                    max_non_normal_avg_probability: 0.04,
                    pre_warning_buffer_lift_vs_normal: Some(1.25),
                    positive_window_lift_vs_normal: Some(0.95),
                    early_warning_lift_vs_normal: Some(1.25),
                    in_crisis_lift_vs_normal: Some(2.0),
                    post_crisis_cooldown_lift_vs_normal: Some(1.5),
                    positive_window_gap_vs_normal: Some(-0.001),
                    post_crisis_cooldown_gap_vs_normal: Some(0.01),
                    max_non_normal_lift_vs_normal: Some(2.0),
                    diagnosis: "cold_across_all_regimes".to_string(),
                }],
                usable_early_warning_horizon_count: 0,
                insufficient_early_warning_horizon_count: 1,
                note: "test".to_string(),
            }),
            actionability: None,
        };
        let release = test_release_with_bundle(&bundle);
        let bundle_path = release.manifest.bundle_uri.clone();

        let regressions = compare_probability_guardrails(&release).unwrap();

        let _ = std::fs::remove_file(bundle_path);
        assert!(regressions
            .iter()
            .any(|item| item.contains("zero usable early-warning horizons")));
        assert!(regressions
            .iter()
            .any(|item| item.contains("20d positive_window avg")));
        assert!(regressions
            .iter()
            .any(|item| item.contains("cold_across_all_regimes")));
    }

    #[test]
    fn probability_guardrails_reject_cooldown_bleed_on_medium_horizons() {
        let bundle = ProbabilityBundle {
            bundle_id: "candidate_guard_cooldown".to_string(),
            market_scope: "financial_system".to_string(),
            probability_mode: "formal_bundle_v1".to_string(),
            model_family: PROBABILITY_MODEL_FAMILY_LINEAR_V1.to_string(),
            feature_transform: PROBABILITY_FEATURE_TRANSFORM_IDENTITY_V1.to_string(),
            created_at: Utc::now(),
            feature_names: Vec::new(),
            monotonic_min_gap_5d_to_20d: 0.0,
            monotonic_min_gap_20d_to_60d: 0.0,
            note: "test".to_string(),
            horizons: Vec::new(),
            evaluation: Some(ProbabilityBundleEvaluation {
                sample_count: 100,
                brier_score: 0.1,
                log_loss: 0.2,
                ece: 0.1,
                regime_separation_summaries: vec![RegimeSeparationEvaluationSummary {
                    horizon_days: 60,
                    early_warning_regime: "pre_warning_buffer".to_string(),
                    normal_sample_count: 50,
                    pre_warning_buffer_sample_count: 10,
                    positive_window_sample_count: 10,
                    early_warning_sample_count: 10,
                    in_crisis_sample_count: 10,
                    post_crisis_cooldown_sample_count: 10,
                    normal_avg_probability: 0.03,
                    pre_warning_buffer_avg_probability: 0.05,
                    positive_window_avg_probability: 0.065,
                    early_warning_avg_probability: 0.05,
                    in_crisis_avg_probability: 0.06,
                    post_crisis_cooldown_avg_probability: 0.068,
                    max_non_normal_avg_probability: 0.068,
                    pre_warning_buffer_lift_vs_normal: Some(1.67),
                    positive_window_lift_vs_normal: Some(2.17),
                    early_warning_lift_vs_normal: Some(1.67),
                    in_crisis_lift_vs_normal: Some(2.0),
                    post_crisis_cooldown_lift_vs_normal: Some(2.27),
                    positive_window_gap_vs_normal: Some(0.035),
                    post_crisis_cooldown_gap_vs_normal: Some(0.038),
                    max_non_normal_lift_vs_normal: Some(2.27),
                    diagnosis: "cooldown_bleed".to_string(),
                }],
                usable_early_warning_horizon_count: 1,
                insufficient_early_warning_horizon_count: 1,
                note: "test".to_string(),
            }),
            actionability: None,
        };
        let release = test_release_with_bundle(&bundle);
        let bundle_path = release.manifest.bundle_uri.clone();

        let regressions = compare_probability_guardrails(&release).unwrap();

        let _ = std::fs::remove_file(bundle_path);
        assert!(regressions
            .iter()
            .any(|item| item.contains("cooldown_bleed")));
    }

    #[test]
    fn release_review_backtest_comparison_marks_lost_timely_warning() {
        let baseline = vec![
            synthetic_backtest_summary("scenario_a", "Scenario A", Some(20), Some(14), 0),
            synthetic_backtest_summary("scenario_b", "Scenario B", Some(9), None, 1),
        ];
        let candidate = vec![
            synthetic_backtest_summary("scenario_a", "Scenario A", Some(18), None, 2),
            synthetic_backtest_summary("scenario_b", "Scenario B", Some(9), Some(5), 1),
        ];

        let rows = build_release_review_backtest_scenario_comparisons(&baseline, &candidate);

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].scenario_id, "scenario_a");
        assert_eq!(rows[0].outcome, "timely_to_missed");
        assert_eq!(rows[0].actionable_delta_days, None);
        assert_eq!(rows[1].scenario_id, "scenario_b");
        assert_eq!(rows[1].outcome, "missed_to_late_only");
    }

    #[test]
    fn release_review_focus_diagnostic_highlights_missing_actionable_window() {
        let crisis_start = NaiveDate::from_ymd_opt(2023, 3, 10).unwrap();
        let first_l2 = NaiveDate::from_ymd_opt(2022, 12, 17).unwrap();
        let first_l3 = NaiveDate::from_ymd_opt(2022, 12, 30).unwrap();
        let follow_up_1 = NaiveDate::from_ymd_opt(2023, 1, 6).unwrap();
        let follow_up_2 = NaiveDate::from_ymd_opt(2023, 1, 13).unwrap();
        let follow_up_3 = NaiveDate::from_ymd_opt(2023, 1, 20).unwrap();
        let baseline = vec![synthetic_backtest_summary_with_dates(
            "scenario_a",
            "Scenario A",
            Some(first_l2),
            Some(first_l3),
            Some(83),
            Some(70),
            2,
        )];
        let candidate = vec![synthetic_backtest_summary_with_dates(
            "scenario_a",
            "Scenario A",
            Some(first_l2),
            None,
            Some(83),
            None,
            2,
        )];
        let baseline_history = vec![
            runtime_history_point_with_state(
                first_l2,
                56.0,
                0.02,
                0.14,
                0.42,
                DecisionPosture::Prepare,
                TimeToRiskBucket::Months,
                43.0,
                &["prepare_p60d_structural"],
            ),
            runtime_history_point_with_state(
                first_l3,
                62.0,
                0.02,
                0.21,
                0.48,
                DecisionPosture::Prepare,
                TimeToRiskBucket::Months,
                48.0,
                &["prepare_p60d_structural"],
            ),
            runtime_history_point_with_state(
                follow_up_1,
                63.0,
                0.03,
                0.22,
                0.49,
                DecisionPosture::Prepare,
                TimeToRiskBucket::Months,
                48.0,
                &["prepare_p60d_structural"],
            ),
            runtime_history_point_with_state(
                follow_up_2,
                64.0,
                0.04,
                0.23,
                0.50,
                DecisionPosture::Prepare,
                TimeToRiskBucket::Months,
                49.0,
                &["prepare_p60d_structural"],
            ),
            runtime_history_point_with_state(
                follow_up_3,
                60.0,
                0.03,
                0.19,
                0.47,
                DecisionPosture::Prepare,
                TimeToRiskBucket::Months,
                47.0,
                &["prepare_p60d_structural"],
            ),
            runtime_history_point_with_state(
                crisis_start,
                66.0,
                0.08,
                0.31,
                0.52,
                DecisionPosture::Hedge,
                TimeToRiskBucket::Weeks,
                50.0,
                &["hedge_p20d_context"],
            ),
        ];
        let candidate_history = vec![
            runtime_history_point_with_state(
                first_l2,
                55.0,
                0.02,
                0.13,
                0.40,
                DecisionPosture::Prepare,
                TimeToRiskBucket::Months,
                42.0,
                &["prepare_p60d_structural"],
            ),
            runtime_history_point_with_state(
                first_l3,
                57.0,
                0.02,
                0.16,
                0.44,
                DecisionPosture::Prepare,
                TimeToRiskBucket::Months,
                45.0,
                &["prepare_p60d_structural"],
            ),
            runtime_history_point_with_state(
                follow_up_1,
                56.0,
                0.02,
                0.17,
                0.44,
                DecisionPosture::Prepare,
                TimeToRiskBucket::Months,
                45.0,
                &["prepare_p60d_structural"],
            ),
            runtime_history_point_with_state(
                follow_up_2,
                55.0,
                0.02,
                0.16,
                0.43,
                DecisionPosture::Prepare,
                TimeToRiskBucket::Months,
                44.0,
                &["prepare_p60d_structural"],
            ),
            runtime_history_point_with_state(
                follow_up_3,
                63.0,
                0.04,
                0.22,
                0.48,
                DecisionPosture::Prepare,
                TimeToRiskBucket::Months,
                48.0,
                &["prepare_p60d_structural"],
            ),
            runtime_history_point_with_state(
                crisis_start,
                65.0,
                0.08,
                0.31,
                0.50,
                DecisionPosture::Hedge,
                TimeToRiskBucket::Weeks,
                49.0,
                &["hedge_p20d_context"],
            ),
        ];
        let method = formal_main_audit_method_wire();

        let rows = build_release_review_scenario_focus_diagnostics(
            &baseline,
            &candidate,
            &baseline_history,
            &candidate_history,
            &method,
            &method,
        );

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].scenario_id, "scenario_a");
        assert_eq!(rows[0].outcome, "timely_to_missed");
        assert_eq!(rows[0].baseline_first_l3_date, Some(first_l3));
        assert_eq!(rows[0].candidate_first_l3_date, None);
        assert_eq!(rows[0].baseline_actionable_point_count, 4);
        assert_eq!(rows[0].candidate_actionable_point_count, 1);
        assert_eq!(rows[0].baseline_runtime_floor_hit_point_count, 5);
        assert_eq!(rows[0].candidate_runtime_floor_hit_point_count, 5);
        assert_eq!(
            rows[0].baseline_primary_failure_mode.as_deref(),
            Some("strict_gate_mismatch")
        );
        assert_eq!(
            rows[0].candidate_primary_failure_mode.as_deref(),
            Some("strict_gate_mismatch")
        );
        assert_eq!(rows[0].runtime_block_counts.len(), 1);
        assert_eq!(rows[0].runtime_block_counts[0].category, "review_gate_gap");
        assert_eq!(rows[0].runtime_block_counts[0].baseline_count, 1);
        assert_eq!(rows[0].runtime_block_counts[0].candidate_count, 4);
        assert_eq!(
            rows[0].dominant_runtime_blocks.baseline_categories,
            vec!["review_gate_gap".to_string()]
        );
        assert_eq!(rows[0].dominant_runtime_blocks.baseline_count, 1);
        assert_eq!(
            rows[0].dominant_runtime_blocks.candidate_categories,
            vec!["review_gate_gap".to_string()]
        );
        assert_eq!(rows[0].dominant_runtime_blocks.candidate_count, 4);
        assert!(rows[0]
            .dominant_runtime_continuity_facets
            .baseline_categories
            .contains(&"posture:prepare".to_string()));
        assert_eq!(rows[0].dominant_runtime_continuity_facets.baseline_count, 1);
        assert!(rows[0]
            .dominant_runtime_continuity_facets
            .candidate_categories
            .contains(&"posture:prepare".to_string()));
        assert_eq!(
            rows[0].dominant_runtime_continuity_facets.candidate_count,
            4
        );
        assert!(rows[0]
            .runtime_continuity_facet_counts
            .iter()
            .any(|facet| facet.category == "posture:prepare"
                && facet.baseline_count == 1
                && facet.candidate_count == 4));
        assert!(rows[0]
            .runtime_continuity_facet_counts
            .iter()
            .any(|facet| facet.category == "bucket:months"
                && facet.baseline_count == 1
                && facet.candidate_count == 4));
        assert!(rows[0]
            .runtime_continuity_facet_counts
            .iter()
            .any(|facet| facet.category == "trigger:prepare"
                && facet.baseline_count == 1
                && facet.candidate_count == 4));
        assert!(rows[0]
            .runtime_continuity_facet_counts
            .iter()
            .any(|facet| facet.category == "gate_gap:p20d_and_p60d"
                && facet.baseline_count == 1
                && facet.candidate_count == 4));
        assert!(rows[0]
            .runtime_continuity_facet_counts
            .iter()
            .any(|facet| facet.category == "confirmation:months_score_low"
                && facet.baseline_count == 1
                && facet.candidate_count == 4));
        assert_eq!(
            rows[0].candidate_first_runtime_floor_hit_without_l3_date,
            Some(first_l2)
        );
        assert!(rows[0]
            .candidate_first_runtime_floor_hit_without_l3_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("hit runtime floor")));

        let first_l3_point = rows[0]
            .interesting_points
            .iter()
            .find(|point| point.as_of_date == first_l3)
            .expect("first_l3 point should be present");
        assert!(first_l3_point.baseline_actionable);
        assert!(!first_l3_point.candidate_actionable);
        assert!(first_l3_point.baseline_strict_review_actionable);
        assert!(!first_l3_point.candidate_strict_review_actionable);
        assert!(first_l3_point.baseline_runtime_floor_hit);
        assert!(first_l3_point.candidate_runtime_floor_hit);
        assert_eq!(
            first_l3_point.baseline_runtime_actionable_block_category,
            None
        );
        assert_eq!(
            first_l3_point
                .candidate_runtime_actionable_block_category
                .as_deref(),
            Some("review_gate_gap")
        );
        assert_eq!(first_l3_point.baseline_actionable_forward_5d_hits, Some(4));
        assert_eq!(first_l3_point.candidate_actionable_forward_5d_hits, Some(1));
        assert_eq!(first_l3_point.baseline_actionable_sustained, Some(true));
        assert_eq!(first_l3_point.candidate_actionable_sustained, Some(false));
        assert_eq!(
            first_l3_point.baseline_runtime_actionable_block_reason,
            None
        );
        assert!(first_l3_point
            .candidate_runtime_actionable_block_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("hit runtime floor")));
        assert_eq!(
            first_l3_point.baseline_actionable_diagnostic.as_deref(),
            Some("actionable")
        );
        assert!(first_l3_point
            .candidate_actionable_diagnostic
            .as_deref()
            .is_some_and(|reason| reason.contains("hit runtime floor")));
    }

    #[test]
    fn release_review_focus_diagnostic_includes_structural_only_missed_scenarios() {
        let crisis_start = NaiveDate::from_ymd_opt(2023, 3, 10).unwrap();
        let runtime_floor_date = NaiveDate::from_ymd_opt(2023, 2, 10).unwrap();
        let first_l2 = NaiveDate::from_ymd_opt(2023, 2, 20).unwrap();
        let baseline = vec![synthetic_backtest_summary_with_dates(
            "scenario_structural",
            "Structural Only",
            Some(first_l2),
            None,
            Some(18),
            None,
            0,
        )];
        let candidate = vec![synthetic_backtest_summary_with_dates(
            "scenario_structural",
            "Structural Only",
            Some(first_l2),
            None,
            Some(18),
            None,
            0,
        )];
        let shared_history = vec![
            runtime_history_point_with_state(
                runtime_floor_date,
                52.0,
                0.02,
                0.08,
                0.14,
                DecisionPosture::Normal,
                TimeToRiskBucket::Normal,
                41.0,
                &[],
            ),
            runtime_history_point_with_state(
                first_l2,
                54.0,
                0.02,
                0.09,
                0.16,
                DecisionPosture::Normal,
                TimeToRiskBucket::Normal,
                42.0,
                &[],
            ),
            runtime_history_point_with_state(
                crisis_start,
                60.0,
                0.05,
                0.21,
                0.32,
                DecisionPosture::Normal,
                TimeToRiskBucket::Normal,
                44.0,
                &[],
            ),
        ];
        let method = formal_main_audit_method_wire();

        let rows = build_release_review_scenario_focus_diagnostics(
            &baseline,
            &candidate,
            &shared_history,
            &shared_history,
            &method,
            &method,
        );

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].scenario_id, "scenario_structural");
        assert_eq!(rows[0].outcome, "missed_to_missed");
        assert_eq!(rows[0].baseline_runtime_floor_hit_point_count, 2);
        assert_eq!(rows[0].candidate_runtime_floor_hit_point_count, 2);
        assert_eq!(
            rows[0].baseline_primary_failure_mode.as_deref(),
            Some("strict_gate_mismatch")
        );
        assert_eq!(
            rows[0].candidate_primary_failure_mode.as_deref(),
            Some("strict_gate_mismatch")
        );
        assert_eq!(rows[0].runtime_block_counts.len(), 1);
        assert_eq!(rows[0].runtime_block_counts[0].category, "review_gate_gap");
        assert_eq!(rows[0].runtime_block_counts[0].baseline_count, 2);
        assert_eq!(rows[0].runtime_block_counts[0].candidate_count, 2);
        assert_eq!(
            rows[0].dominant_runtime_blocks.baseline_categories,
            vec!["review_gate_gap".to_string()]
        );
        assert_eq!(rows[0].dominant_runtime_blocks.baseline_count, 2);
        assert_eq!(
            rows[0].dominant_runtime_blocks.candidate_categories,
            vec!["review_gate_gap".to_string()]
        );
        assert_eq!(rows[0].dominant_runtime_blocks.candidate_count, 2);
        assert!(rows[0]
            .dominant_runtime_continuity_facets
            .baseline_categories
            .contains(&"posture:normal".to_string()));
        assert_eq!(rows[0].dominant_runtime_continuity_facets.baseline_count, 2);
        assert!(rows[0]
            .runtime_continuity_facet_counts
            .iter()
            .any(|facet| facet.category == "posture:normal"
                && facet.baseline_count == 2
                && facet.candidate_count == 2));
        assert!(rows[0]
            .runtime_continuity_facet_counts
            .iter()
            .any(|facet| facet.category == "bucket:normal"
                && facet.baseline_count == 2
                && facet.candidate_count == 2));
        assert!(rows[0]
            .runtime_continuity_facet_counts
            .iter()
            .any(|facet| facet.category == "trigger:none"
                && facet.baseline_count == 2
                && facet.candidate_count == 2));
        assert_eq!(
            rows[0].baseline_first_runtime_floor_hit_without_l3_date,
            Some(runtime_floor_date)
        );
        assert!(rows[0]
            .baseline_first_runtime_floor_hit_without_l3_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("hit runtime floor")));
        assert!(rows[0]
            .interesting_points
            .iter()
            .any(|point| point.as_of_date == runtime_floor_date));
    }

    #[test]
    fn release_review_focus_diagnostic_counts_posture_continuity_facets() {
        let crisis_start = NaiveDate::from_ymd_opt(2023, 3, 10).unwrap();
        let first_l2 = NaiveDate::from_ymd_opt(2023, 2, 10).unwrap();
        let baseline = vec![synthetic_backtest_summary_with_dates(
            "scenario_posture",
            "Posture Continuity",
            Some(first_l2),
            None,
            Some(28),
            None,
            0,
        )];
        let candidate = baseline.clone();
        let history = vec![
            runtime_history_point_with_state(
                first_l2,
                66.0,
                0.03,
                0.26,
                0.58,
                DecisionPosture::Normal,
                TimeToRiskBucket::Normal,
                52.0,
                &[],
            ),
            runtime_history_point_with_state(
                NaiveDate::from_ymd_opt(2023, 2, 17).unwrap(),
                67.0,
                0.03,
                0.28,
                0.61,
                DecisionPosture::Normal,
                TimeToRiskBucket::Normal,
                54.0,
                &[],
            ),
            runtime_history_point_with_state(
                crisis_start,
                70.0,
                0.05,
                0.31,
                0.64,
                DecisionPosture::Hedge,
                TimeToRiskBucket::Weeks,
                56.0,
                &["hedge_p20d_context"],
            ),
        ];
        let method = formal_main_audit_method_wire();

        let rows = build_release_review_scenario_focus_diagnostics(
            &baseline, &candidate, &history, &history, &method, &method,
        );

        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].baseline_primary_failure_mode.as_deref(),
            Some("posture_continuity_failure")
        );
        assert_eq!(
            rows[0].candidate_primary_failure_mode.as_deref(),
            Some("posture_continuity_failure")
        );
        assert_eq!(rows[0].runtime_block_counts.len(), 1);
        assert_eq!(
            rows[0].runtime_block_counts[0].category,
            "posture_bucket_normal"
        );
        assert_eq!(rows[0].runtime_block_counts[0].baseline_count, 2);
        assert_eq!(rows[0].runtime_block_counts[0].candidate_count, 2);
        assert_eq!(
            rows[0].dominant_runtime_blocks.baseline_categories,
            vec!["posture_bucket_normal".to_string()]
        );
        assert_eq!(rows[0].dominant_runtime_blocks.baseline_count, 2);
        assert_eq!(
            rows[0].dominant_runtime_blocks.candidate_categories,
            vec!["posture_bucket_normal".to_string()]
        );
        assert_eq!(rows[0].dominant_runtime_blocks.candidate_count, 2);
        assert!(rows[0]
            .dominant_runtime_continuity_facets
            .baseline_categories
            .contains(&"posture:normal".to_string()));
        assert_eq!(rows[0].dominant_runtime_continuity_facets.baseline_count, 2);
        assert!(rows[0]
            .runtime_continuity_facet_counts
            .iter()
            .any(|facet| facet.category == "posture:normal"
                && facet.baseline_count == 2
                && facet.candidate_count == 2));
        assert!(rows[0]
            .runtime_continuity_facet_counts
            .iter()
            .any(|facet| facet.category == "bucket:normal"
                && facet.baseline_count == 2
                && facet.candidate_count == 2));
        assert!(rows[0]
            .runtime_continuity_facet_counts
            .iter()
            .any(|facet| facet.category == "trigger:none"
                && facet.baseline_count == 2
                && facet.candidate_count == 2));
        assert!(rows[0]
            .runtime_continuity_facet_counts
            .iter()
            .any(|facet| facet.category == "gate_gap:none"
                && facet.baseline_count == 2
                && facet.candidate_count == 2));
        assert!(rows[0]
            .runtime_continuity_facet_counts
            .iter()
            .any(|facet| facet.category == "confirmation:ok_or_not_needed"
                && facet.baseline_count == 2
                && facet.candidate_count == 2));
    }

    #[test]
    fn release_review_failure_mode_summary_groups_focus_scenarios() {
        let crisis_start = NaiveDate::from_ymd_opt(2023, 3, 10).unwrap();
        let gate_rows = vec![ReleaseReviewScenarioFocusDiagnostic {
            scenario_id: "gate_a".to_string(),
            name: "Gate Mismatch".to_string(),
            outcome: "missed_to_missed".to_string(),
            window_start: crisis_start,
            window_end: crisis_start,
            crisis_start,
            crisis_end: crisis_start,
            baseline_first_l2_date: None,
            candidate_first_l2_date: None,
            baseline_first_l3_date: None,
            candidate_first_l3_date: None,
            baseline_first_non_normal_date: None,
            candidate_first_non_normal_date: None,
            baseline_actionable_point_count: 0,
            candidate_actionable_point_count: 0,
            baseline_runtime_floor_hit_point_count: 2,
            candidate_runtime_floor_hit_point_count: 3,
            baseline_max_p20d: None,
            candidate_max_p20d: None,
            baseline_max_p60d: None,
            candidate_max_p60d: None,
            baseline_first_runtime_floor_hit_without_l3_date: None,
            candidate_first_runtime_floor_hit_without_l3_date: None,
            baseline_first_runtime_floor_hit_without_l3_reason: None,
            candidate_first_runtime_floor_hit_without_l3_reason: None,
            baseline_primary_failure_mode: Some("strict_gate_mismatch".to_string()),
            candidate_primary_failure_mode: Some("strict_gate_mismatch".to_string()),
            dominant_runtime_blocks: ReleaseReviewRuntimeDominantCategories {
                baseline_categories: vec!["review_gate_gap".to_string()],
                baseline_count: 2,
                candidate_categories: vec!["review_gate_gap".to_string()],
                candidate_count: 3,
            },
            dominant_runtime_continuity_facets: ReleaseReviewRuntimeDominantCategories {
                baseline_categories: vec!["gate_gap:p20d_and_p60d".to_string()],
                baseline_count: 2,
                candidate_categories: vec!["gate_gap:p20d_and_p60d".to_string()],
                candidate_count: 3,
            },
            runtime_block_counts: Vec::new(),
            runtime_continuity_facet_counts: Vec::new(),
            interesting_points: Vec::new(),
        }];
        let posture_rows = vec![ReleaseReviewScenarioFocusDiagnostic {
            scenario_id: "posture_a".to_string(),
            name: "Posture Continuity".to_string(),
            outcome: "missed_to_missed".to_string(),
            window_start: crisis_start,
            window_end: crisis_start,
            crisis_start,
            crisis_end: crisis_start,
            baseline_first_l2_date: None,
            candidate_first_l2_date: None,
            baseline_first_l3_date: None,
            candidate_first_l3_date: None,
            baseline_first_non_normal_date: None,
            candidate_first_non_normal_date: None,
            baseline_actionable_point_count: 0,
            candidate_actionable_point_count: 0,
            baseline_runtime_floor_hit_point_count: 2,
            candidate_runtime_floor_hit_point_count: 2,
            baseline_max_p20d: None,
            candidate_max_p20d: None,
            baseline_max_p60d: None,
            candidate_max_p60d: None,
            baseline_first_runtime_floor_hit_without_l3_date: None,
            candidate_first_runtime_floor_hit_without_l3_date: None,
            baseline_first_runtime_floor_hit_without_l3_reason: None,
            candidate_first_runtime_floor_hit_without_l3_reason: None,
            baseline_primary_failure_mode: Some("posture_continuity_failure".to_string()),
            candidate_primary_failure_mode: None,
            dominant_runtime_blocks: ReleaseReviewRuntimeDominantCategories {
                baseline_categories: vec!["posture_bucket_normal".to_string()],
                baseline_count: 2,
                candidate_categories: Vec::new(),
                candidate_count: 0,
            },
            dominant_runtime_continuity_facets: ReleaseReviewRuntimeDominantCategories {
                baseline_categories: vec!["posture:normal".to_string()],
                baseline_count: 2,
                candidate_categories: Vec::new(),
                candidate_count: 0,
            },
            runtime_block_counts: Vec::new(),
            runtime_continuity_facet_counts: Vec::new(),
            interesting_points: Vec::new(),
        }];

        let summary = summarize_release_review_failure_modes(&[
            gate_rows[0].clone(),
            posture_rows[0].clone(),
        ]);

        assert_eq!(summary.len(), 2);
        let strict_gate = summary
            .iter()
            .find(|row| row.failure_mode == "strict_gate_mismatch")
            .expect("strict gate mismatch row");
        assert_eq!(strict_gate.baseline_count, 1);
        assert_eq!(strict_gate.candidate_count, 1);
        assert_eq!(
            strict_gate.baseline_scenarios,
            vec!["Gate Mismatch".to_string()]
        );
        assert_eq!(
            strict_gate.candidate_scenarios,
            vec!["Gate Mismatch".to_string()]
        );
        let posture = summary
            .iter()
            .find(|row| row.failure_mode == "posture_continuity_failure")
            .expect("posture continuity row");
        assert_eq!(posture.baseline_count, 1);
        assert_eq!(posture.candidate_count, 0);
        assert_eq!(
            posture.baseline_scenarios,
            vec!["Posture Continuity".to_string()]
        );
        assert!(posture.candidate_scenarios.is_empty());
    }

    #[test]
    fn release_review_historical_audit_priorities_map_scenarios_to_workstreams() {
        let crisis_start = NaiveDate::from_ymd_opt(2023, 3, 10).unwrap();
        let summary = summarize_release_review_historical_audit_priorities(&[
            ReleaseReviewScenarioFocusDiagnostic {
                scenario_id: "us_dotcom_unwind_2000".to_string(),
                name: "2000-2001 科网泡沫出清".to_string(),
                outcome: "missed_to_missed".to_string(),
                window_start: crisis_start,
                window_end: crisis_start,
                crisis_start,
                crisis_end: crisis_start,
                baseline_first_l2_date: None,
                candidate_first_l2_date: None,
                baseline_first_l3_date: None,
                candidate_first_l3_date: None,
                baseline_first_non_normal_date: None,
                candidate_first_non_normal_date: None,
                baseline_actionable_point_count: 0,
                candidate_actionable_point_count: 0,
                baseline_runtime_floor_hit_point_count: 3,
                candidate_runtime_floor_hit_point_count: 2,
                baseline_max_p20d: None,
                candidate_max_p20d: None,
                baseline_max_p60d: None,
                candidate_max_p60d: None,
                baseline_first_runtime_floor_hit_without_l3_date: None,
                candidate_first_runtime_floor_hit_without_l3_date: None,
                baseline_first_runtime_floor_hit_without_l3_reason: None,
                candidate_first_runtime_floor_hit_without_l3_reason: None,
                baseline_primary_failure_mode: Some("strict_gate_mismatch".to_string()),
                candidate_primary_failure_mode: Some("strict_gate_mismatch".to_string()),
                dominant_runtime_blocks: ReleaseReviewRuntimeDominantCategories {
                    baseline_categories: vec!["review_gate_gap".to_string()],
                    baseline_count: 3,
                    candidate_categories: vec!["review_gate_gap".to_string()],
                    candidate_count: 2,
                },
                dominant_runtime_continuity_facets: ReleaseReviewRuntimeDominantCategories {
                    baseline_categories: vec!["gate_gap:p20d_and_p60d".to_string()],
                    baseline_count: 3,
                    candidate_categories: vec!["gate_gap:p20d_and_p60d".to_string()],
                    candidate_count: 2,
                },
                runtime_block_counts: Vec::new(),
                runtime_continuity_facet_counts: Vec::new(),
                interesting_points: Vec::new(),
            },
            ReleaseReviewScenarioFocusDiagnostic {
                scenario_id: "us_early_90s_banking_stress".to_string(),
                name: "1990-1993 美国银行与衰退压力".to_string(),
                outcome: "missed_to_missed".to_string(),
                window_start: crisis_start,
                window_end: crisis_start,
                crisis_start,
                crisis_end: crisis_start,
                baseline_first_l2_date: None,
                candidate_first_l2_date: None,
                baseline_first_l3_date: None,
                candidate_first_l3_date: None,
                baseline_first_non_normal_date: None,
                candidate_first_non_normal_date: None,
                baseline_actionable_point_count: 0,
                candidate_actionable_point_count: 0,
                baseline_runtime_floor_hit_point_count: 5,
                candidate_runtime_floor_hit_point_count: 5,
                baseline_max_p20d: None,
                candidate_max_p20d: None,
                baseline_max_p60d: None,
                candidate_max_p60d: None,
                baseline_first_runtime_floor_hit_without_l3_date: None,
                candidate_first_runtime_floor_hit_without_l3_date: None,
                baseline_first_runtime_floor_hit_without_l3_reason: None,
                candidate_first_runtime_floor_hit_without_l3_reason: None,
                baseline_primary_failure_mode: Some("posture_continuity_failure".to_string()),
                candidate_primary_failure_mode: Some("posture_continuity_failure".to_string()),
                dominant_runtime_blocks: ReleaseReviewRuntimeDominantCategories {
                    baseline_categories: vec!["posture_bucket_normal".to_string()],
                    baseline_count: 5,
                    candidate_categories: vec!["posture_bucket_normal".to_string()],
                    candidate_count: 5,
                },
                dominant_runtime_continuity_facets: ReleaseReviewRuntimeDominantCategories {
                    baseline_categories: vec!["posture:normal".to_string()],
                    baseline_count: 5,
                    candidate_categories: vec!["posture:normal".to_string()],
                    candidate_count: 5,
                },
                runtime_block_counts: Vec::new(),
                runtime_continuity_facet_counts: Vec::new(),
                interesting_points: Vec::new(),
            },
            ReleaseReviewScenarioFocusDiagnostic {
                scenario_id: "us_regional_banks_2023".to_string(),
                name: "2023 美国区域银行危机".to_string(),
                outcome: "timely_to_missed".to_string(),
                window_start: crisis_start,
                window_end: crisis_start,
                crisis_start,
                crisis_end: crisis_start,
                baseline_first_l2_date: None,
                candidate_first_l2_date: None,
                baseline_first_l3_date: None,
                candidate_first_l3_date: None,
                baseline_first_non_normal_date: None,
                candidate_first_non_normal_date: None,
                baseline_actionable_point_count: 0,
                candidate_actionable_point_count: 0,
                baseline_runtime_floor_hit_point_count: 1,
                candidate_runtime_floor_hit_point_count: 0,
                baseline_max_p20d: None,
                candidate_max_p20d: None,
                baseline_max_p60d: None,
                candidate_max_p60d: None,
                baseline_first_runtime_floor_hit_without_l3_date: None,
                candidate_first_runtime_floor_hit_without_l3_date: None,
                baseline_first_runtime_floor_hit_without_l3_reason: None,
                candidate_first_runtime_floor_hit_without_l3_reason: None,
                baseline_primary_failure_mode: Some("residual_review_l3_failure".to_string()),
                candidate_primary_failure_mode: Some("score_confirmation_failure".to_string()),
                dominant_runtime_blocks: ReleaseReviewRuntimeDominantCategories {
                    baseline_categories: vec!["review_l3_gate_not_satisfied".to_string()],
                    baseline_count: 1,
                    candidate_categories: vec!["prepare_score_low".to_string()],
                    candidate_count: 1,
                },
                dominant_runtime_continuity_facets: ReleaseReviewRuntimeDominantCategories {
                    baseline_categories: vec!["confirmation:ok_or_not_needed".to_string()],
                    baseline_count: 1,
                    candidate_categories: vec!["confirmation:prepare_score_low".to_string()],
                    candidate_count: 1,
                },
                runtime_block_counts: Vec::new(),
                runtime_continuity_facet_counts: Vec::new(),
                interesting_points: Vec::new(),
            },
        ]);

        assert_eq!(summary.len(), 2);
        assert_eq!(summary[0].scenario_id, "us_dotcom_unwind_2000");
        assert_eq!(
            summary[0].primary_workstream,
            "strict_review_vs_runtime_mapping"
        );
        assert_eq!(summary[0].training_role, "candidate_optional");
        assert!(summary[0].protected_window);
        assert!(summary[0]
            .suggested_review
            .contains("strict review gate 与 runtime floor"));

        assert_eq!(summary[1].scenario_id, "us_early_90s_banking_stress");
        assert_eq!(summary[1].primary_workstream, "posture_continuity");
        assert_eq!(summary[1].training_role, "extension_only");
        assert!(summary[1]
            .suggested_review
            .contains("prepare/months 连续性"));
    }

    #[test]
    fn release_review_historical_audit_workstreams_group_priorities() {
        let rows = summarize_release_review_historical_audit_workstreams(&[
            ReleaseReviewHistoricalAuditPriority {
                scenario_id: "us_dotcom_unwind_2000".to_string(),
                scenario_name: "2000-2001 科网泡沫出清".to_string(),
                scenario_family: "mixed_systemic_stress".to_string(),
                training_role: "candidate_optional".to_string(),
                protected_window: true,
                baseline_failure_mode: "strict_gate_mismatch".to_string(),
                candidate_failure_mode: "strict_gate_mismatch".to_string(),
                primary_workstream: "strict_review_vs_runtime_mapping".to_string(),
                suggested_review: "复核 strict review gate 与 runtime floor 的映射".to_string(),
            },
            ReleaseReviewHistoricalAuditPriority {
                scenario_id: "us_early_90s_banking_stress".to_string(),
                scenario_name: "1990-1993 美国银行与衰退压力".to_string(),
                scenario_family: "mixed_systemic_stress".to_string(),
                training_role: "extension_only".to_string(),
                protected_window: true,
                baseline_failure_mode: "posture_continuity_failure".to_string(),
                candidate_failure_mode: "posture_continuity_failure".to_string(),
                primary_workstream: "posture_continuity".to_string(),
                suggested_review: "复核 prepare/months 连续性".to_string(),
            },
            ReleaseReviewHistoricalAuditPriority {
                scenario_id: "us_funding_stress_2011".to_string(),
                scenario_name: "2011 美欧融资压力".to_string(),
                scenario_family: "mixed_systemic_stress".to_string(),
                training_role: "extension_only".to_string(),
                protected_window: true,
                baseline_failure_mode: "posture_continuity_failure".to_string(),
                candidate_failure_mode: "score_confirmation_failure".to_string(),
                primary_workstream: "posture_continuity".to_string(),
                suggested_review: "复核 prepare/months 连续性".to_string(),
            },
        ]);

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].workstream, "strict_review_vs_runtime_mapping");
        assert_eq!(rows[0].scenario_count, 1);
        assert_eq!(rows[0].protected_count, 1);
        assert_eq!(
            rows[0].scenarios,
            vec!["2000-2001 科网泡沫出清".to_string()]
        );
        let posture = rows
            .iter()
            .find(|row| row.workstream == "posture_continuity")
            .expect("posture workstream row");
        assert_eq!(posture.scenario_count, 2);
        assert_eq!(posture.protected_count, 2);
        assert_eq!(
            posture.scenario_families,
            vec!["mixed_systemic_stress".to_string()]
        );
        assert_eq!(posture.training_roles, vec!["extension_only".to_string()]);
        assert!(posture
            .scenarios
            .contains(&"1990-1993 美国银行与衰退压力".to_string()));
        assert!(posture.scenarios.contains(&"2011 美欧融资压力".to_string()));
    }

    #[test]
    fn release_review_historical_audit_attribution_distinguishes_shared_and_regression() {
        let rows = summarize_release_review_historical_audit_attribution(&[
            ReleaseReviewHistoricalAuditPriority {
                scenario_id: "us_dotcom_unwind_2000".to_string(),
                scenario_name: "2000-2001 科网泡沫出清".to_string(),
                scenario_family: "mixed_systemic_stress".to_string(),
                training_role: "candidate_optional".to_string(),
                protected_window: true,
                baseline_failure_mode: "strict_gate_mismatch".to_string(),
                candidate_failure_mode: "strict_gate_mismatch".to_string(),
                primary_workstream: "strict_review_vs_runtime_mapping".to_string(),
                suggested_review: "复核 strict review gate 与 runtime floor 的映射".to_string(),
            },
            ReleaseReviewHistoricalAuditPriority {
                scenario_id: "us_early_90s_banking_stress".to_string(),
                scenario_name: "1990-1993 美国银行与衰退压力".to_string(),
                scenario_family: "mixed_systemic_stress".to_string(),
                training_role: "extension_only".to_string(),
                protected_window: true,
                baseline_failure_mode: "posture_continuity_failure".to_string(),
                candidate_failure_mode: "—".to_string(),
                primary_workstream: "posture_continuity".to_string(),
                suggested_review: "复核 prepare/months 连续性".to_string(),
            },
            ReleaseReviewHistoricalAuditPriority {
                scenario_id: "us_regional_banks_2023".to_string(),
                scenario_name: "2023 美国区域银行危机".to_string(),
                scenario_family: "banking_crisis".to_string(),
                training_role: "mandatory".to_string(),
                protected_window: true,
                baseline_failure_mode: "residual_review_l3_failure".to_string(),
                candidate_failure_mode: "score_confirmation_failure".to_string(),
                primary_workstream: "score_confirmation".to_string(),
                suggested_review: "复核 months/prepare 的 score confirmation".to_string(),
            },
        ]);

        assert_eq!(rows.len(), 3);

        let shared = rows
            .iter()
            .find(|row| row.attribution == "both_baseline_and_candidate")
            .expect("shared row");
        assert_eq!(shared.workstream, "strict_review_vs_runtime_mapping");
        assert_eq!(shared.baseline_count, 1);
        assert_eq!(shared.candidate_count, 1);
        assert!(shared.explanation.contains("共性短板"));

        let baseline_only = rows
            .iter()
            .find(|row| row.attribution == "baseline_shared_weakness")
            .expect("baseline-only row");
        assert_eq!(baseline_only.workstream, "posture_continuity");
        assert_eq!(baseline_only.baseline_count, 1);
        assert_eq!(baseline_only.candidate_count, 0);
        assert!(baseline_only.explanation.contains("既有短板"));

        let regression = rows
            .iter()
            .find(|row| row.attribution == "candidate_regression")
            .expect("candidate regression row");
        assert_eq!(regression.workstream, "score_confirmation");
        assert_eq!(regression.baseline_count, 0);
        assert_eq!(regression.candidate_count, 1);
        assert!(regression.explanation.contains("自己退化出来"));
    }

    #[test]
    fn release_review_historical_audit_actions_translate_attribution_to_next_step() {
        let actions = summarize_release_review_historical_audit_actions(&[
            ReleaseReviewHistoricalAuditAttributionSummary {
                workstream: "score_confirmation".to_string(),
                attribution: "candidate_regression".to_string(),
                scenario_count: 1,
                protected_count: 1,
                baseline_count: 0,
                candidate_count: 1,
                baseline_scenarios: Vec::new(),
                candidate_scenarios: vec!["2023 美国区域银行危机".to_string()],
                explanation: "candidate regression".to_string(),
            },
            ReleaseReviewHistoricalAuditAttributionSummary {
                workstream: "strict_review_vs_runtime_mapping".to_string(),
                attribution: "both_baseline_and_candidate".to_string(),
                scenario_count: 2,
                protected_count: 2,
                baseline_count: 2,
                candidate_count: 2,
                baseline_scenarios: vec![
                    "2000-2001 科网泡沫出清".to_string(),
                    "2011 美欧融资压力".to_string(),
                ],
                candidate_scenarios: vec![
                    "2000-2001 科网泡沫出清".to_string(),
                    "2011 美欧融资压力".to_string(),
                ],
                explanation: "shared blocker".to_string(),
            },
            ReleaseReviewHistoricalAuditAttributionSummary {
                workstream: "posture_continuity".to_string(),
                attribution: "baseline_shared_weakness".to_string(),
                scenario_count: 1,
                protected_count: 1,
                baseline_count: 1,
                candidate_count: 0,
                baseline_scenarios: vec!["1990-1993 美国银行与衰退压力".to_string()],
                candidate_scenarios: Vec::new(),
                explanation: "baseline weakness".to_string(),
            },
        ]);

        assert_eq!(actions.len(), 3);
        let candidate = actions
            .iter()
            .find(|row| row.action_type == "candidate_reject_or_retrain")
            .expect("candidate regression action");
        assert_eq!(candidate.workstream, "score_confirmation");
        assert!(candidate.recommendation.contains("不具备晋升条件"));

        let shared = actions
            .iter()
            .find(|row| row.action_type == "shared_blocker_fix_before_promotion")
            .expect("shared blocker action");
        assert_eq!(shared.workstream, "strict_review_vs_runtime_mapping");
        assert!(shared.recommendation.contains("晋升前置 blocker"));

        let baseline = actions
            .iter()
            .find(|row| row.action_type == "baseline_research_fix")
            .expect("baseline research action");
        assert_eq!(baseline.workstream, "posture_continuity");
        assert!(baseline.recommendation.contains("formal main 研究修复"));
    }

    #[test]
    fn release_review_historical_audit_takeaways_explain_primary_workstreams() {
        let takeaways = summarize_release_review_historical_audit_workstreams(&[
            ReleaseReviewHistoricalAuditPriority {
                scenario_id: "us_dotcom_unwind_2000".to_string(),
                scenario_name: "2000-2001 科网泡沫出清".to_string(),
                scenario_family: "mixed_systemic_stress".to_string(),
                training_role: "candidate_optional".to_string(),
                protected_window: true,
                baseline_failure_mode: "strict_gate_mismatch".to_string(),
                candidate_failure_mode: "strict_gate_mismatch".to_string(),
                primary_workstream: "strict_review_vs_runtime_mapping".to_string(),
                suggested_review: "复核 strict review gate 与 runtime floor 的映射".to_string(),
            },
            ReleaseReviewHistoricalAuditPriority {
                scenario_id: "us_early_90s_banking_stress".to_string(),
                scenario_name: "1990-1993 美国银行与衰退压力".to_string(),
                scenario_family: "mixed_systemic_stress".to_string(),
                training_role: "extension_only".to_string(),
                protected_window: true,
                baseline_failure_mode: "posture_continuity_failure".to_string(),
                candidate_failure_mode: "posture_continuity_failure".to_string(),
                primary_workstream: "posture_continuity".to_string(),
                suggested_review: "复核 prepare/months 连续性".to_string(),
            },
        ]);
        let rendered = release_review_historical_audit_takeaways(&takeaways);

        assert_eq!(rendered.len(), 2);
        assert!(rendered
            .iter()
            .any(|row| row.contains("strict review gate 与 runtime floor")));
        assert!(rendered
            .iter()
            .any(|row| row.contains("高 p20d/p60d 仍长期停在 normal")));
    }

    #[test]
    fn release_review_structured_signal_counts_distinguish_strict_and_runtime_hits() {
        let crisis_start = NaiveDate::from_ymd_opt(2023, 3, 10).unwrap();
        let backtests = vec![synthetic_backtest_summary_with_dates(
            "scenario_structural",
            "Structural Only",
            Some(NaiveDate::from_ymd_opt(2023, 2, 20).unwrap()),
            None,
            Some(18),
            None,
            0,
        )];
        let history = vec![
            runtime_history_point_with_state(
                NaiveDate::from_ymd_opt(2023, 2, 10).unwrap(),
                52.0,
                0.02,
                0.08,
                0.14,
                DecisionPosture::Normal,
                TimeToRiskBucket::Normal,
                41.0,
                &[],
            ),
            runtime_history_point_with_state(
                NaiveDate::from_ymd_opt(2023, 2, 20).unwrap(),
                54.0,
                0.02,
                0.09,
                0.16,
                DecisionPosture::Normal,
                TimeToRiskBucket::Normal,
                42.0,
                &[],
            ),
            runtime_history_point_with_state(
                crisis_start,
                60.0,
                0.05,
                0.21,
                0.32,
                DecisionPosture::Normal,
                TimeToRiskBucket::Normal,
                44.0,
                &[],
            ),
        ];
        let method = formal_main_audit_method_wire();

        let (strict_actionable_point_count, runtime_floor_hit_count) =
            release_review_structured_signal_counts(&backtests, &history, &method);

        assert_eq!(strict_actionable_point_count, 0);
        assert_eq!(runtime_floor_hit_count, 2);
    }

    #[test]
    fn release_review_runtime_separation_comparison_highlights_60d_floor_gap() {
        let baseline = ReleaseRuntimeReviewDiagnostics {
            release_id: "baseline".to_string(),
            history_point_count: 120,
            posture_distribution: Vec::new(),
            time_bucket_distribution: Vec::new(),
            posture_trigger_distribution: Vec::new(),
            posture_blocker_distribution: Vec::new(),
            regime_probability_summaries: Vec::new(),
            regime_separation_summaries: vec![ReleaseRuntimeSeparationSummary {
                horizon_days: 60,
                early_warning_regime: "pre_warning_buffer".to_string(),
                normal_avg_probability: 0.28,
                pre_warning_buffer_avg_probability: 0.52,
                positive_window_avg_probability: 0.61,
                in_crisis_avg_probability: 0.66,
                post_crisis_cooldown_avg_probability: 0.35,
                early_warning_raw_lift_vs_normal: Some(1.92),
                early_warning_calibrated_lift_vs_normal: Some(1.86),
                early_warning_gap_retention: Some(0.81),
                positive_window_calibrated_lift_vs_normal: Some(2.18),
                positive_window_gap_vs_normal: Some(0.33),
                in_crisis_raw_lift_vs_normal: Some(2.36),
                in_crisis_calibrated_lift_vs_normal: Some(2.36),
                post_crisis_cooldown_calibrated_lift_vs_normal: Some(1.25),
                post_crisis_cooldown_gap_vs_normal: Some(0.07),
                max_non_normal_calibrated_lift_vs_normal: Some(2.36),
                max_non_normal_threshold_hit_rate: Some(0.0),
                diagnosis: "separated_but_below_runtime_floor".to_string(),
            }],
            runtime_thresholds: Some(RuntimeThresholdDiagnosticsWire {
                prepare_p60d: 0.65,
                hedge_p20d: 0.07,
                defend_p5d: 0.03,
                severe_now_p20d: 0.27,
                elevated_weeks_p60d: 0.20,
                external_prepare_p20d: 0.05,
                carry_prepare_p60d: 0.08,
                downgrade_prepare_p60d: 0.075,
                downgrade_hedge_p20d: 0.053,
                downgrade_defend_p5d: 0.02,
                history_runtime_policy_version: "runtime_history_test".to_string(),
            }),
            points_at_or_above_prepare_p60d: Some(0),
            points_at_or_above_hedge_p20d: Some(14),
            points_at_or_above_defend_p5d: Some(6),
            note: "test".to_string(),
        };
        let candidate = ReleaseRuntimeReviewDiagnostics {
            release_id: "candidate".to_string(),
            history_point_count: 120,
            posture_distribution: Vec::new(),
            time_bucket_distribution: Vec::new(),
            posture_trigger_distribution: Vec::new(),
            posture_blocker_distribution: Vec::new(),
            regime_probability_summaries: Vec::new(),
            regime_separation_summaries: vec![ReleaseRuntimeSeparationSummary {
                horizon_days: 60,
                early_warning_regime: "pre_warning_buffer".to_string(),
                normal_avg_probability: 0.24,
                pre_warning_buffer_avg_probability: 0.58,
                positive_window_avg_probability: 0.64,
                in_crisis_avg_probability: 0.69,
                post_crisis_cooldown_avg_probability: 0.30,
                early_warning_raw_lift_vs_normal: Some(2.48),
                early_warning_calibrated_lift_vs_normal: Some(2.42),
                early_warning_gap_retention: Some(0.88),
                positive_window_calibrated_lift_vs_normal: Some(2.67),
                positive_window_gap_vs_normal: Some(0.40),
                in_crisis_raw_lift_vs_normal: Some(2.88),
                in_crisis_calibrated_lift_vs_normal: Some(2.88),
                post_crisis_cooldown_calibrated_lift_vs_normal: Some(1.25),
                post_crisis_cooldown_gap_vs_normal: Some(0.06),
                max_non_normal_calibrated_lift_vs_normal: Some(2.88),
                max_non_normal_threshold_hit_rate: Some(0.12),
                diagnosis: "usable_early_warning_separation".to_string(),
            }],
            runtime_thresholds: Some(RuntimeThresholdDiagnosticsWire {
                prepare_p60d: 0.45,
                hedge_p20d: 0.07,
                defend_p5d: 0.03,
                severe_now_p20d: 0.27,
                elevated_weeks_p60d: 0.20,
                external_prepare_p20d: 0.05,
                carry_prepare_p60d: 0.08,
                downgrade_prepare_p60d: 0.075,
                downgrade_hedge_p20d: 0.053,
                downgrade_defend_p5d: 0.02,
                history_runtime_policy_version: "runtime_history_test".to_string(),
            }),
            points_at_or_above_prepare_p60d: Some(9),
            points_at_or_above_hedge_p20d: Some(16),
            points_at_or_above_defend_p5d: Some(6),
            note: "test".to_string(),
        };

        let rows = build_release_review_runtime_separation_comparisons(&baseline, &candidate);

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].horizon_days, 60);
        assert_eq!(
            rows[0].baseline_diagnosis,
            "separated_but_below_runtime_floor"
        );
        assert_eq!(
            rows[0].candidate_diagnosis,
            "usable_early_warning_separation"
        );
        assert_eq!(rows[0].baseline_threshold, Some(0.65));
        assert_eq!(rows[0].candidate_threshold, Some(0.45));
        assert_eq!(rows[0].baseline_early_warning_avg_probability, Some(0.52));
        assert_eq!(rows[0].candidate_early_warning_avg_probability, Some(0.58));
        assert_eq!(rows[0].baseline_floor_gap, Some(-0.13));
        assert_eq!(rows[0].candidate_floor_gap, Some(0.13));
        assert_eq!(rows[0].baseline_threshold_hit_rate, Some(0.0));
        assert_eq!(rows[0].candidate_threshold_hit_rate, Some(0.12));
    }

    #[test]
    fn release_review_runtime_separation_takeaways_explain_floor_gap() {
        let rows = vec![ReleaseReviewRuntimeSeparationComparison {
            horizon_days: 60,
            baseline_diagnosis: "usable_early_warning_separation".to_string(),
            candidate_diagnosis: "separated_but_below_runtime_floor".to_string(),
            baseline_threshold: Some(0.45),
            candidate_threshold: Some(0.65),
            baseline_early_warning_regime: "pre_warning_buffer".to_string(),
            candidate_early_warning_regime: "pre_warning_buffer".to_string(),
            baseline_early_warning_avg_probability: Some(0.58),
            candidate_early_warning_avg_probability: Some(0.52),
            baseline_normal_avg_probability: Some(0.24),
            candidate_normal_avg_probability: Some(0.28),
            baseline_early_warning_gap_vs_normal: Some(0.34),
            candidate_early_warning_gap_vs_normal: Some(0.24),
            baseline_floor_gap: Some(0.13),
            candidate_floor_gap: Some(-0.13),
            baseline_early_warning_lift_vs_normal: Some(2.42),
            candidate_early_warning_lift_vs_normal: Some(1.86),
            baseline_threshold_hit_rate: Some(0.12),
            candidate_threshold_hit_rate: Some(0.0),
        }];

        let takeaways = release_review_runtime_separation_takeaways(&rows);

        assert_eq!(takeaways.len(), 1);
        assert!(takeaways[0].contains("60d"));
        assert!(takeaways[0].contains("runtime floor"));
        assert!(takeaways[0].contains("阈值 / runtime policy 瓶颈"));
    }

    #[test]
    fn probability_decision_threshold_keeps_5d_floor_conservative() {
        let probabilities = vec![0.0086, 0.0082, 0.0079, 0.0034, 0.0028, 0.0021];
        let labels = vec![1.0, 1.0, 1.0, 0.0, 0.0, 0.0];

        let threshold = select_probability_decision_threshold(&probabilities, &labels, 5);

        assert_eq!(threshold, 0.03);
    }

    #[test]
    fn actionability_guardrails_flag_narrow_or_zero_hit_reviews() {
        let review = ReleaseActionabilityReview {
            release_id: "candidate".to_string(),
            enabled: true,
            model_version: Some("actionability_bundle_test".to_string()),
            calibration_version: Some("actionability_platt_test".to_string()),
            fusion_policy_version: Some("fusion_policy_test".to_string()),
            levels: vec![ReleaseActionabilityLevelReview {
                level: ActionabilityLevel::Prepare,
                proxy_horizon_days: 60,
                sample_count: 100,
                positive_rate: 0.03,
                threshold: 0.3,
                predicted_positive_count: 0,
                primary_positive_count: 12,
                late_validation_row_count: 4,
                protected_row_count: 0,
                primary_hit_count: 0,
                late_validation_hit_count: 0,
                protected_hit_count: 0,
                false_positive_count: 0,
                scenario_count: 1,
                on_time_scenario_count: 0,
                late_only_scenario_count: 0,
                missed_scenario_count: 1,
                precision_at_threshold: None,
                primary_recall_at_threshold: Some(0.0),
                late_validation_capture_rate: Some(0.0),
                on_time_rate: Some(0.0),
                late_only_rate: Some(0.0),
                missed_rate: Some(1.0),
                note: "test".to_string(),
            }],
            guard_regressions: Vec::new(),
            guard_passed: true,
            note: "test".to_string(),
        };

        let regressions = compare_actionability_guardrails(&review);
        assert!(regressions
            .iter()
            .any(|item| item.contains("scenario_count")));
        assert!(regressions
            .iter()
            .any(|item| item.contains("produced no primary or late-validation hits")));
        assert!(regressions.iter().any(|item| item.contains("on_time_rate")));
        assert!(regressions.iter().any(|item| item.contains("missed_rate")));
    }

    #[test]
    fn actionability_guardrails_apply_level_specific_rate_thresholds() {
        let review = ReleaseActionabilityReview {
            release_id: "candidate".to_string(),
            enabled: true,
            model_version: Some("actionability_bundle_test".to_string()),
            calibration_version: Some("actionability_platt_test".to_string()),
            fusion_policy_version: Some("fusion_policy_test".to_string()),
            levels: vec![ReleaseActionabilityLevelReview {
                level: ActionabilityLevel::Defend,
                proxy_horizon_days: 5,
                sample_count: 120,
                positive_rate: 0.04,
                threshold: 0.12,
                predicted_positive_count: 5,
                primary_positive_count: 10,
                late_validation_row_count: 7,
                protected_row_count: 0,
                primary_hit_count: 1,
                late_validation_hit_count: 3,
                protected_hit_count: 0,
                false_positive_count: 1,
                scenario_count: 3,
                on_time_scenario_count: 0,
                late_only_scenario_count: 2,
                missed_scenario_count: 1,
                precision_at_threshold: Some(0.2),
                primary_recall_at_threshold: Some(0.33),
                late_validation_capture_rate: Some(0.43),
                on_time_rate: Some(0.0),
                late_only_rate: Some(0.67),
                missed_rate: Some(0.33),
                note: "test".to_string(),
            }],
            guard_regressions: Vec::new(),
            guard_passed: true,
            note: "test".to_string(),
        };

        let regressions = compare_actionability_guardrails(&review);
        assert!(regressions
            .iter()
            .any(|item| item.contains("late_only_rate")));
        assert!(!regressions.iter().any(|item| item.contains("on_time_rate")));
    }

    #[test]
    fn scenario_aware_split_spreads_adjacent_scenarios_across_calibration_and_evaluation() {
        let mut rows = (0..180)
            .map(|index| {
                let scenario_id = match index {
                    40..=59 => Some("scenario_a"),
                    90..=109 => Some("scenario_b"),
                    140..=159 => Some("scenario_c"),
                    _ => None,
                };
                FormalDatasetRowRecord {
                    dataset_key: "dataset".to_string(),
                    split_name: String::new(),
                    entity_id: "us".to_string(),
                    market_scope: "financial_system".to_string(),
                    as_of_date: NaiveDate::from_ymd_opt(2000, 1, 1)
                        .unwrap()
                        .checked_add_signed(chrono::Duration::days(index as i64))
                        .unwrap(),
                    point_in_time_mode: "best_effort".to_string(),
                    latest_visible_at: None,
                    coverage_score: 1.0,
                    core_feature_coverage: 1.0,
                    trigger_feature_coverage: 1.0,
                    external_feature_coverage: 1.0,
                    sample_quality_grade: "a".to_string(),
                    primary_scenario_id: scenario_id.map(str::to_string),
                    scenario_family: scenario_id
                        .map(|_| "systemic_credit_banking_crisis".to_string()),
                    scenario_training_role: scenario_id.map(|_| "mandatory".to_string()),
                    label_5d: u8::from(matches!(index, 56..=59 | 106..=109 | 156..=159)),
                    label_20d: u8::from(matches!(index, 52..=59 | 102..=109 | 152..=159)),
                    label_60d: u8::from(matches!(index, 44..=59 | 94..=109 | 144..=159)),
                    regime_5d: "normal".to_string(),
                    regime_20d: "normal".to_string(),
                    regime_60d: "normal".to_string(),
                    action_label_5d: u8::from(matches!(index, 55..=59 | 105..=109 | 155..=159)),
                    action_label_20d: u8::from(matches!(index, 50..=59 | 100..=109 | 150..=159)),
                    action_label_60d: u8::from(matches!(index, 42..=59 | 92..=109 | 142..=159)),
                    prepare_episode_label: u8::from(
                        matches!(index, 42..=59 | 92..=109 | 142..=159),
                    ),
                    hedge_episode_label: u8::from(matches!(index, 50..=59 | 100..=109 | 150..=159)),
                    defend_episode_label: u8::from(
                        matches!(index, 55..=59 | 105..=109 | 155..=159),
                    ),
                    primary_action_level: None,
                    action_episode_id: None,
                    action_episode_phase: "outside".to_string(),
                    protected_action_window: false,
                    features: BTreeMap::new(),
                    created_at: Utc::now(),
                }
            })
            .collect::<Vec<_>>();

        let ranges = vec![
            ScenarioRowRange {
                scenario_id: "scenario_a".to_string(),
                family: "systemic_credit_banking_crisis".to_string(),
                start_index: 40,
                end_index: 59,
            },
            ScenarioRowRange {
                scenario_id: "scenario_b".to_string(),
                family: "systemic_credit_banking_crisis".to_string(),
                start_index: 90,
                end_index: 109,
            },
            ScenarioRowRange {
                scenario_id: "scenario_c".to_string(),
                family: "systemic_credit_banking_crisis".to_string(),
                start_index: 140,
                end_index: 159,
            },
        ];

        let split_requirements = formal_dataset_split_requirements("formal_label_v1_main");
        let (train_end, calibration_end) =
            scenario_aware_formal_split_bounds(&rows, &ranges, split_requirements).unwrap();
        assert!((56..=59).contains(&train_end));
        assert!((106..=109).contains(&calibration_end));

        for (index, row) in rows.iter_mut().enumerate() {
            row.split_name = if index < train_end {
                "train".to_string()
            } else if index < calibration_end {
                "calibration".to_string()
            } else {
                "evaluation".to_string()
            };
        }

        let calibration_scenarios = rows
            .iter()
            .filter(|row| row.split_name == "calibration")
            .filter_map(|row| row.primary_scenario_id.as_deref())
            .collect::<BTreeSet<_>>();
        let evaluation_scenarios = rows
            .iter()
            .filter(|row| row.split_name == "evaluation")
            .filter_map(|row| row.primary_scenario_id.as_deref())
            .collect::<BTreeSet<_>>();

        assert_eq!(calibration_scenarios.len(), 2);
        assert!(calibration_scenarios.contains("scenario_a"));
        assert!(calibration_scenarios.contains("scenario_b"));
        assert_eq!(evaluation_scenarios.len(), 2);
        assert!(evaluation_scenarios.contains("scenario_b"));
        assert!(evaluation_scenarios.contains("scenario_c"));
        assert_eq!(
            scenario_count_for_index_range(&rows, train_end, calibration_end),
            2
        );
        assert_eq!(
            scenario_count_for_index_range(&rows, calibration_end, rows.len()),
            2
        );

        let label_support = FormalSplitLabelSupport::from_rows(&rows);
        assert!(label_support.split_has_required_label_support(0, train_end, split_requirements));
        assert!(label_support.split_has_required_label_support(
            train_end,
            calibration_end,
            split_requirements
        ));
        assert!(label_support.split_has_required_label_support(
            calibration_end,
            rows.len(),
            split_requirements
        ));
    }

    #[test]
    fn extension_acute_split_allows_two_scenarios_with_single_scenario_evaluation() {
        let rows = (0..220)
            .map(|index| {
                let scenario_id = match index {
                    40..=69 => Some("acute_a"),
                    150..=179 => Some("acute_b"),
                    _ => None,
                };
                FormalDatasetRowRecord {
                    dataset_key: "dataset".to_string(),
                    split_name: String::new(),
                    entity_id: "us".to_string(),
                    market_scope: "financial_system".to_string(),
                    as_of_date: NaiveDate::from_ymd_opt(2000, 1, 1)
                        .unwrap()
                        .checked_add_signed(chrono::Duration::days(index as i64))
                        .unwrap(),
                    point_in_time_mode: "best_effort".to_string(),
                    latest_visible_at: None,
                    coverage_score: 1.0,
                    core_feature_coverage: 1.0,
                    trigger_feature_coverage: 1.0,
                    external_feature_coverage: 1.0,
                    sample_quality_grade: "a".to_string(),
                    primary_scenario_id: scenario_id.map(str::to_string),
                    scenario_family: scenario_id
                        .map(|_| "acute_market_liquidity_crash".to_string()),
                    scenario_training_role: scenario_id.map(|_| "extension_only".to_string()),
                    label_5d: u8::from(matches!(index, 62..=69 | 172..=179)),
                    label_20d: u8::from(matches!(index, 50..=69 | 160..=179)),
                    label_60d: 0,
                    regime_5d: "normal".to_string(),
                    regime_20d: "normal".to_string(),
                    regime_60d: "normal".to_string(),
                    action_label_5d: u8::from(matches!(index, 62..=69 | 172..=179)),
                    action_label_20d: u8::from(matches!(index, 50..=69 | 160..=179)),
                    action_label_60d: 0,
                    prepare_episode_label: u8::from(matches!(index, 48..=69 | 158..=179)),
                    hedge_episode_label: u8::from(matches!(index, 56..=69 | 166..=179)),
                    defend_episode_label: u8::from(matches!(index, 62..=69 | 172..=179)),
                    primary_action_level: None,
                    action_episode_id: None,
                    action_episode_phase: "outside".to_string(),
                    protected_action_window: false,
                    features: BTreeMap::new(),
                    created_at: Utc::now(),
                }
            })
            .collect::<Vec<_>>();

        let ranges = vec![
            ScenarioRowRange {
                scenario_id: "acute_a".to_string(),
                family: "acute_market_liquidity_crash".to_string(),
                start_index: 40,
                end_index: 69,
            },
            ScenarioRowRange {
                scenario_id: "acute_b".to_string(),
                family: "acute_market_liquidity_crash".to_string(),
                start_index: 150,
                end_index: 179,
            },
        ];

        let split_requirements = formal_dataset_split_requirements("formal_label_v1_ext_acute");
        let (train_end, calibration_end) =
            scenario_aware_formal_split_bounds(&rows, &ranges, split_requirements).unwrap();

        assert!((62..=69).contains(&train_end));
        assert!((172..=179).contains(&calibration_end));

        let label_support = FormalSplitLabelSupport::from_rows(&rows);
        assert!(label_support.split_has_required_label_support(0, train_end, split_requirements));
        assert!(label_support.split_has_required_label_support(
            train_end,
            calibration_end,
            split_requirements
        ));
        assert!(label_support.split_has_required_label_support(
            calibration_end,
            rows.len(),
            split_requirements
        ));

        assert_eq!(
            scenario_count_for_index_range(&rows, calibration_end, rows.len()),
            1
        );
    }

    #[test]
    fn extension_stress_split_uses_20d_60d_prepare_hedge_requirements() {
        let rows = (0..260)
            .map(|index| {
                let scenario_id = match index {
                    30..=59 => Some("stress_a"),
                    90..=119 => Some("stress_b"),
                    150..=179 => Some("stress_c"),
                    210..=239 => Some("stress_d"),
                    _ => None,
                };
                FormalDatasetRowRecord {
                    dataset_key: "dataset".to_string(),
                    split_name: String::new(),
                    entity_id: "us".to_string(),
                    market_scope: "financial_system".to_string(),
                    as_of_date: NaiveDate::from_ymd_opt(2000, 1, 1)
                        .unwrap()
                        .checked_add_signed(chrono::Duration::days(index as i64))
                        .unwrap(),
                    point_in_time_mode: "best_effort".to_string(),
                    latest_visible_at: None,
                    coverage_score: 1.0,
                    core_feature_coverage: 1.0,
                    trigger_feature_coverage: 1.0,
                    external_feature_coverage: 1.0,
                    sample_quality_grade: "a".to_string(),
                    primary_scenario_id: scenario_id.map(str::to_string),
                    scenario_family: scenario_id.map(|_| "mixed_systemic_stress".to_string()),
                    scenario_training_role: scenario_id.map(|_| "extension_only".to_string()),
                    label_5d: 0,
                    label_20d: u8::from(
                        matches!(index, 42..=59 | 102..=119 | 162..=179 | 222..=239),
                    ),
                    label_60d: u8::from(
                        matches!(index, 34..=59 | 94..=119 | 154..=179 | 214..=239),
                    ),
                    regime_5d: "normal".to_string(),
                    regime_20d: "normal".to_string(),
                    regime_60d: "normal".to_string(),
                    action_label_5d: 0,
                    action_label_20d: u8::from(
                        matches!(index, 42..=59 | 102..=119 | 162..=179 | 222..=239),
                    ),
                    action_label_60d: u8::from(
                        matches!(index, 34..=59 | 94..=119 | 154..=179 | 214..=239),
                    ),
                    prepare_episode_label: u8::from(
                        matches!(index, 34..=59 | 94..=119 | 154..=179 | 214..=239),
                    ),
                    hedge_episode_label: u8::from(
                        matches!(index, 42..=59 | 102..=119 | 162..=179 | 222..=239),
                    ),
                    defend_episode_label: 0,
                    primary_action_level: None,
                    action_episode_id: None,
                    action_episode_phase: "outside".to_string(),
                    protected_action_window: true,
                    features: BTreeMap::new(),
                    created_at: Utc::now(),
                }
            })
            .collect::<Vec<_>>();

        let ranges = vec![
            ScenarioRowRange {
                scenario_id: "stress_a".to_string(),
                family: "mixed_systemic_stress".to_string(),
                start_index: 30,
                end_index: 59,
            },
            ScenarioRowRange {
                scenario_id: "stress_b".to_string(),
                family: "mixed_systemic_stress".to_string(),
                start_index: 90,
                end_index: 119,
            },
            ScenarioRowRange {
                scenario_id: "stress_c".to_string(),
                family: "mixed_systemic_stress".to_string(),
                start_index: 150,
                end_index: 179,
            },
            ScenarioRowRange {
                scenario_id: "stress_d".to_string(),
                family: "mixed_systemic_stress".to_string(),
                start_index: 210,
                end_index: 239,
            },
        ];

        let split_requirements = formal_dataset_split_requirements("formal_label_v1_ext_stress");
        let (train_end, calibration_end) =
            scenario_aware_formal_split_bounds(&rows, &ranges, split_requirements).unwrap();

        assert!((42..=59).contains(&train_end));
        assert!((162..=179).contains(&calibration_end));

        let label_support = FormalSplitLabelSupport::from_rows(&rows);
        assert!(label_support.split_has_required_label_support(0, train_end, split_requirements));
        assert!(label_support.split_has_required_label_support(
            train_end,
            calibration_end,
            split_requirements
        ));
        assert!(label_support.split_has_required_label_support(
            calibration_end,
            rows.len(),
            split_requirements
        ));
    }
}

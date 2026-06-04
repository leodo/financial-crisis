use std::{
    collections::BTreeMap,
    env,
    fmt::Write,
    fs,
    path::{Path, PathBuf},
};

mod actionability;
mod commands;
mod formal;
mod model;
mod output_paths;
mod probability;
mod reporting;
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
use anyhow::{bail, Context};
use chrono::{Duration, NaiveDate, Utc};
pub(crate) use commands::{
    build_formal_dataset_summary, collect_formal_dataset_scenario_ranges, feature_quality_grade,
    formal_dataset_split_profile, has_extension_acute_core_features,
    has_main_dataset_core_features, load_formal_dataset_scenario_sets,
    load_label_set_crisis_scenarios, print_formal_dataset_summary,
    render_formal_dataset_summary_markdown, row_has_action_episode_label,
    scenario_count_for_split_range, FormalDatasetSplitProfile, FormalDatasetSummaryEnvelope,
    ScenarioRowRange,
};
#[cfg(test)]
pub(crate) use commands::{
    formal_dataset_min_date, formal_dataset_snapshot_is_usable, formal_dataset_split_requirements,
    observation_is_visible_for_date, scenario_aware_formal_split_bounds,
    scenario_count_for_index_range, FormalSplitLabelSupport,
};
#[cfg(test)]
use commands::{FeatureSnapshotBuildOptions, PointInTimeMode};
use commands::{PipelineDatasetSource, PipelineTrainOptions};
use fc_domain::{
    embedded_protected_stress_window_catalog, load_crisis_scenario_catalog,
    probability_feature_names_for_transform, resolve_probability_feature_value,
    ActionEpisodeTemplateId, ActionabilityBundle, ActionabilityLevel, AssessmentHistoryPoint,
    AssessmentMethodVersions, AssessmentSnapshot, BacktestScenarioSummary,
    CrisisScenarioActionEpisodeOverrides, Frequency, HorizonEvaluationSummary,
    LogisticProbabilityModel, ModelReleaseManifest, ModelReleaseRecord, PlattCalibrationArtifact,
    ProbabilityBundle, ProbabilityBundleEvaluation, ProbabilityCoefficient, ProbabilityFeatureStat,
    ProbabilityHorizonBundle, ProtectedStressWindowCatalog, FEATURE_BUCKET_MONTHS_OR_HIGHER,
    FEATURE_BUCKET_NOW, FEATURE_BUCKET_WEEKS_OR_HIGHER, FEATURE_COVERAGE_SCORE,
    FEATURE_EXTERNAL_SHOCK_SCORE, FEATURE_FRESHNESS_DELAYED_OR_WORSE,
    FEATURE_FRESHNESS_STALE_OR_MISSING, FEATURE_HEURISTIC_P_20D, FEATURE_HEURISTIC_P_5D,
    FEATURE_HEURISTIC_P_60D, FEATURE_OVERALL_SCORE, FORMAL_PROBABILITY_BUNDLE_FEATURES,
    PROBABILITY_FEATURE_TRANSFORM_FAMILY_CONDITIONAL_V1,
    PROBABILITY_FEATURE_TRANSFORM_FAMILY_HYBRID_V1, PROBABILITY_FEATURE_TRANSFORM_IDENTITY_V1,
    PROBABILITY_FEATURE_TRANSFORM_INTERACTION_TAIL_V1,
    PROBABILITY_MODEL_FAMILY_FAMILY_CONDITIONAL_V1, PROBABILITY_MODEL_FAMILY_FAMILY_HYBRID_V1,
    PROBABILITY_MODEL_FAMILY_INTERACTION_TAIL_V1, PROBABILITY_MODEL_FAMILY_LINEAR_V1,
    TRANSITIONAL_PROBABILITY_BUNDLE_FEATURES,
};
use fc_ingestion::{
    BojConnector, BojDataset, Connector, FetchPlan, FredConnector, FredGraphCsvConnector,
    GdeltConnector, MockConnector, RunMode, SecEdgarConnector, TreasuryYieldCurveConnector,
    WorldBankConnector,
};
use fc_storage::{
    ExternalIndicatorMapping, RawResponseRecord, SqliteStore, BOJ_FX_DATASET_ID,
    BOJ_MONEY_MARKET_DATASET_ID, FRED_DATASET_ID, GDELT_DOC_DATASET_ID, SEC_EVENTS_DATASET_ID,
    SEC_SUBMISSIONS_DATASET_ID, TREASURY_YIELD_DATASET_ID, WORLD_BANK_DATASET_ID,
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
use reporting::write_formal_dataset_summary_report;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
pub(crate) use training::{
    chronological_split, chronological_split_bounds, training_rows_support_label_mode,
    validate_split_bounds, ProbabilityTargetLabelMode, ProbabilityTrainingInput,
    ProbabilityTrainingRow,
};
use uuid::Uuid;

#[cfg(test)]
use commands::{
    FormalDatasetBuildOptions, FormalDatasetSummaryOptions, PredictionSnapshotQueryOptions,
    ProbabilityModelShape,
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

#[derive(Debug, Clone)]
struct AuditExportOptions {
    api_base_url: String,
    output_dir: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ApiReloadHistoryMode {
    Default,
    StrictRebuild,
}

impl ApiReloadHistoryMode {
    fn parse(value: &str) -> anyhow::Result<Self> {
        match value {
            "default" => Ok(Self::Default),
            "strict_rebuild" => Ok(Self::StrictRebuild),
            other => bail!("unsupported API reload history mode: {other}"),
        }
    }

    fn as_query_value(self) -> Option<&'static str> {
        match self {
            Self::Default => None,
            Self::StrictRebuild => Some("strict_rebuild"),
        }
    }

    fn as_label(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::StrictRebuild => "strict_rebuild",
        }
    }
}

#[derive(Debug, Clone)]
struct CrisisScenario {
    scenario_id: String,
    family: String,
    training_role: String,
    pre_warning_start: NaiveDate,
    crisis_start: NaiveDate,
    acute_start: Option<NaiveDate>,
    crisis_end: NaiveDate,
    default_horizon_roles: Vec<u32>,
    protected_window: bool,
    protected_action_levels: Vec<ActionabilityLevel>,
    episode_template_id: ActionEpisodeTemplateId,
    action_episode_overrides: Option<CrisisScenarioActionEpisodeOverrides>,
}

impl AuditExportOptions {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut api_base_url = env::var("FC_AUDIT_API_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_AUDIT_API_BASE_URL.to_string());
        let mut output_dir = PathBuf::from(DEFAULT_AUDIT_OUTPUT_DIR);
        let mut index = 0;

        while index < args.len() {
            match args[index].as_str() {
                "--api-base-url" => {
                    index += 1;
                    api_base_url = args
                        .get(index)
                        .with_context(|| "--api-base-url requires a URL")?
                        .clone();
                }
                "--output-dir" => {
                    index += 1;
                    output_dir = PathBuf::from(
                        args.get(index)
                            .with_context(|| "--output-dir requires a path")?,
                    );
                }
                other => bail!("unknown audit export option: {other}"),
            }
            index += 1;
        }

        Ok(Self {
            api_base_url: api_base_url.trim_end_matches('/').to_string(),
            output_dir,
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct RuntimeThresholdDiagnosticsWire {
    prepare_p60d: f64,
    hedge_p20d: f64,
    defend_p5d: f64,
    severe_now_p20d: f64,
    elevated_weeks_p60d: f64,
    external_prepare_p20d: f64,
    carry_prepare_p60d: f64,
    downgrade_prepare_p60d: f64,
    downgrade_hedge_p20d: f64,
    downgrade_defend_p5d: f64,
    history_runtime_policy_version: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AuditMethodResponse {
    method: AssessmentMethodVersions,
    note: String,
    protected_stress_window_catalog: ProtectedStressWindowCatalog,
    runtime_thresholds: Option<RuntimeThresholdDiagnosticsWire>,
}

#[derive(Debug, Clone, Deserialize)]
struct AuditMethodResponseWire {
    method: AssessmentMethodVersions,
    note: String,
    protected_stress_window_catalog: Option<ProtectedStressWindowCatalog>,
    runtime_thresholds: Option<RuntimeThresholdDiagnosticsWire>,
}

#[derive(Debug, Clone, Serialize)]
struct AuditExportEnvelope {
    exported_at: String,
    api_base_url: String,
    assessment: AssessmentSnapshot,
    backtests: Vec<BacktestScenarioSummary>,
    method: AuditMethodResponse,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseReviewScalarMetric {
    baseline: f64,
    candidate: f64,
    delta: f64,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseReviewCountMetric {
    baseline: u32,
    candidate: u32,
    delta: i64,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseReviewBacktestScenarioComparison {
    scenario_id: String,
    name: String,
    signal_source: String,
    crisis_start: NaiveDate,
    crisis_end: NaiveDate,
    baseline_first_l2_date: Option<NaiveDate>,
    candidate_first_l2_date: Option<NaiveDate>,
    baseline_first_l3_date: Option<NaiveDate>,
    candidate_first_l3_date: Option<NaiveDate>,
    baseline_lead_time_days: Option<i64>,
    candidate_lead_time_days: Option<i64>,
    baseline_actionable_lead_time_days: Option<i64>,
    candidate_actionable_lead_time_days: Option<i64>,
    baseline_false_positive_count: u32,
    candidate_false_positive_count: u32,
    actionable_delta_days: Option<i64>,
    outcome: String,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseReviewScenarioPointComparison {
    as_of_date: NaiveDate,
    baseline_p20d: Option<f64>,
    candidate_p20d: Option<f64>,
    baseline_p60d: Option<f64>,
    candidate_p60d: Option<f64>,
    baseline_posture: Option<String>,
    candidate_posture: Option<String>,
    baseline_time_bucket: Option<String>,
    candidate_time_bucket: Option<String>,
    baseline_strict_review_actionable: bool,
    candidate_strict_review_actionable: bool,
    baseline_runtime_floor_hit: bool,
    candidate_runtime_floor_hit: bool,
    baseline_actionable: bool,
    candidate_actionable: bool,
    baseline_actionable_forward_5d_hits: Option<u32>,
    candidate_actionable_forward_5d_hits: Option<u32>,
    baseline_actionable_sustained: Option<bool>,
    candidate_actionable_sustained: Option<bool>,
    baseline_trigger_codes: Vec<String>,
    candidate_trigger_codes: Vec<String>,
    baseline_runtime_actionable_block_category: Option<String>,
    candidate_runtime_actionable_block_category: Option<String>,
    baseline_runtime_actionable_block_reason: Option<String>,
    candidate_runtime_actionable_block_reason: Option<String>,
    baseline_actionable_diagnostic: Option<String>,
    candidate_actionable_diagnostic: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseReviewRuntimeBlockCount {
    category: String,
    baseline_count: u32,
    candidate_count: u32,
    delta: i64,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseReviewScenarioFocusDiagnostic {
    scenario_id: String,
    name: String,
    outcome: String,
    window_start: NaiveDate,
    window_end: NaiveDate,
    crisis_start: NaiveDate,
    crisis_end: NaiveDate,
    baseline_first_l2_date: Option<NaiveDate>,
    candidate_first_l2_date: Option<NaiveDate>,
    baseline_first_l3_date: Option<NaiveDate>,
    candidate_first_l3_date: Option<NaiveDate>,
    baseline_first_non_normal_date: Option<NaiveDate>,
    candidate_first_non_normal_date: Option<NaiveDate>,
    baseline_actionable_point_count: u32,
    candidate_actionable_point_count: u32,
    baseline_runtime_floor_hit_point_count: u32,
    candidate_runtime_floor_hit_point_count: u32,
    baseline_max_p20d: Option<f64>,
    candidate_max_p20d: Option<f64>,
    baseline_max_p60d: Option<f64>,
    candidate_max_p60d: Option<f64>,
    baseline_first_runtime_floor_hit_without_l3_date: Option<NaiveDate>,
    candidate_first_runtime_floor_hit_without_l3_date: Option<NaiveDate>,
    baseline_first_runtime_floor_hit_without_l3_reason: Option<String>,
    candidate_first_runtime_floor_hit_without_l3_reason: Option<String>,
    runtime_block_counts: Vec<ReleaseReviewRuntimeBlockCount>,
    interesting_points: Vec<ReleaseReviewScenarioPointComparison>,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseReviewComparisonSummary {
    timely_warning_rate: ReleaseReviewScalarMetric,
    strict_actionable_point_count: ReleaseReviewCountMetric,
    runtime_floor_hit_count: ReleaseReviewCountMetric,
    actionable_precision: ReleaseReviewScalarMetric,
    longest_false_positive_episode_days: ReleaseReviewCountMetric,
    current_p_5d: ReleaseReviewScalarMetric,
    current_p_20d: ReleaseReviewScalarMetric,
    current_p_60d: ReleaseReviewScalarMetric,
    backtest_scenarios: Vec<ReleaseReviewBacktestScenarioComparison>,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseActionabilityLevelReview {
    level: ActionabilityLevel,
    proxy_horizon_days: u32,
    sample_count: u32,
    positive_rate: f64,
    threshold: f64,
    predicted_positive_count: u32,
    primary_positive_count: u32,
    late_validation_row_count: u32,
    protected_row_count: u32,
    primary_hit_count: u32,
    late_validation_hit_count: u32,
    protected_hit_count: u32,
    false_positive_count: u32,
    scenario_count: u32,
    on_time_scenario_count: u32,
    late_only_scenario_count: u32,
    missed_scenario_count: u32,
    precision_at_threshold: Option<f64>,
    primary_recall_at_threshold: Option<f64>,
    late_validation_capture_rate: Option<f64>,
    on_time_rate: Option<f64>,
    late_only_rate: Option<f64>,
    missed_rate: Option<f64>,
    note: String,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseActionabilityReview {
    release_id: String,
    enabled: bool,
    model_version: Option<String>,
    calibration_version: Option<String>,
    fusion_policy_version: Option<String>,
    levels: Vec<ReleaseActionabilityLevelReview>,
    guard_regressions: Vec<String>,
    guard_passed: bool,
    note: String,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseRuntimeCount {
    name: String,
    count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseRuntimeReviewDiagnostics {
    release_id: String,
    history_point_count: usize,
    posture_distribution: Vec<ReleaseRuntimeCount>,
    time_bucket_distribution: Vec<ReleaseRuntimeCount>,
    posture_trigger_distribution: Vec<ReleaseRuntimeClauseCount>,
    posture_blocker_distribution: Vec<ReleaseRuntimeClauseCount>,
    regime_probability_summaries: Vec<ReleaseRuntimeRegimeProbabilitySummary>,
    regime_separation_summaries: Vec<ReleaseRuntimeSeparationSummary>,
    runtime_thresholds: Option<RuntimeThresholdDiagnosticsWire>,
    points_at_or_above_prepare_p60d: Option<usize>,
    points_at_or_above_hedge_p20d: Option<usize>,
    points_at_or_above_defend_p5d: Option<usize>,
    note: String,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseRuntimeClauseCount {
    posture: String,
    clause: String,
    count: usize,
    share_of_posture: f64,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseRuntimeRegimeProbabilitySummary {
    horizon_days: u32,
    regime: String,
    row_count: usize,
    row_rate: f64,
    avg_raw_probability: f64,
    max_raw_probability: f64,
    avg_probability: f64,
    max_probability: f64,
    raw_lift_vs_normal: Option<f64>,
    calibrated_lift_vs_normal: Option<f64>,
    raw_gap_vs_normal: Option<f64>,
    calibrated_gap_vs_normal: Option<f64>,
    calibration_gap_retention: Option<f64>,
    threshold_hit_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseRuntimeSeparationSummary {
    horizon_days: u32,
    early_warning_regime: String,
    normal_avg_probability: f64,
    pre_warning_buffer_avg_probability: f64,
    positive_window_avg_probability: f64,
    in_crisis_avg_probability: f64,
    post_crisis_cooldown_avg_probability: f64,
    early_warning_raw_lift_vs_normal: Option<f64>,
    early_warning_calibrated_lift_vs_normal: Option<f64>,
    early_warning_gap_retention: Option<f64>,
    positive_window_calibrated_lift_vs_normal: Option<f64>,
    positive_window_gap_vs_normal: Option<f64>,
    in_crisis_raw_lift_vs_normal: Option<f64>,
    in_crisis_calibrated_lift_vs_normal: Option<f64>,
    post_crisis_cooldown_calibrated_lift_vs_normal: Option<f64>,
    post_crisis_cooldown_gap_vs_normal: Option<f64>,
    max_non_normal_calibrated_lift_vs_normal: Option<f64>,
    max_non_normal_threshold_hit_rate: Option<f64>,
    diagnosis: String,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseReviewEnvelope {
    reviewed_at: String,
    market_scope: String,
    api_reload_url: String,
    history_mode: String,
    history_limit: usize,
    original_active_release_id: String,
    restored_release_id: String,
    baseline_release: ModelReleaseRecord,
    candidate_release: ModelReleaseRecord,
    baseline_assessment: AssessmentSnapshot,
    candidate_assessment: AssessmentSnapshot,
    baseline_runtime_review: ReleaseRuntimeReviewDiagnostics,
    candidate_runtime_review: ReleaseRuntimeReviewDiagnostics,
    baseline_actionability_review: ReleaseActionabilityReview,
    candidate_actionability_review: ReleaseActionabilityReview,
    scenario_focus: Vec<ReleaseReviewScenarioFocusDiagnostic>,
    comparison: ReleaseReviewComparisonSummary,
    probability_guard_regressions: Vec<String>,
    probability_guard_passed: bool,
    operational_guard_regressions: Vec<String>,
    operational_guard_passed: bool,
    actionability_guard_regressions: Vec<String>,
    actionability_guard_passed: bool,
    runtime_sanity_regressions: Vec<String>,
    runtime_sanity_passed: bool,
    overall_guard_regressions: Vec<String>,
    overall_guard_passed: bool,
    recommendation: String,
}

async fn export_current_audit(args: &[String]) -> anyhow::Result<()> {
    let options = AuditExportOptions::parse(args)?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    let assessment: AssessmentSnapshot =
        fetch_api_json(&client, &options.api_base_url, "/api/assessment/current").await?;
    let backtests: Vec<BacktestScenarioSummary> =
        fetch_api_json(&client, &options.api_base_url, "/api/backtests").await?;
    let method_wire: AuditMethodResponseWire =
        fetch_api_json(&client, &options.api_base_url, "/api/assessment/method").await?;
    let method = AuditMethodResponse {
        method: method_wire.method,
        note: method_wire.note,
        protected_stress_window_catalog: method_wire.protected_stress_window_catalog.unwrap_or_else(
            || {
                let mut catalog = embedded_protected_stress_window_catalog();
                catalog.warning = Some(
                    "运行中的 API 仍返回旧版 method 响应，导出命令已退回本地内置压力窗口目录；重启 API 后可获得完全一致的导出结果。"
                        .to_string(),
                );
                catalog
            },
        ),
        runtime_thresholds: method_wire.runtime_thresholds,
    };

    let report = AuditExportEnvelope {
        exported_at: Utc::now().to_rfc3339(),
        api_base_url: options.api_base_url.clone(),
        assessment,
        backtests,
        method,
    };

    fs::create_dir_all(&options.output_dir)?;
    let stem = format!("{}-rolling-audit", report.assessment.as_of_date);
    let json_path = options.output_dir.join(format!("{stem}.json"));
    let markdown_path = options.output_dir.join(format!("{stem}.md"));
    fs::write(&json_path, serde_json::to_string_pretty(&report)?)?;
    fs::write(&markdown_path, render_audit_report_markdown(&report))?;

    println!("Rolling audit report exported.");
    println!("  JSON     {}", json_path.display());
    println!("  Markdown {}", markdown_path.display());
    println!(
        "  Summary  {}",
        report.assessment.backtest_summary.rolling_audit.summary
    );
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionEpisodePhase {
    Outside,
    Cooldown,
    LateValidation,
    Primary,
}

impl ActionEpisodePhase {
    fn as_str(self) -> &'static str {
        match self {
            Self::Outside => "outside",
            Self::Cooldown => "cooldown",
            Self::LateValidation => "late_validation",
            Self::Primary => "primary",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct DateRange {
    start: NaiveDate,
    end: NaiveDate,
}

impl DateRange {
    fn new(start: NaiveDate, end: NaiveDate) -> Option<Self> {
        (start <= end).then_some(Self { start, end })
    }

    fn contains(self, as_of_date: NaiveDate) -> bool {
        as_of_date >= self.start && as_of_date <= self.end
    }
}

#[derive(Debug, Clone, Copy)]
struct ActionEpisodeWindow {
    primary: Option<DateRange>,
    late_validation: Option<DateRange>,
    cooldown: Option<DateRange>,
    protected_action_window: bool,
}

#[derive(Debug, Clone)]
struct ActionEpisodeSelection {
    scenario_id: String,
    level: ActionabilityLevel,
    phase: ActionEpisodePhase,
    protected_action_window: bool,
    crisis_start: NaiveDate,
}

fn shift_date(date: NaiveDate, days: i64) -> NaiveDate {
    date.checked_add_signed(Duration::days(days))
        .unwrap_or(date)
}

fn next_day(date: NaiveDate) -> NaiveDate {
    shift_date(date, 1)
}

fn action_level_rank(level: ActionabilityLevel) -> i32 {
    match level {
        ActionabilityLevel::Defend => 0,
        ActionabilityLevel::Hedge => 1,
        ActionabilityLevel::Prepare => 2,
    }
}

fn action_episode_phase_rank(phase: ActionEpisodePhase) -> i32 {
    match phase {
        ActionEpisodePhase::Primary => 0,
        ActionEpisodePhase::LateValidation => 1,
        ActionEpisodePhase::Cooldown => 2,
        ActionEpisodePhase::Outside => 3,
    }
}

fn action_level_proxy_horizon_days(level: ActionabilityLevel) -> u32 {
    match level {
        ActionabilityLevel::Prepare => 60,
        ActionabilityLevel::Hedge => 20,
        ActionabilityLevel::Defend => 5,
    }
}

fn actionability_level_for_proxy_horizon(horizon_days: u32) -> Option<ActionabilityLevel> {
    match horizon_days {
        60 => Some(ActionabilityLevel::Prepare),
        20 => Some(ActionabilityLevel::Hedge),
        5 => Some(ActionabilityLevel::Defend),
        _ => None,
    }
}

fn action_episode_override_for_level(
    overrides: Option<&CrisisScenarioActionEpisodeOverrides>,
    level: ActionabilityLevel,
) -> Option<&fc_domain::ActionEpisodeWindowOverride> {
    let overrides = overrides?;
    match level {
        ActionabilityLevel::Prepare => overrides.prepare.as_ref(),
        ActionabilityLevel::Hedge => overrides.hedge.as_ref(),
        ActionabilityLevel::Defend => overrides.defend.as_ref(),
    }
}

fn action_episode_default_window(
    scenario: &CrisisScenario,
    level: ActionabilityLevel,
) -> ActionEpisodeWindow {
    let acute_start = scenario.acute_start.unwrap_or(scenario.crisis_start);
    let (primary, late_validation) = match (scenario.episode_template_id, level) {
        (ActionEpisodeTemplateId::SystemicCreditBankingCrisis, ActionabilityLevel::Prepare) => (
            DateRange::new(
                scenario.pre_warning_start,
                shift_date(scenario.crisis_start, -21),
            ),
            DateRange::new(
                shift_date(scenario.crisis_start, -20),
                shift_date(scenario.crisis_start, -11),
            ),
        ),
        (ActionEpisodeTemplateId::SystemicCreditBankingCrisis, ActionabilityLevel::Hedge) => (
            DateRange::new(
                shift_date(scenario.crisis_start, -20),
                shift_date(scenario.crisis_start, -6),
            ),
            DateRange::new(
                shift_date(scenario.crisis_start, -5),
                shift_date(scenario.crisis_start, 3),
            ),
        ),
        (ActionEpisodeTemplateId::SystemicCreditBankingCrisis, ActionabilityLevel::Defend) => (
            DateRange::new(
                shift_date(scenario.crisis_start, -5),
                shift_date(acute_start, 3),
            ),
            DateRange::new(shift_date(acute_start, 4), shift_date(acute_start, 10)),
        ),
        (ActionEpisodeTemplateId::AcuteMarketLiquidityCrash, ActionabilityLevel::Prepare) => (
            DateRange::new(
                scenario.pre_warning_start.max(shift_date(acute_start, -20)),
                shift_date(acute_start, -11),
            ),
            DateRange::new(shift_date(acute_start, -10), shift_date(acute_start, -7)),
        ),
        (ActionEpisodeTemplateId::AcuteMarketLiquidityCrash, ActionabilityLevel::Hedge) => (
            DateRange::new(shift_date(acute_start, -10), shift_date(acute_start, -4)),
            DateRange::new(shift_date(acute_start, -3), shift_date(acute_start, 1)),
        ),
        (ActionEpisodeTemplateId::AcuteMarketLiquidityCrash, ActionabilityLevel::Defend) => (
            DateRange::new(shift_date(acute_start, -3), shift_date(acute_start, 2)),
            DateRange::new(shift_date(acute_start, 3), shift_date(acute_start, 7)),
        ),
        (ActionEpisodeTemplateId::MixedSystemicStress, ActionabilityLevel::Prepare)
        | (ActionEpisodeTemplateId::RateShockOrPolicyDislocation, ActionabilityLevel::Prepare) => (
            DateRange::new(
                scenario.pre_warning_start,
                shift_date(scenario.crisis_start, -16),
            ),
            DateRange::new(
                shift_date(scenario.crisis_start, -15),
                shift_date(scenario.crisis_start, -8),
            ),
        ),
        (ActionEpisodeTemplateId::MixedSystemicStress, ActionabilityLevel::Hedge) => (
            DateRange::new(
                shift_date(scenario.crisis_start, -15),
                shift_date(scenario.crisis_start, -5),
            ),
            DateRange::new(
                shift_date(scenario.crisis_start, -4),
                shift_date(scenario.crisis_start, 3),
            ),
        ),
        (ActionEpisodeTemplateId::RateShockOrPolicyDislocation, ActionabilityLevel::Hedge) => (
            DateRange::new(
                shift_date(scenario.crisis_start, -15),
                shift_date(scenario.crisis_start, -5),
            ),
            DateRange::new(
                shift_date(scenario.crisis_start, -4),
                shift_date(scenario.crisis_start, 2),
            ),
        ),
        (
            ActionEpisodeTemplateId::MixedSystemicStress
            | ActionEpisodeTemplateId::RateShockOrPolicyDislocation,
            ActionabilityLevel::Defend,
        ) => (None, None),
    };
    let cooldown = DateRange::new(
        next_day(scenario.crisis_end),
        shift_date(
            scenario.crisis_end,
            post_crisis_cooldown_days(action_level_proxy_horizon_days(level)),
        ),
    );

    ActionEpisodeWindow {
        primary,
        late_validation,
        cooldown,
        protected_action_window: scenario.protected_window
            && scenario.protected_action_levels.contains(&level),
    }
}

fn action_episode_window(
    scenario: &CrisisScenario,
    level: ActionabilityLevel,
) -> ActionEpisodeWindow {
    let mut window = action_episode_default_window(scenario, level);
    let Some(override_window) =
        action_episode_override_for_level(scenario.action_episode_overrides.as_ref(), level)
    else {
        return window;
    };

    if override_window.enabled == Some(false) {
        return ActionEpisodeWindow {
            primary: None,
            late_validation: None,
            cooldown: None,
            protected_action_window: window.protected_action_window,
        };
    }

    if let (Some(primary_start), Some(primary_end)) =
        (override_window.primary_start, override_window.primary_end)
    {
        window.primary = DateRange::new(primary_start, primary_end);
    }
    if let Some(late_validation_end) = override_window.late_validation_end {
        window.late_validation = window
            .primary
            .and_then(|primary| DateRange::new(next_day(primary.end), late_validation_end));
    }
    if let Some(cooldown_end) = override_window.cooldown_end {
        window.cooldown = DateRange::new(next_day(scenario.crisis_end), cooldown_end);
    }

    window
}

fn action_episode_phase_for_date(
    as_of_date: NaiveDate,
    scenario: &CrisisScenario,
    level: ActionabilityLevel,
) -> ActionEpisodePhase {
    let window = action_episode_window(scenario, level);
    if window
        .primary
        .is_some_and(|range| range.contains(as_of_date))
    {
        return ActionEpisodePhase::Primary;
    }
    if window
        .late_validation
        .is_some_and(|range| range.contains(as_of_date))
    {
        return ActionEpisodePhase::LateValidation;
    }
    if window
        .cooldown
        .is_some_and(|range| range.contains(as_of_date))
    {
        return ActionEpisodePhase::Cooldown;
    }
    ActionEpisodePhase::Outside
}

fn action_episode_label_for_level(
    as_of_date: NaiveDate,
    scenarios: &[CrisisScenario],
    level: ActionabilityLevel,
) -> u8 {
    scenarios.iter().any(|scenario| {
        matches!(
            action_episode_phase_for_date(as_of_date, scenario, level),
            ActionEpisodePhase::Primary
        )
    }) as u8
}

fn dominant_action_episode_for_date(
    as_of_date: NaiveDate,
    scenarios: &[CrisisScenario],
) -> Option<ActionEpisodeSelection> {
    [
        ActionabilityLevel::Prepare,
        ActionabilityLevel::Hedge,
        ActionabilityLevel::Defend,
    ]
    .into_iter()
    .flat_map(|level| {
        scenarios.iter().filter_map(move |scenario| {
            let phase = action_episode_phase_for_date(as_of_date, scenario, level);
            (!matches!(phase, ActionEpisodePhase::Outside)).then_some(ActionEpisodeSelection {
                scenario_id: scenario.scenario_id.clone(),
                level,
                phase,
                protected_action_window: action_episode_window(scenario, level)
                    .protected_action_window,
                crisis_start: scenario.crisis_start,
            })
        })
    })
    .min_by_key(|selection| {
        (
            action_episode_phase_rank(selection.phase),
            action_level_rank(selection.level),
            (selection.crisis_start - as_of_date).num_days().abs(),
        )
    })
}

fn protected_context_phase_for_date(
    as_of_date: NaiveDate,
    positive_scenarios: &[CrisisScenario],
    context_scenarios: &[CrisisScenario],
) -> Option<ActionEpisodePhase> {
    context_scenarios
        .iter()
        .filter(|scenario| {
            scenario.protected_window
                && !positive_scenarios
                    .iter()
                    .any(|positive| positive.scenario_id == scenario.scenario_id)
        })
        .flat_map(|scenario| {
            scenario
                .protected_action_levels
                .iter()
                .copied()
                .filter_map(move |level| {
                    let phase = action_episode_phase_for_date(as_of_date, scenario, level);
                    (!matches!(phase, ActionEpisodePhase::Outside)).then_some((phase, scenario))
                })
        })
        .min_by_key(|(phase, scenario)| {
            (
                action_episode_phase_rank(*phase),
                (scenario.crisis_start - as_of_date).num_days().abs(),
            )
        })
        .map(|(phase, _)| phase)
}

fn scenario_supports_horizon(scenario: &CrisisScenario, horizon_days: u32) -> bool {
    scenario.default_horizon_roles.contains(&horizon_days)
}

fn label_anchor_date(scenario: &CrisisScenario, horizon_days: u32) -> NaiveDate {
    if horizon_days == 5 {
        scenario.acute_start.unwrap_or(scenario.crisis_start)
    } else {
        scenario.crisis_start
    }
}

fn action_window_lead_days(horizon_days: u32) -> i64 {
    match horizon_days {
        5 => 10,
        20 => 35,
        60 => 90,
        _ => horizon_days as i64,
    }
}

fn action_window_start_date(scenario: &CrisisScenario, horizon_days: u32) -> NaiveDate {
    let anchor_date = label_anchor_date(scenario, horizon_days);
    let buffered_start = anchor_date
        .checked_sub_signed(Duration::days(action_window_lead_days(horizon_days)))
        .unwrap_or(anchor_date);
    scenario.pre_warning_start.max(buffered_start)
}

fn action_window_end_days(horizon_days: u32) -> i64 {
    match horizon_days {
        5 => 7,
        20 => 20,
        60 => 30,
        _ => horizon_days as i64,
    }
}

fn action_window_end_date(scenario: &CrisisScenario, horizon_days: u32) -> NaiveDate {
    let anchor_date = label_anchor_date(scenario, horizon_days);
    let buffered_end = anchor_date
        .checked_add_signed(Duration::days(action_window_end_days(horizon_days)))
        .unwrap_or(scenario.crisis_end);
    scenario.crisis_end.min(buffered_end)
}

fn action_window_label(
    as_of_date: NaiveDate,
    scenarios: &[CrisisScenario],
    horizon_days: i64,
) -> u8 {
    let horizon_days_u32 = horizon_days as u32;
    scenarios.iter().any(|scenario| {
        as_of_date >= action_window_start_date(scenario, horizon_days_u32)
            && as_of_date <= action_window_end_date(scenario, horizon_days_u32)
    }) as u8
}

fn scenario_has_action_window(scenario: &CrisisScenario, as_of_date: NaiveDate) -> bool {
    [
        ActionabilityLevel::Prepare,
        ActionabilityLevel::Hedge,
        ActionabilityLevel::Defend,
    ]
    .into_iter()
    .any(|level| {
        !matches!(
            action_episode_phase_for_date(as_of_date, scenario, level),
            ActionEpisodePhase::Outside
        )
    })
}

fn primary_scenario_for_date(
    as_of_date: NaiveDate,
    scenarios: &[CrisisScenario],
) -> Option<CrisisScenario> {
    if let Some(selection) = dominant_action_episode_for_date(as_of_date, scenarios) {
        return scenarios
            .iter()
            .find(|scenario| scenario.scenario_id == selection.scenario_id)
            .cloned();
    }

    scenarios
        .iter()
        .filter(|scenario| scenario_has_action_window(scenario, as_of_date))
        .min_by_key(|scenario| {
            let distance = (scenario.crisis_start - as_of_date).num_days().abs();
            let in_crisis_penalty = if as_of_date > scenario.crisis_start {
                10_000
            } else {
                0
            };
            in_crisis_penalty + distance
        })
        .cloned()
        .or_else(|| forward_scenario(as_of_date, scenarios, 60))
}

fn forward_scenario(
    as_of_date: NaiveDate,
    scenarios: &[CrisisScenario],
    horizon_days: i64,
) -> Option<CrisisScenario> {
    scenarios
        .iter()
        .filter_map(|scenario| {
            let lead_days = (scenario.crisis_start - as_of_date).num_days();
            (1..=horizon_days)
                .contains(&lead_days)
                .then_some((scenario.clone(), lead_days))
        })
        .min_by_key(|(_, lead_days)| *lead_days)
        .map(|(scenario, _)| scenario)
}

fn formal_dataset_key(dataset_id: &str, dataset_version: &str) -> String {
    format!("{dataset_id}:{dataset_version}")
}

pub(crate) fn round3(value: f64) -> f64 {
    (value * 1_000.0).round() / 1_000.0
}

pub(crate) fn round6(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

fn safe_divide(numerator: f64, denominator: f64) -> f64 {
    if denominator.abs() <= f64::EPSILON {
        0.0
    } else {
        numerator / denominator
    }
}

pub(crate) fn safe_ratio(numerator: usize, denominator: usize) -> f64 {
    safe_divide(numerator as f64, denominator as f64)
}

fn actionability_level_text(level: ActionabilityLevel) -> &'static str {
    match level {
        ActionabilityLevel::Prepare => "prepare",
        ActionabilityLevel::Hedge => "hedge",
        ActionabilityLevel::Defend => "defend",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
enum ProbabilityTrainingRegime {
    Normal,
    PositiveWindow,
    PreWarningBuffer,
    InCrisis,
    PostCrisisCooldown,
}

pub(crate) fn probability_training_regime_name(regime: ProbabilityTrainingRegime) -> &'static str {
    match regime {
        ProbabilityTrainingRegime::Normal => "normal",
        ProbabilityTrainingRegime::PositiveWindow => "positive_window",
        ProbabilityTrainingRegime::PreWarningBuffer => "pre_warning_buffer",
        ProbabilityTrainingRegime::InCrisis => "in_crisis",
        ProbabilityTrainingRegime::PostCrisisCooldown => "post_crisis_cooldown",
    }
}

#[derive(Debug, Clone)]
struct PipelineArtifacts {
    release: ModelReleaseRecord,
    bundle: ProbabilityBundle,
    bundle_path: PathBuf,
    manifest_path: PathBuf,
    evaluation_path: PathBuf,
    dataset_source: String,
    dataset_label: String,
}

#[derive(Debug, Clone, Serialize)]
struct PipelineEvaluationReport {
    release_id: String,
    dataset_source: String,
    dataset_label: String,
    model_family: String,
    feature_transform: String,
    target_label_mode: ProbabilityTargetLabelMode,
    market_scope: String,
    feature_names: Vec<String>,
    training_samples: usize,
    calibration_samples: usize,
    evaluation_samples: usize,
    horizons: Vec<ProbabilityHorizonBundle>,
    actionability: Option<ActionabilityBundle>,
    summary: Option<ProbabilityBundleEvaluation>,
}

async fn train_probability_pipeline(
    store: &SqliteStore,
    options: &PipelineTrainOptions,
) -> anyhow::Result<PipelineArtifacts> {
    let generated_at = Utc::now();
    let training = commands::pipeline::load_probability_training_input(store, options).await?;
    let bundle_feature_names = probability_feature_names_for_transform(
        &training.feature_names,
        options.model_shape.feature_transform(),
    );
    let crisis_prior_label_mode = ProbabilityTargetLabelMode::ForwardCrisis;
    let horizons = [5_u32, 20_u32, 60_u32]
        .into_iter()
        .map(|horizon| {
            let base_feature_names = probability_feature_names_for_transform(
                &training.feature_names,
                options
                    .model_shape
                    .base_feature_transform_for_horizon(horizon),
            );
            let overlay_feature_names = probability_feature_names_for_transform(
                &training.feature_names,
                options
                    .model_shape
                    .overlay_feature_transform_for_horizon(horizon),
            );
            train_horizon_bundle(
                &training.train_rows,
                &training.calibration_rows,
                &training.evaluation_rows,
                &base_feature_names,
                &overlay_feature_names,
                horizon,
                crisis_prior_label_mode,
            )
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    let actionability = if matches!(training.dataset_source, PipelineDatasetSource::Formal)
        && training_rows_support_label_mode(
            &training.train_rows,
            &training.calibration_rows,
            &training.evaluation_rows,
            ProbabilityTargetLabelMode::ActionEpisode,
        ) {
        let candidate = train_actionability_bundle(
            &training.train_rows,
            &training.calibration_rows,
            &training.evaluation_rows,
            &training.feature_names,
            &generated_at.format("%Y%m%dT%H%M%S").to_string(),
        )?;
        let guard_regressions = actionability_bundle_quality_regressions(&candidate);
        if guard_regressions.is_empty() {
            Some(candidate)
        } else {
            println!("Actionability head disabled for this release:");
            for regression in &guard_regressions {
                println!("  - {regression}");
            }
            None
        }
    } else {
        None
    };

    let aggregate_evaluation = summarize_bundle_evaluation(&horizons);
    let release_suffix = generated_at.format("%Y%m%dT%H%M%S").to_string();
    let release_id = format!("{}_{}", options.release_prefix, release_suffix);
    let bundle_note = match training.dataset_source {
        PipelineDatasetSource::Formal => format!(
            "Formal bundle trained from persisted formal dataset {} built from raw observations -> feature snapshots -> scenario labels; model_shape={} feature_transform={}; crisis-prior head uses forward-crisis labels, and {}.",
            training.dataset_label,
            options.model_shape.as_str(),
            options.model_shape.feature_transform(),
            if actionability.is_some() {
                "actionability head uses episode-native prepare/hedge/defend labels when quality gates pass"
            } else {
                "independent actionability head was omitted because evaluation quality gates did not pass, so runtime falls back to probability-context fusion"
            }
        ),
        PipelineDatasetSource::Snapshot => {
            "Transitional formal bundle trained from persisted heuristic prediction snapshots, calibrated with chronological holdout slices, and reweighted toward positive warning windows under severe class imbalance.".to_string()
        }
    };
    let bundle = ProbabilityBundle {
        bundle_id: release_id.clone(),
        market_scope: training.market_scope.clone(),
        probability_mode: "formal_bundle_v1".to_string(),
        model_family: options.model_shape.as_str().to_string(),
        feature_transform: options.model_shape.feature_transform().to_string(),
        created_at: generated_at,
        feature_names: bundle_feature_names.clone(),
        monotonic_min_gap_5d_to_20d: 0.02,
        monotonic_min_gap_20d_to_60d: 0.03,
        note: bundle_note.clone(),
        horizons: horizons.clone(),
        evaluation: Some(aggregate_evaluation.clone()),
        actionability: actionability.clone(),
    };

    let bundle_path = options.output_dir.join(format!("{release_id}.json"));
    let manifest_dir = options.manifest_dir.clone();
    let manifest_path = manifest_dir.join(format!("{release_id}.json"));
    let evaluation_path = options
        .output_dir
        .join(format!("{release_id}-evaluation.json"));
    fs::create_dir_all(&options.output_dir)?;
    fs::create_dir_all(&manifest_dir)?;

    let release = ModelReleaseRecord {
        manifest: ModelReleaseManifest {
            release_id: release_id.clone(),
            market_scope: bundle.market_scope.clone(),
            status: "approved".to_string(),
            probability_mode: bundle.probability_mode.clone(),
            serving_status: "healthy".to_string(),
            bundle_uri: bundle_path.to_string_lossy().replace('\\', "/"),
            feature_set_version: training.feature_set_version.clone(),
            label_version: training.label_version.clone(),
            prob_model_version: format!("prob_{}_{}", options.model_shape.as_str(), release_suffix),
            calibration_version: format!("platt_{release_suffix}"),
            posture_policy_version: "posture_v1_20260530".to_string(),
            action_playbook_version: "action_playbook_v1_20260531".to_string(),
            point_in_time_mode: training.point_in_time_mode.clone(),
            training_range_start: training.train_rows.first().map(|row| row.as_of_date),
            training_range_end: training.train_rows.last().map(|row| row.as_of_date),
            calibration_range_start: training.calibration_rows.first().map(|row| row.as_of_date),
            calibration_range_end: training.calibration_rows.last().map(|row| row.as_of_date),
            evaluation_range_start: training.evaluation_rows.first().map(|row| row.as_of_date),
            evaluation_range_end: training.evaluation_rows.last().map(|row| row.as_of_date),
            brier_score: bundle
                .evaluation
                .as_ref()
                .map(|summary| summary.brier_score),
            log_loss: bundle.evaluation.as_ref().map(|summary| summary.log_loss),
            ece: bundle.evaluation.as_ref().map(|summary| summary.ece),
            note: format!(
                "Generated by `research pipeline train-probability` from {} dataset {} with model_shape={}.",
                training.dataset_source.as_str(),
                training.dataset_label,
                options.model_shape.as_str()
            ),
        },
        created_at: generated_at,
        activated_at: None,
        retired_at: None,
    };

    let evaluation_report = PipelineEvaluationReport {
        release_id: release_id.clone(),
        dataset_source: training.dataset_source.as_str().to_string(),
        dataset_label: training.dataset_label.clone(),
        model_family: options.model_shape.as_str().to_string(),
        feature_transform: options.model_shape.feature_transform().to_string(),
        target_label_mode: crisis_prior_label_mode,
        market_scope: release.manifest.market_scope.clone(),
        feature_names: bundle_feature_names.clone(),
        training_samples: training.train_rows.len(),
        calibration_samples: training.calibration_rows.len(),
        evaluation_samples: training.evaluation_rows.len(),
        horizons,
        actionability,
        summary: bundle.evaluation.clone(),
    };

    fs::write(&bundle_path, serde_json::to_string_pretty(&bundle)?)?;
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&release.manifest)?,
    )?;
    fs::write(
        &evaluation_path,
        serde_json::to_string_pretty(&evaluation_report)?,
    )?;

    Ok(PipelineArtifacts {
        release,
        bundle,
        bundle_path,
        manifest_path,
        evaluation_path,
        dataset_source: training.dataset_source.as_str().to_string(),
        dataset_label: training.dataset_label,
    })
}

fn forward_crisis_label(
    as_of_date: NaiveDate,
    scenarios: &[CrisisScenario],
    horizon_days: i64,
) -> u8 {
    let horizon_days_u32 = horizon_days as u32;
    scenarios.iter().any(|scenario| {
        let anchor_date = if scenario_supports_horizon(scenario, horizon_days_u32) {
            label_anchor_date(scenario, horizon_days_u32)
        } else {
            scenario.crisis_start
        };
        let lead_days = (anchor_date - as_of_date).num_days();
        (1..=horizon_days).contains(&lead_days)
    }) as u8
}

fn post_crisis_cooldown_days(horizon_days: u32) -> i64 {
    match horizon_days {
        5 => 14,
        20 => 30,
        60 => 45,
        _ => horizon_days as i64,
    }
}

pub(crate) fn forward_crisis_training_regime(
    as_of_date: NaiveDate,
    scenarios: &[CrisisScenario],
    horizon_days: u32,
) -> ProbabilityTrainingRegime {
    if forward_crisis_label(as_of_date, scenarios, horizon_days as i64) > 0 {
        return ProbabilityTrainingRegime::PositiveWindow;
    }

    let positive_buffer = scenarios.iter().any(|scenario| {
        let anchor_date = if scenario_supports_horizon(scenario, horizon_days) {
            label_anchor_date(scenario, horizon_days)
        } else {
            scenario.crisis_start
        };
        let positive_start = anchor_date
            .checked_sub_signed(Duration::days(horizon_days as i64))
            .unwrap_or(anchor_date);
        as_of_date >= action_window_start_date(scenario, horizon_days)
            && as_of_date < positive_start
    });
    if positive_buffer {
        return ProbabilityTrainingRegime::PreWarningBuffer;
    }

    if scenarios
        .iter()
        .any(|scenario| as_of_date >= scenario.crisis_start && as_of_date <= scenario.crisis_end)
    {
        return ProbabilityTrainingRegime::InCrisis;
    }

    let cooldown = scenarios.iter().any(|scenario| {
        let cooldown_end = scenario
            .crisis_end
            .checked_add_signed(Duration::days(post_crisis_cooldown_days(horizon_days)))
            .unwrap_or(scenario.crisis_end);
        as_of_date > scenario.crisis_end && as_of_date <= cooldown_end
    });
    if cooldown {
        return ProbabilityTrainingRegime::PostCrisisCooldown;
    }

    ProbabilityTrainingRegime::Normal
}

fn forward_crisis_training_regime_with_context(
    as_of_date: NaiveDate,
    positive_scenarios: &[CrisisScenario],
    context_scenarios: &[CrisisScenario],
    horizon_days: u32,
) -> ProbabilityTrainingRegime {
    let base_regime = forward_crisis_training_regime(as_of_date, positive_scenarios, horizon_days);
    if !matches!(base_regime, ProbabilityTrainingRegime::Normal) || horizon_days < 20 {
        return base_regime;
    }

    match protected_context_phase_for_date(as_of_date, positive_scenarios, context_scenarios) {
        Some(ActionEpisodePhase::Primary | ActionEpisodePhase::LateValidation) => {
            ProbabilityTrainingRegime::PreWarningBuffer
        }
        Some(ActionEpisodePhase::Cooldown) => ProbabilityTrainingRegime::PostCrisisCooldown,
        _ => base_regime,
    }
}

fn ensure_positive_labels(
    rows: &[ProbabilityTrainingRow],
    horizon_days: u32,
    split_name: &str,
    label_mode: ProbabilityTargetLabelMode,
) -> anyhow::Result<()> {
    let positives = rows
        .iter()
        .filter(|row| row.label_for_horizon(label_mode, horizon_days) > 0.0)
        .count();
    if positives == 0 {
        bail!(
            "no positive {horizon_days}d {} labels found in the {split_name} split",
            label_mode.as_str()
        );
    }
    Ok(())
}

async fn backfill_fred(args: &[String]) -> anyhow::Result<()> {
    let options = FredBackfillOptions::parse(args)?;
    backfill_fred_with_options(options).await
}

async fn backfill_fred_with_options(options: FredBackfillOptions) -> anyhow::Result<()> {
    let store = open_sqlite_store().await?;
    store.migrate().await?;
    store.seed_fred_metadata().await?;

    let fred_mode = options.fred_mode;
    let connector: Box<dyn Connector> = match fred_mode {
        FredBackfillMode::GraphCsv => Box::new(FredGraphCsvConnector::new()),
        FredBackfillMode::Api => {
            let api_key = env::var("FRED_API_KEY")
                .context("FRED_API_KEY is required only when using `backfill fred --api`.")?;
            Box::new(FredConnector::new(Some(api_key)))
        }
    };
    let mappings = store.load_fred_mappings().await?;
    if mappings.is_empty() {
        bail!("no FRED mappings found; run `just db-seed` first");
    }

    let backfill_options = if matches!(fred_mode, FredBackfillMode::GraphCsv) {
        options.options.with_default_chunk_days(366)
    } else {
        options.options
    };
    backfill_mappings(
        connector.as_ref(),
        mappings,
        FRED_DATASET_ID,
        backfill_options,
        "FRED",
    )
    .await
}

async fn backfill_treasury_yield(args: &[String]) -> anyhow::Result<()> {
    let options = BackfillOptions::parse(args)?;
    backfill_treasury_yield_with_options(options).await
}

async fn backfill_treasury_yield_with_options(options: BackfillOptions) -> anyhow::Result<()> {
    let store = open_sqlite_store().await?;
    store.migrate().await?;
    store.seed_fred_metadata().await?;

    let mappings = store.load_treasury_yield_mappings().await?;
    if mappings.is_empty() {
        bail!("no Treasury yield mappings found; run `just db-seed` first");
    }

    let connector = TreasuryYieldCurveConnector::new();
    backfill_mappings(
        &connector,
        mappings,
        TREASURY_YIELD_DATASET_ID,
        options,
        "Treasury yield",
    )
    .await
}

async fn backfill_world_bank(args: &[String]) -> anyhow::Result<()> {
    let options = BackfillOptions::parse(args)?;
    backfill_world_bank_with_options(options).await
}

async fn backfill_world_bank_with_options(options: BackfillOptions) -> anyhow::Result<()> {
    let store = open_sqlite_store().await?;
    store.migrate().await?;
    store.seed_fred_metadata().await?;

    let mappings = store.load_world_bank_mappings().await?;
    if mappings.is_empty() {
        bail!("no World Bank mappings found; run `just db-seed` first");
    }

    let connector = WorldBankConnector::new();
    backfill_mappings(
        &connector,
        mappings,
        WORLD_BANK_DATASET_ID,
        options,
        "World Bank",
    )
    .await
}

async fn backfill_gdelt(args: &[String]) -> anyhow::Result<()> {
    let options = BackfillOptions::parse(args)?;
    backfill_gdelt_with_options(options).await
}

async fn backfill_gdelt_with_options(options: BackfillOptions) -> anyhow::Result<()> {
    let store = open_sqlite_store().await?;
    store.migrate().await?;
    store.seed_fred_metadata().await?;

    let connector = GdeltConnector::new();
    let effective_start = if let Some(overlap_days) = options.watermark_overlap_days {
        let watermark = store
            .load_watermark_date(
                "gdelt",
                GDELT_DOC_DATASET_ID,
                "global_news_financial_stress_count",
            )
            .await?;
        options.effective_start(watermark, overlap_days)
    } else {
        options.start
    };
    if effective_start > options.end {
        println!("GDELT backfill skipped: watermark is already beyond requested range.");
        return Ok(());
    }

    println!(
        "Backfilling GDELT timeline aggregates into {} [{}..{}]",
        sqlite_path(),
        effective_start,
        options.end
    );
    let output = connector
        .backfill_range(effective_start, options.end)
        .await?;
    let raw_root = raw_data_dir();
    let raw_path = write_raw_payload(
        &raw_root,
        "gdelt",
        "global_news_financial_stress_count",
        "json",
        &output.payload_body,
    )?;
    let raw_payload_id = Uuid::new_v4();
    store
        .insert_raw_response(&RawResponseRecord {
            raw_payload_id,
            source_id: "gdelt".to_string(),
            dataset_id: GDELT_DOC_DATASET_ID.to_string(),
            request_url: output.payload_url.clone(),
            request_params_hash: Some(simple_hash(&output.payload_url)),
            response_hash: simple_hash(&output.payload_body),
            content_type: "application/json".to_string(),
            content_length: output.payload_body.len() as i64,
            raw_file_path: path_to_string(&raw_path),
            fetched_at: Utc::now(),
        })
        .await?;
    store.insert_observations(&output.observations).await?;
    store
        .replace_alerts_for_scope("gdelt_daily", effective_start, options.end, &output.alerts)
        .await?;
    if let Some(latest_date) = output.latest_date {
        store
            .upsert_watermark(
                "gdelt",
                GDELT_DOC_DATASET_ID,
                "global_news_financial_stress_count",
                latest_date,
            )
            .await?;
    }

    println!(
        "GDELT backfill completed: {} observations, {} alerts",
        output.observations.len(),
        output.alerts.len()
    );
    Ok(())
}

async fn backfill_sec_edgar(args: &[String]) -> anyhow::Result<()> {
    let options = BackfillOptions::parse(args)?;
    backfill_sec_edgar_with_options(options).await
}

async fn backfill_sec_edgar_with_options(options: BackfillOptions) -> anyhow::Result<()> {
    let store = open_sqlite_store().await?;
    store.migrate().await?;
    store.seed_fred_metadata().await?;

    println!(
        "Backfilling SEC EDGAR filing events into {} [{}..{}]",
        sqlite_path(),
        options.start,
        options.end
    );

    let connector = SecEdgarConnector::new();
    let output = connector.backfill_range(options.start, options.end).await?;
    let raw_root = raw_data_dir();

    for payload in &output.payloads {
        let raw_path = write_raw_payload(
            &raw_root,
            &payload.source_id,
            SEC_SUBMISSIONS_DATASET_ID,
            raw_file_extension(&payload.content_type),
            &payload.body,
        )?;
        store
            .insert_raw_response(&RawResponseRecord {
                raw_payload_id: payload.raw_payload_id,
                source_id: payload.source_id.clone(),
                dataset_id: payload.dataset_id.clone(),
                request_url: payload.request_url.clone(),
                request_params_hash: Some(simple_hash(&payload.request_url)),
                response_hash: payload.response_hash.clone(),
                content_type: payload.content_type.clone(),
                content_length: payload.body.len() as i64,
                raw_file_path: path_to_string(&raw_path),
                fetched_at: payload.fetched_at,
            })
            .await?;
    }

    store.insert_observations(&output.observations).await?;
    store
        .replace_alerts_for_scope(
            "sec_edgar_daily",
            options.start,
            options.end,
            &output.alerts,
        )
        .await?;
    if let Some(latest_filing_date) = output.latest_filing_date {
        store
            .upsert_watermark("sec_edgar", SEC_EVENTS_DATASET_ID, "us", latest_filing_date)
            .await?;
    }

    println!(
        "SEC EDGAR backfill completed: {} payloads, {} filings, {} observations, {} alerts",
        output.payloads.len(),
        output.filing_count,
        output.observations.len(),
        output.alerts.len()
    );
    Ok(())
}

async fn backfill_jpy_carry(args: &[String]) -> anyhow::Result<()> {
    let options = BackfillOptions::parse(args)?;
    backfill_jpy_carry_with_options(options).await
}

async fn backfill_jpy_carry_with_options(options: BackfillOptions) -> anyhow::Result<()> {
    let store = open_sqlite_store().await?;
    store.migrate().await?;
    store.seed_fred_metadata().await?;

    let mappings = store.load_jpy_carry_mappings().await?;
    if mappings.is_empty() {
        bail!("no JPY carry mappings found; run `just db-seed` first");
    }

    let boj_mappings = mappings
        .iter()
        .filter(|mapping| mapping.external_code.starts_with("FXER"))
        .cloned()
        .collect::<Vec<_>>();
    if !boj_mappings.is_empty() {
        let connector = BojConnector::fx_daily();
        if backfill_mappings(
            &connector,
            boj_mappings,
            BOJ_FX_DATASET_ID,
            options.clone(),
            "JPY carry (BOJ)",
        )
        .await
        .is_ok()
        {
            return Ok(());
        }
        tracing::warn!("BOJ FX backfill failed, falling back to FRED graph CSV");
    }

    let fred_mappings = mappings
        .into_iter()
        .filter(|mapping| mapping.external_code == "DEXJPUS")
        .collect::<Vec<_>>();
    if fred_mappings.is_empty() {
        bail!("no FRED fallback mappings found for JPY carry");
    }
    let connector = FredGraphCsvConnector::new();
    backfill_mappings(
        &connector,
        fred_mappings,
        FRED_DATASET_ID,
        options,
        "JPY carry (FRED fallback)",
    )
    .await
}

async fn backfill_boj(args: &[String]) -> anyhow::Result<()> {
    let options = BojBackfillOptions::parse(args)?;
    backfill_boj_with_options(options).await
}

async fn backfill_boj_with_options(options: BojBackfillOptions) -> anyhow::Result<()> {
    let store = open_sqlite_store().await?;
    store.migrate().await?;
    store.seed_fred_metadata().await?;

    match options.dataset {
        BojDataset::FxDaily => {
            let mappings = store.load_jpy_carry_mappings().await?;
            let mappings = mappings
                .into_iter()
                .filter(|mapping| mapping.external_code.starts_with("FXER"))
                .collect::<Vec<_>>();
            if mappings.is_empty() {
                bail!("no BOJ FX mappings found; run `just db-seed` first");
            }
            let connector = BojConnector::fx_daily();
            backfill_mappings(
                &connector,
                mappings,
                BOJ_FX_DATASET_ID,
                options.options,
                "BOJ FX daily",
            )
            .await
        }
        BojDataset::MoneyMarketRates => {
            let mappings = store.load_boj_money_market_mappings().await?;
            if mappings.is_empty() {
                bail!("no BOJ money market mappings found; run `just db-seed` first");
            }
            let connector = BojConnector::money_market_rates();
            backfill_mappings(
                &connector,
                mappings,
                BOJ_MONEY_MARKET_DATASET_ID,
                options.options,
                "BOJ money market rates",
            )
            .await
        }
    }
}

async fn backfill_mappings(
    connector: &dyn Connector,
    mappings: Vec<ExternalIndicatorMapping>,
    dataset_id: &str,
    options: BackfillOptions,
    label: &str,
) -> anyhow::Result<()> {
    let store = open_sqlite_store().await?;
    let raw_root = raw_data_dir();
    let mappings = options.filter_mappings(mappings);
    if mappings.is_empty() {
        bail!("{label} backfill found no mappings matching the requested filters");
    }
    let mut total_written = 0_usize;
    let mut failures = Vec::new();
    let chunks = options.chunks();
    let chunk_count = chunks.len();
    for mapping in mappings {
        for (chunk_index, (chunk_start, chunk_end)) in chunks.iter().copied().enumerate() {
            let plan = FetchPlan {
                source_id: connector.describe().source_id,
                dataset_id: dataset_id.to_string(),
                target_id: mapping.indicator_id.clone(),
                external_code: Some(mapping.external_code.clone()),
                run_mode: RunMode::Backfill,
                requested_start: Some(chunk_start),
                requested_end: Some(chunk_end),
                frequency: mapping.frequency,
            };
            tracing::info!(
                indicator_id = %plan.target_id,
                external_code = %mapping.external_code,
                source_id = %plan.source_id,
                chunk = chunk_index + 1,
                chunks = chunk_count,
                start = %chunk_start,
                end = %chunk_end,
                "fetching observations"
            );

            let result: anyhow::Result<usize> = async {
                let payload = connector.fetch(&plan).await?;
                let raw_path = write_raw_payload(
                    &raw_root,
                    &payload.source_id,
                    &mapping.external_code,
                    raw_file_extension(&payload.content_type),
                    &payload.body,
                )?;
                store
                    .insert_raw_response(&RawResponseRecord {
                        raw_payload_id: payload.raw_payload_id,
                        source_id: payload.source_id.clone(),
                        dataset_id: payload.dataset_id.clone(),
                        request_url: payload.request_url.clone(),
                        request_params_hash: Some(simple_hash(&payload.request_url)),
                        response_hash: payload.response_hash.clone(),
                        content_type: payload.content_type.clone(),
                        content_length: payload.body.len() as i64,
                        raw_file_path: path_to_string(&raw_path),
                        fetched_at: payload.fetched_at,
                    })
                    .await?;
                let batch = connector.parse(&plan, &payload)?;
                let latest_date = batch
                    .observations
                    .iter()
                    .map(|observation| observation.as_of_date)
                    .max();
                let written = batch.observations.len();
                store
                    .insert_observations_with_raw_payload(
                        &batch.observations,
                        Some(payload.raw_payload_id),
                    )
                    .await?;
                if let Some(latest_date) = latest_date {
                    store
                        .upsert_watermark(
                            &payload.source_id,
                            &payload.dataset_id,
                            &mapping.indicator_id,
                            latest_date,
                        )
                        .await?;
                }
                if written == 0 {
                    tracing::warn!(
                        indicator_id = %mapping.indicator_id,
                        external_code = %mapping.external_code,
                        start = %chunk_start,
                        end = %chunk_end,
                        "no observations were written for requested range"
                    );
                }
                println!(
                    "backfilled {} ({}) from {} with {} observations [{}..{}]",
                    mapping.indicator_id,
                    mapping.external_code,
                    payload.source_id,
                    written,
                    chunk_start,
                    chunk_end
                );
                for warning in batch.warnings.iter().take(3) {
                    tracing::warn!(%warning, indicator_id = %mapping.indicator_id, "parse warning");
                }
                Ok(written)
            }
            .await;

            match result {
                Ok(written) => total_written += written,
                Err(error) => {
                    let failure = format!(
                        "{} ({}) [{}..{}]: {error:#}",
                        mapping.indicator_id, mapping.external_code, chunk_start, chunk_end
                    );
                    tracing::warn!(%failure, "backfill chunk failed");
                    failures.push(failure);
                }
            }
        }
    }

    if failures.is_empty() {
        println!(
            "{} backfill completed: {} observations written to {}",
            label,
            total_written,
            sqlite_path()
        );
        Ok(())
    } else {
        println!(
            "{} backfill partially completed: {} observations written to {}, {} chunk(s) failed",
            label,
            total_written,
            sqlite_path(),
            failures.len()
        );
        for failure in failures.iter().take(5) {
            println!("  failed: {failure}");
        }
        bail!(
            "{} backfill had {} failed chunk(s); retry the command to fill missing gaps",
            label,
            failures.len()
        )
    }
}

async fn run_demo_ingestion() -> anyhow::Result<()> {
    let connector = MockConnector;
    let plan = FetchPlan {
        source_id: "mock".to_string(),
        dataset_id: "demo".to_string(),
        target_id: "us_market_vix_close".to_string(),
        external_code: None,
        run_mode: RunMode::Incremental,
        requested_start: Some(NaiveDate::from_ymd_opt(2026, 5, 1).expect("valid date")),
        requested_end: Some(NaiveDate::from_ymd_opt(2026, 5, 30).expect("valid date")),
        frequency: Frequency::Daily,
    };

    let payload = connector.fetch(&plan).await?;
    let batch = connector.parse(&plan, &payload)?;

    tracing::info!(
        source_id = %batch.source_id,
        dataset_id = %batch.dataset_id,
        records = batch.observations.len(),
        "worker completed one demo ingestion run"
    );
    println!("{}", serde_json::to_string_pretty(&batch)?);

    Ok(())
}

#[derive(Debug, Clone)]
struct BackfillOptions {
    start: NaiveDate,
    end: NaiveDate,
    chunk_days: Option<i64>,
    indicator_filter: Option<String>,
    external_code_filter: Option<String>,
    watermark_overlap_days: Option<i64>,
}

impl BackfillOptions {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut start = NaiveDate::from_ymd_opt(1990, 1, 1).expect("valid date");
        let mut end = Utc::now().date_naive();
        let mut chunk_days = None;
        let mut indicator_filter = None;
        let mut external_code_filter = None;
        let mut watermark_overlap_days = None;
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--start" => {
                    index += 1;
                    start = parse_date_arg(args.get(index), "--start")?;
                }
                "--end" => {
                    index += 1;
                    end = parse_date_arg(args.get(index), "--end")?;
                }
                "--chunk-days" => {
                    index += 1;
                    let value = args
                        .get(index)
                        .with_context(|| "--chunk-days requires a positive integer")?
                        .parse::<i64>()
                        .with_context(|| "--chunk-days requires a positive integer")?;
                    if value <= 0 {
                        bail!("--chunk-days requires a positive integer");
                    }
                    chunk_days = Some(value);
                }
                "--indicator" => {
                    index += 1;
                    indicator_filter = Some(
                        args.get(index)
                            .with_context(|| "--indicator requires an indicator_id")?
                            .clone(),
                    );
                }
                "--external-code" => {
                    index += 1;
                    external_code_filter = Some(
                        args.get(index)
                            .with_context(|| "--external-code requires a source code")?
                            .clone(),
                    );
                }
                "--watermark-overlap-days" => {
                    index += 1;
                    watermark_overlap_days = Some(parse_positive_i64(
                        args.get(index),
                        "--watermark-overlap-days",
                    )?);
                }
                other => bail!("unknown backfill option: {other}"),
            }
            index += 1;
        }
        if start > end {
            bail!("--start must be on or before --end");
        }
        Ok(Self {
            start,
            end,
            chunk_days,
            indicator_filter,
            external_code_filter,
            watermark_overlap_days,
        })
    }

    fn with_default_chunk_days(mut self, chunk_days: i64) -> Self {
        if self.chunk_days.is_none() {
            self.chunk_days = Some(chunk_days);
        }
        self
    }

    fn chunks(&self) -> Vec<(NaiveDate, NaiveDate)> {
        self.chunks_for_range(self.start, self.end)
    }

    fn chunks_for_range(&self, start: NaiveDate, end: NaiveDate) -> Vec<(NaiveDate, NaiveDate)> {
        let Some(chunk_days) = self.chunk_days else {
            return vec![(start, end)];
        };

        let mut chunks = Vec::new();
        let mut cursor = start;
        while cursor <= end {
            let chunk_end = (cursor + chrono::Duration::days(chunk_days - 1)).min(end);
            chunks.push((cursor, chunk_end));
            if chunk_end == end {
                break;
            }
            cursor = chunk_end + chrono::Duration::days(1);
        }
        chunks
    }

    fn filter_mappings(
        &self,
        mappings: Vec<ExternalIndicatorMapping>,
    ) -> Vec<ExternalIndicatorMapping> {
        mappings
            .into_iter()
            .filter(|mapping| {
                self.indicator_filter
                    .as_ref()
                    .map(|filter| mapping.indicator_id == *filter)
                    .unwrap_or(true)
                    && self
                        .external_code_filter
                        .as_ref()
                        .map(|filter| mapping.external_code == *filter)
                        .unwrap_or(true)
            })
            .collect()
    }

    fn effective_start(&self, watermark: Option<NaiveDate>, overlap_days: i64) -> NaiveDate {
        watermark
            .map(|date| (date - chrono::Duration::days(overlap_days)).max(self.start))
            .unwrap_or(self.start)
    }
}

#[derive(Debug, Clone)]
struct FredBackfillOptions {
    options: BackfillOptions,
    fred_mode: FredBackfillMode,
}

impl FredBackfillOptions {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut filtered_args = Vec::new();
        let mut fred_mode = FredBackfillMode::GraphCsv;
        for arg in args {
            match arg.as_str() {
                "--api" => fred_mode = FredBackfillMode::Api,
                "--graph-csv" => fred_mode = FredBackfillMode::GraphCsv,
                _ => filtered_args.push(arg.clone()),
            }
        }
        Ok(Self {
            options: BackfillOptions::parse(&filtered_args)?,
            fred_mode,
        })
    }
}

#[derive(Debug, Clone)]
struct BojBackfillOptions {
    options: BackfillOptions,
    dataset: BojDataset,
}

impl BojBackfillOptions {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut filtered_args = Vec::new();
        let mut dataset = BojDataset::FxDaily;
        let mut index = 0;
        while index < args.len() {
            if args[index] == "--dataset" {
                index += 1;
                let value = args
                    .get(index)
                    .context("--dataset requires `fx-daily` or `money-market`")?;
                dataset = match value.as_str() {
                    "fx-daily" => BojDataset::FxDaily,
                    "money-market" => BojDataset::MoneyMarketRates,
                    other => bail!("unsupported BOJ dataset: {other}"),
                };
            } else {
                filtered_args.push(args[index].clone());
            }
            index += 1;
        }
        Ok(Self {
            options: BackfillOptions::parse(&filtered_args)?,
            dataset,
        })
    }
}

#[derive(Debug, Clone)]
struct RefreshLatestOptions {
    fast_lookback_days: i64,
    slow_lookback_years: i64,
    fred_chunk_days: i64,
    skip_world_bank: bool,
    include_gdelt: bool,
    reload_api: bool,
    api_reload_url: String,
}

impl RefreshLatestOptions {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut fast_lookback_days = 45_i64;
        let mut slow_lookback_years = 15_i64;
        let mut fred_chunk_days = 45_i64;
        let mut skip_world_bank = false;
        let mut include_gdelt = false;
        let mut reload_api = true;
        let mut api_reload_url = DEFAULT_API_RELOAD_URL.to_string();
        let mut index = 0;

        while index < args.len() {
            match args[index].as_str() {
                "--fast-lookback-days" => {
                    index += 1;
                    fast_lookback_days =
                        parse_positive_i64(args.get(index), "--fast-lookback-days")?;
                }
                "--slow-lookback-years" => {
                    index += 1;
                    slow_lookback_years =
                        parse_positive_i64(args.get(index), "--slow-lookback-years")?;
                }
                "--fred-chunk-days" => {
                    index += 1;
                    fred_chunk_days = parse_positive_i64(args.get(index), "--fred-chunk-days")?;
                }
                "--skip-world-bank" => {
                    skip_world_bank = true;
                }
                "--include-gdelt" => {
                    include_gdelt = true;
                }
                "--no-reload-api" => {
                    reload_api = false;
                }
                "--api-reload-url" => {
                    index += 1;
                    api_reload_url = args
                        .get(index)
                        .with_context(|| "--api-reload-url requires a URL")?
                        .clone();
                }
                other => bail!("unknown refresh option: {other}"),
            }
            index += 1;
        }

        Ok(Self {
            fast_lookback_days,
            slow_lookback_years,
            fred_chunk_days,
            skip_world_bank,
            include_gdelt,
            reload_api,
            api_reload_url,
        })
    }
}

#[derive(Debug, Clone, Copy)]
enum FredBackfillMode {
    GraphCsv,
    Api,
}

async fn fetch_api_json<T: DeserializeOwned>(
    client: &reqwest::Client,
    api_base_url: &str,
    path: &str,
) -> anyhow::Result<T> {
    let response = client
        .get(format!("{api_base_url}{path}"))
        .send()
        .await
        .with_context(|| format!("request to {api_base_url}{path} failed"))?;
    let status = response.status();
    if !status.is_success() {
        bail!("request to {api_base_url}{path} returned {status}");
    }
    response
        .json::<T>()
        .await
        .with_context(|| format!("failed to decode JSON from {api_base_url}{path}"))
}

async fn fetch_assessment_snapshot_for_guard(
    api_reload_url: &str,
) -> anyhow::Result<AssessmentSnapshot> {
    let api_base_url = api_reload_url
        .strip_suffix("/api/system/reload")
        .with_context(|| {
            format!(
                "cannot derive API base URL from reload URL {api_reload_url}; expected it to end with /api/system/reload"
            )
        })?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;
    fetch_api_json(&client, api_base_url, "/api/assessment/current").await
}

fn build_release_runtime_review_diagnostics(
    release_id: &str,
    label_version: &str,
    method: &AuditMethodResponseWire,
    history: &[AssessmentHistoryPoint],
) -> ReleaseRuntimeReviewDiagnostics {
    let posture_distribution =
        summarize_named_counts(history.iter().map(|point| match point.posture {
            fc_domain::DecisionPosture::Normal => "normal",
            fc_domain::DecisionPosture::Prepare => "prepare",
            fc_domain::DecisionPosture::Hedge => "hedge",
            fc_domain::DecisionPosture::Defend => "defend",
        }));
    let time_bucket_distribution =
        summarize_named_counts(history.iter().map(|point| match point.time_to_risk_bucket {
            fc_domain::TimeToRiskBucket::Normal => "normal",
            fc_domain::TimeToRiskBucket::Months => "months",
            fc_domain::TimeToRiskBucket::Weeks => "weeks",
            fc_domain::TimeToRiskBucket::Now => "now",
        }));
    let posture_trigger_distribution =
        summarize_posture_clause_counts(history, |point| &point.posture_trigger_codes);
    let posture_blocker_distribution =
        summarize_posture_clause_counts(history, |point| &point.posture_blocker_codes);
    let (
        points_at_or_above_prepare_p60d,
        points_at_or_above_hedge_p20d,
        points_at_or_above_defend_p5d,
        mut notes,
    ) = if let Some(thresholds) = method.runtime_thresholds.as_ref() {
        (
            Some(
                history
                    .iter()
                    .filter(|point| point.p_60d >= thresholds.prepare_p60d)
                    .count(),
            ),
            Some(
                history
                    .iter()
                    .filter(|point| point.p_20d >= thresholds.hedge_p20d)
                    .count(),
            ),
            Some(
                history
                    .iter()
                    .filter(|point| point.p_5d >= thresholds.defend_p5d)
                    .count(),
            ),
            vec!["基于运行中 API 返回的 runtime_thresholds 统计历史概率越线次数。".to_string()],
        )
    } else {
        (
            None,
            None,
            None,
            vec![
                "运行中的 API 没有返回 runtime_thresholds；本报告只保留 posture / time bucket 分布。"
                    .to_string(),
            ],
        )
    };
    let regime_probability_summaries = match load_release_review_regime_scenarios(label_version) {
        Ok((scenarios, scenario_note)) => {
            notes.push(scenario_note);
            summarize_release_runtime_regime_probabilities(
                history,
                &scenarios,
                method.runtime_thresholds.as_ref(),
            )
        }
        Err(error) => {
            notes.push(format!(
                "未能加载 release review 所需的 regime scenario catalog，跳过 regime 概率分布：{error:#}"
            ));
            Vec::new()
        }
    };
    let regime_separation_summaries =
        summarize_release_runtime_regime_separation(&regime_probability_summaries);
    if !regime_separation_summaries.is_empty() {
        notes.push(render_release_runtime_separation_note(
            &regime_separation_summaries,
        ));
    }

    ReleaseRuntimeReviewDiagnostics {
        release_id: release_id.to_string(),
        history_point_count: history.len(),
        posture_distribution,
        time_bucket_distribution,
        posture_trigger_distribution,
        posture_blocker_distribution,
        regime_probability_summaries,
        regime_separation_summaries,
        runtime_thresholds: method.runtime_thresholds.clone(),
        points_at_or_above_prepare_p60d,
        points_at_or_above_hedge_p20d,
        points_at_or_above_defend_p5d,
        note: notes.join(" "),
    }
}

fn load_release_review_regime_scenarios(
    label_version: &str,
) -> anyhow::Result<(Vec<CrisisScenario>, String)> {
    match load_label_set_crisis_scenarios(DEFAULT_FORMAL_SCENARIO_SET_VERSION, label_version) {
        Ok(scenarios) => Ok((
            scenarios,
            format!(
                "Regime 概率分布基于 {DEFAULT_FORMAL_SCENARIO_SET_VERSION}/{label_version} 重算。"
            ),
        )),
        Err(primary_error) if label_version == "label_forward_crisis_v1" => {
            let fallback = load_label_set_crisis_scenarios(
                DEFAULT_FORMAL_SCENARIO_SET_VERSION,
                DEFAULT_FORMAL_LABEL_VERSION,
            )?;
            Ok((
                fallback,
                format!(
                    "当前 release label_version={label_version} 不在 scenario catalog 中，Regime 概率分布回退到 {DEFAULT_FORMAL_SCENARIO_SET_VERSION}/{DEFAULT_FORMAL_LABEL_VERSION} 重算（原始错误：{primary_error:#}）。"
                ),
            ))
        }
        Err(error) => Err(error),
    }
}

fn summarize_release_runtime_regime_probabilities(
    history: &[AssessmentHistoryPoint],
    scenarios: &[CrisisScenario],
    runtime_thresholds: Option<&RuntimeThresholdDiagnosticsWire>,
) -> Vec<ReleaseRuntimeRegimeProbabilitySummary> {
    #[derive(Default)]
    struct Accumulator {
        row_count: usize,
        raw_probability_sum: f64,
        max_raw_probability: f64,
        calibrated_probability_sum: f64,
        max_calibrated_probability: f64,
        threshold_hit_count: usize,
    }

    let mut buckets = BTreeMap::<(u32, String), Accumulator>::new();
    for point in history {
        for (horizon_days, raw_probability, calibrated_probability) in [
            (5_u32, point.raw_p_5d.unwrap_or(point.p_5d), point.p_5d),
            (20_u32, point.raw_p_20d.unwrap_or(point.p_20d), point.p_20d),
            (60_u32, point.raw_p_60d.unwrap_or(point.p_60d), point.p_60d),
        ] {
            let regime = probability_training_regime_name(forward_crisis_training_regime(
                point.as_of_date,
                scenarios,
                horizon_days,
            ));
            let bucket = buckets
                .entry((horizon_days, regime.to_string()))
                .or_default();
            bucket.row_count += 1;
            bucket.raw_probability_sum += raw_probability;
            bucket.max_raw_probability = bucket.max_raw_probability.max(raw_probability);
            bucket.calibrated_probability_sum += calibrated_probability;
            bucket.max_calibrated_probability = bucket
                .max_calibrated_probability
                .max(calibrated_probability);
            if let Some(threshold) =
                runtime_probability_threshold_for_horizon(runtime_thresholds, horizon_days)
            {
                if calibrated_probability >= threshold {
                    bucket.threshold_hit_count += 1;
                }
            }
        }
    }

    let normal_baselines = buckets
        .iter()
        .filter_map(|((horizon_days, regime), bucket)| {
            if regime != "normal" {
                return None;
            }
            Some((
                *horizon_days,
                (
                    safe_divide(bucket.raw_probability_sum, bucket.row_count as f64),
                    safe_divide(bucket.calibrated_probability_sum, bucket.row_count as f64),
                ),
            ))
        })
        .collect::<BTreeMap<_, _>>();

    buckets
        .into_iter()
        .map(|((horizon_days, regime), bucket)| {
            let avg_raw_probability =
                safe_divide(bucket.raw_probability_sum, bucket.row_count as f64);
            let avg_calibrated_probability =
                safe_divide(bucket.calibrated_probability_sum, bucket.row_count as f64);
            let (
                raw_lift_vs_normal,
                calibrated_lift_vs_normal,
                raw_gap_vs_normal,
                calibrated_gap_vs_normal,
                calibration_gap_retention,
            ) = if let Some((normal_avg_raw, normal_avg_calibrated)) =
                normal_baselines.get(&horizon_days).copied()
            {
                let raw_gap = avg_raw_probability - normal_avg_raw;
                let calibrated_gap = avg_calibrated_probability - normal_avg_calibrated;
                (
                    lift_vs_baseline(avg_raw_probability, normal_avg_raw),
                    lift_vs_baseline(avg_calibrated_probability, normal_avg_calibrated),
                    Some(round6(raw_gap)),
                    Some(round6(calibrated_gap)),
                    gap_retention_ratio(raw_gap, calibrated_gap),
                )
            } else {
                (None, None, None, None, None)
            };

            ReleaseRuntimeRegimeProbabilitySummary {
                horizon_days,
                regime,
                row_count: bucket.row_count,
                row_rate: round6(safe_ratio(bucket.row_count, history.len())),
                avg_raw_probability: round6(avg_raw_probability),
                max_raw_probability: round6(bucket.max_raw_probability),
                avg_probability: round6(avg_calibrated_probability),
                max_probability: round6(bucket.max_calibrated_probability),
                raw_lift_vs_normal,
                calibrated_lift_vs_normal,
                raw_gap_vs_normal,
                calibrated_gap_vs_normal,
                calibration_gap_retention,
                threshold_hit_count: runtime_thresholds.map(|_| bucket.threshold_hit_count),
            }
        })
        .collect()
}

fn summarize_release_runtime_regime_separation(
    summaries: &[ReleaseRuntimeRegimeProbabilitySummary],
) -> Vec<ReleaseRuntimeSeparationSummary> {
    let mut by_horizon = BTreeMap::<u32, Vec<&ReleaseRuntimeRegimeProbabilitySummary>>::new();
    for summary in summaries {
        by_horizon
            .entry(summary.horizon_days)
            .or_default()
            .push(summary);
    }

    by_horizon
        .into_iter()
        .filter_map(|(horizon_days, rows)| {
            let normal = rows.iter().copied().find(|row| row.regime == "normal")?;
            let pre_warning_buffer = rows
                .iter()
                .copied()
                .find(|row| row.regime == "pre_warning_buffer");
            let positive_window = rows
                .iter()
                .copied()
                .find(|row| row.regime == "positive_window");
            let max_non_normal = rows
                .iter()
                .copied()
                .filter(|row| row.regime != "normal")
                .max_by(|left, right| {
                    left.avg_probability
                        .partial_cmp(&right.avg_probability)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })?;
            let early_warning_regime_name = early_warning_regime_name(horizon_days);
            let early_warning = rows
                .iter()
                .copied()
                .find(|row| row.regime == early_warning_regime_name);
            let in_crisis = rows.iter().copied().find(|row| row.regime == "in_crisis");
            let post_crisis_cooldown = rows
                .iter()
                .copied()
                .find(|row| row.regime == "post_crisis_cooldown");
            let max_non_normal_threshold_hit_rate = max_non_normal
                .threshold_hit_count
                .map(|count| round6(safe_divide(count as f64, max_non_normal.row_count as f64)));
            let diagnosis = classify_regime_separation(
                horizon_days,
                early_warning
                    .and_then(|row| row.raw_lift_vs_normal)
                    .unwrap_or_default(),
                early_warning
                    .and_then(|row| row.calibrated_lift_vs_normal)
                    .unwrap_or_default(),
                early_warning.and_then(|row| row.calibration_gap_retention),
                positive_window
                    .and_then(|row| row.calibrated_lift_vs_normal)
                    .unwrap_or_default(),
                positive_window
                    .and_then(|row| row.calibrated_gap_vs_normal)
                    .unwrap_or_default(),
                in_crisis
                    .and_then(|row| row.calibrated_lift_vs_normal)
                    .unwrap_or_default(),
                post_crisis_cooldown
                    .and_then(|row| row.calibrated_lift_vs_normal)
                    .unwrap_or_default(),
                post_crisis_cooldown
                    .and_then(|row| row.calibrated_gap_vs_normal)
                    .unwrap_or_default(),
                max_non_normal.calibrated_lift_vs_normal.unwrap_or_default(),
                max_non_normal_threshold_hit_rate.unwrap_or_default(),
            )
            .to_string();

            Some(ReleaseRuntimeSeparationSummary {
                horizon_days,
                early_warning_regime: early_warning_regime_name.to_string(),
                normal_avg_probability: normal.avg_probability,
                pre_warning_buffer_avg_probability: pre_warning_buffer
                    .map(|row| row.avg_probability)
                    .unwrap_or_default(),
                positive_window_avg_probability: positive_window
                    .map(|row| row.avg_probability)
                    .unwrap_or_default(),
                in_crisis_avg_probability: in_crisis
                    .map(|row| row.avg_probability)
                    .unwrap_or_default(),
                post_crisis_cooldown_avg_probability: post_crisis_cooldown
                    .map(|row| row.avg_probability)
                    .unwrap_or_default(),
                early_warning_raw_lift_vs_normal: early_warning
                    .and_then(|row| row.raw_lift_vs_normal),
                early_warning_calibrated_lift_vs_normal: early_warning
                    .and_then(|row| row.calibrated_lift_vs_normal),
                early_warning_gap_retention: early_warning
                    .and_then(|row| row.calibration_gap_retention),
                positive_window_calibrated_lift_vs_normal: positive_window
                    .and_then(|row| row.calibrated_lift_vs_normal),
                positive_window_gap_vs_normal: positive_window
                    .and_then(|row| row.calibrated_gap_vs_normal),
                in_crisis_raw_lift_vs_normal: in_crisis.and_then(|row| row.raw_lift_vs_normal),
                in_crisis_calibrated_lift_vs_normal: in_crisis
                    .and_then(|row| row.calibrated_lift_vs_normal),
                post_crisis_cooldown_calibrated_lift_vs_normal: post_crisis_cooldown
                    .and_then(|row| row.calibrated_lift_vs_normal),
                post_crisis_cooldown_gap_vs_normal: post_crisis_cooldown
                    .and_then(|row| row.calibrated_gap_vs_normal),
                max_non_normal_calibrated_lift_vs_normal: max_non_normal.calibrated_lift_vs_normal,
                max_non_normal_threshold_hit_rate,
                diagnosis,
            })
        })
        .collect()
}

fn early_warning_regime_name(horizon_days: u32) -> &'static str {
    match horizon_days {
        5 => "positive_window",
        20 | 60 => "pre_warning_buffer",
        _ => "positive_window",
    }
}

#[allow(clippy::too_many_arguments)]
fn classify_regime_separation(
    horizon_days: u32,
    early_warning_raw_lift: f64,
    early_warning_calibrated_lift: f64,
    early_warning_gap_retention: Option<f64>,
    positive_window_calibrated_lift: f64,
    positive_window_gap_vs_normal: f64,
    in_crisis_calibrated_lift: f64,
    post_crisis_cooldown_calibrated_lift: f64,
    post_crisis_cooldown_gap_vs_normal: f64,
    max_non_normal_calibrated_lift: f64,
    max_non_normal_threshold_hit_rate: f64,
) -> &'static str {
    if max_non_normal_calibrated_lift < 1.15
        && early_warning_raw_lift < 1.15
        && positive_window_calibrated_lift < 1.15
    {
        return "cold_across_all_regimes";
    }
    if early_warning_raw_lift >= 1.5
        && early_warning_calibrated_lift < 1.15
        && early_warning_gap_retention.unwrap_or_default() < 0.35
    {
        return "calibration_crushed_early_warning";
    }
    if positive_window_calibrated_lift < 1.15 && in_crisis_calibrated_lift >= 1.5 {
        return "late_only_no_early_warning";
    }
    if positive_window_calibrated_lift >= 1.15
        && post_crisis_cooldown_calibrated_lift >= positive_window_calibrated_lift
        && post_crisis_cooldown_gap_vs_normal + 0.002 >= positive_window_gap_vs_normal
    {
        return "cooldown_bleed";
    }
    if max_non_normal_calibrated_lift >= 1.5 && max_non_normal_threshold_hit_rate <= 0.01 {
        return "separated_but_below_runtime_floor";
    }
    if positive_window_calibrated_lift >= 1.5
        && positive_window_gap_vs_normal >= regime_positive_window_gap_floor(horizon_days)
    {
        return "usable_early_warning_separation";
    }
    if max_non_normal_calibrated_lift >= 1.15 || early_warning_calibrated_lift >= 1.15 {
        return "weak_regime_separation";
    }
    "mixed_or_unclear"
}

fn render_release_runtime_separation_note(summaries: &[ReleaseRuntimeSeparationSummary]) -> String {
    let joined = summaries
        .iter()
        .map(|summary| format!("{}d={}", summary.horizon_days, summary.diagnosis))
        .collect::<Vec<_>>()
        .join(", ");
    format!("Runtime separation summary: {joined}.")
}

fn lift_vs_baseline(value: f64, baseline: f64) -> Option<f64> {
    if baseline.abs() <= f64::EPSILON {
        return None;
    }
    Some(round6(value / baseline))
}

fn gap_retention_ratio(raw_gap: f64, calibrated_gap: f64) -> Option<f64> {
    if raw_gap.abs() <= f64::EPSILON {
        return None;
    }
    Some(round6(calibrated_gap / raw_gap))
}

fn runtime_probability_threshold_for_horizon(
    runtime_thresholds: Option<&RuntimeThresholdDiagnosticsWire>,
    horizon_days: u32,
) -> Option<f64> {
    runtime_thresholds.map(|thresholds| match horizon_days {
        5 => thresholds.defend_p5d,
        20 => thresholds.hedge_p20d,
        60 => thresholds.prepare_p60d,
        _ => 1.0,
    })
}

fn summarize_named_counts<'a>(names: impl Iterator<Item = &'a str>) -> Vec<ReleaseRuntimeCount> {
    let mut counts = BTreeMap::<String, usize>::new();
    for name in names {
        *counts.entry(name.to_string()).or_default() += 1;
    }
    let mut rows = counts
        .into_iter()
        .map(|(name, count)| ReleaseRuntimeCount { name, count })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.name.cmp(&right.name))
    });
    rows
}

fn summarize_posture_clause_counts<F>(
    history: &[AssessmentHistoryPoint],
    accessor: F,
) -> Vec<ReleaseRuntimeClauseCount>
where
    F: Fn(&AssessmentHistoryPoint) -> &[String],
{
    let posture_totals = history
        .iter()
        .fold(BTreeMap::<String, usize>::new(), |mut acc, point| {
            *acc.entry(runtime_posture_name(point).to_string())
                .or_default() += 1;
            acc
        });
    let mut counts = BTreeMap::<(String, String), usize>::new();
    for point in history {
        let posture = runtime_posture_name(point).to_string();
        for clause in accessor(point) {
            *counts.entry((posture.clone(), clause.clone())).or_default() += 1;
        }
    }

    let mut rows = counts
        .into_iter()
        .map(|((posture, clause), count)| {
            let posture_total = posture_totals.get(&posture).copied().unwrap_or_default();
            ReleaseRuntimeClauseCount {
                posture,
                clause,
                count,
                share_of_posture: round6(safe_ratio(count, posture_total)),
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.posture.cmp(&right.posture))
            .then_with(|| left.clause.cmp(&right.clause))
    });
    rows
}

fn runtime_posture_name(point: &AssessmentHistoryPoint) -> &'static str {
    match point.posture {
        fc_domain::DecisionPosture::Normal => "normal",
        fc_domain::DecisionPosture::Prepare => "prepare",
        fc_domain::DecisionPosture::Hedge => "hedge",
        fc_domain::DecisionPosture::Defend => "defend",
    }
}

fn render_release_review_markdown(report: &ReleaseReviewEnvelope) -> String {
    let mut markdown = String::new();
    let verdict = if report.overall_guard_passed {
        "PASS"
    } else {
        "FAIL"
    };
    let _ = writeln!(markdown, "# Release Review");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "- Reviewed at: {}", report.reviewed_at);
    let _ = writeln!(markdown, "- Market scope: {}", report.market_scope);
    let _ = writeln!(
        markdown,
        "- History mode: {} (limit {})",
        report.history_mode, report.history_limit
    );
    let _ = writeln!(markdown, "- Verdict: {verdict}");
    let _ = writeln!(
        markdown,
        "- Original active release: {}",
        report.original_active_release_id
    );
    let _ = writeln!(
        markdown,
        "- Restored release after review: {}",
        report.restored_release_id
    );
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Releases");
    let _ = writeln!(markdown);
    let _ = writeln!(
        markdown,
        "| Role | Release ID | Prob Mode | PIT | Feature | Label | Status |"
    );
    let _ = writeln!(markdown, "| --- | --- | --- | --- | --- | --- | --- |");
    for (role, release) in [
        ("baseline", &report.baseline_release),
        ("candidate", &report.candidate_release),
    ] {
        let _ = writeln!(
            markdown,
            "| {} | {} | {} | {} | {} | {} | {} |",
            role,
            release.manifest.release_id,
            release.manifest.probability_mode,
            release.manifest.point_in_time_mode,
            release.manifest.feature_set_version,
            release.manifest.label_version,
            release.manifest.status
        );
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Current Runtime Snapshot");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "| Metric | Baseline | Candidate | Delta |");
    let _ = writeln!(markdown, "| --- | --- | --- | --- |");
    let _ = writeln!(
        markdown,
        "| p_5d | {} | {} | {} |",
        format_pct(report.comparison.current_p_5d.baseline),
        format_pct(report.comparison.current_p_5d.candidate),
        format_signed_pct_delta(report.comparison.current_p_5d.delta)
    );
    let _ = writeln!(
        markdown,
        "| p_20d | {} | {} | {} |",
        format_pct(report.comparison.current_p_20d.baseline),
        format_pct(report.comparison.current_p_20d.candidate),
        format_signed_pct_delta(report.comparison.current_p_20d.delta)
    );
    let _ = writeln!(
        markdown,
        "| p_60d | {} | {} | {} |",
        format_pct(report.comparison.current_p_60d.baseline),
        format_pct(report.comparison.current_p_60d.candidate),
        format_signed_pct_delta(report.comparison.current_p_60d.delta)
    );
    let _ = writeln!(
        markdown,
        "| Posture | {} | {} | — |",
        posture_text(report.baseline_assessment.posture),
        posture_text(report.candidate_assessment.posture)
    );
    let _ = writeln!(
        markdown,
        "| Time bucket | {} | {} | — |",
        time_bucket_text(report.baseline_assessment.time_to_risk_bucket),
        time_bucket_text(report.candidate_assessment.time_to_risk_bucket)
    );
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Runtime Diagnostics");
    let _ = writeln!(markdown);
    render_release_runtime_review_markdown(
        &mut markdown,
        "baseline",
        &report.baseline_runtime_review,
    );
    let _ = writeln!(markdown);
    render_release_runtime_review_markdown(
        &mut markdown,
        "candidate",
        &report.candidate_runtime_review,
    );
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Backtest Guardrails");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "| Metric | Baseline | Candidate | Delta |");
    let _ = writeln!(markdown, "| --- | --- | --- | --- |");
    let _ = writeln!(
        markdown,
        "| timely_warning_rate | {} | {} | {} |",
        format_pct(report.comparison.timely_warning_rate.baseline),
        format_pct(report.comparison.timely_warning_rate.candidate),
        format_signed_pct_delta(report.comparison.timely_warning_rate.delta)
    );
    let _ = writeln!(
        markdown,
        "| strict_actionable_point_count | {} | {} | {} |",
        report.comparison.strict_actionable_point_count.baseline,
        report.comparison.strict_actionable_point_count.candidate,
        format_signed_count_delta(report.comparison.strict_actionable_point_count.delta)
    );
    let _ = writeln!(
        markdown,
        "| runtime_floor_hit_count | {} | {} | {} |",
        report.comparison.runtime_floor_hit_count.baseline,
        report.comparison.runtime_floor_hit_count.candidate,
        format_signed_count_delta(report.comparison.runtime_floor_hit_count.delta)
    );
    let _ = writeln!(
        markdown,
        "| actionable_precision | {} | {} | {} |",
        format_pct(report.comparison.actionable_precision.baseline),
        format_pct(report.comparison.actionable_precision.candidate),
        format_signed_pct_delta(report.comparison.actionable_precision.delta)
    );
    let _ = writeln!(
        markdown,
        "| longest_false_positive_episode_days | {} | {} | {} |",
        report
            .comparison
            .longest_false_positive_episode_days
            .baseline,
        report
            .comparison
            .longest_false_positive_episode_days
            .candidate,
        format_signed_count_delta(report.comparison.longest_false_positive_episode_days.delta)
    );
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Scenario-Level Backtests");
    let _ = writeln!(markdown);
    let _ = writeln!(
        markdown,
        "| Scenario | Source | Baseline L2 | Candidate L2 | Baseline L3 | Candidate L3 | Baseline FP | Candidate FP | Outcome |"
    );
    let _ = writeln!(
        markdown,
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- |"
    );
    for scenario in &report.comparison.backtest_scenarios {
        let _ = writeln!(
            markdown,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            scenario.name,
            scenario.signal_source,
            format_optional_days(scenario.baseline_lead_time_days),
            format_optional_days(scenario.candidate_lead_time_days),
            format_optional_days(scenario.baseline_actionable_lead_time_days),
            format_optional_days(scenario.candidate_actionable_lead_time_days),
            scenario.baseline_false_positive_count,
            scenario.candidate_false_positive_count,
            scenario.outcome
        );
    }
    let _ = writeln!(markdown);
    if !report.scenario_focus.is_empty() {
        let _ = writeln!(markdown, "## Focus Scenarios");
        let _ = writeln!(markdown);
        for scenario in &report.scenario_focus {
            let _ = writeln!(markdown, "### {} ({})", scenario.name, scenario.outcome);
            let _ = writeln!(markdown);
            let _ = writeln!(
                markdown,
                "- Window: {} -> {}",
                scenario.window_start, scenario.window_end
            );
            let _ = writeln!(
                markdown,
                "- Crisis window: {} -> {}",
                scenario.crisis_start, scenario.crisis_end
            );
            let _ = writeln!(
                markdown,
                "- First L2: baseline={} | candidate={}",
                format_optional_date_with_lead(
                    scenario.baseline_first_l2_date,
                    scenario.crisis_start
                ),
                format_optional_date_with_lead(
                    scenario.candidate_first_l2_date,
                    scenario.crisis_start
                )
            );
            let _ = writeln!(
                markdown,
                "- First L3: baseline={} | candidate={}",
                format_optional_date_with_lead(
                    scenario.baseline_first_l3_date,
                    scenario.crisis_start
                ),
                format_optional_date_with_lead(
                    scenario.candidate_first_l3_date,
                    scenario.crisis_start
                )
            );
            let _ = writeln!(
                markdown,
                "- First non-normal point: baseline={} | candidate={}",
                format_optional_date(scenario.baseline_first_non_normal_date),
                format_optional_date(scenario.candidate_first_non_normal_date)
            );
            let _ = writeln!(
                markdown,
                "- Pre-crisis actionable points: baseline={} | candidate={}",
                scenario.baseline_actionable_point_count, scenario.candidate_actionable_point_count
            );
            let _ = writeln!(
                markdown,
                "- Pre-crisis runtime-floor hits: baseline={} | candidate={}",
                scenario.baseline_runtime_floor_hit_point_count,
                scenario.candidate_runtime_floor_hit_point_count
            );
            let _ = writeln!(
                markdown,
                "- Pre-crisis max p_20d: baseline={} | candidate={}",
                format_optional_pct(scenario.baseline_max_p20d),
                format_optional_pct(scenario.candidate_max_p20d)
            );
            let _ = writeln!(
                markdown,
                "- Pre-crisis max p_60d: baseline={} | candidate={}",
                format_optional_pct(scenario.baseline_max_p60d),
                format_optional_pct(scenario.candidate_max_p60d)
            );
            let _ = writeln!(
                markdown,
                "- First runtime-floor hit without L3: baseline={} | candidate={}",
                format_optional_date_with_reason(
                    scenario.baseline_first_runtime_floor_hit_without_l3_date,
                    scenario
                        .baseline_first_runtime_floor_hit_without_l3_reason
                        .as_deref()
                ),
                format_optional_date_with_reason(
                    scenario.candidate_first_runtime_floor_hit_without_l3_date,
                    scenario
                        .candidate_first_runtime_floor_hit_without_l3_reason
                        .as_deref()
                )
            );
            if !scenario.runtime_block_counts.is_empty() {
                let _ = writeln!(markdown, "- Runtime block mix:");
                for block in &scenario.runtime_block_counts {
                    let _ = writeln!(
                        markdown,
                        "  - {}: baseline={} | candidate={} | delta={}",
                        block.category,
                        block.baseline_count,
                        block.candidate_count,
                        format_signed_count_delta(block.delta)
                    );
                }
            }
            let _ = writeln!(markdown);
            if scenario.interesting_points.is_empty() {
                let _ = writeln!(
                    markdown,
                    "- No loaded runtime history points matched this scenario window. Fast review history_limit may be too small for this sample."
                );
                let _ = writeln!(markdown);
                continue;
            }
            let _ = writeln!(
                markdown,
                "| Date | Base p_20d | Cand p_20d | Base p_60d | Cand p_60d | Base posture | Cand posture | Base bucket | Cand bucket | Base strict L3 | Cand strict L3 | Base runtime floor | Cand runtime floor | Base 5d hits | Cand 5d hits | Base sustained | Cand sustained | Base triggers | Cand triggers | Base block cat | Cand block cat | Base runtime block | Cand runtime block | Base diag | Cand diag |"
            );
            let _ = writeln!(
                markdown,
                "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
            );
            for point in &scenario.interesting_points {
                let _ = writeln!(
                    markdown,
                    "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                    point.as_of_date,
                    format_optional_pct(point.baseline_p20d),
                    format_optional_pct(point.candidate_p20d),
                    format_optional_pct(point.baseline_p60d),
                    format_optional_pct(point.candidate_p60d),
                    point.baseline_posture.as_deref().unwrap_or("—"),
                    point.candidate_posture.as_deref().unwrap_or("—"),
                    point.baseline_time_bucket.as_deref().unwrap_or("—"),
                    point.candidate_time_bucket.as_deref().unwrap_or("—"),
                    format_bool_flag(point.baseline_strict_review_actionable),
                    format_bool_flag(point.candidate_strict_review_actionable),
                    format_bool_flag(point.baseline_runtime_floor_hit),
                    format_bool_flag(point.candidate_runtime_floor_hit),
                    format_optional_count(point.baseline_actionable_forward_5d_hits),
                    format_optional_count(point.candidate_actionable_forward_5d_hits),
                    format_optional_bool_flag(point.baseline_actionable_sustained),
                    format_optional_bool_flag(point.candidate_actionable_sustained),
                    format_trigger_codes(&point.baseline_trigger_codes),
                    format_trigger_codes(&point.candidate_trigger_codes),
                    point
                        .baseline_runtime_actionable_block_category
                        .as_deref()
                        .unwrap_or("—"),
                    point
                        .candidate_runtime_actionable_block_category
                        .as_deref()
                        .unwrap_or("—"),
                    point
                        .baseline_runtime_actionable_block_reason
                        .as_deref()
                        .unwrap_or("—"),
                    point
                        .candidate_runtime_actionable_block_reason
                        .as_deref()
                        .unwrap_or("—"),
                    point
                        .baseline_actionable_diagnostic
                        .as_deref()
                        .unwrap_or("—"),
                    point
                        .candidate_actionable_diagnostic
                        .as_deref()
                        .unwrap_or("—")
                );
            }
            let _ = writeln!(markdown);
        }
    }
    let _ = writeln!(markdown, "## Actionability Diagnostics");
    let _ = writeln!(markdown);
    render_release_actionability_review_markdown(
        &mut markdown,
        "baseline",
        &report.baseline_actionability_review,
    );
    let _ = writeln!(markdown);
    render_release_actionability_review_markdown(
        &mut markdown,
        "candidate",
        &report.candidate_actionability_review,
    );
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Guardrail Result");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "### Runtime Guard");
    let _ = writeln!(markdown);
    if report.operational_guard_regressions.is_empty() {
        let _ = writeln!(markdown, "- No runtime guard regressions detected.");
    } else {
        for regression in &report.operational_guard_regressions {
            let _ = writeln!(markdown, "- {regression}");
        }
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "### Probability Guard");
    let _ = writeln!(markdown);
    if report.probability_guard_regressions.is_empty() {
        let _ = writeln!(
            markdown,
            "- No probability-head guard regressions detected."
        );
    } else {
        for regression in &report.probability_guard_regressions {
            let _ = writeln!(markdown, "- {regression}");
        }
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "### Actionability Guard");
    let _ = writeln!(markdown);
    if report.actionability_guard_regressions.is_empty() {
        let _ = writeln!(markdown, "- No actionability guard regressions detected.");
    } else {
        for regression in &report.actionability_guard_regressions {
            let _ = writeln!(markdown, "- {regression}");
        }
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "### Runtime Sanity Guard");
    let _ = writeln!(markdown);
    if report.runtime_sanity_regressions.is_empty() {
        let _ = writeln!(markdown, "- No runtime sanity regressions detected.");
    } else {
        for regression in &report.runtime_sanity_regressions {
            let _ = writeln!(markdown, "- {regression}");
        }
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "### Overall");
    let _ = writeln!(markdown);
    if report.overall_guard_regressions.is_empty() {
        let _ = writeln!(markdown, "- No combined guard regressions detected.");
    } else {
        for regression in &report.overall_guard_regressions {
            let _ = writeln!(markdown, "- {regression}");
        }
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Recommendation");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "{}", report.recommendation);
    markdown
}

fn render_release_actionability_review_markdown(
    markdown: &mut String,
    role: &str,
    review: &ReleaseActionabilityReview,
) {
    let _ = writeln!(markdown, "### {role} Actionability");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "- Enabled: {}", review.enabled);
    let _ = writeln!(markdown, "- Note: {}", review.note);
    if !review.enabled {
        return;
    }
    let _ = writeln!(
        markdown,
        "- Versions: model={} calib={} fusion={}",
        review.model_version.as_deref().unwrap_or("n/a"),
        review.calibration_version.as_deref().unwrap_or("n/a"),
        review.fusion_policy_version.as_deref().unwrap_or("n/a")
    );
    let _ = writeln!(markdown);
    let _ = writeln!(
        markdown,
        "| Level | Scenarios | On Time | Late Only | Missed | Primary Recall | Late Validation | Precision | Pred+ | Primary+ | Protected Hits | FP |"
    );
    let _ = writeln!(
        markdown,
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
    );
    for level in &review.levels {
        let _ = writeln!(
            markdown,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            actionability_level_text(level.level),
            level.scenario_count,
            format_optional_pct(level.on_time_rate),
            format_optional_pct(level.late_only_rate),
            format_optional_pct(level.missed_rate),
            format_optional_pct(level.primary_recall_at_threshold),
            format_optional_pct(level.late_validation_capture_rate),
            format_optional_pct(level.precision_at_threshold),
            level.predicted_positive_count,
            level.primary_positive_count,
            level.protected_hit_count,
            level.false_positive_count
        );
    }
}

fn render_release_runtime_review_markdown(
    markdown: &mut String,
    role: &str,
    diagnostics: &ReleaseRuntimeReviewDiagnostics,
) {
    let _ = writeln!(markdown, "### {role} Runtime");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "- Release: {}", diagnostics.release_id);
    let _ = writeln!(
        markdown,
        "- History points: {}",
        diagnostics.history_point_count
    );
    let _ = writeln!(markdown, "- Note: {}", diagnostics.note);
    if let Some(thresholds) = diagnostics.runtime_thresholds.as_ref() {
        let _ = writeln!(
            markdown,
            "- Thresholds: prepare_p60d={}, hedge_p20d={}, defend_p5d={}",
            format_pct(thresholds.prepare_p60d),
            format_pct(thresholds.hedge_p20d),
            format_pct(thresholds.defend_p5d),
        );
        let _ = writeln!(
            markdown,
            "- Runtime policy version: {}",
            thresholds.history_runtime_policy_version
        );
        let _ = writeln!(
            markdown,
            "- Probability floor hits: p_60d>=prepare {} / p_20d>=hedge {} / p_5d>=defend {}",
            diagnostics
                .points_at_or_above_prepare_p60d
                .unwrap_or_default(),
            diagnostics
                .points_at_or_above_hedge_p20d
                .unwrap_or_default(),
            diagnostics
                .points_at_or_above_defend_p5d
                .unwrap_or_default(),
        );
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "| Posture | Count |");
    let _ = writeln!(markdown, "| --- | --- |");
    for row in &diagnostics.posture_distribution {
        let _ = writeln!(markdown, "| {} | {} |", row.name, row.count);
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "| Time bucket | Count |");
    let _ = writeln!(markdown, "| --- | --- |");
    for row in &diagnostics.time_bucket_distribution {
        let _ = writeln!(markdown, "| {} | {} |", row.name, row.count);
    }
    if !diagnostics.posture_trigger_distribution.is_empty() {
        let _ = writeln!(markdown);
        let _ = writeln!(
            markdown,
            "| Posture | Trigger clause | Count | Share of posture |"
        );
        let _ = writeln!(markdown, "| --- | --- | --- | --- |");
        for row in &diagnostics.posture_trigger_distribution {
            let _ = writeln!(
                markdown,
                "| {} | {} | {} | {} |",
                row.posture,
                row.clause,
                row.count,
                format_pct(row.share_of_posture),
            );
        }
    }
    if !diagnostics.posture_blocker_distribution.is_empty() {
        let _ = writeln!(markdown);
        let _ = writeln!(
            markdown,
            "| Posture | Blocker clause | Count | Share of posture |"
        );
        let _ = writeln!(markdown, "| --- | --- | --- | --- |");
        for row in &diagnostics.posture_blocker_distribution {
            let _ = writeln!(
                markdown,
                "| {} | {} | {} | {} |",
                row.posture,
                row.clause,
                row.count,
                format_pct(row.share_of_posture),
            );
        }
    }
    if !diagnostics.regime_separation_summaries.is_empty() {
        let _ = writeln!(markdown);
        let _ = writeln!(
            markdown,
            "| Horizon | Early regime | Normal P | Positive-window P | Cooldown P | Early raw lift | Early calibrated lift | Positive-window lift | Cooldown lift | Gap retention | Diagnosis |"
        );
        let _ = writeln!(
            markdown,
            "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
        );
        for row in &diagnostics.regime_separation_summaries {
            let _ = writeln!(
                markdown,
                "| {}d | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                row.horizon_days,
                row.early_warning_regime,
                format_pct(row.normal_avg_probability),
                format_pct(row.positive_window_avg_probability),
                format_pct(row.post_crisis_cooldown_avg_probability),
                format_optional_multiplier(row.early_warning_raw_lift_vs_normal),
                format_optional_multiplier(row.early_warning_calibrated_lift_vs_normal),
                format_optional_multiplier(row.positive_window_calibrated_lift_vs_normal),
                format_optional_multiplier(row.post_crisis_cooldown_calibrated_lift_vs_normal),
                format_optional_ratio(row.early_warning_gap_retention),
                row.diagnosis,
            );
        }
    }
    if !diagnostics.regime_probability_summaries.is_empty() {
        let _ = writeln!(markdown);
        let _ = writeln!(
            markdown,
            "| Horizon | Regime | Rows | Share | Avg raw P | Max raw P | Avg calibrated P | Max calibrated P | Raw lift vs normal | Calibrated lift vs normal | Gap retention | Floor hits |"
        );
        let _ = writeln!(
            markdown,
            "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
        );
        for row in &diagnostics.regime_probability_summaries {
            let _ = writeln!(
                markdown,
                "| {}d | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                row.horizon_days,
                row.regime,
                row.row_count,
                format_pct(row.row_rate),
                format_pct(row.avg_raw_probability),
                format_pct(row.max_raw_probability),
                format_pct(row.avg_probability),
                format_pct(row.max_probability),
                format_optional_multiplier(row.raw_lift_vs_normal),
                format_optional_multiplier(row.calibrated_lift_vs_normal),
                format_optional_ratio(row.calibration_gap_retention),
                row.threshold_hit_count
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string()),
            );
        }
    }
}

fn read_release_manifest(path: &Path) -> anyhow::Result<ModelReleaseManifest> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read release manifest {}", path.display()))?;
    serde_json::from_str::<ModelReleaseManifest>(&raw)
        .with_context(|| format!("failed to decode release manifest {}", path.display()))
}

fn read_probability_bundle(path: &Path) -> anyhow::Result<ProbabilityBundle> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read probability bundle {}", path.display()))?;
    serde_json::from_str::<ProbabilityBundle>(&raw)
        .with_context(|| format!("failed to decode probability bundle {}", path.display()))
}

fn truncate_text(value: &str, max_len: usize) -> String {
    if value.chars().count() <= max_len {
        return value.to_string();
    }
    let prefix_len = max_len.saturating_sub(1);
    let mut truncated = value.chars().take(prefix_len).collect::<String>();
    truncated.push('…');
    truncated
}

fn render_audit_report_markdown(report: &AuditExportEnvelope) -> String {
    let rolling_audit = &report.assessment.backtest_summary.rolling_audit;
    let mut markdown = String::new();
    let _ = writeln!(markdown, "# Rolling Audit Report");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "- Exported at: {}", report.exported_at);
    let _ = writeln!(markdown, "- API base: {}", report.api_base_url);
    let _ = writeln!(markdown, "- As of: {}", report.assessment.as_of_date);
    let _ = writeln!(
        markdown,
        "- Data mode: {}",
        data_mode_text(report.assessment.runtime.data_mode)
    );
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Current Assessment");
    let _ = writeln!(markdown);
    let _ = writeln!(
        markdown,
        "- Overall score: {:.1}",
        report.assessment.scores.overall_score
    );
    let _ = writeln!(
        markdown,
        "- Posture: {}",
        posture_text(report.assessment.posture)
    );
    let _ = writeln!(
        markdown,
        "- Time bucket: {}",
        time_bucket_text(report.assessment.time_to_risk_bucket)
    );
    let _ = writeln!(
        markdown,
        "- Probability 5d / 20d / 60d: {} / {} / {}",
        format_pct(report.assessment.probabilities.p_5d),
        format_pct(report.assessment.probabilities.p_20d),
        format_pct(report.assessment.probabilities.p_60d)
    );
    let _ = writeln!(markdown, "- Summary: {}", report.assessment.summary);
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Serving Method");
    let _ = writeln!(markdown);
    let _ = writeln!(
        markdown,
        "- Release ID: {}",
        report
            .method
            .method
            .release_id
            .as_deref()
            .unwrap_or("inline")
    );
    let _ = writeln!(
        markdown,
        "- Probability mode: {}",
        report.method.method.probability_mode
    );
    let _ = writeln!(
        markdown,
        "- Release status: {}",
        report.method.method.release_status
    );
    let _ = writeln!(
        markdown,
        "- Point-in-time mode: {}",
        report.method.method.point_in_time_mode
    );
    let _ = writeln!(
        markdown,
        "- Versions: score={} prob={} calib={} feature={} label={} playbook={}",
        report.method.method.score_method_version,
        report.method.method.prob_model_version,
        report.method.method.calibration_version,
        report.method.method.feature_set_version,
        report.method.method.label_version,
        report.method.method.action_playbook_version
    );
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Rolling Audit");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "{}", rolling_audit.summary);
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "| Metric | Value |");
    let _ = writeln!(markdown, "| --- | --- |");
    let _ = writeln!(
        markdown,
        "| Actionable precision | {} |",
        format_pct(rolling_audit.actionable_precision)
    );
    let _ = writeln!(
        markdown,
        "| Actionable signal count | {} |",
        rolling_audit.actionable_signal_count
    );
    let _ = writeln!(
        markdown,
        "| Pre-crisis hit count | {} |",
        rolling_audit.pre_crisis_signal_count
    );
    let _ = writeln!(
        markdown,
        "| In-crisis signal count | {} |",
        rolling_audit.in_crisis_signal_count
    );
    let _ = writeln!(
        markdown,
        "| Protected stress count | {} |",
        rolling_audit.stress_window_signal_count
    );
    let _ = writeln!(
        markdown,
        "| Pure false-positive count | {} |",
        rolling_audit.false_positive_signal_count
    );
    let _ = writeln!(
        markdown,
        "| Pure false-positive episodes | {} |",
        rolling_audit.false_positive_episode_count
    );
    let _ = writeln!(
        markdown,
        "| Longest pure false-positive episode | {}d |",
        rolling_audit.longest_false_positive_episode_days
    );
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Largest Non-crisis Action Episodes");
    let _ = writeln!(markdown);
    let _ = writeln!(
        markdown,
        "| Classification | Window | Duration | Signals | Note |"
    );
    let _ = writeln!(markdown, "| --- | --- | --- | --- | --- |");
    for episode in &rolling_audit.classified_episodes {
        let _ = writeln!(
            markdown,
            "| {} | {} .. {} | {}d | {} | {} |",
            episode.classification,
            episode.start_date,
            episode.end_date,
            episode.duration_days,
            episode.signal_count,
            episode.note.replace('|', "/")
        );
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Scenario Backtests");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "| Scenario | Source | Crisis Window | Structural Lead | Actionable Lead | Max Score | Foldbacks | Note |");
    let _ = writeln!(
        markdown,
        "| --- | --- | --- | --- | --- | --- | --- | --- |"
    );
    for scenario in &report.backtests {
        let _ = writeln!(
            markdown,
            "| {} | {} | {} .. {} | {} | {} | {:.1} | {} | {} |",
            scenario.name,
            backtest_signal_source_text(scenario.signal_source),
            scenario.crisis_start,
            scenario.crisis_end,
            format_optional_days(scenario.lead_time_days),
            format_optional_days(scenario.actionable_lead_time_days),
            scenario.max_score,
            scenario.false_positive_count,
            scenario.note.replace('|', "/")
        );
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Protected Stress Window Catalog");
    let _ = writeln!(markdown);
    let _ = writeln!(
        markdown,
        "- Catalog: {}",
        report.method.protected_stress_window_catalog.catalog_id
    );
    let _ = writeln!(
        markdown,
        "- Source: {}",
        report.method.protected_stress_window_catalog.source
    );
    let _ = writeln!(
        markdown,
        "- Note: {}",
        report.method.protected_stress_window_catalog.note
    );
    if let Some(warning) = &report.method.protected_stress_window_catalog.warning {
        let _ = writeln!(markdown, "- Warning: {warning}");
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "| Window | Range | Note |");
    let _ = writeln!(markdown, "| --- | --- | --- |");
    for window in &report.method.protected_stress_window_catalog.windows {
        let _ = writeln!(
            markdown,
            "| {} | {} .. {} | {} |",
            window.label,
            window.start_date,
            window.end_date,
            window.note.replace('|', "/")
        );
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Method Note");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "{}", report.method.note);
    markdown
}

fn data_mode_text(mode: fc_domain::DataMode) -> &'static str {
    match mode {
        fc_domain::DataMode::Demo => "demo",
        fc_domain::DataMode::Sqlite => "sqlite",
        fc_domain::DataMode::Postgres => "postgres",
    }
}

fn posture_text(posture: fc_domain::DecisionPosture) -> &'static str {
    match posture {
        fc_domain::DecisionPosture::Normal => "normal",
        fc_domain::DecisionPosture::Prepare => "prepare",
        fc_domain::DecisionPosture::Hedge => "hedge",
        fc_domain::DecisionPosture::Defend => "defend",
    }
}

fn time_bucket_text(bucket: fc_domain::TimeToRiskBucket) -> &'static str {
    match bucket {
        fc_domain::TimeToRiskBucket::Normal => "normal",
        fc_domain::TimeToRiskBucket::Months => "months",
        fc_domain::TimeToRiskBucket::Weeks => "weeks",
        fc_domain::TimeToRiskBucket::Now => "now",
    }
}

fn backtest_signal_source_text(source: fc_domain::BacktestSignalSource) -> &'static str {
    match source {
        fc_domain::BacktestSignalSource::RealHistory => "real_history",
        fc_domain::BacktestSignalSource::FallbackTemplate => "fallback_template",
    }
}

pub(crate) fn format_pct(value: f64) -> String {
    format!("{:.1}%", value * 100.0)
}

fn format_optional_pct(value: Option<f64>) -> String {
    value.map(format_pct).unwrap_or_else(|| "—".to_string())
}

fn format_optional_date(value: Option<NaiveDate>) -> String {
    value
        .map(|date| date.to_string())
        .unwrap_or_else(|| "—".to_string())
}

fn format_optional_date_with_reason(value: Option<NaiveDate>, reason: Option<&str>) -> String {
    match (value, reason) {
        (Some(date), Some(reason)) if !reason.is_empty() => format!("{date} ({reason})"),
        (Some(date), _) => date.to_string(),
        _ => "—".to_string(),
    }
}

fn format_optional_date_with_lead(value: Option<NaiveDate>, crisis_start: NaiveDate) -> String {
    value
        .map(|date| {
            let lead_days = crisis_start.signed_duration_since(date).num_days();
            format!("{date} ({lead_days}d)")
        })
        .unwrap_or_else(|| "—".to_string())
}

fn format_bool_flag(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn format_optional_bool_flag(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "yes",
        Some(false) => "no",
        None => "—",
    }
}

fn format_optional_count(value: Option<u32>) -> String {
    value
        .map(|count| count.to_string())
        .unwrap_or_else(|| "—".to_string())
}

fn format_trigger_codes(codes: &[String]) -> String {
    if codes.is_empty() {
        "—".to_string()
    } else {
        codes.join(", ")
    }
}

fn format_optional_multiplier(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.2}x"))
        .unwrap_or_else(|| "—".to_string())
}

fn format_optional_ratio(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.2}"))
        .unwrap_or_else(|| "—".to_string())
}

fn format_signed_pct_delta(value: f64) -> String {
    format!("{:+.1}pp", value * 100.0)
}

fn format_signed_count_delta(value: i64) -> String {
    format!("{value:+}")
}

fn format_optional_days(value: Option<i64>) -> String {
    value
        .map(|days| format!("{days}d"))
        .unwrap_or_else(|| "—".to_string())
}

fn parse_date_arg(value: Option<&String>, option: &str) -> anyhow::Result<NaiveDate> {
    let value = value.with_context(|| format!("{option} requires a YYYY-MM-DD value"))?;
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .with_context(|| format!("{option} must use YYYY-MM-DD"))
}

fn parse_positive_i64(value: Option<&String>, option: &str) -> anyhow::Result<i64> {
    let value = value
        .with_context(|| format!("{option} requires a positive integer"))?
        .parse::<i64>()
        .with_context(|| format!("{option} requires a positive integer"))?;
    if value <= 0 {
        bail!("{option} requires a positive integer");
    }
    Ok(value)
}

async fn open_sqlite_store() -> anyhow::Result<SqliteStore> {
    SqliteStore::connect(sqlite_path())
        .await
        .map_err(Into::into)
}

fn sqlite_path() -> String {
    env::var("FC_SQLITE_PATH").unwrap_or_else(|_| DEFAULT_SQLITE_PATH.to_string())
}

fn raw_data_dir() -> PathBuf {
    env::var("FC_RAW_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_RAW_DATA_DIR))
}

fn write_raw_payload(
    raw_root: &Path,
    source_id: &str,
    external_code: &str,
    extension: &str,
    body: &str,
) -> anyhow::Result<PathBuf> {
    let year = Utc::now().format("%Y").to_string();
    let directory = raw_root.join(source_id).join(external_code).join(year);
    fs::create_dir_all(&directory)?;
    let path = directory.join(format!("{}.{}", simple_hash(body), extension));
    fs::write(&path, body)?;
    Ok(path)
}

fn raw_file_extension(content_type: &str) -> &'static str {
    if content_type.contains("csv") {
        "csv"
    } else if content_type.contains("json") {
        "json"
    } else if content_type.contains("xml") {
        "xml"
    } else {
        "txt"
    }
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn simple_hash(input: &str) -> String {
    let hash = input.as_bytes().iter().fold(0_u64, |acc, byte| {
        acc.wrapping_mul(31).wrapping_add(*byte as u64)
    });
    format!("{hash:x}")
}

async fn reload_api_runtime(url: &str) -> anyhow::Result<()> {
    reload_api_runtime_with_history_mode(url, ApiReloadHistoryMode::Default).await
}

async fn reload_api_runtime_with_history_mode(
    url: &str,
    history_mode: ApiReloadHistoryMode,
) -> anyhow::Result<()> {
    reload_api_runtime_with_history_options(url, history_mode, None).await
}

async fn reload_api_runtime_with_history_options(
    url: &str,
    history_mode: ApiReloadHistoryMode,
    history_limit: Option<usize>,
) -> anyhow::Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(1_200))
        .build()?;
    let request = client.post(url);
    let mut query = Vec::<(&str, String)>::new();
    if let Some(history_mode) = history_mode.as_query_value() {
        query.push(("history_mode", history_mode.to_string()));
    }
    if let Some(history_limit) = history_limit {
        query.push(("history_limit", history_limit.to_string()));
    }
    let request = if query.is_empty() {
        request
    } else {
        request.query(&query)
    };
    let response = request.send().await?;
    if !response.status().is_success() {
        bail!("reload endpoint returned {}", response.status());
    }
    Ok(())
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
        probability_calibration_selection_rows, probability_decision_threshold_selection, round3,
        scenario_aware_formal_split_bounds, scenario_count_for_index_range,
        select_actionability_calibration_strategy, select_actionability_decision_threshold,
        select_probability_calibration_strategy, select_probability_decision_threshold,
        summarize_release_runtime_regime_probabilities,
        summarize_release_runtime_regime_separation, ActionabilityLevel, AuditExportOptions,
        CrisisScenario, FeatureSnapshotBuildOptions, FormalDatasetBuildOptions,
        FormalDatasetSummaryOptions, FormalSplitLabelSupport, PipelineDatasetSource,
        PipelineTrainOptions, PointInTimeMode, PredictionSnapshotQueryOptions,
        ProbabilityCalibrationSelection, ProbabilityModelShape, ProbabilityTargetLabelMode,
        ProbabilityThresholdDiagnosticsInput, ProbabilityThresholdSelection,
        ProbabilityTrainingRegime, ProbabilityTrainingRow, RefreshLatestOptions,
        ReleaseActionabilityLevelReview, ReleaseActionabilityReview, ScenarioRowRange,
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
        assert_eq!(rows[0].runtime_block_counts.len(), 1);
        assert_eq!(rows[0].runtime_block_counts[0].category, "review_gate_gap");
        assert_eq!(rows[0].runtime_block_counts[0].baseline_count, 1);
        assert_eq!(rows[0].runtime_block_counts[0].candidate_count, 4);
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
        assert_eq!(rows[0].runtime_block_counts.len(), 1);
        assert_eq!(rows[0].runtime_block_counts[0].category, "review_gate_gap");
        assert_eq!(rows[0].runtime_block_counts[0].baseline_count, 2);
        assert_eq!(rows[0].runtime_block_counts[0].candidate_count, 2);
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

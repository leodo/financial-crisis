use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const FEATURE_OVERALL_SCORE: &str = "overall_score";
pub const FEATURE_EXTERNAL_SHOCK_SCORE: &str = "external_shock_score";
pub const FEATURE_HEURISTIC_P_5D: &str = "heuristic_p_5d";
pub const FEATURE_HEURISTIC_P_20D: &str = "heuristic_p_20d";
pub const FEATURE_HEURISTIC_P_60D: &str = "heuristic_p_60d";
pub const FEATURE_COVERAGE_SCORE: &str = "coverage_score";
pub const FEATURE_BUCKET_MONTHS_OR_HIGHER: &str = "bucket_months_or_higher";
pub const FEATURE_BUCKET_WEEKS_OR_HIGHER: &str = "bucket_weeks_or_higher";
pub const FEATURE_BUCKET_NOW: &str = "bucket_now";
pub const FEATURE_FRESHNESS_DELAYED_OR_WORSE: &str = "freshness_delayed_or_worse";
pub const FEATURE_FRESHNESS_STALE_OR_MISSING: &str = "freshness_stale_or_missing";
pub const FEATURE_STRUCTURAL_SCORE: &str = "structural_score";
pub const FEATURE_TRIGGER_SCORE: &str = "trigger_score";
pub const FEATURE_EXTERNAL_DIMENSION_SCORE: &str = "external_dimension_score";
pub const FEATURE_US_VIX_LEVEL: &str = "us_vix_level";
pub const FEATURE_US_VIX_CHANGE_5D: &str = "us_vix_change_5d";
pub const FEATURE_US_CURVE_10Y2Y_LEVEL: &str = "us_curve_10y2y_level";
pub const FEATURE_US_BAA_10Y_SPREAD_LEVEL: &str = "us_baa_10y_spread_level";
pub const FEATURE_US_FED_FUNDS_LEVEL: &str = "us_fed_funds_level";
pub const FEATURE_US_NFCI_LEVEL: &str = "us_nfci_level";
pub const FEATURE_US_STLFSI_LEVEL: &str = "us_stlfsi_level";
pub const FEATURE_US_UNEMPLOYMENT_LEVEL: &str = "us_unemployment_level";
pub const FEATURE_US_HOUSING_STARTS_LEVEL: &str = "us_housing_starts_level";
pub const FEATURE_US_USDJPY_LEVEL: &str = "us_usdjpy_level";
pub const FEATURE_US_USDJPY_CHANGE_20D: &str = "us_usdjpy_change_20d";

pub const TRANSITIONAL_PROBABILITY_BUNDLE_FEATURES: &[&str] = &[
    FEATURE_OVERALL_SCORE,
    FEATURE_EXTERNAL_SHOCK_SCORE,
    FEATURE_HEURISTIC_P_5D,
    FEATURE_HEURISTIC_P_20D,
    FEATURE_HEURISTIC_P_60D,
    FEATURE_COVERAGE_SCORE,
    FEATURE_BUCKET_MONTHS_OR_HIGHER,
    FEATURE_BUCKET_WEEKS_OR_HIGHER,
    FEATURE_BUCKET_NOW,
    FEATURE_FRESHNESS_DELAYED_OR_WORSE,
    FEATURE_FRESHNESS_STALE_OR_MISSING,
];

pub const FORMAL_PROBABILITY_BUNDLE_FEATURES: &[&str] = &[
    FEATURE_OVERALL_SCORE,
    FEATURE_STRUCTURAL_SCORE,
    FEATURE_TRIGGER_SCORE,
    FEATURE_EXTERNAL_DIMENSION_SCORE,
    FEATURE_COVERAGE_SCORE,
    FEATURE_US_VIX_LEVEL,
    FEATURE_US_VIX_CHANGE_5D,
    FEATURE_US_CURVE_10Y2Y_LEVEL,
    FEATURE_US_BAA_10Y_SPREAD_LEVEL,
    FEATURE_US_FED_FUNDS_LEVEL,
    FEATURE_US_NFCI_LEVEL,
    FEATURE_US_STLFSI_LEVEL,
    FEATURE_US_UNEMPLOYMENT_LEVEL,
    FEATURE_US_HOUSING_STARTS_LEVEL,
    FEATURE_US_USDJPY_LEVEL,
    FEATURE_US_USDJPY_CHANGE_20D,
];

pub const PROBABILITY_BUNDLE_FEATURES: &[&str] = TRANSITIONAL_PROBABILITY_BUNDLE_FEATURES;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbabilityBundle {
    pub bundle_id: String,
    pub market_scope: String,
    pub probability_mode: String,
    pub created_at: DateTime<Utc>,
    pub feature_names: Vec<String>,
    pub monotonic_min_gap_5d_to_20d: f64,
    pub monotonic_min_gap_20d_to_60d: f64,
    pub note: String,
    pub horizons: Vec<ProbabilityHorizonBundle>,
    pub evaluation: Option<ProbabilityBundleEvaluation>,
    #[serde(default)]
    pub actionability: Option<ActionabilityBundle>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbabilityHorizonBundle {
    pub horizon_days: u32,
    #[serde(default)]
    pub decision_threshold: Option<f64>,
    pub raw_model: LogisticProbabilityModel,
    pub calibration: Option<PlattCalibrationArtifact>,
    pub evaluation: HorizonEvaluationSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionabilityLevel {
    Prepare,
    Hedge,
    Defend,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionabilityBundle {
    pub model_version: String,
    pub calibration_version: String,
    pub fusion_policy_version: String,
    pub note: String,
    pub levels: Vec<ActionabilityLevelBundle>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionabilityLevelBundle {
    pub level: ActionabilityLevel,
    pub proxy_horizon_days: u32,
    pub target_label_mode: String,
    #[serde(default = "default_actionability_decision_threshold")]
    pub decision_threshold: f64,
    pub raw_model: LogisticProbabilityModel,
    pub calibration: Option<PlattCalibrationArtifact>,
    pub evaluation: HorizonEvaluationSummary,
}

fn default_actionability_decision_threshold() -> f64 {
    0.3
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbabilityFeatureStat {
    pub name: String,
    pub mean: f64,
    pub std_dev: f64,
    pub fill_value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbabilityCoefficient {
    pub name: String,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogisticProbabilityModel {
    pub intercept: f64,
    pub feature_stats: Vec<ProbabilityFeatureStat>,
    pub coefficients: Vec<ProbabilityCoefficient>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlattCalibrationArtifact {
    pub alpha: f64,
    pub beta: f64,
    pub min_input: f64,
    pub max_input: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HorizonEvaluationSummary {
    pub sample_count: u32,
    pub positive_rate: f64,
    pub brier_score: f64,
    pub log_loss: f64,
    pub ece: f64,
    pub precision_at_30pct: Option<f64>,
    pub recall_at_30pct: Option<f64>,
    #[serde(default)]
    pub regime_separation: Option<RegimeSeparationEvaluationSummary>,
    #[serde(default)]
    pub actionability: Option<ActionabilityEvaluationSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProbabilityBundleEvaluation {
    pub sample_count: u32,
    pub brier_score: f64,
    pub log_loss: f64,
    pub ece: f64,
    #[serde(default)]
    pub regime_separation_summaries: Vec<RegimeSeparationEvaluationSummary>,
    #[serde(default)]
    pub usable_early_warning_horizon_count: u32,
    #[serde(default)]
    pub insufficient_early_warning_horizon_count: u32,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegimeSeparationEvaluationSummary {
    pub horizon_days: u32,
    pub early_warning_regime: String,
    pub normal_sample_count: u32,
    #[serde(default)]
    pub pre_warning_buffer_sample_count: u32,
    #[serde(default)]
    pub positive_window_sample_count: u32,
    pub early_warning_sample_count: u32,
    pub in_crisis_sample_count: u32,
    #[serde(default)]
    pub post_crisis_cooldown_sample_count: u32,
    pub normal_avg_probability: f64,
    #[serde(default)]
    pub pre_warning_buffer_avg_probability: f64,
    #[serde(default)]
    pub positive_window_avg_probability: f64,
    pub early_warning_avg_probability: f64,
    pub in_crisis_avg_probability: f64,
    #[serde(default)]
    pub post_crisis_cooldown_avg_probability: f64,
    pub max_non_normal_avg_probability: f64,
    #[serde(default)]
    pub pre_warning_buffer_lift_vs_normal: Option<f64>,
    #[serde(default)]
    pub positive_window_lift_vs_normal: Option<f64>,
    #[serde(default)]
    pub early_warning_lift_vs_normal: Option<f64>,
    #[serde(default)]
    pub in_crisis_lift_vs_normal: Option<f64>,
    #[serde(default)]
    pub post_crisis_cooldown_lift_vs_normal: Option<f64>,
    #[serde(default)]
    pub positive_window_gap_vs_normal: Option<f64>,
    #[serde(default)]
    pub post_crisis_cooldown_gap_vs_normal: Option<f64>,
    #[serde(default)]
    pub max_non_normal_lift_vs_normal: Option<f64>,
    pub diagnosis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActionabilityEvaluationSummary {
    pub threshold: f64,
    pub predicted_positive_count: u32,
    pub actual_positive_count: u32,
    pub pre_start_positive_count: u32,
    pub post_start_positive_count: u32,
    pub unclassified_positive_count: u32,
    pub pre_start_hit_count: u32,
    pub post_start_hit_count: u32,
    pub unclassified_hit_count: u32,
    pub false_positive_count: u32,
    pub scenario_count: u32,
    pub advance_warning_scenario_count: u32,
    pub late_confirmation_scenario_count: u32,
    pub missed_scenario_count: u32,
    pub precision_at_threshold: Option<f64>,
    pub pre_start_recall_at_threshold: Option<f64>,
    pub post_start_recall_at_threshold: Option<f64>,
    pub advance_warning_rate: Option<f64>,
    pub late_confirmation_rate: Option<f64>,
    pub missed_rate: Option<f64>,
    pub note: String,
}

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

pub const PROBABILITY_BUNDLE_FEATURES: &[&str] = &[
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbabilityHorizonBundle {
    pub horizon_days: u32,
    pub raw_model: LogisticProbabilityModel,
    pub calibration: Option<PlattCalibrationArtifact>,
    pub evaluation: HorizonEvaluationSummary,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProbabilityBundleEvaluation {
    pub sample_count: u32,
    pub brier_score: f64,
    pub log_loss: f64,
    pub ece: f64,
    pub note: String,
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::features::{
    PROBABILITY_FEATURE_TRANSFORM_IDENTITY_V1, PROBABILITY_MODEL_FAMILY_LINEAR_V1,
};

fn default_probability_model_family() -> String {
    PROBABILITY_MODEL_FAMILY_LINEAR_V1.to_string()
}

fn default_probability_feature_transform() -> String {
    PROBABILITY_FEATURE_TRANSFORM_IDENTITY_V1.to_string()
}

fn default_actionability_decision_threshold() -> f64 {
    0.3
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbabilityBundle {
    pub bundle_id: String,
    pub market_scope: String,
    pub probability_mode: String,
    #[serde(default = "default_probability_model_family")]
    pub model_family: String,
    #[serde(default = "default_probability_feature_transform")]
    pub feature_transform: String,
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
    #[serde(default)]
    pub threshold_diagnostics: Option<ProbabilityThresholdDiagnostics>,
    pub raw_model: LogisticProbabilityModel,
    pub calibration: Option<PlattCalibrationArtifact>,
    pub evaluation: HorizonEvaluationSummary,
    #[serde(default)]
    pub family_overlays: Vec<ProbabilityFamilyOverlayBundle>,
    #[serde(default)]
    pub family_overlay_audits: Vec<ProbabilityFamilyOverlayAudit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbabilityFamilyOverlayBundle {
    pub family_id: String,
    pub gate_feature: String,
    pub gate_threshold: f64,
    pub gate_slope: f64,
    pub blend_weight: f64,
    pub raw_model: LogisticProbabilityModel,
    #[serde(default)]
    pub calibration: Option<PlattCalibrationArtifact>,
    #[serde(default)]
    pub decision_threshold: Option<f64>,
    #[serde(default)]
    pub evaluation: Option<HorizonEvaluationSummary>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbabilityHorizonScore {
    pub raw_probability: f64,
    pub calibrated_probability: f64,
    pub final_probability: f64,
    pub overlay_contributions: Vec<ProbabilityOverlayContribution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbabilityOverlayContribution {
    pub family_id: String,
    pub gate_feature: String,
    pub gate_value: f64,
    pub gate: f64,
    pub blend: f64,
    pub overlay_probability: f64,
    pub contribution: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogisticProbabilityModelScoreDiagnostics {
    pub intercept: f64,
    pub linear_score: f64,
    pub probability: f64,
    pub feature_contributions: Vec<LogisticProbabilityFeatureContribution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogisticProbabilityFeatureContribution {
    pub name: String,
    pub raw_value: f64,
    pub normalized_value: f64,
    pub weight: f64,
    pub contribution: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbabilityFamilyOverlayAudit {
    pub family_id: String,
    pub gate_feature: String,
    pub gate_active_threshold: f64,
    pub scenario_count: u32,
    pub train_row_count: u32,
    pub calibration_row_count: u32,
    pub evaluation_row_count: u32,
    pub train_gate_active_row_count: u32,
    pub calibration_gate_active_row_count: u32,
    pub evaluation_gate_active_row_count: u32,
    pub positive_label_count: u32,
    pub early_warning_row_count: u32,
    pub protected_action_window_count: u32,
    pub avg_gate_value: f64,
    pub max_gate_value: f64,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbabilityThresholdDiagnostics {
    pub label_mode: String,
    pub early_warning_regime: String,
    pub full_calibration_row_count: usize,
    pub eligible_row_count: usize,
    pub eligible_positive_count: usize,
    pub eligible_negative_count: usize,
    pub used_full_split_fallback: bool,
    pub selected_row_count: usize,
    pub selected_positive_count: usize,
    pub selected_negative_count: usize,
    #[serde(default)]
    pub selected_used_full_split_fallback: bool,
    pub base_threshold: f64,
    pub final_threshold: f64,
    pub repair_applied: bool,
    pub repair_eligible: bool,
    pub repair_reason: String,
    pub early_warning_probability_cap: Option<f64>,
    pub prediction_ceiling: Option<u32>,
    pub relaxed_prediction_ceiling: Option<u32>,
    pub base_summary: ProbabilityThresholdDecisionSummary,
    pub final_summary: ProbabilityThresholdDecisionSummary,
    #[serde(default)]
    pub calibration_regime_evidence: Vec<ProbabilityCalibrationRegimeEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbabilityCalibrationRegimeEvidence {
    pub regime: String,
    pub full_row_count: u32,
    pub full_row_rate: f64,
    pub calibration_eligible_row_count: u32,
    pub calibration_eligible_row_rate: f64,
    pub calibration_used_row_count: u32,
    pub calibration_used_row_rate: f64,
    pub threshold_selected_row_count: u32,
    pub threshold_selected_row_rate: f64,
    pub positive_label_count: u32,
    pub positive_label_rate: f64,
    pub avg_hard_label: f64,
    pub avg_training_target: f64,
    pub objective_weight_sum: f64,
    pub avg_objective_weight: f64,
    pub protected_action_window_count: u32,
    pub protected_action_window_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbabilityThresholdDecisionSummary {
    pub predicted_positive_count: u32,
    pub true_positive_count: u32,
    pub precision: f64,
    pub recall: f64,
    pub early_warning_row_count: u32,
    pub early_warning_hit_count: u32,
    pub early_warning_hit_rate: f64,
    pub normal_row_count: u32,
    pub normal_hit_count: u32,
    pub normal_hit_rate: f64,
    pub positive_window_row_count: u32,
    pub positive_window_hit_count: u32,
    pub positive_window_hit_rate: f64,
    pub in_crisis_row_count: u32,
    pub in_crisis_hit_count: u32,
    pub in_crisis_hit_rate: f64,
    pub cooldown_row_count: u32,
    pub cooldown_hit_count: u32,
    pub cooldown_hit_rate: f64,
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
    #[serde(default = "default_probability_feature_transform")]
    pub feature_transform: String,
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

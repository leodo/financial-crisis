use std::collections::BTreeMap;

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
pub const PROBABILITY_MODEL_FAMILY_LINEAR_V1: &str = "linear_v1";
pub const PROBABILITY_MODEL_FAMILY_INTERACTION_TAIL_V1: &str = "interaction_tail_v1";
pub const PROBABILITY_MODEL_FAMILY_FAMILY_CONDITIONAL_V1: &str = "family_conditional_v1";
pub const PROBABILITY_FEATURE_TRANSFORM_IDENTITY_V1: &str = "identity_v1";
pub const PROBABILITY_FEATURE_TRANSFORM_INTERACTION_TAIL_V1: &str = "interaction_tail_v1";
pub const PROBABILITY_FEATURE_TRANSFORM_FAMILY_CONDITIONAL_V1: &str = "family_conditional_v1";

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
pub const INTERACTION_TAIL_DERIVED_FEATURES: &[&str] = &[
    "interaction__overall_score__us_vix_level",
    "interaction__structural_score__trigger_score",
    "interaction__trigger_score__us_vix_level",
    "interaction__trigger_score__us_usdjpy_change_20d",
    "interaction__external_dimension_score__us_usdjpy_level",
    "interaction__us_curve_10y2y_level__us_fed_funds_level",
    "interaction__us_nfci_level__us_stlfsi_level",
    "interaction__us_baa_10y_spread_level__us_vix_level",
    "tail_pos__us_vix_level__24",
    "tail_pos__us_vix_level__32",
    "tail_pos__us_baa_10y_spread_level__2",
    "tail_pos__us_stlfsi_level__1",
    "tail_pos__us_usdjpy_level__145",
    "tail_abs_pos__us_usdjpy_change_20d__4",
    "tail_neg__us_curve_10y2y_level__0",
    "tail_pos__overall_score__55",
    "tail_pos__structural_score__52",
    "tail_pos__trigger_score__50",
    "tail_pos__external_dimension_score__50",
];

pub const FAMILY_CONDITIONAL_DERIVED_FEATURES: &[&str] = &[
    "family_proxy__systemic_credit",
    "family_proxy__mixed_systemic",
    "family_proxy__rate_shock",
    "family_proxy__acute_liquidity",
    "family_proxy__jpy_carry",
    "family_context__systemic_credit__structural_score",
    "family_context__mixed_systemic__trigger_score",
    "family_context__rate_shock__external_dimension_score",
    "family_context__acute_liquidity__trigger_score",
    "family_context__jpy_carry__external_dimension_score",
];

fn default_probability_model_family() -> String {
    PROBABILITY_MODEL_FAMILY_LINEAR_V1.to_string()
}

fn default_probability_feature_transform() -> String {
    PROBABILITY_FEATURE_TRANSFORM_IDENTITY_V1.to_string()
}

pub fn probability_feature_names_for_transform(
    base_feature_names: &[String],
    feature_transform: &str,
) -> Vec<String> {
    let mut names = base_feature_names.to_vec();
    if feature_transform == PROBABILITY_FEATURE_TRANSFORM_INTERACTION_TAIL_V1
        || feature_transform == PROBABILITY_FEATURE_TRANSFORM_FAMILY_CONDITIONAL_V1
    {
        for feature_name in INTERACTION_TAIL_DERIVED_FEATURES {
            if !names.iter().any(|existing| existing == feature_name) {
                names.push((*feature_name).to_string());
            }
        }
    }
    if feature_transform == PROBABILITY_FEATURE_TRANSFORM_FAMILY_CONDITIONAL_V1 {
        for feature_name in FAMILY_CONDITIONAL_DERIVED_FEATURES {
            if !names.iter().any(|existing| existing == feature_name) {
                names.push((*feature_name).to_string());
            }
        }
    }
    names
}

pub fn resolve_probability_feature_value(
    feature_name: &str,
    features: &BTreeMap<String, f64>,
) -> Option<f64> {
    if let Some(value) = features.get(feature_name) {
        return Some(*value);
    }

    let parts = feature_name.split("__").collect::<Vec<_>>();
    match parts.as_slice() {
        ["interaction", left, right] => Some(
            resolve_probability_feature_value(left, features)?
                * resolve_probability_feature_value(right, features)?,
        ),
        ["tail_pos", base, threshold] => Some(
            (resolve_probability_feature_value(base, features)? - threshold.parse::<f64>().ok()?)
                .max(0.0),
        ),
        ["tail_neg", base, threshold] => Some(
            (threshold.parse::<f64>().ok()? - resolve_probability_feature_value(base, features)?)
                .max(0.0),
        ),
        ["tail_abs_pos", base, threshold] => Some(
            (resolve_probability_feature_value(base, features)?.abs()
                - threshold.parse::<f64>().ok()?)
            .max(0.0),
        ),
        ["family_proxy", family] => resolve_family_proxy_value(family, features),
        ["family_context", family, base] => Some(
            resolve_family_proxy_value(family, features)?
                * resolve_probability_feature_value(base, features)?,
        ),
        _ => None,
    }
}

fn resolve_family_proxy_value(family: &str, features: &BTreeMap<String, f64>) -> Option<f64> {
    match family {
        "systemic_credit" => Some(
            0.35 * scaled_tail_pos(features, FEATURE_US_BAA_10Y_SPREAD_LEVEL, 2.0, 3.0)?
                + 0.25 * scaled_tail_pos(features, FEATURE_US_STLFSI_LEVEL, 1.0, 3.0)?
                + 0.20 * scaled_tail_pos(features, FEATURE_US_NFCI_LEVEL, 0.25, 1.5)?
                + 0.20 * scaled_tail_pos(features, FEATURE_STRUCTURAL_SCORE, 52.0, 28.0)?,
        ),
        "mixed_systemic" => Some(
            0.30 * scaled_tail_pos(features, FEATURE_OVERALL_SCORE, 55.0, 35.0)?
                + 0.30 * scaled_tail_pos(features, FEATURE_TRIGGER_SCORE, 50.0, 35.0)?
                + 0.20 * scaled_tail_pos(features, FEATURE_EXTERNAL_DIMENSION_SCORE, 50.0, 35.0)?
                + 0.20 * scaled_tail_pos(features, FEATURE_US_VIX_LEVEL, 24.0, 20.0)?,
        ),
        "rate_shock" => Some(
            0.35 * scaled_tail_pos(features, FEATURE_US_FED_FUNDS_LEVEL, 4.0, 3.0)?
                + 0.35 * scaled_tail_neg(features, FEATURE_US_CURVE_10Y2Y_LEVEL, 0.0, 2.0)?
                + 0.15 * scaled_tail_pos(features, FEATURE_EXTERNAL_DIMENSION_SCORE, 50.0, 35.0)?
                + 0.15 * scaled_tail_abs(features, FEATURE_US_USDJPY_CHANGE_20D, 4.0, 8.0)?,
        ),
        "acute_liquidity" => Some(
            0.55 * scaled_tail_pos(features, FEATURE_US_VIX_LEVEL, 32.0, 20.0)?
                + 0.45 * scaled_tail_pos(features, FEATURE_US_STLFSI_LEVEL, 1.0, 3.0)?,
        ),
        "jpy_carry" => Some(
            0.35 * scaled_tail_pos(features, FEATURE_US_USDJPY_LEVEL, 145.0, 20.0)?
                + 0.35 * scaled_tail_abs(features, FEATURE_US_USDJPY_CHANGE_20D, 4.0, 8.0)?
                + 0.15 * scaled_tail_pos(features, FEATURE_US_FED_FUNDS_LEVEL, 4.0, 3.0)?
                + 0.15 * scaled_tail_pos(features, FEATURE_EXTERNAL_DIMENSION_SCORE, 50.0, 35.0)?,
        ),
        _ => None,
    }
}

fn scaled_tail_pos(
    features: &BTreeMap<String, f64>,
    feature_name: &str,
    threshold: f64,
    scale: f64,
) -> Option<f64> {
    Some(
        ((resolve_probability_feature_value(feature_name, features)? - threshold)
            / scale.max(1e-6))
        .clamp(0.0, 1.0),
    )
}

fn scaled_tail_neg(
    features: &BTreeMap<String, f64>,
    feature_name: &str,
    threshold: f64,
    scale: f64,
) -> Option<f64> {
    Some(
        ((threshold - resolve_probability_feature_value(feature_name, features)?)
            / scale.max(1e-6))
        .clamp(0.0, 1.0),
    )
}

fn scaled_tail_abs(
    features: &BTreeMap<String, f64>,
    feature_name: &str,
    threshold: f64,
    scale: f64,
) -> Option<f64> {
    Some(
        ((resolve_probability_feature_value(feature_name, features)?.abs() - threshold)
            / scale.max(1e-6))
        .clamp(0.0, 1.0),
    )
}

pub fn score_logistic_probability_model(
    model: &LogisticProbabilityModel,
    features: &BTreeMap<String, f64>,
) -> f64 {
    let mut linear = model.intercept;
    for coefficient in &model.coefficients {
        let stat = model
            .feature_stats
            .iter()
            .find(|stat| stat.name == coefficient.name);
        let raw_value = resolve_probability_feature_value(&coefficient.name, features)
            .or_else(|| stat.map(|stat| stat.fill_value))
            .unwrap_or(0.0);
        let normalized = stat.map_or(raw_value, |stat| {
            let std_dev = if stat.std_dev.abs() < 1e-9 {
                1.0
            } else {
                stat.std_dev
            };
            (raw_value - stat.mean) / std_dev
        });
        linear += normalized * coefficient.weight;
    }
    probability_sigmoid(linear)
}

pub fn apply_platt_probability_calibration(
    raw_probability: f64,
    calibration: &PlattCalibrationArtifact,
) -> f64 {
    let clipped = raw_probability.clamp(calibration.min_input, calibration.max_input);
    probability_sigmoid(calibration.alpha * clipped + calibration.beta)
}

fn probability_sigmoid(value: f64) -> f64 {
    1.0 / (1.0 + (-value).exp())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interaction_tail_transform_appends_derived_features() {
        let base = FORMAL_PROBABILITY_BUNDLE_FEATURES
            .iter()
            .map(|feature| (*feature).to_string())
            .collect::<Vec<_>>();
        let expanded = probability_feature_names_for_transform(
            &base,
            PROBABILITY_FEATURE_TRANSFORM_INTERACTION_TAIL_V1,
        );

        assert!(expanded.len() > base.len());
        assert!(expanded.contains(&"interaction__overall_score__us_vix_level".to_string()));
        assert!(expanded.contains(&"tail_neg__us_curve_10y2y_level__0".to_string()));
    }

    #[test]
    fn family_conditional_transform_appends_family_features() {
        let base = FORMAL_PROBABILITY_BUNDLE_FEATURES
            .iter()
            .map(|feature| (*feature).to_string())
            .collect::<Vec<_>>();
        let expanded = probability_feature_names_for_transform(
            &base,
            PROBABILITY_FEATURE_TRANSFORM_FAMILY_CONDITIONAL_V1,
        );

        assert!(expanded.len() > base.len() + INTERACTION_TAIL_DERIVED_FEATURES.len());
        assert!(expanded.contains(&"interaction__overall_score__us_vix_level".to_string()));
        assert!(expanded.contains(&"family_proxy__systemic_credit".to_string()));
        assert!(
            expanded.contains(&"family_context__jpy_carry__external_dimension_score".to_string())
        );
    }

    #[test]
    fn derived_feature_resolver_handles_interactions_and_tail_features() {
        let mut features = BTreeMap::new();
        features.insert(FEATURE_OVERALL_SCORE.to_string(), 60.0);
        features.insert(FEATURE_US_VIX_LEVEL.to_string(), 28.0);
        features.insert(FEATURE_US_CURVE_10Y2Y_LEVEL.to_string(), -0.5);
        features.insert(FEATURE_US_USDJPY_CHANGE_20D.to_string(), -6.0);

        assert_eq!(
            resolve_probability_feature_value(
                "interaction__overall_score__us_vix_level",
                &features
            ),
            Some(1680.0)
        );
        assert_eq!(
            resolve_probability_feature_value("tail_pos__us_vix_level__24", &features),
            Some(4.0)
        );
        assert_eq!(
            resolve_probability_feature_value("tail_neg__us_curve_10y2y_level__0", &features),
            Some(0.5)
        );
        assert_eq!(
            resolve_probability_feature_value("tail_abs_pos__us_usdjpy_change_20d__4", &features),
            Some(2.0)
        );
    }

    #[test]
    fn derived_feature_resolver_handles_family_conditional_features() {
        let mut features = BTreeMap::new();
        features.insert(FEATURE_STRUCTURAL_SCORE.to_string(), 80.0);
        features.insert(FEATURE_TRIGGER_SCORE.to_string(), 70.0);
        features.insert(FEATURE_EXTERNAL_DIMENSION_SCORE.to_string(), 65.0);
        features.insert(FEATURE_US_VIX_LEVEL.to_string(), 44.0);
        features.insert(FEATURE_US_BAA_10Y_SPREAD_LEVEL.to_string(), 5.0);
        features.insert(FEATURE_US_FED_FUNDS_LEVEL.to_string(), 5.5);
        features.insert(FEATURE_US_NFCI_LEVEL.to_string(), 1.0);
        features.insert(FEATURE_US_STLFSI_LEVEL.to_string(), 2.5);
        features.insert(FEATURE_US_CURVE_10Y2Y_LEVEL.to_string(), -1.0);
        features.insert(FEATURE_US_USDJPY_LEVEL.to_string(), 160.0);
        features.insert(FEATURE_US_USDJPY_CHANGE_20D.to_string(), -8.0);

        let systemic =
            resolve_probability_feature_value("family_proxy__systemic_credit", &features).unwrap();
        let carry =
            resolve_probability_feature_value("family_proxy__jpy_carry", &features).unwrap();
        let carry_context = resolve_probability_feature_value(
            "family_context__jpy_carry__external_dimension_score",
            &features,
        )
        .unwrap();

        assert!(systemic > 0.70 && systemic <= 1.0);
        assert!(carry > 0.55 && carry <= 1.0);
        assert!((carry_context - carry * 65.0).abs() < 1e-9);
    }

    #[test]
    fn shared_probability_scorer_uses_stats_and_fill_values() {
        let model = LogisticProbabilityModel {
            intercept: 0.5,
            feature_transform: PROBABILITY_FEATURE_TRANSFORM_IDENTITY_V1.to_string(),
            feature_stats: vec![
                ProbabilityFeatureStat {
                    name: FEATURE_OVERALL_SCORE.to_string(),
                    mean: 50.0,
                    std_dev: 10.0,
                    fill_value: 42.0,
                },
                ProbabilityFeatureStat {
                    name: FEATURE_US_VIX_LEVEL.to_string(),
                    mean: 20.0,
                    std_dev: 5.0,
                    fill_value: 18.0,
                },
            ],
            coefficients: vec![
                ProbabilityCoefficient {
                    name: FEATURE_OVERALL_SCORE.to_string(),
                    weight: 0.4,
                },
                ProbabilityCoefficient {
                    name: FEATURE_US_VIX_LEVEL.to_string(),
                    weight: 0.2,
                },
            ],
        };
        let mut features = BTreeMap::new();
        features.insert(FEATURE_OVERALL_SCORE.to_string(), 60.0);

        let probability = score_logistic_probability_model(&model, &features);

        assert!(probability > 0.69 && probability < 0.70);
    }

    #[test]
    fn shared_platt_calibration_clips_to_input_bounds() {
        let calibration = PlattCalibrationArtifact {
            alpha: 2.0,
            beta: -1.0,
            min_input: 0.2,
            max_input: 0.8,
        };

        let below = apply_platt_probability_calibration(0.05, &calibration);
        let at_min = apply_platt_probability_calibration(0.2, &calibration);
        let above = apply_platt_probability_calibration(0.95, &calibration);
        let at_max = apply_platt_probability_calibration(0.8, &calibration);

        assert!((below - at_min).abs() < 1e-12);
        assert!((above - at_max).abs() < 1e-12);
    }
}

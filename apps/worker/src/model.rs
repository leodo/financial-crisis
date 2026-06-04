use crate::{
    resolve_probability_feature_value, HorizonEvaluationSummary, LogisticProbabilityModel,
    PlattCalibrationArtifact, ProbabilityCoefficient, ProbabilityFeatureStat,
    ProbabilityTargetLabelMode, ProbabilityTrainingRegime, ProbabilityTrainingRow,
    PROBABILITY_FEATURE_TRANSFORM_IDENTITY_V1, PROBABILITY_FEATURE_TRANSFORM_INTERACTION_TAIL_V1,
};

#[derive(Debug, Clone)]
pub(crate) struct RegimePairwiseTarget {
    left_centroid: Vec<f64>,
    right_centroid: Vec<f64>,
    margin: f64,
    weight: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExpectedCoefficientSign {
    Positive,
    Negative,
}

pub(crate) fn fit_logistic_model(
    rows: &[ProbabilityTrainingRow],
    feature_names: &[String],
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) -> LogisticProbabilityModel {
    let uses_interaction_tail = feature_names.iter().any(|feature_name| {
        feature_name.contains("interaction__") || feature_name.contains("tail_")
    });
    let feature_stats = feature_names
        .iter()
        .map(|feature| build_feature_stat(rows, feature))
        .collect::<Vec<_>>();
    let regime_pairwise_targets = forward_crisis_regime_pairwise_targets(
        rows,
        &feature_stats,
        horizon_days,
        label_mode,
        uses_interaction_tail,
    );
    let positive_class_weight = horizon_positive_class_weight(rows, horizon_days, label_mode);
    let mut intercept = initial_intercept(rows, horizon_days, positive_class_weight, label_mode);
    let mut weights = vec![0.0; feature_names.len()];
    let learning_rate = 0.25;
    let l2 = 0.01;
    let sample_weight_sum = rows
        .iter()
        .map(|row| logistic_sample_weight(row, horizon_days, positive_class_weight, label_mode))
        .sum::<f64>()
        .max(1.0);

    for _ in 0..600 {
        let mut intercept_gradient = 0.0;
        let mut weight_gradients = vec![0.0; weights.len()];
        for row in rows {
            let normalized = normalized_features(row, &feature_stats);
            let prediction = sigmoid(intercept + dot(&weights, &normalized));
            let label = probability_training_target_label(row, horizon_days, label_mode);
            let sample_weight =
                logistic_sample_weight(row, horizon_days, positive_class_weight, label_mode);
            let error = (prediction - label) * sample_weight;
            intercept_gradient += error;
            for (index, value) in normalized.iter().enumerate() {
                weight_gradients[index] += error * value;
            }
        }
        apply_forward_crisis_sign_gradient(
            &mut weight_gradients,
            &weights,
            feature_names,
            sample_weight_sum,
            horizon_days,
            label_mode,
        );
        apply_forward_crisis_coefficient_bound_gradient(
            &mut weight_gradients,
            &weights,
            feature_names,
            sample_weight_sum,
            horizon_days,
            label_mode,
        );
        apply_regime_pairwise_gradient(
            &mut weight_gradients,
            &weights,
            &regime_pairwise_targets,
            sample_weight_sum,
            horizon_days,
            uses_interaction_tail,
        );
        intercept -= learning_rate * intercept_gradient / sample_weight_sum;
        for (index, weight) in weights.iter_mut().enumerate() {
            *weight -=
                learning_rate * ((weight_gradients[index] / sample_weight_sum) + l2 * *weight);
        }
        project_forward_crisis_sign_constraints(
            &mut weights,
            feature_names,
            horizon_days,
            label_mode,
        );
    }

    LogisticProbabilityModel {
        intercept,
        feature_transform: if uses_interaction_tail {
            PROBABILITY_FEATURE_TRANSFORM_INTERACTION_TAIL_V1.to_string()
        } else {
            PROBABILITY_FEATURE_TRANSFORM_IDENTITY_V1.to_string()
        },
        feature_stats: feature_stats.clone(),
        coefficients: feature_names
            .iter()
            .zip(weights)
            .map(|(feature, weight)| ProbabilityCoefficient {
                name: feature.clone(),
                weight,
            })
            .collect(),
    }
}

fn forward_crisis_expected_coefficient_sign(
    feature_name: &str,
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) -> Option<ExpectedCoefficientSign> {
    if label_mode != ProbabilityTargetLabelMode::ForwardCrisis || horizon_days < 20 {
        return None;
    }

    if feature_name.starts_with("family_proxy__") || feature_name.starts_with("family_context__") {
        return Some(ExpectedCoefficientSign::Positive);
    }

    if horizon_days == 20 {
        // The curve inversion tail is not a simple monotonic risk head on 20d.
        // Once inversion is already entrenched, forcing this tail nonnegative
        // removes a stabilizing offset and re-opens broad normal-window noise.
        if feature_name == "tail_neg__us_curve_10y2y_level__0" {
            return None;
        }

        if let Some(base_feature_name) = derived_tail_base_feature_name(feature_name, "tail_pos__")
        {
            if matches!(
                forward_crisis_expected_base_coefficient_sign(base_feature_name),
                Some(ExpectedCoefficientSign::Positive)
            ) {
                return Some(ExpectedCoefficientSign::Positive);
            }
        }

        if let Some(base_feature_name) = derived_tail_base_feature_name(feature_name, "tail_neg__")
        {
            if matches!(
                forward_crisis_expected_base_coefficient_sign(base_feature_name),
                Some(ExpectedCoefficientSign::Negative)
            ) {
                return Some(ExpectedCoefficientSign::Positive);
            }
        }
    }

    forward_crisis_expected_base_coefficient_sign(feature_name)
}

fn forward_crisis_expected_base_coefficient_sign(
    feature_name: &str,
) -> Option<ExpectedCoefficientSign> {
    match feature_name {
        "overall_score"
        | "structural_score"
        | "trigger_score"
        | "external_dimension_score"
        | "interaction__overall_score__us_vix_level"
        | "interaction__structural_score__trigger_score"
        | "interaction__trigger_score__us_vix_level"
        | "interaction__external_dimension_score__us_usdjpy_level"
        | "interaction__us_nfci_level__us_stlfsi_level"
        | "interaction__us_baa_10y_spread_level__us_vix_level"
        | "us_vix_level"
        | "us_vix_change_5d"
        | "us_baa_10y_spread_level"
        | "us_fed_funds_level"
        | "us_nfci_level"
        | "us_stlfsi_level"
        | "us_unemployment_level" => Some(ExpectedCoefficientSign::Positive),
        "us_curve_10y2y_level" | "us_housing_starts_level" => {
            Some(ExpectedCoefficientSign::Negative)
        }
        _ => None,
    }
}

fn derived_tail_base_feature_name<'a>(feature_name: &'a str, prefix: &str) -> Option<&'a str> {
    let rest = feature_name.strip_prefix(prefix)?;
    let (base_feature_name, _) = rest.rsplit_once("__")?;
    Some(base_feature_name)
}

fn forward_crisis_sign_constraint_strength(
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) -> f64 {
    if label_mode != ProbabilityTargetLabelMode::ForwardCrisis {
        return 0.0;
    }
    match horizon_days {
        20 => 0.55,
        60 => 0.70,
        _ => 0.0,
    }
}

#[derive(Debug, Clone, Copy)]
struct CoefficientBounds {
    min: Option<f64>,
    max: Option<f64>,
}

fn forward_crisis_coefficient_bounds(
    feature_name: &str,
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
    uses_family_context_features: bool,
) -> Option<CoefficientBounds> {
    if label_mode != ProbabilityTargetLabelMode::ForwardCrisis {
        return None;
    }

    match (horizon_days, feature_name) {
        // The 2026-06-04 joint audit showed that letting this tail drift negative
        // directly erodes regional-banks 20d continuity. Keep it nonnegative on
        // 20d and force any future refinement into more explicit protected-context
        // semantics instead of blunt raw suppression.
        (20, "tail_neg__us_curve_10y2y_level__0") => Some(CoefficientBounds {
            min: Some(0.0),
            max: Some(0.0),
        }),
        // rate_shock family features should stay as auxiliary context on 20d:
        // they helped recover regional-bank timing, but without a cap they also
        // over-lift non-crisis 2023-02 / 2023-07 windows.
        (20, "family_context__rate_shock__external_dimension_score") => Some(CoefficientBounds {
            min: Some(0.0),
            max: Some(0.12),
        }),
        (20, "family_proxy__rate_shock") => Some(CoefficientBounds {
            min: Some(0.0),
            max: Some(0.06),
        }),
        // jpy_carry is still proxy-only with no labeled primary scenarios in the current
        // formal dataset. Keep it as auxiliary context rather than a broad 20d driver.
        (20, "family_context__jpy_carry__external_dimension_score") => Some(CoefficientBounds {
            min: Some(0.0),
            max: Some(0.10),
        }),
        (20, "family_proxy__jpy_carry") => Some(CoefficientBounds {
            min: Some(0.0),
            max: Some(0.06),
        }),
        // The best current family-hybrid candidate keeps USDJPY level as a real
        // positive driver. The failed 064930 / 064040 branch only looked cleaner
        // because it pushed the base level down toward 0.22 while simultaneously
        // amplifying the external-dimension interaction, which then crushed true
        // positive continuity in regional-banks. Keep the base level in a narrower
        // positive band and prevent the interaction from expanding into a harsher
        // replacement for that base semantics.
        (20, "us_usdjpy_level") if uses_family_context_features => Some(CoefficientBounds {
            min: Some(0.30),
            max: Some(0.40),
        }),
        (20, "interaction__external_dimension_score__us_usdjpy_level")
            if uses_family_context_features =>
        {
            Some(CoefficientBounds {
                min: Some(0.0),
                max: Some(0.58),
            })
        }
        (20, "us_curve_10y2y_level") if uses_family_context_features => Some(CoefficientBounds {
            min: Some(-0.72),
            max: None,
        }),
        (20, "interaction__us_curve_10y2y_level__us_fed_funds_level")
            if uses_family_context_features =>
        {
            Some(CoefficientBounds {
                min: Some(0.0),
                max: Some(0.46),
            })
        }
        _ => None,
    }
}

fn forward_crisis_coefficient_bound_strength(
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) -> f64 {
    if label_mode != ProbabilityTargetLabelMode::ForwardCrisis {
        return 0.0;
    }
    match horizon_days {
        20 => 0.40,
        _ => 0.0,
    }
}

pub(crate) fn apply_forward_crisis_sign_gradient(
    weight_gradients: &mut [f64],
    weights: &[f64],
    feature_names: &[String],
    sample_weight_sum: f64,
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) {
    let strength = forward_crisis_sign_constraint_strength(horizon_days, label_mode);
    if strength <= 0.0 {
        return;
    }

    for ((gradient, weight), feature_name) in weight_gradients
        .iter_mut()
        .zip(weights.iter())
        .zip(feature_names.iter())
    {
        let Some(expected_sign) =
            forward_crisis_expected_coefficient_sign(feature_name, horizon_days, label_mode)
        else {
            continue;
        };
        let violates_sign = match expected_sign {
            ExpectedCoefficientSign::Positive => *weight < 0.0,
            ExpectedCoefficientSign::Negative => *weight > 0.0,
        };
        if violates_sign {
            *gradient += *weight * sample_weight_sum * strength;
        }
    }
}

pub(crate) fn apply_forward_crisis_coefficient_bound_gradient(
    weight_gradients: &mut [f64],
    weights: &[f64],
    feature_names: &[String],
    sample_weight_sum: f64,
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) {
    let strength = forward_crisis_coefficient_bound_strength(horizon_days, label_mode);
    if strength <= 0.0 {
        return;
    }
    let uses_family_context_features = feature_names.iter().any(|feature_name| {
        feature_name.starts_with("family_proxy__") || feature_name.starts_with("family_context__")
    });

    for ((gradient, weight), feature_name) in weight_gradients
        .iter_mut()
        .zip(weights.iter())
        .zip(feature_names.iter())
    {
        let Some(bounds) = forward_crisis_coefficient_bounds(
            feature_name,
            horizon_days,
            label_mode,
            uses_family_context_features,
        ) else {
            continue;
        };

        if let Some(min) = bounds.min {
            if *weight < min {
                *gradient += (*weight - min) * sample_weight_sum * strength;
            }
        }
        if let Some(max) = bounds.max {
            if *weight > max {
                *gradient += (*weight - max) * sample_weight_sum * strength;
            }
        }
    }
}

pub(crate) fn project_forward_crisis_sign_constraints(
    weights: &mut [f64],
    feature_names: &[String],
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) {
    if forward_crisis_sign_constraint_strength(horizon_days, label_mode) <= 0.0 {
        return;
    }
    let uses_family_context_features = feature_names.iter().any(|feature_name| {
        feature_name.starts_with("family_proxy__") || feature_name.starts_with("family_context__")
    });

    for (weight, feature_name) in weights.iter_mut().zip(feature_names.iter()) {
        if let Some(expected_sign) =
            forward_crisis_expected_coefficient_sign(feature_name, horizon_days, label_mode)
        {
            match expected_sign {
                ExpectedCoefficientSign::Positive if *weight < 0.0 => *weight = 0.0,
                ExpectedCoefficientSign::Negative if *weight > 0.0 => *weight = 0.0,
                _ => {}
            }
        }

        if let Some(bounds) = forward_crisis_coefficient_bounds(
            feature_name,
            horizon_days,
            label_mode,
            uses_family_context_features,
        ) {
            if let Some(min) = bounds.min {
                if *weight < min {
                    *weight = min;
                }
            }
            if let Some(max) = bounds.max {
                if *weight > max {
                    *weight = max;
                }
            }
        }
    }
}

pub(crate) fn forward_crisis_regime_pairwise_targets(
    rows: &[ProbabilityTrainingRow],
    feature_stats: &[ProbabilityFeatureStat],
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
    uses_interaction_tail: bool,
) -> Vec<RegimePairwiseTarget> {
    if !matches!(label_mode, ProbabilityTargetLabelMode::ForwardCrisis) {
        return Vec::new();
    }

    let target_specs = match horizon_days {
        5 if uses_interaction_tail => vec![
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::Normal,
                0.45,
                1.35,
            ),
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::PostCrisisCooldown,
                0.30,
                1.05,
            ),
            (
                ProbabilityTrainingRegime::PreWarningBuffer,
                ProbabilityTrainingRegime::Normal,
                0.15,
                0.50,
            ),
        ],
        20 => vec![
            (
                ProbabilityTrainingRegime::PreWarningBuffer,
                ProbabilityTrainingRegime::Normal,
                if uses_interaction_tail { 1.00 } else { 0.85 },
                if uses_interaction_tail { 1.40 } else { 1.25 },
            ),
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::Normal,
                if uses_interaction_tail { 0.55 } else { 0.40 },
                if uses_interaction_tail { 1.00 } else { 0.85 },
            ),
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::PreWarningBuffer,
                if uses_interaction_tail { 0.40 } else { 0.35 },
                if uses_interaction_tail { 0.80 } else { 0.70 },
            ),
            (
                ProbabilityTrainingRegime::PreWarningBuffer,
                ProbabilityTrainingRegime::PostCrisisCooldown,
                if uses_interaction_tail { 0.90 } else { 0.70 },
                if uses_interaction_tail { 1.25 } else { 1.10 },
            ),
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::PostCrisisCooldown,
                if uses_interaction_tail { 0.70 } else { 0.45 },
                if uses_interaction_tail { 1.05 } else { 0.80 },
            ),
        ],
        60 => vec![
            (
                ProbabilityTrainingRegime::PreWarningBuffer,
                ProbabilityTrainingRegime::Normal,
                if uses_interaction_tail { 1.25 } else { 1.05 },
                if uses_interaction_tail { 1.55 } else { 1.30 },
            ),
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::Normal,
                if uses_interaction_tail { 0.85 } else { 0.65 },
                if uses_interaction_tail { 1.20 } else { 0.95 },
            ),
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::PreWarningBuffer,
                if uses_interaction_tail { 0.60 } else { 0.45 },
                if uses_interaction_tail { 0.80 } else { 0.60 },
            ),
            (
                ProbabilityTrainingRegime::PreWarningBuffer,
                ProbabilityTrainingRegime::PostCrisisCooldown,
                if uses_interaction_tail { 1.15 } else { 0.90 },
                if uses_interaction_tail { 1.60 } else { 1.30 },
            ),
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::PostCrisisCooldown,
                if uses_interaction_tail { 1.20 } else { 0.95 },
                if uses_interaction_tail { 1.30 } else { 1.00 },
            ),
        ],
        _ => Vec::new(),
    };

    target_specs
        .into_iter()
        .filter_map(|(left, right, margin, weight)| {
            let left_centroid = regime_centroid(rows, feature_stats, horizon_days, left)?;
            let right_centroid = regime_centroid(rows, feature_stats, horizon_days, right)?;
            Some(RegimePairwiseTarget {
                left_centroid,
                right_centroid,
                margin,
                weight,
            })
        })
        .collect()
}

fn regime_centroid(
    rows: &[ProbabilityTrainingRow],
    feature_stats: &[ProbabilityFeatureStat],
    horizon_days: u32,
    regime: ProbabilityTrainingRegime,
) -> Option<Vec<f64>> {
    let feature_len = feature_stats.len();
    let mut sum = vec![0.0; feature_len];
    let mut count = 0_usize;
    for row in rows {
        if row.regime_for_horizon(horizon_days) != regime {
            continue;
        }
        let normalized = normalized_features(row, feature_stats);
        for (index, value) in normalized.into_iter().enumerate() {
            sum[index] += value;
        }
        count += 1;
    }
    (count > 0).then(|| {
        sum.into_iter()
            .map(|value| value / count as f64)
            .collect::<Vec<_>>()
    })
}

fn regime_pairwise_strength(horizon_days: u32, uses_interaction_tail: bool) -> f64 {
    match (horizon_days, uses_interaction_tail) {
        (5, true) => 0.70,
        (20, true) => 1.00,
        (60, true) => 1.35,
        (20, false) => 0.80,
        (60, false) => 1.15,
        _ => 0.0,
    }
}

pub(crate) fn apply_regime_pairwise_gradient(
    weight_gradients: &mut [f64],
    weights: &[f64],
    targets: &[RegimePairwiseTarget],
    sample_weight_sum: f64,
    horizon_days: u32,
    uses_interaction_tail: bool,
) {
    if targets.is_empty() {
        return;
    }
    let strength = regime_pairwise_strength(horizon_days, uses_interaction_tail);
    if strength <= 0.0 {
        return;
    }
    let scale = sample_weight_sum * strength / targets.len() as f64;
    for target in targets {
        let left_logit = dot(weights, &target.left_centroid);
        let right_logit = dot(weights, &target.right_centroid);
        let pressure = sigmoid(right_logit + target.margin - left_logit);
        for (index, gradient) in weight_gradients.iter_mut().enumerate() {
            *gradient += target.weight
                * pressure
                * (target.right_centroid[index] - target.left_centroid[index])
                * scale;
        }
    }
}

pub(crate) fn build_feature_stat(
    rows: &[ProbabilityTrainingRow],
    feature_name: &str,
) -> ProbabilityFeatureStat {
    let values = rows
        .iter()
        .map(|row| {
            resolve_probability_feature_value(feature_name, &row.features).unwrap_or_default()
        })
        .collect::<Vec<_>>();
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values
        .iter()
        .map(|value| {
            let diff = value - mean;
            diff * diff
        })
        .sum::<f64>()
        / values.len() as f64;
    ProbabilityFeatureStat {
        name: feature_name.to_string(),
        mean,
        std_dev: variance.sqrt().max(1e-6),
        fill_value: mean,
    }
}

fn initial_intercept(
    rows: &[ProbabilityTrainingRow],
    horizon_days: u32,
    positive_class_weight: f64,
    label_mode: ProbabilityTargetLabelMode,
) -> f64 {
    let weighted_positive = rows
        .iter()
        .map(|row| {
            let label = probability_training_target_label(row, horizon_days, label_mode);
            logistic_sample_weight(row, horizon_days, positive_class_weight, label_mode) * label
        })
        .sum::<f64>();
    let weighted_total = rows
        .iter()
        .map(|row| logistic_sample_weight(row, horizon_days, positive_class_weight, label_mode))
        .sum::<f64>()
        .max(1.0);
    let positive_rate = weighted_positive / weighted_total;
    let clipped = positive_rate.clamp(0.01, 0.99);
    (clipped / (1.0 - clipped)).ln()
}

fn horizon_positive_class_weight(
    rows: &[ProbabilityTrainingRow],
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) -> f64 {
    let positive_units = match label_mode {
        ProbabilityTargetLabelMode::ForwardCrisis => rows
            .iter()
            .filter(|row| row.label_for_horizon(label_mode, horizon_days) > 0.0)
            .map(|row| forward_crisis_positive_sample_weight(row, horizon_days))
            .sum::<f64>(),
        _ => rows
            .iter()
            .filter(|row| row.label_for_horizon(label_mode, horizon_days) > 0.0)
            .count() as f64,
    };
    let negative_weight = rows
        .iter()
        .filter(|row| row.label_for_horizon(label_mode, horizon_days) <= 0.0)
        .map(|row| negative_sample_weight(row, horizon_days, label_mode))
        .sum::<f64>();
    if positive_units <= 0.0 || negative_weight <= 0.0 {
        return 1.0;
    }

    let imbalance_weight = (negative_weight / positive_units).sqrt();
    let (horizon_emphasis, cap) = match label_mode {
        ProbabilityTargetLabelMode::ActionWindow | ProbabilityTargetLabelMode::ActionEpisode => {
            match horizon_days {
                5 => (0.65, 6.0),
                20 => (0.75, 7.0),
                60 => (0.85, 8.0),
                _ => (0.75, 7.0),
            }
        }
        ProbabilityTargetLabelMode::ForwardCrisis => match horizon_days {
            5 => (0.9, 18.0),
            20 => (1.15, 18.0),
            60 => (1.35, 18.0),
            _ => (1.0, 18.0),
        },
    };
    (imbalance_weight * horizon_emphasis).clamp(1.0, cap)
}

pub(crate) fn probability_training_target_label(
    row: &ProbabilityTrainingRow,
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) -> f64 {
    let hard_label = row.label_for_horizon(label_mode, horizon_days);
    if hard_label > 0.0 || label_mode != ProbabilityTargetLabelMode::ForwardCrisis {
        return hard_label;
    }

    if let Some(objective) = forward_crisis_prepare_prewarning_objective(row, horizon_days) {
        return objective.target_label;
    }

    match row.regime_for_horizon(horizon_days) {
        ProbabilityTrainingRegime::Normal => 0.0,
        ProbabilityTrainingRegime::PreWarningBuffer => match horizon_days {
            20 => 0.18,
            60 => 0.26,
            _ => 0.0,
        },
        ProbabilityTrainingRegime::PositiveWindow => match horizon_days {
            20 => 0.24,
            60 => 0.32,
            _ => 0.0,
        },
        ProbabilityTrainingRegime::InCrisis => match horizon_days {
            20 => 0.08,
            60 => 0.12,
            _ => 0.0,
        },
        ProbabilityTrainingRegime::PostCrisisCooldown => match horizon_days {
            20 => 0.01,
            60 => 0.02,
            _ => 0.0,
        },
    }
}

#[derive(Debug, Clone, Copy)]
struct ForwardCrisisPreparePrewarningObjective {
    target_label: f64,
    objective_weight: f64,
}

fn forward_crisis_prepare_prewarning_objective(
    row: &ProbabilityTrainingRow,
    horizon_days: u32,
) -> Option<ForwardCrisisPreparePrewarningObjective> {
    if horizon_days != 60 {
        return None;
    }
    if row.regime_for_horizon(horizon_days) != ProbabilityTrainingRegime::PreWarningBuffer {
        return None;
    }
    if row.label_for_horizon(ProbabilityTargetLabelMode::ForwardCrisis, horizon_days) > 0.0 {
        return None;
    }
    if row.primary_scenario_supports_horizon(horizon_days) != Some(true) {
        return None;
    }
    if row
        .days_to_primary_crisis_start
        .is_none_or(|lead_days| lead_days <= 0)
    {
        return None;
    }
    if !is_prepare_episode_row(row) {
        return None;
    }
    if matches!(
        row.scenario_family.as_deref(),
        Some("acute_market_liquidity_crash")
    ) {
        return None;
    }
    if matches!(
        row.scenario_training_role.as_deref(),
        Some("no_positive_main")
    ) {
        return None;
    }

    let extension_or_protected = row.protected_action_window
        || matches!(
            row.scenario_training_role.as_deref(),
            Some("extension_only")
        );
    Some(ForwardCrisisPreparePrewarningObjective {
        target_label: if extension_or_protected { 0.58 } else { 0.64 },
        objective_weight: if extension_or_protected { 1.10 } else { 1.35 },
    })
}

fn is_prepare_episode_row(row: &ProbabilityTrainingRow) -> bool {
    row.prepare_episode_label > 0 || matches!(row.primary_action_level.as_deref(), Some("prepare"))
}

fn logistic_sample_weight(
    row: &ProbabilityTrainingRow,
    horizon_days: u32,
    positive_class_weight: f64,
    label_mode: ProbabilityTargetLabelMode,
) -> f64 {
    let label = row.label_for_horizon(label_mode, horizon_days);
    if label > 0.0 {
        let positive_weight = match label_mode {
            ProbabilityTargetLabelMode::ForwardCrisis => {
                forward_crisis_positive_sample_weight(row, horizon_days)
            }
            _ => positive_sample_action_weight(row, horizon_days),
        };
        (positive_class_weight * positive_weight).clamp(1.0, 36.0)
    } else {
        negative_sample_weight(row, horizon_days, label_mode)
    }
}

pub(crate) fn forward_crisis_regime_sample_weight(
    horizon_days: u32,
    regime: ProbabilityTrainingRegime,
) -> f64 {
    match regime {
        ProbabilityTrainingRegime::Normal => 1.0,
        ProbabilityTrainingRegime::PreWarningBuffer => match horizon_days {
            5 => 0.90,
            20 => 0.60,
            60 => 0.50,
            _ => 0.70,
        },
        ProbabilityTrainingRegime::PositiveWindow => match horizon_days {
            5 => 2.0,
            20 => 2.2,
            60 => 1.8,
            _ => 2.0,
        },
        ProbabilityTrainingRegime::InCrisis => match horizon_days {
            5 => 1.15,
            20 => 1.20,
            60 => 1.15,
            _ => 1.15,
        },
        ProbabilityTrainingRegime::PostCrisisCooldown => match horizon_days {
            5 => 1.10,
            20 => 1.35,
            60 => 1.60,
            _ => 1.25,
        },
    }
}

pub(crate) fn forward_crisis_positive_sample_weight(
    row: &ProbabilityTrainingRow,
    horizon_days: u32,
) -> f64 {
    (forward_crisis_regime_sample_weight(horizon_days, row.regime_for_horizon(horizon_days))
        * positive_sample_action_weight(row, horizon_days)
        * scenario_training_role_weight_multiplier(
            row.scenario_training_role.as_deref(),
            horizon_days,
        ))
    .clamp(1.0, 12.0)
}

fn forward_crisis_negative_regime_sample_weight(
    horizon_days: u32,
    regime: ProbabilityTrainingRegime,
) -> f64 {
    match regime {
        ProbabilityTrainingRegime::Normal => match horizon_days {
            20 => 1.10,
            60 => 1.15,
            _ => 1.0,
        },
        ProbabilityTrainingRegime::PreWarningBuffer => match horizon_days {
            5 => 0.90,
            20 => 0.70,
            60 => 0.60,
            _ => 0.75,
        },
        ProbabilityTrainingRegime::PositiveWindow => 1.0,
        ProbabilityTrainingRegime::InCrisis => match horizon_days {
            5 => 1.15,
            20 => 1.25,
            60 => 1.20,
            _ => 1.20,
        },
        ProbabilityTrainingRegime::PostCrisisCooldown => match horizon_days {
            5 => 1.10,
            20 => 1.45,
            60 => 1.75,
            _ => 1.40,
        },
    }
}

pub(crate) fn negative_sample_weight(
    row: &ProbabilityTrainingRow,
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) -> f64 {
    match label_mode {
        ProbabilityTargetLabelMode::ActionWindow => match row.regime_for_horizon(horizon_days) {
            ProbabilityTrainingRegime::Normal => 1.0,
            ProbabilityTrainingRegime::PositiveWindow => 1.0,
            ProbabilityTrainingRegime::PreWarningBuffer => match horizon_days {
                5 => 0.85,
                20 => 0.75,
                60 => 0.65,
                _ => 0.75,
            },
            ProbabilityTrainingRegime::InCrisis => match horizon_days {
                5 => 1.90,
                20 => 1.70,
                60 => 1.45,
                _ => 1.60,
            },
            ProbabilityTrainingRegime::PostCrisisCooldown => match horizon_days {
                5 => 1.60,
                20 => 1.45,
                60 => 1.25,
                _ => 1.35,
            },
        },
        ProbabilityTargetLabelMode::ActionEpisode => {
            if row.protected_action_window {
                return 0.55;
            }

            match row.action_episode_phase.as_str() {
                "late_validation" => match horizon_days {
                    5 => 0.95,
                    20 => 0.80,
                    60 => 0.70,
                    _ => 0.80,
                },
                "cooldown" => match horizon_days {
                    5 => 0.70,
                    20 => 0.65,
                    60 => 0.60,
                    _ => 0.65,
                },
                _ => match row.regime_for_horizon(horizon_days) {
                    ProbabilityTrainingRegime::Normal => 1.0,
                    ProbabilityTrainingRegime::PositiveWindow => 1.0,
                    ProbabilityTrainingRegime::PreWarningBuffer => match horizon_days {
                        5 => 0.85,
                        20 => 0.75,
                        60 => 0.65,
                        _ => 0.75,
                    },
                    ProbabilityTrainingRegime::InCrisis => match horizon_days {
                        5 => 1.15,
                        20 => 1.05,
                        60 => 0.95,
                        _ => 1.0,
                    },
                    ProbabilityTrainingRegime::PostCrisisCooldown => match horizon_days {
                        5 => 0.75,
                        20 => 0.70,
                        60 => 0.65,
                        _ => 0.70,
                    },
                },
            }
        }
        ProbabilityTargetLabelMode::ForwardCrisis => {
            if let Some(objective) = forward_crisis_prepare_prewarning_objective(row, horizon_days)
            {
                return objective.objective_weight;
            }
            if row.protected_action_window {
                return match row.action_episode_phase.as_str() {
                    "primary" => match horizon_days {
                        5 => 0.95,
                        20 => 0.55,
                        60 => 0.65,
                        _ => 0.55,
                    },
                    "late_validation" => match horizon_days {
                        5 => 0.95,
                        20 => 0.70,
                        60 => 0.80,
                        _ => 0.65,
                    },
                    "cooldown" => match horizon_days {
                        5 => 1.05,
                        20 => 1.20,
                        60 => 1.35,
                        _ => 1.00,
                    },
                    _ => match horizon_days {
                        5 => 0.95,
                        20 => 0.80,
                        60 => 0.90,
                        _ => 0.75,
                    },
                };
            }
            forward_crisis_negative_regime_sample_weight(
                horizon_days,
                row.regime_for_horizon(horizon_days),
            )
        }
    }
}

pub(crate) fn positive_sample_action_weight(
    row: &ProbabilityTrainingRow,
    horizon_days: u32,
) -> f64 {
    let mut weight = 1.0;
    if let Some(lead_days) = row.days_to_primary_crisis_start {
        weight *= lead_time_positive_multiplier(lead_days, horizon_days);
    }
    weight *= horizon_role_weight_multiplier(row, horizon_days);
    weight *= scenario_family_weight_multiplier(row.scenario_family.as_deref(), horizon_days);
    weight.clamp(0.5, 2.75)
}

fn lead_time_positive_multiplier(lead_days: i64, horizon_days: u32) -> f64 {
    if lead_days <= 0 {
        return 1.0;
    }

    let capped = lead_days.min(horizon_days as i64) as f64;
    let horizon = horizon_days.max(1) as f64;
    let normalized = if horizon <= 1.0 {
        0.0
    } else {
        (capped - 1.0) / (horizon - 1.0)
    };
    let max_lift = match horizon_days {
        5 => 0.35,
        20 => 0.45,
        60 => 0.55,
        _ => 0.30,
    };
    1.0 + normalized.clamp(0.0, 1.0) * max_lift
}

fn horizon_role_weight_multiplier(row: &ProbabilityTrainingRow, horizon_days: u32) -> f64 {
    match row.primary_scenario_supports_horizon(horizon_days) {
        Some(true) => 1.25,
        Some(false) => 0.55,
        None => 1.0,
    }
}

fn scenario_training_role_weight_multiplier(
    scenario_training_role: Option<&str>,
    horizon_days: u32,
) -> f64 {
    match (horizon_days, scenario_training_role) {
        (_, Some("mandatory")) => 1.0,
        (5, Some("candidate_optional")) => 1.10,
        (20, Some("candidate_optional")) => 1.30,
        (60, Some("candidate_optional")) => 1.45,
        (5, Some("extension_only")) => 1.45,
        (20, Some("extension_only")) => 1.65,
        (60, Some("extension_only")) => 1.70,
        (_, Some("no_positive_main")) => 1.0,
        _ => 1.0,
    }
}

fn scenario_family_weight_multiplier(scenario_family: Option<&str>, horizon_days: u32) -> f64 {
    match (horizon_days, scenario_family) {
        (5, Some("acute_market_liquidity_crash")) => 1.50,
        (5, Some("systemic_credit_banking_crisis")) => 0.80,
        (5, Some("mixed_systemic_stress")) => 0.85,
        (5, Some("rate_shock_or_policy_dislocation")) => 0.85,
        (20, Some("acute_market_liquidity_crash")) => 1.30,
        (20, Some("systemic_credit_banking_crisis")) => 1.15,
        (20, Some("mixed_systemic_stress")) => 1.35,
        (20, Some("rate_shock_or_policy_dislocation")) => 1.25,
        (60, Some("acute_market_liquidity_crash")) => 0.85,
        (60, Some("systemic_credit_banking_crisis")) => 1.25,
        (60, Some("mixed_systemic_stress")) => 1.45,
        (60, Some("rate_shock_or_policy_dislocation")) => 1.35,
        _ => 1.0,
    }
}

fn normalized_features(
    row: &ProbabilityTrainingRow,
    feature_stats: &[ProbabilityFeatureStat],
) -> Vec<f64> {
    feature_stats
        .iter()
        .map(|stat| {
            let value = resolve_probability_feature_value(&stat.name, &row.features)
                .unwrap_or(stat.fill_value);
            (value - stat.mean) / stat.std_dev.max(1e-6)
        })
        .collect()
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).map(|(l, r)| l * r).sum()
}

pub(crate) fn fit_platt_calibration(inputs: &[f64], labels: &[f64]) -> PlattCalibrationArtifact {
    let mut alpha = 1.0;
    let mut beta = 0.0;
    let learning_rate = 0.5;
    let sample_count = inputs.len() as f64;

    for _ in 0..500 {
        let mut alpha_gradient = 0.0;
        let mut beta_gradient = 0.0;
        for (input, label) in inputs.iter().zip(labels) {
            let prediction = sigmoid(alpha * input + beta);
            let error = prediction - *label;
            alpha_gradient += error * input;
            beta_gradient += error;
        }
        alpha -= learning_rate * alpha_gradient / sample_count;
        beta -= learning_rate * beta_gradient / sample_count;
    }

    let min_input = inputs.iter().copied().fold(1.0, f64::min);
    let max_input = inputs.iter().copied().fold(0.0, f64::max);
    PlattCalibrationArtifact {
        alpha,
        beta,
        min_input,
        max_input,
    }
}

pub(crate) fn score_logistic_model_for_dataset(
    model: &LogisticProbabilityModel,
    row: &ProbabilityTrainingRow,
) -> f64 {
    let normalized = normalized_features(row, &model.feature_stats);
    sigmoid(
        model.intercept
            + model
                .coefficients
                .iter()
                .zip(normalized)
                .map(|(coefficient, value)| coefficient.weight * value)
                .sum::<f64>(),
    )
}

pub(crate) fn evaluate_probabilities(
    probabilities: &[f64],
    labels: &[f64],
) -> HorizonEvaluationSummary {
    let sample_count = probabilities.len() as u32;
    let positive_rate = labels.iter().sum::<f64>() / labels.len().max(1) as f64;
    let brier_score = probabilities
        .iter()
        .zip(labels)
        .map(|(probability, label)| {
            let diff = probability - label;
            diff * diff
        })
        .sum::<f64>()
        / probabilities.len().max(1) as f64;
    let log_loss = probabilities
        .iter()
        .zip(labels)
        .map(|(probability, label)| {
            let clipped = probability.clamp(0.001, 0.999);
            -(label * clipped.ln() + (1.0 - label) * (1.0 - clipped).ln())
        })
        .sum::<f64>()
        / probabilities.len().max(1) as f64;
    let ece = expected_calibration_error(probabilities, labels, 10);
    let predicted_positive = probabilities
        .iter()
        .zip(labels)
        .filter(|(probability, _)| **probability >= 0.3)
        .collect::<Vec<_>>();
    let true_positive = predicted_positive
        .iter()
        .filter(|(_, label)| **label >= 0.5)
        .count();
    let actual_positive = labels.iter().filter(|label| **label >= 0.5).count();

    HorizonEvaluationSummary {
        sample_count,
        positive_rate,
        brier_score,
        log_loss,
        ece,
        precision_at_30pct: (!predicted_positive.is_empty())
            .then_some(true_positive as f64 / predicted_positive.len() as f64),
        recall_at_30pct: (actual_positive > 0)
            .then_some(true_positive as f64 / actual_positive as f64),
        regime_separation: None,
        actionability: None,
    }
}

fn expected_calibration_error(probabilities: &[f64], labels: &[f64], bins: usize) -> f64 {
    let mut error = 0.0;
    for bin in 0..bins {
        let start = bin as f64 / bins as f64;
        let end = (bin + 1) as f64 / bins as f64;
        let bucket = probabilities
            .iter()
            .zip(labels)
            .filter(|(probability, _)| {
                (bin + 1 == bins && **probability >= start && **probability <= end)
                    || (**probability >= start && **probability < end)
            })
            .collect::<Vec<_>>();
        if bucket.is_empty() {
            continue;
        }
        let avg_probability = bucket
            .iter()
            .map(|(probability, _)| **probability)
            .sum::<f64>()
            / bucket.len() as f64;
        let avg_label = bucket.iter().map(|(_, label)| **label).sum::<f64>() / bucket.len() as f64;
        error += (bucket.len() as f64 / probabilities.len() as f64)
            * (avg_probability - avg_label).abs();
    }
    error
}

fn sigmoid(value: f64) -> f64 {
    1.0 / (1.0 + (-value).exp())
}

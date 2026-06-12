use crate::{
    resolve_probability_feature_value, LogisticProbabilityModel, ProbabilityCoefficient,
    ProbabilityFeatureStat, ProbabilityTargetLabelMode, ProbabilityTrainingRow,
    PROBABILITY_FEATURE_TRANSFORM_IDENTITY_V1, PROBABILITY_FEATURE_TRANSFORM_INTERACTION_TAIL_V1,
};

mod calibration;
mod constraints;
mod regime;
mod weighting;

pub(crate) use calibration::{
    evaluate_probabilities, fit_platt_calibration, score_logistic_model_for_dataset,
};
pub(crate) use constraints::{
    apply_forward_crisis_coefficient_bound_gradient, apply_forward_crisis_sign_gradient,
    project_forward_crisis_sign_constraints,
};
pub(crate) use regime::{
    apply_regime_pairwise_gradient, forward_crisis_regime_pairwise_targets, RegimePairwiseTarget,
};
#[cfg(test)]
pub(crate) use weighting::forward_crisis_regime_sample_weight;
pub(crate) use weighting::{
    forward_crisis_has_episode_native_objective,
    forward_crisis_is_protected_no_positive_main_episode_row,
    forward_crisis_positive_sample_weight, negative_sample_weight, positive_sample_action_weight,
    probability_training_target_label,
};
use weighting::{horizon_positive_class_weight, logistic_sample_weight};

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
    let regime_pairwise_targets: Vec<RegimePairwiseTarget> = forward_crisis_regime_pairwise_targets(
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

fn sigmoid(value: f64) -> f64 {
    1.0 / (1.0 + (-value).exp())
}

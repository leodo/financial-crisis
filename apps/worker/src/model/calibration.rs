use crate::{
    HorizonEvaluationSummary, LogisticProbabilityModel, PlattCalibrationArtifact,
    ProbabilityTrainingRow,
};

use super::{normalized_features, sigmoid};

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

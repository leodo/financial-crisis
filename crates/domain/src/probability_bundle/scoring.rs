use std::collections::BTreeMap;

use super::features::resolve_probability_feature_value;
use super::types::{
    LogisticProbabilityFeatureContribution, LogisticProbabilityModel,
    LogisticProbabilityModelScoreDiagnostics, PlattCalibrationArtifact, ProbabilityHorizonBundle,
    ProbabilityHorizonScore, ProbabilityOverlayContribution,
};

pub fn score_logistic_probability_model(
    model: &LogisticProbabilityModel,
    features: &BTreeMap<String, f64>,
) -> f64 {
    score_logistic_probability_model_with_diagnostics(model, features).probability
}

pub fn score_logistic_probability_model_with_diagnostics(
    model: &LogisticProbabilityModel,
    features: &BTreeMap<String, f64>,
) -> LogisticProbabilityModelScoreDiagnostics {
    let mut linear = model.intercept;
    let mut feature_contributions = Vec::with_capacity(model.coefficients.len());
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
        let contribution = normalized * coefficient.weight;
        linear += contribution;
        feature_contributions.push(LogisticProbabilityFeatureContribution {
            name: coefficient.name.clone(),
            raw_value,
            normalized_value: normalized,
            weight: coefficient.weight,
            contribution,
        });
    }
    LogisticProbabilityModelScoreDiagnostics {
        intercept: model.intercept,
        linear_score: linear,
        probability: probability_sigmoid(linear),
        feature_contributions,
    }
}

pub fn score_probability_horizon_bundle(
    horizon: &ProbabilityHorizonBundle,
    features: &BTreeMap<String, f64>,
) -> ProbabilityHorizonScore {
    let raw_probability = score_logistic_probability_model(&horizon.raw_model, features);
    let calibrated_probability = horizon
        .calibration
        .as_ref()
        .map_or(raw_probability, |calibration| {
            apply_platt_probability_calibration(raw_probability, calibration)
        });
    let mut final_probability = calibrated_probability;
    let mut overlay_contributions = Vec::new();

    for overlay in &horizon.family_overlays {
        let Some(gate_value) = resolve_probability_feature_value(&overlay.gate_feature, features)
        else {
            continue;
        };
        let gate = probability_sigmoid((gate_value - overlay.gate_threshold) * overlay.gate_slope);
        let overlay_raw_probability =
            score_logistic_probability_model(&overlay.raw_model, features);
        let overlay_probability = overlay
            .calibration
            .as_ref()
            .map_or(overlay_raw_probability, |calibration| {
                apply_platt_probability_calibration(overlay_raw_probability, calibration)
            });
        let blend = (gate * overlay.blend_weight).clamp(0.0, 0.50);
        let before = final_probability;
        final_probability = final_probability * (1.0 - blend) + overlay_probability * blend;
        overlay_contributions.push(ProbabilityOverlayContribution {
            family_id: overlay.family_id.clone(),
            gate_feature: overlay.gate_feature.clone(),
            gate_value,
            gate,
            blend,
            overlay_probability,
            contribution: final_probability - before,
        });
    }

    ProbabilityHorizonScore {
        raw_probability,
        calibrated_probability,
        final_probability,
        overlay_contributions,
    }
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

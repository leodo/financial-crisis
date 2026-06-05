use std::collections::BTreeMap;

use fc_domain::{
    apply_platt_probability_calibration, score_logistic_probability_model, ActionabilityBundle,
    ActionabilityLevel, ProbabilityBlock, RiskSnapshot,
};

use super::super::{round3, ProbabilityActionThresholds};

pub(super) fn score_actionability_level(
    bundle: &ActionabilityBundle,
    level: ActionabilityLevel,
    features: &BTreeMap<String, f64>,
) -> Option<f64> {
    let level_bundle = bundle
        .levels
        .iter()
        .find(|candidate| candidate.level == level)?;
    let raw_probability = score_logistic_probability_model(&level_bundle.raw_model, features);
    let calibrated_probability = match level_bundle.calibration.as_ref() {
        Some(calibration) => apply_platt_probability_calibration(raw_probability, calibration),
        None => raw_probability,
    };
    Some(actionability_confidence_from_probability(
        calibrated_probability,
        level_bundle.decision_threshold,
    ))
}

pub(super) fn actionability_confidence_from_probability(
    probability: f64,
    decision_threshold: f64,
) -> f64 {
    let threshold = decision_threshold.clamp(0.01, 0.95);
    if probability <= threshold {
        return 0.0;
    }
    let normalized = ((probability - threshold) / (1.0 - threshold)).clamp(0.0, 1.0);
    normalized.powi(2)
}

pub(super) fn fuse_actionability_confidence(
    level: ActionabilityLevel,
    confidence: f64,
    probabilities: &ProbabilityBlock,
    snapshot: &RiskSnapshot,
    external_shock_score: f64,
    thresholds: ProbabilityActionThresholds,
) -> f64 {
    if confidence <= 0.0 {
        return 0.0;
    }

    let context_support = match level {
        ActionabilityLevel::Prepare => {
            0.55 * normalized_score_support(snapshot.structural_score, 48.0, 62.0)
                + 0.25
                    * normalized_probability_support(
                        probabilities.p_60d,
                        thresholds.prepare_p60d,
                        thresholds.elevated_weeks_p60d(),
                    )
                + 0.20 * normalized_score_support(external_shock_score, 45.0, 60.0)
        }
        ActionabilityLevel::Hedge => {
            0.40 * normalized_score_support(snapshot.trigger_score, 42.0, 60.0)
                + 0.25 * normalized_score_support(external_shock_score, 44.0, 58.0)
                + 0.20 * normalized_score_support(snapshot.structural_score, 45.0, 58.0)
                + 0.15
                    * normalized_probability_support(
                        probabilities.p_20d,
                        thresholds.hedge_p20d,
                        thresholds.severe_now_p20d(),
                    )
        }
        ActionabilityLevel::Defend => {
            0.50 * normalized_score_support(snapshot.trigger_score, 55.0, 68.0)
                + 0.20 * normalized_score_support(external_shock_score, 52.0, 65.0)
                + 0.15 * normalized_score_support(snapshot.structural_score, 50.0, 62.0)
                + 0.15
                    * normalized_probability_support(
                        probabilities.p_5d,
                        thresholds.defend_p5d,
                        thresholds.capital_preservation_p5d(),
                    )
        }
    }
    .clamp(0.0, 1.0);

    round3((confidence * context_support).clamp(0.0, 1.0))
}

fn normalized_score_support(value: f64, start: f64, full: f64) -> f64 {
    if full <= start {
        return f64::from(value >= full);
    }
    ((value - start) / (full - start)).clamp(0.0, 1.0)
}

fn normalized_probability_support(value: f64, threshold: f64, full: f64) -> f64 {
    if full <= threshold {
        return f64::from(value >= full);
    }
    ((value - threshold) / (full - threshold)).clamp(0.0, 1.0)
}

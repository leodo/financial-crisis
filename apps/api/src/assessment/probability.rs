mod actionability;
mod features;
mod heuristic;
mod trace;

pub(crate) use trace::ProbabilityComputationTrace;

use fc_domain::{
    DataTrust, JpyCarrySnapshot, KeyIndicatorStatus, Observation, ProbabilityBlock, RiskSnapshot,
};

use super::ServingModelContext;

pub(super) fn build_probabilities(
    snapshot: &RiskSnapshot,
    external_shock_score: f64,
    conviction_score: f64,
    breadth_score: f64,
    data_trust: &DataTrust,
    jpy_carry: &JpyCarrySnapshot,
) -> ProbabilityBlock {
    heuristic::build_probabilities(
        snapshot,
        external_shock_score,
        conviction_score,
        breadth_score,
        data_trust,
        jpy_carry,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_probability_trace(
    snapshot: &RiskSnapshot,
    observations: &[Observation],
    external_shock_score: f64,
    data_trust: &DataTrust,
    jpy_carry: &JpyCarrySnapshot,
    heuristic_probabilities: &ProbabilityBlock,
    key_indicators: &[KeyIndicatorStatus],
    serving_model: Option<&ServingModelContext>,
) -> ProbabilityComputationTrace {
    trace::build_probability_trace(
        snapshot,
        observations,
        external_shock_score,
        data_trust,
        jpy_carry,
        heuristic_probabilities,
        key_indicators,
        serving_model,
    )
}

#[cfg(test)]
pub(super) fn actionability_confidence_from_probability(
    probability: f64,
    decision_threshold: f64,
) -> f64 {
    actionability::actionability_confidence_from_probability(probability, decision_threshold)
}

#[cfg(test)]
pub(super) fn fuse_actionability_confidence(
    level: fc_domain::ActionabilityLevel,
    confidence: f64,
    probabilities: &ProbabilityBlock,
    snapshot: &RiskSnapshot,
    external_shock_score: f64,
    thresholds: super::ProbabilityActionThresholds,
) -> f64 {
    actionability::fuse_actionability_confidence(
        level,
        confidence,
        probabilities,
        snapshot,
        external_shock_score,
        thresholds,
    )
}

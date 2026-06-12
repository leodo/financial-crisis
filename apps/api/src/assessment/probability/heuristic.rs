use fc_domain::{ActionabilityBlock, DataTrust, JpyCarrySnapshot, ProbabilityBlock, RiskSnapshot};

use super::super::common::round_probability;
use super::super::{clamp_probability, round3, scaled_pressure};

pub(super) fn build_probabilities(
    snapshot: &RiskSnapshot,
    external_shock_score: f64,
    conviction_score: f64,
    breadth_score: f64,
    data_trust: &DataTrust,
    jpy_carry: &JpyCarrySnapshot,
) -> ProbabilityBlock {
    let structural_pressure = scaled_pressure(snapshot.structural_score, 52.0, 20.0);
    let trigger_pressure = scaled_pressure(snapshot.trigger_score, 55.0, 18.0);
    let external_pressure = scaled_pressure(external_shock_score, 42.0, 18.0);
    let breadth_pressure = scaled_pressure(breadth_score, 38.0, 24.0);
    let carry_funding_pressure = scaled_pressure(jpy_carry.funding_pressure_score, 38.0, 30.0);
    let carry_state_pressure = scaled_pressure(jpy_carry.score, 34.0, 28.0);
    let confidence_penalty = (1.0 - conviction_score) * 0.18;
    let quality_penalty = (1.0 - data_trust.coverage_score) * 0.15;
    let interaction = structural_pressure * trigger_pressure;
    let acute_interaction = trigger_pressure * external_pressure;
    let carry_trigger_interaction = carry_state_pressure * trigger_pressure;

    let p_60d_raw = clamp_probability(
        0.04 + structural_pressure * 0.44
            + trigger_pressure * 0.18
            + external_pressure * 0.08
            + carry_funding_pressure * 0.08
            + breadth_pressure * 0.08
            - quality_penalty * 0.45,
    );
    let p_20d_raw = clamp_probability(
        0.02 + structural_pressure * 0.16
            + trigger_pressure * 0.34
            + external_pressure * 0.14
            + carry_funding_pressure * 0.07
            + interaction * 0.11
            + carry_trigger_interaction * 0.08
            + breadth_pressure * 0.07
            - confidence_penalty * 0.4
            - quality_penalty * 0.2,
    );
    let p_5d = clamp_probability(
        0.01 + trigger_pressure * 0.15
            + external_pressure * 0.16
            + carry_state_pressure * 0.08
            + acute_interaction * 0.18
            + carry_trigger_interaction * 0.12
            + breadth_pressure * 0.05
            - structural_pressure * 0.03
            - confidence_penalty * 0.5
            - quality_penalty * 0.2,
    );
    let p_20d = clamp_probability(p_20d_raw.max((p_5d + 0.03).min(0.93)));
    let p_60d = clamp_probability(p_60d_raw.max((p_20d + 0.05).min(0.93)));

    ProbabilityBlock {
        p_5d: round_probability(p_5d),
        p_20d: round_probability(p_20d),
        p_60d: round_probability(p_60d),
    }
}

pub(super) fn heuristic_actionability_block(
    snapshot: &RiskSnapshot,
    external_shock_score: f64,
    probabilities: &ProbabilityBlock,
    data_trust: &DataTrust,
    jpy_carry: &JpyCarrySnapshot,
) -> ActionabilityBlock {
    let quality_penalty = (1.0 - data_trust.coverage_score).clamp(0.0, 1.0) * 0.12;
    let prepare = super::super::clamp_probability(
        probabilities.p_60d * 0.72
            + scaled_pressure(snapshot.structural_score, 55.0, 18.0) * 0.22
            + scaled_pressure(external_shock_score, 48.0, 18.0) * 0.08
            - quality_penalty,
    );
    let hedge = super::super::clamp_probability(
        probabilities.p_20d * 0.74
            + scaled_pressure(snapshot.trigger_score, 52.0, 20.0) * 0.22
            + scaled_pressure(external_shock_score, 50.0, 18.0) * 0.10
            + scaled_pressure(jpy_carry.score, 58.0, 18.0) * 0.06
            - quality_penalty,
    );
    let defend = super::super::clamp_probability(
        probabilities.p_5d * 0.78
            + scaled_pressure(snapshot.trigger_score, 60.0, 18.0) * 0.18
            + scaled_pressure(external_shock_score, 55.0, 18.0) * 0.10
            + scaled_pressure(jpy_carry.funding_pressure_score, 55.0, 16.0) * 0.08
            - quality_penalty,
    );

    ActionabilityBlock {
        prepare: round3(prepare),
        hedge: round3(hedge.max((defend + 0.02).min(0.97))),
        defend: round3(defend),
    }
}

use std::collections::BTreeMap;

use fc_domain::{
    ActionabilityBlock, ActionabilityLevel, DataTrust, JpyCarrySnapshot, KeyIndicatorStatus,
    Observation, ProbabilityBlock, ProbabilityBundle, ProbabilityDiagnostics,
    ProbabilityHorizonOverlayDiagnostics, ProbabilityHorizonScore, RiskSnapshot,
};

use super::{
    actionability::{fuse_actionability_confidence, score_actionability_level},
    features::build_probability_feature_map,
    heuristic::heuristic_actionability_block,
};
use crate::assessment::{probability_action_thresholds, round3, ServingModelContext};

#[derive(Debug, Clone)]
pub(crate) struct ProbabilityComputationTrace {
    pub raw_probabilities: ProbabilityBlock,
    pub calibrated_probabilities: ProbabilityBlock,
    pub probability_diagnostics: ProbabilityDiagnostics,
    pub actionability: ActionabilityBlock,
    pub actionability_enabled: bool,
    pub actionability_model_version: Option<String>,
    pub actionability_calibration_version: Option<String>,
    pub fusion_policy_version: Option<String>,
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
    let heuristic_actionability = heuristic_actionability_block(
        snapshot,
        external_shock_score,
        heuristic_probabilities,
        data_trust,
        jpy_carry,
    );
    let Some(serving_model) = serving_model else {
        return ProbabilityComputationTrace {
            raw_probabilities: heuristic_probabilities.clone(),
            calibrated_probabilities: heuristic_probabilities.clone(),
            probability_diagnostics: ProbabilityDiagnostics::default(),
            actionability: heuristic_actionability,
            actionability_enabled: false,
            actionability_model_version: None,
            actionability_calibration_version: None,
            fusion_policy_version: None,
        };
    };
    let Some(bundle) = serving_model.probability_bundle.as_ref() else {
        return ProbabilityComputationTrace {
            raw_probabilities: heuristic_probabilities.clone(),
            calibrated_probabilities: heuristic_probabilities.clone(),
            probability_diagnostics: ProbabilityDiagnostics::default(),
            actionability: heuristic_actionability,
            actionability_enabled: false,
            actionability_model_version: None,
            actionability_calibration_version: None,
            fusion_policy_version: None,
        };
    };

    let features = build_probability_feature_map(
        snapshot,
        observations,
        external_shock_score,
        data_trust,
        jpy_carry,
        heuristic_probabilities,
        key_indicators,
    );

    let score_5d = score_bundle_horizon(bundle, 5, &features);
    let score_20d = score_bundle_horizon(bundle, 20, &features);
    let score_60d = score_bundle_horizon(bundle, 60, &features);
    let raw_p_5d = score_5d
        .as_ref()
        .map_or(heuristic_probabilities.p_5d, |score| score.raw_probability);
    let calibrated_p_5d_raw = score_5d
        .as_ref()
        .map_or(heuristic_probabilities.p_5d, |score| {
            score.final_probability
        });
    let raw_p_20d = score_20d
        .as_ref()
        .map_or(heuristic_probabilities.p_20d, |score| score.raw_probability);
    let calibrated_p_20d_raw = score_20d
        .as_ref()
        .map_or(heuristic_probabilities.p_20d, |score| {
            score.final_probability
        });
    let raw_p_60d = score_60d
        .as_ref()
        .map_or(heuristic_probabilities.p_60d, |score| score.raw_probability);
    let calibrated_p_60d_raw = score_60d
        .as_ref()
        .map_or(heuristic_probabilities.p_60d, |score| {
            score.final_probability
        });

    let raw_probabilities = ProbabilityBlock {
        p_5d: round3(raw_p_5d),
        p_20d: round3(raw_p_20d),
        p_60d: round3(raw_p_60d),
    };

    let min_gap_5_to_20 = bundle.monotonic_min_gap_5d_to_20d.max(0.0);
    let min_gap_20_to_60 = bundle.monotonic_min_gap_20d_to_60d.max(0.0);
    let calibrated_p_5d = calibrated_p_5d_raw;
    let calibrated_p_20d = crate::assessment::clamp_probability(
        calibrated_p_20d_raw.max((calibrated_p_5d + min_gap_5_to_20).min(0.98)),
    );
    let calibrated_p_60d = crate::assessment::clamp_probability(
        calibrated_p_60d_raw.max((calibrated_p_20d + min_gap_20_to_60).min(0.99)),
    );
    let calibrated_probabilities = ProbabilityBlock {
        p_5d: round3(calibrated_p_5d),
        p_20d: round3(calibrated_p_20d),
        p_60d: round3(calibrated_p_60d),
    };
    let probability_diagnostics = ProbabilityDiagnostics {
        horizon_overlays: bundle
            .horizons
            .iter()
            .filter_map(|horizon| {
                let score = match horizon.horizon_days {
                    5 => score_5d.as_ref(),
                    20 => score_20d.as_ref(),
                    60 => score_60d.as_ref(),
                    _ => None,
                }?;
                let diagnostics = ProbabilityHorizonOverlayDiagnostics {
                    horizon_days: horizon.horizon_days,
                    raw_probability: round3(score.raw_probability),
                    calibrated_probability: round3(score.calibrated_probability),
                    final_probability: round3(score.final_probability),
                    runtime_final_probability: Some(match horizon.horizon_days {
                        5 => round3(calibrated_p_5d),
                        20 => round3(calibrated_p_20d),
                        60 => round3(calibrated_p_60d),
                        _ => round3(score.final_probability),
                    }),
                    monotonic_lift: round3(match horizon.horizon_days {
                        5 => calibrated_p_5d - score.final_probability,
                        20 => calibrated_p_20d - score.final_probability,
                        60 => calibrated_p_60d - score.final_probability,
                        _ => 0.0,
                    }),
                    configured_overlay_count: horizon.family_overlays.len() as u32,
                    contributions: score.overlay_contributions.clone(),
                    overlay_audits: horizon.family_overlay_audits.clone(),
                };
                (diagnostics.configured_overlay_count > 0
                    || diagnostics.monotonic_lift.abs() > f64::EPSILON
                    || !diagnostics.contributions.is_empty()
                    || !diagnostics.overlay_audits.is_empty())
                .then_some(diagnostics)
            })
            .collect(),
    };
    let action_thresholds = probability_action_thresholds(Some(serving_model));

    let actionability = bundle
        .actionability
        .as_ref()
        .map(|actionability_bundle| ActionabilityBlock {
            prepare: round3(fuse_actionability_confidence(
                ActionabilityLevel::Prepare,
                score_actionability_level(
                    actionability_bundle,
                    ActionabilityLevel::Prepare,
                    &features,
                )
                .unwrap_or(heuristic_actionability.prepare),
                &calibrated_probabilities,
                snapshot,
                external_shock_score,
                action_thresholds,
            )),
            hedge: round3(fuse_actionability_confidence(
                ActionabilityLevel::Hedge,
                score_actionability_level(
                    actionability_bundle,
                    ActionabilityLevel::Hedge,
                    &features,
                )
                .unwrap_or(heuristic_actionability.hedge),
                &calibrated_probabilities,
                snapshot,
                external_shock_score,
                action_thresholds,
            )),
            defend: round3(fuse_actionability_confidence(
                ActionabilityLevel::Defend,
                score_actionability_level(
                    actionability_bundle,
                    ActionabilityLevel::Defend,
                    &features,
                )
                .unwrap_or(heuristic_actionability.defend),
                &calibrated_probabilities,
                snapshot,
                external_shock_score,
                action_thresholds,
            )),
        })
        .unwrap_or_else(|| heuristic_actionability.clone());

    ProbabilityComputationTrace {
        raw_probabilities,
        calibrated_probabilities,
        probability_diagnostics,
        actionability,
        actionability_enabled: bundle.actionability.is_some(),
        actionability_model_version: bundle
            .actionability
            .as_ref()
            .map(|bundle| bundle.model_version.clone()),
        actionability_calibration_version: bundle
            .actionability
            .as_ref()
            .map(|bundle| bundle.calibration_version.clone()),
        fusion_policy_version: bundle
            .actionability
            .as_ref()
            .map(|_| "fusion_policy_v3_probability_context_gate_20260601".to_string()),
    }
}

fn score_bundle_horizon(
    bundle: &ProbabilityBundle,
    horizon_days: u32,
    features: &BTreeMap<String, f64>,
) -> Option<ProbabilityHorizonScore> {
    let horizon = bundle
        .horizons
        .iter()
        .find(|horizon| horizon.horizon_days == horizon_days)?;
    Some(fc_domain::score_probability_horizon_bundle(
        horizon, features,
    ))
}

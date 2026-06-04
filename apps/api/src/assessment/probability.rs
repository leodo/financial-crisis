use std::collections::BTreeMap;

use fc_domain::{
    apply_platt_probability_calibration, formal_observation_feature_value,
    score_logistic_probability_model, score_probability_horizon_bundle, ActionabilityBlock,
    ActionabilityBundle, ActionabilityLevel, DataTrust, FreshnessStatus, JpyCarrySnapshot,
    KeyIndicatorStatus, Observation, ProbabilityBlock, ProbabilityBundle, ProbabilityDiagnostics,
    ProbabilityHorizonOverlayDiagnostics, ProbabilityHorizonScore, RiskDimension, RiskSnapshot,
    TimeToRiskBucket, FEATURE_BUCKET_MONTHS_OR_HIGHER, FEATURE_BUCKET_NOW,
    FEATURE_BUCKET_WEEKS_OR_HIGHER, FEATURE_COVERAGE_SCORE, FEATURE_EXTERNAL_DIMENSION_SCORE,
    FEATURE_EXTERNAL_SHOCK_SCORE, FEATURE_FRESHNESS_DELAYED_OR_WORSE,
    FEATURE_FRESHNESS_STALE_OR_MISSING, FEATURE_HEURISTIC_P_20D, FEATURE_HEURISTIC_P_5D,
    FEATURE_HEURISTIC_P_60D, FEATURE_OVERALL_SCORE, FEATURE_STRUCTURAL_SCORE,
    FEATURE_TRIGGER_SCORE, FORMAL_OBSERVATION_FEATURE_SPECS,
};

use super::{
    build_time_to_risk_bucket, clamp_probability, high_risk_breadth, probability_action_thresholds,
    round3, round6, scaled_pressure, ProbabilityActionThresholds, ServingModelContext,
};

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
        p_5d: round3(p_5d),
        p_20d: round3(p_20d),
        p_60d: round3(p_60d),
    }
}

fn heuristic_actionability_block(
    snapshot: &RiskSnapshot,
    external_shock_score: f64,
    probabilities: &ProbabilityBlock,
    data_trust: &DataTrust,
    jpy_carry: &JpyCarrySnapshot,
) -> ActionabilityBlock {
    let quality_penalty = (1.0 - data_trust.coverage_score).clamp(0.0, 1.0) * 0.12;
    let prepare = clamp_probability(
        probabilities.p_60d * 0.72
            + scaled_pressure(snapshot.structural_score, 55.0, 18.0) * 0.22
            + scaled_pressure(external_shock_score, 48.0, 18.0) * 0.08
            - quality_penalty,
    );
    let hedge = clamp_probability(
        probabilities.p_20d * 0.74
            + scaled_pressure(snapshot.trigger_score, 52.0, 20.0) * 0.22
            + scaled_pressure(external_shock_score, 50.0, 18.0) * 0.10
            + scaled_pressure(jpy_carry.score, 58.0, 18.0) * 0.06
            - quality_penalty,
    );
    let defend = clamp_probability(
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
    let calibrated_p_20d =
        clamp_probability(calibrated_p_20d_raw.max((calibrated_p_5d + min_gap_5_to_20).min(0.98)));
    let calibrated_p_60d = clamp_probability(
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

fn build_probability_feature_map(
    snapshot: &RiskSnapshot,
    observations: &[Observation],
    external_shock_score: f64,
    data_trust: &DataTrust,
    jpy_carry: &JpyCarrySnapshot,
    heuristic_probabilities: &ProbabilityBlock,
    key_indicators: &[KeyIndicatorStatus],
) -> BTreeMap<String, f64> {
    let heuristic_bucket = build_time_to_risk_bucket(
        heuristic_probabilities,
        None,
        None,
        snapshot.structural_score,
        snapshot.trigger_score,
        external_shock_score,
        high_risk_breadth(snapshot),
        jpy_carry,
        ProbabilityActionThresholds::legacy(),
    );
    let freshness_status = worst_key_indicator_freshness(key_indicators);
    let mut features = BTreeMap::from([
        (
            FEATURE_OVERALL_SCORE.to_string(),
            (snapshot.overall_score / 100.0).clamp(0.0, 1.0),
        ),
        (
            FEATURE_EXTERNAL_SHOCK_SCORE.to_string(),
            (external_shock_score / 100.0).clamp(0.0, 1.0),
        ),
        (
            FEATURE_HEURISTIC_P_5D.to_string(),
            clamp_probability(heuristic_probabilities.p_5d),
        ),
        (
            FEATURE_HEURISTIC_P_20D.to_string(),
            clamp_probability(heuristic_probabilities.p_20d),
        ),
        (
            FEATURE_HEURISTIC_P_60D.to_string(),
            clamp_probability(heuristic_probabilities.p_60d),
        ),
        (
            FEATURE_COVERAGE_SCORE.to_string(),
            data_trust.coverage_score.clamp(0.0, 1.0),
        ),
        (
            FEATURE_BUCKET_MONTHS_OR_HIGHER.to_string(),
            matches!(
                heuristic_bucket,
                TimeToRiskBucket::Months | TimeToRiskBucket::Weeks | TimeToRiskBucket::Now
            ) as u8 as f64,
        ),
        (
            FEATURE_BUCKET_WEEKS_OR_HIGHER.to_string(),
            matches!(
                heuristic_bucket,
                TimeToRiskBucket::Weeks | TimeToRiskBucket::Now
            ) as u8 as f64,
        ),
        (
            FEATURE_BUCKET_NOW.to_string(),
            matches!(heuristic_bucket, TimeToRiskBucket::Now) as u8 as f64,
        ),
        (
            FEATURE_FRESHNESS_DELAYED_OR_WORSE.to_string(),
            matches!(
                freshness_status,
                FreshnessStatus::Delayed | FreshnessStatus::Stale | FreshnessStatus::Missing
            ) as u8 as f64,
        ),
        (
            FEATURE_FRESHNESS_STALE_OR_MISSING.to_string(),
            matches!(
                freshness_status,
                FreshnessStatus::Stale | FreshnessStatus::Missing
            ) as u8 as f64,
        ),
    ]);
    features.extend(build_formal_probability_feature_map(
        snapshot,
        observations,
        data_trust,
    ));
    features
}

fn build_formal_probability_feature_map(
    snapshot: &RiskSnapshot,
    observations: &[Observation],
    data_trust: &DataTrust,
) -> BTreeMap<String, f64> {
    let mut features = BTreeMap::from([
        (
            FEATURE_STRUCTURAL_SCORE.to_string(),
            round6((snapshot.structural_score / 100.0).clamp(0.0, 1.0)),
        ),
        (
            FEATURE_TRIGGER_SCORE.to_string(),
            round6((snapshot.trigger_score / 100.0).clamp(0.0, 1.0)),
        ),
        (
            FEATURE_EXTERNAL_DIMENSION_SCORE.to_string(),
            round6(
                (dimension_score(snapshot, RiskDimension::ExternalSector) / 100.0).clamp(0.0, 1.0),
            ),
        ),
        (
            FEATURE_COVERAGE_SCORE.to_string(),
            data_trust.coverage_score.clamp(0.0, 1.0),
        ),
    ]);
    let as_of_date = snapshot.as_of_date;

    for spec in FORMAL_OBSERVATION_FEATURE_SPECS {
        if let Some(value) = formal_observation_feature_value(observations, spec, as_of_date) {
            features.insert(spec.feature_name.to_string(), round6(value));
        }
    }

    features
}

fn dimension_score(snapshot: &RiskSnapshot, dimension: RiskDimension) -> f64 {
    snapshot
        .dimensions
        .iter()
        .find(|score| score.dimension == dimension)
        .map(|score| score.score)
        .unwrap_or(0.0)
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
    Some(score_probability_horizon_bundle(horizon, features))
}

fn score_actionability_level(
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

fn worst_key_indicator_freshness(key_indicators: &[KeyIndicatorStatus]) -> FreshnessStatus {
    key_indicators
        .iter()
        .map(|indicator| indicator.status)
        .max_by_key(|status| freshness_rank(*status))
        .unwrap_or(FreshnessStatus::Missing)
}

fn freshness_rank(status: FreshnessStatus) -> u8 {
    match status {
        FreshnessStatus::Fresh => 0,
        FreshnessStatus::Delayed => 1,
        FreshnessStatus::Stale => 2,
        FreshnessStatus::Missing => 3,
    }
}

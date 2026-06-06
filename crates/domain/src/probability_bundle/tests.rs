use std::collections::BTreeMap;

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
    assert!(expanded.contains(&"family_context__jpy_carry__external_dimension_score".to_string()));
}

#[test]
fn family_hybrid_transform_reuses_family_feature_expansion() {
    let base = FORMAL_PROBABILITY_BUNDLE_FEATURES
        .iter()
        .map(|feature| (*feature).to_string())
        .collect::<Vec<_>>();
    let expanded = probability_feature_names_for_transform(
        &base,
        PROBABILITY_FEATURE_TRANSFORM_FAMILY_HYBRID_V1,
    );

    assert!(expanded.contains(&"interaction__overall_score__us_vix_level".to_string()));
    assert!(expanded.contains(&"family_proxy__rate_shock".to_string()));
}

#[test]
fn derived_feature_resolver_handles_interactions_and_tail_features() {
    let mut features = BTreeMap::new();
    features.insert(FEATURE_OVERALL_SCORE.to_string(), 60.0);
    features.insert(FEATURE_EXTERNAL_DIMENSION_SCORE.to_string(), 65.0);
    features.insert(FEATURE_US_VIX_LEVEL.to_string(), 28.0);
    features.insert(FEATURE_US_CURVE_10Y2Y_LEVEL.to_string(), -0.5);
    features.insert(FEATURE_US_USDJPY_LEVEL.to_string(), 160.0);
    features.insert(FEATURE_US_USDJPY_CHANGE_20D.to_string(), -6.0);

    assert_eq!(
        resolve_probability_feature_value("interaction__overall_score__us_vix_level", &features),
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
    assert_eq!(
        resolve_probability_feature_value(
            "interaction__external_dimension_score__tail_pos__us_usdjpy_level__145",
            &features,
        ),
        Some(975.0)
    );
}

#[test]
fn derived_feature_resolver_handles_family_conditional_features() {
    let mut features = BTreeMap::new();
    features.insert(FEATURE_OVERALL_SCORE.to_string(), 78.0);
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
    let mixed =
        resolve_probability_feature_value("family_proxy__mixed_systemic", &features).unwrap();
    let carry = resolve_probability_feature_value("family_proxy__jpy_carry", &features).unwrap();
    let carry_context = resolve_probability_feature_value(
        "family_context__jpy_carry__external_dimension_score",
        &features,
    )
    .unwrap();

    assert!(systemic > 0.70 && systemic <= 1.0);
    assert!(mixed > 0.70 && mixed <= 1.0);
    assert!(carry > 0.35 && carry <= 1.0);
    assert!((carry_context - carry * 65.0).abs() < 1e-9);
}

#[test]
fn jpy_carry_proxy_requires_change_or_external_confirmation() {
    let mut features = BTreeMap::new();
    features.insert(FEATURE_EXTERNAL_DIMENSION_SCORE.to_string(), 40.0);
    features.insert(FEATURE_US_FED_FUNDS_LEVEL.to_string(), 5.5);
    features.insert(FEATURE_US_USDJPY_LEVEL.to_string(), 160.0);
    features.insert(FEATURE_US_USDJPY_CHANGE_20D.to_string(), -1.0);

    let carry = resolve_probability_feature_value("family_proxy__jpy_carry", &features).unwrap();

    assert!(carry < 0.20);
}

#[test]
fn mixed_systemic_proxy_requires_chronic_pressure_anchor() {
    let mut features = BTreeMap::new();
    features.insert(FEATURE_OVERALL_SCORE.to_string(), 78.0);
    features.insert(FEATURE_TRIGGER_SCORE.to_string(), 74.0);
    features.insert(FEATURE_EXTERNAL_DIMENSION_SCORE.to_string(), 70.0);
    features.insert(FEATURE_US_VIX_LEVEL.to_string(), 34.0);
    features.insert(FEATURE_US_BAA_10Y_SPREAD_LEVEL.to_string(), 1.1);
    features.insert(FEATURE_US_NFCI_LEVEL.to_string(), 0.0);
    features.insert(FEATURE_US_CURVE_10Y2Y_LEVEL.to_string(), 0.6);
    features.insert(FEATURE_US_USDJPY_CHANGE_20D.to_string(), 0.5);

    let mixed =
        resolve_probability_feature_value("family_proxy__mixed_systemic", &features).unwrap();

    assert!(mixed < 0.20);
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
fn shared_probability_scorer_exposes_feature_contributions() {
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

    let diagnostics = score_logistic_probability_model_with_diagnostics(&model, &features);

    assert_eq!(diagnostics.feature_contributions.len(), 2);
    assert_eq!(
        diagnostics.feature_contributions[0].name,
        FEATURE_OVERALL_SCORE
    );
    assert!((diagnostics.feature_contributions[0].contribution - 0.4).abs() < 1e-12);
    assert_eq!(
        diagnostics.feature_contributions[1].name,
        FEATURE_US_VIX_LEVEL
    );
    assert!((diagnostics.feature_contributions[1].raw_value - 18.0).abs() < 1e-12);
    let contribution_sum = diagnostics
        .feature_contributions
        .iter()
        .map(|item| item.contribution)
        .sum::<f64>();
    assert!((diagnostics.linear_score - (diagnostics.intercept + contribution_sum)).abs() < 1e-12);
    assert!(
        (diagnostics.probability - score_logistic_probability_model(&model, &features)).abs()
            < 1e-12
    );
}

#[test]
fn horizon_bundle_score_without_overlays_matches_base_probability() {
    let horizon = ProbabilityHorizonBundle {
        horizon_days: 60,
        decision_threshold: None,
        threshold_diagnostics: None,
        raw_model: LogisticProbabilityModel {
            intercept: 0.0,
            feature_transform: PROBABILITY_FEATURE_TRANSFORM_IDENTITY_V1.to_string(),
            feature_stats: Vec::new(),
            coefficients: Vec::new(),
        },
        calibration: None,
        evaluation: HorizonEvaluationSummary::default(),
        family_overlays: Vec::new(),
        family_overlay_audits: Vec::new(),
    };
    let features = BTreeMap::new();

    let score = score_probability_horizon_bundle(&horizon, &features);

    assert_eq!(score.raw_probability, 0.5);
    assert_eq!(score.calibrated_probability, 0.5);
    assert_eq!(score.final_probability, 0.5);
    assert!(score.overlay_contributions.is_empty());
}

#[test]
fn horizon_bundle_score_blends_family_overlay_when_gate_is_visible() {
    let base_model = LogisticProbabilityModel {
        intercept: 0.0,
        feature_transform: PROBABILITY_FEATURE_TRANSFORM_FAMILY_CONDITIONAL_V1.to_string(),
        feature_stats: Vec::new(),
        coefficients: Vec::new(),
    };
    let overlay_model = LogisticProbabilityModel {
        intercept: 2.0,
        feature_transform: PROBABILITY_FEATURE_TRANSFORM_FAMILY_CONDITIONAL_V1.to_string(),
        feature_stats: Vec::new(),
        coefficients: Vec::new(),
    };
    let horizon = ProbabilityHorizonBundle {
        horizon_days: 60,
        decision_threshold: None,
        threshold_diagnostics: None,
        raw_model: base_model,
        calibration: None,
        evaluation: HorizonEvaluationSummary::default(),
        family_overlays: vec![ProbabilityFamilyOverlayBundle {
            family_id: "jpy_carry".to_string(),
            gate_feature: "family_proxy__jpy_carry".to_string(),
            gate_threshold: 0.2,
            gate_slope: 8.0,
            blend_weight: 0.4,
            raw_model: overlay_model,
            calibration: None,
            decision_threshold: None,
            evaluation: None,
            note: "test overlay".to_string(),
        }],
        family_overlay_audits: Vec::new(),
    };
    let mut features = BTreeMap::new();
    features.insert(FEATURE_US_USDJPY_LEVEL.to_string(), 160.0);
    features.insert(FEATURE_US_USDJPY_CHANGE_20D.to_string(), -8.0);
    features.insert(FEATURE_US_FED_FUNDS_LEVEL.to_string(), 5.5);
    features.insert(FEATURE_EXTERNAL_DIMENSION_SCORE.to_string(), 65.0);

    let score = score_probability_horizon_bundle(&horizon, &features);

    assert!(score.final_probability > score.calibrated_probability);
    assert_eq!(score.overlay_contributions.len(), 1);
    assert_eq!(score.overlay_contributions[0].family_id, "jpy_carry");
    assert!(score.overlay_contributions[0].blend > 0.0);
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

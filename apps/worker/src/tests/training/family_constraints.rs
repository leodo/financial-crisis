use super::*;

#[test]
fn forward_crisis_rate_shock_family_caps_apply_on_20d_only() {
    let feature_names = vec![
        "family_context__rate_shock__external_dimension_score".to_string(),
        "family_proxy__rate_shock".to_string(),
    ];
    let mut weights_20d = vec![0.32, 0.14];
    crate::project_forward_crisis_sign_constraints(
        &mut weights_20d,
        &feature_names,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(weights_20d[0], 0.12);
    assert_eq!(weights_20d[1], 0.06);

    let mut weights_60d = vec![0.32, 0.14];
    crate::project_forward_crisis_sign_constraints(
        &mut weights_60d,
        &feature_names,
        60,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(weights_60d[0], 0.32);
    assert_eq!(weights_60d[1], 0.14);
}

#[test]
fn forward_crisis_jpy_carry_caps_apply_on_20d_only() {
    let feature_names = vec![
        "family_context__jpy_carry__external_dimension_score".to_string(),
        "family_proxy__jpy_carry".to_string(),
    ];
    let mut weights_20d = vec![0.24, 0.11];
    crate::project_forward_crisis_sign_constraints(
        &mut weights_20d,
        &feature_names,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(weights_20d[0], 0.10);
    assert_eq!(weights_20d[1], 0.06);

    let mut weights_60d = vec![0.24, 0.11];
    crate::project_forward_crisis_sign_constraints(
        &mut weights_60d,
        &feature_names,
        60,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(weights_60d[0], 0.24);
    assert_eq!(weights_60d[1], 0.11);
}

#[test]
fn forward_crisis_curve_family_caps_only_apply_when_family_context_exists() {
    let family_feature_names = vec![
        "us_curve_10y2y_level".to_string(),
        "interaction__us_curve_10y2y_level__us_fed_funds_level".to_string(),
        "family_proxy__rate_shock".to_string(),
    ];
    let mut family_weights_20d = vec![-0.90, 0.60, 0.05];
    crate::project_forward_crisis_sign_constraints(
        &mut family_weights_20d,
        &family_feature_names,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(family_weights_20d[0], -0.72);
    assert_eq!(family_weights_20d[1], 0.46);
    assert_eq!(family_weights_20d[2], 0.05);

    let mut family_weights_60d = vec![-0.90, 0.60, 0.05];
    crate::project_forward_crisis_sign_constraints(
        &mut family_weights_60d,
        &family_feature_names,
        60,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(family_weights_60d[0], -0.90);
    assert_eq!(family_weights_60d[1], 0.60);
    assert_eq!(family_weights_60d[2], 0.05);

    let plain_feature_names = vec![
        "us_curve_10y2y_level".to_string(),
        "interaction__us_curve_10y2y_level__us_fed_funds_level".to_string(),
    ];
    let mut plain_weights_20d = vec![-0.90, 0.60];
    crate::project_forward_crisis_sign_constraints(
        &mut plain_weights_20d,
        &plain_feature_names,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(plain_weights_20d[0], -0.90);
    assert_eq!(plain_weights_20d[1], 0.60);
}

#[test]
fn forward_crisis_usdjpy_level_family_cap_only_applies_when_family_context_exists() {
    let family_feature_names = vec![
        "us_usdjpy_level".to_string(),
        "family_proxy__rate_shock".to_string(),
    ];
    let mut family_weights_20d = vec![0.20, 0.05];
    crate::project_forward_crisis_sign_constraints(
        &mut family_weights_20d,
        &family_feature_names,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(family_weights_20d[0], 0.30);
    assert_eq!(family_weights_20d[1], 0.05);

    let mut family_weights_60d = vec![0.20, 0.05];
    crate::project_forward_crisis_sign_constraints(
        &mut family_weights_60d,
        &family_feature_names,
        60,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(family_weights_60d[0], 0.20);
    assert_eq!(family_weights_60d[1], 0.05);

    let plain_feature_names = vec!["us_usdjpy_level".to_string()];
    let mut plain_weights_20d = vec![0.20];
    crate::project_forward_crisis_sign_constraints(
        &mut plain_weights_20d,
        &plain_feature_names,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(plain_weights_20d[0], 0.20);
}

#[test]
fn forward_crisis_usdjpy_tail_cap_keeps_high_level_tail_auxiliary() {
    let family_feature_names = vec![
        "tail_pos__us_usdjpy_level__145".to_string(),
        "family_proxy__jpy_carry".to_string(),
    ];
    let mut negative_tail_weights_5d = vec![-1.20, 0.03];
    crate::project_forward_crisis_sign_constraints(
        &mut negative_tail_weights_5d,
        &family_feature_names,
        5,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(negative_tail_weights_5d[0], 0.0);
    assert_eq!(negative_tail_weights_5d[1], 0.03);

    let mut excessive_tail_weights_5d = vec![0.40, 0.03];
    crate::project_forward_crisis_sign_constraints(
        &mut excessive_tail_weights_5d,
        &family_feature_names,
        5,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(excessive_tail_weights_5d[0], 0.12);
    assert_eq!(excessive_tail_weights_5d[1], 0.03);

    let mut negative_tail_weights = vec![-1.20, 0.03];
    crate::project_forward_crisis_sign_constraints(
        &mut negative_tail_weights,
        &family_feature_names,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(negative_tail_weights[0], 0.0);
    assert_eq!(negative_tail_weights[1], 0.03);

    let mut excessive_tail_weights = vec![0.40, 0.03];
    crate::project_forward_crisis_sign_constraints(
        &mut excessive_tail_weights,
        &family_feature_names,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(excessive_tail_weights[0], 0.18);
    assert_eq!(excessive_tail_weights[1], 0.03);

    let mut negative_tail_weights_60d = vec![-1.20, 0.03];
    crate::project_forward_crisis_sign_constraints(
        &mut negative_tail_weights_60d,
        &family_feature_names,
        60,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(negative_tail_weights_60d[0], 0.0);
    assert_eq!(negative_tail_weights_60d[1], 0.03);

    let mut excessive_tail_weights_60d = vec![0.40, 0.03];
    crate::project_forward_crisis_sign_constraints(
        &mut excessive_tail_weights_60d,
        &family_feature_names,
        60,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(excessive_tail_weights_60d[0], 0.18);
    assert_eq!(excessive_tail_weights_60d[1], 0.03);

    let plain_feature_names = vec!["tail_pos__us_usdjpy_level__145".to_string()];
    let mut plain_weights_20d = vec![-1.20];
    crate::project_forward_crisis_sign_constraints(
        &mut plain_weights_20d,
        &plain_feature_names,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(plain_weights_20d[0], 0.0);

    let mut excessive_plain_weights_60d = vec![0.40];
    crate::project_forward_crisis_sign_constraints(
        &mut excessive_plain_weights_60d,
        &plain_feature_names,
        60,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(excessive_plain_weights_60d[0], 0.18);
}

#[test]
fn forward_crisis_usdjpy_signed_change_is_neutralized_across_horizons() {
    let feature_names = vec![
        "us_usdjpy_change_20d".to_string(),
        "interaction__trigger_score__us_usdjpy_change_20d".to_string(),
        "tail_abs_pos__us_usdjpy_change_20d__4".to_string(),
    ];

    for horizon_days in [5, 20, 60] {
        let mut weights = vec![-0.80, 0.40, 0.35];
        crate::project_forward_crisis_sign_constraints(
            &mut weights,
            &feature_names,
            horizon_days,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );

        assert_eq!(weights[0], 0.0);
        assert_eq!(weights[1], 0.0);
        assert_eq!(weights[2], 0.22);
    }
}

#[test]
fn forward_crisis_usdjpy_interaction_family_cap_only_applies_when_family_context_exists() {
    let family_feature_names = vec![
        "interaction__external_dimension_score__us_usdjpy_level".to_string(),
        "family_proxy__jpy_carry".to_string(),
    ];
    let mut family_weights_20d = vec![0.72, 0.03];
    crate::project_forward_crisis_sign_constraints(
        &mut family_weights_20d,
        &family_feature_names,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(family_weights_20d[0], 0.58);
    assert_eq!(family_weights_20d[1], 0.03);

    let mut family_weights_60d = vec![0.72, 0.03];
    crate::project_forward_crisis_sign_constraints(
        &mut family_weights_60d,
        &family_feature_names,
        60,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(family_weights_60d[0], 0.72);
    assert_eq!(family_weights_60d[1], 0.03);

    let plain_feature_names =
        vec!["interaction__external_dimension_score__us_usdjpy_level".to_string()];
    let mut plain_weights_20d = vec![0.72];
    crate::project_forward_crisis_sign_constraints(
        &mut plain_weights_20d,
        &plain_feature_names,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(plain_weights_20d[0], 0.72);
}

#[test]
fn forward_crisis_rate_shock_family_cap_gradient_pushes_excess_weight_down() {
    let feature_names = vec![
        "family_context__rate_shock__external_dimension_score".to_string(),
        "family_proxy__rate_shock".to_string(),
    ];
    let weights = vec![0.30, 0.12];
    let mut gradients = vec![0.0; weights.len()];

    crate::apply_forward_crisis_coefficient_bound_gradient(
        &mut gradients,
        &weights,
        &feature_names,
        100.0,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );

    assert!(gradients[0] > 0.0);
    assert!(gradients[1] > 0.0);
}

#[test]
fn forward_crisis_jpy_carry_family_cap_gradient_pushes_excess_weight_down() {
    let feature_names = vec![
        "family_context__jpy_carry__external_dimension_score".to_string(),
        "family_proxy__jpy_carry".to_string(),
    ];
    let weights = vec![0.22, 0.09];
    let mut gradients = vec![0.0; weights.len()];

    crate::apply_forward_crisis_coefficient_bound_gradient(
        &mut gradients,
        &weights,
        &feature_names,
        100.0,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );

    assert!(gradients[0] > 0.0);
    assert!(gradients[1] > 0.0);
}

#[test]
fn forward_crisis_monotonic_interaction_sign_gradient_pushes_wrong_direction_up() {
    let feature_names = vec![
        "interaction__overall_score__us_vix_level".to_string(),
        "interaction__us_baa_10y_spread_level__us_vix_level".to_string(),
    ];
    let weights = vec![-0.20, -0.60];
    let mut gradients = vec![0.0; weights.len()];

    crate::apply_forward_crisis_sign_gradient(
        &mut gradients,
        &weights,
        &feature_names,
        100.0,
        60,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );

    assert!(gradients[0] < 0.0);
    assert!(gradients[1] < 0.0);
}

#[test]
fn forward_crisis_curve_family_cap_gradient_only_activates_for_family_context_sets() {
    let family_feature_names = vec![
        "us_curve_10y2y_level".to_string(),
        "interaction__us_curve_10y2y_level__us_fed_funds_level".to_string(),
        "family_proxy__rate_shock".to_string(),
    ];
    let family_weights = vec![-0.90, 0.60, 0.05];
    let mut family_gradients = vec![0.0; family_weights.len()];

    crate::apply_forward_crisis_coefficient_bound_gradient(
        &mut family_gradients,
        &family_weights,
        &family_feature_names,
        100.0,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );

    assert!(family_gradients[0] < 0.0);
    assert!(family_gradients[1] > 0.0);
    assert_eq!(family_gradients[2], 0.0);

    let plain_feature_names = vec![
        "us_curve_10y2y_level".to_string(),
        "interaction__us_curve_10y2y_level__us_fed_funds_level".to_string(),
    ];
    let plain_weights = vec![-0.90, 0.60];
    let mut plain_gradients = vec![0.0; plain_weights.len()];

    crate::apply_forward_crisis_coefficient_bound_gradient(
        &mut plain_gradients,
        &plain_weights,
        &plain_feature_names,
        100.0,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );

    assert_eq!(plain_gradients[0], 0.0);
    assert_eq!(plain_gradients[1], 0.0);
}

#[test]
fn forward_crisis_usdjpy_level_family_cap_gradient_only_activates_for_family_context_sets() {
    let family_feature_names = vec![
        "us_usdjpy_level".to_string(),
        "family_proxy__rate_shock".to_string(),
    ];
    let family_weights = vec![0.48, 0.05];
    let mut family_gradients = vec![0.0; family_weights.len()];

    crate::apply_forward_crisis_coefficient_bound_gradient(
        &mut family_gradients,
        &family_weights,
        &family_feature_names,
        100.0,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );

    assert!(family_gradients[0] > 0.0);
    assert_eq!(family_gradients[1], 0.0);

    let plain_feature_names = vec!["us_usdjpy_level".to_string()];
    let plain_weights = vec![0.38];
    let mut plain_gradients = vec![0.0; plain_weights.len()];

    crate::apply_forward_crisis_coefficient_bound_gradient(
        &mut plain_gradients,
        &plain_weights,
        &plain_feature_names,
        100.0,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );

    assert_eq!(plain_gradients[0], 0.0);
}

#[test]
fn forward_crisis_usdjpy_tail_cap_gradient_applies_across_feature_sets() {
    let family_feature_names = vec![
        "tail_pos__us_usdjpy_level__145".to_string(),
        "family_proxy__jpy_carry".to_string(),
    ];
    let family_weights = vec![-1.20, 0.03];
    let mut family_gradients = vec![0.0; family_weights.len()];

    crate::apply_forward_crisis_coefficient_bound_gradient(
        &mut family_gradients,
        &family_weights,
        &family_feature_names,
        100.0,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );

    assert!(family_gradients[0] < 0.0);
    assert_eq!(family_gradients[1], 0.0);

    let family_weights_5d = vec![-1.20, 0.03];
    let mut family_gradients_5d = vec![0.0; family_weights_5d.len()];

    crate::apply_forward_crisis_coefficient_bound_gradient(
        &mut family_gradients_5d,
        &family_weights_5d,
        &family_feature_names,
        100.0,
        5,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );

    assert!(family_gradients_5d[0] < 0.0);
    assert_eq!(family_gradients_5d[1], 0.0);

    let family_weights_60d = vec![-1.20, 0.03];
    let mut family_gradients_60d = vec![0.0; family_weights_60d.len()];

    crate::apply_forward_crisis_coefficient_bound_gradient(
        &mut family_gradients_60d,
        &family_weights_60d,
        &family_feature_names,
        100.0,
        60,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );

    assert!(family_gradients_60d[0] < 0.0);
    assert_eq!(family_gradients_60d[1], 0.0);

    let plain_feature_names = vec!["tail_pos__us_usdjpy_level__145".to_string()];
    let plain_weights = vec![-1.20];
    let mut plain_gradients = vec![0.0; plain_weights.len()];

    crate::apply_forward_crisis_coefficient_bound_gradient(
        &mut plain_gradients,
        &plain_weights,
        &plain_feature_names,
        100.0,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );

    assert!(plain_gradients[0] < 0.0);
}

#[test]
fn forward_crisis_usdjpy_change_bounds_gradient_pushes_signed_features_to_neutral() {
    let feature_names = vec![
        "us_usdjpy_change_20d".to_string(),
        "interaction__trigger_score__us_usdjpy_change_20d".to_string(),
        "tail_abs_pos__us_usdjpy_change_20d__4".to_string(),
        "tail_abs_pos__us_usdjpy_change_20d__4".to_string(),
    ];
    let weights = vec![-0.50, 0.30, -0.10, 0.35];
    let mut gradients = vec![0.0; weights.len()];

    crate::apply_forward_crisis_coefficient_bound_gradient(
        &mut gradients,
        &weights,
        &feature_names,
        100.0,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );

    assert!(gradients[0] < 0.0);
    assert!(gradients[1] > 0.0);
    assert!(gradients[2] < 0.0);
    assert!(gradients[3] > 0.0);
}

#[test]
fn forward_crisis_usdjpy_interaction_family_cap_gradient_only_activates_for_family_context_sets() {
    let family_feature_names = vec![
        "interaction__external_dimension_score__us_usdjpy_level".to_string(),
        "family_proxy__jpy_carry".to_string(),
    ];
    let family_weights = vec![0.72, 0.03];
    let mut family_gradients = vec![0.0; family_weights.len()];

    crate::apply_forward_crisis_coefficient_bound_gradient(
        &mut family_gradients,
        &family_weights,
        &family_feature_names,
        100.0,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );

    assert!(family_gradients[0] > 0.0);
    assert_eq!(family_gradients[1], 0.0);

    let plain_feature_names =
        vec!["interaction__external_dimension_score__us_usdjpy_level".to_string()];
    let plain_weights = vec![0.72];
    let mut plain_gradients = vec![0.0; plain_weights.len()];

    crate::apply_forward_crisis_coefficient_bound_gradient(
        &mut plain_gradients,
        &plain_weights,
        &plain_feature_names,
        100.0,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );

    assert_eq!(plain_gradients[0], 0.0);
}

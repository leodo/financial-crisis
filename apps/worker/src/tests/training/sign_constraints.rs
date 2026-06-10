use super::*;

#[test]
fn forward_crisis_sign_gradient_pushes_wrong_direction_coefficients_toward_zero() {
    let feature_names = vec![
        "us_baa_10y_spread_level".to_string(),
        "us_curve_10y2y_level".to_string(),
        "us_stlfsi_level".to_string(),
        "tail_neg__us_curve_10y2y_level__0".to_string(),
        "tail_pos__us_baa_10y_spread_level__2".to_string(),
    ];
    let weights = vec![-0.8, 0.5, -0.4, -0.6, -0.3];
    let mut gradients = vec![0.0; weights.len()];

    crate::apply_forward_crisis_sign_gradient(
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
    assert_eq!(gradients[3], 0.0);
    assert!(gradients[4] < 0.0);
}

#[test]
fn forward_crisis_sign_projection_clips_wrong_direction_coefficients() {
    let feature_names = vec![
        "us_baa_10y_spread_level".to_string(),
        "us_curve_10y2y_level".to_string(),
        "structural_score".to_string(),
        "us_usdjpy_change_20d".to_string(),
    ];
    let mut weights = vec![-0.8, 0.5, -0.2, -0.7];

    crate::project_forward_crisis_sign_constraints(
        &mut weights,
        &feature_names,
        60,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );

    assert_eq!(weights[0], 0.0);
    assert_eq!(weights[1], 0.0);
    assert_eq!(weights[2], 0.0);
    assert_eq!(weights[3], 0.0);
}

#[test]
fn forward_crisis_sign_projection_clips_wrong_direction_monotonic_interactions() {
    let feature_names = vec![
        "interaction__overall_score__us_vix_level".to_string(),
        "interaction__us_baa_10y_spread_level__us_vix_level".to_string(),
        "interaction__external_dimension_score__us_usdjpy_level".to_string(),
    ];
    let mut weights = vec![-0.2, -0.6, -0.4];

    crate::project_forward_crisis_sign_constraints(
        &mut weights,
        &feature_names,
        60,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );

    assert_eq!(weights[0], 0.0);
    assert_eq!(weights[1], 0.0);
    assert_eq!(weights[2], 0.0);
}

#[test]
fn forward_crisis_tail_sign_projection_applies_on_20d_only() {
    let feature_names = vec![
        "tail_neg__us_curve_10y2y_level__0".to_string(),
        "tail_pos__us_baa_10y_spread_level__2".to_string(),
    ];
    let mut weights_20d = vec![-0.4, -0.1];
    crate::project_forward_crisis_sign_constraints(
        &mut weights_20d,
        &feature_names,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(weights_20d[0], 0.0);
    assert_eq!(weights_20d[1], 0.0);

    let mut weights_60d = vec![-0.4, -0.1];
    crate::project_forward_crisis_sign_constraints(
        &mut weights_60d,
        &feature_names,
        60,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(weights_60d[0], -0.4);
    assert_eq!(weights_60d[1], -0.1);
}

#[test]
fn forward_crisis_curve_tail_bound_gradient_pushes_too_negative_weight_up() {
    let feature_names = vec!["tail_neg__us_curve_10y2y_level__0".to_string()];
    let weights = vec![-0.30];
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
}

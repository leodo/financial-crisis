use super::*;

#[test]
fn forward_crisis_pairwise_targets_push_buffer_centroid_above_normal() {
    let make_row = |feature_value: f64, regime_20d: ProbabilityTrainingRegime, label_20d: u8| {
        let mut features = BTreeMap::new();
        features.insert("stress".to_string(), feature_value);
        ProbabilityTrainingRow {
            as_of_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
            market_scope: "financial_system".to_string(),
            release_id: None,
            probability_mode: Some("formal_bundle_v1".to_string()),
            freshness_status: Some("a".to_string()),
            time_to_risk_bucket: Some("weeks".to_string()),
            split_name: Some("train".to_string()),
            features,
            primary_scenario_id: Some("scenario".to_string()),
            scenario_family: Some("systemic_credit_banking_crisis".to_string()),
            scenario_training_role: None,
            days_to_primary_crisis_start: Some(10),
            primary_scenario_supports_5d: false,
            primary_scenario_supports_20d: true,
            primary_scenario_supports_60d: true,
            label_5d: 0,
            label_20d,
            label_60d: 0,
            regime_5d: ProbabilityTrainingRegime::Normal,
            regime_20d,
            regime_60d: ProbabilityTrainingRegime::Normal,
            action_label_5d: 0,
            action_label_20d: label_20d,
            action_label_60d: 0,
            prepare_episode_label: 0,
            hedge_episode_label: 0,
            defend_episode_label: 0,
            primary_action_level: None,
            action_episode_id: None,
            action_episode_phase: "outside".to_string(),
            protected_action_window: false,
        }
    };

    let rows = vec![
        make_row(0.0, ProbabilityTrainingRegime::Normal, 0),
        make_row(0.1, ProbabilityTrainingRegime::Normal, 0),
        make_row(0.8, ProbabilityTrainingRegime::PreWarningBuffer, 0),
        make_row(0.9, ProbabilityTrainingRegime::PreWarningBuffer, 0),
        make_row(1.2, ProbabilityTrainingRegime::PositiveWindow, 1),
        make_row(1.1, ProbabilityTrainingRegime::PositiveWindow, 1),
        make_row(1.0, ProbabilityTrainingRegime::PostCrisisCooldown, 0),
    ];
    let feature_stats = vec![crate::build_feature_stat(&rows, "stress")];
    let targets = forward_crisis_regime_pairwise_targets(
        &rows,
        &feature_stats,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
        false,
    );
    assert!(!targets.is_empty());
    assert!(targets.len() >= 5);
    let mut gradients = vec![0.0];
    crate::apply_regime_pairwise_gradient(&mut gradients, &[0.0], &targets, 100.0, 20, false);
    assert!(gradients[0] < 0.0);
}

#[test]
fn forward_crisis_20d_pairwise_prioritizes_positive_window_over_buffer() {
    let make_row = |feature_value: f64, regime_20d: ProbabilityTrainingRegime, label_20d: u8| {
        let mut features = BTreeMap::new();
        features.insert("stress".to_string(), feature_value);
        ProbabilityTrainingRow {
            as_of_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
            market_scope: "financial_system".to_string(),
            release_id: None,
            probability_mode: Some("formal_bundle_v1".to_string()),
            freshness_status: Some("a".to_string()),
            time_to_risk_bucket: Some("weeks".to_string()),
            split_name: Some("train".to_string()),
            features,
            primary_scenario_id: Some("scenario".to_string()),
            scenario_family: Some("systemic_credit_banking_crisis".to_string()),
            scenario_training_role: None,
            days_to_primary_crisis_start: Some(10),
            primary_scenario_supports_5d: false,
            primary_scenario_supports_20d: true,
            primary_scenario_supports_60d: true,
            label_5d: 0,
            label_20d,
            label_60d: 0,
            regime_5d: ProbabilityTrainingRegime::Normal,
            regime_20d,
            regime_60d: ProbabilityTrainingRegime::Normal,
            action_label_5d: 0,
            action_label_20d: label_20d,
            action_label_60d: 0,
            prepare_episode_label: 0,
            hedge_episode_label: 0,
            defend_episode_label: 0,
            primary_action_level: None,
            action_episode_id: None,
            action_episode_phase: "outside".to_string(),
            protected_action_window: false,
        }
    };

    let rows = vec![
        make_row(0.0, ProbabilityTrainingRegime::Normal, 0),
        make_row(0.2, ProbabilityTrainingRegime::Normal, 0),
        make_row(0.6, ProbabilityTrainingRegime::PreWarningBuffer, 0),
        make_row(0.7, ProbabilityTrainingRegime::PreWarningBuffer, 0),
        make_row(0.9, ProbabilityTrainingRegime::PostCrisisCooldown, 0),
        make_row(1.1, ProbabilityTrainingRegime::PostCrisisCooldown, 0),
        make_row(1.3, ProbabilityTrainingRegime::PositiveWindow, 1),
        make_row(1.4, ProbabilityTrainingRegime::PositiveWindow, 1),
    ];
    let feature_stats = vec![crate::build_feature_stat(&rows, "stress")];
    let targets = forward_crisis_regime_pairwise_targets(
        &rows,
        &feature_stats,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
        true,
    );

    let positive_vs_normal = targets
        .iter()
        .find(|target| {
            target.left_regime == ProbabilityTrainingRegime::PositiveWindow
                && target.right_regime == ProbabilityTrainingRegime::Normal
        })
        .expect("positive_window vs normal target");
    let positive_vs_cooldown = targets
        .iter()
        .find(|target| {
            target.left_regime == ProbabilityTrainingRegime::PositiveWindow
                && target.right_regime == ProbabilityTrainingRegime::PostCrisisCooldown
        })
        .expect("positive_window vs cooldown target");
    let buffer_vs_normal = targets
        .iter()
        .find(|target| {
            target.left_regime == ProbabilityTrainingRegime::PreWarningBuffer
                && target.right_regime == ProbabilityTrainingRegime::Normal
        })
        .expect("buffer vs normal target");

    assert!(positive_vs_normal.margin > buffer_vs_normal.margin);
    assert!(positive_vs_normal.weight > buffer_vs_normal.weight);
    assert!(positive_vs_cooldown.margin > positive_vs_normal.margin);
    assert!(positive_vs_cooldown.weight > positive_vs_normal.weight);
}

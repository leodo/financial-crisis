use super::*;

#[test]
fn actionability_summary_distinguishes_advance_late_and_missed_scenarios() {
    let build_row =
        |scenario_id: &str, as_of_date: (i32, u32, u32), lead_days: i64, predicted: bool| {
            let action_label = 1_u8;
            let mut features = BTreeMap::new();
            if predicted {
                features.insert("predicted".to_string(), 1.0);
            }
            ProbabilityTrainingRow {
                as_of_date: NaiveDate::from_ymd_opt(as_of_date.0, as_of_date.1, as_of_date.2)
                    .unwrap(),
                market_scope: "financial_system".to_string(),
                release_id: None,
                probability_mode: Some("formal_bundle_v1".to_string()),
                freshness_status: Some("a".to_string()),
                time_to_risk_bucket: Some("weeks".to_string()),
                split_name: Some("evaluation".to_string()),
                features,
                primary_scenario_id: Some(scenario_id.to_string()),
                scenario_family: Some("systemic_credit_banking_crisis".to_string()),
                scenario_training_role: None,
                days_to_primary_crisis_start: Some(lead_days),
                primary_scenario_supports_5d: true,
                primary_scenario_supports_20d: true,
                primary_scenario_supports_60d: true,
                label_5d: 0,
                label_20d: 0,
                label_60d: 0,
                regime_5d: ProbabilityTrainingRegime::Normal,
                regime_20d: if lead_days > 0 {
                    ProbabilityTrainingRegime::PositiveWindow
                } else {
                    ProbabilityTrainingRegime::InCrisis
                },
                regime_60d: ProbabilityTrainingRegime::Normal,
                action_label_5d: 0,
                action_label_20d: action_label,
                action_label_60d: 0,
                prepare_episode_label: 0,
                hedge_episode_label: u8::from(lead_days > 0),
                defend_episode_label: 0,
                primary_action_level: (lead_days > 0).then_some("hedge".to_string()),
                action_episode_id: Some(format!("{scenario_id}:hedge")),
                action_episode_phase: if lead_days > 0 {
                    "primary".to_string()
                } else {
                    "late_validation".to_string()
                },
                protected_action_window: false,
            }
        };

    let false_positive_row = ProbabilityTrainingRow {
        as_of_date: NaiveDate::from_ymd_opt(2024, 1, 10).unwrap(),
        market_scope: "financial_system".to_string(),
        release_id: None,
        probability_mode: Some("formal_bundle_v1".to_string()),
        freshness_status: Some("a".to_string()),
        time_to_risk_bucket: Some("normal".to_string()),
        split_name: Some("evaluation".to_string()),
        features: BTreeMap::new(),
        primary_scenario_id: None,
        scenario_family: None,
        scenario_training_role: None,
        days_to_primary_crisis_start: None,
        primary_scenario_supports_5d: false,
        primary_scenario_supports_20d: false,
        primary_scenario_supports_60d: false,
        label_5d: 0,
        label_20d: 0,
        label_60d: 0,
        regime_5d: ProbabilityTrainingRegime::Normal,
        regime_20d: ProbabilityTrainingRegime::Normal,
        regime_60d: ProbabilityTrainingRegime::Normal,
        action_label_5d: 0,
        action_label_20d: 0,
        action_label_60d: 0,
        prepare_episode_label: 0,
        hedge_episode_label: 0,
        defend_episode_label: 0,
        primary_action_level: None,
        action_episode_id: None,
        action_episode_phase: "outside".to_string(),
        protected_action_window: false,
    };

    let rows = vec![
        build_row("scenario_a", (2007, 8, 20), 10, true),
        build_row("scenario_a", (2007, 9, 5), -2, true),
        build_row("scenario_b", (2020, 2, 20), 8, false),
        build_row("scenario_b", (2020, 3, 18), -3, true),
        build_row("scenario_c", (2011, 7, 20), 6, false),
        build_row("scenario_c", (2011, 8, 10), -1, false),
        false_positive_row,
    ];
    let probabilities = vec![0.82, 0.61, 0.12, 0.42, 0.18, 0.07, 0.77];

    let summary = evaluate_actionability_summary(&probabilities, &rows, 20, 0.3);

    assert_eq!(summary.predicted_positive_count, 4);
    assert_eq!(summary.actual_positive_count, 3);
    assert_eq!(summary.pre_start_positive_count, 3);
    assert_eq!(summary.post_start_positive_count, 3);
    assert_eq!(summary.pre_start_hit_count, 1);
    assert_eq!(summary.post_start_hit_count, 2);
    assert_eq!(summary.false_positive_count, 1);
    assert_eq!(summary.scenario_count, 3);
    assert_eq!(summary.advance_warning_scenario_count, 1);
    assert_eq!(summary.late_confirmation_scenario_count, 1);
    assert_eq!(summary.missed_scenario_count, 1);
    assert_eq!(summary.precision_at_threshold, Some(0.75));
    assert_eq!(
        summary.pre_start_recall_at_threshold,
        Some(round3(1.0 / 3.0))
    );
    assert_eq!(
        summary.post_start_recall_at_threshold,
        Some(round3(2.0 / 3.0))
    );
    assert_eq!(summary.advance_warning_rate, Some(round3(1.0 / 3.0)));
    assert_eq!(summary.late_confirmation_rate, Some(round3(1.0 / 3.0)));
    assert_eq!(summary.missed_rate, Some(round3(1.0 / 3.0)));
}

#[test]
fn actionability_threshold_selection_avoids_zero_hit_fixed_cutoff() {
    let build_row = |scenario_id: &str, lead_days: i64| ProbabilityTrainingRow {
        as_of_date: NaiveDate::from_ymd_opt(2020, 3, 1).unwrap(),
        market_scope: "financial_system".to_string(),
        release_id: None,
        probability_mode: Some("formal_bundle_v1".to_string()),
        freshness_status: Some("a".to_string()),
        time_to_risk_bucket: Some("evaluation".to_string()),
        split_name: Some("calibration".to_string()),
        features: BTreeMap::new(),
        primary_scenario_id: Some(scenario_id.to_string()),
        scenario_family: Some("systemic_credit_banking_crisis".to_string()),
        scenario_training_role: None,
        days_to_primary_crisis_start: Some(lead_days),
        primary_scenario_supports_5d: true,
        primary_scenario_supports_20d: true,
        primary_scenario_supports_60d: true,
        label_5d: 0,
        label_20d: 0,
        label_60d: 0,
        regime_5d: ProbabilityTrainingRegime::Normal,
        regime_20d: ProbabilityTrainingRegime::PreWarningBuffer,
        regime_60d: ProbabilityTrainingRegime::Normal,
        action_label_5d: 0,
        action_label_20d: 1,
        action_label_60d: 0,
        prepare_episode_label: 0,
        hedge_episode_label: u8::from(lead_days > 0),
        defend_episode_label: 0,
        primary_action_level: (lead_days > 0).then_some("hedge".to_string()),
        action_episode_id: Some(format!("{scenario_id}:hedge")),
        action_episode_phase: if lead_days > 0 {
            "primary".to_string()
        } else {
            "late_validation".to_string()
        },
        protected_action_window: false,
    };
    let rows = vec![
        build_row("scenario_a", 8),
        build_row("scenario_a", -2),
        build_row("scenario_b", 10),
        build_row("scenario_b", -1),
    ];
    let probabilities = vec![0.24, 0.18, 0.22, 0.07];

    let threshold = select_actionability_decision_threshold(&probabilities, &rows, 20);
    let summary = evaluate_actionability_summary(&probabilities, &rows, 20, threshold);

    assert!(threshold < 0.3);
    assert!(summary.predicted_positive_count > 0);
    assert!(summary.advance_warning_scenario_count + summary.late_confirmation_scenario_count > 0);
}

#[test]
fn actionability_threshold_selection_raises_cutoff_when_low_threshold_is_overbroad() {
    let build_positive_row = |scenario_id: &str, lead_days: i64| ProbabilityTrainingRow {
        as_of_date: NaiveDate::from_ymd_opt(2020, 3, 1).unwrap(),
        market_scope: "financial_system".to_string(),
        release_id: None,
        probability_mode: Some("formal_bundle_v1".to_string()),
        freshness_status: Some("a".to_string()),
        time_to_risk_bucket: Some("calibration".to_string()),
        split_name: Some("calibration".to_string()),
        features: BTreeMap::new(),
        primary_scenario_id: Some(scenario_id.to_string()),
        scenario_family: Some("systemic_credit_banking_crisis".to_string()),
        scenario_training_role: None,
        days_to_primary_crisis_start: Some(lead_days),
        primary_scenario_supports_5d: true,
        primary_scenario_supports_20d: true,
        primary_scenario_supports_60d: true,
        label_5d: 0,
        label_20d: 0,
        label_60d: 0,
        regime_5d: ProbabilityTrainingRegime::Normal,
        regime_20d: ProbabilityTrainingRegime::PreWarningBuffer,
        regime_60d: ProbabilityTrainingRegime::Normal,
        action_label_5d: 0,
        action_label_20d: 1,
        action_label_60d: 0,
        prepare_episode_label: 0,
        hedge_episode_label: u8::from(lead_days > 0),
        defend_episode_label: 0,
        primary_action_level: (lead_days > 0).then_some("hedge".to_string()),
        action_episode_id: Some(format!("{scenario_id}:hedge")),
        action_episode_phase: if lead_days > 0 {
            "primary".to_string()
        } else {
            "late_validation".to_string()
        },
        protected_action_window: false,
    };
    let build_false_positive_row = |day: u32| ProbabilityTrainingRow {
        as_of_date: NaiveDate::from_ymd_opt(2020, 4, day).unwrap(),
        market_scope: "financial_system".to_string(),
        release_id: None,
        probability_mode: Some("formal_bundle_v1".to_string()),
        freshness_status: Some("a".to_string()),
        time_to_risk_bucket: Some("normal".to_string()),
        split_name: Some("calibration".to_string()),
        features: BTreeMap::new(),
        primary_scenario_id: None,
        scenario_family: None,
        scenario_training_role: None,
        days_to_primary_crisis_start: None,
        primary_scenario_supports_5d: false,
        primary_scenario_supports_20d: false,
        primary_scenario_supports_60d: false,
        label_5d: 0,
        label_20d: 0,
        label_60d: 0,
        regime_5d: ProbabilityTrainingRegime::Normal,
        regime_20d: ProbabilityTrainingRegime::Normal,
        regime_60d: ProbabilityTrainingRegime::Normal,
        action_label_5d: 0,
        action_label_20d: 0,
        action_label_60d: 0,
        prepare_episode_label: 0,
        hedge_episode_label: 0,
        defend_episode_label: 0,
        primary_action_level: None,
        action_episode_id: None,
        action_episode_phase: "outside".to_string(),
        protected_action_window: false,
    };

    let mut rows = vec![
        build_positive_row("scenario_a", 8),
        build_positive_row("scenario_b", 10),
    ];
    rows.extend((1..=20).map(build_false_positive_row));

    let mut probabilities = vec![0.82, 0.18];
    probabilities.extend(std::iter::repeat_n(0.18, 20));

    let threshold = select_actionability_decision_threshold(&probabilities, &rows, 20);
    let summary = evaluate_actionability_summary(&probabilities, &rows, 20, threshold);

    assert!(threshold > 0.18);
    assert_eq!(summary.false_positive_count, 0);
    assert_eq!(summary.advance_warning_scenario_count, 1);
}

#[test]
fn actionability_bundle_quality_gate_rejects_overbroad_low_precision_levels() {
    let bundle = ActionabilityBundle {
        model_version: "actionability_bundle_test".to_string(),
        calibration_version: "actionability_platt_test".to_string(),
        fusion_policy_version: "fusion_policy_test".to_string(),
        note: "test".to_string(),
        levels: vec![ActionabilityLevelBundle {
            level: ActionabilityLevel::Prepare,
            proxy_horizon_days: 60,
            target_label_mode: "action_window".to_string(),
            decision_threshold: 0.05,
            raw_model: LogisticProbabilityModel {
                intercept: 0.0,
                feature_transform: PROBABILITY_FEATURE_TRANSFORM_IDENTITY_V1.to_string(),
                feature_stats: Vec::new(),
                coefficients: Vec::new(),
            },
            calibration: None,
            evaluation: HorizonEvaluationSummary {
                sample_count: 2269,
                positive_rate: 0.033,
                brier_score: 0.0,
                log_loss: 0.0,
                ece: 0.0,
                precision_at_30pct: None,
                recall_at_30pct: None,
                regime_separation: None,
                actionability: Some(ActionabilityEvaluationSummary {
                    threshold: 0.05,
                    predicted_positive_count: 1751,
                    actual_positive_count: 77,
                    advance_warning_scenario_count: 1,
                    precision_at_threshold: Some(0.038),
                    ..Default::default()
                }),
            },
        }],
    };

    let regressions = actionability_bundle_quality_regressions(&bundle);

    assert!(!regressions.is_empty());
    assert!(regressions
        .iter()
        .any(|item| item.contains("precision") || item.contains("predicted positives")));
}

#[test]
fn actionability_calibration_strategy_rejects_inverting_fit() {
    let build_row = |scenario_id: &str, lead_days: i64| ProbabilityTrainingRow {
        as_of_date: NaiveDate::from_ymd_opt(2020, 3, 1).unwrap(),
        market_scope: "financial_system".to_string(),
        release_id: None,
        probability_mode: Some("formal_bundle_v1".to_string()),
        freshness_status: Some("a".to_string()),
        time_to_risk_bucket: Some("calibration".to_string()),
        split_name: Some("calibration".to_string()),
        features: BTreeMap::new(),
        primary_scenario_id: Some(scenario_id.to_string()),
        scenario_family: Some("systemic_credit_banking_crisis".to_string()),
        scenario_training_role: None,
        days_to_primary_crisis_start: Some(lead_days),
        primary_scenario_supports_5d: true,
        primary_scenario_supports_20d: true,
        primary_scenario_supports_60d: true,
        label_5d: 0,
        label_20d: 0,
        label_60d: 0,
        regime_5d: ProbabilityTrainingRegime::Normal,
        regime_20d: ProbabilityTrainingRegime::PreWarningBuffer,
        regime_60d: ProbabilityTrainingRegime::Normal,
        action_label_5d: 0,
        action_label_20d: 1,
        action_label_60d: 0,
        prepare_episode_label: 0,
        hedge_episode_label: u8::from(lead_days > 0),
        defend_episode_label: 0,
        primary_action_level: (lead_days > 0).then_some("hedge".to_string()),
        action_episode_id: Some(format!("{scenario_id}:hedge")),
        action_episode_phase: if lead_days > 0 {
            "primary".to_string()
        } else {
            "late_validation".to_string()
        },
        protected_action_window: false,
    };
    let rows = vec![
        build_row("scenario_a", 8),
        build_row("scenario_a", -2),
        build_row("scenario_b", 10),
        build_row("scenario_b", -1),
    ];
    let raw_probabilities = vec![0.31, 0.28, 0.27, 0.24];
    let calibration_candidate = PlattCalibrationArtifact {
        alpha: -1.2,
        beta: -3.5,
        min_input: 0.24,
        max_input: 0.31,
    };

    let (calibration, evaluation_probabilities, threshold) =
        select_actionability_calibration_strategy(
            &raw_probabilities,
            &rows,
            &raw_probabilities,
            20,
            calibration_candidate,
        );

    assert!(calibration.is_none());
    assert_eq!(evaluation_probabilities, raw_probabilities);
    assert!(threshold >= 0.24);
}

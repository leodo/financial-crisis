use super::*;

#[test]
fn render_dataset_csv_includes_scenario_role_columns() {
    let mut row = forward_crisis_row(
        NaiveDate::from_ymd_opt(2020, 2, 20).unwrap(),
        1,
        ProbabilityTrainingRegime::PositiveWindow,
    );
    row.scenario_training_role = Some("mandatory".to_string());
    row.features.insert("stress".to_string(), 0.42);

    let csv = crate::commands::snapshot::render_dataset_csv(&[row], &[String::from("stress")]);
    let mut lines = csv.lines();
    let header = lines.next().unwrap_or_default();
    let first_row = lines.next().unwrap_or_default();

    assert!(header.contains("primary_scenario_id"));
    assert!(header.contains("scenario_family"));
    assert!(header.contains("scenario_training_role"));
    assert!(first_row.contains(",scenario_a,systemic_credit_banking_crisis,mandatory,"));
}

#[test]
fn render_formal_dataset_slice_csv_includes_feature_columns() {
    let mut row = FormalDatasetRowRecord {
        dataset_key: "dataset".to_string(),
        split_name: "evaluation".to_string(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        as_of_date: NaiveDate::from_ymd_opt(2023, 2, 20).unwrap(),
        point_in_time_mode: "best_effort".to_string(),
        latest_visible_at: None,
        coverage_score: 0.92,
        core_feature_coverage: 0.95,
        trigger_feature_coverage: 0.88,
        external_feature_coverage: 0.84,
        sample_quality_grade: "a".to_string(),
        primary_scenario_id: Some("us_regional_banks_2023".to_string()),
        scenario_family: Some("systemic_credit_banking_crisis".to_string()),
        scenario_training_role: Some("mandatory".to_string()),
        label_5d: 0,
        label_20d: 1,
        label_60d: 1,
        regime_5d: "normal".to_string(),
        regime_20d: "pre_warning_buffer".to_string(),
        regime_60d: "positive_window".to_string(),
        action_label_5d: 0,
        action_label_20d: 1,
        action_label_60d: 1,
        prepare_episode_label: 1,
        hedge_episode_label: 1,
        defend_episode_label: 0,
        primary_action_level: Some("hedge".to_string()),
        action_episode_id: Some("us_regional_banks_2023:hedge".to_string()),
        action_episode_phase: "primary".to_string(),
        protected_action_window: false,
        features: BTreeMap::new(),
        created_at: Utc::now(),
    };
    row.features.insert("feature_a".to_string(), 0.42);

    let csv = render_formal_dataset_slice_csv(&[row], &[String::from("feature_a")]);
    let mut lines = csv.lines();
    let header = lines.next().unwrap_or_default();
    let first_row = lines.next().unwrap_or_default();

    assert!(header.contains("primary_scenario_id"));
    assert!(header.contains("feature_a"));
    assert!(first_row.contains("us_regional_banks_2023"));
    assert!(first_row.ends_with(",0.420000"));
}

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

#[test]
fn probability_calibration_strategy_rejects_inverting_fit() {
    let raw_probabilities = vec![0.82, 0.71, 0.24, 0.11];
    let labels = vec![1.0, 1.0, 0.0, 0.0];
    let calibration_rows = vec![
        forward_crisis_row(
            NaiveDate::from_ymd_opt(2020, 1, 5).unwrap(),
            1,
            ProbabilityTrainingRegime::PositiveWindow,
        ),
        forward_crisis_row(
            NaiveDate::from_ymd_opt(2020, 1, 6).unwrap(),
            1,
            ProbabilityTrainingRegime::PositiveWindow,
        ),
        forward_crisis_row(
            NaiveDate::from_ymd_opt(2020, 1, 20).unwrap(),
            0,
            ProbabilityTrainingRegime::Normal,
        ),
        forward_crisis_row(
            NaiveDate::from_ymd_opt(2020, 1, 21).unwrap(),
            0,
            ProbabilityTrainingRegime::Normal,
        ),
    ];
    let calibration_row_refs = calibration_rows.iter().collect::<Vec<_>>();
    let calibration_candidate = PlattCalibrationArtifact {
        alpha: -1.4,
        beta: -3.0,
        min_input: 0.11,
        max_input: 0.82,
    };

    let (calibration, evaluation_probabilities) = select_probability_calibration_strategy(
        &raw_probabilities,
        &labels,
        &calibration_row_refs,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
        &raw_probabilities,
        calibration_candidate,
    );

    assert!(calibration.is_none());
    assert_eq!(evaluation_probabilities, raw_probabilities);
}

#[test]
fn probability_calibration_strategy_keeps_inverting_fit_for_reversed_raw_ranking() {
    let raw_probabilities = vec![0.11, 0.24, 0.71, 0.82];
    let labels = vec![1.0, 1.0, 0.0, 0.0];
    let calibration_rows = vec![
        forward_crisis_row(
            NaiveDate::from_ymd_opt(2020, 1, 5).unwrap(),
            1,
            ProbabilityTrainingRegime::PositiveWindow,
        ),
        forward_crisis_row(
            NaiveDate::from_ymd_opt(2020, 1, 6).unwrap(),
            1,
            ProbabilityTrainingRegime::PositiveWindow,
        ),
        forward_crisis_row(
            NaiveDate::from_ymd_opt(2020, 1, 20).unwrap(),
            0,
            ProbabilityTrainingRegime::Normal,
        ),
        forward_crisis_row(
            NaiveDate::from_ymd_opt(2020, 1, 21).unwrap(),
            0,
            ProbabilityTrainingRegime::Normal,
        ),
    ];
    let calibration_row_refs = calibration_rows.iter().collect::<Vec<_>>();
    let calibration_candidate = fit_platt_calibration(&raw_probabilities, &labels);
    assert!(calibration_candidate.alpha < 0.0);

    let (calibration, evaluation_probabilities) = select_probability_calibration_strategy(
        &raw_probabilities,
        &labels,
        &calibration_row_refs,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
        &raw_probabilities,
        calibration_candidate.clone(),
    );

    assert_eq!(
        calibration.as_ref().map(|artifact| artifact.alpha),
        Some(calibration_candidate.alpha)
    );
    assert_ne!(evaluation_probabilities, raw_probabilities);
    assert!(evaluation_probabilities[0] > evaluation_probabilities[2]);
}

#[test]
fn probability_calibration_strategy_keeps_raw_when_calibration_flattens_early_warning() {
    let calibration_rows = vec![
        forward_crisis_row(
            NaiveDate::from_ymd_opt(2020, 1, 5).unwrap(),
            1,
            ProbabilityTrainingRegime::PositiveWindow,
        ),
        forward_crisis_row(
            NaiveDate::from_ymd_opt(2020, 1, 7).unwrap(),
            1,
            ProbabilityTrainingRegime::PositiveWindow,
        ),
        forward_crisis_row(
            NaiveDate::from_ymd_opt(2020, 1, 12).unwrap(),
            0,
            ProbabilityTrainingRegime::PreWarningBuffer,
        ),
        forward_crisis_row(
            NaiveDate::from_ymd_opt(2020, 1, 20).unwrap(),
            0,
            ProbabilityTrainingRegime::Normal,
        ),
        forward_crisis_row(
            NaiveDate::from_ymd_opt(2020, 2, 3).unwrap(),
            0,
            ProbabilityTrainingRegime::InCrisis,
        ),
    ];
    let calibration_row_refs = calibration_rows.iter().collect::<Vec<_>>();
    let raw_probabilities = vec![0.72, 0.68, 0.44, 0.12, 0.61];
    let labels = calibration_rows
        .iter()
        .map(|row| row.label_20d as f64)
        .collect::<Vec<_>>();
    let flattening_calibration = PlattCalibrationArtifact {
        alpha: 0.02,
        beta: -4.2,
        min_input: 0.12,
        max_input: 0.72,
    };

    let (calibration, evaluation_probabilities) = select_probability_calibration_strategy(
        &raw_probabilities,
        &labels,
        &calibration_row_refs,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
        &raw_probabilities,
        flattening_calibration,
    );

    assert!(calibration.is_none());
    assert_eq!(evaluation_probabilities, raw_probabilities);
}

#[test]
fn probability_decision_threshold_prefers_precision_over_low_cutoff_noise() {
    let probabilities = vec![0.82, 0.71, 0.24, 0.11, 0.09, 0.08];
    let labels = vec![1.0, 1.0, 0.0, 0.0, 0.0, 0.0];

    let threshold = select_probability_decision_threshold(&probabilities, &labels, 5);

    assert!(threshold >= 0.71);
}

#[test]
fn probability_decision_threshold_allows_low_calibrated_ranges() {
    let probabilities = vec![0.024, 0.021, 0.018, 0.007, 0.006, 0.005];
    let labels = vec![1.0, 1.0, 1.0, 0.0, 0.0, 0.0];

    let threshold = select_probability_decision_threshold(&probabilities, &labels, 60);

    assert!(threshold < 0.05);
    assert!(threshold >= 0.018);
}

#[test]
fn probability_decision_threshold_can_drop_below_one_percent() {
    let probabilities = vec![0.0086, 0.0082, 0.0079, 0.0034, 0.0028, 0.0021];
    let labels = vec![1.0, 1.0, 1.0, 0.0, 0.0, 0.0];

    let threshold = select_probability_decision_threshold(&probabilities, &labels, 20);

    assert!(threshold < 0.01);
    assert!(threshold >= 0.007);
}

#[test]
fn probability_decision_threshold_raises_cutoff_when_low_threshold_is_overbroad() {
    let probabilities = vec![0.38, 0.36, 0.35, 0.34, 0.33, 0.32, 0.31, 0.30, 0.29, 0.28];
    let labels = vec![1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];

    let threshold = select_probability_decision_threshold(&probabilities, &labels, 20);

    assert!(threshold >= 0.35);
    assert!(threshold > 0.30);
}

#[test]
fn probability_decision_threshold_keeps_more_recall_for_60d_when_precision_tradeoff_is_small() {
    let probabilities = vec![0.45, 0.40, 0.35, 0.30, 0.25, 0.34, 0.28, 0.22, 0.18, 0.12];
    let labels = vec![1.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0];

    let threshold = select_probability_decision_threshold(&probabilities, &labels, 60);

    assert!(threshold <= 0.30);
    assert!(threshold >= 0.25);
}

#[test]
fn regime_support_adjustment_lowers_60d_threshold_when_base_misses_prewarning_buffer() {
    let build_row = |regime_60d: ProbabilityTrainingRegime, label_60d: u8| ProbabilityTrainingRow {
        as_of_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
        market_scope: "financial_system".to_string(),
        release_id: None,
        probability_mode: Some("formal_bundle_v1".to_string()),
        freshness_status: Some("fresh".to_string()),
        time_to_risk_bucket: Some("test".to_string()),
        split_name: Some("calibration".to_string()),
        features: BTreeMap::new(),
        primary_scenario_id: Some("scenario".to_string()),
        scenario_family: Some("systemic_credit_banking_crisis".to_string()),
        scenario_training_role: None,
        days_to_primary_crisis_start: Some(20),
        primary_scenario_supports_5d: true,
        primary_scenario_supports_20d: true,
        primary_scenario_supports_60d: true,
        label_5d: 0,
        label_20d: 0,
        label_60d,
        regime_5d: ProbabilityTrainingRegime::Normal,
        regime_20d: ProbabilityTrainingRegime::Normal,
        regime_60d,
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
        build_row(ProbabilityTrainingRegime::PositiveWindow, 1),
        build_row(ProbabilityTrainingRegime::PositiveWindow, 1),
        build_row(ProbabilityTrainingRegime::InCrisis, 1),
        build_row(ProbabilityTrainingRegime::InCrisis, 1),
        build_row(ProbabilityTrainingRegime::PreWarningBuffer, 0),
        build_row(ProbabilityTrainingRegime::PreWarningBuffer, 0),
        build_row(ProbabilityTrainingRegime::Normal, 0),
        build_row(ProbabilityTrainingRegime::Normal, 0),
        build_row(ProbabilityTrainingRegime::Normal, 0),
        build_row(ProbabilityTrainingRegime::PostCrisisCooldown, 0),
    ];
    let row_refs = rows.iter().collect::<Vec<_>>();
    let probabilities = vec![0.95, 0.91, 0.84, 0.80, 0.58, 0.56, 0.22, 0.18, 0.14, 0.10];
    let labels = rows
        .iter()
        .map(|row| row.label_60d as f64)
        .collect::<Vec<_>>();
    let calibration_selection = ProbabilityCalibrationSelection {
        rows: row_refs.clone(),
        eligible_row_count: row_refs.len(),
        eligible_positive_count: labels.iter().filter(|label| **label >= 0.5).count(),
        eligible_negative_count: labels.iter().filter(|label| **label < 0.5).count(),
        used_full_split_fallback: false,
    };
    let threshold_selection = ProbabilityThresholdSelection {
        rows: row_refs.clone(),
        probabilities: probabilities.clone(),
        labels: labels.clone(),
        used_full_split_fallback: false,
    };

    let base_threshold = select_probability_decision_threshold(&probabilities, &labels, 60);
    let adjusted_threshold = adjust_probability_decision_threshold_for_regime_support(
        base_threshold,
        &probabilities,
        &labels,
        &row_refs,
        60,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    let diagnostics =
        build_probability_threshold_diagnostics(ProbabilityThresholdDiagnosticsInput {
            full_calibration_rows: &rows,
            calibration_selection: &calibration_selection,
            threshold_selection: &threshold_selection,
            horizon_days: 60,
            label_mode: ProbabilityTargetLabelMode::ForwardCrisis,
            base_threshold,
            final_threshold: adjusted_threshold,
        });

    assert!(base_threshold > 0.58);
    assert!(adjusted_threshold <= base_threshold);
    assert!(diagnostics.repair_applied);
    assert!(matches!(
        diagnostics.repair_reason.as_str(),
        "repaired_to_early_warning_cap" | "repaired_to_regime_support_candidate"
    ));
    assert_eq!(diagnostics.base_summary.early_warning_hit_count, 0);
    assert!(diagnostics.final_summary.early_warning_hit_count > 0);

    let prewarning_evidence = diagnostics
        .calibration_regime_evidence
        .iter()
        .find(|row| row.regime == "pre_warning_buffer")
        .expect("pre-warning calibration evidence");
    assert_eq!(prewarning_evidence.full_row_count, 2);
    assert_eq!(prewarning_evidence.calibration_eligible_row_count, 2);
    assert_eq!(prewarning_evidence.calibration_used_row_count, 2);
    assert_eq!(prewarning_evidence.threshold_selected_row_count, 2);
    assert_eq!(prewarning_evidence.positive_label_count, 0);
    assert_eq!(prewarning_evidence.avg_hard_label, 0.0);
    assert_eq!(prewarning_evidence.avg_training_target, 0.26);
    assert_eq!(prewarning_evidence.avg_objective_weight, 0.6);
}

#[test]
fn threshold_selection_excludes_in_crisis_negatives_for_60d_forward_crisis() {
    let build_row = |regime_60d: ProbabilityTrainingRegime, label_60d: u8| ProbabilityTrainingRow {
        as_of_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
        market_scope: "financial_system".to_string(),
        release_id: None,
        probability_mode: Some("formal_bundle_v1".to_string()),
        freshness_status: Some("fresh".to_string()),
        time_to_risk_bucket: Some("test".to_string()),
        split_name: Some("calibration".to_string()),
        features: BTreeMap::new(),
        primary_scenario_id: Some("scenario".to_string()),
        scenario_family: Some("systemic_credit_banking_crisis".to_string()),
        scenario_training_role: None,
        days_to_primary_crisis_start: Some(20),
        primary_scenario_supports_5d: true,
        primary_scenario_supports_20d: true,
        primary_scenario_supports_60d: true,
        label_5d: 0,
        label_20d: 0,
        label_60d,
        regime_5d: ProbabilityTrainingRegime::Normal,
        regime_20d: ProbabilityTrainingRegime::Normal,
        regime_60d,
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
        build_row(ProbabilityTrainingRegime::PositiveWindow, 1),
        build_row(ProbabilityTrainingRegime::PreWarningBuffer, 0),
        build_row(ProbabilityTrainingRegime::Normal, 0),
        build_row(ProbabilityTrainingRegime::PostCrisisCooldown, 0),
        build_row(ProbabilityTrainingRegime::InCrisis, 0),
    ];
    let row_refs = rows.iter().collect::<Vec<_>>();
    let probabilities = vec![0.9, 0.55, 0.20, 0.10, 0.88];
    let labels = rows
        .iter()
        .map(|row| row.label_60d as f64)
        .collect::<Vec<_>>();

    let selection = probability_decision_threshold_selection(
        &probabilities,
        &labels,
        &row_refs,
        60,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );

    assert!(!selection.used_full_split_fallback);
    assert_eq!(selection.rows.len(), 4);
    assert_eq!(
        selection
            .labels
            .iter()
            .filter(|label| **label >= 0.5)
            .count(),
        1
    );
    assert_eq!(
        selection
            .labels
            .iter()
            .filter(|label| **label < 0.5)
            .count(),
        3
    );
    assert!(selection
        .rows
        .iter()
        .all(|row| row.regime_60d != ProbabilityTrainingRegime::InCrisis));
}

#[test]
fn runtime_regime_separation_detects_calibration_crush() {
    let scenarios = synthetic_runtime_scenarios();
    let history = vec![
        runtime_history_point(NaiveDate::from_ymd_opt(1999, 12, 20).unwrap(), 0.10, 0.020),
        runtime_history_point(NaiveDate::from_ymd_opt(2000, 1, 5).unwrap(), 0.30, 0.021),
        runtime_history_point(NaiveDate::from_ymd_opt(2000, 1, 20).unwrap(), 0.45, 0.022),
        runtime_history_point(NaiveDate::from_ymd_opt(2000, 2, 5).unwrap(), 0.55, 0.023),
    ];

    let summaries = summarize_release_runtime_regime_probabilities(&history, &scenarios, None);
    let separation = summarize_release_runtime_regime_separation(&summaries);
    let row20 = separation
        .iter()
        .find(|row| row.horizon_days == 20)
        .expect("20d summary");

    assert_eq!(row20.early_warning_regime, "pre_warning_buffer");
    assert_eq!(row20.diagnosis, "calibration_crushed_early_warning");
    assert!(
        row20
            .early_warning_raw_lift_vs_normal
            .expect("raw lift should exist")
            >= 2.9
    );
    assert!(
        row20
            .early_warning_calibrated_lift_vs_normal
            .expect("calibrated lift should exist")
            < 1.1
    );
    assert!(
        row20
            .early_warning_gap_retention
            .expect("gap retention should exist")
            < 0.1
    );
}

#[test]
fn runtime_regime_classifier_flags_cooldown_bleed() {
    let diagnosis = classify_regime_separation(
        20,
        1.7,
        1.6,
        Some(0.9),
        1.55,
        0.014,
        1.3,
        1.58,
        0.015,
        1.58,
        0.05,
    );

    assert_eq!(diagnosis, "cooldown_bleed");
}

#[test]
fn offline_regime_classifier_uses_positive_window_gap_not_only_buffer_lift() {
    let diagnosis =
        classify_probability_regime_separation(20, 1.6, 1.52, 1.6, 1.2, 1.1, 0.012, 0.004, 1.6);

    assert_eq!(diagnosis, "usable_early_warning_separation");
}

#[test]
fn probability_guardrails_reject_zero_usable_early_warning_horizons() {
    let bundle = ProbabilityBundle {
        bundle_id: "candidate_guard_zero".to_string(),
        market_scope: "financial_system".to_string(),
        probability_mode: "formal_bundle_v1".to_string(),
        model_family: PROBABILITY_MODEL_FAMILY_LINEAR_V1.to_string(),
        feature_transform: PROBABILITY_FEATURE_TRANSFORM_IDENTITY_V1.to_string(),
        created_at: Utc::now(),
        feature_names: Vec::new(),
        monotonic_min_gap_5d_to_20d: 0.0,
        monotonic_min_gap_20d_to_60d: 0.0,
        note: "test".to_string(),
        horizons: Vec::new(),
        evaluation: Some(ProbabilityBundleEvaluation {
            sample_count: 100,
            brier_score: 0.1,
            log_loss: 0.2,
            ece: 0.1,
            regime_separation_summaries: vec![RegimeSeparationEvaluationSummary {
                horizon_days: 20,
                early_warning_regime: "pre_warning_buffer".to_string(),
                normal_sample_count: 50,
                pre_warning_buffer_sample_count: 10,
                positive_window_sample_count: 10,
                early_warning_sample_count: 10,
                in_crisis_sample_count: 10,
                post_crisis_cooldown_sample_count: 10,
                normal_avg_probability: 0.02,
                pre_warning_buffer_avg_probability: 0.025,
                positive_window_avg_probability: 0.019,
                early_warning_avg_probability: 0.025,
                in_crisis_avg_probability: 0.04,
                post_crisis_cooldown_avg_probability: 0.03,
                max_non_normal_avg_probability: 0.04,
                pre_warning_buffer_lift_vs_normal: Some(1.25),
                positive_window_lift_vs_normal: Some(0.95),
                early_warning_lift_vs_normal: Some(1.25),
                in_crisis_lift_vs_normal: Some(2.0),
                post_crisis_cooldown_lift_vs_normal: Some(1.5),
                positive_window_gap_vs_normal: Some(-0.001),
                post_crisis_cooldown_gap_vs_normal: Some(0.01),
                max_non_normal_lift_vs_normal: Some(2.0),
                diagnosis: "cold_across_all_regimes".to_string(),
            }],
            usable_early_warning_horizon_count: 0,
            insufficient_early_warning_horizon_count: 1,
            note: "test".to_string(),
        }),
        actionability: None,
    };
    let release = test_release_with_bundle(&bundle);
    let bundle_path = release.manifest.bundle_uri.clone();

    let regressions = compare_probability_guardrails(&release).unwrap();

    let _ = std::fs::remove_file(bundle_path);
    assert!(regressions
        .iter()
        .any(|item| item.contains("zero usable early-warning horizons")));
    assert!(regressions
        .iter()
        .any(|item| item.contains("20d positive_window avg")));
    assert!(regressions
        .iter()
        .any(|item| item.contains("cold_across_all_regimes")));
}

#[test]
fn probability_guardrails_reject_cooldown_bleed_on_medium_horizons() {
    let bundle = ProbabilityBundle {
        bundle_id: "candidate_guard_cooldown".to_string(),
        market_scope: "financial_system".to_string(),
        probability_mode: "formal_bundle_v1".to_string(),
        model_family: PROBABILITY_MODEL_FAMILY_LINEAR_V1.to_string(),
        feature_transform: PROBABILITY_FEATURE_TRANSFORM_IDENTITY_V1.to_string(),
        created_at: Utc::now(),
        feature_names: Vec::new(),
        monotonic_min_gap_5d_to_20d: 0.0,
        monotonic_min_gap_20d_to_60d: 0.0,
        note: "test".to_string(),
        horizons: Vec::new(),
        evaluation: Some(ProbabilityBundleEvaluation {
            sample_count: 100,
            brier_score: 0.1,
            log_loss: 0.2,
            ece: 0.1,
            regime_separation_summaries: vec![RegimeSeparationEvaluationSummary {
                horizon_days: 60,
                early_warning_regime: "pre_warning_buffer".to_string(),
                normal_sample_count: 50,
                pre_warning_buffer_sample_count: 10,
                positive_window_sample_count: 10,
                early_warning_sample_count: 10,
                in_crisis_sample_count: 10,
                post_crisis_cooldown_sample_count: 10,
                normal_avg_probability: 0.03,
                pre_warning_buffer_avg_probability: 0.05,
                positive_window_avg_probability: 0.065,
                early_warning_avg_probability: 0.05,
                in_crisis_avg_probability: 0.06,
                post_crisis_cooldown_avg_probability: 0.068,
                max_non_normal_avg_probability: 0.068,
                pre_warning_buffer_lift_vs_normal: Some(1.67),
                positive_window_lift_vs_normal: Some(2.17),
                early_warning_lift_vs_normal: Some(1.67),
                in_crisis_lift_vs_normal: Some(2.0),
                post_crisis_cooldown_lift_vs_normal: Some(2.27),
                positive_window_gap_vs_normal: Some(0.035),
                post_crisis_cooldown_gap_vs_normal: Some(0.038),
                max_non_normal_lift_vs_normal: Some(2.27),
                diagnosis: "cooldown_bleed".to_string(),
            }],
            usable_early_warning_horizon_count: 1,
            insufficient_early_warning_horizon_count: 1,
            note: "test".to_string(),
        }),
        actionability: None,
    };
    let release = test_release_with_bundle(&bundle);
    let bundle_path = release.manifest.bundle_uri.clone();

    let regressions = compare_probability_guardrails(&release).unwrap();

    let _ = std::fs::remove_file(bundle_path);
    assert!(regressions
        .iter()
        .any(|item| item.contains("cooldown_bleed")));
}

use super::*;

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
    let evaluation_row_refs = calibration_rows.iter().collect::<Vec<_>>();
    let calibration_candidate = PlattCalibrationArtifact {
        alpha: -1.4,
        beta: -3.0,
        min_input: 0.11,
        max_input: 0.82,
    };

    let (calibration, evaluation_probabilities) =
        select_probability_calibration_strategy(ProbabilityCalibrationStrategyInput {
            calibration_raw_probabilities: &raw_probabilities,
            calibration_labels: &labels,
            calibration_rows: &calibration_row_refs,
            horizon_days: 20,
            label_mode: ProbabilityTargetLabelMode::ForwardCrisis,
            evaluation_raw_probabilities: &raw_probabilities,
            evaluation_rows: &evaluation_row_refs,
            calibration_candidate,
        });

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
    let evaluation_row_refs = calibration_rows.iter().collect::<Vec<_>>();
    let calibration_candidate = fit_platt_calibration(&raw_probabilities, &labels);
    assert!(calibration_candidate.alpha < 0.0);

    let (calibration, evaluation_probabilities) =
        select_probability_calibration_strategy(ProbabilityCalibrationStrategyInput {
            calibration_raw_probabilities: &raw_probabilities,
            calibration_labels: &labels,
            calibration_rows: &calibration_row_refs,
            horizon_days: 20,
            label_mode: ProbabilityTargetLabelMode::ForwardCrisis,
            evaluation_raw_probabilities: &raw_probabilities,
            evaluation_rows: &evaluation_row_refs,
            calibration_candidate: calibration_candidate.clone(),
        });

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
    let evaluation_row_refs = calibration_rows.iter().collect::<Vec<_>>();
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

    let (calibration, evaluation_probabilities) =
        select_probability_calibration_strategy(ProbabilityCalibrationStrategyInput {
            calibration_raw_probabilities: &raw_probabilities,
            calibration_labels: &labels,
            calibration_rows: &calibration_row_refs,
            horizon_days: 20,
            label_mode: ProbabilityTargetLabelMode::ForwardCrisis,
            evaluation_raw_probabilities: &raw_probabilities,
            evaluation_rows: &evaluation_row_refs,
            calibration_candidate: flattening_calibration,
        });

    assert!(calibration.is_none());
    assert_eq!(evaluation_probabilities, raw_probabilities);
}

#[test]
fn probability_calibration_strategy_rejects_calibration_that_crushes_evaluation_regime_support() {
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
    let calibration_raw_probabilities = vec![0.11, 0.24, 0.71, 0.82];
    let calibration_labels = vec![1.0, 1.0, 0.0, 0.0];
    let calibration_candidate =
        fit_platt_calibration(&calibration_raw_probabilities, &calibration_labels);
    assert!(calibration_candidate.alpha < 0.0);

    let evaluation_rows = vec![
        forward_crisis_row(
            NaiveDate::from_ymd_opt(2020, 2, 1).unwrap(),
            1,
            ProbabilityTrainingRegime::PositiveWindow,
        ),
        forward_crisis_row(
            NaiveDate::from_ymd_opt(2020, 2, 2).unwrap(),
            1,
            ProbabilityTrainingRegime::PositiveWindow,
        ),
        forward_crisis_row(
            NaiveDate::from_ymd_opt(2020, 2, 3).unwrap(),
            0,
            ProbabilityTrainingRegime::PreWarningBuffer,
        ),
        forward_crisis_row(
            NaiveDate::from_ymd_opt(2020, 2, 10).unwrap(),
            0,
            ProbabilityTrainingRegime::Normal,
        ),
        forward_crisis_row(
            NaiveDate::from_ymd_opt(2020, 2, 20).unwrap(),
            0,
            ProbabilityTrainingRegime::InCrisis,
        ),
    ];
    let evaluation_row_refs = evaluation_rows.iter().collect::<Vec<_>>();
    let evaluation_raw_probabilities = vec![0.91, 0.84, 0.58, 0.12, 0.79];

    let (calibration, evaluation_probabilities) =
        select_probability_calibration_strategy(ProbabilityCalibrationStrategyInput {
            calibration_raw_probabilities: &calibration_raw_probabilities,
            calibration_labels: &calibration_labels,
            calibration_rows: &calibration_row_refs,
            horizon_days: 60,
            label_mode: ProbabilityTargetLabelMode::ForwardCrisis,
            evaluation_raw_probabilities: &evaluation_raw_probabilities,
            evaluation_rows: &evaluation_row_refs,
            calibration_candidate,
        });

    assert!(calibration.is_none());
    assert_eq!(evaluation_probabilities, evaluation_raw_probabilities);
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
    assert_eq!(prewarning_evidence.episode_native_objective_row_count, 0);
    assert_eq!(prewarning_evidence.protected_no_positive_main_row_count, 0);
    assert_eq!(
        prewarning_evidence.protected_no_positive_main_avg_training_target,
        0.0
    );
    assert_eq!(
        prewarning_evidence.protected_no_positive_main_avg_objective_weight,
        0.0
    );
}

#[test]
fn regime_support_adjustment_repairs_over_tight_20d_threshold_without_lift_gate() {
    let build_row = |regime_20d: ProbabilityTrainingRegime, label_20d: u8| ProbabilityTrainingRow {
        as_of_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
        market_scope: "financial_system".to_string(),
        release_id: None,
        probability_mode: Some("formal_bundle_v1".to_string()),
        freshness_status: Some("fresh".to_string()),
        time_to_risk_bucket: Some("test".to_string()),
        split_name: Some("calibration".to_string()),
        features: BTreeMap::new(),
        primary_scenario_id: Some("scenario".to_string()),
        scenario_family: Some("mixed_systemic_stress".to_string()),
        scenario_training_role: None,
        days_to_primary_crisis_start: Some(15),
        primary_scenario_supports_5d: true,
        primary_scenario_supports_20d: true,
        primary_scenario_supports_60d: true,
        label_5d: 0,
        label_20d,
        label_60d: 0,
        regime_5d: ProbabilityTrainingRegime::Normal,
        regime_20d,
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
    let probabilities = vec![0.95, 0.93, 0.92, 0.91, 0.84, 0.82, 0.60, 0.59, 0.58, 0.54];
    let labels = rows
        .iter()
        .map(|row| row.label_20d as f64)
        .collect::<Vec<_>>();

    let base_threshold = select_probability_decision_threshold(&probabilities, &labels, 20);
    let adjusted_threshold = adjust_probability_decision_threshold_for_regime_support(
        base_threshold,
        &probabilities,
        &labels,
        &row_refs,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
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
    let diagnostics =
        build_probability_threshold_diagnostics(ProbabilityThresholdDiagnosticsInput {
            full_calibration_rows: &rows,
            calibration_selection: &calibration_selection,
            threshold_selection: &threshold_selection,
            horizon_days: 20,
            label_mode: ProbabilityTargetLabelMode::ForwardCrisis,
            base_threshold,
            final_threshold: adjusted_threshold,
        });

    let early_warning_hit_count = |threshold: f64| {
        probabilities
            .iter()
            .zip(row_refs.iter())
            .filter(|(probability, row)| {
                **probability >= threshold
                    && row.regime_20d == ProbabilityTrainingRegime::PreWarningBuffer
            })
            .count()
    };
    let normal_hit_count = |threshold: f64| {
        probabilities
            .iter()
            .zip(row_refs.iter())
            .filter(|(probability, row)| {
                **probability >= threshold && row.regime_20d == ProbabilityTrainingRegime::Normal
            })
            .count()
    };

    assert_eq!(base_threshold, 0.90);
    assert!(adjusted_threshold < base_threshold);
    assert!(adjusted_threshold <= 0.84);
    assert_eq!(early_warning_hit_count(base_threshold), 0);
    assert!(early_warning_hit_count(adjusted_threshold) > 0);
    assert!(normal_hit_count(adjusted_threshold) < early_warning_hit_count(adjusted_threshold));
    assert!(diagnostics.repair_applied);
    assert_eq!(
        diagnostics.repair_reason,
        "repaired_over_tight_threshold_below_lift_guardrail"
    );
}

#[test]
fn regime_support_adjustment_repairs_sparse_20d_prewarning_hits_at_extreme_threshold() {
    let build_row = |regime_20d: ProbabilityTrainingRegime, label_20d: u8| ProbabilityTrainingRow {
        as_of_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
        market_scope: "financial_system".to_string(),
        release_id: None,
        probability_mode: Some("formal_bundle_v1".to_string()),
        freshness_status: Some("fresh".to_string()),
        time_to_risk_bucket: Some("test".to_string()),
        split_name: Some("calibration".to_string()),
        features: BTreeMap::new(),
        primary_scenario_id: Some("scenario".to_string()),
        scenario_family: Some("mixed_systemic_stress".to_string()),
        scenario_training_role: None,
        days_to_primary_crisis_start: Some(15),
        primary_scenario_supports_5d: true,
        primary_scenario_supports_20d: true,
        primary_scenario_supports_60d: true,
        label_5d: 0,
        label_20d,
        label_60d: 0,
        regime_5d: ProbabilityTrainingRegime::Normal,
        regime_20d,
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
        build_row(ProbabilityTrainingRegime::PositiveWindow, 1),
        build_row(ProbabilityTrainingRegime::PositiveWindow, 1),
        build_row(ProbabilityTrainingRegime::InCrisis, 1),
        build_row(ProbabilityTrainingRegime::InCrisis, 1),
    ];
    let mut probabilities = vec![0.96, 0.94, 0.93, 0.91];
    for index in 0..100 {
        rows.push(build_row(ProbabilityTrainingRegime::PreWarningBuffer, 0));
        probabilities.push(match index {
            0 | 1 => 0.91,
            2..=7 => 0.84,
            _ => 0.62,
        });
    }
    for _ in 0..20 {
        rows.push(build_row(ProbabilityTrainingRegime::Normal, 0));
        probabilities.push(0.58);
    }
    for _ in 0..5 {
        rows.push(build_row(ProbabilityTrainingRegime::PostCrisisCooldown, 0));
        probabilities.push(0.54);
    }

    let row_refs = rows.iter().collect::<Vec<_>>();
    let labels = rows
        .iter()
        .map(|row| row.label_20d as f64)
        .collect::<Vec<_>>();

    let base_threshold = select_probability_decision_threshold(&probabilities, &labels, 20);
    let adjusted_threshold = adjust_probability_decision_threshold_for_regime_support(
        base_threshold,
        &probabilities,
        &labels,
        &row_refs,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    let early_warning_hit_count = |threshold: f64| {
        probabilities
            .iter()
            .zip(row_refs.iter())
            .filter(|(probability, row)| {
                **probability >= threshold
                    && row.regime_20d == ProbabilityTrainingRegime::PreWarningBuffer
            })
            .count()
    };

    assert_eq!(base_threshold, 0.90);
    assert_eq!(early_warning_hit_count(base_threshold), 2);
    assert!(adjusted_threshold < base_threshold);
    assert!(adjusted_threshold <= 0.84);
    assert!(early_warning_hit_count(adjusted_threshold) >= 8);
}

#[test]
fn threshold_diagnostics_rejects_sparse_positive_window_support_as_usable() {
    let mut rows = Vec::new();
    let mut probabilities = Vec::new();
    for index in 0..10 {
        rows.push(forward_crisis_row(
            NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
            1,
            ProbabilityTrainingRegime::PositiveWindow,
        ));
        probabilities.push(if index < 2 { 0.91 } else { 0.42 });
    }
    for index in 0..10 {
        rows.push(forward_crisis_row(
            NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
            0,
            ProbabilityTrainingRegime::PreWarningBuffer,
        ));
        probabilities.push(if index < 4 { 0.92 } else { 0.40 });
    }
    for _ in 0..20 {
        rows.push(forward_crisis_row(
            NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
            0,
            ProbabilityTrainingRegime::Normal,
        ));
        probabilities.push(0.30);
    }

    let row_refs = rows.iter().collect::<Vec<_>>();
    let labels = rows
        .iter()
        .map(|row| row.label_20d as f64)
        .collect::<Vec<_>>();
    let calibration_selection = ProbabilityCalibrationSelection {
        rows: row_refs.clone(),
        eligible_row_count: row_refs.len(),
        eligible_positive_count: labels.iter().filter(|label| **label >= 0.5).count(),
        eligible_negative_count: labels.iter().filter(|label| **label < 0.5).count(),
        used_full_split_fallback: false,
    };
    let threshold_selection = ProbabilityThresholdSelection {
        rows: row_refs,
        probabilities,
        labels,
        used_full_split_fallback: false,
    };
    let diagnostics =
        build_probability_threshold_diagnostics(ProbabilityThresholdDiagnosticsInput {
            full_calibration_rows: &rows,
            calibration_selection: &calibration_selection,
            threshold_selection: &threshold_selection,
            horizon_days: 20,
            label_mode: ProbabilityTargetLabelMode::ForwardCrisis,
            base_threshold: 0.90,
            final_threshold: 0.90,
        });

    assert_eq!(diagnostics.base_summary.early_warning_hit_rate, 0.40);
    assert_eq!(diagnostics.base_summary.positive_window_hit_rate, 0.20);
    assert_eq!(
        diagnostics.repair_reason,
        "base_hits_early_warning_but_positive_window_support_is_too_weak"
    );
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
fn calibration_regime_evidence_surfaces_protected_no_positive_main_episode_rows() {
    let protected_hedge_row = ProbabilityTrainingRow {
        as_of_date: NaiveDate::from_ymd_opt(2022, 4, 1).unwrap(),
        market_scope: "financial_system".to_string(),
        release_id: None,
        probability_mode: Some("formal_bundle_v1".to_string()),
        freshness_status: Some("fresh".to_string()),
        time_to_risk_bucket: Some("weeks".to_string()),
        split_name: Some("calibration".to_string()),
        features: BTreeMap::new(),
        primary_scenario_id: Some("us_rate_shock_2022".to_string()),
        scenario_family: Some("rate_shock_or_policy_dislocation".to_string()),
        scenario_training_role: Some("no_positive_main".to_string()),
        days_to_primary_crisis_start: Some(40),
        primary_scenario_supports_5d: false,
        primary_scenario_supports_20d: true,
        primary_scenario_supports_60d: true,
        label_5d: 0,
        label_20d: 0,
        label_60d: 0,
        regime_5d: ProbabilityTrainingRegime::Normal,
        regime_20d: ProbabilityTrainingRegime::PreWarningBuffer,
        regime_60d: ProbabilityTrainingRegime::PreWarningBuffer,
        action_label_5d: 0,
        action_label_20d: 1,
        action_label_60d: 0,
        prepare_episode_label: 0,
        hedge_episode_label: 1,
        defend_episode_label: 0,
        primary_action_level: Some("hedge".to_string()),
        action_episode_id: Some("us_rate_shock_2022:hedge".to_string()),
        action_episode_phase: "primary".to_string(),
        protected_action_window: true,
    };
    let normal_row = ProbabilityTrainingRow {
        as_of_date: NaiveDate::from_ymd_opt(2022, 4, 2).unwrap(),
        market_scope: "financial_system".to_string(),
        release_id: None,
        probability_mode: Some("formal_bundle_v1".to_string()),
        freshness_status: Some("fresh".to_string()),
        time_to_risk_bucket: Some("weeks".to_string()),
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
    let rows = vec![protected_hedge_row, normal_row];
    let row_refs = rows.iter().collect::<Vec<_>>();
    let probabilities = vec![0.33, 0.08];
    let labels = rows
        .iter()
        .map(|row| row.label_20d as f64)
        .collect::<Vec<_>>();
    let calibration_selection = ProbabilityCalibrationSelection {
        rows: row_refs.clone(),
        eligible_row_count: row_refs.len(),
        eligible_positive_count: 0,
        eligible_negative_count: row_refs.len(),
        used_full_split_fallback: false,
    };
    let threshold_selection = ProbabilityThresholdSelection {
        rows: row_refs.clone(),
        probabilities: probabilities.clone(),
        labels: labels.clone(),
        used_full_split_fallback: false,
    };

    let diagnostics =
        build_probability_threshold_diagnostics(ProbabilityThresholdDiagnosticsInput {
            full_calibration_rows: &rows,
            calibration_selection: &calibration_selection,
            threshold_selection: &threshold_selection,
            horizon_days: 20,
            label_mode: ProbabilityTargetLabelMode::ForwardCrisis,
            base_threshold: 0.28,
            final_threshold: 0.28,
        });
    let prewarning_evidence = diagnostics
        .calibration_regime_evidence
        .iter()
        .find(|row| row.regime == "pre_warning_buffer")
        .expect("pre-warning calibration evidence");

    assert_eq!(prewarning_evidence.full_row_count, 1);
    assert_eq!(prewarning_evidence.protected_action_window_count, 1);
    assert_eq!(prewarning_evidence.episode_native_objective_row_count, 1);
    assert_eq!(prewarning_evidence.protected_no_positive_main_row_count, 1);
    assert_eq!(prewarning_evidence.avg_training_target, 0.34);
    assert_eq!(prewarning_evidence.avg_objective_weight, 0.9);
    assert_eq!(
        prewarning_evidence.protected_no_positive_main_avg_training_target,
        0.34
    );
    assert_eq!(
        prewarning_evidence.protected_no_positive_main_avg_objective_weight,
        0.9
    );
}

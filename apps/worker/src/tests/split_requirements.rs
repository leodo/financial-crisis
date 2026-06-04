#[test]
fn probability_decision_threshold_keeps_5d_floor_conservative() {
    let probabilities = vec![0.0086, 0.0082, 0.0079, 0.0034, 0.0028, 0.0021];
    let labels = vec![1.0, 1.0, 1.0, 0.0, 0.0, 0.0];

    let threshold = select_probability_decision_threshold(&probabilities, &labels, 5);

    assert_eq!(threshold, 0.03);
}

#[test]
fn actionability_guardrails_flag_narrow_or_zero_hit_reviews() {
    let review = ReleaseActionabilityReview {
        release_id: "candidate".to_string(),
        enabled: true,
        model_version: Some("actionability_bundle_test".to_string()),
        calibration_version: Some("actionability_platt_test".to_string()),
        fusion_policy_version: Some("fusion_policy_test".to_string()),
        levels: vec![ReleaseActionabilityLevelReview {
            level: ActionabilityLevel::Prepare,
            proxy_horizon_days: 60,
            sample_count: 100,
            positive_rate: 0.03,
            threshold: 0.3,
            predicted_positive_count: 0,
            primary_positive_count: 12,
            late_validation_row_count: 4,
            protected_row_count: 0,
            primary_hit_count: 0,
            late_validation_hit_count: 0,
            protected_hit_count: 0,
            false_positive_count: 0,
            scenario_count: 1,
            on_time_scenario_count: 0,
            late_only_scenario_count: 0,
            missed_scenario_count: 1,
            precision_at_threshold: None,
            primary_recall_at_threshold: Some(0.0),
            late_validation_capture_rate: Some(0.0),
            on_time_rate: Some(0.0),
            late_only_rate: Some(0.0),
            missed_rate: Some(1.0),
            note: "test".to_string(),
        }],
        guard_regressions: Vec::new(),
        guard_passed: true,
        note: "test".to_string(),
    };

    let regressions = compare_actionability_guardrails(&review);
    assert!(regressions
        .iter()
        .any(|item| item.contains("scenario_count")));
    assert!(regressions
        .iter()
        .any(|item| item.contains("produced no primary or late-validation hits")));
    assert!(regressions.iter().any(|item| item.contains("on_time_rate")));
    assert!(regressions.iter().any(|item| item.contains("missed_rate")));
}

#[test]
fn actionability_guardrails_apply_level_specific_rate_thresholds() {
    let review = ReleaseActionabilityReview {
        release_id: "candidate".to_string(),
        enabled: true,
        model_version: Some("actionability_bundle_test".to_string()),
        calibration_version: Some("actionability_platt_test".to_string()),
        fusion_policy_version: Some("fusion_policy_test".to_string()),
        levels: vec![ReleaseActionabilityLevelReview {
            level: ActionabilityLevel::Defend,
            proxy_horizon_days: 5,
            sample_count: 120,
            positive_rate: 0.04,
            threshold: 0.12,
            predicted_positive_count: 5,
            primary_positive_count: 10,
            late_validation_row_count: 7,
            protected_row_count: 0,
            primary_hit_count: 1,
            late_validation_hit_count: 3,
            protected_hit_count: 0,
            false_positive_count: 1,
            scenario_count: 3,
            on_time_scenario_count: 0,
            late_only_scenario_count: 2,
            missed_scenario_count: 1,
            precision_at_threshold: Some(0.2),
            primary_recall_at_threshold: Some(0.33),
            late_validation_capture_rate: Some(0.43),
            on_time_rate: Some(0.0),
            late_only_rate: Some(0.67),
            missed_rate: Some(0.33),
            note: "test".to_string(),
        }],
        guard_regressions: Vec::new(),
        guard_passed: true,
        note: "test".to_string(),
    };

    let regressions = compare_actionability_guardrails(&review);
    assert!(regressions
        .iter()
        .any(|item| item.contains("late_only_rate")));
    assert!(!regressions.iter().any(|item| item.contains("on_time_rate")));
}

#[test]
fn scenario_aware_split_spreads_adjacent_scenarios_across_calibration_and_evaluation() {
    let mut rows = (0..180)
        .map(|index| {
            let scenario_id = match index {
                40..=59 => Some("scenario_a"),
                90..=109 => Some("scenario_b"),
                140..=159 => Some("scenario_c"),
                _ => None,
            };
            FormalDatasetRowRecord {
                dataset_key: "dataset".to_string(),
                split_name: String::new(),
                entity_id: "us".to_string(),
                market_scope: "financial_system".to_string(),
                as_of_date: NaiveDate::from_ymd_opt(2000, 1, 1)
                    .unwrap()
                    .checked_add_signed(chrono::Duration::days(index as i64))
                    .unwrap(),
                point_in_time_mode: "best_effort".to_string(),
                latest_visible_at: None,
                coverage_score: 1.0,
                core_feature_coverage: 1.0,
                trigger_feature_coverage: 1.0,
                external_feature_coverage: 1.0,
                sample_quality_grade: "a".to_string(),
                primary_scenario_id: scenario_id.map(str::to_string),
                scenario_family: scenario_id.map(|_| "systemic_credit_banking_crisis".to_string()),
                scenario_training_role: scenario_id.map(|_| "mandatory".to_string()),
                label_5d: u8::from(matches!(index, 56..=59 | 106..=109 | 156..=159)),
                label_20d: u8::from(matches!(index, 52..=59 | 102..=109 | 152..=159)),
                label_60d: u8::from(matches!(index, 44..=59 | 94..=109 | 144..=159)),
                regime_5d: "normal".to_string(),
                regime_20d: "normal".to_string(),
                regime_60d: "normal".to_string(),
                action_label_5d: u8::from(matches!(index, 55..=59 | 105..=109 | 155..=159)),
                action_label_20d: u8::from(matches!(index, 50..=59 | 100..=109 | 150..=159)),
                action_label_60d: u8::from(matches!(index, 42..=59 | 92..=109 | 142..=159)),
                prepare_episode_label: u8::from(matches!(index, 42..=59 | 92..=109 | 142..=159)),
                hedge_episode_label: u8::from(matches!(index, 50..=59 | 100..=109 | 150..=159)),
                defend_episode_label: u8::from(matches!(index, 55..=59 | 105..=109 | 155..=159)),
                primary_action_level: None,
                action_episode_id: None,
                action_episode_phase: "outside".to_string(),
                protected_action_window: false,
                features: BTreeMap::new(),
                created_at: Utc::now(),
            }
        })
        .collect::<Vec<_>>();

    let ranges = vec![
        ScenarioRowRange {
            scenario_id: "scenario_a".to_string(),
            family: "systemic_credit_banking_crisis".to_string(),
            start_index: 40,
            end_index: 59,
        },
        ScenarioRowRange {
            scenario_id: "scenario_b".to_string(),
            family: "systemic_credit_banking_crisis".to_string(),
            start_index: 90,
            end_index: 109,
        },
        ScenarioRowRange {
            scenario_id: "scenario_c".to_string(),
            family: "systemic_credit_banking_crisis".to_string(),
            start_index: 140,
            end_index: 159,
        },
    ];

    let split_requirements = formal_dataset_split_requirements("formal_label_v1_main");
    let (train_end, calibration_end) =
        scenario_aware_formal_split_bounds(&rows, &ranges, split_requirements).unwrap();
    assert!((56..=59).contains(&train_end));
    assert!((106..=109).contains(&calibration_end));

    for (index, row) in rows.iter_mut().enumerate() {
        row.split_name = if index < train_end {
            "train".to_string()
        } else if index < calibration_end {
            "calibration".to_string()
        } else {
            "evaluation".to_string()
        };
    }

    let calibration_scenarios = rows
        .iter()
        .filter(|row| row.split_name == "calibration")
        .filter_map(|row| row.primary_scenario_id.as_deref())
        .collect::<BTreeSet<_>>();
    let evaluation_scenarios = rows
        .iter()
        .filter(|row| row.split_name == "evaluation")
        .filter_map(|row| row.primary_scenario_id.as_deref())
        .collect::<BTreeSet<_>>();

    assert_eq!(calibration_scenarios.len(), 2);
    assert!(calibration_scenarios.contains("scenario_a"));
    assert!(calibration_scenarios.contains("scenario_b"));
    assert_eq!(evaluation_scenarios.len(), 2);
    assert!(evaluation_scenarios.contains("scenario_b"));
    assert!(evaluation_scenarios.contains("scenario_c"));
    assert_eq!(
        scenario_count_for_index_range(&rows, train_end, calibration_end),
        2
    );
    assert_eq!(
        scenario_count_for_index_range(&rows, calibration_end, rows.len()),
        2
    );

    let label_support = FormalSplitLabelSupport::from_rows(&rows);
    assert!(label_support.split_has_required_label_support(0, train_end, split_requirements));
    assert!(label_support.split_has_required_label_support(
        train_end,
        calibration_end,
        split_requirements
    ));
    assert!(label_support.split_has_required_label_support(
        calibration_end,
        rows.len(),
        split_requirements
    ));
}

#[test]
fn extension_acute_split_allows_two_scenarios_with_single_scenario_evaluation() {
    let rows = (0..220)
        .map(|index| {
            let scenario_id = match index {
                40..=69 => Some("acute_a"),
                150..=179 => Some("acute_b"),
                _ => None,
            };
            FormalDatasetRowRecord {
                dataset_key: "dataset".to_string(),
                split_name: String::new(),
                entity_id: "us".to_string(),
                market_scope: "financial_system".to_string(),
                as_of_date: NaiveDate::from_ymd_opt(2000, 1, 1)
                    .unwrap()
                    .checked_add_signed(chrono::Duration::days(index as i64))
                    .unwrap(),
                point_in_time_mode: "best_effort".to_string(),
                latest_visible_at: None,
                coverage_score: 1.0,
                core_feature_coverage: 1.0,
                trigger_feature_coverage: 1.0,
                external_feature_coverage: 1.0,
                sample_quality_grade: "a".to_string(),
                primary_scenario_id: scenario_id.map(str::to_string),
                scenario_family: scenario_id.map(|_| "acute_market_liquidity_crash".to_string()),
                scenario_training_role: scenario_id.map(|_| "extension_only".to_string()),
                label_5d: u8::from(matches!(index, 62..=69 | 172..=179)),
                label_20d: u8::from(matches!(index, 50..=69 | 160..=179)),
                label_60d: 0,
                regime_5d: "normal".to_string(),
                regime_20d: "normal".to_string(),
                regime_60d: "normal".to_string(),
                action_label_5d: u8::from(matches!(index, 62..=69 | 172..=179)),
                action_label_20d: u8::from(matches!(index, 50..=69 | 160..=179)),
                action_label_60d: 0,
                prepare_episode_label: u8::from(matches!(index, 48..=69 | 158..=179)),
                hedge_episode_label: u8::from(matches!(index, 56..=69 | 166..=179)),
                defend_episode_label: u8::from(matches!(index, 62..=69 | 172..=179)),
                primary_action_level: None,
                action_episode_id: None,
                action_episode_phase: "outside".to_string(),
                protected_action_window: false,
                features: BTreeMap::new(),
                created_at: Utc::now(),
            }
        })
        .collect::<Vec<_>>();

    let ranges = vec![
        ScenarioRowRange {
            scenario_id: "acute_a".to_string(),
            family: "acute_market_liquidity_crash".to_string(),
            start_index: 40,
            end_index: 69,
        },
        ScenarioRowRange {
            scenario_id: "acute_b".to_string(),
            family: "acute_market_liquidity_crash".to_string(),
            start_index: 150,
            end_index: 179,
        },
    ];

    let split_requirements = formal_dataset_split_requirements("formal_label_v1_ext_acute");
    let (train_end, calibration_end) =
        scenario_aware_formal_split_bounds(&rows, &ranges, split_requirements).unwrap();

    assert!((62..=69).contains(&train_end));
    assert!((172..=179).contains(&calibration_end));

    let label_support = FormalSplitLabelSupport::from_rows(&rows);
    assert!(label_support.split_has_required_label_support(0, train_end, split_requirements));
    assert!(label_support.split_has_required_label_support(
        train_end,
        calibration_end,
        split_requirements
    ));
    assert!(label_support.split_has_required_label_support(
        calibration_end,
        rows.len(),
        split_requirements
    ));

    assert_eq!(
        scenario_count_for_index_range(&rows, calibration_end, rows.len()),
        1
    );
}

#[test]
fn extension_stress_split_uses_20d_60d_prepare_hedge_requirements() {
    let rows = (0..260)
        .map(|index| {
            let scenario_id = match index {
                30..=59 => Some("stress_a"),
                90..=119 => Some("stress_b"),
                150..=179 => Some("stress_c"),
                210..=239 => Some("stress_d"),
                _ => None,
            };
            FormalDatasetRowRecord {
                dataset_key: "dataset".to_string(),
                split_name: String::new(),
                entity_id: "us".to_string(),
                market_scope: "financial_system".to_string(),
                as_of_date: NaiveDate::from_ymd_opt(2000, 1, 1)
                    .unwrap()
                    .checked_add_signed(chrono::Duration::days(index as i64))
                    .unwrap(),
                point_in_time_mode: "best_effort".to_string(),
                latest_visible_at: None,
                coverage_score: 1.0,
                core_feature_coverage: 1.0,
                trigger_feature_coverage: 1.0,
                external_feature_coverage: 1.0,
                sample_quality_grade: "a".to_string(),
                primary_scenario_id: scenario_id.map(str::to_string),
                scenario_family: scenario_id.map(|_| "mixed_systemic_stress".to_string()),
                scenario_training_role: scenario_id.map(|_| "extension_only".to_string()),
                label_5d: 0,
                label_20d: u8::from(matches!(index, 42..=59 | 102..=119 | 162..=179 | 222..=239)),
                label_60d: u8::from(matches!(index, 34..=59 | 94..=119 | 154..=179 | 214..=239)),
                regime_5d: "normal".to_string(),
                regime_20d: "normal".to_string(),
                regime_60d: "normal".to_string(),
                action_label_5d: 0,
                action_label_20d: u8::from(
                    matches!(index, 42..=59 | 102..=119 | 162..=179 | 222..=239),
                ),
                action_label_60d: u8::from(
                    matches!(index, 34..=59 | 94..=119 | 154..=179 | 214..=239),
                ),
                prepare_episode_label: u8::from(
                    matches!(index, 34..=59 | 94..=119 | 154..=179 | 214..=239),
                ),
                hedge_episode_label: u8::from(
                    matches!(index, 42..=59 | 102..=119 | 162..=179 | 222..=239),
                ),
                defend_episode_label: 0,
                primary_action_level: None,
                action_episode_id: None,
                action_episode_phase: "outside".to_string(),
                protected_action_window: true,
                features: BTreeMap::new(),
                created_at: Utc::now(),
            }
        })
        .collect::<Vec<_>>();

    let ranges = vec![
        ScenarioRowRange {
            scenario_id: "stress_a".to_string(),
            family: "mixed_systemic_stress".to_string(),
            start_index: 30,
            end_index: 59,
        },
        ScenarioRowRange {
            scenario_id: "stress_b".to_string(),
            family: "mixed_systemic_stress".to_string(),
            start_index: 90,
            end_index: 119,
        },
        ScenarioRowRange {
            scenario_id: "stress_c".to_string(),
            family: "mixed_systemic_stress".to_string(),
            start_index: 150,
            end_index: 179,
        },
        ScenarioRowRange {
            scenario_id: "stress_d".to_string(),
            family: "mixed_systemic_stress".to_string(),
            start_index: 210,
            end_index: 239,
        },
    ];

    let split_requirements = formal_dataset_split_requirements("formal_label_v1_ext_stress");
    let (train_end, calibration_end) =
        scenario_aware_formal_split_bounds(&rows, &ranges, split_requirements).unwrap();

    assert!((42..=59).contains(&train_end));
    assert!((162..=179).contains(&calibration_end));

    let label_support = FormalSplitLabelSupport::from_rows(&rows);
    assert!(label_support.split_has_required_label_support(0, train_end, split_requirements));
    assert!(label_support.split_has_required_label_support(
        train_end,
        calibration_end,
        split_requirements
    ));
    assert!(label_support.split_has_required_label_support(
        calibration_end,
        rows.len(),
        split_requirements
    ));
}

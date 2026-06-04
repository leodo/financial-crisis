#[test]
fn release_review_backtest_comparison_marks_lost_timely_warning() {
    let baseline = vec![
        synthetic_backtest_summary("scenario_a", "Scenario A", Some(20), Some(14), 0),
        synthetic_backtest_summary("scenario_b", "Scenario B", Some(9), None, 1),
    ];
    let candidate = vec![
        synthetic_backtest_summary("scenario_a", "Scenario A", Some(18), None, 2),
        synthetic_backtest_summary("scenario_b", "Scenario B", Some(9), Some(5), 1),
    ];

    let rows = build_release_review_backtest_scenario_comparisons(&baseline, &candidate);

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].scenario_id, "scenario_a");
    assert_eq!(rows[0].outcome, "timely_to_missed");
    assert_eq!(rows[0].actionable_delta_days, None);
    assert_eq!(rows[1].scenario_id, "scenario_b");
    assert_eq!(rows[1].outcome, "missed_to_late_only");
}

#[test]
fn release_review_focus_diagnostic_highlights_missing_actionable_window() {
    let crisis_start = NaiveDate::from_ymd_opt(2023, 3, 10).unwrap();
    let first_l2 = NaiveDate::from_ymd_opt(2022, 12, 17).unwrap();
    let first_l3 = NaiveDate::from_ymd_opt(2022, 12, 30).unwrap();
    let follow_up_1 = NaiveDate::from_ymd_opt(2023, 1, 6).unwrap();
    let follow_up_2 = NaiveDate::from_ymd_opt(2023, 1, 13).unwrap();
    let follow_up_3 = NaiveDate::from_ymd_opt(2023, 1, 20).unwrap();
    let baseline = vec![synthetic_backtest_summary_with_dates(
        "scenario_a",
        "Scenario A",
        Some(first_l2),
        Some(first_l3),
        Some(83),
        Some(70),
        2,
    )];
    let candidate = vec![synthetic_backtest_summary_with_dates(
        "scenario_a",
        "Scenario A",
        Some(first_l2),
        None,
        Some(83),
        None,
        2,
    )];
    let baseline_history = vec![
        runtime_history_point_with_state(
            first_l2,
            56.0,
            0.02,
            0.14,
            0.42,
            DecisionPosture::Prepare,
            TimeToRiskBucket::Months,
            43.0,
            &["prepare_p60d_structural"],
        ),
        runtime_history_point_with_state(
            first_l3,
            62.0,
            0.02,
            0.21,
            0.48,
            DecisionPosture::Prepare,
            TimeToRiskBucket::Months,
            48.0,
            &["prepare_p60d_structural"],
        ),
        runtime_history_point_with_state(
            follow_up_1,
            63.0,
            0.03,
            0.22,
            0.49,
            DecisionPosture::Prepare,
            TimeToRiskBucket::Months,
            48.0,
            &["prepare_p60d_structural"],
        ),
        runtime_history_point_with_state(
            follow_up_2,
            64.0,
            0.04,
            0.23,
            0.50,
            DecisionPosture::Prepare,
            TimeToRiskBucket::Months,
            49.0,
            &["prepare_p60d_structural"],
        ),
        runtime_history_point_with_state(
            follow_up_3,
            60.0,
            0.03,
            0.19,
            0.47,
            DecisionPosture::Prepare,
            TimeToRiskBucket::Months,
            47.0,
            &["prepare_p60d_structural"],
        ),
        runtime_history_point_with_state(
            crisis_start,
            66.0,
            0.08,
            0.31,
            0.52,
            DecisionPosture::Hedge,
            TimeToRiskBucket::Weeks,
            50.0,
            &["hedge_p20d_context"],
        ),
    ];
    let candidate_history = vec![
        runtime_history_point_with_state(
            first_l2,
            55.0,
            0.02,
            0.13,
            0.40,
            DecisionPosture::Prepare,
            TimeToRiskBucket::Months,
            42.0,
            &["prepare_p60d_structural"],
        ),
        runtime_history_point_with_state(
            first_l3,
            57.0,
            0.02,
            0.16,
            0.44,
            DecisionPosture::Prepare,
            TimeToRiskBucket::Months,
            45.0,
            &["prepare_p60d_structural"],
        ),
        runtime_history_point_with_state(
            follow_up_1,
            56.0,
            0.02,
            0.17,
            0.44,
            DecisionPosture::Prepare,
            TimeToRiskBucket::Months,
            45.0,
            &["prepare_p60d_structural"],
        ),
        runtime_history_point_with_state(
            follow_up_2,
            55.0,
            0.02,
            0.16,
            0.43,
            DecisionPosture::Prepare,
            TimeToRiskBucket::Months,
            44.0,
            &["prepare_p60d_structural"],
        ),
        runtime_history_point_with_state(
            follow_up_3,
            63.0,
            0.04,
            0.22,
            0.48,
            DecisionPosture::Prepare,
            TimeToRiskBucket::Months,
            48.0,
            &["prepare_p60d_structural"],
        ),
        runtime_history_point_with_state(
            crisis_start,
            65.0,
            0.08,
            0.31,
            0.50,
            DecisionPosture::Hedge,
            TimeToRiskBucket::Weeks,
            49.0,
            &["hedge_p20d_context"],
        ),
    ];
    let method = formal_main_audit_method_wire();

    let rows = build_release_review_scenario_focus_diagnostics(
        &baseline,
        &candidate,
        &baseline_history,
        &candidate_history,
        &method,
        &method,
    );

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].scenario_id, "scenario_a");
    assert_eq!(rows[0].outcome, "timely_to_missed");
    assert_eq!(rows[0].baseline_first_l3_date, Some(first_l3));
    assert_eq!(rows[0].candidate_first_l3_date, None);
    assert_eq!(rows[0].baseline_actionable_point_count, 4);
    assert_eq!(rows[0].candidate_actionable_point_count, 1);
    assert_eq!(rows[0].baseline_runtime_floor_hit_point_count, 5);
    assert_eq!(rows[0].candidate_runtime_floor_hit_point_count, 5);
    assert_eq!(
        rows[0].baseline_primary_failure_mode.as_deref(),
        Some("strict_gate_mismatch")
    );
    assert_eq!(
        rows[0].candidate_primary_failure_mode.as_deref(),
        Some("strict_gate_mismatch")
    );
    assert_eq!(rows[0].runtime_block_counts.len(), 1);
    assert_eq!(rows[0].runtime_block_counts[0].category, "review_gate_gap");
    assert_eq!(rows[0].runtime_block_counts[0].baseline_count, 1);
    assert_eq!(rows[0].runtime_block_counts[0].candidate_count, 4);
    assert_eq!(
        rows[0].dominant_runtime_blocks.baseline_categories,
        vec!["review_gate_gap".to_string()]
    );
    assert_eq!(rows[0].dominant_runtime_blocks.baseline_count, 1);
    assert_eq!(
        rows[0].dominant_runtime_blocks.candidate_categories,
        vec!["review_gate_gap".to_string()]
    );
    assert_eq!(rows[0].dominant_runtime_blocks.candidate_count, 4);
    assert!(rows[0]
        .dominant_runtime_continuity_facets
        .baseline_categories
        .contains(&"posture:prepare".to_string()));
    assert_eq!(rows[0].dominant_runtime_continuity_facets.baseline_count, 1);
    assert!(rows[0]
        .dominant_runtime_continuity_facets
        .candidate_categories
        .contains(&"posture:prepare".to_string()));
    assert_eq!(
        rows[0].dominant_runtime_continuity_facets.candidate_count,
        4
    );
    assert!(rows[0]
        .runtime_continuity_facet_counts
        .iter()
        .any(|facet| facet.category == "posture:prepare"
            && facet.baseline_count == 1
            && facet.candidate_count == 4));
    assert!(rows[0]
        .runtime_continuity_facet_counts
        .iter()
        .any(|facet| facet.category == "bucket:months"
            && facet.baseline_count == 1
            && facet.candidate_count == 4));
    assert!(rows[0]
        .runtime_continuity_facet_counts
        .iter()
        .any(|facet| facet.category == "trigger:prepare"
            && facet.baseline_count == 1
            && facet.candidate_count == 4));
    assert!(rows[0]
        .runtime_continuity_facet_counts
        .iter()
        .any(|facet| facet.category == "gate_gap:p20d_and_p60d"
            && facet.baseline_count == 1
            && facet.candidate_count == 4));
    assert!(rows[0]
        .runtime_continuity_facet_counts
        .iter()
        .any(|facet| facet.category == "confirmation:months_score_low"
            && facet.baseline_count == 1
            && facet.candidate_count == 4));
    assert_eq!(
        rows[0].candidate_first_runtime_floor_hit_without_l3_date,
        Some(first_l2)
    );
    assert!(rows[0]
        .candidate_first_runtime_floor_hit_without_l3_reason
        .as_deref()
        .is_some_and(|reason| reason.contains("hit runtime floor")));

    let first_l3_point = rows[0]
        .interesting_points
        .iter()
        .find(|point| point.as_of_date == first_l3)
        .expect("first_l3 point should be present");
    assert!(first_l3_point.baseline_actionable);
    assert!(!first_l3_point.candidate_actionable);
    assert!(first_l3_point.baseline_strict_review_actionable);
    assert!(!first_l3_point.candidate_strict_review_actionable);
    assert!(first_l3_point.baseline_runtime_floor_hit);
    assert!(first_l3_point.candidate_runtime_floor_hit);
    assert_eq!(
        first_l3_point.baseline_runtime_actionable_block_category,
        None
    );
    assert_eq!(
        first_l3_point
            .candidate_runtime_actionable_block_category
            .as_deref(),
        Some("review_gate_gap")
    );
    assert_eq!(first_l3_point.baseline_actionable_forward_5d_hits, Some(4));
    assert_eq!(first_l3_point.candidate_actionable_forward_5d_hits, Some(1));
    assert_eq!(first_l3_point.baseline_actionable_sustained, Some(true));
    assert_eq!(first_l3_point.candidate_actionable_sustained, Some(false));
    assert_eq!(
        first_l3_point.baseline_runtime_actionable_block_reason,
        None
    );
    assert!(first_l3_point
        .candidate_runtime_actionable_block_reason
        .as_deref()
        .is_some_and(|reason| reason.contains("hit runtime floor")));
    assert_eq!(
        first_l3_point.baseline_actionable_diagnostic.as_deref(),
        Some("actionable")
    );
    assert!(first_l3_point
        .candidate_actionable_diagnostic
        .as_deref()
        .is_some_and(|reason| reason.contains("hit runtime floor")));
}

#[test]
fn release_review_focus_diagnostic_includes_structural_only_missed_scenarios() {
    let crisis_start = NaiveDate::from_ymd_opt(2023, 3, 10).unwrap();
    let runtime_floor_date = NaiveDate::from_ymd_opt(2023, 2, 10).unwrap();
    let first_l2 = NaiveDate::from_ymd_opt(2023, 2, 20).unwrap();
    let baseline = vec![synthetic_backtest_summary_with_dates(
        "scenario_structural",
        "Structural Only",
        Some(first_l2),
        None,
        Some(18),
        None,
        0,
    )];
    let candidate = vec![synthetic_backtest_summary_with_dates(
        "scenario_structural",
        "Structural Only",
        Some(first_l2),
        None,
        Some(18),
        None,
        0,
    )];
    let shared_history = vec![
        runtime_history_point_with_state(
            runtime_floor_date,
            52.0,
            0.02,
            0.08,
            0.14,
            DecisionPosture::Normal,
            TimeToRiskBucket::Normal,
            41.0,
            &[],
        ),
        runtime_history_point_with_state(
            first_l2,
            54.0,
            0.02,
            0.09,
            0.16,
            DecisionPosture::Normal,
            TimeToRiskBucket::Normal,
            42.0,
            &[],
        ),
        runtime_history_point_with_state(
            crisis_start,
            60.0,
            0.05,
            0.21,
            0.32,
            DecisionPosture::Normal,
            TimeToRiskBucket::Normal,
            44.0,
            &[],
        ),
    ];
    let method = formal_main_audit_method_wire();

    let rows = build_release_review_scenario_focus_diagnostics(
        &baseline,
        &candidate,
        &shared_history,
        &shared_history,
        &method,
        &method,
    );

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].scenario_id, "scenario_structural");
    assert_eq!(rows[0].outcome, "missed_to_missed");
    assert_eq!(rows[0].baseline_runtime_floor_hit_point_count, 2);
    assert_eq!(rows[0].candidate_runtime_floor_hit_point_count, 2);
    assert_eq!(
        rows[0].baseline_primary_failure_mode.as_deref(),
        Some("strict_gate_mismatch")
    );
    assert_eq!(
        rows[0].candidate_primary_failure_mode.as_deref(),
        Some("strict_gate_mismatch")
    );
    assert_eq!(rows[0].runtime_block_counts.len(), 1);
    assert_eq!(rows[0].runtime_block_counts[0].category, "review_gate_gap");
    assert_eq!(rows[0].runtime_block_counts[0].baseline_count, 2);
    assert_eq!(rows[0].runtime_block_counts[0].candidate_count, 2);
    assert_eq!(
        rows[0].dominant_runtime_blocks.baseline_categories,
        vec!["review_gate_gap".to_string()]
    );
    assert_eq!(rows[0].dominant_runtime_blocks.baseline_count, 2);
    assert_eq!(
        rows[0].dominant_runtime_blocks.candidate_categories,
        vec!["review_gate_gap".to_string()]
    );
    assert_eq!(rows[0].dominant_runtime_blocks.candidate_count, 2);
    assert!(rows[0]
        .dominant_runtime_continuity_facets
        .baseline_categories
        .contains(&"posture:normal".to_string()));
    assert_eq!(rows[0].dominant_runtime_continuity_facets.baseline_count, 2);
    assert!(rows[0]
        .runtime_continuity_facet_counts
        .iter()
        .any(|facet| facet.category == "posture:normal"
            && facet.baseline_count == 2
            && facet.candidate_count == 2));
    assert!(rows[0]
        .runtime_continuity_facet_counts
        .iter()
        .any(|facet| facet.category == "bucket:normal"
            && facet.baseline_count == 2
            && facet.candidate_count == 2));
    assert!(rows[0]
        .runtime_continuity_facet_counts
        .iter()
        .any(|facet| facet.category == "trigger:none"
            && facet.baseline_count == 2
            && facet.candidate_count == 2));
    assert_eq!(
        rows[0].baseline_first_runtime_floor_hit_without_l3_date,
        Some(runtime_floor_date)
    );
    assert!(rows[0]
        .baseline_first_runtime_floor_hit_without_l3_reason
        .as_deref()
        .is_some_and(|reason| reason.contains("hit runtime floor")));
    assert!(rows[0]
        .interesting_points
        .iter()
        .any(|point| point.as_of_date == runtime_floor_date));
}

#[test]
fn release_review_focus_diagnostic_counts_posture_continuity_facets() {
    let crisis_start = NaiveDate::from_ymd_opt(2023, 3, 10).unwrap();
    let first_l2 = NaiveDate::from_ymd_opt(2023, 2, 10).unwrap();
    let baseline = vec![synthetic_backtest_summary_with_dates(
        "scenario_posture",
        "Posture Continuity",
        Some(first_l2),
        None,
        Some(28),
        None,
        0,
    )];
    let candidate = baseline.clone();
    let history = vec![
        runtime_history_point_with_state(
            first_l2,
            66.0,
            0.03,
            0.26,
            0.58,
            DecisionPosture::Normal,
            TimeToRiskBucket::Normal,
            52.0,
            &[],
        ),
        runtime_history_point_with_state(
            NaiveDate::from_ymd_opt(2023, 2, 17).unwrap(),
            67.0,
            0.03,
            0.28,
            0.61,
            DecisionPosture::Normal,
            TimeToRiskBucket::Normal,
            54.0,
            &[],
        ),
        runtime_history_point_with_state(
            crisis_start,
            70.0,
            0.05,
            0.31,
            0.64,
            DecisionPosture::Hedge,
            TimeToRiskBucket::Weeks,
            56.0,
            &["hedge_p20d_context"],
        ),
    ];
    let method = formal_main_audit_method_wire();

    let rows = build_release_review_scenario_focus_diagnostics(
        &baseline, &candidate, &history, &history, &method, &method,
    );

    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].baseline_primary_failure_mode.as_deref(),
        Some("posture_continuity_failure")
    );
    assert_eq!(
        rows[0].candidate_primary_failure_mode.as_deref(),
        Some("posture_continuity_failure")
    );
    assert_eq!(rows[0].runtime_block_counts.len(), 1);
    assert_eq!(
        rows[0].runtime_block_counts[0].category,
        "posture_bucket_normal"
    );
    assert_eq!(rows[0].runtime_block_counts[0].baseline_count, 2);
    assert_eq!(rows[0].runtime_block_counts[0].candidate_count, 2);
    assert_eq!(
        rows[0].dominant_runtime_blocks.baseline_categories,
        vec!["posture_bucket_normal".to_string()]
    );
    assert_eq!(rows[0].dominant_runtime_blocks.baseline_count, 2);
    assert_eq!(
        rows[0].dominant_runtime_blocks.candidate_categories,
        vec!["posture_bucket_normal".to_string()]
    );
    assert_eq!(rows[0].dominant_runtime_blocks.candidate_count, 2);
    assert!(rows[0]
        .dominant_runtime_continuity_facets
        .baseline_categories
        .contains(&"posture:normal".to_string()));
    assert_eq!(rows[0].dominant_runtime_continuity_facets.baseline_count, 2);
    assert!(rows[0]
        .runtime_continuity_facet_counts
        .iter()
        .any(|facet| facet.category == "posture:normal"
            && facet.baseline_count == 2
            && facet.candidate_count == 2));
    assert!(rows[0]
        .runtime_continuity_facet_counts
        .iter()
        .any(|facet| facet.category == "bucket:normal"
            && facet.baseline_count == 2
            && facet.candidate_count == 2));
    assert!(rows[0]
        .runtime_continuity_facet_counts
        .iter()
        .any(|facet| facet.category == "trigger:none"
            && facet.baseline_count == 2
            && facet.candidate_count == 2));
    assert!(rows[0]
        .runtime_continuity_facet_counts
        .iter()
        .any(|facet| facet.category == "gate_gap:none"
            && facet.baseline_count == 2
            && facet.candidate_count == 2));
    assert!(rows[0]
        .runtime_continuity_facet_counts
        .iter()
        .any(|facet| facet.category == "confirmation:ok_or_not_needed"
            && facet.baseline_count == 2
            && facet.candidate_count == 2));
}

#[test]
fn release_review_failure_mode_summary_groups_focus_scenarios() {
    let crisis_start = NaiveDate::from_ymd_opt(2023, 3, 10).unwrap();
    let gate_rows = vec![ReleaseReviewScenarioFocusDiagnostic {
        scenario_id: "gate_a".to_string(),
        name: "Gate Mismatch".to_string(),
        outcome: "missed_to_missed".to_string(),
        window_start: crisis_start,
        window_end: crisis_start,
        crisis_start,
        crisis_end: crisis_start,
        baseline_first_l2_date: None,
        candidate_first_l2_date: None,
        baseline_first_l3_date: None,
        candidate_first_l3_date: None,
        baseline_first_non_normal_date: None,
        candidate_first_non_normal_date: None,
        baseline_actionable_point_count: 0,
        candidate_actionable_point_count: 0,
        baseline_runtime_floor_hit_point_count: 2,
        candidate_runtime_floor_hit_point_count: 3,
        baseline_max_p20d: None,
        candidate_max_p20d: None,
        baseline_max_p60d: None,
        candidate_max_p60d: None,
        baseline_first_runtime_floor_hit_without_l3_date: None,
        candidate_first_runtime_floor_hit_without_l3_date: None,
        baseline_first_runtime_floor_hit_without_l3_reason: None,
        candidate_first_runtime_floor_hit_without_l3_reason: None,
        baseline_primary_failure_mode: Some("strict_gate_mismatch".to_string()),
        candidate_primary_failure_mode: Some("strict_gate_mismatch".to_string()),
        dominant_runtime_blocks: ReleaseReviewRuntimeDominantCategories {
            baseline_categories: vec!["review_gate_gap".to_string()],
            baseline_count: 2,
            candidate_categories: vec!["review_gate_gap".to_string()],
            candidate_count: 3,
        },
        dominant_runtime_continuity_facets: ReleaseReviewRuntimeDominantCategories {
            baseline_categories: vec!["gate_gap:p20d_and_p60d".to_string()],
            baseline_count: 2,
            candidate_categories: vec!["gate_gap:p20d_and_p60d".to_string()],
            candidate_count: 3,
        },
        runtime_block_counts: Vec::new(),
        runtime_continuity_facet_counts: Vec::new(),
        interesting_points: Vec::new(),
    }];
    let posture_rows = vec![ReleaseReviewScenarioFocusDiagnostic {
        scenario_id: "posture_a".to_string(),
        name: "Posture Continuity".to_string(),
        outcome: "missed_to_missed".to_string(),
        window_start: crisis_start,
        window_end: crisis_start,
        crisis_start,
        crisis_end: crisis_start,
        baseline_first_l2_date: None,
        candidate_first_l2_date: None,
        baseline_first_l3_date: None,
        candidate_first_l3_date: None,
        baseline_first_non_normal_date: None,
        candidate_first_non_normal_date: None,
        baseline_actionable_point_count: 0,
        candidate_actionable_point_count: 0,
        baseline_runtime_floor_hit_point_count: 2,
        candidate_runtime_floor_hit_point_count: 2,
        baseline_max_p20d: None,
        candidate_max_p20d: None,
        baseline_max_p60d: None,
        candidate_max_p60d: None,
        baseline_first_runtime_floor_hit_without_l3_date: None,
        candidate_first_runtime_floor_hit_without_l3_date: None,
        baseline_first_runtime_floor_hit_without_l3_reason: None,
        candidate_first_runtime_floor_hit_without_l3_reason: None,
        baseline_primary_failure_mode: Some("posture_continuity_failure".to_string()),
        candidate_primary_failure_mode: None,
        dominant_runtime_blocks: ReleaseReviewRuntimeDominantCategories {
            baseline_categories: vec!["posture_bucket_normal".to_string()],
            baseline_count: 2,
            candidate_categories: Vec::new(),
            candidate_count: 0,
        },
        dominant_runtime_continuity_facets: ReleaseReviewRuntimeDominantCategories {
            baseline_categories: vec!["posture:normal".to_string()],
            baseline_count: 2,
            candidate_categories: Vec::new(),
            candidate_count: 0,
        },
        runtime_block_counts: Vec::new(),
        runtime_continuity_facet_counts: Vec::new(),
        interesting_points: Vec::new(),
    }];

    let summary =
        summarize_release_review_failure_modes(&[gate_rows[0].clone(), posture_rows[0].clone()]);

    assert_eq!(summary.len(), 2);
    let strict_gate = summary
        .iter()
        .find(|row| row.failure_mode == "strict_gate_mismatch")
        .expect("strict gate mismatch row");
    assert_eq!(strict_gate.baseline_count, 1);
    assert_eq!(strict_gate.candidate_count, 1);
    assert_eq!(
        strict_gate.baseline_scenarios,
        vec!["Gate Mismatch".to_string()]
    );
    assert_eq!(
        strict_gate.candidate_scenarios,
        vec!["Gate Mismatch".to_string()]
    );
    let posture = summary
        .iter()
        .find(|row| row.failure_mode == "posture_continuity_failure")
        .expect("posture continuity row");
    assert_eq!(posture.baseline_count, 1);
    assert_eq!(posture.candidate_count, 0);
    assert_eq!(
        posture.baseline_scenarios,
        vec!["Posture Continuity".to_string()]
    );
    assert!(posture.candidate_scenarios.is_empty());
}

#[test]
fn release_review_historical_audit_priorities_map_scenarios_to_workstreams() {
    let crisis_start = NaiveDate::from_ymd_opt(2023, 3, 10).unwrap();
    let summary = summarize_release_review_historical_audit_priorities(&[
        ReleaseReviewScenarioFocusDiagnostic {
            scenario_id: "us_dotcom_unwind_2000".to_string(),
            name: "2000-2001 科网泡沫出清".to_string(),
            outcome: "missed_to_missed".to_string(),
            window_start: crisis_start,
            window_end: crisis_start,
            crisis_start,
            crisis_end: crisis_start,
            baseline_first_l2_date: None,
            candidate_first_l2_date: None,
            baseline_first_l3_date: None,
            candidate_first_l3_date: None,
            baseline_first_non_normal_date: None,
            candidate_first_non_normal_date: None,
            baseline_actionable_point_count: 0,
            candidate_actionable_point_count: 0,
            baseline_runtime_floor_hit_point_count: 3,
            candidate_runtime_floor_hit_point_count: 2,
            baseline_max_p20d: None,
            candidate_max_p20d: None,
            baseline_max_p60d: None,
            candidate_max_p60d: None,
            baseline_first_runtime_floor_hit_without_l3_date: None,
            candidate_first_runtime_floor_hit_without_l3_date: None,
            baseline_first_runtime_floor_hit_without_l3_reason: None,
            candidate_first_runtime_floor_hit_without_l3_reason: None,
            baseline_primary_failure_mode: Some("strict_gate_mismatch".to_string()),
            candidate_primary_failure_mode: Some("strict_gate_mismatch".to_string()),
            dominant_runtime_blocks: ReleaseReviewRuntimeDominantCategories {
                baseline_categories: vec!["review_gate_gap".to_string()],
                baseline_count: 3,
                candidate_categories: vec!["review_gate_gap".to_string()],
                candidate_count: 2,
            },
            dominant_runtime_continuity_facets: ReleaseReviewRuntimeDominantCategories {
                baseline_categories: vec!["gate_gap:p20d_and_p60d".to_string()],
                baseline_count: 3,
                candidate_categories: vec!["gate_gap:p20d_and_p60d".to_string()],
                candidate_count: 2,
            },
            runtime_block_counts: Vec::new(),
            runtime_continuity_facet_counts: Vec::new(),
            interesting_points: Vec::new(),
        },
        ReleaseReviewScenarioFocusDiagnostic {
            scenario_id: "us_early_90s_banking_stress".to_string(),
            name: "1990-1993 美国银行与衰退压力".to_string(),
            outcome: "missed_to_missed".to_string(),
            window_start: crisis_start,
            window_end: crisis_start,
            crisis_start,
            crisis_end: crisis_start,
            baseline_first_l2_date: None,
            candidate_first_l2_date: None,
            baseline_first_l3_date: None,
            candidate_first_l3_date: None,
            baseline_first_non_normal_date: None,
            candidate_first_non_normal_date: None,
            baseline_actionable_point_count: 0,
            candidate_actionable_point_count: 0,
            baseline_runtime_floor_hit_point_count: 5,
            candidate_runtime_floor_hit_point_count: 5,
            baseline_max_p20d: None,
            candidate_max_p20d: None,
            baseline_max_p60d: None,
            candidate_max_p60d: None,
            baseline_first_runtime_floor_hit_without_l3_date: None,
            candidate_first_runtime_floor_hit_without_l3_date: None,
            baseline_first_runtime_floor_hit_without_l3_reason: None,
            candidate_first_runtime_floor_hit_without_l3_reason: None,
            baseline_primary_failure_mode: Some("posture_continuity_failure".to_string()),
            candidate_primary_failure_mode: Some("posture_continuity_failure".to_string()),
            dominant_runtime_blocks: ReleaseReviewRuntimeDominantCategories {
                baseline_categories: vec!["posture_bucket_normal".to_string()],
                baseline_count: 5,
                candidate_categories: vec!["posture_bucket_normal".to_string()],
                candidate_count: 5,
            },
            dominant_runtime_continuity_facets: ReleaseReviewRuntimeDominantCategories {
                baseline_categories: vec!["posture:normal".to_string()],
                baseline_count: 5,
                candidate_categories: vec!["posture:normal".to_string()],
                candidate_count: 5,
            },
            runtime_block_counts: Vec::new(),
            runtime_continuity_facet_counts: Vec::new(),
            interesting_points: Vec::new(),
        },
        ReleaseReviewScenarioFocusDiagnostic {
            scenario_id: "us_regional_banks_2023".to_string(),
            name: "2023 美国区域银行危机".to_string(),
            outcome: "timely_to_missed".to_string(),
            window_start: crisis_start,
            window_end: crisis_start,
            crisis_start,
            crisis_end: crisis_start,
            baseline_first_l2_date: None,
            candidate_first_l2_date: None,
            baseline_first_l3_date: None,
            candidate_first_l3_date: None,
            baseline_first_non_normal_date: None,
            candidate_first_non_normal_date: None,
            baseline_actionable_point_count: 0,
            candidate_actionable_point_count: 0,
            baseline_runtime_floor_hit_point_count: 1,
            candidate_runtime_floor_hit_point_count: 0,
            baseline_max_p20d: None,
            candidate_max_p20d: None,
            baseline_max_p60d: None,
            candidate_max_p60d: None,
            baseline_first_runtime_floor_hit_without_l3_date: None,
            candidate_first_runtime_floor_hit_without_l3_date: None,
            baseline_first_runtime_floor_hit_without_l3_reason: None,
            candidate_first_runtime_floor_hit_without_l3_reason: None,
            baseline_primary_failure_mode: Some("residual_review_l3_failure".to_string()),
            candidate_primary_failure_mode: Some("score_confirmation_failure".to_string()),
            dominant_runtime_blocks: ReleaseReviewRuntimeDominantCategories {
                baseline_categories: vec!["review_l3_gate_not_satisfied".to_string()],
                baseline_count: 1,
                candidate_categories: vec!["prepare_score_low".to_string()],
                candidate_count: 1,
            },
            dominant_runtime_continuity_facets: ReleaseReviewRuntimeDominantCategories {
                baseline_categories: vec!["confirmation:ok_or_not_needed".to_string()],
                baseline_count: 1,
                candidate_categories: vec!["confirmation:prepare_score_low".to_string()],
                candidate_count: 1,
            },
            runtime_block_counts: Vec::new(),
            runtime_continuity_facet_counts: Vec::new(),
            interesting_points: Vec::new(),
        },
    ]);

    assert_eq!(summary.len(), 2);
    assert_eq!(summary[0].scenario_id, "us_dotcom_unwind_2000");
    assert_eq!(
        summary[0].primary_workstream,
        "strict_review_vs_runtime_mapping"
    );
    assert_eq!(summary[0].training_role, "candidate_optional");
    assert!(summary[0].protected_window);
    assert!(summary[0]
        .suggested_review
        .contains("strict review gate 与 runtime floor"));

    assert_eq!(summary[1].scenario_id, "us_early_90s_banking_stress");
    assert_eq!(summary[1].primary_workstream, "posture_continuity");
    assert_eq!(summary[1].training_role, "extension_only");
    assert!(summary[1]
        .suggested_review
        .contains("prepare/months 连续性"));
}

#[test]
fn release_review_historical_audit_workstreams_group_priorities() {
    let rows = summarize_release_review_historical_audit_workstreams(&[
        ReleaseReviewHistoricalAuditPriority {
            scenario_id: "us_dotcom_unwind_2000".to_string(),
            scenario_name: "2000-2001 科网泡沫出清".to_string(),
            scenario_family: "mixed_systemic_stress".to_string(),
            training_role: "candidate_optional".to_string(),
            protected_window: true,
            baseline_failure_mode: "strict_gate_mismatch".to_string(),
            candidate_failure_mode: "strict_gate_mismatch".to_string(),
            primary_workstream: "strict_review_vs_runtime_mapping".to_string(),
            suggested_review: "复核 strict review gate 与 runtime floor 的映射".to_string(),
        },
        ReleaseReviewHistoricalAuditPriority {
            scenario_id: "us_early_90s_banking_stress".to_string(),
            scenario_name: "1990-1993 美国银行与衰退压力".to_string(),
            scenario_family: "mixed_systemic_stress".to_string(),
            training_role: "extension_only".to_string(),
            protected_window: true,
            baseline_failure_mode: "posture_continuity_failure".to_string(),
            candidate_failure_mode: "posture_continuity_failure".to_string(),
            primary_workstream: "posture_continuity".to_string(),
            suggested_review: "复核 prepare/months 连续性".to_string(),
        },
        ReleaseReviewHistoricalAuditPriority {
            scenario_id: "us_funding_stress_2011".to_string(),
            scenario_name: "2011 美欧融资压力".to_string(),
            scenario_family: "mixed_systemic_stress".to_string(),
            training_role: "extension_only".to_string(),
            protected_window: true,
            baseline_failure_mode: "posture_continuity_failure".to_string(),
            candidate_failure_mode: "score_confirmation_failure".to_string(),
            primary_workstream: "posture_continuity".to_string(),
            suggested_review: "复核 prepare/months 连续性".to_string(),
        },
    ]);

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].workstream, "strict_review_vs_runtime_mapping");
    assert_eq!(rows[0].scenario_count, 1);
    assert_eq!(rows[0].protected_count, 1);
    assert_eq!(
        rows[0].scenarios,
        vec!["2000-2001 科网泡沫出清".to_string()]
    );
    let posture = rows
        .iter()
        .find(|row| row.workstream == "posture_continuity")
        .expect("posture workstream row");
    assert_eq!(posture.scenario_count, 2);
    assert_eq!(posture.protected_count, 2);
    assert_eq!(
        posture.scenario_families,
        vec!["mixed_systemic_stress".to_string()]
    );
    assert_eq!(posture.training_roles, vec!["extension_only".to_string()]);
    assert!(posture
        .scenarios
        .contains(&"1990-1993 美国银行与衰退压力".to_string()));
    assert!(posture.scenarios.contains(&"2011 美欧融资压力".to_string()));
}

#[test]
fn release_review_historical_audit_attribution_distinguishes_shared_and_regression() {
    let rows = summarize_release_review_historical_audit_attribution(&[
        ReleaseReviewHistoricalAuditPriority {
            scenario_id: "us_dotcom_unwind_2000".to_string(),
            scenario_name: "2000-2001 科网泡沫出清".to_string(),
            scenario_family: "mixed_systemic_stress".to_string(),
            training_role: "candidate_optional".to_string(),
            protected_window: true,
            baseline_failure_mode: "strict_gate_mismatch".to_string(),
            candidate_failure_mode: "strict_gate_mismatch".to_string(),
            primary_workstream: "strict_review_vs_runtime_mapping".to_string(),
            suggested_review: "复核 strict review gate 与 runtime floor 的映射".to_string(),
        },
        ReleaseReviewHistoricalAuditPriority {
            scenario_id: "us_early_90s_banking_stress".to_string(),
            scenario_name: "1990-1993 美国银行与衰退压力".to_string(),
            scenario_family: "mixed_systemic_stress".to_string(),
            training_role: "extension_only".to_string(),
            protected_window: true,
            baseline_failure_mode: "posture_continuity_failure".to_string(),
            candidate_failure_mode: "—".to_string(),
            primary_workstream: "posture_continuity".to_string(),
            suggested_review: "复核 prepare/months 连续性".to_string(),
        },
        ReleaseReviewHistoricalAuditPriority {
            scenario_id: "us_regional_banks_2023".to_string(),
            scenario_name: "2023 美国区域银行危机".to_string(),
            scenario_family: "banking_crisis".to_string(),
            training_role: "mandatory".to_string(),
            protected_window: true,
            baseline_failure_mode: "residual_review_l3_failure".to_string(),
            candidate_failure_mode: "score_confirmation_failure".to_string(),
            primary_workstream: "score_confirmation".to_string(),
            suggested_review: "复核 months/prepare 的 score confirmation".to_string(),
        },
    ]);

    assert_eq!(rows.len(), 3);

    let shared = rows
        .iter()
        .find(|row| row.attribution == "both_baseline_and_candidate")
        .expect("shared row");
    assert_eq!(shared.workstream, "strict_review_vs_runtime_mapping");
    assert_eq!(shared.baseline_count, 1);
    assert_eq!(shared.candidate_count, 1);
    assert!(shared.explanation.contains("共性短板"));

    let baseline_only = rows
        .iter()
        .find(|row| row.attribution == "baseline_shared_weakness")
        .expect("baseline-only row");
    assert_eq!(baseline_only.workstream, "posture_continuity");
    assert_eq!(baseline_only.baseline_count, 1);
    assert_eq!(baseline_only.candidate_count, 0);
    assert!(baseline_only.explanation.contains("既有短板"));

    let regression = rows
        .iter()
        .find(|row| row.attribution == "candidate_regression")
        .expect("candidate regression row");
    assert_eq!(regression.workstream, "score_confirmation");
    assert_eq!(regression.baseline_count, 0);
    assert_eq!(regression.candidate_count, 1);
    assert!(regression.explanation.contains("自己退化出来"));
}

#[test]
fn release_review_historical_audit_actions_translate_attribution_to_next_step() {
    let actions = summarize_release_review_historical_audit_actions(&[
        ReleaseReviewHistoricalAuditAttributionSummary {
            workstream: "score_confirmation".to_string(),
            attribution: "candidate_regression".to_string(),
            scenario_count: 1,
            protected_count: 1,
            baseline_count: 0,
            candidate_count: 1,
            baseline_scenarios: Vec::new(),
            candidate_scenarios: vec!["2023 美国区域银行危机".to_string()],
            explanation: "candidate regression".to_string(),
        },
        ReleaseReviewHistoricalAuditAttributionSummary {
            workstream: "strict_review_vs_runtime_mapping".to_string(),
            attribution: "both_baseline_and_candidate".to_string(),
            scenario_count: 2,
            protected_count: 2,
            baseline_count: 2,
            candidate_count: 2,
            baseline_scenarios: vec![
                "2000-2001 科网泡沫出清".to_string(),
                "2011 美欧融资压力".to_string(),
            ],
            candidate_scenarios: vec![
                "2000-2001 科网泡沫出清".to_string(),
                "2011 美欧融资压力".to_string(),
            ],
            explanation: "shared blocker".to_string(),
        },
        ReleaseReviewHistoricalAuditAttributionSummary {
            workstream: "posture_continuity".to_string(),
            attribution: "baseline_shared_weakness".to_string(),
            scenario_count: 1,
            protected_count: 1,
            baseline_count: 1,
            candidate_count: 0,
            baseline_scenarios: vec!["1990-1993 美国银行与衰退压力".to_string()],
            candidate_scenarios: Vec::new(),
            explanation: "baseline weakness".to_string(),
        },
    ]);

    assert_eq!(actions.len(), 3);
    let candidate = actions
        .iter()
        .find(|row| row.action_type == "candidate_reject_or_retrain")
        .expect("candidate regression action");
    assert_eq!(candidate.workstream, "score_confirmation");
    assert!(candidate.recommendation.contains("不具备晋升条件"));

    let shared = actions
        .iter()
        .find(|row| row.action_type == "shared_blocker_fix_before_promotion")
        .expect("shared blocker action");
    assert_eq!(shared.workstream, "strict_review_vs_runtime_mapping");
    assert!(shared.recommendation.contains("晋升前置 blocker"));

    let baseline = actions
        .iter()
        .find(|row| row.action_type == "baseline_research_fix")
        .expect("baseline research action");
    assert_eq!(baseline.workstream, "posture_continuity");
    assert!(baseline.recommendation.contains("formal main 研究修复"));
}

#[test]
fn release_review_historical_audit_takeaways_explain_primary_workstreams() {
    let takeaways = summarize_release_review_historical_audit_workstreams(&[
        ReleaseReviewHistoricalAuditPriority {
            scenario_id: "us_dotcom_unwind_2000".to_string(),
            scenario_name: "2000-2001 科网泡沫出清".to_string(),
            scenario_family: "mixed_systemic_stress".to_string(),
            training_role: "candidate_optional".to_string(),
            protected_window: true,
            baseline_failure_mode: "strict_gate_mismatch".to_string(),
            candidate_failure_mode: "strict_gate_mismatch".to_string(),
            primary_workstream: "strict_review_vs_runtime_mapping".to_string(),
            suggested_review: "复核 strict review gate 与 runtime floor 的映射".to_string(),
        },
        ReleaseReviewHistoricalAuditPriority {
            scenario_id: "us_early_90s_banking_stress".to_string(),
            scenario_name: "1990-1993 美国银行与衰退压力".to_string(),
            scenario_family: "mixed_systemic_stress".to_string(),
            training_role: "extension_only".to_string(),
            protected_window: true,
            baseline_failure_mode: "posture_continuity_failure".to_string(),
            candidate_failure_mode: "posture_continuity_failure".to_string(),
            primary_workstream: "posture_continuity".to_string(),
            suggested_review: "复核 prepare/months 连续性".to_string(),
        },
    ]);
    let rendered = release_review_historical_audit_takeaways(&takeaways);

    assert_eq!(rendered.len(), 2);
    assert!(rendered
        .iter()
        .any(|row| row.contains("strict review gate 与 runtime floor")));
    assert!(rendered
        .iter()
        .any(|row| row.contains("高 p20d/p60d 仍长期停在 normal")));
}

#[test]
fn release_review_structured_signal_counts_distinguish_strict_and_runtime_hits() {
    let crisis_start = NaiveDate::from_ymd_opt(2023, 3, 10).unwrap();
    let backtests = vec![synthetic_backtest_summary_with_dates(
        "scenario_structural",
        "Structural Only",
        Some(NaiveDate::from_ymd_opt(2023, 2, 20).unwrap()),
        None,
        Some(18),
        None,
        0,
    )];
    let history = vec![
        runtime_history_point_with_state(
            NaiveDate::from_ymd_opt(2023, 2, 10).unwrap(),
            52.0,
            0.02,
            0.08,
            0.14,
            DecisionPosture::Normal,
            TimeToRiskBucket::Normal,
            41.0,
            &[],
        ),
        runtime_history_point_with_state(
            NaiveDate::from_ymd_opt(2023, 2, 20).unwrap(),
            54.0,
            0.02,
            0.09,
            0.16,
            DecisionPosture::Normal,
            TimeToRiskBucket::Normal,
            42.0,
            &[],
        ),
        runtime_history_point_with_state(
            crisis_start,
            60.0,
            0.05,
            0.21,
            0.32,
            DecisionPosture::Normal,
            TimeToRiskBucket::Normal,
            44.0,
            &[],
        ),
    ];
    let method = formal_main_audit_method_wire();

    let (strict_actionable_point_count, runtime_floor_hit_count) =
        release_review_structured_signal_counts(&backtests, &history, &method);

    assert_eq!(strict_actionable_point_count, 0);
    assert_eq!(runtime_floor_hit_count, 2);
}

#[test]
fn release_review_runtime_separation_comparison_highlights_60d_floor_gap() {
    let baseline = ReleaseRuntimeReviewDiagnostics {
        release_id: "baseline".to_string(),
        history_point_count: 120,
        posture_distribution: Vec::new(),
        time_bucket_distribution: Vec::new(),
        posture_trigger_distribution: Vec::new(),
        posture_blocker_distribution: Vec::new(),
        regime_probability_summaries: Vec::new(),
        regime_separation_summaries: vec![ReleaseRuntimeSeparationSummary {
            horizon_days: 60,
            early_warning_regime: "pre_warning_buffer".to_string(),
            normal_avg_probability: 0.28,
            pre_warning_buffer_avg_probability: 0.52,
            positive_window_avg_probability: 0.61,
            in_crisis_avg_probability: 0.66,
            post_crisis_cooldown_avg_probability: 0.35,
            early_warning_raw_lift_vs_normal: Some(1.92),
            early_warning_calibrated_lift_vs_normal: Some(1.86),
            early_warning_gap_retention: Some(0.81),
            positive_window_calibrated_lift_vs_normal: Some(2.18),
            positive_window_gap_vs_normal: Some(0.33),
            in_crisis_raw_lift_vs_normal: Some(2.36),
            in_crisis_calibrated_lift_vs_normal: Some(2.36),
            post_crisis_cooldown_calibrated_lift_vs_normal: Some(1.25),
            post_crisis_cooldown_gap_vs_normal: Some(0.07),
            max_non_normal_calibrated_lift_vs_normal: Some(2.36),
            max_non_normal_threshold_hit_rate: Some(0.0),
            diagnosis: "separated_but_below_runtime_floor".to_string(),
        }],
        runtime_thresholds: Some(RuntimeThresholdDiagnosticsWire {
            prepare_p60d: 0.65,
            hedge_p20d: 0.07,
            defend_p5d: 0.03,
            severe_now_p20d: 0.27,
            elevated_weeks_p60d: 0.20,
            external_prepare_p20d: 0.05,
            carry_prepare_p60d: 0.08,
            downgrade_prepare_p60d: 0.075,
            downgrade_hedge_p20d: 0.053,
            downgrade_defend_p5d: 0.02,
            history_runtime_policy_version: "runtime_history_test".to_string(),
        }),
        points_at_or_above_prepare_p60d: Some(0),
        points_at_or_above_hedge_p20d: Some(14),
        points_at_or_above_defend_p5d: Some(6),
        note: "test".to_string(),
    };
    let candidate = ReleaseRuntimeReviewDiagnostics {
        release_id: "candidate".to_string(),
        history_point_count: 120,
        posture_distribution: Vec::new(),
        time_bucket_distribution: Vec::new(),
        posture_trigger_distribution: Vec::new(),
        posture_blocker_distribution: Vec::new(),
        regime_probability_summaries: Vec::new(),
        regime_separation_summaries: vec![ReleaseRuntimeSeparationSummary {
            horizon_days: 60,
            early_warning_regime: "pre_warning_buffer".to_string(),
            normal_avg_probability: 0.24,
            pre_warning_buffer_avg_probability: 0.58,
            positive_window_avg_probability: 0.64,
            in_crisis_avg_probability: 0.69,
            post_crisis_cooldown_avg_probability: 0.30,
            early_warning_raw_lift_vs_normal: Some(2.48),
            early_warning_calibrated_lift_vs_normal: Some(2.42),
            early_warning_gap_retention: Some(0.88),
            positive_window_calibrated_lift_vs_normal: Some(2.67),
            positive_window_gap_vs_normal: Some(0.40),
            in_crisis_raw_lift_vs_normal: Some(2.88),
            in_crisis_calibrated_lift_vs_normal: Some(2.88),
            post_crisis_cooldown_calibrated_lift_vs_normal: Some(1.25),
            post_crisis_cooldown_gap_vs_normal: Some(0.06),
            max_non_normal_calibrated_lift_vs_normal: Some(2.88),
            max_non_normal_threshold_hit_rate: Some(0.12),
            diagnosis: "usable_early_warning_separation".to_string(),
        }],
        runtime_thresholds: Some(RuntimeThresholdDiagnosticsWire {
            prepare_p60d: 0.45,
            hedge_p20d: 0.07,
            defend_p5d: 0.03,
            severe_now_p20d: 0.27,
            elevated_weeks_p60d: 0.20,
            external_prepare_p20d: 0.05,
            carry_prepare_p60d: 0.08,
            downgrade_prepare_p60d: 0.075,
            downgrade_hedge_p20d: 0.053,
            downgrade_defend_p5d: 0.02,
            history_runtime_policy_version: "runtime_history_test".to_string(),
        }),
        points_at_or_above_prepare_p60d: Some(9),
        points_at_or_above_hedge_p20d: Some(16),
        points_at_or_above_defend_p5d: Some(6),
        note: "test".to_string(),
    };

    let rows = build_release_review_runtime_separation_comparisons(&baseline, &candidate);

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].horizon_days, 60);
    assert_eq!(
        rows[0].baseline_diagnosis,
        "separated_but_below_runtime_floor"
    );
    assert_eq!(
        rows[0].candidate_diagnosis,
        "usable_early_warning_separation"
    );
    assert_eq!(rows[0].baseline_threshold, Some(0.65));
    assert_eq!(rows[0].candidate_threshold, Some(0.45));
    assert_eq!(rows[0].baseline_early_warning_avg_probability, Some(0.52));
    assert_eq!(rows[0].candidate_early_warning_avg_probability, Some(0.58));
    assert_eq!(rows[0].baseline_floor_gap, Some(-0.13));
    assert_eq!(rows[0].candidate_floor_gap, Some(0.13));
    assert_eq!(rows[0].baseline_threshold_hit_rate, Some(0.0));
    assert_eq!(rows[0].candidate_threshold_hit_rate, Some(0.12));
}

#[test]
fn release_review_runtime_separation_takeaways_explain_floor_gap() {
    let rows = vec![ReleaseReviewRuntimeSeparationComparison {
        horizon_days: 60,
        baseline_diagnosis: "usable_early_warning_separation".to_string(),
        candidate_diagnosis: "separated_but_below_runtime_floor".to_string(),
        baseline_threshold: Some(0.45),
        candidate_threshold: Some(0.65),
        baseline_early_warning_regime: "pre_warning_buffer".to_string(),
        candidate_early_warning_regime: "pre_warning_buffer".to_string(),
        baseline_early_warning_avg_probability: Some(0.58),
        candidate_early_warning_avg_probability: Some(0.52),
        baseline_normal_avg_probability: Some(0.24),
        candidate_normal_avg_probability: Some(0.28),
        baseline_early_warning_gap_vs_normal: Some(0.34),
        candidate_early_warning_gap_vs_normal: Some(0.24),
        baseline_floor_gap: Some(0.13),
        candidate_floor_gap: Some(-0.13),
        baseline_early_warning_lift_vs_normal: Some(2.42),
        candidate_early_warning_lift_vs_normal: Some(1.86),
        baseline_threshold_hit_rate: Some(0.12),
        candidate_threshold_hit_rate: Some(0.0),
    }];

    let takeaways = release_review_runtime_separation_takeaways(&rows);

    assert_eq!(takeaways.len(), 1);
    assert!(takeaways[0].contains("60d"));
    assert!(takeaways[0].contains("runtime floor"));
    assert!(takeaways[0].contains("阈值 / runtime policy 瓶颈"));
}

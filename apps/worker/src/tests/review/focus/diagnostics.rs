use super::*;

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
        .any(|facet| facet.category == "gate_gap:p20d_only"
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
fn release_review_focus_diagnostic_includes_runtime_floor_only_scenarios_without_l2() {
    let crisis_start = NaiveDate::from_ymd_opt(2023, 3, 10).unwrap();
    let runtime_floor_date = NaiveDate::from_ymd_opt(2023, 2, 10).unwrap();
    let baseline = vec![synthetic_backtest_summary_with_dates(
        "scenario_runtime_floor_only",
        "Runtime Floor Only",
        None,
        None,
        None,
        None,
        0,
    )];
    let candidate = baseline.clone();
    let history = vec![
        runtime_history_point_with_state(
            runtime_floor_date,
            58.0,
            0.02,
            0.12,
            0.47,
            DecisionPosture::Normal,
            TimeToRiskBucket::Normal,
            41.0,
            &[],
        ),
        runtime_history_point_with_state(
            NaiveDate::from_ymd_opt(2023, 2, 17).unwrap(),
            61.0,
            0.03,
            0.17,
            0.49,
            DecisionPosture::Normal,
            TimeToRiskBucket::Normal,
            43.0,
            &[],
        ),
        runtime_history_point_with_state(
            crisis_start,
            65.0,
            0.06,
            0.23,
            0.52,
            DecisionPosture::Prepare,
            TimeToRiskBucket::Weeks,
            45.0,
            &["hedge_p20d_context"],
        ),
    ];
    let method = formal_main_audit_method_wire();

    let rows = build_release_review_scenario_focus_diagnostics(
        &baseline, &candidate, &history, &history, &method, &method,
    );

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].scenario_id, "scenario_runtime_floor_only");
    assert_eq!(rows[0].outcome, "missed_to_missed");
    assert_eq!(rows[0].baseline_first_l2_date, None);
    assert_eq!(rows[0].candidate_first_l2_date, None);
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
    assert_eq!(
        rows[0].baseline_first_runtime_floor_hit_without_l3_date,
        Some(runtime_floor_date)
    );
    assert_eq!(
        rows[0].candidate_first_runtime_floor_hit_without_l3_date,
        Some(runtime_floor_date)
    );
}

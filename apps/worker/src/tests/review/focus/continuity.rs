use super::*;

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

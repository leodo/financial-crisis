use super::*;

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

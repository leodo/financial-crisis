use super::*;

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

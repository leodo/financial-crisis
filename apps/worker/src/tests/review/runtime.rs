use super::*;

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
fn release_review_structured_signal_counts_accept_relaxed_strict_p20d_mapping() {
    let crisis_start = NaiveDate::from_ymd_opt(2023, 3, 10).unwrap();
    let backtests = vec![synthetic_backtest_summary_with_dates(
        "scenario_prepare",
        "Prepare Window",
        Some(NaiveDate::from_ymd_opt(2023, 2, 20).unwrap()),
        Some(NaiveDate::from_ymd_opt(2023, 2, 20).unwrap()),
        Some(18),
        Some(18),
        1,
    )];
    let history = vec![runtime_history_point_with_state(
        NaiveDate::from_ymd_opt(2023, 2, 20).unwrap(),
        57.0,
        0.02,
        0.13,
        0.48,
        DecisionPosture::Prepare,
        TimeToRiskBucket::Months,
        45.0,
        &["prepare_p60d_structural"],
    )];
    let method = formal_main_audit_method_wire();

    let (strict_actionable_point_count, runtime_floor_hit_count) =
        release_review_structured_signal_counts(&backtests, &history, &method);

    assert_eq!(strict_actionable_point_count, 1);
    assert_eq!(runtime_floor_hit_count, 1);
    assert!(crisis_start > history[0].as_of_date);
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

#[test]
fn release_review_structured_signal_counts_accept_probability_plateau_clause_for_formal_main() {
    let crisis_start = NaiveDate::from_ymd_opt(2023, 3, 10).unwrap();
    let plateau_date = NaiveDate::from_ymd_opt(2023, 2, 1).unwrap();
    let backtests = vec![synthetic_backtest_summary_with_dates(
        "scenario_probability_plateau",
        "Probability Plateau",
        Some(plateau_date),
        Some(plateau_date),
        Some(56),
        Some(56),
        0,
    )];
    let history = vec![
        runtime_history_point_with_state(
            plateau_date,
            44.4,
            0.03,
            0.905,
            0.892,
            DecisionPosture::Prepare,
            TimeToRiskBucket::Months,
            48.2,
            &["prepare_probability_plateau"],
        ),
        runtime_history_point_with_state(
            crisis_start,
            62.0,
            0.05,
            0.35,
            0.52,
            DecisionPosture::Hedge,
            TimeToRiskBucket::Weeks,
            52.0,
            &["hedge_p20d_context"],
        ),
    ];
    let method = formal_main_audit_method_wire();

    let (strict_actionable_point_count, runtime_floor_hit_count) =
        release_review_structured_signal_counts(&backtests, &history, &method);

    assert_eq!(strict_actionable_point_count, 1);
    assert_eq!(runtime_floor_hit_count, 1);
}

#[test]
fn release_review_structured_signal_counts_accept_relaxed_probability_plateau_clause() {
    let plateau_date = NaiveDate::from_ymd_opt(2023, 2, 1).unwrap();
    let backtests = vec![synthetic_backtest_summary_with_dates(
        "scenario_probability_plateau_relaxed",
        "Relaxed Probability Plateau",
        Some(plateau_date),
        Some(plateau_date),
        Some(74),
        Some(74),
        0,
    )];
    let history = vec![runtime_history_point_with_state(
        plateau_date,
        44.4,
        0.03,
        0.50,
        0.67,
        DecisionPosture::Prepare,
        TimeToRiskBucket::Months,
        40.7,
        &["prepare_probability_plateau"],
    )];
    let mut method = formal_main_audit_method_wire();
    method.runtime_thresholds = Some(RuntimeThresholdDiagnosticsWire {
        prepare_p60d: 0.568,
        hedge_p20d: 0.282,
        defend_p5d: 0.05,
        severe_now_p20d: 0.564,
        elevated_weeks_p60d: 0.909,
        external_prepare_p20d: 0.197,
        carry_prepare_p60d: 0.454,
        downgrade_prepare_p60d: 0.426,
        downgrade_hedge_p20d: 0.212,
        downgrade_defend_p5d: 0.034,
        history_runtime_policy_version: "runtime_history_test".to_string(),
    });

    let (strict_actionable_point_count, runtime_floor_hit_count) =
        release_review_structured_signal_counts(&backtests, &history, &method);

    assert_eq!(strict_actionable_point_count, 1);
    assert_eq!(runtime_floor_hit_count, 1);
}

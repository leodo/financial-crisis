use super::*;
use fc_domain::{
    HorizonEvaluationSummary, LogisticProbabilityModel, ProbabilityHorizonBundle,
    ProbabilityThresholdDecisionSummary, ProbabilityThresholdDiagnostics,
};

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

#[test]
fn probability_guardrails_reject_medium_horizon_threshold_above_positive_window_average() {
    let regime_summary = RegimeSeparationEvaluationSummary {
        horizon_days: 20,
        early_warning_regime: "pre_warning_buffer".to_string(),
        normal_sample_count: 50,
        pre_warning_buffer_sample_count: 10,
        positive_window_sample_count: 10,
        early_warning_sample_count: 10,
        in_crisis_sample_count: 10,
        post_crisis_cooldown_sample_count: 10,
        normal_avg_probability: 0.128,
        pre_warning_buffer_avg_probability: 0.834,
        positive_window_avg_probability: 0.8502,
        early_warning_avg_probability: 0.834,
        in_crisis_avg_probability: 0.901,
        post_crisis_cooldown_avg_probability: 0.333,
        max_non_normal_avg_probability: 0.901,
        pre_warning_buffer_lift_vs_normal: Some(6.516),
        positive_window_lift_vs_normal: Some(6.642),
        early_warning_lift_vs_normal: Some(6.516),
        in_crisis_lift_vs_normal: Some(7.039),
        post_crisis_cooldown_lift_vs_normal: Some(2.602),
        positive_window_gap_vs_normal: Some(0.7222),
        post_crisis_cooldown_gap_vs_normal: Some(0.205),
        max_non_normal_lift_vs_normal: Some(7.039),
        diagnosis: "usable_early_warning_separation".to_string(),
    };
    let horizon_bundle = ProbabilityHorizonBundle {
        horizon_days: 20,
        decision_threshold: Some(0.878),
        threshold_diagnostics: Some(ProbabilityThresholdDiagnostics {
            label_mode: "forward_crisis".to_string(),
            early_warning_regime: "pre_warning_buffer".to_string(),
            full_calibration_row_count: 100,
            eligible_row_count: 100,
            eligible_positive_count: 20,
            eligible_negative_count: 80,
            used_full_split_fallback: false,
            selected_row_count: 100,
            selected_positive_count: 20,
            selected_negative_count: 80,
            selected_used_full_split_fallback: false,
            base_threshold: 0.878,
            final_threshold: 0.878,
            repair_applied: false,
            repair_eligible: true,
            repair_reason: "base_threshold_has_usable_early_warning_gap".to_string(),
            early_warning_probability_cap: Some(0.912),
            prediction_ceiling: Some(80),
            relaxed_prediction_ceiling: Some(160),
            base_summary: ProbabilityThresholdDecisionSummary {
                predicted_positive_count: 18,
                true_positive_count: 14,
                precision: 0.778,
                recall: 0.700,
                early_warning_row_count: 10,
                early_warning_hit_count: 4,
                early_warning_hit_rate: 0.400,
                normal_row_count: 50,
                normal_hit_count: 0,
                normal_hit_rate: 0.0,
                positive_window_row_count: 10,
                positive_window_hit_count: 4,
                positive_window_hit_rate: 0.400,
                in_crisis_row_count: 10,
                in_crisis_hit_count: 10,
                in_crisis_hit_rate: 1.0,
                cooldown_row_count: 10,
                cooldown_hit_count: 0,
                cooldown_hit_rate: 0.0,
            },
            final_summary: ProbabilityThresholdDecisionSummary {
                predicted_positive_count: 18,
                true_positive_count: 14,
                precision: 0.778,
                recall: 0.700,
                early_warning_row_count: 10,
                early_warning_hit_count: 4,
                early_warning_hit_rate: 0.400,
                normal_row_count: 50,
                normal_hit_count: 0,
                normal_hit_rate: 0.0,
                positive_window_row_count: 10,
                positive_window_hit_count: 4,
                positive_window_hit_rate: 0.400,
                in_crisis_row_count: 10,
                in_crisis_hit_count: 10,
                in_crisis_hit_rate: 1.0,
                cooldown_row_count: 10,
                cooldown_hit_count: 0,
                cooldown_hit_rate: 0.0,
            },
            calibration_regime_evidence: Vec::new(),
        }),
        raw_model: LogisticProbabilityModel {
            intercept: 0.0,
            feature_transform: PROBABILITY_FEATURE_TRANSFORM_IDENTITY_V1.to_string(),
            feature_stats: Vec::new(),
            coefficients: Vec::new(),
        },
        calibration: None,
        evaluation: HorizonEvaluationSummary {
            sample_count: 100,
            positive_rate: 0.2,
            brier_score: 0.1,
            log_loss: 0.2,
            ece: 0.1,
            precision_at_30pct: None,
            recall_at_30pct: None,
            regime_separation: Some(regime_summary.clone()),
            actionability: None,
        },
        family_overlays: Vec::new(),
        family_overlay_audits: Vec::new(),
    };
    let bundle = ProbabilityBundle {
        bundle_id: "candidate_guard_threshold_above_positive_window".to_string(),
        market_scope: "financial_system".to_string(),
        probability_mode: "formal_bundle_v1".to_string(),
        model_family: PROBABILITY_MODEL_FAMILY_LINEAR_V1.to_string(),
        feature_transform: PROBABILITY_FEATURE_TRANSFORM_IDENTITY_V1.to_string(),
        created_at: Utc::now(),
        feature_names: Vec::new(),
        monotonic_min_gap_5d_to_20d: 0.0,
        monotonic_min_gap_20d_to_60d: 0.0,
        note: "test".to_string(),
        horizons: vec![horizon_bundle],
        evaluation: Some(ProbabilityBundleEvaluation {
            sample_count: 100,
            brier_score: 0.1,
            log_loss: 0.2,
            ece: 0.1,
            regime_separation_summaries: vec![regime_summary],
            usable_early_warning_horizon_count: 1,
            insufficient_early_warning_horizon_count: 0,
            note: "test".to_string(),
        }),
        actionability: None,
    };
    let release = test_release_with_bundle(&bundle);
    let bundle_path = release.manifest.bundle_uri.clone();

    let regressions = compare_probability_guardrails(&release).unwrap();

    let _ = std::fs::remove_file(bundle_path);
    assert!(regressions.iter().any(|item| {
        item.contains("20d decision threshold") && item.contains("positive_window avg")
    }));
}

#[test]
fn probability_guardrails_reject_medium_horizon_threshold_when_cooldown_hits_match_positive_window()
{
    let regime_summary = RegimeSeparationEvaluationSummary {
        horizon_days: 60,
        early_warning_regime: "pre_warning_buffer".to_string(),
        normal_sample_count: 50,
        pre_warning_buffer_sample_count: 10,
        positive_window_sample_count: 10,
        early_warning_sample_count: 10,
        in_crisis_sample_count: 10,
        post_crisis_cooldown_sample_count: 10,
        normal_avg_probability: 0.03,
        pre_warning_buffer_avg_probability: 0.09,
        positive_window_avg_probability: 0.12,
        early_warning_avg_probability: 0.09,
        in_crisis_avg_probability: 0.16,
        post_crisis_cooldown_avg_probability: 0.08,
        max_non_normal_avg_probability: 0.16,
        pre_warning_buffer_lift_vs_normal: Some(3.0),
        positive_window_lift_vs_normal: Some(4.0),
        early_warning_lift_vs_normal: Some(3.0),
        in_crisis_lift_vs_normal: Some(5.333),
        post_crisis_cooldown_lift_vs_normal: Some(2.667),
        positive_window_gap_vs_normal: Some(0.09),
        post_crisis_cooldown_gap_vs_normal: Some(0.05),
        max_non_normal_lift_vs_normal: Some(5.333),
        diagnosis: "usable_early_warning_separation".to_string(),
    };
    let horizon_bundle = ProbabilityHorizonBundle {
        horizon_days: 60,
        decision_threshold: Some(0.11),
        threshold_diagnostics: Some(ProbabilityThresholdDiagnostics {
            label_mode: "forward_crisis".to_string(),
            early_warning_regime: "pre_warning_buffer".to_string(),
            full_calibration_row_count: 100,
            eligible_row_count: 100,
            eligible_positive_count: 20,
            eligible_negative_count: 80,
            used_full_split_fallback: false,
            selected_row_count: 100,
            selected_positive_count: 20,
            selected_negative_count: 80,
            selected_used_full_split_fallback: false,
            base_threshold: 0.11,
            final_threshold: 0.11,
            repair_applied: false,
            repair_eligible: true,
            repair_reason: "base_threshold_has_usable_early_warning_gap".to_string(),
            early_warning_probability_cap: Some(0.14),
            prediction_ceiling: Some(100),
            relaxed_prediction_ceiling: Some(300),
            base_summary: ProbabilityThresholdDecisionSummary {
                predicted_positive_count: 22,
                true_positive_count: 15,
                precision: 0.682,
                recall: 0.750,
                early_warning_row_count: 10,
                early_warning_hit_count: 5,
                early_warning_hit_rate: 0.500,
                normal_row_count: 50,
                normal_hit_count: 0,
                normal_hit_rate: 0.0,
                positive_window_row_count: 10,
                positive_window_hit_count: 4,
                positive_window_hit_rate: 0.400,
                in_crisis_row_count: 10,
                in_crisis_hit_count: 10,
                in_crisis_hit_rate: 1.0,
                cooldown_row_count: 10,
                cooldown_hit_count: 4,
                cooldown_hit_rate: 0.400,
            },
            final_summary: ProbabilityThresholdDecisionSummary {
                predicted_positive_count: 22,
                true_positive_count: 15,
                precision: 0.682,
                recall: 0.750,
                early_warning_row_count: 10,
                early_warning_hit_count: 5,
                early_warning_hit_rate: 0.500,
                normal_row_count: 50,
                normal_hit_count: 0,
                normal_hit_rate: 0.0,
                positive_window_row_count: 10,
                positive_window_hit_count: 4,
                positive_window_hit_rate: 0.400,
                in_crisis_row_count: 10,
                in_crisis_hit_count: 10,
                in_crisis_hit_rate: 1.0,
                cooldown_row_count: 10,
                cooldown_hit_count: 4,
                cooldown_hit_rate: 0.400,
            },
            calibration_regime_evidence: Vec::new(),
        }),
        raw_model: LogisticProbabilityModel {
            intercept: 0.0,
            feature_transform: PROBABILITY_FEATURE_TRANSFORM_IDENTITY_V1.to_string(),
            feature_stats: Vec::new(),
            coefficients: Vec::new(),
        },
        calibration: None,
        evaluation: HorizonEvaluationSummary {
            sample_count: 100,
            positive_rate: 0.2,
            brier_score: 0.1,
            log_loss: 0.2,
            ece: 0.1,
            precision_at_30pct: None,
            recall_at_30pct: None,
            regime_separation: Some(regime_summary.clone()),
            actionability: None,
        },
        family_overlays: Vec::new(),
        family_overlay_audits: Vec::new(),
    };
    let bundle = ProbabilityBundle {
        bundle_id: "candidate_guard_threshold_cooldown_hits".to_string(),
        market_scope: "financial_system".to_string(),
        probability_mode: "formal_bundle_v1".to_string(),
        model_family: PROBABILITY_MODEL_FAMILY_LINEAR_V1.to_string(),
        feature_transform: PROBABILITY_FEATURE_TRANSFORM_IDENTITY_V1.to_string(),
        created_at: Utc::now(),
        feature_names: Vec::new(),
        monotonic_min_gap_5d_to_20d: 0.0,
        monotonic_min_gap_20d_to_60d: 0.0,
        note: "test".to_string(),
        horizons: vec![horizon_bundle],
        evaluation: Some(ProbabilityBundleEvaluation {
            sample_count: 100,
            brier_score: 0.1,
            log_loss: 0.2,
            ece: 0.1,
            regime_separation_summaries: vec![regime_summary],
            usable_early_warning_horizon_count: 1,
            insufficient_early_warning_horizon_count: 0,
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
        .any(|item| { item.contains("60d threshold hit rates") && item.contains("cooldown") }));
}

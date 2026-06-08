use std::path::PathBuf;

use chrono::{NaiveDate, Utc};
use fc_domain::{
    AssessmentMethodVersions, AssessmentScores, AssessmentSnapshot, BacktestPerformanceSummary,
    BacktestRollingAudit, DataMode, DataQualitySummary, DataTrust, DecisionPosture,
    EventAssessment, EventConfirmationState, JpyCarrySnapshot, JpyCarryState, ModelReleaseManifest,
    ModelReleaseRecord, PositionGuidance, PositionGuidanceGovernance, ProbabilityBlock,
    ProbabilityDiagnostics, QualityGrade, RuntimeMetadata, TimeToRiskBucket, UserRiskPreferences,
    UserRiskProfile,
};

fn test_release_record(
    release_id: &str,
    probability_mode: &str,
    point_in_time_mode: &str,
) -> ModelReleaseRecord {
    ModelReleaseRecord {
        manifest: ModelReleaseManifest {
            release_id: release_id.to_string(),
            market_scope: "financial_system".to_string(),
            status: "approved".to_string(),
            probability_mode: probability_mode.to_string(),
            serving_status: "healthy".to_string(),
            bundle_uri: format!("bundles/{release_id}.json"),
            feature_set_version: crate::DEFAULT_FORMAL_FEATURE_SET_VERSION.to_string(),
            label_version: crate::DEFAULT_FORMAL_LABEL_VERSION.to_string(),
            prob_model_version: "prob_v1".to_string(),
            calibration_version: "calib_v1".to_string(),
            posture_policy_version: "posture_v1".to_string(),
            action_playbook_version: "playbook_v1".to_string(),
            point_in_time_mode: point_in_time_mode.to_string(),
            training_range_start: None,
            training_range_end: None,
            calibration_range_start: None,
            calibration_range_end: None,
            evaluation_range_start: None,
            evaluation_range_end: None,
            brier_score: Some(0.12),
            log_loss: Some(0.23),
            ece: Some(0.04),
            note: "test".to_string(),
        },
        created_at: Utc::now(),
        activated_at: None,
        retired_at: None,
    }
}

fn test_assessment_snapshot(
    release_id: &str,
    probability_mode: &str,
    point_in_time_mode: &str,
    posture: DecisionPosture,
    time_to_risk_bucket: TimeToRiskBucket,
    p_20d: f64,
    p_60d: f64,
) -> AssessmentSnapshot {
    let as_of_date = NaiveDate::from_ymd_opt(2026, 6, 8).unwrap();
    AssessmentSnapshot {
        as_of_date,
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        probabilities: ProbabilityBlock {
            p_5d: 0.01,
            p_20d,
            p_60d,
        },
        actionability: fc_domain::ActionabilityBlock {
            prepare: 0.08,
            hedge: 0.03,
            defend: 0.01,
        },
        probability_diagnostics: ProbabilityDiagnostics::default(),
        time_to_risk_bucket,
        posture,
        conviction_score: 0.52,
        scores: AssessmentScores {
            overall_score: 54.0,
            structural_score: 49.0,
            trigger_score: 42.0,
            external_shock_score: 33.0,
        },
        summary: "test assessment".to_string(),
        posture_reason: "test posture reason".to_string(),
        top_risk_drivers: Vec::new(),
        top_relief_drivers: Vec::new(),
        historical_analogs: Vec::new(),
        data_trust: DataTrust {
            coverage_score: 0.97,
            core_feature_coverage: 1.0,
            trigger_feature_coverage: 0.92,
            external_feature_coverage: 0.98,
            quality_grade: QualityGrade::A,
            data_quality_summary: DataQualitySummary {
                overall_score: 90.0,
                grade: QualityGrade::A,
                stale_indicator_count: 0,
                low_quality_indicator_count: 0,
                prototype_source_count: 0,
                blocked_indicator_count: 0,
            },
            warnings: Vec::new(),
        },
        jpy_carry: JpyCarrySnapshot {
            state: JpyCarryState::Quiet,
            score: 12.4,
            usdjpy_level: Some(159.3),
            jp_call_rate: Some(0.7),
            us_short_rate: Some(3.6),
            us_jp_short_rate_diff: Some(2.9),
            change_5d: Some(0.4),
            change_20d: Some(0.0),
            realized_vol_20d: Some(0.0),
            funding_pressure_score: 34.7,
            vix_coupling_score: 24.6,
            credit_coupling_score: 18.0,
            reason: "test carry state".to_string(),
        },
        position_guidance: PositionGuidance {
            action_playbook_version: "playbook_v1".to_string(),
            execution_urgency: "observe".to_string(),
            confidence_gate: "manual_confirmation".to_string(),
            target_equity_exposure_pct: 70.0,
            target_cash_pct: 15.0,
            hedge_ratio_pct: 0.0,
            leverage_cap_pct: 100.0,
            option_overlay_pct: 5.0,
            action_summary: "maintain core exposure".to_string(),
            actions: vec!["keep monitoring".to_string()],
            forbidden_actions: vec!["do not auto trade".to_string()],
            reentry_conditions: vec!["wait for review".to_string()],
            guardrails: vec!["manual confirmation".to_string()],
            capital_preservation_overlay_enabled: false,
            governance: PositionGuidanceGovernance::default(),
        },
        runtime: RuntimeMetadata {
            data_mode: DataMode::Sqlite,
            generated_at: Utc::now(),
            requested_as_of_date: as_of_date,
            latest_observation_at: Some(as_of_date),
            latest_observation_lag_days: Some(0),
            demo_mode: false,
            stale_warning: None,
        },
        key_indicators: Vec::new(),
        event_assessment: EventAssessment {
            state: EventConfirmationState::Quiet,
            confirmation_score: 0.0,
            recent_event_count: 0,
            summary: "quiet".to_string(),
            confirmed_signals: Vec::new(),
            pending_gaps: Vec::new(),
            recent_events: Vec::new(),
        },
        backtest_summary: BacktestPerformanceSummary {
            scenario_count: 1,
            real_scenario_count: 1,
            fallback_scenario_count: 0,
            coverage_scope_note: "test coverage scope".to_string(),
            structural_warning_rate: 0.7,
            timely_warning_rate: 0.375,
            missed_rate: 0.625,
            avg_structural_lead_time_days: Some(12.0),
            avg_lead_time_days: Some(7.0),
            median_lead_time_days: Some(6.0),
            total_false_positive_count: 3,
            history_start: Some(NaiveDate::from_ymd_opt(1990, 1, 1).unwrap()),
            history_end: Some(as_of_date),
            rolling_audit: BacktestRollingAudit {
                history_start: Some(NaiveDate::from_ymd_opt(1990, 1, 1).unwrap()),
                history_end: Some(as_of_date),
                history_point_count: 260,
                scope_note: "test rolling audit scope".to_string(),
                actionable_signal_count: 12,
                pre_crisis_signal_count: 6,
                in_crisis_signal_count: 4,
                stress_window_signal_count: 2,
                false_positive_signal_count: 3,
                false_positive_episode_count: 1,
                longest_false_positive_episode_days: 9,
                actionable_precision: 0.296,
                classified_episodes: Vec::new(),
                summary: "test rolling audit".to_string(),
            },
            summary: "test backtest summary".to_string(),
        },
        user_preferences: UserRiskPreferences {
            profile: UserRiskProfile::Neutral,
            cash_floor_pct: 15.0,
            max_equity_cap_pct: 70.0,
            max_leverage_pct: 100.0,
            option_overlay_preference_pct: 5.0,
            allow_aggressive_reentry: false,
            note: "test preferences".to_string(),
        },
        method: AssessmentMethodVersions {
            score_method_version: "score_v1".to_string(),
            prob_model_version: "prob_v1".to_string(),
            calibration_version: "calib_v1".to_string(),
            actionability_model_version: None,
            actionability_calibration_version: None,
            feature_set_version: crate::DEFAULT_FORMAL_FEATURE_SET_VERSION.to_string(),
            label_version: crate::DEFAULT_FORMAL_LABEL_VERSION.to_string(),
            posture_policy_version: "posture_v1".to_string(),
            action_playbook_version: "playbook_v1".to_string(),
            fusion_policy_version: None,
            actionability_enabled: false,
            probability_mode: probability_mode.to_string(),
            release_status: "approved".to_string(),
            release_id: Some(release_id.to_string()),
            point_in_time_mode: point_in_time_mode.to_string(),
        },
    }
}

fn test_runtime_review(
    release_id: &str,
    prepare_floor_hits: usize,
) -> crate::ReleaseRuntimeReviewDiagnostics {
    crate::ReleaseRuntimeReviewDiagnostics {
        release_id: release_id.to_string(),
        history_point_count: 260,
        posture_distribution: vec![crate::release_review::ReleaseRuntimeCount {
            name: "normal".to_string(),
            count: 240,
        }],
        time_bucket_distribution: vec![crate::release_review::ReleaseRuntimeCount {
            name: "normal".to_string(),
            count: 240,
        }],
        posture_trigger_distribution: Vec::new(),
        posture_blocker_distribution: Vec::new(),
        regime_probability_summaries: Vec::new(),
        regime_separation_summaries: Vec::new(),
        runtime_thresholds: Some(crate::RuntimeThresholdDiagnosticsWire {
            prepare_p60d: 0.56,
            hedge_p20d: 0.36,
            defend_p5d: 0.05,
            severe_now_p20d: 0.27,
            elevated_weeks_p60d: 0.20,
            external_prepare_p20d: 0.05,
            carry_prepare_p60d: 0.08,
            downgrade_prepare_p60d: 0.075,
            downgrade_hedge_p20d: 0.053,
            downgrade_defend_p5d: 0.02,
            history_runtime_policy_version: "runtime_history_test".to_string(),
        }),
        points_at_or_above_prepare_p60d: Some(prepare_floor_hits),
        points_at_or_above_hedge_p20d: Some(4),
        points_at_or_above_defend_p5d: Some(1),
        note: "test runtime review".to_string(),
    }
}

fn test_actionability_review(release_id: &str) -> crate::ReleaseActionabilityReview {
    crate::ReleaseActionabilityReview {
        release_id: release_id.to_string(),
        enabled: false,
        model_version: None,
        calibration_version: None,
        fusion_policy_version: None,
        levels: Vec::new(),
        guard_regressions: Vec::new(),
        guard_passed: true,
        note: "actionability disabled for test".to_string(),
    }
}

fn sample_release_review_report() -> crate::ReleaseReviewEnvelope {
    let baseline_release = test_release_record(
        "us_formal_family_hybrid_20260605T202246",
        "formal_bundle_v1",
        "raw_feature_replay",
    );
    let candidate_release = test_release_record(
        "us_formal_family_hybrid_20260606T112926",
        "formal_bundle_v1",
        "raw_feature_replay",
    );
    crate::ReleaseReviewEnvelope {
        reviewed_at: "2026-06-08T15:12:00Z".to_string(),
        market_scope: "financial_system".to_string(),
        api_reload_url: crate::DEFAULT_API_RELOAD_URL.to_string(),
        history_mode: "strict_rebuild".to_string(),
        history_limit: 260,
        original_active_release_id: baseline_release.manifest.release_id.clone(),
        restored_release_id: baseline_release.manifest.release_id.clone(),
        baseline_release: baseline_release.clone(),
        candidate_release: candidate_release.clone(),
        baseline_assessment: test_assessment_snapshot(
            &baseline_release.manifest.release_id,
            &baseline_release.manifest.probability_mode,
            &baseline_release.manifest.point_in_time_mode,
            DecisionPosture::Prepare,
            TimeToRiskBucket::Months,
            0.22,
            0.58,
        ),
        candidate_assessment: test_assessment_snapshot(
            &candidate_release.manifest.release_id,
            &candidate_release.manifest.probability_mode,
            &candidate_release.manifest.point_in_time_mode,
            DecisionPosture::Normal,
            TimeToRiskBucket::Normal,
            0.11,
            0.52,
        ),
        baseline_runtime_review: test_runtime_review(&baseline_release.manifest.release_id, 9),
        candidate_runtime_review: test_runtime_review(&candidate_release.manifest.release_id, 7),
        baseline_actionability_review: test_actionability_review(
            &baseline_release.manifest.release_id,
        ),
        candidate_actionability_review: test_actionability_review(
            &candidate_release.manifest.release_id,
        ),
        scenario_focus: Vec::new(),
        historical_audit_workstreams: Vec::new(),
        historical_audit_priorities: Vec::new(),
        historical_audit_attribution: Vec::new(),
        historical_audit_actions: Vec::new(),
        comparison: crate::ReleaseReviewComparisonSummary {
            timely_warning_rate: crate::ReleaseReviewScalarMetric {
                baseline: 0.375,
                candidate: 0.125,
                delta: -0.25,
            },
            strict_actionable_point_count: crate::ReleaseReviewCountMetric {
                baseline: 6,
                candidate: 2,
                delta: -4,
            },
            runtime_floor_hit_count: crate::ReleaseReviewCountMetric {
                baseline: 9,
                candidate: 7,
                delta: -2,
            },
            actionable_precision: crate::ReleaseReviewScalarMetric {
                baseline: 0.296,
                candidate: 0.206,
                delta: -0.09,
            },
            longest_false_positive_episode_days: crate::ReleaseReviewCountMetric {
                baseline: 9,
                candidate: 18,
                delta: 9,
            },
            current_p_5d: crate::ReleaseReviewScalarMetric {
                baseline: 0.03,
                candidate: 0.01,
                delta: -0.02,
            },
            current_p_20d: crate::ReleaseReviewScalarMetric {
                baseline: 0.22,
                candidate: 0.11,
                delta: -0.11,
            },
            current_p_60d: crate::ReleaseReviewScalarMetric {
                baseline: 0.58,
                candidate: 0.52,
                delta: -0.06,
            },
            runtime_separation_summary: vec![crate::ReleaseReviewRuntimeSeparationComparison {
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
            }],
            backtest_scenarios: vec![crate::ReleaseReviewBacktestScenarioComparison {
                scenario_id: "us_regional_banks_2023".to_string(),
                name: "2023 美国区域银行危机".to_string(),
                signal_source: "real_history".to_string(),
                crisis_start: NaiveDate::from_ymd_opt(2023, 3, 10).unwrap(),
                crisis_end: NaiveDate::from_ymd_opt(2023, 3, 20).unwrap(),
                baseline_first_l2_date: Some(NaiveDate::from_ymd_opt(2023, 2, 20).unwrap()),
                candidate_first_l2_date: Some(NaiveDate::from_ymd_opt(2023, 2, 24).unwrap()),
                baseline_first_l3_date: Some(NaiveDate::from_ymd_opt(2023, 2, 21).unwrap()),
                candidate_first_l3_date: None,
                baseline_lead_time_days: Some(19),
                candidate_lead_time_days: Some(15),
                baseline_actionable_lead_time_days: Some(18),
                candidate_actionable_lead_time_days: None,
                baseline_false_positive_count: 0,
                candidate_false_positive_count: 1,
                actionable_delta_days: Some(-18),
                outcome: "timely_to_missed".to_string(),
            }],
        },
        probability_guard_regressions: vec!["timely_warning_rate regressed".to_string()],
        probability_guard_passed: false,
        operational_guard_regressions: vec![
            "runtime floor hits fell while strict actionable points also regressed".to_string(),
        ],
        operational_guard_passed: false,
        actionability_guard_regressions: Vec::new(),
        actionability_guard_passed: true,
        runtime_sanity_regressions: Vec::new(),
        runtime_sanity_passed: true,
        overall_guard_regressions: vec![
            "candidate failed probability and runtime release review guardrails".to_string(),
        ],
        overall_guard_passed: false,
        recommendation: "候选版未通过当前 review，不应替代默认线上版本。".to_string(),
    }
}

#[test]
fn render_release_review_markdown_keeps_runtime_separation_and_dual_counts() {
    let markdown = crate::render_release_review_markdown(&sample_release_review_report());

    assert!(markdown.contains("- History mode: strict_rebuild (limit 260)"));
    assert!(markdown.contains("## Runtime Separation Comparison"));
    assert!(markdown.contains("### Runtime Interpretation"));
    assert!(markdown.contains("| strict_actionable_point_count | 6 | 2 | -4 |"));
    assert!(markdown.contains("| runtime_floor_hit_count | 9 | 7 | -2 |"));
    assert!(markdown
        .contains("| 60d | usable_early_warning_separation | separated_but_below_runtime_floor |"));
    assert!(markdown.contains("candidate 的 pre_warning_buffer 已经和 normal 拉开"));
}

#[test]
fn write_release_review_report_keeps_history_mode_in_exported_artifact_name() {
    let report = sample_release_review_report();
    let output_dir = std::env::temp_dir().join(format!(
        "fc-release-review-reporting-{}",
        Utc::now()
            .timestamp_nanos_opt()
            .unwrap_or_default()
            .unsigned_abs()
    ));

    crate::reporting::write_release_review_report(&output_dir, &report).unwrap();

    let stem = format!(
        "{}-{}-vs-{}-{}-release-review",
        report.candidate_assessment.as_of_date,
        report.baseline_release.manifest.release_id,
        report.candidate_release.manifest.release_id,
        report.history_mode
    );
    let markdown_path: PathBuf = output_dir.join(format!("{stem}.md"));
    let json_path: PathBuf = output_dir.join(format!("{stem}.json"));

    assert!(markdown_path.exists());
    assert!(json_path.exists());

    let markdown = std::fs::read_to_string(&markdown_path).unwrap();
    assert!(markdown.contains("- History mode: strict_rebuild (limit 260)"));

    let _ = std::fs::remove_dir_all(&output_dir);
}

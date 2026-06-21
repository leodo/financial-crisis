use super::*;

fn summary_test_snapshot() -> RiskSnapshot {
    RiskSnapshot {
        as_of_date: NaiveDate::from_ymd_opt(2026, 6, 10).unwrap(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        overall_score: 36.8,
        overall_level: RiskLevel::Watch,
        structural_score: 38.4,
        trigger_score: 35.0,
        level_reason: "test".to_string(),
        dimensions: Vec::new(),
        top_contributors: Vec::new(),
        data_quality_summary: DataQualitySummary {
            overall_score: 91.0,
            grade: QualityGrade::A,
            stale_indicator_count: 0,
            low_quality_indicator_count: 0,
            prototype_source_count: 0,
            blocked_indicator_count: 0,
        },
        generated_at: Utc::now(),
        method_version: "test".to_string(),
    }
}

fn degraded_heuristic_release() -> ModelReleaseRecord {
    ModelReleaseRecord {
        manifest: ModelReleaseManifest {
            release_id: "us_heuristic_bootstrap_20260531".to_string(),
            market_scope: "financial_system".to_string(),
            status: "active".to_string(),
            probability_mode: "heuristic_mvp".to_string(),
            serving_status: "degraded".to_string(),
            bundle_uri: "config/model-releases/us-heuristic-bootstrap.json".to_string(),
            feature_set_version: "feature_v2_20260531".to_string(),
            label_version: "label_v1_20260530".to_string(),
            prob_model_version: "prob_v1_20260531".to_string(),
            calibration_version: "calib_v1_20260531".to_string(),
            posture_policy_version: "posture_v1_20260530".to_string(),
            action_playbook_version: "action_playbook_v1_20260531".to_string(),
            point_in_time_mode: "best_effort".to_string(),
            training_range_start: None,
            training_range_end: None,
            calibration_range_start: None,
            calibration_range_end: None,
            evaluation_range_start: None,
            evaluation_range_end: None,
            brier_score: None,
            log_loss: None,
            ece: None,
            note: "test degraded heuristic release".to_string(),
        },
        created_at: Utc::now(),
        activated_at: Some(Utc::now()),
        retired_at: None,
    }
}

#[test]
fn position_guidance_governance_enforces_manual_review_and_release_boundaries() {
    let guidance = build_position_guidance(
        &posture_guidance_for(DecisionPosture::Defend),
        &ProbabilityBlock {
            p_5d: 0.08,
            p_20d: 0.14,
            p_60d: 0.22,
        },
        TimeToRiskBucket::Now,
        &test_data_trust(QualityGrade::B),
        &stressed_jpy_carry(72.0, 58.0),
        &EventAssessment {
            state: EventConfirmationState::Confirmed,
            confirmation_score: 72.0,
            recent_event_count: 2,
            summary: "test".to_string(),
            confirmed_signals: Vec::new(),
            pending_gaps: Vec::new(),
            recent_events: Vec::new(),
        },
        &MvpRiskState {
            code: MvpRiskStateCode::Hedge,
            label: "对冲准备".to_string(),
            probability_input_status: MvpProbabilityInputStatus::ReferenceOnly,
            summary: "test".to_string(),
            primary_evidence: Vec::new(),
            blockers: Vec::new(),
            next_actions: Vec::new(),
        },
        None,
        &neutral_preferences(),
        ProbabilityActionThresholds {
            prepare_p60d: 0.12,
            hedge_p20d: 0.06,
            defend_p5d: 0.05,
        },
    );

    assert!(guidance.governance.system_budget_only);
    assert!(!guidance.governance.auto_execution_allowed);
    assert!(guidance.governance.manual_confirmation_required);
    assert!(guidance.governance.policy_change_requires_release_review);
    assert!(guidance.governance.policy_change_requires_go_no_go);
    assert!(guidance
        .governance
        .required_operator_checks
        .iter()
        .any(|row| row.contains("release review")));
    assert!(guidance
        .governance
        .required_operator_checks
        .iter()
        .any(|row| row.contains("Go/No-Go")));
    assert!(guidance
        .governance
        .required_operator_checks
        .iter()
        .any(|row| row.contains("人工复核")));
    assert!(guidance
        .inapplicable_scenarios
        .iter()
        .any(|row| row.contains("reference_only")));
    assert!(guidance
        .manual_confirmation_items
        .iter()
        .any(|row| row.contains("不是自动交易")));
    assert!(guidance
        .manual_confirmation_items
        .iter()
        .any(|row| row.contains("对冲") || row.contains("期权")));
}

#[test]
fn position_guidance_governance_keeps_degraded_active_release_warning() {
    let active_release = degraded_heuristic_release();
    let guidance = build_position_guidance(
        &posture_guidance_for(DecisionPosture::Normal),
        &ProbabilityBlock {
            p_5d: 0.0,
            p_20d: 0.03,
            p_60d: 0.08,
        },
        TimeToRiskBucket::Normal,
        &test_data_trust(QualityGrade::A),
        &quiet_jpy_carry(12.0),
        &quiet_event_assessment(0.0),
        &MvpRiskState {
            code: MvpRiskStateCode::Observe,
            label: "观察为主".to_string(),
            probability_input_status: MvpProbabilityInputStatus::Usable,
            summary: "test".to_string(),
            primary_evidence: Vec::new(),
            blockers: Vec::new(),
            next_actions: Vec::new(),
        },
        Some(&active_release),
        &neutral_preferences(),
        ProbabilityActionThresholds {
            prepare_p60d: 0.12,
            hedge_p20d: 0.06,
            defend_p5d: 0.05,
        },
    );

    assert!(guidance
        .governance
        .required_operator_checks
        .iter()
        .any(|row| row.contains("heuristic/degraded")));
    assert!(guidance
        .governance
        .required_operator_checks
        .iter()
        .any(|row| row.contains("不代表正式概率 Go/No-Go 已放行")));
}

#[test]
fn summary_uses_mvp_state_when_probability_is_reference_only() {
    let summary = build_summary(
        &summary_test_snapshot(),
        &ProbabilityBlock {
            p_5d: 0.001421,
            p_20d: 0.000067,
            p_60d: 0.000757,
        },
        TimeToRiskBucket::Normal,
        &posture_guidance_for(DecisionPosture::Normal),
        &MvpRiskState {
            code: MvpRiskStateCode::Observe,
            label: "观察为主（概率参考）".to_string(),
            probability_input_status: MvpProbabilityInputStatus::ReferenceOnly,
            summary: "MVP 规则层未看到足够证据支持主动减仓或对冲，正式概率当前只作为参考输入。"
                .to_string(),
            primary_evidence: Vec::new(),
            blockers: Vec::new(),
            next_actions: Vec::new(),
        },
    );

    assert!(summary.contains("MVP 风险状态：观察为主（概率参考）"));
    assert!(summary.contains("不参与主结论"));
    assert!(!summary.contains("当前仍偏常态区间"));
}

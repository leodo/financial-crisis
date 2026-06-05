use super::probability::{
    actionability_confidence_from_probability, fuse_actionability_confidence,
};
use super::{
    build_position_guidance, build_posture_guidance, build_time_to_risk_bucket,
    ProbabilityActionThresholds,
};
use chrono::{NaiveDate, Utc};
use fc_domain::{
    ActionabilityLevel, DataQualitySummary, DataTrust, DecisionPosture, EventAssessment,
    EventConfirmationState, JpyCarrySnapshot, JpyCarryState, PostureGuidance, ProbabilityBlock,
    QualityGrade, RiskLevel, RiskSnapshot, TimeToRiskBucket, UserRiskPreferences, UserRiskProfile,
};

fn neutral_preferences() -> UserRiskPreferences {
    UserRiskPreferences {
        profile: UserRiskProfile::Neutral,
        cash_floor_pct: 15.0,
        max_equity_cap_pct: 70.0,
        max_leverage_pct: 100.0,
        option_overlay_preference_pct: 5.0,
        allow_aggressive_reentry: false,
        note: "test".to_string(),
    }
}

fn test_data_trust(quality_grade: QualityGrade) -> DataTrust {
    DataTrust {
        coverage_score: 0.98,
        core_feature_coverage: 1.0,
        trigger_feature_coverage: 0.95,
        external_feature_coverage: 0.95,
        quality_grade,
        data_quality_summary: DataQualitySummary {
            overall_score: 91.0,
            grade: quality_grade,
            stale_indicator_count: 0,
            low_quality_indicator_count: 0,
            prototype_source_count: 0,
            blocked_indicator_count: 0,
        },
        warnings: Vec::new(),
    }
}

fn quiet_event_assessment(confirmation_score: f64) -> EventAssessment {
    EventAssessment {
        state: EventConfirmationState::Quiet,
        confirmation_score,
        recent_event_count: 0,
        summary: "test".to_string(),
        confirmed_signals: Vec::new(),
        pending_gaps: Vec::new(),
        recent_events: Vec::new(),
    }
}

fn quiet_jpy_carry(funding_pressure_score: f64) -> JpyCarrySnapshot {
    JpyCarrySnapshot {
        state: JpyCarryState::Quiet,
        score: 10.0,
        usdjpy_level: Some(150.0),
        jp_call_rate: Some(0.25),
        us_short_rate: Some(4.0),
        us_jp_short_rate_diff: Some(3.75),
        change_5d: Some(0.2),
        change_20d: Some(1.0),
        realized_vol_20d: Some(0.01),
        funding_pressure_score,
        vix_coupling_score: 15.0,
        credit_coupling_score: 15.0,
        reason: "test".to_string(),
    }
}

fn stressed_jpy_carry(score: f64, funding_pressure_score: f64) -> JpyCarrySnapshot {
    JpyCarrySnapshot {
        state: JpyCarryState::Stress,
        score,
        usdjpy_level: Some(159.0),
        jp_call_rate: Some(0.10),
        us_short_rate: Some(5.25),
        us_jp_short_rate_diff: Some(5.15),
        change_5d: Some(2.5),
        change_20d: Some(4.2),
        realized_vol_20d: Some(0.11),
        funding_pressure_score,
        vix_coupling_score: 52.0,
        credit_coupling_score: 48.0,
        reason: "test".to_string(),
    }
}

fn posture_guidance_for(posture: DecisionPosture) -> PostureGuidance {
    PostureGuidance {
        posture,
        summary: "test".to_string(),
        reasons: Vec::new(),
        upgrade_condition: "test".to_string(),
        downgrade_condition: "test".to_string(),
        trigger_codes: Vec::new(),
        blocker_codes: Vec::new(),
    }
}

#[test]
fn actionability_confidence_requires_margin_above_decision_threshold() {
    assert_eq!(actionability_confidence_from_probability(0.05, 0.05), 0.0);
    assert!(actionability_confidence_from_probability(0.20, 0.05) < 0.05);
    assert!(actionability_confidence_from_probability(0.55, 0.05) > 0.25);
}

#[test]
fn fused_actionability_suppresses_high_confidence_without_context() {
    let snapshot = RiskSnapshot {
        as_of_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        overall_score: 33.3,
        overall_level: RiskLevel::Watch,
        structural_score: 39.7,
        trigger_score: 25.4,
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
    };
    let probabilities = ProbabilityBlock {
        p_5d: 0.005,
        p_20d: 0.025,
        p_60d: 0.055,
    };
    let thresholds = ProbabilityActionThresholds {
        prepare_p60d: 0.023,
        hedge_p20d: 0.008,
        defend_p5d: 0.005,
    };

    let prepare = fuse_actionability_confidence(
        ActionabilityLevel::Prepare,
        0.954,
        &probabilities,
        &snapshot,
        29.8,
        thresholds,
    );
    let hedge = fuse_actionability_confidence(
        ActionabilityLevel::Hedge,
        0.812,
        &probabilities,
        &snapshot,
        29.8,
        thresholds,
    );

    assert!(prepare < 0.10);
    assert!(hedge < 0.10);
}

#[test]
fn fused_actionability_preserves_supported_prepare_context() {
    let snapshot = RiskSnapshot {
        as_of_date: NaiveDate::from_ymd_opt(2020, 2, 20).unwrap(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        overall_score: 61.0,
        overall_level: RiskLevel::Stress,
        structural_score: 58.0,
        trigger_score: 54.0,
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
    };
    let probabilities = ProbabilityBlock {
        p_5d: 0.018,
        p_20d: 0.052,
        p_60d: 0.118,
    };
    let thresholds = ProbabilityActionThresholds {
        prepare_p60d: 0.023,
        hedge_p20d: 0.008,
        defend_p5d: 0.005,
    };

    let prepare = fuse_actionability_confidence(
        ActionabilityLevel::Prepare,
        0.82,
        &probabilities,
        &snapshot,
        52.0,
        thresholds,
    );

    assert!(prepare > 0.35);
}

#[test]
fn time_to_risk_bucket_requires_confirmation_for_months_bucket() {
    let bucket = build_time_to_risk_bucket(
        &ProbabilityBlock {
            p_5d: 0.004,
            p_20d: 0.018,
            p_60d: 0.14,
        },
        None,
        None,
        59.0,
        40.0,
        44.0,
        32.0,
        &quiet_jpy_carry(20.0),
        ProbabilityActionThresholds {
            prepare_p60d: 0.12,
            hedge_p20d: 0.06,
            defend_p5d: 0.05,
        },
    );

    assert_eq!(bucket, TimeToRiskBucket::Normal);
}

#[test]
fn time_to_risk_bucket_allows_months_when_probability_and_context_align() {
    let bucket = build_time_to_risk_bucket(
        &ProbabilityBlock {
            p_5d: 0.004,
            p_20d: 0.05,
            p_60d: 0.14,
        },
        None,
        None,
        59.0,
        47.0,
        52.0,
        38.0,
        &quiet_jpy_carry(20.0),
        ProbabilityActionThresholds {
            prepare_p60d: 0.12,
            hedge_p20d: 0.06,
            defend_p5d: 0.05,
        },
    );

    assert_eq!(bucket, TimeToRiskBucket::Months);
}

#[test]
fn time_to_risk_bucket_ignores_monotonic_only_prepare_crossing() {
    let bucket = build_time_to_risk_bucket(
        &ProbabilityBlock {
            p_5d: 0.004,
            p_20d: 0.09,
            p_60d: 0.14,
        },
        Some(0.09),
        None,
        60.0,
        46.0,
        44.0,
        36.0,
        &quiet_jpy_carry(20.0),
        ProbabilityActionThresholds {
            prepare_p60d: 0.12,
            hedge_p20d: 0.06,
            defend_p5d: 0.05,
        },
    );

    assert_eq!(bucket, TimeToRiskBucket::Normal);
}

#[test]
fn posture_guidance_blocks_prepare_external_without_probability_companion() {
    let snapshot = RiskSnapshot {
        as_of_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        overall_score: 57.0,
        overall_level: RiskLevel::Stress,
        structural_score: 54.0,
        trigger_score: 32.0,
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
    };
    let probabilities = ProbabilityBlock {
        p_5d: 0.004,
        p_20d: 0.018,
        p_60d: 0.010,
    };
    let posture = build_posture_guidance(
        &snapshot,
        &probabilities,
        None,
        None,
        0.60,
        &test_data_trust(QualityGrade::A),
        56.0,
        30.0,
        &[],
        &quiet_jpy_carry(20.0),
        &quiet_event_assessment(20.0),
        &neutral_preferences(),
        ProbabilityActionThresholds {
            prepare_p60d: 0.12,
            hedge_p20d: 0.06,
            defend_p5d: 0.05,
        },
    );

    assert_eq!(posture.posture, fc_domain::DecisionPosture::Normal);
    assert!(posture.trigger_codes.is_empty());
    assert!(posture.blocker_codes.is_empty());
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
}

#[test]
fn posture_guidance_ignores_monotonic_only_prepare_crossing() {
    let snapshot = RiskSnapshot {
        as_of_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        overall_score: 50.0,
        overall_level: RiskLevel::Watch,
        structural_score: 60.0,
        trigger_score: 46.0,
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
    };
    let probabilities = ProbabilityBlock {
        p_5d: 0.004,
        p_20d: 0.09,
        p_60d: 0.14,
    };
    let posture = build_posture_guidance(
        &snapshot,
        &probabilities,
        Some(0.09),
        None,
        0.60,
        &test_data_trust(QualityGrade::A),
        44.0,
        36.0,
        &[],
        &quiet_jpy_carry(20.0),
        &quiet_event_assessment(20.0),
        &neutral_preferences(),
        ProbabilityActionThresholds {
            prepare_p60d: 0.12,
            hedge_p20d: 0.06,
            defend_p5d: 0.05,
        },
    );

    assert_eq!(posture.posture, fc_domain::DecisionPosture::Normal);
    assert!(posture.trigger_codes.is_empty());
}

#[test]
fn posture_guidance_emits_prepare_external_structural_clause_when_probability_confirms() {
    let snapshot = RiskSnapshot {
        as_of_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        overall_score: 57.0,
        overall_level: RiskLevel::Stress,
        structural_score: 55.0,
        trigger_score: 46.0,
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
    };
    let probabilities = ProbabilityBlock {
        p_5d: 0.004,
        p_20d: 0.05,
        p_60d: 0.010,
    };
    let posture = build_posture_guidance(
        &snapshot,
        &probabilities,
        None,
        None,
        0.60,
        &test_data_trust(QualityGrade::A),
        59.0,
        38.0,
        &[],
        &quiet_jpy_carry(20.0),
        &quiet_event_assessment(42.0),
        &neutral_preferences(),
        ProbabilityActionThresholds {
            prepare_p60d: 0.12,
            hedge_p20d: 0.06,
            defend_p5d: 0.05,
        },
    );

    assert_eq!(posture.posture, fc_domain::DecisionPosture::Prepare);
    assert_eq!(
        posture.trigger_codes,
        vec!["prepare_external_structural".to_string()]
    );
    assert!(posture.blocker_codes.is_empty());
}

#[test]
fn posture_guidance_marks_quality_blocked_hedge_clause() {
    let snapshot = RiskSnapshot {
        as_of_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        overall_score: 49.0,
        overall_level: RiskLevel::Watch,
        structural_score: 44.0,
        trigger_score: 54.0,
        level_reason: "test".to_string(),
        dimensions: Vec::new(),
        top_contributors: Vec::new(),
        data_quality_summary: DataQualitySummary {
            overall_score: 62.0,
            grade: QualityGrade::F,
            stale_indicator_count: 2,
            low_quality_indicator_count: 0,
            prototype_source_count: 0,
            blocked_indicator_count: 1,
        },
        generated_at: Utc::now(),
        method_version: "test".to_string(),
    };
    let probabilities = ProbabilityBlock {
        p_5d: 0.01,
        p_20d: 0.07,
        p_60d: 0.10,
    };
    let posture = build_posture_guidance(
        &snapshot,
        &probabilities,
        None,
        None,
        0.58,
        &test_data_trust(QualityGrade::F),
        52.0,
        41.0,
        &[],
        &quiet_jpy_carry(20.0),
        &quiet_event_assessment(42.0),
        &neutral_preferences(),
        ProbabilityActionThresholds {
            prepare_p60d: 0.12,
            hedge_p20d: 0.06,
            defend_p5d: 0.05,
        },
    );

    assert_eq!(posture.posture, fc_domain::DecisionPosture::Normal);
    assert!(posture.trigger_codes.is_empty());
    assert_eq!(
        posture.blocker_codes,
        vec!["quality_blocked_hedge".to_string()]
    );
}

#[test]
fn posture_guidance_requires_multi_signal_context_for_hedge_clause() {
    let snapshot = RiskSnapshot {
        as_of_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        overall_score: 48.0,
        overall_level: RiskLevel::Watch,
        structural_score: 46.0,
        trigger_score: 53.0,
        level_reason: "test".to_string(),
        dimensions: Vec::new(),
        top_contributors: Vec::new(),
        data_quality_summary: DataQualitySummary {
            overall_score: 88.0,
            grade: QualityGrade::A,
            stale_indicator_count: 0,
            low_quality_indicator_count: 0,
            prototype_source_count: 0,
            blocked_indicator_count: 0,
        },
        generated_at: Utc::now(),
        method_version: "test".to_string(),
    };
    let probabilities = ProbabilityBlock {
        p_5d: 0.01,
        p_20d: 0.18,
        p_60d: 0.03,
    };
    let posture = build_posture_guidance(
        &snapshot,
        &probabilities,
        None,
        None,
        0.58,
        &test_data_trust(QualityGrade::A),
        42.0,
        34.0,
        &[],
        &quiet_jpy_carry(18.0),
        &quiet_event_assessment(25.0),
        &neutral_preferences(),
        ProbabilityActionThresholds {
            prepare_p60d: 0.12,
            hedge_p20d: 0.06,
            defend_p5d: 0.05,
        },
    );

    assert_eq!(posture.posture, fc_domain::DecisionPosture::Normal);
    assert!(posture.trigger_codes.is_empty());
}

#[test]
fn posture_guidance_allows_hedge_when_short_and_medium_horizon_context_align() {
    let snapshot = RiskSnapshot {
        as_of_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        overall_score: 56.0,
        overall_level: RiskLevel::Stress,
        structural_score: 52.0,
        trigger_score: 54.0,
        level_reason: "test".to_string(),
        dimensions: Vec::new(),
        top_contributors: Vec::new(),
        data_quality_summary: DataQualitySummary {
            overall_score: 90.0,
            grade: QualityGrade::A,
            stale_indicator_count: 0,
            low_quality_indicator_count: 0,
            prototype_source_count: 0,
            blocked_indicator_count: 0,
        },
        generated_at: Utc::now(),
        method_version: "test".to_string(),
    };
    let probabilities = ProbabilityBlock {
        p_5d: 0.01,
        p_20d: 0.18,
        p_60d: 0.08,
    };
    let posture = build_posture_guidance(
        &snapshot,
        &probabilities,
        None,
        None,
        0.60,
        &test_data_trust(QualityGrade::A),
        52.0,
        42.0,
        &[],
        &quiet_jpy_carry(20.0),
        &quiet_event_assessment(45.0),
        &neutral_preferences(),
        ProbabilityActionThresholds {
            prepare_p60d: 0.12,
            hedge_p20d: 0.06,
            defend_p5d: 0.05,
        },
    );

    assert_eq!(posture.posture, fc_domain::DecisionPosture::Hedge);
    assert_eq!(
        posture.trigger_codes,
        vec!["hedge_p20d_context".to_string()]
    );
}

#[test]
fn posture_guidance_blocks_hedge_when_short_horizon_lacks_overall_confirmation() {
    let snapshot = RiskSnapshot {
        as_of_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        overall_score: 56.5,
        overall_level: RiskLevel::Stress,
        structural_score: 52.0,
        trigger_score: 54.0,
        level_reason: "test".to_string(),
        dimensions: Vec::new(),
        top_contributors: Vec::new(),
        data_quality_summary: DataQualitySummary {
            overall_score: 90.0,
            grade: QualityGrade::A,
            stale_indicator_count: 0,
            low_quality_indicator_count: 0,
            prototype_source_count: 0,
            blocked_indicator_count: 0,
        },
        generated_at: Utc::now(),
        method_version: "test".to_string(),
    };
    let probabilities = ProbabilityBlock {
        p_5d: 0.01,
        p_20d: 0.18,
        p_60d: 0.08,
    };
    let posture = build_posture_guidance(
        &snapshot,
        &probabilities,
        None,
        None,
        0.60,
        &test_data_trust(QualityGrade::A),
        37.0,
        42.0,
        &[],
        &quiet_jpy_carry(20.0),
        &quiet_event_assessment(25.0),
        &neutral_preferences(),
        ProbabilityActionThresholds {
            prepare_p60d: 0.12,
            hedge_p20d: 0.06,
            defend_p5d: 0.05,
        },
    );

    assert_eq!(posture.posture, fc_domain::DecisionPosture::Normal);
    assert!(posture.trigger_codes.is_empty());
}

#[test]
fn posture_guidance_blocks_prepare_carry_without_noncarry_confirmation() {
    let snapshot = RiskSnapshot {
        as_of_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        overall_score: 56.5,
        overall_level: RiskLevel::Stress,
        structural_score: 57.0,
        trigger_score: 34.0,
        level_reason: "test".to_string(),
        dimensions: Vec::new(),
        top_contributors: Vec::new(),
        data_quality_summary: DataQualitySummary {
            overall_score: 90.0,
            grade: QualityGrade::A,
            stale_indicator_count: 0,
            low_quality_indicator_count: 0,
            prototype_source_count: 0,
            blocked_indicator_count: 0,
        },
        generated_at: Utc::now(),
        method_version: "test".to_string(),
    };
    let probabilities = ProbabilityBlock {
        p_5d: 0.01,
        p_20d: 0.03,
        p_60d: 0.10,
    };
    let posture = build_posture_guidance(
        &snapshot,
        &probabilities,
        None,
        None,
        0.60,
        &test_data_trust(QualityGrade::A),
        41.0,
        32.0,
        &[],
        &stressed_jpy_carry(60.0, 52.0),
        &quiet_event_assessment(30.0),
        &neutral_preferences(),
        ProbabilityActionThresholds {
            prepare_p60d: 0.12,
            hedge_p20d: 0.06,
            defend_p5d: 0.05,
        },
    );

    assert_eq!(posture.posture, fc_domain::DecisionPosture::Normal);
    assert!(posture.trigger_codes.is_empty());
}

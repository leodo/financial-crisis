use super::super::*;

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

    assert_eq!(posture.posture, DecisionPosture::Normal);
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

    assert_eq!(posture.posture, DecisionPosture::Normal);
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

    assert_eq!(posture.posture, DecisionPosture::Hedge);
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

    assert_eq!(posture.posture, DecisionPosture::Normal);
    assert!(posture.trigger_codes.is_empty());
}

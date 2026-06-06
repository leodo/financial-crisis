use super::super::*;

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

    assert_eq!(posture.posture, DecisionPosture::Normal);
    assert!(posture.trigger_codes.is_empty());
    assert!(posture.blocker_codes.is_empty());
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

    assert_eq!(posture.posture, DecisionPosture::Normal);
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

    assert_eq!(posture.posture, DecisionPosture::Prepare);
    assert_eq!(
        posture.trigger_codes,
        vec!["prepare_external_structural".to_string()]
    );
    assert!(posture.blocker_codes.is_empty());
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

    assert_eq!(posture.posture, DecisionPosture::Normal);
    assert!(posture.trigger_codes.is_empty());
}

#[test]
fn posture_guidance_emits_prepare_continuity_bridge_for_long_window_pressure() {
    let snapshot = RiskSnapshot {
        as_of_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        overall_score: 53.5,
        overall_level: RiskLevel::Stress,
        structural_score: 62.6,
        trigger_score: 42.3,
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
        p_20d: 0.93,
        p_60d: 0.99,
    };
    let posture = build_posture_guidance(
        &snapshot,
        &probabilities,
        Some(0.93),
        Some(&ActionabilityBlock {
            prepare: 0.245,
            hedge: 0.02,
            defend: 0.0,
        }),
        Some(&ActionabilityBlock {
            prepare: 0.245,
            hedge: 0.02,
            defend: 0.0,
        }),
        0.56,
        &test_data_trust(QualityGrade::A),
        43.8,
        40.0,
        &[],
        &quiet_jpy_carry(20.0),
        &quiet_event_assessment(30.0),
        &neutral_preferences(),
        ProbabilityActionThresholds {
            prepare_p60d: 0.12,
            hedge_p20d: 0.06,
            defend_p5d: 0.05,
        },
    );

    assert_eq!(posture.posture, DecisionPosture::Prepare);
    assert_eq!(
        posture.trigger_codes,
        vec!["prepare_continuity_bridge".to_string()]
    );
    assert!(posture.blocker_codes.is_empty());
}

#[test]
fn posture_guidance_does_not_emit_prepare_continuity_bridge_without_support() {
    let snapshot = RiskSnapshot {
        as_of_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        overall_score: 52.0,
        overall_level: RiskLevel::Watch,
        structural_score: 62.6,
        trigger_score: 38.0,
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
        p_20d: 0.93,
        p_60d: 0.99,
    };
    let posture = build_posture_guidance(
        &snapshot,
        &probabilities,
        Some(0.93),
        Some(&ActionabilityBlock {
            prepare: 0.16,
            hedge: 0.02,
            defend: 0.0,
        }),
        Some(&ActionabilityBlock {
            prepare: 0.16,
            hedge: 0.02,
            defend: 0.0,
        }),
        0.56,
        &test_data_trust(QualityGrade::A),
        40.0,
        34.0,
        &[],
        &quiet_jpy_carry(20.0),
        &quiet_event_assessment(30.0),
        &neutral_preferences(),
        ProbabilityActionThresholds {
            prepare_p60d: 0.12,
            hedge_p20d: 0.06,
            defend_p5d: 0.05,
        },
    );

    assert_eq!(posture.posture, DecisionPosture::Normal);
    assert!(posture.trigger_codes.is_empty());
    assert!(posture.blocker_codes.is_empty());
}

#[test]
fn posture_guidance_uses_support_actionability_for_continuity_without_trigger_head() {
    let snapshot = RiskSnapshot {
        as_of_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        overall_score: 53.5,
        overall_level: RiskLevel::Stress,
        structural_score: 62.6,
        trigger_score: 42.3,
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
        p_20d: 0.93,
        p_60d: 0.99,
    };
    let posture = build_posture_guidance(
        &snapshot,
        &probabilities,
        Some(0.93),
        None,
        Some(&ActionabilityBlock {
            prepare: 0.245,
            hedge: 0.02,
            defend: 0.0,
        }),
        0.56,
        &test_data_trust(QualityGrade::A),
        43.8,
        40.0,
        &[],
        &quiet_jpy_carry(20.0),
        &quiet_event_assessment(30.0),
        &neutral_preferences(),
        ProbabilityActionThresholds {
            prepare_p60d: 0.12,
            hedge_p20d: 0.06,
            defend_p5d: 0.05,
        },
    );

    assert_eq!(posture.posture, DecisionPosture::Prepare);
    assert_eq!(
        posture.trigger_codes,
        vec!["prepare_continuity_bridge".to_string()]
    );
    assert!(posture.blocker_codes.is_empty());
}

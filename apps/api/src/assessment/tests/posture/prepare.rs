use super::super::*;

#[test]
fn posture_guidance_describes_low_conviction_as_action_evidence_not_reliability() {
    let snapshot = RiskSnapshot {
        as_of_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        overall_score: 36.8,
        overall_level: RiskLevel::Normal,
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
    };
    let probabilities = ProbabilityBlock {
        p_5d: 0.001,
        p_20d: 0.001,
        p_60d: 0.001,
    };
    let posture = build_posture_guidance(
        &snapshot,
        &probabilities,
        None,
        None,
        None,
        0.10,
        &test_data_trust(QualityGrade::A),
        29.0,
        14.0,
        &[],
        &quiet_jpy_carry(20.0),
        &quiet_event_assessment(18.0),
        &neutral_preferences(),
        ProbabilityActionThresholds {
            prepare_p60d: 0.57,
            hedge_p20d: 0.28,
            defend_p5d: 0.05,
        },
    );

    assert_eq!(posture.posture, DecisionPosture::Normal);
    let reason_text = posture.reasons.join(" ");
    assert!(reason_text.contains("动作升级证据不足"));
    assert!(reason_text.contains("风险广度、压力或共振尚未打开"));
    assert!(!reason_text.contains("可信度一般"));
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
        p_20d: 0.44,
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
    assert!(posture
        .trigger_codes
        .iter()
        .any(|code| code == "prepare_continuity_bridge"));
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
        p_20d: 0.44,
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
        p_20d: 0.44,
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
    assert!(posture
        .trigger_codes
        .iter()
        .any(|code| code == "prepare_continuity_bridge"));
    assert!(posture.blocker_codes.is_empty());
}

#[test]
fn posture_guidance_keeps_prepare_continuity_bridge_under_low_conviction() {
    let snapshot = RiskSnapshot {
        as_of_date: NaiveDate::from_ymd_opt(2007, 8, 1).unwrap(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        overall_score: 56.0,
        overall_level: RiskLevel::Stress,
        structural_score: 70.5,
        trigger_score: 38.4,
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
        p_5d: 0.079,
        p_20d: 0.609,
        p_60d: 0.252,
    };
    let posture = build_posture_guidance(
        &snapshot,
        &probabilities,
        Some(0.639),
        Some(&ActionabilityBlock {
            prepare: 0.548,
            hedge: 0.15,
            defend: 0.0,
        }),
        Some(&ActionabilityBlock {
            prepare: 0.548,
            hedge: 0.15,
            defend: 0.0,
        }),
        0.51,
        &test_data_trust(QualityGrade::A),
        49.1,
        20.0,
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

    assert_eq!(posture.posture, DecisionPosture::Prepare);
    assert_eq!(
        posture.trigger_codes,
        vec!["prepare_continuity_bridge".to_string()]
    );
    assert!(posture.blocker_codes.is_empty());
}

#[test]
fn posture_guidance_uses_runtime_derived_prepare_continuity_floors() {
    let snapshot = RiskSnapshot {
        as_of_date: NaiveDate::from_ymd_opt(2007, 8, 1).unwrap(),
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
        p_20d: 0.13,
        p_60d: 0.22,
    };
    let posture = build_posture_guidance(
        &snapshot,
        &probabilities,
        Some(0.23),
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
        39.0,
        30.0,
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
fn posture_guidance_emits_prepare_probability_plateau_for_long_window_high_probability_regime() {
    let snapshot = RiskSnapshot {
        as_of_date: NaiveDate::from_ymd_opt(1987, 8, 18).unwrap(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        overall_score: 42.7,
        overall_level: RiskLevel::Stress,
        structural_score: 47.5,
        trigger_score: 36.8,
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
        p_20d: 0.749,
        p_60d: 0.84,
    };
    let posture = build_posture_guidance(
        &snapshot,
        &probabilities,
        Some(0.84),
        None,
        None,
        0.53,
        &test_data_trust(QualityGrade::A),
        42.7,
        38.0,
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

    assert_eq!(posture.posture, DecisionPosture::Prepare);
    assert_eq!(
        posture.trigger_codes,
        vec!["prepare_probability_plateau".to_string()]
    );
    assert!(posture.blocker_codes.is_empty());
}

#[test]
fn posture_guidance_emits_prepare_probability_plateau_for_relaxed_extreme_context() {
    let snapshot = RiskSnapshot {
        as_of_date: NaiveDate::from_ymd_opt(1987, 10, 8).unwrap(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        overall_score: 42.4,
        overall_level: RiskLevel::Stress,
        structural_score: 44.5,
        trigger_score: 35.8,
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
        p_5d: 0.03,
        p_20d: 0.689,
        p_60d: 0.852,
    };
    let posture = build_posture_guidance(
        &snapshot,
        &probabilities,
        Some(0.852),
        None,
        None,
        0.52,
        &test_data_trust(QualityGrade::A),
        40.7,
        34.0,
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

    assert_eq!(posture.posture, DecisionPosture::Prepare);
    assert_eq!(
        posture.trigger_codes,
        vec!["prepare_probability_plateau".to_string()]
    );
    assert!(posture.blocker_codes.is_empty());
}

#[test]
fn posture_guidance_uses_runtime_derived_plateau_p20d_floor() {
    let snapshot = RiskSnapshot {
        as_of_date: NaiveDate::from_ymd_opt(1998, 9, 3).unwrap(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        overall_score: 55.0,
        overall_level: RiskLevel::Stress,
        structural_score: 59.6,
        trigger_score: 49.3,
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
        p_5d: 0.138,
        p_20d: 0.393,
        p_60d: 0.718,
    };
    let posture = build_posture_guidance(
        &snapshot,
        &probabilities,
        Some(0.718),
        Some(&ActionabilityBlock {
            prepare: 0.225,
            hedge: 0.056,
            defend: 0.005,
        }),
        Some(&ActionabilityBlock {
            prepare: 0.225,
            hedge: 0.056,
            defend: 0.005,
        }),
        0.55,
        &test_data_trust(QualityGrade::A),
        42.8,
        20.0,
        &[],
        &quiet_jpy_carry(20.0),
        &quiet_event_assessment(20.0),
        &neutral_preferences(),
        ProbabilityActionThresholds {
            prepare_p60d: 0.568,
            hedge_p20d: 0.282,
            defend_p5d: 0.05,
        },
    );

    assert_eq!(posture.posture, DecisionPosture::Prepare);
    assert_eq!(
        posture.trigger_codes,
        vec!["prepare_probability_plateau".to_string()]
    );
    assert!(posture.blocker_codes.is_empty());
}

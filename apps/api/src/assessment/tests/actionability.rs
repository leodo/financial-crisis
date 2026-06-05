use super::*;
use fc_domain::ActionabilityLevel;

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

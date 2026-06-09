use super::*;

fn snapshot_with_scores(structural_score: f64, trigger_score: f64) -> RiskSnapshot {
    RiskSnapshot {
        as_of_date: NaiveDate::from_ymd_opt(2026, 6, 9).unwrap(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        overall_score: 35.0,
        overall_level: RiskLevel::Watch,
        structural_score,
        trigger_score,
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

#[test]
fn action_evidence_breakdown_does_not_mistake_data_quality_for_risk_evidence() {
    let snapshot = snapshot_with_scores(44.0, 37.0);
    let evidence =
        build_action_evidence_breakdown(&snapshot, &test_data_trust(QualityGrade::A), 32.0);

    assert_eq!(evidence.score, 0.178);
    assert_eq!(evidence.data_quality_component, 0.098);
    assert_eq!(evidence.breadth_component, 0.08);
    assert_eq!(evidence.risk_pressure_component, 0.0);
    assert_eq!(evidence.agreement_component, 0.0);
    assert!(!evidence.structural_trigger_agreement);
}

#[test]
fn action_evidence_rises_when_breadth_and_agreement_confirm() {
    let snapshot = snapshot_with_scores(61.0, 59.0);
    let evidence =
        build_action_evidence_breakdown(&snapshot, &test_data_trust(QualityGrade::A), 67.0);

    assert_eq!(evidence.score, 0.712);
    assert_eq!(evidence.breadth_component, 0.3);
    assert_eq!(evidence.risk_pressure_component, 0.194);
    assert_eq!(evidence.agreement_component, 0.12);
    assert!(evidence.structural_trigger_agreement);
}

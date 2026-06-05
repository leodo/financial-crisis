use super::*;

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

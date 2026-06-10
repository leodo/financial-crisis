pub(super) use super::market_context::build_action_evidence_breakdown;
pub(super) use super::probability::{
    actionability_confidence_from_probability, fuse_actionability_confidence,
};
pub(super) use super::{
    build_position_guidance, build_posture_guidance, build_summary, build_time_to_risk_bucket,
    ProbabilityActionThresholds,
};
pub(super) use chrono::{NaiveDate, Utc};
pub(super) use fc_domain::{
    ActionabilityBlock, DataQualitySummary, DataTrust, DecisionPosture, EventAssessment,
    EventConfirmationState, JpyCarrySnapshot, JpyCarryState, MvpProbabilityInputStatus,
    MvpRiskState, MvpRiskStateCode, PostureGuidance, ProbabilityBlock, QualityGrade, RiskLevel,
    RiskSnapshot, TimeToRiskBucket, UserRiskPreferences, UserRiskProfile,
};

mod actionability;
mod evidence;
mod position;
mod posture;
mod time_bucket;

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

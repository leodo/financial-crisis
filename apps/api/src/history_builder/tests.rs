use std::collections::BTreeMap;

use chrono::{NaiveDate, Utc};
use fc_domain::{
    ActionEvidenceBreakdown, AssessmentMethodVersions, AssessmentScores, AssessmentSnapshot,
    BacktestPerformanceSummary, BacktestRollingAudit, DataMode, DataQualitySummary, DataTrust,
    DecisionPosture, EventAssessment, EventConfirmationState, FeatureSnapshotRecord,
    HistoricalAssessmentPointRecord, HistoricalReplayRunRecord, JpyCarrySnapshot, JpyCarryState,
    ModelReleaseManifest, ModelReleaseRecord, PositionGuidance, PositionGuidanceGovernance,
    PostureGuidance, PredictionSnapshotRecord, ProbabilityBlock, ProbabilityBundle,
    ProbabilityDiagnostics, QualityGrade, RuntimeMetadata, UserRiskPreferences, UserRiskProfile,
};
use fc_storage::SqliteStore;

use super::{
    load_sqlite_assessment_history, AssessmentHistoryBuildMode, HistoricalPrepareHysteresisState,
};
use crate::{
    assessment::ServingModelContext,
    demo::{load_user_preferences, FORMAL_MAIN_FEATURE_SET_VERSION, FORMAL_MAIN_LABEL_VERSION},
    demo_seed::{indicators as demo_indicators, observations as demo_observations},
    history_replay::expected_prediction_snapshot_method_version,
};

async fn in_memory_store() -> SqliteStore {
    let store = SqliteStore::connect_url("sqlite::memory:").await.unwrap();
    store.migrate().await.unwrap();
    store
}

fn history_test_preferences() -> UserRiskPreferences {
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

fn history_test_data_trust() -> DataTrust {
    DataTrust {
        coverage_score: 0.98,
        core_feature_coverage: 1.0,
        trigger_feature_coverage: 0.95,
        external_feature_coverage: 0.95,
        quality_grade: QualityGrade::A,
        data_quality_summary: DataQualitySummary {
            overall_score: 91.0,
            grade: QualityGrade::A,
            stale_indicator_count: 0,
            low_quality_indicator_count: 0,
            prototype_source_count: 0,
            blocked_indicator_count: 0,
        },
        warnings: Vec::new(),
    }
}

fn history_test_jpy_carry(funding_pressure_score: f64) -> JpyCarrySnapshot {
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

fn history_test_event_assessment() -> EventAssessment {
    EventAssessment {
        state: EventConfirmationState::Quiet,
        confirmation_score: 0.0,
        recent_event_count: 0,
        summary: "test".to_string(),
        confirmed_signals: Vec::new(),
        pending_gaps: Vec::new(),
        recent_events: Vec::new(),
    }
}

fn history_test_feature_snapshot(as_of_date: NaiveDate) -> FeatureSnapshotRecord {
    FeatureSnapshotRecord {
        as_of_date,
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        feature_set_version: FORMAL_MAIN_FEATURE_SET_VERSION.to_string(),
        point_in_time_mode: "best_effort".to_string(),
        visibility_status: "ready".to_string(),
        latest_visible_at: Some(Utc::now()),
        coverage_score: 1.0,
        core_feature_coverage: 1.0,
        trigger_feature_coverage: 1.0,
        external_feature_coverage: 1.0,
        feature_count: 1,
        features: BTreeMap::from([("placeholder".to_string(), 1.0)]),
        created_at: Utc::now(),
    }
}

fn history_test_position_guidance() -> PositionGuidance {
    PositionGuidance {
        action_playbook_version: "test".to_string(),
        execution_urgency: "test".to_string(),
        confidence_gate: "test".to_string(),
        target_equity_exposure_pct: 50.0,
        target_cash_pct: 20.0,
        hedge_ratio_pct: 10.0,
        leverage_cap_pct: 50.0,
        option_overlay_pct: 5.0,
        action_summary: "test".to_string(),
        actions: Vec::new(),
        forbidden_actions: Vec::new(),
        reentry_conditions: Vec::new(),
        guardrails: Vec::new(),
        capital_preservation_overlay_enabled: false,
        governance: PositionGuidanceGovernance::default(),
    }
}

fn history_test_backtest_summary() -> BacktestPerformanceSummary {
    BacktestPerformanceSummary {
        scenario_count: 0,
        real_scenario_count: 0,
        fallback_scenario_count: 0,
        coverage_scope_note: "test".to_string(),
        structural_warning_rate: 0.0,
        timely_warning_rate: 0.0,
        missed_rate: 0.0,
        avg_structural_lead_time_days: None,
        avg_lead_time_days: None,
        median_lead_time_days: None,
        total_false_positive_count: 0,
        history_start: None,
        history_end: None,
        rolling_audit: BacktestRollingAudit {
            history_start: None,
            history_end: None,
            history_point_count: 0,
            scope_note: "test".to_string(),
            actionable_signal_count: 0,
            pre_crisis_signal_count: 0,
            in_crisis_signal_count: 0,
            stress_window_signal_count: 0,
            false_positive_signal_count: 0,
            false_positive_episode_count: 0,
            longest_false_positive_episode_days: 0,
            actionable_precision: 0.0,
            classified_episodes: Vec::new(),
            summary: "test".to_string(),
        },
        summary: "test".to_string(),
    }
}

fn history_test_posture_guidance(
    posture: DecisionPosture,
    trigger_codes: &[&str],
) -> PostureGuidance {
    PostureGuidance {
        posture,
        summary: "test".to_string(),
        reasons: Vec::new(),
        upgrade_condition: "test".to_string(),
        downgrade_condition: "test".to_string(),
        trigger_codes: trigger_codes
            .iter()
            .map(|code| (*code).to_string())
            .collect(),
        blocker_codes: Vec::new(),
    }
}

#[allow(clippy::too_many_arguments)]
fn history_test_assessment(
    posture: DecisionPosture,
    time_to_risk_bucket: fc_domain::TimeToRiskBucket,
    p_20d: f64,
    p_60d: f64,
    action_prepare: f64,
    overall_score: f64,
    structural_score: f64,
    trigger_score: f64,
    external_shock_score: f64,
    funding_pressure_score: f64,
) -> AssessmentSnapshot {
    let as_of_date = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
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
            prepare: action_prepare,
            hedge: 0.0,
            defend: 0.0,
        },
        probability_diagnostics: ProbabilityDiagnostics::default(),
        time_to_risk_bucket,
        posture,
        conviction_score: 0.6,
        action_evidence: ActionEvidenceBreakdown {
            score: 0.6,
            ..ActionEvidenceBreakdown::default()
        },
        scores: AssessmentScores {
            overall_score,
            structural_score,
            trigger_score,
            external_shock_score,
        },
        summary: "test".to_string(),
        posture_reason: "test".to_string(),
        top_risk_drivers: Vec::new(),
        top_relief_drivers: Vec::new(),
        historical_analogs: Vec::new(),
        data_trust: history_test_data_trust(),
        jpy_carry: history_test_jpy_carry(funding_pressure_score),
        position_guidance: history_test_position_guidance(),
        runtime: RuntimeMetadata {
            data_mode: DataMode::Sqlite,
            generated_at: Utc::now(),
            requested_as_of_date: as_of_date,
            latest_observation_at: Some(as_of_date),
            latest_observation_lag_days: Some(0),
            latest_observation_lag_business_days: Some(0),
            latest_key_indicator_at: Some(as_of_date),
            latest_key_indicator_lag_days: Some(0),
            latest_key_indicator_lag_business_days: Some(0),
            demo_mode: false,
            stale_warning: None,
        },
        key_indicators: Vec::new(),
        event_assessment: history_test_event_assessment(),
        backtest_summary: history_test_backtest_summary(),
        user_preferences: history_test_preferences(),
        method: AssessmentMethodVersions {
            score_method_version: "test".to_string(),
            prob_model_version: "test".to_string(),
            calibration_version: "test".to_string(),
            actionability_model_version: None,
            actionability_calibration_version: None,
            feature_set_version: "test".to_string(),
            label_version: "test".to_string(),
            posture_policy_version: "test".to_string(),
            action_playbook_version: "test".to_string(),
            fusion_policy_version: None,
            actionability_enabled: true,
            probability_mode: "test".to_string(),
            release_status: "healthy".to_string(),
            release_id: Some("test-release".to_string()),
            point_in_time_mode: "best_effort".to_string(),
        },
    }
}

fn formal_serving_model_context() -> ServingModelContext {
    ServingModelContext {
        release: ModelReleaseRecord {
            manifest: ModelReleaseManifest {
                release_id: "formal-history-test-release".to_string(),
                market_scope: "financial_system".to_string(),
                status: "active".to_string(),
                probability_mode: "formal_bundle_v1".to_string(),
                serving_status: "healthy".to_string(),
                bundle_uri: "bundle.json".to_string(),
                feature_set_version: FORMAL_MAIN_FEATURE_SET_VERSION.to_string(),
                label_version: FORMAL_MAIN_LABEL_VERSION.to_string(),
                prob_model_version: "prob_bundle_test".to_string(),
                calibration_version: "platt_test".to_string(),
                posture_policy_version: "posture_test".to_string(),
                action_playbook_version: "action_test".to_string(),
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
                note: String::new(),
            },
            created_at: Utc::now(),
            activated_at: None,
            retired_at: None,
        },
        probability_bundle: Some(ProbabilityBundle {
            bundle_id: "bundle".to_string(),
            market_scope: "financial_system".to_string(),
            probability_mode: "formal_bundle_v1".to_string(),
            model_family: "linear_v1".to_string(),
            feature_transform: "identity_v1".to_string(),
            created_at: Utc::now(),
            feature_names: Vec::new(),
            monotonic_min_gap_5d_to_20d: 0.0,
            monotonic_min_gap_20d_to_60d: 0.0,
            note: String::new(),
            horizons: Vec::new(),
            evaluation: None,
            actionability: None,
        }),
        runtime_probability_mode: "formal_bundle_v1".to_string(),
        runtime_release_status: "healthy".to_string(),
    }
}

#[test]
fn history_prepare_hysteresis_promotes_supportive_normal_point_after_prepare_anchor() {
    let mut state = HistoricalPrepareHysteresisState::default();
    let mut anchor_assessment = history_test_assessment(
        DecisionPosture::Prepare,
        fc_domain::TimeToRiskBucket::Months,
        0.63,
        0.79,
        0.22,
        56.0,
        52.0,
        36.0,
        44.0,
        30.0,
    );
    let mut anchor_guidance =
        history_test_posture_guidance(DecisionPosture::Prepare, &["prepare_probability_plateau"]);
    state.apply(true, &mut anchor_assessment, &mut anchor_guidance);
    assert!(state.active);

    let mut assessment = history_test_assessment(
        DecisionPosture::Normal,
        fc_domain::TimeToRiskBucket::Normal,
        0.58,
        0.74,
        0.16,
        50.0,
        47.0,
        35.0,
        42.0,
        30.0,
    );
    let mut guidance = history_test_posture_guidance(DecisionPosture::Normal, &[]);
    state.apply(true, &mut assessment, &mut guidance);

    assert_eq!(assessment.posture, DecisionPosture::Prepare);
    assert_eq!(
        assessment.time_to_risk_bucket,
        fc_domain::TimeToRiskBucket::Months
    );
    assert_eq!(guidance.posture, DecisionPosture::Prepare);
    assert!(guidance
        .trigger_codes
        .iter()
        .any(|code| code == "prepare_history_hysteresis"));
    assert!(state.active);
}

#[test]
fn history_prepare_hysteresis_expires_after_support_breaks() {
    let mut state = HistoricalPrepareHysteresisState::default();
    let mut anchor_assessment = history_test_assessment(
        DecisionPosture::Prepare,
        fc_domain::TimeToRiskBucket::Months,
        0.63,
        0.79,
        0.22,
        56.0,
        52.0,
        36.0,
        44.0,
        30.0,
    );
    let mut anchor_guidance =
        history_test_posture_guidance(DecisionPosture::Prepare, &["prepare_probability_plateau"]);
    state.apply(true, &mut anchor_assessment, &mut anchor_guidance);
    assert!(state.active);

    let mut assessment = history_test_assessment(
        DecisionPosture::Normal,
        fc_domain::TimeToRiskBucket::Normal,
        0.08,
        0.20,
        0.02,
        34.0,
        40.0,
        20.0,
        18.0,
        18.0,
    );
    let mut guidance = history_test_posture_guidance(DecisionPosture::Normal, &[]);
    state.apply(true, &mut assessment, &mut guidance);

    assert_eq!(assessment.posture, DecisionPosture::Normal);
    assert_eq!(
        assessment.time_to_risk_bucket,
        fc_domain::TimeToRiskBucket::Normal
    );
    assert!(!guidance
        .trigger_codes
        .iter()
        .any(|code| code == "prepare_history_hysteresis"));
    assert!(!state.active);
}

#[test]
fn history_prepare_hysteresis_requires_prepare_anchor_trigger_code() {
    let mut state = HistoricalPrepareHysteresisState::default();
    let mut anchor_assessment = history_test_assessment(
        DecisionPosture::Prepare,
        fc_domain::TimeToRiskBucket::Months,
        0.63,
        0.79,
        0.22,
        56.0,
        52.0,
        36.0,
        44.0,
        30.0,
    );
    let mut anchor_guidance = history_test_posture_guidance(DecisionPosture::Prepare, &[]);
    state.apply(true, &mut anchor_assessment, &mut anchor_guidance);
    assert!(!state.active);

    let mut assessment = history_test_assessment(
        DecisionPosture::Normal,
        fc_domain::TimeToRiskBucket::Normal,
        0.58,
        0.74,
        0.16,
        50.0,
        47.0,
        35.0,
        42.0,
        30.0,
    );
    let mut guidance = history_test_posture_guidance(DecisionPosture::Normal, &[]);
    state.apply(true, &mut assessment, &mut guidance);

    assert_eq!(assessment.posture, DecisionPosture::Normal);
    assert_eq!(
        assessment.time_to_risk_bucket,
        fc_domain::TimeToRiskBucket::Normal
    );
    assert!(!guidance
        .trigger_codes
        .iter()
        .any(|code| code == "prepare_history_hysteresis"));
}

#[test]
fn history_prepare_hysteresis_does_not_rescue_moderate_probability_drift() {
    let mut state = HistoricalPrepareHysteresisState::default();
    let mut anchor_assessment = history_test_assessment(
        DecisionPosture::Prepare,
        fc_domain::TimeToRiskBucket::Months,
        0.63,
        0.79,
        0.22,
        56.0,
        52.0,
        36.0,
        44.0,
        30.0,
    );
    let mut anchor_guidance =
        history_test_posture_guidance(DecisionPosture::Prepare, &["prepare_probability_plateau"]);
    state.apply(true, &mut anchor_assessment, &mut anchor_guidance);
    assert!(state.active);

    let mut assessment = history_test_assessment(
        DecisionPosture::Normal,
        fc_domain::TimeToRiskBucket::Normal,
        0.28,
        0.52,
        0.18,
        48.0,
        46.0,
        34.0,
        42.0,
        30.0,
    );
    let mut guidance = history_test_posture_guidance(DecisionPosture::Normal, &[]);
    state.apply(true, &mut assessment, &mut guidance);

    assert_eq!(assessment.posture, DecisionPosture::Normal);
    assert_eq!(
        assessment.time_to_risk_bucket,
        fc_domain::TimeToRiskBucket::Normal
    );
    assert!(!guidance
        .trigger_codes
        .iter()
        .any(|code| code == "prepare_history_hysteresis"));
    assert!(!state.active);
}

#[test]
fn history_prepare_hysteresis_rescues_extreme_carry_after_anchor() {
    let mut state = HistoricalPrepareHysteresisState::default();
    let mut anchor_assessment = history_test_assessment(
        DecisionPosture::Prepare,
        fc_domain::TimeToRiskBucket::Months,
        0.75,
        0.89,
        0.22,
        43.3,
        48.8,
        36.7,
        44.3,
        30.0,
    );
    let mut anchor_guidance =
        history_test_posture_guidance(DecisionPosture::Prepare, &["prepare_probability_plateau"]);
    state.apply(true, &mut anchor_assessment, &mut anchor_guidance);
    assert!(state.active);

    let mut assessment = history_test_assessment(
        DecisionPosture::Normal,
        fc_domain::TimeToRiskBucket::Normal,
        0.733,
        0.763,
        0.06,
        35.5,
        36.7,
        34.0,
        44.5,
        30.0,
    );
    let mut guidance = history_test_posture_guidance(DecisionPosture::Normal, &[]);
    state.apply(true, &mut assessment, &mut guidance);

    assert_eq!(assessment.posture, DecisionPosture::Prepare);
    assert_eq!(
        assessment.time_to_risk_bucket,
        fc_domain::TimeToRiskBucket::Months
    );
    assert_eq!(guidance.posture, DecisionPosture::Prepare);
    assert!(guidance
        .trigger_codes
        .iter()
        .any(|code| code == "prepare_history_hysteresis"));
    assert!(state.active);
}

#[test]
fn history_prepare_hysteresis_extreme_carry_does_not_create_new_anchor() {
    let mut state = HistoricalPrepareHysteresisState::default();
    let mut assessment = history_test_assessment(
        DecisionPosture::Normal,
        fc_domain::TimeToRiskBucket::Normal,
        0.733,
        0.763,
        0.06,
        35.5,
        36.7,
        34.0,
        44.5,
        30.0,
    );
    let mut guidance = history_test_posture_guidance(DecisionPosture::Normal, &[]);
    state.apply(true, &mut assessment, &mut guidance);

    assert_eq!(assessment.posture, DecisionPosture::Normal);
    assert_eq!(
        assessment.time_to_risk_bucket,
        fc_domain::TimeToRiskBucket::Normal
    );
    assert!(!guidance
        .trigger_codes
        .iter()
        .any(|code| code == "prepare_history_hysteresis"));
    assert!(!state.active);
}

#[test]
fn history_prepare_hysteresis_extreme_carry_still_requires_external_support() {
    let mut state = HistoricalPrepareHysteresisState::default();
    let mut anchor_assessment = history_test_assessment(
        DecisionPosture::Prepare,
        fc_domain::TimeToRiskBucket::Months,
        0.75,
        0.89,
        0.22,
        43.3,
        48.8,
        36.7,
        44.3,
        30.0,
    );
    let mut anchor_guidance =
        history_test_posture_guidance(DecisionPosture::Prepare, &["prepare_probability_plateau"]);
    state.apply(true, &mut anchor_assessment, &mut anchor_guidance);
    assert!(state.active);

    let mut assessment = history_test_assessment(
        DecisionPosture::Normal,
        fc_domain::TimeToRiskBucket::Normal,
        0.733,
        0.763,
        0.06,
        35.5,
        36.7,
        34.0,
        41.9,
        30.0,
    );
    let mut guidance = history_test_posture_guidance(DecisionPosture::Normal, &[]);
    state.apply(true, &mut assessment, &mut guidance);

    assert_eq!(assessment.posture, DecisionPosture::Normal);
    assert_eq!(
        assessment.time_to_risk_bucket,
        fc_domain::TimeToRiskBucket::Normal
    );
    assert!(!guidance
        .trigger_codes
        .iter()
        .any(|code| code == "prepare_history_hysteresis"));
}

#[test]
fn history_prepare_hysteresis_extreme_carry_accepts_near_floor_external_support() {
    let mut state = HistoricalPrepareHysteresisState::default();
    let mut anchor_assessment = history_test_assessment(
        DecisionPosture::Prepare,
        fc_domain::TimeToRiskBucket::Months,
        0.75,
        0.89,
        0.22,
        43.3,
        48.8,
        36.7,
        44.3,
        30.0,
    );
    let mut anchor_guidance =
        history_test_posture_guidance(DecisionPosture::Prepare, &["prepare_probability_plateau"]);
    state.apply(true, &mut anchor_assessment, &mut anchor_guidance);
    assert!(state.active);

    let mut assessment = history_test_assessment(
        DecisionPosture::Normal,
        fc_domain::TimeToRiskBucket::Normal,
        0.725,
        0.799,
        0.06,
        36.3,
        36.1,
        36.5,
        43.3,
        30.0,
    );
    let mut guidance = history_test_posture_guidance(DecisionPosture::Normal, &[]);
    state.apply(true, &mut assessment, &mut guidance);

    assert_eq!(assessment.posture, DecisionPosture::Prepare);
    assert_eq!(
        assessment.time_to_risk_bucket,
        fc_domain::TimeToRiskBucket::Months
    );
    assert_eq!(guidance.posture, DecisionPosture::Prepare);
    assert!(guidance
        .trigger_codes
        .iter()
        .any(|code| code == "prepare_history_hysteresis"));
}

#[test]
fn history_prepare_hysteresis_rescues_structural_carry_after_anchor() {
    let mut state = HistoricalPrepareHysteresisState::default();
    let mut anchor_assessment = history_test_assessment(
        DecisionPosture::Prepare,
        fc_domain::TimeToRiskBucket::Months,
        0.63,
        0.79,
        0.22,
        56.0,
        52.0,
        36.0,
        44.0,
        30.0,
    );
    let mut anchor_guidance =
        history_test_posture_guidance(DecisionPosture::Prepare, &["prepare_probability_plateau"]);
    state.apply(true, &mut anchor_assessment, &mut anchor_guidance);
    assert!(state.active);

    let mut assessment = history_test_assessment(
        DecisionPosture::Normal,
        fc_domain::TimeToRiskBucket::Normal,
        0.37,
        0.658,
        0.24,
        43.9,
        63.1,
        20.6,
        37.0,
        30.0,
    );
    let mut guidance = history_test_posture_guidance(DecisionPosture::Normal, &[]);
    state.apply(true, &mut assessment, &mut guidance);

    assert_eq!(assessment.posture, DecisionPosture::Prepare);
    assert_eq!(
        assessment.time_to_risk_bucket,
        fc_domain::TimeToRiskBucket::Months
    );
    assert_eq!(guidance.posture, DecisionPosture::Prepare);
    assert!(guidance
        .trigger_codes
        .iter()
        .any(|code| code == "prepare_history_hysteresis"));
    assert!(state.active);
}

#[test]
fn history_prepare_hysteresis_structural_carry_requires_low_trigger_context() {
    let mut state = HistoricalPrepareHysteresisState::default();
    let mut anchor_assessment = history_test_assessment(
        DecisionPosture::Prepare,
        fc_domain::TimeToRiskBucket::Months,
        0.63,
        0.79,
        0.22,
        56.0,
        52.0,
        36.0,
        44.0,
        30.0,
    );
    let mut anchor_guidance =
        history_test_posture_guidance(DecisionPosture::Prepare, &["prepare_probability_plateau"]);
    state.apply(true, &mut anchor_assessment, &mut anchor_guidance);
    assert!(state.active);

    let mut assessment = history_test_assessment(
        DecisionPosture::Normal,
        fc_domain::TimeToRiskBucket::Normal,
        0.37,
        0.658,
        0.24,
        43.9,
        63.1,
        32.0,
        37.0,
        30.0,
    );
    let mut guidance = history_test_posture_guidance(DecisionPosture::Normal, &[]);
    state.apply(true, &mut assessment, &mut guidance);

    assert_eq!(assessment.posture, DecisionPosture::Normal);
    assert_eq!(
        assessment.time_to_risk_bucket,
        fc_domain::TimeToRiskBucket::Normal
    );
    assert!(!guidance
        .trigger_codes
        .iter()
        .any(|code| code == "prepare_history_hysteresis"));
}

#[test]
fn history_prepare_hysteresis_rescues_structural_carry_with_lower_p20d_after_anchor() {
    let mut state = HistoricalPrepareHysteresisState::default();
    let mut anchor_assessment = history_test_assessment(
        DecisionPosture::Prepare,
        fc_domain::TimeToRiskBucket::Months,
        0.63,
        0.79,
        0.22,
        56.0,
        52.0,
        36.0,
        44.0,
        30.0,
    );
    let mut anchor_guidance =
        history_test_posture_guidance(DecisionPosture::Prepare, &["prepare_probability_plateau"]);
    state.apply(true, &mut anchor_assessment, &mut anchor_guidance);
    assert!(state.active);

    let mut assessment = history_test_assessment(
        DecisionPosture::Normal,
        fc_domain::TimeToRiskBucket::Normal,
        0.26,
        0.804,
        0.18,
        44.1,
        60.4,
        24.0,
        35.3,
        30.0,
    );
    let mut guidance = history_test_posture_guidance(DecisionPosture::Normal, &[]);
    state.apply(true, &mut assessment, &mut guidance);

    assert_eq!(assessment.posture, DecisionPosture::Prepare);
    assert_eq!(
        assessment.time_to_risk_bucket,
        fc_domain::TimeToRiskBucket::Months
    );
    assert_eq!(guidance.posture, DecisionPosture::Prepare);
    assert!(guidance
        .trigger_codes
        .iter()
        .any(|code| code == "prepare_history_hysteresis"));
}

#[test]
fn history_prepare_hysteresis_rescues_structural_carry_near_structural_floor() {
    let mut state = HistoricalPrepareHysteresisState::default();
    let mut anchor_assessment = history_test_assessment(
        DecisionPosture::Prepare,
        fc_domain::TimeToRiskBucket::Months,
        0.63,
        0.79,
        0.22,
        56.0,
        52.0,
        36.0,
        44.0,
        30.0,
    );
    let mut anchor_guidance =
        history_test_posture_guidance(DecisionPosture::Prepare, &["prepare_probability_plateau"]);
    state.apply(true, &mut anchor_assessment, &mut anchor_guidance);
    assert!(state.active);

    let mut assessment = history_test_assessment(
        DecisionPosture::Normal,
        fc_domain::TimeToRiskBucket::Normal,
        0.309,
        0.804,
        0.18,
        44.0,
        58.2,
        26.6,
        31.5,
        30.0,
    );
    let mut guidance = history_test_posture_guidance(DecisionPosture::Normal, &[]);
    state.apply(true, &mut assessment, &mut guidance);

    assert_eq!(assessment.posture, DecisionPosture::Prepare);
    assert_eq!(
        assessment.time_to_risk_bucket,
        fc_domain::TimeToRiskBucket::Months
    );
    assert_eq!(guidance.posture, DecisionPosture::Prepare);
    assert!(guidance
        .trigger_codes
        .iter()
        .any(|code| code == "prepare_history_hysteresis"));
}

#[test]
fn history_prepare_hysteresis_structural_carry_still_requires_structural_floor() {
    let mut state = HistoricalPrepareHysteresisState::default();
    let mut anchor_assessment = history_test_assessment(
        DecisionPosture::Prepare,
        fc_domain::TimeToRiskBucket::Months,
        0.63,
        0.79,
        0.22,
        56.0,
        52.0,
        36.0,
        44.0,
        30.0,
    );
    let mut anchor_guidance =
        history_test_posture_guidance(DecisionPosture::Prepare, &["prepare_probability_plateau"]);
    state.apply(true, &mut anchor_assessment, &mut anchor_guidance);
    assert!(state.active);

    let mut assessment = history_test_assessment(
        DecisionPosture::Normal,
        fc_domain::TimeToRiskBucket::Normal,
        0.309,
        0.804,
        0.18,
        44.0,
        57.9,
        26.6,
        31.5,
        30.0,
    );
    let mut guidance = history_test_posture_guidance(DecisionPosture::Normal, &[]);
    state.apply(true, &mut assessment, &mut guidance);

    assert_eq!(assessment.posture, DecisionPosture::Normal);
    assert_eq!(
        assessment.time_to_risk_bucket,
        fc_domain::TimeToRiskBucket::Normal
    );
    assert!(!guidance
        .trigger_codes
        .iter()
        .any(|code| code == "prepare_history_hysteresis"));
}

#[test]
fn history_prepare_hysteresis_carry_grace_preserves_state_for_next_day_rescue() {
    let mut state = HistoricalPrepareHysteresisState::default();
    let mut anchor_assessment = history_test_assessment(
        DecisionPosture::Prepare,
        fc_domain::TimeToRiskBucket::Months,
        0.63,
        0.79,
        0.22,
        56.0,
        52.0,
        36.0,
        44.0,
        30.0,
    );
    let mut anchor_guidance =
        history_test_posture_guidance(DecisionPosture::Prepare, &["prepare_probability_plateau"]);
    state.apply(true, &mut anchor_assessment, &mut anchor_guidance);
    assert!(state.active);

    let mut grace_day = history_test_assessment(
        DecisionPosture::Normal,
        fc_domain::TimeToRiskBucket::Normal,
        0.075,
        0.77,
        0.18,
        42.3,
        55.0,
        26.8,
        27.4,
        30.0,
    );
    let mut grace_guidance = history_test_posture_guidance(DecisionPosture::Normal, &[]);
    state.apply(true, &mut grace_day, &mut grace_guidance);

    assert_eq!(grace_day.posture, DecisionPosture::Normal);
    assert_eq!(
        grace_day.time_to_risk_bucket,
        fc_domain::TimeToRiskBucket::Normal
    );
    assert!(state.active);

    let mut rescue_day = history_test_assessment(
        DecisionPosture::Normal,
        fc_domain::TimeToRiskBucket::Normal,
        0.362,
        0.85,
        0.18,
        44.6,
        58.2,
        28.0,
        30.2,
        30.0,
    );
    let mut rescue_guidance = history_test_posture_guidance(DecisionPosture::Normal, &[]);
    state.apply(true, &mut rescue_day, &mut rescue_guidance);

    assert_eq!(rescue_day.posture, DecisionPosture::Prepare);
    assert_eq!(
        rescue_day.time_to_risk_bucket,
        fc_domain::TimeToRiskBucket::Months
    );
    assert_eq!(rescue_guidance.posture, DecisionPosture::Prepare);
    assert!(rescue_guidance
        .trigger_codes
        .iter()
        .any(|code| code == "prepare_history_hysteresis"));
}

#[test]
fn history_prepare_hysteresis_carry_grace_expires_after_one_day() {
    let mut state = HistoricalPrepareHysteresisState::default();
    let mut anchor_assessment = history_test_assessment(
        DecisionPosture::Prepare,
        fc_domain::TimeToRiskBucket::Months,
        0.63,
        0.79,
        0.22,
        56.0,
        52.0,
        36.0,
        44.0,
        30.0,
    );
    let mut anchor_guidance =
        history_test_posture_guidance(DecisionPosture::Prepare, &["prepare_probability_plateau"]);
    state.apply(true, &mut anchor_assessment, &mut anchor_guidance);
    assert!(state.active);

    let mut first_grace_day = history_test_assessment(
        DecisionPosture::Normal,
        fc_domain::TimeToRiskBucket::Normal,
        0.075,
        0.77,
        0.18,
        42.3,
        55.0,
        26.8,
        27.4,
        30.0,
    );
    let mut first_grace_guidance = history_test_posture_guidance(DecisionPosture::Normal, &[]);
    state.apply(true, &mut first_grace_day, &mut first_grace_guidance);
    assert!(state.active);

    let mut second_grace_day = history_test_assessment(
        DecisionPosture::Normal,
        fc_domain::TimeToRiskBucket::Normal,
        0.075,
        0.77,
        0.18,
        42.3,
        55.0,
        26.8,
        27.4,
        30.0,
    );
    let mut second_grace_guidance = history_test_posture_guidance(DecisionPosture::Normal, &[]);
    state.apply(true, &mut second_grace_day, &mut second_grace_guidance);

    assert_eq!(second_grace_day.posture, DecisionPosture::Normal);
    assert_eq!(
        second_grace_day.time_to_risk_bucket,
        fc_domain::TimeToRiskBucket::Normal
    );
    assert!(!state.active);

    let mut rescue_day = history_test_assessment(
        DecisionPosture::Normal,
        fc_domain::TimeToRiskBucket::Normal,
        0.362,
        0.85,
        0.18,
        44.6,
        58.2,
        28.0,
        30.2,
        30.0,
    );
    let mut rescue_guidance = history_test_posture_guidance(DecisionPosture::Normal, &[]);
    state.apply(true, &mut rescue_day, &mut rescue_guidance);

    assert_eq!(rescue_day.posture, DecisionPosture::Normal);
    assert_eq!(
        rescue_day.time_to_risk_bucket,
        fc_domain::TimeToRiskBucket::Normal
    );
    assert!(!rescue_guidance
        .trigger_codes
        .iter()
        .any(|code| code == "prepare_history_hysteresis"));
}

#[test]
fn history_prepare_hysteresis_structural_carry_still_requires_p20d_decay_floor() {
    let mut state = HistoricalPrepareHysteresisState::default();
    let mut anchor_assessment = history_test_assessment(
        DecisionPosture::Prepare,
        fc_domain::TimeToRiskBucket::Months,
        0.63,
        0.79,
        0.22,
        56.0,
        52.0,
        36.0,
        44.0,
        30.0,
    );
    let mut anchor_guidance =
        history_test_posture_guidance(DecisionPosture::Prepare, &["prepare_probability_plateau"]);
    state.apply(true, &mut anchor_assessment, &mut anchor_guidance);
    assert!(state.active);

    let mut assessment = history_test_assessment(
        DecisionPosture::Normal,
        fc_domain::TimeToRiskBucket::Normal,
        0.24,
        0.804,
        0.18,
        44.1,
        60.4,
        24.0,
        35.3,
        30.0,
    );
    let mut guidance = history_test_posture_guidance(DecisionPosture::Normal, &[]);
    state.apply(true, &mut assessment, &mut guidance);

    assert_eq!(assessment.posture, DecisionPosture::Normal);
    assert_eq!(
        assessment.time_to_risk_bucket,
        fc_domain::TimeToRiskBucket::Normal
    );
    assert!(!guidance
        .trigger_codes
        .iter()
        .any(|code| code == "prepare_history_hysteresis"));
}

fn persisted_snapshot(
    as_of_date: NaiveDate,
    release_id: Option<&str>,
    method_version: &str,
) -> PredictionSnapshotRecord {
    PredictionSnapshotRecord {
        as_of_date,
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        release_id: release_id.map(str::to_string),
        probability_mode: "formal_bundle_v1".to_string(),
        release_status: "healthy".to_string(),
        point_in_time_mode: "best_effort".to_string(),
        overall_score: 99.0,
        external_shock_score: 99.0,
        raw_p_5d: 0.99,
        raw_p_20d: 0.99,
        raw_p_60d: 0.99,
        calibrated_p_5d: 0.99,
        calibrated_p_20d: 0.99,
        calibrated_p_60d: 0.99,
        posture: "defend".to_string(),
        time_to_risk_bucket: "now".to_string(),
        feature_set_version: FORMAL_MAIN_FEATURE_SET_VERSION.to_string(),
        label_version: FORMAL_MAIN_LABEL_VERSION.to_string(),
        coverage_score: 1.0,
        freshness_status: "fresh".to_string(),
        method_version: method_version.to_string(),
        posture_trigger_codes: vec!["legacy_snapshot_only".to_string()],
        posture_blocker_codes: Vec::new(),
        recorded_at: Utc::now(),
    }
}

#[tokio::test]
async fn default_mode_bundle_release_rebuilds_full_history_when_replay_cache_is_missing() {
    let store = in_memory_store().await;
    store.seed_fred_metadata().await.unwrap();
    let as_of_date = NaiveDate::from_ymd_opt(2026, 5, 30).unwrap();
    let indicators = demo_indicators();
    for indicator in &indicators {
        store.upsert_indicator(indicator).await.unwrap();
    }
    let observations = demo_observations(as_of_date);
    store.insert_observations(&observations).await.unwrap();

    let serving_model = formal_serving_model_context();
    store
        .upsert_model_release(&serving_model.release)
        .await
        .unwrap();
    let release_id = serving_model.release.manifest.release_id.clone();
    let method_version = expected_prediction_snapshot_method_version(Some(&serving_model));

    let target_dates = observations
        .iter()
        .filter(|observation| observation.entity_id == "us")
        .map(|observation| observation.as_of_date)
        .collect::<std::collections::BTreeSet<_>>();
    let persisted = target_dates
        .iter()
        .copied()
        .map(|date| persisted_snapshot(date, Some(&release_id), &method_version))
        .collect::<Vec<_>>();
    store.upsert_prediction_snapshots(&persisted).await.unwrap();

    let history = load_sqlite_assessment_history(
        &store,
        &indicators,
        &observations,
        &[],
        Some(&serving_model),
        &load_user_preferences(),
        as_of_date,
        260,
        AssessmentHistoryBuildMode::Default,
    )
    .await
    .unwrap();

    assert_eq!(history.len(), target_dates.len());
    assert!(
        history.iter().any(|point| point.overall_score != 99.0),
        "default mode should rebuild bundle-backed history from raw observations instead of reusing only persisted snapshots"
    );
    assert!(
        history.iter().any(|point| point.p_20d != 0.99),
        "rebuilt history should not mirror the placeholder snapshot probabilities"
    );
    assert!(
        history
            .iter()
            .all(|point| point.history_source.as_deref() == Some("raw_pit_feature_replay")),
        "once bundle history rebuild can persist exact same-day PIT snapshots, the returned history should already expose the stronger raw_pit_feature_replay source instead of stopping at raw observation replay"
    );

    let replay_runs = store
        .list_historical_replay_runs(
            Some("financial_system"),
            Some(&release_id),
            None,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(replay_runs.len(), 1);
    assert_eq!(replay_runs[0].point_count, target_dates.len());
    assert_eq!(replay_runs[0].status, "success");
    assert!(
        history
            .iter()
            .all(|point| point.replay_run_id.as_deref() == Some(replay_runs[0].replay_run_id.as_str())),
        "freshly rebuilt bundle-backed history should expose the persisted replay run id immediately instead of waiting for a second cache-backed read"
    );
}

#[tokio::test]
async fn bundle_history_rebuild_binds_persisted_feature_snapshot_ids_when_available() {
    let store = in_memory_store().await;
    store.seed_fred_metadata().await.unwrap();
    let as_of_date = NaiveDate::from_ymd_opt(2026, 5, 30).unwrap();
    let indicators = demo_indicators();
    for indicator in &indicators {
        store.upsert_indicator(indicator).await.unwrap();
    }
    let observations = demo_observations(as_of_date);
    store.insert_observations(&observations).await.unwrap();

    let serving_model = formal_serving_model_context();
    store
        .upsert_model_release(&serving_model.release)
        .await
        .unwrap();
    let release_id = serving_model.release.manifest.release_id.clone();

    let target_dates = observations
        .iter()
        .filter(|observation| observation.entity_id == "us")
        .map(|observation| observation.as_of_date)
        .collect::<std::collections::BTreeSet<_>>();
    let feature_snapshots = target_dates
        .iter()
        .copied()
        .map(history_test_feature_snapshot)
        .collect::<Vec<_>>();
    store
        .upsert_feature_snapshots(&feature_snapshots)
        .await
        .unwrap();

    let history = load_sqlite_assessment_history(
        &store,
        &indicators,
        &observations,
        &[],
        Some(&serving_model),
        &load_user_preferences(),
        as_of_date,
        260,
        AssessmentHistoryBuildMode::Default,
    )
    .await
    .unwrap();

    assert_eq!(history.len(), target_dates.len());
    assert!(
        history.iter().all(|point| point.feature_snapshot_id.is_some()),
        "bundle-backed history should expose persisted PIT feature snapshot ids when matching snapshots exist"
    );
    assert!(
        history
            .iter()
            .all(|point| point.history_source.as_deref() == Some("raw_pit_feature_replay")),
        "history points backed by persisted feature snapshots should be marked as raw PIT feature replay"
    );

    let replay_runs = store
        .list_historical_replay_runs(
            Some("financial_system"),
            Some(&release_id),
            None,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(replay_runs.len(), 1);
    assert!(
        history
            .iter()
            .all(|point| point.replay_run_id.as_deref() == Some(replay_runs[0].replay_run_id.as_str())),
        "history points backed by persisted PIT snapshots should still expose the replay run id on the first rebuild response"
    );

    let replay_points = store
        .list_historical_assessment_points(
            Some(&replay_runs[0].replay_run_id),
            Some("financial_system"),
            Some(&release_id),
            None,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(replay_points.len(), target_dates.len());
    assert!(replay_points
        .iter()
        .all(|point| point.feature_snapshot_id.is_some()));
}

#[tokio::test]
async fn bundle_history_rebuild_materializes_exact_snapshot_when_no_new_visible_data_arrived() {
    let store = in_memory_store().await;
    store.seed_fred_metadata().await.unwrap();
    let as_of_date = NaiveDate::from_ymd_opt(2026, 5, 30).unwrap();
    let indicators = demo_indicators();
    for indicator in &indicators {
        store.upsert_indicator(indicator).await.unwrap();
    }
    let observations = demo_observations(as_of_date);
    store.insert_observations(&observations).await.unwrap();

    let serving_model = formal_serving_model_context();
    store
        .upsert_model_release(&serving_model.release)
        .await
        .unwrap();

    let target_dates = observations
        .iter()
        .filter(|observation| observation.entity_id == "us")
        .map(|observation| observation.as_of_date)
        .chain(std::iter::once(as_of_date))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let latest_date = *target_dates.last().unwrap();
    let prior_date = target_dates[target_dates.len() - 2];
    let feature_snapshots = target_dates[..target_dates.len() - 1]
        .iter()
        .copied()
        .map(history_test_feature_snapshot)
        .collect::<Vec<_>>();
    store
        .upsert_feature_snapshots(&feature_snapshots)
        .await
        .unwrap();

    let history = load_sqlite_assessment_history(
        &store,
        &indicators,
        &observations,
        &[],
        Some(&serving_model),
        &load_user_preferences(),
        as_of_date,
        260,
        AssessmentHistoryBuildMode::Default,
    )
    .await
    .unwrap();

    let latest_point = history
        .iter()
        .find(|point| point.as_of_date == latest_date)
        .expect("latest history point");
    let prior_point = history
        .iter()
        .find(|point| point.as_of_date == prior_date)
        .expect("prior history point");

    assert_eq!(
        latest_point.history_source.as_deref(),
        Some("raw_pit_feature_replay"),
        "when no new visible data arrived after the prior exact PIT snapshot, history should materialize an exact same-day PIT snapshot instead of leaving the point in reuse state"
    );
    assert_ne!(
        latest_point.feature_snapshot_id, prior_point.feature_snapshot_id,
        "latest date should receive its own exact PIT snapshot id once carry-forward materialization is possible"
    );
    assert!(
        latest_point.replay_run_id.is_some(),
        "same-day point should keep the replay run id after raw rebuild metadata is persisted"
    );

    let latest_snapshots = store
        .list_feature_snapshots_for_mode(
            &serving_model.release.manifest.market_scope,
            &serving_model.release.manifest.feature_set_version,
            &serving_model.release.manifest.point_in_time_mode,
            Some(latest_date),
            Some(latest_date),
        )
        .await
        .unwrap();
    assert!(
        latest_snapshots.iter().any(|snapshot| snapshot.as_of_date == latest_date),
        "API should persist an exact same-day PIT feature snapshot when the prior snapshot can be carried forward without new visible data"
    );
}

#[tokio::test]
async fn bundle_history_rebuild_rebuilds_exact_snapshot_when_new_visible_data_arrived() {
    let store = in_memory_store().await;
    store.seed_fred_metadata().await.unwrap();
    let as_of_date = NaiveDate::from_ymd_opt(2026, 5, 30).unwrap();
    let indicators = demo_indicators();
    for indicator in &indicators {
        store.upsert_indicator(indicator).await.unwrap();
    }
    let observations = demo_observations(as_of_date);
    store.insert_observations(&observations).await.unwrap();

    let serving_model = formal_serving_model_context();
    store
        .upsert_model_release(&serving_model.release)
        .await
        .unwrap();

    let target_dates = observations
        .iter()
        .filter(|observation| observation.entity_id == "us")
        .map(|observation| observation.as_of_date)
        .chain(std::iter::once(as_of_date))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let latest_date = *target_dates.last().unwrap();
    let prior_date = target_dates[target_dates.len() - 3];
    let mut prior_snapshot = history_test_feature_snapshot(prior_date);
    prior_snapshot.latest_visible_at = prior_date
        .and_hms_opt(22, 0, 0)
        .map(|value| chrono::DateTime::<Utc>::from_naive_utc_and_offset(value, Utc));
    store
        .upsert_feature_snapshots(&[prior_snapshot.clone()])
        .await
        .unwrap();

    let history = load_sqlite_assessment_history(
        &store,
        &indicators,
        &observations,
        &[],
        Some(&serving_model),
        &load_user_preferences(),
        as_of_date,
        260,
        AssessmentHistoryBuildMode::Default,
    )
    .await
    .unwrap();

    let latest_point = history
        .iter()
        .find(|point| point.as_of_date == latest_date)
        .expect("latest history point");
    assert_eq!(
        latest_point.history_source.as_deref(),
        Some("raw_pit_feature_replay"),
        "when newer observations became visible after the prior snapshot, API should rebuild a true same-day PIT feature snapshot instead of staying in reuse state"
    );
    let rebuilt_snapshot_id =
        format!("financial_system:us:{latest_date}:{FORMAL_MAIN_FEATURE_SET_VERSION}:best_effort");
    assert_eq!(
        latest_point.feature_snapshot_id.as_deref(),
        Some(rebuilt_snapshot_id.as_str()),
        "rebuilt mode should bind the latest point to the exact same-day PIT snapshot id"
    );

    let latest_snapshots = store
        .list_feature_snapshots_for_mode(
            &serving_model.release.manifest.market_scope,
            &serving_model.release.manifest.feature_set_version,
            &serving_model.release.manifest.point_in_time_mode,
            Some(latest_date),
            Some(latest_date),
        )
        .await
        .unwrap();
    assert!(
        latest_snapshots.iter().any(|snapshot| snapshot.as_of_date == latest_date),
        "API should persist the rebuilt same-day PIT feature snapshot after newer visible data arrives"
    );
}

#[tokio::test]
async fn default_mode_heuristic_history_can_still_reuse_persisted_snapshots() {
    let store = in_memory_store().await;
    store.seed_fred_metadata().await.unwrap();
    let as_of_date = NaiveDate::from_ymd_opt(2026, 5, 30).unwrap();
    let indicators = demo_indicators();
    for indicator in &indicators {
        store.upsert_indicator(indicator).await.unwrap();
    }
    let observations = demo_observations(as_of_date);
    store.insert_observations(&observations).await.unwrap();

    let target_dates = observations
        .iter()
        .filter(|observation| observation.entity_id == "us")
        .map(|observation| observation.as_of_date)
        .collect::<std::collections::BTreeSet<_>>();
    let persisted = target_dates
        .iter()
        .copied()
        .map(|date| persisted_snapshot(date, None, "heuristic_history_test"))
        .collect::<Vec<_>>();
    store.upsert_prediction_snapshots(&persisted).await.unwrap();

    let history = load_sqlite_assessment_history(
        &store,
        &indicators,
        &observations,
        &[],
        None,
        &load_user_preferences(),
        as_of_date,
        260,
        AssessmentHistoryBuildMode::Default,
    )
    .await
    .unwrap();

    assert_eq!(history.len(), target_dates.len());
    assert!(history.iter().all(|point| point.overall_score == 99.0));
    assert!(history.iter().all(|point| point.p_20d == 0.99));

    let replay_runs = store
        .list_historical_replay_runs(Some("financial_system"), None, None, None, None)
        .await
        .unwrap();
    assert!(
        replay_runs.is_empty(),
        "heuristic history should still be allowed to reuse persisted snapshots without creating replay runs"
    );
}

#[tokio::test]
async fn bundle_history_rebuild_does_not_backfill_prediction_snapshots() {
    let store = in_memory_store().await;
    store.seed_fred_metadata().await.unwrap();
    let as_of_date = NaiveDate::from_ymd_opt(2026, 5, 30).unwrap();
    let indicators = demo_indicators();
    for indicator in &indicators {
        store.upsert_indicator(indicator).await.unwrap();
    }
    let observations = demo_observations(as_of_date);
    store.insert_observations(&observations).await.unwrap();

    let serving_model = formal_serving_model_context();
    store
        .upsert_model_release(&serving_model.release)
        .await
        .unwrap();
    let release_id = serving_model.release.manifest.release_id.clone();

    let history = load_sqlite_assessment_history(
        &store,
        &indicators,
        &observations,
        &[],
        Some(&serving_model),
        &load_user_preferences(),
        as_of_date,
        260,
        AssessmentHistoryBuildMode::Default,
    )
    .await
    .unwrap();

    let target_dates = observations
        .iter()
        .filter(|observation| observation.entity_id == "us")
        .map(|observation| observation.as_of_date)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(history.len(), target_dates.len());

    let replay_runs = store
        .list_historical_replay_runs(
            Some("financial_system"),
            Some(&release_id),
            None,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(replay_runs.len(), 1);
    assert_eq!(replay_runs[0].point_count, target_dates.len());

    let snapshots = store
        .list_prediction_snapshots(
            Some("financial_system"),
            Some(&release_id),
            None,
            None,
            None,
        )
        .await
        .unwrap();
    assert!(
        snapshots.is_empty(),
        "bundle-backed history rebuild should persist replay points only, not backfill prediction snapshots"
    );
}

#[tokio::test]
async fn default_mode_bundle_release_ignores_partial_prediction_snapshot_bridge() {
    let store = in_memory_store().await;
    store.seed_fred_metadata().await.unwrap();
    let as_of_date = NaiveDate::from_ymd_opt(2026, 5, 30).unwrap();
    let indicators = demo_indicators();
    for indicator in &indicators {
        store.upsert_indicator(indicator).await.unwrap();
    }
    let observations = demo_observations(as_of_date);
    store.insert_observations(&observations).await.unwrap();

    let serving_model = formal_serving_model_context();
    store
        .upsert_model_release(&serving_model.release)
        .await
        .unwrap();
    let release_id = serving_model.release.manifest.release_id.clone();
    let method_version = expected_prediction_snapshot_method_version(Some(&serving_model));

    let target_dates = observations
        .iter()
        .filter(|observation| observation.entity_id == "us")
        .map(|observation| observation.as_of_date)
        .collect::<std::collections::BTreeSet<_>>();
    let partial_snapshot_date = target_dates.iter().copied().next().unwrap();
    store
        .upsert_prediction_snapshots(&[persisted_snapshot(
            partial_snapshot_date,
            Some(&release_id),
            &method_version,
        )])
        .await
        .unwrap();

    let history = load_sqlite_assessment_history(
        &store,
        &indicators,
        &observations,
        &[],
        Some(&serving_model),
        &load_user_preferences(),
        as_of_date,
        260,
        AssessmentHistoryBuildMode::Default,
    )
    .await
    .unwrap();

    assert_eq!(history.len(), target_dates.len());
    assert!(
        history.iter().any(|point| point.overall_score != 99.0),
        "bundle-backed default history should rebuild from raw even if partial prediction snapshots exist"
    );

    let replay_runs = store
        .list_historical_replay_runs(
            Some("financial_system"),
            Some(&release_id),
            None,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(replay_runs.len(), 1);
    assert_eq!(replay_runs[0].point_count, target_dates.len());

    let snapshots = store
        .list_prediction_snapshots(
            Some("financial_system"),
            Some(&release_id),
            None,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].as_of_date, partial_snapshot_date);
}

#[tokio::test]
async fn default_mode_bundle_release_respects_history_limit_window() {
    let store = in_memory_store().await;
    store.seed_fred_metadata().await.unwrap();
    let as_of_date = NaiveDate::from_ymd_opt(2026, 5, 30).unwrap();
    let indicators = demo_indicators();
    for indicator in &indicators {
        store.upsert_indicator(indicator).await.unwrap();
    }
    let observations = demo_observations(as_of_date);
    store.insert_observations(&observations).await.unwrap();

    let serving_model = formal_serving_model_context();
    store
        .upsert_model_release(&serving_model.release)
        .await
        .unwrap();
    let release_id = serving_model.release.manifest.release_id.clone();
    let target_dates = observations
        .iter()
        .filter(|observation| observation.entity_id == "us")
        .map(|observation| observation.as_of_date)
        .collect::<std::collections::BTreeSet<_>>();

    let history = load_sqlite_assessment_history(
        &store,
        &indicators,
        &observations,
        &[],
        Some(&serving_model),
        &load_user_preferences(),
        as_of_date,
        25,
        AssessmentHistoryBuildMode::Default,
    )
    .await
    .unwrap();

    let expected_points = target_dates.len().min(25);
    assert_eq!(history.len(), expected_points);

    let replay_runs = store
        .list_historical_replay_runs(
            Some("financial_system"),
            Some(&release_id),
            None,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(replay_runs.len(), 1);
    assert_eq!(replay_runs[0].point_count, expected_points);
}

#[tokio::test]
async fn strict_rebuild_bundle_release_bypasses_matching_replay_cache() {
    let store = in_memory_store().await;
    store.seed_fred_metadata().await.unwrap();
    let as_of_date = NaiveDate::from_ymd_opt(2026, 5, 30).unwrap();
    let indicators = demo_indicators();
    for indicator in &indicators {
        store.upsert_indicator(indicator).await.unwrap();
    }
    let observations = demo_observations(as_of_date);
    store.insert_observations(&observations).await.unwrap();

    let serving_model = formal_serving_model_context();
    store
        .upsert_model_release(&serving_model.release)
        .await
        .unwrap();
    let release_id = serving_model.release.manifest.release_id.clone();
    let history_cache_key = expected_prediction_snapshot_method_version(Some(&serving_model));
    let target_dates = observations
        .iter()
        .filter(|observation| observation.entity_id == "us")
        .map(|observation| observation.as_of_date)
        .collect::<std::collections::BTreeSet<_>>();
    let from_date = *target_dates.first().unwrap();
    let to_date = *target_dates.last().unwrap();
    let source_watermark = format!("us_observations={to_date}");
    let replay_run_id = "cached-strict-run".to_string();

    store
        .upsert_historical_replay_run(&HistoricalReplayRunRecord {
            replay_run_id: replay_run_id.clone(),
            release_id: Some(release_id.clone()),
            market_scope: "financial_system".to_string(),
            from_date,
            to_date,
            history_cache_key,
            feature_set_version: FORMAL_MAIN_FEATURE_SET_VERSION.to_string(),
            label_version: FORMAL_MAIN_LABEL_VERSION.to_string(),
            point_in_time_mode: "best_effort".to_string(),
            runtime_policy_version: "cached-runtime".to_string(),
            action_playbook_version: "action_test".to_string(),
            protected_window_catalog_id: "catalog".to_string(),
            source_watermark,
            status: "success".to_string(),
            point_count: target_dates.len(),
            failure_reason: None,
            created_at: Utc::now(),
        })
        .await
        .unwrap();

    let cached_points = target_dates
        .iter()
        .copied()
        .map(|date| HistoricalAssessmentPointRecord {
            replay_run_id: replay_run_id.clone(),
            entity_id: "us".to_string(),
            market_scope: "financial_system".to_string(),
            release_id: Some(release_id.clone()),
            as_of_date: date,
            feature_snapshot_id: None,
            point_in_time_mode: "best_effort".to_string(),
            runtime_policy_version: "cached-runtime".to_string(),
            action_playbook_version: "action_test".to_string(),
            overall_score: 99.0,
            structural_score: 99.0,
            trigger_score: 99.0,
            external_shock_score: 99.0,
            raw_p_5d: 0.99,
            raw_p_20d: 0.99,
            raw_p_60d: 0.99,
            calibrated_p_5d: 0.99,
            calibrated_p_20d: 0.99,
            calibrated_p_60d: 0.99,
            posture: "defend".to_string(),
            time_to_risk_bucket: "now".to_string(),
            actionability_prepare: 0.99,
            actionability_hedge: 0.99,
            actionability_defend: 0.99,
            probability_diagnostics: ProbabilityDiagnostics::default(),
            posture_trigger_codes: vec!["cached_only".to_string()],
            posture_blocker_codes: Vec::new(),
            coverage_score: 1.0,
            freshness_status: "fresh".to_string(),
            generated_at: Utc::now(),
        })
        .collect::<Vec<_>>();
    store
        .replace_historical_assessment_points(&replay_run_id, &cached_points)
        .await
        .unwrap();

    let history = load_sqlite_assessment_history(
        &store,
        &indicators,
        &observations,
        &[],
        Some(&serving_model),
        &load_user_preferences(),
        as_of_date,
        260,
        AssessmentHistoryBuildMode::StrictRebuild,
    )
    .await
    .unwrap();

    assert_eq!(history.len(), target_dates.len());
    assert!(
        history.iter().any(|point| point.overall_score != 99.0),
        "strict rebuild should bypass matching replay cache and recompute from raw observations"
    );
    assert!(
        history
            .iter()
            .all(|point| point.posture_trigger_codes != vec!["cached_only".to_string()]),
        "strict rebuild should not return cached replay-only trigger codes"
    );

    let replay_runs = store
        .list_historical_replay_runs(
            Some("financial_system"),
            Some(&release_id),
            None,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(replay_runs.len(), 2);
    assert_eq!(replay_runs[0].point_count, target_dates.len());
    assert_ne!(replay_runs[0].replay_run_id, replay_run_id);
}

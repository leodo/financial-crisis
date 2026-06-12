use std::collections::BTreeMap;

use chrono::{NaiveDate, Utc};
use fc_domain::{
    ActionEpisodeTemplateId, AssessmentHistoryPoint, AssessmentMethodVersions,
    BacktestScenarioSummary, BacktestSignalSource, DecisionPosture, Frequency,
    ModelReleaseManifest, ModelReleaseRecord, Observation, ProbabilityBundle, RiskLevel,
    TimeToRiskBucket,
};

use crate::{
    CrisisScenario, ProbabilityTrainingRegime, ProbabilityTrainingRow,
    RuntimeThresholdDiagnosticsWire,
};

pub(super) fn observation(
    source_id: &str,
    frequency: Frequency,
    as_of_date: NaiveDate,
    publication_time: Option<chrono::DateTime<Utc>>,
) -> Observation {
    Observation {
        indicator_id: "test_indicator".to_string(),
        entity_id: "us".to_string(),
        as_of_date,
        period_start: Some(as_of_date),
        period_end: Some(as_of_date),
        frequency,
        value: 1.0,
        unit: "value".to_string(),
        source_id: source_id.to_string(),
        dataset_id: "test_dataset".to_string(),
        revision_time: None,
        publication_time,
        quality_score: 90.0,
        quality_flags: Vec::new(),
    }
}

pub(super) fn test_release_with_bundle(bundle: &ProbabilityBundle) -> ModelReleaseRecord {
    let bundle_path = std::env::temp_dir().join(format!(
        "fc-probability-guard-{}.json",
        Utc::now()
            .timestamp_nanos_opt()
            .unwrap_or_default()
            .unsigned_abs()
    ));
    std::fs::write(&bundle_path, serde_json::to_string_pretty(bundle).unwrap()).unwrap();
    ModelReleaseRecord {
        manifest: ModelReleaseManifest {
            release_id: bundle.bundle_id.clone(),
            market_scope: bundle.market_scope.clone(),
            status: "approved".to_string(),
            probability_mode: bundle.probability_mode.clone(),
            serving_status: "healthy".to_string(),
            bundle_uri: bundle_path.to_string_lossy().to_string(),
            feature_set_version: crate::DEFAULT_FORMAL_FEATURE_SET_VERSION.to_string(),
            label_version: "formal_label_v1_main".to_string(),
            prob_model_version: "prob".to_string(),
            calibration_version: "calib".to_string(),
            posture_policy_version: "posture".to_string(),
            action_playbook_version: "playbook".to_string(),
            point_in_time_mode: "best_effort".to_string(),
            training_range_start: None,
            training_range_end: None,
            calibration_range_start: None,
            calibration_range_end: None,
            evaluation_range_start: None,
            evaluation_range_end: None,
            brier_score: Some(0.1),
            log_loss: Some(0.2),
            ece: Some(0.1),
            note: "test".to_string(),
        },
        created_at: Utc::now(),
        activated_at: None,
        retired_at: None,
    }
}

pub(super) fn synthetic_runtime_scenarios() -> Vec<CrisisScenario> {
    vec![CrisisScenario {
        scenario_id: "synthetic".to_string(),
        family: "systemic_credit_banking_crisis".to_string(),
        training_role: "mandatory".to_string(),
        pre_warning_start: NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
        crisis_start: NaiveDate::from_ymd_opt(2000, 2, 1).unwrap(),
        acute_start: None,
        crisis_end: NaiveDate::from_ymd_opt(2000, 2, 20).unwrap(),
        default_horizon_roles: vec![20, 60],
        protected_window: false,
        protected_action_levels: Vec::new(),
        episode_template_id: ActionEpisodeTemplateId::SystemicCreditBankingCrisis,
        action_episode_overrides: None,
    }]
}

pub(super) fn synthetic_backtest_summary(
    scenario_id: &str,
    name: &str,
    lead_time_days: Option<i64>,
    actionable_lead_time_days: Option<i64>,
    false_positive_count: u32,
) -> BacktestScenarioSummary {
    BacktestScenarioSummary {
        scenario_id: scenario_id.to_string(),
        name: name.to_string(),
        region: "us".to_string(),
        signal_source: BacktestSignalSource::RealHistory,
        crisis_start: NaiveDate::from_ymd_opt(2023, 3, 10).unwrap(),
        crisis_end: NaiveDate::from_ymd_opt(2023, 3, 20).unwrap(),
        first_l2_date: None,
        first_l3_date: None,
        max_level: RiskLevel::Crisis,
        max_score: 72.0,
        lead_time_days,
        actionable_lead_time_days,
        false_positive_count,
        missed: actionable_lead_time_days.is_none(),
        history_start: Some(NaiveDate::from_ymd_opt(2023, 1, 1).unwrap()),
        history_end: Some(NaiveDate::from_ymd_opt(2023, 3, 20).unwrap()),
        history_point_count: 50,
        note: "test".to_string(),
        top_contributors: Vec::new(),
        method_version: "test".to_string(),
    }
}

pub(super) fn synthetic_backtest_summary_with_dates(
    scenario_id: &str,
    name: &str,
    first_l2_date: Option<NaiveDate>,
    first_l3_date: Option<NaiveDate>,
    lead_time_days: Option<i64>,
    actionable_lead_time_days: Option<i64>,
    false_positive_count: u32,
) -> BacktestScenarioSummary {
    let mut summary = synthetic_backtest_summary(
        scenario_id,
        name,
        lead_time_days,
        actionable_lead_time_days,
        false_positive_count,
    );
    summary.first_l2_date = first_l2_date;
    summary.first_l3_date = first_l3_date;
    summary
}

#[allow(clippy::too_many_arguments)]
pub(super) fn synthetic_backtest_summary_with_window(
    scenario_id: &str,
    name: &str,
    crisis_start: NaiveDate,
    crisis_end: NaiveDate,
    first_l2_date: Option<NaiveDate>,
    first_l3_date: Option<NaiveDate>,
    lead_time_days: Option<i64>,
    actionable_lead_time_days: Option<i64>,
    false_positive_count: u32,
) -> BacktestScenarioSummary {
    let mut summary = synthetic_backtest_summary(
        scenario_id,
        name,
        lead_time_days,
        actionable_lead_time_days,
        false_positive_count,
    );
    summary.crisis_start = crisis_start;
    summary.crisis_end = crisis_end;
    summary.first_l2_date = first_l2_date;
    summary.first_l3_date = first_l3_date;
    summary.history_start = Some(first_l2_date.or(first_l3_date).unwrap_or(crisis_start));
    summary.history_end = Some(crisis_end);
    summary
}

pub(super) fn runtime_history_point(
    as_of_date: NaiveDate,
    raw_probability: f64,
    calibrated_probability: f64,
) -> AssessmentHistoryPoint {
    AssessmentHistoryPoint {
        as_of_date,
        overall_score: 50.0,
        p_5d: calibrated_probability,
        p_20d: calibrated_probability,
        p_60d: calibrated_probability,
        raw_p_5d: Some(raw_probability),
        raw_p_20d: Some(raw_probability),
        raw_p_60d: Some(raw_probability),
        posture: DecisionPosture::Normal,
        time_to_risk_bucket: TimeToRiskBucket::Normal,
        external_shock_score: 20.0,
        posture_trigger_codes: Vec::new(),
        posture_blocker_codes: Vec::new(),
        replay_run_id: None,
        feature_snapshot_id: None,
        history_source: None,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn runtime_history_point_with_state(
    as_of_date: NaiveDate,
    overall_score: f64,
    p_5d: f64,
    p_20d: f64,
    p_60d: f64,
    posture: DecisionPosture,
    time_to_risk_bucket: TimeToRiskBucket,
    external_shock_score: f64,
    posture_trigger_codes: &[&str],
) -> AssessmentHistoryPoint {
    AssessmentHistoryPoint {
        as_of_date,
        overall_score,
        p_5d,
        p_20d,
        p_60d,
        raw_p_5d: Some(p_5d),
        raw_p_20d: Some(p_20d),
        raw_p_60d: Some(p_60d),
        posture,
        time_to_risk_bucket,
        external_shock_score,
        posture_trigger_codes: posture_trigger_codes
            .iter()
            .map(|code| (*code).to_string())
            .collect(),
        posture_blocker_codes: Vec::new(),
        replay_run_id: None,
        feature_snapshot_id: None,
        history_source: None,
    }
}

pub(super) fn formal_main_audit_method_wire() -> crate::AuditMethodResponseWire {
    crate::AuditMethodResponseWire {
        method: AssessmentMethodVersions {
            score_method_version: "score_v1".to_string(),
            prob_model_version: "prob_v1".to_string(),
            calibration_version: "calib_v1".to_string(),
            actionability_model_version: None,
            actionability_calibration_version: None,
            feature_set_version: crate::DEFAULT_FORMAL_FEATURE_SET_VERSION.to_string(),
            label_version: "formal_label_v1_main".to_string(),
            posture_policy_version: "posture_v1".to_string(),
            action_playbook_version: "playbook_v1".to_string(),
            fusion_policy_version: None,
            actionability_enabled: false,
            probability_mode: "formal_bundle_v1".to_string(),
            release_status: "active_formal".to_string(),
            release_id: Some("test_release".to_string()),
            point_in_time_mode: "raw_feature_replay".to_string(),
        },
        note: "test".to_string(),
        protected_stress_window_catalog: None,
        runtime_thresholds: Some(RuntimeThresholdDiagnosticsWire {
            prepare_p60d: 0.10,
            hedge_p20d: 0.07,
            defend_p5d: 0.03,
            severe_now_p20d: 0.27,
            elevated_weeks_p60d: 0.20,
            external_prepare_p20d: 0.05,
            carry_prepare_p60d: 0.08,
            downgrade_prepare_p60d: 0.075,
            downgrade_hedge_p20d: 0.053,
            downgrade_defend_p5d: 0.02,
            history_runtime_policy_version: "runtime_history_test".to_string(),
        }),
    }
}

pub(super) fn forward_crisis_row(
    as_of_date: NaiveDate,
    label_20d: u8,
    regime_20d: ProbabilityTrainingRegime,
) -> ProbabilityTrainingRow {
    ProbabilityTrainingRow {
        as_of_date,
        market_scope: "financial_system".to_string(),
        release_id: None,
        probability_mode: Some("formal_bundle_v1".to_string()),
        freshness_status: Some("fresh".to_string()),
        time_to_risk_bucket: Some("test".to_string()),
        split_name: Some("calibration".to_string()),
        features: BTreeMap::new(),
        primary_scenario_id: Some("scenario_a".to_string()),
        scenario_family: Some("systemic_credit_banking_crisis".to_string()),
        scenario_training_role: None,
        days_to_primary_crisis_start: Some(10),
        primary_scenario_supports_5d: true,
        primary_scenario_supports_20d: true,
        primary_scenario_supports_60d: true,
        label_5d: 0,
        label_20d,
        label_60d: 0,
        regime_5d: ProbabilityTrainingRegime::Normal,
        regime_20d,
        regime_60d: ProbabilityTrainingRegime::Normal,
        action_label_5d: 0,
        action_label_20d: 0,
        action_label_60d: 0,
        prepare_episode_label: 0,
        hedge_episode_label: 0,
        defend_episode_label: 0,
        primary_action_level: None,
        action_episode_id: None,
        action_episode_phase: "outside".to_string(),
        protected_action_window: false,
    }
}

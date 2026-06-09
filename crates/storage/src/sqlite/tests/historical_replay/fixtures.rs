use chrono::{DateTime, NaiveDate, Utc};
use fc_domain::{
    HistoricalAssessmentPointRecord, HistoricalReplayRunRecord, ModelReleaseManifest,
    ModelReleaseRecord, ProbabilityDiagnostics, ProbabilityHorizonOverlayDiagnostics,
    ProbabilityOverlayContribution,
};

pub(super) fn model_release(created_at: DateTime<Utc>) -> ModelReleaseRecord {
    ModelReleaseRecord {
        manifest: ModelReleaseManifest {
            release_id: "release-1".to_string(),
            market_scope: "financial_system".to_string(),
            status: "candidate".to_string(),
            probability_mode: "formal_bundle_v1".to_string(),
            serving_status: "shadow".to_string(),
            bundle_uri: "file:///tmp/release.json".to_string(),
            feature_set_version: "feature_formal_v1".to_string(),
            label_version: "formal_label_v1_main".to_string(),
            prob_model_version: "prob_v1".to_string(),
            calibration_version: "calib_v1".to_string(),
            posture_policy_version: "posture_v1".to_string(),
            action_playbook_version: "action_playbook_v1".to_string(),
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
        created_at,
        activated_at: None,
        retired_at: None,
    }
}

pub(super) fn replay_run(created_at: DateTime<Utc>) -> HistoricalReplayRunRecord {
    HistoricalReplayRunRecord {
        replay_run_id: "replay-1".to_string(),
        release_id: Some("release-1".to_string()),
        market_scope: "financial_system".to_string(),
        from_date: NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
        to_date: NaiveDate::from_ymd_opt(2026, 5, 30).unwrap(),
        history_cache_key: "history_cache_v3|release=release-1".to_string(),
        feature_set_version: "feature_formal_v1".to_string(),
        label_version: "formal_label_v1_main".to_string(),
        point_in_time_mode: "best_effort".to_string(),
        runtime_policy_version: "runtime_history_v1".to_string(),
        action_playbook_version: "action_playbook_v1".to_string(),
        protected_window_catalog_id: "scenario_v1_main".to_string(),
        source_watermark: "observations=2026-05-30".to_string(),
        status: "success".to_string(),
        point_count: 1,
        failure_reason: None,
        created_at,
    }
}

pub(super) fn assessment_point(
    created_at: DateTime<Utc>,
    replay_run_id: &str,
) -> HistoricalAssessmentPointRecord {
    HistoricalAssessmentPointRecord {
        replay_run_id: replay_run_id.to_string(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        release_id: Some("release-1".to_string()),
        as_of_date: NaiveDate::from_ymd_opt(2026, 5, 30).unwrap(),
        feature_snapshot_id: Some(
            "financial_system:us:2026-05-30:feature_formal_v1:best_effort".to_string(),
        ),
        point_in_time_mode: "best_effort".to_string(),
        runtime_policy_version: "runtime_history_v1".to_string(),
        action_playbook_version: "action_playbook_v1".to_string(),
        overall_score: 72.4,
        structural_score: 68.1,
        trigger_score: 64.2,
        external_shock_score: 55.8,
        raw_p_5d: 0.08,
        raw_p_20d: 0.19,
        raw_p_60d: 0.27,
        calibrated_p_5d: 0.06,
        calibrated_p_20d: 0.17,
        calibrated_p_60d: 0.24,
        posture: "prepare".to_string(),
        time_to_risk_bucket: "months".to_string(),
        actionability_prepare: 0.61,
        actionability_hedge: 0.28,
        actionability_defend: 0.09,
        probability_diagnostics: ProbabilityDiagnostics {
            horizon_overlays: vec![ProbabilityHorizonOverlayDiagnostics {
                horizon_days: 20,
                raw_probability: 0.19,
                calibrated_probability: 0.17,
                final_probability: 0.21,
                runtime_final_probability: Some(0.23),
                monotonic_lift: 0.02,
                configured_overlay_count: 1,
                base_contributions: Vec::new(),
                contributions: vec![ProbabilityOverlayContribution {
                    family_id: "jpy_carry".to_string(),
                    gate_feature: "us_usdjpy_level".to_string(),
                    gate_value: 138.4,
                    gate: 0.74,
                    blend: 0.25,
                    overlay_probability: 0.33,
                    contribution: 0.04,
                }],
                overlay_audits: Vec::new(),
            }],
        },
        posture_trigger_codes: vec!["prepare_p60d_structural".to_string()],
        posture_blocker_codes: vec!["quality_blocked_hedge".to_string()],
        coverage_score: 0.92,
        freshness_status: "fresh".to_string(),
        generated_at: created_at,
    }
}

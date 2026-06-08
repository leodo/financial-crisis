use chrono::{NaiveDate, TimeZone, Utc};
use fc_domain::{
    load_protected_stress_window_catalog, DecisionPosture, ModelReleaseManifest,
    ModelReleaseRecord, PredictionSnapshotRecord, ProbabilityBundle, TimeToRiskBucket,
};

use super::{
    build_app_data_from_inputs, build_rolling_backtest_audit,
    expected_prediction_snapshot_method_version, is_actionable_warning_point,
    is_actionable_warning_point_with_thresholds, load_user_preferences,
    use_transitional_actionable_bridge, ServingModelContext, FORMAL_MAIN_FEATURE_SET_VERSION,
    FORMAL_MAIN_LABEL_VERSION,
};
use crate::assessment::{runtime_threshold_diagnostics, ProbabilityActionThresholds};
use crate::demo_seed::{indicators, observations};
use crate::history_replay::{
    historical_output_from_prediction_snapshots, should_refresh_full_release_history,
};

fn history_point(
    as_of_date: NaiveDate,
    overall_score: f64,
    posture: DecisionPosture,
    time_to_risk_bucket: TimeToRiskBucket,
    external_shock_score: f64,
) -> fc_domain::AssessmentHistoryPoint {
    fc_domain::AssessmentHistoryPoint {
        as_of_date,
        overall_score,
        p_5d: 0.026,
        p_20d: 0.026,
        p_60d: 0.056,
        raw_p_5d: Some(0.012),
        raw_p_20d: Some(0.028),
        raw_p_60d: Some(0.081),
        posture,
        time_to_risk_bucket,
        external_shock_score,
        posture_trigger_codes: Vec::new(),
        posture_blocker_codes: Vec::new(),
        replay_run_id: None,
        feature_snapshot_id: None,
        history_source: None,
    }
}

fn snapshot(
    as_of_date: NaiveDate,
    release_id: Option<&str>,
    p_20d: f64,
    posture: &str,
    recorded_at_hour: u32,
) -> PredictionSnapshotRecord {
    PredictionSnapshotRecord {
        as_of_date,
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        release_id: release_id.map(str::to_string),
        probability_mode: "heuristic_mvp".to_string(),
        release_status: "degraded".to_string(),
        point_in_time_mode: "best_effort".to_string(),
        overall_score: 42.0,
        external_shock_score: 25.0,
        raw_p_5d: 0.01,
        raw_p_20d: p_20d,
        raw_p_60d: 0.08,
        calibrated_p_5d: 0.01,
        calibrated_p_20d: p_20d,
        calibrated_p_60d: 0.08,
        posture: posture.to_string(),
        time_to_risk_bucket: "weeks".to_string(),
        feature_set_version: "feature_v2".to_string(),
        label_version: "label_v1".to_string(),
        coverage_score: 0.95,
        freshness_status: "fresh".to_string(),
        method_version: "score_v1".to_string(),
        posture_trigger_codes: vec!["prepare_p60d_structural".to_string()],
        posture_blocker_codes: Vec::new(),
        recorded_at: Utc
            .with_ymd_and_hms(2026, 5, 31, recorded_at_hour, 0, 0)
            .single()
            .unwrap(),
    }
}

fn formal_serving_model_context() -> ServingModelContext {
    ServingModelContext {
        release: ModelReleaseRecord {
            manifest: ModelReleaseManifest {
                release_id: "formal-release".to_string(),
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
fn prediction_history_filters_by_release_and_keeps_latest_daily_snapshot() {
    let as_of_date = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();
    let output = historical_output_from_prediction_snapshots(
        vec![
            snapshot(as_of_date, Some("release-a"), 0.12, "normal", 1),
            snapshot(as_of_date, Some("release-a"), 0.27, "hedge", 3),
            snapshot(as_of_date, Some("release-b"), 0.88, "defend", 4),
        ],
        Some("release-a"),
    );

    assert_eq!(output.history_points.len(), 1);
    assert_eq!(output.prediction_snapshots.len(), 1);
    assert_eq!(output.history_points[0].p_20d, 0.27);
    assert_eq!(
        output.history_points[0].posture,
        fc_domain::DecisionPosture::Hedge
    );
    assert_eq!(
        output.history_points[0].posture_trigger_codes,
        vec!["prepare_p60d_structural".to_string()]
    );
    assert_eq!(
        output.history_points[0].history_source.as_deref(),
        Some("transitional_snapshot_bridge")
    );
}

#[test]
fn build_app_data_preserves_existing_history_metadata_when_refreshing_same_day_point() {
    let as_of_date = NaiveDate::from_ymd_opt(2026, 5, 30).unwrap();
    let history = vec![fc_domain::AssessmentHistoryPoint {
        as_of_date,
        overall_score: 42.0,
        p_5d: 0.01,
        p_20d: 0.02,
        p_60d: 0.03,
        raw_p_5d: Some(0.01),
        raw_p_20d: Some(0.02),
        raw_p_60d: Some(0.03),
        posture: DecisionPosture::Normal,
        time_to_risk_bucket: TimeToRiskBucket::Normal,
        external_shock_score: 12.0,
        posture_trigger_codes: vec!["cached".to_string()],
        posture_blocker_codes: Vec::new(),
        replay_run_id: Some("replay-run".to_string()),
        feature_snapshot_id: Some("feature-snapshot".to_string()),
        history_source: Some("raw_pit_feature_replay".to_string()),
    }];

    let built = build_app_data_from_inputs(
        fc_domain::DataMode::Sqlite,
        indicators(),
        observations(as_of_date),
        Some(Vec::new()),
        Some(formal_serving_model_context()),
        as_of_date,
        history,
        load_user_preferences(),
    );

    let last = built
        .app_data
        .assessment_history
        .last()
        .expect("latest history point");
    assert_eq!(last.replay_run_id.as_deref(), Some("replay-run"));
    assert_eq!(
        last.feature_snapshot_id.as_deref(),
        Some("feature-snapshot")
    );
    assert_eq!(
        last.history_source.as_deref(),
        Some("raw_pit_feature_replay")
    );
}

#[test]
fn actionable_warning_point_accepts_prepare_bridge_for_persisted_snapshots() {
    let point = history_point(
        NaiveDate::from_ymd_opt(2008, 7, 25).unwrap(),
        58.0,
        DecisionPosture::Prepare,
        TimeToRiskBucket::Normal,
        46.0,
    );

    assert!(is_actionable_warning_point(&point, true));
}

#[test]
fn actionable_warning_point_rejects_weak_prepare_bridge() {
    let point = history_point(
        NaiveDate::from_ymd_opt(2008, 7, 25).unwrap(),
        57.9,
        DecisionPosture::Prepare,
        TimeToRiskBucket::Normal,
        45.9,
    );

    assert!(!is_actionable_warning_point(&point, true));
}

#[test]
fn actionable_warning_point_disables_prepare_bridge_for_formal_main() {
    let point = history_point(
        NaiveDate::from_ymd_opt(2008, 7, 25).unwrap(),
        58.0,
        DecisionPosture::Prepare,
        TimeToRiskBucket::Normal,
        46.0,
    );

    assert!(!is_actionable_warning_point(&point, false));
}

#[test]
fn actionable_warning_point_accepts_strong_prepare_clause_for_formal_main() {
    let point = fc_domain::AssessmentHistoryPoint {
        as_of_date: NaiveDate::from_ymd_opt(2007, 3, 1).unwrap(),
        overall_score: 53.4,
        p_5d: 0.03,
        p_20d: 0.70,
        p_60d: 0.73,
        raw_p_5d: Some(0.02),
        raw_p_20d: Some(0.68),
        raw_p_60d: Some(0.70),
        posture: DecisionPosture::Prepare,
        time_to_risk_bucket: TimeToRiskBucket::Months,
        external_shock_score: 38.5,
        posture_trigger_codes: vec!["prepare_p60d_structural".to_string()],
        posture_blocker_codes: Vec::new(),
        replay_run_id: None,
        feature_snapshot_id: None,
        history_source: None,
    };

    assert!(is_actionable_warning_point(&point, false));
}

#[test]
fn actionable_warning_point_accepts_probability_plateau_clause_for_formal_main() {
    let point = fc_domain::AssessmentHistoryPoint {
        as_of_date: NaiveDate::from_ymd_opt(1987, 8, 24).unwrap(),
        overall_score: 44.4,
        p_5d: 0.03,
        p_20d: 0.905,
        p_60d: 0.892,
        raw_p_5d: Some(0.02),
        raw_p_20d: Some(0.741),
        raw_p_60d: Some(0.892),
        posture: DecisionPosture::Prepare,
        time_to_risk_bucket: TimeToRiskBucket::Months,
        external_shock_score: 48.2,
        posture_trigger_codes: vec!["prepare_probability_plateau".to_string()],
        posture_blocker_codes: Vec::new(),
        replay_run_id: None,
        feature_snapshot_id: None,
        history_source: None,
    };

    assert!(is_actionable_warning_point(&point, false));
}

#[test]
fn actionable_warning_point_rejects_weak_prepare_clause_for_formal_main() {
    let point = fc_domain::AssessmentHistoryPoint {
        as_of_date: NaiveDate::from_ymd_opt(2007, 3, 1).unwrap(),
        overall_score: 52.9,
        p_5d: 0.03,
        p_20d: 0.70,
        p_60d: 0.73,
        raw_p_5d: Some(0.02),
        raw_p_20d: Some(0.68),
        raw_p_60d: Some(0.70),
        posture: DecisionPosture::Prepare,
        time_to_risk_bucket: TimeToRiskBucket::Months,
        external_shock_score: 38.5,
        posture_trigger_codes: vec!["prepare_p60d_structural".to_string()],
        posture_blocker_codes: Vec::new(),
        replay_run_id: None,
        feature_snapshot_id: None,
        history_source: None,
    };

    assert!(!is_actionable_warning_point(&point, false));
}

#[test]
fn actionable_warning_point_rejects_weak_probability_plateau_clause_for_formal_main() {
    let point = fc_domain::AssessmentHistoryPoint {
        as_of_date: NaiveDate::from_ymd_opt(1987, 8, 24).unwrap(),
        overall_score: 41.9,
        p_5d: 0.03,
        p_20d: 0.905,
        p_60d: 0.892,
        raw_p_5d: Some(0.02),
        raw_p_20d: Some(0.741),
        raw_p_60d: Some(0.892),
        posture: DecisionPosture::Prepare,
        time_to_risk_bucket: TimeToRiskBucket::Months,
        external_shock_score: 31.5,
        posture_trigger_codes: vec!["prepare_probability_plateau".to_string()],
        posture_blocker_codes: Vec::new(),
        replay_run_id: None,
        feature_snapshot_id: None,
        history_source: None,
    };

    assert!(!is_actionable_warning_point(&point, false));
}

#[test]
fn actionable_warning_point_accepts_relaxed_probability_plateau_clause_with_runtime_thresholds() {
    let point = fc_domain::AssessmentHistoryPoint {
        as_of_date: NaiveDate::from_ymd_opt(1998, 5, 28).unwrap(),
        overall_score: 44.4,
        p_5d: 0.03,
        p_20d: 0.50,
        p_60d: 0.67,
        raw_p_5d: Some(0.02),
        raw_p_20d: Some(0.48),
        raw_p_60d: Some(0.65),
        posture: DecisionPosture::Prepare,
        time_to_risk_bucket: TimeToRiskBucket::Months,
        external_shock_score: 40.7,
        posture_trigger_codes: vec!["prepare_probability_plateau".to_string()],
        posture_blocker_codes: Vec::new(),
        replay_run_id: None,
        feature_snapshot_id: None,
        history_source: None,
    };

    assert!(!is_actionable_warning_point(&point, false));
    assert!(is_actionable_warning_point_with_thresholds(
        &point,
        false,
        ProbabilityActionThresholds {
            prepare_p60d: 0.568,
            hedge_p20d: 0.282,
            defend_p5d: 0.05,
        }
    ));
}

#[test]
fn actionable_warning_point_accepts_prepare_weeks_plateau_hysteresis_clause() {
    let point = fc_domain::AssessmentHistoryPoint {
        as_of_date: NaiveDate::from_ymd_opt(2022, 12, 9).unwrap(),
        overall_score: 52.1,
        p_5d: 0.02,
        p_20d: 0.614,
        p_60d: 0.93,
        raw_p_5d: Some(0.02),
        raw_p_20d: Some(0.59),
        raw_p_60d: Some(0.93),
        posture: DecisionPosture::Prepare,
        time_to_risk_bucket: TimeToRiskBucket::Weeks,
        external_shock_score: 33.9,
        posture_trigger_codes: vec![
            "prepare_probability_plateau".to_string(),
            "prepare_history_hysteresis".to_string(),
        ],
        posture_blocker_codes: Vec::new(),
        replay_run_id: None,
        feature_snapshot_id: None,
        history_source: None,
    };

    assert!(is_actionable_warning_point(&point, false));
    assert!(is_actionable_warning_point_with_thresholds(
        &point,
        false,
        ProbabilityActionThresholds {
            prepare_p60d: 0.568,
            hedge_p20d: 0.282,
            defend_p5d: 0.05,
        }
    ));
}

#[test]
fn actionable_warning_point_accepts_weeks_trigger_dominant_clause() {
    let point = fc_domain::AssessmentHistoryPoint {
        as_of_date: NaiveDate::from_ymd_opt(2000, 1, 28).unwrap(),
        overall_score: 53.1,
        p_5d: 0.01,
        p_20d: 0.564,
        p_60d: 0.315,
        raw_p_5d: Some(0.01),
        raw_p_20d: Some(0.55),
        raw_p_60d: Some(0.31),
        posture: DecisionPosture::Normal,
        time_to_risk_bucket: TimeToRiskBucket::Weeks,
        external_shock_score: 36.1,
        posture_trigger_codes: Vec::new(),
        posture_blocker_codes: Vec::new(),
        replay_run_id: None,
        feature_snapshot_id: None,
        history_source: None,
    };

    assert!(!is_actionable_warning_point(&point, false));
    assert!(is_actionable_warning_point_with_thresholds(
        &point,
        false,
        ProbabilityActionThresholds {
            prepare_p60d: 0.568,
            hedge_p20d: 0.282,
            defend_p5d: 0.05,
        }
    ));
}

#[test]
fn actionable_warning_point_rejects_weak_weeks_trigger_dominant_clause() {
    let point = fc_domain::AssessmentHistoryPoint {
        as_of_date: NaiveDate::from_ymd_opt(2000, 2, 24).unwrap(),
        overall_score: 54.5,
        p_5d: 0.01,
        p_20d: 0.325,
        p_60d: 0.245,
        raw_p_5d: Some(0.01),
        raw_p_20d: Some(0.32),
        raw_p_60d: Some(0.24),
        posture: DecisionPosture::Normal,
        time_to_risk_bucket: TimeToRiskBucket::Weeks,
        external_shock_score: 38.5,
        posture_trigger_codes: Vec::new(),
        posture_blocker_codes: Vec::new(),
        replay_run_id: None,
        feature_snapshot_id: None,
        history_source: None,
    };

    assert!(!is_actionable_warning_point(&point, false));
    assert!(!is_actionable_warning_point_with_thresholds(
        &point,
        false,
        ProbabilityActionThresholds {
            prepare_p60d: 0.568,
            hedge_p20d: 0.282,
            defend_p5d: 0.05,
        }
    ));
}

#[test]
fn actionable_warning_point_accepts_history_hysteresis_months_structural_carry_clause() {
    let point = fc_domain::AssessmentHistoryPoint {
        as_of_date: NaiveDate::from_ymd_opt(1990, 7, 27).unwrap(),
        overall_score: 44.6,
        p_5d: 0.02,
        p_20d: 0.322,
        p_60d: 0.85,
        raw_p_5d: Some(0.02),
        raw_p_20d: Some(0.30),
        raw_p_60d: Some(0.84),
        posture: DecisionPosture::Prepare,
        time_to_risk_bucket: TimeToRiskBucket::Months,
        external_shock_score: 30.2,
        posture_trigger_codes: vec!["prepare_history_hysteresis".to_string()],
        posture_blocker_codes: Vec::new(),
        replay_run_id: None,
        feature_snapshot_id: None,
        history_source: None,
    };

    assert!(!is_actionable_warning_point(&point, false));
    assert!(is_actionable_warning_point_with_thresholds(
        &point,
        false,
        ProbabilityActionThresholds {
            prepare_p60d: 0.568,
            hedge_p20d: 0.282,
            defend_p5d: 0.05,
        }
    ));
}

#[test]
fn actionable_warning_point_accepts_formal_main_relaxed_strict_p60d_mapping() {
    let point = fc_domain::AssessmentHistoryPoint {
        as_of_date: NaiveDate::from_ymd_opt(2007, 3, 1).unwrap(),
        overall_score: 63.0,
        p_5d: 0.03,
        p_20d: 0.19,
        p_60d: 0.26,
        raw_p_5d: Some(0.02),
        raw_p_20d: Some(0.18),
        raw_p_60d: Some(0.24),
        posture: DecisionPosture::Prepare,
        time_to_risk_bucket: TimeToRiskBucket::Months,
        external_shock_score: 49.0,
        posture_trigger_codes: vec!["prepare_p60d_structural".to_string()],
        posture_blocker_codes: Vec::new(),
        replay_run_id: None,
        feature_snapshot_id: None,
        history_source: None,
    };

    assert!(!is_actionable_warning_point(&point, false));
    assert!(is_actionable_warning_point_with_thresholds(
        &point,
        false,
        ProbabilityActionThresholds {
            prepare_p60d: 0.12,
            hedge_p20d: 0.06,
            defend_p5d: 0.05,
        }
    ));
}

#[test]
fn actionable_warning_point_accepts_formal_main_relaxed_strict_p20d_mapping() {
    let point = fc_domain::AssessmentHistoryPoint {
        as_of_date: NaiveDate::from_ymd_opt(2007, 3, 1).unwrap(),
        overall_score: 57.0,
        p_5d: 0.02,
        p_20d: 0.13,
        p_60d: 0.48,
        raw_p_5d: Some(0.02),
        raw_p_20d: Some(0.12),
        raw_p_60d: Some(0.46),
        posture: DecisionPosture::Prepare,
        time_to_risk_bucket: TimeToRiskBucket::Months,
        external_shock_score: 45.0,
        posture_trigger_codes: vec!["prepare_p60d_structural".to_string()],
        posture_blocker_codes: Vec::new(),
        replay_run_id: None,
        feature_snapshot_id: None,
        history_source: None,
    };

    assert!(!is_actionable_warning_point(&point, false));
    assert!(is_actionable_warning_point_with_thresholds(
        &point,
        false,
        ProbabilityActionThresholds {
            prepare_p60d: 0.12,
            hedge_p20d: 0.06,
            defend_p5d: 0.05,
        }
    ));
}

#[test]
fn rolling_audit_counts_catalog_protected_windows_as_stress() {
    let stress_windows = load_protected_stress_window_catalog();
    let history = vec![history_point(
        NaiveDate::from_ymd_opt(2015, 9, 1).unwrap(),
        60.0,
        DecisionPosture::Prepare,
        TimeToRiskBucket::Months,
        46.0,
    )];

    let audit = build_rolling_backtest_audit(&history, &stress_windows.windows, true);

    assert_eq!(audit.actionable_signal_count, 1);
    assert_eq!(audit.stress_window_signal_count, 1);
    assert_eq!(audit.pre_crisis_signal_count, 0);
    assert_eq!(audit.false_positive_signal_count, 0);
    assert_eq!(audit.classified_episodes.len(), 1);
    assert_eq!(audit.classified_episodes[0].classification, "stress_window");
}

#[test]
fn rolling_audit_counts_prepare_signal_within_sixty_days_as_pre_crisis() {
    let history = vec![fc_domain::AssessmentHistoryPoint {
        as_of_date: NaiveDate::from_ymd_opt(2000, 1, 31).unwrap(),
        overall_score: 63.0,
        p_5d: 0.03,
        p_20d: 0.19,
        p_60d: 0.48,
        raw_p_5d: Some(0.02),
        raw_p_20d: Some(0.18),
        raw_p_60d: Some(0.45),
        posture: DecisionPosture::Prepare,
        time_to_risk_bucket: TimeToRiskBucket::Months,
        external_shock_score: 49.0,
        posture_trigger_codes: vec!["prepare_p60d_structural".to_string()],
        posture_blocker_codes: Vec::new(),
        replay_run_id: None,
        feature_snapshot_id: None,
        history_source: None,
    }];

    let audit = build_rolling_backtest_audit(&history, &[], false);

    assert_eq!(audit.actionable_signal_count, 1);
    assert_eq!(audit.pre_crisis_signal_count, 1);
    assert_eq!(audit.false_positive_signal_count, 0);
}

#[test]
fn bundle_backed_history_refreshes_when_cached_method_version_is_stale() {
    let serving_model = formal_serving_model_context();
    let mut persisted = vec![snapshot(
        NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
        Some("formal-release"),
        0.27,
        "hedge",
        3,
    )];
    persisted[0].method_version = "legacy-cache".to_string();

    assert!(should_refresh_full_release_history(
        Some(&serving_model),
        &persisted,
        false,
    ));
}

#[test]
fn bundle_backed_history_keeps_cache_when_method_version_matches() {
    let serving_model = formal_serving_model_context();
    let expected_method_version = expected_prediction_snapshot_method_version(Some(&serving_model));
    let mut persisted = vec![snapshot(
        NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
        Some("formal-release"),
        0.27,
        "hedge",
        3,
    )];
    persisted[0].method_version = expected_method_version;

    assert!(!should_refresh_full_release_history(
        Some(&serving_model),
        &persisted,
        false,
    ));
}

#[test]
fn formal_main_disables_transitional_actionable_bridge() {
    let serving_model = formal_serving_model_context();

    assert!(!use_transitional_actionable_bridge(Some(&serving_model)));
    assert!(use_transitional_actionable_bridge(None));
}

#[test]
fn formal_main_method_version_carries_runtime_policy_cache_key() {
    let serving_model = formal_serving_model_context();
    let method_version = expected_prediction_snapshot_method_version(Some(&serving_model));

    assert!(method_version.contains("runtime_policy="));
    assert!(method_version.contains("class=formal_main"));
}

#[test]
fn formal_main_detection_accepts_versioned_feature_set_variants() {
    let mut serving_model = formal_serving_model_context();
    serving_model.release.manifest.feature_set_version =
        "feature_formal_v1_main_20260607_posturefix".to_string();

    let thresholds = runtime_threshold_diagnostics(Some(&serving_model));
    let method_version = expected_prediction_snapshot_method_version(Some(&serving_model));

    assert_eq!(thresholds.prepare_p60d, 0.12);
    assert_eq!(thresholds.hedge_p20d, 0.07);
    assert_eq!(thresholds.defend_p5d, 0.05);
    assert!(!use_transitional_actionable_bridge(Some(&serving_model)));
    assert!(method_version.contains("class=formal_main"));
}

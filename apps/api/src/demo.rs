use std::env;

use chrono::NaiveDate;
use fc_domain::{
    load_protected_stress_window_catalog, AlertEvent, DataMode, Indicator, Observation,
    PredictionSnapshotRecord, UserRiskPreferences, UserRiskProfile,
};
use fc_scoring::ScoringEngine;

use crate::assessment::{
    build_assessment_snapshot, build_backtest_summary, runtime_threshold_diagnostics,
    ServingModelContext,
};
#[cfg(test)]
use crate::backtest::is_actionable_warning_point;
use crate::backtest::{
    build_backtest_timeline, build_backtests, build_rolling_backtest_audit,
    use_transitional_actionable_bridge,
};
use crate::demo_seed::{
    build_alerts, indicators, observations, select_recent_alerts_for_date, sources_demo,
    sources_runtime,
};
use crate::history_builder::{build_assessment_history, HistoryQueryWindow};
#[cfg(test)]
pub(crate) use crate::history_replay::expected_prediction_snapshot_method_version;
pub(crate) use crate::history_replay::{
    assessment_history_point_from_assessment, prediction_snapshot_from_assessment,
};
use crate::AppData;

pub(crate) const FORMAL_MAIN_FEATURE_SET_VERSION: &str = "feature_formal_v1_main_20260531";
pub(crate) const FORMAL_MAIN_LABEL_VERSION: &str = "formal_label_v1_main";

#[derive(Debug)]
pub(crate) struct BuiltAppData {
    pub(crate) app_data: AppData,
    pub(crate) prediction_snapshots: Vec<PredictionSnapshotRecord>,
}

pub fn build_demo_data(_max_history_points: usize) -> AppData {
    let as_of_date = NaiveDate::from_ymd_opt(2026, 5, 30).expect("valid date");
    let indicators = indicators();
    let observations = observations(as_of_date);
    let user_preferences = load_user_preferences();
    let historical = build_assessment_history(
        DataMode::Demo,
        &ScoringEngine::default(),
        &indicators,
        &observations,
        None,
        None,
        &user_preferences,
        HistoryQueryWindow {
            from: None,
            to: None,
            limit: None,
        },
    );
    build_app_data_from_inputs(
        DataMode::Demo,
        indicators,
        observations,
        None,
        None,
        as_of_date,
        historical.history_points,
        user_preferences,
    )
    .app_data
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_app_data_from_inputs(
    data_mode: DataMode,
    indicators: Vec<Indicator>,
    observations: Vec<Observation>,
    stored_alerts: Option<Vec<AlertEvent>>,
    serving_model: Option<ServingModelContext>,
    as_of_date: NaiveDate,
    mut assessment_history: Vec<fc_domain::AssessmentHistoryPoint>,
    user_preferences: UserRiskPreferences,
) -> BuiltAppData {
    let use_transitional_bridge = use_transitional_actionable_bridge(serving_model.as_ref());
    let scoring = ScoringEngine::default();
    let protected_stress_window_catalog = load_protected_stress_window_catalog();
    let threshold_diagnostics = runtime_threshold_diagnostics(serving_model.as_ref());
    let output = scoring.score(
        &indicators,
        &observations,
        as_of_date,
        "us",
        "financial_system",
    );
    let backtests = build_backtests(
        &output.snapshot,
        &assessment_history,
        use_transitional_bridge,
    );
    let rolling_audit = build_rolling_backtest_audit(
        &assessment_history,
        &protected_stress_window_catalog.windows,
        use_transitional_bridge,
    );
    let alerts = stored_alerts
        .map(|alerts| select_recent_alerts_for_date(&alerts, as_of_date))
        .unwrap_or_else(|| build_alerts(&output.snapshot));
    let backtest_summary = build_backtest_summary(&backtests, Some(&rolling_audit));
    let (assessment, posture_guidance, probability_trace) = build_assessment_snapshot(
        data_mode,
        &output.snapshot,
        &output.indicator_risks,
        &observations,
        &alerts,
        &backtests,
        Some(&rolling_audit),
        serving_model.as_ref(),
        &user_preferences,
    );
    let assessment = fc_domain::AssessmentSnapshot {
        backtest_summary,
        ..assessment
    };
    let current_history_point = assessment_history_point_from_assessment(
        &assessment,
        &posture_guidance,
        &probability_trace,
    );
    match assessment_history.last_mut() {
        Some(last) if last.as_of_date == current_history_point.as_of_date => {
            *last = current_history_point;
        }
        _ => assessment_history.push(current_history_point),
    }
    let backtest_timeline = build_backtest_timeline(&assessment_history, use_transitional_bridge);
    let current_prediction_snapshot = prediction_snapshot_from_assessment(
        &assessment,
        &posture_guidance,
        &probability_trace,
        serving_model.as_ref(),
    );
    BuiltAppData {
        app_data: AppData {
            data_mode,
            user_preferences,
            overview: output.snapshot,
            indicators: output.indicator_risks,
            alerts,
            sources: if matches!(data_mode, DataMode::Demo) {
                sources_demo()
            } else {
                sources_runtime(&observations, as_of_date)
            },
            backtests,
            backtest_timeline,
            assessment,
            assessment_history,
            posture_guidance,
            protected_stress_window_catalog,
            runtime_thresholds: threshold_diagnostics,
        },
        prediction_snapshots: vec![current_prediction_snapshot],
    }
}

pub(crate) fn load_user_preferences() -> UserRiskPreferences {
    let profile = match env::var("FC_USER_RISK_PROFILE")
        .unwrap_or_else(|_| "neutral".to_string())
        .to_lowercase()
        .as_str()
    {
        "conservative" => UserRiskProfile::Conservative,
        "aggressive" => UserRiskProfile::Aggressive,
        _ => UserRiskProfile::Neutral,
    };
    let cash_floor_pct = env::var("FC_USER_CASH_FLOOR_PCT")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(15.0);
    let max_equity_cap_pct = env::var("FC_USER_MAX_EQUITY_CAP_PCT")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(70.0);
    let max_leverage_pct = env::var("FC_USER_MAX_LEVERAGE_PCT")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(100.0);
    let option_overlay_preference_pct = env::var("FC_USER_OPTION_OVERLAY_PCT")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(5.0);
    let allow_aggressive_reentry = env::var("FC_USER_ALLOW_AGGRESSIVE_REENTRY")
        .ok()
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "True"))
        .unwrap_or(false);

    let note = format!(
        "profile={}, cash_floor={}%, max_equity={}%, max_leverage={}%, option_overlay={}%",
        match profile {
            UserRiskProfile::Conservative => "conservative",
            UserRiskProfile::Neutral => "neutral",
            UserRiskProfile::Aggressive => "aggressive",
        },
        cash_floor_pct,
        max_equity_cap_pct,
        max_leverage_pct,
        option_overlay_preference_pct
    );

    UserRiskPreferences {
        profile,
        cash_floor_pct,
        max_equity_cap_pct,
        max_leverage_pct,
        option_overlay_preference_pct,
        allow_aggressive_reentry,
        note,
    }
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, TimeZone, Utc};
    use fc_domain::{
        load_protected_stress_window_catalog, DecisionPosture, ModelReleaseManifest,
        ModelReleaseRecord, PredictionSnapshotRecord, ProbabilityBundle, TimeToRiskBucket,
    };

    use super::{
        build_rolling_backtest_audit, expected_prediction_snapshot_method_version,
        is_actionable_warning_point, use_transitional_actionable_bridge, ServingModelContext,
    };
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
                    feature_set_version: super::FORMAL_MAIN_FEATURE_SET_VERSION.to_string(),
                    label_version: super::FORMAL_MAIN_LABEL_VERSION.to_string(),
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
        };

        assert!(!is_actionable_warning_point(&point, false));
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
        let expected_method_version =
            expected_prediction_snapshot_method_version(Some(&serving_model));
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
}

use chrono::{NaiveDate, Utc};
use fc_domain::{
    AlertEvent, AlertStatus, AlertType, FeatureSnapshotRecord, FormalDatasetManifest,
    FormalDatasetRecord, FormalDatasetRowRecord, HistoricalAssessmentPointRecord,
    HistoricalReplayRunRecord, ModelReleaseManifest, ModelReleaseRecord, PredictionSnapshotRecord,
    ProbabilityDiagnostics, ProbabilityHorizonOverlayDiagnostics, ProbabilityOverlayContribution,
    RiskContributor, RiskDimension, RiskLevel,
};
use uuid::Uuid;

use crate::SqliteStore;

#[tokio::test]
async fn sqlite_store_round_trips_seeded_observations() {
    let store = SqliteStore::connect_url("sqlite::memory:").await.unwrap();
    store.migrate().await.unwrap();
    store.seed_fred_metadata().await.unwrap();

    let indicators = store.load_indicators().await.unwrap();
    assert!(indicators.len() >= 10);

    let indicator = indicators
        .iter()
        .find(|indicator| indicator.indicator_id == "us_market_vix_close")
        .unwrap()
        .clone();
    let observation = fc_domain::Observation {
        indicator_id: indicator.indicator_id,
        entity_id: "us".to_string(),
        as_of_date: NaiveDate::from_ymd_opt(2020, 3, 16).unwrap(),
        period_start: Some(NaiveDate::from_ymd_opt(2020, 3, 16).unwrap()),
        period_end: Some(NaiveDate::from_ymd_opt(2020, 3, 16).unwrap()),
        frequency: indicator.frequency,
        value: 82.69,
        unit: indicator.unit,
        source_id: "fred".to_string(),
        dataset_id: super::FRED_DATASET_ID.to_string(),
        revision_time: None,
        publication_time: None,
        quality_score: 95.0,
        quality_flags: Vec::new(),
    };
    store.insert_observations(&[observation]).await.unwrap();
    let observations = store
        .load_observations("us", NaiveDate::from_ymd_opt(2020, 3, 17).unwrap())
        .await
        .unwrap();

    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].value, 82.69);
}

#[tokio::test]
async fn sqlite_store_round_trips_alerts() {
    let store = SqliteStore::connect_url("sqlite::memory:").await.unwrap();
    store.migrate().await.unwrap();

    let alert = AlertEvent {
        alert_id: Uuid::new_v4(),
        event_type: AlertType::RiskStress,
        scope: "sec_edgar_daily".to_string(),
        entity_id: "us".to_string(),
        dimension: Some(RiskDimension::EventsSentiment),
        level: RiskLevel::Stress,
        status: AlertStatus::Open,
        triggered_at: Utc::now(),
        triggered_as_of_date: NaiveDate::from_ymd_opt(2026, 5, 30).unwrap(),
        resolved_at: None,
        score: 61.0,
        previous_score: Some(28.0),
        trigger_reason: "SEC filing stress cluster".to_string(),
        top_contributors: vec![RiskContributor {
            indicator_id: "us_event_official_filing_severity".to_string(),
            display_name: "SEC 官方公告严重度".to_string(),
            dimension: RiskDimension::EventsSentiment,
            score: 61.0,
            contribution: 61.0,
            explanation: "bank filing spike".to_string(),
        }],
        related_indicators: vec![
            "us_event_bank_8k_count".to_string(),
            "us_event_official_filing_severity".to_string(),
        ],
        method_version: "sec_rules_v1".to_string(),
    };

    store
        .replace_alerts_for_scope(
            "sec_edgar_daily",
            NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 5, 31).unwrap(),
            &[alert.clone()],
        )
        .await
        .unwrap();

    let alerts = store
        .load_alerts_recent(
            NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 5, 31).unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].alert_id, alert.alert_id);
    assert_eq!(alerts[0].related_indicators.len(), 2);
    assert_eq!(
        alerts[0].top_contributors[0].dimension,
        RiskDimension::EventsSentiment
    );
}

#[tokio::test]
async fn sqlite_store_round_trips_model_releases_and_active_pointer() {
    let store = SqliteStore::connect_url("sqlite::memory:").await.unwrap();
    store.migrate().await.unwrap();
    let created_at = Utc::now();
    let release = ModelReleaseRecord {
        manifest: ModelReleaseManifest {
            release_id: "heuristic_bootstrap_20260531".to_string(),
            market_scope: "financial_system".to_string(),
            status: "candidate".to_string(),
            probability_mode: "heuristic_mvp".to_string(),
            serving_status: "degraded".to_string(),
            bundle_uri: "config/model-releases/us-heuristic-bootstrap.json".to_string(),
            feature_set_version: "feature_v2_20260531".to_string(),
            label_version: "label_v1_20260530".to_string(),
            prob_model_version: "prob_v1_20260531".to_string(),
            calibration_version: "calib_v1_20260531".to_string(),
            posture_policy_version: "posture_v1_20260530".to_string(),
            action_playbook_version: "action_playbook_v1_20260531".to_string(),
            point_in_time_mode: "best_effort".to_string(),
            training_range_start: Some(NaiveDate::from_ymd_opt(2007, 1, 1).unwrap()),
            training_range_end: Some(NaiveDate::from_ymd_opt(2021, 12, 31).unwrap()),
            calibration_range_start: Some(NaiveDate::from_ymd_opt(2022, 1, 1).unwrap()),
            calibration_range_end: Some(NaiveDate::from_ymd_opt(2023, 12, 31).unwrap()),
            evaluation_range_start: Some(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
            evaluation_range_end: Some(NaiveDate::from_ymd_opt(2025, 12, 31).unwrap()),
            brier_score: Some(0.082),
            log_loss: Some(0.241),
            ece: Some(0.031),
            note: "bootstrap heuristic release".to_string(),
        },
        created_at,
        activated_at: None,
        retired_at: None,
    };
    store.upsert_model_release(&release).await.unwrap();

    let releases = store
        .list_model_releases(Some("financial_system"))
        .await
        .unwrap();
    assert_eq!(releases.len(), 1);
    assert_eq!(releases[0].manifest.release_id, release.manifest.release_id);

    let active = store
        .activate_model_release("financial_system", &release.manifest.release_id, "test")
        .await
        .unwrap();
    assert_eq!(active.manifest.status, "active");

    let active_pointer = store
        .load_active_model_pointer("financial_system")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(active_pointer.release_id, release.manifest.release_id);

    let loaded = store
        .load_active_model_release("financial_system")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded.manifest.bundle_uri, release.manifest.bundle_uri);
}

#[tokio::test]
async fn sqlite_store_round_trips_prediction_snapshots() {
    let store = SqliteStore::connect_url("sqlite::memory:").await.unwrap();
    store.migrate().await.unwrap();
    let recorded_at = Utc::now();
    let snapshot = PredictionSnapshotRecord {
        as_of_date: NaiveDate::from_ymd_opt(2026, 5, 31).unwrap(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        release_id: Some("us_heuristic_bootstrap_20260531".to_string()),
        probability_mode: "heuristic_mvp".to_string(),
        release_status: "degraded".to_string(),
        point_in_time_mode: "best_effort".to_string(),
        overall_score: 67.4,
        external_shock_score: 58.1,
        raw_p_5d: 0.18,
        raw_p_20d: 0.34,
        raw_p_60d: 0.41,
        calibrated_p_5d: 0.18,
        calibrated_p_20d: 0.34,
        calibrated_p_60d: 0.41,
        posture: "prepare".to_string(),
        time_to_risk_bucket: "weeks".to_string(),
        feature_set_version: "feature_v2_20260531".to_string(),
        label_version: "label_v1_20260530".to_string(),
        coverage_score: 0.87,
        freshness_status: "fresh".to_string(),
        method_version: "scoring_v2_20260531".to_string(),
        posture_trigger_codes: vec!["prepare_p60d_structural".to_string()],
        posture_blocker_codes: vec!["quality_blocked_hedge".to_string()],
        recorded_at,
    };

    store
        .upsert_prediction_snapshots(std::slice::from_ref(&snapshot))
        .await
        .unwrap();

    let rows = store
        .list_prediction_snapshots(
            Some("financial_system"),
            Some("us_heuristic_bootstrap_20260531"),
            Some(NaiveDate::from_ymd_opt(2026, 5, 1).unwrap()),
            Some(NaiveDate::from_ymd_opt(2026, 5, 31).unwrap()),
            Some(10),
        )
        .await
        .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].release_id.as_deref(),
        Some("us_heuristic_bootstrap_20260531")
    );
    assert_eq!(rows[0].time_to_risk_bucket, "weeks");
    assert_eq!(rows[0].freshness_status, "fresh");
    assert_eq!(
        rows[0].posture_trigger_codes,
        vec!["prepare_p60d_structural".to_string()]
    );
    assert_eq!(
        rows[0].posture_blocker_codes,
        vec!["quality_blocked_hedge".to_string()]
    );
}

#[tokio::test]
async fn sqlite_store_round_trips_feature_snapshots_and_formal_datasets() {
    let store = SqliteStore::connect_url("sqlite::memory:").await.unwrap();
    store.migrate().await.unwrap();
    let created_at = Utc::now();

    let snapshot = FeatureSnapshotRecord {
        as_of_date: NaiveDate::from_ymd_opt(2026, 5, 30).unwrap(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        feature_set_version: "feature_formal_v1".to_string(),
        point_in_time_mode: "best_effort".to_string(),
        visibility_status: "best_effort".to_string(),
        latest_visible_at: Some(created_at),
        coverage_score: 0.91,
        core_feature_coverage: 0.94,
        trigger_feature_coverage: 0.88,
        external_feature_coverage: 0.81,
        feature_count: 4,
        features: [
            ("us_vix_level".to_string(), 22.4),
            ("us_curve_10y2y_level".to_string(), -0.42),
            ("structural_score".to_string(), 0.61),
            ("trigger_score".to_string(), 0.64),
        ]
        .into_iter()
        .collect(),
        created_at,
    };

    store
        .upsert_feature_snapshots(std::slice::from_ref(&snapshot))
        .await
        .unwrap();

    let snapshots = store
        .list_feature_snapshots(
            Some("financial_system"),
            Some("feature_formal_v1"),
            Some(NaiveDate::from_ymd_opt(2026, 5, 1).unwrap()),
            Some(NaiveDate::from_ymd_opt(2026, 5, 31).unwrap()),
            Some(10),
        )
        .await
        .unwrap();
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].feature_count, 4);
    assert!(snapshots[0].features.contains_key("us_vix_level"));

    let exact_snapshots = store
        .list_feature_snapshots_for_mode(
            "financial_system",
            "feature_formal_v1",
            "best_effort",
            Some(NaiveDate::from_ymd_opt(2026, 5, 1).unwrap()),
            Some(NaiveDate::from_ymd_opt(2026, 5, 31).unwrap()),
        )
        .await
        .unwrap();
    assert_eq!(exact_snapshots.len(), 1);
    assert_eq!(exact_snapshots[0].point_in_time_mode, "best_effort");

    let dataset = FormalDatasetRecord {
        manifest: FormalDatasetManifest {
            dataset_id: "formal_v1_main_1990_daily".to_string(),
            dataset_version: "20260531T120000".to_string(),
            market_scope: "financial_system".to_string(),
            feature_set_version: "feature_formal_v1".to_string(),
            label_version: "formal_label_v1_main".to_string(),
            scenario_set_version: "scenario_v1".to_string(),
            point_in_time_mode: "best_effort".to_string(),
            from_date: Some(NaiveDate::from_ymd_opt(1990, 1, 2).unwrap()),
            to_date: Some(NaiveDate::from_ymd_opt(2026, 5, 30).unwrap()),
            train_end_date: Some(NaiveDate::from_ymd_opt(2014, 12, 31).unwrap()),
            calibration_end_date: Some(NaiveDate::from_ymd_opt(2019, 12, 31).unwrap()),
            evaluation_start_date: Some(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
            row_count: 1,
            note: "unit test dataset".to_string(),
        },
        created_at,
    };
    store.upsert_formal_dataset(&dataset).await.unwrap();
    let dataset_key = super::formal_dataset_key(
        &dataset.manifest.dataset_id,
        &dataset.manifest.dataset_version,
    );
    let row = FormalDatasetRowRecord {
        dataset_key: dataset_key.clone(),
        split_name: "evaluation".to_string(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        as_of_date: NaiveDate::from_ymd_opt(2026, 5, 30).unwrap(),
        point_in_time_mode: "best_effort".to_string(),
        latest_visible_at: Some(created_at),
        coverage_score: 0.91,
        core_feature_coverage: 0.94,
        trigger_feature_coverage: 0.88,
        external_feature_coverage: 0.81,
        sample_quality_grade: "a".to_string(),
        primary_scenario_id: None,
        scenario_family: None,
        scenario_training_role: None,
        label_5d: 0,
        label_20d: 0,
        label_60d: 0,
        regime_5d: "normal".to_string(),
        regime_20d: "normal".to_string(),
        regime_60d: "normal".to_string(),
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
        features: snapshot.features.clone(),
        created_at,
    };
    store
        .replace_formal_dataset_rows(&dataset_key, &[row.clone()])
        .await
        .unwrap();

    let loaded_dataset = store
        .load_formal_dataset(&dataset_key)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded_dataset.manifest.row_count, 1);
    assert_eq!(
        loaded_dataset.manifest.dataset_id,
        "formal_v1_main_1990_daily"
    );

    let rows = store
        .list_formal_dataset_rows(&dataset_key, Some("evaluation"), Some(10))
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].split_name, "evaluation");
    assert_eq!(rows[0].dataset_key, dataset_key);
    assert_eq!(rows[0].regime_60d, "normal");
    assert_eq!(rows[0].features["us_vix_level"], 22.4);
}

#[tokio::test]
async fn sqlite_store_round_trips_historical_replay_runs_and_points() {
    let store = SqliteStore::connect_url("sqlite::memory:").await.unwrap();
    store.migrate().await.unwrap();
    let created_at = Utc::now();
    let release = ModelReleaseRecord {
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
    };
    store.upsert_model_release(&release).await.unwrap();

    let run = HistoricalReplayRunRecord {
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
    };
    store.upsert_historical_replay_run(&run).await.unwrap();

    let point = HistoricalAssessmentPointRecord {
        replay_run_id: run.replay_run_id.clone(),
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
    };
    store
        .replace_historical_assessment_points(&run.replay_run_id, &[point.clone()])
        .await
        .unwrap();

    let loaded_run = store
        .load_latest_historical_replay_run(
            "financial_system",
            Some("release-1"),
            "history_cache_v3|release=release-1",
            NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 5, 30).unwrap(),
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded_run.replay_run_id, "replay-1");
    assert_eq!(loaded_run.point_count, 1);

    let runs = store
        .list_historical_replay_runs(
            Some("financial_system"),
            Some("release-1"),
            Some(NaiveDate::from_ymd_opt(2026, 5, 1).unwrap()),
            Some(NaiveDate::from_ymd_opt(2026, 5, 31).unwrap()),
            Some(10),
        )
        .await
        .unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(
        runs[0].history_cache_key,
        "history_cache_v3|release=release-1"
    );

    let points = store
        .list_historical_assessment_points(
            Some("replay-1"),
            Some("financial_system"),
            Some("release-1"),
            Some(NaiveDate::from_ymd_opt(2026, 5, 1).unwrap()),
            Some(NaiveDate::from_ymd_opt(2026, 5, 31).unwrap()),
            Some(10),
        )
        .await
        .unwrap();
    assert_eq!(points.len(), 1);
    assert_eq!(points[0].posture, "prepare");
    assert_eq!(points[0].actionability_prepare, 0.61);
    assert_eq!(
        points[0].posture_trigger_codes,
        vec!["prepare_p60d_structural".to_string()]
    );
    assert_eq!(
        points[0].probability_diagnostics.horizon_overlays[0].final_probability,
        0.21
    );
    assert_eq!(
        points[0].probability_diagnostics.horizon_overlays[0].contributions[0].family_id,
        "jpy_carry"
    );
}

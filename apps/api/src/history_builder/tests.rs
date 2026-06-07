use chrono::{NaiveDate, Utc};
use fc_domain::{
    HistoricalAssessmentPointRecord, HistoricalReplayRunRecord, ModelReleaseManifest,
    ModelReleaseRecord, PredictionSnapshotRecord, ProbabilityBundle, ProbabilityDiagnostics,
};
use fc_storage::SqliteStore;

use super::{load_sqlite_assessment_history, AssessmentHistoryBuildMode};
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

use chrono::{NaiveDate, Utc};
use fc_domain::PredictionSnapshotRecord;

use crate::sqlite::tests::in_memory_store;

#[tokio::test]
async fn sqlite_store_round_trips_prediction_snapshots() {
    let store = in_memory_store().await;
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
async fn sqlite_store_can_prune_prediction_snapshot_history_for_release() {
    let store = in_memory_store().await;
    let recorded_at = Utc::now();
    let release_id = "formal_release_20260606";
    let old_snapshot = PredictionSnapshotRecord {
        as_of_date: NaiveDate::from_ymd_opt(2026, 5, 30).unwrap(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        release_id: Some(release_id.to_string()),
        probability_mode: "formal_bundle_v1".to_string(),
        release_status: "healthy".to_string(),
        point_in_time_mode: "best_effort".to_string(),
        overall_score: 50.0,
        external_shock_score: 40.0,
        raw_p_5d: 0.1,
        raw_p_20d: 0.2,
        raw_p_60d: 0.3,
        calibrated_p_5d: 0.1,
        calibrated_p_20d: 0.2,
        calibrated_p_60d: 0.3,
        posture: "normal".to_string(),
        time_to_risk_bucket: "normal".to_string(),
        feature_set_version: "feature_formal_v1_main_20260531".to_string(),
        label_version: "formal_label_v1_main".to_string(),
        coverage_score: 0.9,
        freshness_status: "fresh".to_string(),
        method_version: "runtime_history_v2".to_string(),
        posture_trigger_codes: Vec::new(),
        posture_blocker_codes: Vec::new(),
        recorded_at,
    };
    let keep_snapshot = PredictionSnapshotRecord {
        as_of_date: NaiveDate::from_ymd_opt(2026, 5, 31).unwrap(),
        recorded_at,
        ..old_snapshot.clone()
    };
    let other_release_snapshot = PredictionSnapshotRecord {
        release_id: Some("heuristic_release_20260606".to_string()),
        recorded_at,
        ..keep_snapshot.clone()
    };

    store
        .upsert_prediction_snapshots(&[
            old_snapshot.clone(),
            keep_snapshot.clone(),
            other_release_snapshot.clone(),
        ])
        .await
        .unwrap();

    let deleted = store
        .delete_prediction_snapshot_history_for_release(
            "financial_system",
            release_id,
            keep_snapshot.as_of_date,
        )
        .await
        .unwrap();

    assert_eq!(deleted, 1);

    let kept_rows = store
        .list_prediction_snapshots(Some("financial_system"), Some(release_id), None, None, None)
        .await
        .unwrap();
    assert_eq!(kept_rows.len(), 1);
    assert_eq!(kept_rows[0].as_of_date, keep_snapshot.as_of_date);

    let other_rows = store
        .list_prediction_snapshots(
            Some("financial_system"),
            Some("heuristic_release_20260606"),
            None,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(other_rows.len(), 1);
    assert_eq!(other_rows[0].as_of_date, other_release_snapshot.as_of_date);
}

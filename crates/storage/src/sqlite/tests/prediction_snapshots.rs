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

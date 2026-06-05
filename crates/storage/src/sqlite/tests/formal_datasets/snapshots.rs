use chrono::{NaiveDate, Utc};

use crate::sqlite::tests::formal_datasets::fixtures::feature_snapshot;
use crate::sqlite::tests::in_memory_store;

#[tokio::test]
async fn sqlite_store_round_trips_feature_snapshots() {
    let store = in_memory_store().await;
    let created_at = Utc::now();
    let snapshot = feature_snapshot(created_at);

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
}

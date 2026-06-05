use chrono::Utc;

use crate::sqlite::tests::formal_datasets::fixtures::{
    feature_snapshot, formal_dataset, formal_dataset_row,
};
use crate::sqlite::tests::in_memory_store;

#[tokio::test]
async fn sqlite_store_round_trips_formal_datasets_and_rows() {
    let store = in_memory_store().await;
    let created_at = Utc::now();
    let snapshot = feature_snapshot(created_at);
    let dataset = formal_dataset(created_at);
    store.upsert_formal_dataset(&dataset).await.unwrap();

    let dataset_key = super::super::super::formal_dataset_key(
        &dataset.manifest.dataset_id,
        &dataset.manifest.dataset_version,
    );
    let row = formal_dataset_row(created_at, &dataset_key, &snapshot);
    store
        .replace_formal_dataset_rows(&dataset_key, &[row])
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

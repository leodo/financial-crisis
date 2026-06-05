use chrono::{NaiveDate, Utc};
use fc_domain::{
    FeatureSnapshotRecord, FormalDatasetManifest, FormalDatasetRecord, FormalDatasetRowRecord,
};

use crate::sqlite::tests::in_memory_store;

#[tokio::test]
async fn sqlite_store_round_trips_feature_snapshots_and_formal_datasets() {
    let store = in_memory_store().await;
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
    let dataset_key = super::super::formal_dataset_key(
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

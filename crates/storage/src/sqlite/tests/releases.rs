use chrono::{NaiveDate, Utc};
use fc_domain::{ModelReleaseManifest, ModelReleaseRecord};

use crate::sqlite::tests::in_memory_store;

#[tokio::test]
async fn sqlite_store_round_trips_model_releases_and_active_pointer() {
    let store = in_memory_store().await;
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

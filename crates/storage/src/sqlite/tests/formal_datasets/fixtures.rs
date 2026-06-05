use chrono::{DateTime, NaiveDate, Utc};
use fc_domain::{
    FeatureSnapshotRecord, FormalDatasetManifest, FormalDatasetRecord, FormalDatasetRowRecord,
};

pub(super) fn feature_snapshot(created_at: DateTime<Utc>) -> FeatureSnapshotRecord {
    FeatureSnapshotRecord {
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
    }
}

pub(super) fn formal_dataset(created_at: DateTime<Utc>) -> FormalDatasetRecord {
    FormalDatasetRecord {
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
    }
}

pub(super) fn formal_dataset_row(
    created_at: DateTime<Utc>,
    dataset_key: &str,
    snapshot: &FeatureSnapshotRecord,
) -> FormalDatasetRowRecord {
    FormalDatasetRowRecord {
        dataset_key: dataset_key.to_string(),
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
    }
}

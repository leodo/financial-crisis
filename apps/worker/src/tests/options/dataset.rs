use super::*;

#[test]
fn parses_formal_dataset_build_options() {
    let args = vec![
        "--market-scope".to_string(),
        "financial_system".to_string(),
        "--dataset-id".to_string(),
        "formal_v1_main_1990_daily".to_string(),
        "--dataset-version".to_string(),
        "20260531T120000".to_string(),
        "--label-version".to_string(),
        "formal_label_v1_main".to_string(),
    ];
    let options = FormalDatasetBuildOptions::parse(&args).unwrap();
    assert_eq!(options.feature.market_scope, "financial_system");
    assert_eq!(options.dataset_id, "formal_v1_main_1990_daily");
    assert_eq!(options.dataset_version.as_deref(), Some("20260531T120000"));
    assert_eq!(options.label_version, "formal_label_v1_main");
}

#[test]
fn extension_acute_dataset_min_date_allows_pre1990_history() {
    assert_eq!(
        crate::formal_dataset_min_date("formal_label_v1_ext_acute"),
        NaiveDate::from_ymd_opt(1987, 1, 1).unwrap()
    );
    assert_eq!(
        crate::formal_dataset_min_date("formal_label_v1_main"),
        NaiveDate::from_ymd_opt(1990, 1, 2).unwrap()
    );
}

#[test]
fn extension_acute_dataset_allows_proxy_feature_gate_without_vix() {
    let snapshot = FeatureSnapshotRecord {
        as_of_date: NaiveDate::from_ymd_opt(1987, 10, 19).unwrap(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        feature_set_version: "feature_formal_v1_main_20260531".to_string(),
        point_in_time_mode: "best_effort".to_string(),
        visibility_status: "coverage_or_visibility_failed".to_string(),
        latest_visible_at: Some(Utc::now()),
        coverage_score: 0.56,
        core_feature_coverage: 0.625,
        trigger_feature_coverage: 0.50,
        external_feature_coverage: 0.50,
        feature_count: 4,
        features: [
            ("us_curve_10y2y_level".to_string(), -0.2),
            ("us_baa_10y_spread_level".to_string(), 2.8),
            ("us_fed_funds_level".to_string(), 6.5),
            ("us_usdjpy_level".to_string(), 0.0068),
        ]
        .into_iter()
        .collect(),
        created_at: Utc::now(),
    };

    assert!(crate::formal_dataset_snapshot_is_usable(
        &snapshot,
        "formal_label_v1_ext_acute"
    ));
    assert!(!crate::formal_dataset_snapshot_is_usable(
        &snapshot,
        "formal_label_v1_main"
    ));
}

#[test]
fn extension_stress_dataset_allows_1990s_partial_coverage_gate() {
    let snapshot = FeatureSnapshotRecord {
        as_of_date: NaiveDate::from_ymd_opt(1993, 1, 5).unwrap(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        feature_set_version: "feature_formal_v1_main_20260531".to_string(),
        point_in_time_mode: "best_effort".to_string(),
        visibility_status: "ready".to_string(),
        latest_visible_at: Some(Utc::now()),
        coverage_score: 0.785,
        core_feature_coverage: 0.875,
        trigger_feature_coverage: 0.833,
        external_feature_coverage: 0.50,
        feature_count: 4,
        features: [
            ("us_vix_level".to_string(), 12.0),
            ("us_curve_10y2y_level".to_string(), 1.2),
            ("us_baa_10y_spread_level".to_string(), 2.1),
            ("us_fed_funds_level".to_string(), 3.0),
        ]
        .into_iter()
        .collect(),
        created_at: Utc::now(),
    };

    assert!(crate::formal_dataset_snapshot_is_usable(
        &snapshot,
        "formal_label_v1_ext_stress"
    ));
    assert!(!crate::formal_dataset_snapshot_is_usable(
        &snapshot,
        "formal_label_v1_main"
    ));
}

#[test]
fn parses_formal_dataset_summary_options() {
    let args = vec![
        "--dataset-id".to_string(),
        "formal_v1_main_1990_daily".to_string(),
        "--dataset-version".to_string(),
        "20260531Tpitmain".to_string(),
        "--output-dir".to_string(),
        "reports/formal-dataset".to_string(),
    ];
    let options = FormalDatasetSummaryOptions::parse(&args).unwrap();
    assert_eq!(options.dataset_id, "formal_v1_main_1990_daily");
    assert_eq!(options.dataset_version.as_deref(), Some("20260531Tpitmain"));
    assert_eq!(options.output_dir, PathBuf::from("reports/formal-dataset"));
}

#[test]
fn formal_dataset_summary_defaults_to_ignored_artifact_dir() {
    let options = FormalDatasetSummaryOptions::parse(&[]).unwrap();
    assert_eq!(
        options.output_dir,
        PathBuf::from("artifacts/research/formal-dataset")
    );
}

#[test]
fn parses_formal_dataset_slice_options() {
    let args = vec![
        "--dataset-key".to_string(),
        "formal_v1_main_1990_daily:20260531Tpitmain".to_string(),
        "--scenario-id".to_string(),
        "us_regional_banks_2023".to_string(),
        "--from".to_string(),
        "2022-12-01".to_string(),
        "--to".to_string(),
        "2023-03-15".to_string(),
        "--split-name".to_string(),
        "evaluation".to_string(),
        "--limit".to_string(),
        "120".to_string(),
        "--output-dir".to_string(),
        "reports/formal-dataset-slices".to_string(),
    ];
    let options = FormalDatasetSliceOptions::parse(&args).unwrap();
    assert_eq!(
        options.dataset_key.as_deref(),
        Some("formal_v1_main_1990_daily:20260531Tpitmain")
    );
    assert_eq!(options.scenario_id, "us_regional_banks_2023");
    assert_eq!(
        options.from_date,
        Some(NaiveDate::from_ymd_opt(2022, 12, 1).unwrap())
    );
    assert_eq!(
        options.to_date,
        Some(NaiveDate::from_ymd_opt(2023, 3, 15).unwrap())
    );
    assert_eq!(options.split_name.as_deref(), Some("evaluation"));
    assert_eq!(options.limit, Some(120));
    assert_eq!(
        options.output_dir,
        PathBuf::from("reports/formal-dataset-slices")
    );
}

#[test]
fn formal_dataset_slice_defaults_to_ignored_artifact_dir() {
    let args = vec![
        "--scenario-id".to_string(),
        "us_regional_banks_2023".to_string(),
    ];
    let options = FormalDatasetSliceOptions::parse(&args).unwrap();
    assert_eq!(
        options.output_dir,
        PathBuf::from("artifacts/research/formal-dataset-slices")
    );
}

#[test]
fn sanitize_filename_component_replaces_windows_reserved_characters() {
    assert_eq!(
        sanitize_filename_component("formal_v1_main_1990_daily:20260601T172759"),
        "formal_v1_main_1990_daily_20260601T172759"
    );
}

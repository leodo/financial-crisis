use super::*;

#[test]
fn parses_prediction_snapshot_query_options() {
    let args = vec![
        "--market-scope".to_string(),
        "financial_system".to_string(),
        "--from".to_string(),
        "2026-05-01".to_string(),
        "--to".to_string(),
        "2026-05-31".to_string(),
        "--limit".to_string(),
        "50".to_string(),
    ];
    let options = PredictionSnapshotQueryOptions::parse(&args).unwrap();
    assert_eq!(options.market_scope.as_deref(), Some("financial_system"));
    assert_eq!(
        options.from,
        Some(NaiveDate::from_ymd_opt(2026, 5, 1).unwrap())
    );
    assert_eq!(
        options.to,
        Some(NaiveDate::from_ymd_opt(2026, 5, 31).unwrap())
    );
    assert_eq!(options.limit, Some(50));
}

#[test]
fn parses_feature_snapshot_build_options() {
    let args = vec![
        "--market-scope".to_string(),
        "financial_system".to_string(),
        "--from".to_string(),
        "2020-01-01".to_string(),
        "--to".to_string(),
        "2020-12-31".to_string(),
        "--feature-set-version".to_string(),
        "feature_formal_v1_test".to_string(),
    ];
    let options = FeatureSnapshotBuildOptions::parse(&args).unwrap();
    assert_eq!(options.market_scope, "financial_system");
    assert_eq!(
        options.from,
        Some(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap())
    );
    assert_eq!(
        options.to,
        Some(NaiveDate::from_ymd_opt(2020, 12, 31).unwrap())
    );
    assert_eq!(options.feature_set_version, "feature_formal_v1_test");
    assert_eq!(options.point_in_time_mode, "best_effort");
    assert!(!options.force_rebuild);
}

#[test]
fn parses_feature_snapshot_force_rebuild_option() {
    let args = vec!["--force-rebuild".to_string()];
    let options = FeatureSnapshotBuildOptions::parse(&args).unwrap();
    assert!(options.force_rebuild);
}

use super::*;

#[test]
fn parses_release_publish_options() {
    let args = vec![
        "--manifest".to_string(),
        "config/model-releases/us-heuristic-bootstrap.json".to_string(),
        "--activate".to_string(),
        "--reload-api".to_string(),
        "--skip-operational-guard".to_string(),
        "--updated-by".to_string(),
        "tester".to_string(),
    ];
    let options = ReleasePublishOptions::parse(&args).unwrap();
    assert!(options.activate);
    assert!(options.reload_api);
    assert!(options.skip_operational_guard);
    assert_eq!(options.updated_by, "tester");
    assert_eq!(
        options.manifest_path,
        PathBuf::from("config/model-releases/us-heuristic-bootstrap.json")
    );
}

#[test]
fn parses_release_switch_options() {
    let args = vec![
        "--release-id".to_string(),
        "release-123".to_string(),
        "--market-scope".to_string(),
        "financial_system".to_string(),
        "--reload-api".to_string(),
        "--skip-operational-guard".to_string(),
    ];
    let options = ReleaseSwitchOptions::parse(&args).unwrap();
    assert_eq!(options.release_id, "release-123");
    assert_eq!(options.market_scope.as_deref(), Some("financial_system"));
    assert!(options.reload_api);
    assert!(options.skip_operational_guard);
}

#[test]
fn parses_release_review_options() {
    let args = vec![
        "--candidate-release-id".to_string(),
        "candidate-123".to_string(),
        "--baseline-release-id".to_string(),
        "baseline-456".to_string(),
        "--market-scope".to_string(),
        "financial_system".to_string(),
        "--output-dir".to_string(),
        "reports/release-review".to_string(),
        "--history-mode".to_string(),
        "default".to_string(),
        "--history-limit".to_string(),
        "5000".to_string(),
    ];
    let options = ReleaseReviewOptions::parse(&args).unwrap();
    assert_eq!(options.candidate_release_id, "candidate-123");
    assert_eq!(options.baseline_release_id.as_deref(), Some("baseline-456"));
    assert_eq!(options.market_scope.as_deref(), Some("financial_system"));
    assert_eq!(options.output_dir, PathBuf::from("reports/release-review"));
    assert_eq!(options.history_mode, crate::ApiReloadHistoryMode::Default);
    assert_eq!(options.history_limit, 5000);
}

#[test]
fn release_review_defaults_to_ignored_artifact_dir() {
    let args = vec![
        "--candidate-release-id".to_string(),
        "candidate-123".to_string(),
    ];
    let options = ReleaseReviewOptions::parse(&args).unwrap();
    assert_eq!(
        options.output_dir,
        PathBuf::from("artifacts/research/release-review")
    );
    assert_eq!(
        options.history_mode,
        crate::ApiReloadHistoryMode::StrictRebuild
    );
    assert_eq!(options.history_limit, 20_000);
}

#[test]
fn parses_release_probability_slice_options() {
    let args = vec![
        "--release-id".to_string(),
        "us_formal_family_hybrid_20260603T144814".to_string(),
        "--from".to_string(),
        "2022-12-01".to_string(),
        "--to".to_string(),
        "2023-03-15".to_string(),
        "--output-dir".to_string(),
        "reports/release-probability-slices".to_string(),
        "--history-mode".to_string(),
        "default".to_string(),
        "--history-limit".to_string(),
        "5000".to_string(),
    ];
    let options = ReleaseProbabilitySliceOptions::parse(&args).unwrap();
    assert_eq!(
        options.release_id,
        "us_formal_family_hybrid_20260603T144814"
    );
    assert_eq!(
        options.from_date,
        NaiveDate::from_ymd_opt(2022, 12, 1).unwrap()
    );
    assert_eq!(
        options.to_date,
        NaiveDate::from_ymd_opt(2023, 3, 15).unwrap()
    );
    assert_eq!(
        options.output_dir,
        PathBuf::from("reports/release-probability-slices")
    );
    assert_eq!(options.history_mode, crate::ApiReloadHistoryMode::Default);
    assert_eq!(options.history_limit, 5000);
}

#[test]
fn release_probability_slice_defaults_to_ignored_artifact_dir() {
    let args = vec![
        "--release-id".to_string(),
        "candidate-123".to_string(),
        "--from".to_string(),
        "2023-01-01".to_string(),
        "--to".to_string(),
        "2023-01-31".to_string(),
    ];
    let options = ReleaseProbabilitySliceOptions::parse(&args).unwrap();
    assert_eq!(
        options.output_dir,
        PathBuf::from("artifacts/research/release-probability-slices")
    );
    assert_eq!(
        options.history_mode,
        crate::ApiReloadHistoryMode::StrictRebuild
    );
    assert_eq!(options.history_limit, 20_000);
}

#[test]
fn parses_release_formal_probability_slice_options() {
    let args = vec![
        "--release-id".to_string(),
        "us_formal_family_hybrid_20260603T144814".to_string(),
        "--dataset-id".to_string(),
        "formal_v1_main_1990_daily".to_string(),
        "--dataset-version".to_string(),
        "20260601T172759".to_string(),
        "--scenario-id".to_string(),
        "us_regional_banks_2023".to_string(),
        "--from".to_string(),
        "2022-12-01".to_string(),
        "--to".to_string(),
        "2023-03-15".to_string(),
        "--output-dir".to_string(),
        "reports/formal-dataset-slices".to_string(),
    ];
    let options = ReleaseFormalProbabilitySliceOptions::parse(&args).unwrap();
    assert_eq!(
        options.release_id,
        "us_formal_family_hybrid_20260603T144814"
    );
    assert_eq!(options.dataset_id, "formal_v1_main_1990_daily");
    assert_eq!(options.dataset_version.as_deref(), Some("20260601T172759"));
    assert_eq!(
        options.scenario_id.as_deref(),
        Some("us_regional_banks_2023")
    );
    assert_eq!(
        options.from_date,
        NaiveDate::from_ymd_opt(2022, 12, 1).unwrap()
    );
    assert_eq!(
        options.to_date,
        NaiveDate::from_ymd_opt(2023, 3, 15).unwrap()
    );
    assert_eq!(
        options.output_dir,
        PathBuf::from("reports/formal-dataset-slices")
    );
}

#[test]
fn release_formal_probability_slice_defaults_to_formal_dataset_artifact_dir() {
    let args = vec![
        "--release-id".to_string(),
        "candidate-123".to_string(),
        "--from".to_string(),
        "2023-01-01".to_string(),
        "--to".to_string(),
        "2023-01-31".to_string(),
    ];
    let options = ReleaseFormalProbabilitySliceOptions::parse(&args).unwrap();
    assert_eq!(
        options.output_dir,
        PathBuf::from("artifacts/research/formal-dataset-slices")
    );
    assert_eq!(options.dataset_id, "formal_v1_main_1990_daily");
    assert_eq!(options.dataset_version, None);
    assert_eq!(options.dataset_key, None);
    assert_eq!(options.scenario_id, None);
}

#[test]
fn parses_release_formal_probability_compare_options() {
    let args = vec![
        "--baseline-release-id".to_string(),
        "baseline-123".to_string(),
        "--candidate-release-id".to_string(),
        "candidate-456".to_string(),
        "--dataset-id".to_string(),
        "formal_v1_main_1990_daily".to_string(),
        "--scenario-id".to_string(),
        "us_regional_banks_2023".to_string(),
        "--from".to_string(),
        "2022-12-01".to_string(),
        "--to".to_string(),
        "2023-03-15".to_string(),
        "--output-dir".to_string(),
        "reports/formal-probability-compares".to_string(),
    ];
    let options = ReleaseFormalProbabilityCompareOptions::parse(&args).unwrap();
    assert_eq!(options.baseline_release_id, "baseline-123");
    assert_eq!(options.candidate_release_id, "candidate-456");
    assert_eq!(options.dataset_id, "formal_v1_main_1990_daily");
    assert_eq!(
        options.scenario_id.as_deref(),
        Some("us_regional_banks_2023")
    );
    assert_eq!(
        options.from_date,
        NaiveDate::from_ymd_opt(2022, 12, 1).unwrap()
    );
    assert_eq!(
        options.to_date,
        NaiveDate::from_ymd_opt(2023, 3, 15).unwrap()
    );
    assert_eq!(
        options.output_dir,
        PathBuf::from("reports/formal-probability-compares")
    );
}

#[test]
fn release_formal_probability_compare_defaults_to_compare_artifact_dir() {
    let args = vec![
        "--baseline-release-id".to_string(),
        "baseline-123".to_string(),
        "--candidate-release-id".to_string(),
        "candidate-456".to_string(),
        "--from".to_string(),
        "2023-01-01".to_string(),
        "--to".to_string(),
        "2023-01-31".to_string(),
    ];
    let options = ReleaseFormalProbabilityCompareOptions::parse(&args).unwrap();
    assert_eq!(
        options.output_dir,
        PathBuf::from("artifacts/research/formal-probability-compares")
    );
    assert_eq!(options.dataset_id, "formal_v1_main_1990_daily");
    assert_eq!(options.dataset_version, None);
    assert_eq!(options.dataset_key, None);
    assert_eq!(options.scenario_id, None);
}

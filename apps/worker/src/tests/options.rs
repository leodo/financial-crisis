#[test]
fn parses_refresh_latest_defaults() {
    let options = RefreshLatestOptions::parse(&[]).unwrap();
    assert_eq!(options.fast_lookback_days, 45);
    assert_eq!(options.slow_lookback_years, 15);
    assert_eq!(options.fred_chunk_days, 45);
    assert!(!options.skip_world_bank);
    assert!(!options.include_gdelt);
    assert!(options.reload_api);
}

#[test]
fn parses_refresh_latest_overrides() {
    let args = vec![
        "--fast-lookback-days".to_string(),
        "90".to_string(),
        "--skip-world-bank".to_string(),
        "--include-gdelt".to_string(),
        "--no-reload-api".to_string(),
    ];
    let options = RefreshLatestOptions::parse(&args).unwrap();
    assert_eq!(options.fast_lookback_days, 90);
    assert!(options.skip_world_bank);
    assert!(options.include_gdelt);
    assert!(!options.reload_api);
}

#[test]
fn parses_audit_export_overrides() {
    let args = vec![
        "--api-base-url".to_string(),
        "http://127.0.0.1:18081".to_string(),
        "--output-dir".to_string(),
        "tmp/audit".to_string(),
    ];
    let options = AuditExportOptions::parse(&args).unwrap();
    assert_eq!(options.api_base_url, "http://127.0.0.1:18081");
    assert_eq!(options.output_dir, PathBuf::from("tmp/audit"));
}

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
    assert_eq!(options.history_mode, super::ApiReloadHistoryMode::Default);
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
        super::ApiReloadHistoryMode::StrictRebuild
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
    assert_eq!(options.history_mode, super::ApiReloadHistoryMode::Default);
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
        super::ApiReloadHistoryMode::StrictRebuild
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
        super::formal_dataset_min_date("formal_label_v1_ext_acute"),
        NaiveDate::from_ymd_opt(1987, 1, 1).unwrap()
    );
    assert_eq!(
        super::formal_dataset_min_date("formal_label_v1_main"),
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

    assert!(super::formal_dataset_snapshot_is_usable(
        &snapshot,
        "formal_label_v1_ext_acute"
    ));
    assert!(!super::formal_dataset_snapshot_is_usable(
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

    assert!(super::formal_dataset_snapshot_is_usable(
        &snapshot,
        "formal_label_v1_ext_stress"
    ));
    assert!(!super::formal_dataset_snapshot_is_usable(
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

#[test]
fn parses_pipeline_train_defaults_to_formal_dataset() {
    let options = PipelineTrainOptions::parse(&[]).unwrap();
    assert_eq!(options.dataset_source, PipelineDatasetSource::Formal);
    assert_eq!(options.model_shape, ProbabilityModelShape::LinearV1);
    assert_eq!(options.dataset_id, "formal_v1_main_1990_daily");
    assert_eq!(options.dataset_version, None);
    assert_eq!(options.dataset_key, None);
    assert!(options.aux_dataset_keys.is_empty());
    assert_eq!(
        options.output_dir,
        PathBuf::from("artifacts/research/model-bundles/generated")
    );
    assert_eq!(
        options.manifest_dir,
        PathBuf::from("artifacts/research/model-releases/generated")
    );
    assert_eq!(options.release_prefix, "us_formal_main");
}

#[test]
fn parses_pipeline_train_snapshot_override() {
    let args = vec![
        "--dataset-source".to_string(),
        "snapshot".to_string(),
        "--release-prefix".to_string(),
        "custom_prefix".to_string(),
        "--market-scope".to_string(),
        "financial_system".to_string(),
    ];
    let options = PipelineTrainOptions::parse(&args).unwrap();
    assert_eq!(options.dataset_source, PipelineDatasetSource::Snapshot);
    assert_eq!(options.model_shape, ProbabilityModelShape::LinearV1);
    assert_eq!(options.release_prefix, "custom_prefix");
    assert_eq!(
        options.query.market_scope.as_deref(),
        Some("financial_system")
    );
}

#[test]
fn parses_pipeline_train_interaction_tail_shape() {
    let args = vec![
        "--model-shape".to_string(),
        "interaction_tail_v1".to_string(),
    ];
    let options = PipelineTrainOptions::parse(&args).unwrap();

    assert_eq!(
        options.model_shape,
        ProbabilityModelShape::InteractionTailV1
    );
    assert_eq!(options.release_prefix, "us_formal_interaction_tail");
}

#[test]
fn parses_pipeline_train_family_conditional_shape() {
    let args = vec![
        "--model-shape".to_string(),
        "family_conditional_v1".to_string(),
    ];
    let options = PipelineTrainOptions::parse(&args).unwrap();

    assert_eq!(
        options.model_shape,
        ProbabilityModelShape::FamilyConditionalV1
    );
    assert_eq!(options.release_prefix, "us_formal_family_conditional");
}

#[test]
fn parses_pipeline_train_family_hybrid_shape() {
    let args = vec!["--model-shape".to_string(), "family_hybrid_v1".to_string()];
    let options = PipelineTrainOptions::parse(&args).unwrap();

    assert_eq!(options.model_shape, ProbabilityModelShape::FamilyHybridV1);
    assert_eq!(options.release_prefix, "us_formal_family_hybrid");
}

#[test]
fn parses_pipeline_train_aux_dataset_keys() {
    let args = vec![
        "--dataset-key".to_string(),
        "formal_v1_main_1990_daily:20260601T172759".to_string(),
        "--aux-dataset-key".to_string(),
        "formal_v1_ext_stress_1990_daily:20260601T162655".to_string(),
        "--aux-dataset-key".to_string(),
        "formal_v1_ext_acute_pre1990:20260601T163102".to_string(),
    ];
    let options = PipelineTrainOptions::parse(&args).unwrap();
    assert_eq!(
        options.dataset_key.as_deref(),
        Some("formal_v1_main_1990_daily:20260601T172759")
    );
    assert_eq!(
        options.aux_dataset_keys,
        vec![
            "formal_v1_ext_stress_1990_daily:20260601T162655".to_string(),
            "formal_v1_ext_acute_pre1990:20260601T163102".to_string()
        ]
    );
}

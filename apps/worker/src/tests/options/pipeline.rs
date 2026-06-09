use super::*;

#[test]
fn parses_pipeline_train_defaults_to_formal_dataset() {
    let options = PipelineTrainOptions::parse(&[]).unwrap();
    assert_eq!(options.dataset_source, PipelineDatasetSource::Formal);
    assert_eq!(options.model_shape, ProbabilityModelShape::LinearV1);
    assert!(!options.dry_run);
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
fn parses_pipeline_train_dry_run_flag() {
    let args = vec![
        "--dry-run".to_string(),
        "--model-shape".to_string(),
        "family_hybrid_v1".to_string(),
    ];
    let options = PipelineTrainOptions::parse(&args).unwrap();

    assert!(options.dry_run);
    assert_eq!(options.model_shape, ProbabilityModelShape::FamilyHybridV1);
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

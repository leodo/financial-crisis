use super::*;

#[test]
fn render_dataset_csv_includes_scenario_role_columns() {
    let mut row = forward_crisis_row(
        NaiveDate::from_ymd_opt(2020, 2, 20).unwrap(),
        1,
        ProbabilityTrainingRegime::PositiveWindow,
    );
    row.scenario_training_role = Some("mandatory".to_string());
    row.features.insert("stress".to_string(), 0.42);

    let csv = crate::commands::snapshot::render_dataset_csv(&[row], &[String::from("stress")]);
    let mut lines = csv.lines();
    let header = lines.next().unwrap_or_default();
    let first_row = lines.next().unwrap_or_default();

    assert!(header.contains("primary_scenario_id"));
    assert!(header.contains("scenario_family"));
    assert!(header.contains("scenario_training_role"));
    assert!(first_row.contains(",scenario_a,systemic_credit_banking_crisis,mandatory,"));
}

#[test]
fn render_formal_dataset_slice_csv_includes_feature_columns() {
    let mut row = FormalDatasetRowRecord {
        dataset_key: "dataset".to_string(),
        split_name: "evaluation".to_string(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        as_of_date: NaiveDate::from_ymd_opt(2023, 2, 20).unwrap(),
        point_in_time_mode: "best_effort".to_string(),
        latest_visible_at: None,
        coverage_score: 0.92,
        core_feature_coverage: 0.95,
        trigger_feature_coverage: 0.88,
        external_feature_coverage: 0.84,
        sample_quality_grade: "a".to_string(),
        primary_scenario_id: Some("us_regional_banks_2023".to_string()),
        scenario_family: Some("systemic_credit_banking_crisis".to_string()),
        scenario_training_role: Some("mandatory".to_string()),
        label_5d: 0,
        label_20d: 1,
        label_60d: 1,
        regime_5d: "normal".to_string(),
        regime_20d: "pre_warning_buffer".to_string(),
        regime_60d: "positive_window".to_string(),
        action_label_5d: 0,
        action_label_20d: 1,
        action_label_60d: 1,
        prepare_episode_label: 1,
        hedge_episode_label: 1,
        defend_episode_label: 0,
        primary_action_level: Some("hedge".to_string()),
        action_episode_id: Some("us_regional_banks_2023:hedge".to_string()),
        action_episode_phase: "primary".to_string(),
        protected_action_window: false,
        features: BTreeMap::new(),
        created_at: Utc::now(),
    };
    row.features.insert("feature_a".to_string(), 0.42);

    let csv = render_formal_dataset_slice_csv(&[row], &[String::from("feature_a")]);
    let mut lines = csv.lines();
    let header = lines.next().unwrap_or_default();
    let first_row = lines.next().unwrap_or_default();

    assert!(header.contains("primary_scenario_id"));
    assert!(header.contains("feature_a"));
    assert!(first_row.contains("us_regional_banks_2023"));
    assert!(first_row.ends_with(",0.420000"));
}

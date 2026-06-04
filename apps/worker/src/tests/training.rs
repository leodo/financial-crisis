use super::*;

#[test]
fn best_effort_visibility_uses_release_rule_not_backfill_fetch_time_for_fred() {
    let observation = observation(
        "fred",
        Frequency::Monthly,
        NaiveDate::from_ymd_opt(2020, 1, 31).unwrap(),
        Some(Utc.with_ymd_and_hms(2026, 5, 31, 0, 0, 0).single().unwrap()),
    );

    assert!(!observation_is_visible_for_date(
        &observation,
        NaiveDate::from_ymd_opt(2020, 2, 14).unwrap(),
        PointInTimeMode::BestEffort
    ));
    assert!(observation_is_visible_for_date(
        &observation,
        NaiveDate::from_ymd_opt(2020, 2, 15).unwrap(),
        PointInTimeMode::BestEffort
    ));
}

#[test]
fn strict_visibility_requires_timestamp_to_arrive_before_cutoff() {
    let observation = observation(
        "sec_edgar",
        Frequency::Daily,
        NaiveDate::from_ymd_opt(2020, 1, 2).unwrap(),
        Some(Utc.with_ymd_and_hms(2020, 1, 2, 23, 0, 0).single().unwrap()),
    );

    assert!(!observation_is_visible_for_date(
        &observation,
        NaiveDate::from_ymd_opt(2020, 1, 2).unwrap(),
        PointInTimeMode::Strict
    ));
    assert!(observation_is_visible_for_date(
        &observation,
        NaiveDate::from_ymd_opt(2020, 1, 3).unwrap(),
        PointInTimeMode::Strict
    ));
}

#[test]
fn forward_crisis_label_uses_acute_anchor_for_5d_without_dropping_other_crisis_starts() {
    let acute_only = CrisisScenario {
        scenario_id: "acute".to_string(),
        family: "acute_market_liquidity_crash".to_string(),
        training_role: "mandatory".to_string(),
        pre_warning_start: NaiveDate::from_ymd_opt(2020, 1, 24).unwrap(),
        crisis_start: NaiveDate::from_ymd_opt(2020, 2, 24).unwrap(),
        acute_start: Some(NaiveDate::from_ymd_opt(2020, 3, 9).unwrap()),
        crisis_end: NaiveDate::from_ymd_opt(2020, 4, 30).unwrap(),
        default_horizon_roles: vec![5, 20],
        protected_window: false,
        protected_action_levels: Vec::new(),
        episode_template_id: ActionEpisodeTemplateId::AcuteMarketLiquidityCrash,
        action_episode_overrides: None,
    };
    let systemic_only = CrisisScenario {
        scenario_id: "systemic".to_string(),
        family: "systemic_credit_banking_crisis".to_string(),
        training_role: "mandatory".to_string(),
        pre_warning_start: NaiveDate::from_ymd_opt(2023, 2, 1).unwrap(),
        crisis_start: NaiveDate::from_ymd_opt(2023, 3, 8).unwrap(),
        acute_start: Some(NaiveDate::from_ymd_opt(2023, 3, 10).unwrap()),
        crisis_end: NaiveDate::from_ymd_opt(2023, 5, 15).unwrap(),
        default_horizon_roles: vec![20, 60],
        protected_window: false,
        protected_action_levels: Vec::new(),
        episode_template_id: ActionEpisodeTemplateId::SystemicCreditBankingCrisis,
        action_episode_overrides: None,
    };

    assert_eq!(
        forward_crisis_label(
            NaiveDate::from_ymd_opt(2020, 3, 4).unwrap(),
            &[acute_only.clone(), systemic_only.clone()],
            5,
        ),
        1
    );
    assert_eq!(
        forward_crisis_label(
            NaiveDate::from_ymd_opt(2020, 2, 20).unwrap(),
            &[acute_only.clone(), systemic_only.clone()],
            5,
        ),
        0
    );
    assert_eq!(
        forward_crisis_label(
            NaiveDate::from_ymd_opt(2023, 3, 4).unwrap(),
            &[acute_only.clone(), systemic_only.clone()],
            5,
        ),
        1
    );
    assert_eq!(
        forward_crisis_label(
            NaiveDate::from_ymd_opt(2023, 2, 20).unwrap(),
            &[acute_only, systemic_only],
            20,
        ),
        1
    );
}

#[test]
fn action_window_label_extends_before_crisis_start_and_stays_near_onset() {
    let systemic = CrisisScenario {
        scenario_id: "systemic".to_string(),
        family: "systemic_credit_banking_crisis".to_string(),
        training_role: "mandatory".to_string(),
        pre_warning_start: NaiveDate::from_ymd_opt(2007, 2, 27).unwrap(),
        crisis_start: NaiveDate::from_ymd_opt(2007, 8, 1).unwrap(),
        acute_start: Some(NaiveDate::from_ymd_opt(2008, 9, 15).unwrap()),
        crisis_end: NaiveDate::from_ymd_opt(2009, 6, 30).unwrap(),
        default_horizon_roles: vec![20, 60],
        protected_window: false,
        protected_action_levels: Vec::new(),
        episode_template_id: ActionEpisodeTemplateId::SystemicCreditBankingCrisis,
        action_episode_overrides: None,
    };
    let acute = CrisisScenario {
        scenario_id: "acute".to_string(),
        family: "acute_market_liquidity_crash".to_string(),
        training_role: "mandatory".to_string(),
        pre_warning_start: NaiveDate::from_ymd_opt(2020, 1, 24).unwrap(),
        crisis_start: NaiveDate::from_ymd_opt(2020, 2, 24).unwrap(),
        acute_start: Some(NaiveDate::from_ymd_opt(2020, 3, 9).unwrap()),
        crisis_end: NaiveDate::from_ymd_opt(2020, 4, 30).unwrap(),
        default_horizon_roles: vec![5, 20],
        protected_window: false,
        protected_action_levels: Vec::new(),
        episode_template_id: ActionEpisodeTemplateId::AcuteMarketLiquidityCrash,
        action_episode_overrides: None,
    };

    assert_eq!(
        action_window_label(
            NaiveDate::from_ymd_opt(2007, 5, 10).unwrap(),
            std::slice::from_ref(&systemic),
            60,
        ),
        1
    );
    assert_eq!(
        action_window_label(
            NaiveDate::from_ymd_opt(2020, 2, 28).unwrap(),
            std::slice::from_ref(&acute),
            5,
        ),
        1
    );
    assert_eq!(
        action_window_label(
            NaiveDate::from_ymd_opt(2007, 8, 15).unwrap(),
            std::slice::from_ref(&systemic),
            20,
        ),
        1
    );
    assert_eq!(
        action_window_label(
            NaiveDate::from_ymd_opt(2007, 9, 15).unwrap(),
            std::slice::from_ref(&systemic),
            20,
        ),
        0
    );
}

#[test]
fn forward_crisis_training_regime_marks_buffer_crisis_and_cooldown() {
    let systemic = CrisisScenario {
        scenario_id: "systemic".to_string(),
        family: "systemic_credit_banking_crisis".to_string(),
        training_role: "mandatory".to_string(),
        pre_warning_start: NaiveDate::from_ymd_opt(2007, 2, 27).unwrap(),
        crisis_start: NaiveDate::from_ymd_opt(2007, 8, 1).unwrap(),
        acute_start: Some(NaiveDate::from_ymd_opt(2008, 9, 15).unwrap()),
        crisis_end: NaiveDate::from_ymd_opt(2009, 6, 30).unwrap(),
        default_horizon_roles: vec![20, 60],
        protected_window: false,
        protected_action_levels: Vec::new(),
        episode_template_id: ActionEpisodeTemplateId::SystemicCreditBankingCrisis,
        action_episode_overrides: None,
    };

    assert_eq!(
        forward_crisis_training_regime(
            NaiveDate::from_ymd_opt(2007, 5, 10).unwrap(),
            std::slice::from_ref(&systemic),
            60,
        ),
        ProbabilityTrainingRegime::PreWarningBuffer
    );
    assert_eq!(
        forward_crisis_training_regime(
            NaiveDate::from_ymd_opt(2007, 6, 15).unwrap(),
            std::slice::from_ref(&systemic),
            60,
        ),
        ProbabilityTrainingRegime::PositiveWindow
    );
    assert_eq!(
        forward_crisis_training_regime(
            NaiveDate::from_ymd_opt(2008, 10, 1).unwrap(),
            std::slice::from_ref(&systemic),
            20,
        ),
        ProbabilityTrainingRegime::InCrisis
    );
    assert_eq!(
        forward_crisis_training_regime(
            NaiveDate::from_ymd_opt(2009, 7, 20).unwrap(),
            std::slice::from_ref(&systemic),
            20,
        ),
        ProbabilityTrainingRegime::PostCrisisCooldown
    );
    assert_eq!(
        forward_crisis_training_regime(
            NaiveDate::from_ymd_opt(2010, 1, 20).unwrap(),
            std::slice::from_ref(&systemic),
            20,
        ),
        ProbabilityTrainingRegime::Normal
    );
}

#[test]
fn protected_context_promotes_main_regime_buffer_without_changing_positive_labels() {
    let scenario_sets = crate::load_formal_dataset_scenario_sets(
        crate::DEFAULT_FORMAL_SCENARIO_SET_VERSION,
        crate::DEFAULT_FORMAL_LABEL_VERSION,
    )
    .unwrap();
    let protected_date = NaiveDate::from_ymd_opt(2021, 11, 15).unwrap();
    let cooldown_date = NaiveDate::from_ymd_opt(2022, 11, 10).unwrap();

    assert_eq!(
        forward_crisis_label(protected_date, &scenario_sets.positive_scenarios, 20),
        0
    );
    assert_eq!(
        forward_crisis_training_regime_with_context(
            protected_date,
            &scenario_sets.positive_scenarios,
            &scenario_sets.context_scenarios,
            20,
        ),
        ProbabilityTrainingRegime::PreWarningBuffer
    );
    assert_eq!(
        forward_crisis_training_regime_with_context(
            cooldown_date,
            &scenario_sets.positive_scenarios,
            &scenario_sets.context_scenarios,
            20,
        ),
        ProbabilityTrainingRegime::PostCrisisCooldown
    );
}

#[test]
fn forward_crisis_negative_weights_and_calibration_scope_follow_regime() {
    let positive_row = ProbabilityTrainingRow {
        as_of_date: NaiveDate::from_ymd_opt(2020, 2, 20).unwrap(),
        market_scope: "financial_system".to_string(),
        release_id: None,
        probability_mode: Some("formal_bundle_v1".to_string()),
        freshness_status: Some("a".to_string()),
        time_to_risk_bucket: Some("weeks".to_string()),
        split_name: Some("calibration".to_string()),
        features: BTreeMap::new(),
        primary_scenario_id: Some("scenario_a".to_string()),
        scenario_family: Some("systemic_credit_banking_crisis".to_string()),
        scenario_training_role: None,
        days_to_primary_crisis_start: Some(10),
        primary_scenario_supports_5d: true,
        primary_scenario_supports_20d: true,
        primary_scenario_supports_60d: true,
        label_5d: 0,
        label_20d: 1,
        label_60d: 0,
        regime_5d: ProbabilityTrainingRegime::Normal,
        regime_20d: ProbabilityTrainingRegime::PositiveWindow,
        regime_60d: ProbabilityTrainingRegime::Normal,
        action_label_5d: 0,
        action_label_20d: 1,
        action_label_60d: 0,
        prepare_episode_label: 0,
        hedge_episode_label: 1,
        defend_episode_label: 0,
        primary_action_level: Some("hedge".to_string()),
        action_episode_id: Some("scenario_a:hedge".to_string()),
        action_episode_phase: "primary".to_string(),
        protected_action_window: false,
    };
    let mut normal_negative = positive_row.clone();
    normal_negative.label_20d = 0;
    normal_negative.regime_20d = ProbabilityTrainingRegime::Normal;
    normal_negative.primary_scenario_id = None;
    normal_negative.scenario_family = None;
    normal_negative.days_to_primary_crisis_start = None;
    let mut buffer_negative = normal_negative.clone();
    buffer_negative.primary_scenario_id = Some("scenario_a".to_string());
    buffer_negative.scenario_family = Some("systemic_credit_banking_crisis".to_string());
    buffer_negative.days_to_primary_crisis_start = Some(28);
    buffer_negative.regime_20d = ProbabilityTrainingRegime::PreWarningBuffer;
    buffer_negative.regime_60d = ProbabilityTrainingRegime::PreWarningBuffer;
    let mut crisis_negative = normal_negative.clone();
    crisis_negative.primary_scenario_id = Some("scenario_a".to_string());
    crisis_negative.scenario_family = Some("systemic_credit_banking_crisis".to_string());
    crisis_negative.days_to_primary_crisis_start = Some(-5);
    crisis_negative.regime_20d = ProbabilityTrainingRegime::InCrisis;
    crisis_negative.regime_60d = ProbabilityTrainingRegime::InCrisis;
    let mut cooldown_negative = normal_negative.clone();
    cooldown_negative.primary_scenario_id = Some("scenario_a".to_string());
    cooldown_negative.scenario_family = Some("systemic_credit_banking_crisis".to_string());
    cooldown_negative.days_to_primary_crisis_start = Some(-35);
    cooldown_negative.regime_20d = ProbabilityTrainingRegime::PostCrisisCooldown;
    cooldown_negative.regime_60d = ProbabilityTrainingRegime::PostCrisisCooldown;

    assert_eq!(
        negative_sample_weight(
            &normal_negative,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
        ),
        1.10
    );
    assert_eq!(
        negative_sample_weight(
            &buffer_negative,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
        ),
        0.70
    );
    assert_eq!(
        negative_sample_weight(
            &buffer_negative,
            60,
            ProbabilityTargetLabelMode::ForwardCrisis,
        ),
        0.60
    );
    assert_eq!(
        negative_sample_weight(
            &crisis_negative,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
        ),
        1.25
    );
    assert_eq!(
        negative_sample_weight(
            &cooldown_negative,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
        ),
        1.45
    );
    let mut protected_negative = normal_negative.clone();
    protected_negative.primary_scenario_id = Some("scenario_protected".to_string());
    protected_negative.scenario_family = Some("mixed_systemic_stress".to_string());
    protected_negative.protected_action_window = true;
    assert_eq!(
        negative_sample_weight(
            &protected_negative,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
        ),
        0.55
    );
    assert_eq!(
        negative_sample_weight(
            &protected_negative,
            60,
            ProbabilityTargetLabelMode::ForwardCrisis,
        ),
        0.65
    );
    let mut protected_cooldown_negative = protected_negative.clone();
    protected_cooldown_negative.action_episode_phase = "cooldown".to_string();
    assert_eq!(
        negative_sample_weight(
            &protected_cooldown_negative,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
        ),
        1.20
    );
    assert_eq!(
        negative_sample_weight(
            &protected_cooldown_negative,
            60,
            ProbabilityTargetLabelMode::ForwardCrisis,
        ),
        1.35
    );
    assert_eq!(
        negative_sample_weight(
            &buffer_negative,
            20,
            ProbabilityTargetLabelMode::ActionWindow,
        ),
        0.75
    );
    assert_eq!(
        negative_sample_weight(
            &crisis_negative,
            20,
            ProbabilityTargetLabelMode::ActionWindow,
        ),
        1.70
    );
    assert_eq!(
        negative_sample_weight(
            &cooldown_negative,
            20,
            ProbabilityTargetLabelMode::ActionWindow,
        ),
        1.45
    );

    let calibration_rows = vec![
        positive_row.clone(),
        normal_negative.clone(),
        buffer_negative.clone(),
        crisis_negative.clone(),
    ];
    let selection = probability_calibration_selection_rows(
        &calibration_rows,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(selection.rows.len(), 4);
    assert!(!selection.used_full_split_fallback);
    assert_eq!(
        selection
            .rows
            .iter()
            .filter(|row| {
                row.label_20d > 0
                    || matches!(
                        row.regime_20d,
                        ProbabilityTrainingRegime::Normal
                            | ProbabilityTrainingRegime::PreWarningBuffer
                            | ProbabilityTrainingRegime::InCrisis
                    )
            })
            .count(),
        4
    );
    assert_eq!(
        forward_crisis_regime_sample_weight(20, ProbabilityTrainingRegime::PositiveWindow),
        2.2
    );
}

#[test]
fn formal_main_context_scenarios_include_protected_window_set() {
    let scenario_sets = crate::load_formal_dataset_scenario_sets(
        crate::DEFAULT_FORMAL_SCENARIO_SET_VERSION,
        crate::DEFAULT_FORMAL_LABEL_VERSION,
    )
    .unwrap();
    let positive_ids = scenario_sets
        .positive_scenarios
        .iter()
        .map(|scenario| scenario.scenario_id.as_str())
        .collect::<BTreeSet<_>>();
    let context_ids = scenario_sets
        .context_scenarios
        .iter()
        .map(|scenario| scenario.scenario_id.as_str())
        .collect::<BTreeSet<_>>();

    assert_eq!(positive_ids.len(), 3);
    assert!(positive_ids.contains("us_gfc_2008"));
    assert!(positive_ids.contains("us_covid_liquidity_2020"));
    assert!(positive_ids.contains("us_regional_banks_2023"));

    assert!(context_ids.len() > positive_ids.len());
    assert!(context_ids.contains("us_dotcom_unwind_2000"));
    assert!(context_ids.contains("us_funding_stress_2011"));
    assert!(context_ids.contains("us_rate_shock_2022"));
}

#[test]
fn forward_crisis_pairwise_targets_push_buffer_centroid_above_normal() {
    let make_row = |feature_value: f64, regime_20d: ProbabilityTrainingRegime, label_20d: u8| {
        let mut features = BTreeMap::new();
        features.insert("stress".to_string(), feature_value);
        ProbabilityTrainingRow {
            as_of_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
            market_scope: "financial_system".to_string(),
            release_id: None,
            probability_mode: Some("formal_bundle_v1".to_string()),
            freshness_status: Some("a".to_string()),
            time_to_risk_bucket: Some("weeks".to_string()),
            split_name: Some("train".to_string()),
            features,
            primary_scenario_id: Some("scenario".to_string()),
            scenario_family: Some("systemic_credit_banking_crisis".to_string()),
            scenario_training_role: None,
            days_to_primary_crisis_start: Some(10),
            primary_scenario_supports_5d: false,
            primary_scenario_supports_20d: true,
            primary_scenario_supports_60d: true,
            label_5d: 0,
            label_20d,
            label_60d: 0,
            regime_5d: ProbabilityTrainingRegime::Normal,
            regime_20d,
            regime_60d: ProbabilityTrainingRegime::Normal,
            action_label_5d: 0,
            action_label_20d: label_20d,
            action_label_60d: 0,
            prepare_episode_label: 0,
            hedge_episode_label: 0,
            defend_episode_label: 0,
            primary_action_level: None,
            action_episode_id: None,
            action_episode_phase: "outside".to_string(),
            protected_action_window: false,
        }
    };

    let rows = vec![
        make_row(0.0, ProbabilityTrainingRegime::Normal, 0),
        make_row(0.1, ProbabilityTrainingRegime::Normal, 0),
        make_row(0.8, ProbabilityTrainingRegime::PreWarningBuffer, 0),
        make_row(0.9, ProbabilityTrainingRegime::PreWarningBuffer, 0),
        make_row(1.2, ProbabilityTrainingRegime::PositiveWindow, 1),
        make_row(1.1, ProbabilityTrainingRegime::PositiveWindow, 1),
        make_row(1.0, ProbabilityTrainingRegime::PostCrisisCooldown, 0),
    ];
    let feature_stats = vec![crate::build_feature_stat(&rows, "stress")];
    let targets = forward_crisis_regime_pairwise_targets(
        &rows,
        &feature_stats,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
        false,
    );
    assert!(!targets.is_empty());
    assert!(targets.len() >= 5);
    let mut gradients = vec![0.0];
    crate::apply_regime_pairwise_gradient(&mut gradients, &[0.0], &targets, 100.0, 20, false);
    assert!(gradients[0] < 0.0);
}

#[test]
fn forward_crisis_sign_gradient_pushes_wrong_direction_coefficients_toward_zero() {
    let feature_names = vec![
        "us_baa_10y_spread_level".to_string(),
        "us_curve_10y2y_level".to_string(),
        "us_stlfsi_level".to_string(),
        "tail_neg__us_curve_10y2y_level__0".to_string(),
        "tail_pos__us_baa_10y_spread_level__2".to_string(),
    ];
    let weights = vec![-0.8, 0.5, -0.4, -0.6, -0.3];
    let mut gradients = vec![0.0; weights.len()];

    crate::apply_forward_crisis_sign_gradient(
        &mut gradients,
        &weights,
        &feature_names,
        100.0,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );

    assert!(gradients[0] < 0.0);
    assert!(gradients[1] > 0.0);
    assert!(gradients[2] < 0.0);
    assert_eq!(gradients[3], 0.0);
    assert!(gradients[4] < 0.0);
}

#[test]
fn forward_crisis_sign_projection_clips_wrong_direction_coefficients() {
    let feature_names = vec![
        "us_baa_10y_spread_level".to_string(),
        "us_curve_10y2y_level".to_string(),
        "structural_score".to_string(),
        "us_usdjpy_change_20d".to_string(),
    ];
    let mut weights = vec![-0.8, 0.5, -0.2, -0.7];

    crate::project_forward_crisis_sign_constraints(
        &mut weights,
        &feature_names,
        60,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );

    assert_eq!(weights[0], 0.0);
    assert_eq!(weights[1], 0.0);
    assert_eq!(weights[2], 0.0);
    assert_eq!(weights[3], -0.7);
}

#[test]
fn forward_crisis_sign_projection_clips_wrong_direction_monotonic_interactions() {
    let feature_names = vec![
        "interaction__overall_score__us_vix_level".to_string(),
        "interaction__us_baa_10y_spread_level__us_vix_level".to_string(),
        "interaction__external_dimension_score__us_usdjpy_level".to_string(),
    ];
    let mut weights = vec![-0.2, -0.6, -0.4];

    crate::project_forward_crisis_sign_constraints(
        &mut weights,
        &feature_names,
        60,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );

    assert_eq!(weights[0], 0.0);
    assert_eq!(weights[1], 0.0);
    assert_eq!(weights[2], 0.0);
}

#[test]
fn forward_crisis_tail_sign_projection_applies_on_20d_only() {
    let feature_names = vec![
        "tail_neg__us_curve_10y2y_level__0".to_string(),
        "tail_pos__us_baa_10y_spread_level__2".to_string(),
    ];
    let mut weights_20d = vec![-0.4, -0.1];
    crate::project_forward_crisis_sign_constraints(
        &mut weights_20d,
        &feature_names,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(weights_20d[0], 0.0);
    assert_eq!(weights_20d[1], 0.0);

    let mut weights_60d = vec![-0.4, -0.1];
    crate::project_forward_crisis_sign_constraints(
        &mut weights_60d,
        &feature_names,
        60,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(weights_60d[0], -0.4);
    assert_eq!(weights_60d[1], -0.1);
}

#[test]
fn forward_crisis_curve_tail_bound_gradient_pushes_too_negative_weight_up() {
    let feature_names = vec!["tail_neg__us_curve_10y2y_level__0".to_string()];
    let weights = vec![-0.30];
    let mut gradients = vec![0.0; weights.len()];

    crate::apply_forward_crisis_coefficient_bound_gradient(
        &mut gradients,
        &weights,
        &feature_names,
        100.0,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );

    assert!(gradients[0] < 0.0);
}

#[test]
fn forward_crisis_rate_shock_family_caps_apply_on_20d_only() {
    let feature_names = vec![
        "family_context__rate_shock__external_dimension_score".to_string(),
        "family_proxy__rate_shock".to_string(),
    ];
    let mut weights_20d = vec![0.32, 0.14];
    crate::project_forward_crisis_sign_constraints(
        &mut weights_20d,
        &feature_names,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(weights_20d[0], 0.12);
    assert_eq!(weights_20d[1], 0.06);

    let mut weights_60d = vec![0.32, 0.14];
    crate::project_forward_crisis_sign_constraints(
        &mut weights_60d,
        &feature_names,
        60,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(weights_60d[0], 0.32);
    assert_eq!(weights_60d[1], 0.14);
}

#[test]
fn forward_crisis_jpy_carry_caps_apply_on_20d_only() {
    let feature_names = vec![
        "family_context__jpy_carry__external_dimension_score".to_string(),
        "family_proxy__jpy_carry".to_string(),
    ];
    let mut weights_20d = vec![0.24, 0.11];
    crate::project_forward_crisis_sign_constraints(
        &mut weights_20d,
        &feature_names,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(weights_20d[0], 0.10);
    assert_eq!(weights_20d[1], 0.06);

    let mut weights_60d = vec![0.24, 0.11];
    crate::project_forward_crisis_sign_constraints(
        &mut weights_60d,
        &feature_names,
        60,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(weights_60d[0], 0.24);
    assert_eq!(weights_60d[1], 0.11);
}

#[test]
fn forward_crisis_curve_family_caps_only_apply_when_family_context_exists() {
    let family_feature_names = vec![
        "us_curve_10y2y_level".to_string(),
        "interaction__us_curve_10y2y_level__us_fed_funds_level".to_string(),
        "family_proxy__rate_shock".to_string(),
    ];
    let mut family_weights_20d = vec![-0.90, 0.60, 0.05];
    crate::project_forward_crisis_sign_constraints(
        &mut family_weights_20d,
        &family_feature_names,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(family_weights_20d[0], -0.72);
    assert_eq!(family_weights_20d[1], 0.46);
    assert_eq!(family_weights_20d[2], 0.05);

    let mut family_weights_60d = vec![-0.90, 0.60, 0.05];
    crate::project_forward_crisis_sign_constraints(
        &mut family_weights_60d,
        &family_feature_names,
        60,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(family_weights_60d[0], -0.90);
    assert_eq!(family_weights_60d[1], 0.60);
    assert_eq!(family_weights_60d[2], 0.05);

    let plain_feature_names = vec![
        "us_curve_10y2y_level".to_string(),
        "interaction__us_curve_10y2y_level__us_fed_funds_level".to_string(),
    ];
    let mut plain_weights_20d = vec![-0.90, 0.60];
    crate::project_forward_crisis_sign_constraints(
        &mut plain_weights_20d,
        &plain_feature_names,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(plain_weights_20d[0], -0.90);
    assert_eq!(plain_weights_20d[1], 0.60);
}

#[test]
fn forward_crisis_usdjpy_level_family_cap_only_applies_when_family_context_exists() {
    let family_feature_names = vec![
        "us_usdjpy_level".to_string(),
        "family_proxy__rate_shock".to_string(),
    ];
    let mut family_weights_20d = vec![0.20, 0.05];
    crate::project_forward_crisis_sign_constraints(
        &mut family_weights_20d,
        &family_feature_names,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(family_weights_20d[0], 0.30);
    assert_eq!(family_weights_20d[1], 0.05);

    let mut family_weights_60d = vec![0.20, 0.05];
    crate::project_forward_crisis_sign_constraints(
        &mut family_weights_60d,
        &family_feature_names,
        60,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(family_weights_60d[0], 0.20);
    assert_eq!(family_weights_60d[1], 0.05);

    let plain_feature_names = vec!["us_usdjpy_level".to_string()];
    let mut plain_weights_20d = vec![0.20];
    crate::project_forward_crisis_sign_constraints(
        &mut plain_weights_20d,
        &plain_feature_names,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(plain_weights_20d[0], 0.20);
}

#[test]
fn forward_crisis_usdjpy_interaction_family_cap_only_applies_when_family_context_exists() {
    let family_feature_names = vec![
        "interaction__external_dimension_score__us_usdjpy_level".to_string(),
        "family_proxy__jpy_carry".to_string(),
    ];
    let mut family_weights_20d = vec![0.72, 0.03];
    crate::project_forward_crisis_sign_constraints(
        &mut family_weights_20d,
        &family_feature_names,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(family_weights_20d[0], 0.58);
    assert_eq!(family_weights_20d[1], 0.03);

    let mut family_weights_60d = vec![0.72, 0.03];
    crate::project_forward_crisis_sign_constraints(
        &mut family_weights_60d,
        &family_feature_names,
        60,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(family_weights_60d[0], 0.72);
    assert_eq!(family_weights_60d[1], 0.03);

    let plain_feature_names =
        vec!["interaction__external_dimension_score__us_usdjpy_level".to_string()];
    let mut plain_weights_20d = vec![0.72];
    crate::project_forward_crisis_sign_constraints(
        &mut plain_weights_20d,
        &plain_feature_names,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    assert_eq!(plain_weights_20d[0], 0.72);
}

#[test]
fn forward_crisis_rate_shock_family_cap_gradient_pushes_excess_weight_down() {
    let feature_names = vec![
        "family_context__rate_shock__external_dimension_score".to_string(),
        "family_proxy__rate_shock".to_string(),
    ];
    let weights = vec![0.30, 0.12];
    let mut gradients = vec![0.0; weights.len()];

    crate::apply_forward_crisis_coefficient_bound_gradient(
        &mut gradients,
        &weights,
        &feature_names,
        100.0,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );

    assert!(gradients[0] > 0.0);
    assert!(gradients[1] > 0.0);
}

#[test]
fn forward_crisis_jpy_carry_family_cap_gradient_pushes_excess_weight_down() {
    let feature_names = vec![
        "family_context__jpy_carry__external_dimension_score".to_string(),
        "family_proxy__jpy_carry".to_string(),
    ];
    let weights = vec![0.22, 0.09];
    let mut gradients = vec![0.0; weights.len()];

    crate::apply_forward_crisis_coefficient_bound_gradient(
        &mut gradients,
        &weights,
        &feature_names,
        100.0,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );

    assert!(gradients[0] > 0.0);
    assert!(gradients[1] > 0.0);
}

#[test]
fn forward_crisis_monotonic_interaction_sign_gradient_pushes_wrong_direction_up() {
    let feature_names = vec![
        "interaction__overall_score__us_vix_level".to_string(),
        "interaction__us_baa_10y_spread_level__us_vix_level".to_string(),
    ];
    let weights = vec![-0.20, -0.60];
    let mut gradients = vec![0.0; weights.len()];

    crate::apply_forward_crisis_sign_gradient(
        &mut gradients,
        &weights,
        &feature_names,
        100.0,
        60,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );

    assert!(gradients[0] < 0.0);
    assert!(gradients[1] < 0.0);
}

#[test]
fn forward_crisis_curve_family_cap_gradient_only_activates_for_family_context_sets() {
    let family_feature_names = vec![
        "us_curve_10y2y_level".to_string(),
        "interaction__us_curve_10y2y_level__us_fed_funds_level".to_string(),
        "family_proxy__rate_shock".to_string(),
    ];
    let family_weights = vec![-0.90, 0.60, 0.05];
    let mut family_gradients = vec![0.0; family_weights.len()];

    crate::apply_forward_crisis_coefficient_bound_gradient(
        &mut family_gradients,
        &family_weights,
        &family_feature_names,
        100.0,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );

    assert!(family_gradients[0] < 0.0);
    assert!(family_gradients[1] > 0.0);
    assert_eq!(family_gradients[2], 0.0);

    let plain_feature_names = vec![
        "us_curve_10y2y_level".to_string(),
        "interaction__us_curve_10y2y_level__us_fed_funds_level".to_string(),
    ];
    let plain_weights = vec![-0.90, 0.60];
    let mut plain_gradients = vec![0.0; plain_weights.len()];

    crate::apply_forward_crisis_coefficient_bound_gradient(
        &mut plain_gradients,
        &plain_weights,
        &plain_feature_names,
        100.0,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );

    assert_eq!(plain_gradients[0], 0.0);
    assert_eq!(plain_gradients[1], 0.0);
}

#[test]
fn forward_crisis_usdjpy_level_family_cap_gradient_only_activates_for_family_context_sets() {
    let family_feature_names = vec![
        "us_usdjpy_level".to_string(),
        "family_proxy__rate_shock".to_string(),
    ];
    let family_weights = vec![0.48, 0.05];
    let mut family_gradients = vec![0.0; family_weights.len()];

    crate::apply_forward_crisis_coefficient_bound_gradient(
        &mut family_gradients,
        &family_weights,
        &family_feature_names,
        100.0,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );

    assert!(family_gradients[0] > 0.0);
    assert_eq!(family_gradients[1], 0.0);

    let plain_feature_names = vec!["us_usdjpy_level".to_string()];
    let plain_weights = vec![0.38];
    let mut plain_gradients = vec![0.0; plain_weights.len()];

    crate::apply_forward_crisis_coefficient_bound_gradient(
        &mut plain_gradients,
        &plain_weights,
        &plain_feature_names,
        100.0,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );

    assert_eq!(plain_gradients[0], 0.0);
}

#[test]
fn forward_crisis_usdjpy_interaction_family_cap_gradient_only_activates_for_family_context_sets() {
    let family_feature_names = vec![
        "interaction__external_dimension_score__us_usdjpy_level".to_string(),
        "family_proxy__jpy_carry".to_string(),
    ];
    let family_weights = vec![0.72, 0.03];
    let mut family_gradients = vec![0.0; family_weights.len()];

    crate::apply_forward_crisis_coefficient_bound_gradient(
        &mut family_gradients,
        &family_weights,
        &family_feature_names,
        100.0,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );

    assert!(family_gradients[0] > 0.0);
    assert_eq!(family_gradients[1], 0.0);

    let plain_feature_names =
        vec!["interaction__external_dimension_score__us_usdjpy_level".to_string()];
    let plain_weights = vec![0.72];
    let mut plain_gradients = vec![0.0; plain_weights.len()];

    crate::apply_forward_crisis_coefficient_bound_gradient(
        &mut plain_gradients,
        &plain_weights,
        &plain_feature_names,
        100.0,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );

    assert_eq!(plain_gradients[0], 0.0);
}

#[test]
fn forward_crisis_training_target_softens_buffer_and_cooldown_negatives() {
    let build_row = |regime_20d: ProbabilityTrainingRegime,
                     regime_60d: ProbabilityTrainingRegime,
                     label_20d: u8,
                     label_60d: u8| ProbabilityTrainingRow {
        as_of_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
        market_scope: "financial_system".to_string(),
        release_id: None,
        probability_mode: Some("formal_bundle_v1".to_string()),
        freshness_status: Some("a".to_string()),
        time_to_risk_bucket: Some("weeks".to_string()),
        split_name: Some("train".to_string()),
        features: BTreeMap::new(),
        primary_scenario_id: Some("scenario".to_string()),
        scenario_family: Some("systemic_credit_banking_crisis".to_string()),
        scenario_training_role: None,
        days_to_primary_crisis_start: Some(25),
        primary_scenario_supports_5d: false,
        primary_scenario_supports_20d: true,
        primary_scenario_supports_60d: true,
        label_5d: 0,
        label_20d,
        label_60d,
        regime_5d: ProbabilityTrainingRegime::Normal,
        regime_20d,
        regime_60d,
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
    };

    let buffer_row = build_row(
        ProbabilityTrainingRegime::PreWarningBuffer,
        ProbabilityTrainingRegime::PreWarningBuffer,
        0,
        0,
    );
    let cooldown_row = build_row(
        ProbabilityTrainingRegime::PostCrisisCooldown,
        ProbabilityTrainingRegime::PostCrisisCooldown,
        0,
        0,
    );
    let positive_row = build_row(
        ProbabilityTrainingRegime::PositiveWindow,
        ProbabilityTrainingRegime::PositiveWindow,
        1,
        1,
    );

    assert_eq!(
        crate::probability_training_target_label(
            &buffer_row,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis
        ),
        0.18
    );
    assert_eq!(
        crate::probability_training_target_label(
            &buffer_row,
            60,
            ProbabilityTargetLabelMode::ForwardCrisis
        ),
        0.26
    );
    assert_eq!(
        crate::probability_training_target_label(
            &cooldown_row,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis
        ),
        0.01
    );
    assert_eq!(
        crate::probability_training_target_label(
            &cooldown_row,
            60,
            ProbabilityTargetLabelMode::ForwardCrisis
        ),
        0.02
    );
    assert_eq!(
        crate::probability_training_target_label(
            &positive_row,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis
        ),
        1.0
    );
}

#[test]
fn forward_crisis_60d_prepare_buffer_uses_episode_native_objective() {
    let build_row = |prepare_episode_label: u8,
                     scenario_training_role: Option<&str>,
                     scenario_family: &str,
                     supports_60d: bool,
                     lead_days: Option<i64>,
                     protected_action_window: bool| {
        ProbabilityTrainingRow {
            as_of_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
            market_scope: "financial_system".to_string(),
            release_id: None,
            probability_mode: Some("formal_bundle_v1".to_string()),
            freshness_status: Some("a".to_string()),
            time_to_risk_bucket: Some("weeks".to_string()),
            split_name: Some("train".to_string()),
            features: BTreeMap::new(),
            primary_scenario_id: Some("scenario".to_string()),
            scenario_family: Some(scenario_family.to_string()),
            scenario_training_role: scenario_training_role.map(str::to_string),
            days_to_primary_crisis_start: lead_days,
            primary_scenario_supports_5d: false,
            primary_scenario_supports_20d: true,
            primary_scenario_supports_60d: supports_60d,
            label_5d: 0,
            label_20d: 0,
            label_60d: 0,
            regime_5d: ProbabilityTrainingRegime::Normal,
            regime_20d: ProbabilityTrainingRegime::Normal,
            regime_60d: ProbabilityTrainingRegime::PreWarningBuffer,
            action_label_5d: 0,
            action_label_20d: 0,
            action_label_60d: prepare_episode_label,
            prepare_episode_label,
            hedge_episode_label: 0,
            defend_episode_label: 0,
            primary_action_level: (prepare_episode_label > 0).then_some("prepare".to_string()),
            action_episode_id: (prepare_episode_label > 0)
                .then_some("scenario:prepare".to_string()),
            action_episode_phase: if prepare_episode_label > 0 {
                "primary".to_string()
            } else {
                "outside".to_string()
            },
            protected_action_window,
        }
    };

    let mandatory_prepare = build_row(
        1,
        Some("mandatory"),
        "systemic_credit_banking_crisis",
        true,
        Some(75),
        false,
    );
    assert_eq!(
        crate::probability_training_target_label(
            &mandatory_prepare,
            60,
            ProbabilityTargetLabelMode::ForwardCrisis
        ),
        0.64
    );
    assert_eq!(
        negative_sample_weight(
            &mandatory_prepare,
            60,
            ProbabilityTargetLabelMode::ForwardCrisis
        ),
        1.35
    );

    let protected_extension = build_row(
        1,
        Some("extension_only"),
        "mixed_systemic_stress",
        true,
        Some(82),
        true,
    );
    assert_eq!(
        crate::probability_training_target_label(
            &protected_extension,
            60,
            ProbabilityTargetLabelMode::ForwardCrisis
        ),
        0.58
    );
    assert_eq!(
        negative_sample_weight(
            &protected_extension,
            60,
            ProbabilityTargetLabelMode::ForwardCrisis
        ),
        1.10
    );

    let outside_prepare = build_row(
        0,
        Some("mandatory"),
        "systemic_credit_banking_crisis",
        true,
        Some(75),
        false,
    );
    assert_eq!(
        crate::probability_training_target_label(
            &outside_prepare,
            60,
            ProbabilityTargetLabelMode::ForwardCrisis
        ),
        0.26
    );

    let acute_prepare = build_row(
        1,
        Some("mandatory"),
        "acute_market_liquidity_crash",
        true,
        Some(75),
        false,
    );
    assert_eq!(
        crate::probability_training_target_label(
            &acute_prepare,
            60,
            ProbabilityTargetLabelMode::ForwardCrisis
        ),
        0.26
    );

    let unsupported_prepare = build_row(
        1,
        Some("mandatory"),
        "systemic_credit_banking_crisis",
        false,
        Some(75),
        false,
    );
    assert_eq!(
        crate::probability_training_target_label(
            &unsupported_prepare,
            60,
            ProbabilityTargetLabelMode::ForwardCrisis
        ),
        0.26
    );
}

#[test]
fn positive_sample_action_weight_prefers_early_role_aligned_systemic_samples_for_60d() {
    let aligned = ProbabilityTrainingRow {
        as_of_date: NaiveDate::from_ymd_opt(2007, 6, 5).unwrap(),
        market_scope: "financial_system".to_string(),
        release_id: None,
        probability_mode: Some("formal_bundle_v1".to_string()),
        freshness_status: Some("a".to_string()),
        time_to_risk_bucket: Some("us_gfc_2008".to_string()),
        split_name: Some("train".to_string()),
        features: BTreeMap::new(),
        primary_scenario_id: Some("us_gfc_2008".to_string()),
        scenario_family: Some("systemic_credit_banking_crisis".to_string()),
        scenario_training_role: Some("mandatory".to_string()),
        days_to_primary_crisis_start: Some(57),
        primary_scenario_supports_5d: false,
        primary_scenario_supports_20d: true,
        primary_scenario_supports_60d: true,
        label_5d: 0,
        label_20d: 0,
        label_60d: 1,
        regime_5d: ProbabilityTrainingRegime::Normal,
        regime_20d: ProbabilityTrainingRegime::PreWarningBuffer,
        regime_60d: ProbabilityTrainingRegime::PositiveWindow,
        action_label_5d: 0,
        action_label_20d: 1,
        action_label_60d: 1,
        prepare_episode_label: 1,
        hedge_episode_label: 1,
        defend_episode_label: 0,
        primary_action_level: Some("hedge".to_string()),
        action_episode_id: Some("us_gfc_2008:hedge".to_string()),
        action_episode_phase: "primary".to_string(),
        protected_action_window: false,
    };
    let misaligned = ProbabilityTrainingRow {
        as_of_date: NaiveDate::from_ymd_opt(2020, 2, 20).unwrap(),
        market_scope: "financial_system".to_string(),
        release_id: None,
        probability_mode: Some("formal_bundle_v1".to_string()),
        freshness_status: Some("a".to_string()),
        time_to_risk_bucket: Some("us_covid_liquidity_2020".to_string()),
        split_name: Some("train".to_string()),
        features: BTreeMap::new(),
        primary_scenario_id: Some("us_covid_liquidity_2020".to_string()),
        scenario_family: Some("acute_market_liquidity_crash".to_string()),
        scenario_training_role: Some("mandatory".to_string()),
        days_to_primary_crisis_start: Some(4),
        primary_scenario_supports_5d: true,
        primary_scenario_supports_20d: true,
        primary_scenario_supports_60d: false,
        label_5d: 1,
        label_20d: 1,
        label_60d: 1,
        regime_5d: ProbabilityTrainingRegime::PositiveWindow,
        regime_20d: ProbabilityTrainingRegime::PositiveWindow,
        regime_60d: ProbabilityTrainingRegime::PositiveWindow,
        action_label_5d: 1,
        action_label_20d: 1,
        action_label_60d: 0,
        prepare_episode_label: 0,
        hedge_episode_label: 1,
        defend_episode_label: 1,
        primary_action_level: Some("defend".to_string()),
        action_episode_id: Some("us_covid_liquidity_2020:defend".to_string()),
        action_episode_phase: "primary".to_string(),
        protected_action_window: false,
    };

    assert!(
        positive_sample_action_weight(&aligned, 60)
            > positive_sample_action_weight(&misaligned, 60)
    );
}

#[test]
fn forward_crisis_positive_weight_boosts_extension_role_on_supported_horizon() {
    let mandatory = ProbabilityTrainingRow {
        as_of_date: NaiveDate::from_ymd_opt(2007, 6, 5).unwrap(),
        market_scope: "financial_system".to_string(),
        release_id: None,
        probability_mode: Some("formal_bundle_v1".to_string()),
        freshness_status: Some("a".to_string()),
        time_to_risk_bucket: Some("us_gfc_2008".to_string()),
        split_name: Some("train".to_string()),
        features: BTreeMap::new(),
        primary_scenario_id: Some("us_gfc_2008".to_string()),
        scenario_family: Some("systemic_credit_banking_crisis".to_string()),
        scenario_training_role: Some("mandatory".to_string()),
        days_to_primary_crisis_start: Some(48),
        primary_scenario_supports_5d: false,
        primary_scenario_supports_20d: true,
        primary_scenario_supports_60d: true,
        label_5d: 0,
        label_20d: 0,
        label_60d: 1,
        regime_5d: ProbabilityTrainingRegime::Normal,
        regime_20d: ProbabilityTrainingRegime::PreWarningBuffer,
        regime_60d: ProbabilityTrainingRegime::PositiveWindow,
        action_label_5d: 0,
        action_label_20d: 1,
        action_label_60d: 1,
        prepare_episode_label: 1,
        hedge_episode_label: 1,
        defend_episode_label: 0,
        primary_action_level: Some("hedge".to_string()),
        action_episode_id: Some("us_gfc_2008:hedge".to_string()),
        action_episode_phase: "primary".to_string(),
        protected_action_window: false,
    };
    let extension = ProbabilityTrainingRow {
        as_of_date: NaiveDate::from_ymd_opt(2011, 6, 20).unwrap(),
        market_scope: "financial_system".to_string(),
        release_id: None,
        probability_mode: Some("formal_bundle_v1".to_string()),
        freshness_status: Some("a".to_string()),
        time_to_risk_bucket: Some("us_funding_stress_2011".to_string()),
        split_name: Some("train".to_string()),
        features: BTreeMap::new(),
        primary_scenario_id: Some("us_funding_stress_2011".to_string()),
        scenario_family: Some("mixed_systemic_stress".to_string()),
        scenario_training_role: Some("extension_only".to_string()),
        days_to_primary_crisis_start: Some(39),
        primary_scenario_supports_5d: false,
        primary_scenario_supports_20d: true,
        primary_scenario_supports_60d: true,
        label_5d: 0,
        label_20d: 0,
        label_60d: 1,
        regime_5d: ProbabilityTrainingRegime::Normal,
        regime_20d: ProbabilityTrainingRegime::PreWarningBuffer,
        regime_60d: ProbabilityTrainingRegime::PositiveWindow,
        action_label_5d: 0,
        action_label_20d: 1,
        action_label_60d: 1,
        prepare_episode_label: 1,
        hedge_episode_label: 1,
        defend_episode_label: 0,
        primary_action_level: Some("hedge".to_string()),
        action_episode_id: Some("us_funding_stress_2011:hedge".to_string()),
        action_episode_phase: "primary".to_string(),
        protected_action_window: true,
    };

    assert!(
        crate::forward_crisis_positive_sample_weight(&extension, 60)
            > crate::forward_crisis_positive_sample_weight(&mandatory, 60)
    );
}

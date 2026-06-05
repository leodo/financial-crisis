use super::*;

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

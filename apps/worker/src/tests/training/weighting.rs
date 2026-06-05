use super::*;

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

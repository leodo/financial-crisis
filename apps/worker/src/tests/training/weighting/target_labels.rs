use super::*;

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

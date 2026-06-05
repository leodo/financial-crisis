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

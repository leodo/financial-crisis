use super::*;

fn threshold_regime_row(
    regime_20d: ProbabilityTrainingRegime,
    label_20d: u8,
) -> ProbabilityTrainingRow {
    ProbabilityTrainingRow {
        as_of_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
        market_scope: "financial_system".to_string(),
        release_id: None,
        probability_mode: Some("formal_bundle_v1".to_string()),
        freshness_status: Some("fresh".to_string()),
        time_to_risk_bucket: Some("test".to_string()),
        split_name: Some("calibration".to_string()),
        features: BTreeMap::new(),
        primary_scenario_id: Some("scenario".to_string()),
        scenario_family: Some("mixed_systemic_stress".to_string()),
        scenario_training_role: None,
        days_to_primary_crisis_start: Some(15),
        primary_scenario_supports_5d: true,
        primary_scenario_supports_20d: true,
        primary_scenario_supports_60d: true,
        label_5d: 0,
        label_20d,
        label_60d: 0,
        regime_5d: ProbabilityTrainingRegime::Normal,
        regime_20d,
        regime_60d: ProbabilityTrainingRegime::Normal,
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
    }
}

#[test]
fn regime_support_adjustment_rejects_prewarning_only_20d_threshold() {
    let rows = vec![
        threshold_regime_row(ProbabilityTrainingRegime::PositiveWindow, 1),
        threshold_regime_row(ProbabilityTrainingRegime::PositiveWindow, 1),
        threshold_regime_row(ProbabilityTrainingRegime::PositiveWindow, 1),
        threshold_regime_row(ProbabilityTrainingRegime::PositiveWindow, 1),
        threshold_regime_row(ProbabilityTrainingRegime::InCrisis, 1),
        threshold_regime_row(ProbabilityTrainingRegime::InCrisis, 1),
        threshold_regime_row(ProbabilityTrainingRegime::PreWarningBuffer, 0),
        threshold_regime_row(ProbabilityTrainingRegime::PreWarningBuffer, 0),
        threshold_regime_row(ProbabilityTrainingRegime::PreWarningBuffer, 0),
        threshold_regime_row(ProbabilityTrainingRegime::PreWarningBuffer, 0),
        threshold_regime_row(ProbabilityTrainingRegime::Normal, 0),
        threshold_regime_row(ProbabilityTrainingRegime::Normal, 0),
        threshold_regime_row(ProbabilityTrainingRegime::Normal, 0),
        threshold_regime_row(ProbabilityTrainingRegime::PostCrisisCooldown, 0),
    ];
    let row_refs = rows.iter().collect::<Vec<_>>();
    let probabilities = vec![
        0.62, 0.61, 0.60, 0.59, 0.95, 0.94, 0.93, 0.92, 0.91, 0.90, 0.20, 0.18,
        0.16, 0.30,
    ];
    let labels = rows
        .iter()
        .map(|row| row.label_20d as f64)
        .collect::<Vec<_>>();
    let base_threshold = 0.90;
    let adjusted_threshold = adjust_probability_decision_threshold_for_regime_support(
        base_threshold,
        &probabilities,
        &labels,
        &row_refs,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    let positive_window_hit_count = |threshold: f64| {
        probabilities
            .iter()
            .zip(row_refs.iter())
            .filter(|(probability, row)| {
                **probability >= threshold
                    && row.regime_20d == ProbabilityTrainingRegime::PositiveWindow
            })
            .count()
    };
    let cooldown_hit_count = |threshold: f64| {
        probabilities
            .iter()
            .zip(row_refs.iter())
            .filter(|(probability, row)| {
                **probability >= threshold
                    && row.regime_20d == ProbabilityTrainingRegime::PostCrisisCooldown
            })
            .count()
    };

    assert_eq!(positive_window_hit_count(base_threshold), 0);
    assert!(adjusted_threshold < base_threshold);
    assert!(adjusted_threshold <= 0.62);
    assert!(positive_window_hit_count(adjusted_threshold) > 0);
    assert_eq!(cooldown_hit_count(adjusted_threshold), 0);
}

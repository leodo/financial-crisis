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

fn threshold_regime_row_5d(
    regime_5d: ProbabilityTrainingRegime,
    label_5d: u8,
) -> ProbabilityTrainingRow {
    ProbabilityTrainingRow {
        regime_5d,
        label_5d,
        primary_scenario_supports_20d: false,
        primary_scenario_supports_60d: false,
        days_to_primary_crisis_start: Some(3),
        ..threshold_regime_row(ProbabilityTrainingRegime::Normal, 0)
    }
}

fn threshold_regime_row_60d(
    regime_60d: ProbabilityTrainingRegime,
    label_60d: u8,
) -> ProbabilityTrainingRow {
    ProbabilityTrainingRow {
        regime_60d,
        label_60d,
        primary_scenario_supports_20d: false,
        days_to_primary_crisis_start: Some(45),
        ..threshold_regime_row(ProbabilityTrainingRegime::Normal, 0)
    }
}

#[test]
fn regime_support_adjustment_rejects_weak_5d_threshold_with_normal_bleed() {
    let rows = vec![
        threshold_regime_row_5d(ProbabilityTrainingRegime::PositiveWindow, 1),
        threshold_regime_row_5d(ProbabilityTrainingRegime::PositiveWindow, 1),
        threshold_regime_row_5d(ProbabilityTrainingRegime::Normal, 0),
        threshold_regime_row_5d(ProbabilityTrainingRegime::Normal, 0),
        threshold_regime_row_5d(ProbabilityTrainingRegime::PostCrisisCooldown, 0),
        threshold_regime_row_5d(ProbabilityTrainingRegime::PostCrisisCooldown, 0),
    ];
    let row_refs = rows.iter().collect::<Vec<_>>();
    let probabilities = vec![0.06, 0.07, 0.18, 0.17, 0.16, 0.15];
    let labels = rows
        .iter()
        .map(|row| row.label_5d as f64)
        .collect::<Vec<_>>();
    let adjusted_threshold = adjust_probability_decision_threshold_for_regime_support(
        0.05,
        &probabilities,
        &labels,
        &row_refs,
        5,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );

    // threshold repair no longer pushes weak-signal horizons to 0.99;
    // falls back to base_threshold instead.
    assert_eq!(adjusted_threshold, 0.05);
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
        0.62, 0.61, 0.60, 0.59, 0.95, 0.94, 0.93, 0.92, 0.91, 0.90, 0.20, 0.18, 0.16, 0.30,
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

#[test]
fn regime_support_adjustment_uses_soft_forward_crisis_targets_for_20d_repair() {
    let rows = vec![
        threshold_regime_row(ProbabilityTrainingRegime::PositiveWindow, 0),
        threshold_regime_row(ProbabilityTrainingRegime::PositiveWindow, 0),
        threshold_regime_row(ProbabilityTrainingRegime::PositiveWindow, 0),
        threshold_regime_row(ProbabilityTrainingRegime::PositiveWindow, 0),
        threshold_regime_row(ProbabilityTrainingRegime::PreWarningBuffer, 0),
        threshold_regime_row(ProbabilityTrainingRegime::PreWarningBuffer, 0),
        threshold_regime_row(ProbabilityTrainingRegime::PreWarningBuffer, 0),
        threshold_regime_row(ProbabilityTrainingRegime::Normal, 0),
        threshold_regime_row(ProbabilityTrainingRegime::Normal, 0),
        threshold_regime_row(ProbabilityTrainingRegime::PostCrisisCooldown, 0),
    ];
    let row_refs = rows.iter().collect::<Vec<_>>();
    let probabilities = vec![0.64, 0.62, 0.60, 0.58, 0.56, 0.54, 0.52, 0.18, 0.16, 0.12];
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

    assert!(labels.iter().all(|label| *label == 0.0));
    assert!(adjusted_threshold < base_threshold);
    assert!(adjusted_threshold <= 0.56);
}

#[test]
fn regime_support_adjustment_rejects_soft_20d_repair_when_cooldown_bleeds() {
    let rows = vec![
        threshold_regime_row(ProbabilityTrainingRegime::PositiveWindow, 0),
        threshold_regime_row(ProbabilityTrainingRegime::PositiveWindow, 0),
        threshold_regime_row(ProbabilityTrainingRegime::PositiveWindow, 0),
        threshold_regime_row(ProbabilityTrainingRegime::PositiveWindow, 0),
        threshold_regime_row(ProbabilityTrainingRegime::PreWarningBuffer, 0),
        threshold_regime_row(ProbabilityTrainingRegime::PreWarningBuffer, 0),
        threshold_regime_row(ProbabilityTrainingRegime::PreWarningBuffer, 0),
        threshold_regime_row(ProbabilityTrainingRegime::Normal, 0),
        threshold_regime_row(ProbabilityTrainingRegime::Normal, 0),
        threshold_regime_row(ProbabilityTrainingRegime::PostCrisisCooldown, 0),
        threshold_regime_row(ProbabilityTrainingRegime::PostCrisisCooldown, 0),
    ];
    let row_refs = rows.iter().collect::<Vec<_>>();
    let probabilities = vec![
        0.64, 0.62, 0.60, 0.58, 0.56, 0.54, 0.52, 0.18, 0.16, 0.62, 0.60,
    ];
    let labels = rows
        .iter()
        .map(|row| row.label_20d as f64)
        .collect::<Vec<_>>();
    let adjusted_threshold = adjust_probability_decision_threshold_for_regime_support(
        0.90,
        &probabilities,
        &labels,
        &row_refs,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );

    // Cooldown bleed rejected: base_threshold (0.90) is preserved
    // instead of pushing to 0.99.
    assert_eq!(adjusted_threshold, 0.90);
}

#[test]
fn threshold_diagnostics_count_accepted_soft_repair_candidates() {
    let rows = vec![
        threshold_regime_row(ProbabilityTrainingRegime::PositiveWindow, 0),
        threshold_regime_row(ProbabilityTrainingRegime::PositiveWindow, 0),
        threshold_regime_row(ProbabilityTrainingRegime::PositiveWindow, 0),
        threshold_regime_row(ProbabilityTrainingRegime::PositiveWindow, 0),
        threshold_regime_row(ProbabilityTrainingRegime::PreWarningBuffer, 0),
        threshold_regime_row(ProbabilityTrainingRegime::PreWarningBuffer, 0),
        threshold_regime_row(ProbabilityTrainingRegime::PreWarningBuffer, 0),
        threshold_regime_row(ProbabilityTrainingRegime::Normal, 0),
        threshold_regime_row(ProbabilityTrainingRegime::Normal, 0),
        threshold_regime_row(ProbabilityTrainingRegime::PostCrisisCooldown, 0),
    ];
    let row_refs = rows.iter().collect::<Vec<_>>();
    let probabilities = vec![0.64, 0.62, 0.60, 0.58, 0.56, 0.54, 0.52, 0.18, 0.16, 0.12];
    let labels = rows
        .iter()
        .map(|row| row.label_20d as f64)
        .collect::<Vec<_>>();
    let calibration_selection = ProbabilityCalibrationSelection {
        rows: row_refs.clone(),
        eligible_row_count: row_refs.len(),
        eligible_positive_count: labels.iter().filter(|label| **label >= 0.5).count(),
        eligible_negative_count: labels.iter().filter(|label| **label < 0.5).count(),
        used_full_split_fallback: false,
    };
    let threshold_selection = ProbabilityThresholdSelection {
        rows: row_refs,
        probabilities,
        labels,
        used_full_split_fallback: false,
    };
    let diagnostics =
        build_probability_threshold_diagnostics(ProbabilityThresholdDiagnosticsInput {
            full_calibration_rows: &rows,
            calibration_selection: &calibration_selection,
            threshold_selection: &threshold_selection,
            horizon_days: 20,
            label_mode: ProbabilityTargetLabelMode::ForwardCrisis,
            base_threshold: 0.90,
            final_threshold: 0.56,
        });

    let repair_candidates = diagnostics
        .repair_candidate_diagnostics
        .expect("repair candidate diagnostics");
    assert!(repair_candidates.candidate_count > 0);
    assert!(repair_candidates.accepted_candidate_count > 0);
}

#[test]
fn threshold_diagnostics_explain_cooldown_bleed_repair_rejections() {
    let rows = vec![
        threshold_regime_row(ProbabilityTrainingRegime::PositiveWindow, 0),
        threshold_regime_row(ProbabilityTrainingRegime::PositiveWindow, 0),
        threshold_regime_row(ProbabilityTrainingRegime::PositiveWindow, 0),
        threshold_regime_row(ProbabilityTrainingRegime::PositiveWindow, 0),
        threshold_regime_row(ProbabilityTrainingRegime::PreWarningBuffer, 0),
        threshold_regime_row(ProbabilityTrainingRegime::PreWarningBuffer, 0),
        threshold_regime_row(ProbabilityTrainingRegime::PreWarningBuffer, 0),
        threshold_regime_row(ProbabilityTrainingRegime::Normal, 0),
        threshold_regime_row(ProbabilityTrainingRegime::Normal, 0),
        threshold_regime_row(ProbabilityTrainingRegime::PostCrisisCooldown, 0),
        threshold_regime_row(ProbabilityTrainingRegime::PostCrisisCooldown, 0),
    ];
    let row_refs = rows.iter().collect::<Vec<_>>();
    let probabilities = vec![
        0.64, 0.62, 0.60, 0.58, 0.56, 0.54, 0.52, 0.18, 0.16, 0.64, 0.60,
    ];
    let labels = rows
        .iter()
        .map(|row| row.label_20d as f64)
        .collect::<Vec<_>>();
    let calibration_selection = ProbabilityCalibrationSelection {
        rows: row_refs.clone(),
        eligible_row_count: row_refs.len(),
        eligible_positive_count: labels.iter().filter(|label| **label >= 0.5).count(),
        eligible_negative_count: labels.iter().filter(|label| **label < 0.5).count(),
        used_full_split_fallback: false,
    };
    let threshold_selection = ProbabilityThresholdSelection {
        rows: row_refs,
        probabilities,
        labels,
        used_full_split_fallback: false,
    };
    let diagnostics =
        build_probability_threshold_diagnostics(ProbabilityThresholdDiagnosticsInput {
            full_calibration_rows: &rows,
            calibration_selection: &calibration_selection,
            threshold_selection: &threshold_selection,
            horizon_days: 20,
            label_mode: ProbabilityTargetLabelMode::ForwardCrisis,
            base_threshold: 0.90,
            final_threshold: 0.90,
        });

    let repair_candidates = diagnostics
        .repair_candidate_diagnostics
        .expect("repair candidate diagnostics");
    assert!(repair_candidates.candidate_count > 0);
    assert_eq!(repair_candidates.accepted_candidate_count, 0);
    assert!(repair_candidates.rejected_regime_support_count > 0);
    assert_eq!(
        repair_candidates.best_rejected_reason,
        "regime_support_rejected"
    );
    assert!(
        repair_candidates.best_rejected_cooldown_hit_rate
            >= repair_candidates.best_rejected_positive_window_hit_rate
    );
}

#[test]
fn regime_support_adjustment_keeps_20d_threshold_actionable_when_prewarning_separates() {
    let mut rows = Vec::new();
    let mut probabilities = Vec::new();
    for _ in 0..44 {
        rows.push(threshold_regime_row(
            ProbabilityTrainingRegime::PositiveWindow,
            1,
        ));
        probabilities.push(0.34);
    }
    for index in 0..71 {
        rows.push(threshold_regime_row(
            ProbabilityTrainingRegime::PreWarningBuffer,
            0,
        ));
        probabilities.push(if index < 9 { 0.349 } else { 0.28 });
    }
    for index in 0..8166 {
        rows.push(threshold_regime_row(ProbabilityTrainingRegime::Normal, 0));
        probabilities.push(if index < 40 { 0.349 } else { 0.18 });
    }
    for index in 0..112 {
        rows.push(threshold_regime_row(
            ProbabilityTrainingRegime::PostCrisisCooldown,
            0,
        ));
        probabilities.push(if index < 4 { 0.349 } else { 0.16 });
    }

    let row_refs = rows.iter().collect::<Vec<_>>();
    let labels = rows
        .iter()
        .map(|row| row.label_20d as f64)
        .collect::<Vec<_>>();
    let adjusted_threshold = adjust_probability_decision_threshold_for_regime_support(
        0.36,
        &probabilities,
        &labels,
        &row_refs,
        20,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    let prewarning_hit_count = probabilities
        .iter()
        .zip(row_refs.iter())
        .filter(|(probability, row)| {
            **probability >= adjusted_threshold
                && row.regime_20d == ProbabilityTrainingRegime::PreWarningBuffer
        })
        .count();
    let normal_hit_rate = probabilities
        .iter()
        .zip(row_refs.iter())
        .filter(|(_, row)| row.regime_20d == ProbabilityTrainingRegime::Normal)
        .filter(|(probability, _)| **probability >= adjusted_threshold)
        .count() as f64
        / 8166.0;

    assert!(adjusted_threshold < 0.36);
    assert!(adjusted_threshold <= 0.349);
    assert!(prewarning_hit_count > 0);
    assert!(normal_hit_rate < 0.01);
}

#[test]
fn regime_support_adjustment_rejects_60d_threshold_that_ties_cooldown_hits() {
    let rows = vec![
        threshold_regime_row_60d(ProbabilityTrainingRegime::PositiveWindow, 1),
        threshold_regime_row_60d(ProbabilityTrainingRegime::PositiveWindow, 1),
        threshold_regime_row_60d(ProbabilityTrainingRegime::PreWarningBuffer, 0),
        threshold_regime_row_60d(ProbabilityTrainingRegime::PreWarningBuffer, 0),
        threshold_regime_row_60d(ProbabilityTrainingRegime::Normal, 0),
        threshold_regime_row_60d(ProbabilityTrainingRegime::Normal, 0),
        threshold_regime_row_60d(ProbabilityTrainingRegime::PostCrisisCooldown, 0),
        threshold_regime_row_60d(ProbabilityTrainingRegime::PostCrisisCooldown, 0),
    ];
    let row_refs = rows.iter().collect::<Vec<_>>();
    let probabilities = vec![0.58, 0.56, 0.90, 0.89, 0.20, 0.18, 0.58, 0.56];
    let labels = rows
        .iter()
        .map(|row| row.label_60d as f64)
        .collect::<Vec<_>>();
    let base_threshold = 0.90;
    let adjusted_threshold = adjust_probability_decision_threshold_for_regime_support(
        base_threshold,
        &probabilities,
        &labels,
        &row_refs,
        60,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );

    // Cooldown-tied 60d threshold: base_threshold (0.90) is preserved
    // instead of pushing to 0.99.
    assert_eq!(adjusted_threshold, 0.90);
}

#[test]
fn regime_support_adjustment_raises_60d_threshold_to_suppress_cooldown_bleed() {
    let rows = vec![
        threshold_regime_row_60d(ProbabilityTrainingRegime::PositiveWindow, 1),
        threshold_regime_row_60d(ProbabilityTrainingRegime::PositiveWindow, 1),
        threshold_regime_row_60d(ProbabilityTrainingRegime::PositiveWindow, 1),
        threshold_regime_row_60d(ProbabilityTrainingRegime::PositiveWindow, 1),
        threshold_regime_row_60d(ProbabilityTrainingRegime::PreWarningBuffer, 0),
        threshold_regime_row_60d(ProbabilityTrainingRegime::PreWarningBuffer, 0),
        threshold_regime_row_60d(ProbabilityTrainingRegime::Normal, 0),
        threshold_regime_row_60d(ProbabilityTrainingRegime::Normal, 0),
        threshold_regime_row_60d(ProbabilityTrainingRegime::PostCrisisCooldown, 0),
        threshold_regime_row_60d(ProbabilityTrainingRegime::PostCrisisCooldown, 0),
    ];
    let row_refs = rows.iter().collect::<Vec<_>>();
    let probabilities = vec![0.92, 0.91, 0.30, 0.28, 0.94, 0.93, 0.20, 0.18, 0.89, 0.88];
    let labels = rows
        .iter()
        .map(|row| row.label_60d as f64)
        .collect::<Vec<_>>();
    let base_threshold = 0.50;
    let adjusted_threshold = adjust_probability_decision_threshold_for_regime_support(
        base_threshold,
        &probabilities,
        &labels,
        &row_refs,
        60,
        ProbabilityTargetLabelMode::ForwardCrisis,
    );
    let regime_hit_count = |threshold: f64, regime: ProbabilityTrainingRegime| {
        probabilities
            .iter()
            .zip(row_refs.iter())
            .filter(|(probability, row)| **probability >= threshold && row.regime_60d == regime)
            .count()
    };

    assert_eq!(adjusted_threshold, 0.90);
    assert!(
        regime_hit_count(
            adjusted_threshold,
            ProbabilityTrainingRegime::PreWarningBuffer
        ) > 0
    );
    assert!(
        regime_hit_count(
            adjusted_threshold,
            ProbabilityTrainingRegime::PositiveWindow
        ) > 0
    );
    assert_eq!(
        regime_hit_count(
            adjusted_threshold,
            ProbabilityTrainingRegime::PostCrisisCooldown
        ),
        0
    );
}

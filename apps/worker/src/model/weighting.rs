use crate::{ProbabilityTargetLabelMode, ProbabilityTrainingRegime, ProbabilityTrainingRow};

pub(super) fn horizon_positive_class_weight(
    rows: &[ProbabilityTrainingRow],
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) -> f64 {
    let positive_units = match label_mode {
        ProbabilityTargetLabelMode::ForwardCrisis => rows
            .iter()
            .filter(|row| row.label_for_horizon(label_mode, horizon_days) > 0.0)
            .map(|row| forward_crisis_positive_sample_weight(row, horizon_days))
            .sum::<f64>(),
        _ => rows
            .iter()
            .filter(|row| row.label_for_horizon(label_mode, horizon_days) > 0.0)
            .count() as f64,
    };
    let negative_weight = rows
        .iter()
        .filter(|row| row.label_for_horizon(label_mode, horizon_days) <= 0.0)
        .map(|row| negative_sample_weight(row, horizon_days, label_mode))
        .sum::<f64>();
    if positive_units <= 0.0 || negative_weight <= 0.0 {
        return 1.0;
    }

    let imbalance_weight = (negative_weight / positive_units).sqrt();
    let (horizon_emphasis, cap) = match label_mode {
        ProbabilityTargetLabelMode::ActionWindow | ProbabilityTargetLabelMode::ActionEpisode => {
            match horizon_days {
                5 => (0.65, 6.0),
                20 => (0.75, 7.0),
                60 => (0.85, 8.0),
                _ => (0.75, 7.0),
            }
        }
        ProbabilityTargetLabelMode::ForwardCrisis => match horizon_days {
            5 => (0.9, 18.0),
            20 => (1.15, 18.0),
            60 => (1.35, 18.0),
            _ => (1.0, 18.0),
        },
    };
    (imbalance_weight * horizon_emphasis).clamp(1.0, cap)
}

pub(crate) fn probability_training_target_label(
    row: &ProbabilityTrainingRow,
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) -> f64 {
    let hard_label = row.label_for_horizon(label_mode, horizon_days);
    if hard_label > 0.0 || label_mode != ProbabilityTargetLabelMode::ForwardCrisis {
        return hard_label;
    }

    if let Some(objective) = forward_crisis_episode_native_objective(row, horizon_days) {
        return objective.target_label;
    }

    match row.regime_for_horizon(horizon_days) {
        ProbabilityTrainingRegime::Normal => 0.0,
        ProbabilityTrainingRegime::PreWarningBuffer => match horizon_days {
            20 => 0.18,
            60 => 0.26,
            _ => 0.0,
        },
        ProbabilityTrainingRegime::PositiveWindow => match horizon_days {
            20 => 0.24,
            60 => 0.32,
            _ => 0.0,
        },
        ProbabilityTrainingRegime::InCrisis => match horizon_days {
            20 => 0.08,
            60 => 0.12,
            _ => 0.0,
        },
        ProbabilityTrainingRegime::PostCrisisCooldown => match horizon_days {
            20 => 0.01,
            60 => 0.02,
            _ => 0.0,
        },
    }
}

#[derive(Debug, Clone, Copy)]
struct ForwardCrisisPreparePrewarningObjective {
    target_label: f64,
    objective_weight: f64,
}

fn forward_crisis_episode_native_objective(
    row: &ProbabilityTrainingRow,
    horizon_days: u32,
) -> Option<ForwardCrisisPreparePrewarningObjective> {
    match horizon_days {
        20 => forward_crisis_hedge_prewarning_objective(row),
        60 => forward_crisis_prepare_prewarning_objective(row),
        _ => None,
    }
}

pub(crate) fn forward_crisis_has_episode_native_objective(
    row: &ProbabilityTrainingRow,
    horizon_days: u32,
) -> bool {
    forward_crisis_episode_native_objective(row, horizon_days).is_some()
}

pub(crate) fn forward_crisis_is_protected_no_positive_main_episode_row(
    row: &ProbabilityTrainingRow,
    horizon_days: u32,
) -> bool {
    forward_crisis_protected_no_positive_main_objective(row, horizon_days).is_some()
}

fn forward_crisis_prepare_prewarning_objective(
    row: &ProbabilityTrainingRow,
) -> Option<ForwardCrisisPreparePrewarningObjective> {
    let horizon_days = 60;
    if horizon_days != 60 {
        return None;
    }
    if row.regime_for_horizon(horizon_days) != ProbabilityTrainingRegime::PreWarningBuffer {
        return None;
    }
    if row.label_for_horizon(ProbabilityTargetLabelMode::ForwardCrisis, horizon_days) > 0.0 {
        return None;
    }
    if row.primary_scenario_supports_horizon(horizon_days) != Some(true) {
        return None;
    }
    if row
        .days_to_primary_crisis_start
        .is_none_or(|lead_days| lead_days <= 0)
    {
        return None;
    }
    if !is_prepare_episode_row(row) {
        return None;
    }
    if matches!(
        row.scenario_family.as_deref(),
        Some("acute_market_liquidity_crash")
    ) {
        return None;
    }

    if let Some(objective) = forward_crisis_protected_no_positive_main_objective(row, horizon_days)
    {
        return Some(objective);
    }

    let extension_or_protected = row.protected_action_window
        || matches!(
            row.scenario_training_role.as_deref(),
            Some("extension_only")
        );
    Some(ForwardCrisisPreparePrewarningObjective {
        target_label: if extension_or_protected { 0.58 } else { 0.64 },
        objective_weight: if extension_or_protected { 1.10 } else { 1.35 },
    })
}

fn forward_crisis_hedge_prewarning_objective(
    row: &ProbabilityTrainingRow,
) -> Option<ForwardCrisisPreparePrewarningObjective> {
    let horizon_days = 20;
    if row.regime_for_horizon(horizon_days) != ProbabilityTrainingRegime::PreWarningBuffer {
        return None;
    }
    if row.label_for_horizon(ProbabilityTargetLabelMode::ForwardCrisis, horizon_days) > 0.0 {
        return None;
    }
    if row.primary_scenario_supports_horizon(horizon_days) != Some(true) {
        return None;
    }
    if row
        .days_to_primary_crisis_start
        .is_none_or(|lead_days| lead_days <= 0)
    {
        return None;
    }
    if !is_hedge_episode_row(row) {
        return None;
    }

    forward_crisis_protected_no_positive_main_objective(row, horizon_days)
}

fn forward_crisis_protected_no_positive_main_objective(
    row: &ProbabilityTrainingRow,
    horizon_days: u32,
) -> Option<ForwardCrisisPreparePrewarningObjective> {
    if !row.protected_action_window
        || !matches!(
            row.scenario_training_role.as_deref(),
            Some("no_positive_main")
        )
    {
        return None;
    }

    match horizon_days {
        20 => Some(ForwardCrisisPreparePrewarningObjective {
            target_label: 0.34,
            objective_weight: 0.90,
        }),
        60 => Some(ForwardCrisisPreparePrewarningObjective {
            target_label: 0.48,
            objective_weight: 0.95,
        }),
        _ => None,
    }
}

fn is_prepare_episode_row(row: &ProbabilityTrainingRow) -> bool {
    row.prepare_episode_label > 0 || matches!(row.primary_action_level.as_deref(), Some("prepare"))
}

fn is_hedge_episode_row(row: &ProbabilityTrainingRow) -> bool {
    row.hedge_episode_label > 0 || matches!(row.primary_action_level.as_deref(), Some("hedge"))
}

pub(super) fn logistic_sample_weight(
    row: &ProbabilityTrainingRow,
    horizon_days: u32,
    positive_class_weight: f64,
    label_mode: ProbabilityTargetLabelMode,
) -> f64 {
    let label = row.label_for_horizon(label_mode, horizon_days);
    if label > 0.0 {
        let positive_weight = match label_mode {
            ProbabilityTargetLabelMode::ForwardCrisis => {
                forward_crisis_positive_sample_weight(row, horizon_days)
            }
            _ => positive_sample_action_weight(row, horizon_days),
        };
        (positive_class_weight * positive_weight).clamp(1.0, 36.0)
    } else {
        negative_sample_weight(row, horizon_days, label_mode)
    }
}

pub(crate) fn forward_crisis_regime_sample_weight(
    horizon_days: u32,
    regime: ProbabilityTrainingRegime,
) -> f64 {
    match regime {
        ProbabilityTrainingRegime::Normal => 1.0,
        ProbabilityTrainingRegime::PreWarningBuffer => match horizon_days {
            5 => 0.90,
            20 => 0.60,
            60 => 0.50,
            _ => 0.70,
        },
        ProbabilityTrainingRegime::PositiveWindow => match horizon_days {
            5 => 2.0,
            20 => 2.2,
            60 => 1.8,
            _ => 2.0,
        },
        ProbabilityTrainingRegime::InCrisis => match horizon_days {
            5 => 1.15,
            20 => 1.20,
            60 => 1.15,
            _ => 1.15,
        },
        ProbabilityTrainingRegime::PostCrisisCooldown => match horizon_days {
            5 => 1.10,
            20 => 1.35,
            60 => 1.60,
            _ => 1.25,
        },
    }
}

pub(crate) fn forward_crisis_positive_sample_weight(
    row: &ProbabilityTrainingRow,
    horizon_days: u32,
) -> f64 {
    (forward_crisis_regime_sample_weight(horizon_days, row.regime_for_horizon(horizon_days))
        * positive_sample_action_weight(row, horizon_days)
        * scenario_training_role_weight_multiplier(
            row.scenario_training_role.as_deref(),
            horizon_days,
        ))
    .clamp(1.0, 12.0)
}

fn forward_crisis_negative_regime_sample_weight(
    horizon_days: u32,
    regime: ProbabilityTrainingRegime,
) -> f64 {
    match regime {
        ProbabilityTrainingRegime::Normal => match horizon_days {
            20 => 1.10,
            60 => 1.15,
            _ => 1.0,
        },
        ProbabilityTrainingRegime::PreWarningBuffer => match horizon_days {
            5 => 0.90,
            20 => 0.70,
            60 => 0.60,
            _ => 0.75,
        },
        ProbabilityTrainingRegime::PositiveWindow => 1.0,
        ProbabilityTrainingRegime::InCrisis => match horizon_days {
            5 => 1.15,
            20 => 1.25,
            60 => 1.20,
            _ => 1.20,
        },
        ProbabilityTrainingRegime::PostCrisisCooldown => match horizon_days {
            5 => 1.10,
            20 => 1.45,
            60 => 1.75,
            _ => 1.40,
        },
    }
}

pub(crate) fn negative_sample_weight(
    row: &ProbabilityTrainingRow,
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) -> f64 {
    match label_mode {
        ProbabilityTargetLabelMode::ActionWindow => match row.regime_for_horizon(horizon_days) {
            ProbabilityTrainingRegime::Normal => 1.0,
            ProbabilityTrainingRegime::PositiveWindow => 1.0,
            ProbabilityTrainingRegime::PreWarningBuffer => match horizon_days {
                5 => 0.85,
                20 => 0.75,
                60 => 0.65,
                _ => 0.75,
            },
            ProbabilityTrainingRegime::InCrisis => match horizon_days {
                5 => 1.90,
                20 => 1.70,
                60 => 1.45,
                _ => 1.60,
            },
            ProbabilityTrainingRegime::PostCrisisCooldown => match horizon_days {
                5 => 1.60,
                20 => 1.45,
                60 => 1.25,
                _ => 1.35,
            },
        },
        ProbabilityTargetLabelMode::ActionEpisode => {
            if row.protected_action_window {
                return 0.55;
            }

            match row.action_episode_phase.as_str() {
                "late_validation" => match horizon_days {
                    5 => 0.95,
                    20 => 0.80,
                    60 => 0.70,
                    _ => 0.80,
                },
                "cooldown" => match horizon_days {
                    5 => 0.70,
                    20 => 0.65,
                    60 => 0.60,
                    _ => 0.65,
                },
                _ => match row.regime_for_horizon(horizon_days) {
                    ProbabilityTrainingRegime::Normal => 1.0,
                    ProbabilityTrainingRegime::PositiveWindow => 1.0,
                    ProbabilityTrainingRegime::PreWarningBuffer => match horizon_days {
                        5 => 0.85,
                        20 => 0.75,
                        60 => 0.65,
                        _ => 0.75,
                    },
                    ProbabilityTrainingRegime::InCrisis => match horizon_days {
                        5 => 1.15,
                        20 => 1.05,
                        60 => 0.95,
                        _ => 1.0,
                    },
                    ProbabilityTrainingRegime::PostCrisisCooldown => match horizon_days {
                        5 => 0.75,
                        20 => 0.70,
                        60 => 0.65,
                        _ => 0.70,
                    },
                },
            }
        }
        ProbabilityTargetLabelMode::ForwardCrisis => {
            if let Some(objective) = forward_crisis_episode_native_objective(row, horizon_days) {
                return objective.objective_weight;
            }
            if row.protected_action_window {
                return match row.action_episode_phase.as_str() {
                    "primary" => match horizon_days {
                        5 => 0.95,
                        20 => 0.55,
                        60 => 0.65,
                        _ => 0.55,
                    },
                    "late_validation" => match horizon_days {
                        5 => 0.95,
                        20 => 0.70,
                        60 => 0.80,
                        _ => 0.65,
                    },
                    "cooldown" => match horizon_days {
                        5 => 1.05,
                        20 => 1.20,
                        60 => 1.35,
                        _ => 1.00,
                    },
                    _ => match horizon_days {
                        5 => 0.95,
                        20 => 0.80,
                        60 => 0.90,
                        _ => 0.75,
                    },
                };
            }
            forward_crisis_negative_regime_sample_weight(
                horizon_days,
                row.regime_for_horizon(horizon_days),
            )
        }
    }
}

pub(crate) fn positive_sample_action_weight(
    row: &ProbabilityTrainingRow,
    horizon_days: u32,
) -> f64 {
    let mut weight = 1.0;
    if let Some(lead_days) = row.days_to_primary_crisis_start {
        weight *= lead_time_positive_multiplier(lead_days, horizon_days);
    }
    weight *= horizon_role_weight_multiplier(row, horizon_days);
    weight *= scenario_family_weight_multiplier(row.scenario_family.as_deref(), horizon_days);
    weight.clamp(0.5, 2.75)
}

fn lead_time_positive_multiplier(lead_days: i64, horizon_days: u32) -> f64 {
    if lead_days <= 0 {
        return 1.0;
    }

    let capped = lead_days.min(horizon_days as i64) as f64;
    let horizon = horizon_days.max(1) as f64;
    let normalized = if horizon <= 1.0 {
        0.0
    } else {
        (capped - 1.0) / (horizon - 1.0)
    };
    let max_lift = match horizon_days {
        5 => 0.35,
        20 => 0.45,
        60 => 0.55,
        _ => 0.30,
    };
    1.0 + normalized.clamp(0.0, 1.0) * max_lift
}

fn horizon_role_weight_multiplier(row: &ProbabilityTrainingRow, horizon_days: u32) -> f64 {
    match row.primary_scenario_supports_horizon(horizon_days) {
        Some(true) => 1.25,
        Some(false) => 0.55,
        None => 1.0,
    }
}

fn scenario_training_role_weight_multiplier(
    scenario_training_role: Option<&str>,
    horizon_days: u32,
) -> f64 {
    match (horizon_days, scenario_training_role) {
        (_, Some("mandatory")) => 1.0,
        (5, Some("candidate_optional")) => 1.10,
        (20, Some("candidate_optional")) => 1.30,
        (60, Some("candidate_optional")) => 1.45,
        (5, Some("extension_only")) => 1.45,
        (20, Some("extension_only")) => 1.65,
        (60, Some("extension_only")) => 1.70,
        (_, Some("no_positive_main")) => 1.0,
        _ => 1.0,
    }
}

fn scenario_family_weight_multiplier(scenario_family: Option<&str>, horizon_days: u32) -> f64 {
    match (horizon_days, scenario_family) {
        (5, Some("acute_market_liquidity_crash")) => 1.50,
        (5, Some("systemic_credit_banking_crisis")) => 0.80,
        (5, Some("mixed_systemic_stress")) => 0.85,
        (5, Some("rate_shock_or_policy_dislocation")) => 0.85,
        (20, Some("acute_market_liquidity_crash")) => 1.30,
        (20, Some("systemic_credit_banking_crisis")) => 1.15,
        (20, Some("mixed_systemic_stress")) => 1.35,
        (20, Some("rate_shock_or_policy_dislocation")) => 1.25,
        (60, Some("acute_market_liquidity_crash")) => 0.85,
        (60, Some("systemic_credit_banking_crisis")) => 1.25,
        (60, Some("mixed_systemic_stress")) => 1.45,
        (60, Some("rate_shock_or_policy_dislocation")) => 1.35,
        _ => 1.0,
    }
}

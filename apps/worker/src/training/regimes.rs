use chrono::{Duration, NaiveDate};

use super::ProbabilityTrainingRegime;

pub(crate) fn probability_training_regime_name(regime: ProbabilityTrainingRegime) -> &'static str {
    match regime {
        ProbabilityTrainingRegime::Normal => "normal",
        ProbabilityTrainingRegime::PositiveWindow => "positive_window",
        ProbabilityTrainingRegime::PreWarningBuffer => "pre_warning_buffer",
        ProbabilityTrainingRegime::InCrisis => "in_crisis",
        ProbabilityTrainingRegime::PostCrisisCooldown => "post_crisis_cooldown",
    }
}

pub(crate) fn forward_crisis_label(
    as_of_date: NaiveDate,
    scenarios: &[crate::CrisisScenario],
    horizon_days: i64,
) -> u8 {
    let horizon_days_u32 = horizon_days as u32;
    scenarios.iter().any(|scenario| {
        let anchor_date = if crate::scenario_supports_horizon(scenario, horizon_days_u32) {
            crate::label_anchor_date(scenario, horizon_days_u32)
        } else {
            scenario.crisis_start
        };
        let lead_days = (anchor_date - as_of_date).num_days();
        (1..=horizon_days).contains(&lead_days)
    }) as u8
}

pub(crate) fn post_crisis_cooldown_days(horizon_days: u32) -> i64 {
    match horizon_days {
        5 => 14,
        20 => 30,
        60 => 45,
        _ => horizon_days as i64,
    }
}

pub(crate) fn forward_crisis_training_regime(
    as_of_date: NaiveDate,
    scenarios: &[crate::CrisisScenario],
    horizon_days: u32,
) -> ProbabilityTrainingRegime {
    if forward_crisis_label(as_of_date, scenarios, horizon_days as i64) > 0 {
        return ProbabilityTrainingRegime::PositiveWindow;
    }

    let positive_buffer = scenarios.iter().any(|scenario| {
        let anchor_date = if crate::scenario_supports_horizon(scenario, horizon_days) {
            crate::label_anchor_date(scenario, horizon_days)
        } else {
            scenario.crisis_start
        };
        let positive_start = anchor_date
            .checked_sub_signed(Duration::days(horizon_days as i64))
            .unwrap_or(anchor_date);
        as_of_date >= crate::action_window_start_date(scenario, horizon_days)
            && as_of_date < positive_start
    });
    if positive_buffer {
        return ProbabilityTrainingRegime::PreWarningBuffer;
    }

    if scenarios
        .iter()
        .any(|scenario| as_of_date >= scenario.crisis_start && as_of_date <= scenario.crisis_end)
    {
        return ProbabilityTrainingRegime::InCrisis;
    }

    let cooldown = scenarios.iter().any(|scenario| {
        let cooldown_end = scenario
            .crisis_end
            .checked_add_signed(Duration::days(post_crisis_cooldown_days(horizon_days)))
            .unwrap_or(scenario.crisis_end);
        as_of_date > scenario.crisis_end && as_of_date <= cooldown_end
    });
    if cooldown {
        return ProbabilityTrainingRegime::PostCrisisCooldown;
    }

    ProbabilityTrainingRegime::Normal
}

pub(crate) fn forward_crisis_training_regime_with_context(
    as_of_date: NaiveDate,
    positive_scenarios: &[crate::CrisisScenario],
    context_scenarios: &[crate::CrisisScenario],
    horizon_days: u32,
) -> ProbabilityTrainingRegime {
    let base_regime = forward_crisis_training_regime(as_of_date, positive_scenarios, horizon_days);
    if !matches!(base_regime, ProbabilityTrainingRegime::Normal) || horizon_days < 20 {
        return base_regime;
    }

    match crate::protected_context_phase_for_date(as_of_date, positive_scenarios, context_scenarios)
    {
        Some(crate::ActionEpisodePhase::Primary | crate::ActionEpisodePhase::LateValidation) => {
            ProbabilityTrainingRegime::PreWarningBuffer
        }
        Some(crate::ActionEpisodePhase::Cooldown) => ProbabilityTrainingRegime::PostCrisisCooldown,
        _ => base_regime,
    }
}

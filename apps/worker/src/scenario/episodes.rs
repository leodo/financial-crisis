use chrono::NaiveDate;
use fc_domain::ActionabilityLevel;

use super::timing::{
    action_episode_phase_for_date, action_episode_phase_rank, action_episode_window,
    action_level_rank,
};
use super::{ActionEpisodePhase, ActionEpisodeSelection, CrisisScenario};

pub(super) fn action_level_proxy_horizon_days(level: ActionabilityLevel) -> u32 {
    match level {
        ActionabilityLevel::Prepare => 60,
        ActionabilityLevel::Hedge => 20,
        ActionabilityLevel::Defend => 5,
    }
}

pub(crate) fn actionability_level_for_proxy_horizon(
    horizon_days: u32,
) -> Option<ActionabilityLevel> {
    match horizon_days {
        60 => Some(ActionabilityLevel::Prepare),
        20 => Some(ActionabilityLevel::Hedge),
        5 => Some(ActionabilityLevel::Defend),
        _ => None,
    }
}

pub(crate) fn action_episode_label_for_level(
    as_of_date: NaiveDate,
    scenarios: &[CrisisScenario],
    level: ActionabilityLevel,
) -> u8 {
    scenarios.iter().any(|scenario| {
        matches!(
            action_episode_phase_for_date(as_of_date, scenario, level),
            ActionEpisodePhase::Primary
        )
    }) as u8
}

pub(crate) fn dominant_action_episode_for_date(
    as_of_date: NaiveDate,
    scenarios: &[CrisisScenario],
) -> Option<ActionEpisodeSelection> {
    [
        ActionabilityLevel::Prepare,
        ActionabilityLevel::Hedge,
        ActionabilityLevel::Defend,
    ]
    .into_iter()
    .flat_map(|level| {
        scenarios.iter().filter_map(move |scenario| {
            let phase = action_episode_phase_for_date(as_of_date, scenario, level);
            (!matches!(phase, ActionEpisodePhase::Outside)).then_some(ActionEpisodeSelection {
                scenario_id: scenario.scenario_id.clone(),
                level,
                phase,
                protected_action_window: action_episode_window(scenario, level)
                    .protected_action_window,
                crisis_start: scenario.crisis_start,
            })
        })
    })
    .min_by_key(|selection| {
        (
            action_episode_phase_rank(selection.phase),
            action_level_rank(selection.level),
            (selection.crisis_start - as_of_date).num_days().abs(),
        )
    })
}

pub(crate) fn protected_context_phase_for_date(
    as_of_date: NaiveDate,
    positive_scenarios: &[CrisisScenario],
    context_scenarios: &[CrisisScenario],
) -> Option<ActionEpisodePhase> {
    context_scenarios
        .iter()
        .filter(|scenario| {
            scenario.protected_window
                && !positive_scenarios
                    .iter()
                    .any(|positive| positive.scenario_id == scenario.scenario_id)
        })
        .flat_map(|scenario| {
            scenario
                .protected_action_levels
                .iter()
                .copied()
                .filter_map(move |level| {
                    let phase = action_episode_phase_for_date(as_of_date, scenario, level);
                    (!matches!(phase, ActionEpisodePhase::Outside)).then_some((phase, scenario))
                })
        })
        .min_by_key(|(phase, scenario)| {
            (
                action_episode_phase_rank(*phase),
                (scenario.crisis_start - as_of_date).num_days().abs(),
            )
        })
        .map(|(phase, _)| phase)
}

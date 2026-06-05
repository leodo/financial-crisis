use chrono::NaiveDate;
use fc_domain::ActionabilityLevel;

use super::timing::action_episode_phase_for_date;
use super::{dominant_action_episode_for_date, ActionEpisodePhase, CrisisScenario};

fn scenario_has_action_window(scenario: &CrisisScenario, as_of_date: NaiveDate) -> bool {
    [
        ActionabilityLevel::Prepare,
        ActionabilityLevel::Hedge,
        ActionabilityLevel::Defend,
    ]
    .into_iter()
    .any(|level| {
        !matches!(
            action_episode_phase_for_date(as_of_date, scenario, level),
            ActionEpisodePhase::Outside
        )
    })
}

pub(crate) fn primary_scenario_for_date(
    as_of_date: NaiveDate,
    scenarios: &[CrisisScenario],
) -> Option<CrisisScenario> {
    if let Some(selection) = dominant_action_episode_for_date(as_of_date, scenarios) {
        return scenarios
            .iter()
            .find(|scenario| scenario.scenario_id == selection.scenario_id)
            .cloned();
    }

    scenarios
        .iter()
        .filter(|scenario| scenario_has_action_window(scenario, as_of_date))
        .min_by_key(|scenario| {
            let distance = (scenario.crisis_start - as_of_date).num_days().abs();
            let in_crisis_penalty = if as_of_date > scenario.crisis_start {
                10_000
            } else {
                0
            };
            in_crisis_penalty + distance
        })
        .cloned()
        .or_else(|| forward_scenario(as_of_date, scenarios, 60))
}

fn forward_scenario(
    as_of_date: NaiveDate,
    scenarios: &[CrisisScenario],
    horizon_days: i64,
) -> Option<CrisisScenario> {
    scenarios
        .iter()
        .filter_map(|scenario| {
            let lead_days = (scenario.crisis_start - as_of_date).num_days();
            (1..=horizon_days)
                .contains(&lead_days)
                .then_some((scenario.clone(), lead_days))
        })
        .min_by_key(|(_, lead_days)| *lead_days)
        .map(|(scenario, _)| scenario)
}

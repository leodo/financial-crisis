use chrono::NaiveDate;
use fc_domain::ActionabilityLevel;

#[derive(Debug, Clone)]
pub(crate) struct ScenarioLabelSnapshot {
    pub(crate) primary_scenario_id: Option<String>,
    pub(crate) scenario_family: Option<String>,
    pub(crate) scenario_training_role: Option<String>,
    pub(crate) days_to_primary_crisis_start: Option<i64>,
    pub(crate) primary_scenario_supports_5d: bool,
    pub(crate) primary_scenario_supports_20d: bool,
    pub(crate) primary_scenario_supports_60d: bool,
    pub(crate) label_5d: u8,
    pub(crate) label_20d: u8,
    pub(crate) label_60d: u8,
    pub(crate) regime_5d: crate::ProbabilityTrainingRegime,
    pub(crate) regime_20d: crate::ProbabilityTrainingRegime,
    pub(crate) regime_60d: crate::ProbabilityTrainingRegime,
    pub(crate) action_label_5d: u8,
    pub(crate) action_label_20d: u8,
    pub(crate) action_label_60d: u8,
    pub(crate) prepare_episode_label: u8,
    pub(crate) hedge_episode_label: u8,
    pub(crate) defend_episode_label: u8,
    pub(crate) primary_action_level: Option<String>,
    pub(crate) action_episode_id: Option<String>,
    pub(crate) action_episode_phase: String,
    pub(crate) protected_action_window: bool,
}

pub(crate) fn derive_scenario_label_snapshot(
    as_of_date: NaiveDate,
    positive_scenarios: &[crate::CrisisScenario],
    context_scenarios: &[crate::CrisisScenario],
) -> ScenarioLabelSnapshot {
    let primary_scenario = crate::primary_scenario_for_date(as_of_date, context_scenarios);
    let dominant_action_episode =
        crate::dominant_action_episode_for_date(as_of_date, context_scenarios);

    ScenarioLabelSnapshot {
        primary_scenario_id: primary_scenario
            .as_ref()
            .map(|scenario| scenario.scenario_id.clone()),
        scenario_family: primary_scenario
            .as_ref()
            .map(|scenario| scenario.family.clone()),
        scenario_training_role: primary_scenario
            .as_ref()
            .map(|scenario| scenario.training_role.clone()),
        days_to_primary_crisis_start: primary_scenario
            .as_ref()
            .map(|scenario| (scenario.crisis_start - as_of_date).num_days()),
        primary_scenario_supports_5d: primary_scenario
            .as_ref()
            .is_some_and(|scenario| crate::scenario_supports_horizon(scenario, 5)),
        primary_scenario_supports_20d: primary_scenario
            .as_ref()
            .is_some_and(|scenario| crate::scenario_supports_horizon(scenario, 20)),
        primary_scenario_supports_60d: primary_scenario
            .as_ref()
            .is_some_and(|scenario| crate::scenario_supports_horizon(scenario, 60)),
        label_5d: crate::forward_crisis_label(as_of_date, positive_scenarios, 5),
        label_20d: crate::forward_crisis_label(as_of_date, positive_scenarios, 20),
        label_60d: crate::forward_crisis_label(as_of_date, positive_scenarios, 60),
        regime_5d: crate::forward_crisis_training_regime_with_context(
            as_of_date,
            positive_scenarios,
            context_scenarios,
            5,
        ),
        regime_20d: crate::forward_crisis_training_regime_with_context(
            as_of_date,
            positive_scenarios,
            context_scenarios,
            20,
        ),
        regime_60d: crate::forward_crisis_training_regime_with_context(
            as_of_date,
            positive_scenarios,
            context_scenarios,
            60,
        ),
        action_label_5d: crate::action_window_label(as_of_date, context_scenarios, 5),
        action_label_20d: crate::action_window_label(as_of_date, context_scenarios, 20),
        action_label_60d: crate::action_window_label(as_of_date, context_scenarios, 60),
        prepare_episode_label: crate::action_episode_label_for_level(
            as_of_date,
            context_scenarios,
            ActionabilityLevel::Prepare,
        ),
        hedge_episode_label: crate::action_episode_label_for_level(
            as_of_date,
            context_scenarios,
            ActionabilityLevel::Hedge,
        ),
        defend_episode_label: crate::action_episode_label_for_level(
            as_of_date,
            context_scenarios,
            ActionabilityLevel::Defend,
        ),
        primary_action_level: dominant_action_episode
            .as_ref()
            .filter(|selection| matches!(selection.phase, crate::ActionEpisodePhase::Primary))
            .map(|selection| crate::actionability_level_text(selection.level).to_string()),
        action_episode_id: dominant_action_episode.as_ref().map(|selection| {
            format!(
                "{}:{}",
                selection.scenario_id,
                crate::actionability_level_text(selection.level)
            )
        }),
        action_episode_phase: dominant_action_episode
            .as_ref()
            .map(|selection| selection.phase.as_str().to_string())
            .unwrap_or_else(|| crate::ActionEpisodePhase::Outside.as_str().to_string()),
        protected_action_window: dominant_action_episode
            .as_ref()
            .is_some_and(|selection| selection.protected_action_window),
    }
}

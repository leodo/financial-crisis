use chrono::{Duration, NaiveDate};
use fc_domain::{ActionEpisodeTemplateId, ActionEpisodeWindowOverride, ActionabilityLevel};

use super::{ActionEpisodePhase, CrisisScenario};
use crate::training::post_crisis_cooldown_days;

#[derive(Debug, Clone, Copy)]
pub(super) struct DateRange {
    start: NaiveDate,
    end: NaiveDate,
}

impl DateRange {
    fn new(start: NaiveDate, end: NaiveDate) -> Option<Self> {
        (start <= end).then_some(Self { start, end })
    }

    fn contains(self, as_of_date: NaiveDate) -> bool {
        as_of_date >= self.start && as_of_date <= self.end
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ActionEpisodeWindow {
    pub(super) primary: Option<DateRange>,
    pub(super) late_validation: Option<DateRange>,
    pub(super) cooldown: Option<DateRange>,
    pub(super) protected_action_window: bool,
}

fn shift_date(date: NaiveDate, days: i64) -> NaiveDate {
    date.checked_add_signed(Duration::days(days))
        .unwrap_or(date)
}

fn next_day(date: NaiveDate) -> NaiveDate {
    shift_date(date, 1)
}

pub(super) fn action_level_rank(level: ActionabilityLevel) -> i32 {
    match level {
        ActionabilityLevel::Defend => 0,
        ActionabilityLevel::Hedge => 1,
        ActionabilityLevel::Prepare => 2,
    }
}

pub(super) fn action_episode_phase_rank(phase: ActionEpisodePhase) -> i32 {
    match phase {
        ActionEpisodePhase::Primary => 0,
        ActionEpisodePhase::LateValidation => 1,
        ActionEpisodePhase::Cooldown => 2,
        ActionEpisodePhase::Outside => 3,
    }
}

fn action_episode_override_for_level(
    scenario: &CrisisScenario,
    level: ActionabilityLevel,
) -> Option<&ActionEpisodeWindowOverride> {
    let overrides = scenario.action_episode_overrides.as_ref()?;
    match level {
        ActionabilityLevel::Prepare => overrides.prepare.as_ref(),
        ActionabilityLevel::Hedge => overrides.hedge.as_ref(),
        ActionabilityLevel::Defend => overrides.defend.as_ref(),
    }
}

fn action_episode_default_window(
    scenario: &CrisisScenario,
    level: ActionabilityLevel,
) -> ActionEpisodeWindow {
    let acute_start = scenario.acute_start.unwrap_or(scenario.crisis_start);
    let (primary, late_validation) = match (scenario.episode_template_id, level) {
        (ActionEpisodeTemplateId::SystemicCreditBankingCrisis, ActionabilityLevel::Prepare) => (
            DateRange::new(
                scenario.pre_warning_start,
                shift_date(scenario.crisis_start, -21),
            ),
            DateRange::new(
                shift_date(scenario.crisis_start, -20),
                shift_date(scenario.crisis_start, -11),
            ),
        ),
        (ActionEpisodeTemplateId::SystemicCreditBankingCrisis, ActionabilityLevel::Hedge) => (
            DateRange::new(
                shift_date(scenario.crisis_start, -20),
                shift_date(scenario.crisis_start, -6),
            ),
            DateRange::new(
                shift_date(scenario.crisis_start, -5),
                shift_date(scenario.crisis_start, 3),
            ),
        ),
        (ActionEpisodeTemplateId::SystemicCreditBankingCrisis, ActionabilityLevel::Defend) => (
            DateRange::new(
                shift_date(scenario.crisis_start, -5),
                shift_date(acute_start, 3),
            ),
            DateRange::new(shift_date(acute_start, 4), shift_date(acute_start, 10)),
        ),
        (ActionEpisodeTemplateId::AcuteMarketLiquidityCrash, ActionabilityLevel::Prepare) => (
            DateRange::new(
                scenario.pre_warning_start.max(shift_date(acute_start, -20)),
                shift_date(acute_start, -11),
            ),
            DateRange::new(shift_date(acute_start, -10), shift_date(acute_start, -7)),
        ),
        (ActionEpisodeTemplateId::AcuteMarketLiquidityCrash, ActionabilityLevel::Hedge) => (
            DateRange::new(shift_date(acute_start, -10), shift_date(acute_start, -4)),
            DateRange::new(shift_date(acute_start, -3), shift_date(acute_start, 1)),
        ),
        (ActionEpisodeTemplateId::AcuteMarketLiquidityCrash, ActionabilityLevel::Defend) => (
            DateRange::new(shift_date(acute_start, -3), shift_date(acute_start, 2)),
            DateRange::new(shift_date(acute_start, 3), shift_date(acute_start, 7)),
        ),
        (ActionEpisodeTemplateId::MixedSystemicStress, ActionabilityLevel::Prepare)
        | (ActionEpisodeTemplateId::RateShockOrPolicyDislocation, ActionabilityLevel::Prepare) => (
            DateRange::new(
                scenario.pre_warning_start,
                shift_date(scenario.crisis_start, -16),
            ),
            DateRange::new(
                shift_date(scenario.crisis_start, -15),
                shift_date(scenario.crisis_start, -8),
            ),
        ),
        (ActionEpisodeTemplateId::MixedSystemicStress, ActionabilityLevel::Hedge) => (
            DateRange::new(
                shift_date(scenario.crisis_start, -15),
                shift_date(scenario.crisis_start, -5),
            ),
            DateRange::new(
                shift_date(scenario.crisis_start, -4),
                shift_date(scenario.crisis_start, 3),
            ),
        ),
        (ActionEpisodeTemplateId::RateShockOrPolicyDislocation, ActionabilityLevel::Hedge) => (
            DateRange::new(
                shift_date(scenario.crisis_start, -15),
                shift_date(scenario.crisis_start, -5),
            ),
            DateRange::new(
                shift_date(scenario.crisis_start, -4),
                shift_date(scenario.crisis_start, 2),
            ),
        ),
        (
            ActionEpisodeTemplateId::MixedSystemicStress
            | ActionEpisodeTemplateId::RateShockOrPolicyDislocation,
            ActionabilityLevel::Defend,
        ) => (None, None),
    };
    let cooldown = DateRange::new(
        next_day(scenario.crisis_end),
        shift_date(
            scenario.crisis_end,
            post_crisis_cooldown_days(super::episodes::action_level_proxy_horizon_days(level)),
        ),
    );

    ActionEpisodeWindow {
        primary,
        late_validation,
        cooldown,
        protected_action_window: scenario.protected_window
            && scenario.protected_action_levels.contains(&level),
    }
}

pub(super) fn action_episode_window(
    scenario: &CrisisScenario,
    level: ActionabilityLevel,
) -> ActionEpisodeWindow {
    let mut window = action_episode_default_window(scenario, level);
    let Some(override_window) = action_episode_override_for_level(scenario, level) else {
        return window;
    };

    if override_window.enabled == Some(false) {
        return ActionEpisodeWindow {
            primary: None,
            late_validation: None,
            cooldown: None,
            protected_action_window: window.protected_action_window,
        };
    }

    if let (Some(primary_start), Some(primary_end)) =
        (override_window.primary_start, override_window.primary_end)
    {
        window.primary = DateRange::new(primary_start, primary_end);
    }
    if let Some(late_validation_end) = override_window.late_validation_end {
        window.late_validation = window
            .primary
            .and_then(|primary| DateRange::new(next_day(primary.end), late_validation_end));
    }
    if let Some(cooldown_end) = override_window.cooldown_end {
        window.cooldown = DateRange::new(next_day(scenario.crisis_end), cooldown_end);
    }

    window
}

pub(super) fn action_episode_phase_for_date(
    as_of_date: NaiveDate,
    scenario: &CrisisScenario,
    level: ActionabilityLevel,
) -> ActionEpisodePhase {
    let window = action_episode_window(scenario, level);
    if window
        .primary
        .is_some_and(|range| range.contains(as_of_date))
    {
        return ActionEpisodePhase::Primary;
    }
    if window
        .late_validation
        .is_some_and(|range| range.contains(as_of_date))
    {
        return ActionEpisodePhase::LateValidation;
    }
    if window
        .cooldown
        .is_some_and(|range| range.contains(as_of_date))
    {
        return ActionEpisodePhase::Cooldown;
    }
    ActionEpisodePhase::Outside
}

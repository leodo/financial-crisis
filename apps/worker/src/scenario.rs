use chrono::{Duration, NaiveDate};
use fc_domain::{
    ActionEpisodeTemplateId, ActionabilityLevel, CrisisScenarioActionEpisodeOverrides,
};

#[derive(Debug, Clone)]
pub(crate) struct CrisisScenario {
    pub(crate) scenario_id: String,
    pub(crate) family: String,
    pub(crate) training_role: String,
    pub(crate) pre_warning_start: NaiveDate,
    pub(crate) crisis_start: NaiveDate,
    pub(crate) acute_start: Option<NaiveDate>,
    pub(crate) crisis_end: NaiveDate,
    pub(crate) default_horizon_roles: Vec<u32>,
    pub(crate) protected_window: bool,
    pub(crate) protected_action_levels: Vec<ActionabilityLevel>,
    pub(crate) episode_template_id: ActionEpisodeTemplateId,
    pub(crate) action_episode_overrides: Option<CrisisScenarioActionEpisodeOverrides>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActionEpisodePhase {
    Outside,
    Cooldown,
    LateValidation,
    Primary,
}

impl ActionEpisodePhase {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Outside => "outside",
            Self::Cooldown => "cooldown",
            Self::LateValidation => "late_validation",
            Self::Primary => "primary",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ActionEpisodeSelection {
    pub(crate) scenario_id: String,
    pub(crate) level: ActionabilityLevel,
    pub(crate) phase: ActionEpisodePhase,
    pub(crate) protected_action_window: bool,
    pub(crate) crisis_start: NaiveDate,
}

#[derive(Debug, Clone, Copy)]
struct DateRange {
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
struct ActionEpisodeWindow {
    primary: Option<DateRange>,
    late_validation: Option<DateRange>,
    cooldown: Option<DateRange>,
    protected_action_window: bool,
}

fn shift_date(date: NaiveDate, days: i64) -> NaiveDate {
    date.checked_add_signed(Duration::days(days))
        .unwrap_or(date)
}

fn next_day(date: NaiveDate) -> NaiveDate {
    shift_date(date, 1)
}

fn action_level_rank(level: ActionabilityLevel) -> i32 {
    match level {
        ActionabilityLevel::Defend => 0,
        ActionabilityLevel::Hedge => 1,
        ActionabilityLevel::Prepare => 2,
    }
}

fn action_episode_phase_rank(phase: ActionEpisodePhase) -> i32 {
    match phase {
        ActionEpisodePhase::Primary => 0,
        ActionEpisodePhase::LateValidation => 1,
        ActionEpisodePhase::Cooldown => 2,
        ActionEpisodePhase::Outside => 3,
    }
}

pub(crate) fn action_level_proxy_horizon_days(level: ActionabilityLevel) -> u32 {
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

fn action_episode_override_for_level(
    overrides: Option<&CrisisScenarioActionEpisodeOverrides>,
    level: ActionabilityLevel,
) -> Option<&fc_domain::ActionEpisodeWindowOverride> {
    let overrides = overrides?;
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
            crate::training::post_crisis_cooldown_days(action_level_proxy_horizon_days(level)),
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

fn action_episode_window(
    scenario: &CrisisScenario,
    level: ActionabilityLevel,
) -> ActionEpisodeWindow {
    let mut window = action_episode_default_window(scenario, level);
    let Some(override_window) =
        action_episode_override_for_level(scenario.action_episode_overrides.as_ref(), level)
    else {
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

fn action_episode_phase_for_date(
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

pub(crate) fn scenario_supports_horizon(scenario: &CrisisScenario, horizon_days: u32) -> bool {
    scenario.default_horizon_roles.contains(&horizon_days)
}

pub(crate) fn label_anchor_date(scenario: &CrisisScenario, horizon_days: u32) -> NaiveDate {
    if horizon_days == 5 {
        scenario.acute_start.unwrap_or(scenario.crisis_start)
    } else {
        scenario.crisis_start
    }
}

fn action_window_lead_days(horizon_days: u32) -> i64 {
    match horizon_days {
        5 => 10,
        20 => 35,
        60 => 90,
        _ => horizon_days as i64,
    }
}

pub(crate) fn action_window_start_date(scenario: &CrisisScenario, horizon_days: u32) -> NaiveDate {
    let anchor_date = label_anchor_date(scenario, horizon_days);
    let buffered_start = anchor_date
        .checked_sub_signed(Duration::days(action_window_lead_days(horizon_days)))
        .unwrap_or(anchor_date);
    scenario.pre_warning_start.max(buffered_start)
}

fn action_window_end_days(horizon_days: u32) -> i64 {
    match horizon_days {
        5 => 7,
        20 => 20,
        60 => 30,
        _ => horizon_days as i64,
    }
}

fn action_window_end_date(scenario: &CrisisScenario, horizon_days: u32) -> NaiveDate {
    let anchor_date = label_anchor_date(scenario, horizon_days);
    let buffered_end = anchor_date
        .checked_add_signed(Duration::days(action_window_end_days(horizon_days)))
        .unwrap_or(scenario.crisis_end);
    scenario.crisis_end.min(buffered_end)
}

pub(crate) fn action_window_label(
    as_of_date: NaiveDate,
    scenarios: &[CrisisScenario],
    horizon_days: i64,
) -> u8 {
    let horizon_days_u32 = horizon_days as u32;
    scenarios.iter().any(|scenario| {
        as_of_date >= action_window_start_date(scenario, horizon_days_u32)
            && as_of_date <= action_window_end_date(scenario, horizon_days_u32)
    }) as u8
}

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

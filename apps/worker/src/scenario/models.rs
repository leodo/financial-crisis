use chrono::NaiveDate;
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

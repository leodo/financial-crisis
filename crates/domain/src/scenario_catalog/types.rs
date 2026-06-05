use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::ActionabilityLevel;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrisisScenarioFamily {
    AcuteMarketLiquidityCrash,
    SystemicCreditBankingCrisis,
    MixedSystemicStress,
    RateShockOrPolicyDislocation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrisisScenarioTrainingRole {
    Mandatory,
    CandidateOptional,
    ExtensionOnly,
    NoPositiveMain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionEpisodeTemplateId {
    AcuteMarketLiquidityCrash,
    SystemicCreditBankingCrisis,
    MixedSystemicStress,
    RateShockOrPolicyDislocation,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActionEpisodeWindowOverride {
    #[serde(default)]
    pub enabled: Option<bool>,
    pub primary_start: Option<NaiveDate>,
    pub primary_end: Option<NaiveDate>,
    pub late_validation_end: Option<NaiveDate>,
    pub cooldown_end: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CrisisScenarioActionEpisodeOverrides {
    pub prepare: Option<ActionEpisodeWindowOverride>,
    pub hedge: Option<ActionEpisodeWindowOverride>,
    pub defend: Option<ActionEpisodeWindowOverride>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrisisScenarioDefinition {
    pub scenario_id: String,
    pub family: CrisisScenarioFamily,
    pub label: String,
    pub pre_warning_start: NaiveDate,
    pub crisis_start: NaiveDate,
    pub acute_start: Option<NaiveDate>,
    pub crisis_peak: Option<NaiveDate>,
    pub crisis_end: NaiveDate,
    pub default_horizon_roles: Vec<u32>,
    pub training_role: CrisisScenarioTrainingRole,
    #[serde(default)]
    pub episode_template_id: Option<ActionEpisodeTemplateId>,
    #[serde(default)]
    pub action_episode_overrides: Option<CrisisScenarioActionEpisodeOverrides>,
    pub protected_window: bool,
    #[serde(default)]
    pub protected_action_levels: Vec<ActionabilityLevel>,
    pub evidence_basis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrisisScenarioLabelSet {
    pub label_set_id: String,
    pub scenario_ids: Vec<String>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrisisScenarioWindowSet {
    pub window_set_id: String,
    pub scenario_ids: Vec<String>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrisisScenarioCatalog {
    pub catalog_id: String,
    pub market_scope: String,
    pub note: String,
    pub source: String,
    pub warning: Option<String>,
    pub label_sets: Vec<CrisisScenarioLabelSet>,
    pub window_sets: Vec<CrisisScenarioWindowSet>,
    pub scenarios: Vec<CrisisScenarioDefinition>,
}

impl CrisisScenarioCatalog {
    pub fn scenarios_for_label_set(
        &self,
        label_set_id: &str,
    ) -> Option<Vec<&CrisisScenarioDefinition>> {
        let label_set = self
            .label_sets
            .iter()
            .find(|label_set| label_set.label_set_id == label_set_id)?;
        let mut scenarios = Vec::with_capacity(label_set.scenario_ids.len());
        for scenario_id in &label_set.scenario_ids {
            let scenario = self
                .scenarios
                .iter()
                .find(|scenario| scenario.scenario_id == *scenario_id)?;
            scenarios.push(scenario);
        }
        Some(scenarios)
    }

    pub fn scenario_ids_for_window_set(&self, window_set_id: &str) -> Option<&[String]> {
        self.window_sets
            .iter()
            .find(|window_set| window_set.window_set_id == window_set_id)
            .map(|window_set| window_set.scenario_ids.as_slice())
    }
}

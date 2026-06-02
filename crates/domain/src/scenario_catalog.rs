use std::{collections::HashSet, env, fs};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::ActionabilityLevel;

const CRISIS_SCENARIO_CATALOG_ENV: &str = "FC_SCENARIO_CATALOG_PATH";
const EMBEDDED_SCENARIO_CATALOG_PATH: &str = "embedded:config/research_crisis_scenarios.us.json";
const EMBEDDED_SCENARIO_CATALOG_JSON: &str =
    include_str!("../../../config/research_crisis_scenarios.us.json");

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

#[derive(Debug, Clone, Deserialize)]
struct CrisisScenarioCatalogFile {
    catalog_id: String,
    market_scope: String,
    note: String,
    label_sets: Vec<CrisisScenarioLabelSet>,
    window_sets: Vec<CrisisScenarioWindowSet>,
    scenarios: Vec<CrisisScenarioDefinition>,
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

pub fn load_crisis_scenario_catalog() -> CrisisScenarioCatalog {
    match env::var(CRISIS_SCENARIO_CATALOG_ENV) {
        Ok(path) => match load_crisis_scenario_catalog_from_file(&path) {
            Ok(catalog) => catalog,
            Err(error) => {
                let mut fallback = embedded_crisis_scenario_catalog();
                fallback.warning = Some(format!(
                    "无法从 {path} 加载危机场景目录，已退回内置默认配置：{error}"
                ));
                fallback
            }
        },
        Err(_) => embedded_crisis_scenario_catalog(),
    }
}

pub fn embedded_crisis_scenario_catalog() -> CrisisScenarioCatalog {
    parse_catalog(
        EMBEDDED_SCENARIO_CATALOG_JSON,
        EMBEDDED_SCENARIO_CATALOG_PATH,
        None,
    )
    .expect("embedded crisis scenario catalog must be valid")
}

pub fn load_crisis_scenario_catalog_from_file(path: &str) -> Result<CrisisScenarioCatalog, String> {
    let raw = fs::read_to_string(path).map_err(|error| format!("读取危机场景目录失败: {error}"))?;
    parse_catalog(&raw, path, None)
}

fn parse_catalog(
    raw: &str,
    source: &str,
    warning: Option<String>,
) -> Result<CrisisScenarioCatalog, String> {
    let mut parsed: CrisisScenarioCatalogFile =
        serde_json::from_str(raw).map_err(|error| format!("解析危机场景目录失败: {error}"))?;
    for scenario in &mut parsed.scenarios {
        if scenario.episode_template_id.is_none() {
            scenario.episode_template_id = Some(default_action_episode_template(scenario.family));
        }
        if scenario.protected_window && scenario.protected_action_levels.is_empty() {
            scenario.protected_action_levels =
                vec![ActionabilityLevel::Prepare, ActionabilityLevel::Hedge];
        }
    }
    validate_scenarios(&parsed.scenarios)?;
    validate_label_sets(&parsed.label_sets, &parsed.scenarios)?;
    validate_window_sets(&parsed.window_sets, &parsed.scenarios)?;
    parsed
        .scenarios
        .sort_by_key(|scenario| scenario.crisis_start);

    Ok(CrisisScenarioCatalog {
        catalog_id: parsed.catalog_id,
        market_scope: parsed.market_scope,
        note: parsed.note,
        source: source.to_string(),
        warning,
        label_sets: parsed.label_sets,
        window_sets: parsed.window_sets,
        scenarios: parsed.scenarios,
    })
}

fn default_action_episode_template(family: CrisisScenarioFamily) -> ActionEpisodeTemplateId {
    match family {
        CrisisScenarioFamily::AcuteMarketLiquidityCrash => {
            ActionEpisodeTemplateId::AcuteMarketLiquidityCrash
        }
        CrisisScenarioFamily::SystemicCreditBankingCrisis => {
            ActionEpisodeTemplateId::SystemicCreditBankingCrisis
        }
        CrisisScenarioFamily::MixedSystemicStress => ActionEpisodeTemplateId::MixedSystemicStress,
        CrisisScenarioFamily::RateShockOrPolicyDislocation => {
            ActionEpisodeTemplateId::RateShockOrPolicyDislocation
        }
    }
}

fn validate_scenarios(scenarios: &[CrisisScenarioDefinition]) -> Result<(), String> {
    if scenarios.is_empty() {
        return Err("危机场景目录不能为空，至少需要一个场景。".to_string());
    }

    let mut ids = HashSet::new();
    for scenario in scenarios {
        if !ids.insert(scenario.scenario_id.clone()) {
            return Err(format!("场景 {} 重复定义。", scenario.scenario_id));
        }
        if scenario.pre_warning_start > scenario.crisis_start {
            return Err(format!(
                "场景 {} 的 pre_warning_start 晚于 crisis_start。",
                scenario.scenario_id
            ));
        }
        if scenario.crisis_start > scenario.crisis_end {
            return Err(format!(
                "场景 {} 的 crisis_start 晚于 crisis_end。",
                scenario.scenario_id
            ));
        }
        if let Some(acute_start) = scenario.acute_start {
            if acute_start < scenario.crisis_start || acute_start > scenario.crisis_end {
                return Err(format!(
                    "场景 {} 的 acute_start 不在 crisis_start 与 crisis_end 之间。",
                    scenario.scenario_id
                ));
            }
        }
        if let Some(crisis_peak) = scenario.crisis_peak {
            if crisis_peak < scenario.crisis_start || crisis_peak > scenario.crisis_end {
                return Err(format!(
                    "场景 {} 的 crisis_peak 不在 crisis_start 与 crisis_end 之间。",
                    scenario.scenario_id
                ));
            }
        }
        if scenario.default_horizon_roles.is_empty() {
            return Err(format!(
                "场景 {} 的 default_horizon_roles 不能为空。",
                scenario.scenario_id
            ));
        }
        if scenario
            .default_horizon_roles
            .iter()
            .any(|role| !matches!(role, 5 | 20 | 60))
        {
            return Err(format!(
                "场景 {} 的 default_horizon_roles 只能包含 5、20、60。",
                scenario.scenario_id
            ));
        }
        if scenario.episode_template_id.is_none() {
            return Err(format!(
                "场景 {} 缺少 episode_template_id。",
                scenario.scenario_id
            ));
        }
        let mut protected_levels = HashSet::new();
        for level in &scenario.protected_action_levels {
            if !protected_levels.insert(*level) {
                return Err(format!(
                    "场景 {} 的 protected_action_levels 存在重复动作层级。",
                    scenario.scenario_id
                ));
            }
        }
        if let Some(overrides) = scenario.action_episode_overrides.as_ref() {
            validate_action_episode_override(
                &scenario.scenario_id,
                "prepare",
                overrides.prepare.as_ref(),
            )?;
            validate_action_episode_override(
                &scenario.scenario_id,
                "hedge",
                overrides.hedge.as_ref(),
            )?;
            validate_action_episode_override(
                &scenario.scenario_id,
                "defend",
                overrides.defend.as_ref(),
            )?;
        }
    }

    Ok(())
}

fn validate_action_episode_override(
    scenario_id: &str,
    level_name: &str,
    override_window: Option<&ActionEpisodeWindowOverride>,
) -> Result<(), String> {
    let Some(override_window) = override_window else {
        return Ok(());
    };

    if let (Some(primary_start), Some(primary_end)) =
        (override_window.primary_start, override_window.primary_end)
    {
        if primary_start > primary_end {
            return Err(format!(
                "场景 {} 的 {} action episode override 中 primary_start 晚于 primary_end。",
                scenario_id, level_name
            ));
        }
    }

    if override_window.primary_start.is_some() ^ override_window.primary_end.is_some() {
        return Err(format!(
            "场景 {} 的 {} action episode override 必须同时提供 primary_start 和 primary_end。",
            scenario_id, level_name
        ));
    }

    if let (Some(primary_end), Some(late_validation_end)) = (
        override_window.primary_end,
        override_window.late_validation_end,
    ) {
        if late_validation_end < primary_end {
            return Err(format!(
                "场景 {} 的 {} action episode override 中 late_validation_end 早于 primary_end。",
                scenario_id, level_name
            ));
        }
    }

    if let (Some(late_validation_end), Some(cooldown_end)) = (
        override_window.late_validation_end,
        override_window.cooldown_end,
    ) {
        if cooldown_end < late_validation_end {
            return Err(format!(
                "场景 {} 的 {} action episode override 中 cooldown_end 早于 late_validation_end。",
                scenario_id, level_name
            ));
        }
    }

    Ok(())
}

fn validate_label_sets(
    label_sets: &[CrisisScenarioLabelSet],
    scenarios: &[CrisisScenarioDefinition],
) -> Result<(), String> {
    validate_scenario_refs(
        label_sets.iter().map(|label_set| {
            (
                label_set.label_set_id.as_str(),
                label_set.scenario_ids.as_slice(),
                "label_set",
            )
        }),
        scenarios,
    )
}

fn validate_window_sets(
    window_sets: &[CrisisScenarioWindowSet],
    scenarios: &[CrisisScenarioDefinition],
) -> Result<(), String> {
    validate_scenario_refs(
        window_sets.iter().map(|window_set| {
            (
                window_set.window_set_id.as_str(),
                window_set.scenario_ids.as_slice(),
                "window_set",
            )
        }),
        scenarios,
    )
}

fn validate_scenario_refs<'a>(
    sets: impl Iterator<Item = (&'a str, &'a [String], &'a str)>,
    scenarios: &[CrisisScenarioDefinition],
) -> Result<(), String> {
    let known_ids = scenarios
        .iter()
        .map(|scenario| scenario.scenario_id.as_str())
        .collect::<HashSet<_>>();
    let mut set_ids = HashSet::new();

    for (set_id, scenario_ids, set_kind) in sets {
        if !set_ids.insert(format!("{set_kind}:{set_id}")) {
            return Err(format!("{set_kind} {set_id} 重复定义。"));
        }
        if scenario_ids.is_empty() {
            return Err(format!("{set_kind} {set_id} 不能为空。"));
        }

        let mut local_ids = HashSet::new();
        for scenario_id in scenario_ids {
            if !known_ids.contains(scenario_id.as_str()) {
                return Err(format!(
                    "{set_kind} {set_id} 引用了不存在的场景 {scenario_id}。"
                ));
            }
            if !local_ids.insert(scenario_id.as_str()) {
                return Err(format!(
                    "{set_kind} {set_id} 内部重复引用场景 {scenario_id}。"
                ));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::ActionabilityLevel;

    use super::{embedded_crisis_scenario_catalog, CrisisScenarioTrainingRole};

    #[test]
    fn embedded_catalog_contains_main_and_extension_sets() {
        let catalog = embedded_crisis_scenario_catalog();
        assert_eq!(catalog.catalog_id, "scenario_v1_main");
        assert!(catalog
            .scenarios
            .iter()
            .any(|scenario| scenario.scenario_id == "us_black_monday_1987"));
        assert!(catalog
            .scenarios
            .iter()
            .any(|scenario| scenario.training_role == CrisisScenarioTrainingRole::Mandatory));
        let main = catalog
            .scenarios_for_label_set("formal_label_v1_main")
            .expect("main label set");
        assert_eq!(main.len(), 3);
        let ext = catalog
            .scenarios_for_label_set("formal_label_v1_ext_acute")
            .expect("extension label set");
        assert_eq!(ext.len(), 2);
        let protected_ext = catalog
            .scenarios_for_label_set("formal_label_v1_ext_stress")
            .expect("protected extension label set");
        assert_eq!(protected_ext.len(), 4);
        let protected = catalog
            .scenarios
            .iter()
            .find(|scenario| scenario.scenario_id == "us_funding_stress_2011")
            .expect("protected scenario");
        assert_eq!(
            protected.protected_action_levels,
            vec![ActionabilityLevel::Prepare, ActionabilityLevel::Hedge]
        );
        assert!(protected.episode_template_id.is_some());
    }
}

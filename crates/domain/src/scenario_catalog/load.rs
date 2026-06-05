use std::{env, fs};

use serde::Deserialize;

use crate::ActionabilityLevel;

use super::validate::{validate_label_sets, validate_scenarios, validate_window_sets};
use super::{
    ActionEpisodeTemplateId, CrisisScenarioCatalog, CrisisScenarioDefinition, CrisisScenarioFamily,
    CrisisScenarioLabelSet, CrisisScenarioWindowSet,
};

const CRISIS_SCENARIO_CATALOG_ENV: &str = "FC_SCENARIO_CATALOG_PATH";
const EMBEDDED_SCENARIO_CATALOG_PATH: &str = "embedded:config/research_crisis_scenarios.us.json";
const EMBEDDED_SCENARIO_CATALOG_JSON: &str =
    include_str!("../../../../config/research_crisis_scenarios.us.json");

#[derive(Debug, Clone, Deserialize)]
struct CrisisScenarioCatalogFile {
    catalog_id: String,
    market_scope: String,
    note: String,
    label_sets: Vec<CrisisScenarioLabelSet>,
    window_sets: Vec<CrisisScenarioWindowSet>,
    scenarios: Vec<CrisisScenarioDefinition>,
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

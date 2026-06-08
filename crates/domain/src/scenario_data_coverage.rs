use std::{collections::BTreeSet, env, fs};

use serde::{Deserialize, Serialize};

use crate::load_crisis_scenario_catalog;

const SCENARIO_DATA_COVERAGE_ENV: &str = "FC_SCENARIO_DATA_COVERAGE_PATH";
const EMBEDDED_SCENARIO_DATA_COVERAGE_PATH: &str =
    "embedded:config/research_scenario_data_coverage.us.json";
const EMBEDDED_SCENARIO_DATA_COVERAGE_JSON: &str =
    include_str!("../../../config/research_scenario_data_coverage.us.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioDataCoverageRecord {
    pub scenario_id: String,
    pub scenario_label: String,
    pub recommended_role: String,
    pub coverage_grade: String,
    pub point_in_time_mode: String,
    pub usable_for_main_training: bool,
    pub usable_for_extension_training: bool,
    pub usable_for_protected_stress: bool,
    pub usable_for_historical_analog: bool,
    pub free_sources: Vec<String>,
    pub current_status: String,
    pub blocking_gaps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioDataCoverageCatalog {
    pub catalog_id: String,
    pub scenario_catalog_id: String,
    pub market_scope: String,
    pub note: String,
    pub source: String,
    pub warning: Option<String>,
    pub records: Vec<ScenarioDataCoverageRecord>,
}

#[derive(Debug, Clone, Deserialize)]
struct ScenarioDataCoverageCatalogFile {
    catalog_id: String,
    scenario_catalog_id: String,
    market_scope: String,
    note: String,
    records: Vec<ScenarioDataCoverageRecord>,
}

impl ScenarioDataCoverageCatalog {
    pub fn record_for_scenario(&self, scenario_id: &str) -> Option<&ScenarioDataCoverageRecord> {
        self.records
            .iter()
            .find(|record| record.scenario_id == scenario_id)
    }
}

pub fn load_scenario_data_coverage_catalog() -> ScenarioDataCoverageCatalog {
    match env::var(SCENARIO_DATA_COVERAGE_ENV) {
        Ok(path) => match load_scenario_data_coverage_catalog_from_file(&path) {
            Ok(catalog) => catalog,
            Err(error) => {
                let mut fallback = embedded_scenario_data_coverage_catalog();
                fallback.warning = Some(format!(
                    "无法从 {path} 加载历史场景数据覆盖配置，已退回内置默认配置：{error}"
                ));
                fallback
            }
        },
        Err(_) => embedded_scenario_data_coverage_catalog(),
    }
}

pub fn embedded_scenario_data_coverage_catalog() -> ScenarioDataCoverageCatalog {
    parse_scenario_data_coverage_catalog(
        EMBEDDED_SCENARIO_DATA_COVERAGE_JSON,
        EMBEDDED_SCENARIO_DATA_COVERAGE_PATH,
        None,
    )
    .expect("embedded scenario data coverage catalog must be valid")
}

pub fn load_scenario_data_coverage_catalog_from_file(
    path: &str,
) -> Result<ScenarioDataCoverageCatalog, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("读取历史场景数据覆盖配置失败: {error}"))?;
    parse_scenario_data_coverage_catalog(&raw, path, None)
}

fn parse_scenario_data_coverage_catalog(
    raw: &str,
    source: &str,
    warning: Option<String>,
) -> Result<ScenarioDataCoverageCatalog, String> {
    let parsed: ScenarioDataCoverageCatalogFile = serde_json::from_str(raw)
        .map_err(|error| format!("解析历史场景数据覆盖配置失败: {error}"))?;
    let crisis_catalog = load_crisis_scenario_catalog();
    validate_scenario_data_coverage_catalog(&parsed, &crisis_catalog)?;

    Ok(ScenarioDataCoverageCatalog {
        catalog_id: parsed.catalog_id,
        scenario_catalog_id: parsed.scenario_catalog_id,
        market_scope: parsed.market_scope,
        note: parsed.note,
        source: source.to_string(),
        warning,
        records: parsed.records,
    })
}

fn validate_scenario_data_coverage_catalog(
    catalog: &ScenarioDataCoverageCatalogFile,
    crisis_catalog: &crate::CrisisScenarioCatalog,
) -> Result<(), String> {
    if catalog.catalog_id.trim().is_empty() {
        return Err("历史场景数据覆盖配置缺少 catalog_id".to_string());
    }
    if catalog.market_scope != crisis_catalog.market_scope {
        return Err(format!(
            "历史场景数据覆盖配置 market_scope={} 与危机场景目录 market_scope={} 不一致",
            catalog.market_scope, crisis_catalog.market_scope
        ));
    }
    if catalog.scenario_catalog_id != crisis_catalog.catalog_id {
        return Err(format!(
            "历史场景数据覆盖配置 scenario_catalog_id={} 与危机场景目录 catalog_id={} 不一致",
            catalog.scenario_catalog_id, crisis_catalog.catalog_id
        ));
    }
    if catalog.records.is_empty() {
        return Err("历史场景数据覆盖配置至少要包含一条记录".to_string());
    }

    let mut seen = BTreeSet::new();
    for record in &catalog.records {
        if record.scenario_id.trim().is_empty() {
            return Err("历史场景数据覆盖记录缺少 scenario_id".to_string());
        }
        if !seen.insert(record.scenario_id.clone()) {
            return Err(format!(
                "历史场景数据覆盖配置里存在重复 scenario_id={}",
                record.scenario_id
            ));
        }
        let scenario = crisis_catalog
            .scenarios
            .iter()
            .find(|scenario| scenario.scenario_id == record.scenario_id)
            .ok_or_else(|| {
                format!(
                    "历史场景数据覆盖配置引用了危机场景目录中不存在的 scenario_id={}",
                    record.scenario_id
                )
            })?;
        if record.scenario_label.trim().is_empty() {
            return Err(format!(
                "历史场景数据覆盖记录 {} 缺少 scenario_label",
                record.scenario_id
            ));
        }
        if record.free_sources.is_empty() {
            return Err(format!(
                "历史场景数据覆盖记录 {} 至少要列出一个 free_sources",
                record.scenario_id
            ));
        }
        if !(record.usable_for_main_training
            || record.usable_for_extension_training
            || record.usable_for_protected_stress
            || record.usable_for_historical_analog)
        {
            return Err(format!(
                "历史场景数据覆盖记录 {} 至少要启用一种 usable_for_* 角色",
                record.scenario_id
            ));
        }
        if scenario.label != record.scenario_label {
            return Err(format!(
                "历史场景数据覆盖记录 {} 的 scenario_label={} 与危机场景目录 label={} 不一致",
                record.scenario_id, record.scenario_label, scenario.label
            ));
        }
    }

    let covered_ids = catalog
        .records
        .iter()
        .map(|record| record.scenario_id.as_str())
        .collect::<BTreeSet<_>>();
    for scenario in &crisis_catalog.scenarios {
        if !covered_ids.contains(scenario.scenario_id.as_str()) {
            return Err(format!(
                "历史场景数据覆盖配置缺少危机场景目录里的 scenario_id={}",
                scenario.scenario_id
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::embedded_scenario_data_coverage_catalog;

    #[test]
    fn embedded_scenario_data_coverage_catalog_covers_all_scenarios() {
        let catalog = embedded_scenario_data_coverage_catalog();
        assert_eq!(catalog.scenario_catalog_id, "scenario_v1_main");
        assert_eq!(catalog.market_scope, "financial_system");
        assert!(catalog
            .record_for_scenario("us_black_monday_1987")
            .is_some());
        assert!(catalog
            .record_for_scenario("us_regional_banks_2023")
            .is_some());
        assert_eq!(catalog.records.len(), 10);
    }
}

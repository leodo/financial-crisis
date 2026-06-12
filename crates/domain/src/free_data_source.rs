use std::{collections::HashSet, env, fs};

use serde::{Deserialize, Serialize};

const FREE_DATA_SOURCE_CATALOG_ENV: &str = "FC_FREE_DATA_SOURCE_CATALOG_PATH";
const EMBEDDED_FREE_DATA_SOURCE_CATALOG_PATH: &str =
    "embedded:config/free_data_source_catalog.us.json";
const EMBEDDED_FREE_DATA_SOURCE_CATALOG_JSON: &str =
    include_str!("../../../config/free_data_source_catalog.us.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreeDataSourceAlternative {
    pub source_id: String,
    pub dataset: String,
    pub access_tier: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreeDataSourceRecord {
    pub indicator_id: String,
    pub display_name: String,
    pub primary_source_id: String,
    pub primary_dataset: String,
    pub primary_access_tier: String,
    pub primary_timing_note: String,
    pub alternatives: Vec<FreeDataSourceAlternative>,
    pub missing_impact: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreeDataSourceCatalog {
    pub catalog_id: String,
    pub market_scope: String,
    pub note: String,
    pub source: String,
    pub warning: Option<String>,
    pub records: Vec<FreeDataSourceRecord>,
}

#[derive(Debug, Clone, Deserialize)]
struct FreeDataSourceCatalogFile {
    catalog_id: String,
    market_scope: String,
    note: String,
    records: Vec<FreeDataSourceRecord>,
}

impl FreeDataSourceCatalog {
    pub fn record_for_indicator(&self, indicator_id: &str) -> Option<&FreeDataSourceRecord> {
        self.records
            .iter()
            .find(|record| record.indicator_id == indicator_id)
    }
}

pub fn load_free_data_source_catalog() -> FreeDataSourceCatalog {
    match env::var(FREE_DATA_SOURCE_CATALOG_ENV) {
        Ok(path) => match load_free_data_source_catalog_from_file(&path) {
            Ok(catalog) => catalog,
            Err(error) => {
                let mut fallback = embedded_free_data_source_catalog();
                fallback.warning = Some(format!(
                    "无法从 {path} 加载免费数据源目录，已退回内置默认配置：{error}"
                ));
                fallback
            }
        },
        Err(_) => embedded_free_data_source_catalog(),
    }
}

pub fn embedded_free_data_source_catalog() -> FreeDataSourceCatalog {
    parse_catalog(
        EMBEDDED_FREE_DATA_SOURCE_CATALOG_JSON,
        EMBEDDED_FREE_DATA_SOURCE_CATALOG_PATH,
        None,
    )
    .expect("embedded free data source catalog must be valid")
}

pub fn load_free_data_source_catalog_from_file(
    path: &str,
) -> Result<FreeDataSourceCatalog, String> {
    let raw =
        fs::read_to_string(path).map_err(|error| format!("读取免费数据源目录失败: {error}"))?;
    parse_catalog(&raw, path, None)
}

fn parse_catalog(
    raw: &str,
    source: &str,
    warning: Option<String>,
) -> Result<FreeDataSourceCatalog, String> {
    let parsed: FreeDataSourceCatalogFile =
        serde_json::from_str(raw).map_err(|error| format!("解析免费数据源目录失败: {error}"))?;
    validate_records(&parsed.records)?;
    Ok(FreeDataSourceCatalog {
        catalog_id: parsed.catalog_id,
        market_scope: parsed.market_scope,
        note: parsed.note,
        source: source.to_string(),
        warning,
        records: parsed.records,
    })
}

fn validate_records(records: &[FreeDataSourceRecord]) -> Result<(), String> {
    if records.is_empty() {
        return Err("目录不能为空，至少需要一个关键指标的免费数据源记录。".to_string());
    }

    let mut ids = HashSet::new();
    for record in records {
        if record.primary_source_id.is_empty() {
            return Err(format!("指标 {} 缺少主源。", record.indicator_id));
        }
        if !ids.insert(record.indicator_id.clone()) {
            return Err(format!("指标 {} 重复定义。", record.indicator_id));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{embedded_free_data_source_catalog, load_free_data_source_catalog};

    #[test]
    fn embedded_catalog_covers_core_key_indicators() {
        let catalog = embedded_free_data_source_catalog();
        for indicator_id in [
            "us_external_usdjpy_level",
            "jp_rates_call_rate",
            "us_liquidity_effr",
            "us_market_vix_close",
        ] {
            assert!(
                catalog.record_for_indicator(indicator_id).is_some(),
                "missing catalog record for {indicator_id}"
            );
        }
    }

    #[test]
    fn usdjpy_has_fred_fallback_alternative() {
        let catalog = load_free_data_source_catalog();
        let record = catalog
            .record_for_indicator("us_external_usdjpy_level")
            .expect("usdjpy record must exist");
        assert!(record
            .alternatives
            .iter()
            .any(|alt| alt.source_id == "fred" && alt.dataset == "DEXJPUS"));
    }
}

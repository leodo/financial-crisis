use std::{collections::HashSet, env, fs};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

const PROTECTED_STRESS_WINDOWS_ENV: &str = "FC_PROTECTED_STRESS_WINDOWS_PATH";
const EMBEDDED_STRESS_WINDOWS_PATH: &str = "embedded:config/protected_stress_windows.us.json";
const EMBEDDED_STRESS_WINDOWS_JSON: &str =
    include_str!("../../../config/protected_stress_windows.us.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectedStressWindow {
    pub window_id: String,
    pub label: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectedStressWindowCatalog {
    pub catalog_id: String,
    pub market_scope: String,
    pub note: String,
    pub source: String,
    pub warning: Option<String>,
    pub windows: Vec<ProtectedStressWindow>,
}

#[derive(Debug, Clone, Deserialize)]
struct ProtectedStressWindowCatalogFile {
    catalog_id: String,
    market_scope: String,
    note: String,
    windows: Vec<ProtectedStressWindow>,
}

pub fn load_protected_stress_window_catalog() -> ProtectedStressWindowCatalog {
    match env::var(PROTECTED_STRESS_WINDOWS_ENV) {
        Ok(path) => match load_protected_stress_window_catalog_from_file(&path) {
            Ok(catalog) => catalog,
            Err(error) => {
                let mut fallback = embedded_protected_stress_window_catalog();
                fallback.warning = Some(format!(
                    "无法从 {path} 加载受保护压力窗口目录，已退回内置默认配置：{error}"
                ));
                fallback
            }
        },
        Err(_) => embedded_protected_stress_window_catalog(),
    }
}

pub fn embedded_protected_stress_window_catalog() -> ProtectedStressWindowCatalog {
    parse_catalog(
        EMBEDDED_STRESS_WINDOWS_JSON,
        EMBEDDED_STRESS_WINDOWS_PATH,
        None,
    )
    .expect("embedded protected stress window catalog must be valid")
}

pub fn load_protected_stress_window_catalog_from_file(
    path: &str,
) -> Result<ProtectedStressWindowCatalog, String> {
    let raw =
        fs::read_to_string(path).map_err(|error| format!("读取受保护压力窗口目录失败: {error}"))?;
    parse_catalog(&raw, path, None)
}

fn parse_catalog(
    raw: &str,
    source: &str,
    warning: Option<String>,
) -> Result<ProtectedStressWindowCatalog, String> {
    let mut parsed: ProtectedStressWindowCatalogFile = serde_json::from_str(raw)
        .map_err(|error| format!("解析受保护压力窗口目录失败: {error}"))?;
    validate_windows(&parsed.windows)?;
    parsed.windows.sort_by_key(|window| window.start_date);
    Ok(ProtectedStressWindowCatalog {
        catalog_id: parsed.catalog_id,
        market_scope: parsed.market_scope,
        note: parsed.note,
        source: source.to_string(),
        warning,
        windows: parsed.windows,
    })
}

fn validate_windows(windows: &[ProtectedStressWindow]) -> Result<(), String> {
    if windows.is_empty() {
        return Err("目录不能为空，至少需要一个受保护压力窗口。".to_string());
    }

    let mut ids = HashSet::new();
    for window in windows {
        if window.start_date > window.end_date {
            return Err(format!(
                "窗口 {} 的开始日期晚于结束日期。",
                window.window_id
            ));
        }
        if !ids.insert(window.window_id.clone()) {
            return Err(format!("窗口 {} 重复定义。", window.window_id));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::embedded_protected_stress_window_catalog;

    #[test]
    fn embedded_catalog_is_sorted_and_non_empty() {
        let catalog = embedded_protected_stress_window_catalog();
        assert!(!catalog.windows.is_empty());
        assert_eq!(
            catalog.windows[0].window_id,
            "us_dotcom_credit_aftershock_2002_2004"
        );
        assert!(catalog
            .windows
            .windows(2)
            .all(|pair| pair[0].start_date <= pair[1].start_date));
    }
}

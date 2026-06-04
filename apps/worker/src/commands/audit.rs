use std::{env, path::PathBuf};

use anyhow::{bail, Context, Result};
use chrono::Utc;
use fc_domain::{
    embedded_protected_stress_window_catalog, AssessmentMethodVersions, AssessmentSnapshot,
    BacktestScenarioSummary, ProtectedStressWindowCatalog,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub(crate) struct AuditExportOptions {
    pub(crate) api_base_url: String,
    pub(crate) output_dir: PathBuf,
}

impl AuditExportOptions {
    pub(crate) fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut api_base_url = env::var("FC_AUDIT_API_BASE_URL")
            .unwrap_or_else(|_| crate::DEFAULT_AUDIT_API_BASE_URL.to_string());
        let mut output_dir = PathBuf::from(crate::DEFAULT_AUDIT_OUTPUT_DIR);
        let mut index = 0;

        while index < args.len() {
            match args[index].as_str() {
                "--api-base-url" => {
                    index += 1;
                    api_base_url = args
                        .get(index)
                        .with_context(|| "--api-base-url requires a URL")?
                        .clone();
                }
                "--output-dir" => {
                    index += 1;
                    output_dir = PathBuf::from(
                        args.get(index)
                            .with_context(|| "--output-dir requires a path")?,
                    );
                }
                other => bail!("unknown audit export option: {other}"),
            }
            index += 1;
        }

        Ok(Self {
            api_base_url: api_base_url.trim_end_matches('/').to_string(),
            output_dir,
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct RuntimeThresholdDiagnosticsWire {
    pub(crate) prepare_p60d: f64,
    pub(crate) hedge_p20d: f64,
    pub(crate) defend_p5d: f64,
    pub(crate) severe_now_p20d: f64,
    pub(crate) elevated_weeks_p60d: f64,
    pub(crate) external_prepare_p20d: f64,
    pub(crate) carry_prepare_p60d: f64,
    pub(crate) downgrade_prepare_p60d: f64,
    pub(crate) downgrade_hedge_p20d: f64,
    pub(crate) downgrade_defend_p5d: f64,
    pub(crate) history_runtime_policy_version: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct AuditMethodResponse {
    pub(crate) method: AssessmentMethodVersions,
    pub(crate) note: String,
    pub(crate) protected_stress_window_catalog: ProtectedStressWindowCatalog,
    pub(crate) runtime_thresholds: Option<RuntimeThresholdDiagnosticsWire>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct AuditMethodResponseWire {
    pub(crate) method: AssessmentMethodVersions,
    pub(crate) note: String,
    pub(crate) protected_stress_window_catalog: Option<ProtectedStressWindowCatalog>,
    pub(crate) runtime_thresholds: Option<RuntimeThresholdDiagnosticsWire>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AuditExportEnvelope {
    pub(crate) exported_at: String,
    pub(crate) api_base_url: String,
    pub(crate) assessment: AssessmentSnapshot,
    pub(crate) backtests: Vec<BacktestScenarioSummary>,
    pub(crate) method: AuditMethodResponse,
}

pub(crate) async fn handle_audit_command(action: &str, rest: &[String]) -> Result<()> {
    match action {
        "export-current" => export_current_audit(rest).await,
        _ => {
            super::print_help();
            bail!("unknown audit command: {action}")
        }
    }
}

pub(crate) async fn export_current_audit(args: &[String]) -> anyhow::Result<()> {
    let options = AuditExportOptions::parse(args)?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    let assessment: AssessmentSnapshot =
        crate::fetch_api_json(&client, &options.api_base_url, "/api/assessment/current").await?;
    let backtests: Vec<BacktestScenarioSummary> =
        crate::fetch_api_json(&client, &options.api_base_url, "/api/backtests").await?;
    let method_wire: AuditMethodResponseWire =
        crate::fetch_api_json(&client, &options.api_base_url, "/api/assessment/method").await?;
    let method = AuditMethodResponse {
        method: method_wire.method,
        note: method_wire.note,
        protected_stress_window_catalog: method_wire
            .protected_stress_window_catalog
            .unwrap_or_else(|| {
                let mut catalog = embedded_protected_stress_window_catalog();
                catalog.warning = Some(
                    "运行中的 API 仍返回旧版 method 响应，导出命令已退回本地内置压力窗口目录；重启 API 后可获得完全一致的导出结果。"
                        .to_string(),
                );
                catalog
            }),
        runtime_thresholds: method_wire.runtime_thresholds,
    };

    let report = AuditExportEnvelope {
        exported_at: Utc::now().to_rfc3339(),
        api_base_url: options.api_base_url.clone(),
        assessment,
        backtests,
        method,
    };

    crate::reporting::write_rolling_audit_report(&options.output_dir, &report)?;
    println!(
        "  Summary  {}",
        report.assessment.backtest_summary.rolling_audit.summary
    );
    Ok(())
}

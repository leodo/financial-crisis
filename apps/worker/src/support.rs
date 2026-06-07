use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use chrono::{NaiveDate, Utc};
use fc_domain::{
    ActionabilityLevel, AssessmentSnapshot, BacktestSignalSource, DataMode, DecisionPosture,
    Frequency, ModelReleaseManifest, ProbabilityBundle, TimeToRiskBucket,
};
use fc_ingestion::{Connector, FetchPlan, MockConnector, RunMode};
use fc_storage::SqliteStore;
use serde::de::DeserializeOwned;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ApiReloadHistoryMode {
    Default,
    StrictRebuild,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ApiReloadRuntimePurpose {
    Production,
    Review,
}

impl ApiReloadRuntimePurpose {
    pub(crate) fn as_query_value(self) -> &'static str {
        match self {
            Self::Production => "production",
            Self::Review => "review",
        }
    }
}

impl ApiReloadHistoryMode {
    pub(crate) fn parse(value: &str) -> anyhow::Result<Self> {
        match value {
            "default" => Ok(Self::Default),
            "strict_rebuild" => Ok(Self::StrictRebuild),
            other => bail!("unsupported API reload history mode: {other}"),
        }
    }

    pub(crate) fn as_query_value(self) -> Option<&'static str> {
        match self {
            Self::Default => None,
            Self::StrictRebuild => Some("strict_rebuild"),
        }
    }

    pub(crate) fn as_label(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::StrictRebuild => "strict_rebuild",
        }
    }
}

pub(crate) fn formal_dataset_key(dataset_id: &str, dataset_version: &str) -> String {
    format!("{dataset_id}:{dataset_version}")
}

pub(crate) fn round3(value: f64) -> f64 {
    (value * 1_000.0).round() / 1_000.0
}

pub(crate) fn round6(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

pub(crate) fn safe_divide(numerator: f64, denominator: f64) -> f64 {
    if denominator.abs() <= f64::EPSILON {
        0.0
    } else {
        numerator / denominator
    }
}

pub(crate) fn safe_ratio(numerator: usize, denominator: usize) -> f64 {
    safe_divide(numerator as f64, denominator as f64)
}

pub(crate) fn actionability_level_text(level: ActionabilityLevel) -> &'static str {
    match level {
        ActionabilityLevel::Prepare => "prepare",
        ActionabilityLevel::Hedge => "hedge",
        ActionabilityLevel::Defend => "defend",
    }
}

pub(crate) async fn run_demo_ingestion() -> anyhow::Result<()> {
    let connector = MockConnector;
    let plan = FetchPlan {
        source_id: "mock".to_string(),
        dataset_id: "demo".to_string(),
        target_id: "us_market_vix_close".to_string(),
        external_code: None,
        run_mode: RunMode::Incremental,
        requested_start: Some(NaiveDate::from_ymd_opt(2026, 5, 1).expect("valid date")),
        requested_end: Some(NaiveDate::from_ymd_opt(2026, 5, 30).expect("valid date")),
        frequency: Frequency::Daily,
    };

    let payload = connector.fetch(&plan).await?;
    let batch = connector.parse(&plan, &payload)?;

    tracing::info!(
        source_id = %batch.source_id,
        dataset_id = %batch.dataset_id,
        records = batch.observations.len(),
        "worker completed one demo ingestion run"
    );
    println!("{}", serde_json::to_string_pretty(&batch)?);

    Ok(())
}

pub(crate) async fn fetch_api_json<T: DeserializeOwned>(
    client: &reqwest::Client,
    api_base_url: &str,
    path: &str,
) -> anyhow::Result<T> {
    let response = client
        .get(format!("{api_base_url}{path}"))
        .send()
        .await
        .with_context(|| format!("request to {api_base_url}{path} failed"))?;
    let status = response.status();
    if !status.is_success() {
        bail!("request to {api_base_url}{path} returned {status}");
    }
    response
        .json::<T>()
        .await
        .with_context(|| format!("failed to decode JSON from {api_base_url}{path}"))
}

pub(crate) async fn fetch_assessment_snapshot_for_guard(
    api_reload_url: &str,
) -> anyhow::Result<AssessmentSnapshot> {
    let api_base_url = api_reload_url
        .strip_suffix("/api/system/reload")
        .with_context(|| {
            format!(
                "cannot derive API base URL from reload URL {api_reload_url}; expected it to end with /api/system/reload"
            )
        })?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;
    fetch_api_json(&client, api_base_url, "/api/assessment/current").await
}

pub(crate) fn read_release_manifest(path: &Path) -> anyhow::Result<ModelReleaseManifest> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read release manifest {}", path.display()))?;
    serde_json::from_str::<ModelReleaseManifest>(&raw)
        .with_context(|| format!("failed to decode release manifest {}", path.display()))
}

pub(crate) fn read_probability_bundle(path: &Path) -> anyhow::Result<ProbabilityBundle> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read probability bundle {}", path.display()))?;
    serde_json::from_str::<ProbabilityBundle>(&raw)
        .with_context(|| format!("failed to decode probability bundle {}", path.display()))
}

pub(crate) fn truncate_text(value: &str, max_len: usize) -> String {
    if value.chars().count() <= max_len {
        return value.to_string();
    }
    let prefix_len = max_len.saturating_sub(1);
    let mut truncated = value.chars().take(prefix_len).collect::<String>();
    truncated.push('…');
    truncated
}

pub(crate) fn data_mode_text(mode: DataMode) -> &'static str {
    match mode {
        DataMode::Demo => "demo",
        DataMode::Sqlite => "sqlite",
        DataMode::Postgres => "postgres",
    }
}

pub(crate) fn posture_text(posture: DecisionPosture) -> &'static str {
    match posture {
        DecisionPosture::Normal => "normal",
        DecisionPosture::Prepare => "prepare",
        DecisionPosture::Hedge => "hedge",
        DecisionPosture::Defend => "defend",
    }
}

pub(crate) fn time_bucket_text(bucket: TimeToRiskBucket) -> &'static str {
    match bucket {
        TimeToRiskBucket::Normal => "normal",
        TimeToRiskBucket::Months => "months",
        TimeToRiskBucket::Weeks => "weeks",
        TimeToRiskBucket::Now => "now",
    }
}

pub(crate) fn backtest_signal_source_text(source: BacktestSignalSource) -> &'static str {
    match source {
        BacktestSignalSource::RealHistory => "real_history",
        BacktestSignalSource::FallbackTemplate => "fallback_template",
    }
}

pub(crate) fn format_pct(value: f64) -> String {
    format!("{:.1}%", value * 100.0)
}

pub(crate) fn format_optional_pct(value: Option<f64>) -> String {
    value.map(format_pct).unwrap_or_else(|| "—".to_string())
}

pub(crate) fn format_score(value: f64) -> String {
    format!("{value:.1}")
}

pub(crate) fn format_optional_score(value: Option<f64>) -> String {
    value.map(format_score).unwrap_or_else(|| "—".to_string())
}

pub(crate) fn format_optional_date(value: Option<NaiveDate>) -> String {
    value
        .map(|date| date.to_string())
        .unwrap_or_else(|| "—".to_string())
}

pub(crate) fn format_optional_date_with_reason(
    value: Option<NaiveDate>,
    reason: Option<&str>,
) -> String {
    match (value, reason) {
        (Some(date), Some(reason)) if !reason.is_empty() => format!("{date} ({reason})"),
        (Some(date), _) => date.to_string(),
        _ => "—".to_string(),
    }
}

pub(crate) fn format_optional_date_with_lead(
    value: Option<NaiveDate>,
    crisis_start: NaiveDate,
) -> String {
    value
        .map(|date| {
            let lead_days = crisis_start.signed_duration_since(date).num_days();
            format!("{date} ({lead_days}d)")
        })
        .unwrap_or_else(|| "—".to_string())
}

pub(crate) fn format_bool_flag(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

pub(crate) fn format_optional_bool_flag(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "yes",
        Some(false) => "no",
        None => "—",
    }
}

pub(crate) fn format_optional_count(value: Option<u32>) -> String {
    value
        .map(|count| count.to_string())
        .unwrap_or_else(|| "—".to_string())
}

pub(crate) fn format_trigger_codes(codes: &[String]) -> String {
    if codes.is_empty() {
        "—".to_string()
    } else {
        codes.join(", ")
    }
}

pub(crate) fn format_optional_multiplier(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.2}x"))
        .unwrap_or_else(|| "—".to_string())
}

pub(crate) fn format_optional_ratio(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.2}"))
        .unwrap_or_else(|| "—".to_string())
}

pub(crate) fn format_signed_pct_delta(value: f64) -> String {
    format!("{:+.1}pp", value * 100.0)
}

pub(crate) fn format_signed_count_delta(value: i64) -> String {
    format!("{value:+}")
}

pub(crate) fn format_optional_days(value: Option<i64>) -> String {
    value
        .map(|days| format!("{days}d"))
        .unwrap_or_else(|| "—".to_string())
}

pub(crate) fn parse_date_arg(value: Option<&String>, option: &str) -> anyhow::Result<NaiveDate> {
    let value = value.with_context(|| format!("{option} requires a YYYY-MM-DD value"))?;
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .with_context(|| format!("{option} must use YYYY-MM-DD"))
}

pub(crate) fn parse_positive_i64(value: Option<&String>, option: &str) -> anyhow::Result<i64> {
    let value = value
        .with_context(|| format!("{option} requires a positive integer"))?
        .parse::<i64>()
        .with_context(|| format!("{option} requires a positive integer"))?;
    if value <= 0 {
        bail!("{option} requires a positive integer");
    }
    Ok(value)
}

pub(crate) async fn open_sqlite_store() -> anyhow::Result<SqliteStore> {
    SqliteStore::connect(sqlite_path())
        .await
        .map_err(Into::into)
}

pub(crate) fn sqlite_path() -> String {
    env::var("FC_SQLITE_PATH").unwrap_or_else(|_| crate::DEFAULT_SQLITE_PATH.to_string())
}

pub(crate) fn raw_data_dir() -> PathBuf {
    env::var("FC_RAW_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(crate::DEFAULT_RAW_DATA_DIR))
}

pub(crate) fn write_raw_payload(
    raw_root: &Path,
    source_id: &str,
    external_code: &str,
    extension: &str,
    body: &str,
) -> anyhow::Result<PathBuf> {
    let year = Utc::now().format("%Y").to_string();
    let directory = raw_root.join(source_id).join(external_code).join(year);
    fs::create_dir_all(&directory)?;
    let path = directory.join(format!("{}.{}", simple_hash(body), extension));
    fs::write(&path, body)?;
    Ok(path)
}

pub(crate) fn raw_file_extension(content_type: &str) -> &'static str {
    if content_type.contains("csv") {
        "csv"
    } else if content_type.contains("json") {
        "json"
    } else if content_type.contains("xml") {
        "xml"
    } else {
        "txt"
    }
}

pub(crate) fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub(crate) fn simple_hash(input: &str) -> String {
    let hash = input.as_bytes().iter().fold(0_u64, |acc, byte| {
        acc.wrapping_mul(31).wrapping_add(*byte as u64)
    });
    format!("{hash:x}")
}

pub(crate) async fn reload_api_runtime(url: &str) -> anyhow::Result<()> {
    reload_api_runtime_with_history_mode(url, ApiReloadHistoryMode::Default).await
}

pub(crate) async fn reload_api_runtime_with_history_mode(
    url: &str,
    history_mode: ApiReloadHistoryMode,
) -> anyhow::Result<()> {
    reload_api_runtime_with_options(url, history_mode, None, ApiReloadRuntimePurpose::Production)
        .await
}

pub(crate) async fn reload_api_runtime_with_history_options(
    url: &str,
    history_mode: ApiReloadHistoryMode,
    history_limit: Option<usize>,
) -> anyhow::Result<()> {
    reload_api_runtime_with_options(
        url,
        history_mode,
        history_limit,
        ApiReloadRuntimePurpose::Review,
    )
    .await
}

pub(crate) async fn reload_api_runtime_with_options(
    url: &str,
    history_mode: ApiReloadHistoryMode,
    history_limit: Option<usize>,
    runtime_purpose: ApiReloadRuntimePurpose,
) -> anyhow::Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(1_200))
        .build()?;
    let request = client.post(url);
    let mut query = Vec::<(&str, String)>::new();
    if let Some(history_mode) = history_mode.as_query_value() {
        query.push(("history_mode", history_mode.to_string()));
    }
    if let Some(history_limit) = history_limit {
        query.push(("history_limit", history_limit.to_string()));
    }
    if runtime_purpose != ApiReloadRuntimePurpose::Production {
        query.push((
            "runtime_purpose",
            runtime_purpose.as_query_value().to_string(),
        ));
    }
    let request = if query.is_empty() {
        request
    } else {
        request.query(&query)
    };
    let response = request.send().await?;
    if !response.status().is_success() {
        bail!("reload endpoint returned {}", response.status());
    }
    Ok(())
}

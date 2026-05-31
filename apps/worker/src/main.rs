use std::{
    collections::BTreeMap,
    env,
    fmt::Write,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use chrono::{NaiveDate, Utc};
use fc_domain::{
    embedded_protected_stress_window_catalog, AssessmentMethodVersions, AssessmentSnapshot,
    BacktestScenarioSummary, Frequency, HorizonEvaluationSummary, LogisticProbabilityModel,
    ModelReleaseManifest, ModelReleaseRecord, PlattCalibrationArtifact, PredictionSnapshotRecord,
    ProbabilityBundle, ProbabilityBundleEvaluation, ProbabilityCoefficient, ProbabilityFeatureStat,
    ProbabilityHorizonBundle, ProtectedStressWindowCatalog, FEATURE_BUCKET_MONTHS_OR_HIGHER,
    FEATURE_BUCKET_NOW, FEATURE_BUCKET_WEEKS_OR_HIGHER, FEATURE_COVERAGE_SCORE,
    FEATURE_EXTERNAL_SHOCK_SCORE, FEATURE_FRESHNESS_DELAYED_OR_WORSE,
    FEATURE_FRESHNESS_STALE_OR_MISSING, FEATURE_HEURISTIC_P_20D, FEATURE_HEURISTIC_P_5D,
    FEATURE_HEURISTIC_P_60D, FEATURE_OVERALL_SCORE, PROBABILITY_BUNDLE_FEATURES,
};
use fc_ingestion::{
    BojConnector, BojDataset, Connector, FetchPlan, FredConnector, FredGraphCsvConnector,
    GdeltConnector, MockConnector, RunMode, SecEdgarConnector, TreasuryYieldCurveConnector,
    WorldBankConnector,
};
use fc_storage::{
    ExternalIndicatorMapping, RawResponseRecord, SqliteStore, BOJ_FX_DATASET_ID,
    BOJ_MONEY_MARKET_DATASET_ID, FRED_DATASET_ID, GDELT_DOC_DATASET_ID, SEC_EVENTS_DATASET_ID,
    SEC_SUBMISSIONS_DATASET_ID, TREASURY_YIELD_DATASET_ID, WORLD_BANK_DATASET_ID,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use uuid::Uuid;

const DEFAULT_SQLITE_PATH: &str = "data/fc-local.sqlite";
const DEFAULT_RAW_DATA_DIR: &str = "data/raw";
const DEFAULT_API_RELOAD_URL: &str = "http://127.0.0.1:18080/api/system/reload";
const DEFAULT_AUDIT_API_BASE_URL: &str = "http://127.0.0.1:18080";
const DEFAULT_AUDIT_OUTPUT_DIR: &str = "reports/rolling-audit";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_target(false).init();

    let args = env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [] => run_demo_ingestion().await,
        [scope, action] if scope == "db" && action == "init" => db_init().await,
        [scope, action] if scope == "db" && action == "seed" => db_seed().await,
        [scope, action] if scope == "db" && action == "check" => db_check().await,
        [scope, action, rest @ ..] if scope == "audit" && action == "export-current" => {
            export_current_audit(rest).await
        }
        [scope, area, action, rest @ ..] if scope == "research" && area == "release" => {
            match action.as_str() {
                "publish" => research_release_publish(rest).await,
                "list" => research_release_list(rest).await,
                "show" => research_release_show(rest).await,
                "activate" => research_release_activate(rest).await,
                "rollback" => research_release_rollback(rest).await,
                _ => {
                    print_help();
                    bail!("unknown research release command")
                }
            }
        }
        [scope, area, action, rest @ ..] if scope == "research" && area == "snapshot" => {
            match action.as_str() {
                "list" => research_prediction_snapshot_list(rest).await,
                "export" => research_prediction_snapshot_export(rest).await,
                "dataset" => research_prediction_snapshot_dataset(rest).await,
                _ => {
                    print_help();
                    bail!("unknown research snapshot command")
                }
            }
        }
        [scope, area, action, rest @ ..] if scope == "research" && area == "pipeline" => {
            match action.as_str() {
                "train-probability" => research_pipeline_train_probability(rest).await,
                "bootstrap-formal-release" => {
                    research_pipeline_bootstrap_formal_release(rest).await
                }
                _ => {
                    print_help();
                    bail!("unknown research pipeline command")
                }
            }
        }
        [scope, action, rest @ ..] if scope == "refresh" && action == "latest-free" => {
            refresh_latest_free(rest).await
        }
        [scope, source, rest @ ..] if scope == "backfill" && source == "fred" => {
            backfill_fred(rest).await
        }
        [scope, source, rest @ ..] if scope == "backfill" && source == "treasury-yield" => {
            backfill_treasury_yield(rest).await
        }
        [scope, source, rest @ ..] if scope == "backfill" && source == "world-bank" => {
            backfill_world_bank(rest).await
        }
        [scope, source, rest @ ..] if scope == "backfill" && source == "gdelt" => {
            backfill_gdelt(rest).await
        }
        [scope, source, rest @ ..] if scope == "backfill" && source == "sec-edgar" => {
            backfill_sec_edgar(rest).await
        }
        [scope, source, rest @ ..] if scope == "backfill" && source == "boj" => {
            backfill_boj(rest).await
        }
        [scope, source, rest @ ..] if scope == "backfill" && source == "jpy-carry" => {
            backfill_jpy_carry(rest).await
        }
        [scope, ..] if scope == "help" || scope == "--help" || scope == "-h" => {
            print_help();
            Ok(())
        }
        _ => {
            print_help();
            bail!("unknown worker command")
        }
    }
}

async fn db_init() -> anyhow::Result<()> {
    let store = open_sqlite_store().await?;
    store.migrate().await?;
    println!("SQLite database initialized at {}", sqlite_path());
    Ok(())
}

async fn db_seed() -> anyhow::Result<()> {
    let store = open_sqlite_store().await?;
    store.migrate().await?;
    store.seed_fred_metadata().await?;
    println!(
        "Seeded FRED, Treasury, BOJ, SEC EDGAR, and World Bank metadata into {}",
        sqlite_path()
    );
    Ok(())
}

async fn db_check() -> anyhow::Result<()> {
    let store = open_sqlite_store().await?;
    store.migrate().await?;
    let as_of_date = Utc::now().date_naive();
    let observations = store
        .load_observations_for_entities(&["us", "jp"], as_of_date)
        .await?;
    if observations.is_empty() {
        bail!(
            "SQLite has no observations yet. Run `just bootstrap-sqlite` then backfill free sources."
        );
    }

    let refill_start = as_of_date - chrono::Duration::days(540);
    let refill_end = as_of_date;
    let checks = vec![
        (
            "us_external_usdjpy_level",
            "us",
            "USDJPY",
            3_i64,
            format!("just backfill-boj-fx-range {refill_start} {refill_end}"),
        ),
        (
            "jp_rates_call_rate",
            "jp",
            "日本无担保隔夜拆借利率",
            5_i64,
            format!("just backfill-boj-money-market-range {refill_start} {refill_end}"),
        ),
        (
            "us_liquidity_effr",
            "us",
            "有效联邦基金利率",
            5_i64,
            format!("just backfill-fred-range {refill_start} {refill_end}"),
        ),
        (
            "us_market_vix_close",
            "us",
            "VIX",
            3_i64,
            format!("just backfill-fred-range {refill_start} {refill_end}"),
        ),
        (
            "us_event_official_filing_severity",
            "us",
            "SEC 银行公告严重度",
            7_i64,
            format!("just backfill-sec-edgar-range {refill_start} {refill_end}"),
        ),
    ];

    println!("SQLite health check as of {as_of_date}");
    for (indicator_id, entity_id, display_name, stale_days, refill_hint) in checks {
        let latest = observations
            .iter()
            .filter(|observation| observation.indicator_id == indicator_id)
            .filter(|observation| observation.entity_id == entity_id)
            .max_by_key(|observation| observation.as_of_date);
        match latest {
            Some(observation) => {
                let lag_days = (as_of_date - observation.as_of_date).num_days();
                let status = if lag_days > stale_days * 3 {
                    "STALE"
                } else if lag_days > stale_days {
                    "DELAYED"
                } else {
                    "FRESH"
                };
                println!(
                    "[{}] {} => {} {} @ {} (source={} dataset={} lag={}d)",
                    status,
                    display_name,
                    observation.value,
                    observation.unit,
                    observation.as_of_date,
                    observation.source_id,
                    observation.dataset_id,
                    lag_days
                );
                if status != "FRESH" {
                    println!("  quick refresh: just refresh-latest");
                    println!("  refresh with: {refill_hint}");
                }
            }
            None => {
                println!("[MISSING] {display_name} => no data");
                println!("  quick refresh: just refresh-latest");
                println!("  backfill with: {refill_hint}");
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct AuditExportOptions {
    api_base_url: String,
    output_dir: PathBuf,
}

#[derive(Debug, Clone)]
struct ReleasePublishOptions {
    manifest_path: PathBuf,
    activate: bool,
    reload_api: bool,
    api_reload_url: String,
    updated_by: String,
}

impl ReleasePublishOptions {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut manifest_path = None;
        let mut activate = false;
        let mut reload_api = false;
        let mut api_reload_url = DEFAULT_API_RELOAD_URL.to_string();
        let mut updated_by = "fc-worker".to_string();
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--manifest" => {
                    index += 1;
                    manifest_path = Some(PathBuf::from(
                        args.get(index)
                            .with_context(|| "--manifest requires a file path")?,
                    ));
                }
                "--activate" => activate = true,
                "--reload-api" => reload_api = true,
                "--api-reload-url" => {
                    index += 1;
                    api_reload_url = args
                        .get(index)
                        .with_context(|| "--api-reload-url requires a URL")?
                        .clone();
                }
                "--updated-by" => {
                    index += 1;
                    updated_by = args
                        .get(index)
                        .with_context(|| "--updated-by requires a value")?
                        .clone();
                }
                other => bail!("unknown release publish option: {other}"),
            }
            index += 1;
        }
        Ok(Self {
            manifest_path: manifest_path.with_context(|| "--manifest is required")?,
            activate,
            reload_api,
            api_reload_url,
            updated_by,
        })
    }
}

#[derive(Debug, Clone)]
struct ReleaseListOptions {
    market_scope: Option<String>,
}

impl ReleaseListOptions {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut market_scope = None;
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--market-scope" => {
                    index += 1;
                    market_scope = Some(
                        args.get(index)
                            .with_context(|| "--market-scope requires a value")?
                            .clone(),
                    );
                }
                other => bail!("unknown release list option: {other}"),
            }
            index += 1;
        }
        Ok(Self { market_scope })
    }
}

#[derive(Debug, Clone)]
struct ReleaseShowOptions {
    release_id: String,
}

impl ReleaseShowOptions {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut release_id = None;
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--release-id" => {
                    index += 1;
                    release_id = Some(
                        args.get(index)
                            .with_context(|| "--release-id requires a value")?
                            .clone(),
                    );
                }
                other => bail!("unknown release show option: {other}"),
            }
            index += 1;
        }
        Ok(Self {
            release_id: release_id.with_context(|| "--release-id is required")?,
        })
    }
}

#[derive(Debug, Clone)]
struct ReleaseSwitchOptions {
    release_id: String,
    market_scope: Option<String>,
    reload_api: bool,
    api_reload_url: String,
    updated_by: String,
}

impl ReleaseSwitchOptions {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut release_id = None;
        let mut market_scope = None;
        let mut reload_api = false;
        let mut api_reload_url = DEFAULT_API_RELOAD_URL.to_string();
        let mut updated_by = "fc-worker".to_string();
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--release-id" | "--to-release-id" => {
                    index += 1;
                    release_id = Some(
                        args.get(index)
                            .with_context(|| "--release-id/--to-release-id requires a value")?
                            .clone(),
                    );
                }
                "--market-scope" => {
                    index += 1;
                    market_scope = Some(
                        args.get(index)
                            .with_context(|| "--market-scope requires a value")?
                            .clone(),
                    );
                }
                "--reload-api" => reload_api = true,
                "--api-reload-url" => {
                    index += 1;
                    api_reload_url = args
                        .get(index)
                        .with_context(|| "--api-reload-url requires a URL")?
                        .clone();
                }
                "--updated-by" => {
                    index += 1;
                    updated_by = args
                        .get(index)
                        .with_context(|| "--updated-by requires a value")?
                        .clone();
                }
                other => bail!("unknown release switch option: {other}"),
            }
            index += 1;
        }
        Ok(Self {
            release_id: release_id.with_context(|| "--release-id/--to-release-id is required")?,
            market_scope,
            reload_api,
            api_reload_url,
            updated_by,
        })
    }
}

#[derive(Debug, Clone)]
struct PredictionSnapshotQueryOptions {
    market_scope: Option<String>,
    release_id: Option<String>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    limit: Option<usize>,
}

impl PredictionSnapshotQueryOptions {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        Self::parse_with_default_limit(args, Some(20))
    }

    fn parse_with_default_limit(
        args: &[String],
        default_limit: Option<usize>,
    ) -> anyhow::Result<Self> {
        let mut market_scope = None;
        let mut release_id = None;
        let mut from = None;
        let mut to = None;
        let mut limit = default_limit;
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--market-scope" => {
                    index += 1;
                    market_scope = Some(
                        args.get(index)
                            .with_context(|| "--market-scope requires a value")?
                            .clone(),
                    );
                }
                "--release-id" => {
                    index += 1;
                    release_id = Some(
                        args.get(index)
                            .with_context(|| "--release-id requires a value")?
                            .clone(),
                    );
                }
                "--from" => {
                    index += 1;
                    from = Some(parse_date_arg(args.get(index), "--from")?);
                }
                "--to" => {
                    index += 1;
                    to = Some(parse_date_arg(args.get(index), "--to")?);
                }
                "--limit" => {
                    index += 1;
                    limit = Some(
                        args.get(index)
                            .with_context(|| "--limit requires a number")?
                            .parse::<usize>()
                            .context("--limit must be an integer")?,
                    );
                }
                other => bail!("unknown prediction snapshot query option: {other}"),
            }
            index += 1;
        }
        Ok(Self {
            market_scope,
            release_id,
            from,
            to,
            limit,
        })
    }
}

#[derive(Debug, Clone, Copy)]
enum ExportFormat {
    Json,
    Csv,
}

impl ExportFormat {
    fn parse(raw: &str) -> anyhow::Result<Self> {
        match raw {
            "json" => Ok(Self::Json),
            "csv" => Ok(Self::Csv),
            other => bail!("unsupported format: {other}"),
        }
    }
}

#[derive(Debug, Clone)]
struct PredictionSnapshotExportOptions {
    query: PredictionSnapshotQueryOptions,
    format: ExportFormat,
    output_path: Option<PathBuf>,
}

impl PredictionSnapshotExportOptions {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut output_path = None;
        let mut format = ExportFormat::Json;
        let mut query_args = Vec::new();
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--output-path" => {
                    index += 1;
                    output_path = Some(PathBuf::from(
                        args.get(index)
                            .with_context(|| "--output-path requires a path")?,
                    ));
                }
                "--format" => {
                    index += 1;
                    format = ExportFormat::parse(
                        args.get(index)
                            .with_context(|| "--format requires json or csv")?,
                    )?;
                }
                other => query_args.push(other.to_string()),
            }
            index += 1;
        }

        Ok(Self {
            query: PredictionSnapshotQueryOptions::parse(&query_args)?,
            format,
            output_path,
        })
    }
}

#[derive(Debug, Clone)]
struct SnapshotDatasetExportOptions {
    query: PredictionSnapshotQueryOptions,
    format: ExportFormat,
    output_path: Option<PathBuf>,
}

impl SnapshotDatasetExportOptions {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut output_path = None;
        let mut format = ExportFormat::Json;
        let mut query_args = Vec::new();
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--output-path" => {
                    index += 1;
                    output_path = Some(PathBuf::from(
                        args.get(index)
                            .with_context(|| "--output-path requires a path")?,
                    ));
                }
                "--format" => {
                    index += 1;
                    format = ExportFormat::parse(
                        args.get(index)
                            .with_context(|| "--format requires json or csv")?,
                    )?;
                }
                other => query_args.push(other.to_string()),
            }
            index += 1;
        }

        Ok(Self {
            query: PredictionSnapshotQueryOptions::parse_with_default_limit(&query_args, None)?,
            format,
            output_path,
        })
    }
}

#[derive(Debug, Clone)]
struct PipelineTrainOptions {
    query: PredictionSnapshotQueryOptions,
    output_dir: PathBuf,
    release_prefix: String,
}

impl PipelineTrainOptions {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut output_dir = PathBuf::from("config/model-bundles/generated");
        let mut release_prefix = "us_formal_transitional".to_string();
        let mut query_args = Vec::new();
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--output-dir" => {
                    index += 1;
                    output_dir = PathBuf::from(
                        args.get(index)
                            .with_context(|| "--output-dir requires a path")?,
                    );
                }
                "--release-prefix" => {
                    index += 1;
                    release_prefix = args
                        .get(index)
                        .with_context(|| "--release-prefix requires a value")?
                        .clone();
                }
                other => query_args.push(other.to_string()),
            }
            index += 1;
        }

        Ok(Self {
            query: PredictionSnapshotQueryOptions::parse_with_default_limit(&query_args, None)?,
            output_dir,
            release_prefix,
        })
    }
}

#[derive(Debug, Clone)]
struct PipelineBootstrapOptions {
    train: PipelineTrainOptions,
    activate: bool,
    reload_api: bool,
    api_reload_url: String,
    updated_by: String,
}

impl PipelineBootstrapOptions {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut activate = true;
        let mut reload_api = true;
        let mut api_reload_url = DEFAULT_API_RELOAD_URL.to_string();
        let mut updated_by = "fc-worker".to_string();
        let mut train_args = Vec::new();
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--no-activate" => activate = false,
                "--no-reload-api" => reload_api = false,
                "--api-reload-url" => {
                    index += 1;
                    api_reload_url = args
                        .get(index)
                        .with_context(|| "--api-reload-url requires a URL")?
                        .clone();
                }
                "--updated-by" => {
                    index += 1;
                    updated_by = args
                        .get(index)
                        .with_context(|| "--updated-by requires a value")?
                        .clone();
                }
                other => train_args.push(other.to_string()),
            }
            index += 1;
        }

        Ok(Self {
            train: PipelineTrainOptions::parse(&train_args)?,
            activate,
            reload_api,
            api_reload_url,
            updated_by,
        })
    }
}

impl AuditExportOptions {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut api_base_url = env::var("FC_AUDIT_API_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_AUDIT_API_BASE_URL.to_string());
        let mut output_dir = PathBuf::from(DEFAULT_AUDIT_OUTPUT_DIR);
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
struct AuditMethodResponse {
    method: AssessmentMethodVersions,
    note: String,
    protected_stress_window_catalog: ProtectedStressWindowCatalog,
}

#[derive(Debug, Clone, Deserialize)]
struct AuditMethodResponseWire {
    method: AssessmentMethodVersions,
    note: String,
    protected_stress_window_catalog: Option<ProtectedStressWindowCatalog>,
}

#[derive(Debug, Clone, Serialize)]
struct AuditExportEnvelope {
    exported_at: String,
    api_base_url: String,
    assessment: AssessmentSnapshot,
    backtests: Vec<BacktestScenarioSummary>,
    method: AuditMethodResponse,
}

async fn export_current_audit(args: &[String]) -> anyhow::Result<()> {
    let options = AuditExportOptions::parse(args)?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    let assessment: AssessmentSnapshot =
        fetch_api_json(&client, &options.api_base_url, "/api/assessment/current").await?;
    let backtests: Vec<BacktestScenarioSummary> =
        fetch_api_json(&client, &options.api_base_url, "/api/backtests").await?;
    let method_wire: AuditMethodResponseWire =
        fetch_api_json(&client, &options.api_base_url, "/api/assessment/method").await?;
    let method = AuditMethodResponse {
        method: method_wire.method,
        note: method_wire.note,
        protected_stress_window_catalog: method_wire.protected_stress_window_catalog.unwrap_or_else(
            || {
                let mut catalog = embedded_protected_stress_window_catalog();
                catalog.warning = Some(
                    "运行中的 API 仍返回旧版 method 响应，导出命令已退回本地内置压力窗口目录；重启 API 后可获得完全一致的导出结果。"
                        .to_string(),
                );
                catalog
            },
        ),
    };

    let report = AuditExportEnvelope {
        exported_at: Utc::now().to_rfc3339(),
        api_base_url: options.api_base_url.clone(),
        assessment,
        backtests,
        method,
    };

    fs::create_dir_all(&options.output_dir)?;
    let stem = format!("{}-rolling-audit", report.assessment.as_of_date);
    let json_path = options.output_dir.join(format!("{stem}.json"));
    let markdown_path = options.output_dir.join(format!("{stem}.md"));
    fs::write(&json_path, serde_json::to_string_pretty(&report)?)?;
    fs::write(&markdown_path, render_audit_report_markdown(&report))?;

    println!("Rolling audit report exported.");
    println!("  JSON     {}", json_path.display());
    println!("  Markdown {}", markdown_path.display());
    println!(
        "  Summary  {}",
        report.assessment.backtest_summary.rolling_audit.summary
    );
    Ok(())
}

async fn research_release_publish(args: &[String]) -> anyhow::Result<()> {
    let options = ReleasePublishOptions::parse(args)?;
    let store = open_sqlite_store().await?;
    store.migrate().await?;
    let manifest = read_release_manifest(&options.manifest_path)?;
    let record = ModelReleaseRecord {
        manifest,
        created_at: Utc::now(),
        activated_at: None,
        retired_at: None,
    };
    store.upsert_model_release(&record).await?;
    println!(
        "Saved release {} for market scope {}.",
        record.manifest.release_id, record.manifest.market_scope
    );
    println!("  Bundle     {}", record.manifest.bundle_uri);
    println!("  Prob mode  {}", record.manifest.probability_mode);
    println!("  PIT mode   {}", record.manifest.point_in_time_mode);

    if options.activate {
        let activated = store
            .activate_model_release(
                &record.manifest.market_scope,
                &record.manifest.release_id,
                &options.updated_by,
            )
            .await?;
        println!(
            "Activated release {} for {}.",
            activated.manifest.release_id, activated.manifest.market_scope
        );
        if options.reload_api {
            reload_api_runtime(&options.api_reload_url).await?;
            println!("Reloaded API runtime via {}.", options.api_reload_url);
        }
    }

    Ok(())
}

async fn research_release_list(args: &[String]) -> anyhow::Result<()> {
    let options = ReleaseListOptions::parse(args)?;
    let store = open_sqlite_store().await?;
    store.migrate().await?;
    let releases = store
        .list_model_releases(options.market_scope.as_deref())
        .await?;
    if releases.is_empty() {
        println!("No model releases found.");
        return Ok(());
    }
    println!(
        "{:<32} {:<18} {:<12} {:<12} {:<16} {:<24}",
        "release_id", "market_scope", "status", "serving", "prob_mode", "created_at"
    );
    for release in releases {
        println!(
            "{:<32} {:<18} {:<12} {:<12} {:<16} {:<24}",
            truncate_text(&release.manifest.release_id, 32),
            truncate_text(&release.manifest.market_scope, 18),
            truncate_text(&release.manifest.status, 12),
            truncate_text(&release.manifest.serving_status, 12),
            truncate_text(&release.manifest.probability_mode, 16),
            release.created_at.to_rfc3339()
        );
    }
    Ok(())
}

async fn research_release_show(args: &[String]) -> anyhow::Result<()> {
    let options = ReleaseShowOptions::parse(args)?;
    let store = open_sqlite_store().await?;
    store.migrate().await?;
    let release = store
        .load_model_release(&options.release_id)
        .await?
        .with_context(|| format!("release {} not found", options.release_id))?;
    println!("{}", serde_json::to_string_pretty(&release)?);
    Ok(())
}

async fn research_release_activate(args: &[String]) -> anyhow::Result<()> {
    let options = ReleaseSwitchOptions::parse(args)?;
    let store = open_sqlite_store().await?;
    store.migrate().await?;
    let market_scope =
        resolve_release_market_scope(&store, &options.release_id, options.market_scope.as_deref())
            .await?;
    let activated = store
        .activate_model_release(&market_scope, &options.release_id, &options.updated_by)
        .await?;
    println!(
        "Activated release {} for {}.",
        activated.manifest.release_id, activated.manifest.market_scope
    );
    println!(
        "  mode={} serving={} pit={}",
        activated.manifest.probability_mode,
        activated.manifest.serving_status,
        activated.manifest.point_in_time_mode
    );
    if options.reload_api {
        reload_api_runtime(&options.api_reload_url).await?;
        println!("Reloaded API runtime via {}.", options.api_reload_url);
    }
    Ok(())
}

async fn research_release_rollback(args: &[String]) -> anyhow::Result<()> {
    let options = ReleaseSwitchOptions::parse(args)?;
    let store = open_sqlite_store().await?;
    store.migrate().await?;
    let market_scope =
        resolve_release_market_scope(&store, &options.release_id, options.market_scope.as_deref())
            .await?;
    let activated = store
        .rollback_model_release(&market_scope, &options.release_id, &options.updated_by)
        .await?;
    println!(
        "Rolled back {} to release {}.",
        market_scope, activated.manifest.release_id
    );
    println!(
        "  mode={} serving={} pit={}",
        activated.manifest.probability_mode,
        activated.manifest.serving_status,
        activated.manifest.point_in_time_mode
    );
    if options.reload_api {
        reload_api_runtime(&options.api_reload_url).await?;
        println!("Reloaded API runtime via {}.", options.api_reload_url);
    }
    Ok(())
}

async fn research_prediction_snapshot_list(args: &[String]) -> anyhow::Result<()> {
    let options = PredictionSnapshotQueryOptions::parse(args)?;
    let store = open_sqlite_store().await?;
    store.migrate().await?;
    let snapshots = load_prediction_snapshots(&store, &options).await?;
    if snapshots.is_empty() {
        println!("No prediction snapshots found.");
        return Ok(());
    }
    println!(
        "{:<12} {:<18} {:<16} {:<12} {:<10} {:<8} {:<10}",
        "as_of_date", "market_scope", "release_id", "prob_mode", "p20d", "posture", "freshness"
    );
    for snapshot in snapshots {
        println!(
            "{:<12} {:<18} {:<16} {:<12} {:<10} {:<8} {:<10}",
            snapshot.as_of_date,
            truncate_text(&snapshot.market_scope, 18),
            truncate_text(snapshot.release_id.as_deref().unwrap_or("inline"), 16),
            truncate_text(&snapshot.probability_mode, 12),
            format_pct(snapshot.calibrated_p_20d),
            truncate_text(&snapshot.posture, 8),
            truncate_text(&snapshot.freshness_status, 10),
        );
    }
    Ok(())
}

async fn research_prediction_snapshot_export(args: &[String]) -> anyhow::Result<()> {
    let options = PredictionSnapshotExportOptions::parse(args)?;
    let store = open_sqlite_store().await?;
    store.migrate().await?;
    let snapshots = load_prediction_snapshots(&store, &options.query).await?;
    write_snapshot_export(&snapshots, options.format, options.output_path.as_deref())?;
    Ok(())
}

async fn research_prediction_snapshot_dataset(args: &[String]) -> anyhow::Result<()> {
    let options = SnapshotDatasetExportOptions::parse(args)?;
    let store = open_sqlite_store().await?;
    store.migrate().await?;
    let snapshots = load_training_snapshots(&store, &options.query).await?;
    let dataset = build_pipeline_dataset_rows(&snapshots);
    write_dataset_export(&dataset, options.format, options.output_path.as_deref())?;
    Ok(())
}

async fn research_pipeline_train_probability(args: &[String]) -> anyhow::Result<()> {
    let options = PipelineTrainOptions::parse(args)?;
    let store = open_sqlite_store().await?;
    store.migrate().await?;
    let artifacts = train_probability_pipeline(&store, &options).await?;
    println!("Formal probability bundle generated.");
    println!(
        "  release_id       {}",
        artifacts.release.manifest.release_id
    );
    println!("  bundle_path      {}", artifacts.bundle_path.display());
    println!("  manifest_path    {}", artifacts.manifest_path.display());
    println!("  evaluation_path  {}", artifacts.evaluation_path.display());
    if let Some(summary) = artifacts.bundle.evaluation.as_ref() {
        println!(
            "  eval             brier={:.4} log_loss={:.4} ece={:.4}",
            summary.brier_score, summary.log_loss, summary.ece
        );
    }
    Ok(())
}

async fn research_pipeline_bootstrap_formal_release(args: &[String]) -> anyhow::Result<()> {
    let options = PipelineBootstrapOptions::parse(args)?;
    let store = open_sqlite_store().await?;
    store.migrate().await?;
    let artifacts = train_probability_pipeline(&store, &options.train).await?;
    store.upsert_model_release(&artifacts.release).await?;
    println!(
        "Published formal release {}.",
        artifacts.release.manifest.release_id
    );
    println!("  manifest {}", artifacts.manifest_path.display());
    println!("  bundle   {}", artifacts.bundle_path.display());

    if options.activate {
        let activated = store
            .activate_model_release(
                &artifacts.release.manifest.market_scope,
                &artifacts.release.manifest.release_id,
                &options.updated_by,
            )
            .await?;
        println!(
            "Activated release {} for {}.",
            activated.manifest.release_id, activated.manifest.market_scope
        );
        if options.reload_api {
            reload_api_runtime(&options.api_reload_url).await?;
            println!("Reloaded API runtime via {}.", options.api_reload_url);
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize)]
struct SnapshotDatasetRow {
    as_of_date: NaiveDate,
    market_scope: String,
    release_id: Option<String>,
    probability_mode: String,
    freshness_status: String,
    time_to_risk_bucket: String,
    features: BTreeMap<String, f64>,
    label_5d: u8,
    label_20d: u8,
    label_60d: u8,
}

impl SnapshotDatasetRow {
    fn label_for_horizon(&self, horizon_days: u32) -> f64 {
        match horizon_days {
            5 => self.label_5d as f64,
            20 => self.label_20d as f64,
            60 => self.label_60d as f64,
            _ => 0.0,
        }
    }
}

#[derive(Debug, Clone)]
struct PipelineArtifacts {
    release: ModelReleaseRecord,
    bundle: ProbabilityBundle,
    bundle_path: PathBuf,
    manifest_path: PathBuf,
    evaluation_path: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
struct PipelineEvaluationReport {
    release_id: String,
    market_scope: String,
    training_samples: usize,
    calibration_samples: usize,
    evaluation_samples: usize,
    horizons: Vec<ProbabilityHorizonBundle>,
    summary: Option<ProbabilityBundleEvaluation>,
}

async fn load_prediction_snapshots(
    store: &SqliteStore,
    options: &PredictionSnapshotQueryOptions,
) -> anyhow::Result<Vec<PredictionSnapshotRecord>> {
    Ok(store
        .list_prediction_snapshots(
            options.market_scope.as_deref(),
            options.release_id.as_deref(),
            options.from,
            options.to,
            options.limit,
        )
        .await?)
}

async fn load_training_snapshots(
    store: &SqliteStore,
    options: &PredictionSnapshotQueryOptions,
) -> anyhow::Result<Vec<PredictionSnapshotRecord>> {
    let market_scope = options
        .market_scope
        .clone()
        .unwrap_or_else(|| "financial_system".to_string());
    let release_id = match options.release_id.clone() {
        Some(release_id) => Some(release_id),
        None => Some(resolve_default_training_release_id(store, &market_scope).await?),
    };
    let snapshots = store
        .list_prediction_snapshots(
            Some(&market_scope),
            release_id.as_deref(),
            options.from,
            options.to,
            options.limit,
        )
        .await?;
    if snapshots.is_empty() {
        bail!("no training snapshots found for market scope {market_scope}");
    }
    Ok(snapshots)
}

async fn resolve_default_training_release_id(
    store: &SqliteStore,
    market_scope: &str,
) -> anyhow::Result<String> {
    if let Some(active_release) = store.load_active_model_release(market_scope).await? {
        if active_release.manifest.probability_mode == "heuristic_mvp" {
            return Ok(active_release.manifest.release_id);
        }
    }

    let heuristic_release = store
        .list_model_releases(Some(market_scope))
        .await?
        .into_iter()
        .find(|release| release.manifest.probability_mode == "heuristic_mvp");

    heuristic_release
        .map(|release| release.manifest.release_id)
        .with_context(|| {
            format!(
                "no heuristic training release found for market scope {market_scope}; pass --release-id explicitly or bootstrap a heuristic release first"
            )
        })
}

fn write_snapshot_export(
    snapshots: &[PredictionSnapshotRecord],
    format: ExportFormat,
    output_path: Option<&Path>,
) -> anyhow::Result<()> {
    let content = match format {
        ExportFormat::Json => serde_json::to_string_pretty(snapshots)?,
        ExportFormat::Csv => render_snapshot_csv(snapshots),
    };
    write_or_print_export(content, output_path)
}

fn write_dataset_export(
    dataset: &[SnapshotDatasetRow],
    format: ExportFormat,
    output_path: Option<&Path>,
) -> anyhow::Result<()> {
    let content = match format {
        ExportFormat::Json => serde_json::to_string_pretty(dataset)?,
        ExportFormat::Csv => render_dataset_csv(dataset),
    };
    write_or_print_export(content, output_path)
}

fn write_or_print_export(content: String, output_path: Option<&Path>) -> anyhow::Result<()> {
    if let Some(path) = output_path {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(path, content)?;
        println!("Exported {}", path.display());
    } else {
        println!("{content}");
    }
    Ok(())
}

fn render_snapshot_csv(snapshots: &[PredictionSnapshotRecord]) -> String {
    let mut csv = String::from(
        "as_of_date,market_scope,release_id,probability_mode,release_status,point_in_time_mode,overall_score,external_shock_score,raw_p_5d,raw_p_20d,raw_p_60d,calibrated_p_5d,calibrated_p_20d,calibrated_p_60d,posture,time_to_risk_bucket,coverage_score,freshness_status,method_version,recorded_at\n",
    );
    for snapshot in snapshots {
        let _ = writeln!(
            csv,
            "{},{},{},{},{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{},{},{:.6},{},{},{}",
            snapshot.as_of_date,
            snapshot.market_scope,
            snapshot.release_id.as_deref().unwrap_or(""),
            snapshot.probability_mode,
            snapshot.release_status,
            snapshot.point_in_time_mode,
            snapshot.overall_score,
            snapshot.external_shock_score,
            snapshot.raw_p_5d,
            snapshot.raw_p_20d,
            snapshot.raw_p_60d,
            snapshot.calibrated_p_5d,
            snapshot.calibrated_p_20d,
            snapshot.calibrated_p_60d,
            snapshot.posture,
            snapshot.time_to_risk_bucket,
            snapshot.coverage_score,
            snapshot.freshness_status,
            snapshot.method_version,
            snapshot.recorded_at.to_rfc3339()
        );
    }
    csv
}

fn render_dataset_csv(dataset: &[SnapshotDatasetRow]) -> String {
    let mut header = String::from(
        "as_of_date,market_scope,release_id,probability_mode,freshness_status,time_to_risk_bucket,label_5d,label_20d,label_60d",
    );
    for feature in PROBABILITY_BUNDLE_FEATURES {
        header.push(',');
        header.push_str(feature);
    }
    header.push('\n');

    let mut csv = header;
    for row in dataset {
        let _ = write!(
            csv,
            "{},{},{},{},{},{},{},{},{}",
            row.as_of_date,
            row.market_scope,
            row.release_id.as_deref().unwrap_or(""),
            row.probability_mode,
            row.freshness_status,
            row.time_to_risk_bucket,
            row.label_5d,
            row.label_20d,
            row.label_60d
        );
        for feature in PROBABILITY_BUNDLE_FEATURES {
            let value = row.features.get(*feature).copied().unwrap_or_default();
            let _ = write!(csv, ",{value:.6}");
        }
        csv.push('\n');
    }
    csv
}

async fn train_probability_pipeline(
    store: &SqliteStore,
    options: &PipelineTrainOptions,
) -> anyhow::Result<PipelineArtifacts> {
    let generated_at = Utc::now();
    let snapshots = load_training_snapshots(store, &options.query).await?;
    let dataset = build_pipeline_dataset_rows(&snapshots);
    if dataset.len() < 90 {
        bail!(
            "training dataset is too small: {} rows found, at least 90 are required",
            dataset.len()
        );
    }

    let (train_rows, calibration_rows, evaluation_rows) = chronological_split(&dataset)?;
    let horizons = [5_u32, 20_u32, 60_u32]
        .into_iter()
        .map(|horizon| {
            train_horizon_bundle(&train_rows, &calibration_rows, &evaluation_rows, horizon)
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    let aggregate_evaluation = summarize_bundle_evaluation(&horizons);
    let release_suffix = generated_at.format("%Y%m%dT%H%M%S").to_string();
    let release_id = format!("{}_{}", options.release_prefix, release_suffix);
    let bundle = ProbabilityBundle {
        bundle_id: release_id.clone(),
        market_scope: train_rows
            .first()
            .map(|row| row.market_scope.clone())
            .unwrap_or_else(|| "financial_system".to_string()),
        probability_mode: "formal_bundle_v1".to_string(),
        created_at: generated_at,
        feature_names: PROBABILITY_BUNDLE_FEATURES
            .iter()
            .map(|feature| feature.to_string())
            .collect(),
        monotonic_min_gap_5d_to_20d: 0.02,
        monotonic_min_gap_20d_to_60d: 0.03,
        note: "Transitional formal bundle trained from persisted heuristic prediction snapshots and calibrated with chronological holdout slices.".to_string(),
        horizons: horizons.clone(),
        evaluation: Some(aggregate_evaluation.clone()),
    };

    let bundle_path = options.output_dir.join(format!("{release_id}.json"));
    let manifest_dir = PathBuf::from("config/model-releases/generated");
    let manifest_path = manifest_dir.join(format!("{release_id}.json"));
    let evaluation_path = options
        .output_dir
        .join(format!("{release_id}-evaluation.json"));
    fs::create_dir_all(&options.output_dir)?;
    fs::create_dir_all(&manifest_dir)?;

    let release = ModelReleaseRecord {
        manifest: ModelReleaseManifest {
            release_id: release_id.clone(),
            market_scope: bundle.market_scope.clone(),
            status: "approved".to_string(),
            probability_mode: bundle.probability_mode.clone(),
            serving_status: "healthy".to_string(),
            bundle_uri: bundle_path.to_string_lossy().replace('\\', "/"),
            feature_set_version: "feature_prob_meta_v1".to_string(),
            label_version: "label_forward_crisis_v1".to_string(),
            prob_model_version: format!("prob_bundle_{release_suffix}"),
            calibration_version: format!("platt_{release_suffix}"),
            posture_policy_version: "posture_v1_20260530".to_string(),
            action_playbook_version: "action_playbook_v1_20260531".to_string(),
            point_in_time_mode: "best_effort".to_string(),
            training_range_start: train_rows.first().map(|row| row.as_of_date),
            training_range_end: train_rows.last().map(|row| row.as_of_date),
            calibration_range_start: calibration_rows.first().map(|row| row.as_of_date),
            calibration_range_end: calibration_rows.last().map(|row| row.as_of_date),
            evaluation_range_start: evaluation_rows.first().map(|row| row.as_of_date),
            evaluation_range_end: evaluation_rows.last().map(|row| row.as_of_date),
            brier_score: bundle.evaluation.as_ref().map(|summary| summary.brier_score),
            log_loss: bundle.evaluation.as_ref().map(|summary| summary.log_loss),
            ece: bundle.evaluation.as_ref().map(|summary| summary.ece),
            note: "Generated by `research pipeline train-probability` from free-data-backed persisted prediction snapshots.".to_string(),
        },
        created_at: generated_at,
        activated_at: None,
        retired_at: None,
    };

    let evaluation_report = PipelineEvaluationReport {
        release_id: release_id.clone(),
        market_scope: release.manifest.market_scope.clone(),
        training_samples: train_rows.len(),
        calibration_samples: calibration_rows.len(),
        evaluation_samples: evaluation_rows.len(),
        horizons,
        summary: bundle.evaluation.clone(),
    };

    fs::write(&bundle_path, serde_json::to_string_pretty(&bundle)?)?;
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&release.manifest)?,
    )?;
    fs::write(
        &evaluation_path,
        serde_json::to_string_pretty(&evaluation_report)?,
    )?;

    Ok(PipelineArtifacts {
        release,
        bundle,
        bundle_path,
        manifest_path,
        evaluation_path,
    })
}

fn build_pipeline_dataset_rows(snapshots: &[PredictionSnapshotRecord]) -> Vec<SnapshotDatasetRow> {
    let crisis_starts = us_crisis_start_dates();
    let mut rows = snapshots
        .iter()
        .map(|snapshot| {
            let features = pipeline_features_from_snapshot(snapshot);
            SnapshotDatasetRow {
                as_of_date: snapshot.as_of_date,
                market_scope: snapshot.market_scope.clone(),
                release_id: snapshot.release_id.clone(),
                probability_mode: snapshot.probability_mode.clone(),
                freshness_status: snapshot.freshness_status.clone(),
                time_to_risk_bucket: snapshot.time_to_risk_bucket.clone(),
                features,
                label_5d: forward_crisis_label(snapshot.as_of_date, &crisis_starts, 5),
                label_20d: forward_crisis_label(snapshot.as_of_date, &crisis_starts, 20),
                label_60d: forward_crisis_label(snapshot.as_of_date, &crisis_starts, 60),
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| row.as_of_date);
    rows
}

fn pipeline_features_from_snapshot(snapshot: &PredictionSnapshotRecord) -> BTreeMap<String, f64> {
    BTreeMap::from([
        (
            FEATURE_OVERALL_SCORE.to_string(),
            (snapshot.overall_score / 100.0).clamp(0.0, 1.0),
        ),
        (
            FEATURE_EXTERNAL_SHOCK_SCORE.to_string(),
            (snapshot.external_shock_score / 100.0).clamp(0.0, 1.0),
        ),
        (
            FEATURE_HEURISTIC_P_5D.to_string(),
            snapshot.raw_p_5d.clamp(0.0, 0.99),
        ),
        (
            FEATURE_HEURISTIC_P_20D.to_string(),
            snapshot.raw_p_20d.clamp(0.0, 0.99),
        ),
        (
            FEATURE_HEURISTIC_P_60D.to_string(),
            snapshot.raw_p_60d.clamp(0.0, 0.99),
        ),
        (
            FEATURE_COVERAGE_SCORE.to_string(),
            snapshot.coverage_score.clamp(0.0, 1.0),
        ),
        (
            FEATURE_BUCKET_MONTHS_OR_HIGHER.to_string(),
            matches!(
                snapshot.time_to_risk_bucket.as_str(),
                "months" | "weeks" | "now"
            ) as u8 as f64,
        ),
        (
            FEATURE_BUCKET_WEEKS_OR_HIGHER.to_string(),
            matches!(snapshot.time_to_risk_bucket.as_str(), "weeks" | "now") as u8 as f64,
        ),
        (
            FEATURE_BUCKET_NOW.to_string(),
            matches!(snapshot.time_to_risk_bucket.as_str(), "now") as u8 as f64,
        ),
        (
            FEATURE_FRESHNESS_DELAYED_OR_WORSE.to_string(),
            matches!(
                snapshot.freshness_status.as_str(),
                "delayed" | "stale" | "missing"
            ) as u8 as f64,
        ),
        (
            FEATURE_FRESHNESS_STALE_OR_MISSING.to_string(),
            matches!(snapshot.freshness_status.as_str(), "stale" | "missing") as u8 as f64,
        ),
    ])
}

fn us_crisis_start_dates() -> Vec<NaiveDate> {
    vec![
        NaiveDate::from_ymd_opt(2007, 8, 1).expect("valid date"),
        NaiveDate::from_ymd_opt(2020, 2, 24).expect("valid date"),
        NaiveDate::from_ymd_opt(2023, 3, 8).expect("valid date"),
    ]
}

fn forward_crisis_label(
    as_of_date: NaiveDate,
    crisis_starts: &[NaiveDate],
    horizon_days: i64,
) -> u8 {
    crisis_starts.iter().any(|crisis_start| {
        let lead_days = (*crisis_start - as_of_date).num_days();
        (1..=horizon_days).contains(&lead_days)
    }) as u8
}

fn chronological_split(
    dataset: &[SnapshotDatasetRow],
) -> anyhow::Result<(
    Vec<SnapshotDatasetRow>,
    Vec<SnapshotDatasetRow>,
    Vec<SnapshotDatasetRow>,
)> {
    if dataset.len() < 30 {
        bail!("dataset is too small for chronological split");
    }
    let train_end = (dataset.len() * 6 / 10)
        .max(30)
        .min(dataset.len().saturating_sub(20));
    let calibration_end = (dataset.len() * 8 / 10)
        .max(train_end + 10)
        .min(dataset.len().saturating_sub(10));
    if train_end >= calibration_end || calibration_end >= dataset.len() {
        bail!("unable to construct train/calibration/evaluation split");
    }
    Ok((
        dataset[..train_end].to_vec(),
        dataset[train_end..calibration_end].to_vec(),
        dataset[calibration_end..].to_vec(),
    ))
}

fn train_horizon_bundle(
    train_rows: &[SnapshotDatasetRow],
    calibration_rows: &[SnapshotDatasetRow],
    evaluation_rows: &[SnapshotDatasetRow],
    horizon_days: u32,
) -> anyhow::Result<ProbabilityHorizonBundle> {
    ensure_positive_labels(train_rows, horizon_days, "train")?;
    ensure_positive_labels(calibration_rows, horizon_days, "calibration")?;
    ensure_positive_labels(evaluation_rows, horizon_days, "evaluation")?;

    let raw_model = fit_logistic_model(train_rows, horizon_days);
    let calibration_inputs = calibration_rows
        .iter()
        .map(|row| score_logistic_model_for_dataset(&raw_model, row))
        .collect::<Vec<_>>();
    let calibration_labels = calibration_rows
        .iter()
        .map(|row| row.label_for_horizon(horizon_days))
        .collect::<Vec<_>>();
    let calibration = fit_platt_calibration(&calibration_inputs, &calibration_labels);
    let evaluation_probabilities = evaluation_rows
        .iter()
        .map(|row| {
            let raw_probability = score_logistic_model_for_dataset(&raw_model, row);
            apply_platt_calibration(raw_probability, &calibration)
        })
        .collect::<Vec<_>>();
    let evaluation_labels = evaluation_rows
        .iter()
        .map(|row| row.label_for_horizon(horizon_days))
        .collect::<Vec<_>>();
    let evaluation = evaluate_probabilities(&evaluation_probabilities, &evaluation_labels);

    Ok(ProbabilityHorizonBundle {
        horizon_days,
        raw_model,
        calibration: Some(calibration),
        evaluation,
    })
}

fn ensure_positive_labels(
    rows: &[SnapshotDatasetRow],
    horizon_days: u32,
    split_name: &str,
) -> anyhow::Result<()> {
    let positives = rows
        .iter()
        .filter(|row| row.label_for_horizon(horizon_days) > 0.0)
        .count();
    if positives == 0 {
        bail!("no positive {horizon_days}d labels found in the {split_name} split");
    }
    Ok(())
}

fn fit_logistic_model(rows: &[SnapshotDatasetRow], horizon_days: u32) -> LogisticProbabilityModel {
    let feature_stats = PROBABILITY_BUNDLE_FEATURES
        .iter()
        .map(|feature| build_feature_stat(rows, feature))
        .collect::<Vec<_>>();
    let mut intercept = initial_intercept(rows, horizon_days);
    let mut weights = vec![0.0; PROBABILITY_BUNDLE_FEATURES.len()];
    let learning_rate = 0.25;
    let l2 = 0.01;
    let sample_count = rows.len() as f64;

    for _ in 0..600 {
        let mut intercept_gradient = 0.0;
        let mut weight_gradients = vec![0.0; weights.len()];
        for row in rows {
            let normalized = normalized_features(row, &feature_stats);
            let prediction = sigmoid(intercept + dot(&weights, &normalized));
            let error = prediction - row.label_for_horizon(horizon_days);
            intercept_gradient += error;
            for (index, value) in normalized.iter().enumerate() {
                weight_gradients[index] += error * value;
            }
        }
        intercept -= learning_rate * intercept_gradient / sample_count;
        for (index, weight) in weights.iter_mut().enumerate() {
            *weight -= learning_rate * ((weight_gradients[index] / sample_count) + l2 * *weight);
        }
    }

    LogisticProbabilityModel {
        intercept,
        feature_stats: feature_stats.clone(),
        coefficients: PROBABILITY_BUNDLE_FEATURES
            .iter()
            .zip(weights)
            .map(|(feature, weight)| ProbabilityCoefficient {
                name: (*feature).to_string(),
                weight,
            })
            .collect(),
    }
}

fn build_feature_stat(rows: &[SnapshotDatasetRow], feature_name: &str) -> ProbabilityFeatureStat {
    let values = rows
        .iter()
        .map(|row| row.features.get(feature_name).copied().unwrap_or_default())
        .collect::<Vec<_>>();
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values
        .iter()
        .map(|value| {
            let diff = value - mean;
            diff * diff
        })
        .sum::<f64>()
        / values.len() as f64;
    ProbabilityFeatureStat {
        name: feature_name.to_string(),
        mean,
        std_dev: variance.sqrt().max(1e-6),
        fill_value: mean,
    }
}

fn initial_intercept(rows: &[SnapshotDatasetRow], horizon_days: u32) -> f64 {
    let positive_rate = rows
        .iter()
        .map(|row| row.label_for_horizon(horizon_days))
        .sum::<f64>()
        / rows.len() as f64;
    let clipped = positive_rate.clamp(0.01, 0.99);
    (clipped / (1.0 - clipped)).ln()
}

fn normalized_features(
    row: &SnapshotDatasetRow,
    feature_stats: &[ProbabilityFeatureStat],
) -> Vec<f64> {
    feature_stats
        .iter()
        .map(|stat| {
            let value = row
                .features
                .get(&stat.name)
                .copied()
                .unwrap_or(stat.fill_value);
            (value - stat.mean) / stat.std_dev.max(1e-6)
        })
        .collect()
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).map(|(l, r)| l * r).sum()
}

fn fit_platt_calibration(inputs: &[f64], labels: &[f64]) -> PlattCalibrationArtifact {
    let mut alpha = 1.0;
    let mut beta = 0.0;
    let learning_rate = 0.5;
    let sample_count = inputs.len() as f64;

    for _ in 0..500 {
        let mut alpha_gradient = 0.0;
        let mut beta_gradient = 0.0;
        for (input, label) in inputs.iter().zip(labels) {
            let prediction = sigmoid(alpha * input + beta);
            let error = prediction - *label;
            alpha_gradient += error * input;
            beta_gradient += error;
        }
        alpha -= learning_rate * alpha_gradient / sample_count;
        beta -= learning_rate * beta_gradient / sample_count;
    }

    let min_input = inputs.iter().copied().fold(1.0, f64::min);
    let max_input = inputs.iter().copied().fold(0.0, f64::max);
    PlattCalibrationArtifact {
        alpha,
        beta,
        min_input,
        max_input,
    }
}

fn score_logistic_model_for_dataset(
    model: &LogisticProbabilityModel,
    row: &SnapshotDatasetRow,
) -> f64 {
    let normalized = normalized_features(row, &model.feature_stats);
    sigmoid(
        model.intercept
            + model
                .coefficients
                .iter()
                .zip(normalized)
                .map(|(coefficient, value)| coefficient.weight * value)
                .sum::<f64>(),
    )
}

fn apply_platt_calibration(raw_probability: f64, calibration: &PlattCalibrationArtifact) -> f64 {
    let clipped = raw_probability.clamp(calibration.min_input, calibration.max_input);
    sigmoid(calibration.alpha * clipped + calibration.beta)
}

fn evaluate_probabilities(probabilities: &[f64], labels: &[f64]) -> HorizonEvaluationSummary {
    let sample_count = probabilities.len() as u32;
    let positive_rate = labels.iter().sum::<f64>() / labels.len().max(1) as f64;
    let brier_score = probabilities
        .iter()
        .zip(labels)
        .map(|(probability, label)| {
            let diff = probability - label;
            diff * diff
        })
        .sum::<f64>()
        / probabilities.len().max(1) as f64;
    let log_loss = probabilities
        .iter()
        .zip(labels)
        .map(|(probability, label)| {
            let clipped = probability.clamp(0.001, 0.999);
            -(label * clipped.ln() + (1.0 - label) * (1.0 - clipped).ln())
        })
        .sum::<f64>()
        / probabilities.len().max(1) as f64;
    let ece = expected_calibration_error(probabilities, labels, 10);
    let predicted_positive = probabilities
        .iter()
        .zip(labels)
        .filter(|(probability, _)| **probability >= 0.3)
        .collect::<Vec<_>>();
    let true_positive = predicted_positive
        .iter()
        .filter(|(_, label)| **label >= 0.5)
        .count();
    let actual_positive = labels.iter().filter(|label| **label >= 0.5).count();

    HorizonEvaluationSummary {
        sample_count,
        positive_rate,
        brier_score,
        log_loss,
        ece,
        precision_at_30pct: (!predicted_positive.is_empty())
            .then_some(true_positive as f64 / predicted_positive.len() as f64),
        recall_at_30pct: (actual_positive > 0)
            .then_some(true_positive as f64 / actual_positive as f64),
    }
}

fn expected_calibration_error(probabilities: &[f64], labels: &[f64], bins: usize) -> f64 {
    let mut error = 0.0;
    for bin in 0..bins {
        let start = bin as f64 / bins as f64;
        let end = (bin + 1) as f64 / bins as f64;
        let bucket = probabilities
            .iter()
            .zip(labels)
            .filter(|(probability, _)| {
                (bin + 1 == bins && **probability >= start && **probability <= end)
                    || (**probability >= start && **probability < end)
            })
            .collect::<Vec<_>>();
        if bucket.is_empty() {
            continue;
        }
        let avg_probability = bucket
            .iter()
            .map(|(probability, _)| **probability)
            .sum::<f64>()
            / bucket.len() as f64;
        let avg_label = bucket.iter().map(|(_, label)| **label).sum::<f64>() / bucket.len() as f64;
        error += (bucket.len() as f64 / probabilities.len() as f64)
            * (avg_probability - avg_label).abs();
    }
    error
}

fn summarize_bundle_evaluation(
    horizons: &[ProbabilityHorizonBundle],
) -> ProbabilityBundleEvaluation {
    let total_samples = horizons
        .iter()
        .map(|horizon| horizon.evaluation.sample_count as f64)
        .sum::<f64>()
        .max(1.0);
    let weighted_brier = horizons
        .iter()
        .map(|horizon| horizon.evaluation.brier_score * horizon.evaluation.sample_count as f64)
        .sum::<f64>()
        / total_samples;
    let weighted_log_loss = horizons
        .iter()
        .map(|horizon| horizon.evaluation.log_loss * horizon.evaluation.sample_count as f64)
        .sum::<f64>()
        / total_samples;
    let weighted_ece = horizons
        .iter()
        .map(|horizon| horizon.evaluation.ece * horizon.evaluation.sample_count as f64)
        .sum::<f64>()
        / total_samples;
    ProbabilityBundleEvaluation {
        sample_count: total_samples as u32,
        brier_score: weighted_brier,
        log_loss: weighted_log_loss,
        ece: weighted_ece,
        note: "Weighted average across 5d / 20d / 60d evaluation slices.".to_string(),
    }
}

fn sigmoid(value: f64) -> f64 {
    1.0 / (1.0 + (-value).exp())
}

async fn backfill_fred(args: &[String]) -> anyhow::Result<()> {
    let options = FredBackfillOptions::parse(args)?;
    backfill_fred_with_options(options).await
}

async fn backfill_fred_with_options(options: FredBackfillOptions) -> anyhow::Result<()> {
    let store = open_sqlite_store().await?;
    store.migrate().await?;
    store.seed_fred_metadata().await?;

    let fred_mode = options.fred_mode;
    let connector: Box<dyn Connector> = match fred_mode {
        FredBackfillMode::GraphCsv => Box::new(FredGraphCsvConnector::new()),
        FredBackfillMode::Api => {
            let api_key = env::var("FRED_API_KEY")
                .context("FRED_API_KEY is required only when using `backfill fred --api`.")?;
            Box::new(FredConnector::new(Some(api_key)))
        }
    };
    let mappings = store.load_fred_mappings().await?;
    if mappings.is_empty() {
        bail!("no FRED mappings found; run `just db-seed` first");
    }

    let backfill_options = if matches!(fred_mode, FredBackfillMode::GraphCsv) {
        options.options.with_default_chunk_days(366)
    } else {
        options.options
    };
    backfill_mappings(
        connector.as_ref(),
        mappings,
        FRED_DATASET_ID,
        backfill_options,
        "FRED",
    )
    .await
}

async fn backfill_treasury_yield(args: &[String]) -> anyhow::Result<()> {
    let options = BackfillOptions::parse(args)?;
    backfill_treasury_yield_with_options(options).await
}

async fn backfill_treasury_yield_with_options(options: BackfillOptions) -> anyhow::Result<()> {
    let store = open_sqlite_store().await?;
    store.migrate().await?;
    store.seed_fred_metadata().await?;

    let mappings = store.load_treasury_yield_mappings().await?;
    if mappings.is_empty() {
        bail!("no Treasury yield mappings found; run `just db-seed` first");
    }

    let connector = TreasuryYieldCurveConnector::new();
    backfill_mappings(
        &connector,
        mappings,
        TREASURY_YIELD_DATASET_ID,
        options,
        "Treasury yield",
    )
    .await
}

async fn backfill_world_bank(args: &[String]) -> anyhow::Result<()> {
    let options = BackfillOptions::parse(args)?;
    backfill_world_bank_with_options(options).await
}

async fn backfill_world_bank_with_options(options: BackfillOptions) -> anyhow::Result<()> {
    let store = open_sqlite_store().await?;
    store.migrate().await?;
    store.seed_fred_metadata().await?;

    let mappings = store.load_world_bank_mappings().await?;
    if mappings.is_empty() {
        bail!("no World Bank mappings found; run `just db-seed` first");
    }

    let connector = WorldBankConnector::new();
    backfill_mappings(
        &connector,
        mappings,
        WORLD_BANK_DATASET_ID,
        options,
        "World Bank",
    )
    .await
}

async fn backfill_gdelt(args: &[String]) -> anyhow::Result<()> {
    let options = BackfillOptions::parse(args)?;
    backfill_gdelt_with_options(options).await
}

async fn backfill_gdelt_with_options(options: BackfillOptions) -> anyhow::Result<()> {
    let store = open_sqlite_store().await?;
    store.migrate().await?;
    store.seed_fred_metadata().await?;

    let connector = GdeltConnector::new();
    let effective_start = if let Some(overlap_days) = options.watermark_overlap_days {
        let watermark = store
            .load_watermark_date(
                "gdelt",
                GDELT_DOC_DATASET_ID,
                "global_news_financial_stress_count",
            )
            .await?;
        options.effective_start(watermark, overlap_days)
    } else {
        options.start
    };
    if effective_start > options.end {
        println!("GDELT backfill skipped: watermark is already beyond requested range.");
        return Ok(());
    }

    println!(
        "Backfilling GDELT timeline aggregates into {} [{}..{}]",
        sqlite_path(),
        effective_start,
        options.end
    );
    let output = connector
        .backfill_range(effective_start, options.end)
        .await?;
    let raw_root = raw_data_dir();
    let raw_path = write_raw_payload(
        &raw_root,
        "gdelt",
        "global_news_financial_stress_count",
        "json",
        &output.payload_body,
    )?;
    let raw_payload_id = Uuid::new_v4();
    store
        .insert_raw_response(&RawResponseRecord {
            raw_payload_id,
            source_id: "gdelt".to_string(),
            dataset_id: GDELT_DOC_DATASET_ID.to_string(),
            request_url: output.payload_url.clone(),
            request_params_hash: Some(simple_hash(&output.payload_url)),
            response_hash: simple_hash(&output.payload_body),
            content_type: "application/json".to_string(),
            content_length: output.payload_body.len() as i64,
            raw_file_path: path_to_string(&raw_path),
            fetched_at: Utc::now(),
        })
        .await?;
    store.insert_observations(&output.observations).await?;
    store
        .replace_alerts_for_scope("gdelt_daily", effective_start, options.end, &output.alerts)
        .await?;
    if let Some(latest_date) = output.latest_date {
        store
            .upsert_watermark(
                "gdelt",
                GDELT_DOC_DATASET_ID,
                "global_news_financial_stress_count",
                latest_date,
            )
            .await?;
    }

    println!(
        "GDELT backfill completed: {} observations, {} alerts",
        output.observations.len(),
        output.alerts.len()
    );
    Ok(())
}

async fn backfill_sec_edgar(args: &[String]) -> anyhow::Result<()> {
    let options = BackfillOptions::parse(args)?;
    backfill_sec_edgar_with_options(options).await
}

async fn backfill_sec_edgar_with_options(options: BackfillOptions) -> anyhow::Result<()> {
    let store = open_sqlite_store().await?;
    store.migrate().await?;
    store.seed_fred_metadata().await?;

    println!(
        "Backfilling SEC EDGAR filing events into {} [{}..{}]",
        sqlite_path(),
        options.start,
        options.end
    );

    let connector = SecEdgarConnector::new();
    let output = connector.backfill_range(options.start, options.end).await?;
    let raw_root = raw_data_dir();

    for payload in &output.payloads {
        let raw_path = write_raw_payload(
            &raw_root,
            &payload.source_id,
            SEC_SUBMISSIONS_DATASET_ID,
            raw_file_extension(&payload.content_type),
            &payload.body,
        )?;
        store
            .insert_raw_response(&RawResponseRecord {
                raw_payload_id: payload.raw_payload_id,
                source_id: payload.source_id.clone(),
                dataset_id: payload.dataset_id.clone(),
                request_url: payload.request_url.clone(),
                request_params_hash: Some(simple_hash(&payload.request_url)),
                response_hash: payload.response_hash.clone(),
                content_type: payload.content_type.clone(),
                content_length: payload.body.len() as i64,
                raw_file_path: path_to_string(&raw_path),
                fetched_at: payload.fetched_at,
            })
            .await?;
    }

    store.insert_observations(&output.observations).await?;
    store
        .replace_alerts_for_scope(
            "sec_edgar_daily",
            options.start,
            options.end,
            &output.alerts,
        )
        .await?;
    if let Some(latest_filing_date) = output.latest_filing_date {
        store
            .upsert_watermark("sec_edgar", SEC_EVENTS_DATASET_ID, "us", latest_filing_date)
            .await?;
    }

    println!(
        "SEC EDGAR backfill completed: {} payloads, {} filings, {} observations, {} alerts",
        output.payloads.len(),
        output.filing_count,
        output.observations.len(),
        output.alerts.len()
    );
    Ok(())
}

async fn backfill_jpy_carry(args: &[String]) -> anyhow::Result<()> {
    let options = BackfillOptions::parse(args)?;
    backfill_jpy_carry_with_options(options).await
}

async fn backfill_jpy_carry_with_options(options: BackfillOptions) -> anyhow::Result<()> {
    let store = open_sqlite_store().await?;
    store.migrate().await?;
    store.seed_fred_metadata().await?;

    let mappings = store.load_jpy_carry_mappings().await?;
    if mappings.is_empty() {
        bail!("no JPY carry mappings found; run `just db-seed` first");
    }

    let boj_mappings = mappings
        .iter()
        .filter(|mapping| mapping.external_code.starts_with("FXER"))
        .cloned()
        .collect::<Vec<_>>();
    if !boj_mappings.is_empty() {
        let connector = BojConnector::fx_daily();
        if backfill_mappings(
            &connector,
            boj_mappings,
            BOJ_FX_DATASET_ID,
            options.clone(),
            "JPY carry (BOJ)",
        )
        .await
        .is_ok()
        {
            return Ok(());
        }
        tracing::warn!("BOJ FX backfill failed, falling back to FRED graph CSV");
    }

    let fred_mappings = mappings
        .into_iter()
        .filter(|mapping| mapping.external_code == "DEXJPUS")
        .collect::<Vec<_>>();
    if fred_mappings.is_empty() {
        bail!("no FRED fallback mappings found for JPY carry");
    }
    let connector = FredGraphCsvConnector::new();
    backfill_mappings(
        &connector,
        fred_mappings,
        FRED_DATASET_ID,
        options,
        "JPY carry (FRED fallback)",
    )
    .await
}

async fn backfill_boj(args: &[String]) -> anyhow::Result<()> {
    let options = BojBackfillOptions::parse(args)?;
    backfill_boj_with_options(options).await
}

async fn backfill_boj_with_options(options: BojBackfillOptions) -> anyhow::Result<()> {
    let store = open_sqlite_store().await?;
    store.migrate().await?;
    store.seed_fred_metadata().await?;

    match options.dataset {
        BojDataset::FxDaily => {
            let mappings = store.load_jpy_carry_mappings().await?;
            let mappings = mappings
                .into_iter()
                .filter(|mapping| mapping.external_code.starts_with("FXER"))
                .collect::<Vec<_>>();
            if mappings.is_empty() {
                bail!("no BOJ FX mappings found; run `just db-seed` first");
            }
            let connector = BojConnector::fx_daily();
            backfill_mappings(
                &connector,
                mappings,
                BOJ_FX_DATASET_ID,
                options.options,
                "BOJ FX daily",
            )
            .await
        }
        BojDataset::MoneyMarketRates => {
            let mappings = store.load_boj_money_market_mappings().await?;
            if mappings.is_empty() {
                bail!("no BOJ money market mappings found; run `just db-seed` first");
            }
            let connector = BojConnector::money_market_rates();
            backfill_mappings(
                &connector,
                mappings,
                BOJ_MONEY_MARKET_DATASET_ID,
                options.options,
                "BOJ money market rates",
            )
            .await
        }
    }
}

async fn refresh_latest_free(args: &[String]) -> anyhow::Result<()> {
    let options = RefreshLatestOptions::parse(args)?;
    let today = Utc::now().date_naive();
    let fast_start = today - chrono::Duration::days(options.fast_lookback_days);
    let slow_start = today - chrono::Duration::days(options.slow_lookback_years * 366);

    println!(
        "Refreshing latest free data into {} (fast window {}..{}, slow window {}..{})",
        sqlite_path(),
        fast_start,
        today,
        slow_start,
        today
    );

    db_init().await?;
    db_seed().await?;

    backfill_fred_with_options(FredBackfillOptions {
        options: BackfillOptions {
            start: fast_start,
            end: today,
            chunk_days: Some(options.fred_chunk_days),
            indicator_filter: None,
            external_code_filter: None,
            watermark_overlap_days: None,
        },
        fred_mode: FredBackfillMode::GraphCsv,
    })
    .await?;

    backfill_treasury_yield_with_options(BackfillOptions {
        start: fast_start,
        end: today,
        chunk_days: Some(options.fast_lookback_days.min(180)),
        indicator_filter: None,
        external_code_filter: None,
        watermark_overlap_days: None,
    })
    .await?;

    backfill_boj_with_options(BojBackfillOptions {
        dataset: BojDataset::FxDaily,
        options: BackfillOptions {
            start: fast_start,
            end: today,
            chunk_days: Some(options.fast_lookback_days.min(180)),
            indicator_filter: None,
            external_code_filter: None,
            watermark_overlap_days: None,
        },
    })
    .await?;

    backfill_boj_with_options(BojBackfillOptions {
        dataset: BojDataset::MoneyMarketRates,
        options: BackfillOptions {
            start: fast_start,
            end: today,
            chunk_days: Some(options.fast_lookback_days.min(180)),
            indicator_filter: None,
            external_code_filter: None,
            watermark_overlap_days: None,
        },
    })
    .await?;

    backfill_sec_edgar_with_options(BackfillOptions {
        start: fast_start,
        end: today,
        chunk_days: None,
        indicator_filter: None,
        external_code_filter: None,
        watermark_overlap_days: None,
    })
    .await?;

    if !options.skip_world_bank {
        backfill_world_bank_with_options(BackfillOptions {
            start: slow_start,
            end: today,
            chunk_days: None,
            indicator_filter: None,
            external_code_filter: None,
            watermark_overlap_days: None,
        })
        .await?;
    }

    if options.include_gdelt {
        backfill_gdelt_with_options(BackfillOptions {
            start: fast_start,
            end: today,
            chunk_days: None,
            indicator_filter: None,
            external_code_filter: None,
            watermark_overlap_days: Some(7),
        })
        .await?;
    }

    db_check().await?;

    if options.reload_api {
        match reload_api_runtime(&options.api_reload_url).await {
            Ok(()) => println!("API runtime reloaded via {}", options.api_reload_url),
            Err(error) => {
                println!(
                    "API reload skipped or failed via {}: {error:#}",
                    options.api_reload_url
                );
                println!("You can still reload manually with POST /api/system/reload.");
            }
        }
    }

    Ok(())
}

async fn backfill_mappings(
    connector: &dyn Connector,
    mappings: Vec<ExternalIndicatorMapping>,
    dataset_id: &str,
    options: BackfillOptions,
    label: &str,
) -> anyhow::Result<()> {
    let store = open_sqlite_store().await?;
    let raw_root = raw_data_dir();
    let mappings = options.filter_mappings(mappings);
    if mappings.is_empty() {
        bail!("{label} backfill found no mappings matching the requested filters");
    }
    let mut total_written = 0_usize;
    let mut failures = Vec::new();
    let chunks = options.chunks();
    let chunk_count = chunks.len();
    for mapping in mappings {
        for (chunk_index, (chunk_start, chunk_end)) in chunks.iter().copied().enumerate() {
            let plan = FetchPlan {
                source_id: connector.describe().source_id,
                dataset_id: dataset_id.to_string(),
                target_id: mapping.indicator_id.clone(),
                external_code: Some(mapping.external_code.clone()),
                run_mode: RunMode::Backfill,
                requested_start: Some(chunk_start),
                requested_end: Some(chunk_end),
                frequency: mapping.frequency,
            };
            tracing::info!(
                indicator_id = %plan.target_id,
                external_code = %mapping.external_code,
                source_id = %plan.source_id,
                chunk = chunk_index + 1,
                chunks = chunk_count,
                start = %chunk_start,
                end = %chunk_end,
                "fetching observations"
            );

            let result: anyhow::Result<usize> = async {
                let payload = connector.fetch(&plan).await?;
                let raw_path = write_raw_payload(
                    &raw_root,
                    &payload.source_id,
                    &mapping.external_code,
                    raw_file_extension(&payload.content_type),
                    &payload.body,
                )?;
                store
                    .insert_raw_response(&RawResponseRecord {
                        raw_payload_id: payload.raw_payload_id,
                        source_id: payload.source_id.clone(),
                        dataset_id: payload.dataset_id.clone(),
                        request_url: payload.request_url.clone(),
                        request_params_hash: Some(simple_hash(&payload.request_url)),
                        response_hash: payload.response_hash.clone(),
                        content_type: payload.content_type.clone(),
                        content_length: payload.body.len() as i64,
                        raw_file_path: path_to_string(&raw_path),
                        fetched_at: payload.fetched_at,
                    })
                    .await?;
                let batch = connector.parse(&plan, &payload)?;
                let latest_date = batch
                    .observations
                    .iter()
                    .map(|observation| observation.as_of_date)
                    .max();
                let written = batch.observations.len();
                store
                    .insert_observations_with_raw_payload(
                        &batch.observations,
                        Some(payload.raw_payload_id),
                    )
                    .await?;
                if let Some(latest_date) = latest_date {
                    store
                        .upsert_watermark(
                            &payload.source_id,
                            &payload.dataset_id,
                            &mapping.indicator_id,
                            latest_date,
                        )
                        .await?;
                }
                if written == 0 {
                    tracing::warn!(
                        indicator_id = %mapping.indicator_id,
                        external_code = %mapping.external_code,
                        start = %chunk_start,
                        end = %chunk_end,
                        "no observations were written for requested range"
                    );
                }
                println!(
                    "backfilled {} ({}) from {} with {} observations [{}..{}]",
                    mapping.indicator_id,
                    mapping.external_code,
                    payload.source_id,
                    written,
                    chunk_start,
                    chunk_end
                );
                for warning in batch.warnings.iter().take(3) {
                    tracing::warn!(%warning, indicator_id = %mapping.indicator_id, "parse warning");
                }
                Ok(written)
            }
            .await;

            match result {
                Ok(written) => total_written += written,
                Err(error) => {
                    let failure = format!(
                        "{} ({}) [{}..{}]: {error:#}",
                        mapping.indicator_id, mapping.external_code, chunk_start, chunk_end
                    );
                    tracing::warn!(%failure, "backfill chunk failed");
                    failures.push(failure);
                }
            }
        }
    }

    if failures.is_empty() {
        println!(
            "{} backfill completed: {} observations written to {}",
            label,
            total_written,
            sqlite_path()
        );
        Ok(())
    } else {
        println!(
            "{} backfill partially completed: {} observations written to {}, {} chunk(s) failed",
            label,
            total_written,
            sqlite_path(),
            failures.len()
        );
        for failure in failures.iter().take(5) {
            println!("  failed: {failure}");
        }
        bail!(
            "{} backfill had {} failed chunk(s); retry the command to fill missing gaps",
            label,
            failures.len()
        )
    }
}

async fn run_demo_ingestion() -> anyhow::Result<()> {
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

#[derive(Debug, Clone)]
struct BackfillOptions {
    start: NaiveDate,
    end: NaiveDate,
    chunk_days: Option<i64>,
    indicator_filter: Option<String>,
    external_code_filter: Option<String>,
    watermark_overlap_days: Option<i64>,
}

impl BackfillOptions {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut start = NaiveDate::from_ymd_opt(1990, 1, 1).expect("valid date");
        let mut end = Utc::now().date_naive();
        let mut chunk_days = None;
        let mut indicator_filter = None;
        let mut external_code_filter = None;
        let mut watermark_overlap_days = None;
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--start" => {
                    index += 1;
                    start = parse_date_arg(args.get(index), "--start")?;
                }
                "--end" => {
                    index += 1;
                    end = parse_date_arg(args.get(index), "--end")?;
                }
                "--chunk-days" => {
                    index += 1;
                    let value = args
                        .get(index)
                        .with_context(|| "--chunk-days requires a positive integer")?
                        .parse::<i64>()
                        .with_context(|| "--chunk-days requires a positive integer")?;
                    if value <= 0 {
                        bail!("--chunk-days requires a positive integer");
                    }
                    chunk_days = Some(value);
                }
                "--indicator" => {
                    index += 1;
                    indicator_filter = Some(
                        args.get(index)
                            .with_context(|| "--indicator requires an indicator_id")?
                            .clone(),
                    );
                }
                "--external-code" => {
                    index += 1;
                    external_code_filter = Some(
                        args.get(index)
                            .with_context(|| "--external-code requires a source code")?
                            .clone(),
                    );
                }
                "--watermark-overlap-days" => {
                    index += 1;
                    watermark_overlap_days = Some(parse_positive_i64(
                        args.get(index),
                        "--watermark-overlap-days",
                    )?);
                }
                other => bail!("unknown backfill option: {other}"),
            }
            index += 1;
        }
        if start > end {
            bail!("--start must be on or before --end");
        }
        Ok(Self {
            start,
            end,
            chunk_days,
            indicator_filter,
            external_code_filter,
            watermark_overlap_days,
        })
    }

    fn with_default_chunk_days(mut self, chunk_days: i64) -> Self {
        if self.chunk_days.is_none() {
            self.chunk_days = Some(chunk_days);
        }
        self
    }

    fn chunks(&self) -> Vec<(NaiveDate, NaiveDate)> {
        self.chunks_for_range(self.start, self.end)
    }

    fn chunks_for_range(&self, start: NaiveDate, end: NaiveDate) -> Vec<(NaiveDate, NaiveDate)> {
        let Some(chunk_days) = self.chunk_days else {
            return vec![(start, end)];
        };

        let mut chunks = Vec::new();
        let mut cursor = start;
        while cursor <= end {
            let chunk_end = (cursor + chrono::Duration::days(chunk_days - 1)).min(end);
            chunks.push((cursor, chunk_end));
            if chunk_end == end {
                break;
            }
            cursor = chunk_end + chrono::Duration::days(1);
        }
        chunks
    }

    fn filter_mappings(
        &self,
        mappings: Vec<ExternalIndicatorMapping>,
    ) -> Vec<ExternalIndicatorMapping> {
        mappings
            .into_iter()
            .filter(|mapping| {
                self.indicator_filter
                    .as_ref()
                    .map(|filter| mapping.indicator_id == *filter)
                    .unwrap_or(true)
                    && self
                        .external_code_filter
                        .as_ref()
                        .map(|filter| mapping.external_code == *filter)
                        .unwrap_or(true)
            })
            .collect()
    }

    fn effective_start(&self, watermark: Option<NaiveDate>, overlap_days: i64) -> NaiveDate {
        watermark
            .map(|date| (date - chrono::Duration::days(overlap_days)).max(self.start))
            .unwrap_or(self.start)
    }
}

#[derive(Debug, Clone)]
struct FredBackfillOptions {
    options: BackfillOptions,
    fred_mode: FredBackfillMode,
}

impl FredBackfillOptions {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut filtered_args = Vec::new();
        let mut fred_mode = FredBackfillMode::GraphCsv;
        for arg in args {
            match arg.as_str() {
                "--api" => fred_mode = FredBackfillMode::Api,
                "--graph-csv" => fred_mode = FredBackfillMode::GraphCsv,
                _ => filtered_args.push(arg.clone()),
            }
        }
        Ok(Self {
            options: BackfillOptions::parse(&filtered_args)?,
            fred_mode,
        })
    }
}

#[derive(Debug, Clone)]
struct BojBackfillOptions {
    options: BackfillOptions,
    dataset: BojDataset,
}

impl BojBackfillOptions {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut filtered_args = Vec::new();
        let mut dataset = BojDataset::FxDaily;
        let mut index = 0;
        while index < args.len() {
            if args[index] == "--dataset" {
                index += 1;
                let value = args
                    .get(index)
                    .context("--dataset requires `fx-daily` or `money-market`")?;
                dataset = match value.as_str() {
                    "fx-daily" => BojDataset::FxDaily,
                    "money-market" => BojDataset::MoneyMarketRates,
                    other => bail!("unsupported BOJ dataset: {other}"),
                };
            } else {
                filtered_args.push(args[index].clone());
            }
            index += 1;
        }
        Ok(Self {
            options: BackfillOptions::parse(&filtered_args)?,
            dataset,
        })
    }
}

#[derive(Debug, Clone)]
struct RefreshLatestOptions {
    fast_lookback_days: i64,
    slow_lookback_years: i64,
    fred_chunk_days: i64,
    skip_world_bank: bool,
    include_gdelt: bool,
    reload_api: bool,
    api_reload_url: String,
}

impl RefreshLatestOptions {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut fast_lookback_days = 45_i64;
        let mut slow_lookback_years = 15_i64;
        let mut fred_chunk_days = 45_i64;
        let mut skip_world_bank = false;
        let mut include_gdelt = false;
        let mut reload_api = true;
        let mut api_reload_url = DEFAULT_API_RELOAD_URL.to_string();
        let mut index = 0;

        while index < args.len() {
            match args[index].as_str() {
                "--fast-lookback-days" => {
                    index += 1;
                    fast_lookback_days =
                        parse_positive_i64(args.get(index), "--fast-lookback-days")?;
                }
                "--slow-lookback-years" => {
                    index += 1;
                    slow_lookback_years =
                        parse_positive_i64(args.get(index), "--slow-lookback-years")?;
                }
                "--fred-chunk-days" => {
                    index += 1;
                    fred_chunk_days = parse_positive_i64(args.get(index), "--fred-chunk-days")?;
                }
                "--skip-world-bank" => {
                    skip_world_bank = true;
                }
                "--include-gdelt" => {
                    include_gdelt = true;
                }
                "--no-reload-api" => {
                    reload_api = false;
                }
                "--api-reload-url" => {
                    index += 1;
                    api_reload_url = args
                        .get(index)
                        .with_context(|| "--api-reload-url requires a URL")?
                        .clone();
                }
                other => bail!("unknown refresh option: {other}"),
            }
            index += 1;
        }

        Ok(Self {
            fast_lookback_days,
            slow_lookback_years,
            fred_chunk_days,
            skip_world_bank,
            include_gdelt,
            reload_api,
            api_reload_url,
        })
    }
}

#[derive(Debug, Clone, Copy)]
enum FredBackfillMode {
    GraphCsv,
    Api,
}

async fn fetch_api_json<T: DeserializeOwned>(
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

fn read_release_manifest(path: &Path) -> anyhow::Result<ModelReleaseManifest> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read release manifest {}", path.display()))?;
    serde_json::from_str::<ModelReleaseManifest>(&raw)
        .with_context(|| format!("failed to decode release manifest {}", path.display()))
}

async fn resolve_release_market_scope(
    store: &SqliteStore,
    release_id: &str,
    override_market_scope: Option<&str>,
) -> anyhow::Result<String> {
    if let Some(market_scope) = override_market_scope {
        return Ok(market_scope.to_string());
    }
    let release = store
        .load_model_release(release_id)
        .await?
        .with_context(|| format!("release {release_id} not found"))?;
    Ok(release.manifest.market_scope)
}

fn truncate_text(value: &str, max_len: usize) -> String {
    if value.chars().count() <= max_len {
        return value.to_string();
    }
    let prefix_len = max_len.saturating_sub(1);
    let mut truncated = value.chars().take(prefix_len).collect::<String>();
    truncated.push('…');
    truncated
}

fn render_audit_report_markdown(report: &AuditExportEnvelope) -> String {
    let rolling_audit = &report.assessment.backtest_summary.rolling_audit;
    let mut markdown = String::new();
    let _ = writeln!(markdown, "# Rolling Audit Report");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "- Exported at: {}", report.exported_at);
    let _ = writeln!(markdown, "- API base: {}", report.api_base_url);
    let _ = writeln!(markdown, "- As of: {}", report.assessment.as_of_date);
    let _ = writeln!(
        markdown,
        "- Data mode: {}",
        data_mode_text(report.assessment.runtime.data_mode)
    );
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Current Assessment");
    let _ = writeln!(markdown);
    let _ = writeln!(
        markdown,
        "- Overall score: {:.1}",
        report.assessment.scores.overall_score
    );
    let _ = writeln!(
        markdown,
        "- Posture: {}",
        posture_text(report.assessment.posture)
    );
    let _ = writeln!(
        markdown,
        "- Time bucket: {}",
        time_bucket_text(report.assessment.time_to_risk_bucket)
    );
    let _ = writeln!(
        markdown,
        "- Probability 5d / 20d / 60d: {} / {} / {}",
        format_pct(report.assessment.probabilities.p_5d),
        format_pct(report.assessment.probabilities.p_20d),
        format_pct(report.assessment.probabilities.p_60d)
    );
    let _ = writeln!(markdown, "- Summary: {}", report.assessment.summary);
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Serving Method");
    let _ = writeln!(markdown);
    let _ = writeln!(
        markdown,
        "- Release ID: {}",
        report
            .method
            .method
            .release_id
            .as_deref()
            .unwrap_or("inline")
    );
    let _ = writeln!(
        markdown,
        "- Probability mode: {}",
        report.method.method.probability_mode
    );
    let _ = writeln!(
        markdown,
        "- Release status: {}",
        report.method.method.release_status
    );
    let _ = writeln!(
        markdown,
        "- Point-in-time mode: {}",
        report.method.method.point_in_time_mode
    );
    let _ = writeln!(
        markdown,
        "- Versions: score={} prob={} calib={} feature={} label={} playbook={}",
        report.method.method.score_method_version,
        report.method.method.prob_model_version,
        report.method.method.calibration_version,
        report.method.method.feature_set_version,
        report.method.method.label_version,
        report.method.method.action_playbook_version
    );
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Rolling Audit");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "{}", rolling_audit.summary);
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "| Metric | Value |");
    let _ = writeln!(markdown, "| --- | --- |");
    let _ = writeln!(
        markdown,
        "| Actionable precision | {} |",
        format_pct(rolling_audit.actionable_precision)
    );
    let _ = writeln!(
        markdown,
        "| Actionable signal count | {} |",
        rolling_audit.actionable_signal_count
    );
    let _ = writeln!(
        markdown,
        "| Pre-crisis hit count | {} |",
        rolling_audit.pre_crisis_signal_count
    );
    let _ = writeln!(
        markdown,
        "| In-crisis signal count | {} |",
        rolling_audit.in_crisis_signal_count
    );
    let _ = writeln!(
        markdown,
        "| Protected stress count | {} |",
        rolling_audit.stress_window_signal_count
    );
    let _ = writeln!(
        markdown,
        "| Pure false-positive count | {} |",
        rolling_audit.false_positive_signal_count
    );
    let _ = writeln!(
        markdown,
        "| Pure false-positive episodes | {} |",
        rolling_audit.false_positive_episode_count
    );
    let _ = writeln!(
        markdown,
        "| Longest pure false-positive episode | {}d |",
        rolling_audit.longest_false_positive_episode_days
    );
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Largest Non-crisis Action Episodes");
    let _ = writeln!(markdown);
    let _ = writeln!(
        markdown,
        "| Classification | Window | Duration | Signals | Note |"
    );
    let _ = writeln!(markdown, "| --- | --- | --- | --- | --- |");
    for episode in &rolling_audit.classified_episodes {
        let _ = writeln!(
            markdown,
            "| {} | {} .. {} | {}d | {} | {} |",
            episode.classification,
            episode.start_date,
            episode.end_date,
            episode.duration_days,
            episode.signal_count,
            episode.note.replace('|', "/")
        );
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Scenario Backtests");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "| Scenario | Source | Crisis Window | Structural Lead | Actionable Lead | Max Score | Foldbacks | Note |");
    let _ = writeln!(
        markdown,
        "| --- | --- | --- | --- | --- | --- | --- | --- |"
    );
    for scenario in &report.backtests {
        let _ = writeln!(
            markdown,
            "| {} | {} | {} .. {} | {} | {} | {:.1} | {} | {} |",
            scenario.name,
            backtest_signal_source_text(scenario.signal_source),
            scenario.crisis_start,
            scenario.crisis_end,
            format_optional_days(scenario.lead_time_days),
            format_optional_days(scenario.actionable_lead_time_days),
            scenario.max_score,
            scenario.false_positive_count,
            scenario.note.replace('|', "/")
        );
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Protected Stress Window Catalog");
    let _ = writeln!(markdown);
    let _ = writeln!(
        markdown,
        "- Catalog: {}",
        report.method.protected_stress_window_catalog.catalog_id
    );
    let _ = writeln!(
        markdown,
        "- Source: {}",
        report.method.protected_stress_window_catalog.source
    );
    let _ = writeln!(
        markdown,
        "- Note: {}",
        report.method.protected_stress_window_catalog.note
    );
    if let Some(warning) = &report.method.protected_stress_window_catalog.warning {
        let _ = writeln!(markdown, "- Warning: {warning}");
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "| Window | Range | Note |");
    let _ = writeln!(markdown, "| --- | --- | --- |");
    for window in &report.method.protected_stress_window_catalog.windows {
        let _ = writeln!(
            markdown,
            "| {} | {} .. {} | {} |",
            window.label,
            window.start_date,
            window.end_date,
            window.note.replace('|', "/")
        );
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Method Note");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "{}", report.method.note);
    markdown
}

fn data_mode_text(mode: fc_domain::DataMode) -> &'static str {
    match mode {
        fc_domain::DataMode::Demo => "demo",
        fc_domain::DataMode::Sqlite => "sqlite",
        fc_domain::DataMode::Postgres => "postgres",
    }
}

fn posture_text(posture: fc_domain::DecisionPosture) -> &'static str {
    match posture {
        fc_domain::DecisionPosture::Normal => "normal",
        fc_domain::DecisionPosture::Prepare => "prepare",
        fc_domain::DecisionPosture::Hedge => "hedge",
        fc_domain::DecisionPosture::Defend => "defend",
    }
}

fn time_bucket_text(bucket: fc_domain::TimeToRiskBucket) -> &'static str {
    match bucket {
        fc_domain::TimeToRiskBucket::Normal => "normal",
        fc_domain::TimeToRiskBucket::Months => "months",
        fc_domain::TimeToRiskBucket::Weeks => "weeks",
        fc_domain::TimeToRiskBucket::Now => "now",
    }
}

fn backtest_signal_source_text(source: fc_domain::BacktestSignalSource) -> &'static str {
    match source {
        fc_domain::BacktestSignalSource::RealHistory => "real_history",
        fc_domain::BacktestSignalSource::FallbackTemplate => "fallback_template",
    }
}

fn format_pct(value: f64) -> String {
    format!("{:.1}%", value * 100.0)
}

fn format_optional_days(value: Option<i64>) -> String {
    value
        .map(|days| format!("{days}d"))
        .unwrap_or_else(|| "—".to_string())
}

fn parse_date_arg(value: Option<&String>, option: &str) -> anyhow::Result<NaiveDate> {
    let value = value.with_context(|| format!("{option} requires a YYYY-MM-DD value"))?;
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .with_context(|| format!("{option} must use YYYY-MM-DD"))
}

fn parse_positive_i64(value: Option<&String>, option: &str) -> anyhow::Result<i64> {
    let value = value
        .with_context(|| format!("{option} requires a positive integer"))?
        .parse::<i64>()
        .with_context(|| format!("{option} requires a positive integer"))?;
    if value <= 0 {
        bail!("{option} requires a positive integer");
    }
    Ok(value)
}

async fn open_sqlite_store() -> anyhow::Result<SqliteStore> {
    SqliteStore::connect(sqlite_path())
        .await
        .map_err(Into::into)
}

fn sqlite_path() -> String {
    env::var("FC_SQLITE_PATH").unwrap_or_else(|_| DEFAULT_SQLITE_PATH.to_string())
}

fn raw_data_dir() -> PathBuf {
    env::var("FC_RAW_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_RAW_DATA_DIR))
}

fn write_raw_payload(
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

fn raw_file_extension(content_type: &str) -> &'static str {
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

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn simple_hash(input: &str) -> String {
    let hash = input.as_bytes().iter().fold(0_u64, |acc, byte| {
        acc.wrapping_mul(31).wrapping_add(*byte as u64)
    });
    format!("{hash:x}")
}

async fn reload_api_runtime(url: &str) -> anyhow::Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()?;
    let response = client.post(url).send().await?;
    if !response.status().is_success() {
        bail!("reload endpoint returned {}", response.status());
    }
    Ok(())
}

fn print_help() {
    println!(
        r#"fc-worker commands:
  cargo run -p fc-worker
      Run the original mock ingestion demo.

  cargo run -p fc-worker -- db init
      Create or migrate the local SQLite database.

  cargo run -p fc-worker -- db seed
      Seed FRED, Treasury, entity, indicator, and mapping metadata.

  cargo run -p fc-worker -- db check
      Check whether key SQLite indicators are fresh enough for the dashboard.

  cargo run -p fc-worker -- audit export-current [--api-base-url URL] [--output-dir DIR]
      Fetch /api/assessment/current, /api/backtests, and /api/assessment/method from the running API, then export a JSON + Markdown rolling-audit report.

  cargo run -p fc-worker -- research release publish --manifest FILE [--activate] [--reload-api] [--api-reload-url URL] [--updated-by NAME]
      Save a release manifest into SQLite, and optionally activate it and reload the API runtime.

  cargo run -p fc-worker -- research release list [--market-scope SCOPE]
      List model releases stored in SQLite.

  cargo run -p fc-worker -- research release show --release-id ID
      Print a stored model release as JSON.

  cargo run -p fc-worker -- research release activate --release-id ID [--market-scope SCOPE] [--reload-api] [--api-reload-url URL] [--updated-by NAME]
      Mark a release active for the selected market scope and optionally reload the API runtime.

  cargo run -p fc-worker -- research release rollback --to-release-id ID [--market-scope SCOPE] [--reload-api] [--api-reload-url URL] [--updated-by NAME]
      Roll back the selected market scope to an earlier release and optionally reload the API runtime.

  cargo run -p fc-worker -- research snapshot list [--market-scope SCOPE] [--release-id ID] [--from YYYY-MM-DD] [--to YYYY-MM-DD] [--limit N]
      List persisted prediction snapshots stored in SQLite for audit and release-review work.

  cargo run -p fc-worker -- research snapshot export [--market-scope SCOPE] [--release-id ID] [--from YYYY-MM-DD] [--to YYYY-MM-DD] [--limit N] [--format json|csv] [--output-path FILE]
      Export persisted prediction snapshots as JSON or CSV for external audit and release review.

  cargo run -p fc-worker -- research snapshot dataset [--market-scope SCOPE] [--release-id ID] [--from YYYY-MM-DD] [--to YYYY-MM-DD] [--format json|csv] [--output-path FILE]
      Build a point-in-time feature + forward-crisis-label dataset from persisted prediction snapshots.

  cargo run -p fc-worker -- research pipeline train-probability [--market-scope SCOPE] [--release-id ID] [--from YYYY-MM-DD] [--to YYYY-MM-DD] [--output-dir DIR] [--release-prefix PREFIX]
      Train a transitional formal probability bundle from persisted prediction snapshots, then emit bundle / manifest / evaluation artifacts.

  cargo run -p fc-worker -- research pipeline bootstrap-formal-release [--market-scope SCOPE] [--release-id ID] [--from YYYY-MM-DD] [--to YYYY-MM-DD] [--output-dir DIR] [--release-prefix PREFIX] [--no-activate] [--no-reload-api] [--api-reload-url URL] [--updated-by NAME]
      Train a formal bundle, publish it into SQLite as a model release, optionally activate it, and optionally reload the API runtime.

  cargo run -p fc-worker -- refresh latest-free [--fast-lookback-days N] [--slow-lookback-years N] [--fred-chunk-days N] [--skip-world-bank] [--include-gdelt] [--no-reload-api] [--api-reload-url URL]
      Refresh the latest free-source data set for the dashboard, then optionally POST /api/system/reload.

  cargo run -p fc-worker -- backfill fred [--start YYYY-MM-DD] [--end YYYY-MM-DD] [--chunk-days N] [--indicator ID] [--external-code CODE]
      Fetch FRED public graph CSV observations into SQLite. No API key required. Graph CSV is chunked by default.

  cargo run -p fc-worker -- backfill fred --api [--start YYYY-MM-DD] [--end YYYY-MM-DD] [--indicator ID] [--external-code CODE]
      Fetch FRED official API observations into SQLite. Requires FRED_API_KEY.

  cargo run -p fc-worker -- backfill treasury-yield [--start YYYY-MM-DD] [--end YYYY-MM-DD] [--indicator ID] [--external-code CODE]
      Fetch U.S. Treasury yield curve observations into SQLite. No API key required.

  cargo run -p fc-worker -- backfill world-bank [--start YYYY-MM-DD] [--end YYYY-MM-DD] [--indicator ID] [--external-code CODE]
      Fetch World Bank annual macro indicators into SQLite. No API key required.

  cargo run -p fc-worker -- backfill sec-edgar [--start YYYY-MM-DD] [--end YYYY-MM-DD]
      Fetch SEC submissions metadata for the U.S. financial watchlist, aggregate filing-event features, and write alerts into SQLite. No API key required.

  cargo run -p fc-worker -- backfill gdelt [--start YYYY-MM-DD] [--end YYYY-MM-DD] [--watermark-overlap-days N]
      Fetch GDELT DOC timeline aggregates for banking/liquidity stress coverage, write raw payloads, observations, and prototype alerts into SQLite. No API key required.

  cargo run -p fc-worker -- backfill boj --dataset fx-daily [--start YYYY-MM-DD] [--end YYYY-MM-DD] [--indicator ID] [--external-code CODE]
      Fetch official BOJ USDJPY history into SQLite. No API key required.

  cargo run -p fc-worker -- backfill boj --dataset money-market [--start YYYY-MM-DD] [--end YYYY-MM-DD] [--indicator ID] [--external-code CODE]
      Fetch official BOJ uncollateralized overnight call rate history into SQLite. No API key required.

  cargo run -p fc-worker -- backfill jpy-carry [--start YYYY-MM-DD] [--end YYYY-MM-DD] [--indicator ID] [--external-code CODE]
      Fetch JPY carry USDJPY history. BOJ official FX is tried first, then FRED graph CSV is used as fallback.
"#
    );
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::NaiveDate;

    use super::{
        AuditExportOptions, PredictionSnapshotQueryOptions, RefreshLatestOptions,
        ReleasePublishOptions, ReleaseSwitchOptions,
    };

    #[test]
    fn parses_refresh_latest_defaults() {
        let options = RefreshLatestOptions::parse(&[]).unwrap();
        assert_eq!(options.fast_lookback_days, 45);
        assert_eq!(options.slow_lookback_years, 15);
        assert_eq!(options.fred_chunk_days, 45);
        assert!(!options.skip_world_bank);
        assert!(!options.include_gdelt);
        assert!(options.reload_api);
    }

    #[test]
    fn parses_refresh_latest_overrides() {
        let args = vec![
            "--fast-lookback-days".to_string(),
            "90".to_string(),
            "--skip-world-bank".to_string(),
            "--include-gdelt".to_string(),
            "--no-reload-api".to_string(),
        ];
        let options = RefreshLatestOptions::parse(&args).unwrap();
        assert_eq!(options.fast_lookback_days, 90);
        assert!(options.skip_world_bank);
        assert!(options.include_gdelt);
        assert!(!options.reload_api);
    }

    #[test]
    fn parses_audit_export_overrides() {
        let args = vec![
            "--api-base-url".to_string(),
            "http://127.0.0.1:18081".to_string(),
            "--output-dir".to_string(),
            "tmp/audit".to_string(),
        ];
        let options = AuditExportOptions::parse(&args).unwrap();
        assert_eq!(options.api_base_url, "http://127.0.0.1:18081");
        assert_eq!(options.output_dir, PathBuf::from("tmp/audit"));
    }

    #[test]
    fn parses_release_publish_options() {
        let args = vec![
            "--manifest".to_string(),
            "config/model-releases/us-heuristic-bootstrap.json".to_string(),
            "--activate".to_string(),
            "--reload-api".to_string(),
            "--updated-by".to_string(),
            "tester".to_string(),
        ];
        let options = ReleasePublishOptions::parse(&args).unwrap();
        assert!(options.activate);
        assert!(options.reload_api);
        assert_eq!(options.updated_by, "tester");
        assert_eq!(
            options.manifest_path,
            PathBuf::from("config/model-releases/us-heuristic-bootstrap.json")
        );
    }

    #[test]
    fn parses_release_switch_options() {
        let args = vec![
            "--release-id".to_string(),
            "release-123".to_string(),
            "--market-scope".to_string(),
            "financial_system".to_string(),
            "--reload-api".to_string(),
        ];
        let options = ReleaseSwitchOptions::parse(&args).unwrap();
        assert_eq!(options.release_id, "release-123");
        assert_eq!(options.market_scope.as_deref(), Some("financial_system"));
        assert!(options.reload_api);
    }

    #[test]
    fn parses_prediction_snapshot_query_options() {
        let args = vec![
            "--market-scope".to_string(),
            "financial_system".to_string(),
            "--from".to_string(),
            "2026-05-01".to_string(),
            "--to".to_string(),
            "2026-05-31".to_string(),
            "--limit".to_string(),
            "50".to_string(),
        ];
        let options = PredictionSnapshotQueryOptions::parse(&args).unwrap();
        assert_eq!(options.market_scope.as_deref(), Some("financial_system"));
        assert_eq!(
            options.from,
            Some(NaiveDate::from_ymd_opt(2026, 5, 1).unwrap())
        );
        assert_eq!(
            options.to,
            Some(NaiveDate::from_ymd_opt(2026, 5, 31).unwrap())
        );
        assert_eq!(options.limit, Some(50));
    }
}

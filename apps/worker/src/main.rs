use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    fmt::Write,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc, Weekday};
use fc_domain::{
    embedded_protected_stress_window_catalog, load_crisis_scenario_catalog, ActionabilityBundle,
    ActionabilityEvaluationSummary, ActionabilityLevel, ActionabilityLevelBundle,
    AssessmentMethodVersions, AssessmentSnapshot, BacktestScenarioSummary, FeatureSnapshotRecord,
    FormalDatasetManifest, FormalDatasetRecord, FormalDatasetRowRecord, Frequency,
    HorizonEvaluationSummary, Indicator, IndicatorRisk, LogisticProbabilityModel,
    ModelReleaseManifest, ModelReleaseRecord, Observation, PlattCalibrationArtifact,
    PredictionSnapshotRecord, ProbabilityBundle, ProbabilityBundleEvaluation,
    ProbabilityCoefficient, ProbabilityFeatureStat, ProbabilityHorizonBundle,
    ProtectedStressWindowCatalog, RiskDimension, FEATURE_BUCKET_MONTHS_OR_HIGHER,
    FEATURE_BUCKET_NOW, FEATURE_BUCKET_WEEKS_OR_HIGHER, FEATURE_COVERAGE_SCORE,
    FEATURE_EXTERNAL_SHOCK_SCORE, FEATURE_FRESHNESS_DELAYED_OR_WORSE,
    FEATURE_FRESHNESS_STALE_OR_MISSING, FEATURE_HEURISTIC_P_20D, FEATURE_HEURISTIC_P_5D,
    FEATURE_HEURISTIC_P_60D, FEATURE_OVERALL_SCORE, FORMAL_PROBABILITY_BUNDLE_FEATURES,
    TRANSITIONAL_PROBABILITY_BUNDLE_FEATURES,
};
use fc_ingestion::{
    BojConnector, BojDataset, Connector, FetchPlan, FredConnector, FredGraphCsvConnector,
    GdeltConnector, MockConnector, RunMode, SecEdgarConnector, TreasuryYieldCurveConnector,
    WorldBankConnector,
};
use fc_scoring::ScoringEngine;
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
const DEFAULT_FORMAL_FEATURE_SET_VERSION: &str = "feature_formal_v1_main_20260531";
const DEFAULT_FORMAL_DATASET_ID: &str = "formal_v1_main_1990_daily";
const DEFAULT_FORMAL_LABEL_VERSION: &str = "formal_label_v1_main";
const DEFAULT_FORMAL_SCENARIO_SET_VERSION: &str = "scenario_v1_main";
const FEATURE_SNAPSHOT_STATUS_READY: &str = "ready";
const FEATURE_SNAPSHOT_STATUS_COVERAGE_OR_VISIBILITY_FAILED: &str = "coverage_or_visibility_failed";

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
                "review" => research_release_review(rest).await,
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
        [scope, area, action, rest @ ..] if scope == "research" && area == "feature" => {
            match action.as_str() {
                "build" => research_feature_snapshot_build(rest).await,
                "list" => research_feature_snapshot_list(rest).await,
                _ => {
                    print_help();
                    bail!("unknown research feature command")
                }
            }
        }
        [scope, area, action, rest @ ..] if scope == "research" && area == "dataset" => {
            match action.as_str() {
                "build-main" => research_formal_dataset_build_main(rest).await,
                "list-main" => research_formal_dataset_list_main(rest).await,
                "summarize-main" => research_formal_dataset_summarize_main(rest).await,
                _ => {
                    print_help();
                    bail!("unknown research dataset command")
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
    skip_operational_guard: bool,
    updated_by: String,
}

impl ReleasePublishOptions {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut manifest_path = None;
        let mut activate = false;
        let mut reload_api = false;
        let mut api_reload_url = DEFAULT_API_RELOAD_URL.to_string();
        let mut skip_operational_guard = false;
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
                "--skip-operational-guard" => skip_operational_guard = true,
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
            skip_operational_guard,
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
    skip_operational_guard: bool,
    updated_by: String,
}

impl ReleaseSwitchOptions {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut release_id = None;
        let mut market_scope = None;
        let mut reload_api = false;
        let mut api_reload_url = DEFAULT_API_RELOAD_URL.to_string();
        let mut skip_operational_guard = false;
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
                "--skip-operational-guard" => skip_operational_guard = true,
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
            skip_operational_guard,
            updated_by,
        })
    }
}

#[derive(Debug, Clone)]
struct ReleaseReviewOptions {
    candidate_release_id: String,
    baseline_release_id: Option<String>,
    market_scope: Option<String>,
    api_reload_url: String,
    output_dir: PathBuf,
    updated_by: String,
}

impl ReleaseReviewOptions {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut candidate_release_id = None;
        let mut baseline_release_id = None;
        let mut market_scope = None;
        let mut api_reload_url = DEFAULT_API_RELOAD_URL.to_string();
        let mut output_dir = PathBuf::from("reports/release-review");
        let mut updated_by = "fc-worker-review".to_string();
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--candidate-release-id" => {
                    index += 1;
                    candidate_release_id = Some(
                        args.get(index)
                            .with_context(|| "--candidate-release-id requires a value")?
                            .clone(),
                    );
                }
                "--baseline-release-id" => {
                    index += 1;
                    baseline_release_id = Some(
                        args.get(index)
                            .with_context(|| "--baseline-release-id requires a value")?
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
                "--api-reload-url" => {
                    index += 1;
                    api_reload_url = args
                        .get(index)
                        .with_context(|| "--api-reload-url requires a URL")?
                        .clone();
                }
                "--output-dir" => {
                    index += 1;
                    output_dir = PathBuf::from(
                        args.get(index)
                            .with_context(|| "--output-dir requires a directory path")?,
                    );
                }
                "--updated-by" => {
                    index += 1;
                    updated_by = args
                        .get(index)
                        .with_context(|| "--updated-by requires a value")?
                        .clone();
                }
                other => bail!("unknown release review option: {other}"),
            }
            index += 1;
        }
        Ok(Self {
            candidate_release_id: candidate_release_id
                .with_context(|| "--candidate-release-id is required")?,
            baseline_release_id,
            market_scope,
            api_reload_url,
            output_dir,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PipelineDatasetSource {
    Formal,
    Snapshot,
}

impl PipelineDatasetSource {
    fn parse(value: &str) -> anyhow::Result<Self> {
        match value {
            "formal" => Ok(Self::Formal),
            "snapshot" => Ok(Self::Snapshot),
            other => bail!("unsupported --dataset-source value: {other}"),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Formal => "formal",
            Self::Snapshot => "snapshot",
        }
    }
}

#[derive(Debug, Clone)]
struct PipelineTrainOptions {
    dataset_source: PipelineDatasetSource,
    dataset_id: String,
    dataset_version: Option<String>,
    dataset_key: Option<String>,
    query: PredictionSnapshotQueryOptions,
    output_dir: PathBuf,
    release_prefix: String,
}

impl PipelineTrainOptions {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut output_dir = PathBuf::from("config/model-bundles/generated");
        let mut release_prefix = None;
        let mut dataset_source = PipelineDatasetSource::Formal;
        let mut dataset_id = DEFAULT_FORMAL_DATASET_ID.to_string();
        let mut dataset_version = None;
        let mut dataset_key = None;
        let mut query_args = Vec::new();
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--dataset-source" => {
                    index += 1;
                    dataset_source = PipelineDatasetSource::parse(
                        args.get(index)
                            .with_context(|| "--dataset-source requires a value")?,
                    )?;
                }
                "--dataset-id" => {
                    index += 1;
                    dataset_id = args
                        .get(index)
                        .with_context(|| "--dataset-id requires a value")?
                        .clone();
                }
                "--dataset-version" => {
                    index += 1;
                    dataset_version = Some(
                        args.get(index)
                            .with_context(|| "--dataset-version requires a value")?
                            .clone(),
                    );
                }
                "--dataset-key" => {
                    index += 1;
                    dataset_key = Some(
                        args.get(index)
                            .with_context(|| "--dataset-key requires a value")?
                            .clone(),
                    );
                }
                "--output-dir" => {
                    index += 1;
                    output_dir = PathBuf::from(
                        args.get(index)
                            .with_context(|| "--output-dir requires a path")?,
                    );
                }
                "--release-prefix" => {
                    index += 1;
                    release_prefix = Some(
                        args.get(index)
                            .with_context(|| "--release-prefix requires a value")?
                            .clone(),
                    );
                }
                other => query_args.push(other.to_string()),
            }
            index += 1;
        }

        let release_prefix = release_prefix.unwrap_or_else(|| match dataset_source {
            PipelineDatasetSource::Formal => "us_formal_main".to_string(),
            PipelineDatasetSource::Snapshot => "us_formal_transitional".to_string(),
        });

        Ok(Self {
            dataset_source,
            dataset_id,
            dataset_version,
            dataset_key,
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
    skip_operational_guard: bool,
    updated_by: String,
}

impl PipelineBootstrapOptions {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut activate = true;
        let mut reload_api = true;
        let mut api_reload_url = DEFAULT_API_RELOAD_URL.to_string();
        let mut skip_operational_guard = false;
        let mut updated_by = "fc-worker".to_string();
        let mut train_args = Vec::new();
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--no-activate" => activate = false,
                "--no-reload-api" => reload_api = false,
                "--skip-operational-guard" => skip_operational_guard = true,
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
            skip_operational_guard,
            updated_by,
        })
    }
}

#[derive(Debug, Clone)]
struct FeatureSnapshotBuildOptions {
    market_scope: String,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    feature_set_version: String,
    point_in_time_mode: String,
    force_rebuild: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PointInTimeMode {
    BestEffort,
    Strict,
}

impl PointInTimeMode {
    fn parse(raw: &str) -> anyhow::Result<Self> {
        match raw {
            "best_effort" => Ok(Self::BestEffort),
            "strict" => Ok(Self::Strict),
            other => bail!(
                "unsupported --point-in-time-mode value: {other}; supported values are best_effort and strict"
            ),
        }
    }
}

impl FeatureSnapshotBuildOptions {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut market_scope = "financial_system".to_string();
        let mut from = None;
        let mut to = None;
        let mut feature_set_version = DEFAULT_FORMAL_FEATURE_SET_VERSION.to_string();
        let mut point_in_time_mode = "best_effort".to_string();
        let mut force_rebuild = false;
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--market-scope" => {
                    index += 1;
                    market_scope = args
                        .get(index)
                        .with_context(|| "--market-scope requires a value")?
                        .clone();
                }
                "--from" => {
                    index += 1;
                    from = Some(parse_date_arg(args.get(index), "--from")?);
                }
                "--to" => {
                    index += 1;
                    to = Some(parse_date_arg(args.get(index), "--to")?);
                }
                "--feature-set-version" => {
                    index += 1;
                    feature_set_version = args
                        .get(index)
                        .with_context(|| "--feature-set-version requires a value")?
                        .clone();
                }
                "--point-in-time-mode" => {
                    index += 1;
                    point_in_time_mode = args
                        .get(index)
                        .with_context(|| "--point-in-time-mode requires a value")?
                        .clone();
                }
                "--force-rebuild" => {
                    force_rebuild = true;
                }
                other => bail!("unknown feature snapshot build option: {other}"),
            }
            index += 1;
        }
        PointInTimeMode::parse(&point_in_time_mode)?;
        Ok(Self {
            market_scope,
            from,
            to,
            feature_set_version,
            point_in_time_mode,
            force_rebuild,
        })
    }
}

#[derive(Debug, Clone)]
struct FeatureSnapshotListOptions {
    market_scope: Option<String>,
    feature_set_version: Option<String>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    limit: Option<usize>,
}

impl FeatureSnapshotListOptions {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut market_scope = None;
        let mut feature_set_version = None;
        let mut from = None;
        let mut to = None;
        let mut limit = Some(20_usize);
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
                "--feature-set-version" => {
                    index += 1;
                    feature_set_version = Some(
                        args.get(index)
                            .with_context(|| "--feature-set-version requires a value")?
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
                other => bail!("unknown feature snapshot list option: {other}"),
            }
            index += 1;
        }
        Ok(Self {
            market_scope,
            feature_set_version,
            from,
            to,
            limit,
        })
    }
}

#[derive(Debug, Clone)]
struct FormalDatasetBuildOptions {
    feature: FeatureSnapshotBuildOptions,
    dataset_id: String,
    dataset_version: Option<String>,
    label_version: String,
    scenario_set_version: String,
}

impl FormalDatasetBuildOptions {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut dataset_id = DEFAULT_FORMAL_DATASET_ID.to_string();
        let mut dataset_version = None;
        let mut label_version = DEFAULT_FORMAL_LABEL_VERSION.to_string();
        let mut scenario_set_version = DEFAULT_FORMAL_SCENARIO_SET_VERSION.to_string();
        let mut feature_args = Vec::new();
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--dataset-id" => {
                    index += 1;
                    dataset_id = args
                        .get(index)
                        .with_context(|| "--dataset-id requires a value")?
                        .clone();
                }
                "--dataset-version" => {
                    index += 1;
                    dataset_version = Some(
                        args.get(index)
                            .with_context(|| "--dataset-version requires a value")?
                            .clone(),
                    );
                }
                "--label-version" => {
                    index += 1;
                    label_version = args
                        .get(index)
                        .with_context(|| "--label-version requires a value")?
                        .clone();
                }
                "--scenario-set-version" => {
                    index += 1;
                    scenario_set_version = args
                        .get(index)
                        .with_context(|| "--scenario-set-version requires a value")?
                        .clone();
                }
                other => feature_args.push(other.to_string()),
            }
            index += 1;
        }
        Ok(Self {
            feature: FeatureSnapshotBuildOptions::parse(&feature_args)?,
            dataset_id,
            dataset_version,
            label_version,
            scenario_set_version,
        })
    }
}

#[derive(Debug, Clone)]
struct FormalDatasetListOptions {
    market_scope: Option<String>,
    dataset_id: Option<String>,
    limit: Option<usize>,
}

impl FormalDatasetListOptions {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut market_scope = None;
        let mut dataset_id = None;
        let mut limit = Some(10_usize);
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
                "--dataset-id" => {
                    index += 1;
                    dataset_id = Some(
                        args.get(index)
                            .with_context(|| "--dataset-id requires a value")?
                            .clone(),
                    );
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
                other => bail!("unknown formal dataset list option: {other}"),
            }
            index += 1;
        }
        Ok(Self {
            market_scope,
            dataset_id,
            limit,
        })
    }
}

#[derive(Debug, Clone)]
struct FormalDatasetSummaryOptions {
    market_scope: Option<String>,
    dataset_id: String,
    dataset_version: Option<String>,
    dataset_key: Option<String>,
    output_dir: PathBuf,
}

impl FormalDatasetSummaryOptions {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut market_scope = None;
        let mut dataset_id = DEFAULT_FORMAL_DATASET_ID.to_string();
        let mut dataset_version = None;
        let mut dataset_key = None;
        let mut output_dir = PathBuf::from("reports/formal-dataset");
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
                "--dataset-id" => {
                    index += 1;
                    dataset_id = args
                        .get(index)
                        .with_context(|| "--dataset-id requires a value")?
                        .clone();
                }
                "--dataset-version" => {
                    index += 1;
                    dataset_version = Some(
                        args.get(index)
                            .with_context(|| "--dataset-version requires a value")?
                            .clone(),
                    );
                }
                "--dataset-key" => {
                    index += 1;
                    dataset_key = Some(
                        args.get(index)
                            .with_context(|| "--dataset-key requires a value")?
                            .clone(),
                    );
                }
                "--output-dir" => {
                    index += 1;
                    output_dir = PathBuf::from(
                        args.get(index)
                            .with_context(|| "--output-dir requires a directory path")?,
                    );
                }
                other => bail!("unknown formal dataset summary option: {other}"),
            }
            index += 1;
        }
        Ok(Self {
            market_scope,
            dataset_id,
            dataset_version,
            dataset_key,
            output_dir,
        })
    }
}

#[derive(Debug, Clone)]
struct CrisisScenario {
    scenario_id: String,
    family: String,
    pre_warning_start: NaiveDate,
    crisis_start: NaiveDate,
    acute_start: Option<NaiveDate>,
    crisis_end: NaiveDate,
    default_horizon_roles: Vec<u32>,
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

#[derive(Debug, Clone, Serialize)]
struct ReleaseReviewScalarMetric {
    baseline: f64,
    candidate: f64,
    delta: f64,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseReviewCountMetric {
    baseline: u32,
    candidate: u32,
    delta: i64,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseReviewComparisonSummary {
    timely_warning_rate: ReleaseReviewScalarMetric,
    actionable_precision: ReleaseReviewScalarMetric,
    longest_false_positive_episode_days: ReleaseReviewCountMetric,
    current_p_5d: ReleaseReviewScalarMetric,
    current_p_20d: ReleaseReviewScalarMetric,
    current_p_60d: ReleaseReviewScalarMetric,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseActionabilityLevelReview {
    level: ActionabilityLevel,
    proxy_horizon_days: u32,
    sample_count: u32,
    positive_rate: f64,
    threshold: f64,
    predicted_positive_count: u32,
    actual_positive_count: u32,
    pre_start_positive_count: u32,
    post_start_positive_count: u32,
    pre_start_hit_count: u32,
    post_start_hit_count: u32,
    false_positive_count: u32,
    scenario_count: u32,
    advance_warning_scenario_count: u32,
    late_confirmation_scenario_count: u32,
    missed_scenario_count: u32,
    precision_at_threshold: Option<f64>,
    pre_start_recall_at_threshold: Option<f64>,
    post_start_recall_at_threshold: Option<f64>,
    advance_warning_rate: Option<f64>,
    late_confirmation_rate: Option<f64>,
    missed_rate: Option<f64>,
    note: String,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseActionabilityReview {
    release_id: String,
    enabled: bool,
    model_version: Option<String>,
    calibration_version: Option<String>,
    fusion_policy_version: Option<String>,
    levels: Vec<ReleaseActionabilityLevelReview>,
    guard_regressions: Vec<String>,
    guard_passed: bool,
    note: String,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseReviewEnvelope {
    reviewed_at: String,
    market_scope: String,
    api_reload_url: String,
    original_active_release_id: String,
    restored_release_id: String,
    baseline_release: ModelReleaseRecord,
    candidate_release: ModelReleaseRecord,
    baseline_assessment: AssessmentSnapshot,
    candidate_assessment: AssessmentSnapshot,
    baseline_actionability_review: ReleaseActionabilityReview,
    candidate_actionability_review: ReleaseActionabilityReview,
    comparison: ReleaseReviewComparisonSummary,
    operational_guard_regressions: Vec<String>,
    operational_guard_passed: bool,
    actionability_guard_regressions: Vec<String>,
    actionability_guard_passed: bool,
    overall_guard_regressions: Vec<String>,
    overall_guard_passed: bool,
    recommendation: String,
}

#[derive(Debug, Clone, Serialize)]
struct FormalDatasetSplitSummary {
    split_name: String,
    row_count: usize,
    positive_5d_count: usize,
    positive_5d_rate: f64,
    positive_20d_count: usize,
    positive_20d_rate: f64,
    positive_60d_count: usize,
    positive_60d_rate: f64,
    action_positive_5d_count: usize,
    action_positive_5d_rate: f64,
    action_positive_20d_count: usize,
    action_positive_20d_rate: f64,
    action_positive_60d_count: usize,
    action_positive_60d_rate: f64,
    avg_coverage_score: f64,
    avg_core_feature_coverage: f64,
    avg_trigger_feature_coverage: f64,
    avg_external_feature_coverage: f64,
    scenario_count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct FormalDatasetScenarioSummary {
    scenario_id: String,
    row_count: usize,
    split_count: usize,
    first_as_of_date: NaiveDate,
    last_as_of_date: NaiveDate,
    family: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct FormalDatasetFamilySummary {
    family: String,
    row_count: usize,
    scenario_count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct FormalDatasetQualitySummary {
    grade: String,
    row_count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct FormalDatasetSummaryEnvelope {
    generated_at: String,
    dataset_key: String,
    dataset: FormalDatasetRecord,
    split_summaries: Vec<FormalDatasetSplitSummary>,
    scenario_summaries: Vec<FormalDatasetScenarioSummary>,
    family_summaries: Vec<FormalDatasetFamilySummary>,
    quality_summaries: Vec<FormalDatasetQualitySummary>,
    recommendation: String,
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
        activate_release_with_runtime_guard(
            &store,
            &record.manifest.market_scope,
            &record.manifest.release_id,
            options.reload_api,
            &options.api_reload_url,
            options.skip_operational_guard,
            &options.updated_by,
        )
        .await?;
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
    activate_release_with_runtime_guard(
        &store,
        &market_scope,
        &options.release_id,
        options.reload_api,
        &options.api_reload_url,
        options.skip_operational_guard,
        &options.updated_by,
    )
    .await?;
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

async fn research_release_review(args: &[String]) -> anyhow::Result<()> {
    let options = ReleaseReviewOptions::parse(args)?;
    let store = open_sqlite_store().await?;
    store.migrate().await?;

    let candidate_release = store
        .load_model_release(&options.candidate_release_id)
        .await?
        .with_context(|| {
            format!(
                "candidate release {} not found",
                options.candidate_release_id
            )
        })?;
    let market_scope = options
        .market_scope
        .clone()
        .unwrap_or_else(|| candidate_release.manifest.market_scope.clone());
    if candidate_release.manifest.market_scope != market_scope {
        bail!(
            "candidate release {} belongs to {}, not {}",
            candidate_release.manifest.release_id,
            candidate_release.manifest.market_scope,
            market_scope
        );
    }

    let original_active = store
        .load_active_model_release(&market_scope)
        .await?
        .with_context(|| format!("no active release found for market scope {market_scope}"))?;
    let baseline_release = if let Some(baseline_release_id) = options.baseline_release_id.as_deref()
    {
        let release = store
            .load_model_release(baseline_release_id)
            .await?
            .with_context(|| format!("baseline release {baseline_release_id} not found"))?;
        if release.manifest.market_scope != market_scope {
            bail!(
                "baseline release {} belongs to {}, not {}",
                release.manifest.release_id,
                release.manifest.market_scope,
                market_scope
            );
        }
        release
    } else {
        original_active.clone()
    };

    if baseline_release.manifest.release_id == candidate_release.manifest.release_id {
        bail!("baseline release and candidate release must be different");
    }

    let mut original_records = BTreeMap::<String, ModelReleaseRecord>::new();
    for release in [
        original_active.clone(),
        baseline_release.clone(),
        candidate_release.clone(),
    ] {
        original_records.insert(release.manifest.release_id.clone(), release);
    }

    let review_result = run_release_review(
        &store,
        &market_scope,
        &options,
        &original_active,
        &baseline_release,
        &candidate_release,
    )
    .await;
    let restore_result = restore_release_review_state(
        &store,
        &market_scope,
        &original_active.manifest.release_id,
        &original_records,
        &options.api_reload_url,
        &options.updated_by,
    )
    .await;

    if let Err(restore_error) = restore_result {
        if let Err(review_error) = review_result {
            bail!(
                "release review failed and restore also failed:\nreview: {review_error:#}\nrestore: {restore_error:#}"
            );
        }
        bail!("release review completed but restore failed: {restore_error:#}");
    }

    review_result?;
    println!(
        "Release review restored original active release {}.",
        original_active.manifest.release_id
    );
    Ok(())
}

async fn run_release_review(
    store: &SqliteStore,
    market_scope: &str,
    options: &ReleaseReviewOptions,
    original_active: &ModelReleaseRecord,
    baseline_release: &ModelReleaseRecord,
    candidate_release: &ModelReleaseRecord,
) -> anyhow::Result<()> {
    println!(
        "Review baseline={} candidate={} market_scope={market_scope}.",
        baseline_release.manifest.release_id, candidate_release.manifest.release_id
    );

    activate_release_for_review(
        store,
        market_scope,
        &baseline_release.manifest.release_id,
        &options.api_reload_url,
        &options.updated_by,
        "baseline",
    )
    .await?;
    let baseline_assessment = fetch_assessment_snapshot_for_guard(&options.api_reload_url).await?;

    activate_release_for_review(
        store,
        market_scope,
        &candidate_release.manifest.release_id,
        &options.api_reload_url,
        &options.updated_by,
        "candidate",
    )
    .await?;
    let candidate_assessment = fetch_assessment_snapshot_for_guard(&options.api_reload_url).await?;

    let baseline_actionability_review = build_release_actionability_review(baseline_release)?;
    let candidate_actionability_review = build_release_actionability_review(candidate_release)?;
    let candidate_has_actionability = candidate_actionability_review.enabled;
    let operational_regressions =
        compare_operational_guardrails(&baseline_assessment, &candidate_assessment);
    let actionability_regressions =
        compare_actionability_guardrails(&candidate_actionability_review);
    let mut overall_regressions = operational_regressions.clone();
    overall_regressions.extend(actionability_regressions.iter().cloned());
    let report = ReleaseReviewEnvelope {
        reviewed_at: Utc::now().to_rfc3339(),
        market_scope: market_scope.to_string(),
        api_reload_url: options.api_reload_url.clone(),
        original_active_release_id: original_active.manifest.release_id.clone(),
        restored_release_id: original_active.manifest.release_id.clone(),
        baseline_release: baseline_release.clone(),
        candidate_release: candidate_release.clone(),
        comparison: build_release_review_comparison(&baseline_assessment, &candidate_assessment),
        baseline_assessment,
        candidate_assessment,
        baseline_actionability_review,
        candidate_actionability_review,
        operational_guard_passed: operational_regressions.is_empty(),
        actionability_guard_passed: actionability_regressions.is_empty(),
        overall_guard_passed: overall_regressions.is_empty(),
        recommendation: build_release_review_recommendation(
            &overall_regressions,
            candidate_has_actionability,
        ),
        operational_guard_regressions: operational_regressions,
        actionability_guard_regressions: actionability_regressions,
        overall_guard_regressions: overall_regressions,
    };
    write_release_review_report(&options.output_dir, &report)?;

    println!(
        "Release review complete: guard_passed={} baseline={} candidate={}.",
        report.overall_guard_passed,
        report.baseline_release.manifest.release_id,
        report.candidate_release.manifest.release_id
    );
    print_release_review_summary(&report);

    Ok(())
}

async fn activate_release_for_review(
    store: &SqliteStore,
    market_scope: &str,
    release_id: &str,
    api_reload_url: &str,
    updated_by: &str,
    stage: &str,
) -> anyhow::Result<()> {
    store
        .activate_model_release(market_scope, release_id, updated_by)
        .await?;
    println!("Review step {stage}: activated {release_id}.");
    println!("Review step {stage}: reloading API runtime via {api_reload_url}.");
    reload_api_runtime(api_reload_url).await?;
    println!("Review step {stage}: runtime ready.");
    Ok(())
}

async fn restore_release_review_state(
    store: &SqliteStore,
    market_scope: &str,
    original_active_release_id: &str,
    original_records: &BTreeMap<String, ModelReleaseRecord>,
    api_reload_url: &str,
    updated_by: &str,
) -> anyhow::Result<()> {
    store
        .activate_model_release(market_scope, original_active_release_id, updated_by)
        .await?;
    reload_api_runtime(api_reload_url).await?;
    for record in original_records.values() {
        store.upsert_model_release(record).await?;
    }
    Ok(())
}

async fn activate_release_with_runtime_guard(
    store: &SqliteStore,
    market_scope: &str,
    release_id: &str,
    reload_api: bool,
    api_reload_url: &str,
    skip_operational_guard: bool,
    updated_by: &str,
) -> anyhow::Result<ModelReleaseRecord> {
    let previous_active = store.load_active_model_release(market_scope).await?;
    let previous_release_id = previous_active
        .as_ref()
        .map(|release| release.manifest.release_id.clone());
    let should_check_guard =
        reload_api && !skip_operational_guard && previous_release_id.as_deref() != Some(release_id);
    let baseline_assessment = if should_check_guard {
        Some(fetch_assessment_snapshot_for_guard(api_reload_url).await?)
    } else {
        None
    };

    let activated = store
        .activate_model_release(market_scope, release_id, updated_by)
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

    if reload_api {
        println!(
            "Reloading API runtime via {api_reload_url}. First load for a new release may take several minutes while history snapshots are materialized."
        );
        reload_api_runtime(api_reload_url).await?;
        println!("Reloaded API runtime via {api_reload_url}.");
    }

    if let Some(baseline_assessment) = baseline_assessment {
        let candidate_assessment = fetch_assessment_snapshot_for_guard(api_reload_url).await?;
        let regressions =
            compare_operational_guardrails(&baseline_assessment, &candidate_assessment);
        if regressions.is_empty() {
            print_operational_guardrail_summary(&baseline_assessment, &candidate_assessment);
            return Ok(activated);
        }

        if let Some(previous_release_id) = previous_release_id
            .as_deref()
            .filter(|previous_release_id| *previous_release_id != release_id)
        {
            println!(
                "Operational guard failed after activating {release_id}. Rolling back to {previous_release_id}."
            );
            let rolled_back = store
                .rollback_model_release(market_scope, previous_release_id, updated_by)
                .await?;
            if reload_api {
                println!(
                    "Reloading API runtime after rollback via {api_reload_url}. This may also take several minutes."
                );
                reload_api_runtime(api_reload_url).await?;
                println!("Reloaded API runtime after rollback.");
            }
            bail!(
                "release {} regressed against baseline release {} and was rolled back to {}:\n  - {}",
                release_id,
                baseline_assessment
                    .method
                    .release_id
                    .as_deref()
                    .unwrap_or("unknown"),
                rolled_back.manifest.release_id,
                regressions.join("\n  - ")
            );
        }

        bail!(
            "release {} regressed against baseline but no previous active release was available for automatic rollback:\n  - {}",
            release_id,
            regressions.join("\n  - ")
        );
    }

    if !reload_api && !skip_operational_guard {
        println!(
            "Operational guard skipped because --reload-api was not enabled; use --reload-api to compare the new runtime against the current baseline."
        );
    } else if skip_operational_guard {
        println!("Operational guard explicitly skipped.");
    }

    Ok(activated)
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
    write_dataset_export(
        &dataset,
        &transitional_feature_names(),
        options.format,
        options.output_path.as_deref(),
    )?;
    Ok(())
}

async fn research_pipeline_train_probability(args: &[String]) -> anyhow::Result<()> {
    let options = PipelineTrainOptions::parse(args)?;
    let store = open_sqlite_store().await?;
    store.migrate().await?;
    let artifacts = train_probability_pipeline(&store, &options).await?;
    println!("Formal probability bundle generated.");
    println!("  dataset_source   {}", artifacts.dataset_source);
    println!("  dataset_label    {}", artifacts.dataset_label);
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
    if let Some(actionability) = artifacts.bundle.actionability.as_ref() {
        for level in &actionability.levels {
            if let Some(summary) = level.evaluation.actionability.as_ref() {
                println!(
                    "  actionability    {:>7} scenarios={} advance_warn={} late_confirm={} missed={}",
                    actionability_level_text(level.level),
                    summary.scenario_count,
                    format_pct(summary.advance_warning_rate.unwrap_or(0.0)),
                    format_pct(summary.late_confirmation_rate.unwrap_or(0.0)),
                    format_pct(summary.missed_rate.unwrap_or(0.0)),
                );
            }
        }
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
        activate_release_with_runtime_guard(
            &store,
            &artifacts.release.manifest.market_scope,
            &artifacts.release.manifest.release_id,
            options.reload_api,
            &options.api_reload_url,
            options.skip_operational_guard,
            &options.updated_by,
        )
        .await?;
    }

    Ok(())
}

async fn research_feature_snapshot_build(args: &[String]) -> anyhow::Result<()> {
    let options = FeatureSnapshotBuildOptions::parse(args)?;
    let store = open_sqlite_store().await?;
    store.migrate().await?;
    let (indicators, observations) = load_formal_feature_inputs(&store, options.to).await?;
    let snapshot_build =
        build_or_load_feature_snapshots(&store, &indicators, &observations, &options).await?;
    let snapshots = snapshot_build.snapshots;
    if snapshots.is_empty() {
        bail!("no feature snapshots were generated for the requested range");
    }
    let ready_count = snapshots
        .iter()
        .filter(|snapshot| snapshot.visibility_status == FEATURE_SNAPSHOT_STATUS_READY)
        .count();
    let blocked_count = snapshots.len().saturating_sub(ready_count);
    store.upsert_feature_snapshots(&snapshots).await?;
    let first_date = snapshots.first().map(|snapshot| snapshot.as_of_date);
    let last_date = snapshots.last().map(|snapshot| snapshot.as_of_date);
    println!(
        "Built {} feature snapshots for {} ({} -> {}, feature_set_version={}, ready={}, blocked={}).",
        snapshots.len(),
        options.market_scope,
        first_date
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string()),
        last_date
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string()),
        options.feature_set_version,
        ready_count,
        blocked_count
    );
    println!(
        "  reused={} recomputed={} pit={} force_rebuild={}",
        snapshot_build.reused_count,
        snapshot_build.recomputed_count,
        options.point_in_time_mode,
        options.force_rebuild
    );
    Ok(())
}

async fn research_feature_snapshot_list(args: &[String]) -> anyhow::Result<()> {
    let options = FeatureSnapshotListOptions::parse(args)?;
    let store = open_sqlite_store().await?;
    store.migrate().await?;
    let snapshots = store
        .list_feature_snapshots(
            options.market_scope.as_deref(),
            options.feature_set_version.as_deref(),
            options.from,
            options.to,
            options.limit,
        )
        .await?;
    if snapshots.is_empty() {
        println!("No feature snapshots found.");
        return Ok(());
    }

    for snapshot in snapshots {
        println!(
            "[{}] {} {} pit={} status={} coverage={:.3} core={:.3} trigger={:.3} external={:.3} features={} latest_visible_at={}",
            snapshot.as_of_date,
            snapshot.market_scope,
            snapshot.feature_set_version,
            snapshot.point_in_time_mode,
            snapshot.visibility_status,
            snapshot.coverage_score,
            snapshot.core_feature_coverage,
            snapshot.trigger_feature_coverage,
            snapshot.external_feature_coverage,
            snapshot.feature_count,
            snapshot
                .latest_visible_at
                .map(|value| value.to_rfc3339())
                .unwrap_or_else(|| "-".to_string())
        );
    }
    Ok(())
}

async fn research_formal_dataset_build_main(args: &[String]) -> anyhow::Result<()> {
    let options = FormalDatasetBuildOptions::parse(args)?;
    let store = open_sqlite_store().await?;
    store.migrate().await?;
    let (indicators, observations) = load_formal_feature_inputs(&store, options.feature.to).await?;
    let snapshot_build =
        build_or_load_feature_snapshots(&store, &indicators, &observations, &options.feature)
            .await?;
    let snapshots = snapshot_build.snapshots;
    if snapshots.is_empty() {
        bail!("no feature snapshots were generated for the requested range");
    }
    store.upsert_feature_snapshots(&snapshots).await?;

    let generated_at = Utc::now();
    let dataset_version = options
        .dataset_version
        .clone()
        .unwrap_or_else(|| format!("{}", generated_at.format("%Y%m%dT%H%M%S")));
    let dataset_key = formal_dataset_key(&options.dataset_id, &dataset_version);
    let rows = build_main_formal_dataset_rows_with_catalog(
        &dataset_key,
        &snapshots,
        &options.feature.point_in_time_mode,
        &options.label_version,
        &options.scenario_set_version,
    )?;
    if rows.is_empty() {
        let ready_count = snapshots
            .iter()
            .filter(|snapshot| snapshot.visibility_status == FEATURE_SNAPSHOT_STATUS_READY)
            .count();
        bail!(
            "no formal dataset rows passed the minimum coverage / visibility thresholds (pit_mode={}, ready_snapshots={}, total_snapshots={})",
            options.feature.point_in_time_mode,
            ready_count,
            snapshots.len()
        );
    }

    let train_count = rows.iter().filter(|row| row.split_name == "train").count();
    let calibration_count = rows
        .iter()
        .filter(|row| row.split_name == "calibration")
        .count();
    let evaluation_count = rows
        .iter()
        .filter(|row| row.split_name == "evaluation")
        .count();
    if train_count == 0 || calibration_count == 0 || evaluation_count == 0 {
        bail!(
            "formal dataset range is too short to produce train/calibration/evaluation splits (rows={}, train={}, calibration={}, evaluation={}); expand the date range before persisting this dataset",
            rows.len(),
            train_count,
            calibration_count,
            evaluation_count
        );
    }

    let dataset = FormalDatasetRecord {
        manifest: FormalDatasetManifest {
            dataset_id: options.dataset_id.clone(),
            dataset_version: dataset_version.clone(),
            market_scope: options.feature.market_scope.clone(),
            feature_set_version: options.feature.feature_set_version.clone(),
            label_version: options.label_version.clone(),
            scenario_set_version: options.scenario_set_version.clone(),
            point_in_time_mode: options.feature.point_in_time_mode.clone(),
            from_date: rows.first().map(|row| row.as_of_date),
            to_date: rows.last().map(|row| row.as_of_date),
            train_end_date: rows
                .iter()
                .rev()
                .find(|row| row.split_name == "train")
                .map(|row| row.as_of_date),
            calibration_end_date: rows
                .iter()
                .rev()
                .find(|row| row.split_name == "calibration")
                .map(|row| row.as_of_date),
            evaluation_start_date: rows
                .iter()
                .find(|row| row.split_name == "evaluation")
                .map(|row| row.as_of_date),
            row_count: rows.len(),
            note: "Built from raw observations and point-in-time feature snapshots; persists both forward crisis labels and action-window labels so formal training can optimize for earlier executable warnings without losing the original crisis-start reference.".to_string(),
        },
        created_at: generated_at,
    };
    store.upsert_formal_dataset(&dataset).await?;
    store
        .replace_formal_dataset_rows(&dataset_key, &rows)
        .await?;

    println!("Built formal dataset {dataset_key}.");
    println!(
        "  rows={} train={} calibration={} evaluation={}",
        rows.len(),
        train_count,
        calibration_count,
        evaluation_count
    );
    println!(
        "  range={} -> {} feature_set_version={} point_in_time_mode={}",
        dataset
            .manifest
            .from_date
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string()),
        dataset
            .manifest
            .to_date
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string()),
        dataset.manifest.feature_set_version,
        dataset.manifest.point_in_time_mode
    );
    println!(
        "  snapshots reused={} recomputed={}",
        snapshot_build.reused_count, snapshot_build.recomputed_count
    );
    Ok(())
}

async fn research_formal_dataset_list_main(args: &[String]) -> anyhow::Result<()> {
    let options = FormalDatasetListOptions::parse(args)?;
    let store = open_sqlite_store().await?;
    store.migrate().await?;
    let datasets = store
        .list_formal_datasets(
            options.market_scope.as_deref(),
            options.dataset_id.as_deref(),
            options.limit,
        )
        .await?;
    if datasets.is_empty() {
        println!("No formal datasets found.");
        return Ok(());
    }

    for dataset in datasets {
        let dataset_key = formal_dataset_key(
            &dataset.manifest.dataset_id,
            &dataset.manifest.dataset_version,
        );
        println!(
            "[{}] {} rows={} feature_set={} label={} pit={} range={} -> {}",
            dataset_key,
            dataset.manifest.market_scope,
            dataset.manifest.row_count,
            dataset.manifest.feature_set_version,
            dataset.manifest.label_version,
            dataset.manifest.point_in_time_mode,
            dataset
                .manifest
                .from_date
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
            dataset
                .manifest
                .to_date
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string())
        );
    }
    Ok(())
}

async fn research_formal_dataset_summarize_main(args: &[String]) -> anyhow::Result<()> {
    let options = FormalDatasetSummaryOptions::parse(args)?;
    let store = open_sqlite_store().await?;
    store.migrate().await?;
    let dataset_key = resolve_formal_dataset_key(
        &store,
        options.dataset_key.as_deref(),
        &options.dataset_id,
        options.dataset_version.as_deref(),
        options.market_scope.as_deref(),
    )
    .await?;
    let dataset = store
        .load_formal_dataset(&dataset_key)
        .await?
        .with_context(|| format!("formal dataset {dataset_key} was not found in SQLite"))?;
    let rows = store
        .list_formal_dataset_rows(&dataset_key, None, None)
        .await?;
    if rows.is_empty() {
        bail!("formal dataset {dataset_key} has no persisted rows");
    }
    let summary = build_formal_dataset_summary(&dataset_key, dataset, &rows)?;
    write_formal_dataset_summary_report(&options.output_dir, &summary)?;
    print_formal_dataset_summary(&summary);
    Ok(())
}

async fn load_formal_feature_inputs(
    store: &SqliteStore,
    to: Option<NaiveDate>,
) -> anyhow::Result<(Vec<Indicator>, Vec<Observation>)> {
    let indicators = store.load_indicators().await?;
    let upper_bound = to.unwrap_or_else(|| Utc::now().date_naive());
    let observations = store
        .load_observations_for_entities(&["us", "jp"], upper_bound)
        .await?;
    if observations.is_empty() {
        bail!("no observations found in SQLite; run bootstrap/backfill first");
    }
    Ok((indicators, observations))
}

#[derive(Debug, Clone)]
struct FeatureSnapshotBuildResult {
    snapshots: Vec<FeatureSnapshotRecord>,
    reused_count: usize,
    recomputed_count: usize,
}

async fn build_or_load_feature_snapshots(
    store: &SqliteStore,
    indicators: &[Indicator],
    observations: &[Observation],
    options: &FeatureSnapshotBuildOptions,
) -> anyhow::Result<FeatureSnapshotBuildResult> {
    let target_dates = formal_feature_dates(observations, options.from, options.to);
    if target_dates.is_empty() {
        return Ok(FeatureSnapshotBuildResult {
            snapshots: Vec::new(),
            reused_count: 0,
            recomputed_count: 0,
        });
    }

    let reusable = if options.force_rebuild {
        BTreeMap::new()
    } else {
        load_reusable_feature_snapshots(store, options).await?
    };

    let missing_dates = target_dates
        .iter()
        .copied()
        .filter(|date| !reusable.contains_key(date))
        .collect::<Vec<_>>();
    let recomputed = build_formal_feature_snapshots_for_dates(
        indicators,
        observations,
        options,
        &missing_dates,
    )?;

    let mut combined = reusable.into_values().chain(recomputed).collect::<Vec<_>>();
    combined.sort_by_key(|snapshot| snapshot.as_of_date);

    Ok(FeatureSnapshotBuildResult {
        reused_count: combined.len().saturating_sub(missing_dates.len()),
        recomputed_count: missing_dates.len(),
        snapshots: combined,
    })
}

async fn load_reusable_feature_snapshots(
    store: &SqliteStore,
    options: &FeatureSnapshotBuildOptions,
) -> anyhow::Result<BTreeMap<NaiveDate, FeatureSnapshotRecord>> {
    let rows = store
        .list_feature_snapshots_for_mode(
            &options.market_scope,
            &options.feature_set_version,
            &options.point_in_time_mode,
            options.from,
            options.to,
        )
        .await?;
    let reusable = rows
        .into_iter()
        .filter(feature_snapshot_status_is_current)
        .fold(BTreeMap::new(), |mut acc, snapshot| {
            acc.entry(snapshot.as_of_date).or_insert(snapshot);
            acc
        });
    Ok(reusable)
}

fn feature_snapshot_status_is_current(snapshot: &FeatureSnapshotRecord) -> bool {
    matches!(
        snapshot.visibility_status.as_str(),
        FEATURE_SNAPSHOT_STATUS_READY | FEATURE_SNAPSHOT_STATUS_COVERAGE_OR_VISIBILITY_FAILED
    )
}

fn build_formal_feature_snapshots_for_dates(
    indicators: &[Indicator],
    observations: &[Observation],
    options: &FeatureSnapshotBuildOptions,
    dates: &[NaiveDate],
) -> anyhow::Result<Vec<FeatureSnapshotRecord>> {
    let scoring = ScoringEngine::default();
    let mut snapshots = Vec::with_capacity(dates.len());
    for as_of_date in dates.iter().copied() {
        snapshots.push(build_formal_feature_snapshot_for_date(
            indicators,
            observations,
            &scoring,
            as_of_date,
            options,
        )?);
    }
    Ok(snapshots)
}

fn build_formal_feature_snapshot_for_date(
    indicators: &[Indicator],
    observations: &[Observation],
    scoring: &ScoringEngine,
    as_of_date: NaiveDate,
    options: &FeatureSnapshotBuildOptions,
) -> anyhow::Result<FeatureSnapshotRecord> {
    let point_in_time_mode = PointInTimeMode::parse(&options.point_in_time_mode)?;
    let output = scoring.score_with_observation_filter(
        indicators,
        observations,
        as_of_date,
        "us",
        &options.market_scope,
        |observation| observation_is_visible_for_date(observation, as_of_date, point_in_time_mode),
    );
    let mut features = BTreeMap::new();
    let mut visible_candidates = Vec::new();

    let vix_history = observations_for_indicator(
        observations,
        "us_market_vix_close",
        as_of_date,
        point_in_time_mode,
    );
    insert_latest_feature(
        &mut features,
        &mut visible_candidates,
        "us_vix_level",
        &vix_history,
        point_in_time_mode,
    );
    insert_derived_feature(
        &mut features,
        "us_vix_change_5d",
        difference_from_tail(&vix_history, 5),
    );

    let curve_history = observations_for_indicator(
        observations,
        "us_rates_yield_curve_10y2y",
        as_of_date,
        point_in_time_mode,
    );
    insert_latest_feature(
        &mut features,
        &mut visible_candidates,
        "us_curve_10y2y_level",
        &curve_history,
        point_in_time_mode,
    );

    let baa_history = observations_for_indicator(
        observations,
        "us_credit_baa_10y_spread",
        as_of_date,
        point_in_time_mode,
    );
    insert_latest_feature(
        &mut features,
        &mut visible_candidates,
        "us_baa_10y_spread_level",
        &baa_history,
        point_in_time_mode,
    );

    let effr_history = observations_for_indicator(
        observations,
        "us_liquidity_effr",
        as_of_date,
        point_in_time_mode,
    );
    insert_latest_feature(
        &mut features,
        &mut visible_candidates,
        "us_fed_funds_level",
        &effr_history,
        point_in_time_mode,
    );

    let nfci_history = observations_for_indicator(
        observations,
        "us_liquidity_national_financial_conditions",
        as_of_date,
        point_in_time_mode,
    );
    insert_latest_feature(
        &mut features,
        &mut visible_candidates,
        "us_nfci_level",
        &nfci_history,
        point_in_time_mode,
    );

    let stlfsi_history = observations_for_indicator(
        observations,
        "us_liquidity_financial_stress_stl",
        as_of_date,
        point_in_time_mode,
    );
    insert_latest_feature(
        &mut features,
        &mut visible_candidates,
        "us_stlfsi_level",
        &stlfsi_history,
        point_in_time_mode,
    );

    let unemployment_history = observations_for_indicator(
        observations,
        "us_macro_unemployment_rate",
        as_of_date,
        point_in_time_mode,
    );
    insert_latest_feature(
        &mut features,
        &mut visible_candidates,
        "us_unemployment_level",
        &unemployment_history,
        point_in_time_mode,
    );

    let housing_history = observations_for_indicator(
        observations,
        "us_real_estate_housing_starts",
        as_of_date,
        point_in_time_mode,
    );
    insert_latest_feature(
        &mut features,
        &mut visible_candidates,
        "us_housing_starts_level",
        &housing_history,
        point_in_time_mode,
    );

    let usdjpy_history = observations_for_indicator(
        observations,
        "us_external_usdjpy_level",
        as_of_date,
        point_in_time_mode,
    );
    insert_latest_feature(
        &mut features,
        &mut visible_candidates,
        "us_usdjpy_level",
        &usdjpy_history,
        point_in_time_mode,
    );
    insert_derived_feature(
        &mut features,
        "us_usdjpy_change_20d",
        difference_from_tail(&usdjpy_history, 20),
    );

    features.insert(
        "overall_score".to_string(),
        round6((output.snapshot.overall_score / 100.0).clamp(0.0, 1.0)),
    );
    features.insert(
        "structural_score".to_string(),
        round6((output.snapshot.structural_score / 100.0).clamp(0.0, 1.0)),
    );
    features.insert(
        "trigger_score".to_string(),
        round6((output.snapshot.trigger_score / 100.0).clamp(0.0, 1.0)),
    );
    features.insert(
        "external_dimension_score".to_string(),
        round6(
            (find_dimension_score(&output.indicator_risks, RiskDimension::ExternalSector) / 100.0)
                .clamp(0.0, 1.0),
        ),
    );

    let (
        core_feature_coverage,
        trigger_feature_coverage,
        external_feature_coverage,
        coverage_score,
    ) = coverage_summary(&output.indicator_risks);
    let latest_visible_at = visible_candidates.into_iter().max();
    let visibility_status =
        feature_snapshot_visibility_status(&features, coverage_score, latest_visible_at);

    Ok(FeatureSnapshotRecord {
        as_of_date,
        entity_id: "us".to_string(),
        market_scope: options.market_scope.clone(),
        feature_set_version: options.feature_set_version.clone(),
        point_in_time_mode: options.point_in_time_mode.clone(),
        visibility_status: visibility_status.to_string(),
        latest_visible_at,
        coverage_score,
        core_feature_coverage,
        trigger_feature_coverage,
        external_feature_coverage,
        feature_count: features.len(),
        features,
        created_at: Utc::now(),
    })
}

fn build_main_formal_dataset_rows_with_catalog(
    dataset_key: &str,
    snapshots: &[FeatureSnapshotRecord],
    point_in_time_mode: &str,
    label_version: &str,
    scenario_set_version: &str,
) -> anyhow::Result<Vec<FormalDatasetRowRecord>> {
    let scenarios = load_label_set_crisis_scenarios(scenario_set_version, label_version)?;
    let min_date = NaiveDate::from_ymd_opt(1990, 1, 2).expect("valid date");
    let mut rows = snapshots
        .iter()
        .filter(|snapshot| snapshot.as_of_date >= min_date)
        .filter(|snapshot| snapshot.visibility_status == FEATURE_SNAPSHOT_STATUS_READY)
        .filter(|snapshot| snapshot.coverage_score >= 0.85)
        .filter(|snapshot| snapshot.core_feature_coverage >= 0.90)
        .filter(|snapshot| snapshot.trigger_feature_coverage >= 0.75)
        .filter(|snapshot| snapshot.external_feature_coverage >= 0.70)
        .filter(|snapshot| has_main_dataset_core_features(&snapshot.features))
        .map(|snapshot| {
            let primary_scenario = primary_scenario_for_date(snapshot.as_of_date, &scenarios);
            FormalDatasetRowRecord {
                dataset_key: dataset_key.to_string(),
                split_name: String::new(),
                entity_id: snapshot.entity_id.clone(),
                market_scope: snapshot.market_scope.clone(),
                as_of_date: snapshot.as_of_date,
                point_in_time_mode: point_in_time_mode.to_string(),
                latest_visible_at: snapshot.latest_visible_at,
                coverage_score: snapshot.coverage_score,
                core_feature_coverage: snapshot.core_feature_coverage,
                trigger_feature_coverage: snapshot.trigger_feature_coverage,
                external_feature_coverage: snapshot.external_feature_coverage,
                sample_quality_grade: feature_quality_grade(snapshot.coverage_score).to_string(),
                primary_scenario_id: primary_scenario
                    .as_ref()
                    .map(|scenario| scenario.scenario_id.clone()),
                scenario_family: primary_scenario.map(|scenario| scenario.family),
                label_5d: forward_crisis_label(snapshot.as_of_date, &scenarios, 5),
                label_20d: forward_crisis_label(snapshot.as_of_date, &scenarios, 20),
                label_60d: forward_crisis_label(snapshot.as_of_date, &scenarios, 60),
                action_label_5d: action_window_label(snapshot.as_of_date, &scenarios, 5),
                action_label_20d: action_window_label(snapshot.as_of_date, &scenarios, 20),
                action_label_60d: action_window_label(snapshot.as_of_date, &scenarios, 60),
                features: snapshot.features.clone(),
                created_at: Utc::now(),
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| row.as_of_date);
    assign_formal_dataset_splits(&mut rows, &scenarios);
    Ok(rows)
}

fn formal_feature_dates(
    observations: &[Observation],
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
) -> Vec<NaiveDate> {
    let mut dates = observations
        .iter()
        .filter(|observation| {
            matches!(observation.frequency, Frequency::Daily | Frequency::Event)
                && (observation.entity_id == "us"
                    || matches!(
                        observation.indicator_id.as_str(),
                        "us_external_usdjpy_level" | "jp_rates_call_rate"
                    ))
        })
        .map(|observation| observation.as_of_date)
        .collect::<BTreeSet<_>>();
    if dates.is_empty() {
        dates.extend(
            observations
                .iter()
                .filter(|observation| observation.entity_id == "us")
                .map(|observation| observation.as_of_date),
        );
    }
    let mut dates = dates.into_iter().collect::<Vec<_>>();
    if let Some(from) = from {
        dates.retain(|date| *date >= from);
    }
    if let Some(to) = to {
        dates.retain(|date| *date <= to);
    }
    dates.sort();
    dates
}

fn observations_for_indicator<'a>(
    observations: &'a [Observation],
    indicator_id: &str,
    as_of_date: NaiveDate,
    point_in_time_mode: PointInTimeMode,
) -> Vec<&'a Observation> {
    let mut rows = observations
        .iter()
        .filter(|observation| observation.indicator_id == indicator_id)
        .filter(|observation| observation.as_of_date <= as_of_date)
        .filter(|observation| {
            observation_is_visible_for_date(observation, as_of_date, point_in_time_mode)
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|observation| observation.as_of_date);
    rows
}

fn insert_latest_feature(
    features: &mut BTreeMap<String, f64>,
    visible_candidates: &mut Vec<chrono::DateTime<Utc>>,
    feature_name: &str,
    history: &[&Observation],
    point_in_time_mode: PointInTimeMode,
) {
    if let Some(latest) = history.last() {
        features.insert(feature_name.to_string(), round6(latest.value));
        if let Some(visible_at) = observation_visible_at_for_mode(latest, point_in_time_mode) {
            visible_candidates.push(visible_at);
        }
    }
}

fn insert_derived_feature(
    features: &mut BTreeMap<String, f64>,
    feature_name: &str,
    value: Option<f64>,
) {
    if let Some(value) = value {
        features.insert(feature_name.to_string(), round6(value));
    }
}

fn difference_from_tail(observations: &[&Observation], lookback: usize) -> Option<f64> {
    let latest = observations.last()?;
    let previous_index = observations.len().checked_sub(lookback + 1)?;
    let previous = observations.get(previous_index)?;
    Some(latest.value - previous.value)
}

fn feature_snapshot_visibility_status(
    features: &BTreeMap<String, f64>,
    coverage_score: f64,
    latest_visible_at: Option<DateTime<Utc>>,
) -> &'static str {
    if latest_visible_at.is_none()
        || coverage_score < 0.70
        || !has_main_dataset_core_features(features)
    {
        FEATURE_SNAPSHOT_STATUS_COVERAGE_OR_VISIBILITY_FAILED
    } else {
        FEATURE_SNAPSHOT_STATUS_READY
    }
}

fn observation_is_visible_for_date(
    observation: &Observation,
    as_of_date: NaiveDate,
    point_in_time_mode: PointInTimeMode,
) -> bool {
    observation_visible_at_for_mode(observation, point_in_time_mode)
        .map(|visible_at| visible_at <= assessment_cutoff_utc(as_of_date))
        .unwrap_or(false)
}

fn observation_visible_at_for_mode(
    observation: &Observation,
    point_in_time_mode: PointInTimeMode,
) -> Option<DateTime<Utc>> {
    match point_in_time_mode {
        PointInTimeMode::BestEffort => best_effort_visible_at(observation),
        PointInTimeMode::Strict => strict_visible_at(observation),
    }
}

fn best_effort_visible_at(observation: &Observation) -> Option<DateTime<Utc>> {
    let anchor_date = observation.period_end.unwrap_or(observation.as_of_date);
    match observation.source_id.as_str() {
        "treasury" => Some(new_york_time_to_utc(anchor_date, 18, 0)),
        "world_bank" => anchor_date
            .checked_add_signed(Duration::days(270))
            .map(|date| new_york_time_to_utc(date, 17, 30)),
        "boj" => Some(tokyo_time_to_utc(anchor_date, 17, 0)),
        "sec_edgar" => Some(
            observation
                .publication_time
                .unwrap_or_else(|| new_york_time_to_utc(anchor_date, 18, 0)),
        ),
        "gdelt" => None,
        "mock" => Some(
            observation
                .publication_time
                .unwrap_or_else(|| new_york_time_to_utc(anchor_date, 17, 30)),
        ),
        _ => anchor_date
            .checked_add_signed(Duration::days(default_visibility_lag_days(
                observation.frequency,
            )))
            .map(|date| new_york_time_to_utc(date, 17, 30)),
    }
}

fn strict_visible_at(observation: &Observation) -> Option<DateTime<Utc>> {
    match observation.source_id.as_str() {
        "sec_edgar" | "mock" => observation.publication_time,
        _ => None,
    }
}

fn default_visibility_lag_days(frequency: Frequency) -> i64 {
    match frequency {
        Frequency::Daily | Frequency::Event => 0,
        Frequency::Weekly => 3,
        Frequency::Monthly => 15,
        Frequency::Quarterly => 45,
        Frequency::Annual => 270,
    }
}

fn assessment_cutoff_utc(as_of_date: NaiveDate) -> DateTime<Utc> {
    new_york_time_to_utc(as_of_date, 17, 30)
}

fn new_york_time_to_utc(date: NaiveDate, hour: u32, minute: u32) -> DateTime<Utc> {
    let utc_offset_hours = if is_new_york_dst(date) { 4 } else { 5 };
    let local = date
        .and_hms_opt(hour, minute, 0)
        .expect("local wall-clock timestamp must be valid");
    DateTime::<Utc>::from_naive_utc_and_offset(local + Duration::hours(utc_offset_hours), Utc)
}

fn tokyo_time_to_utc(date: NaiveDate, hour: u32, minute: u32) -> DateTime<Utc> {
    let local = date
        .and_hms_opt(hour, minute, 0)
        .expect("tokyo wall-clock timestamp must be valid");
    DateTime::<Utc>::from_naive_utc_and_offset(local - Duration::hours(9), Utc)
}

fn is_new_york_dst(date: NaiveDate) -> bool {
    let year = date.year();
    let (start, end) = if year >= 2007 {
        (
            nth_weekday_of_month(year, 3, Weekday::Sun, 2),
            nth_weekday_of_month(year, 11, Weekday::Sun, 1),
        )
    } else {
        (
            nth_weekday_of_month(year, 4, Weekday::Sun, 1),
            last_weekday_of_month(year, 10, Weekday::Sun),
        )
    };
    date >= start && date < end
}

fn nth_weekday_of_month(year: i32, month: u32, weekday: Weekday, nth: u32) -> NaiveDate {
    let first_day = NaiveDate::from_ymd_opt(year, month, 1).expect("valid calendar date");
    let first_weekday_offset = (7 + weekday.num_days_from_monday() as i64
        - first_day.weekday().num_days_from_monday() as i64)
        % 7;
    first_day
        .checked_add_signed(Duration::days(
            first_weekday_offset + 7 * i64::from(nth - 1),
        ))
        .expect("nth weekday must be representable")
}

fn last_weekday_of_month(year: i32, month: u32, weekday: Weekday) -> NaiveDate {
    let next_month = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).expect("valid calendar date")
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).expect("valid calendar date")
    };
    let last_day = next_month
        .checked_sub_signed(Duration::days(1))
        .expect("previous day must be valid");
    let backward_offset = (7 + last_day.weekday().num_days_from_monday() as i64
        - weekday.num_days_from_monday() as i64)
        % 7;
    last_day
        .checked_sub_signed(Duration::days(backward_offset))
        .expect("last weekday must be representable")
}

fn coverage_summary(indicator_risks: &[IndicatorRisk]) -> (f64, f64, f64, f64) {
    const FORMAL_CORE_INDICATORS: &[&str] = &[
        "us_market_vix_close",
        "us_rates_yield_curve_10y2y",
        "us_credit_baa_10y_spread",
        "us_liquidity_effr",
        "us_liquidity_national_financial_conditions",
        "us_liquidity_financial_stress_stl",
        "us_macro_unemployment_rate",
        "us_real_estate_housing_starts",
    ];
    const FORMAL_TRIGGER_INDICATORS: &[&str] = &[
        "us_market_vix_close",
        "us_rates_yield_curve_10y2y",
        "us_credit_baa_10y_spread",
        "us_liquidity_effr",
        "us_liquidity_national_financial_conditions",
        "us_liquidity_financial_stress_stl",
    ];
    const FORMAL_EXTERNAL_INDICATORS: &[&str] = &["us_external_usdjpy_level", "jp_rates_call_rate"];

    let (core_total, core_present) =
        coverage_by_indicator_ids(indicator_risks, FORMAL_CORE_INDICATORS);
    let (trigger_total, trigger_present) =
        coverage_by_indicator_ids(indicator_risks, FORMAL_TRIGGER_INDICATORS);
    let (external_total, external_present) =
        coverage_by_indicator_ids(indicator_risks, FORMAL_EXTERNAL_INDICATORS);

    let core_feature_coverage = ratio(core_present, core_total);
    let trigger_feature_coverage = ratio(trigger_present, trigger_total);
    let external_feature_coverage = ratio(external_present, external_total);
    let coverage_score = round3(
        (core_feature_coverage * 0.45
            + trigger_feature_coverage * 0.35
            + external_feature_coverage * 0.2)
            .clamp(0.0, 1.0),
    );
    (
        round3(core_feature_coverage),
        round3(trigger_feature_coverage),
        round3(external_feature_coverage),
        coverage_score,
    )
}

fn coverage_by_indicator_ids(
    indicator_risks: &[IndicatorRisk],
    indicator_ids: &[&str],
) -> (usize, usize) {
    indicator_risks
        .iter()
        .filter(|risk| indicator_ids.contains(&risk.indicator.indicator_id.as_str()))
        .fold((0_usize, 0_usize), |(total, present), risk| {
            (
                total + 1,
                present + usize::from(risk.latest_observation.is_some()),
            )
        })
}

fn ratio(present: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        present as f64 / total as f64
    }
}

fn find_dimension_score(indicator_risks: &[IndicatorRisk], dimension: RiskDimension) -> f64 {
    let scores = indicator_risks
        .iter()
        .filter(|risk| risk.indicator.dimension == dimension)
        .filter(|risk| risk.latest_observation.is_some())
        .map(|risk| risk.score)
        .collect::<Vec<_>>();
    if scores.is_empty() {
        0.0
    } else {
        scores.iter().sum::<f64>() / scores.len() as f64
    }
}

fn has_main_dataset_core_features(features: &BTreeMap<String, f64>) -> bool {
    [
        "us_vix_level",
        "us_curve_10y2y_level",
        "us_baa_10y_spread_level",
        "us_fed_funds_level",
    ]
    .into_iter()
    .all(|feature| features.contains_key(feature))
}

fn feature_quality_grade(coverage_score: f64) -> &'static str {
    if coverage_score >= 0.9 {
        "a"
    } else if coverage_score >= 0.8 {
        "b"
    } else if coverage_score >= 0.7 {
        "c"
    } else if coverage_score >= 0.6 {
        "d"
    } else {
        "f"
    }
}

fn assign_formal_dataset_splits(rows: &mut [FormalDatasetRowRecord], scenarios: &[CrisisScenario]) {
    let ranges = collect_formal_dataset_scenario_ranges(rows, scenarios);
    let Ok((train_end, calibration_end)) = scenario_aware_formal_split_bounds(rows, &ranges)
        .or_else(|_| chronological_split_bounds(rows.len()))
    else {
        return;
    };
    for (index, row) in rows.iter_mut().enumerate() {
        row.split_name = split_name_for_index(index, train_end, calibration_end).to_string();
    }
}

#[derive(Debug, Clone)]
struct ScenarioRowRange {
    scenario_id: String,
    family: String,
    start_index: usize,
    end_index: usize,
}

fn scenario_aware_formal_split_bounds(
    rows: &[FormalDatasetRowRecord],
    ranges: &[ScenarioRowRange],
) -> anyhow::Result<(usize, usize)> {
    if ranges.len() < 3 {
        bail!("fewer than 3 scenario ranges available for scenario-aware split");
    }
    let (baseline_train_end, baseline_calibration_end) = chronological_split_bounds(rows.len())?;
    let label_support = FormalSplitLabelSupport::from_rows(rows);
    let mut best_candidate = None::<(usize, usize, usize, usize, usize)>;

    for first_boundary_scenario in 0..ranges.len().saturating_sub(1) {
        let train_candidates = split_boundaries_within_scenario(&ranges[first_boundary_scenario]);
        for second_boundary_scenario in (first_boundary_scenario + 1)..ranges.len() {
            let calibration_candidates =
                split_boundaries_within_scenario(&ranges[second_boundary_scenario]);
            for &train_end in &train_candidates {
                for &calibration_end in &calibration_candidates {
                    if validate_split_bounds(rows.len(), train_end, calibration_end).is_err() {
                        continue;
                    }

                    let calibration_scenario_count =
                        scenario_count_for_split_range(ranges, train_end, calibration_end);
                    let evaluation_scenario_count =
                        scenario_count_for_split_range(ranges, calibration_end, rows.len());
                    if calibration_scenario_count < 2 || evaluation_scenario_count < 2 {
                        continue;
                    }

                    if !label_support.split_has_required_label_support(0, train_end)
                        || !label_support
                            .split_has_required_label_support(train_end, calibration_end)
                        || !label_support
                            .split_has_required_label_support(calibration_end, rows.len())
                    {
                        continue;
                    }

                    let scenario_coverage =
                        calibration_scenario_count.saturating_add(evaluation_scenario_count);
                    let evaluation_actionability_support_score =
                        split_actionability_scenario_support_score(
                            rows,
                            ranges,
                            calibration_end,
                            rows.len(),
                        );
                    let deviation_from_baseline = train_end.abs_diff(baseline_train_end)
                        + calibration_end.abs_diff(baseline_calibration_end);
                    let replace_candidate = match best_candidate {
                        None => true,
                        Some((
                            best_train_end,
                            best_calibration_end,
                            best_coverage,
                            best_actionability_support_score,
                            best_deviation,
                        )) => {
                            scenario_coverage > best_coverage
                                || (scenario_coverage == best_coverage
                                    && evaluation_actionability_support_score
                                        > best_actionability_support_score)
                                || (scenario_coverage == best_coverage
                                    && evaluation_actionability_support_score
                                        == best_actionability_support_score
                                    && deviation_from_baseline < best_deviation)
                                || (scenario_coverage == best_coverage
                                    && evaluation_actionability_support_score
                                        == best_actionability_support_score
                                    && deviation_from_baseline == best_deviation
                                    && (train_end > best_train_end
                                        || (train_end == best_train_end
                                            && calibration_end > best_calibration_end)))
                        }
                    };
                    if replace_candidate {
                        best_candidate = Some((
                            train_end,
                            calibration_end,
                            scenario_coverage,
                            evaluation_actionability_support_score,
                            deviation_from_baseline,
                        ));
                    }
                }
            }
        }
    }

    best_candidate
        .map(|(train_end, calibration_end, _, _, _)| (train_end, calibration_end))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "no scenario-aware split preserves multi-scenario calibration/evaluation coverage together with forward/action label support"
            )
        })
}

fn collect_formal_dataset_scenario_ranges(
    rows: &[FormalDatasetRowRecord],
    scenarios: &[CrisisScenario],
) -> Vec<ScenarioRowRange> {
    scenarios
        .iter()
        .filter_map(|scenario| {
            let start_date = scenario_support_window_start_date(scenario);
            let end_date = scenario_support_window_end_date(scenario);
            let start_index = rows.partition_point(|row| row.as_of_date < start_date);
            let end_exclusive = rows.partition_point(|row| row.as_of_date <= end_date);
            (start_index < end_exclusive).then(|| ScenarioRowRange {
                scenario_id: scenario.scenario_id.clone(),
                family: scenario.family.clone(),
                start_index,
                end_index: end_exclusive.saturating_sub(1),
            })
        })
        .collect()
}

fn scenario_support_window_start_date(scenario: &CrisisScenario) -> NaiveDate {
    [5_u32, 20, 60]
        .into_iter()
        .map(|horizon_days| action_window_start_date(scenario, horizon_days))
        .min()
        .unwrap_or(scenario.pre_warning_start)
}

fn scenario_support_window_end_date(scenario: &CrisisScenario) -> NaiveDate {
    [5_u32, 20, 60]
        .into_iter()
        .map(|horizon_days| action_window_end_date(scenario, horizon_days))
        .max()
        .unwrap_or(scenario.crisis_end)
}

#[derive(Debug, Clone)]
struct FormalSplitLabelSupport {
    forward_5d: Vec<usize>,
    forward_20d: Vec<usize>,
    forward_60d: Vec<usize>,
    action_5d: Vec<usize>,
    action_20d: Vec<usize>,
    action_60d: Vec<usize>,
}

impl FormalSplitLabelSupport {
    fn from_rows(rows: &[FormalDatasetRowRecord]) -> Self {
        let mut support = Self {
            forward_5d: Vec::with_capacity(rows.len() + 1),
            forward_20d: Vec::with_capacity(rows.len() + 1),
            forward_60d: Vec::with_capacity(rows.len() + 1),
            action_5d: Vec::with_capacity(rows.len() + 1),
            action_20d: Vec::with_capacity(rows.len() + 1),
            action_60d: Vec::with_capacity(rows.len() + 1),
        };
        support.forward_5d.push(0);
        support.forward_20d.push(0);
        support.forward_60d.push(0);
        support.action_5d.push(0);
        support.action_20d.push(0);
        support.action_60d.push(0);
        for row in rows {
            support.forward_5d.push(
                support.forward_5d.last().copied().unwrap_or_default()
                    + usize::from(row.label_5d > 0),
            );
            support.forward_20d.push(
                support.forward_20d.last().copied().unwrap_or_default()
                    + usize::from(row.label_20d > 0),
            );
            support.forward_60d.push(
                support.forward_60d.last().copied().unwrap_or_default()
                    + usize::from(row.label_60d > 0),
            );
            support.action_5d.push(
                support.action_5d.last().copied().unwrap_or_default()
                    + usize::from(row.action_label_5d > 0),
            );
            support.action_20d.push(
                support.action_20d.last().copied().unwrap_or_default()
                    + usize::from(row.action_label_20d > 0),
            );
            support.action_60d.push(
                support.action_60d.last().copied().unwrap_or_default()
                    + usize::from(row.action_label_60d > 0),
            );
        }
        support
    }

    fn split_has_required_label_support(&self, start: usize, end: usize) -> bool {
        end > start
            && self.has_positive(&self.forward_5d, start, end)
            && self.has_positive(&self.forward_20d, start, end)
            && self.has_positive(&self.forward_60d, start, end)
            && self.has_positive(&self.action_5d, start, end)
            && self.has_positive(&self.action_20d, start, end)
            && self.has_positive(&self.action_60d, start, end)
    }

    fn has_positive(&self, prefix: &[usize], start: usize, end: usize) -> bool {
        prefix[end] > prefix[start]
    }
}

fn split_boundaries_within_scenario(range: &ScenarioRowRange) -> Vec<usize> {
    ((range.start_index + 1)..=range.end_index.saturating_add(1)).collect()
}

fn scenario_count_for_split_range(ranges: &[ScenarioRowRange], start: usize, end: usize) -> usize {
    ranges
        .iter()
        .filter(|range| start <= range.end_index && end > range.start_index)
        .count()
}

fn split_actionability_scenario_support_score(
    rows: &[FormalDatasetRowRecord],
    ranges: &[ScenarioRowRange],
    start: usize,
    end: usize,
) -> usize {
    [5_u32, 20, 60]
        .into_iter()
        .map(|horizon_days| {
            actionability_positive_scenario_count_for_split_range(
                rows,
                ranges,
                start,
                end,
                horizon_days,
            )
            .min(2)
        })
        .sum()
}

fn actionability_positive_scenario_count_for_split_range(
    rows: &[FormalDatasetRowRecord],
    ranges: &[ScenarioRowRange],
    start: usize,
    end: usize,
    horizon_days: u32,
) -> usize {
    ranges
        .iter()
        .filter(|range| {
            let overlap_start = start.max(range.start_index);
            let overlap_end = end.min(range.end_index.saturating_add(1));
            overlap_start < overlap_end
                && rows[overlap_start..overlap_end].iter().any(|row| {
                    row.primary_scenario_id.as_deref() == Some(range.scenario_id.as_str())
                        && row_has_action_label(row, horizon_days)
                })
        })
        .count()
}

fn row_has_action_label(row: &FormalDatasetRowRecord, horizon_days: u32) -> bool {
    match horizon_days {
        5 => row.action_label_5d > 0,
        20 => row.action_label_20d > 0,
        60 => row.action_label_60d > 0,
        _ => false,
    }
}

#[cfg(test)]
fn scenario_count_for_index_range(
    rows: &[FormalDatasetRowRecord],
    start: usize,
    end: usize,
) -> usize {
    rows[start.min(rows.len())..end.min(rows.len())]
        .iter()
        .filter_map(|row| row.primary_scenario_id.as_ref())
        .collect::<BTreeSet<_>>()
        .len()
}

fn split_name_for_index(index: usize, train_end: usize, calibration_end: usize) -> &'static str {
    if index < train_end {
        "train"
    } else if index < calibration_end {
        "calibration"
    } else {
        "evaluation"
    }
}

fn load_label_set_crisis_scenarios(
    scenario_set_version: &str,
    label_set_id: &str,
) -> anyhow::Result<Vec<CrisisScenario>> {
    let catalog = load_crisis_scenario_catalog();
    if catalog.catalog_id != scenario_set_version {
        bail!(
            "scenario set version {} is not available in the active catalog {}; set FC_SCENARIO_CATALOG_PATH to another catalog or use {}",
            scenario_set_version,
            catalog.catalog_id,
            catalog.catalog_id
        );
    }

    let scenarios = catalog
        .scenarios_for_label_set(label_set_id)
        .with_context(|| format!("label set {label_set_id} was not found in scenario catalog"))?;

    Ok(scenarios
        .into_iter()
        .map(|scenario| CrisisScenario {
            scenario_id: scenario.scenario_id.clone(),
            family: scenario_family_code(scenario.family).to_string(),
            pre_warning_start: scenario.pre_warning_start,
            crisis_start: scenario.crisis_start,
            acute_start: scenario.acute_start,
            crisis_end: scenario.crisis_end,
            default_horizon_roles: scenario.default_horizon_roles.clone(),
        })
        .collect())
}

fn scenario_family_code(family: fc_domain::CrisisScenarioFamily) -> &'static str {
    match family {
        fc_domain::CrisisScenarioFamily::AcuteMarketLiquidityCrash => {
            "acute_market_liquidity_crash"
        }
        fc_domain::CrisisScenarioFamily::SystemicCreditBankingCrisis => {
            "systemic_credit_banking_crisis"
        }
        fc_domain::CrisisScenarioFamily::MixedSystemicStress => "mixed_systemic_stress",
        fc_domain::CrisisScenarioFamily::RateShockOrPolicyDislocation => {
            "rate_shock_or_policy_dislocation"
        }
    }
}

fn scenario_supports_horizon(scenario: &CrisisScenario, horizon_days: u32) -> bool {
    scenario.default_horizon_roles.contains(&horizon_days)
}

fn label_anchor_date(scenario: &CrisisScenario, horizon_days: u32) -> NaiveDate {
    if horizon_days == 5 {
        scenario.acute_start.unwrap_or(scenario.crisis_start)
    } else {
        scenario.crisis_start
    }
}

fn action_window_lead_days(horizon_days: u32) -> i64 {
    match horizon_days {
        5 => 10,
        20 => 35,
        60 => 90,
        _ => horizon_days as i64,
    }
}

fn action_window_start_date(scenario: &CrisisScenario, horizon_days: u32) -> NaiveDate {
    let anchor_date = label_anchor_date(scenario, horizon_days);
    let buffered_start = anchor_date
        .checked_sub_signed(Duration::days(action_window_lead_days(horizon_days)))
        .unwrap_or(anchor_date);
    scenario.pre_warning_start.max(buffered_start)
}

fn action_window_end_days(horizon_days: u32) -> i64 {
    match horizon_days {
        5 => 7,
        20 => 20,
        60 => 30,
        _ => horizon_days as i64,
    }
}

fn action_window_end_date(scenario: &CrisisScenario, horizon_days: u32) -> NaiveDate {
    let anchor_date = label_anchor_date(scenario, horizon_days);
    let buffered_end = anchor_date
        .checked_add_signed(Duration::days(action_window_end_days(horizon_days)))
        .unwrap_or(scenario.crisis_end);
    scenario.crisis_end.min(buffered_end)
}

fn action_window_label(
    as_of_date: NaiveDate,
    scenarios: &[CrisisScenario],
    horizon_days: i64,
) -> u8 {
    let horizon_days_u32 = horizon_days as u32;
    scenarios.iter().any(|scenario| {
        as_of_date >= action_window_start_date(scenario, horizon_days_u32)
            && as_of_date <= action_window_end_date(scenario, horizon_days_u32)
    }) as u8
}

fn scenario_has_action_window(scenario: &CrisisScenario, as_of_date: NaiveDate) -> bool {
    scenario.default_horizon_roles.iter().any(|horizon_days| {
        as_of_date >= action_window_start_date(scenario, *horizon_days)
            && as_of_date <= action_window_end_date(scenario, *horizon_days)
    })
}

fn primary_scenario_for_date(
    as_of_date: NaiveDate,
    scenarios: &[CrisisScenario],
) -> Option<CrisisScenario> {
    scenarios
        .iter()
        .filter(|scenario| scenario_has_action_window(scenario, as_of_date))
        .min_by_key(|scenario| {
            let distance = (scenario.crisis_start - as_of_date).num_days().abs();
            let in_crisis_penalty = if as_of_date > scenario.crisis_start {
                10_000
            } else {
                0
            };
            in_crisis_penalty + distance
        })
        .cloned()
        .or_else(|| forward_scenario(as_of_date, scenarios, 60))
}

fn forward_scenario(
    as_of_date: NaiveDate,
    scenarios: &[CrisisScenario],
    horizon_days: i64,
) -> Option<CrisisScenario> {
    scenarios
        .iter()
        .filter_map(|scenario| {
            let lead_days = (scenario.crisis_start - as_of_date).num_days();
            (1..=horizon_days)
                .contains(&lead_days)
                .then_some((scenario.clone(), lead_days))
        })
        .min_by_key(|(_, lead_days)| *lead_days)
        .map(|(scenario, _)| scenario)
}

fn formal_dataset_key(dataset_id: &str, dataset_version: &str) -> String {
    format!("{dataset_id}:{dataset_version}")
}

fn round3(value: f64) -> f64 {
    (value * 1_000.0).round() / 1_000.0
}

fn round6(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

fn actionability_level_text(level: ActionabilityLevel) -> &'static str {
    match level {
        ActionabilityLevel::Prepare => "prepare",
        ActionabilityLevel::Hedge => "hedge",
        ActionabilityLevel::Defend => "defend",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ProbabilityTrainingRegime {
    Normal,
    PositiveWindow,
    PreWarningBuffer,
    InCrisis,
    PostCrisisCooldown,
}

#[derive(Debug, Clone, Serialize)]
struct ProbabilityTrainingRow {
    as_of_date: NaiveDate,
    market_scope: String,
    release_id: Option<String>,
    probability_mode: Option<String>,
    freshness_status: Option<String>,
    time_to_risk_bucket: Option<String>,
    split_name: Option<String>,
    features: BTreeMap<String, f64>,
    primary_scenario_id: Option<String>,
    scenario_family: Option<String>,
    days_to_primary_crisis_start: Option<i64>,
    primary_scenario_supports_5d: bool,
    primary_scenario_supports_20d: bool,
    primary_scenario_supports_60d: bool,
    label_5d: u8,
    label_20d: u8,
    label_60d: u8,
    regime_5d: ProbabilityTrainingRegime,
    regime_20d: ProbabilityTrainingRegime,
    regime_60d: ProbabilityTrainingRegime,
    action_label_5d: u8,
    action_label_20d: u8,
    action_label_60d: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ProbabilityTargetLabelMode {
    ForwardCrisis,
    ActionWindow,
}

impl ProbabilityTargetLabelMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::ForwardCrisis => "forward_crisis",
            Self::ActionWindow => "action_window",
        }
    }
}

impl ProbabilityTrainingRow {
    fn label_for_horizon(&self, label_mode: ProbabilityTargetLabelMode, horizon_days: u32) -> f64 {
        match (label_mode, horizon_days) {
            (ProbabilityTargetLabelMode::ForwardCrisis, 5) => self.label_5d as f64,
            (ProbabilityTargetLabelMode::ForwardCrisis, 20) => self.label_20d as f64,
            (ProbabilityTargetLabelMode::ForwardCrisis, 60) => self.label_60d as f64,
            (ProbabilityTargetLabelMode::ActionWindow, 5) => self.action_label_5d as f64,
            (ProbabilityTargetLabelMode::ActionWindow, 20) => self.action_label_20d as f64,
            (ProbabilityTargetLabelMode::ActionWindow, 60) => self.action_label_60d as f64,
            _ => 0.0,
        }
    }

    fn primary_scenario_supports_horizon(&self, horizon_days: u32) -> Option<bool> {
        self.primary_scenario_id
            .as_ref()
            .map(|_| match horizon_days {
                5 => self.primary_scenario_supports_5d,
                20 => self.primary_scenario_supports_20d,
                60 => self.primary_scenario_supports_60d,
                _ => false,
            })
    }

    fn regime_for_horizon(&self, horizon_days: u32) -> ProbabilityTrainingRegime {
        match horizon_days {
            5 => self.regime_5d,
            20 => self.regime_20d,
            60 => self.regime_60d,
            _ => ProbabilityTrainingRegime::Normal,
        }
    }
}

#[derive(Debug, Clone)]
struct ProbabilityTrainingInput {
    dataset_source: PipelineDatasetSource,
    dataset_label: String,
    market_scope: String,
    point_in_time_mode: String,
    feature_set_version: String,
    label_version: String,
    feature_names: Vec<String>,
    train_rows: Vec<ProbabilityTrainingRow>,
    calibration_rows: Vec<ProbabilityTrainingRow>,
    evaluation_rows: Vec<ProbabilityTrainingRow>,
}

#[derive(Debug, Clone)]
struct PipelineArtifacts {
    release: ModelReleaseRecord,
    bundle: ProbabilityBundle,
    bundle_path: PathBuf,
    manifest_path: PathBuf,
    evaluation_path: PathBuf,
    dataset_source: String,
    dataset_label: String,
}

#[derive(Debug, Clone, Serialize)]
struct PipelineEvaluationReport {
    release_id: String,
    dataset_source: String,
    dataset_label: String,
    target_label_mode: ProbabilityTargetLabelMode,
    market_scope: String,
    feature_names: Vec<String>,
    training_samples: usize,
    calibration_samples: usize,
    evaluation_samples: usize,
    horizons: Vec<ProbabilityHorizonBundle>,
    actionability: Option<ActionabilityBundle>,
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

fn transitional_feature_names() -> Vec<String> {
    TRANSITIONAL_PROBABILITY_BUNDLE_FEATURES
        .iter()
        .map(|feature| (*feature).to_string())
        .collect()
}

fn formal_feature_names() -> Vec<String> {
    FORMAL_PROBABILITY_BUNDLE_FEATURES
        .iter()
        .map(|feature| (*feature).to_string())
        .collect()
}

async fn resolve_formal_dataset_key(
    store: &SqliteStore,
    dataset_key: Option<&str>,
    dataset_id: &str,
    dataset_version: Option<&str>,
    market_scope: Option<&str>,
) -> anyhow::Result<String> {
    if let Some(dataset_key) = dataset_key {
        return Ok(dataset_key.to_string());
    }
    if let Some(dataset_version) = dataset_version {
        return Ok(formal_dataset_key(dataset_id, dataset_version));
    }

    let market_scope = market_scope.unwrap_or("financial_system");
    let latest = store
        .list_formal_datasets(Some(market_scope), Some(dataset_id), Some(1))
        .await?
        .into_iter()
        .next()
        .with_context(|| {
            format!(
                "no persisted formal dataset found for market scope {market_scope} and dataset id {dataset_id}"
            )
        })?;
    Ok(formal_dataset_key(
        &latest.manifest.dataset_id,
        &latest.manifest.dataset_version,
    ))
}

async fn resolve_formal_training_dataset_key(
    store: &SqliteStore,
    options: &PipelineTrainOptions,
) -> anyhow::Result<String> {
    resolve_formal_dataset_key(
        store,
        options.dataset_key.as_deref(),
        &options.dataset_id,
        options.dataset_version.as_deref(),
        options.query.market_scope.as_deref(),
    )
    .await
}

async fn load_formal_training_dataset(
    store: &SqliteStore,
    options: &PipelineTrainOptions,
) -> anyhow::Result<ProbabilityTrainingInput> {
    let dataset_key = resolve_formal_training_dataset_key(store, options).await?;
    let dataset = store
        .load_formal_dataset(&dataset_key)
        .await?
        .with_context(|| format!("formal dataset {dataset_key} was not found in SQLite"))?;
    let mut rows = store
        .list_formal_dataset_rows(&dataset_key, None, None)
        .await?;
    if let Some(from) = options.query.from {
        rows.retain(|row| row.as_of_date >= from);
    }
    if let Some(to) = options.query.to {
        rows.retain(|row| row.as_of_date <= to);
    }
    if rows.len() < 90 {
        bail!(
            "formal dataset {dataset_key} is too small after filters: {} rows found, at least 90 are required; backfill more free historical observations and rebuild the formal dataset, or use --dataset-source snapshot as a temporary fallback",
            rows.len()
        );
    }

    let scenarios = load_label_set_crisis_scenarios(
        &dataset.manifest.scenario_set_version,
        &dataset.manifest.label_version,
    )?;
    let scenario_by_id = scenarios
        .iter()
        .cloned()
        .map(|scenario| (scenario.scenario_id.clone(), scenario))
        .collect::<BTreeMap<_, _>>();

    let to_training_row = |row: &FormalDatasetRowRecord| {
        let primary_scenario = row
            .primary_scenario_id
            .as_ref()
            .and_then(|scenario_id| scenario_by_id.get(scenario_id));
        ProbabilityTrainingRow {
            as_of_date: row.as_of_date,
            market_scope: row.market_scope.clone(),
            release_id: None,
            probability_mode: Some("formal_bundle_v1".to_string()),
            freshness_status: Some(row.sample_quality_grade.clone()),
            time_to_risk_bucket: row.primary_scenario_id.clone(),
            split_name: Some(row.split_name.clone()),
            features: row.features.clone(),
            primary_scenario_id: row.primary_scenario_id.clone(),
            scenario_family: row.scenario_family.clone(),
            days_to_primary_crisis_start: primary_scenario
                .map(|scenario| (scenario.crisis_start - row.as_of_date).num_days()),
            primary_scenario_supports_5d: primary_scenario
                .is_some_and(|scenario| scenario_supports_horizon(scenario, 5)),
            primary_scenario_supports_20d: primary_scenario
                .is_some_and(|scenario| scenario_supports_horizon(scenario, 20)),
            primary_scenario_supports_60d: primary_scenario
                .is_some_and(|scenario| scenario_supports_horizon(scenario, 60)),
            label_5d: row.label_5d,
            label_20d: row.label_20d,
            label_60d: row.label_60d,
            regime_5d: forward_crisis_training_regime(row.as_of_date, &scenarios, 5),
            regime_20d: forward_crisis_training_regime(row.as_of_date, &scenarios, 20),
            regime_60d: forward_crisis_training_regime(row.as_of_date, &scenarios, 60),
            action_label_5d: row.action_label_5d,
            action_label_20d: row.action_label_20d,
            action_label_60d: row.action_label_60d,
        }
    };

    let train_rows = rows
        .iter()
        .filter(|row| row.split_name == "train")
        .map(to_training_row)
        .collect::<Vec<_>>();
    let calibration_rows = rows
        .iter()
        .filter(|row| row.split_name == "calibration")
        .map(to_training_row)
        .collect::<Vec<_>>();
    let evaluation_rows = rows
        .iter()
        .filter(|row| row.split_name == "evaluation")
        .map(to_training_row)
        .collect::<Vec<_>>();

    if train_rows.is_empty() || calibration_rows.is_empty() || evaluation_rows.is_empty() {
        bail!(
            "formal dataset {dataset_key} is missing one or more required splits after filters (train={}, calibration={}, evaluation={}); rebuild it from a broader historical range before training the formal bundle",
            train_rows.len(),
            calibration_rows.len(),
            evaluation_rows.len()
        );
    }

    Ok(ProbabilityTrainingInput {
        dataset_source: PipelineDatasetSource::Formal,
        dataset_label: dataset_key,
        market_scope: dataset.manifest.market_scope.clone(),
        point_in_time_mode: dataset.manifest.point_in_time_mode.clone(),
        feature_set_version: dataset.manifest.feature_set_version.clone(),
        label_version: dataset.manifest.label_version.clone(),
        feature_names: formal_feature_names(),
        train_rows,
        calibration_rows,
        evaluation_rows,
    })
}

async fn load_snapshot_training_dataset(
    store: &SqliteStore,
    options: &PipelineTrainOptions,
) -> anyhow::Result<ProbabilityTrainingInput> {
    let snapshots = load_training_snapshots(store, &options.query).await?;
    let dataset = build_pipeline_dataset_rows(&snapshots);
    if dataset.len() < 90 {
        bail!(
            "training dataset is too small: {} rows found, at least 90 are required",
            dataset.len()
        );
    }

    let (train_rows, calibration_rows, evaluation_rows) = chronological_split(&dataset)?;
    let market_scope = train_rows
        .first()
        .map(|row| row.market_scope.clone())
        .unwrap_or_else(|| "financial_system".to_string());
    let dataset_label = train_rows
        .first()
        .and_then(|row| row.release_id.clone())
        .unwrap_or_else(|| "heuristic_prediction_snapshots".to_string());

    Ok(ProbabilityTrainingInput {
        dataset_source: PipelineDatasetSource::Snapshot,
        dataset_label,
        market_scope,
        point_in_time_mode: "best_effort".to_string(),
        feature_set_version: "feature_prob_meta_v1".to_string(),
        label_version: "label_forward_crisis_v1".to_string(),
        feature_names: transitional_feature_names(),
        train_rows,
        calibration_rows,
        evaluation_rows,
    })
}

async fn load_probability_training_input(
    store: &SqliteStore,
    options: &PipelineTrainOptions,
) -> anyhow::Result<ProbabilityTrainingInput> {
    match options.dataset_source {
        PipelineDatasetSource::Formal => load_formal_training_dataset(store, options).await,
        PipelineDatasetSource::Snapshot => load_snapshot_training_dataset(store, options).await,
    }
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
    dataset: &[ProbabilityTrainingRow],
    feature_names: &[String],
    format: ExportFormat,
    output_path: Option<&Path>,
) -> anyhow::Result<()> {
    let content = match format {
        ExportFormat::Json => serde_json::to_string_pretty(dataset)?,
        ExportFormat::Csv => render_dataset_csv(dataset, feature_names),
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

fn render_dataset_csv(dataset: &[ProbabilityTrainingRow], feature_names: &[String]) -> String {
    let mut header = String::from(
        "as_of_date,market_scope,release_id,probability_mode,freshness_status,time_to_risk_bucket,split_name,label_5d,label_20d,label_60d,action_label_5d,action_label_20d,action_label_60d",
    );
    for feature in feature_names {
        header.push(',');
        header.push_str(feature);
    }
    header.push('\n');

    let mut csv = header;
    for row in dataset {
        let _ = write!(
            csv,
            "{},{},{},{},{},{},{},{},{},{},{},{},{}",
            row.as_of_date,
            row.market_scope,
            row.release_id.as_deref().unwrap_or(""),
            row.probability_mode.as_deref().unwrap_or(""),
            row.freshness_status.as_deref().unwrap_or(""),
            row.time_to_risk_bucket.as_deref().unwrap_or(""),
            row.split_name.as_deref().unwrap_or(""),
            row.label_5d,
            row.label_20d,
            row.label_60d,
            row.action_label_5d,
            row.action_label_20d,
            row.action_label_60d
        );
        for feature in feature_names {
            let value = row.features.get(feature).copied().unwrap_or_default();
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
    let training = load_probability_training_input(store, options).await?;
    let crisis_prior_label_mode = ProbabilityTargetLabelMode::ForwardCrisis;
    let horizons = [5_u32, 20_u32, 60_u32]
        .into_iter()
        .map(|horizon| {
            train_horizon_bundle(
                &training.train_rows,
                &training.calibration_rows,
                &training.evaluation_rows,
                &training.feature_names,
                horizon,
                crisis_prior_label_mode,
            )
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    let actionability = if matches!(training.dataset_source, PipelineDatasetSource::Formal)
        && training_rows_support_label_mode(
            &training.train_rows,
            &training.calibration_rows,
            &training.evaluation_rows,
            ProbabilityTargetLabelMode::ActionWindow,
        ) {
        let candidate = train_actionability_bundle(
            &training.train_rows,
            &training.calibration_rows,
            &training.evaluation_rows,
            &training.feature_names,
            &generated_at.format("%Y%m%dT%H%M%S").to_string(),
        )?;
        let guard_regressions = actionability_bundle_quality_regressions(&candidate);
        if guard_regressions.is_empty() {
            Some(candidate)
        } else {
            println!("Actionability head disabled for this release:");
            for regression in &guard_regressions {
                println!("  - {regression}");
            }
            None
        }
    } else {
        None
    };

    let aggregate_evaluation = summarize_bundle_evaluation(&horizons);
    let release_suffix = generated_at.format("%Y%m%dT%H%M%S").to_string();
    let release_id = format!("{}_{}", options.release_prefix, release_suffix);
    let bundle_note = match training.dataset_source {
        PipelineDatasetSource::Formal => format!(
            "Formal bundle trained from persisted formal dataset {} built from raw observations -> feature snapshots -> scenario labels; crisis-prior head uses forward-crisis labels, and {}.",
            training.dataset_label,
            if actionability.is_some() {
                "actionability head uses bounded action-window labels when quality gates pass"
            } else {
                "independent actionability head was omitted because evaluation quality gates did not pass, so runtime falls back to probability-context fusion"
            }
        ),
        PipelineDatasetSource::Snapshot => {
            "Transitional formal bundle trained from persisted heuristic prediction snapshots, calibrated with chronological holdout slices, and reweighted toward positive warning windows under severe class imbalance.".to_string()
        }
    };
    let bundle = ProbabilityBundle {
        bundle_id: release_id.clone(),
        market_scope: training.market_scope.clone(),
        probability_mode: "formal_bundle_v1".to_string(),
        created_at: generated_at,
        feature_names: training.feature_names.clone(),
        monotonic_min_gap_5d_to_20d: 0.02,
        monotonic_min_gap_20d_to_60d: 0.03,
        note: bundle_note.clone(),
        horizons: horizons.clone(),
        evaluation: Some(aggregate_evaluation.clone()),
        actionability: actionability.clone(),
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
            feature_set_version: training.feature_set_version.clone(),
            label_version: training.label_version.clone(),
            prob_model_version: format!("prob_bundle_{release_suffix}"),
            calibration_version: format!("platt_{release_suffix}"),
            posture_policy_version: "posture_v1_20260530".to_string(),
            action_playbook_version: "action_playbook_v1_20260531".to_string(),
            point_in_time_mode: training.point_in_time_mode.clone(),
            training_range_start: training.train_rows.first().map(|row| row.as_of_date),
            training_range_end: training.train_rows.last().map(|row| row.as_of_date),
            calibration_range_start: training.calibration_rows.first().map(|row| row.as_of_date),
            calibration_range_end: training.calibration_rows.last().map(|row| row.as_of_date),
            evaluation_range_start: training.evaluation_rows.first().map(|row| row.as_of_date),
            evaluation_range_end: training.evaluation_rows.last().map(|row| row.as_of_date),
            brier_score: bundle
                .evaluation
                .as_ref()
                .map(|summary| summary.brier_score),
            log_loss: bundle.evaluation.as_ref().map(|summary| summary.log_loss),
            ece: bundle.evaluation.as_ref().map(|summary| summary.ece),
            note: format!(
                "Generated by `research pipeline train-probability` from {} dataset {}.",
                training.dataset_source.as_str(),
                training.dataset_label
            ),
        },
        created_at: generated_at,
        activated_at: None,
        retired_at: None,
    };

    let evaluation_report = PipelineEvaluationReport {
        release_id: release_id.clone(),
        dataset_source: training.dataset_source.as_str().to_string(),
        dataset_label: training.dataset_label.clone(),
        target_label_mode: crisis_prior_label_mode,
        market_scope: release.manifest.market_scope.clone(),
        feature_names: training.feature_names.clone(),
        training_samples: training.train_rows.len(),
        calibration_samples: training.calibration_rows.len(),
        evaluation_samples: training.evaluation_rows.len(),
        horizons,
        actionability,
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
        dataset_source: training.dataset_source.as_str().to_string(),
        dataset_label: training.dataset_label,
    })
}

fn build_pipeline_dataset_rows(
    snapshots: &[PredictionSnapshotRecord],
) -> Vec<ProbabilityTrainingRow> {
    let scenarios = load_label_set_crisis_scenarios(
        DEFAULT_FORMAL_SCENARIO_SET_VERSION,
        DEFAULT_FORMAL_LABEL_VERSION,
    )
    .expect("default scenario catalog must contain the main training label set");
    let mut rows = snapshots
        .iter()
        .map(|snapshot| {
            let features = pipeline_features_from_snapshot(snapshot);
            let primary_scenario = primary_scenario_for_date(snapshot.as_of_date, &scenarios);
            ProbabilityTrainingRow {
                as_of_date: snapshot.as_of_date,
                market_scope: snapshot.market_scope.clone(),
                release_id: snapshot.release_id.clone(),
                probability_mode: Some(snapshot.probability_mode.clone()),
                freshness_status: Some(snapshot.freshness_status.clone()),
                time_to_risk_bucket: Some(snapshot.time_to_risk_bucket.clone()),
                split_name: None,
                features,
                primary_scenario_id: primary_scenario
                    .as_ref()
                    .map(|scenario| scenario.scenario_id.clone()),
                scenario_family: primary_scenario
                    .as_ref()
                    .map(|scenario| scenario.family.clone()),
                days_to_primary_crisis_start: primary_scenario
                    .as_ref()
                    .map(|scenario| (scenario.crisis_start - snapshot.as_of_date).num_days()),
                primary_scenario_supports_5d: primary_scenario
                    .as_ref()
                    .is_some_and(|scenario| scenario_supports_horizon(scenario, 5)),
                primary_scenario_supports_20d: primary_scenario
                    .as_ref()
                    .is_some_and(|scenario| scenario_supports_horizon(scenario, 20)),
                primary_scenario_supports_60d: primary_scenario
                    .as_ref()
                    .is_some_and(|scenario| scenario_supports_horizon(scenario, 60)),
                label_5d: forward_crisis_label(snapshot.as_of_date, &scenarios, 5),
                label_20d: forward_crisis_label(snapshot.as_of_date, &scenarios, 20),
                label_60d: forward_crisis_label(snapshot.as_of_date, &scenarios, 60),
                regime_5d: forward_crisis_training_regime(snapshot.as_of_date, &scenarios, 5),
                regime_20d: forward_crisis_training_regime(snapshot.as_of_date, &scenarios, 20),
                regime_60d: forward_crisis_training_regime(snapshot.as_of_date, &scenarios, 60),
                action_label_5d: action_window_label(snapshot.as_of_date, &scenarios, 5),
                action_label_20d: action_window_label(snapshot.as_of_date, &scenarios, 20),
                action_label_60d: action_window_label(snapshot.as_of_date, &scenarios, 60),
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

fn forward_crisis_label(
    as_of_date: NaiveDate,
    scenarios: &[CrisisScenario],
    horizon_days: i64,
) -> u8 {
    let horizon_days_u32 = horizon_days as u32;
    scenarios.iter().any(|scenario| {
        let anchor_date = if scenario_supports_horizon(scenario, horizon_days_u32) {
            label_anchor_date(scenario, horizon_days_u32)
        } else {
            scenario.crisis_start
        };
        let lead_days = (anchor_date - as_of_date).num_days();
        (1..=horizon_days).contains(&lead_days)
    }) as u8
}

fn post_crisis_cooldown_days(horizon_days: u32) -> i64 {
    match horizon_days {
        5 => 14,
        20 => 30,
        60 => 45,
        _ => horizon_days as i64,
    }
}

fn forward_crisis_training_regime(
    as_of_date: NaiveDate,
    scenarios: &[CrisisScenario],
    horizon_days: u32,
) -> ProbabilityTrainingRegime {
    if forward_crisis_label(as_of_date, scenarios, horizon_days as i64) > 0 {
        return ProbabilityTrainingRegime::PositiveWindow;
    }

    let positive_buffer = scenarios.iter().any(|scenario| {
        let anchor_date = if scenario_supports_horizon(scenario, horizon_days) {
            label_anchor_date(scenario, horizon_days)
        } else {
            scenario.crisis_start
        };
        let positive_start = anchor_date
            .checked_sub_signed(Duration::days(horizon_days as i64))
            .unwrap_or(anchor_date);
        as_of_date >= action_window_start_date(scenario, horizon_days)
            && as_of_date < positive_start
    });
    if positive_buffer {
        return ProbabilityTrainingRegime::PreWarningBuffer;
    }

    if scenarios
        .iter()
        .any(|scenario| as_of_date >= scenario.crisis_start && as_of_date <= scenario.crisis_end)
    {
        return ProbabilityTrainingRegime::InCrisis;
    }

    let cooldown = scenarios.iter().any(|scenario| {
        let cooldown_end = scenario
            .crisis_end
            .checked_add_signed(Duration::days(post_crisis_cooldown_days(horizon_days)))
            .unwrap_or(scenario.crisis_end);
        as_of_date > scenario.crisis_end && as_of_date <= cooldown_end
    });
    if cooldown {
        return ProbabilityTrainingRegime::PostCrisisCooldown;
    }

    ProbabilityTrainingRegime::Normal
}

fn chronological_split(
    dataset: &[ProbabilityTrainingRow],
) -> anyhow::Result<(
    Vec<ProbabilityTrainingRow>,
    Vec<ProbabilityTrainingRow>,
    Vec<ProbabilityTrainingRow>,
)> {
    let (train_end, calibration_end) = chronological_split_bounds(dataset.len())?;
    Ok((
        dataset[..train_end].to_vec(),
        dataset[train_end..calibration_end].to_vec(),
        dataset[calibration_end..].to_vec(),
    ))
}

fn validate_split_bounds(
    dataset_len: usize,
    train_end: usize,
    calibration_end: usize,
) -> anyhow::Result<()> {
    if dataset_len < 30 {
        bail!("dataset is too small for chronological split");
    }
    if train_end < 30 || calibration_end <= train_end + 10 || calibration_end >= dataset_len {
        bail!("unable to construct train/calibration/evaluation split");
    }
    if dataset_len.saturating_sub(calibration_end) < 10 {
        bail!("evaluation split would be too small");
    }
    Ok(())
}

fn chronological_split_bounds(dataset_len: usize) -> anyhow::Result<(usize, usize)> {
    let train_end = (dataset_len * 6 / 10)
        .max(30)
        .min(dataset_len.saturating_sub(20));
    let calibration_end = (dataset_len * 8 / 10)
        .max(train_end + 10)
        .min(dataset_len.saturating_sub(10));
    validate_split_bounds(dataset_len, train_end, calibration_end)?;
    Ok((train_end, calibration_end))
}

fn training_rows_support_label_mode(
    train_rows: &[ProbabilityTrainingRow],
    calibration_rows: &[ProbabilityTrainingRow],
    evaluation_rows: &[ProbabilityTrainingRow],
    label_mode: ProbabilityTargetLabelMode,
) -> bool {
    [5_u32, 20_u32, 60_u32].into_iter().all(|horizon_days| {
        train_rows
            .iter()
            .any(|row| row.label_for_horizon(label_mode, horizon_days) > 0.0)
            && calibration_rows
                .iter()
                .any(|row| row.label_for_horizon(label_mode, horizon_days) > 0.0)
            && evaluation_rows
                .iter()
                .any(|row| row.label_for_horizon(label_mode, horizon_days) > 0.0)
    })
}

fn train_actionability_bundle(
    train_rows: &[ProbabilityTrainingRow],
    calibration_rows: &[ProbabilityTrainingRow],
    evaluation_rows: &[ProbabilityTrainingRow],
    feature_names: &[String],
    release_suffix: &str,
) -> anyhow::Result<ActionabilityBundle> {
    let levels = [
        (ActionabilityLevel::Prepare, 60_u32),
        (ActionabilityLevel::Hedge, 20_u32),
        (ActionabilityLevel::Defend, 5_u32),
    ]
    .into_iter()
    .map(|(level, proxy_horizon_days)| {
        train_actionability_level_bundle(
            train_rows,
            calibration_rows,
            evaluation_rows,
            feature_names,
            level,
            proxy_horizon_days,
        )
    })
    .collect::<anyhow::Result<Vec<_>>>()?;

    Ok(ActionabilityBundle {
        model_version: format!("actionability_bundle_{release_suffix}"),
        calibration_version: format!("actionability_platt_{release_suffix}"),
        fusion_policy_version: "fusion_policy_v3_probability_context_gate_20260601".to_string(),
        note: "Separate actionability head trained from bounded action-window labels to complement the crisis-prior horizons; runtime consumes threshold-aware confidence instead of treating raw action-window probabilities as direct posture signals.".to_string(),
        levels,
    })
}

fn train_actionability_level_bundle(
    train_rows: &[ProbabilityTrainingRow],
    calibration_rows: &[ProbabilityTrainingRow],
    evaluation_rows: &[ProbabilityTrainingRow],
    feature_names: &[String],
    level: ActionabilityLevel,
    proxy_horizon_days: u32,
) -> anyhow::Result<ActionabilityLevelBundle> {
    let label_mode = ProbabilityTargetLabelMode::ActionWindow;
    ensure_positive_labels(train_rows, proxy_horizon_days, "train", label_mode)?;
    ensure_positive_labels(
        calibration_rows,
        proxy_horizon_days,
        "calibration",
        label_mode,
    )?;
    ensure_positive_labels(
        evaluation_rows,
        proxy_horizon_days,
        "evaluation",
        label_mode,
    )?;

    let raw_model = fit_logistic_model(train_rows, feature_names, proxy_horizon_days, label_mode);
    let calibration_inputs = calibration_rows
        .iter()
        .map(|row| score_logistic_model_for_dataset(&raw_model, row))
        .collect::<Vec<_>>();
    let calibration_labels = calibration_rows
        .iter()
        .map(|row| row.label_for_horizon(label_mode, proxy_horizon_days))
        .collect::<Vec<_>>();
    let calibration_candidate = fit_platt_calibration(&calibration_inputs, &calibration_labels);
    let evaluation_raw_probabilities = evaluation_rows
        .iter()
        .map(|row| score_logistic_model_for_dataset(&raw_model, row))
        .collect::<Vec<_>>();
    let evaluation_labels = evaluation_rows
        .iter()
        .map(|row| row.label_for_horizon(label_mode, proxy_horizon_days))
        .collect::<Vec<_>>();
    let (calibration, evaluation_probabilities, decision_threshold) =
        select_actionability_calibration_strategy(
            &calibration_inputs,
            calibration_rows,
            &evaluation_raw_probabilities,
            proxy_horizon_days,
            calibration_candidate,
        );

    let mut evaluation = evaluate_probabilities(&evaluation_probabilities, &evaluation_labels);
    evaluation.actionability = Some(evaluate_actionability_summary(
        &evaluation_probabilities,
        evaluation_rows,
        proxy_horizon_days,
        decision_threshold,
    ));

    Ok(ActionabilityLevelBundle {
        level,
        proxy_horizon_days,
        target_label_mode: label_mode.as_str().to_string(),
        decision_threshold,
        raw_model,
        calibration,
        evaluation,
    })
}

fn select_actionability_decision_threshold(
    probabilities: &[f64],
    rows: &[ProbabilityTrainingRow],
    horizon_days: u32,
) -> f64 {
    let mut thresholds = probabilities
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .filter(|value| (0.01..0.99).contains(value))
        .collect::<Vec<_>>();
    thresholds.extend((5..=60).map(|value| value as f64 / 100.0));
    thresholds.push(0.3);
    thresholds.sort_by(f64::total_cmp);
    thresholds.dedup_by(|left, right| (*left - *right).abs() < 1e-6);

    let mut best_threshold = 0.3;
    let mut best_score = None::<(bool, bool, bool, u32, u32, i64, i64, i64)>;
    for threshold in thresholds {
        let summary = evaluate_actionability_summary(probabilities, rows, horizon_days, threshold);
        if summary.predicted_positive_count == 0 {
            continue;
        }
        let hit_scenario_count =
            summary.advance_warning_scenario_count + summary.late_confirmation_scenario_count;
        if hit_scenario_count == 0 {
            continue;
        }
        let precision_score =
            (summary.precision_at_threshold.unwrap_or_default() * 1_000.0).round() as i64;
        let false_positive_penalty = -(summary.false_positive_count as i64);
        let threshold_score = (threshold * 1_000.0).round() as i64;
        let meets_precision_floor =
            precision_score >= actionability_precision_floor_score(horizon_days);
        let meets_volume_ceiling = summary.predicted_positive_count
            <= actionability_prediction_count_ceiling(&summary, horizon_days);
        let score = (
            meets_precision_floor && meets_volume_ceiling,
            meets_precision_floor,
            meets_volume_ceiling,
            hit_scenario_count,
            summary.advance_warning_scenario_count,
            precision_score,
            false_positive_penalty,
            threshold_score,
        );
        if best_score.is_none_or(|best| score > best) {
            best_score = Some(score);
            best_threshold = threshold;
        }
    }

    round3(best_threshold).clamp(0.05, 0.60)
}

fn actionability_precision_floor_score(horizon_days: u32) -> i64 {
    match horizon_days {
        5 => 120,
        20 => 100,
        60 => 80,
        _ => 100,
    }
}

fn actionability_prediction_count_ceiling(
    summary: &ActionabilityEvaluationSummary,
    horizon_days: u32,
) -> u32 {
    let multiple = match horizon_days {
        5 => 6_u32,
        20 => 8_u32,
        60 => 10_u32,
        _ => 8_u32,
    };
    summary
        .actual_positive_count
        .max(1)
        .saturating_mul(multiple)
}

fn actionability_bundle_quality_regressions(bundle: &ActionabilityBundle) -> Vec<String> {
    let mut regressions = Vec::new();
    for level in &bundle.levels {
        let Some(summary) = level.evaluation.actionability.as_ref() else {
            regressions.push(format!(
                "{} has no evaluation summary",
                actionability_level_text(level.level)
            ));
            continue;
        };

        let precision_score =
            (summary.precision_at_threshold.unwrap_or_default() * 1_000.0).round() as i64;
        let precision_floor = actionability_precision_floor_score(level.proxy_horizon_days);
        if precision_score < precision_floor {
            regressions.push(format!(
                "{} precision {:.1}% is below required {:.1}%",
                actionability_level_text(level.level),
                precision_score as f64 / 10.0,
                precision_floor as f64 / 10.0
            ));
        }

        let prediction_ceiling =
            actionability_prediction_count_ceiling(summary, level.proxy_horizon_days);
        if summary.predicted_positive_count > prediction_ceiling {
            regressions.push(format!(
                "{} predicted positives {} exceed ceiling {} for {} actual positives",
                actionability_level_text(level.level),
                summary.predicted_positive_count,
                prediction_ceiling,
                summary.actual_positive_count
            ));
        }
    }
    regressions
}

fn select_actionability_calibration_strategy(
    calibration_raw_probabilities: &[f64],
    calibration_rows: &[ProbabilityTrainingRow],
    evaluation_raw_probabilities: &[f64],
    horizon_days: u32,
    calibration_candidate: PlattCalibrationArtifact,
) -> (Option<PlattCalibrationArtifact>, Vec<f64>, f64) {
    let raw_threshold = select_actionability_decision_threshold(
        calibration_raw_probabilities,
        calibration_rows,
        horizon_days,
    );
    let raw_summary = evaluate_actionability_summary(
        calibration_raw_probabilities,
        calibration_rows,
        horizon_days,
        raw_threshold,
    );
    let raw_score = actionability_summary_selection_score(&raw_summary, raw_threshold, horizon_days);

    let calibration_probabilities = calibration_raw_probabilities
        .iter()
        .map(|raw_probability| apply_platt_calibration(*raw_probability, &calibration_candidate))
        .collect::<Vec<_>>();
    let calibrated_threshold = select_actionability_decision_threshold(
        &calibration_probabilities,
        calibration_rows,
        horizon_days,
    );
    let calibrated_summary = evaluate_actionability_summary(
        &calibration_probabilities,
        calibration_rows,
        horizon_days,
        calibrated_threshold,
    );
    let calibrated_score = actionability_summary_selection_score(
        &calibrated_summary,
        calibrated_threshold,
        horizon_days,
    );

    let keep_calibration = calibration_candidate.alpha > 0.0 && calibrated_score > raw_score;
    if keep_calibration {
        let evaluation_probabilities = evaluation_raw_probabilities
            .iter()
            .map(|raw_probability| {
                apply_platt_calibration(*raw_probability, &calibration_candidate)
            })
            .collect::<Vec<_>>();
        (
            Some(calibration_candidate),
            evaluation_probabilities,
            calibrated_threshold,
        )
    } else {
        (None, evaluation_raw_probabilities.to_vec(), raw_threshold)
    }
}

fn actionability_summary_selection_score(
    summary: &ActionabilityEvaluationSummary,
    threshold: f64,
    horizon_days: u32,
) -> (bool, bool, bool, u32, u32, i64, i64, i64) {
    let hit_scenario_count =
        summary.advance_warning_scenario_count + summary.late_confirmation_scenario_count;
    let precision_score =
        (summary.precision_at_threshold.unwrap_or_default() * 1_000.0).round() as i64;
    let false_positive_penalty = -(summary.false_positive_count as i64);
    let threshold_score = (threshold * 1_000.0).round() as i64;
    let meets_precision_floor =
        precision_score >= actionability_precision_floor_score(horizon_days);
    let meets_volume_ceiling = summary.predicted_positive_count
        <= actionability_prediction_count_ceiling(summary, horizon_days);
    (
        meets_precision_floor && meets_volume_ceiling,
        meets_precision_floor,
        meets_volume_ceiling,
        hit_scenario_count,
        summary.advance_warning_scenario_count,
        precision_score,
        false_positive_penalty,
        threshold_score,
    )
}

fn train_horizon_bundle(
    train_rows: &[ProbabilityTrainingRow],
    calibration_rows: &[ProbabilityTrainingRow],
    evaluation_rows: &[ProbabilityTrainingRow],
    feature_names: &[String],
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) -> anyhow::Result<ProbabilityHorizonBundle> {
    ensure_positive_labels(train_rows, horizon_days, "train", label_mode)?;
    ensure_positive_labels(calibration_rows, horizon_days, "calibration", label_mode)?;
    ensure_positive_labels(evaluation_rows, horizon_days, "evaluation", label_mode)?;

    let raw_model = fit_logistic_model(train_rows, feature_names, horizon_days, label_mode);
    let calibration_selection_rows =
        probability_calibration_selection_rows(calibration_rows, horizon_days, label_mode);
    let calibration_inputs = calibration_selection_rows
        .iter()
        .map(|row| score_logistic_model_for_dataset(&raw_model, row))
        .collect::<Vec<_>>();
    let calibration_labels = calibration_selection_rows
        .iter()
        .map(|row| row.label_for_horizon(label_mode, horizon_days))
        .collect::<Vec<_>>();
    let calibration_candidate = fit_platt_calibration(&calibration_inputs, &calibration_labels);
    let evaluation_raw_probabilities = evaluation_rows
        .iter()
        .map(|row| score_logistic_model_for_dataset(&raw_model, row))
        .collect::<Vec<_>>();
    let (calibration, evaluation_probabilities) = select_probability_calibration_strategy(
        &calibration_inputs,
        &calibration_labels,
        &evaluation_raw_probabilities,
        calibration_candidate,
    );
    let calibration_decision_probabilities = calibration.as_ref().map_or_else(
        || calibration_inputs.clone(),
        |calibration| {
            calibration_inputs
                .iter()
                .map(|raw_probability| apply_platt_calibration(*raw_probability, calibration))
                .collect::<Vec<_>>()
        },
    );
    let decision_threshold = select_probability_decision_threshold(
        &calibration_decision_probabilities,
        &calibration_labels,
        horizon_days,
    );
    let evaluation_labels = evaluation_rows
        .iter()
        .map(|row| row.label_for_horizon(label_mode, horizon_days))
        .collect::<Vec<_>>();
    let evaluation = evaluate_probabilities(&evaluation_probabilities, &evaluation_labels);

    Ok(ProbabilityHorizonBundle {
        horizon_days,
        decision_threshold: Some(decision_threshold),
        raw_model,
        calibration,
        evaluation,
    })
}

fn probability_calibration_selection_rows(
    rows: &[ProbabilityTrainingRow],
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) -> Vec<&ProbabilityTrainingRow> {
    let filtered = rows
        .iter()
        .filter(|row| probability_row_is_calibration_eligible(row, horizon_days, label_mode))
        .collect::<Vec<_>>();

    let filtered_positive_count = filtered
        .iter()
        .filter(|row| row.label_for_horizon(label_mode, horizon_days) > 0.0)
        .count();
    let filtered_negative_count = filtered.len().saturating_sub(filtered_positive_count);

    if filtered_positive_count > 0 && filtered_negative_count > 0 {
        filtered
    } else {
        rows.iter().collect()
    }
}

fn probability_row_is_calibration_eligible(
    row: &ProbabilityTrainingRow,
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) -> bool {
    if row.label_for_horizon(label_mode, horizon_days) > 0.0 {
        return true;
    }

    match label_mode {
        ProbabilityTargetLabelMode::ActionWindow => true,
        ProbabilityTargetLabelMode::ForwardCrisis => {
            matches!(
                row.regime_for_horizon(horizon_days),
                ProbabilityTrainingRegime::Normal
            )
        }
    }
}

fn select_probability_calibration_strategy(
    calibration_raw_probabilities: &[f64],
    calibration_labels: &[f64],
    evaluation_raw_probabilities: &[f64],
    calibration_candidate: PlattCalibrationArtifact,
) -> (Option<PlattCalibrationArtifact>, Vec<f64>) {
    let raw_summary = evaluate_probabilities(calibration_raw_probabilities, calibration_labels);
    let raw_score = probability_calibration_selection_score(&raw_summary);

    let calibration_probabilities = calibration_raw_probabilities
        .iter()
        .map(|raw_probability| apply_platt_calibration(*raw_probability, &calibration_candidate))
        .collect::<Vec<_>>();
    let calibrated_summary = evaluate_probabilities(&calibration_probabilities, calibration_labels);
    let calibrated_score = probability_calibration_selection_score(&calibrated_summary);

    let raw_ranking_reversed =
        probability_raw_ranking_is_reversed(calibration_raw_probabilities, calibration_labels);
    let keep_calibration = calibrated_score > raw_score
        && (calibration_candidate.alpha > 0.0
            || (calibration_candidate.alpha < 0.0 && raw_ranking_reversed));
    if keep_calibration {
        let evaluation_probabilities = evaluation_raw_probabilities
            .iter()
            .map(|raw_probability| {
                apply_platt_calibration(*raw_probability, &calibration_candidate)
            })
            .collect::<Vec<_>>();
        (Some(calibration_candidate), evaluation_probabilities)
    } else {
        (None, evaluation_raw_probabilities.to_vec())
    }
}

fn probability_calibration_selection_score(summary: &HorizonEvaluationSummary) -> (i64, i64, i64) {
    (
        -((summary.log_loss * 1_000_000.0).round() as i64),
        -((summary.brier_score * 1_000_000.0).round() as i64),
        -((summary.ece * 1_000_000.0).round() as i64),
    )
}

fn probability_raw_ranking_is_reversed(probabilities: &[f64], labels: &[f64]) -> bool {
    let mut positive_sum = 0.0;
    let mut positive_count = 0_u32;
    let mut negative_sum = 0.0;
    let mut negative_count = 0_u32;

    for (probability, label) in probabilities.iter().zip(labels) {
        if *label >= 0.5 {
            positive_sum += *probability;
            positive_count += 1;
        } else {
            negative_sum += *probability;
            negative_count += 1;
        }
    }

    if positive_count == 0 || negative_count == 0 {
        return false;
    }

    let positive_mean = positive_sum / positive_count as f64;
    let negative_mean = negative_sum / negative_count as f64;
    positive_mean < negative_mean
}

fn select_probability_decision_threshold(
    probabilities: &[f64],
    labels: &[f64],
    horizon_days: u32,
) -> f64 {
    let mut thresholds = probabilities
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .filter(|value| (0.001..0.99).contains(value))
        .collect::<Vec<_>>();
    thresholds.extend((1..=20).map(|value| value as f64 / 1_000.0));
    thresholds.extend((2..=90).map(|value| value as f64 / 100.0));
    thresholds.push(0.3);
    thresholds.sort_by(f64::total_cmp);
    thresholds.dedup_by(|left, right| (*left - *right).abs() < 1e-6);

    let positive_count = labels.iter().filter(|label| **label >= 0.5).count() as f64;
    let mut best_threshold = 0.3;
    let mut best_score = None::<(i64, i64, i64, i64)>;
    for threshold in thresholds {
        let mut true_positive_count = 0_u32;
        let mut predicted_positive_count = 0_u32;
        for (probability, label) in probabilities.iter().zip(labels) {
            if *probability >= threshold {
                predicted_positive_count += 1;
                if *label >= 0.5 {
                    true_positive_count += 1;
                }
            }
        }
        if predicted_positive_count == 0 || positive_count <= 0.0 {
            continue;
        }
        let minimum_true_positives = (positive_count.min(2.0)) as u32;
        if true_positive_count < minimum_true_positives.max(1) {
            continue;
        }
        let precision = true_positive_count as f64 / predicted_positive_count as f64;
        let recall = true_positive_count as f64 / positive_count;
        let beta_sq = 0.25_f64;
        let f05 = if precision > 0.0 || recall > 0.0 {
            (1.0 + beta_sq) * precision * recall / (beta_sq * precision + recall).max(1e-9)
        } else {
            0.0
        };
        let score = (
            (precision * 1_000_000.0).round() as i64,
            (f05 * 1_000_000.0).round() as i64,
            (recall * 1_000_000.0).round() as i64,
            (threshold * 1_000.0).round() as i64,
        );
        if best_score.is_none_or(|best| score > best) {
            best_score = Some(score);
            best_threshold = threshold;
        }
    }

    let minimum_threshold = match horizon_days {
        5 => 0.03,
        20 => 0.005,
        60 => 0.01,
        _ => 0.001,
    };

    round3(best_threshold).clamp(minimum_threshold, 0.90)
}

fn ensure_positive_labels(
    rows: &[ProbabilityTrainingRow],
    horizon_days: u32,
    split_name: &str,
    label_mode: ProbabilityTargetLabelMode,
) -> anyhow::Result<()> {
    let positives = rows
        .iter()
        .filter(|row| row.label_for_horizon(label_mode, horizon_days) > 0.0)
        .count();
    if positives == 0 {
        bail!(
            "no positive {horizon_days}d {} labels found in the {split_name} split",
            label_mode.as_str()
        );
    }
    Ok(())
}

fn fit_logistic_model(
    rows: &[ProbabilityTrainingRow],
    feature_names: &[String],
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) -> LogisticProbabilityModel {
    let feature_stats = feature_names
        .iter()
        .map(|feature| build_feature_stat(rows, feature))
        .collect::<Vec<_>>();
    let positive_class_weight = horizon_positive_class_weight(rows, horizon_days, label_mode);
    let mut intercept = initial_intercept(rows, horizon_days, positive_class_weight, label_mode);
    let mut weights = vec![0.0; feature_names.len()];
    let learning_rate = 0.25;
    let l2 = 0.01;
    let sample_weight_sum = rows
        .iter()
        .map(|row| logistic_sample_weight(row, horizon_days, positive_class_weight, label_mode))
        .sum::<f64>()
        .max(1.0);

    for _ in 0..600 {
        let mut intercept_gradient = 0.0;
        let mut weight_gradients = vec![0.0; weights.len()];
        for row in rows {
            let normalized = normalized_features(row, &feature_stats);
            let prediction = sigmoid(intercept + dot(&weights, &normalized));
            let label = row.label_for_horizon(label_mode, horizon_days);
            let sample_weight =
                logistic_sample_weight(row, horizon_days, positive_class_weight, label_mode);
            let error = (prediction - label) * sample_weight;
            intercept_gradient += error;
            for (index, value) in normalized.iter().enumerate() {
                weight_gradients[index] += error * value;
            }
        }
        intercept -= learning_rate * intercept_gradient / sample_weight_sum;
        for (index, weight) in weights.iter_mut().enumerate() {
            *weight -=
                learning_rate * ((weight_gradients[index] / sample_weight_sum) + l2 * *weight);
        }
    }

    LogisticProbabilityModel {
        intercept,
        feature_stats: feature_stats.clone(),
        coefficients: feature_names
            .iter()
            .zip(weights)
            .map(|(feature, weight)| ProbabilityCoefficient {
                name: feature.clone(),
                weight,
            })
            .collect(),
    }
}

fn build_feature_stat(
    rows: &[ProbabilityTrainingRow],
    feature_name: &str,
) -> ProbabilityFeatureStat {
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

fn initial_intercept(
    rows: &[ProbabilityTrainingRow],
    horizon_days: u32,
    positive_class_weight: f64,
    label_mode: ProbabilityTargetLabelMode,
) -> f64 {
    let weighted_positive = rows
        .iter()
        .map(|row| {
            let label = row.label_for_horizon(label_mode, horizon_days);
            logistic_sample_weight(row, horizon_days, positive_class_weight, label_mode) * label
        })
        .sum::<f64>();
    let weighted_total = rows
        .iter()
        .map(|row| logistic_sample_weight(row, horizon_days, positive_class_weight, label_mode))
        .sum::<f64>()
        .max(1.0);
    let positive_rate = weighted_positive / weighted_total;
    let clipped = positive_rate.clamp(0.01, 0.99);
    (clipped / (1.0 - clipped)).ln()
}

fn horizon_positive_class_weight(
    rows: &[ProbabilityTrainingRow],
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) -> f64 {
    let positive_count = rows
        .iter()
        .filter(|row| row.label_for_horizon(label_mode, horizon_days) > 0.0)
        .count() as f64;
    let negative_weight = rows
        .iter()
        .filter(|row| row.label_for_horizon(label_mode, horizon_days) <= 0.0)
        .map(|row| negative_sample_weight(row, horizon_days, label_mode))
        .sum::<f64>();
    if positive_count <= 0.0 || negative_weight <= 0.0 {
        return 1.0;
    }

    let imbalance_weight = (negative_weight / positive_count).sqrt();
    let (horizon_emphasis, cap) = match label_mode {
        ProbabilityTargetLabelMode::ActionWindow => match horizon_days {
            5 => (0.65, 6.0),
            20 => (0.75, 7.0),
            60 => (0.85, 8.0),
            _ => (0.75, 7.0),
        },
        ProbabilityTargetLabelMode::ForwardCrisis => match horizon_days {
            5 => (0.9, 18.0),
            20 => (1.15, 18.0),
            60 => (1.35, 18.0),
            _ => (1.0, 18.0),
        },
    };
    (imbalance_weight * horizon_emphasis).clamp(1.0, cap)
}

fn logistic_sample_weight(
    row: &ProbabilityTrainingRow,
    horizon_days: u32,
    positive_class_weight: f64,
    label_mode: ProbabilityTargetLabelMode,
) -> f64 {
    let label = row.label_for_horizon(label_mode, horizon_days);
    if label > 0.0 {
        (positive_class_weight * positive_sample_action_weight(row, horizon_days)).clamp(1.0, 36.0)
    } else {
        negative_sample_weight(row, horizon_days, label_mode)
    }
}

fn negative_sample_weight(
    row: &ProbabilityTrainingRow,
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) -> f64 {
    match label_mode {
        ProbabilityTargetLabelMode::ActionWindow => match row.regime_for_horizon(horizon_days) {
            ProbabilityTrainingRegime::Normal => 1.0,
            ProbabilityTrainingRegime::PositiveWindow => 1.0,
            ProbabilityTrainingRegime::PreWarningBuffer => match horizon_days {
                5 => 0.85,
                20 => 0.75,
                60 => 0.65,
                _ => 0.75,
            },
            ProbabilityTrainingRegime::InCrisis => match horizon_days {
                5 => 1.90,
                20 => 1.70,
                60 => 1.45,
                _ => 1.60,
            },
            ProbabilityTrainingRegime::PostCrisisCooldown => match horizon_days {
                5 => 1.60,
                20 => 1.45,
                60 => 1.25,
                _ => 1.35,
            },
        },
        ProbabilityTargetLabelMode::ForwardCrisis => match row.regime_for_horizon(horizon_days) {
            ProbabilityTrainingRegime::Normal => 1.0,
            ProbabilityTrainingRegime::PositiveWindow => 1.0,
            ProbabilityTrainingRegime::PreWarningBuffer => match horizon_days {
                5 => 0.35,
                20 => 0.30,
                60 => 0.25,
                _ => 0.30,
            },
            ProbabilityTrainingRegime::InCrisis => 0.0,
            ProbabilityTrainingRegime::PostCrisisCooldown => match horizon_days {
                5 => 0.12,
                20 => 0.18,
                60 => 0.22,
                _ => 0.20,
            },
        },
    }
}

fn positive_sample_action_weight(row: &ProbabilityTrainingRow, horizon_days: u32) -> f64 {
    let mut weight = 1.0;
    if let Some(lead_days) = row.days_to_primary_crisis_start {
        weight *= lead_time_positive_multiplier(lead_days, horizon_days);
    }
    weight *= horizon_role_weight_multiplier(row, horizon_days);
    weight *= scenario_family_weight_multiplier(row.scenario_family.as_deref(), horizon_days);
    weight.clamp(0.5, 2.75)
}

fn lead_time_positive_multiplier(lead_days: i64, horizon_days: u32) -> f64 {
    if lead_days <= 0 {
        return 1.0;
    }

    let capped = lead_days.min(horizon_days as i64) as f64;
    let horizon = horizon_days.max(1) as f64;
    let normalized = if horizon <= 1.0 {
        0.0
    } else {
        (capped - 1.0) / (horizon - 1.0)
    };
    let max_lift = match horizon_days {
        5 => 0.35,
        20 => 0.45,
        60 => 0.55,
        _ => 0.30,
    };
    1.0 + normalized.clamp(0.0, 1.0) * max_lift
}

fn horizon_role_weight_multiplier(row: &ProbabilityTrainingRow, horizon_days: u32) -> f64 {
    match row.primary_scenario_supports_horizon(horizon_days) {
        Some(true) => 1.25,
        Some(false) => 0.55,
        None => 1.0,
    }
}

fn scenario_family_weight_multiplier(scenario_family: Option<&str>, horizon_days: u32) -> f64 {
    match (horizon_days, scenario_family) {
        (5, Some("acute_market_liquidity_crash")) => 1.35,
        (5, Some("systemic_credit_banking_crisis")) => 0.75,
        (5, Some("mixed_systemic_stress")) => 0.70,
        (5, Some("rate_shock_or_policy_dislocation")) => 0.65,
        (20, Some("acute_market_liquidity_crash")) => 1.15,
        (20, Some("systemic_credit_banking_crisis")) => 1.20,
        (20, Some("mixed_systemic_stress")) => 1.00,
        (20, Some("rate_shock_or_policy_dislocation")) => 0.80,
        (60, Some("acute_market_liquidity_crash")) => 0.65,
        (60, Some("systemic_credit_banking_crisis")) => 1.35,
        (60, Some("mixed_systemic_stress")) => 1.10,
        (60, Some("rate_shock_or_policy_dislocation")) => 0.70,
        _ => 1.0,
    }
}

fn normalized_features(
    row: &ProbabilityTrainingRow,
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
    row: &ProbabilityTrainingRow,
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
        actionability: None,
    }
}

#[derive(Default)]
struct ActionabilityScenarioEvaluationState {
    saw_positive: bool,
    has_pre_start_hit: bool,
    has_post_start_hit: bool,
}

fn evaluate_actionability_summary(
    probabilities: &[f64],
    rows: &[ProbabilityTrainingRow],
    horizon_days: u32,
    threshold: f64,
) -> ActionabilityEvaluationSummary {
    let mut predicted_positive_count = 0_u32;
    let mut actual_positive_count = 0_u32;
    let mut pre_start_positive_count = 0_u32;
    let mut post_start_positive_count = 0_u32;
    let mut unclassified_positive_count = 0_u32;
    let mut pre_start_hit_count = 0_u32;
    let mut post_start_hit_count = 0_u32;
    let mut unclassified_hit_count = 0_u32;
    let mut false_positive_count = 0_u32;
    let mut scenario_states = BTreeMap::<String, ActionabilityScenarioEvaluationState>::new();

    for (probability, row) in probabilities.iter().zip(rows) {
        let predicted_positive = *probability >= threshold;
        let actual_positive =
            row.label_for_horizon(ProbabilityTargetLabelMode::ActionWindow, horizon_days) >= 0.5;
        let lead_days = row.days_to_primary_crisis_start;
        let pre_start = lead_days.is_some_and(|days| days > 0);
        let post_start = lead_days.is_some_and(|days| days <= 0);

        if predicted_positive {
            predicted_positive_count += 1;
        }

        if actual_positive {
            actual_positive_count += 1;
            if pre_start {
                pre_start_positive_count += 1;
            } else if post_start {
                post_start_positive_count += 1;
            } else {
                unclassified_positive_count += 1;
            }

            if let Some(scenario_id) = row.primary_scenario_id.as_ref() {
                let state = scenario_states.entry(scenario_id.clone()).or_default();
                state.saw_positive = true;
                if predicted_positive {
                    if pre_start {
                        state.has_pre_start_hit = true;
                    } else if post_start {
                        state.has_post_start_hit = true;
                    }
                }
            }
        }

        if predicted_positive {
            if actual_positive {
                if pre_start {
                    pre_start_hit_count += 1;
                } else if post_start {
                    post_start_hit_count += 1;
                } else {
                    unclassified_hit_count += 1;
                }
            } else {
                false_positive_count += 1;
            }
        }
    }

    let mut advance_warning_scenario_count = 0_u32;
    let mut late_confirmation_scenario_count = 0_u32;
    let mut missed_scenario_count = 0_u32;
    for state in scenario_states.values().filter(|state| state.saw_positive) {
        if state.has_pre_start_hit {
            advance_warning_scenario_count += 1;
        } else if state.has_post_start_hit {
            late_confirmation_scenario_count += 1;
        } else {
            missed_scenario_count += 1;
        }
    }

    let hit_count = pre_start_hit_count + post_start_hit_count + unclassified_hit_count;
    let scenario_count =
        advance_warning_scenario_count + late_confirmation_scenario_count + missed_scenario_count;

    ActionabilityEvaluationSummary {
        threshold: round3(threshold),
        predicted_positive_count,
        actual_positive_count,
        pre_start_positive_count,
        post_start_positive_count,
        unclassified_positive_count,
        pre_start_hit_count,
        post_start_hit_count,
        unclassified_hit_count,
        false_positive_count,
        scenario_count,
        advance_warning_scenario_count,
        late_confirmation_scenario_count,
        missed_scenario_count,
        precision_at_threshold: (predicted_positive_count > 0)
            .then_some(round3(hit_count as f64 / predicted_positive_count as f64)),
        pre_start_recall_at_threshold: (pre_start_positive_count > 0)
            .then_some(round3(pre_start_hit_count as f64 / pre_start_positive_count as f64)),
        post_start_recall_at_threshold: (post_start_positive_count > 0)
            .then_some(round3(post_start_hit_count as f64 / post_start_positive_count as f64)),
        advance_warning_rate: (scenario_count > 0)
            .then_some(round3(
                advance_warning_scenario_count as f64 / scenario_count as f64,
            )),
        late_confirmation_rate: (scenario_count > 0)
            .then_some(round3(
                late_confirmation_scenario_count as f64 / scenario_count as f64,
            )),
        missed_rate: (scenario_count > 0)
            .then_some(round3(missed_scenario_count as f64 / scenario_count as f64)),
        note: "Pre-start hit means the action signal fired before crisis_start; post-start hit means it only appeared on or after crisis_start within the bounded action window.".to_string(),
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

async fn fetch_assessment_snapshot_for_guard(
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

fn build_release_actionability_review(
    release: &ModelReleaseRecord,
) -> anyhow::Result<ReleaseActionabilityReview> {
    let bundle = read_probability_bundle(Path::new(&release.manifest.bundle_uri))?;
    let Some(actionability) = bundle.actionability.as_ref() else {
        return Ok(ReleaseActionabilityReview {
            release_id: release.manifest.release_id.clone(),
            enabled: false,
            model_version: None,
            calibration_version: None,
            fusion_policy_version: None,
            levels: Vec::new(),
            guard_regressions: Vec::new(),
            guard_passed: true,
            note: "This release has no independent actionability head; release review only applies runtime guardrails.".to_string(),
        });
    };

    let levels = actionability
        .levels
        .iter()
        .map(|level| {
            let evaluation = level
                .evaluation
                .actionability
                .as_ref()
                .cloned()
                .unwrap_or_default();
            ReleaseActionabilityLevelReview {
                level: level.level,
                proxy_horizon_days: level.proxy_horizon_days,
                sample_count: level.evaluation.sample_count,
                positive_rate: level.evaluation.positive_rate,
                threshold: evaluation.threshold,
                predicted_positive_count: evaluation.predicted_positive_count,
                actual_positive_count: evaluation.actual_positive_count,
                pre_start_positive_count: evaluation.pre_start_positive_count,
                post_start_positive_count: evaluation.post_start_positive_count,
                pre_start_hit_count: evaluation.pre_start_hit_count,
                post_start_hit_count: evaluation.post_start_hit_count,
                false_positive_count: evaluation.false_positive_count,
                scenario_count: evaluation.scenario_count,
                advance_warning_scenario_count: evaluation.advance_warning_scenario_count,
                late_confirmation_scenario_count: evaluation.late_confirmation_scenario_count,
                missed_scenario_count: evaluation.missed_scenario_count,
                precision_at_threshold: evaluation.precision_at_threshold,
                pre_start_recall_at_threshold: evaluation.pre_start_recall_at_threshold,
                post_start_recall_at_threshold: evaluation.post_start_recall_at_threshold,
                advance_warning_rate: evaluation.advance_warning_rate,
                late_confirmation_rate: evaluation.late_confirmation_rate,
                missed_rate: evaluation.missed_rate,
                note: evaluation.note,
            }
        })
        .collect::<Vec<_>>();

    let mut review = ReleaseActionabilityReview {
        release_id: release.manifest.release_id.clone(),
        enabled: true,
        model_version: Some(actionability.model_version.clone()),
        calibration_version: Some(actionability.calibration_version.clone()),
        fusion_policy_version: Some(actionability.fusion_policy_version.clone()),
        levels,
        guard_regressions: Vec::new(),
        guard_passed: true,
        note: actionability.note.clone(),
    };
    review.guard_regressions = compare_actionability_guardrails(&review);
    review.guard_passed = review.guard_regressions.is_empty();
    Ok(review)
}

fn compare_actionability_guardrails(review: &ReleaseActionabilityReview) -> Vec<String> {
    if !review.enabled {
        return Vec::new();
    }

    let mut regressions = Vec::new();
    for level in &review.levels {
        let level_name = actionability_level_text(level.level);
        if level.scenario_count < 2 {
            regressions.push(format!(
                "actionability {level_name} scenario_count is {} (<2), so the evaluation slice is too narrow for go/no-go",
                level.scenario_count
            ));
        }
        if level.actual_positive_count > 0
            && level.pre_start_hit_count == 0
            && level.post_start_hit_count == 0
        {
            regressions.push(format!(
                "actionability {level_name} produced no hits in {} labeled evaluation positives",
                level.actual_positive_count
            ));
        }
    }
    regressions
}

fn compare_operational_guardrails(
    baseline: &AssessmentSnapshot,
    candidate: &AssessmentSnapshot,
) -> Vec<String> {
    let mut regressions = Vec::new();
    let baseline_summary = &baseline.backtest_summary;
    let candidate_summary = &candidate.backtest_summary;
    let baseline_rolling = &baseline_summary.rolling_audit;
    let candidate_rolling = &candidate_summary.rolling_audit;

    if candidate_summary.timely_warning_rate + 0.05 < baseline_summary.timely_warning_rate {
        regressions.push(format!(
            "timely_warning_rate dropped from {:.1}% to {:.1}%",
            baseline_summary.timely_warning_rate * 100.0,
            candidate_summary.timely_warning_rate * 100.0
        ));
    }

    if candidate_rolling.actionable_precision + 0.05 < baseline_rolling.actionable_precision {
        regressions.push(format!(
            "actionable_precision dropped from {:.1}% to {:.1}%",
            baseline_rolling.actionable_precision * 100.0,
            candidate_rolling.actionable_precision * 100.0
        ));
    }

    if candidate_rolling.longest_false_positive_episode_days
        > baseline_rolling.longest_false_positive_episode_days + 7
    {
        regressions.push(format!(
            "longest_false_positive_episode_days increased from {} to {}",
            baseline_rolling.longest_false_positive_episode_days,
            candidate_rolling.longest_false_positive_episode_days
        ));
    }

    regressions
}

fn print_operational_guardrail_summary(
    baseline: &AssessmentSnapshot,
    candidate: &AssessmentSnapshot,
) {
    println!("Operational guard summary:");
    println!(
        "  timely_warning_rate   {} -> {}",
        format_pct(baseline.backtest_summary.timely_warning_rate),
        format_pct(candidate.backtest_summary.timely_warning_rate)
    );
    println!(
        "  actionable_precision  {} -> {}",
        format_pct(baseline.backtest_summary.rolling_audit.actionable_precision),
        format_pct(
            candidate
                .backtest_summary
                .rolling_audit
                .actionable_precision
        )
    );
    println!(
        "  longest_false_positive_episode_days  {} -> {}",
        baseline
            .backtest_summary
            .rolling_audit
            .longest_false_positive_episode_days,
        candidate
            .backtest_summary
            .rolling_audit
            .longest_false_positive_episode_days
    );
}

fn build_release_review_comparison(
    baseline: &AssessmentSnapshot,
    candidate: &AssessmentSnapshot,
) -> ReleaseReviewComparisonSummary {
    ReleaseReviewComparisonSummary {
        timely_warning_rate: scalar_metric(
            baseline.backtest_summary.timely_warning_rate,
            candidate.backtest_summary.timely_warning_rate,
        ),
        actionable_precision: scalar_metric(
            baseline.backtest_summary.rolling_audit.actionable_precision,
            candidate
                .backtest_summary
                .rolling_audit
                .actionable_precision,
        ),
        longest_false_positive_episode_days: count_metric(
            baseline
                .backtest_summary
                .rolling_audit
                .longest_false_positive_episode_days,
            candidate
                .backtest_summary
                .rolling_audit
                .longest_false_positive_episode_days,
        ),
        current_p_5d: scalar_metric(baseline.probabilities.p_5d, candidate.probabilities.p_5d),
        current_p_20d: scalar_metric(baseline.probabilities.p_20d, candidate.probabilities.p_20d),
        current_p_60d: scalar_metric(baseline.probabilities.p_60d, candidate.probabilities.p_60d),
    }
}

fn scalar_metric(baseline: f64, candidate: f64) -> ReleaseReviewScalarMetric {
    ReleaseReviewScalarMetric {
        baseline,
        candidate,
        delta: candidate - baseline,
    }
}

fn count_metric(baseline: u32, candidate: u32) -> ReleaseReviewCountMetric {
    ReleaseReviewCountMetric {
        baseline,
        candidate,
        delta: i64::from(candidate) - i64::from(baseline),
    }
}

fn build_release_review_recommendation(
    regressions: &[String],
    candidate_has_actionability: bool,
) -> String {
    if regressions.is_empty() {
        if candidate_has_actionability {
            "候选版通过当前运行时与动作层护栏，可进入下一轮人工复核。仍需结合标签质量、样本覆盖和前端解释能力决定是否晋升。".to_string()
        } else {
            "候选版通过当前运行时护栏，可进入下一轮人工复核。仍需结合标签质量、样本覆盖和前端解释能力决定是否晋升。".to_string()
        }
    } else if candidate_has_actionability {
        "候选版未通过当前运行时 / 动作层护栏，不应替代当前默认线上版本。应先修正训练目标、标签口径、样本切分或样本治理，再重新训练复核。".to_string()
    } else {
        "候选版未通过当前运行时护栏，不应替代当前默认线上版本。应先修正训练目标、标签口径或样本治理，再重新训练复核。".to_string()
    }
}

fn write_release_review_report(
    output_dir: &Path,
    report: &ReleaseReviewEnvelope,
) -> anyhow::Result<()> {
    fs::create_dir_all(output_dir)?;
    let stem = format!(
        "{}-{}-vs-{}-release-review",
        report.candidate_assessment.as_of_date,
        report.baseline_release.manifest.release_id,
        report.candidate_release.manifest.release_id
    );
    let json_path = output_dir.join(format!("{stem}.json"));
    let markdown_path = output_dir.join(format!("{stem}.md"));
    fs::write(&json_path, serde_json::to_string_pretty(report)?)?;
    fs::write(&markdown_path, render_release_review_markdown(report))?;
    println!("Release review report exported.");
    println!("  JSON     {}", json_path.display());
    println!("  Markdown {}", markdown_path.display());
    Ok(())
}

fn render_release_review_markdown(report: &ReleaseReviewEnvelope) -> String {
    let mut markdown = String::new();
    let verdict = if report.overall_guard_passed {
        "PASS"
    } else {
        "FAIL"
    };
    let _ = writeln!(markdown, "# Release Review");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "- Reviewed at: {}", report.reviewed_at);
    let _ = writeln!(markdown, "- Market scope: {}", report.market_scope);
    let _ = writeln!(markdown, "- Verdict: {verdict}");
    let _ = writeln!(
        markdown,
        "- Original active release: {}",
        report.original_active_release_id
    );
    let _ = writeln!(
        markdown,
        "- Restored release after review: {}",
        report.restored_release_id
    );
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Releases");
    let _ = writeln!(markdown);
    let _ = writeln!(
        markdown,
        "| Role | Release ID | Prob Mode | PIT | Feature | Label | Status |"
    );
    let _ = writeln!(markdown, "| --- | --- | --- | --- | --- | --- | --- |");
    for (role, release) in [
        ("baseline", &report.baseline_release),
        ("candidate", &report.candidate_release),
    ] {
        let _ = writeln!(
            markdown,
            "| {} | {} | {} | {} | {} | {} | {} |",
            role,
            release.manifest.release_id,
            release.manifest.probability_mode,
            release.manifest.point_in_time_mode,
            release.manifest.feature_set_version,
            release.manifest.label_version,
            release.manifest.status
        );
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Current Runtime Snapshot");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "| Metric | Baseline | Candidate | Delta |");
    let _ = writeln!(markdown, "| --- | --- | --- | --- |");
    let _ = writeln!(
        markdown,
        "| p_5d | {} | {} | {} |",
        format_pct(report.comparison.current_p_5d.baseline),
        format_pct(report.comparison.current_p_5d.candidate),
        format_signed_pct_delta(report.comparison.current_p_5d.delta)
    );
    let _ = writeln!(
        markdown,
        "| p_20d | {} | {} | {} |",
        format_pct(report.comparison.current_p_20d.baseline),
        format_pct(report.comparison.current_p_20d.candidate),
        format_signed_pct_delta(report.comparison.current_p_20d.delta)
    );
    let _ = writeln!(
        markdown,
        "| p_60d | {} | {} | {} |",
        format_pct(report.comparison.current_p_60d.baseline),
        format_pct(report.comparison.current_p_60d.candidate),
        format_signed_pct_delta(report.comparison.current_p_60d.delta)
    );
    let _ = writeln!(
        markdown,
        "| Posture | {} | {} | — |",
        posture_text(report.baseline_assessment.posture),
        posture_text(report.candidate_assessment.posture)
    );
    let _ = writeln!(
        markdown,
        "| Time bucket | {} | {} | — |",
        time_bucket_text(report.baseline_assessment.time_to_risk_bucket),
        time_bucket_text(report.candidate_assessment.time_to_risk_bucket)
    );
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Backtest Guardrails");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "| Metric | Baseline | Candidate | Delta |");
    let _ = writeln!(markdown, "| --- | --- | --- | --- |");
    let _ = writeln!(
        markdown,
        "| timely_warning_rate | {} | {} | {} |",
        format_pct(report.comparison.timely_warning_rate.baseline),
        format_pct(report.comparison.timely_warning_rate.candidate),
        format_signed_pct_delta(report.comparison.timely_warning_rate.delta)
    );
    let _ = writeln!(
        markdown,
        "| actionable_precision | {} | {} | {} |",
        format_pct(report.comparison.actionable_precision.baseline),
        format_pct(report.comparison.actionable_precision.candidate),
        format_signed_pct_delta(report.comparison.actionable_precision.delta)
    );
    let _ = writeln!(
        markdown,
        "| longest_false_positive_episode_days | {} | {} | {} |",
        report
            .comparison
            .longest_false_positive_episode_days
            .baseline,
        report
            .comparison
            .longest_false_positive_episode_days
            .candidate,
        format_signed_count_delta(report.comparison.longest_false_positive_episode_days.delta)
    );
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Actionability Diagnostics");
    let _ = writeln!(markdown);
    render_release_actionability_review_markdown(
        &mut markdown,
        "baseline",
        &report.baseline_actionability_review,
    );
    let _ = writeln!(markdown);
    render_release_actionability_review_markdown(
        &mut markdown,
        "candidate",
        &report.candidate_actionability_review,
    );
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Guardrail Result");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "### Runtime Guard");
    let _ = writeln!(markdown);
    if report.operational_guard_regressions.is_empty() {
        let _ = writeln!(markdown, "- No runtime guard regressions detected.");
    } else {
        for regression in &report.operational_guard_regressions {
            let _ = writeln!(markdown, "- {regression}");
        }
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "### Actionability Guard");
    let _ = writeln!(markdown);
    if report.actionability_guard_regressions.is_empty() {
        let _ = writeln!(markdown, "- No actionability guard regressions detected.");
    } else {
        for regression in &report.actionability_guard_regressions {
            let _ = writeln!(markdown, "- {regression}");
        }
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "### Overall");
    let _ = writeln!(markdown);
    if report.overall_guard_regressions.is_empty() {
        let _ = writeln!(markdown, "- No combined guard regressions detected.");
    } else {
        for regression in &report.overall_guard_regressions {
            let _ = writeln!(markdown, "- {regression}");
        }
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Recommendation");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "{}", report.recommendation);
    markdown
}

fn render_release_actionability_review_markdown(
    markdown: &mut String,
    role: &str,
    review: &ReleaseActionabilityReview,
) {
    let _ = writeln!(markdown, "### {role} Actionability");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "- Enabled: {}", review.enabled);
    let _ = writeln!(markdown, "- Note: {}", review.note);
    if !review.enabled {
        return;
    }
    let _ = writeln!(
        markdown,
        "- Versions: model={} calib={} fusion={}",
        review.model_version.as_deref().unwrap_or("n/a"),
        review.calibration_version.as_deref().unwrap_or("n/a"),
        review.fusion_policy_version.as_deref().unwrap_or("n/a")
    );
    let _ = writeln!(markdown);
    let _ = writeln!(
        markdown,
        "| Level | Scenarios | Advance Warn | Late Confirm | Missed | Pre-start Recall | Post-start Recall | Precision | Pred+ | Actual+ | FP |"
    );
    let _ = writeln!(
        markdown,
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
    );
    for level in &review.levels {
        let _ = writeln!(
            markdown,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            actionability_level_text(level.level),
            level.scenario_count,
            format_optional_pct(level.advance_warning_rate),
            format_optional_pct(level.late_confirmation_rate),
            format_optional_pct(level.missed_rate),
            format_optional_pct(level.pre_start_recall_at_threshold),
            format_optional_pct(level.post_start_recall_at_threshold),
            format_optional_pct(level.precision_at_threshold),
            level.predicted_positive_count,
            level.actual_positive_count,
            level.false_positive_count
        );
    }
}

fn print_release_review_summary(report: &ReleaseReviewEnvelope) {
    println!("Review comparison:");
    println!(
        "  timely_warning_rate   {} -> {}",
        format_pct(report.comparison.timely_warning_rate.baseline),
        format_pct(report.comparison.timely_warning_rate.candidate)
    );
    println!(
        "  actionable_precision  {} -> {}",
        format_pct(report.comparison.actionable_precision.baseline),
        format_pct(report.comparison.actionable_precision.candidate)
    );
    println!(
        "  longest_false_positive_episode_days  {} -> {}",
        report
            .comparison
            .longest_false_positive_episode_days
            .baseline,
        report
            .comparison
            .longest_false_positive_episode_days
            .candidate
    );
    if report.candidate_actionability_review.enabled {
        println!("Actionability guard summary:");
        for level in &report.candidate_actionability_review.levels {
            println!(
                "  {:>7} scenarios={} advance_warn={} late_confirm={} missed={}",
                actionability_level_text(level.level),
                level.scenario_count,
                format_optional_pct(level.advance_warning_rate),
                format_optional_pct(level.late_confirmation_rate),
                format_optional_pct(level.missed_rate),
            );
        }
    }
    println!("  recommendation        {}", report.recommendation);
}

fn build_formal_dataset_summary(
    dataset_key: &str,
    dataset: FormalDatasetRecord,
    rows: &[FormalDatasetRowRecord],
) -> anyhow::Result<FormalDatasetSummaryEnvelope> {
    let scenarios = load_label_set_crisis_scenarios(
        &dataset.manifest.scenario_set_version,
        &dataset.manifest.label_version,
    )?;
    let scenario_ranges = collect_formal_dataset_scenario_ranges(rows, &scenarios);
    let split_summaries = summarize_formal_dataset_splits(rows, &scenario_ranges);
    let scenario_summaries = summarize_formal_dataset_scenarios(rows, &scenario_ranges);
    let family_summaries = summarize_formal_dataset_families(rows);
    let quality_summaries = summarize_formal_dataset_quality(rows);
    let recommendation = build_formal_dataset_recommendation(&split_summaries, rows.len());

    Ok(FormalDatasetSummaryEnvelope {
        generated_at: Utc::now().to_rfc3339(),
        dataset_key: dataset_key.to_string(),
        dataset,
        split_summaries,
        scenario_summaries,
        family_summaries,
        quality_summaries,
        recommendation,
    })
}

fn summarize_formal_dataset_splits(
    rows: &[FormalDatasetRowRecord],
    scenario_ranges: &[ScenarioRowRange],
) -> Vec<FormalDatasetSplitSummary> {
    ["train", "calibration", "evaluation"]
        .into_iter()
        .filter_map(|split_name| {
            let split_rows = rows
                .iter()
                .filter(|row| row.split_name == split_name)
                .collect::<Vec<_>>();
            let split_start = rows
                .iter()
                .position(|row| row.split_name == split_name)
                .unwrap_or_default();
            let split_end = rows
                .iter()
                .rposition(|row| row.split_name == split_name)
                .map(|index| index + 1)
                .unwrap_or_default();
            (!split_rows.is_empty()).then(|| FormalDatasetSplitSummary {
                split_name: split_name.to_string(),
                row_count: split_rows.len(),
                positive_5d_count: split_rows.iter().filter(|row| row.label_5d > 0).count(),
                positive_5d_rate: round6(forward_label_rate(&split_rows, 5)),
                positive_20d_count: split_rows.iter().filter(|row| row.label_20d > 0).count(),
                positive_20d_rate: round6(forward_label_rate(&split_rows, 20)),
                positive_60d_count: split_rows.iter().filter(|row| row.label_60d > 0).count(),
                positive_60d_rate: round6(forward_label_rate(&split_rows, 60)),
                action_positive_5d_count: split_rows
                    .iter()
                    .filter(|row| row.action_label_5d > 0)
                    .count(),
                action_positive_5d_rate: round6(action_label_rate(&split_rows, 5)),
                action_positive_20d_count: split_rows
                    .iter()
                    .filter(|row| row.action_label_20d > 0)
                    .count(),
                action_positive_20d_rate: round6(action_label_rate(&split_rows, 20)),
                action_positive_60d_count: split_rows
                    .iter()
                    .filter(|row| row.action_label_60d > 0)
                    .count(),
                action_positive_60d_rate: round6(action_label_rate(&split_rows, 60)),
                avg_coverage_score: round3(avg_metric(&split_rows, |row| row.coverage_score)),
                avg_core_feature_coverage: round3(avg_metric(&split_rows, |row| {
                    row.core_feature_coverage
                })),
                avg_trigger_feature_coverage: round3(avg_metric(&split_rows, |row| {
                    row.trigger_feature_coverage
                })),
                avg_external_feature_coverage: round3(avg_metric(&split_rows, |row| {
                    row.external_feature_coverage
                })),
                scenario_count: scenario_count_for_split_range(
                    scenario_ranges,
                    split_start,
                    split_end,
                ),
            })
        })
        .collect()
}

fn summarize_formal_dataset_scenarios(
    rows: &[FormalDatasetRowRecord],
    scenario_ranges: &[ScenarioRowRange],
) -> Vec<FormalDatasetScenarioSummary> {
    scenario_ranges
        .iter()
        .map(|range| FormalDatasetScenarioSummary {
            family: Some(range.family.clone()),
            split_count: rows[range.start_index..=range.end_index]
                .iter()
                .map(|row| row.split_name.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            row_count: range.end_index.saturating_sub(range.start_index) + 1,
            first_as_of_date: rows[range.start_index].as_of_date,
            last_as_of_date: rows[range.end_index].as_of_date,
            scenario_id: range.scenario_id.clone(),
        })
        .collect()
}

fn summarize_formal_dataset_families(
    rows: &[FormalDatasetRowRecord],
) -> Vec<FormalDatasetFamilySummary> {
    let mut buckets = BTreeMap::<String, Vec<&FormalDatasetRowRecord>>::new();
    for row in rows.iter().filter(|row| row.scenario_family.is_some()) {
        let family = row.scenario_family.clone().unwrap_or_default();
        buckets.entry(family).or_default().push(row);
    }

    buckets
        .into_iter()
        .map(|(family, family_rows)| FormalDatasetFamilySummary {
            row_count: family_rows.len(),
            scenario_count: family_rows
                .iter()
                .filter_map(|row| row.primary_scenario_id.as_ref())
                .collect::<BTreeSet<_>>()
                .len(),
            family,
        })
        .collect()
}

fn summarize_formal_dataset_quality(
    rows: &[FormalDatasetRowRecord],
) -> Vec<FormalDatasetQualitySummary> {
    let mut buckets = BTreeMap::<String, usize>::new();
    for row in rows {
        *buckets.entry(row.sample_quality_grade.clone()).or_default() += 1;
    }
    buckets
        .into_iter()
        .map(|(grade, row_count)| FormalDatasetQualitySummary { grade, row_count })
        .collect()
}

fn build_formal_dataset_recommendation(
    split_summaries: &[FormalDatasetSplitSummary],
    total_rows: usize,
) -> String {
    let evaluation = split_summaries
        .iter()
        .find(|split| split.split_name == "evaluation");
    if total_rows < 5_000 {
        return "样本量仍偏小，先继续补历史数据，再用这版数据集训练正式候选版。".to_string();
    }
    if let Some(evaluation) = evaluation {
        if evaluation.action_positive_20d_count < 10 || evaluation.action_positive_60d_count < 10 {
            return "evaluation 的 action-window 正样本仍偏少，当前更适合作研究版比较，不适合直接给正式模型做上线判断。".to_string();
        }
        if evaluation.scenario_count < 2 {
            return format!(
                "evaluation split 的动作标签目前只覆盖 {} 个场景，动作头评估很不稳；应先扩历史场景或重做 split，再用它判断 formal 候选版优劣。",
                evaluation.scenario_count
            );
        }
        if evaluation.avg_coverage_score < 0.85 {
            return "evaluation 覆盖率偏低，应先补可见性/覆盖率，再看训练结果。".to_string();
        }
    }
    "样本量、split 和覆盖率已具备基础研究条件，可以进入正式训练与 release review。".to_string()
}

fn forward_label_rate(rows: &[&FormalDatasetRowRecord], horizon_days: u32) -> f64 {
    let positives = rows
        .iter()
        .filter(|row| match horizon_days {
            5 => row.label_5d > 0,
            20 => row.label_20d > 0,
            60 => row.label_60d > 0,
            _ => false,
        })
        .count();
    positives as f64 / rows.len() as f64
}

fn action_label_rate(rows: &[&FormalDatasetRowRecord], horizon_days: u32) -> f64 {
    let positives = rows
        .iter()
        .filter(|row| match horizon_days {
            5 => row.action_label_5d > 0,
            20 => row.action_label_20d > 0,
            60 => row.action_label_60d > 0,
            _ => false,
        })
        .count();
    positives as f64 / rows.len() as f64
}

fn avg_metric<F>(rows: &[&FormalDatasetRowRecord], accessor: F) -> f64
where
    F: Fn(&FormalDatasetRowRecord) -> f64,
{
    rows.iter().map(|row| accessor(row)).sum::<f64>() / rows.len() as f64
}

fn write_formal_dataset_summary_report(
    output_dir: &Path,
    summary: &FormalDatasetSummaryEnvelope,
) -> anyhow::Result<()> {
    fs::create_dir_all(output_dir)?;
    let stem = format!(
        "{}-{}-formal-dataset-summary",
        summary.dataset.manifest.dataset_id, summary.dataset.manifest.dataset_version
    );
    let json_path = output_dir.join(format!("{stem}.json"));
    let markdown_path = output_dir.join(format!("{stem}.md"));
    fs::write(&json_path, serde_json::to_string_pretty(summary)?)?;
    fs::write(
        &markdown_path,
        render_formal_dataset_summary_markdown(summary),
    )?;
    println!("Formal dataset summary exported.");
    println!("  JSON     {}", json_path.display());
    println!("  Markdown {}", markdown_path.display());
    Ok(())
}

fn render_formal_dataset_summary_markdown(summary: &FormalDatasetSummaryEnvelope) -> String {
    let mut markdown = String::new();
    let manifest = &summary.dataset.manifest;
    let _ = writeln!(markdown, "# Formal Dataset Summary");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "- Generated at: {}", summary.generated_at);
    let _ = writeln!(markdown, "- Dataset key: {}", summary.dataset_key);
    let _ = writeln!(markdown, "- Market scope: {}", manifest.market_scope);
    let _ = writeln!(markdown, "- Feature set: {}", manifest.feature_set_version);
    let _ = writeln!(markdown, "- Label version: {}", manifest.label_version);
    let _ = writeln!(
        markdown,
        "- Scenario set: {}",
        manifest.scenario_set_version
    );
    let _ = writeln!(markdown, "- PIT mode: {}", manifest.point_in_time_mode);
    let _ = writeln!(markdown, "- Rows: {}", manifest.row_count);
    let _ = writeln!(
        markdown,
        "- Range: {} -> {}",
        manifest
            .from_date
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string()),
        manifest
            .to_date
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string())
    );
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Split Summary");
    let _ = writeln!(markdown);
    let _ = writeln!(
        markdown,
        "| Split | Rows | Forward 5d+ | Forward 20d+ | Forward 60d+ | Action 5d+ | Action 20d+ | Action 60d+ | Avg Coverage | Core | Trigger | External | Scenarios |"
    );
    let _ = writeln!(
        markdown,
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
    );
    for split in &summary.split_summaries {
        let _ = writeln!(
            markdown,
            "| {} | {} | {} ({}) | {} ({}) | {} ({}) | {} ({}) | {} ({}) | {} ({}) | {:.1}% | {:.1}% | {:.1}% | {:.1}% | {} |",
            split.split_name,
            split.row_count,
            split.positive_5d_count,
            format_pct(split.positive_5d_rate),
            split.positive_20d_count,
            format_pct(split.positive_20d_rate),
            split.positive_60d_count,
            format_pct(split.positive_60d_rate),
            split.action_positive_5d_count,
            format_pct(split.action_positive_5d_rate),
            split.action_positive_20d_count,
            format_pct(split.action_positive_20d_rate),
            split.action_positive_60d_count,
            format_pct(split.action_positive_60d_rate),
            split.avg_coverage_score * 100.0,
            split.avg_core_feature_coverage * 100.0,
            split.avg_trigger_feature_coverage * 100.0,
            split.avg_external_feature_coverage * 100.0,
            split.scenario_count
        );
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Scenario Coverage");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "| Scenario | Family | Rows | Splits | Range |");
    let _ = writeln!(markdown, "| --- | --- | --- | --- | --- |");
    for scenario in &summary.scenario_summaries {
        let _ = writeln!(
            markdown,
            "| {} | {} | {} | {} | {} -> {} |",
            scenario.scenario_id,
            scenario.family.as_deref().unwrap_or("-"),
            scenario.row_count,
            scenario.split_count,
            scenario.first_as_of_date,
            scenario.last_as_of_date
        );
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Quality Mix");
    let _ = writeln!(markdown);
    for quality in &summary.quality_summaries {
        let _ = writeln!(
            markdown,
            "- grade {}: {} rows",
            quality.grade, quality.row_count
        );
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Recommendation");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "{}", summary.recommendation);
    markdown
}

fn print_formal_dataset_summary(summary: &FormalDatasetSummaryEnvelope) {
    println!(
        "Formal dataset {} rows={} pit={} feature_set={}",
        summary.dataset_key,
        summary.dataset.manifest.row_count,
        summary.dataset.manifest.point_in_time_mode,
        summary.dataset.manifest.feature_set_version
    );
    for split in &summary.split_summaries {
        println!(
            "  split={} rows={} forward[5d={}({}) 20d={}({}) 60d={}({})] action[5d={}({}) 20d={}({}) 60d={}({})] avg_coverage={:.1}%",
            split.split_name,
            split.row_count,
            split.positive_5d_count,
            format_pct(split.positive_5d_rate),
            split.positive_20d_count,
            format_pct(split.positive_20d_rate),
            split.positive_60d_count,
            format_pct(split.positive_60d_rate),
            split.action_positive_5d_count,
            format_pct(split.action_positive_5d_rate),
            split.action_positive_20d_count,
            format_pct(split.action_positive_20d_rate),
            split.action_positive_60d_count,
            format_pct(split.action_positive_60d_rate),
            split.avg_coverage_score * 100.0
        );
    }
    println!("  recommendation {}", summary.recommendation);
}

fn read_release_manifest(path: &Path) -> anyhow::Result<ModelReleaseManifest> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read release manifest {}", path.display()))?;
    serde_json::from_str::<ModelReleaseManifest>(&raw)
        .with_context(|| format!("failed to decode release manifest {}", path.display()))
}

fn read_probability_bundle(path: &Path) -> anyhow::Result<ProbabilityBundle> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read probability bundle {}", path.display()))?;
    serde_json::from_str::<ProbabilityBundle>(&raw)
        .with_context(|| format!("failed to decode probability bundle {}", path.display()))
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

fn format_optional_pct(value: Option<f64>) -> String {
    value.map(format_pct).unwrap_or_else(|| "—".to_string())
}

fn format_signed_pct_delta(value: f64) -> String {
    format!("{:+.1}pp", value * 100.0)
}

fn format_signed_count_delta(value: i64) -> String {
    format!("{value:+}")
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
        .timeout(std::time::Duration::from_secs(1_200))
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

  cargo run -p fc-worker -- research release publish --manifest FILE [--activate] [--reload-api] [--skip-operational-guard] [--api-reload-url URL] [--updated-by NAME]
      Save a release manifest into SQLite, and optionally activate it and reload the API runtime. With --reload-api, worker compares timely-warning / actionable-precision guardrails and auto-rolls back on clear regression unless --skip-operational-guard is set.

  cargo run -p fc-worker -- research release list [--market-scope SCOPE]
      List model releases stored in SQLite.

  cargo run -p fc-worker -- research release show --release-id ID
      Print a stored model release as JSON.

  cargo run -p fc-worker -- research release activate --release-id ID [--market-scope SCOPE] [--reload-api] [--skip-operational-guard] [--api-reload-url URL] [--updated-by NAME]
      Mark a release active for the selected market scope and optionally reload the API runtime. With --reload-api, worker compares runtime backtest guardrails and auto-rolls back on clear regression unless --skip-operational-guard is set.

  cargo run -p fc-worker -- research release rollback --to-release-id ID [--market-scope SCOPE] [--reload-api] [--api-reload-url URL] [--updated-by NAME]
      Roll back the selected market scope to an earlier release and optionally reload the API runtime.

  cargo run -p fc-worker -- research release review --candidate-release-id ID [--baseline-release-id ID] [--market-scope SCOPE] [--api-reload-url URL] [--output-dir DIR] [--updated-by NAME]
      Temporarily switch the running API between baseline and candidate releases, export a JSON + Markdown comparison report, then restore the original active release.

  cargo run -p fc-worker -- research snapshot list [--market-scope SCOPE] [--release-id ID] [--from YYYY-MM-DD] [--to YYYY-MM-DD] [--limit N]
      List persisted prediction snapshots stored in SQLite for audit and release-review work.

  cargo run -p fc-worker -- research snapshot export [--market-scope SCOPE] [--release-id ID] [--from YYYY-MM-DD] [--to YYYY-MM-DD] [--limit N] [--format json|csv] [--output-path FILE]
      Export persisted prediction snapshots as JSON or CSV for external audit and release review.

  cargo run -p fc-worker -- research snapshot dataset [--market-scope SCOPE] [--release-id ID] [--from YYYY-MM-DD] [--to YYYY-MM-DD] [--format json|csv] [--output-path FILE]
      Build a point-in-time feature + forward-crisis-label dataset from persisted prediction snapshots.

  cargo run -p fc-worker -- research feature build [--market-scope SCOPE] [--from YYYY-MM-DD] [--to YYYY-MM-DD] [--feature-set-version VERSION] [--point-in-time-mode MODE] [--force-rebuild]
      Build raw-observation-backed feature snapshots for the formal model pipeline and persist them into SQLite. Existing snapshots with the same feature_set_version + PIT mode are reused unless --force-rebuild is passed.

  cargo run -p fc-worker -- research feature list [--market-scope SCOPE] [--feature-set-version VERSION] [--from YYYY-MM-DD] [--to YYYY-MM-DD] [--limit N]
      List persisted feature snapshots stored in SQLite.

  cargo run -p fc-worker -- research dataset build-main [--market-scope SCOPE] [--from YYYY-MM-DD] [--to YYYY-MM-DD] [--feature-set-version VERSION] [--point-in-time-mode MODE] [--force-rebuild] [--dataset-id ID] [--dataset-version VERSION] [--label-version VERSION] [--scenario-set-version VERSION]
      Build the formal_v1 main dataset from raw observations -> feature snapshots -> forward crisis labels, then persist the dataset manifest and rows into SQLite. Existing snapshots with the same feature_set_version + PIT mode are reused unless --force-rebuild is passed.

  cargo run -p fc-worker -- research dataset list-main [--market-scope SCOPE] [--dataset-id ID] [--limit N]
      List persisted formal dataset manifests stored in SQLite.

  cargo run -p fc-worker -- research dataset summarize-main [--market-scope SCOPE] [--dataset-id ID] [--dataset-version VERSION] [--dataset-key KEY] [--output-dir DIR]
      Summarize a persisted formal dataset, export JSON + Markdown stats, and show split/scenario/coverage diagnostics before training.

  cargo run -p fc-worker -- research pipeline train-probability [--dataset-source formal|snapshot] [--dataset-id ID] [--dataset-version VERSION] [--dataset-key KEY] [--market-scope SCOPE] [--release-id ID] [--from YYYY-MM-DD] [--to YYYY-MM-DD] [--output-dir DIR] [--release-prefix PREFIX]
      Train a formal probability bundle. By default it uses the latest persisted formal dataset; pass --dataset-source snapshot to fall back to the old prediction-snapshot transitional path.

  cargo run -p fc-worker -- research pipeline bootstrap-formal-release [--dataset-source formal|snapshot] [--dataset-id ID] [--dataset-version VERSION] [--dataset-key KEY] [--market-scope SCOPE] [--release-id ID] [--from YYYY-MM-DD] [--to YYYY-MM-DD] [--output-dir DIR] [--release-prefix PREFIX] [--no-activate] [--no-reload-api] [--skip-operational-guard] [--api-reload-url URL] [--updated-by NAME]
      Train a formal bundle, publish it into SQLite as a model release, optionally activate it, and optionally reload the API runtime. Default source is the latest persisted formal dataset.

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
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    use chrono::{NaiveDate, TimeZone, Utc};
    use fc_domain::{
        ActionabilityBundle, ActionabilityEvaluationSummary, ActionabilityLevelBundle,
        FormalDatasetRowRecord, Frequency, HorizonEvaluationSummary, LogisticProbabilityModel,
        Observation, PlattCalibrationArtifact,
    };

    use super::{
        action_window_label, actionability_bundle_quality_regressions,
        compare_actionability_guardrails, evaluate_actionability_summary,
        fit_platt_calibration, forward_crisis_label, forward_crisis_training_regime,
        negative_sample_weight, observation_is_visible_for_date, positive_sample_action_weight,
        probability_calibration_selection_rows, round3, scenario_aware_formal_split_bounds,
        scenario_count_for_index_range, select_actionability_calibration_strategy,
        select_actionability_decision_threshold, select_probability_calibration_strategy,
        select_probability_decision_threshold, ActionabilityLevel, AuditExportOptions,
        CrisisScenario, FeatureSnapshotBuildOptions, FormalDatasetBuildOptions,
        FormalDatasetSummaryOptions, FormalSplitLabelSupport, PipelineDatasetSource,
        PipelineTrainOptions, PointInTimeMode, PredictionSnapshotQueryOptions,
        ProbabilityTargetLabelMode, ProbabilityTrainingRegime, ProbabilityTrainingRow,
        RefreshLatestOptions, ReleaseActionabilityLevelReview, ReleaseActionabilityReview,
        ReleasePublishOptions, ReleaseReviewOptions, ReleaseSwitchOptions, ScenarioRowRange,
    };

    fn observation(
        source_id: &str,
        frequency: Frequency,
        as_of_date: NaiveDate,
        publication_time: Option<chrono::DateTime<Utc>>,
    ) -> Observation {
        Observation {
            indicator_id: "test_indicator".to_string(),
            entity_id: "us".to_string(),
            as_of_date,
            period_start: Some(as_of_date),
            period_end: Some(as_of_date),
            frequency,
            value: 1.0,
            unit: "value".to_string(),
            source_id: source_id.to_string(),
            dataset_id: "test_dataset".to_string(),
            revision_time: None,
            publication_time,
            quality_score: 90.0,
            quality_flags: Vec::new(),
        }
    }

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
            "--skip-operational-guard".to_string(),
            "--updated-by".to_string(),
            "tester".to_string(),
        ];
        let options = ReleasePublishOptions::parse(&args).unwrap();
        assert!(options.activate);
        assert!(options.reload_api);
        assert!(options.skip_operational_guard);
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
            "--skip-operational-guard".to_string(),
        ];
        let options = ReleaseSwitchOptions::parse(&args).unwrap();
        assert_eq!(options.release_id, "release-123");
        assert_eq!(options.market_scope.as_deref(), Some("financial_system"));
        assert!(options.reload_api);
        assert!(options.skip_operational_guard);
    }

    #[test]
    fn parses_release_review_options() {
        let args = vec![
            "--candidate-release-id".to_string(),
            "candidate-123".to_string(),
            "--baseline-release-id".to_string(),
            "baseline-456".to_string(),
            "--market-scope".to_string(),
            "financial_system".to_string(),
            "--output-dir".to_string(),
            "reports/release-review".to_string(),
        ];
        let options = ReleaseReviewOptions::parse(&args).unwrap();
        assert_eq!(options.candidate_release_id, "candidate-123");
        assert_eq!(options.baseline_release_id.as_deref(), Some("baseline-456"));
        assert_eq!(options.market_scope.as_deref(), Some("financial_system"));
        assert_eq!(options.output_dir, PathBuf::from("reports/release-review"));
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

    #[test]
    fn parses_feature_snapshot_build_options() {
        let args = vec![
            "--market-scope".to_string(),
            "financial_system".to_string(),
            "--from".to_string(),
            "2020-01-01".to_string(),
            "--to".to_string(),
            "2020-12-31".to_string(),
            "--feature-set-version".to_string(),
            "feature_formal_v1_test".to_string(),
        ];
        let options = FeatureSnapshotBuildOptions::parse(&args).unwrap();
        assert_eq!(options.market_scope, "financial_system");
        assert_eq!(
            options.from,
            Some(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap())
        );
        assert_eq!(
            options.to,
            Some(NaiveDate::from_ymd_opt(2020, 12, 31).unwrap())
        );
        assert_eq!(options.feature_set_version, "feature_formal_v1_test");
        assert_eq!(options.point_in_time_mode, "best_effort");
        assert!(!options.force_rebuild);
    }

    #[test]
    fn parses_feature_snapshot_force_rebuild_option() {
        let args = vec!["--force-rebuild".to_string()];
        let options = FeatureSnapshotBuildOptions::parse(&args).unwrap();
        assert!(options.force_rebuild);
    }

    #[test]
    fn parses_formal_dataset_build_options() {
        let args = vec![
            "--market-scope".to_string(),
            "financial_system".to_string(),
            "--dataset-id".to_string(),
            "formal_v1_main_1990_daily".to_string(),
            "--dataset-version".to_string(),
            "20260531T120000".to_string(),
            "--label-version".to_string(),
            "formal_label_v1_main".to_string(),
        ];
        let options = FormalDatasetBuildOptions::parse(&args).unwrap();
        assert_eq!(options.feature.market_scope, "financial_system");
        assert_eq!(options.dataset_id, "formal_v1_main_1990_daily");
        assert_eq!(options.dataset_version.as_deref(), Some("20260531T120000"));
        assert_eq!(options.label_version, "formal_label_v1_main");
    }

    #[test]
    fn parses_formal_dataset_summary_options() {
        let args = vec![
            "--dataset-id".to_string(),
            "formal_v1_main_1990_daily".to_string(),
            "--dataset-version".to_string(),
            "20260531Tpitmain".to_string(),
            "--output-dir".to_string(),
            "reports/formal-dataset".to_string(),
        ];
        let options = FormalDatasetSummaryOptions::parse(&args).unwrap();
        assert_eq!(options.dataset_id, "formal_v1_main_1990_daily");
        assert_eq!(options.dataset_version.as_deref(), Some("20260531Tpitmain"));
        assert_eq!(options.output_dir, PathBuf::from("reports/formal-dataset"));
    }

    #[test]
    fn parses_pipeline_train_defaults_to_formal_dataset() {
        let options = PipelineTrainOptions::parse(&[]).unwrap();
        assert_eq!(options.dataset_source, PipelineDatasetSource::Formal);
        assert_eq!(options.dataset_id, "formal_v1_main_1990_daily");
        assert_eq!(options.dataset_version, None);
        assert_eq!(options.dataset_key, None);
        assert_eq!(options.release_prefix, "us_formal_main");
    }

    #[test]
    fn parses_pipeline_train_snapshot_override() {
        let args = vec![
            "--dataset-source".to_string(),
            "snapshot".to_string(),
            "--release-prefix".to_string(),
            "custom_prefix".to_string(),
            "--market-scope".to_string(),
            "financial_system".to_string(),
        ];
        let options = PipelineTrainOptions::parse(&args).unwrap();
        assert_eq!(options.dataset_source, PipelineDatasetSource::Snapshot);
        assert_eq!(options.release_prefix, "custom_prefix");
        assert_eq!(
            options.query.market_scope.as_deref(),
            Some("financial_system")
        );
    }

    #[test]
    fn best_effort_visibility_uses_release_rule_not_backfill_fetch_time_for_fred() {
        let observation = observation(
            "fred",
            Frequency::Monthly,
            NaiveDate::from_ymd_opt(2020, 1, 31).unwrap(),
            Some(Utc.with_ymd_and_hms(2026, 5, 31, 0, 0, 0).single().unwrap()),
        );

        assert!(!observation_is_visible_for_date(
            &observation,
            NaiveDate::from_ymd_opt(2020, 2, 14).unwrap(),
            PointInTimeMode::BestEffort
        ));
        assert!(observation_is_visible_for_date(
            &observation,
            NaiveDate::from_ymd_opt(2020, 2, 15).unwrap(),
            PointInTimeMode::BestEffort
        ));
    }

    #[test]
    fn strict_visibility_requires_timestamp_to_arrive_before_cutoff() {
        let observation = observation(
            "sec_edgar",
            Frequency::Daily,
            NaiveDate::from_ymd_opt(2020, 1, 2).unwrap(),
            Some(Utc.with_ymd_and_hms(2020, 1, 2, 23, 0, 0).single().unwrap()),
        );

        assert!(!observation_is_visible_for_date(
            &observation,
            NaiveDate::from_ymd_opt(2020, 1, 2).unwrap(),
            PointInTimeMode::Strict
        ));
        assert!(observation_is_visible_for_date(
            &observation,
            NaiveDate::from_ymd_opt(2020, 1, 3).unwrap(),
            PointInTimeMode::Strict
        ));
    }

    #[test]
    fn forward_crisis_label_uses_acute_anchor_for_5d_without_dropping_other_crisis_starts() {
        let acute_only = CrisisScenario {
            scenario_id: "acute".to_string(),
            family: "acute_market_liquidity_crash".to_string(),
            pre_warning_start: NaiveDate::from_ymd_opt(2020, 1, 24).unwrap(),
            crisis_start: NaiveDate::from_ymd_opt(2020, 2, 24).unwrap(),
            acute_start: Some(NaiveDate::from_ymd_opt(2020, 3, 9).unwrap()),
            crisis_end: NaiveDate::from_ymd_opt(2020, 4, 30).unwrap(),
            default_horizon_roles: vec![5, 20],
        };
        let systemic_only = CrisisScenario {
            scenario_id: "systemic".to_string(),
            family: "systemic_credit_banking_crisis".to_string(),
            pre_warning_start: NaiveDate::from_ymd_opt(2023, 2, 1).unwrap(),
            crisis_start: NaiveDate::from_ymd_opt(2023, 3, 8).unwrap(),
            acute_start: Some(NaiveDate::from_ymd_opt(2023, 3, 10).unwrap()),
            crisis_end: NaiveDate::from_ymd_opt(2023, 5, 15).unwrap(),
            default_horizon_roles: vec![20, 60],
        };

        assert_eq!(
            forward_crisis_label(
                NaiveDate::from_ymd_opt(2020, 3, 4).unwrap(),
                &[acute_only.clone(), systemic_only.clone()],
                5,
            ),
            1
        );
        assert_eq!(
            forward_crisis_label(
                NaiveDate::from_ymd_opt(2020, 2, 20).unwrap(),
                &[acute_only.clone(), systemic_only.clone()],
                5,
            ),
            0
        );
        assert_eq!(
            forward_crisis_label(
                NaiveDate::from_ymd_opt(2023, 3, 4).unwrap(),
                &[acute_only.clone(), systemic_only.clone()],
                5,
            ),
            1
        );
        assert_eq!(
            forward_crisis_label(
                NaiveDate::from_ymd_opt(2023, 2, 20).unwrap(),
                &[acute_only, systemic_only],
                20,
            ),
            1
        );
    }

    #[test]
    fn action_window_label_extends_before_crisis_start_and_stays_near_onset() {
        let systemic = CrisisScenario {
            scenario_id: "systemic".to_string(),
            family: "systemic_credit_banking_crisis".to_string(),
            pre_warning_start: NaiveDate::from_ymd_opt(2007, 2, 27).unwrap(),
            crisis_start: NaiveDate::from_ymd_opt(2007, 8, 1).unwrap(),
            acute_start: Some(NaiveDate::from_ymd_opt(2008, 9, 15).unwrap()),
            crisis_end: NaiveDate::from_ymd_opt(2009, 6, 30).unwrap(),
            default_horizon_roles: vec![20, 60],
        };
        let acute = CrisisScenario {
            scenario_id: "acute".to_string(),
            family: "acute_market_liquidity_crash".to_string(),
            pre_warning_start: NaiveDate::from_ymd_opt(2020, 1, 24).unwrap(),
            crisis_start: NaiveDate::from_ymd_opt(2020, 2, 24).unwrap(),
            acute_start: Some(NaiveDate::from_ymd_opt(2020, 3, 9).unwrap()),
            crisis_end: NaiveDate::from_ymd_opt(2020, 4, 30).unwrap(),
            default_horizon_roles: vec![5, 20],
        };

        assert_eq!(
            action_window_label(
                NaiveDate::from_ymd_opt(2007, 5, 10).unwrap(),
                std::slice::from_ref(&systemic),
                60,
            ),
            1
        );
        assert_eq!(
            action_window_label(
                NaiveDate::from_ymd_opt(2020, 2, 28).unwrap(),
                std::slice::from_ref(&acute),
                5,
            ),
            1
        );
        assert_eq!(
            action_window_label(
                NaiveDate::from_ymd_opt(2007, 8, 15).unwrap(),
                std::slice::from_ref(&systemic),
                20,
            ),
            1
        );
        assert_eq!(
            action_window_label(
                NaiveDate::from_ymd_opt(2007, 9, 15).unwrap(),
                std::slice::from_ref(&systemic),
                20,
            ),
            0
        );
    }

    #[test]
    fn forward_crisis_training_regime_marks_buffer_crisis_and_cooldown() {
        let systemic = CrisisScenario {
            scenario_id: "systemic".to_string(),
            family: "systemic_credit_banking_crisis".to_string(),
            pre_warning_start: NaiveDate::from_ymd_opt(2007, 2, 27).unwrap(),
            crisis_start: NaiveDate::from_ymd_opt(2007, 8, 1).unwrap(),
            acute_start: Some(NaiveDate::from_ymd_opt(2008, 9, 15).unwrap()),
            crisis_end: NaiveDate::from_ymd_opt(2009, 6, 30).unwrap(),
            default_horizon_roles: vec![20, 60],
        };

        assert_eq!(
            forward_crisis_training_regime(
                NaiveDate::from_ymd_opt(2007, 5, 10).unwrap(),
                std::slice::from_ref(&systemic),
                60,
            ),
            ProbabilityTrainingRegime::PreWarningBuffer
        );
        assert_eq!(
            forward_crisis_training_regime(
                NaiveDate::from_ymd_opt(2007, 6, 15).unwrap(),
                std::slice::from_ref(&systemic),
                60,
            ),
            ProbabilityTrainingRegime::PositiveWindow
        );
        assert_eq!(
            forward_crisis_training_regime(
                NaiveDate::from_ymd_opt(2008, 10, 1).unwrap(),
                std::slice::from_ref(&systemic),
                20,
            ),
            ProbabilityTrainingRegime::InCrisis
        );
        assert_eq!(
            forward_crisis_training_regime(
                NaiveDate::from_ymd_opt(2009, 7, 20).unwrap(),
                std::slice::from_ref(&systemic),
                20,
            ),
            ProbabilityTrainingRegime::PostCrisisCooldown
        );
        assert_eq!(
            forward_crisis_training_regime(
                NaiveDate::from_ymd_opt(2010, 1, 20).unwrap(),
                std::slice::from_ref(&systemic),
                20,
            ),
            ProbabilityTrainingRegime::Normal
        );
    }

    #[test]
    fn forward_crisis_negative_weights_and_calibration_scope_follow_regime() {
        let positive_row = ProbabilityTrainingRow {
            as_of_date: NaiveDate::from_ymd_opt(2020, 2, 20).unwrap(),
            market_scope: "financial_system".to_string(),
            release_id: None,
            probability_mode: Some("formal_bundle_v1".to_string()),
            freshness_status: Some("a".to_string()),
            time_to_risk_bucket: Some("weeks".to_string()),
            split_name: Some("calibration".to_string()),
            features: BTreeMap::new(),
            primary_scenario_id: Some("scenario_a".to_string()),
            scenario_family: Some("systemic_credit_banking_crisis".to_string()),
            days_to_primary_crisis_start: Some(10),
            primary_scenario_supports_5d: true,
            primary_scenario_supports_20d: true,
            primary_scenario_supports_60d: true,
            label_5d: 0,
            label_20d: 1,
            label_60d: 0,
            regime_5d: ProbabilityTrainingRegime::Normal,
            regime_20d: ProbabilityTrainingRegime::PositiveWindow,
            regime_60d: ProbabilityTrainingRegime::Normal,
            action_label_5d: 0,
            action_label_20d: 1,
            action_label_60d: 0,
        };
        let mut normal_negative = positive_row.clone();
        normal_negative.label_20d = 0;
        normal_negative.regime_20d = ProbabilityTrainingRegime::Normal;
        normal_negative.primary_scenario_id = None;
        normal_negative.scenario_family = None;
        normal_negative.days_to_primary_crisis_start = None;
        let mut buffer_negative = normal_negative.clone();
        buffer_negative.primary_scenario_id = Some("scenario_a".to_string());
        buffer_negative.scenario_family = Some("systemic_credit_banking_crisis".to_string());
        buffer_negative.days_to_primary_crisis_start = Some(28);
        buffer_negative.regime_20d = ProbabilityTrainingRegime::PreWarningBuffer;
        let mut crisis_negative = normal_negative.clone();
        crisis_negative.primary_scenario_id = Some("scenario_a".to_string());
        crisis_negative.scenario_family = Some("systemic_credit_banking_crisis".to_string());
        crisis_negative.days_to_primary_crisis_start = Some(-5);
        crisis_negative.regime_20d = ProbabilityTrainingRegime::InCrisis;
        let mut cooldown_negative = normal_negative.clone();
        cooldown_negative.primary_scenario_id = Some("scenario_a".to_string());
        cooldown_negative.scenario_family =
            Some("systemic_credit_banking_crisis".to_string());
        cooldown_negative.days_to_primary_crisis_start = Some(-35);
        cooldown_negative.regime_20d = ProbabilityTrainingRegime::PostCrisisCooldown;

        assert_eq!(
            negative_sample_weight(
                &normal_negative,
                20,
                ProbabilityTargetLabelMode::ForwardCrisis,
            ),
            1.0
        );
        assert!(
            negative_sample_weight(
                &buffer_negative,
                20,
                ProbabilityTargetLabelMode::ForwardCrisis,
            ) < 1.0
        );
        assert_eq!(
            negative_sample_weight(
                &crisis_negative,
                20,
                ProbabilityTargetLabelMode::ForwardCrisis,
            ),
            0.0
        );
        assert_eq!(
            negative_sample_weight(
                &buffer_negative,
                20,
                ProbabilityTargetLabelMode::ActionWindow,
            ),
            0.75
        );
        assert_eq!(
            negative_sample_weight(
                &crisis_negative,
                20,
                ProbabilityTargetLabelMode::ActionWindow,
            ),
            1.70
        );
        assert_eq!(
            negative_sample_weight(
                &cooldown_negative,
                20,
                ProbabilityTargetLabelMode::ActionWindow,
            ),
            1.45
        );

        let calibration_rows = vec![
            positive_row.clone(),
            normal_negative.clone(),
            buffer_negative.clone(),
            crisis_negative.clone(),
        ];
        let selection = probability_calibration_selection_rows(
            &calibration_rows,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );
        assert_eq!(selection.len(), 2);
        assert_eq!(
            selection
                .iter()
                .filter(|row| row.label_20d > 0
                    || matches!(row.regime_20d, ProbabilityTrainingRegime::Normal))
                .count(),
            2
        );
    }

    #[test]
    fn positive_sample_action_weight_prefers_early_role_aligned_systemic_samples_for_60d() {
        let aligned = ProbabilityTrainingRow {
            as_of_date: NaiveDate::from_ymd_opt(2007, 6, 5).unwrap(),
            market_scope: "financial_system".to_string(),
            release_id: None,
            probability_mode: Some("formal_bundle_v1".to_string()),
            freshness_status: Some("a".to_string()),
            time_to_risk_bucket: Some("us_gfc_2008".to_string()),
            split_name: Some("train".to_string()),
            features: BTreeMap::new(),
            primary_scenario_id: Some("us_gfc_2008".to_string()),
            scenario_family: Some("systemic_credit_banking_crisis".to_string()),
            days_to_primary_crisis_start: Some(57),
            primary_scenario_supports_5d: false,
            primary_scenario_supports_20d: true,
            primary_scenario_supports_60d: true,
            label_5d: 0,
            label_20d: 0,
            label_60d: 1,
            regime_5d: ProbabilityTrainingRegime::Normal,
            regime_20d: ProbabilityTrainingRegime::PreWarningBuffer,
            regime_60d: ProbabilityTrainingRegime::PositiveWindow,
            action_label_5d: 0,
            action_label_20d: 1,
            action_label_60d: 1,
        };
        let misaligned = ProbabilityTrainingRow {
            as_of_date: NaiveDate::from_ymd_opt(2020, 2, 20).unwrap(),
            market_scope: "financial_system".to_string(),
            release_id: None,
            probability_mode: Some("formal_bundle_v1".to_string()),
            freshness_status: Some("a".to_string()),
            time_to_risk_bucket: Some("us_covid_liquidity_2020".to_string()),
            split_name: Some("train".to_string()),
            features: BTreeMap::new(),
            primary_scenario_id: Some("us_covid_liquidity_2020".to_string()),
            scenario_family: Some("acute_market_liquidity_crash".to_string()),
            days_to_primary_crisis_start: Some(4),
            primary_scenario_supports_5d: true,
            primary_scenario_supports_20d: true,
            primary_scenario_supports_60d: false,
            label_5d: 1,
            label_20d: 1,
            label_60d: 1,
            regime_5d: ProbabilityTrainingRegime::PositiveWindow,
            regime_20d: ProbabilityTrainingRegime::PositiveWindow,
            regime_60d: ProbabilityTrainingRegime::PositiveWindow,
            action_label_5d: 1,
            action_label_20d: 1,
            action_label_60d: 0,
        };

        assert!(
            positive_sample_action_weight(&aligned, 60)
                > positive_sample_action_weight(&misaligned, 60)
        );
    }

    #[test]
    fn actionability_summary_distinguishes_advance_late_and_missed_scenarios() {
        let build_row =
            |scenario_id: &str, as_of_date: (i32, u32, u32), lead_days: i64, predicted: bool| {
                let action_label = 1_u8;
                let mut features = BTreeMap::new();
                if predicted {
                    features.insert("predicted".to_string(), 1.0);
                }
                ProbabilityTrainingRow {
                    as_of_date: NaiveDate::from_ymd_opt(as_of_date.0, as_of_date.1, as_of_date.2)
                        .unwrap(),
                    market_scope: "financial_system".to_string(),
                    release_id: None,
                    probability_mode: Some("formal_bundle_v1".to_string()),
                    freshness_status: Some("a".to_string()),
                    time_to_risk_bucket: Some("weeks".to_string()),
                    split_name: Some("evaluation".to_string()),
                    features,
                    primary_scenario_id: Some(scenario_id.to_string()),
                    scenario_family: Some("systemic_credit_banking_crisis".to_string()),
                    days_to_primary_crisis_start: Some(lead_days),
                    primary_scenario_supports_5d: true,
                    primary_scenario_supports_20d: true,
                    primary_scenario_supports_60d: true,
                    label_5d: 0,
                    label_20d: 0,
                    label_60d: 0,
                    regime_5d: ProbabilityTrainingRegime::Normal,
                    regime_20d: if lead_days > 0 {
                        ProbabilityTrainingRegime::PositiveWindow
                    } else {
                        ProbabilityTrainingRegime::InCrisis
                    },
                    regime_60d: ProbabilityTrainingRegime::Normal,
                    action_label_5d: 0,
                    action_label_20d: action_label,
                    action_label_60d: 0,
                }
            };

        let false_positive_row = ProbabilityTrainingRow {
            as_of_date: NaiveDate::from_ymd_opt(2024, 1, 10).unwrap(),
            market_scope: "financial_system".to_string(),
            release_id: None,
            probability_mode: Some("formal_bundle_v1".to_string()),
            freshness_status: Some("a".to_string()),
            time_to_risk_bucket: Some("normal".to_string()),
            split_name: Some("evaluation".to_string()),
            features: BTreeMap::new(),
            primary_scenario_id: None,
            scenario_family: None,
            days_to_primary_crisis_start: None,
            primary_scenario_supports_5d: false,
            primary_scenario_supports_20d: false,
            primary_scenario_supports_60d: false,
            label_5d: 0,
            label_20d: 0,
            label_60d: 0,
            regime_5d: ProbabilityTrainingRegime::Normal,
            regime_20d: ProbabilityTrainingRegime::Normal,
            regime_60d: ProbabilityTrainingRegime::Normal,
            action_label_5d: 0,
            action_label_20d: 0,
            action_label_60d: 0,
        };

        let rows = vec![
            build_row("scenario_a", (2007, 8, 20), 10, true),
            build_row("scenario_a", (2007, 9, 5), -2, true),
            build_row("scenario_b", (2020, 2, 20), 8, false),
            build_row("scenario_b", (2020, 3, 18), -3, true),
            build_row("scenario_c", (2011, 7, 20), 6, false),
            build_row("scenario_c", (2011, 8, 10), -1, false),
            false_positive_row,
        ];
        let probabilities = vec![0.82, 0.61, 0.12, 0.42, 0.18, 0.07, 0.77];

        let summary = evaluate_actionability_summary(&probabilities, &rows, 20, 0.3);

        assert_eq!(summary.predicted_positive_count, 4);
        assert_eq!(summary.actual_positive_count, 6);
        assert_eq!(summary.pre_start_positive_count, 3);
        assert_eq!(summary.post_start_positive_count, 3);
        assert_eq!(summary.pre_start_hit_count, 1);
        assert_eq!(summary.post_start_hit_count, 2);
        assert_eq!(summary.false_positive_count, 1);
        assert_eq!(summary.scenario_count, 3);
        assert_eq!(summary.advance_warning_scenario_count, 1);
        assert_eq!(summary.late_confirmation_scenario_count, 1);
        assert_eq!(summary.missed_scenario_count, 1);
        assert_eq!(summary.precision_at_threshold, Some(0.75));
        assert_eq!(
            summary.pre_start_recall_at_threshold,
            Some(round3(1.0 / 3.0))
        );
        assert_eq!(
            summary.post_start_recall_at_threshold,
            Some(round3(2.0 / 3.0))
        );
        assert_eq!(summary.advance_warning_rate, Some(round3(1.0 / 3.0)));
        assert_eq!(summary.late_confirmation_rate, Some(round3(1.0 / 3.0)));
        assert_eq!(summary.missed_rate, Some(round3(1.0 / 3.0)));
    }

    #[test]
    fn actionability_threshold_selection_avoids_zero_hit_fixed_cutoff() {
        let build_row = |scenario_id: &str, lead_days: i64| ProbabilityTrainingRow {
            as_of_date: NaiveDate::from_ymd_opt(2020, 3, 1).unwrap(),
            market_scope: "financial_system".to_string(),
            release_id: None,
            probability_mode: Some("formal_bundle_v1".to_string()),
            freshness_status: Some("a".to_string()),
            time_to_risk_bucket: Some("evaluation".to_string()),
            split_name: Some("calibration".to_string()),
            features: BTreeMap::new(),
            primary_scenario_id: Some(scenario_id.to_string()),
            scenario_family: Some("systemic_credit_banking_crisis".to_string()),
            days_to_primary_crisis_start: Some(lead_days),
            primary_scenario_supports_5d: true,
            primary_scenario_supports_20d: true,
            primary_scenario_supports_60d: true,
            label_5d: 0,
            label_20d: 0,
            label_60d: 0,
            regime_5d: ProbabilityTrainingRegime::Normal,
            regime_20d: ProbabilityTrainingRegime::PreWarningBuffer,
            regime_60d: ProbabilityTrainingRegime::Normal,
            action_label_5d: 0,
            action_label_20d: 1,
            action_label_60d: 0,
        };
        let rows = vec![
            build_row("scenario_a", 8),
            build_row("scenario_a", -2),
            build_row("scenario_b", 10),
            build_row("scenario_b", -1),
        ];
        let probabilities = vec![0.24, 0.18, 0.22, 0.07];

        let threshold = select_actionability_decision_threshold(&probabilities, &rows, 20);
        let summary = evaluate_actionability_summary(&probabilities, &rows, 20, threshold);

        assert!(threshold < 0.3);
        assert!(summary.predicted_positive_count > 0);
        assert!(
            summary.advance_warning_scenario_count + summary.late_confirmation_scenario_count > 0
        );
    }

    #[test]
    fn actionability_threshold_selection_raises_cutoff_when_low_threshold_is_overbroad() {
        let build_positive_row = |scenario_id: &str, lead_days: i64| ProbabilityTrainingRow {
            as_of_date: NaiveDate::from_ymd_opt(2020, 3, 1).unwrap(),
            market_scope: "financial_system".to_string(),
            release_id: None,
            probability_mode: Some("formal_bundle_v1".to_string()),
            freshness_status: Some("a".to_string()),
            time_to_risk_bucket: Some("calibration".to_string()),
            split_name: Some("calibration".to_string()),
            features: BTreeMap::new(),
            primary_scenario_id: Some(scenario_id.to_string()),
            scenario_family: Some("systemic_credit_banking_crisis".to_string()),
            days_to_primary_crisis_start: Some(lead_days),
            primary_scenario_supports_5d: true,
            primary_scenario_supports_20d: true,
            primary_scenario_supports_60d: true,
            label_5d: 0,
            label_20d: 0,
            label_60d: 0,
            regime_5d: ProbabilityTrainingRegime::Normal,
            regime_20d: ProbabilityTrainingRegime::PreWarningBuffer,
            regime_60d: ProbabilityTrainingRegime::Normal,
            action_label_5d: 0,
            action_label_20d: 1,
            action_label_60d: 0,
        };
        let build_false_positive_row = |day: u32| ProbabilityTrainingRow {
            as_of_date: NaiveDate::from_ymd_opt(2020, 4, day).unwrap(),
            market_scope: "financial_system".to_string(),
            release_id: None,
            probability_mode: Some("formal_bundle_v1".to_string()),
            freshness_status: Some("a".to_string()),
            time_to_risk_bucket: Some("normal".to_string()),
            split_name: Some("calibration".to_string()),
            features: BTreeMap::new(),
            primary_scenario_id: None,
            scenario_family: None,
            days_to_primary_crisis_start: None,
            primary_scenario_supports_5d: false,
            primary_scenario_supports_20d: false,
            primary_scenario_supports_60d: false,
            label_5d: 0,
            label_20d: 0,
            label_60d: 0,
            regime_5d: ProbabilityTrainingRegime::Normal,
            regime_20d: ProbabilityTrainingRegime::Normal,
            regime_60d: ProbabilityTrainingRegime::Normal,
            action_label_5d: 0,
            action_label_20d: 0,
            action_label_60d: 0,
        };

        let mut rows = vec![
            build_positive_row("scenario_a", 8),
            build_positive_row("scenario_b", 10),
        ];
        rows.extend((1..=20).map(build_false_positive_row));

        let mut probabilities = vec![0.82, 0.18];
        probabilities.extend(std::iter::repeat_n(0.18, 20));

        let threshold = select_actionability_decision_threshold(&probabilities, &rows, 20);
        let summary = evaluate_actionability_summary(&probabilities, &rows, 20, threshold);

        assert!(threshold > 0.18);
        assert_eq!(summary.false_positive_count, 0);
        assert_eq!(summary.advance_warning_scenario_count, 1);
    }

    #[test]
    fn actionability_bundle_quality_gate_rejects_overbroad_low_precision_levels() {
        let bundle = ActionabilityBundle {
            model_version: "actionability_bundle_test".to_string(),
            calibration_version: "actionability_platt_test".to_string(),
            fusion_policy_version: "fusion_policy_test".to_string(),
            note: "test".to_string(),
            levels: vec![ActionabilityLevelBundle {
                level: ActionabilityLevel::Prepare,
                proxy_horizon_days: 60,
                target_label_mode: "action_window".to_string(),
                decision_threshold: 0.05,
                raw_model: LogisticProbabilityModel {
                    intercept: 0.0,
                    feature_stats: Vec::new(),
                    coefficients: Vec::new(),
                },
                calibration: None,
                evaluation: HorizonEvaluationSummary {
                    sample_count: 2269,
                    positive_rate: 0.033,
                    brier_score: 0.0,
                    log_loss: 0.0,
                    ece: 0.0,
                    precision_at_30pct: None,
                    recall_at_30pct: None,
                    actionability: Some(ActionabilityEvaluationSummary {
                        threshold: 0.05,
                        predicted_positive_count: 1751,
                        actual_positive_count: 77,
                        advance_warning_scenario_count: 1,
                        precision_at_threshold: Some(0.038),
                        ..Default::default()
                    }),
                },
            }],
        };

        let regressions = actionability_bundle_quality_regressions(&bundle);

        assert!(!regressions.is_empty());
        assert!(regressions
            .iter()
            .any(|item| item.contains("precision") || item.contains("predicted positives")));
    }

    #[test]
    fn actionability_calibration_strategy_rejects_inverting_fit() {
        let build_row = |scenario_id: &str, lead_days: i64| ProbabilityTrainingRow {
            as_of_date: NaiveDate::from_ymd_opt(2020, 3, 1).unwrap(),
            market_scope: "financial_system".to_string(),
            release_id: None,
            probability_mode: Some("formal_bundle_v1".to_string()),
            freshness_status: Some("a".to_string()),
            time_to_risk_bucket: Some("calibration".to_string()),
            split_name: Some("calibration".to_string()),
            features: BTreeMap::new(),
            primary_scenario_id: Some(scenario_id.to_string()),
            scenario_family: Some("systemic_credit_banking_crisis".to_string()),
            days_to_primary_crisis_start: Some(lead_days),
            primary_scenario_supports_5d: true,
            primary_scenario_supports_20d: true,
            primary_scenario_supports_60d: true,
            label_5d: 0,
            label_20d: 0,
            label_60d: 0,
            regime_5d: ProbabilityTrainingRegime::Normal,
            regime_20d: ProbabilityTrainingRegime::PreWarningBuffer,
            regime_60d: ProbabilityTrainingRegime::Normal,
            action_label_5d: 0,
            action_label_20d: 1,
            action_label_60d: 0,
        };
        let rows = vec![
            build_row("scenario_a", 8),
            build_row("scenario_a", -2),
            build_row("scenario_b", 10),
            build_row("scenario_b", -1),
        ];
        let raw_probabilities = vec![0.31, 0.28, 0.27, 0.24];
        let calibration_candidate = PlattCalibrationArtifact {
            alpha: -1.2,
            beta: -3.5,
            min_input: 0.24,
            max_input: 0.31,
        };

        let (calibration, evaluation_probabilities, threshold) =
            select_actionability_calibration_strategy(
                &raw_probabilities,
                &rows,
                &raw_probabilities,
                20,
                calibration_candidate,
            );

        assert!(calibration.is_none());
        assert_eq!(evaluation_probabilities, raw_probabilities);
        assert!(threshold >= 0.24);
    }

    #[test]
    fn probability_calibration_strategy_rejects_inverting_fit() {
        let raw_probabilities = vec![0.82, 0.71, 0.24, 0.11];
        let labels = vec![1.0, 1.0, 0.0, 0.0];
        let calibration_candidate = PlattCalibrationArtifact {
            alpha: -1.4,
            beta: -3.0,
            min_input: 0.11,
            max_input: 0.82,
        };

        let (calibration, evaluation_probabilities) = select_probability_calibration_strategy(
            &raw_probabilities,
            &labels,
            &raw_probabilities,
            calibration_candidate,
        );

        assert!(calibration.is_none());
        assert_eq!(evaluation_probabilities, raw_probabilities);
    }

    #[test]
    fn probability_calibration_strategy_keeps_inverting_fit_for_reversed_raw_ranking() {
        let raw_probabilities = vec![0.11, 0.24, 0.71, 0.82];
        let labels = vec![1.0, 1.0, 0.0, 0.0];
        let calibration_candidate = fit_platt_calibration(&raw_probabilities, &labels);
        assert!(calibration_candidate.alpha < 0.0);

        let (calibration, evaluation_probabilities) = select_probability_calibration_strategy(
            &raw_probabilities,
            &labels,
            &raw_probabilities,
            calibration_candidate.clone(),
        );

        assert_eq!(
            calibration.as_ref().map(|artifact| artifact.alpha),
            Some(calibration_candidate.alpha)
        );
        assert_ne!(evaluation_probabilities, raw_probabilities);
        assert!(evaluation_probabilities[0] > evaluation_probabilities[2]);
    }

    #[test]
    fn probability_decision_threshold_prefers_precision_over_low_cutoff_noise() {
        let probabilities = vec![0.82, 0.71, 0.24, 0.11, 0.09, 0.08];
        let labels = vec![1.0, 1.0, 0.0, 0.0, 0.0, 0.0];

        let threshold = select_probability_decision_threshold(&probabilities, &labels, 5);

        assert!(threshold >= 0.71);
    }

    #[test]
    fn probability_decision_threshold_allows_low_calibrated_ranges() {
        let probabilities = vec![0.024, 0.021, 0.018, 0.007, 0.006, 0.005];
        let labels = vec![1.0, 1.0, 1.0, 0.0, 0.0, 0.0];

        let threshold = select_probability_decision_threshold(&probabilities, &labels, 60);

        assert!(threshold < 0.05);
        assert!(threshold >= 0.018);
    }

    #[test]
    fn probability_decision_threshold_can_drop_below_one_percent() {
        let probabilities = vec![0.0086, 0.0082, 0.0079, 0.0034, 0.0028, 0.0021];
        let labels = vec![1.0, 1.0, 1.0, 0.0, 0.0, 0.0];

        let threshold = select_probability_decision_threshold(&probabilities, &labels, 20);

        assert!(threshold < 0.01);
        assert!(threshold >= 0.007);
    }

    #[test]
    fn probability_decision_threshold_keeps_5d_floor_conservative() {
        let probabilities = vec![0.0086, 0.0082, 0.0079, 0.0034, 0.0028, 0.0021];
        let labels = vec![1.0, 1.0, 1.0, 0.0, 0.0, 0.0];

        let threshold = select_probability_decision_threshold(&probabilities, &labels, 5);

        assert_eq!(threshold, 0.03);
    }

    #[test]
    fn actionability_guardrails_flag_narrow_or_zero_hit_reviews() {
        let review = ReleaseActionabilityReview {
            release_id: "candidate".to_string(),
            enabled: true,
            model_version: Some("actionability_bundle_test".to_string()),
            calibration_version: Some("actionability_platt_test".to_string()),
            fusion_policy_version: Some("fusion_policy_test".to_string()),
            levels: vec![ReleaseActionabilityLevelReview {
                level: ActionabilityLevel::Prepare,
                proxy_horizon_days: 60,
                sample_count: 100,
                positive_rate: 0.03,
                threshold: 0.3,
                predicted_positive_count: 0,
                actual_positive_count: 12,
                pre_start_positive_count: 8,
                post_start_positive_count: 4,
                pre_start_hit_count: 0,
                post_start_hit_count: 0,
                false_positive_count: 0,
                scenario_count: 1,
                advance_warning_scenario_count: 0,
                late_confirmation_scenario_count: 0,
                missed_scenario_count: 1,
                precision_at_threshold: None,
                pre_start_recall_at_threshold: Some(0.0),
                post_start_recall_at_threshold: Some(0.0),
                advance_warning_rate: Some(0.0),
                late_confirmation_rate: Some(0.0),
                missed_rate: Some(1.0),
                note: "test".to_string(),
            }],
            guard_regressions: Vec::new(),
            guard_passed: true,
            note: "test".to_string(),
        };

        let regressions = compare_actionability_guardrails(&review);
        assert_eq!(regressions.len(), 2);
        assert!(regressions[0].contains("scenario_count"));
        assert!(regressions[1].contains("produced no hits"));
    }

    #[test]
    fn scenario_aware_split_spreads_adjacent_scenarios_across_calibration_and_evaluation() {
        let mut rows = (0..180)
            .map(|index| {
                let scenario_id = match index {
                    40..=59 => Some("scenario_a"),
                    90..=109 => Some("scenario_b"),
                    140..=159 => Some("scenario_c"),
                    _ => None,
                };
                FormalDatasetRowRecord {
                    dataset_key: "dataset".to_string(),
                    split_name: String::new(),
                    entity_id: "us".to_string(),
                    market_scope: "financial_system".to_string(),
                    as_of_date: NaiveDate::from_ymd_opt(2000, 1, 1)
                        .unwrap()
                        .checked_add_signed(chrono::Duration::days(index as i64))
                        .unwrap(),
                    point_in_time_mode: "best_effort".to_string(),
                    latest_visible_at: None,
                    coverage_score: 1.0,
                    core_feature_coverage: 1.0,
                    trigger_feature_coverage: 1.0,
                    external_feature_coverage: 1.0,
                    sample_quality_grade: "a".to_string(),
                    primary_scenario_id: scenario_id.map(str::to_string),
                    scenario_family: scenario_id
                        .map(|_| "systemic_credit_banking_crisis".to_string()),
                    label_5d: u8::from(matches!(index, 56..=59 | 106..=109 | 156..=159)),
                    label_20d: u8::from(matches!(index, 52..=59 | 102..=109 | 152..=159)),
                    label_60d: u8::from(matches!(index, 44..=59 | 94..=109 | 144..=159)),
                    action_label_5d: u8::from(matches!(index, 55..=59 | 105..=109 | 155..=159)),
                    action_label_20d: u8::from(matches!(index, 50..=59 | 100..=109 | 150..=159)),
                    action_label_60d: u8::from(matches!(index, 42..=59 | 92..=109 | 142..=159)),
                    features: BTreeMap::new(),
                    created_at: Utc::now(),
                }
            })
            .collect::<Vec<_>>();

        let ranges = vec![
            ScenarioRowRange {
                scenario_id: "scenario_a".to_string(),
                family: "systemic_credit_banking_crisis".to_string(),
                start_index: 40,
                end_index: 59,
            },
            ScenarioRowRange {
                scenario_id: "scenario_b".to_string(),
                family: "systemic_credit_banking_crisis".to_string(),
                start_index: 90,
                end_index: 109,
            },
            ScenarioRowRange {
                scenario_id: "scenario_c".to_string(),
                family: "systemic_credit_banking_crisis".to_string(),
                start_index: 140,
                end_index: 159,
            },
        ];

        let (train_end, calibration_end) =
            scenario_aware_formal_split_bounds(&rows, &ranges).unwrap();
        assert!((56..=59).contains(&train_end));
        assert!((106..=109).contains(&calibration_end));

        for (index, row) in rows.iter_mut().enumerate() {
            row.split_name = if index < train_end {
                "train".to_string()
            } else if index < calibration_end {
                "calibration".to_string()
            } else {
                "evaluation".to_string()
            };
        }

        let calibration_scenarios = rows
            .iter()
            .filter(|row| row.split_name == "calibration")
            .filter_map(|row| row.primary_scenario_id.as_deref())
            .collect::<BTreeSet<_>>();
        let evaluation_scenarios = rows
            .iter()
            .filter(|row| row.split_name == "evaluation")
            .filter_map(|row| row.primary_scenario_id.as_deref())
            .collect::<BTreeSet<_>>();

        assert_eq!(calibration_scenarios.len(), 2);
        assert!(calibration_scenarios.contains("scenario_a"));
        assert!(calibration_scenarios.contains("scenario_b"));
        assert_eq!(evaluation_scenarios.len(), 2);
        assert!(evaluation_scenarios.contains("scenario_b"));
        assert!(evaluation_scenarios.contains("scenario_c"));
        assert_eq!(
            scenario_count_for_index_range(&rows, train_end, calibration_end),
            2
        );
        assert_eq!(
            scenario_count_for_index_range(&rows, calibration_end, rows.len()),
            2
        );

        let label_support = FormalSplitLabelSupport::from_rows(&rows);
        assert!(label_support.split_has_required_label_support(0, train_end));
        assert!(label_support.split_has_required_label_support(train_end, calibration_end));
        assert!(label_support.split_has_required_label_support(calibration_end, rows.len()));
    }
}

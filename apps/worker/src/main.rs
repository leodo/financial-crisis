use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    fmt::Write,
    fs,
    path::{Path, PathBuf},
};

mod commands;
mod output_paths;
mod reporting;

use anyhow::{bail, Context};
use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc, Weekday};
use fc_domain::{
    apply_platt_probability_calibration, embedded_protected_stress_window_catalog,
    load_crisis_scenario_catalog, probability_feature_names_for_transform,
    resolve_probability_feature_value, ActionEpisodeTemplateId, ActionabilityBundle,
    ActionabilityEvaluationSummary, ActionabilityLevel, ActionabilityLevelBundle,
    AssessmentHistoryPoint, AssessmentMethodVersions, AssessmentSnapshot, BacktestScenarioSummary,
    CrisisScenarioActionEpisodeOverrides, FeatureSnapshotRecord, FormalDatasetManifest,
    FormalDatasetRecord, FormalDatasetRowRecord, Frequency, HorizonEvaluationSummary, Indicator,
    IndicatorRisk, LogisticProbabilityModel, ModelReleaseManifest, ModelReleaseRecord, Observation,
    PlattCalibrationArtifact, PredictionSnapshotRecord, ProbabilityBundle,
    ProbabilityBundleEvaluation, ProbabilityCoefficient, ProbabilityFeatureStat,
    ProbabilityHorizonBundle,
    ProbabilityThresholdDecisionSummary as ProbabilityThresholdDecisionSummaryWire,
    ProbabilityThresholdDiagnostics as ProbabilityThresholdDiagnosticsWire,
    ProtectedStressWindowCatalog, RegimeSeparationEvaluationSummary, RiskDimension,
    FEATURE_BUCKET_MONTHS_OR_HIGHER, FEATURE_BUCKET_NOW, FEATURE_BUCKET_WEEKS_OR_HIGHER,
    FEATURE_COVERAGE_SCORE, FEATURE_EXTERNAL_SHOCK_SCORE, FEATURE_FRESHNESS_DELAYED_OR_WORSE,
    FEATURE_FRESHNESS_STALE_OR_MISSING, FEATURE_HEURISTIC_P_20D, FEATURE_HEURISTIC_P_5D,
    FEATURE_HEURISTIC_P_60D, FEATURE_OVERALL_SCORE, FORMAL_PROBABILITY_BUNDLE_FEATURES,
    PROBABILITY_FEATURE_TRANSFORM_IDENTITY_V1, PROBABILITY_FEATURE_TRANSFORM_INTERACTION_TAIL_V1,
    PROBABILITY_MODEL_FAMILY_INTERACTION_TAIL_V1, PROBABILITY_MODEL_FAMILY_LINEAR_V1,
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
use output_paths::{
    DEFAULT_FORMAL_DATASET_SUMMARY_OUTPUT_DIR, DEFAULT_PIPELINE_BUNDLE_OUTPUT_DIR,
    DEFAULT_PIPELINE_MANIFEST_OUTPUT_DIR, DEFAULT_RELEASE_REVIEW_OUTPUT_DIR,
};
use reporting::{write_formal_dataset_summary_report, write_release_review_report};
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
const DEFAULT_FORMAL_MAIN_CONTEXT_WINDOW_SET_ID: &str = "protected_stress_windows_v1";
const FEATURE_SNAPSHOT_STATUS_READY: &str = "ready";
const FEATURE_SNAPSHOT_STATUS_COVERAGE_OR_VISIBILITY_FAILED: &str = "coverage_or_visibility_failed";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_target(false).init();

    let args = env::args().skip(1).collect::<Vec<_>>();
    commands::run_from_args(args).await
}

#[derive(Debug, Clone)]
struct AuditExportOptions {
    api_base_url: String,
    output_dir: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ApiReloadHistoryMode {
    Default,
    StrictRebuild,
}

impl ApiReloadHistoryMode {
    fn as_query_value(self) -> Option<&'static str> {
        match self {
            Self::Default => None,
            Self::StrictRebuild => Some("strict_rebuild"),
        }
    }

    fn as_label(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::StrictRebuild => "strict_rebuild",
        }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProbabilityModelShape {
    LinearV1,
    InteractionTailV1,
}

impl ProbabilityModelShape {
    fn parse(value: &str) -> anyhow::Result<Self> {
        match value {
            "linear_v1" => Ok(Self::LinearV1),
            "interaction_tail_v1" => Ok(Self::InteractionTailV1),
            other => bail!("unsupported --model-shape value: {other}"),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::LinearV1 => PROBABILITY_MODEL_FAMILY_LINEAR_V1,
            Self::InteractionTailV1 => PROBABILITY_MODEL_FAMILY_INTERACTION_TAIL_V1,
        }
    }

    fn feature_transform(self) -> &'static str {
        match self {
            Self::LinearV1 => PROBABILITY_FEATURE_TRANSFORM_IDENTITY_V1,
            Self::InteractionTailV1 => PROBABILITY_FEATURE_TRANSFORM_INTERACTION_TAIL_V1,
        }
    }
}

#[derive(Debug, Clone)]
struct PipelineTrainOptions {
    dataset_source: PipelineDatasetSource,
    model_shape: ProbabilityModelShape,
    dataset_id: String,
    dataset_version: Option<String>,
    dataset_key: Option<String>,
    aux_dataset_keys: Vec<String>,
    query: PredictionSnapshotQueryOptions,
    output_dir: PathBuf,
    manifest_dir: PathBuf,
    release_prefix: String,
}

impl PipelineTrainOptions {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut output_dir = PathBuf::from(DEFAULT_PIPELINE_BUNDLE_OUTPUT_DIR);
        let mut manifest_dir = PathBuf::from(DEFAULT_PIPELINE_MANIFEST_OUTPUT_DIR);
        let mut release_prefix = None;
        let mut dataset_source = PipelineDatasetSource::Formal;
        let mut model_shape = ProbabilityModelShape::LinearV1;
        let mut dataset_id = DEFAULT_FORMAL_DATASET_ID.to_string();
        let mut dataset_version = None;
        let mut dataset_key = None;
        let mut aux_dataset_keys = Vec::new();
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
                "--model-shape" => {
                    index += 1;
                    model_shape = ProbabilityModelShape::parse(
                        args.get(index)
                            .with_context(|| "--model-shape requires a value")?,
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
                "--aux-dataset-key" => {
                    index += 1;
                    aux_dataset_keys.push(
                        args.get(index)
                            .with_context(|| "--aux-dataset-key requires a value")?
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
                "--manifest-dir" => {
                    index += 1;
                    manifest_dir = PathBuf::from(
                        args.get(index)
                            .with_context(|| "--manifest-dir requires a path")?,
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

        let release_prefix =
            release_prefix.unwrap_or_else(|| match (dataset_source, model_shape) {
                (PipelineDatasetSource::Formal, ProbabilityModelShape::LinearV1) => {
                    "us_formal_main".to_string()
                }
                (PipelineDatasetSource::Formal, ProbabilityModelShape::InteractionTailV1) => {
                    "us_formal_interaction_tail".to_string()
                }
                (PipelineDatasetSource::Snapshot, ProbabilityModelShape::LinearV1) => {
                    "us_formal_transitional".to_string()
                }
                (PipelineDatasetSource::Snapshot, ProbabilityModelShape::InteractionTailV1) => {
                    "us_formal_transitional_interaction_tail".to_string()
                }
            });

        Ok(Self {
            dataset_source,
            model_shape,
            dataset_id,
            dataset_version,
            dataset_key,
            aux_dataset_keys,
            query: PredictionSnapshotQueryOptions::parse_with_default_limit(&query_args, None)?,
            output_dir,
            manifest_dir,
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
        let mut output_dir = PathBuf::from(DEFAULT_FORMAL_DATASET_SUMMARY_OUTPUT_DIR);
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
    training_role: String,
    pre_warning_start: NaiveDate,
    crisis_start: NaiveDate,
    acute_start: Option<NaiveDate>,
    crisis_end: NaiveDate,
    default_horizon_roles: Vec<u32>,
    protected_window: bool,
    protected_action_levels: Vec<ActionabilityLevel>,
    episode_template_id: ActionEpisodeTemplateId,
    action_episode_overrides: Option<CrisisScenarioActionEpisodeOverrides>,
}

#[derive(Debug, Clone)]
struct FormalDatasetScenarioSets {
    positive_scenarios: Vec<CrisisScenario>,
    context_scenarios: Vec<CrisisScenario>,
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
struct RuntimeThresholdDiagnosticsWire {
    prepare_p60d: f64,
    hedge_p20d: f64,
    defend_p5d: f64,
    severe_now_p20d: f64,
    elevated_weeks_p60d: f64,
    external_prepare_p20d: f64,
    carry_prepare_p60d: f64,
    downgrade_prepare_p60d: f64,
    downgrade_hedge_p20d: f64,
    downgrade_defend_p5d: f64,
    history_runtime_policy_version: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AuditMethodResponse {
    method: AssessmentMethodVersions,
    note: String,
    protected_stress_window_catalog: ProtectedStressWindowCatalog,
    runtime_thresholds: Option<RuntimeThresholdDiagnosticsWire>,
}

#[derive(Debug, Clone, Deserialize)]
struct AuditMethodResponseWire {
    method: AssessmentMethodVersions,
    note: String,
    protected_stress_window_catalog: Option<ProtectedStressWindowCatalog>,
    runtime_thresholds: Option<RuntimeThresholdDiagnosticsWire>,
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
    primary_positive_count: u32,
    late_validation_row_count: u32,
    protected_row_count: u32,
    primary_hit_count: u32,
    late_validation_hit_count: u32,
    protected_hit_count: u32,
    false_positive_count: u32,
    scenario_count: u32,
    on_time_scenario_count: u32,
    late_only_scenario_count: u32,
    missed_scenario_count: u32,
    precision_at_threshold: Option<f64>,
    primary_recall_at_threshold: Option<f64>,
    late_validation_capture_rate: Option<f64>,
    on_time_rate: Option<f64>,
    late_only_rate: Option<f64>,
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
struct ReleaseRuntimeCount {
    name: String,
    count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseRuntimeReviewDiagnostics {
    release_id: String,
    history_point_count: usize,
    posture_distribution: Vec<ReleaseRuntimeCount>,
    time_bucket_distribution: Vec<ReleaseRuntimeCount>,
    posture_trigger_distribution: Vec<ReleaseRuntimeClauseCount>,
    posture_blocker_distribution: Vec<ReleaseRuntimeClauseCount>,
    regime_probability_summaries: Vec<ReleaseRuntimeRegimeProbabilitySummary>,
    regime_separation_summaries: Vec<ReleaseRuntimeSeparationSummary>,
    runtime_thresholds: Option<RuntimeThresholdDiagnosticsWire>,
    points_at_or_above_prepare_p60d: Option<usize>,
    points_at_or_above_hedge_p20d: Option<usize>,
    points_at_or_above_defend_p5d: Option<usize>,
    note: String,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseRuntimeClauseCount {
    posture: String,
    clause: String,
    count: usize,
    share_of_posture: f64,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseRuntimeRegimeProbabilitySummary {
    horizon_days: u32,
    regime: String,
    row_count: usize,
    row_rate: f64,
    avg_raw_probability: f64,
    max_raw_probability: f64,
    avg_probability: f64,
    max_probability: f64,
    raw_lift_vs_normal: Option<f64>,
    calibrated_lift_vs_normal: Option<f64>,
    raw_gap_vs_normal: Option<f64>,
    calibrated_gap_vs_normal: Option<f64>,
    calibration_gap_retention: Option<f64>,
    threshold_hit_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseRuntimeSeparationSummary {
    horizon_days: u32,
    early_warning_regime: String,
    normal_avg_probability: f64,
    pre_warning_buffer_avg_probability: f64,
    positive_window_avg_probability: f64,
    in_crisis_avg_probability: f64,
    post_crisis_cooldown_avg_probability: f64,
    early_warning_raw_lift_vs_normal: Option<f64>,
    early_warning_calibrated_lift_vs_normal: Option<f64>,
    early_warning_gap_retention: Option<f64>,
    positive_window_calibrated_lift_vs_normal: Option<f64>,
    positive_window_gap_vs_normal: Option<f64>,
    in_crisis_raw_lift_vs_normal: Option<f64>,
    in_crisis_calibrated_lift_vs_normal: Option<f64>,
    post_crisis_cooldown_calibrated_lift_vs_normal: Option<f64>,
    post_crisis_cooldown_gap_vs_normal: Option<f64>,
    max_non_normal_calibrated_lift_vs_normal: Option<f64>,
    max_non_normal_threshold_hit_rate: Option<f64>,
    diagnosis: String,
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
    baseline_runtime_review: ReleaseRuntimeReviewDiagnostics,
    candidate_runtime_review: ReleaseRuntimeReviewDiagnostics,
    baseline_actionability_review: ReleaseActionabilityReview,
    candidate_actionability_review: ReleaseActionabilityReview,
    comparison: ReleaseReviewComparisonSummary,
    probability_guard_regressions: Vec<String>,
    probability_guard_passed: bool,
    operational_guard_regressions: Vec<String>,
    operational_guard_passed: bool,
    actionability_guard_regressions: Vec<String>,
    actionability_guard_passed: bool,
    runtime_sanity_regressions: Vec<String>,
    runtime_sanity_passed: bool,
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
    prepare_primary_count: usize,
    prepare_primary_rate: f64,
    hedge_primary_count: usize,
    hedge_primary_rate: f64,
    defend_primary_count: usize,
    defend_primary_rate: f64,
    late_validation_row_count: usize,
    late_validation_row_rate: f64,
    protected_row_count: usize,
    protected_row_rate: f64,
    avg_coverage_score: f64,
    avg_core_feature_coverage: f64,
    avg_trigger_feature_coverage: f64,
    avg_external_feature_coverage: f64,
    scenario_count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct FormalDatasetScenarioSummary {
    scenario_id: String,
    label: Option<String>,
    row_count: usize,
    split_count: usize,
    first_as_of_date: NaiveDate,
    last_as_of_date: NaiveDate,
    family: Option<String>,
    training_role: Option<String>,
    protected_window: Option<bool>,
    episode_template_id: Option<String>,
    default_horizon_roles: Vec<u32>,
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

#[derive(Debug, Clone)]
struct ScenarioSummaryMetadata {
    label: String,
    family: String,
    training_role: String,
    protected_window: bool,
    episode_template_id: String,
    default_horizon_roles: Vec<u32>,
}

#[derive(Debug, Clone, Serialize)]
struct FormalDatasetRegimeSummary {
    split_name: String,
    horizon_days: u32,
    regime: String,
    row_count: usize,
    row_rate: f64,
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
    regime_summaries: Vec<FormalDatasetRegimeSummary>,
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
        runtime_thresholds: method_wire.runtime_thresholds,
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

async fn run_release_review(
    store: &SqliteStore,
    market_scope: &str,
    options: &commands::release::ReleaseReviewOptions,
    original_active: &ModelReleaseRecord,
    baseline_release: &ModelReleaseRecord,
    candidate_release: &ModelReleaseRecord,
) -> anyhow::Result<()> {
    println!(
        "Review baseline={} candidate={} market_scope={market_scope}.",
        baseline_release.manifest.release_id, candidate_release.manifest.release_id
    );

    commands::release::activate_release_for_review(
        store,
        market_scope,
        &baseline_release.manifest.release_id,
        &options.api_reload_url,
        ApiReloadHistoryMode::StrictRebuild,
        &options.updated_by,
        "baseline",
    )
    .await?;
    let baseline_runtime_snapshot =
        fetch_release_review_runtime_snapshot(&options.api_reload_url).await?;

    commands::release::activate_release_for_review(
        store,
        market_scope,
        &candidate_release.manifest.release_id,
        &options.api_reload_url,
        ApiReloadHistoryMode::StrictRebuild,
        &options.updated_by,
        "candidate",
    )
    .await?;
    let candidate_runtime_snapshot =
        fetch_release_review_runtime_snapshot(&options.api_reload_url).await?;

    let baseline_assessment = baseline_runtime_snapshot.assessment;
    let candidate_assessment = candidate_runtime_snapshot.assessment;
    let baseline_runtime_review = build_release_runtime_review_diagnostics(
        &baseline_release.manifest.release_id,
        &baseline_release.manifest.label_version,
        &baseline_runtime_snapshot.method,
        &baseline_runtime_snapshot.history,
    );
    let candidate_runtime_review = build_release_runtime_review_diagnostics(
        &candidate_release.manifest.release_id,
        &candidate_release.manifest.label_version,
        &candidate_runtime_snapshot.method,
        &candidate_runtime_snapshot.history,
    );

    let baseline_actionability_review = build_release_actionability_review(baseline_release)?;
    let candidate_actionability_review = build_release_actionability_review(candidate_release)?;
    let probability_regressions = compare_probability_guardrails(candidate_release)?;
    let candidate_has_actionability = candidate_actionability_review.enabled;
    let operational_regressions =
        compare_operational_guardrails(&baseline_assessment, &candidate_assessment);
    let actionability_regressions =
        compare_actionability_guardrails(&candidate_actionability_review);
    let runtime_sanity_regressions =
        compare_runtime_sanity_guardrails(&baseline_runtime_review, &candidate_runtime_review);
    let mut overall_regressions = operational_regressions.clone();
    overall_regressions.extend(probability_regressions.iter().cloned());
    overall_regressions.extend(actionability_regressions.iter().cloned());
    overall_regressions.extend(runtime_sanity_regressions.iter().cloned());
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
        baseline_runtime_review,
        candidate_runtime_review,
        baseline_actionability_review,
        candidate_actionability_review,
        probability_guard_passed: probability_regressions.is_empty(),
        operational_guard_passed: operational_regressions.is_empty(),
        actionability_guard_passed: actionability_regressions.is_empty(),
        runtime_sanity_passed: runtime_sanity_regressions.is_empty(),
        overall_guard_passed: overall_regressions.is_empty(),
        recommendation: build_release_review_recommendation(
            &overall_regressions,
            candidate_has_actionability,
        ),
        operational_guard_regressions: operational_regressions,
        probability_guard_regressions: probability_regressions,
        actionability_guard_regressions: actionability_regressions,
        runtime_sanity_regressions,
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
    println!("  model_shape      {}", options.model_shape.as_str());
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
        println!(
            "  regime_eval      usable_early_warning_horizons={} insufficient_early_warning_horizons={}",
            summary.usable_early_warning_horizon_count,
            summary.insufficient_early_warning_horizon_count,
        );
        for regime in &summary.regime_separation_summaries {
            println!(
                "  regime_horizon   {:>2}d early={} positive_window={} cooldown={} in_crisis={} diagnosis={}",
                regime.horizon_days,
                regime.early_warning_regime,
                format_optional_multiplier(regime.positive_window_lift_vs_normal),
                format_optional_multiplier(regime.post_crisis_cooldown_lift_vs_normal),
                format_optional_multiplier(regime.in_crisis_lift_vs_normal),
                regime.diagnosis,
            );
        }
    }
    for horizon in &artifacts.bundle.horizons {
        if let Some(diag) = horizon.threshold_diagnostics.as_ref() {
            println!(
                "  threshold_diag   {:>2}d base={:.3} final={:.3} repair={} reason={} selected_rows={} early_rows={}",
                horizon.horizon_days,
                diag.base_threshold,
                diag.final_threshold,
                diag.repair_applied,
                diag.repair_reason,
                diag.selected_row_count,
                diag.base_summary.early_warning_row_count,
            );
            println!(
                "                   base_hits early={}/{} normal={}/{} final_hits early={}/{} normal={}/{}",
                diag.base_summary.early_warning_hit_count,
                diag.base_summary.early_warning_row_count,
                diag.base_summary.normal_hit_count,
                diag.base_summary.normal_row_count,
                diag.final_summary.early_warning_hit_count,
                diag.final_summary.early_warning_row_count,
                diag.final_summary.normal_hit_count,
                diag.final_summary.normal_row_count,
            );
        }
    }
    if let Some(actionability) = artifacts.bundle.actionability.as_ref() {
        for level in &actionability.levels {
            if let Some(summary) = level.evaluation.actionability.as_ref() {
                println!(
                    "  actionability    {:>7} scenarios={} on_time={} late_only={} missed={}",
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
        commands::release::activate_release_with_runtime_guard(
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
            note: "Built from raw observations and point-in-time feature snapshots; persists forward crisis labels, bounded action-window proxy labels, and episode-native prepare/hedge/defend labels so formal training can optimize for earlier executable warnings without losing the original crisis-start reference.".to_string(),
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
    let scenario_sets = load_formal_dataset_scenario_sets(scenario_set_version, label_version)?;
    let positive_scenarios = scenario_sets.positive_scenarios;
    let context_scenarios = scenario_sets.context_scenarios;
    let min_date = formal_dataset_min_date(label_version);
    let mut rows = snapshots
        .iter()
        .filter(|snapshot| snapshot.as_of_date >= min_date)
        .filter(|snapshot| formal_dataset_snapshot_is_usable(snapshot, label_version))
        .map(|snapshot| {
            let primary_scenario =
                primary_scenario_for_date(snapshot.as_of_date, &context_scenarios);
            let dominant_action_episode =
                dominant_action_episode_for_date(snapshot.as_of_date, &context_scenarios);
            let regime_5d = forward_crisis_training_regime_with_context(
                snapshot.as_of_date,
                &positive_scenarios,
                &context_scenarios,
                5,
            );
            let regime_20d = forward_crisis_training_regime_with_context(
                snapshot.as_of_date,
                &positive_scenarios,
                &context_scenarios,
                20,
            );
            let regime_60d = forward_crisis_training_regime_with_context(
                snapshot.as_of_date,
                &positive_scenarios,
                &context_scenarios,
                60,
            );
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
                scenario_family: primary_scenario
                    .as_ref()
                    .map(|scenario| scenario.family.clone()),
                scenario_training_role: primary_scenario
                    .as_ref()
                    .map(|scenario| scenario.training_role.clone()),
                label_5d: forward_crisis_label(snapshot.as_of_date, &positive_scenarios, 5),
                label_20d: forward_crisis_label(snapshot.as_of_date, &positive_scenarios, 20),
                label_60d: forward_crisis_label(snapshot.as_of_date, &positive_scenarios, 60),
                regime_5d: probability_training_regime_name(regime_5d).to_string(),
                regime_20d: probability_training_regime_name(regime_20d).to_string(),
                regime_60d: probability_training_regime_name(regime_60d).to_string(),
                action_label_5d: action_window_label(snapshot.as_of_date, &context_scenarios, 5),
                action_label_20d: action_window_label(snapshot.as_of_date, &context_scenarios, 20),
                action_label_60d: action_window_label(snapshot.as_of_date, &context_scenarios, 60),
                prepare_episode_label: action_episode_label_for_level(
                    snapshot.as_of_date,
                    &context_scenarios,
                    ActionabilityLevel::Prepare,
                ),
                hedge_episode_label: action_episode_label_for_level(
                    snapshot.as_of_date,
                    &context_scenarios,
                    ActionabilityLevel::Hedge,
                ),
                defend_episode_label: action_episode_label_for_level(
                    snapshot.as_of_date,
                    &context_scenarios,
                    ActionabilityLevel::Defend,
                ),
                primary_action_level: dominant_action_episode
                    .as_ref()
                    .filter(|selection| matches!(selection.phase, ActionEpisodePhase::Primary))
                    .map(|selection| actionability_level_text(selection.level).to_string()),
                action_episode_id: dominant_action_episode.as_ref().map(|selection| {
                    format!(
                        "{}:{}",
                        selection.scenario_id,
                        actionability_level_text(selection.level)
                    )
                }),
                action_episode_phase: dominant_action_episode
                    .as_ref()
                    .map(|selection| selection.phase.as_str().to_string())
                    .unwrap_or_else(|| ActionEpisodePhase::Outside.as_str().to_string()),
                protected_action_window: dominant_action_episode
                    .as_ref()
                    .is_some_and(|selection| selection.protected_action_window),
                features: snapshot.features.clone(),
                created_at: Utc::now(),
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| row.as_of_date);
    assign_formal_dataset_splits(&mut rows, &context_scenarios, label_version);
    Ok(rows)
}

fn formal_dataset_min_date(label_version: &str) -> NaiveDate {
    match label_version {
        "formal_label_v1_ext_acute" => NaiveDate::from_ymd_opt(1987, 1, 1).expect("valid date"),
        _ => NaiveDate::from_ymd_opt(1990, 1, 2).expect("valid date"),
    }
}

fn formal_dataset_snapshot_is_usable(
    snapshot: &FeatureSnapshotRecord,
    label_version: &str,
) -> bool {
    match label_version {
        "formal_label_v1_ext_stress" => {
            snapshot.visibility_status == FEATURE_SNAPSHOT_STATUS_READY
                && snapshot.coverage_score >= 0.75
                && snapshot.core_feature_coverage >= 0.85
                && snapshot.trigger_feature_coverage >= 0.80
                && snapshot.external_feature_coverage >= 0.50
                && has_main_dataset_core_features(&snapshot.features)
        }
        "formal_label_v1_ext_acute" => {
            matches!(
                snapshot.visibility_status.as_str(),
                FEATURE_SNAPSHOT_STATUS_READY
                    | FEATURE_SNAPSHOT_STATUS_COVERAGE_OR_VISIBILITY_FAILED
            ) && snapshot.coverage_score >= 0.55
                && snapshot.core_feature_coverage >= 0.60
                && snapshot.trigger_feature_coverage >= 0.50
                && snapshot.external_feature_coverage >= 0.50
                && has_extension_acute_core_features(&snapshot.features)
        }
        _ => {
            snapshot.visibility_status == FEATURE_SNAPSHOT_STATUS_READY
                && snapshot.coverage_score >= 0.85
                && snapshot.core_feature_coverage >= 0.90
                && snapshot.trigger_feature_coverage >= 0.75
                && snapshot.external_feature_coverage >= 0.70
                && has_main_dataset_core_features(&snapshot.features)
        }
    }
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

fn has_extension_acute_core_features(features: &BTreeMap<String, f64>) -> bool {
    [
        "us_curve_10y2y_level",
        "us_baa_10y_spread_level",
        "us_fed_funds_level",
        "us_usdjpy_level",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FormalDatasetSplitProfile {
    Main,
    ExtensionAcute,
    ExtensionStress,
}

#[derive(Debug, Clone, Copy)]
struct FormalDatasetSplitRequirements {
    minimum_scenario_ranges: usize,
    minimum_calibration_scenarios: usize,
    minimum_evaluation_scenarios: usize,
    require_forward_5d: bool,
    require_forward_20d: bool,
    require_forward_60d: bool,
    require_prepare: bool,
    require_hedge: bool,
    require_defend: bool,
}

fn formal_dataset_split_profile(label_version: &str) -> FormalDatasetSplitProfile {
    match label_version {
        "formal_label_v1_ext_acute" => FormalDatasetSplitProfile::ExtensionAcute,
        "formal_label_v1_ext_stress" => FormalDatasetSplitProfile::ExtensionStress,
        _ => FormalDatasetSplitProfile::Main,
    }
}

fn formal_dataset_split_requirements(label_version: &str) -> FormalDatasetSplitRequirements {
    match formal_dataset_split_profile(label_version) {
        FormalDatasetSplitProfile::Main => FormalDatasetSplitRequirements {
            minimum_scenario_ranges: 3,
            minimum_calibration_scenarios: 2,
            minimum_evaluation_scenarios: 2,
            require_forward_5d: true,
            require_forward_20d: true,
            require_forward_60d: true,
            require_prepare: true,
            require_hedge: true,
            require_defend: true,
        },
        FormalDatasetSplitProfile::ExtensionAcute => FormalDatasetSplitRequirements {
            minimum_scenario_ranges: 2,
            minimum_calibration_scenarios: 2,
            minimum_evaluation_scenarios: 1,
            require_forward_5d: true,
            require_forward_20d: true,
            require_forward_60d: false,
            require_prepare: false,
            require_hedge: false,
            require_defend: true,
        },
        FormalDatasetSplitProfile::ExtensionStress => FormalDatasetSplitRequirements {
            minimum_scenario_ranges: 3,
            minimum_calibration_scenarios: 2,
            minimum_evaluation_scenarios: 2,
            require_forward_5d: false,
            require_forward_20d: true,
            require_forward_60d: true,
            require_prepare: true,
            require_hedge: true,
            require_defend: false,
        },
    }
}

fn assign_formal_dataset_splits(
    rows: &mut [FormalDatasetRowRecord],
    scenarios: &[CrisisScenario],
    label_version: &str,
) {
    let ranges = collect_formal_dataset_scenario_ranges(rows, scenarios);
    let split_requirements = formal_dataset_split_requirements(label_version);
    let Ok((train_end, calibration_end)) =
        scenario_aware_formal_split_bounds(rows, &ranges, split_requirements)
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
    split_requirements: FormalDatasetSplitRequirements,
) -> anyhow::Result<(usize, usize)> {
    if ranges.len() < split_requirements.minimum_scenario_ranges {
        bail!(
            "fewer than {} scenario ranges available for scenario-aware split",
            split_requirements.minimum_scenario_ranges
        );
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
                    if calibration_scenario_count < split_requirements.minimum_calibration_scenarios
                        || evaluation_scenario_count
                            < split_requirements.minimum_evaluation_scenarios
                    {
                        continue;
                    }

                    if !label_support.split_has_required_label_support(
                        0,
                        train_end,
                        split_requirements,
                    ) || !label_support.split_has_required_label_support(
                        train_end,
                        calibration_end,
                        split_requirements,
                    ) || !label_support.split_has_required_label_support(
                        calibration_end,
                        rows.len(),
                        split_requirements,
                    ) {
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
                            split_requirements,
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
                "no scenario-aware split preserves multi-scenario calibration/evaluation coverage together with forward/action-episode label support"
            )
        })
}

fn collect_formal_dataset_scenario_ranges(
    rows: &[FormalDatasetRowRecord],
    scenarios: &[CrisisScenario],
) -> Vec<ScenarioRowRange> {
    let family_by_id = scenarios
        .iter()
        .map(|scenario| (scenario.scenario_id.as_str(), scenario.family.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut ranges = BTreeMap::<String, (usize, usize)>::new();
    for (index, row) in rows.iter().enumerate() {
        let Some(scenario_id) = row.primary_scenario_id.as_ref() else {
            continue;
        };
        ranges
            .entry(scenario_id.clone())
            .and_modify(|range| range.1 = index)
            .or_insert((index, index));
    }

    let mut summaries = ranges
        .into_iter()
        .map(|(scenario_id, (start_index, end_index))| ScenarioRowRange {
            family: family_by_id
                .get(scenario_id.as_str())
                .cloned()
                .or_else(|| rows[start_index].scenario_family.clone())
                .unwrap_or_else(|| "unknown".to_string()),
            scenario_id,
            start_index,
            end_index,
        })
        .collect::<Vec<_>>();
    summaries.sort_by_key(|range| range.start_index);
    summaries
}

#[derive(Debug, Clone)]
struct FormalSplitLabelSupport {
    forward_5d: Vec<usize>,
    forward_20d: Vec<usize>,
    forward_60d: Vec<usize>,
    prepare_primary: Vec<usize>,
    hedge_primary: Vec<usize>,
    defend_primary: Vec<usize>,
}

impl FormalSplitLabelSupport {
    fn from_rows(rows: &[FormalDatasetRowRecord]) -> Self {
        let mut support = Self {
            forward_5d: Vec::with_capacity(rows.len() + 1),
            forward_20d: Vec::with_capacity(rows.len() + 1),
            forward_60d: Vec::with_capacity(rows.len() + 1),
            prepare_primary: Vec::with_capacity(rows.len() + 1),
            hedge_primary: Vec::with_capacity(rows.len() + 1),
            defend_primary: Vec::with_capacity(rows.len() + 1),
        };
        support.forward_5d.push(0);
        support.forward_20d.push(0);
        support.forward_60d.push(0);
        support.prepare_primary.push(0);
        support.hedge_primary.push(0);
        support.defend_primary.push(0);
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
            support.prepare_primary.push(
                support.prepare_primary.last().copied().unwrap_or_default()
                    + usize::from(row.prepare_episode_label > 0),
            );
            support.hedge_primary.push(
                support.hedge_primary.last().copied().unwrap_or_default()
                    + usize::from(row.hedge_episode_label > 0),
            );
            support.defend_primary.push(
                support.defend_primary.last().copied().unwrap_or_default()
                    + usize::from(row.defend_episode_label > 0),
            );
        }
        support
    }

    fn split_has_required_label_support(
        &self,
        start: usize,
        end: usize,
        split_requirements: FormalDatasetSplitRequirements,
    ) -> bool {
        end > start
            && (!split_requirements.require_forward_5d
                || self.has_positive(&self.forward_5d, start, end))
            && (!split_requirements.require_forward_20d
                || self.has_positive(&self.forward_20d, start, end))
            && (!split_requirements.require_forward_60d
                || self.has_positive(&self.forward_60d, start, end))
            && (!split_requirements.require_prepare
                || self.has_positive(&self.prepare_primary, start, end))
            && (!split_requirements.require_hedge
                || self.has_positive(&self.hedge_primary, start, end))
            && (!split_requirements.require_defend
                || self.has_positive(&self.defend_primary, start, end))
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
    split_requirements: FormalDatasetSplitRequirements,
) -> usize {
    let mut score = 0;
    if split_requirements.require_prepare {
        score += actionability_positive_scenario_count_for_split_range(
            rows,
            ranges,
            start,
            end,
            ActionabilityLevel::Prepare,
        )
        .min(2);
    }
    if split_requirements.require_hedge {
        score += actionability_positive_scenario_count_for_split_range(
            rows,
            ranges,
            start,
            end,
            ActionabilityLevel::Hedge,
        )
        .min(2);
    }
    if split_requirements.require_defend {
        score += actionability_positive_scenario_count_for_split_range(
            rows,
            ranges,
            start,
            end,
            ActionabilityLevel::Defend,
        )
        .min(2);
    }
    score
}

fn actionability_positive_scenario_count_for_split_range(
    rows: &[FormalDatasetRowRecord],
    ranges: &[ScenarioRowRange],
    start: usize,
    end: usize,
    level: ActionabilityLevel,
) -> usize {
    ranges
        .iter()
        .filter(|range| {
            let overlap_start = start.max(range.start_index);
            let overlap_end = end.min(range.end_index.saturating_add(1));
            overlap_start < overlap_end
                && rows[overlap_start..overlap_end].iter().any(|row| {
                    row.primary_scenario_id.as_deref() == Some(range.scenario_id.as_str())
                        && row_has_action_episode_label(row, level)
                })
        })
        .count()
}

fn row_has_action_episode_label(row: &FormalDatasetRowRecord, level: ActionabilityLevel) -> bool {
    match level {
        ActionabilityLevel::Prepare => row.prepare_episode_label > 0,
        ActionabilityLevel::Hedge => row.hedge_episode_label > 0,
        ActionabilityLevel::Defend => row.defend_episode_label > 0,
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

    load_label_set_crisis_scenarios_from_catalog(&catalog, label_set_id)
}

fn load_formal_dataset_scenario_sets(
    scenario_set_version: &str,
    label_set_id: &str,
) -> anyhow::Result<FormalDatasetScenarioSets> {
    let catalog = load_crisis_scenario_catalog();
    if catalog.catalog_id != scenario_set_version {
        bail!(
            "scenario set version {} is not available in the active catalog {}; set FC_SCENARIO_CATALOG_PATH to another catalog or use {}",
            scenario_set_version,
            catalog.catalog_id,
            catalog.catalog_id
        );
    }

    let positive_scenarios = load_label_set_crisis_scenarios_from_catalog(&catalog, label_set_id)?;
    let mut context_scenarios = positive_scenarios.clone();
    if label_set_id == DEFAULT_FORMAL_LABEL_VERSION {
        let protected_context_scenarios = load_window_set_crisis_scenarios_from_catalog(
            &catalog,
            DEFAULT_FORMAL_MAIN_CONTEXT_WINDOW_SET_ID,
        )?;
        for scenario in protected_context_scenarios {
            if context_scenarios
                .iter()
                .any(|existing| existing.scenario_id == scenario.scenario_id)
            {
                continue;
            }
            context_scenarios.push(scenario);
        }
        context_scenarios.sort_by_key(|scenario| scenario.crisis_start);
    }

    Ok(FormalDatasetScenarioSets {
        positive_scenarios,
        context_scenarios,
    })
}

fn load_label_set_crisis_scenarios_from_catalog(
    catalog: &fc_domain::CrisisScenarioCatalog,
    label_set_id: &str,
) -> anyhow::Result<Vec<CrisisScenario>> {
    let scenarios = catalog
        .scenarios_for_label_set(label_set_id)
        .with_context(|| format!("label set {label_set_id} was not found in scenario catalog"))?;
    Ok(scenarios
        .into_iter()
        .map(crisis_scenario_from_definition)
        .collect())
}

fn load_window_set_crisis_scenarios_from_catalog(
    catalog: &fc_domain::CrisisScenarioCatalog,
    window_set_id: &str,
) -> anyhow::Result<Vec<CrisisScenario>> {
    let scenario_ids = catalog
        .scenario_ids_for_window_set(window_set_id)
        .with_context(|| format!("window set {window_set_id} was not found in scenario catalog"))?;
    let mut scenarios = Vec::with_capacity(scenario_ids.len());
    for scenario_id in scenario_ids {
        let scenario = catalog
            .scenarios
            .iter()
            .find(|scenario| scenario.scenario_id == *scenario_id)
            .with_context(|| {
                format!("window set {window_set_id} references unknown scenario {scenario_id}")
            })?;
        scenarios.push(crisis_scenario_from_definition(scenario));
    }
    Ok(scenarios)
}

fn crisis_scenario_from_definition(
    scenario: &fc_domain::CrisisScenarioDefinition,
) -> CrisisScenario {
    CrisisScenario {
        scenario_id: scenario.scenario_id.clone(),
        family: scenario_family_code(scenario.family).to_string(),
        training_role: scenario_training_role_code(scenario.training_role).to_string(),
        pre_warning_start: scenario.pre_warning_start,
        crisis_start: scenario.crisis_start,
        acute_start: scenario.acute_start,
        crisis_end: scenario.crisis_end,
        default_horizon_roles: scenario.default_horizon_roles.clone(),
        protected_window: scenario.protected_window,
        protected_action_levels: scenario.protected_action_levels.clone(),
        episode_template_id: scenario
            .episode_template_id
            .expect("validated scenario catalog must include episode_template_id"),
        action_episode_overrides: scenario.action_episode_overrides.clone(),
    }
}

fn load_formal_dataset_scenario_metadata(
    scenario_set_version: &str,
) -> anyhow::Result<BTreeMap<String, ScenarioSummaryMetadata>> {
    let catalog = load_crisis_scenario_catalog();
    if catalog.catalog_id != scenario_set_version {
        bail!(
            "scenario set version {} is not available in the active catalog {}; set FC_SCENARIO_CATALOG_PATH to another catalog or use {}",
            scenario_set_version,
            catalog.catalog_id,
            catalog.catalog_id
        );
    }

    Ok(catalog
        .scenarios
        .into_iter()
        .map(|scenario| {
            (
                scenario.scenario_id.clone(),
                ScenarioSummaryMetadata {
                    label: scenario.label,
                    family: scenario_family_code(scenario.family).to_string(),
                    training_role: scenario_training_role_code(scenario.training_role).to_string(),
                    protected_window: scenario.protected_window,
                    episode_template_id: action_episode_template_code(
                        scenario
                            .episode_template_id
                            .expect("validated scenario catalog must include episode_template_id"),
                    )
                    .to_string(),
                    default_horizon_roles: scenario.default_horizon_roles,
                },
            )
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

fn scenario_training_role_code(role: fc_domain::CrisisScenarioTrainingRole) -> &'static str {
    match role {
        fc_domain::CrisisScenarioTrainingRole::Mandatory => "mandatory",
        fc_domain::CrisisScenarioTrainingRole::CandidateOptional => "candidate_optional",
        fc_domain::CrisisScenarioTrainingRole::ExtensionOnly => "extension_only",
        fc_domain::CrisisScenarioTrainingRole::NoPositiveMain => "no_positive_main",
    }
}

fn action_episode_template_code(template: ActionEpisodeTemplateId) -> &'static str {
    match template {
        ActionEpisodeTemplateId::AcuteMarketLiquidityCrash => "acute_market_liquidity_crash",
        ActionEpisodeTemplateId::SystemicCreditBankingCrisis => "systemic_credit_banking_crisis",
        ActionEpisodeTemplateId::MixedSystemicStress => "mixed_systemic_stress",
        ActionEpisodeTemplateId::RateShockOrPolicyDislocation => "rate_shock_or_policy_dislocation",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionEpisodePhase {
    Outside,
    Cooldown,
    LateValidation,
    Primary,
}

impl ActionEpisodePhase {
    fn as_str(self) -> &'static str {
        match self {
            Self::Outside => "outside",
            Self::Cooldown => "cooldown",
            Self::LateValidation => "late_validation",
            Self::Primary => "primary",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct DateRange {
    start: NaiveDate,
    end: NaiveDate,
}

impl DateRange {
    fn new(start: NaiveDate, end: NaiveDate) -> Option<Self> {
        (start <= end).then_some(Self { start, end })
    }

    fn contains(self, as_of_date: NaiveDate) -> bool {
        as_of_date >= self.start && as_of_date <= self.end
    }
}

#[derive(Debug, Clone, Copy)]
struct ActionEpisodeWindow {
    primary: Option<DateRange>,
    late_validation: Option<DateRange>,
    cooldown: Option<DateRange>,
    protected_action_window: bool,
}

#[derive(Debug, Clone)]
struct ActionEpisodeSelection {
    scenario_id: String,
    level: ActionabilityLevel,
    phase: ActionEpisodePhase,
    protected_action_window: bool,
    crisis_start: NaiveDate,
}

fn shift_date(date: NaiveDate, days: i64) -> NaiveDate {
    date.checked_add_signed(Duration::days(days))
        .unwrap_or(date)
}

fn next_day(date: NaiveDate) -> NaiveDate {
    shift_date(date, 1)
}

fn action_level_rank(level: ActionabilityLevel) -> i32 {
    match level {
        ActionabilityLevel::Defend => 0,
        ActionabilityLevel::Hedge => 1,
        ActionabilityLevel::Prepare => 2,
    }
}

fn action_episode_phase_rank(phase: ActionEpisodePhase) -> i32 {
    match phase {
        ActionEpisodePhase::Primary => 0,
        ActionEpisodePhase::LateValidation => 1,
        ActionEpisodePhase::Cooldown => 2,
        ActionEpisodePhase::Outside => 3,
    }
}

fn action_level_proxy_horizon_days(level: ActionabilityLevel) -> u32 {
    match level {
        ActionabilityLevel::Prepare => 60,
        ActionabilityLevel::Hedge => 20,
        ActionabilityLevel::Defend => 5,
    }
}

fn actionability_level_for_proxy_horizon(horizon_days: u32) -> Option<ActionabilityLevel> {
    match horizon_days {
        60 => Some(ActionabilityLevel::Prepare),
        20 => Some(ActionabilityLevel::Hedge),
        5 => Some(ActionabilityLevel::Defend),
        _ => None,
    }
}

fn action_episode_override_for_level(
    overrides: Option<&CrisisScenarioActionEpisodeOverrides>,
    level: ActionabilityLevel,
) -> Option<&fc_domain::ActionEpisodeWindowOverride> {
    let overrides = overrides?;
    match level {
        ActionabilityLevel::Prepare => overrides.prepare.as_ref(),
        ActionabilityLevel::Hedge => overrides.hedge.as_ref(),
        ActionabilityLevel::Defend => overrides.defend.as_ref(),
    }
}

fn action_episode_default_window(
    scenario: &CrisisScenario,
    level: ActionabilityLevel,
) -> ActionEpisodeWindow {
    let acute_start = scenario.acute_start.unwrap_or(scenario.crisis_start);
    let (primary, late_validation) = match (scenario.episode_template_id, level) {
        (ActionEpisodeTemplateId::SystemicCreditBankingCrisis, ActionabilityLevel::Prepare) => (
            DateRange::new(
                scenario.pre_warning_start,
                shift_date(scenario.crisis_start, -21),
            ),
            DateRange::new(
                shift_date(scenario.crisis_start, -20),
                shift_date(scenario.crisis_start, -11),
            ),
        ),
        (ActionEpisodeTemplateId::SystemicCreditBankingCrisis, ActionabilityLevel::Hedge) => (
            DateRange::new(
                shift_date(scenario.crisis_start, -20),
                shift_date(scenario.crisis_start, -6),
            ),
            DateRange::new(
                shift_date(scenario.crisis_start, -5),
                shift_date(scenario.crisis_start, 3),
            ),
        ),
        (ActionEpisodeTemplateId::SystemicCreditBankingCrisis, ActionabilityLevel::Defend) => (
            DateRange::new(
                shift_date(scenario.crisis_start, -5),
                shift_date(acute_start, 3),
            ),
            DateRange::new(shift_date(acute_start, 4), shift_date(acute_start, 10)),
        ),
        (ActionEpisodeTemplateId::AcuteMarketLiquidityCrash, ActionabilityLevel::Prepare) => (
            DateRange::new(
                scenario.pre_warning_start.max(shift_date(acute_start, -20)),
                shift_date(acute_start, -11),
            ),
            DateRange::new(shift_date(acute_start, -10), shift_date(acute_start, -7)),
        ),
        (ActionEpisodeTemplateId::AcuteMarketLiquidityCrash, ActionabilityLevel::Hedge) => (
            DateRange::new(shift_date(acute_start, -10), shift_date(acute_start, -4)),
            DateRange::new(shift_date(acute_start, -3), shift_date(acute_start, 1)),
        ),
        (ActionEpisodeTemplateId::AcuteMarketLiquidityCrash, ActionabilityLevel::Defend) => (
            DateRange::new(shift_date(acute_start, -3), shift_date(acute_start, 2)),
            DateRange::new(shift_date(acute_start, 3), shift_date(acute_start, 7)),
        ),
        (ActionEpisodeTemplateId::MixedSystemicStress, ActionabilityLevel::Prepare)
        | (ActionEpisodeTemplateId::RateShockOrPolicyDislocation, ActionabilityLevel::Prepare) => (
            DateRange::new(
                scenario.pre_warning_start,
                shift_date(scenario.crisis_start, -16),
            ),
            DateRange::new(
                shift_date(scenario.crisis_start, -15),
                shift_date(scenario.crisis_start, -8),
            ),
        ),
        (ActionEpisodeTemplateId::MixedSystemicStress, ActionabilityLevel::Hedge) => (
            DateRange::new(
                shift_date(scenario.crisis_start, -15),
                shift_date(scenario.crisis_start, -5),
            ),
            DateRange::new(
                shift_date(scenario.crisis_start, -4),
                shift_date(scenario.crisis_start, 3),
            ),
        ),
        (ActionEpisodeTemplateId::RateShockOrPolicyDislocation, ActionabilityLevel::Hedge) => (
            DateRange::new(
                shift_date(scenario.crisis_start, -15),
                shift_date(scenario.crisis_start, -5),
            ),
            DateRange::new(
                shift_date(scenario.crisis_start, -4),
                shift_date(scenario.crisis_start, 2),
            ),
        ),
        (
            ActionEpisodeTemplateId::MixedSystemicStress
            | ActionEpisodeTemplateId::RateShockOrPolicyDislocation,
            ActionabilityLevel::Defend,
        ) => (None, None),
    };
    let cooldown = DateRange::new(
        next_day(scenario.crisis_end),
        shift_date(
            scenario.crisis_end,
            post_crisis_cooldown_days(action_level_proxy_horizon_days(level)),
        ),
    );

    ActionEpisodeWindow {
        primary,
        late_validation,
        cooldown,
        protected_action_window: scenario.protected_window
            && scenario.protected_action_levels.contains(&level),
    }
}

fn action_episode_window(
    scenario: &CrisisScenario,
    level: ActionabilityLevel,
) -> ActionEpisodeWindow {
    let mut window = action_episode_default_window(scenario, level);
    let Some(override_window) =
        action_episode_override_for_level(scenario.action_episode_overrides.as_ref(), level)
    else {
        return window;
    };

    if override_window.enabled == Some(false) {
        return ActionEpisodeWindow {
            primary: None,
            late_validation: None,
            cooldown: None,
            protected_action_window: window.protected_action_window,
        };
    }

    if let (Some(primary_start), Some(primary_end)) =
        (override_window.primary_start, override_window.primary_end)
    {
        window.primary = DateRange::new(primary_start, primary_end);
    }
    if let Some(late_validation_end) = override_window.late_validation_end {
        window.late_validation = window
            .primary
            .and_then(|primary| DateRange::new(next_day(primary.end), late_validation_end));
    }
    if let Some(cooldown_end) = override_window.cooldown_end {
        window.cooldown = DateRange::new(next_day(scenario.crisis_end), cooldown_end);
    }

    window
}

fn action_episode_phase_for_date(
    as_of_date: NaiveDate,
    scenario: &CrisisScenario,
    level: ActionabilityLevel,
) -> ActionEpisodePhase {
    let window = action_episode_window(scenario, level);
    if window
        .primary
        .is_some_and(|range| range.contains(as_of_date))
    {
        return ActionEpisodePhase::Primary;
    }
    if window
        .late_validation
        .is_some_and(|range| range.contains(as_of_date))
    {
        return ActionEpisodePhase::LateValidation;
    }
    if window
        .cooldown
        .is_some_and(|range| range.contains(as_of_date))
    {
        return ActionEpisodePhase::Cooldown;
    }
    ActionEpisodePhase::Outside
}

fn action_episode_label_for_level(
    as_of_date: NaiveDate,
    scenarios: &[CrisisScenario],
    level: ActionabilityLevel,
) -> u8 {
    scenarios.iter().any(|scenario| {
        matches!(
            action_episode_phase_for_date(as_of_date, scenario, level),
            ActionEpisodePhase::Primary
        )
    }) as u8
}

fn dominant_action_episode_for_date(
    as_of_date: NaiveDate,
    scenarios: &[CrisisScenario],
) -> Option<ActionEpisodeSelection> {
    [
        ActionabilityLevel::Prepare,
        ActionabilityLevel::Hedge,
        ActionabilityLevel::Defend,
    ]
    .into_iter()
    .flat_map(|level| {
        scenarios.iter().filter_map(move |scenario| {
            let phase = action_episode_phase_for_date(as_of_date, scenario, level);
            (!matches!(phase, ActionEpisodePhase::Outside)).then_some(ActionEpisodeSelection {
                scenario_id: scenario.scenario_id.clone(),
                level,
                phase,
                protected_action_window: action_episode_window(scenario, level)
                    .protected_action_window,
                crisis_start: scenario.crisis_start,
            })
        })
    })
    .min_by_key(|selection| {
        (
            action_episode_phase_rank(selection.phase),
            action_level_rank(selection.level),
            (selection.crisis_start - as_of_date).num_days().abs(),
        )
    })
}

fn protected_context_phase_for_date(
    as_of_date: NaiveDate,
    positive_scenarios: &[CrisisScenario],
    context_scenarios: &[CrisisScenario],
) -> Option<ActionEpisodePhase> {
    context_scenarios
        .iter()
        .filter(|scenario| {
            scenario.protected_window
                && !positive_scenarios
                    .iter()
                    .any(|positive| positive.scenario_id == scenario.scenario_id)
        })
        .flat_map(|scenario| {
            scenario
                .protected_action_levels
                .iter()
                .copied()
                .filter_map(move |level| {
                    let phase = action_episode_phase_for_date(as_of_date, scenario, level);
                    (!matches!(phase, ActionEpisodePhase::Outside)).then_some((phase, scenario))
                })
        })
        .min_by_key(|(phase, scenario)| {
            (
                action_episode_phase_rank(*phase),
                (scenario.crisis_start - as_of_date).num_days().abs(),
            )
        })
        .map(|(phase, _)| phase)
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
    [
        ActionabilityLevel::Prepare,
        ActionabilityLevel::Hedge,
        ActionabilityLevel::Defend,
    ]
    .into_iter()
    .any(|level| {
        !matches!(
            action_episode_phase_for_date(as_of_date, scenario, level),
            ActionEpisodePhase::Outside
        )
    })
}

fn primary_scenario_for_date(
    as_of_date: NaiveDate,
    scenarios: &[CrisisScenario],
) -> Option<CrisisScenario> {
    if let Some(selection) = dominant_action_episode_for_date(as_of_date, scenarios) {
        return scenarios
            .iter()
            .find(|scenario| scenario.scenario_id == selection.scenario_id)
            .cloned();
    }

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

fn safe_divide(numerator: f64, denominator: f64) -> f64 {
    if denominator.abs() <= f64::EPSILON {
        0.0
    } else {
        numerator / denominator
    }
}

fn safe_ratio(numerator: usize, denominator: usize) -> f64 {
    safe_divide(numerator as f64, denominator as f64)
}

fn actionability_level_text(level: ActionabilityLevel) -> &'static str {
    match level {
        ActionabilityLevel::Prepare => "prepare",
        ActionabilityLevel::Hedge => "hedge",
        ActionabilityLevel::Defend => "defend",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
enum ProbabilityTrainingRegime {
    Normal,
    PositiveWindow,
    PreWarningBuffer,
    InCrisis,
    PostCrisisCooldown,
}

fn probability_training_regime_name(regime: ProbabilityTrainingRegime) -> &'static str {
    match regime {
        ProbabilityTrainingRegime::Normal => "normal",
        ProbabilityTrainingRegime::PositiveWindow => "positive_window",
        ProbabilityTrainingRegime::PreWarningBuffer => "pre_warning_buffer",
        ProbabilityTrainingRegime::InCrisis => "in_crisis",
        ProbabilityTrainingRegime::PostCrisisCooldown => "post_crisis_cooldown",
    }
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
    scenario_training_role: Option<String>,
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
    prepare_episode_label: u8,
    hedge_episode_label: u8,
    defend_episode_label: u8,
    primary_action_level: Option<String>,
    action_episode_id: Option<String>,
    action_episode_phase: String,
    protected_action_window: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[allow(dead_code)]
#[serde(rename_all = "snake_case")]
enum ProbabilityTargetLabelMode {
    ForwardCrisis,
    ActionWindow,
    ActionEpisode,
}

impl ProbabilityTargetLabelMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::ForwardCrisis => "forward_crisis",
            Self::ActionWindow => "action_window",
            Self::ActionEpisode => "action_episode",
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
            (ProbabilityTargetLabelMode::ActionEpisode, 5) => self.defend_episode_label as f64,
            (ProbabilityTargetLabelMode::ActionEpisode, 20) => self.hedge_episode_label as f64,
            (ProbabilityTargetLabelMode::ActionEpisode, 60) => self.prepare_episode_label as f64,
            _ => 0.0,
        }
    }

    fn action_episode_phase_for_horizon(&self, horizon_days: u32) -> ActionEpisodePhase {
        let Some(level) = actionability_level_for_proxy_horizon(horizon_days) else {
            return ActionEpisodePhase::Outside;
        };
        let Some(action_episode_id) = self.action_episode_id.as_deref() else {
            return ActionEpisodePhase::Outside;
        };
        if !action_episode_id.ends_with(actionability_level_text(level)) {
            return ActionEpisodePhase::Outside;
        }
        match self.action_episode_phase.as_str() {
            "primary" => ActionEpisodePhase::Primary,
            "late_validation" => ActionEpisodePhase::LateValidation,
            "cooldown" => ActionEpisodePhase::Cooldown,
            _ => ActionEpisodePhase::Outside,
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
    model_family: String,
    feature_transform: String,
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

#[derive(Debug, Clone)]
struct ProbabilityCalibrationSelection<'a> {
    rows: Vec<&'a ProbabilityTrainingRow>,
    eligible_row_count: usize,
    eligible_positive_count: usize,
    eligible_negative_count: usize,
    used_full_split_fallback: bool,
}

#[derive(Debug, Clone, Copy, Default)]
struct ProbabilityThresholdDecisionMetrics {
    regime_hits: ProbabilityThresholdRegimeHitSummary,
    predicted_positive_count: u32,
    true_positive_count: u32,
    precision: f64,
    recall: f64,
}

#[derive(Debug, Clone)]
struct ProbabilityThresholdSelection<'a> {
    rows: Vec<&'a ProbabilityTrainingRow>,
    probabilities: Vec<f64>,
    labels: Vec<f64>,
    used_full_split_fallback: bool,
}

#[derive(Debug, Clone, Copy)]
struct ProbabilityThresholdDiagnosticsInput<'a> {
    full_calibration_rows: &'a [ProbabilityTrainingRow],
    calibration_selection: &'a ProbabilityCalibrationSelection<'a>,
    threshold_selection: &'a ProbabilityThresholdSelection<'a>,
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
    base_threshold: f64,
    final_threshold: f64,
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
    let primary_dataset_key = resolve_formal_training_dataset_key(store, options).await?;
    let mut dataset_keys = vec![primary_dataset_key.clone()];
    for dataset_key in &options.aux_dataset_keys {
        if !dataset_keys.contains(dataset_key) {
            dataset_keys.push(dataset_key.clone());
        }
    }

    let primary_dataset = store
        .load_formal_dataset(&primary_dataset_key)
        .await?
        .with_context(|| format!("formal dataset {primary_dataset_key} was not found in SQLite"))?;

    let mut combined_rows = Vec::<FormalDatasetRowRecord>::new();
    let mut positive_by_id = BTreeMap::<String, CrisisScenario>::new();
    let mut context_by_id = BTreeMap::<String, CrisisScenario>::new();

    for dataset_key in &dataset_keys {
        let dataset = store
            .load_formal_dataset(dataset_key)
            .await?
            .with_context(|| format!("formal dataset {dataset_key} was not found in SQLite"))?;
        if dataset.manifest.market_scope != primary_dataset.manifest.market_scope {
            bail!(
                "auxiliary formal dataset {dataset_key} has market scope {} but primary dataset {} uses {}; mixed-market training is not supported",
                dataset.manifest.market_scope,
                primary_dataset_key,
                primary_dataset.manifest.market_scope
            );
        }
        if dataset.manifest.point_in_time_mode != primary_dataset.manifest.point_in_time_mode {
            bail!(
                "auxiliary formal dataset {dataset_key} has point_in_time_mode {} but primary dataset {} uses {}; mixed PIT modes are not supported",
                dataset.manifest.point_in_time_mode,
                primary_dataset_key,
                primary_dataset.manifest.point_in_time_mode
            );
        }
        if dataset.manifest.feature_set_version != primary_dataset.manifest.feature_set_version {
            bail!(
                "auxiliary formal dataset {dataset_key} has feature_set_version {} but primary dataset {} uses {}; mixed feature sets are not supported",
                dataset.manifest.feature_set_version,
                primary_dataset_key,
                primary_dataset.manifest.feature_set_version
            );
        }

        let mut rows = store
            .list_formal_dataset_rows(dataset_key, None, None)
            .await?;
        if let Some(from) = options.query.from {
            rows.retain(|row| row.as_of_date >= from);
        }
        if let Some(to) = options.query.to {
            rows.retain(|row| row.as_of_date <= to);
        }
        if rows.is_empty() {
            bail!(
                "formal dataset {dataset_key} has no rows after the requested date filters; widen --from/--to or choose a different auxiliary dataset"
            );
        }
        combined_rows.extend(rows);

        let scenario_sets = load_formal_dataset_scenario_sets(
            &dataset.manifest.scenario_set_version,
            &dataset.manifest.label_version,
        )?;
        for scenario in scenario_sets.positive_scenarios {
            positive_by_id.insert(scenario.scenario_id.clone(), scenario);
        }
        for scenario in scenario_sets.context_scenarios {
            context_by_id.insert(scenario.scenario_id.clone(), scenario);
        }
    }

    if combined_rows.len() < 90 {
        bail!(
            "formal dataset {} is too small after filters: {} rows found across {} dataset(s), at least 90 are required; backfill more free historical observations and rebuild the formal dataset, or use --dataset-source snapshot as a temporary fallback",
            primary_dataset_key,
            combined_rows.len(),
            dataset_keys.len()
        );
    }

    let positive_scenarios = positive_by_id.into_values().collect::<Vec<_>>();
    let context_scenarios = context_by_id.into_values().collect::<Vec<_>>();
    let scenario_by_id = context_scenarios
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
            scenario_training_role: row
                .scenario_training_role
                .clone()
                .or_else(|| primary_scenario.map(|scenario| scenario.training_role.clone())),
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
            regime_5d: forward_crisis_training_regime_with_context(
                row.as_of_date,
                &positive_scenarios,
                &context_scenarios,
                5,
            ),
            regime_20d: forward_crisis_training_regime_with_context(
                row.as_of_date,
                &positive_scenarios,
                &context_scenarios,
                20,
            ),
            regime_60d: forward_crisis_training_regime_with_context(
                row.as_of_date,
                &positive_scenarios,
                &context_scenarios,
                60,
            ),
            action_label_5d: row.action_label_5d,
            action_label_20d: row.action_label_20d,
            action_label_60d: row.action_label_60d,
            prepare_episode_label: row.prepare_episode_label,
            hedge_episode_label: row.hedge_episode_label,
            defend_episode_label: row.defend_episode_label,
            primary_action_level: row.primary_action_level.clone(),
            action_episode_id: row.action_episode_id.clone(),
            action_episode_phase: row.action_episode_phase.clone(),
            protected_action_window: row.protected_action_window,
        }
    };

    let mut train_rows = combined_rows
        .iter()
        .filter(|row| row.split_name == "train")
        .map(to_training_row)
        .collect::<Vec<_>>();
    let mut calibration_rows = combined_rows
        .iter()
        .filter(|row| row.split_name == "calibration")
        .map(to_training_row)
        .collect::<Vec<_>>();
    let mut evaluation_rows = combined_rows
        .iter()
        .filter(|row| row.split_name == "evaluation")
        .map(to_training_row)
        .collect::<Vec<_>>();

    train_rows.sort_by_key(|row| row.as_of_date);
    calibration_rows.sort_by_key(|row| row.as_of_date);
    evaluation_rows.sort_by_key(|row| row.as_of_date);

    if train_rows.is_empty() || calibration_rows.is_empty() || evaluation_rows.is_empty() {
        bail!(
            "formal dataset {} is missing one or more required splits after filters (train={}, calibration={}, evaluation={}); rebuild it from a broader historical range before training the formal bundle",
            primary_dataset_key,
            train_rows.len(),
            calibration_rows.len(),
            evaluation_rows.len()
        );
    }

    let dataset_label = if dataset_keys.len() == 1 {
        primary_dataset_key.clone()
    } else {
        format!(
            "{} + aux({})",
            primary_dataset_key,
            dataset_keys[1..].join(", ")
        )
    };

    Ok(ProbabilityTrainingInput {
        dataset_source: PipelineDatasetSource::Formal,
        dataset_label,
        market_scope: primary_dataset.manifest.market_scope.clone(),
        point_in_time_mode: primary_dataset.manifest.point_in_time_mode.clone(),
        feature_set_version: primary_dataset.manifest.feature_set_version.clone(),
        label_version: primary_dataset.manifest.label_version.clone(),
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
        "as_of_date,market_scope,release_id,probability_mode,release_status,point_in_time_mode,overall_score,external_shock_score,raw_p_5d,raw_p_20d,raw_p_60d,calibrated_p_5d,calibrated_p_20d,calibrated_p_60d,posture,time_to_risk_bucket,coverage_score,freshness_status,method_version,posture_trigger_codes,posture_blocker_codes,recorded_at\n",
    );
    for snapshot in snapshots {
        let _ = writeln!(
            csv,
            "{},{},{},{},{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{},{},{:.6},{},{},{},{},{}",
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
            snapshot.posture_trigger_codes.join("|"),
            snapshot.posture_blocker_codes.join("|"),
            snapshot.recorded_at.to_rfc3339()
        );
    }
    csv
}

fn render_dataset_csv(dataset: &[ProbabilityTrainingRow], feature_names: &[String]) -> String {
    let mut header = String::from(
        "as_of_date,market_scope,release_id,probability_mode,freshness_status,time_to_risk_bucket,split_name,primary_scenario_id,scenario_family,scenario_training_role,label_5d,label_20d,label_60d,action_label_5d,action_label_20d,action_label_60d,prepare_episode_label,hedge_episode_label,defend_episode_label,primary_action_level,action_episode_id,action_episode_phase,protected_action_window",
    );
    for feature in feature_names {
        header.push(',');
        header.push_str(feature);
    }
    header.push('\n');

    let mut csv = header;
    for row in dataset {
        let columns = [
            row.as_of_date.to_string(),
            row.market_scope.clone(),
            row.release_id.clone().unwrap_or_default(),
            row.probability_mode.clone().unwrap_or_default(),
            row.freshness_status.clone().unwrap_or_default(),
            row.time_to_risk_bucket.clone().unwrap_or_default(),
            row.split_name.clone().unwrap_or_default(),
            row.primary_scenario_id.clone().unwrap_or_default(),
            row.scenario_family.clone().unwrap_or_default(),
            row.scenario_training_role.clone().unwrap_or_default(),
            row.label_5d.to_string(),
            row.label_20d.to_string(),
            row.label_60d.to_string(),
            row.action_label_5d.to_string(),
            row.action_label_20d.to_string(),
            row.action_label_60d.to_string(),
            row.prepare_episode_label.to_string(),
            row.hedge_episode_label.to_string(),
            row.defend_episode_label.to_string(),
            row.primary_action_level.clone().unwrap_or_default(),
            row.action_episode_id.clone().unwrap_or_default(),
            row.action_episode_phase.clone(),
            (row.protected_action_window as u8).to_string(),
        ];
        csv.push_str(&columns.join(","));
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
    let model_feature_names = probability_feature_names_for_transform(
        &training.feature_names,
        options.model_shape.feature_transform(),
    );
    let crisis_prior_label_mode = ProbabilityTargetLabelMode::ForwardCrisis;
    let horizons = [5_u32, 20_u32, 60_u32]
        .into_iter()
        .map(|horizon| {
            train_horizon_bundle(
                &training.train_rows,
                &training.calibration_rows,
                &training.evaluation_rows,
                &model_feature_names,
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
            ProbabilityTargetLabelMode::ActionEpisode,
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
            "Formal bundle trained from persisted formal dataset {} built from raw observations -> feature snapshots -> scenario labels; model_shape={} feature_transform={}; crisis-prior head uses forward-crisis labels, and {}.",
            training.dataset_label,
            options.model_shape.as_str(),
            options.model_shape.feature_transform(),
            if actionability.is_some() {
                "actionability head uses episode-native prepare/hedge/defend labels when quality gates pass"
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
        model_family: options.model_shape.as_str().to_string(),
        feature_transform: options.model_shape.feature_transform().to_string(),
        created_at: generated_at,
        feature_names: model_feature_names.clone(),
        monotonic_min_gap_5d_to_20d: 0.02,
        monotonic_min_gap_20d_to_60d: 0.03,
        note: bundle_note.clone(),
        horizons: horizons.clone(),
        evaluation: Some(aggregate_evaluation.clone()),
        actionability: actionability.clone(),
    };

    let bundle_path = options.output_dir.join(format!("{release_id}.json"));
    let manifest_dir = options.manifest_dir.clone();
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
            prob_model_version: format!("prob_{}_{}", options.model_shape.as_str(), release_suffix),
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
                "Generated by `research pipeline train-probability` from {} dataset {} with model_shape={}.",
                training.dataset_source.as_str(),
                training.dataset_label,
                options.model_shape.as_str()
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
        model_family: options.model_shape.as_str().to_string(),
        feature_transform: options.model_shape.feature_transform().to_string(),
        target_label_mode: crisis_prior_label_mode,
        market_scope: release.manifest.market_scope.clone(),
        feature_names: model_feature_names.clone(),
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
    let scenario_sets = load_formal_dataset_scenario_sets(
        DEFAULT_FORMAL_SCENARIO_SET_VERSION,
        DEFAULT_FORMAL_LABEL_VERSION,
    )
    .expect("default scenario catalog must contain the main training label set");
    let positive_scenarios = scenario_sets.positive_scenarios;
    let context_scenarios = scenario_sets.context_scenarios;
    let mut rows = snapshots
        .iter()
        .map(|snapshot| {
            let features = pipeline_features_from_snapshot(snapshot);
            let primary_scenario =
                primary_scenario_for_date(snapshot.as_of_date, &context_scenarios);
            let dominant_action_episode =
                dominant_action_episode_for_date(snapshot.as_of_date, &context_scenarios);
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
                scenario_training_role: primary_scenario
                    .as_ref()
                    .map(|scenario| scenario.training_role.clone()),
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
                label_5d: forward_crisis_label(snapshot.as_of_date, &positive_scenarios, 5),
                label_20d: forward_crisis_label(snapshot.as_of_date, &positive_scenarios, 20),
                label_60d: forward_crisis_label(snapshot.as_of_date, &positive_scenarios, 60),
                regime_5d: forward_crisis_training_regime_with_context(
                    snapshot.as_of_date,
                    &positive_scenarios,
                    &context_scenarios,
                    5,
                ),
                regime_20d: forward_crisis_training_regime_with_context(
                    snapshot.as_of_date,
                    &positive_scenarios,
                    &context_scenarios,
                    20,
                ),
                regime_60d: forward_crisis_training_regime_with_context(
                    snapshot.as_of_date,
                    &positive_scenarios,
                    &context_scenarios,
                    60,
                ),
                action_label_5d: action_window_label(snapshot.as_of_date, &context_scenarios, 5),
                action_label_20d: action_window_label(snapshot.as_of_date, &context_scenarios, 20),
                action_label_60d: action_window_label(snapshot.as_of_date, &context_scenarios, 60),
                prepare_episode_label: action_episode_label_for_level(
                    snapshot.as_of_date,
                    &context_scenarios,
                    ActionabilityLevel::Prepare,
                ),
                hedge_episode_label: action_episode_label_for_level(
                    snapshot.as_of_date,
                    &context_scenarios,
                    ActionabilityLevel::Hedge,
                ),
                defend_episode_label: action_episode_label_for_level(
                    snapshot.as_of_date,
                    &context_scenarios,
                    ActionabilityLevel::Defend,
                ),
                primary_action_level: dominant_action_episode
                    .as_ref()
                    .filter(|selection| matches!(selection.phase, ActionEpisodePhase::Primary))
                    .map(|selection| actionability_level_text(selection.level).to_string()),
                action_episode_id: dominant_action_episode.as_ref().map(|selection| {
                    format!(
                        "{}:{}",
                        selection.scenario_id,
                        actionability_level_text(selection.level)
                    )
                }),
                action_episode_phase: dominant_action_episode
                    .as_ref()
                    .map(|selection| selection.phase.as_str().to_string())
                    .unwrap_or_else(|| ActionEpisodePhase::Outside.as_str().to_string()),
                protected_action_window: dominant_action_episode
                    .as_ref()
                    .is_some_and(|selection| selection.protected_action_window),
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

fn forward_crisis_training_regime_with_context(
    as_of_date: NaiveDate,
    positive_scenarios: &[CrisisScenario],
    context_scenarios: &[CrisisScenario],
    horizon_days: u32,
) -> ProbabilityTrainingRegime {
    let base_regime = forward_crisis_training_regime(as_of_date, positive_scenarios, horizon_days);
    if !matches!(base_regime, ProbabilityTrainingRegime::Normal) || horizon_days < 20 {
        return base_regime;
    }

    match protected_context_phase_for_date(as_of_date, positive_scenarios, context_scenarios) {
        Some(ActionEpisodePhase::Primary | ActionEpisodePhase::LateValidation) => {
            ProbabilityTrainingRegime::PreWarningBuffer
        }
        Some(ActionEpisodePhase::Cooldown) => ProbabilityTrainingRegime::PostCrisisCooldown,
        _ => base_regime,
    }
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
        note: "Separate actionability head trained from episode-native prepare/hedge/defend labels to complement the crisis-prior horizons; runtime consumes threshold-aware confidence instead of treating raw action probabilities as direct posture signals.".to_string(),
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
    let label_mode = ProbabilityTargetLabelMode::ActionEpisode;
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

#[derive(Debug, Clone, Copy)]
struct ActionabilityGuardrailPolicy {
    min_scenario_count: u32,
    min_precision_score: i64,
    min_advance_warning_rate_score: Option<i64>,
    max_late_confirmation_rate_score: Option<i64>,
    max_missed_rate_score: i64,
}

fn actionability_guardrail_policy(
    level: ActionabilityLevel,
    horizon_days: u32,
) -> ActionabilityGuardrailPolicy {
    match level {
        ActionabilityLevel::Prepare => ActionabilityGuardrailPolicy {
            min_scenario_count: 2,
            min_precision_score: actionability_precision_floor_score(horizon_days),
            min_advance_warning_rate_score: Some(350),
            max_late_confirmation_rate_score: Some(500),
            max_missed_rate_score: 650,
        },
        ActionabilityLevel::Hedge => ActionabilityGuardrailPolicy {
            min_scenario_count: 2,
            min_precision_score: actionability_precision_floor_score(horizon_days),
            min_advance_warning_rate_score: Some(250),
            max_late_confirmation_rate_score: Some(500),
            max_missed_rate_score: 650,
        },
        ActionabilityLevel::Defend => ActionabilityGuardrailPolicy {
            min_scenario_count: 2,
            min_precision_score: actionability_precision_floor_score(horizon_days),
            min_advance_warning_rate_score: None,
            max_late_confirmation_rate_score: Some(400),
            max_missed_rate_score: 500,
        },
    }
}

fn percentage_score(value: Option<f64>) -> Option<i64> {
    value.map(|rate| (rate * 1_000.0).round() as i64)
}

fn actionability_prediction_count_ceiling_from_actual_positive_count(
    actual_positive_count: u32,
    horizon_days: u32,
) -> u32 {
    let multiple = match horizon_days {
        5 => 6_u32,
        20 => 8_u32,
        60 => 10_u32,
        _ => 8_u32,
    };
    actual_positive_count.max(1).saturating_mul(multiple)
}

fn actionability_prediction_count_ceiling(
    summary: &ActionabilityEvaluationSummary,
    horizon_days: u32,
) -> u32 {
    actionability_prediction_count_ceiling_from_actual_positive_count(
        summary.actual_positive_count,
        horizon_days,
    )
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

        let policy = actionability_guardrail_policy(level.level, level.proxy_horizon_days);
        let precision_score =
            (summary.precision_at_threshold.unwrap_or_default() * 1_000.0).round() as i64;
        if precision_score < policy.min_precision_score {
            regressions.push(format!(
                "{} precision {:.1}% is below required {:.1}%",
                actionability_level_text(level.level),
                precision_score as f64 / 10.0,
                policy.min_precision_score as f64 / 10.0
            ));
        }

        if summary.scenario_count < policy.min_scenario_count {
            regressions.push(format!(
                "{} scenario_count {} is below required {}",
                actionability_level_text(level.level),
                summary.scenario_count,
                policy.min_scenario_count
            ));
        }

        let prediction_ceiling =
            actionability_prediction_count_ceiling(summary, level.proxy_horizon_days);
        if summary.predicted_positive_count > prediction_ceiling {
            regressions.push(format!(
                "{} predicted positives {} exceed ceiling {} for {} primary episode rows",
                actionability_level_text(level.level),
                summary.predicted_positive_count,
                prediction_ceiling,
                summary.actual_positive_count
            ));
        }

        if summary.actual_positive_count > 0 {
            if let Some(min_advance_warning_rate_score) = policy.min_advance_warning_rate_score {
                let advance_warning_rate_score =
                    percentage_score(summary.advance_warning_rate).unwrap_or_default();
                if advance_warning_rate_score < min_advance_warning_rate_score {
                    regressions.push(format!(
                        "{} on_time_rate {:.1}% is below required {:.1}%",
                        actionability_level_text(level.level),
                        advance_warning_rate_score as f64 / 10.0,
                        min_advance_warning_rate_score as f64 / 10.0
                    ));
                }
            }

            if let Some(max_late_confirmation_rate_score) = policy.max_late_confirmation_rate_score
            {
                let late_confirmation_rate_score =
                    percentage_score(summary.late_confirmation_rate).unwrap_or_default();
                if late_confirmation_rate_score > max_late_confirmation_rate_score {
                    regressions.push(format!(
                        "{} late_only_rate {:.1}% exceeds ceiling {:.1}%",
                        actionability_level_text(level.level),
                        late_confirmation_rate_score as f64 / 10.0,
                        max_late_confirmation_rate_score as f64 / 10.0
                    ));
                }
            }

            let missed_rate_score = percentage_score(summary.missed_rate).unwrap_or_default();
            if missed_rate_score > policy.max_missed_rate_score {
                regressions.push(format!(
                    "{} missed_rate {:.1}% exceeds ceiling {:.1}%",
                    actionability_level_text(level.level),
                    missed_rate_score as f64 / 10.0,
                    policy.max_missed_rate_score as f64 / 10.0
                ));
            }
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
    let raw_score =
        actionability_summary_selection_score(&raw_summary, raw_threshold, horizon_days);

    let calibration_probabilities = calibration_raw_probabilities
        .iter()
        .map(|raw_probability| {
            apply_platt_probability_calibration(*raw_probability, &calibration_candidate)
        })
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
                apply_platt_probability_calibration(*raw_probability, &calibration_candidate)
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
    let calibration_selection =
        probability_calibration_selection_rows(calibration_rows, horizon_days, label_mode);
    let calibration_inputs = calibration_selection
        .rows
        .iter()
        .map(|row| score_logistic_model_for_dataset(&raw_model, row))
        .collect::<Vec<_>>();
    let calibration_labels = calibration_selection
        .rows
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
        &calibration_selection.rows,
        horizon_days,
        label_mode,
        &evaluation_raw_probabilities,
        calibration_candidate,
    );
    let calibration_decision_probabilities = calibration.as_ref().map_or_else(
        || calibration_inputs.clone(),
        |calibration| {
            calibration_inputs
                .iter()
                .map(|raw_probability| {
                    apply_platt_probability_calibration(*raw_probability, calibration)
                })
                .collect::<Vec<_>>()
        },
    );
    let threshold_selection = probability_decision_threshold_selection(
        &calibration_decision_probabilities,
        &calibration_labels,
        &calibration_selection.rows,
        horizon_days,
        label_mode,
    );
    let base_decision_threshold = select_probability_decision_threshold(
        &threshold_selection.probabilities,
        &threshold_selection.labels,
        horizon_days,
    );
    let decision_threshold = adjust_probability_decision_threshold_for_regime_support(
        base_decision_threshold,
        &threshold_selection.probabilities,
        &threshold_selection.labels,
        &threshold_selection.rows,
        horizon_days,
        label_mode,
    );
    let threshold_diagnostics =
        build_probability_threshold_diagnostics(ProbabilityThresholdDiagnosticsInput {
            full_calibration_rows: calibration_rows,
            calibration_selection: &calibration_selection,
            threshold_selection: &threshold_selection,
            horizon_days,
            label_mode,
            base_threshold: base_decision_threshold,
            final_threshold: decision_threshold,
        });
    let evaluation = evaluate_probabilities_for_rows(
        &evaluation_probabilities,
        evaluation_rows,
        horizon_days,
        label_mode,
    );

    Ok(ProbabilityHorizonBundle {
        horizon_days,
        decision_threshold: Some(decision_threshold),
        threshold_diagnostics: Some(threshold_diagnostics),
        raw_model,
        calibration,
        evaluation,
    })
}

fn probability_calibration_selection_rows(
    rows: &[ProbabilityTrainingRow],
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) -> ProbabilityCalibrationSelection<'_> {
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
        ProbabilityCalibrationSelection {
            rows: filtered,
            eligible_row_count: filtered_positive_count + filtered_negative_count,
            eligible_positive_count: filtered_positive_count,
            eligible_negative_count: filtered_negative_count,
            used_full_split_fallback: false,
        }
    } else {
        ProbabilityCalibrationSelection {
            rows: rows.iter().collect(),
            eligible_row_count: filtered_positive_count + filtered_negative_count,
            eligible_positive_count: filtered_positive_count,
            eligible_negative_count: filtered_negative_count,
            used_full_split_fallback: true,
        }
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
        ProbabilityTargetLabelMode::ActionWindow | ProbabilityTargetLabelMode::ActionEpisode => {
            true
        }
        ProbabilityTargetLabelMode::ForwardCrisis => match horizon_days {
            20 | 60 => matches!(
                row.regime_for_horizon(horizon_days),
                ProbabilityTrainingRegime::Normal
                    | ProbabilityTrainingRegime::PreWarningBuffer
                    | ProbabilityTrainingRegime::InCrisis
                    | ProbabilityTrainingRegime::PostCrisisCooldown
            ),
            _ => matches!(
                row.regime_for_horizon(horizon_days),
                ProbabilityTrainingRegime::Normal
            ),
        },
    }
}

fn probability_decision_threshold_selection<'a>(
    probabilities: &[f64],
    labels: &[f64],
    rows: &[&'a ProbabilityTrainingRow],
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) -> ProbabilityThresholdSelection<'a> {
    let mut filtered_rows = Vec::new();
    let mut filtered_probabilities = Vec::new();
    let mut filtered_labels = Vec::new();
    let mut filtered_positive_count = 0_usize;
    let mut filtered_negative_count = 0_usize;

    for ((probability, label), row) in probabilities.iter().zip(labels).zip(rows.iter().copied()) {
        if !probability_row_is_threshold_eligible(row, horizon_days, label_mode) {
            continue;
        }
        filtered_rows.push(row);
        filtered_probabilities.push(*probability);
        filtered_labels.push(*label);
        if *label >= 0.5 {
            filtered_positive_count += 1;
        } else {
            filtered_negative_count += 1;
        }
    }

    if filtered_positive_count > 0 && filtered_negative_count > 0 {
        ProbabilityThresholdSelection {
            rows: filtered_rows,
            probabilities: filtered_probabilities,
            labels: filtered_labels,
            used_full_split_fallback: false,
        }
    } else {
        ProbabilityThresholdSelection {
            rows: rows.to_vec(),
            probabilities: probabilities.to_vec(),
            labels: labels.to_vec(),
            used_full_split_fallback: true,
        }
    }
}

fn probability_row_is_threshold_eligible(
    row: &ProbabilityTrainingRow,
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) -> bool {
    if row.label_for_horizon(label_mode, horizon_days) > 0.0 {
        return true;
    }

    match label_mode {
        ProbabilityTargetLabelMode::ActionWindow | ProbabilityTargetLabelMode::ActionEpisode => {
            true
        }
        ProbabilityTargetLabelMode::ForwardCrisis => match horizon_days {
            20 | 60 => matches!(
                row.regime_for_horizon(horizon_days),
                ProbabilityTrainingRegime::Normal
                    | ProbabilityTrainingRegime::PreWarningBuffer
                    | ProbabilityTrainingRegime::PostCrisisCooldown
            ),
            _ => matches!(
                row.regime_for_horizon(horizon_days),
                ProbabilityTrainingRegime::Normal
            ),
        },
    }
}

fn select_probability_calibration_strategy(
    calibration_raw_probabilities: &[f64],
    calibration_labels: &[f64],
    calibration_rows: &[&ProbabilityTrainingRow],
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
    evaluation_raw_probabilities: &[f64],
    calibration_candidate: PlattCalibrationArtifact,
) -> (Option<PlattCalibrationArtifact>, Vec<f64>) {
    let raw_summary = evaluate_probabilities(calibration_raw_probabilities, calibration_labels);
    let raw_regime_separation = evaluate_regime_separation_summary_refs(
        calibration_raw_probabilities,
        calibration_rows,
        horizon_days,
        label_mode,
    );
    let raw_score =
        probability_calibration_selection_score(&raw_summary, raw_regime_separation.as_ref());

    let calibration_probabilities = calibration_raw_probabilities
        .iter()
        .map(|raw_probability| {
            apply_platt_probability_calibration(*raw_probability, &calibration_candidate)
        })
        .collect::<Vec<_>>();
    let calibrated_summary = evaluate_probabilities(&calibration_probabilities, calibration_labels);
    let calibrated_regime_separation = evaluate_regime_separation_summary_refs(
        &calibration_probabilities,
        calibration_rows,
        horizon_days,
        label_mode,
    );
    let calibrated_score = probability_calibration_selection_score(
        &calibrated_summary,
        calibrated_regime_separation.as_ref(),
    );

    let raw_ranking_reversed =
        probability_raw_ranking_is_reversed(calibration_raw_probabilities, calibration_labels);
    let keep_calibration = calibrated_score > raw_score
        && (calibration_candidate.alpha > 0.0
            || (calibration_candidate.alpha < 0.0 && raw_ranking_reversed));
    if keep_calibration {
        let evaluation_probabilities = evaluation_raw_probabilities
            .iter()
            .map(|raw_probability| {
                apply_platt_probability_calibration(*raw_probability, &calibration_candidate)
            })
            .collect::<Vec<_>>();
        (Some(calibration_candidate), evaluation_probabilities)
    } else {
        (None, evaluation_raw_probabilities.to_vec())
    }
}

fn probability_calibration_selection_score(
    summary: &HorizonEvaluationSummary,
    regime_separation: Option<&RegimeSeparationEvaluationSummary>,
) -> (i64, i64, i64, i64, i64, i64, i64, i64, i64) {
    (
        probability_regime_diagnosis_score(regime_separation),
        probability_regime_positive_window_lift_score(regime_separation),
        probability_regime_positive_window_gap_score(regime_separation),
        probability_regime_positive_window_minus_cooldown_score(regime_separation),
        probability_regime_early_warning_lift_score(regime_separation),
        probability_regime_max_non_normal_lift_score(regime_separation),
        -((summary.log_loss * 1_000_000.0).round() as i64),
        -((summary.brier_score * 1_000_000.0).round() as i64),
        -((summary.ece * 1_000_000.0).round() as i64),
    )
}

fn probability_regime_diagnosis_score(
    regime_separation: Option<&RegimeSeparationEvaluationSummary>,
) -> i64 {
    match regime_separation.map(|summary| summary.diagnosis.as_str()) {
        Some("usable_early_warning_separation") => 6,
        Some("weak_regime_separation") => 5,
        Some("mixed_or_unclear") => 4,
        Some("late_only_no_early_warning") => 3,
        Some("cooldown_bleed") => 2,
        Some("cold_across_all_regimes") => 1,
        Some(_) => 0,
        None => 2,
    }
}

fn probability_regime_positive_window_lift_score(
    regime_separation: Option<&RegimeSeparationEvaluationSummary>,
) -> i64 {
    (regime_separation
        .and_then(|summary| summary.positive_window_lift_vs_normal)
        .unwrap_or_default()
        * 1_000.0)
        .round() as i64
}

fn probability_regime_positive_window_gap_score(
    regime_separation: Option<&RegimeSeparationEvaluationSummary>,
) -> i64 {
    (regime_separation
        .and_then(|summary| summary.positive_window_gap_vs_normal)
        .unwrap_or_default()
        * 1_000_000.0)
        .round() as i64
}

fn probability_regime_positive_window_minus_cooldown_score(
    regime_separation: Option<&RegimeSeparationEvaluationSummary>,
) -> i64 {
    let Some(summary) = regime_separation else {
        return 0;
    };
    let positive_window = summary.positive_window_lift_vs_normal.unwrap_or_default();
    let cooldown = summary
        .post_crisis_cooldown_lift_vs_normal
        .unwrap_or_default();
    ((positive_window - cooldown) * 1_000.0).round() as i64
}

fn probability_regime_early_warning_lift_score(
    regime_separation: Option<&RegimeSeparationEvaluationSummary>,
) -> i64 {
    (regime_separation
        .and_then(|summary| summary.early_warning_lift_vs_normal)
        .unwrap_or_default()
        * 1_000.0)
        .round() as i64
}

fn probability_regime_max_non_normal_lift_score(
    regime_separation: Option<&RegimeSeparationEvaluationSummary>,
) -> i64 {
    (regime_separation
        .and_then(|summary| summary.max_non_normal_lift_vs_normal)
        .unwrap_or_default()
        * 1_000.0)
        .round() as i64
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
    let thresholds = probability_decision_threshold_candidates(probabilities);

    let actual_positive_count = labels.iter().filter(|label| **label >= 0.5).count() as u32;
    let positive_count = actual_positive_count as f64;
    let prediction_ceiling = probability_prediction_count_ceiling_from_actual_positive_count(
        actual_positive_count,
        horizon_days,
    );
    let mut best_threshold = 0.3;
    let beta_sq = probability_threshold_beta_sq(horizon_days);
    let mut best_score = None::<(i64, i64, i64, i64)>;
    let mut best_capped_threshold = None::<f64>;
    let mut best_capped_score = None::<(i64, i64, i64, i64)>;
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
        let f_beta = if precision > 0.0 || recall > 0.0 {
            (1.0 + beta_sq) * precision * recall / (beta_sq * precision + recall).max(1e-9)
        } else {
            0.0
        };
        let score =
            probability_threshold_score_tuple(horizon_days, precision, recall, f_beta, threshold);
        if best_score.is_none_or(|best| score > best) {
            best_score = Some(score);
            best_threshold = threshold;
        }
        if predicted_positive_count <= prediction_ceiling
            && best_capped_score.is_none_or(|best| score > best)
        {
            best_capped_score = Some(score);
            best_capped_threshold = Some(threshold);
        }
    }

    let minimum_threshold = match horizon_days {
        5 => 0.03,
        20 => 0.005,
        60 => 0.01,
        _ => 0.001,
    };

    round3(best_capped_threshold.unwrap_or(best_threshold)).clamp(minimum_threshold, 0.90)
}

fn probability_decision_threshold_candidates(probabilities: &[f64]) -> Vec<f64> {
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
    thresholds
}

fn probability_threshold_beta_sq(horizon_days: u32) -> f64 {
    match horizon_days {
        5 => 0.25,
        20 => 1.0,
        60 => 2.25,
        _ => 1.0,
    }
}

fn probability_threshold_score_tuple(
    horizon_days: u32,
    precision: f64,
    recall: f64,
    f_beta: f64,
    threshold: f64,
) -> (i64, i64, i64, i64) {
    let precision_score = (precision * 1_000_000.0).round() as i64;
    let recall_score = (recall * 1_000_000.0).round() as i64;
    let f_beta_score = (f_beta * 1_000_000.0).round() as i64;
    let threshold_score = (threshold * 1_000.0).round() as i64;

    match horizon_days {
        5 => (precision_score, f_beta_score, recall_score, threshold_score),
        20 => (f_beta_score, precision_score, recall_score, threshold_score),
        60 => (f_beta_score, recall_score, precision_score, threshold_score),
        _ => (f_beta_score, precision_score, recall_score, threshold_score),
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct ProbabilityThresholdRegimeHitSummary {
    early_warning_row_count: u32,
    early_warning_hit_count: u32,
    normal_row_count: u32,
    normal_hit_count: u32,
    positive_window_row_count: u32,
    positive_window_hit_count: u32,
    in_crisis_row_count: u32,
    in_crisis_hit_count: u32,
    cooldown_row_count: u32,
    cooldown_hit_count: u32,
}

impl ProbabilityThresholdRegimeHitSummary {
    fn early_warning_hit_rate(self) -> f64 {
        safe_divide(
            self.early_warning_hit_count as f64,
            self.early_warning_row_count as f64,
        )
    }

    fn normal_hit_rate(self) -> f64 {
        safe_divide(self.normal_hit_count as f64, self.normal_row_count as f64)
    }

    fn positive_window_hit_rate(self) -> f64 {
        safe_divide(
            self.positive_window_hit_count as f64,
            self.positive_window_row_count as f64,
        )
    }

    fn in_crisis_hit_rate(self) -> f64 {
        safe_divide(
            self.in_crisis_hit_count as f64,
            self.in_crisis_row_count as f64,
        )
    }

    fn cooldown_hit_rate(self) -> f64 {
        safe_divide(
            self.cooldown_hit_count as f64,
            self.cooldown_row_count as f64,
        )
    }
}

fn probability_early_warning_regime(horizon_days: u32) -> ProbabilityTrainingRegime {
    match horizon_days {
        5 => ProbabilityTrainingRegime::PositiveWindow,
        20 | 60 => ProbabilityTrainingRegime::PreWarningBuffer,
        _ => ProbabilityTrainingRegime::PositiveWindow,
    }
}

fn probability_threshold_regime_hit_summary(
    probabilities: &[f64],
    rows: &[&ProbabilityTrainingRow],
    horizon_days: u32,
    threshold: f64,
) -> ProbabilityThresholdRegimeHitSummary {
    let early_warning_regime = probability_early_warning_regime(horizon_days);

    let mut summary = ProbabilityThresholdRegimeHitSummary::default();
    for (probability, row) in probabilities.iter().zip(rows.iter().copied()) {
        let regime = row.regime_for_horizon(horizon_days);
        let hit = *probability >= threshold;

        if regime == early_warning_regime {
            summary.early_warning_row_count += 1;
            if hit {
                summary.early_warning_hit_count += 1;
            }
        }

        match regime {
            ProbabilityTrainingRegime::Normal => {
                summary.normal_row_count += 1;
                if hit {
                    summary.normal_hit_count += 1;
                }
            }
            ProbabilityTrainingRegime::PositiveWindow => {
                summary.positive_window_row_count += 1;
                if hit {
                    summary.positive_window_hit_count += 1;
                }
            }
            ProbabilityTrainingRegime::InCrisis => {
                summary.in_crisis_row_count += 1;
                if hit {
                    summary.in_crisis_hit_count += 1;
                }
            }
            ProbabilityTrainingRegime::PostCrisisCooldown => {
                summary.cooldown_row_count += 1;
                if hit {
                    summary.cooldown_hit_count += 1;
                }
            }
            ProbabilityTrainingRegime::PreWarningBuffer => {}
        }
    }

    summary
}

fn regime_aware_threshold_prediction_ceiling(actual_positive_count: u32, horizon_days: u32) -> u32 {
    let base = probability_prediction_count_ceiling_from_actual_positive_count(
        actual_positive_count,
        horizon_days,
    );
    match horizon_days {
        60 => base.saturating_mul(3),
        20 => base.saturating_mul(2),
        _ => base,
    }
}

fn regime_floor_min_hit_rate(horizon_days: u32) -> f64 {
    match horizon_days {
        60 => 0.05,
        20 => 0.03,
        _ => 0.0,
    }
}

fn regime_floor_min_gap_vs_normal(horizon_days: u32) -> f64 {
    match horizon_days {
        60 => 0.02,
        20 => 0.01,
        _ => 0.0,
    }
}

fn threshold_has_usable_early_warning_support(
    hits: ProbabilityThresholdRegimeHitSummary,
    horizon_days: u32,
) -> bool {
    hits.early_warning_hit_count > 0
        && hits.early_warning_hit_rate() >= regime_floor_min_hit_rate(horizon_days)
        && (hits.early_warning_hit_rate() - hits.normal_hit_rate())
            >= regime_floor_min_gap_vs_normal(horizon_days)
}

fn adjust_probability_decision_threshold_for_regime_support(
    base_threshold: f64,
    probabilities: &[f64],
    labels: &[f64],
    rows: &[&ProbabilityTrainingRow],
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) -> f64 {
    if label_mode != ProbabilityTargetLabelMode::ForwardCrisis
        || !matches!(horizon_days, 20 | 60)
        || probabilities.is_empty()
        || rows.is_empty()
        || probabilities.len() != rows.len()
    {
        return base_threshold;
    }

    let Some(regime_summary) =
        evaluate_regime_separation_summary_refs(probabilities, rows, horizon_days, label_mode)
    else {
        return base_threshold;
    };

    let base_hits =
        probability_threshold_regime_hit_summary(probabilities, rows, horizon_days, base_threshold);
    if threshold_has_usable_early_warning_support(base_hits, horizon_days) {
        return base_threshold;
    }
    if regime_summary
        .early_warning_lift_vs_normal
        .unwrap_or_default()
        < 1.5
    {
        return base_threshold;
    }

    let actual_positive_count = labels.iter().filter(|label| **label >= 0.5).count() as u32;
    let positive_count = actual_positive_count as f64;
    if positive_count <= 0.0 {
        return base_threshold;
    }

    let early_warning_regime = probability_early_warning_regime(horizon_days);
    let early_warning_probability_cap = probabilities
        .iter()
        .zip(rows.iter().copied())
        .filter(|(_, row)| row.regime_for_horizon(horizon_days) == early_warning_regime)
        .map(|(probability, _)| *probability)
        .fold(0.0_f64, f64::max);

    let relaxed_prediction_ceiling =
        regime_aware_threshold_prediction_ceiling(actual_positive_count, horizon_days);
    let beta_sq = probability_threshold_beta_sq(horizon_days);
    let mut best_score = None::<(bool, bool, i64, i64, i64, i64, i64, i64, i64)>;
    let mut best_threshold = base_threshold;

    for threshold in probability_decision_threshold_candidates(probabilities) {
        if threshold >= base_threshold {
            continue;
        }

        let hits =
            probability_threshold_regime_hit_summary(probabilities, rows, horizon_days, threshold);
        let early_warning_hit_rate = hits.early_warning_hit_rate();
        if hits.early_warning_hit_count == 0 {
            continue;
        }

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
        if predicted_positive_count == 0 || true_positive_count == 0 {
            continue;
        }

        let precision = true_positive_count as f64 / predicted_positive_count as f64;
        let recall = true_positive_count as f64 / positive_count;
        let f_beta = if precision > 0.0 || recall > 0.0 {
            (1.0 + beta_sq) * precision * recall / (beta_sq * precision + recall).max(1e-9)
        } else {
            0.0
        };

        let normal_hit_rate = hits.normal_hit_rate();
        let cooldown_hit_rate = hits.cooldown_hit_rate();
        let score = (
            early_warning_hit_rate >= regime_floor_min_hit_rate(horizon_days),
            predicted_positive_count <= relaxed_prediction_ceiling,
            ((early_warning_hit_rate - normal_hit_rate) * 1_000_000.0).round() as i64,
            ((hits.positive_window_hit_rate() - cooldown_hit_rate) * 1_000_000.0).round() as i64,
            ((hits.in_crisis_hit_rate() - cooldown_hit_rate) * 1_000_000.0).round() as i64,
            (f_beta * 1_000_000.0).round() as i64,
            (precision * 1_000_000.0).round() as i64,
            (recall * 1_000_000.0).round() as i64,
            -((threshold * 1_000.0).round() as i64),
        );
        if best_score.is_none_or(|best| score > best) {
            best_score = Some(score);
            best_threshold = threshold;
        }
    }

    let repaired_threshold =
        if early_warning_probability_cap > 0.0 && early_warning_probability_cap < base_threshold {
            best_threshold.min(early_warning_probability_cap)
        } else {
            best_threshold
        };

    round3(repaired_threshold).clamp(0.005, base_threshold)
}

fn probability_threshold_decision_metrics(
    probabilities: &[f64],
    labels: &[f64],
    rows: &[&ProbabilityTrainingRow],
    horizon_days: u32,
    threshold: f64,
) -> ProbabilityThresholdDecisionMetrics {
    let regime_hits =
        probability_threshold_regime_hit_summary(probabilities, rows, horizon_days, threshold);
    let mut predicted_positive_count = 0_u32;
    let mut true_positive_count = 0_u32;
    let positive_count = labels.iter().filter(|label| **label >= 0.5).count() as f64;

    for (probability, label) in probabilities.iter().zip(labels) {
        if *probability >= threshold {
            predicted_positive_count += 1;
            if *label >= 0.5 {
                true_positive_count += 1;
            }
        }
    }

    ProbabilityThresholdDecisionMetrics {
        regime_hits,
        predicted_positive_count,
        true_positive_count,
        precision: safe_divide(true_positive_count as f64, predicted_positive_count as f64),
        recall: safe_divide(true_positive_count as f64, positive_count),
    }
}

fn probability_threshold_decision_summary_wire(
    metrics: ProbabilityThresholdDecisionMetrics,
) -> ProbabilityThresholdDecisionSummaryWire {
    ProbabilityThresholdDecisionSummaryWire {
        predicted_positive_count: metrics.predicted_positive_count,
        true_positive_count: metrics.true_positive_count,
        precision: round3(metrics.precision),
        recall: round3(metrics.recall),
        early_warning_row_count: metrics.regime_hits.early_warning_row_count,
        early_warning_hit_count: metrics.regime_hits.early_warning_hit_count,
        early_warning_hit_rate: round3(metrics.regime_hits.early_warning_hit_rate()),
        normal_row_count: metrics.regime_hits.normal_row_count,
        normal_hit_count: metrics.regime_hits.normal_hit_count,
        normal_hit_rate: round3(metrics.regime_hits.normal_hit_rate()),
        positive_window_row_count: metrics.regime_hits.positive_window_row_count,
        positive_window_hit_count: metrics.regime_hits.positive_window_hit_count,
        positive_window_hit_rate: round3(metrics.regime_hits.positive_window_hit_rate()),
        in_crisis_row_count: metrics.regime_hits.in_crisis_row_count,
        in_crisis_hit_count: metrics.regime_hits.in_crisis_hit_count,
        in_crisis_hit_rate: round3(metrics.regime_hits.in_crisis_hit_rate()),
        cooldown_row_count: metrics.regime_hits.cooldown_row_count,
        cooldown_hit_count: metrics.regime_hits.cooldown_hit_count,
        cooldown_hit_rate: round3(metrics.regime_hits.cooldown_hit_rate()),
    }
}

fn build_probability_threshold_diagnostics(
    input: ProbabilityThresholdDiagnosticsInput<'_>,
) -> ProbabilityThresholdDiagnosticsWire {
    let ProbabilityThresholdDiagnosticsInput {
        full_calibration_rows,
        calibration_selection,
        threshold_selection,
        horizon_days,
        label_mode,
        base_threshold,
        final_threshold,
    } = input;
    let early_warning_regime = probability_early_warning_regime(horizon_days);
    let probabilities = &threshold_selection.probabilities;
    let labels = &threshold_selection.labels;
    let selected_positive_count = labels.iter().filter(|label| **label >= 0.5).count();
    let selected_negative_count = labels.len().saturating_sub(selected_positive_count);
    let actual_positive_count = selected_positive_count as u32;
    let prediction_ceiling = (actual_positive_count > 0).then(|| {
        probability_prediction_count_ceiling_from_actual_positive_count(
            actual_positive_count,
            horizon_days,
        )
    });
    let relaxed_prediction_ceiling = (label_mode == ProbabilityTargetLabelMode::ForwardCrisis
        && matches!(horizon_days, 20 | 60)
        && actual_positive_count > 0)
        .then(|| regime_aware_threshold_prediction_ceiling(actual_positive_count, horizon_days));
    let early_warning_probability_cap = probabilities
        .iter()
        .zip(threshold_selection.rows.iter().copied())
        .filter(|(_, row)| row.regime_for_horizon(horizon_days) == early_warning_regime)
        .map(|(probability, _)| *probability)
        .max_by(f64::total_cmp);
    let base_metrics = probability_threshold_decision_metrics(
        probabilities,
        labels,
        &threshold_selection.rows,
        horizon_days,
        base_threshold,
    );
    let final_metrics = probability_threshold_decision_metrics(
        probabilities,
        labels,
        &threshold_selection.rows,
        horizon_days,
        final_threshold,
    );
    let regime_summary = evaluate_regime_separation_summary_refs(
        probabilities,
        &threshold_selection.rows,
        horizon_days,
        label_mode,
    );
    let repair_eligible = label_mode == ProbabilityTargetLabelMode::ForwardCrisis
        && matches!(horizon_days, 20 | 60)
        && !probabilities.is_empty()
        && !threshold_selection.rows.is_empty()
        && probabilities.len() == threshold_selection.rows.len();
    let repair_applied = (final_threshold - base_threshold).abs() >= 0.000_5;
    let repair_reason = if !repair_eligible {
        "not_applicable".to_string()
    } else if base_metrics.regime_hits.early_warning_row_count == 0 {
        "no_early_warning_rows".to_string()
    } else if threshold_has_usable_early_warning_support(base_metrics.regime_hits, horizon_days) {
        "base_threshold_has_usable_early_warning_gap".to_string()
    } else if regime_summary
        .as_ref()
        .and_then(|summary| summary.early_warning_lift_vs_normal)
        .unwrap_or_default()
        < 1.5
    {
        "early_warning_lift_below_guardrail".to_string()
    } else if base_metrics.regime_hits.early_warning_hit_count > 0 {
        "base_hits_early_warning_but_gap_is_too_weak".to_string()
    } else if actual_positive_count == 0 {
        "no_positive_labels".to_string()
    } else if !repair_applied {
        "repair_considered_but_no_better_candidate".to_string()
    } else if early_warning_probability_cap
        .is_some_and(|cap| cap < base_threshold && (final_threshold - cap).abs() < 0.000_5)
    {
        "repaired_to_early_warning_cap".to_string()
    } else {
        "repaired_to_regime_support_candidate".to_string()
    };

    ProbabilityThresholdDiagnosticsWire {
        label_mode: label_mode.as_str().to_string(),
        early_warning_regime: probability_training_regime_name(early_warning_regime).to_string(),
        full_calibration_row_count: full_calibration_rows.len(),
        eligible_row_count: calibration_selection.eligible_row_count,
        eligible_positive_count: calibration_selection.eligible_positive_count,
        eligible_negative_count: calibration_selection.eligible_negative_count,
        used_full_split_fallback: calibration_selection.used_full_split_fallback,
        selected_row_count: threshold_selection.rows.len(),
        selected_positive_count,
        selected_negative_count,
        selected_used_full_split_fallback: threshold_selection.used_full_split_fallback,
        base_threshold: round3(base_threshold),
        final_threshold: round3(final_threshold),
        repair_applied,
        repair_eligible,
        repair_reason,
        early_warning_probability_cap: early_warning_probability_cap.map(round3),
        prediction_ceiling,
        relaxed_prediction_ceiling,
        base_summary: probability_threshold_decision_summary_wire(base_metrics),
        final_summary: probability_threshold_decision_summary_wire(final_metrics),
    }
}

fn probability_prediction_count_ceiling_from_actual_positive_count(
    actual_positive_count: u32,
    horizon_days: u32,
) -> u32 {
    let multiple = match horizon_days {
        5 => 4_u32,
        20 => 4_u32,
        60 => 5_u32,
        _ => 5_u32,
    };
    actual_positive_count.max(1).saturating_mul(multiple)
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
    let uses_interaction_tail = feature_names.iter().any(|feature_name| {
        feature_name.contains("interaction__") || feature_name.contains("tail_")
    });
    let feature_stats = feature_names
        .iter()
        .map(|feature| build_feature_stat(rows, feature))
        .collect::<Vec<_>>();
    let regime_pairwise_targets = forward_crisis_regime_pairwise_targets(
        rows,
        &feature_stats,
        horizon_days,
        label_mode,
        uses_interaction_tail,
    );
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
            let label = probability_training_target_label(row, horizon_days, label_mode);
            let sample_weight =
                logistic_sample_weight(row, horizon_days, positive_class_weight, label_mode);
            let error = (prediction - label) * sample_weight;
            intercept_gradient += error;
            for (index, value) in normalized.iter().enumerate() {
                weight_gradients[index] += error * value;
            }
        }
        apply_forward_crisis_sign_gradient(
            &mut weight_gradients,
            &weights,
            feature_names,
            sample_weight_sum,
            horizon_days,
            label_mode,
        );
        apply_regime_pairwise_gradient(
            &mut weight_gradients,
            &weights,
            &regime_pairwise_targets,
            sample_weight_sum,
            horizon_days,
            uses_interaction_tail,
        );
        intercept -= learning_rate * intercept_gradient / sample_weight_sum;
        for (index, weight) in weights.iter_mut().enumerate() {
            *weight -=
                learning_rate * ((weight_gradients[index] / sample_weight_sum) + l2 * *weight);
        }
        project_forward_crisis_sign_constraints(
            &mut weights,
            feature_names,
            horizon_days,
            label_mode,
        );
    }

    LogisticProbabilityModel {
        intercept,
        feature_transform: if uses_interaction_tail {
            PROBABILITY_FEATURE_TRANSFORM_INTERACTION_TAIL_V1.to_string()
        } else {
            PROBABILITY_FEATURE_TRANSFORM_IDENTITY_V1.to_string()
        },
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

#[derive(Debug, Clone)]
struct RegimePairwiseTarget {
    left_centroid: Vec<f64>,
    right_centroid: Vec<f64>,
    margin: f64,
    weight: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExpectedCoefficientSign {
    Positive,
    Negative,
}

fn forward_crisis_expected_coefficient_sign(
    feature_name: &str,
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) -> Option<ExpectedCoefficientSign> {
    if label_mode != ProbabilityTargetLabelMode::ForwardCrisis || horizon_days < 20 {
        return None;
    }

    match feature_name {
        "overall_score"
        | "structural_score"
        | "trigger_score"
        | "external_dimension_score"
        | "us_vix_level"
        | "us_vix_change_5d"
        | "us_baa_10y_spread_level"
        | "us_fed_funds_level"
        | "us_nfci_level"
        | "us_stlfsi_level"
        | "us_unemployment_level" => Some(ExpectedCoefficientSign::Positive),
        "us_curve_10y2y_level" | "us_housing_starts_level" => {
            Some(ExpectedCoefficientSign::Negative)
        }
        _ => None,
    }
}

fn forward_crisis_sign_constraint_strength(
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) -> f64 {
    if label_mode != ProbabilityTargetLabelMode::ForwardCrisis {
        return 0.0;
    }
    match horizon_days {
        20 => 0.55,
        60 => 0.70,
        _ => 0.0,
    }
}

fn apply_forward_crisis_sign_gradient(
    weight_gradients: &mut [f64],
    weights: &[f64],
    feature_names: &[String],
    sample_weight_sum: f64,
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) {
    let strength = forward_crisis_sign_constraint_strength(horizon_days, label_mode);
    if strength <= 0.0 {
        return;
    }

    for ((gradient, weight), feature_name) in weight_gradients
        .iter_mut()
        .zip(weights.iter())
        .zip(feature_names.iter())
    {
        let Some(expected_sign) =
            forward_crisis_expected_coefficient_sign(feature_name, horizon_days, label_mode)
        else {
            continue;
        };
        let violates_sign = match expected_sign {
            ExpectedCoefficientSign::Positive => *weight < 0.0,
            ExpectedCoefficientSign::Negative => *weight > 0.0,
        };
        if violates_sign {
            *gradient += *weight * sample_weight_sum * strength;
        }
    }
}

fn project_forward_crisis_sign_constraints(
    weights: &mut [f64],
    feature_names: &[String],
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) {
    if forward_crisis_sign_constraint_strength(horizon_days, label_mode) <= 0.0 {
        return;
    }

    for (weight, feature_name) in weights.iter_mut().zip(feature_names.iter()) {
        let Some(expected_sign) =
            forward_crisis_expected_coefficient_sign(feature_name, horizon_days, label_mode)
        else {
            continue;
        };
        match expected_sign {
            ExpectedCoefficientSign::Positive if *weight < 0.0 => *weight = 0.0,
            ExpectedCoefficientSign::Negative if *weight > 0.0 => *weight = 0.0,
            _ => {}
        }
    }
}

fn forward_crisis_regime_pairwise_targets(
    rows: &[ProbabilityTrainingRow],
    feature_stats: &[ProbabilityFeatureStat],
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
    uses_interaction_tail: bool,
) -> Vec<RegimePairwiseTarget> {
    if !matches!(label_mode, ProbabilityTargetLabelMode::ForwardCrisis) {
        return Vec::new();
    }

    let target_specs = match horizon_days {
        5 if uses_interaction_tail => vec![
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::Normal,
                0.45,
                1.35,
            ),
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::PostCrisisCooldown,
                0.30,
                1.05,
            ),
            (
                ProbabilityTrainingRegime::PreWarningBuffer,
                ProbabilityTrainingRegime::Normal,
                0.15,
                0.50,
            ),
        ],
        20 => vec![
            (
                ProbabilityTrainingRegime::PreWarningBuffer,
                ProbabilityTrainingRegime::Normal,
                if uses_interaction_tail { 1.00 } else { 0.85 },
                if uses_interaction_tail { 1.40 } else { 1.25 },
            ),
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::Normal,
                if uses_interaction_tail { 0.55 } else { 0.40 },
                if uses_interaction_tail { 1.00 } else { 0.85 },
            ),
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::PreWarningBuffer,
                if uses_interaction_tail { 0.40 } else { 0.35 },
                if uses_interaction_tail { 0.80 } else { 0.70 },
            ),
            (
                ProbabilityTrainingRegime::PreWarningBuffer,
                ProbabilityTrainingRegime::PostCrisisCooldown,
                if uses_interaction_tail { 0.90 } else { 0.70 },
                if uses_interaction_tail { 1.25 } else { 1.10 },
            ),
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::PostCrisisCooldown,
                if uses_interaction_tail { 0.70 } else { 0.45 },
                if uses_interaction_tail { 1.05 } else { 0.80 },
            ),
        ],
        60 => vec![
            (
                ProbabilityTrainingRegime::PreWarningBuffer,
                ProbabilityTrainingRegime::Normal,
                if uses_interaction_tail { 1.25 } else { 1.05 },
                if uses_interaction_tail { 1.55 } else { 1.30 },
            ),
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::Normal,
                if uses_interaction_tail { 0.85 } else { 0.65 },
                if uses_interaction_tail { 1.20 } else { 0.95 },
            ),
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::PreWarningBuffer,
                if uses_interaction_tail { 0.60 } else { 0.45 },
                if uses_interaction_tail { 0.80 } else { 0.60 },
            ),
            (
                ProbabilityTrainingRegime::PreWarningBuffer,
                ProbabilityTrainingRegime::PostCrisisCooldown,
                if uses_interaction_tail { 1.15 } else { 0.90 },
                if uses_interaction_tail { 1.60 } else { 1.30 },
            ),
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::PostCrisisCooldown,
                if uses_interaction_tail { 1.20 } else { 0.95 },
                if uses_interaction_tail { 1.30 } else { 1.00 },
            ),
        ],
        _ => Vec::new(),
    };

    target_specs
        .into_iter()
        .filter_map(|(left, right, margin, weight)| {
            let left_centroid = regime_centroid(rows, feature_stats, horizon_days, left)?;
            let right_centroid = regime_centroid(rows, feature_stats, horizon_days, right)?;
            Some(RegimePairwiseTarget {
                left_centroid,
                right_centroid,
                margin,
                weight,
            })
        })
        .collect()
}

fn regime_centroid(
    rows: &[ProbabilityTrainingRow],
    feature_stats: &[ProbabilityFeatureStat],
    horizon_days: u32,
    regime: ProbabilityTrainingRegime,
) -> Option<Vec<f64>> {
    let feature_len = feature_stats.len();
    let mut sum = vec![0.0; feature_len];
    let mut count = 0_usize;
    for row in rows {
        if row.regime_for_horizon(horizon_days) != regime {
            continue;
        }
        let normalized = normalized_features(row, feature_stats);
        for (index, value) in normalized.into_iter().enumerate() {
            sum[index] += value;
        }
        count += 1;
    }
    (count > 0).then(|| {
        sum.into_iter()
            .map(|value| value / count as f64)
            .collect::<Vec<_>>()
    })
}

fn regime_pairwise_strength(horizon_days: u32, uses_interaction_tail: bool) -> f64 {
    match (horizon_days, uses_interaction_tail) {
        (5, true) => 0.70,
        (20, true) => 1.00,
        (60, true) => 1.35,
        (20, false) => 0.80,
        (60, false) => 1.15,
        _ => 0.0,
    }
}

fn apply_regime_pairwise_gradient(
    weight_gradients: &mut [f64],
    weights: &[f64],
    targets: &[RegimePairwiseTarget],
    sample_weight_sum: f64,
    horizon_days: u32,
    uses_interaction_tail: bool,
) {
    if targets.is_empty() {
        return;
    }
    let strength = regime_pairwise_strength(horizon_days, uses_interaction_tail);
    if strength <= 0.0 {
        return;
    }
    let scale = sample_weight_sum * strength / targets.len() as f64;
    for target in targets {
        let left_logit = dot(weights, &target.left_centroid);
        let right_logit = dot(weights, &target.right_centroid);
        let pressure = sigmoid(right_logit + target.margin - left_logit);
        for (index, gradient) in weight_gradients.iter_mut().enumerate() {
            *gradient += target.weight
                * pressure
                * (target.right_centroid[index] - target.left_centroid[index])
                * scale;
        }
    }
}

fn build_feature_stat(
    rows: &[ProbabilityTrainingRow],
    feature_name: &str,
) -> ProbabilityFeatureStat {
    let values = rows
        .iter()
        .map(|row| {
            resolve_probability_feature_value(feature_name, &row.features).unwrap_or_default()
        })
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
            let label = probability_training_target_label(row, horizon_days, label_mode);
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
    let positive_units = match label_mode {
        ProbabilityTargetLabelMode::ForwardCrisis => rows
            .iter()
            .filter(|row| row.label_for_horizon(label_mode, horizon_days) > 0.0)
            .map(|row| forward_crisis_positive_sample_weight(row, horizon_days))
            .sum::<f64>(),
        _ => rows
            .iter()
            .filter(|row| row.label_for_horizon(label_mode, horizon_days) > 0.0)
            .count() as f64,
    };
    let negative_weight = rows
        .iter()
        .filter(|row| row.label_for_horizon(label_mode, horizon_days) <= 0.0)
        .map(|row| negative_sample_weight(row, horizon_days, label_mode))
        .sum::<f64>();
    if positive_units <= 0.0 || negative_weight <= 0.0 {
        return 1.0;
    }

    let imbalance_weight = (negative_weight / positive_units).sqrt();
    let (horizon_emphasis, cap) = match label_mode {
        ProbabilityTargetLabelMode::ActionWindow | ProbabilityTargetLabelMode::ActionEpisode => {
            match horizon_days {
                5 => (0.65, 6.0),
                20 => (0.75, 7.0),
                60 => (0.85, 8.0),
                _ => (0.75, 7.0),
            }
        }
        ProbabilityTargetLabelMode::ForwardCrisis => match horizon_days {
            5 => (0.9, 18.0),
            20 => (1.15, 18.0),
            60 => (1.35, 18.0),
            _ => (1.0, 18.0),
        },
    };
    (imbalance_weight * horizon_emphasis).clamp(1.0, cap)
}

fn probability_training_target_label(
    row: &ProbabilityTrainingRow,
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) -> f64 {
    let hard_label = row.label_for_horizon(label_mode, horizon_days);
    if hard_label > 0.0 || label_mode != ProbabilityTargetLabelMode::ForwardCrisis {
        return hard_label;
    }

    match row.regime_for_horizon(horizon_days) {
        ProbabilityTrainingRegime::Normal => 0.0,
        ProbabilityTrainingRegime::PreWarningBuffer => match horizon_days {
            20 => 0.18,
            60 => 0.26,
            _ => 0.0,
        },
        ProbabilityTrainingRegime::PositiveWindow => match horizon_days {
            20 => 0.24,
            60 => 0.32,
            _ => 0.0,
        },
        ProbabilityTrainingRegime::InCrisis => match horizon_days {
            20 => 0.08,
            60 => 0.12,
            _ => 0.0,
        },
        ProbabilityTrainingRegime::PostCrisisCooldown => match horizon_days {
            20 => 0.01,
            60 => 0.02,
            _ => 0.0,
        },
    }
}

fn logistic_sample_weight(
    row: &ProbabilityTrainingRow,
    horizon_days: u32,
    positive_class_weight: f64,
    label_mode: ProbabilityTargetLabelMode,
) -> f64 {
    let label = row.label_for_horizon(label_mode, horizon_days);
    if label > 0.0 {
        let positive_weight = match label_mode {
            ProbabilityTargetLabelMode::ForwardCrisis => {
                forward_crisis_positive_sample_weight(row, horizon_days)
            }
            _ => positive_sample_action_weight(row, horizon_days),
        };
        (positive_class_weight * positive_weight).clamp(1.0, 36.0)
    } else {
        negative_sample_weight(row, horizon_days, label_mode)
    }
}

fn forward_crisis_regime_sample_weight(
    horizon_days: u32,
    regime: ProbabilityTrainingRegime,
) -> f64 {
    match regime {
        ProbabilityTrainingRegime::Normal => 1.0,
        ProbabilityTrainingRegime::PreWarningBuffer => match horizon_days {
            5 => 0.90,
            20 => 0.60,
            60 => 0.50,
            _ => 0.70,
        },
        ProbabilityTrainingRegime::PositiveWindow => match horizon_days {
            5 => 2.0,
            20 => 2.2,
            60 => 1.8,
            _ => 2.0,
        },
        ProbabilityTrainingRegime::InCrisis => match horizon_days {
            5 => 1.15,
            20 => 1.20,
            60 => 1.15,
            _ => 1.15,
        },
        ProbabilityTrainingRegime::PostCrisisCooldown => match horizon_days {
            5 => 1.10,
            20 => 1.35,
            60 => 1.60,
            _ => 1.25,
        },
    }
}

fn forward_crisis_positive_sample_weight(row: &ProbabilityTrainingRow, horizon_days: u32) -> f64 {
    (forward_crisis_regime_sample_weight(horizon_days, row.regime_for_horizon(horizon_days))
        * positive_sample_action_weight(row, horizon_days)
        * scenario_training_role_weight_multiplier(
            row.scenario_training_role.as_deref(),
            horizon_days,
        ))
    .clamp(1.0, 12.0)
}

fn forward_crisis_negative_regime_sample_weight(
    horizon_days: u32,
    regime: ProbabilityTrainingRegime,
) -> f64 {
    match regime {
        ProbabilityTrainingRegime::Normal => match horizon_days {
            20 => 1.10,
            60 => 1.15,
            _ => 1.0,
        },
        ProbabilityTrainingRegime::PreWarningBuffer => match horizon_days {
            5 => 0.90,
            20 => 0.70,
            60 => 0.60,
            _ => 0.75,
        },
        ProbabilityTrainingRegime::PositiveWindow => 1.0,
        ProbabilityTrainingRegime::InCrisis => match horizon_days {
            5 => 1.15,
            20 => 1.25,
            60 => 1.20,
            _ => 1.20,
        },
        ProbabilityTrainingRegime::PostCrisisCooldown => match horizon_days {
            5 => 1.10,
            20 => 1.45,
            60 => 1.75,
            _ => 1.40,
        },
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
        ProbabilityTargetLabelMode::ActionEpisode => {
            if row.protected_action_window {
                return 0.55;
            }

            match row.action_episode_phase.as_str() {
                "late_validation" => match horizon_days {
                    5 => 0.95,
                    20 => 0.80,
                    60 => 0.70,
                    _ => 0.80,
                },
                "cooldown" => match horizon_days {
                    5 => 0.70,
                    20 => 0.65,
                    60 => 0.60,
                    _ => 0.65,
                },
                _ => match row.regime_for_horizon(horizon_days) {
                    ProbabilityTrainingRegime::Normal => 1.0,
                    ProbabilityTrainingRegime::PositiveWindow => 1.0,
                    ProbabilityTrainingRegime::PreWarningBuffer => match horizon_days {
                        5 => 0.85,
                        20 => 0.75,
                        60 => 0.65,
                        _ => 0.75,
                    },
                    ProbabilityTrainingRegime::InCrisis => match horizon_days {
                        5 => 1.15,
                        20 => 1.05,
                        60 => 0.95,
                        _ => 1.0,
                    },
                    ProbabilityTrainingRegime::PostCrisisCooldown => match horizon_days {
                        5 => 0.75,
                        20 => 0.70,
                        60 => 0.65,
                        _ => 0.70,
                    },
                },
            }
        }
        ProbabilityTargetLabelMode::ForwardCrisis => {
            if row.protected_action_window {
                return match row.action_episode_phase.as_str() {
                    "primary" => match horizon_days {
                        5 => 0.95,
                        20 => 0.55,
                        60 => 0.65,
                        _ => 0.55,
                    },
                    "late_validation" => match horizon_days {
                        5 => 0.95,
                        20 => 0.70,
                        60 => 0.80,
                        _ => 0.65,
                    },
                    "cooldown" => match horizon_days {
                        5 => 1.05,
                        20 => 1.20,
                        60 => 1.35,
                        _ => 1.00,
                    },
                    _ => match horizon_days {
                        5 => 0.95,
                        20 => 0.80,
                        60 => 0.90,
                        _ => 0.75,
                    },
                };
            }
            forward_crisis_negative_regime_sample_weight(
                horizon_days,
                row.regime_for_horizon(horizon_days),
            )
        }
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

fn scenario_training_role_weight_multiplier(
    scenario_training_role: Option<&str>,
    horizon_days: u32,
) -> f64 {
    match (horizon_days, scenario_training_role) {
        (_, Some("mandatory")) => 1.0,
        (5, Some("candidate_optional")) => 1.10,
        (20, Some("candidate_optional")) => 1.30,
        (60, Some("candidate_optional")) => 1.45,
        (5, Some("extension_only")) => 1.45,
        (20, Some("extension_only")) => 1.65,
        (60, Some("extension_only")) => 1.70,
        (_, Some("no_positive_main")) => 1.0,
        _ => 1.0,
    }
}

fn scenario_family_weight_multiplier(scenario_family: Option<&str>, horizon_days: u32) -> f64 {
    match (horizon_days, scenario_family) {
        (5, Some("acute_market_liquidity_crash")) => 1.50,
        (5, Some("systemic_credit_banking_crisis")) => 0.80,
        (5, Some("mixed_systemic_stress")) => 0.85,
        (5, Some("rate_shock_or_policy_dislocation")) => 0.85,
        (20, Some("acute_market_liquidity_crash")) => 1.30,
        (20, Some("systemic_credit_banking_crisis")) => 1.15,
        (20, Some("mixed_systemic_stress")) => 1.35,
        (20, Some("rate_shock_or_policy_dislocation")) => 1.25,
        (60, Some("acute_market_liquidity_crash")) => 0.85,
        (60, Some("systemic_credit_banking_crisis")) => 1.25,
        (60, Some("mixed_systemic_stress")) => 1.45,
        (60, Some("rate_shock_or_policy_dislocation")) => 1.35,
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
            let value = resolve_probability_feature_value(&stat.name, &row.features)
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
        regime_separation: None,
        actionability: None,
    }
}

fn evaluate_probabilities_for_rows(
    probabilities: &[f64],
    rows: &[ProbabilityTrainingRow],
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) -> HorizonEvaluationSummary {
    let labels = rows
        .iter()
        .map(|row| row.label_for_horizon(label_mode, horizon_days))
        .collect::<Vec<_>>();
    let mut summary = evaluate_probabilities(probabilities, &labels);
    let row_refs = rows.iter().collect::<Vec<_>>();
    summary.regime_separation =
        evaluate_regime_separation_summary_refs(probabilities, &row_refs, horizon_days, label_mode);
    summary
}

fn evaluate_regime_separation_summary_refs(
    probabilities: &[f64],
    rows: &[&ProbabilityTrainingRow],
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
) -> Option<RegimeSeparationEvaluationSummary> {
    if label_mode != ProbabilityTargetLabelMode::ForwardCrisis
        || probabilities.is_empty()
        || rows.is_empty()
    {
        return None;
    }

    #[derive(Default, Clone, Copy)]
    struct Bucket {
        sample_count: u32,
        probability_sum: f64,
    }

    let mut buckets = BTreeMap::<ProbabilityTrainingRegime, Bucket>::new();
    for (probability, row) in probabilities.iter().zip(rows.iter().copied()) {
        let bucket = buckets
            .entry(row.regime_for_horizon(horizon_days))
            .or_default();
        bucket.sample_count += 1;
        bucket.probability_sum += *probability;
    }

    let average_probability = |regime: ProbabilityTrainingRegime| {
        buckets
            .get(&regime)
            .map(|bucket| safe_divide(bucket.probability_sum, bucket.sample_count as f64))
    };
    let sample_count = |regime: ProbabilityTrainingRegime| {
        buckets.get(&regime).map_or(0, |bucket| bucket.sample_count)
    };

    let early_warning_regime = match horizon_days {
        5 => ProbabilityTrainingRegime::PositiveWindow,
        20 | 60 => ProbabilityTrainingRegime::PreWarningBuffer,
        _ => ProbabilityTrainingRegime::PositiveWindow,
    };
    let normal_avg = average_probability(ProbabilityTrainingRegime::Normal)?;
    let pre_warning_buffer_avg =
        average_probability(ProbabilityTrainingRegime::PreWarningBuffer).unwrap_or(0.0);
    let positive_window_avg =
        average_probability(ProbabilityTrainingRegime::PositiveWindow).unwrap_or(0.0);
    let early_warning_avg = average_probability(early_warning_regime).unwrap_or(0.0);
    let in_crisis_avg = average_probability(ProbabilityTrainingRegime::InCrisis).unwrap_or(0.0);
    let post_crisis_cooldown_avg =
        average_probability(ProbabilityTrainingRegime::PostCrisisCooldown).unwrap_or(0.0);
    let max_non_normal_avg = buckets
        .iter()
        .filter(|(regime, _)| **regime != ProbabilityTrainingRegime::Normal)
        .map(|(_, bucket)| safe_divide(bucket.probability_sum, bucket.sample_count as f64))
        .fold(0.0_f64, f64::max);
    let pre_warning_buffer_lift_vs_normal = lift_vs_baseline(pre_warning_buffer_avg, normal_avg);
    let positive_window_lift_vs_normal = lift_vs_baseline(positive_window_avg, normal_avg);
    let early_warning_lift_vs_normal = lift_vs_baseline(early_warning_avg, normal_avg);
    let in_crisis_lift_vs_normal = lift_vs_baseline(in_crisis_avg, normal_avg);
    let post_crisis_cooldown_lift_vs_normal =
        lift_vs_baseline(post_crisis_cooldown_avg, normal_avg);
    let positive_window_gap_vs_normal = round6(positive_window_avg - normal_avg);
    let post_crisis_cooldown_gap_vs_normal = round6(post_crisis_cooldown_avg - normal_avg);
    let max_non_normal_lift_vs_normal = lift_vs_baseline(max_non_normal_avg, normal_avg);
    let diagnosis = classify_probability_regime_separation(
        horizon_days,
        pre_warning_buffer_lift_vs_normal.unwrap_or_default(),
        positive_window_lift_vs_normal.unwrap_or_default(),
        early_warning_lift_vs_normal.unwrap_or_default(),
        in_crisis_lift_vs_normal.unwrap_or_default(),
        post_crisis_cooldown_lift_vs_normal.unwrap_or_default(),
        positive_window_gap_vs_normal,
        post_crisis_cooldown_gap_vs_normal,
        max_non_normal_lift_vs_normal.unwrap_or_default(),
    )
    .to_string();

    Some(RegimeSeparationEvaluationSummary {
        horizon_days,
        early_warning_regime: probability_training_regime_name(early_warning_regime).to_string(),
        normal_sample_count: sample_count(ProbabilityTrainingRegime::Normal),
        pre_warning_buffer_sample_count: sample_count(ProbabilityTrainingRegime::PreWarningBuffer),
        positive_window_sample_count: sample_count(ProbabilityTrainingRegime::PositiveWindow),
        early_warning_sample_count: sample_count(early_warning_regime),
        in_crisis_sample_count: sample_count(ProbabilityTrainingRegime::InCrisis),
        post_crisis_cooldown_sample_count: sample_count(
            ProbabilityTrainingRegime::PostCrisisCooldown,
        ),
        normal_avg_probability: round6(normal_avg),
        pre_warning_buffer_avg_probability: round6(pre_warning_buffer_avg),
        positive_window_avg_probability: round6(positive_window_avg),
        early_warning_avg_probability: round6(early_warning_avg),
        in_crisis_avg_probability: round6(in_crisis_avg),
        post_crisis_cooldown_avg_probability: round6(post_crisis_cooldown_avg),
        max_non_normal_avg_probability: round6(max_non_normal_avg),
        pre_warning_buffer_lift_vs_normal,
        positive_window_lift_vs_normal,
        early_warning_lift_vs_normal,
        in_crisis_lift_vs_normal,
        post_crisis_cooldown_lift_vs_normal,
        positive_window_gap_vs_normal: Some(positive_window_gap_vs_normal),
        post_crisis_cooldown_gap_vs_normal: Some(post_crisis_cooldown_gap_vs_normal),
        max_non_normal_lift_vs_normal,
        diagnosis,
    })
}

fn regime_positive_window_gap_floor(horizon_days: u32) -> f64 {
    match horizon_days {
        5 => 0.005,
        20 | 60 => 0.010,
        _ => 0.010,
    }
}

#[allow(clippy::too_many_arguments)]
fn classify_probability_regime_separation(
    horizon_days: u32,
    pre_warning_buffer_lift_vs_normal: f64,
    positive_window_lift_vs_normal: f64,
    early_warning_lift_vs_normal: f64,
    in_crisis_lift_vs_normal: f64,
    post_crisis_cooldown_lift_vs_normal: f64,
    positive_window_gap_vs_normal: f64,
    post_crisis_cooldown_gap_vs_normal: f64,
    max_non_normal_lift_vs_normal: f64,
) -> &'static str {
    if max_non_normal_lift_vs_normal < 1.15
        && positive_window_lift_vs_normal < 1.15
        && early_warning_lift_vs_normal < 1.15
    {
        return "cold_across_all_regimes";
    }
    if positive_window_lift_vs_normal < 1.15 && in_crisis_lift_vs_normal >= 1.5 {
        return "late_only_no_early_warning";
    }
    if positive_window_lift_vs_normal >= 1.15
        && post_crisis_cooldown_lift_vs_normal >= positive_window_lift_vs_normal
        && post_crisis_cooldown_gap_vs_normal + 0.002 >= positive_window_gap_vs_normal
    {
        return "cooldown_bleed";
    }
    if positive_window_lift_vs_normal >= 1.5
        && positive_window_gap_vs_normal >= regime_positive_window_gap_floor(horizon_days)
    {
        return "usable_early_warning_separation";
    }
    if max_non_normal_lift_vs_normal >= 1.15 || pre_warning_buffer_lift_vs_normal >= 1.15 {
        return "weak_regime_separation";
    }
    "mixed_or_unclear"
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
    let label_mode = ProbabilityTargetLabelMode::ActionEpisode;
    let mut predicted_positive_count = 0_u32;
    let mut actual_positive_count = 0_u32;
    let mut primary_positive_count = 0_u32;
    let mut late_validation_row_count = 0_u32;
    let mut cooldown_row_count = 0_u32;
    let mut primary_hit_count = 0_u32;
    let mut late_validation_hit_count = 0_u32;
    let mut cooldown_hit_count = 0_u32;
    let mut false_positive_count = 0_u32;
    let mut scenario_states = BTreeMap::<String, ActionabilityScenarioEvaluationState>::new();

    for (probability, row) in probabilities.iter().zip(rows) {
        let predicted_positive = *probability >= threshold;
        let actual_positive = row.label_for_horizon(label_mode, horizon_days) >= 0.5;
        let phase = row.action_episode_phase_for_horizon(horizon_days);

        if predicted_positive {
            predicted_positive_count += 1;
        }

        if actual_positive {
            actual_positive_count += 1;
            primary_positive_count += 1;
            if let Some(scenario_id) = row.primary_scenario_id.as_ref() {
                scenario_states
                    .entry(scenario_id.clone())
                    .or_default()
                    .saw_positive = true;
            }
        } else {
            match phase {
                ActionEpisodePhase::LateValidation => late_validation_row_count += 1,
                ActionEpisodePhase::Cooldown => cooldown_row_count += 1,
                _ => {}
            }
        }

        if predicted_positive {
            if actual_positive {
                primary_hit_count += 1;
                if let Some(scenario_id) = row.primary_scenario_id.as_ref() {
                    let state = scenario_states.entry(scenario_id.clone()).or_default();
                    state.saw_positive = true;
                    state.has_pre_start_hit = true;
                }
            } else if matches!(phase, ActionEpisodePhase::LateValidation) {
                late_validation_hit_count += 1;
                if let Some(scenario_id) = row.primary_scenario_id.as_ref() {
                    let state = scenario_states.entry(scenario_id.clone()).or_default();
                    state.saw_positive = true;
                    state.has_post_start_hit = true;
                }
            } else if matches!(phase, ActionEpisodePhase::Cooldown) {
                cooldown_hit_count += 1;
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

    let hit_count = primary_hit_count + late_validation_hit_count;
    let scenario_count =
        advance_warning_scenario_count + late_confirmation_scenario_count + missed_scenario_count;

    ActionabilityEvaluationSummary {
        threshold: round3(threshold),
        predicted_positive_count,
        actual_positive_count,
        pre_start_positive_count: primary_positive_count,
        post_start_positive_count: late_validation_row_count,
        unclassified_positive_count: cooldown_row_count,
        pre_start_hit_count: primary_hit_count,
        post_start_hit_count: late_validation_hit_count,
        unclassified_hit_count: cooldown_hit_count,
        false_positive_count,
        scenario_count,
        advance_warning_scenario_count,
        late_confirmation_scenario_count,
        missed_scenario_count,
        precision_at_threshold: (predicted_positive_count > 0)
            .then_some(round3(hit_count as f64 / predicted_positive_count as f64)),
        pre_start_recall_at_threshold: (primary_positive_count > 0)
            .then_some(round3(primary_hit_count as f64 / primary_positive_count as f64)),
        post_start_recall_at_threshold: (late_validation_row_count > 0).then_some(round3(
            late_validation_hit_count as f64 / late_validation_row_count as f64,
        )),
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
        note: "Primary means the episode-native action window fired on time; post-start metrics now represent late-validation tracking rather than crisis-start proxy labels.".to_string(),
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
    let regime_separation_summaries = horizons
        .iter()
        .filter_map(|horizon| horizon.evaluation.regime_separation.clone())
        .collect::<Vec<_>>();
    let usable_early_warning_horizon_count = regime_separation_summaries
        .iter()
        .filter(|summary| summary.diagnosis == "usable_early_warning_separation")
        .count() as u32;
    let insufficient_early_warning_horizon_count = regime_separation_summaries
        .iter()
        .filter(|summary| {
            matches!(
                summary.diagnosis.as_str(),
                "cold_across_all_regimes"
                    | "late_only_no_early_warning"
                    | "mixed_or_unclear"
                    | "cooldown_bleed"
            )
        })
        .count() as u32;
    ProbabilityBundleEvaluation {
        sample_count: total_samples as u32,
        brier_score: weighted_brier,
        log_loss: weighted_log_loss,
        ece: weighted_ece,
        regime_separation_summaries,
        usable_early_warning_horizon_count,
        insufficient_early_warning_horizon_count,
        note: format!(
            "Weighted average across 5d / 20d / 60d evaluation slices. Usable early-warning horizons: {usable_early_warning_horizon_count}. Insufficient or cooldown-bleed horizons: {insufficient_early_warning_horizon_count}."
        ),
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

#[derive(Debug, Clone)]
struct ReleaseReviewRuntimeSnapshot {
    assessment: AssessmentSnapshot,
    method: AuditMethodResponseWire,
    history: Vec<AssessmentHistoryPoint>,
}

async fn fetch_release_review_runtime_snapshot(
    api_reload_url: &str,
) -> anyhow::Result<ReleaseReviewRuntimeSnapshot> {
    let api_base_url = api_reload_url
        .strip_suffix("/api/system/reload")
        .with_context(|| {
            format!(
                "cannot derive API base URL from reload URL {api_reload_url}; expected it to end with /api/system/reload"
            )
        })?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()?;
    let assessment: AssessmentSnapshot =
        fetch_api_json(&client, api_base_url, "/api/assessment/current").await?;
    let method: AuditMethodResponseWire =
        fetch_api_json(&client, api_base_url, "/api/assessment/method").await?;
    let history: Vec<AssessmentHistoryPoint> =
        fetch_api_json(&client, api_base_url, "/api/assessment/history?limit=20000").await?;
    Ok(ReleaseReviewRuntimeSnapshot {
        assessment,
        method,
        history,
    })
}

fn build_release_runtime_review_diagnostics(
    release_id: &str,
    label_version: &str,
    method: &AuditMethodResponseWire,
    history: &[AssessmentHistoryPoint],
) -> ReleaseRuntimeReviewDiagnostics {
    let posture_distribution =
        summarize_named_counts(history.iter().map(|point| match point.posture {
            fc_domain::DecisionPosture::Normal => "normal",
            fc_domain::DecisionPosture::Prepare => "prepare",
            fc_domain::DecisionPosture::Hedge => "hedge",
            fc_domain::DecisionPosture::Defend => "defend",
        }));
    let time_bucket_distribution =
        summarize_named_counts(history.iter().map(|point| match point.time_to_risk_bucket {
            fc_domain::TimeToRiskBucket::Normal => "normal",
            fc_domain::TimeToRiskBucket::Months => "months",
            fc_domain::TimeToRiskBucket::Weeks => "weeks",
            fc_domain::TimeToRiskBucket::Now => "now",
        }));
    let posture_trigger_distribution =
        summarize_posture_clause_counts(history, |point| &point.posture_trigger_codes);
    let posture_blocker_distribution =
        summarize_posture_clause_counts(history, |point| &point.posture_blocker_codes);
    let (
        points_at_or_above_prepare_p60d,
        points_at_or_above_hedge_p20d,
        points_at_or_above_defend_p5d,
        mut notes,
    ) = if let Some(thresholds) = method.runtime_thresholds.as_ref() {
        (
            Some(
                history
                    .iter()
                    .filter(|point| point.p_60d >= thresholds.prepare_p60d)
                    .count(),
            ),
            Some(
                history
                    .iter()
                    .filter(|point| point.p_20d >= thresholds.hedge_p20d)
                    .count(),
            ),
            Some(
                history
                    .iter()
                    .filter(|point| point.p_5d >= thresholds.defend_p5d)
                    .count(),
            ),
            vec!["基于运行中 API 返回的 runtime_thresholds 统计历史概率越线次数。".to_string()],
        )
    } else {
        (
            None,
            None,
            None,
            vec![
                "运行中的 API 没有返回 runtime_thresholds；本报告只保留 posture / time bucket 分布。"
                    .to_string(),
            ],
        )
    };
    let regime_probability_summaries = match load_release_review_regime_scenarios(label_version) {
        Ok((scenarios, scenario_note)) => {
            notes.push(scenario_note);
            summarize_release_runtime_regime_probabilities(
                history,
                &scenarios,
                method.runtime_thresholds.as_ref(),
            )
        }
        Err(error) => {
            notes.push(format!(
                "未能加载 release review 所需的 regime scenario catalog，跳过 regime 概率分布：{error:#}"
            ));
            Vec::new()
        }
    };
    let regime_separation_summaries =
        summarize_release_runtime_regime_separation(&regime_probability_summaries);
    if !regime_separation_summaries.is_empty() {
        notes.push(render_release_runtime_separation_note(
            &regime_separation_summaries,
        ));
    }

    ReleaseRuntimeReviewDiagnostics {
        release_id: release_id.to_string(),
        history_point_count: history.len(),
        posture_distribution,
        time_bucket_distribution,
        posture_trigger_distribution,
        posture_blocker_distribution,
        regime_probability_summaries,
        regime_separation_summaries,
        runtime_thresholds: method.runtime_thresholds.clone(),
        points_at_or_above_prepare_p60d,
        points_at_or_above_hedge_p20d,
        points_at_or_above_defend_p5d,
        note: notes.join(" "),
    }
}

fn load_release_review_regime_scenarios(
    label_version: &str,
) -> anyhow::Result<(Vec<CrisisScenario>, String)> {
    match load_label_set_crisis_scenarios(DEFAULT_FORMAL_SCENARIO_SET_VERSION, label_version) {
        Ok(scenarios) => Ok((
            scenarios,
            format!(
                "Regime 概率分布基于 {DEFAULT_FORMAL_SCENARIO_SET_VERSION}/{label_version} 重算。"
            ),
        )),
        Err(primary_error) if label_version == "label_forward_crisis_v1" => {
            let fallback = load_label_set_crisis_scenarios(
                DEFAULT_FORMAL_SCENARIO_SET_VERSION,
                DEFAULT_FORMAL_LABEL_VERSION,
            )?;
            Ok((
                fallback,
                format!(
                    "当前 release label_version={label_version} 不在 scenario catalog 中，Regime 概率分布回退到 {DEFAULT_FORMAL_SCENARIO_SET_VERSION}/{DEFAULT_FORMAL_LABEL_VERSION} 重算（原始错误：{primary_error:#}）。"
                ),
            ))
        }
        Err(error) => Err(error),
    }
}

fn summarize_release_runtime_regime_probabilities(
    history: &[AssessmentHistoryPoint],
    scenarios: &[CrisisScenario],
    runtime_thresholds: Option<&RuntimeThresholdDiagnosticsWire>,
) -> Vec<ReleaseRuntimeRegimeProbabilitySummary> {
    #[derive(Default)]
    struct Accumulator {
        row_count: usize,
        raw_probability_sum: f64,
        max_raw_probability: f64,
        calibrated_probability_sum: f64,
        max_calibrated_probability: f64,
        threshold_hit_count: usize,
    }

    let mut buckets = BTreeMap::<(u32, String), Accumulator>::new();
    for point in history {
        for (horizon_days, raw_probability, calibrated_probability) in [
            (5_u32, point.raw_p_5d.unwrap_or(point.p_5d), point.p_5d),
            (20_u32, point.raw_p_20d.unwrap_or(point.p_20d), point.p_20d),
            (60_u32, point.raw_p_60d.unwrap_or(point.p_60d), point.p_60d),
        ] {
            let regime = probability_training_regime_name(forward_crisis_training_regime(
                point.as_of_date,
                scenarios,
                horizon_days,
            ));
            let bucket = buckets
                .entry((horizon_days, regime.to_string()))
                .or_default();
            bucket.row_count += 1;
            bucket.raw_probability_sum += raw_probability;
            bucket.max_raw_probability = bucket.max_raw_probability.max(raw_probability);
            bucket.calibrated_probability_sum += calibrated_probability;
            bucket.max_calibrated_probability = bucket
                .max_calibrated_probability
                .max(calibrated_probability);
            if let Some(threshold) =
                runtime_probability_threshold_for_horizon(runtime_thresholds, horizon_days)
            {
                if calibrated_probability >= threshold {
                    bucket.threshold_hit_count += 1;
                }
            }
        }
    }

    let normal_baselines = buckets
        .iter()
        .filter_map(|((horizon_days, regime), bucket)| {
            if regime != "normal" {
                return None;
            }
            Some((
                *horizon_days,
                (
                    safe_divide(bucket.raw_probability_sum, bucket.row_count as f64),
                    safe_divide(bucket.calibrated_probability_sum, bucket.row_count as f64),
                ),
            ))
        })
        .collect::<BTreeMap<_, _>>();

    buckets
        .into_iter()
        .map(|((horizon_days, regime), bucket)| {
            let avg_raw_probability =
                safe_divide(bucket.raw_probability_sum, bucket.row_count as f64);
            let avg_calibrated_probability =
                safe_divide(bucket.calibrated_probability_sum, bucket.row_count as f64);
            let (
                raw_lift_vs_normal,
                calibrated_lift_vs_normal,
                raw_gap_vs_normal,
                calibrated_gap_vs_normal,
                calibration_gap_retention,
            ) = if let Some((normal_avg_raw, normal_avg_calibrated)) =
                normal_baselines.get(&horizon_days).copied()
            {
                let raw_gap = avg_raw_probability - normal_avg_raw;
                let calibrated_gap = avg_calibrated_probability - normal_avg_calibrated;
                (
                    lift_vs_baseline(avg_raw_probability, normal_avg_raw),
                    lift_vs_baseline(avg_calibrated_probability, normal_avg_calibrated),
                    Some(round6(raw_gap)),
                    Some(round6(calibrated_gap)),
                    gap_retention_ratio(raw_gap, calibrated_gap),
                )
            } else {
                (None, None, None, None, None)
            };

            ReleaseRuntimeRegimeProbabilitySummary {
                horizon_days,
                regime,
                row_count: bucket.row_count,
                row_rate: round6(safe_ratio(bucket.row_count, history.len())),
                avg_raw_probability: round6(avg_raw_probability),
                max_raw_probability: round6(bucket.max_raw_probability),
                avg_probability: round6(avg_calibrated_probability),
                max_probability: round6(bucket.max_calibrated_probability),
                raw_lift_vs_normal,
                calibrated_lift_vs_normal,
                raw_gap_vs_normal,
                calibrated_gap_vs_normal,
                calibration_gap_retention,
                threshold_hit_count: runtime_thresholds.map(|_| bucket.threshold_hit_count),
            }
        })
        .collect()
}

fn summarize_release_runtime_regime_separation(
    summaries: &[ReleaseRuntimeRegimeProbabilitySummary],
) -> Vec<ReleaseRuntimeSeparationSummary> {
    let mut by_horizon = BTreeMap::<u32, Vec<&ReleaseRuntimeRegimeProbabilitySummary>>::new();
    for summary in summaries {
        by_horizon
            .entry(summary.horizon_days)
            .or_default()
            .push(summary);
    }

    by_horizon
        .into_iter()
        .filter_map(|(horizon_days, rows)| {
            let normal = rows.iter().copied().find(|row| row.regime == "normal")?;
            let pre_warning_buffer = rows
                .iter()
                .copied()
                .find(|row| row.regime == "pre_warning_buffer");
            let positive_window = rows
                .iter()
                .copied()
                .find(|row| row.regime == "positive_window");
            let max_non_normal = rows
                .iter()
                .copied()
                .filter(|row| row.regime != "normal")
                .max_by(|left, right| {
                    left.avg_probability
                        .partial_cmp(&right.avg_probability)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })?;
            let early_warning_regime_name = early_warning_regime_name(horizon_days);
            let early_warning = rows
                .iter()
                .copied()
                .find(|row| row.regime == early_warning_regime_name);
            let in_crisis = rows.iter().copied().find(|row| row.regime == "in_crisis");
            let post_crisis_cooldown = rows
                .iter()
                .copied()
                .find(|row| row.regime == "post_crisis_cooldown");
            let max_non_normal_threshold_hit_rate = max_non_normal
                .threshold_hit_count
                .map(|count| round6(safe_divide(count as f64, max_non_normal.row_count as f64)));
            let diagnosis = classify_regime_separation(
                horizon_days,
                early_warning
                    .and_then(|row| row.raw_lift_vs_normal)
                    .unwrap_or_default(),
                early_warning
                    .and_then(|row| row.calibrated_lift_vs_normal)
                    .unwrap_or_default(),
                early_warning.and_then(|row| row.calibration_gap_retention),
                positive_window
                    .and_then(|row| row.calibrated_lift_vs_normal)
                    .unwrap_or_default(),
                positive_window
                    .and_then(|row| row.calibrated_gap_vs_normal)
                    .unwrap_or_default(),
                in_crisis
                    .and_then(|row| row.calibrated_lift_vs_normal)
                    .unwrap_or_default(),
                post_crisis_cooldown
                    .and_then(|row| row.calibrated_lift_vs_normal)
                    .unwrap_or_default(),
                post_crisis_cooldown
                    .and_then(|row| row.calibrated_gap_vs_normal)
                    .unwrap_or_default(),
                max_non_normal.calibrated_lift_vs_normal.unwrap_or_default(),
                max_non_normal_threshold_hit_rate.unwrap_or_default(),
            )
            .to_string();

            Some(ReleaseRuntimeSeparationSummary {
                horizon_days,
                early_warning_regime: early_warning_regime_name.to_string(),
                normal_avg_probability: normal.avg_probability,
                pre_warning_buffer_avg_probability: pre_warning_buffer
                    .map(|row| row.avg_probability)
                    .unwrap_or_default(),
                positive_window_avg_probability: positive_window
                    .map(|row| row.avg_probability)
                    .unwrap_or_default(),
                in_crisis_avg_probability: in_crisis
                    .map(|row| row.avg_probability)
                    .unwrap_or_default(),
                post_crisis_cooldown_avg_probability: post_crisis_cooldown
                    .map(|row| row.avg_probability)
                    .unwrap_or_default(),
                early_warning_raw_lift_vs_normal: early_warning
                    .and_then(|row| row.raw_lift_vs_normal),
                early_warning_calibrated_lift_vs_normal: early_warning
                    .and_then(|row| row.calibrated_lift_vs_normal),
                early_warning_gap_retention: early_warning
                    .and_then(|row| row.calibration_gap_retention),
                positive_window_calibrated_lift_vs_normal: positive_window
                    .and_then(|row| row.calibrated_lift_vs_normal),
                positive_window_gap_vs_normal: positive_window
                    .and_then(|row| row.calibrated_gap_vs_normal),
                in_crisis_raw_lift_vs_normal: in_crisis.and_then(|row| row.raw_lift_vs_normal),
                in_crisis_calibrated_lift_vs_normal: in_crisis
                    .and_then(|row| row.calibrated_lift_vs_normal),
                post_crisis_cooldown_calibrated_lift_vs_normal: post_crisis_cooldown
                    .and_then(|row| row.calibrated_lift_vs_normal),
                post_crisis_cooldown_gap_vs_normal: post_crisis_cooldown
                    .and_then(|row| row.calibrated_gap_vs_normal),
                max_non_normal_calibrated_lift_vs_normal: max_non_normal.calibrated_lift_vs_normal,
                max_non_normal_threshold_hit_rate,
                diagnosis,
            })
        })
        .collect()
}

fn early_warning_regime_name(horizon_days: u32) -> &'static str {
    match horizon_days {
        5 => "positive_window",
        20 | 60 => "pre_warning_buffer",
        _ => "positive_window",
    }
}

#[allow(clippy::too_many_arguments)]
fn classify_regime_separation(
    horizon_days: u32,
    early_warning_raw_lift: f64,
    early_warning_calibrated_lift: f64,
    early_warning_gap_retention: Option<f64>,
    positive_window_calibrated_lift: f64,
    positive_window_gap_vs_normal: f64,
    in_crisis_calibrated_lift: f64,
    post_crisis_cooldown_calibrated_lift: f64,
    post_crisis_cooldown_gap_vs_normal: f64,
    max_non_normal_calibrated_lift: f64,
    max_non_normal_threshold_hit_rate: f64,
) -> &'static str {
    if max_non_normal_calibrated_lift < 1.15
        && early_warning_raw_lift < 1.15
        && positive_window_calibrated_lift < 1.15
    {
        return "cold_across_all_regimes";
    }
    if early_warning_raw_lift >= 1.5
        && early_warning_calibrated_lift < 1.15
        && early_warning_gap_retention.unwrap_or_default() < 0.35
    {
        return "calibration_crushed_early_warning";
    }
    if positive_window_calibrated_lift < 1.15 && in_crisis_calibrated_lift >= 1.5 {
        return "late_only_no_early_warning";
    }
    if positive_window_calibrated_lift >= 1.15
        && post_crisis_cooldown_calibrated_lift >= positive_window_calibrated_lift
        && post_crisis_cooldown_gap_vs_normal + 0.002 >= positive_window_gap_vs_normal
    {
        return "cooldown_bleed";
    }
    if max_non_normal_calibrated_lift >= 1.5 && max_non_normal_threshold_hit_rate <= 0.01 {
        return "separated_but_below_runtime_floor";
    }
    if positive_window_calibrated_lift >= 1.5
        && positive_window_gap_vs_normal >= regime_positive_window_gap_floor(horizon_days)
    {
        return "usable_early_warning_separation";
    }
    if max_non_normal_calibrated_lift >= 1.15 || early_warning_calibrated_lift >= 1.15 {
        return "weak_regime_separation";
    }
    "mixed_or_unclear"
}

fn render_release_runtime_separation_note(summaries: &[ReleaseRuntimeSeparationSummary]) -> String {
    let joined = summaries
        .iter()
        .map(|summary| format!("{}d={}", summary.horizon_days, summary.diagnosis))
        .collect::<Vec<_>>()
        .join(", ");
    format!("Runtime separation summary: {joined}.")
}

fn lift_vs_baseline(value: f64, baseline: f64) -> Option<f64> {
    if baseline.abs() <= f64::EPSILON {
        return None;
    }
    Some(round6(value / baseline))
}

fn gap_retention_ratio(raw_gap: f64, calibrated_gap: f64) -> Option<f64> {
    if raw_gap.abs() <= f64::EPSILON {
        return None;
    }
    Some(round6(calibrated_gap / raw_gap))
}

fn runtime_probability_threshold_for_horizon(
    runtime_thresholds: Option<&RuntimeThresholdDiagnosticsWire>,
    horizon_days: u32,
) -> Option<f64> {
    runtime_thresholds.map(|thresholds| match horizon_days {
        5 => thresholds.defend_p5d,
        20 => thresholds.hedge_p20d,
        60 => thresholds.prepare_p60d,
        _ => 1.0,
    })
}

fn summarize_named_counts<'a>(names: impl Iterator<Item = &'a str>) -> Vec<ReleaseRuntimeCount> {
    let mut counts = BTreeMap::<String, usize>::new();
    for name in names {
        *counts.entry(name.to_string()).or_default() += 1;
    }
    let mut rows = counts
        .into_iter()
        .map(|(name, count)| ReleaseRuntimeCount { name, count })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.name.cmp(&right.name))
    });
    rows
}

fn summarize_posture_clause_counts<F>(
    history: &[AssessmentHistoryPoint],
    accessor: F,
) -> Vec<ReleaseRuntimeClauseCount>
where
    F: Fn(&AssessmentHistoryPoint) -> &[String],
{
    let posture_totals = history
        .iter()
        .fold(BTreeMap::<String, usize>::new(), |mut acc, point| {
            *acc.entry(runtime_posture_name(point).to_string())
                .or_default() += 1;
            acc
        });
    let mut counts = BTreeMap::<(String, String), usize>::new();
    for point in history {
        let posture = runtime_posture_name(point).to_string();
        for clause in accessor(point) {
            *counts.entry((posture.clone(), clause.clone())).or_default() += 1;
        }
    }

    let mut rows = counts
        .into_iter()
        .map(|((posture, clause), count)| {
            let posture_total = posture_totals.get(&posture).copied().unwrap_or_default();
            ReleaseRuntimeClauseCount {
                posture,
                clause,
                count,
                share_of_posture: round6(safe_ratio(count, posture_total)),
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.posture.cmp(&right.posture))
            .then_with(|| left.clause.cmp(&right.clause))
    });
    rows
}

fn runtime_posture_name(point: &AssessmentHistoryPoint) -> &'static str {
    match point.posture {
        fc_domain::DecisionPosture::Normal => "normal",
        fc_domain::DecisionPosture::Prepare => "prepare",
        fc_domain::DecisionPosture::Hedge => "hedge",
        fc_domain::DecisionPosture::Defend => "defend",
    }
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
                primary_positive_count: evaluation.actual_positive_count,
                late_validation_row_count: evaluation.post_start_positive_count,
                protected_row_count: evaluation.unclassified_positive_count,
                primary_hit_count: evaluation.pre_start_hit_count,
                late_validation_hit_count: evaluation.post_start_hit_count,
                protected_hit_count: evaluation.unclassified_hit_count,
                false_positive_count: evaluation.false_positive_count,
                scenario_count: evaluation.scenario_count,
                on_time_scenario_count: evaluation.advance_warning_scenario_count,
                late_only_scenario_count: evaluation.late_confirmation_scenario_count,
                missed_scenario_count: evaluation.missed_scenario_count,
                precision_at_threshold: evaluation.precision_at_threshold,
                primary_recall_at_threshold: evaluation.pre_start_recall_at_threshold,
                late_validation_capture_rate: evaluation.post_start_recall_at_threshold,
                on_time_rate: evaluation.advance_warning_rate,
                late_only_rate: evaluation.late_confirmation_rate,
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
        let policy = actionability_guardrail_policy(level.level, level.proxy_horizon_days);

        if level.scenario_count < policy.min_scenario_count {
            regressions.push(format!(
                "actionability {level_name} scenario_count is {} (<{}), so the evaluation slice is too narrow for go/no-go",
                level.scenario_count, policy.min_scenario_count
            ));
        }

        let precision_score =
            (level.precision_at_threshold.unwrap_or_default() * 1_000.0).round() as i64;
        if precision_score < policy.min_precision_score {
            regressions.push(format!(
                "actionability {level_name} precision {:.1}% is below required {:.1}%",
                precision_score as f64 / 10.0,
                policy.min_precision_score as f64 / 10.0
            ));
        }

        let prediction_ceiling = actionability_prediction_count_ceiling_from_actual_positive_count(
            level.primary_positive_count,
            level.proxy_horizon_days,
        );
        if level.predicted_positive_count > prediction_ceiling {
            regressions.push(format!(
                "actionability {level_name} predicted positives {} exceed ceiling {} for {} primary episode rows",
                level.predicted_positive_count,
                prediction_ceiling,
                level.primary_positive_count
            ));
        }

        if level.primary_positive_count > 0
            && level.primary_hit_count == 0
            && level.late_validation_hit_count == 0
        {
            regressions.push(format!(
                "actionability {level_name} produced no primary or late-validation hits across {} primary episode rows",
                level.primary_positive_count
            ));
        }

        if level.primary_positive_count > 0 {
            if let Some(min_advance_warning_rate_score) = policy.min_advance_warning_rate_score {
                let on_time_rate_score = percentage_score(level.on_time_rate).unwrap_or_default();
                if on_time_rate_score < min_advance_warning_rate_score {
                    regressions.push(format!(
                        "actionability {level_name} on_time_rate {:.1}% is below required {:.1}%",
                        on_time_rate_score as f64 / 10.0,
                        min_advance_warning_rate_score as f64 / 10.0
                    ));
                }
            }

            if let Some(max_late_confirmation_rate_score) = policy.max_late_confirmation_rate_score
            {
                let late_only_rate_score =
                    percentage_score(level.late_only_rate).unwrap_or_default();
                if late_only_rate_score > max_late_confirmation_rate_score {
                    regressions.push(format!(
                        "actionability {level_name} late_only_rate {:.1}% exceeds ceiling {:.1}%",
                        late_only_rate_score as f64 / 10.0,
                        max_late_confirmation_rate_score as f64 / 10.0
                    ));
                }
            }

            let missed_rate_score = percentage_score(level.missed_rate).unwrap_or_default();
            if missed_rate_score > policy.max_missed_rate_score {
                regressions.push(format!(
                    "actionability {level_name} missed_rate {:.1}% exceeds ceiling {:.1}%",
                    missed_rate_score as f64 / 10.0,
                    policy.max_missed_rate_score as f64 / 10.0
                ));
            }
        }
    }
    regressions
}

fn compare_probability_guardrails(release: &ModelReleaseRecord) -> anyhow::Result<Vec<String>> {
    if release.manifest.probability_mode == "heuristic_mvp" {
        return Ok(vec![format!(
            "release {} has no formal probability bundle evaluation, so it cannot satisfy formal promotion guard",
            release.manifest.release_id
        )]);
    }

    let bundle = read_probability_bundle(Path::new(&release.manifest.bundle_uri))?;
    let Some(summary) = bundle.evaluation.as_ref() else {
        return Ok(vec![format!(
            "release {} bundle is missing aggregate probability evaluation summary",
            release.manifest.release_id
        )]);
    };

    let mut regressions = Vec::new();
    if summary.usable_early_warning_horizon_count == 0 {
        regressions.push(
            "probability head has zero usable early-warning horizons in bundle evaluation"
                .to_string(),
        );
    }

    for horizon in &summary.regime_separation_summaries {
        if horizon.horizon_days == 20
            && horizon.positive_window_avg_probability <= horizon.normal_avg_probability
        {
            regressions.push(format!(
                "20d positive_window avg {} is at or below normal {} in bundle evaluation",
                format_pct(horizon.positive_window_avg_probability),
                format_pct(horizon.normal_avg_probability),
            ));
        }
        if matches!(horizon.horizon_days, 20 | 60)
            && matches!(
                horizon.diagnosis.as_str(),
                "cooldown_bleed" | "cold_across_all_regimes"
            )
        {
            regressions.push(format!(
                "{}d regime diagnosis is {} in bundle evaluation",
                horizon.horizon_days, horizon.diagnosis
            ));
        }
    }

    Ok(regressions)
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

fn compare_runtime_sanity_guardrails(
    baseline: &ReleaseRuntimeReviewDiagnostics,
    candidate: &ReleaseRuntimeReviewDiagnostics,
) -> Vec<String> {
    let mut regressions = Vec::new();
    let usable_early_warning_horizon_count = candidate
        .regime_separation_summaries
        .iter()
        .filter(|summary| summary.diagnosis == "usable_early_warning_separation")
        .count();

    if usable_early_warning_horizon_count == 0 {
        regressions.push(format!(
            "candidate {} has zero usable early-warning horizons in runtime regime audit",
            candidate.release_id
        ));
    }

    for summary in &candidate.regime_separation_summaries {
        if summary.horizon_days == 20
            && summary.positive_window_avg_probability <= summary.normal_avg_probability
        {
            regressions.push(format!(
                "candidate {} keeps 20d positive_window avg {} at or below normal {} in runtime history",
                candidate.release_id,
                format_pct(summary.positive_window_avg_probability),
                format_pct(summary.normal_avg_probability),
            ));
        }
        if matches!(summary.horizon_days, 20 | 60) && summary.diagnosis == "cooldown_bleed" {
            regressions.push(format!(
                "candidate {} shows cooldown_bleed on {}d runtime regime audit: cooldown {} vs positive_window {}",
                candidate.release_id,
                summary.horizon_days,
                format_pct(summary.post_crisis_cooldown_avg_probability),
                format_pct(summary.positive_window_avg_probability),
            ));
        }
    }

    if release_has_cold_runtime_history(candidate) {
        regressions.push(format!(
            "candidate {} stayed all-normal across {} history points, hit zero runtime probability floors, and still showed no usable early-warning regime separation",
            candidate.release_id, candidate.history_point_count
        ));
    }

    if release_has_cold_runtime_history(baseline) {
        regressions.push(format!(
            "baseline {} is also all-normal / zero-floor-hit, so relative guardrails alone are not a sufficient promotion test",
            baseline.release_id
        ));
    }

    regressions
}

fn release_has_cold_runtime_history(diagnostics: &ReleaseRuntimeReviewDiagnostics) -> bool {
    let all_normal = diagnostics.posture_distribution.len() == 1
        && diagnostics.posture_distribution.first().is_some_and(|row| {
            row.name == "normal" && row.count == diagnostics.history_point_count
        });
    let zero_floor_hits = diagnostics.runtime_thresholds.is_some()
        && [
            diagnostics
                .points_at_or_above_prepare_p60d
                .unwrap_or_default(),
            diagnostics
                .points_at_or_above_hedge_p20d
                .unwrap_or_default(),
            diagnostics
                .points_at_or_above_defend_p5d
                .unwrap_or_default(),
        ]
        .into_iter()
        .all(|count| count == 0);
    let no_usable_early_warning = !diagnostics
        .regime_separation_summaries
        .iter()
        .any(|summary| {
            matches!(
                summary.diagnosis.as_str(),
                "usable_early_warning_separation" | "separated_but_below_runtime_floor"
            )
        });

    all_normal && zero_floor_hits && no_usable_early_warning
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
    let baseline_cold_only = regressions.len() == 1
        && regressions[0].contains("relative guardrails alone are not a sufficient promotion test");
    if regressions.is_empty() {
        if candidate_has_actionability {
            "候选版通过当前概率头、运行时与动作层护栏，可进入下一轮人工复核。仍需结合标签质量、样本覆盖和前端解释能力决定是否晋升。".to_string()
        } else {
            "候选版通过当前概率头与运行时护栏，可进入下一轮人工复核。仍需结合标签质量、样本覆盖和前端解释能力决定是否晋升。".to_string()
        }
    } else if baseline_cold_only {
        "候选版已经通过当前概率头、相对运行时护栏与动作精度约束，当前唯一阻塞是 baseline 仍属于全程 normal 的冷模型，因此这次 review 还不能直接支持“替代默认正式版”。更合适的结论是：该候选版可以视为新的 active_experimental 研究基线，但要晋升为默认正式版，仍需补足绝对提前量门槛与样本/标签治理证据。".to_string()
    } else if candidate_has_actionability {
        "候选版未通过当前概率头 / 运行时 / 动作层护栏，不应替代当前默认线上版本。应先修正训练目标、标签口径、样本切分或样本治理，再重新训练复核。".to_string()
    } else {
        "候选版未通过当前概率头 / 运行时护栏，不应替代当前默认线上版本。应先修正训练目标、标签口径或样本治理，再重新训练复核。".to_string()
    }
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
    let _ = writeln!(markdown, "## Runtime Diagnostics");
    let _ = writeln!(markdown);
    render_release_runtime_review_markdown(
        &mut markdown,
        "baseline",
        &report.baseline_runtime_review,
    );
    let _ = writeln!(markdown);
    render_release_runtime_review_markdown(
        &mut markdown,
        "candidate",
        &report.candidate_runtime_review,
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
    let _ = writeln!(markdown, "### Probability Guard");
    let _ = writeln!(markdown);
    if report.probability_guard_regressions.is_empty() {
        let _ = writeln!(
            markdown,
            "- No probability-head guard regressions detected."
        );
    } else {
        for regression in &report.probability_guard_regressions {
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
    let _ = writeln!(markdown, "### Runtime Sanity Guard");
    let _ = writeln!(markdown);
    if report.runtime_sanity_regressions.is_empty() {
        let _ = writeln!(markdown, "- No runtime sanity regressions detected.");
    } else {
        for regression in &report.runtime_sanity_regressions {
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
        "| Level | Scenarios | On Time | Late Only | Missed | Primary Recall | Late Validation | Precision | Pred+ | Primary+ | Protected Hits | FP |"
    );
    let _ = writeln!(
        markdown,
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
    );
    for level in &review.levels {
        let _ = writeln!(
            markdown,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            actionability_level_text(level.level),
            level.scenario_count,
            format_optional_pct(level.on_time_rate),
            format_optional_pct(level.late_only_rate),
            format_optional_pct(level.missed_rate),
            format_optional_pct(level.primary_recall_at_threshold),
            format_optional_pct(level.late_validation_capture_rate),
            format_optional_pct(level.precision_at_threshold),
            level.predicted_positive_count,
            level.primary_positive_count,
            level.protected_hit_count,
            level.false_positive_count
        );
    }
}

fn render_release_runtime_review_markdown(
    markdown: &mut String,
    role: &str,
    diagnostics: &ReleaseRuntimeReviewDiagnostics,
) {
    let _ = writeln!(markdown, "### {role} Runtime");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "- Release: {}", diagnostics.release_id);
    let _ = writeln!(
        markdown,
        "- History points: {}",
        diagnostics.history_point_count
    );
    let _ = writeln!(markdown, "- Note: {}", diagnostics.note);
    if let Some(thresholds) = diagnostics.runtime_thresholds.as_ref() {
        let _ = writeln!(
            markdown,
            "- Thresholds: prepare_p60d={}, hedge_p20d={}, defend_p5d={}",
            format_pct(thresholds.prepare_p60d),
            format_pct(thresholds.hedge_p20d),
            format_pct(thresholds.defend_p5d),
        );
        let _ = writeln!(
            markdown,
            "- Runtime policy version: {}",
            thresholds.history_runtime_policy_version
        );
        let _ = writeln!(
            markdown,
            "- Probability floor hits: p_60d>=prepare {} / p_20d>=hedge {} / p_5d>=defend {}",
            diagnostics
                .points_at_or_above_prepare_p60d
                .unwrap_or_default(),
            diagnostics
                .points_at_or_above_hedge_p20d
                .unwrap_or_default(),
            diagnostics
                .points_at_or_above_defend_p5d
                .unwrap_or_default(),
        );
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "| Posture | Count |");
    let _ = writeln!(markdown, "| --- | --- |");
    for row in &diagnostics.posture_distribution {
        let _ = writeln!(markdown, "| {} | {} |", row.name, row.count);
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "| Time bucket | Count |");
    let _ = writeln!(markdown, "| --- | --- |");
    for row in &diagnostics.time_bucket_distribution {
        let _ = writeln!(markdown, "| {} | {} |", row.name, row.count);
    }
    if !diagnostics.posture_trigger_distribution.is_empty() {
        let _ = writeln!(markdown);
        let _ = writeln!(
            markdown,
            "| Posture | Trigger clause | Count | Share of posture |"
        );
        let _ = writeln!(markdown, "| --- | --- | --- | --- |");
        for row in &diagnostics.posture_trigger_distribution {
            let _ = writeln!(
                markdown,
                "| {} | {} | {} | {} |",
                row.posture,
                row.clause,
                row.count,
                format_pct(row.share_of_posture),
            );
        }
    }
    if !diagnostics.posture_blocker_distribution.is_empty() {
        let _ = writeln!(markdown);
        let _ = writeln!(
            markdown,
            "| Posture | Blocker clause | Count | Share of posture |"
        );
        let _ = writeln!(markdown, "| --- | --- | --- | --- |");
        for row in &diagnostics.posture_blocker_distribution {
            let _ = writeln!(
                markdown,
                "| {} | {} | {} | {} |",
                row.posture,
                row.clause,
                row.count,
                format_pct(row.share_of_posture),
            );
        }
    }
    if !diagnostics.regime_separation_summaries.is_empty() {
        let _ = writeln!(markdown);
        let _ = writeln!(
            markdown,
            "| Horizon | Early regime | Normal P | Positive-window P | Cooldown P | Early raw lift | Early calibrated lift | Positive-window lift | Cooldown lift | Gap retention | Diagnosis |"
        );
        let _ = writeln!(
            markdown,
            "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
        );
        for row in &diagnostics.regime_separation_summaries {
            let _ = writeln!(
                markdown,
                "| {}d | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                row.horizon_days,
                row.early_warning_regime,
                format_pct(row.normal_avg_probability),
                format_pct(row.positive_window_avg_probability),
                format_pct(row.post_crisis_cooldown_avg_probability),
                format_optional_multiplier(row.early_warning_raw_lift_vs_normal),
                format_optional_multiplier(row.early_warning_calibrated_lift_vs_normal),
                format_optional_multiplier(row.positive_window_calibrated_lift_vs_normal),
                format_optional_multiplier(row.post_crisis_cooldown_calibrated_lift_vs_normal),
                format_optional_ratio(row.early_warning_gap_retention),
                row.diagnosis,
            );
        }
    }
    if !diagnostics.regime_probability_summaries.is_empty() {
        let _ = writeln!(markdown);
        let _ = writeln!(
            markdown,
            "| Horizon | Regime | Rows | Share | Avg raw P | Max raw P | Avg calibrated P | Max calibrated P | Raw lift vs normal | Calibrated lift vs normal | Gap retention | Floor hits |"
        );
        let _ = writeln!(
            markdown,
            "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
        );
        for row in &diagnostics.regime_probability_summaries {
            let _ = writeln!(
                markdown,
                "| {}d | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                row.horizon_days,
                row.regime,
                row.row_count,
                format_pct(row.row_rate),
                format_pct(row.avg_raw_probability),
                format_pct(row.max_raw_probability),
                format_pct(row.avg_probability),
                format_pct(row.max_probability),
                format_optional_multiplier(row.raw_lift_vs_normal),
                format_optional_multiplier(row.calibrated_lift_vs_normal),
                format_optional_ratio(row.calibration_gap_retention),
                row.threshold_hit_count
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string()),
            );
        }
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
    if report.probability_guard_regressions.is_empty() {
        println!("Probability guard summary:");
        println!("  no bundle-level probability guard regressions");
    } else {
        println!("Probability guard summary:");
        for regression in &report.probability_guard_regressions {
            println!("  - {regression}");
        }
    }
    if report.candidate_actionability_review.enabled {
        println!("Actionability guard summary:");
        for level in &report.candidate_actionability_review.levels {
            println!(
                "  {:>7} scenarios={} on_time={} late_only={} missed={}",
                actionability_level_text(level.level),
                level.scenario_count,
                format_optional_pct(level.on_time_rate),
                format_optional_pct(level.late_only_rate),
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
    let scenario_metadata =
        load_formal_dataset_scenario_metadata(&dataset.manifest.scenario_set_version)?;
    let scenario_ranges = collect_formal_dataset_scenario_ranges(rows, &scenarios);
    let split_summaries = summarize_formal_dataset_splits(rows, &scenario_ranges);
    let scenario_summaries =
        summarize_formal_dataset_scenarios(rows, &scenario_ranges, &scenario_metadata);
    let family_summaries = summarize_formal_dataset_families(rows);
    let quality_summaries = summarize_formal_dataset_quality(rows);
    let regime_summaries = summarize_formal_dataset_regimes(rows, &scenarios);
    let recommendation = build_formal_dataset_recommendation(
        &dataset.manifest.label_version,
        &split_summaries,
        rows.len(),
    );

    Ok(FormalDatasetSummaryEnvelope {
        generated_at: Utc::now().to_rfc3339(),
        dataset_key: dataset_key.to_string(),
        dataset,
        split_summaries,
        scenario_summaries,
        family_summaries,
        quality_summaries,
        regime_summaries,
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
                prepare_primary_count: split_rows
                    .iter()
                    .filter(|row| row.prepare_episode_label > 0)
                    .count(),
                prepare_primary_rate: round6(action_episode_primary_rate(
                    &split_rows,
                    ActionabilityLevel::Prepare,
                )),
                hedge_primary_count: split_rows
                    .iter()
                    .filter(|row| row.hedge_episode_label > 0)
                    .count(),
                hedge_primary_rate: round6(action_episode_primary_rate(
                    &split_rows,
                    ActionabilityLevel::Hedge,
                )),
                defend_primary_count: split_rows
                    .iter()
                    .filter(|row| row.defend_episode_label > 0)
                    .count(),
                defend_primary_rate: round6(action_episode_primary_rate(
                    &split_rows,
                    ActionabilityLevel::Defend,
                )),
                late_validation_row_count: split_rows
                    .iter()
                    .filter(|row| row.action_episode_phase == "late_validation")
                    .count(),
                late_validation_row_rate: round6(late_validation_row_rate(&split_rows)),
                protected_row_count: split_rows
                    .iter()
                    .filter(|row| row.protected_action_window)
                    .count(),
                protected_row_rate: round6(protected_action_window_rate(&split_rows)),
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
    scenario_metadata: &BTreeMap<String, ScenarioSummaryMetadata>,
) -> Vec<FormalDatasetScenarioSummary> {
    scenario_ranges
        .iter()
        .map(|range| {
            let metadata = scenario_metadata.get(&range.scenario_id);
            FormalDatasetScenarioSummary {
                scenario_id: range.scenario_id.clone(),
                label: metadata.map(|item| item.label.clone()),
                row_count: range.end_index.saturating_sub(range.start_index) + 1,
                split_count: rows[range.start_index..=range.end_index]
                    .iter()
                    .map(|row| row.split_name.as_str())
                    .collect::<BTreeSet<_>>()
                    .len(),
                first_as_of_date: rows[range.start_index].as_of_date,
                last_as_of_date: rows[range.end_index].as_of_date,
                family: metadata
                    .map(|item| item.family.clone())
                    .or_else(|| Some(range.family.clone())),
                training_role: metadata.map(|item| item.training_role.clone()),
                protected_window: metadata.map(|item| item.protected_window),
                episode_template_id: metadata.map(|item| item.episode_template_id.clone()),
                default_horizon_roles: metadata
                    .map(|item| item.default_horizon_roles.clone())
                    .unwrap_or_default(),
            }
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

fn summarize_formal_dataset_regimes(
    rows: &[FormalDatasetRowRecord],
    scenarios: &[CrisisScenario],
) -> Vec<FormalDatasetRegimeSummary> {
    let split_totals = rows
        .iter()
        .fold(BTreeMap::<String, usize>::new(), |mut acc, row| {
            *acc.entry(row.split_name.clone()).or_default() += 1;
            acc
        });
    let mut buckets = BTreeMap::<(String, u32, String), usize>::new();
    for row in rows {
        for horizon_days in [5_u32, 20_u32, 60_u32] {
            let regime = probability_training_regime_name(forward_crisis_training_regime(
                row.as_of_date,
                scenarios,
                horizon_days,
            ));
            *buckets
                .entry((row.split_name.clone(), horizon_days, regime.to_string()))
                .or_default() += 1;
        }
    }

    buckets
        .into_iter()
        .map(|((split_name, horizon_days, regime), row_count)| {
            let split_total = split_totals.get(&split_name).copied().unwrap_or_default();
            FormalDatasetRegimeSummary {
                split_name,
                horizon_days,
                regime,
                row_count,
                row_rate: round6(safe_ratio(row_count, split_total)),
            }
        })
        .collect()
}

fn build_formal_dataset_recommendation(
    label_version: &str,
    split_summaries: &[FormalDatasetSplitSummary],
    total_rows: usize,
) -> String {
    let evaluation = split_summaries
        .iter()
        .find(|split| split.split_name == "evaluation");
    if total_rows < 5_000 {
        return "样本量仍偏小，先继续补历史数据，再用这版数据集训练正式候选版。".to_string();
    }
    match formal_dataset_split_profile(label_version) {
        FormalDatasetSplitProfile::ExtensionAcute => {
            let Some(evaluation) = evaluation else {
                return "缺少 evaluation split，当前还不能稳定比较 1987/1998 的急性冲击表现。"
                    .to_string();
            };
            if evaluation.scenario_count < 1 || evaluation.defend_primary_count == 0 {
                return "evaluation 仍未覆盖足够的 acute 尾段主正例，先继续重做 split 或补齐 1987/1998 proxy 覆盖。".to_string();
            }
            if evaluation.prepare_primary_count == 0 || evaluation.hedge_primary_count == 0 {
                return "这套扩展 acute 数据集已经能用于 1987/1998 的 5d/20d 与急性尾段类比，但 evaluation 还不足以单独评估完整的 prepare/hedge/defend episode 头。".to_string();
            }
            return "这套扩展 acute 数据集已经可以用于 1987/1998 的 5d/20d 历史类比与短窗研究；它是研究包，不应用作正式主模型上线判断。".to_string();
        }
        FormalDatasetSplitProfile::ExtensionStress => {
            let Some(evaluation) = evaluation else {
                return "缺少 evaluation split，当前还不能稳定比较 protected stress 扩展场景。"
                    .to_string();
            };
            if evaluation.scenario_count < 1
                || evaluation.prepare_primary_count == 0
                || evaluation.hedge_primary_count == 0
            {
                return "evaluation 的 protected stress / extension 主正例仍偏少，先继续重做 split，再把它用于扩展研究和 posture 对照。".to_string();
            }
            if evaluation.protected_row_count < 1 {
                return "evaluation 还没有 protected stress 尾段样本，当前不适合拿它判断受保护压力窗口是否稳定。".to_string();
            }
            return "这套扩展 stress 数据集已经可以用于 protected stress、历史对照和扩展训练研究；它不是正式主模型 go/no-go 的单独依据。".to_string();
        }
        FormalDatasetSplitProfile::Main => {}
    }
    if let Some(evaluation) = evaluation {
        if evaluation.hedge_primary_count < 10 || evaluation.prepare_primary_count < 10 {
            return "evaluation 的 episode-native 主正例仍偏少，当前更适合作研究版比较，不适合直接给正式模型做上线判断。".to_string();
        }
        if evaluation.late_validation_row_count < 5 {
            return "evaluation 的 late-validation 样本仍偏少，动作头很难判断“过晚确认”到底是偶然还是系统性问题。".to_string();
        }
        if evaluation.protected_row_count < 5 {
            return "evaluation 的 protected stress 样本仍偏少，当前还不适合把 protected/cooldown 行为当成稳定结论。".to_string();
        }
        if evaluation.scenario_count < 2 {
            return format!(
                "evaluation split 的 episode-native 动作标签目前只覆盖 {} 个场景，动作头评估很不稳；应先扩历史场景或重做 split，再用它判断 formal 候选版优劣。",
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

fn action_episode_primary_rate(rows: &[&FormalDatasetRowRecord], level: ActionabilityLevel) -> f64 {
    let positives = rows
        .iter()
        .filter(|row| row_has_action_episode_label(row, level))
        .count();
    positives as f64 / rows.len() as f64
}

fn late_validation_row_rate(rows: &[&FormalDatasetRowRecord]) -> f64 {
    let positives = rows
        .iter()
        .filter(|row| row.action_episode_phase == "late_validation")
        .count();
    positives as f64 / rows.len() as f64
}

fn protected_action_window_rate(rows: &[&FormalDatasetRowRecord]) -> f64 {
    let positives = rows
        .iter()
        .filter(|row| row.protected_action_window)
        .count();
    positives as f64 / rows.len() as f64
}

fn avg_metric<F>(rows: &[&FormalDatasetRowRecord], accessor: F) -> f64
where
    F: Fn(&FormalDatasetRowRecord) -> f64,
{
    rows.iter().map(|row| accessor(row)).sum::<f64>() / rows.len() as f64
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
        "| Split | Rows | Forward 5d+ | Forward 20d+ | Forward 60d+ | Prepare Primary | Hedge Primary | Defend Primary | Late Validation | Protected | Avg Coverage | Core | Trigger | External | Scenarios |"
    );
    let _ = writeln!(
        markdown,
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
    );
    for split in &summary.split_summaries {
        let _ = writeln!(
            markdown,
            "| {} | {} | {} ({}) | {} ({}) | {} ({}) | {} ({}) | {} ({}) | {} ({}) | {} ({}) | {} ({}) | {:.1}% | {:.1}% | {:.1}% | {:.1}% | {} |",
            split.split_name,
            split.row_count,
            split.positive_5d_count,
            format_pct(split.positive_5d_rate),
            split.positive_20d_count,
            format_pct(split.positive_20d_rate),
            split.positive_60d_count,
            format_pct(split.positive_60d_rate),
            split.prepare_primary_count,
            format_pct(split.prepare_primary_rate),
            split.hedge_primary_count,
            format_pct(split.hedge_primary_rate),
            split.defend_primary_count,
            format_pct(split.defend_primary_rate),
            split.late_validation_row_count,
            format_pct(split.late_validation_row_rate),
            split.protected_row_count,
            format_pct(split.protected_row_rate),
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
    let _ = writeln!(
        markdown,
        "| Scenario | Label | Family | Role | Protected | Horizons | Template | Rows | Splits | Range |"
    );
    let _ = writeln!(
        markdown,
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
    );
    for scenario in &summary.scenario_summaries {
        let default_horizon_roles = if scenario.default_horizon_roles.is_empty() {
            "-".to_string()
        } else {
            scenario
                .default_horizon_roles
                .iter()
                .map(|value| format!("{value}d"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let _ = writeln!(
            markdown,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} -> {} |",
            scenario.scenario_id,
            scenario.label.as_deref().unwrap_or("-"),
            scenario.family.as_deref().unwrap_or("-"),
            scenario.training_role.as_deref().unwrap_or("-"),
            scenario
                .protected_window
                .map(|value| if value { "yes" } else { "no" })
                .unwrap_or("-"),
            default_horizon_roles,
            scenario.episode_template_id.as_deref().unwrap_or("-"),
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
    let _ = writeln!(markdown, "## Regime Mix");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "| Split | Horizon | Regime | Rows | Share |");
    let _ = writeln!(markdown, "| --- | --- | --- | --- | --- |");
    for regime in &summary.regime_summaries {
        let _ = writeln!(
            markdown,
            "| {} | {}d | {} | {} | {} |",
            regime.split_name,
            regime.horizon_days,
            regime.regime,
            regime.row_count,
            format_pct(regime.row_rate),
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
            "  split={} rows={} forward[5d={}({}) 20d={}({}) 60d={}({})] action[prepare={}({}) hedge={}({}) defend={}({}) late_validation={}({}) protected={}({})] avg_coverage={:.1}%",
            split.split_name,
            split.row_count,
            split.positive_5d_count,
            format_pct(split.positive_5d_rate),
            split.positive_20d_count,
            format_pct(split.positive_20d_rate),
            split.positive_60d_count,
            format_pct(split.positive_60d_rate),
            split.prepare_primary_count,
            format_pct(split.prepare_primary_rate),
            split.hedge_primary_count,
            format_pct(split.hedge_primary_rate),
            split.defend_primary_count,
            format_pct(split.defend_primary_rate),
            split.late_validation_row_count,
            format_pct(split.late_validation_row_rate),
            split.protected_row_count,
            format_pct(split.protected_row_rate),
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

fn format_optional_multiplier(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.2}x"))
        .unwrap_or_else(|| "—".to_string())
}

fn format_optional_ratio(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.2}"))
        .unwrap_or_else(|| "—".to_string())
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
    reload_api_runtime_with_history_mode(url, ApiReloadHistoryMode::Default).await
}

async fn reload_api_runtime_with_history_mode(
    url: &str,
    history_mode: ApiReloadHistoryMode,
) -> anyhow::Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(1_200))
        .build()?;
    let request = client.post(url);
    let request = if let Some(history_mode) = history_mode.as_query_value() {
        request.query(&[("history_mode", history_mode)])
    } else {
        request
    };
    let response = request.send().await?;
    if !response.status().is_success() {
        bail!("reload endpoint returned {}", response.status());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    use chrono::{NaiveDate, TimeZone, Utc};
    use fc_domain::{
        ActionEpisodeTemplateId, ActionabilityBundle, ActionabilityEvaluationSummary,
        ActionabilityLevelBundle, AssessmentHistoryPoint, DecisionPosture, FeatureSnapshotRecord,
        FormalDatasetRowRecord, Frequency, HorizonEvaluationSummary, LogisticProbabilityModel,
        ModelReleaseManifest, ModelReleaseRecord, Observation, PlattCalibrationArtifact,
        ProbabilityBundle, ProbabilityBundleEvaluation, RegimeSeparationEvaluationSummary,
        TimeToRiskBucket, PROBABILITY_FEATURE_TRANSFORM_IDENTITY_V1,
        PROBABILITY_MODEL_FAMILY_LINEAR_V1,
    };

    use super::commands::release::{
        ReleasePublishOptions, ReleaseReviewOptions, ReleaseSwitchOptions,
    };
    use super::{
        action_window_label, actionability_bundle_quality_regressions,
        adjust_probability_decision_threshold_for_regime_support,
        build_probability_threshold_diagnostics, classify_probability_regime_separation,
        classify_regime_separation, compare_actionability_guardrails,
        compare_probability_guardrails, evaluate_actionability_summary, fit_platt_calibration,
        formal_dataset_split_requirements, forward_crisis_label,
        forward_crisis_regime_pairwise_targets, forward_crisis_regime_sample_weight,
        forward_crisis_training_regime, forward_crisis_training_regime_with_context,
        negative_sample_weight, observation_is_visible_for_date, positive_sample_action_weight,
        probability_calibration_selection_rows, probability_decision_threshold_selection, round3,
        scenario_aware_formal_split_bounds, scenario_count_for_index_range,
        select_actionability_calibration_strategy, select_actionability_decision_threshold,
        select_probability_calibration_strategy, select_probability_decision_threshold,
        summarize_release_runtime_regime_probabilities,
        summarize_release_runtime_regime_separation, ActionabilityLevel, AuditExportOptions,
        CrisisScenario, FeatureSnapshotBuildOptions, FormalDatasetBuildOptions,
        FormalDatasetSummaryOptions, FormalSplitLabelSupport, PipelineDatasetSource,
        PipelineTrainOptions, PointInTimeMode, PredictionSnapshotQueryOptions,
        ProbabilityCalibrationSelection, ProbabilityModelShape, ProbabilityTargetLabelMode,
        ProbabilityThresholdDiagnosticsInput, ProbabilityThresholdSelection,
        ProbabilityTrainingRegime, ProbabilityTrainingRow, RefreshLatestOptions,
        ReleaseActionabilityLevelReview, ReleaseActionabilityReview, ScenarioRowRange,
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

    fn test_release_with_bundle(bundle: &ProbabilityBundle) -> ModelReleaseRecord {
        let bundle_path = std::env::temp_dir().join(format!(
            "fc-probability-guard-{}.json",
            Utc::now()
                .timestamp_nanos_opt()
                .unwrap_or_default()
                .unsigned_abs()
        ));
        std::fs::write(&bundle_path, serde_json::to_string_pretty(bundle).unwrap()).unwrap();
        ModelReleaseRecord {
            manifest: ModelReleaseManifest {
                release_id: bundle.bundle_id.clone(),
                market_scope: bundle.market_scope.clone(),
                status: "approved".to_string(),
                probability_mode: bundle.probability_mode.clone(),
                serving_status: "healthy".to_string(),
                bundle_uri: bundle_path.to_string_lossy().to_string(),
                feature_set_version: "feature_formal_v1_main_20260531".to_string(),
                label_version: "formal_label_v1_main".to_string(),
                prob_model_version: "prob".to_string(),
                calibration_version: "calib".to_string(),
                posture_policy_version: "posture".to_string(),
                action_playbook_version: "playbook".to_string(),
                point_in_time_mode: "best_effort".to_string(),
                training_range_start: None,
                training_range_end: None,
                calibration_range_start: None,
                calibration_range_end: None,
                evaluation_range_start: None,
                evaluation_range_end: None,
                brier_score: Some(0.1),
                log_loss: Some(0.2),
                ece: Some(0.1),
                note: "test".to_string(),
            },
            created_at: Utc::now(),
            activated_at: None,
            retired_at: None,
        }
    }

    fn synthetic_runtime_scenarios() -> Vec<CrisisScenario> {
        vec![CrisisScenario {
            scenario_id: "synthetic".to_string(),
            family: "systemic_credit_banking_crisis".to_string(),
            training_role: "mandatory".to_string(),
            pre_warning_start: NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
            crisis_start: NaiveDate::from_ymd_opt(2000, 2, 1).unwrap(),
            acute_start: None,
            crisis_end: NaiveDate::from_ymd_opt(2000, 2, 20).unwrap(),
            default_horizon_roles: vec![20, 60],
            protected_window: false,
            protected_action_levels: Vec::new(),
            episode_template_id: ActionEpisodeTemplateId::SystemicCreditBankingCrisis,
            action_episode_overrides: None,
        }]
    }

    fn runtime_history_point(
        as_of_date: NaiveDate,
        raw_probability: f64,
        calibrated_probability: f64,
    ) -> AssessmentHistoryPoint {
        AssessmentHistoryPoint {
            as_of_date,
            overall_score: 50.0,
            p_5d: calibrated_probability,
            p_20d: calibrated_probability,
            p_60d: calibrated_probability,
            raw_p_5d: Some(raw_probability),
            raw_p_20d: Some(raw_probability),
            raw_p_60d: Some(raw_probability),
            posture: DecisionPosture::Normal,
            time_to_risk_bucket: TimeToRiskBucket::Normal,
            external_shock_score: 20.0,
            posture_trigger_codes: Vec::new(),
            posture_blocker_codes: Vec::new(),
        }
    }

    fn forward_crisis_row(
        as_of_date: NaiveDate,
        label_20d: u8,
        regime_20d: ProbabilityTrainingRegime,
    ) -> ProbabilityTrainingRow {
        ProbabilityTrainingRow {
            as_of_date,
            market_scope: "financial_system".to_string(),
            release_id: None,
            probability_mode: Some("formal_bundle_v1".to_string()),
            freshness_status: Some("fresh".to_string()),
            time_to_risk_bucket: Some("test".to_string()),
            split_name: Some("calibration".to_string()),
            features: BTreeMap::new(),
            primary_scenario_id: Some("scenario_a".to_string()),
            scenario_family: Some("systemic_credit_banking_crisis".to_string()),
            scenario_training_role: None,
            days_to_primary_crisis_start: Some(10),
            primary_scenario_supports_5d: true,
            primary_scenario_supports_20d: true,
            primary_scenario_supports_60d: true,
            label_5d: 0,
            label_20d,
            label_60d: 0,
            regime_5d: ProbabilityTrainingRegime::Normal,
            regime_20d,
            regime_60d: ProbabilityTrainingRegime::Normal,
            action_label_5d: 0,
            action_label_20d: 0,
            action_label_60d: 0,
            prepare_episode_label: 0,
            hedge_episode_label: 0,
            defend_episode_label: 0,
            primary_action_level: None,
            action_episode_id: None,
            action_episode_phase: "outside".to_string(),
            protected_action_window: false,
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
    fn release_review_defaults_to_ignored_artifact_dir() {
        let args = vec![
            "--candidate-release-id".to_string(),
            "candidate-123".to_string(),
        ];
        let options = ReleaseReviewOptions::parse(&args).unwrap();
        assert_eq!(
            options.output_dir,
            PathBuf::from("artifacts/research/release-review")
        );
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
    fn extension_acute_dataset_min_date_allows_pre1990_history() {
        assert_eq!(
            super::formal_dataset_min_date("formal_label_v1_ext_acute"),
            NaiveDate::from_ymd_opt(1987, 1, 1).unwrap()
        );
        assert_eq!(
            super::formal_dataset_min_date("formal_label_v1_main"),
            NaiveDate::from_ymd_opt(1990, 1, 2).unwrap()
        );
    }

    #[test]
    fn extension_acute_dataset_allows_proxy_feature_gate_without_vix() {
        let snapshot = FeatureSnapshotRecord {
            as_of_date: NaiveDate::from_ymd_opt(1987, 10, 19).unwrap(),
            entity_id: "us".to_string(),
            market_scope: "financial_system".to_string(),
            feature_set_version: "feature_formal_v1_main_20260531".to_string(),
            point_in_time_mode: "best_effort".to_string(),
            visibility_status: "coverage_or_visibility_failed".to_string(),
            latest_visible_at: Some(Utc::now()),
            coverage_score: 0.56,
            core_feature_coverage: 0.625,
            trigger_feature_coverage: 0.50,
            external_feature_coverage: 0.50,
            feature_count: 4,
            features: [
                ("us_curve_10y2y_level".to_string(), -0.2),
                ("us_baa_10y_spread_level".to_string(), 2.8),
                ("us_fed_funds_level".to_string(), 6.5),
                ("us_usdjpy_level".to_string(), 0.0068),
            ]
            .into_iter()
            .collect(),
            created_at: Utc::now(),
        };

        assert!(super::formal_dataset_snapshot_is_usable(
            &snapshot,
            "formal_label_v1_ext_acute"
        ));
        assert!(!super::formal_dataset_snapshot_is_usable(
            &snapshot,
            "formal_label_v1_main"
        ));
    }

    #[test]
    fn extension_stress_dataset_allows_1990s_partial_coverage_gate() {
        let snapshot = FeatureSnapshotRecord {
            as_of_date: NaiveDate::from_ymd_opt(1993, 1, 5).unwrap(),
            entity_id: "us".to_string(),
            market_scope: "financial_system".to_string(),
            feature_set_version: "feature_formal_v1_main_20260531".to_string(),
            point_in_time_mode: "best_effort".to_string(),
            visibility_status: "ready".to_string(),
            latest_visible_at: Some(Utc::now()),
            coverage_score: 0.785,
            core_feature_coverage: 0.875,
            trigger_feature_coverage: 0.833,
            external_feature_coverage: 0.50,
            feature_count: 4,
            features: [
                ("us_vix_level".to_string(), 12.0),
                ("us_curve_10y2y_level".to_string(), 1.2),
                ("us_baa_10y_spread_level".to_string(), 2.1),
                ("us_fed_funds_level".to_string(), 3.0),
            ]
            .into_iter()
            .collect(),
            created_at: Utc::now(),
        };

        assert!(super::formal_dataset_snapshot_is_usable(
            &snapshot,
            "formal_label_v1_ext_stress"
        ));
        assert!(!super::formal_dataset_snapshot_is_usable(
            &snapshot,
            "formal_label_v1_main"
        ));
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
    fn formal_dataset_summary_defaults_to_ignored_artifact_dir() {
        let options = FormalDatasetSummaryOptions::parse(&[]).unwrap();
        assert_eq!(
            options.output_dir,
            PathBuf::from("artifacts/research/formal-dataset")
        );
    }

    #[test]
    fn parses_pipeline_train_defaults_to_formal_dataset() {
        let options = PipelineTrainOptions::parse(&[]).unwrap();
        assert_eq!(options.dataset_source, PipelineDatasetSource::Formal);
        assert_eq!(options.model_shape, ProbabilityModelShape::LinearV1);
        assert_eq!(options.dataset_id, "formal_v1_main_1990_daily");
        assert_eq!(options.dataset_version, None);
        assert_eq!(options.dataset_key, None);
        assert!(options.aux_dataset_keys.is_empty());
        assert_eq!(
            options.output_dir,
            PathBuf::from("artifacts/research/model-bundles/generated")
        );
        assert_eq!(
            options.manifest_dir,
            PathBuf::from("artifacts/research/model-releases/generated")
        );
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
        assert_eq!(options.model_shape, ProbabilityModelShape::LinearV1);
        assert_eq!(options.release_prefix, "custom_prefix");
        assert_eq!(
            options.query.market_scope.as_deref(),
            Some("financial_system")
        );
    }

    #[test]
    fn parses_pipeline_train_interaction_tail_shape() {
        let args = vec![
            "--model-shape".to_string(),
            "interaction_tail_v1".to_string(),
        ];
        let options = PipelineTrainOptions::parse(&args).unwrap();

        assert_eq!(
            options.model_shape,
            ProbabilityModelShape::InteractionTailV1
        );
        assert_eq!(options.release_prefix, "us_formal_interaction_tail");
    }

    #[test]
    fn parses_pipeline_train_aux_dataset_keys() {
        let args = vec![
            "--dataset-key".to_string(),
            "formal_v1_main_1990_daily:20260601T172759".to_string(),
            "--aux-dataset-key".to_string(),
            "formal_v1_ext_stress_1990_daily:20260601T162655".to_string(),
            "--aux-dataset-key".to_string(),
            "formal_v1_ext_acute_pre1990:20260601T163102".to_string(),
        ];
        let options = PipelineTrainOptions::parse(&args).unwrap();
        assert_eq!(
            options.dataset_key.as_deref(),
            Some("formal_v1_main_1990_daily:20260601T172759")
        );
        assert_eq!(
            options.aux_dataset_keys,
            vec![
                "formal_v1_ext_stress_1990_daily:20260601T162655".to_string(),
                "formal_v1_ext_acute_pre1990:20260601T163102".to_string()
            ]
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
            training_role: "mandatory".to_string(),
            pre_warning_start: NaiveDate::from_ymd_opt(2020, 1, 24).unwrap(),
            crisis_start: NaiveDate::from_ymd_opt(2020, 2, 24).unwrap(),
            acute_start: Some(NaiveDate::from_ymd_opt(2020, 3, 9).unwrap()),
            crisis_end: NaiveDate::from_ymd_opt(2020, 4, 30).unwrap(),
            default_horizon_roles: vec![5, 20],
            protected_window: false,
            protected_action_levels: Vec::new(),
            episode_template_id: ActionEpisodeTemplateId::AcuteMarketLiquidityCrash,
            action_episode_overrides: None,
        };
        let systemic_only = CrisisScenario {
            scenario_id: "systemic".to_string(),
            family: "systemic_credit_banking_crisis".to_string(),
            training_role: "mandatory".to_string(),
            pre_warning_start: NaiveDate::from_ymd_opt(2023, 2, 1).unwrap(),
            crisis_start: NaiveDate::from_ymd_opt(2023, 3, 8).unwrap(),
            acute_start: Some(NaiveDate::from_ymd_opt(2023, 3, 10).unwrap()),
            crisis_end: NaiveDate::from_ymd_opt(2023, 5, 15).unwrap(),
            default_horizon_roles: vec![20, 60],
            protected_window: false,
            protected_action_levels: Vec::new(),
            episode_template_id: ActionEpisodeTemplateId::SystemicCreditBankingCrisis,
            action_episode_overrides: None,
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
            training_role: "mandatory".to_string(),
            pre_warning_start: NaiveDate::from_ymd_opt(2007, 2, 27).unwrap(),
            crisis_start: NaiveDate::from_ymd_opt(2007, 8, 1).unwrap(),
            acute_start: Some(NaiveDate::from_ymd_opt(2008, 9, 15).unwrap()),
            crisis_end: NaiveDate::from_ymd_opt(2009, 6, 30).unwrap(),
            default_horizon_roles: vec![20, 60],
            protected_window: false,
            protected_action_levels: Vec::new(),
            episode_template_id: ActionEpisodeTemplateId::SystemicCreditBankingCrisis,
            action_episode_overrides: None,
        };
        let acute = CrisisScenario {
            scenario_id: "acute".to_string(),
            family: "acute_market_liquidity_crash".to_string(),
            training_role: "mandatory".to_string(),
            pre_warning_start: NaiveDate::from_ymd_opt(2020, 1, 24).unwrap(),
            crisis_start: NaiveDate::from_ymd_opt(2020, 2, 24).unwrap(),
            acute_start: Some(NaiveDate::from_ymd_opt(2020, 3, 9).unwrap()),
            crisis_end: NaiveDate::from_ymd_opt(2020, 4, 30).unwrap(),
            default_horizon_roles: vec![5, 20],
            protected_window: false,
            protected_action_levels: Vec::new(),
            episode_template_id: ActionEpisodeTemplateId::AcuteMarketLiquidityCrash,
            action_episode_overrides: None,
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
            training_role: "mandatory".to_string(),
            pre_warning_start: NaiveDate::from_ymd_opt(2007, 2, 27).unwrap(),
            crisis_start: NaiveDate::from_ymd_opt(2007, 8, 1).unwrap(),
            acute_start: Some(NaiveDate::from_ymd_opt(2008, 9, 15).unwrap()),
            crisis_end: NaiveDate::from_ymd_opt(2009, 6, 30).unwrap(),
            default_horizon_roles: vec![20, 60],
            protected_window: false,
            protected_action_levels: Vec::new(),
            episode_template_id: ActionEpisodeTemplateId::SystemicCreditBankingCrisis,
            action_episode_overrides: None,
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
    fn protected_context_promotes_main_regime_buffer_without_changing_positive_labels() {
        let scenario_sets = super::load_formal_dataset_scenario_sets(
            super::DEFAULT_FORMAL_SCENARIO_SET_VERSION,
            super::DEFAULT_FORMAL_LABEL_VERSION,
        )
        .unwrap();
        let protected_date = NaiveDate::from_ymd_opt(2021, 11, 15).unwrap();
        let cooldown_date = NaiveDate::from_ymd_opt(2022, 11, 10).unwrap();

        assert_eq!(
            forward_crisis_label(protected_date, &scenario_sets.positive_scenarios, 20),
            0
        );
        assert_eq!(
            forward_crisis_training_regime_with_context(
                protected_date,
                &scenario_sets.positive_scenarios,
                &scenario_sets.context_scenarios,
                20,
            ),
            ProbabilityTrainingRegime::PreWarningBuffer
        );
        assert_eq!(
            forward_crisis_training_regime_with_context(
                cooldown_date,
                &scenario_sets.positive_scenarios,
                &scenario_sets.context_scenarios,
                20,
            ),
            ProbabilityTrainingRegime::PostCrisisCooldown
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
            scenario_training_role: None,
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
            prepare_episode_label: 0,
            hedge_episode_label: 1,
            defend_episode_label: 0,
            primary_action_level: Some("hedge".to_string()),
            action_episode_id: Some("scenario_a:hedge".to_string()),
            action_episode_phase: "primary".to_string(),
            protected_action_window: false,
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
        buffer_negative.regime_60d = ProbabilityTrainingRegime::PreWarningBuffer;
        let mut crisis_negative = normal_negative.clone();
        crisis_negative.primary_scenario_id = Some("scenario_a".to_string());
        crisis_negative.scenario_family = Some("systemic_credit_banking_crisis".to_string());
        crisis_negative.days_to_primary_crisis_start = Some(-5);
        crisis_negative.regime_20d = ProbabilityTrainingRegime::InCrisis;
        crisis_negative.regime_60d = ProbabilityTrainingRegime::InCrisis;
        let mut cooldown_negative = normal_negative.clone();
        cooldown_negative.primary_scenario_id = Some("scenario_a".to_string());
        cooldown_negative.scenario_family = Some("systemic_credit_banking_crisis".to_string());
        cooldown_negative.days_to_primary_crisis_start = Some(-35);
        cooldown_negative.regime_20d = ProbabilityTrainingRegime::PostCrisisCooldown;
        cooldown_negative.regime_60d = ProbabilityTrainingRegime::PostCrisisCooldown;

        assert_eq!(
            negative_sample_weight(
                &normal_negative,
                20,
                ProbabilityTargetLabelMode::ForwardCrisis,
            ),
            1.10
        );
        assert_eq!(
            negative_sample_weight(
                &buffer_negative,
                20,
                ProbabilityTargetLabelMode::ForwardCrisis,
            ),
            0.70
        );
        assert_eq!(
            negative_sample_weight(
                &buffer_negative,
                60,
                ProbabilityTargetLabelMode::ForwardCrisis,
            ),
            0.60
        );
        assert_eq!(
            negative_sample_weight(
                &crisis_negative,
                20,
                ProbabilityTargetLabelMode::ForwardCrisis,
            ),
            1.25
        );
        assert_eq!(
            negative_sample_weight(
                &cooldown_negative,
                20,
                ProbabilityTargetLabelMode::ForwardCrisis,
            ),
            1.45
        );
        let mut protected_negative = normal_negative.clone();
        protected_negative.primary_scenario_id = Some("scenario_protected".to_string());
        protected_negative.scenario_family = Some("mixed_systemic_stress".to_string());
        protected_negative.protected_action_window = true;
        assert_eq!(
            negative_sample_weight(
                &protected_negative,
                20,
                ProbabilityTargetLabelMode::ForwardCrisis,
            ),
            0.55
        );
        assert_eq!(
            negative_sample_weight(
                &protected_negative,
                60,
                ProbabilityTargetLabelMode::ForwardCrisis,
            ),
            0.65
        );
        let mut protected_cooldown_negative = protected_negative.clone();
        protected_cooldown_negative.action_episode_phase = "cooldown".to_string();
        assert_eq!(
            negative_sample_weight(
                &protected_cooldown_negative,
                20,
                ProbabilityTargetLabelMode::ForwardCrisis,
            ),
            1.20
        );
        assert_eq!(
            negative_sample_weight(
                &protected_cooldown_negative,
                60,
                ProbabilityTargetLabelMode::ForwardCrisis,
            ),
            1.35
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
        assert_eq!(selection.rows.len(), 4);
        assert!(!selection.used_full_split_fallback);
        assert_eq!(
            selection
                .rows
                .iter()
                .filter(|row| {
                    row.label_20d > 0
                        || matches!(
                            row.regime_20d,
                            ProbabilityTrainingRegime::Normal
                                | ProbabilityTrainingRegime::PreWarningBuffer
                                | ProbabilityTrainingRegime::InCrisis
                        )
                })
                .count(),
            4
        );
        assert_eq!(
            forward_crisis_regime_sample_weight(20, ProbabilityTrainingRegime::PositiveWindow),
            2.2
        );
    }

    #[test]
    fn formal_main_context_scenarios_include_protected_window_set() {
        let scenario_sets = super::load_formal_dataset_scenario_sets(
            super::DEFAULT_FORMAL_SCENARIO_SET_VERSION,
            super::DEFAULT_FORMAL_LABEL_VERSION,
        )
        .unwrap();
        let positive_ids = scenario_sets
            .positive_scenarios
            .iter()
            .map(|scenario| scenario.scenario_id.as_str())
            .collect::<BTreeSet<_>>();
        let context_ids = scenario_sets
            .context_scenarios
            .iter()
            .map(|scenario| scenario.scenario_id.as_str())
            .collect::<BTreeSet<_>>();

        assert_eq!(positive_ids.len(), 3);
        assert!(positive_ids.contains("us_gfc_2008"));
        assert!(positive_ids.contains("us_covid_liquidity_2020"));
        assert!(positive_ids.contains("us_regional_banks_2023"));

        assert!(context_ids.len() > positive_ids.len());
        assert!(context_ids.contains("us_dotcom_unwind_2000"));
        assert!(context_ids.contains("us_funding_stress_2011"));
        assert!(context_ids.contains("us_rate_shock_2022"));
    }

    #[test]
    fn forward_crisis_pairwise_targets_push_buffer_centroid_above_normal() {
        let make_row =
            |feature_value: f64, regime_20d: ProbabilityTrainingRegime, label_20d: u8| {
                let mut features = BTreeMap::new();
                features.insert("stress".to_string(), feature_value);
                ProbabilityTrainingRow {
                    as_of_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
                    market_scope: "financial_system".to_string(),
                    release_id: None,
                    probability_mode: Some("formal_bundle_v1".to_string()),
                    freshness_status: Some("a".to_string()),
                    time_to_risk_bucket: Some("weeks".to_string()),
                    split_name: Some("train".to_string()),
                    features,
                    primary_scenario_id: Some("scenario".to_string()),
                    scenario_family: Some("systemic_credit_banking_crisis".to_string()),
                    scenario_training_role: None,
                    days_to_primary_crisis_start: Some(10),
                    primary_scenario_supports_5d: false,
                    primary_scenario_supports_20d: true,
                    primary_scenario_supports_60d: true,
                    label_5d: 0,
                    label_20d,
                    label_60d: 0,
                    regime_5d: ProbabilityTrainingRegime::Normal,
                    regime_20d,
                    regime_60d: ProbabilityTrainingRegime::Normal,
                    action_label_5d: 0,
                    action_label_20d: label_20d,
                    action_label_60d: 0,
                    prepare_episode_label: 0,
                    hedge_episode_label: 0,
                    defend_episode_label: 0,
                    primary_action_level: None,
                    action_episode_id: None,
                    action_episode_phase: "outside".to_string(),
                    protected_action_window: false,
                }
            };

        let rows = vec![
            make_row(0.0, ProbabilityTrainingRegime::Normal, 0),
            make_row(0.1, ProbabilityTrainingRegime::Normal, 0),
            make_row(0.8, ProbabilityTrainingRegime::PreWarningBuffer, 0),
            make_row(0.9, ProbabilityTrainingRegime::PreWarningBuffer, 0),
            make_row(1.2, ProbabilityTrainingRegime::PositiveWindow, 1),
            make_row(1.1, ProbabilityTrainingRegime::PositiveWindow, 1),
            make_row(1.0, ProbabilityTrainingRegime::PostCrisisCooldown, 0),
        ];
        let feature_stats = vec![super::build_feature_stat(&rows, "stress")];
        let targets = forward_crisis_regime_pairwise_targets(
            &rows,
            &feature_stats,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
            false,
        );
        assert!(!targets.is_empty());
        assert!(targets.len() >= 5);
        let mut gradients = vec![0.0];
        super::apply_regime_pairwise_gradient(&mut gradients, &[0.0], &targets, 100.0, 20, false);
        assert!(gradients[0] < 0.0);
    }

    #[test]
    fn forward_crisis_sign_gradient_pushes_wrong_direction_coefficients_toward_zero() {
        let feature_names = vec![
            "us_baa_10y_spread_level".to_string(),
            "us_curve_10y2y_level".to_string(),
            "us_stlfsi_level".to_string(),
        ];
        let weights = vec![-0.8, 0.5, -0.4];
        let mut gradients = vec![0.0; weights.len()];

        super::apply_forward_crisis_sign_gradient(
            &mut gradients,
            &weights,
            &feature_names,
            100.0,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );

        assert!(gradients[0] < 0.0);
        assert!(gradients[1] > 0.0);
        assert!(gradients[2] < 0.0);
    }

    #[test]
    fn forward_crisis_sign_projection_clips_wrong_direction_coefficients() {
        let feature_names = vec![
            "us_baa_10y_spread_level".to_string(),
            "us_curve_10y2y_level".to_string(),
            "structural_score".to_string(),
            "us_usdjpy_change_20d".to_string(),
        ];
        let mut weights = vec![-0.8, 0.5, -0.2, -0.7];

        super::project_forward_crisis_sign_constraints(
            &mut weights,
            &feature_names,
            60,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );

        assert_eq!(weights[0], 0.0);
        assert_eq!(weights[1], 0.0);
        assert_eq!(weights[2], 0.0);
        assert_eq!(weights[3], -0.7);
    }

    #[test]
    fn forward_crisis_training_target_softens_buffer_and_cooldown_negatives() {
        let build_row = |regime_20d: ProbabilityTrainingRegime,
                         regime_60d: ProbabilityTrainingRegime,
                         label_20d: u8,
                         label_60d: u8| ProbabilityTrainingRow {
            as_of_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
            market_scope: "financial_system".to_string(),
            release_id: None,
            probability_mode: Some("formal_bundle_v1".to_string()),
            freshness_status: Some("a".to_string()),
            time_to_risk_bucket: Some("weeks".to_string()),
            split_name: Some("train".to_string()),
            features: BTreeMap::new(),
            primary_scenario_id: Some("scenario".to_string()),
            scenario_family: Some("systemic_credit_banking_crisis".to_string()),
            scenario_training_role: None,
            days_to_primary_crisis_start: Some(25),
            primary_scenario_supports_5d: false,
            primary_scenario_supports_20d: true,
            primary_scenario_supports_60d: true,
            label_5d: 0,
            label_20d,
            label_60d,
            regime_5d: ProbabilityTrainingRegime::Normal,
            regime_20d,
            regime_60d,
            action_label_5d: 0,
            action_label_20d: 0,
            action_label_60d: 0,
            prepare_episode_label: 0,
            hedge_episode_label: 0,
            defend_episode_label: 0,
            primary_action_level: None,
            action_episode_id: None,
            action_episode_phase: "outside".to_string(),
            protected_action_window: false,
        };

        let buffer_row = build_row(
            ProbabilityTrainingRegime::PreWarningBuffer,
            ProbabilityTrainingRegime::PreWarningBuffer,
            0,
            0,
        );
        let cooldown_row = build_row(
            ProbabilityTrainingRegime::PostCrisisCooldown,
            ProbabilityTrainingRegime::PostCrisisCooldown,
            0,
            0,
        );
        let positive_row = build_row(
            ProbabilityTrainingRegime::PositiveWindow,
            ProbabilityTrainingRegime::PositiveWindow,
            1,
            1,
        );

        assert_eq!(
            super::probability_training_target_label(
                &buffer_row,
                20,
                ProbabilityTargetLabelMode::ForwardCrisis
            ),
            0.18
        );
        assert_eq!(
            super::probability_training_target_label(
                &buffer_row,
                60,
                ProbabilityTargetLabelMode::ForwardCrisis
            ),
            0.26
        );
        assert_eq!(
            super::probability_training_target_label(
                &cooldown_row,
                20,
                ProbabilityTargetLabelMode::ForwardCrisis
            ),
            0.01
        );
        assert_eq!(
            super::probability_training_target_label(
                &cooldown_row,
                60,
                ProbabilityTargetLabelMode::ForwardCrisis
            ),
            0.02
        );
        assert_eq!(
            super::probability_training_target_label(
                &positive_row,
                20,
                ProbabilityTargetLabelMode::ForwardCrisis
            ),
            1.0
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
            scenario_training_role: Some("mandatory".to_string()),
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
            prepare_episode_label: 1,
            hedge_episode_label: 1,
            defend_episode_label: 0,
            primary_action_level: Some("hedge".to_string()),
            action_episode_id: Some("us_gfc_2008:hedge".to_string()),
            action_episode_phase: "primary".to_string(),
            protected_action_window: false,
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
            scenario_training_role: Some("mandatory".to_string()),
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
            prepare_episode_label: 0,
            hedge_episode_label: 1,
            defend_episode_label: 1,
            primary_action_level: Some("defend".to_string()),
            action_episode_id: Some("us_covid_liquidity_2020:defend".to_string()),
            action_episode_phase: "primary".to_string(),
            protected_action_window: false,
        };

        assert!(
            positive_sample_action_weight(&aligned, 60)
                > positive_sample_action_weight(&misaligned, 60)
        );
    }

    #[test]
    fn forward_crisis_positive_weight_boosts_extension_role_on_supported_horizon() {
        let mandatory = ProbabilityTrainingRow {
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
            scenario_training_role: Some("mandatory".to_string()),
            days_to_primary_crisis_start: Some(48),
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
            prepare_episode_label: 1,
            hedge_episode_label: 1,
            defend_episode_label: 0,
            primary_action_level: Some("hedge".to_string()),
            action_episode_id: Some("us_gfc_2008:hedge".to_string()),
            action_episode_phase: "primary".to_string(),
            protected_action_window: false,
        };
        let extension = ProbabilityTrainingRow {
            as_of_date: NaiveDate::from_ymd_opt(2011, 6, 20).unwrap(),
            market_scope: "financial_system".to_string(),
            release_id: None,
            probability_mode: Some("formal_bundle_v1".to_string()),
            freshness_status: Some("a".to_string()),
            time_to_risk_bucket: Some("us_funding_stress_2011".to_string()),
            split_name: Some("train".to_string()),
            features: BTreeMap::new(),
            primary_scenario_id: Some("us_funding_stress_2011".to_string()),
            scenario_family: Some("mixed_systemic_stress".to_string()),
            scenario_training_role: Some("extension_only".to_string()),
            days_to_primary_crisis_start: Some(39),
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
            prepare_episode_label: 1,
            hedge_episode_label: 1,
            defend_episode_label: 0,
            primary_action_level: Some("hedge".to_string()),
            action_episode_id: Some("us_funding_stress_2011:hedge".to_string()),
            action_episode_phase: "primary".to_string(),
            protected_action_window: true,
        };

        assert!(
            super::forward_crisis_positive_sample_weight(&extension, 60)
                > super::forward_crisis_positive_sample_weight(&mandatory, 60)
        );
    }

    #[test]
    fn render_dataset_csv_includes_scenario_role_columns() {
        let mut row = forward_crisis_row(
            NaiveDate::from_ymd_opt(2020, 2, 20).unwrap(),
            1,
            ProbabilityTrainingRegime::PositiveWindow,
        );
        row.scenario_training_role = Some("mandatory".to_string());
        row.features.insert("stress".to_string(), 0.42);

        let csv = super::render_dataset_csv(&[row], &[String::from("stress")]);
        let mut lines = csv.lines();
        let header = lines.next().unwrap_or_default();
        let first_row = lines.next().unwrap_or_default();

        assert!(header.contains("primary_scenario_id"));
        assert!(header.contains("scenario_family"));
        assert!(header.contains("scenario_training_role"));
        assert!(first_row.contains(",scenario_a,systemic_credit_banking_crisis,mandatory,"));
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
                    scenario_training_role: None,
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
                    prepare_episode_label: 0,
                    hedge_episode_label: u8::from(lead_days > 0),
                    defend_episode_label: 0,
                    primary_action_level: (lead_days > 0).then_some("hedge".to_string()),
                    action_episode_id: Some(format!("{scenario_id}:hedge")),
                    action_episode_phase: if lead_days > 0 {
                        "primary".to_string()
                    } else {
                        "late_validation".to_string()
                    },
                    protected_action_window: false,
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
            scenario_training_role: None,
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
            prepare_episode_label: 0,
            hedge_episode_label: 0,
            defend_episode_label: 0,
            primary_action_level: None,
            action_episode_id: None,
            action_episode_phase: "outside".to_string(),
            protected_action_window: false,
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
        assert_eq!(summary.actual_positive_count, 3);
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
            scenario_training_role: None,
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
            prepare_episode_label: 0,
            hedge_episode_label: u8::from(lead_days > 0),
            defend_episode_label: 0,
            primary_action_level: (lead_days > 0).then_some("hedge".to_string()),
            action_episode_id: Some(format!("{scenario_id}:hedge")),
            action_episode_phase: if lead_days > 0 {
                "primary".to_string()
            } else {
                "late_validation".to_string()
            },
            protected_action_window: false,
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
            scenario_training_role: None,
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
            prepare_episode_label: 0,
            hedge_episode_label: u8::from(lead_days > 0),
            defend_episode_label: 0,
            primary_action_level: (lead_days > 0).then_some("hedge".to_string()),
            action_episode_id: Some(format!("{scenario_id}:hedge")),
            action_episode_phase: if lead_days > 0 {
                "primary".to_string()
            } else {
                "late_validation".to_string()
            },
            protected_action_window: false,
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
            scenario_training_role: None,
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
            prepare_episode_label: 0,
            hedge_episode_label: 0,
            defend_episode_label: 0,
            primary_action_level: None,
            action_episode_id: None,
            action_episode_phase: "outside".to_string(),
            protected_action_window: false,
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
                    feature_transform: PROBABILITY_FEATURE_TRANSFORM_IDENTITY_V1.to_string(),
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
                    regime_separation: None,
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
            scenario_training_role: None,
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
            prepare_episode_label: 0,
            hedge_episode_label: u8::from(lead_days > 0),
            defend_episode_label: 0,
            primary_action_level: (lead_days > 0).then_some("hedge".to_string()),
            action_episode_id: Some(format!("{scenario_id}:hedge")),
            action_episode_phase: if lead_days > 0 {
                "primary".to_string()
            } else {
                "late_validation".to_string()
            },
            protected_action_window: false,
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
        let calibration_rows = vec![
            forward_crisis_row(
                NaiveDate::from_ymd_opt(2020, 1, 5).unwrap(),
                1,
                ProbabilityTrainingRegime::PositiveWindow,
            ),
            forward_crisis_row(
                NaiveDate::from_ymd_opt(2020, 1, 6).unwrap(),
                1,
                ProbabilityTrainingRegime::PositiveWindow,
            ),
            forward_crisis_row(
                NaiveDate::from_ymd_opt(2020, 1, 20).unwrap(),
                0,
                ProbabilityTrainingRegime::Normal,
            ),
            forward_crisis_row(
                NaiveDate::from_ymd_opt(2020, 1, 21).unwrap(),
                0,
                ProbabilityTrainingRegime::Normal,
            ),
        ];
        let calibration_row_refs = calibration_rows.iter().collect::<Vec<_>>();
        let calibration_candidate = PlattCalibrationArtifact {
            alpha: -1.4,
            beta: -3.0,
            min_input: 0.11,
            max_input: 0.82,
        };

        let (calibration, evaluation_probabilities) = select_probability_calibration_strategy(
            &raw_probabilities,
            &labels,
            &calibration_row_refs,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
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
        let calibration_rows = vec![
            forward_crisis_row(
                NaiveDate::from_ymd_opt(2020, 1, 5).unwrap(),
                1,
                ProbabilityTrainingRegime::PositiveWindow,
            ),
            forward_crisis_row(
                NaiveDate::from_ymd_opt(2020, 1, 6).unwrap(),
                1,
                ProbabilityTrainingRegime::PositiveWindow,
            ),
            forward_crisis_row(
                NaiveDate::from_ymd_opt(2020, 1, 20).unwrap(),
                0,
                ProbabilityTrainingRegime::Normal,
            ),
            forward_crisis_row(
                NaiveDate::from_ymd_opt(2020, 1, 21).unwrap(),
                0,
                ProbabilityTrainingRegime::Normal,
            ),
        ];
        let calibration_row_refs = calibration_rows.iter().collect::<Vec<_>>();
        let calibration_candidate = fit_platt_calibration(&raw_probabilities, &labels);
        assert!(calibration_candidate.alpha < 0.0);

        let (calibration, evaluation_probabilities) = select_probability_calibration_strategy(
            &raw_probabilities,
            &labels,
            &calibration_row_refs,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
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
    fn probability_calibration_strategy_keeps_raw_when_calibration_flattens_early_warning() {
        let calibration_rows = vec![
            forward_crisis_row(
                NaiveDate::from_ymd_opt(2020, 1, 5).unwrap(),
                1,
                ProbabilityTrainingRegime::PositiveWindow,
            ),
            forward_crisis_row(
                NaiveDate::from_ymd_opt(2020, 1, 7).unwrap(),
                1,
                ProbabilityTrainingRegime::PositiveWindow,
            ),
            forward_crisis_row(
                NaiveDate::from_ymd_opt(2020, 1, 12).unwrap(),
                0,
                ProbabilityTrainingRegime::PreWarningBuffer,
            ),
            forward_crisis_row(
                NaiveDate::from_ymd_opt(2020, 1, 20).unwrap(),
                0,
                ProbabilityTrainingRegime::Normal,
            ),
            forward_crisis_row(
                NaiveDate::from_ymd_opt(2020, 2, 3).unwrap(),
                0,
                ProbabilityTrainingRegime::InCrisis,
            ),
        ];
        let calibration_row_refs = calibration_rows.iter().collect::<Vec<_>>();
        let raw_probabilities = vec![0.72, 0.68, 0.44, 0.12, 0.61];
        let labels = calibration_rows
            .iter()
            .map(|row| row.label_20d as f64)
            .collect::<Vec<_>>();
        let flattening_calibration = PlattCalibrationArtifact {
            alpha: 0.02,
            beta: -4.2,
            min_input: 0.12,
            max_input: 0.72,
        };

        let (calibration, evaluation_probabilities) = select_probability_calibration_strategy(
            &raw_probabilities,
            &labels,
            &calibration_row_refs,
            20,
            ProbabilityTargetLabelMode::ForwardCrisis,
            &raw_probabilities,
            flattening_calibration,
        );

        assert!(calibration.is_none());
        assert_eq!(evaluation_probabilities, raw_probabilities);
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
    fn probability_decision_threshold_raises_cutoff_when_low_threshold_is_overbroad() {
        let probabilities = vec![0.38, 0.36, 0.35, 0.34, 0.33, 0.32, 0.31, 0.30, 0.29, 0.28];
        let labels = vec![1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];

        let threshold = select_probability_decision_threshold(&probabilities, &labels, 20);

        assert!(threshold >= 0.35);
        assert!(threshold > 0.30);
    }

    #[test]
    fn probability_decision_threshold_keeps_more_recall_for_60d_when_precision_tradeoff_is_small() {
        let probabilities = vec![0.45, 0.40, 0.35, 0.30, 0.25, 0.34, 0.28, 0.22, 0.18, 0.12];
        let labels = vec![1.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0];

        let threshold = select_probability_decision_threshold(&probabilities, &labels, 60);

        assert!(threshold <= 0.30);
        assert!(threshold >= 0.25);
    }

    #[test]
    fn regime_support_adjustment_lowers_60d_threshold_when_base_misses_prewarning_buffer() {
        let build_row =
            |regime_60d: ProbabilityTrainingRegime, label_60d: u8| ProbabilityTrainingRow {
                as_of_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
                market_scope: "financial_system".to_string(),
                release_id: None,
                probability_mode: Some("formal_bundle_v1".to_string()),
                freshness_status: Some("fresh".to_string()),
                time_to_risk_bucket: Some("test".to_string()),
                split_name: Some("calibration".to_string()),
                features: BTreeMap::new(),
                primary_scenario_id: Some("scenario".to_string()),
                scenario_family: Some("systemic_credit_banking_crisis".to_string()),
                scenario_training_role: None,
                days_to_primary_crisis_start: Some(20),
                primary_scenario_supports_5d: true,
                primary_scenario_supports_20d: true,
                primary_scenario_supports_60d: true,
                label_5d: 0,
                label_20d: 0,
                label_60d,
                regime_5d: ProbabilityTrainingRegime::Normal,
                regime_20d: ProbabilityTrainingRegime::Normal,
                regime_60d,
                action_label_5d: 0,
                action_label_20d: 0,
                action_label_60d: 0,
                prepare_episode_label: 0,
                hedge_episode_label: 0,
                defend_episode_label: 0,
                primary_action_level: None,
                action_episode_id: None,
                action_episode_phase: "outside".to_string(),
                protected_action_window: false,
            };

        let rows = vec![
            build_row(ProbabilityTrainingRegime::PositiveWindow, 1),
            build_row(ProbabilityTrainingRegime::PositiveWindow, 1),
            build_row(ProbabilityTrainingRegime::InCrisis, 1),
            build_row(ProbabilityTrainingRegime::InCrisis, 1),
            build_row(ProbabilityTrainingRegime::PreWarningBuffer, 0),
            build_row(ProbabilityTrainingRegime::PreWarningBuffer, 0),
            build_row(ProbabilityTrainingRegime::Normal, 0),
            build_row(ProbabilityTrainingRegime::Normal, 0),
            build_row(ProbabilityTrainingRegime::Normal, 0),
            build_row(ProbabilityTrainingRegime::PostCrisisCooldown, 0),
        ];
        let row_refs = rows.iter().collect::<Vec<_>>();
        let probabilities = vec![0.95, 0.91, 0.84, 0.80, 0.58, 0.56, 0.22, 0.18, 0.14, 0.10];
        let labels = rows
            .iter()
            .map(|row| row.label_60d as f64)
            .collect::<Vec<_>>();
        let calibration_selection = ProbabilityCalibrationSelection {
            rows: row_refs.clone(),
            eligible_row_count: row_refs.len(),
            eligible_positive_count: labels.iter().filter(|label| **label >= 0.5).count(),
            eligible_negative_count: labels.iter().filter(|label| **label < 0.5).count(),
            used_full_split_fallback: false,
        };
        let threshold_selection = ProbabilityThresholdSelection {
            rows: row_refs.clone(),
            probabilities: probabilities.clone(),
            labels: labels.clone(),
            used_full_split_fallback: false,
        };

        let base_threshold = select_probability_decision_threshold(&probabilities, &labels, 60);
        let adjusted_threshold = adjust_probability_decision_threshold_for_regime_support(
            base_threshold,
            &probabilities,
            &labels,
            &row_refs,
            60,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );
        let diagnostics =
            build_probability_threshold_diagnostics(ProbabilityThresholdDiagnosticsInput {
                full_calibration_rows: &rows,
                calibration_selection: &calibration_selection,
                threshold_selection: &threshold_selection,
                horizon_days: 60,
                label_mode: ProbabilityTargetLabelMode::ForwardCrisis,
                base_threshold,
                final_threshold: adjusted_threshold,
            });

        assert!(base_threshold > 0.58);
        assert!(adjusted_threshold <= base_threshold);
        assert!(diagnostics.repair_applied);
        assert!(matches!(
            diagnostics.repair_reason.as_str(),
            "repaired_to_early_warning_cap" | "repaired_to_regime_support_candidate"
        ));
        assert_eq!(diagnostics.base_summary.early_warning_hit_count, 0);
        assert!(diagnostics.final_summary.early_warning_hit_count > 0);
    }

    #[test]
    fn threshold_selection_excludes_in_crisis_negatives_for_60d_forward_crisis() {
        let build_row =
            |regime_60d: ProbabilityTrainingRegime, label_60d: u8| ProbabilityTrainingRow {
                as_of_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
                market_scope: "financial_system".to_string(),
                release_id: None,
                probability_mode: Some("formal_bundle_v1".to_string()),
                freshness_status: Some("fresh".to_string()),
                time_to_risk_bucket: Some("test".to_string()),
                split_name: Some("calibration".to_string()),
                features: BTreeMap::new(),
                primary_scenario_id: Some("scenario".to_string()),
                scenario_family: Some("systemic_credit_banking_crisis".to_string()),
                scenario_training_role: None,
                days_to_primary_crisis_start: Some(20),
                primary_scenario_supports_5d: true,
                primary_scenario_supports_20d: true,
                primary_scenario_supports_60d: true,
                label_5d: 0,
                label_20d: 0,
                label_60d,
                regime_5d: ProbabilityTrainingRegime::Normal,
                regime_20d: ProbabilityTrainingRegime::Normal,
                regime_60d,
                action_label_5d: 0,
                action_label_20d: 0,
                action_label_60d: 0,
                prepare_episode_label: 0,
                hedge_episode_label: 0,
                defend_episode_label: 0,
                primary_action_level: None,
                action_episode_id: None,
                action_episode_phase: "outside".to_string(),
                protected_action_window: false,
            };

        let rows = vec![
            build_row(ProbabilityTrainingRegime::PositiveWindow, 1),
            build_row(ProbabilityTrainingRegime::PreWarningBuffer, 0),
            build_row(ProbabilityTrainingRegime::Normal, 0),
            build_row(ProbabilityTrainingRegime::PostCrisisCooldown, 0),
            build_row(ProbabilityTrainingRegime::InCrisis, 0),
        ];
        let row_refs = rows.iter().collect::<Vec<_>>();
        let probabilities = vec![0.9, 0.55, 0.20, 0.10, 0.88];
        let labels = rows
            .iter()
            .map(|row| row.label_60d as f64)
            .collect::<Vec<_>>();

        let selection = probability_decision_threshold_selection(
            &probabilities,
            &labels,
            &row_refs,
            60,
            ProbabilityTargetLabelMode::ForwardCrisis,
        );

        assert!(!selection.used_full_split_fallback);
        assert_eq!(selection.rows.len(), 4);
        assert_eq!(
            selection
                .labels
                .iter()
                .filter(|label| **label >= 0.5)
                .count(),
            1
        );
        assert_eq!(
            selection
                .labels
                .iter()
                .filter(|label| **label < 0.5)
                .count(),
            3
        );
        assert!(selection
            .rows
            .iter()
            .all(|row| row.regime_60d != ProbabilityTrainingRegime::InCrisis));
    }

    #[test]
    fn runtime_regime_separation_detects_calibration_crush() {
        let scenarios = synthetic_runtime_scenarios();
        let history = vec![
            runtime_history_point(NaiveDate::from_ymd_opt(1999, 12, 20).unwrap(), 0.10, 0.020),
            runtime_history_point(NaiveDate::from_ymd_opt(2000, 1, 5).unwrap(), 0.30, 0.021),
            runtime_history_point(NaiveDate::from_ymd_opt(2000, 1, 20).unwrap(), 0.45, 0.022),
            runtime_history_point(NaiveDate::from_ymd_opt(2000, 2, 5).unwrap(), 0.55, 0.023),
        ];

        let summaries = summarize_release_runtime_regime_probabilities(&history, &scenarios, None);
        let separation = summarize_release_runtime_regime_separation(&summaries);
        let row20 = separation
            .iter()
            .find(|row| row.horizon_days == 20)
            .expect("20d summary");

        assert_eq!(row20.early_warning_regime, "pre_warning_buffer");
        assert_eq!(row20.diagnosis, "calibration_crushed_early_warning");
        assert!(
            row20
                .early_warning_raw_lift_vs_normal
                .expect("raw lift should exist")
                >= 2.9
        );
        assert!(
            row20
                .early_warning_calibrated_lift_vs_normal
                .expect("calibrated lift should exist")
                < 1.1
        );
        assert!(
            row20
                .early_warning_gap_retention
                .expect("gap retention should exist")
                < 0.1
        );
    }

    #[test]
    fn runtime_regime_classifier_flags_cooldown_bleed() {
        let diagnosis = classify_regime_separation(
            20,
            1.7,
            1.6,
            Some(0.9),
            1.55,
            0.014,
            1.3,
            1.58,
            0.015,
            1.58,
            0.05,
        );

        assert_eq!(diagnosis, "cooldown_bleed");
    }

    #[test]
    fn offline_regime_classifier_uses_positive_window_gap_not_only_buffer_lift() {
        let diagnosis =
            classify_probability_regime_separation(20, 1.6, 1.52, 1.6, 1.2, 1.1, 0.012, 0.004, 1.6);

        assert_eq!(diagnosis, "usable_early_warning_separation");
    }

    #[test]
    fn probability_guardrails_reject_zero_usable_early_warning_horizons() {
        let bundle = ProbabilityBundle {
            bundle_id: "candidate_guard_zero".to_string(),
            market_scope: "financial_system".to_string(),
            probability_mode: "formal_bundle_v1".to_string(),
            model_family: PROBABILITY_MODEL_FAMILY_LINEAR_V1.to_string(),
            feature_transform: PROBABILITY_FEATURE_TRANSFORM_IDENTITY_V1.to_string(),
            created_at: Utc::now(),
            feature_names: Vec::new(),
            monotonic_min_gap_5d_to_20d: 0.0,
            monotonic_min_gap_20d_to_60d: 0.0,
            note: "test".to_string(),
            horizons: Vec::new(),
            evaluation: Some(ProbabilityBundleEvaluation {
                sample_count: 100,
                brier_score: 0.1,
                log_loss: 0.2,
                ece: 0.1,
                regime_separation_summaries: vec![RegimeSeparationEvaluationSummary {
                    horizon_days: 20,
                    early_warning_regime: "pre_warning_buffer".to_string(),
                    normal_sample_count: 50,
                    pre_warning_buffer_sample_count: 10,
                    positive_window_sample_count: 10,
                    early_warning_sample_count: 10,
                    in_crisis_sample_count: 10,
                    post_crisis_cooldown_sample_count: 10,
                    normal_avg_probability: 0.02,
                    pre_warning_buffer_avg_probability: 0.025,
                    positive_window_avg_probability: 0.019,
                    early_warning_avg_probability: 0.025,
                    in_crisis_avg_probability: 0.04,
                    post_crisis_cooldown_avg_probability: 0.03,
                    max_non_normal_avg_probability: 0.04,
                    pre_warning_buffer_lift_vs_normal: Some(1.25),
                    positive_window_lift_vs_normal: Some(0.95),
                    early_warning_lift_vs_normal: Some(1.25),
                    in_crisis_lift_vs_normal: Some(2.0),
                    post_crisis_cooldown_lift_vs_normal: Some(1.5),
                    positive_window_gap_vs_normal: Some(-0.001),
                    post_crisis_cooldown_gap_vs_normal: Some(0.01),
                    max_non_normal_lift_vs_normal: Some(2.0),
                    diagnosis: "cold_across_all_regimes".to_string(),
                }],
                usable_early_warning_horizon_count: 0,
                insufficient_early_warning_horizon_count: 1,
                note: "test".to_string(),
            }),
            actionability: None,
        };
        let release = test_release_with_bundle(&bundle);
        let bundle_path = release.manifest.bundle_uri.clone();

        let regressions = compare_probability_guardrails(&release).unwrap();

        let _ = std::fs::remove_file(bundle_path);
        assert!(regressions
            .iter()
            .any(|item| item.contains("zero usable early-warning horizons")));
        assert!(regressions
            .iter()
            .any(|item| item.contains("20d positive_window avg")));
        assert!(regressions
            .iter()
            .any(|item| item.contains("cold_across_all_regimes")));
    }

    #[test]
    fn probability_guardrails_reject_cooldown_bleed_on_medium_horizons() {
        let bundle = ProbabilityBundle {
            bundle_id: "candidate_guard_cooldown".to_string(),
            market_scope: "financial_system".to_string(),
            probability_mode: "formal_bundle_v1".to_string(),
            model_family: PROBABILITY_MODEL_FAMILY_LINEAR_V1.to_string(),
            feature_transform: PROBABILITY_FEATURE_TRANSFORM_IDENTITY_V1.to_string(),
            created_at: Utc::now(),
            feature_names: Vec::new(),
            monotonic_min_gap_5d_to_20d: 0.0,
            monotonic_min_gap_20d_to_60d: 0.0,
            note: "test".to_string(),
            horizons: Vec::new(),
            evaluation: Some(ProbabilityBundleEvaluation {
                sample_count: 100,
                brier_score: 0.1,
                log_loss: 0.2,
                ece: 0.1,
                regime_separation_summaries: vec![RegimeSeparationEvaluationSummary {
                    horizon_days: 60,
                    early_warning_regime: "pre_warning_buffer".to_string(),
                    normal_sample_count: 50,
                    pre_warning_buffer_sample_count: 10,
                    positive_window_sample_count: 10,
                    early_warning_sample_count: 10,
                    in_crisis_sample_count: 10,
                    post_crisis_cooldown_sample_count: 10,
                    normal_avg_probability: 0.03,
                    pre_warning_buffer_avg_probability: 0.05,
                    positive_window_avg_probability: 0.065,
                    early_warning_avg_probability: 0.05,
                    in_crisis_avg_probability: 0.06,
                    post_crisis_cooldown_avg_probability: 0.068,
                    max_non_normal_avg_probability: 0.068,
                    pre_warning_buffer_lift_vs_normal: Some(1.67),
                    positive_window_lift_vs_normal: Some(2.17),
                    early_warning_lift_vs_normal: Some(1.67),
                    in_crisis_lift_vs_normal: Some(2.0),
                    post_crisis_cooldown_lift_vs_normal: Some(2.27),
                    positive_window_gap_vs_normal: Some(0.035),
                    post_crisis_cooldown_gap_vs_normal: Some(0.038),
                    max_non_normal_lift_vs_normal: Some(2.27),
                    diagnosis: "cooldown_bleed".to_string(),
                }],
                usable_early_warning_horizon_count: 1,
                insufficient_early_warning_horizon_count: 1,
                note: "test".to_string(),
            }),
            actionability: None,
        };
        let release = test_release_with_bundle(&bundle);
        let bundle_path = release.manifest.bundle_uri.clone();

        let regressions = compare_probability_guardrails(&release).unwrap();

        let _ = std::fs::remove_file(bundle_path);
        assert!(regressions
            .iter()
            .any(|item| item.contains("cooldown_bleed")));
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
                primary_positive_count: 12,
                late_validation_row_count: 4,
                protected_row_count: 0,
                primary_hit_count: 0,
                late_validation_hit_count: 0,
                protected_hit_count: 0,
                false_positive_count: 0,
                scenario_count: 1,
                on_time_scenario_count: 0,
                late_only_scenario_count: 0,
                missed_scenario_count: 1,
                precision_at_threshold: None,
                primary_recall_at_threshold: Some(0.0),
                late_validation_capture_rate: Some(0.0),
                on_time_rate: Some(0.0),
                late_only_rate: Some(0.0),
                missed_rate: Some(1.0),
                note: "test".to_string(),
            }],
            guard_regressions: Vec::new(),
            guard_passed: true,
            note: "test".to_string(),
        };

        let regressions = compare_actionability_guardrails(&review);
        assert!(regressions
            .iter()
            .any(|item| item.contains("scenario_count")));
        assert!(regressions
            .iter()
            .any(|item| item.contains("produced no primary or late-validation hits")));
        assert!(regressions.iter().any(|item| item.contains("on_time_rate")));
        assert!(regressions.iter().any(|item| item.contains("missed_rate")));
    }

    #[test]
    fn actionability_guardrails_apply_level_specific_rate_thresholds() {
        let review = ReleaseActionabilityReview {
            release_id: "candidate".to_string(),
            enabled: true,
            model_version: Some("actionability_bundle_test".to_string()),
            calibration_version: Some("actionability_platt_test".to_string()),
            fusion_policy_version: Some("fusion_policy_test".to_string()),
            levels: vec![ReleaseActionabilityLevelReview {
                level: ActionabilityLevel::Defend,
                proxy_horizon_days: 5,
                sample_count: 120,
                positive_rate: 0.04,
                threshold: 0.12,
                predicted_positive_count: 5,
                primary_positive_count: 10,
                late_validation_row_count: 7,
                protected_row_count: 0,
                primary_hit_count: 1,
                late_validation_hit_count: 3,
                protected_hit_count: 0,
                false_positive_count: 1,
                scenario_count: 3,
                on_time_scenario_count: 0,
                late_only_scenario_count: 2,
                missed_scenario_count: 1,
                precision_at_threshold: Some(0.2),
                primary_recall_at_threshold: Some(0.33),
                late_validation_capture_rate: Some(0.43),
                on_time_rate: Some(0.0),
                late_only_rate: Some(0.67),
                missed_rate: Some(0.33),
                note: "test".to_string(),
            }],
            guard_regressions: Vec::new(),
            guard_passed: true,
            note: "test".to_string(),
        };

        let regressions = compare_actionability_guardrails(&review);
        assert!(regressions
            .iter()
            .any(|item| item.contains("late_only_rate")));
        assert!(!regressions.iter().any(|item| item.contains("on_time_rate")));
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
                    scenario_training_role: scenario_id.map(|_| "mandatory".to_string()),
                    label_5d: u8::from(matches!(index, 56..=59 | 106..=109 | 156..=159)),
                    label_20d: u8::from(matches!(index, 52..=59 | 102..=109 | 152..=159)),
                    label_60d: u8::from(matches!(index, 44..=59 | 94..=109 | 144..=159)),
                    regime_5d: "normal".to_string(),
                    regime_20d: "normal".to_string(),
                    regime_60d: "normal".to_string(),
                    action_label_5d: u8::from(matches!(index, 55..=59 | 105..=109 | 155..=159)),
                    action_label_20d: u8::from(matches!(index, 50..=59 | 100..=109 | 150..=159)),
                    action_label_60d: u8::from(matches!(index, 42..=59 | 92..=109 | 142..=159)),
                    prepare_episode_label: u8::from(
                        matches!(index, 42..=59 | 92..=109 | 142..=159),
                    ),
                    hedge_episode_label: u8::from(matches!(index, 50..=59 | 100..=109 | 150..=159)),
                    defend_episode_label: u8::from(
                        matches!(index, 55..=59 | 105..=109 | 155..=159),
                    ),
                    primary_action_level: None,
                    action_episode_id: None,
                    action_episode_phase: "outside".to_string(),
                    protected_action_window: false,
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

        let split_requirements = formal_dataset_split_requirements("formal_label_v1_main");
        let (train_end, calibration_end) =
            scenario_aware_formal_split_bounds(&rows, &ranges, split_requirements).unwrap();
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
        assert!(label_support.split_has_required_label_support(0, train_end, split_requirements));
        assert!(label_support.split_has_required_label_support(
            train_end,
            calibration_end,
            split_requirements
        ));
        assert!(label_support.split_has_required_label_support(
            calibration_end,
            rows.len(),
            split_requirements
        ));
    }

    #[test]
    fn extension_acute_split_allows_two_scenarios_with_single_scenario_evaluation() {
        let rows = (0..220)
            .map(|index| {
                let scenario_id = match index {
                    40..=69 => Some("acute_a"),
                    150..=179 => Some("acute_b"),
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
                        .map(|_| "acute_market_liquidity_crash".to_string()),
                    scenario_training_role: scenario_id.map(|_| "extension_only".to_string()),
                    label_5d: u8::from(matches!(index, 62..=69 | 172..=179)),
                    label_20d: u8::from(matches!(index, 50..=69 | 160..=179)),
                    label_60d: 0,
                    regime_5d: "normal".to_string(),
                    regime_20d: "normal".to_string(),
                    regime_60d: "normal".to_string(),
                    action_label_5d: u8::from(matches!(index, 62..=69 | 172..=179)),
                    action_label_20d: u8::from(matches!(index, 50..=69 | 160..=179)),
                    action_label_60d: 0,
                    prepare_episode_label: u8::from(matches!(index, 48..=69 | 158..=179)),
                    hedge_episode_label: u8::from(matches!(index, 56..=69 | 166..=179)),
                    defend_episode_label: u8::from(matches!(index, 62..=69 | 172..=179)),
                    primary_action_level: None,
                    action_episode_id: None,
                    action_episode_phase: "outside".to_string(),
                    protected_action_window: false,
                    features: BTreeMap::new(),
                    created_at: Utc::now(),
                }
            })
            .collect::<Vec<_>>();

        let ranges = vec![
            ScenarioRowRange {
                scenario_id: "acute_a".to_string(),
                family: "acute_market_liquidity_crash".to_string(),
                start_index: 40,
                end_index: 69,
            },
            ScenarioRowRange {
                scenario_id: "acute_b".to_string(),
                family: "acute_market_liquidity_crash".to_string(),
                start_index: 150,
                end_index: 179,
            },
        ];

        let split_requirements = formal_dataset_split_requirements("formal_label_v1_ext_acute");
        let (train_end, calibration_end) =
            scenario_aware_formal_split_bounds(&rows, &ranges, split_requirements).unwrap();

        assert!((62..=69).contains(&train_end));
        assert!((172..=179).contains(&calibration_end));

        let label_support = FormalSplitLabelSupport::from_rows(&rows);
        assert!(label_support.split_has_required_label_support(0, train_end, split_requirements));
        assert!(label_support.split_has_required_label_support(
            train_end,
            calibration_end,
            split_requirements
        ));
        assert!(label_support.split_has_required_label_support(
            calibration_end,
            rows.len(),
            split_requirements
        ));

        assert_eq!(
            scenario_count_for_index_range(&rows, calibration_end, rows.len()),
            1
        );
    }

    #[test]
    fn extension_stress_split_uses_20d_60d_prepare_hedge_requirements() {
        let rows = (0..260)
            .map(|index| {
                let scenario_id = match index {
                    30..=59 => Some("stress_a"),
                    90..=119 => Some("stress_b"),
                    150..=179 => Some("stress_c"),
                    210..=239 => Some("stress_d"),
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
                    scenario_family: scenario_id.map(|_| "mixed_systemic_stress".to_string()),
                    scenario_training_role: scenario_id.map(|_| "extension_only".to_string()),
                    label_5d: 0,
                    label_20d: u8::from(
                        matches!(index, 42..=59 | 102..=119 | 162..=179 | 222..=239),
                    ),
                    label_60d: u8::from(
                        matches!(index, 34..=59 | 94..=119 | 154..=179 | 214..=239),
                    ),
                    regime_5d: "normal".to_string(),
                    regime_20d: "normal".to_string(),
                    regime_60d: "normal".to_string(),
                    action_label_5d: 0,
                    action_label_20d: u8::from(
                        matches!(index, 42..=59 | 102..=119 | 162..=179 | 222..=239),
                    ),
                    action_label_60d: u8::from(
                        matches!(index, 34..=59 | 94..=119 | 154..=179 | 214..=239),
                    ),
                    prepare_episode_label: u8::from(
                        matches!(index, 34..=59 | 94..=119 | 154..=179 | 214..=239),
                    ),
                    hedge_episode_label: u8::from(
                        matches!(index, 42..=59 | 102..=119 | 162..=179 | 222..=239),
                    ),
                    defend_episode_label: 0,
                    primary_action_level: None,
                    action_episode_id: None,
                    action_episode_phase: "outside".to_string(),
                    protected_action_window: true,
                    features: BTreeMap::new(),
                    created_at: Utc::now(),
                }
            })
            .collect::<Vec<_>>();

        let ranges = vec![
            ScenarioRowRange {
                scenario_id: "stress_a".to_string(),
                family: "mixed_systemic_stress".to_string(),
                start_index: 30,
                end_index: 59,
            },
            ScenarioRowRange {
                scenario_id: "stress_b".to_string(),
                family: "mixed_systemic_stress".to_string(),
                start_index: 90,
                end_index: 119,
            },
            ScenarioRowRange {
                scenario_id: "stress_c".to_string(),
                family: "mixed_systemic_stress".to_string(),
                start_index: 150,
                end_index: 179,
            },
            ScenarioRowRange {
                scenario_id: "stress_d".to_string(),
                family: "mixed_systemic_stress".to_string(),
                start_index: 210,
                end_index: 239,
            },
        ];

        let split_requirements = formal_dataset_split_requirements("formal_label_v1_ext_stress");
        let (train_end, calibration_end) =
            scenario_aware_formal_split_bounds(&rows, &ranges, split_requirements).unwrap();

        assert!((42..=59).contains(&train_end));
        assert!((162..=179).contains(&calibration_end));

        let label_support = FormalSplitLabelSupport::from_rows(&rows);
        assert!(label_support.split_has_required_label_support(0, train_end, split_requirements));
        assert!(label_support.split_has_required_label_support(
            train_end,
            calibration_end,
            split_requirements
        ));
        assert!(label_support.split_has_required_label_support(
            calibration_end,
            rows.len(),
            split_requirements
        ));
    }
}

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use chrono::{NaiveDate, Utc};
use fc_domain::Frequency;
use fc_ingestion::{
    Connector, FetchPlan, FredConnector, FredGraphCsvConnector, MockConnector, RunMode,
    TreasuryYieldCurveConnector,
};
use fc_storage::{
    ExternalIndicatorMapping, RawResponseRecord, SqliteStore, FRED_DATASET_ID,
    TREASURY_YIELD_DATASET_ID,
};

const DEFAULT_SQLITE_PATH: &str = "data/fc-local.sqlite";
const DEFAULT_RAW_DATA_DIR: &str = "data/raw";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_target(false).init();

    let args = env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [] => run_demo_ingestion().await,
        [scope, action] if scope == "db" && action == "init" => db_init().await,
        [scope, action] if scope == "db" && action == "seed" => db_seed().await,
        [scope, source, rest @ ..] if scope == "backfill" && source == "fred" => {
            backfill_fred(rest).await
        }
        [scope, source, rest @ ..] if scope == "backfill" && source == "treasury-yield" => {
            backfill_treasury_yield(rest).await
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
    println!("Seeded FRED and Treasury metadata into {}", sqlite_path());
    Ok(())
}

async fn backfill_fred(args: &[String]) -> anyhow::Result<()> {
    let options = FredBackfillOptions::parse(args)?;
    let store = open_sqlite_store().await?;
    store.migrate().await?;
    store.seed_fred_metadata().await?;

    let connector: Box<dyn Connector> = match options.fred_mode {
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

    backfill_mappings(
        connector.as_ref(),
        mappings,
        FRED_DATASET_ID,
        options.options,
        "FRED",
    )
    .await
}

async fn backfill_treasury_yield(args: &[String]) -> anyhow::Result<()> {
    let options = BackfillOptions::parse(args)?;
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

async fn backfill_mappings(
    connector: &dyn Connector,
    mappings: Vec<ExternalIndicatorMapping>,
    dataset_id: &str,
    options: BackfillOptions,
    label: &str,
) -> anyhow::Result<()> {
    let store = open_sqlite_store().await?;
    let raw_root = raw_data_dir();
    let mut total_written = 0_usize;
    for mapping in mappings {
        let plan = FetchPlan {
            source_id: connector.describe().source_id,
            dataset_id: dataset_id.to_string(),
            target_id: mapping.indicator_id.clone(),
            external_code: Some(mapping.external_code.clone()),
            run_mode: RunMode::Backfill,
            requested_start: Some(options.start),
            requested_end: Some(options.end),
            frequency: mapping.frequency,
        };
        tracing::info!(
            indicator_id = %plan.target_id,
            external_code = %mapping.external_code,
            source_id = %plan.source_id,
            "fetching observations"
        );
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
            .insert_observations_with_raw_payload(&batch.observations, Some(payload.raw_payload_id))
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
                start = %options.start,
                end = %options.end,
                "no observations were written for requested range"
            );
        }
        total_written += written;
        println!(
            "backfilled {} ({}) from {} with {} observations",
            mapping.indicator_id, mapping.external_code, payload.source_id, written
        );
        for warning in batch.warnings.iter().take(3) {
            tracing::warn!(%warning, indicator_id = %mapping.indicator_id, "parse warning");
        }
    }

    println!(
        "{} backfill completed: {} observations written to {}",
        label,
        total_written,
        sqlite_path()
    );
    Ok(())
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
}

impl BackfillOptions {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut start = NaiveDate::from_ymd_opt(1990, 1, 1).expect("valid date");
        let mut end = Utc::now().date_naive();
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
                other => bail!("unknown backfill option: {other}"),
            }
            index += 1;
        }
        if start > end {
            bail!("--start must be on or before --end");
        }
        Ok(Self { start, end })
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

#[derive(Debug, Clone, Copy)]
enum FredBackfillMode {
    GraphCsv,
    Api,
}

fn parse_date_arg(value: Option<&String>, option: &str) -> anyhow::Result<NaiveDate> {
    let value = value.with_context(|| format!("{option} requires a YYYY-MM-DD value"))?;
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .with_context(|| format!("{option} must use YYYY-MM-DD"))
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

fn print_help() {
    println!(
        r#"fc-worker commands:
  cargo run -p fc-worker
      Run the original mock ingestion demo.

  cargo run -p fc-worker -- db init
      Create or migrate the local SQLite database.

  cargo run -p fc-worker -- db seed
      Seed FRED, Treasury, entity, indicator, and mapping metadata.

  cargo run -p fc-worker -- backfill fred [--start YYYY-MM-DD] [--end YYYY-MM-DD]
      Fetch FRED public graph CSV observations into SQLite. No API key required.

  cargo run -p fc-worker -- backfill fred --api [--start YYYY-MM-DD] [--end YYYY-MM-DD]
      Fetch FRED official API observations into SQLite. Requires FRED_API_KEY.

  cargo run -p fc-worker -- backfill treasury-yield [--start YYYY-MM-DD] [--end YYYY-MM-DD]
      Fetch U.S. Treasury yield curve observations into SQLite. No API key required.
"#
    );
}

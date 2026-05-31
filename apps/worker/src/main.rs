use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use chrono::{NaiveDate, Utc};
use fc_domain::Frequency;
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
use uuid::Uuid;

const DEFAULT_SQLITE_PATH: &str = "data/fc-local.sqlite";
const DEFAULT_RAW_DATA_DIR: &str = "data/raw";
const DEFAULT_API_RELOAD_URL: &str = "http://127.0.0.1:18080/api/system/reload";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_target(false).init();

    let args = env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [] => run_demo_ingestion().await,
        [scope, action] if scope == "db" && action == "init" => db_init().await,
        [scope, action] if scope == "db" && action == "seed" => db_seed().await,
        [scope, action] if scope == "db" && action == "check" => db_check().await,
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
        .timeout(std::time::Duration::from_secs(8))
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
    use super::RefreshLatestOptions;

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
}

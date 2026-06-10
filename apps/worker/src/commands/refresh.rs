use anyhow::{bail, Context, Result};
use chrono::Utc;
use fc_ingestion::BojDataset;
use fc_storage::IngestionSourceHealthSummary;

use super::backfill::{
    backfill_boj_with_options, backfill_fred_with_options, backfill_gdelt_with_options,
    backfill_sec_edgar_with_options, backfill_treasury_yield_with_options,
    backfill_world_bank_with_options, BackfillOptions, BojBackfillOptions, FredBackfillMode,
    FredBackfillOptions,
};

#[derive(Debug, Clone)]
pub(crate) struct RefreshLatestOptions {
    pub(crate) fast_lookback_days: i64,
    pub(crate) slow_lookback_years: i64,
    pub(crate) fred_chunk_days: i64,
    pub(crate) skip_world_bank: bool,
    pub(crate) include_gdelt: bool,
    pub(crate) reload_api: bool,
    pub(crate) api_reload_url: String,
}

impl RefreshLatestOptions {
    pub(crate) fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut fast_lookback_days = 45_i64;
        let mut slow_lookback_years = 15_i64;
        let mut fred_chunk_days = 45_i64;
        let mut skip_world_bank = false;
        let mut include_gdelt = false;
        let mut reload_api = true;
        let mut api_reload_url = crate::DEFAULT_API_RELOAD_URL.to_string();
        let mut index = 0;

        while index < args.len() {
            match args[index].as_str() {
                "--fast-lookback-days" => {
                    index += 1;
                    fast_lookback_days =
                        crate::parse_positive_i64(args.get(index), "--fast-lookback-days")?;
                }
                "--slow-lookback-years" => {
                    index += 1;
                    slow_lookback_years =
                        crate::parse_positive_i64(args.get(index), "--slow-lookback-years")?;
                }
                "--fred-chunk-days" => {
                    index += 1;
                    fred_chunk_days =
                        crate::parse_positive_i64(args.get(index), "--fred-chunk-days")?;
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

pub(crate) async fn handle_refresh_command(action: &str, rest: &[String]) -> Result<()> {
    match action {
        "latest-free" => refresh_latest_free(rest).await,
        "status" => refresh_status(rest).await,
        _ => {
            super::print_help();
            bail!("unknown refresh command: {action}")
        }
    }
}

async fn refresh_latest_free(args: &[String]) -> Result<()> {
    let options = RefreshLatestOptions::parse(args)?;
    let today = Utc::now().date_naive();
    let fast_start = today - chrono::Duration::days(options.fast_lookback_days);
    let slow_start = today - chrono::Duration::days(options.slow_lookback_years * 366);

    println!(
        "Refreshing latest free data into {} (fast window {}..{}, slow window {}..{})",
        crate::sqlite_path(),
        fast_start,
        today,
        slow_start,
        today
    );

    super::db_init().await?;
    super::db_seed().await?;

    let total_stages =
        5 + if options.skip_world_bank { 0 } else { 1 } + if options.include_gdelt { 1 } else { 0 };

    println!("Stage 1/{total_stages}: FRED market series");
    backfill_fred_with_options(FredBackfillOptions {
        options: BackfillOptions {
            start: fast_start,
            end: today,
            chunk_days: Some(options.fred_chunk_days),
            indicator_filter: None,
            external_code_filter: None,
            watermark_overlap_days: None,
            respect_frequency_watermark: false,
        }
        .with_frequency_watermark_refresh(),
        fred_mode: FredBackfillMode::GraphCsv,
    })
    .await?;

    println!("Stage 2/{total_stages}: Treasury yield curve");
    backfill_treasury_yield_with_options(
        BackfillOptions {
            start: fast_start,
            end: today,
            chunk_days: Some(options.fast_lookback_days.min(180)),
            indicator_filter: None,
            external_code_filter: None,
            watermark_overlap_days: None,
            respect_frequency_watermark: false,
        }
        .with_frequency_watermark_refresh(),
    )
    .await?;

    println!("Stage 3/{total_stages}: BOJ FX");
    backfill_boj_with_options(BojBackfillOptions {
        dataset: BojDataset::FxDaily,
        options: BackfillOptions {
            start: fast_start,
            end: today,
            chunk_days: Some(options.fast_lookback_days.min(180)),
            indicator_filter: None,
            external_code_filter: None,
            watermark_overlap_days: None,
            respect_frequency_watermark: false,
        }
        .with_frequency_watermark_refresh(),
    })
    .await?;

    println!("Stage 4/{total_stages}: BOJ money market");
    backfill_boj_with_options(BojBackfillOptions {
        dataset: BojDataset::MoneyMarketRates,
        options: BackfillOptions {
            start: fast_start,
            end: today,
            chunk_days: Some(options.fast_lookback_days.min(180)),
            indicator_filter: None,
            external_code_filter: None,
            watermark_overlap_days: None,
            respect_frequency_watermark: false,
        }
        .with_frequency_watermark_refresh(),
    })
    .await?;

    println!("Stage 5/{total_stages}: SEC EDGAR");
    backfill_sec_edgar_with_options(
        BackfillOptions {
            start: fast_start,
            end: today,
            chunk_days: None,
            indicator_filter: None,
            external_code_filter: None,
            watermark_overlap_days: None,
            respect_frequency_watermark: false,
        }
        .with_frequency_watermark_refresh(),
    )
    .await?;

    let mut next_stage = 6;
    if !options.skip_world_bank {
        println!("Stage {next_stage}/{total_stages}: World Bank slow variables");
        backfill_world_bank_with_options(
            BackfillOptions {
                start: slow_start,
                end: today,
                chunk_days: None,
                indicator_filter: None,
                external_code_filter: None,
                watermark_overlap_days: None,
                respect_frequency_watermark: false,
            }
            .with_frequency_watermark_refresh(),
        )
        .await?;
        next_stage += 1;
    }

    if options.include_gdelt {
        println!("Stage {next_stage}/{total_stages}: GDELT prototype events");
        backfill_gdelt_with_options(
            BackfillOptions {
                start: fast_start,
                end: today,
                chunk_days: None,
                indicator_filter: None,
                external_code_filter: None,
                watermark_overlap_days: Some(7),
                respect_frequency_watermark: false,
            }
            .with_frequency_watermark_refresh(),
        )
        .await?;
    }

    super::db_check().await?;

    if options.reload_api {
        match crate::reload_api_runtime(&options.api_reload_url).await {
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

async fn refresh_status(args: &[String]) -> Result<()> {
    if !args.is_empty() {
        bail!("refresh status does not accept options yet");
    }

    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;
    let summaries = store.load_ingestion_source_health_summaries().await?;
    if summaries.is_empty() {
        println!(
            "No ingestion runs found in {}. Run `just refresh-latest` or a backfill command first.",
            crate::sqlite_path()
        );
        return Ok(());
    }

    println!("Free-data refresh status from {}", crate::sqlite_path());
    println!(
        "{:<12} {:<12} {:<24} {:<12} {:<8} {}",
        "source", "latest", "last_success", "data_period", "failures", "message"
    );
    for summary in summaries {
        println!("{}", render_status_row(&summary));
    }
    Ok(())
}

fn render_status_row(summary: &IngestionSourceHealthSummary) -> String {
    let latest = summary
        .latest_status
        .as_deref()
        .unwrap_or("unknown")
        .to_string();
    let last_success = summary
        .last_success_at
        .map(|timestamp| timestamp.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "never".to_string());
    let data_period = summary
        .last_successful_period
        .map(|period| period.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let message = summary
        .latest_error_message
        .as_deref()
        .map(truncate_status_message)
        .unwrap_or_else(|| {
            format!(
                "{} run(s), {} success",
                summary.total_run_count, summary.successful_run_count
            )
        });

    format!(
        "{:<12} {:<12} {:<24} {:<12} {:<8} {}",
        summary.source_id,
        latest,
        last_success,
        data_period,
        summary.failures_after_last_success,
        message
    )
}

fn truncate_status_message(message: &str) -> String {
    const MAX_STATUS_MESSAGE_CHARS: usize = 96;
    let truncated = message
        .chars()
        .take(MAX_STATUS_MESSAGE_CHARS)
        .collect::<String>();
    if message.chars().count() > MAX_STATUS_MESSAGE_CHARS {
        format!("{truncated}...")
    } else {
        truncated
    }
}

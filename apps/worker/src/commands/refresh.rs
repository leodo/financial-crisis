use anyhow::{bail, Result};
use chrono::Utc;
use fc_ingestion::BojDataset;

pub(crate) async fn handle_refresh_command(action: &str, rest: &[String]) -> Result<()> {
    match action {
        "latest-free" => refresh_latest_free(rest).await,
        _ => {
            super::print_help();
            bail!("unknown refresh command: {action}")
        }
    }
}

async fn refresh_latest_free(args: &[String]) -> Result<()> {
    let options = crate::RefreshLatestOptions::parse(args)?;
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

    crate::backfill_fred_with_options(crate::FredBackfillOptions {
        options: crate::BackfillOptions {
            start: fast_start,
            end: today,
            chunk_days: Some(options.fred_chunk_days),
            indicator_filter: None,
            external_code_filter: None,
            watermark_overlap_days: None,
        },
        fred_mode: crate::FredBackfillMode::GraphCsv,
    })
    .await?;

    crate::backfill_treasury_yield_with_options(crate::BackfillOptions {
        start: fast_start,
        end: today,
        chunk_days: Some(options.fast_lookback_days.min(180)),
        indicator_filter: None,
        external_code_filter: None,
        watermark_overlap_days: None,
    })
    .await?;

    crate::backfill_boj_with_options(crate::BojBackfillOptions {
        dataset: BojDataset::FxDaily,
        options: crate::BackfillOptions {
            start: fast_start,
            end: today,
            chunk_days: Some(options.fast_lookback_days.min(180)),
            indicator_filter: None,
            external_code_filter: None,
            watermark_overlap_days: None,
        },
    })
    .await?;

    crate::backfill_boj_with_options(crate::BojBackfillOptions {
        dataset: BojDataset::MoneyMarketRates,
        options: crate::BackfillOptions {
            start: fast_start,
            end: today,
            chunk_days: Some(options.fast_lookback_days.min(180)),
            indicator_filter: None,
            external_code_filter: None,
            watermark_overlap_days: None,
        },
    })
    .await?;

    crate::backfill_sec_edgar_with_options(crate::BackfillOptions {
        start: fast_start,
        end: today,
        chunk_days: None,
        indicator_filter: None,
        external_code_filter: None,
        watermark_overlap_days: None,
    })
    .await?;

    if !options.skip_world_bank {
        crate::backfill_world_bank_with_options(crate::BackfillOptions {
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
        crate::backfill_gdelt_with_options(crate::BackfillOptions {
            start: fast_start,
            end: today,
            chunk_days: None,
            indicator_filter: None,
            external_code_filter: None,
            watermark_overlap_days: Some(7),
        })
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

use anyhow::{bail, Result};
use chrono::Utc;

pub(crate) async fn handle_db_command(action: &str) -> Result<()> {
    match action {
        "init" => db_init().await,
        "seed" => db_seed().await,
        "check" => db_check().await,
        _ => {
            super::print_help();
            bail!("unknown db command: {action}")
        }
    }
}

pub(crate) async fn db_init() -> Result<()> {
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;
    println!("SQLite database initialized at {}", crate::sqlite_path());
    Ok(())
}

pub(crate) async fn db_seed() -> Result<()> {
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;
    store.seed_fred_metadata().await?;
    println!(
        "Seeded FRED, Treasury, BOJ, SEC EDGAR, and World Bank metadata into {}",
        crate::sqlite_path()
    );
    Ok(())
}

pub(crate) async fn db_check() -> Result<()> {
    let store = crate::open_sqlite_store().await?;
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

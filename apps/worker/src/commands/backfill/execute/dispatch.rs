use anyhow::{bail, Result};

use super::super::options::{BackfillOptions, BojBackfillOptions, FredBackfillOptions};
use super::events::{backfill_gdelt_with_options, backfill_sec_edgar_with_options};
use super::market::{
    backfill_boj_with_options, backfill_fred_with_options, backfill_jpy_carry_with_options,
    backfill_treasury_yield_with_options, backfill_world_bank_with_options,
};

pub(crate) async fn handle_backfill_command(source: &str, rest: &[String]) -> Result<()> {
    match source {
        "fred" => backfill_fred(rest).await,
        "treasury-yield" => backfill_treasury_yield(rest).await,
        "world-bank" => backfill_world_bank(rest).await,
        "gdelt" => backfill_gdelt(rest).await,
        "sec-edgar" => backfill_sec_edgar(rest).await,
        "boj" => backfill_boj(rest).await,
        "jpy-carry" => backfill_jpy_carry(rest).await,
        _ => {
            super::super::super::print_help();
            bail!("unknown backfill source: {source}")
        }
    }
}

async fn backfill_fred(args: &[String]) -> anyhow::Result<()> {
    let options = FredBackfillOptions::parse(args)?;
    backfill_fred_with_options(options).await
}

async fn backfill_treasury_yield(args: &[String]) -> anyhow::Result<()> {
    let options = BackfillOptions::parse(args)?;
    backfill_treasury_yield_with_options(options).await
}

async fn backfill_world_bank(args: &[String]) -> anyhow::Result<()> {
    let options = BackfillOptions::parse(args)?;
    backfill_world_bank_with_options(options).await
}

async fn backfill_gdelt(args: &[String]) -> anyhow::Result<()> {
    let options = BackfillOptions::parse(args)?;
    backfill_gdelt_with_options(options).await
}

async fn backfill_sec_edgar(args: &[String]) -> anyhow::Result<()> {
    let options = BackfillOptions::parse(args)?;
    backfill_sec_edgar_with_options(options).await
}

async fn backfill_jpy_carry(args: &[String]) -> anyhow::Result<()> {
    let options = BackfillOptions::parse(args)?;
    backfill_jpy_carry_with_options(options).await
}

async fn backfill_boj(args: &[String]) -> anyhow::Result<()> {
    let options = BojBackfillOptions::parse(args)?;
    backfill_boj_with_options(options).await
}

use anyhow::{bail, Result};

pub(crate) async fn handle_backfill_command(source: &str, rest: &[String]) -> Result<()> {
    match source {
        "fred" => crate::backfill_fred(rest).await,
        "treasury-yield" => crate::backfill_treasury_yield(rest).await,
        "world-bank" => crate::backfill_world_bank(rest).await,
        "gdelt" => crate::backfill_gdelt(rest).await,
        "sec-edgar" => crate::backfill_sec_edgar(rest).await,
        "boj" => crate::backfill_boj(rest).await,
        "jpy-carry" => crate::backfill_jpy_carry(rest).await,
        _ => {
            super::print_help();
            bail!("unknown backfill source: {source}")
        }
    }
}

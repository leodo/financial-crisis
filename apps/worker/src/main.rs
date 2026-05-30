use chrono::NaiveDate;
use fc_domain::Frequency;
use fc_ingestion::{Connector, FetchPlan, MockConnector, RunMode};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_target(false).init();

    let connector = MockConnector;
    let plan = FetchPlan {
        source_id: "mock".to_string(),
        dataset_id: "demo".to_string(),
        target_id: "us_market_vix_close".to_string(),
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

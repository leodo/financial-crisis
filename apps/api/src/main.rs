#[tokio::main]
async fn main() -> anyhow::Result<()> {
    fc_api::run().await
}

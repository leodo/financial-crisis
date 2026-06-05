#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_target(false).init();

    let args = std::env::args().skip(1).collect::<Vec<_>>();
    fc_worker::run_from_args(args).await
}

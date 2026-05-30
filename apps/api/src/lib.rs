mod config;
mod demo;
mod handlers;
mod state;

use std::sync::Arc;

use axum::Router;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

pub use config::AppConfig;
pub use state::{AppData, AppState};

pub async fn run() -> anyhow::Result<()> {
    init_tracing();
    let config = AppConfig::from_env();
    let state = Arc::new(AppState::new(demo::load_app_data().await));
    let app = router(state);
    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
    tracing::info!(bind_addr = %config.bind_addr, "fc-api listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", axum::routing::get(handlers::health))
        .route("/api/overview", axum::routing::get(handlers::overview))
        .route("/api/dimensions", axum::routing::get(handlers::dimensions))
        .route("/api/indicators", axum::routing::get(handlers::indicators))
        .route(
            "/api/indicators/:indicator_id",
            axum::routing::get(handlers::indicator_detail),
        )
        .route("/api/alerts", axum::routing::get(handlers::alerts))
        .route("/api/sources", axum::routing::get(handlers::sources))
        .route("/api/backtests", axum::routing::get(handlers::backtests))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info,fc_api=debug,tower_http=info".into());
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };
    ctrl_c.await;
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    use crate::{demo, router, AppState};

    #[tokio::test]
    async fn health_endpoint_works() {
        let app = router(Arc::new(AppState::new(demo::build_demo_data())));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(response.status().is_success());
    }
}

mod assessment;
mod backtest;
mod config;
mod data_source;
mod demo;
mod demo_seed;
mod handlers;
mod history_builder;
mod history_replay;
mod state;

use std::sync::Arc;

use axum::{
    extract::Request,
    http::{
        header::{CACHE_CONTROL, EXPIRES, PRAGMA},
        HeaderValue,
    },
    middleware::{self, Next},
    response::Response,
    Router,
};
use chrono::Utc;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

pub use config::AppConfig;
pub use data_source::{AppDataSource, AssessmentHistoryBuildMode};
pub use state::{AppData, AppState};

pub async fn run() -> anyhow::Result<()> {
    init_tracing();
    let config = AppConfig::from_env();
    let source = data_source::source_from_env()?;
    let data = data_source::load_app_data(&source, config.max_history_points).await?;
    let state = Arc::new(AppState::new(
        data,
        source.clone(),
        config.default_history_points,
        config.max_history_points,
    ));
    if config.refresh_interval_seconds > 0 && !matches!(source, data_source::AppDataSource::Demo) {
        tokio::spawn(refresh_loop(state.clone(), config.refresh_interval_seconds));
    }
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
        .route(
            "/api/events/recent",
            axum::routing::get(handlers::events_recent),
        )
        .route("/api/sources", axum::routing::get(handlers::sources))
        .route("/api/backtests", axum::routing::get(handlers::backtests))
        .route(
            "/api/backtests/timeline",
            axum::routing::get(handlers::backtest_timeline),
        )
        .route(
            "/api/assessment/current",
            axum::routing::get(handlers::assessment_current),
        )
        .route(
            "/api/assessment/history",
            axum::routing::get(handlers::assessment_history),
        )
        .route(
            "/api/assessment/analogs",
            axum::routing::get(handlers::assessment_analogs),
        )
        .route(
            "/api/assessment/data-trust",
            axum::routing::get(handlers::assessment_data_trust),
        )
        .route(
            "/api/assessment/posture",
            axum::routing::get(handlers::assessment_posture),
        )
        .route(
            "/api/assessment/method",
            axum::routing::get(handlers::assessment_method),
        )
        .route(
            "/api/research/audit",
            axum::routing::get(handlers::research_audit),
        )
        .route(
            "/api/system/reload",
            axum::routing::post(handlers::system_reload),
        )
        .layer(middleware::from_fn(disable_cache))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn disable_cache(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        CACHE_CONTROL,
        HeaderValue::from_static("no-store, max-age=0, must-revalidate"),
    );
    response
        .headers_mut()
        .insert(PRAGMA, HeaderValue::from_static("no-cache"));
    response
        .headers_mut()
        .insert(EXPIRES, HeaderValue::from_static("0"));
    response
}

async fn refresh_loop(state: Arc<AppState>, refresh_interval_seconds: u64) {
    let interval = std::time::Duration::from_secs(refresh_interval_seconds);
    loop {
        tokio::time::sleep(interval).await;
        match state.reload().await {
            Ok(data) => tracing::info!(
                data_mode = ?data.data_mode,
                as_of_date = %data.assessment.as_of_date,
                generated_at = %data.assessment.runtime.generated_at,
                "fc-api state refreshed"
            ),
            Err(error) => tracing::warn!(
                refreshed_at = %Utc::now(),
                error = %format!("{error:#}"),
                "fc-api background refresh failed"
            ),
        }
    }
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
mod tests;

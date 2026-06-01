mod assessment;
mod config;
mod demo;
mod handlers;
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
pub use state::{AppData, AppState};

pub async fn run() -> anyhow::Result<()> {
    init_tracing();
    let config = AppConfig::from_env();
    let source = demo::source_from_env()?;
    let data = demo::load_app_data(&source, config.max_history_points).await?;
    let state = Arc::new(AppState::new(
        data,
        source.clone(),
        config.default_history_points,
        config.max_history_points,
    ));
    if config.refresh_interval_seconds > 0 && !matches!(source, demo::AppDataSource::Demo) {
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
mod tests {
    use std::sync::Arc;

    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    use crate::{demo, router, AppState};

    async fn get_json(uri: &str) -> serde_json::Value {
        let app = router(Arc::new(AppState::new(
            demo::build_demo_data(260),
            demo::AppDataSource::Demo,
            260,
            260,
        )));
        let response = app
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert!(
            response.status().is_success(),
            "{uri} returned {}",
            response.status()
        );
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    #[tokio::test]
    async fn health_endpoint_works() {
        let app = router(Arc::new(AppState::new(
            demo::build_demo_data(260),
            demo::AppDataSource::Demo,
            260,
            260,
        )));
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

    #[tokio::test]
    async fn assessment_current_includes_jpy_carry_funding_fields() {
        let json = get_json("/api/assessment/current").await;
        assert!(json["jpy_carry"]["jp_call_rate"].is_number());
        assert!(json["jpy_carry"]["us_short_rate"].is_number());
        assert!(json["jpy_carry"]["us_jp_short_rate_diff"].is_number());
        assert!(json["jpy_carry"]["funding_pressure_score"].is_number());
        assert!(json["method"]["probability_mode"].is_string());
        assert!(json["method"]["release_status"].is_string());
        assert!(json["method"]["action_playbook_version"].is_string());
        assert!(json["method"]["point_in_time_mode"].is_string());
        assert!(json["position_guidance"]["action_playbook_version"].is_string());
        assert!(json["position_guidance"]["execution_urgency"].is_string());
        assert!(json["position_guidance"]["confidence_gate"].is_string());
        assert!(json["position_guidance"]["target_equity_exposure_pct"].is_number());
        assert!(json["position_guidance"]["target_cash_pct"].is_number());
        assert!(json["position_guidance"]["hedge_ratio_pct"].is_number());
        assert!(json["position_guidance"]["leverage_cap_pct"].is_number());
        assert!(json["position_guidance"]["option_overlay_pct"].is_number());
        assert!(json["position_guidance"]["action_summary"].is_string());
        assert!(json["position_guidance"]["actions"].is_array());
        assert!(json["position_guidance"]["forbidden_actions"].is_array());
        assert!(json["position_guidance"]["reentry_conditions"].is_array());
        assert!(json["position_guidance"]["guardrails"].is_array());
        assert!(json["position_guidance"]["capital_preservation_overlay_enabled"].is_boolean());
    }

    #[tokio::test]
    async fn assessment_current_includes_runtime_freshness_and_supporting_blocks() {
        let json = get_json("/api/assessment/current").await;
        assert_eq!(json["runtime"]["data_mode"], "demo");
        assert_eq!(json["runtime"]["demo_mode"], true);
        assert!(json["runtime"]["stale_warning"].is_string());

        let key_indicators = json["key_indicators"].as_array().unwrap();
        assert!(key_indicators.iter().any(|indicator| {
            indicator["display_name"] == "USDJPY"
                && indicator["latest_as_of_date"].is_string()
                && indicator["source_id"].is_string()
                && indicator["status"] == "stale"
        }));
        assert!(json["event_assessment"]["state"].is_string());
        assert!(json["event_assessment"]["confirmed_signals"].is_array());
        assert!(json["actionability"]["prepare"].is_number());
        assert!(json["actionability"]["hedge"].is_number());
        assert!(json["actionability"]["defend"].is_number());
        assert!(json["method"]["actionability_enabled"].is_boolean());
        assert!(json["backtest_summary"]["timely_warning_rate"].is_number());
        assert!(json["backtest_summary"]["rolling_audit"]["actionable_precision"].is_number());
        assert!(json["backtest_summary"]["rolling_audit"]["classified_episodes"].is_array());
        assert!(json["backtest_summary"]["rolling_audit"]["summary"].is_string());
        assert!(json["user_preferences"]["profile"].is_string());
    }

    #[tokio::test]
    async fn events_recent_endpoint_returns_event_signals() {
        let json = get_json("/api/events/recent").await;
        let events = json.as_array().unwrap();
        assert!(!events.is_empty());
        assert!(events[0]["alert_id"].is_string());
        assert!(events[0]["event_type"].is_string());
        assert!(events[0]["triggered_as_of_date"].is_string());
        assert!(events[0]["related_indicators"].is_array());
    }

    #[tokio::test]
    async fn backtest_timeline_endpoint_returns_window_points() {
        let json = get_json("/api/backtests/timeline").await;
        let points = json.as_array().unwrap();
        assert!(!points.is_empty());
        assert!(points[0]["as_of_date"].is_string());
        assert!(points[0]["p_5d"].is_number());
        assert!(points[0]["p_20d"].is_number());
        assert!(points[0]["p_60d"].is_number());
        assert!(points[0]["posture"].is_string());
        assert!(points[0]["crisis_window_open"].is_boolean());
    }

    #[tokio::test]
    async fn assessment_history_endpoint_honors_limit_query() {
        let json = get_json("/api/assessment/history?limit=5").await;
        let points = json.as_array().unwrap();
        assert_eq!(points.len(), 5);
    }

    #[tokio::test]
    async fn research_audit_endpoint_returns_runtime_audit_shape() {
        let json = get_json("/api/research/audit").await;
        assert!(json["supported"].is_boolean());
        assert!(json["storage_mode"].is_string());
        assert!(json["runtime_probability_mode"].is_string());
        assert!(json["runtime_release_status"].is_string());
        assert!(json["releases"].is_array());
        assert!(json["snapshots"].is_array());
    }

    #[tokio::test]
    async fn system_reload_endpoint_returns_refresh_metadata() {
        let app = router(Arc::new(AppState::new(
            demo::build_demo_data(260),
            demo::AppDataSource::Demo,
            260,
            260,
        )));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/system/reload")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(response.status().is_success());

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
        assert!(json["generated_at"].is_string());
    }

    #[tokio::test]
    async fn assessment_posture_endpoint_returns_upgrade_and_downgrade_rules() {
        let json = get_json("/api/assessment/posture").await;
        assert!(json["posture"].is_string());
        assert!(json["summary"].is_string());
        assert!(json["reasons"].is_array());
        assert!(json["upgrade_condition"].is_string());
        assert!(json["downgrade_condition"].is_string());
    }

    #[tokio::test]
    async fn assessment_method_endpoint_returns_protected_stress_window_catalog() {
        let json = get_json("/api/assessment/method").await;
        assert!(json["method"]["score_method_version"].is_string());
        assert!(json["method"]["probability_mode"].is_string());
        assert!(json["method"]["release_status"].is_string());
        assert!(json["method"]["action_playbook_version"].is_string());
        assert!(json["method"]["point_in_time_mode"].is_string());
        assert!(json["note"].is_string());
        assert!(json["protected_stress_window_catalog"]["catalog_id"].is_string());
        assert!(json["protected_stress_window_catalog"]["windows"].is_array());
        assert!(json["runtime_thresholds"]["prepare_p60d"].is_number());
        assert!(json["runtime_thresholds"]["hedge_p20d"].is_number());
        assert!(json["runtime_thresholds"]["defend_p5d"].is_number());
        assert!(json["runtime_thresholds"]["history_runtime_policy_version"].is_string());
    }

    #[tokio::test]
    async fn api_responses_disable_browser_cache() {
        let app = router(Arc::new(AppState::new(
            demo::build_demo_data(260),
            demo::AppDataSource::Demo,
            260,
            260,
        )));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/assessment/current")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            response.headers().get("cache-control").unwrap(),
            "no-store, max-age=0, must-revalidate"
        );
        assert_eq!(response.headers().get("pragma").unwrap(), "no-cache");
    }
}

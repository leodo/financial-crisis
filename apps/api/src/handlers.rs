use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde_json::json;

use crate::AppState;

pub async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "fc-api"
    }))
}

pub async fn overview(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(json!(state.data().overview))
}

pub async fn dimensions(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(json!(state.data().overview.dimensions))
}

pub async fn indicators(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(json!(state.data().indicators))
}

pub async fn indicator_detail(
    State(state): State<Arc<AppState>>,
    Path(indicator_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    state
        .data()
        .indicators
        .iter()
        .find(|risk| risk.indicator.indicator_id == indicator_id)
        .map(|risk| Json(json!(risk)))
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn alerts(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(json!(state.data().alerts))
}

pub async fn sources(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(json!(state.data().sources))
}

pub async fn backtests(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(json!(state.data().backtests))
}

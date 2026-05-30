use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::json;

use crate::{
    demo::{select_assessment_history, select_backtest_timeline, HistoryQueryWindow},
    AppState,
};

#[derive(Debug, Default, Deserialize)]
pub struct HistoryQuery {
    from: Option<String>,
    to: Option<String>,
    limit: Option<usize>,
}

impl HistoryQuery {
    fn into_window(self, default_limit: usize) -> Result<HistoryQueryWindow, StatusCode> {
        Ok(HistoryQueryWindow {
            from: parse_date(self.from)?,
            to: parse_date(self.to)?,
            limit: Some(self.limit.unwrap_or(default_limit)),
        })
    }
}

fn parse_date(value: Option<String>) -> Result<Option<NaiveDate>, StatusCode> {
    value
        .map(|raw| NaiveDate::parse_from_str(&raw, "%Y-%m-%d").map_err(|_| StatusCode::BAD_REQUEST))
        .transpose()
}

pub async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "fc-api"
    }))
}

pub async fn overview(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let data = state.data().await;
    Json(json!(data.overview))
}

pub async fn dimensions(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let data = state.data().await;
    Json(json!(data.overview.dimensions))
}

pub async fn indicators(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let data = state.data().await;
    Json(json!(data.indicators))
}

pub async fn indicator_detail(
    State(state): State<Arc<AppState>>,
    Path(indicator_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let data = state.data().await;
    data.indicators
        .iter()
        .find(|risk| risk.indicator.indicator_id == indicator_id)
        .map(|risk| Json(json!(risk)))
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn alerts(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let data = state.data().await;
    Json(json!(data.alerts))
}

pub async fn sources(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let data = state.data().await;
    Json(json!(data.sources))
}

pub async fn backtests(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let data = state.data().await;
    Json(json!(data.backtests))
}

pub async fn backtest_timeline(
    State(state): State<Arc<AppState>>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let window = query.into_window(state.default_history_points())?;
    let data = state.data().await;
    Ok(Json(json!(select_backtest_timeline(
        &data.backtest_timeline,
        window,
    ))))
}

pub async fn events_recent(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let data = state.data().await;
    Json(json!(data.alerts))
}

pub async fn assessment_current(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let data = state.data().await;
    Json(json!(data.assessment))
}

pub async fn assessment_history(
    State(state): State<Arc<AppState>>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let window = query.into_window(state.default_history_points())?;
    let data = state.data().await;
    Ok(Json(json!(select_assessment_history(
        &data.assessment_history,
        window,
    ))))
}

pub async fn assessment_analogs(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let data = state.data().await;
    Json(json!(data.assessment.historical_analogs))
}

pub async fn assessment_data_trust(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let data = state.data().await;
    Json(json!(data.assessment.data_trust))
}

pub async fn assessment_posture(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let data = state.data().await;
    Json(json!(data.posture_guidance))
}

pub async fn assessment_method(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let data = state.data().await;
    Json(json!({
        "method": data.assessment.method,
        "note": "assessment 概率、风险强度和 posture 为不同层的输出；当前版本为启发式 MVP，不是校准后的正式危机概率模型。页面应优先检查 data_mode、关键指标日期和 stale warning，再解释当前数值。"
    }))
}

pub async fn system_reload(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let data = state
        .reload()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({
        "status": "ok",
        "data_mode": data.data_mode,
        "as_of_date": data.assessment.as_of_date,
        "generated_at": data.assessment.runtime.generated_at,
    })))
}

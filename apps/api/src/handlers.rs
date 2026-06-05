use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::json;

mod research_audit;

pub(crate) use research_audit::research_audit;

use crate::{
    data_source::{AssessmentHistoryBuildMode, ServingRuntimePurpose},
    history_builder::{select_assessment_history, select_backtest_timeline, HistoryQueryWindow},
    AppState,
};

#[derive(Debug, Default, Deserialize)]
pub struct HistoryQuery {
    from: Option<String>,
    to: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
pub struct ReloadQuery {
    history_mode: Option<String>,
    history_limit: Option<usize>,
    runtime_purpose: Option<String>,
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
    let method = &data.assessment.method;
    let history_note = if matches!(
        data.assessment.runtime.data_mode,
        fc_domain::DataMode::Sqlite
    ) {
        "assessment/history 在 SQLite 模式下会优先复用同口径的 historical replay points；若 active release 带有 bundle-backed 概率模型且未命中可复用 replay cache，会直接基于原始观测全量重建并写回 replay store，不再静默退回旧 prediction snapshots；只有 heuristic / 兼容路径仍会复用已落库 prediction snapshots。"
    } else {
        "assessment/history 当前仍由运行时即时构造。"
    };
    let note = format!(
        "assessment 概率、风险强度、episode-native 动作层和 posture 为不同层的输出；5d / 20d / 60d 回答风险窗口距离，prepare / hedge / defend 回答动作优先级，不是把 60d / 20d / 5d 直接改名。当前 probability_mode={}、release_status={}{}。{} 页面应先检查 data_mode、关键指标日期、stale warning 和 action playbook，再解释当前数值。",
        method.probability_mode,
        method.release_status,
        method
            .release_id
            .as_ref()
            .map(|release_id| format!("、release_id={release_id}"))
            .unwrap_or_default(),
        history_note
    );
    Json(json!({
        "method": data.assessment.method,
        "note": note,
        "protected_stress_window_catalog": data.protected_stress_window_catalog,
        "runtime_thresholds": data.runtime_thresholds,
    }))
}

pub async fn system_reload(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ReloadQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let history_build_mode = match query.history_mode.as_deref() {
        Some("strict_rebuild") => AssessmentHistoryBuildMode::StrictRebuild,
        _ => AssessmentHistoryBuildMode::Default,
    };
    let runtime_purpose = match query.runtime_purpose.as_deref() {
        Some("review") => ServingRuntimePurpose::Review,
        Some("production") | None => ServingRuntimePurpose::Production,
        Some(_) => return Err(StatusCode::BAD_REQUEST),
    };
    let history_limit = query.history_limit.unwrap_or(state.max_history_points());
    if history_limit == 0 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let data = state
        .reload_with_runtime_options(history_build_mode, history_limit, runtime_purpose)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({
        "status": "ok",
        "data_mode": data.data_mode,
        "as_of_date": data.assessment.as_of_date,
        "generated_at": data.assessment.runtime.generated_at,
        "history_mode": match history_build_mode {
            AssessmentHistoryBuildMode::Default => "default",
            AssessmentHistoryBuildMode::StrictRebuild => "strict_rebuild",
        },
        "history_limit": history_limit,
        "runtime_purpose": runtime_purpose.as_label(),
    })))
}

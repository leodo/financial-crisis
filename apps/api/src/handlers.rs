use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::json;

mod research_audit;

pub(crate) use research_audit::research_audit;

use crate::{
    data_source::{AssessmentHistoryBuildMode, ServingRuntimePurpose},
    history_builder::{select_assessment_history, select_backtest_timeline, HistoryQueryWindow},
    history_replay::{
        HISTORY_SOURCE_RAW_OBSERVATION_REBUILD, HISTORY_SOURCE_RAW_OBSERVATION_REPLAY,
        HISTORY_SOURCE_RAW_PIT_FEATURE_REPLAY, HISTORY_SOURCE_RAW_PIT_FEATURE_REUSE,
        HISTORY_SOURCE_TRANSITIONAL_SNAPSHOT_BRIDGE,
    },
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

#[derive(Debug, Clone, Serialize)]
pub(crate) struct HistoryProvenanceSourceSummary {
    pub(crate) source_id: String,
    pub(crate) count: usize,
    pub(crate) latest_as_of_date: Option<NaiveDate>,
    pub(crate) note: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct HistoryProvenanceSummary {
    pub(crate) evidence_tier: String,
    pub(crate) dominant_source: String,
    pub(crate) total_points: usize,
    pub(crate) feature_backed_points: usize,
    pub(crate) reused_feature_snapshot_points: usize,
    pub(crate) raw_observation_points: usize,
    pub(crate) snapshot_bridge_points: usize,
    pub(crate) runtime_only_points: usize,
    pub(crate) latest_feature_backed_date: Option<NaiveDate>,
    pub(crate) latest_reused_feature_snapshot_date: Option<NaiveDate>,
    pub(crate) latest_raw_observation_date: Option<NaiveDate>,
    pub(crate) latest_snapshot_bridge_date: Option<NaiveDate>,
    pub(crate) latest_replay_run_id: Option<String>,
    pub(crate) note: String,
    pub(crate) sources: Vec<HistoryProvenanceSourceSummary>,
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

pub(crate) fn summarize_history_provenance(
    history: &[fc_domain::AssessmentHistoryPoint],
) -> HistoryProvenanceSummary {
    let total_points = history.len();
    let feature_backed_points = history
        .iter()
        .filter(|point| {
            point.history_source.as_deref() == Some(HISTORY_SOURCE_RAW_PIT_FEATURE_REPLAY)
        })
        .count();
    let reused_feature_snapshot_points = history
        .iter()
        .filter(|point| {
            point.history_source.as_deref() == Some(HISTORY_SOURCE_RAW_PIT_FEATURE_REUSE)
        })
        .count();
    let raw_observation_points = history
        .iter()
        .filter(|point| {
            matches!(
                point.history_source.as_deref(),
                Some(
                    HISTORY_SOURCE_RAW_OBSERVATION_REPLAY | HISTORY_SOURCE_RAW_OBSERVATION_REBUILD
                )
            )
        })
        .count();
    let snapshot_bridge_points = history
        .iter()
        .filter(|point| {
            point.history_source.as_deref() == Some(HISTORY_SOURCE_TRANSITIONAL_SNAPSHOT_BRIDGE)
        })
        .count();
    let runtime_only_points = total_points.saturating_sub(
        feature_backed_points
            + reused_feature_snapshot_points
            + raw_observation_points
            + snapshot_bridge_points,
    );

    let latest_feature_backed_date = history
        .iter()
        .filter(|point| {
            point.history_source.as_deref() == Some(HISTORY_SOURCE_RAW_PIT_FEATURE_REPLAY)
        })
        .map(|point| point.as_of_date)
        .max();
    let latest_reused_feature_snapshot_date = history
        .iter()
        .filter(|point| {
            point.history_source.as_deref() == Some(HISTORY_SOURCE_RAW_PIT_FEATURE_REUSE)
        })
        .map(|point| point.as_of_date)
        .max();
    let latest_raw_observation_date = history
        .iter()
        .filter(|point| {
            matches!(
                point.history_source.as_deref(),
                Some(
                    HISTORY_SOURCE_RAW_OBSERVATION_REPLAY | HISTORY_SOURCE_RAW_OBSERVATION_REBUILD
                )
            )
        })
        .map(|point| point.as_of_date)
        .max();
    let latest_snapshot_bridge_date = history
        .iter()
        .filter(|point| {
            point.history_source.as_deref() == Some(HISTORY_SOURCE_TRANSITIONAL_SNAPSHOT_BRIDGE)
        })
        .map(|point| point.as_of_date)
        .max();
    let latest_replay_run_id = history
        .iter()
        .rev()
        .find_map(|point| point.replay_run_id.clone());

    let sources = [
        (
            HISTORY_SOURCE_RAW_PIT_FEATURE_REPLAY,
            "这类点已经绑定到已落库的 PIT feature snapshot，可作为 formal history 审计的正式证据层。",
        ),
        (
            HISTORY_SOURCE_RAW_PIT_FEATURE_REUSE,
            "这类点虽然绑定到了已落库的 PIT feature snapshot，但复用了更早日期的 snapshot，不是当天精确 PIT，仍属于 formal history 的过渡口径。",
        ),
        (
            HISTORY_SOURCE_RAW_OBSERVATION_REPLAY,
            "这类点来自 historical replay store，但还没有对上已落库的 PIT feature snapshot，仍属于 raw observation 过渡口径。",
        ),
        (
            HISTORY_SOURCE_RAW_OBSERVATION_REBUILD,
            "这类点是运行时按原始观测即时重建的结果，说明当前默认历史还没有完全收口到 persisted replay / PIT snapshot。",
        ),
        (
            HISTORY_SOURCE_TRANSITIONAL_SNAPSHOT_BRIDGE,
            "这类点仍复用了旧 prediction snapshot bridge，只适合辅助观察，不应直接当成正式 Go/No-Go 历史证据。",
        ),
    ]
    .into_iter()
    .map(|(source_id, note)| HistoryProvenanceSourceSummary {
        source_id: source_id.to_string(),
        count: history
            .iter()
            .filter(|point| point.history_source.as_deref() == Some(source_id))
            .count(),
        latest_as_of_date: history
            .iter()
            .filter(|point| point.history_source.as_deref() == Some(source_id))
            .map(|point| point.as_of_date)
            .max(),
        note: note.to_string(),
    })
    .collect::<Vec<_>>();

    let dominant_source = sources
        .iter()
        .max_by_key(|source| source.count)
        .filter(|source| source.count > 0)
        .map(|source| source.source_id.clone())
        .unwrap_or_else(|| "runtime_only".to_string());

    let evidence_tier = if snapshot_bridge_points > 0 {
        "snapshot_bridge_transitional"
    } else if raw_observation_points > 0 {
        "raw_observation_transitional"
    } else if reused_feature_snapshot_points > 0 {
        "pit_feature_reuse_transitional"
    } else if feature_backed_points > 0 {
        "pit_feature_backed"
    } else {
        "runtime_only"
    };

    let note = if total_points == 0 {
        "当前默认历史窗口里还没有可用评估点，方法页无法判断 replay provenance。".to_string()
    } else if snapshot_bridge_points > 0 {
        format!(
            "默认历史轨迹里仍有 {snapshot_bridge_points}/{total_points} 个点来自旧 prediction snapshot bridge，只适合辅助观察，不应直接当成 formal history 审计证据。"
        )
    } else if raw_observation_points > 0 {
        format!(
            "默认历史轨迹已经避开旧 snapshot bridge，但仍有 {raw_observation_points}/{total_points} 个点只是 raw observation 过渡口径，说明 replay 还没有完全绑定到 persisted PIT feature snapshot。"
        )
    } else if reused_feature_snapshot_points > 0 {
        format!(
            "默认历史轨迹里已有 {feature_backed_points}/{total_points} 个点绑定到当天 PIT feature snapshot，但仍有 {reused_feature_snapshot_points}/{total_points} 个点沿用了更早日期的 PIT snapshot；它已经明显强于 raw observation / bridge，但还不是完全精确的 raw PIT formal history。"
        )
    } else if feature_backed_points > 0 {
        format!(
            "默认历史轨迹当前 {feature_backed_points}/{total_points} 个点都绑定到已落库 PIT feature snapshot，可作为 formal history 审计的正式证据层。"
        )
    } else {
        "当前默认历史轨迹没有 bridge 或 replay provenance 标记，说明它仍主要是运行时即时构造结果。"
            .to_string()
    };

    HistoryProvenanceSummary {
        evidence_tier: evidence_tier.to_string(),
        dominant_source,
        total_points,
        feature_backed_points,
        reused_feature_snapshot_points,
        raw_observation_points,
        snapshot_bridge_points,
        runtime_only_points,
        latest_feature_backed_date,
        latest_reused_feature_snapshot_date,
        latest_raw_observation_date,
        latest_snapshot_bridge_date,
        latest_replay_run_id,
        note,
        sources,
    }
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
    let history_provenance = summarize_history_provenance(&data.assessment_history);
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
        "history_provenance": history_provenance,
        "protected_stress_window_catalog": data.protected_stress_window_catalog,
        "scenario_data_coverage_catalog": data.scenario_data_coverage_catalog,
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

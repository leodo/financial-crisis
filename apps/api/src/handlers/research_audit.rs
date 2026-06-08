use std::{fs, path::Path as FsPath, sync::Arc};

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, FixedOffset, NaiveDate};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{data_source::AppDataSource, handlers::HistoryProvenanceSummary, AppState};
use fc_storage::SqliteStore;

#[derive(Debug, Default, Deserialize)]
pub(crate) struct ResearchAuditQuery {
    market_scope: Option<String>,
    release_id: Option<String>,
    from: Option<String>,
    to: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
struct ResearchAuditResponse {
    supported: bool,
    storage_mode: String,
    market_scope: String,
    active_release_id: Option<String>,
    runtime_probability_mode: String,
    runtime_release_status: String,
    history_provenance: HistoryProvenanceSummary,
    latest_snapshot_date: Option<NaiveDate>,
    latest_replay_run_id: Option<String>,
    latest_release_review: Option<ReleaseReviewArtifactSummary>,
    note: String,
    releases: Vec<fc_domain::ModelReleaseRecord>,
    replay_runs: Vec<fc_domain::HistoricalReplayRunRecord>,
    snapshots: Vec<fc_domain::PredictionSnapshotRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReleaseReviewArtifactReleaseRef {
    release_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReleaseReviewArtifactAttributionSummary {
    workstream: String,
    attribution: String,
    scenario_count: u32,
    protected_count: u32,
    baseline_count: u32,
    candidate_count: u32,
    baseline_scenarios: Vec<String>,
    candidate_scenarios: Vec<String>,
    explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReleaseReviewArtifactActionSummary {
    workstream: String,
    attribution: String,
    action_type: String,
    scenario_count: u32,
    protected_count: u32,
    recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReleaseReviewArtifactWire {
    reviewed_at: String,
    market_scope: String,
    history_mode: String,
    original_active_release_id: String,
    restored_release_id: String,
    baseline_release: ReleaseReviewArtifactReleaseRef,
    candidate_release: ReleaseReviewArtifactReleaseRef,
    overall_guard_passed: bool,
    recommendation: String,
    #[serde(default)]
    historical_audit_attribution: Vec<ReleaseReviewArtifactAttributionSummary>,
    #[serde(default)]
    historical_audit_actions: Vec<ReleaseReviewArtifactActionSummary>,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseReviewArtifactSummary {
    reviewed_at: String,
    market_scope: String,
    history_mode: String,
    original_active_release_id: String,
    restored_release_id: String,
    baseline_release_id: String,
    candidate_release_id: String,
    overall_guard_passed: bool,
    recommendation: String,
    historical_audit_attribution: Vec<ReleaseReviewArtifactAttributionSummary>,
    historical_audit_actions: Vec<ReleaseReviewArtifactActionSummary>,
}

fn load_latest_release_review_summary(
    market_scope: &str,
    active_release_id: Option<&str>,
) -> Option<ReleaseReviewArtifactSummary> {
    let mut candidates = Vec::<(
        bool,
        Option<DateTime<FixedOffset>>,
        ReleaseReviewArtifactSummary,
    )>::new();
    for directory in [
        "artifacts/research/release-review",
        "reports/release-review",
    ] {
        let path = FsPath::new(directory);
        let Ok(entries) = fs::read_dir(path) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                continue;
            }
            let Ok(body) = fs::read_to_string(&path) else {
                continue;
            };
            let Ok(wire) = serde_json::from_str::<ReleaseReviewArtifactWire>(&body) else {
                continue;
            };
            if wire.market_scope != market_scope {
                continue;
            }
            let active_match = active_release_id.is_some_and(|release_id| {
                wire.original_active_release_id == release_id
                    || wire.restored_release_id == release_id
                    || wire.baseline_release.release_id == release_id
                    || wire.candidate_release.release_id == release_id
            });
            candidates.push((
                active_match,
                DateTime::parse_from_rfc3339(&wire.reviewed_at).ok(),
                ReleaseReviewArtifactSummary {
                    reviewed_at: wire.reviewed_at,
                    market_scope: wire.market_scope,
                    history_mode: wire.history_mode,
                    original_active_release_id: wire.original_active_release_id,
                    restored_release_id: wire.restored_release_id,
                    baseline_release_id: wire.baseline_release.release_id,
                    candidate_release_id: wire.candidate_release.release_id,
                    overall_guard_passed: wire.overall_guard_passed,
                    recommendation: wire.recommendation,
                    historical_audit_attribution: wire.historical_audit_attribution,
                    historical_audit_actions: wire.historical_audit_actions,
                },
            ));
        }
    }

    candidates.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| right.1.cmp(&left.1))
            .then_with(|| right.2.reviewed_at.cmp(&left.2.reviewed_at))
    });
    candidates.into_iter().next().map(|(_, _, summary)| summary)
}

pub(crate) async fn research_audit(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ResearchAuditQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let data = state.data().await;
    let market_scope = query
        .market_scope
        .unwrap_or_else(|| "financial_system".to_string());
    let from = super::parse_date(query.from)?;
    let to = super::parse_date(query.to)?;
    let limit = query.limit.unwrap_or(60);

    let response = match state.source() {
        AppDataSource::Sqlite { path } => {
            let store = SqliteStore::connect(path)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            store
                .migrate()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let releases = store
                .list_model_releases(Some(&market_scope))
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let snapshots = store
                .list_prediction_snapshots(
                    Some(&market_scope),
                    query.release_id.as_deref(),
                    from,
                    to,
                    Some(limit),
                )
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let replay_runs = store
                .list_historical_replay_runs(
                    Some(&market_scope),
                    query.release_id.as_deref(),
                    from,
                    to,
                    Some(limit),
                )
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let latest_snapshot_date = snapshots.iter().map(|snapshot| snapshot.as_of_date).max();
            let latest_replay_run_id = replay_runs.first().map(|run| run.replay_run_id.clone());
            let latest_release_review = load_latest_release_review_summary(
                &market_scope,
                data.assessment.method.release_id.as_deref(),
            );
            ResearchAuditResponse {
                supported: true,
                storage_mode: "sqlite".to_string(),
                market_scope,
                active_release_id: data.assessment.method.release_id.clone(),
                runtime_probability_mode: data.assessment.method.probability_mode.clone(),
                runtime_release_status: data.assessment.method.release_status.clone(),
                history_provenance: super::summarize_history_provenance(&data.assessment_history),
                latest_snapshot_date,
                latest_replay_run_id,
                latest_release_review,
                note: "当前页面展示的是 release registry、historical replay run / point、prediction snapshot，以及最近一次 release review 的落库审计。若 runtime probability mode 与 release manifest 不一致，说明线上已自动降级回 heuristic。".to_string(),
                releases,
                replay_runs,
                snapshots,
            }
        }
        AppDataSource::Demo => ResearchAuditResponse {
            supported: false,
            storage_mode: "demo".to_string(),
            market_scope,
            active_release_id: data.assessment.method.release_id.clone(),
            runtime_probability_mode: data.assessment.method.probability_mode.clone(),
            runtime_release_status: data.assessment.method.release_status.clone(),
            history_provenance: super::summarize_history_provenance(&data.assessment_history),
            latest_snapshot_date: None,
            latest_replay_run_id: None,
            latest_release_review: None,
            note: "当前运行在 demo 模式，release registry、historical replay、prediction snapshot 审计不可用。切到 SQLite 后该页面会显示真实审计数据。".to_string(),
            releases: Vec::new(),
            replay_runs: Vec::new(),
            snapshots: Vec::new(),
        },
        AppDataSource::Postgres { .. } => ResearchAuditResponse {
            supported: false,
            storage_mode: "postgres".to_string(),
            market_scope,
            active_release_id: data.assessment.method.release_id.clone(),
            runtime_probability_mode: data.assessment.method.probability_mode.clone(),
            runtime_release_status: data.assessment.method.release_status.clone(),
            history_provenance: super::summarize_history_provenance(&data.assessment_history),
            latest_snapshot_date: None,
            latest_replay_run_id: None,
            latest_release_review: None,
            note: "当前 Postgres 路径尚未接入本地 release registry、historical replay 与 prediction snapshot 审计，建议先通过 SQLite 研究链路完成模型训练、发布与复盘。".to_string(),
            releases: Vec::new(),
            replay_runs: Vec::new(),
            snapshots: Vec::new(),
        },
    };

    Ok(Json(json!(response)))
}

#[cfg(test)]
mod tests {
    use super::ReleaseReviewArtifactWire;

    #[test]
    fn release_review_wire_allows_missing_historical_audit_fields() {
        let body = r#"
        {
          "reviewed_at": "2026-06-04T13:21:42.242886500+00:00",
          "market_scope": "financial_system",
          "history_mode": "strict_rebuild",
          "original_active_release_id": "baseline_release",
          "restored_release_id": "baseline_release",
          "baseline_release": { "release_id": "baseline_release" },
          "candidate_release": { "release_id": "candidate_release" },
          "overall_guard_passed": true,
          "recommendation": "candidate passed"
        }
        "#;

        let wire: ReleaseReviewArtifactWire =
            serde_json::from_str(body).expect("wire should deserialize");
        assert!(wire.historical_audit_attribution.is_empty());
        assert!(wire.historical_audit_actions.is_empty());
    }
}

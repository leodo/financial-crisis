use std::{fs, path::Path as FsPath, sync::Arc};

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, FixedOffset, NaiveDate};
use serde::{Deserialize, Serialize};
use serde_json::json;

mod dataset_summary;
mod workstream_audit;

use self::dataset_summary::{load_latest_dataset_summaries, DatasetSummaryArtifactSummary};
use self::workstream_audit::{
    load_latest_workstream_audit_summary, WorkstreamAuditArtifactSummary,
};
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
    latest_scenario_pack_audit: Option<ScenarioPackAuditArtifactSummary>,
    latest_workstream_audit: Option<WorkstreamAuditArtifactSummary>,
    latest_rate_shock_audit: Option<RateShockAuditArtifactSummary>,
    latest_dataset_summaries: Vec<DatasetSummaryArtifactSummary>,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ReleaseReviewScenarioCoverageCatalogSummary {
    catalog_id: String,
    scenario_catalog_id: String,
    market_scope: String,
    source: String,
    warning: Option<String>,
    backtest_scenario_count: usize,
    covered_backtest_scenario_count: usize,
    focus_scenario_count: usize,
    covered_focus_scenario_count: usize,
    main_training_eligible_count: usize,
    extension_training_eligible_count: usize,
    protected_stress_eligible_count: usize,
    historical_analog_eligible_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReleaseReviewScenarioCoverageSummary {
    scenario_id: String,
    scenario_name: String,
    scenario_family: String,
    training_role: String,
    protected_window: bool,
    in_backtest_comparison: bool,
    in_focus_review: bool,
    recommended_role: String,
    coverage_grade: String,
    point_in_time_mode: String,
    current_status: String,
    #[serde(default)]
    blocking_gaps: Vec<String>,
    #[serde(default)]
    free_sources: Vec<String>,
    usable_for_main_training: bool,
    usable_for_extension_training: bool,
    usable_for_protected_stress: bool,
    usable_for_historical_analog: bool,
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
    #[serde(default)]
    scenario_coverage_catalog: ReleaseReviewScenarioCoverageCatalogSummary,
    #[serde(default)]
    scenario_coverages: Vec<ReleaseReviewScenarioCoverageSummary>,
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
    scenario_coverage_catalog: ReleaseReviewScenarioCoverageCatalogSummary,
    scenario_coverages: Vec<ReleaseReviewScenarioCoverageSummary>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ScenarioPackAuditBlockerCountSummary {
    key: String,
    count: usize,
    #[serde(default)]
    scenarios: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ScenarioPackAuditScenarioSummary {
    scenario_id: String,
    scenario_label: String,
    family: String,
    training_role: String,
    recommended_role: String,
    coverage_grade: String,
    point_in_time_mode: String,
    current_status: String,
    protected_window: bool,
    #[serde(default)]
    free_sources: Vec<String>,
    #[serde(default)]
    blocking_gaps: Vec<String>,
    outcome: Option<String>,
    signal_source: Option<String>,
    baseline_lead_time_days: Option<i64>,
    candidate_lead_time_days: Option<i64>,
    baseline_actionable_lead_time_days: Option<i64>,
    candidate_actionable_lead_time_days: Option<i64>,
    primary_workstream: Option<String>,
    suggested_review: Option<String>,
    candidate_primary_failure_mode: Option<String>,
    compare_status: String,
    compare_dataset_key: Option<String>,
    #[serde(default)]
    attempted_datasets: Vec<String>,
    row_count: usize,
    positive_window_retention_20d: Option<f64>,
    overall_avg_delta_p_20d: Option<f64>,
    blocker_class: String,
    takeaway: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ScenarioPackAuditArtifactWire {
    generated_at: String,
    baseline_release_id: String,
    candidate_release_id: String,
    history_mode: String,
    market_scope: String,
    compare_ok_count: usize,
    compare_missing_count: usize,
    #[serde(default)]
    blocker_counts: Vec<ScenarioPackAuditBlockerCountSummary>,
    #[serde(default)]
    scenario_summaries: Vec<ScenarioPackAuditScenarioSummary>,
}

#[derive(Debug, Clone, Serialize)]
struct ScenarioPackAuditArtifactSummary {
    generated_at: String,
    source: String,
    baseline_release_id: String,
    candidate_release_id: String,
    history_mode: String,
    market_scope: String,
    compare_ok_count: usize,
    compare_missing_count: usize,
    blocker_counts: Vec<ScenarioPackAuditBlockerCountSummary>,
    scenario_summaries: Vec<ScenarioPackAuditScenarioSummary>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct RateShockAuditThresholdSummary {
    baseline_20d: Option<f64>,
    candidate_20d: Option<f64>,
    baseline_60d: Option<f64>,
    candidate_60d: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct RateShockAuditWindowSummary {
    row_count: usize,
    avg_delta_p_20d: Option<f64>,
    avg_abs_delta_p_20d: Option<f64>,
    avg_delta_p_60d: Option<f64>,
    avg_abs_delta_p_60d: Option<f64>,
    baseline_hit_rate_20d: Option<f64>,
    candidate_hit_rate_20d: Option<f64>,
    baseline_hit_rate_60d: Option<f64>,
    candidate_hit_rate_60d: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct RateShockAuditCompareSummary {
    baseline_hit_count_20d: usize,
    candidate_hit_count_20d: usize,
    baseline_hit_count_60d: usize,
    candidate_hit_count_60d: usize,
    baseline_max_p_20d: Option<f64>,
    baseline_max_p_20d_date: Option<String>,
    candidate_max_p_20d: Option<f64>,
    candidate_max_p_20d_date: Option<String>,
    baseline_max_p_60d: Option<f64>,
    baseline_max_p_60d_date: Option<String>,
    candidate_max_p_60d: Option<f64>,
    candidate_max_p_60d_date: Option<String>,
    #[serde(default)]
    overall_window: RateShockAuditWindowSummary,
    #[serde(default)]
    hedge_window: RateShockAuditWindowSummary,
    #[serde(default)]
    positive_window_20d: RateShockAuditWindowSummary,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct RateShockAuditSplitSummary {
    split_name: String,
    row_count: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct RateShockAuditHitSummary {
    hit_count: usize,
    segment_count: usize,
    max_streak: usize,
    first_hit_date: Option<String>,
    last_hit_date: Option<String>,
    max_streak_start: Option<String>,
    max_streak_end: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct RateShockAuditGroupSummary {
    label: String,
    row_count: usize,
    baseline_avg_p_20d: Option<f64>,
    candidate_avg_p_20d: Option<f64>,
    avg_delta_p_20d: Option<f64>,
    baseline_avg_gap_to_threshold_20d: Option<f64>,
    candidate_avg_gap_to_threshold_20d: Option<f64>,
    baseline_avg_p_60d: Option<f64>,
    candidate_avg_p_60d: Option<f64>,
    avg_delta_p_60d: Option<f64>,
    baseline_avg_gap_to_threshold_60d: Option<f64>,
    candidate_avg_gap_to_threshold_60d: Option<f64>,
    baseline_hit_rate_20d: Option<f64>,
    candidate_hit_rate_20d: Option<f64>,
    baseline_hit_rate_60d: Option<f64>,
    candidate_hit_rate_60d: Option<f64>,
    #[serde(default)]
    baseline_hit_20d: RateShockAuditHitSummary,
    #[serde(default)]
    candidate_hit_20d: RateShockAuditHitSummary,
    #[serde(default)]
    baseline_hit_60d: RateShockAuditHitSummary,
    #[serde(default)]
    candidate_hit_60d: RateShockAuditHitSummary,
    baseline_near_threshold_20d_within_5pp_count: usize,
    candidate_near_threshold_20d_within_5pp_count: usize,
    baseline_near_threshold_60d_within_5pp_count: usize,
    candidate_near_threshold_60d_within_5pp_count: usize,
    baseline_max_p_20d: Option<f64>,
    baseline_max_p_20d_date: Option<String>,
    candidate_max_p_20d: Option<f64>,
    candidate_max_p_20d_date: Option<String>,
    baseline_max_p_60d: Option<f64>,
    baseline_max_p_60d_date: Option<String>,
    candidate_max_p_60d: Option<f64>,
    candidate_max_p_60d_date: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct RateShockAuditContinuityFocus {
    #[serde(default)]
    prepare_primary: RateShockAuditGroupSummary,
    #[serde(default)]
    hedge_primary: RateShockAuditGroupSummary,
    #[serde(default)]
    primary_phase: RateShockAuditGroupSummary,
    #[serde(default)]
    late_validation: RateShockAuditGroupSummary,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct RateShockAuditArtifactWire {
    generated_at: String,
    compare_path: String,
    slice_path: String,
    baseline_release_id: String,
    candidate_release_id: String,
    dataset_key: String,
    scenario_id: String,
    from_date: String,
    to_date: String,
    #[serde(default)]
    thresholds: RateShockAuditThresholdSummary,
    #[serde(default)]
    compare_summary: RateShockAuditCompareSummary,
    #[serde(default)]
    split_counts: Vec<RateShockAuditSplitSummary>,
    #[serde(default)]
    phase_summaries: Vec<RateShockAuditGroupSummary>,
    #[serde(default)]
    action_level_summaries: Vec<RateShockAuditGroupSummary>,
    #[serde(default)]
    continuity_focus: RateShockAuditContinuityFocus,
}

#[derive(Debug, Clone, Serialize)]
struct RateShockAuditArtifactSummary {
    generated_at: String,
    source: String,
    compare_path: String,
    slice_path: String,
    baseline_release_id: String,
    candidate_release_id: String,
    dataset_key: String,
    scenario_id: String,
    from_date: String,
    to_date: String,
    thresholds: RateShockAuditThresholdSummary,
    compare_summary: RateShockAuditCompareSummary,
    split_counts: Vec<RateShockAuditSplitSummary>,
    phase_summaries: Vec<RateShockAuditGroupSummary>,
    action_level_summaries: Vec<RateShockAuditGroupSummary>,
    continuity_focus: RateShockAuditContinuityFocus,
}

fn decode_json_artifact(bytes: Vec<u8>) -> Option<String> {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8(bytes[3..].to_vec()).ok();
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        let utf16: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        return String::from_utf16(&utf16).ok();
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let utf16: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
            .collect();
        return String::from_utf16(&utf16).ok();
    }
    String::from_utf8(bytes).ok()
}

fn read_json_artifact(path: &FsPath) -> Option<String> {
    decode_json_artifact(fs::read(path).ok()?)
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
            let Some(body) = read_json_artifact(&path) else {
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
                    scenario_coverage_catalog: wire.scenario_coverage_catalog,
                    scenario_coverages: wire.scenario_coverages,
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

fn load_latest_scenario_pack_audit_summary(
    market_scope: &str,
    release_review: Option<&ReleaseReviewArtifactSummary>,
) -> Option<ScenarioPackAuditArtifactSummary> {
    let release_review = release_review?;
    let mut candidates = Vec::<(
        Option<DateTime<FixedOffset>>,
        ScenarioPackAuditArtifactSummary,
    )>::new();
    for directory in ["artifacts/research/spa"] {
        let path = FsPath::new(directory);
        let Ok(entries) = fs::read_dir(path) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                continue;
            }
            let Some(body) = read_json_artifact(&path) else {
                continue;
            };
            let wire = match serde_json::from_str::<ScenarioPackAuditArtifactWire>(&body) {
                Ok(wire) => wire,
                Err(error) => {
                    tracing::warn!(
                        path = %path.to_string_lossy(),
                        %error,
                        "failed to parse scenario-pack audit artifact"
                    );
                    continue;
                }
            };
            if wire.market_scope != market_scope
                || wire.baseline_release_id != release_review.baseline_release_id
                || wire.candidate_release_id != release_review.candidate_release_id
                || wire.history_mode != release_review.history_mode
            {
                continue;
            }
            candidates.push((
                DateTime::parse_from_rfc3339(&wire.generated_at).ok(),
                ScenarioPackAuditArtifactSummary {
                    generated_at: wire.generated_at,
                    source: path.to_string_lossy().into_owned(),
                    baseline_release_id: wire.baseline_release_id,
                    candidate_release_id: wire.candidate_release_id,
                    history_mode: wire.history_mode,
                    market_scope: wire.market_scope,
                    compare_ok_count: wire.compare_ok_count,
                    compare_missing_count: wire.compare_missing_count,
                    blocker_counts: wire.blocker_counts,
                    scenario_summaries: wire.scenario_summaries,
                },
            ));
        }
    }

    candidates.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| right.1.generated_at.cmp(&left.1.generated_at))
    });
    candidates.into_iter().next().map(|(_, summary)| summary)
}

fn load_latest_rate_shock_audit_summary(
    release_review: Option<&ReleaseReviewArtifactSummary>,
) -> Option<RateShockAuditArtifactSummary> {
    let release_review = release_review?;
    let mut candidates =
        Vec::<(Option<DateTime<FixedOffset>>, RateShockAuditArtifactSummary)>::new();
    for directory in ["artifacts/research/rate-shock-audit"] {
        let path = FsPath::new(directory);
        let Ok(entries) = fs::read_dir(path) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                continue;
            }
            let Some(body) = read_json_artifact(&path) else {
                continue;
            };
            let wire = match serde_json::from_str::<RateShockAuditArtifactWire>(&body) {
                Ok(wire) => wire,
                Err(error) => {
                    tracing::warn!(
                        path = %path.to_string_lossy(),
                        %error,
                        "failed to parse rate-shock audit artifact"
                    );
                    continue;
                }
            };
            if wire.baseline_release_id != release_review.baseline_release_id
                || wire.candidate_release_id != release_review.candidate_release_id
                || wire.scenario_id != "us_rate_shock_2022"
            {
                continue;
            }
            candidates.push((
                DateTime::parse_from_rfc3339(&wire.generated_at).ok(),
                RateShockAuditArtifactSummary {
                    generated_at: wire.generated_at,
                    source: path.to_string_lossy().into_owned(),
                    compare_path: wire.compare_path,
                    slice_path: wire.slice_path,
                    baseline_release_id: wire.baseline_release_id,
                    candidate_release_id: wire.candidate_release_id,
                    dataset_key: wire.dataset_key,
                    scenario_id: wire.scenario_id,
                    from_date: wire.from_date,
                    to_date: wire.to_date,
                    thresholds: wire.thresholds,
                    compare_summary: wire.compare_summary,
                    split_counts: wire.split_counts,
                    phase_summaries: wire.phase_summaries,
                    action_level_summaries: wire.action_level_summaries,
                    continuity_focus: wire.continuity_focus,
                },
            ));
        }
    }

    candidates.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| right.1.generated_at.cmp(&left.1.generated_at))
    });
    candidates.into_iter().next().map(|(_, summary)| summary)
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
            let latest_scenario_pack_audit =
                load_latest_scenario_pack_audit_summary(&market_scope, latest_release_review.as_ref());
            let latest_workstream_audit = load_latest_workstream_audit_summary(
                &market_scope,
                data.assessment.method.release_id.as_deref(),
                latest_release_review.as_ref(),
            );
            let latest_rate_shock_audit =
                load_latest_rate_shock_audit_summary(latest_release_review.as_ref());
            let latest_dataset_summaries = load_latest_dataset_summaries(&market_scope);
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
                latest_scenario_pack_audit,
                latest_workstream_audit,
                latest_rate_shock_audit,
                latest_dataset_summaries,
                note: "当前页面展示的是 release registry、historical replay run / point、prediction snapshot、最近一次 release review、对应的历史场景包审计、formal dataset 摘要、residual workstream 审计，以及 2022 利率冲击专项 continuity 审计。若 runtime probability mode 与 release manifest 不一致，说明线上已自动降级回 heuristic。".to_string(),
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
            latest_scenario_pack_audit: None,
            latest_workstream_audit: None,
            latest_rate_shock_audit: None,
            latest_dataset_summaries: Vec::new(),
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
            latest_scenario_pack_audit: None,
            latest_workstream_audit: None,
            latest_rate_shock_audit: None,
            latest_dataset_summaries: Vec::new(),
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
    use super::{
        decode_json_artifact, RateShockAuditArtifactWire, ReleaseReviewArtifactWire,
        ScenarioPackAuditArtifactWire,
    };

    #[test]
    fn release_review_wire_allows_missing_optional_audit_fields() {
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
        assert!(wire.scenario_coverages.is_empty());
        assert_eq!(wire.scenario_coverage_catalog.catalog_id, "");
        assert_eq!(
            wire.scenario_coverage_catalog
                .covered_backtest_scenario_count,
            0
        );
    }

    #[test]
    fn scenario_pack_wire_allows_missing_optional_arrays() {
        let body = r#"
        {
          "generated_at": "2026-06-09T00:00:00+00:00",
          "baseline_release_id": "baseline_release",
          "candidate_release_id": "candidate_release",
          "history_mode": "default",
          "market_scope": "financial_system",
          "compare_ok_count": 0,
          "compare_missing_count": 0
        }
        "#;

        let wire: ScenarioPackAuditArtifactWire =
            serde_json::from_str(body).expect("wire should deserialize");
        assert!(wire.blocker_counts.is_empty());
        assert!(wire.scenario_summaries.is_empty());
    }

    #[test]
    fn rate_shock_wire_allows_missing_optional_arrays() {
        let body = r#"
        {
          "generated_at": "2026-06-09T00:00:00+00:00",
          "compare_path": "compare.json",
          "slice_path": "slice.json",
          "baseline_release_id": "baseline_release",
          "candidate_release_id": "candidate_release",
          "dataset_key": "formal_v1_main_1990_daily:test",
          "scenario_id": "us_rate_shock_2022",
          "from_date": "2021-10-05",
          "to_date": "2022-10-31"
        }
        "#;

        let wire: RateShockAuditArtifactWire =
            serde_json::from_str(body).expect("wire should deserialize");
        assert!(wire.phase_summaries.is_empty());
        assert!(wire.action_level_summaries.is_empty());
        assert!(wire.split_counts.is_empty());
        assert_eq!(wire.compare_summary.overall_window.row_count, 0);
    }

    #[test]
    fn decode_json_artifact_accepts_utf8_bom() {
        let bytes = b"\xEF\xBB\xBF{\"ok\":true}".to_vec();
        let decoded = decode_json_artifact(bytes).expect("utf8 bom should decode");
        assert_eq!(decoded, "{\"ok\":true}");
    }

    #[test]
    fn decode_json_artifact_accepts_utf16le_bom() {
        let bytes = vec![
            0xFF, 0xFE, 0x7B, 0x00, 0x22, 0x00, 0x6F, 0x00, 0x6B, 0x00, 0x22, 0x00, 0x3A, 0x00,
            0x74, 0x00, 0x72, 0x00, 0x75, 0x00, 0x65, 0x00, 0x7D, 0x00,
        ];
        let decoded = decode_json_artifact(bytes).expect("utf16le bom should decode");
        assert_eq!(decoded, "{\"ok\":true}");
    }
}

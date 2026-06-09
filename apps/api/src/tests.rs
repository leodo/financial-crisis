use std::sync::Arc;

use axum::{body::Body, http::Request, Router};
use tower::ServiceExt;

use crate::{data_source, demo, router, AppState};

fn demo_app() -> Router {
    router(Arc::new(AppState::new(
        demo::build_demo_data(260),
        data_source::AppDataSource::Demo,
        260,
        260,
    )))
}

async fn get_json(uri: &str) -> serde_json::Value {
    let response = demo_app()
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
    let response = demo_app()
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
    assert!(json["position_guidance"]["governance"]["system_budget_only"].is_boolean());
    assert!(json["position_guidance"]["governance"]["auto_execution_allowed"].is_boolean());
    assert!(json["position_guidance"]["governance"]["manual_confirmation_required"].is_boolean());
    assert!(
        json["position_guidance"]["governance"]["policy_change_requires_release_review"]
            .is_boolean()
    );
    assert!(
        json["position_guidance"]["governance"]["policy_change_requires_go_no_go"].is_boolean()
    );
    assert!(json["position_guidance"]["governance"]["required_operator_checks"].is_array());
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
    assert!(json["probability_diagnostics"]["horizon_overlays"].is_array());
    assert!(json["method"]["actionability_enabled"].is_boolean());
    assert!(json["backtest_summary"]["timely_warning_rate"].is_number());
    assert!(json["backtest_summary"]["rolling_audit"]["actionable_precision"].is_number());
    assert!(
        json["backtest_summary"]["rolling_audit"]["history_start"].is_null()
            || json["backtest_summary"]["rolling_audit"]["history_start"].is_string()
    );
    assert!(
        json["backtest_summary"]["rolling_audit"]["history_end"].is_null()
            || json["backtest_summary"]["rolling_audit"]["history_end"].is_string()
    );
    assert!(json["backtest_summary"]["rolling_audit"]["scope_note"].is_string());
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
    assert!(json["history_provenance"]["evidence_tier"].is_string());
    assert!(json["history_provenance"]["dominant_source"].is_string());
    assert!(json["history_provenance"]["sources"].is_array());
    assert!(json["latest_replay_run_id"].is_null() || json["latest_replay_run_id"].is_string());
    assert!(
        json["latest_release_review"].is_null()
            || (json["latest_release_review"]["reviewed_at"].is_string()
                && json["latest_release_review"]["overall_guard_passed"].is_boolean()
                && json["latest_release_review"]["historical_audit_actions"].is_array()
                && json["latest_release_review"]["historical_audit_attribution"].is_array())
    );
    assert!(
        json["latest_scenario_pack_audit"].is_null()
            || (json["latest_scenario_pack_audit"]["generated_at"].is_string()
                && json["latest_scenario_pack_audit"]["blocker_counts"].is_array()
                && json["latest_scenario_pack_audit"]["scenario_summaries"].is_array())
    );
    assert!(
        json["latest_rate_shock_audit"].is_null()
            || (json["latest_rate_shock_audit"]["generated_at"].is_string()
                && json["latest_rate_shock_audit"]["phase_summaries"].is_array()
                && json["latest_rate_shock_audit"]["action_level_summaries"].is_array())
    );
    assert!(json["releases"].is_array());
    assert!(json["replay_runs"].is_array());
    assert!(json["snapshots"].is_array());
}

#[tokio::test]
async fn system_reload_endpoint_returns_refresh_metadata() {
    let response = demo_app()
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
    assert_eq!(json["history_mode"], "default");
    assert_eq!(json["history_limit"], 260);
    assert_eq!(json["runtime_purpose"], "production");
    assert!(json["generated_at"].is_string());
}

#[tokio::test]
async fn system_reload_endpoint_accepts_strict_rebuild_history_mode() {
    let response = demo_app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/system/reload?history_mode=strict_rebuild")
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
    assert_eq!(json["history_mode"], "strict_rebuild");
    assert_eq!(json["history_limit"], 260);
}

#[tokio::test]
async fn system_reload_endpoint_accepts_history_limit() {
    let response = demo_app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/system/reload?history_limit=25")
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
    assert_eq!(json["history_limit"], 25);
}

#[tokio::test]
async fn system_reload_endpoint_accepts_review_runtime_purpose() {
    let response = demo_app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/system/reload?runtime_purpose=review")
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
    assert_eq!(json["runtime_purpose"], "review");
}

#[tokio::test]
async fn system_reload_endpoint_rejects_unknown_runtime_purpose() {
    let response = demo_app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/system/reload?runtime_purpose=unknown")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn app_state_preserves_review_reload_options_across_refresh() {
    let state = AppState::new(
        demo::build_demo_data(260),
        data_source::AppDataSource::Demo,
        260,
        260,
    );

    state
        .reload_with_runtime_options(
            data_source::AssessmentHistoryBuildMode::StrictRebuild,
            25,
            data_source::ServingRuntimePurpose::Review,
        )
        .await
        .unwrap();

    let (history_mode, history_limit, runtime_purpose) = state.current_reload_config().await;
    assert_eq!(
        history_mode,
        data_source::AssessmentHistoryBuildMode::StrictRebuild
    );
    assert_eq!(history_limit, 25);
    assert_eq!(runtime_purpose, data_source::ServingRuntimePurpose::Review);

    state.reload().await.unwrap();

    let (history_mode, history_limit, runtime_purpose) = state.current_reload_config().await;
    assert_eq!(
        history_mode,
        data_source::AssessmentHistoryBuildMode::StrictRebuild
    );
    assert_eq!(history_limit, 25);
    assert_eq!(runtime_purpose, data_source::ServingRuntimePurpose::Review);
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
    assert!(json["history_provenance"]["evidence_tier"].is_string());
    assert!(json["history_provenance"]["dominant_source"].is_string());
    assert!(json["history_provenance"]["total_points"].is_number());
    assert!(json["history_provenance"]["feature_backed_points"].is_number());
    assert!(json["history_provenance"]["reused_feature_snapshot_points"].is_number());
    assert!(json["history_provenance"]["raw_observation_points"].is_number());
    assert!(json["history_provenance"]["snapshot_bridge_points"].is_number());
    assert!(json["history_provenance"]["runtime_only_points"].is_number());
    assert!(json["history_provenance"]["note"].is_string());
    assert!(json["history_provenance"]["sources"].is_array());
    assert!(json["protected_stress_window_catalog"]["catalog_id"].is_string());
    assert!(json["protected_stress_window_catalog"]["windows"].is_array());
    assert!(json["scenario_data_coverage_catalog"]["catalog_id"].is_string());
    assert!(json["scenario_data_coverage_catalog"]["records"].is_array());
    assert!(json["runtime_thresholds"]["prepare_p60d"].is_number());
    assert!(json["runtime_thresholds"]["hedge_p20d"].is_number());
    assert!(json["runtime_thresholds"]["defend_p5d"].is_number());
    assert!(json["runtime_thresholds"]["history_runtime_policy_version"].is_string());
}

#[tokio::test]
async fn api_responses_disable_browser_cache() {
    let response = demo_app()
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

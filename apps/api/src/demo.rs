use std::env;

use chrono::NaiveDate;
use fc_domain::{
    load_protected_stress_window_catalog, AlertEvent, DataMode, Indicator, Observation,
    PredictionSnapshotRecord, UserRiskPreferences, UserRiskProfile,
};
use fc_scoring::ScoringEngine;

use crate::assessment::{
    build_assessment_snapshot, build_backtest_summary, probability_action_thresholds,
    runtime_threshold_diagnostics, ServingModelContext,
};
use crate::backtest::{
    build_backtest_timeline, build_backtest_timeline_with_thresholds, build_backtests,
    build_backtests_with_thresholds, build_rolling_backtest_audit,
    build_rolling_backtest_audit_with_thresholds, use_transitional_actionable_bridge,
};
#[cfg(test)]
use crate::backtest::{is_actionable_warning_point, is_actionable_warning_point_with_thresholds};
use crate::demo_seed::{
    build_alerts, indicators, observations, select_recent_alerts_for_date, sources_demo,
    sources_runtime,
};
use crate::history_builder::{build_assessment_history, HistoryQueryWindow};
#[cfg(test)]
pub(crate) use crate::history_replay::expected_prediction_snapshot_method_version;
pub(crate) use crate::history_replay::{
    assessment_history_point_from_assessment, prediction_snapshot_from_assessment,
};
use crate::AppData;

pub(crate) const FORMAL_MAIN_FEATURE_SET_PREFIX: &str = "feature_formal_v1_main";
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) const FORMAL_MAIN_FEATURE_SET_VERSION: &str = "feature_formal_v1_main_20260606_gatefix";
pub(crate) const FORMAL_MAIN_LABEL_VERSION: &str = "formal_label_v1_main";

pub(crate) fn is_formal_main_feature_set(feature_set_version: &str, label_version: &str) -> bool {
    label_version == FORMAL_MAIN_LABEL_VERSION
        && feature_set_version.starts_with(FORMAL_MAIN_FEATURE_SET_PREFIX)
}

#[derive(Debug)]
pub(crate) struct BuiltAppData {
    pub(crate) app_data: AppData,
    pub(crate) prediction_snapshots: Vec<PredictionSnapshotRecord>,
}

#[derive(Debug, Clone)]
pub(crate) struct ScenarioBacktestContext {
    pub(crate) history: Vec<fc_domain::AssessmentHistoryPoint>,
    pub(crate) coverage_scope_note: String,
}

pub fn build_demo_data(_max_history_points: usize) -> AppData {
    let as_of_date = NaiveDate::from_ymd_opt(2026, 5, 30).expect("valid date");
    let indicators = indicators();
    let observations = observations(as_of_date);
    let user_preferences = load_user_preferences();
    let historical = build_assessment_history(
        DataMode::Demo,
        &ScoringEngine::default(),
        &indicators,
        &observations,
        None,
        None,
        &user_preferences,
        HistoryQueryWindow {
            from: None,
            to: None,
            limit: None,
        },
    );
    build_app_data_from_inputs(
        DataMode::Demo,
        indicators,
        observations,
        None,
        None,
        as_of_date,
        historical.history_points,
        None,
        user_preferences,
    )
    .app_data
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_app_data_from_inputs(
    data_mode: DataMode,
    indicators: Vec<Indicator>,
    observations: Vec<Observation>,
    stored_alerts: Option<Vec<AlertEvent>>,
    serving_model: Option<ServingModelContext>,
    as_of_date: NaiveDate,
    mut assessment_history: Vec<fc_domain::AssessmentHistoryPoint>,
    scenario_backtest_context: Option<ScenarioBacktestContext>,
    user_preferences: UserRiskPreferences,
) -> BuiltAppData {
    let use_transitional_bridge = use_transitional_actionable_bridge(serving_model.as_ref());
    let scoring = ScoringEngine::default();
    let protected_stress_window_catalog = load_protected_stress_window_catalog();
    let threshold_diagnostics = runtime_threshold_diagnostics(serving_model.as_ref());
    let strict_thresholds =
        (!use_transitional_bridge).then(|| probability_action_thresholds(serving_model.as_ref()));
    let scenario_backtest_history = scenario_backtest_context
        .as_ref()
        .map(|context| context.history.as_slice())
        .unwrap_or(assessment_history.as_slice());
    let scenario_backtest_history_start =
        scenario_backtest_history.first().map(|point| point.as_of_date);
    let scenario_backtest_history_end =
        scenario_backtest_history.last().map(|point| point.as_of_date);
    let output = scoring.score(
        &indicators,
        &observations,
        as_of_date,
        "us",
        "financial_system",
    );
    let backtests = strict_thresholds
        .map(|thresholds| {
            build_backtests_with_thresholds(
                &output.snapshot,
                scenario_backtest_history,
                use_transitional_bridge,
                Some(thresholds),
            )
        })
        .unwrap_or_else(|| {
            build_backtests(
                &output.snapshot,
                scenario_backtest_history,
                use_transitional_bridge,
            )
        });
    let rolling_audit = strict_thresholds
        .map(|thresholds| {
            build_rolling_backtest_audit_with_thresholds(
                &assessment_history,
                &protected_stress_window_catalog.windows,
                use_transitional_bridge,
                Some(thresholds),
            )
        })
        .unwrap_or_else(|| {
            build_rolling_backtest_audit(
                &assessment_history,
                &protected_stress_window_catalog.windows,
                use_transitional_bridge,
            )
        });
    let alerts = stored_alerts
        .map(|alerts| select_recent_alerts_for_date(&alerts, as_of_date))
        .unwrap_or_else(|| build_alerts(&output.snapshot));
    let mut backtest_summary = build_backtest_summary(&backtests, Some(&rolling_audit));
    backtest_summary.history_start = scenario_backtest_history_start;
    backtest_summary.history_end = scenario_backtest_history_end;
    if let Some(context) = scenario_backtest_context.as_ref() {
        backtest_summary.coverage_scope_note = context.coverage_scope_note.clone();
    } else {
        backtest_summary.coverage_scope_note =
            match (scenario_backtest_history_start, scenario_backtest_history_end) {
                (Some(start), Some(end)) => format!(
                    "这里的“本地覆盖场景 / 模板参照场景”按场景回测历史窗口 {start} 到 {end} 统计；它回答的是危机场景目录里有多少样本能直接落在这段本地历史上，不等于上面默认历史轨迹是否已经进入 PIT 正式证据层。"
                ),
                _ => "这里的“本地覆盖场景 / 模板参照场景”按当前场景回测历史窗口统计；它回答的是危机场景目录里有多少样本能直接落在本地历史上，不等于上面默认历史轨迹是否已经进入 PIT 正式证据层。".to_string(),
            };
    }
    let (assessment, posture_guidance, probability_trace) = build_assessment_snapshot(
        data_mode,
        &output.snapshot,
        &output.indicator_risks,
        &observations,
        &alerts,
        &backtests,
        Some(&rolling_audit),
        serving_model.as_ref(),
        &user_preferences,
    );
    let assessment = fc_domain::AssessmentSnapshot {
        backtest_summary,
        ..assessment
    };
    let mut current_history_point = assessment_history_point_from_assessment(
        &assessment,
        &posture_guidance,
        &probability_trace,
    );
    match assessment_history.last_mut() {
        Some(last) if last.as_of_date == current_history_point.as_of_date => {
            current_history_point.replay_run_id = last.replay_run_id.clone();
            current_history_point.feature_snapshot_id = last.feature_snapshot_id.clone();
            current_history_point.history_source = last.history_source.clone();
            *last = current_history_point;
        }
        _ => assessment_history.push(current_history_point),
    }
    let backtest_timeline = strict_thresholds
        .map(|thresholds| {
            build_backtest_timeline_with_thresholds(
                &assessment_history,
                use_transitional_bridge,
                Some(thresholds),
            )
        })
        .unwrap_or_else(|| build_backtest_timeline(&assessment_history, use_transitional_bridge));
    let current_prediction_snapshot = prediction_snapshot_from_assessment(
        &assessment,
        &posture_guidance,
        &probability_trace,
        serving_model.as_ref(),
    );
    BuiltAppData {
        app_data: AppData {
            data_mode,
            user_preferences,
            overview: output.snapshot,
            indicators: output.indicator_risks,
            alerts,
            sources: if matches!(data_mode, DataMode::Demo) {
                sources_demo()
            } else {
                sources_runtime(&observations, as_of_date)
            },
            backtests,
            backtest_timeline,
            assessment,
            assessment_history,
            posture_guidance,
            protected_stress_window_catalog,
            runtime_thresholds: threshold_diagnostics,
        },
        prediction_snapshots: vec![current_prediction_snapshot],
    }
}

pub(crate) fn load_user_preferences() -> UserRiskPreferences {
    let profile = match env::var("FC_USER_RISK_PROFILE")
        .unwrap_or_else(|_| "neutral".to_string())
        .to_lowercase()
        .as_str()
    {
        "conservative" => UserRiskProfile::Conservative,
        "aggressive" => UserRiskProfile::Aggressive,
        _ => UserRiskProfile::Neutral,
    };
    let cash_floor_pct = env::var("FC_USER_CASH_FLOOR_PCT")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(15.0);
    let max_equity_cap_pct = env::var("FC_USER_MAX_EQUITY_CAP_PCT")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(70.0);
    let max_leverage_pct = env::var("FC_USER_MAX_LEVERAGE_PCT")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(100.0);
    let option_overlay_preference_pct = env::var("FC_USER_OPTION_OVERLAY_PCT")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(5.0);
    let allow_aggressive_reentry = env::var("FC_USER_ALLOW_AGGRESSIVE_REENTRY")
        .ok()
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "True"))
        .unwrap_or(false);

    let note = format!(
        "profile={}, cash_floor={}%, max_equity={}%, max_leverage={}%, option_overlay={}%",
        match profile {
            UserRiskProfile::Conservative => "conservative",
            UserRiskProfile::Neutral => "neutral",
            UserRiskProfile::Aggressive => "aggressive",
        },
        cash_floor_pct,
        max_equity_cap_pct,
        max_leverage_pct,
        option_overlay_preference_pct
    );

    UserRiskPreferences {
        profile,
        cash_floor_pct,
        max_equity_cap_pct,
        max_leverage_pct,
        option_overlay_preference_pct,
        allow_aggressive_reentry,
        note,
    }
}

#[cfg(test)]
mod tests;

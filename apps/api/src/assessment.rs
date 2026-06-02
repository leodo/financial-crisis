use std::env;

use chrono::NaiveDate;
use chrono::Utc;
use fc_domain::{
    AlertEvent, AssessmentMethodVersions, AssessmentScores, AssessmentSnapshot,
    BacktestPerformanceSummary, BacktestRollingAudit, BacktestScenarioSummary,
    BacktestSignalSource, DataMode, DataTrust, DecisionPosture, EventAssessment,
    EventConfirmationState, EventSignalSummary, FreshnessStatus, HistoricalAnalog, IndicatorRisk,
    JpyCarrySnapshot, JpyCarryState, KeyIndicatorStatus, ModelReleaseRecord, Observation,
    PostureGuidance, ProbabilityBlock, ProbabilityBundle, RiskContributor, RiskDimension,
    RiskSnapshot, RuntimeMetadata, UserRiskPreferences,
};
use serde::Serialize;

mod posture;
mod probability;

use posture::{
    build_position_guidance, build_posture_guidance, build_summary, build_time_to_risk_bucket,
};
pub(crate) use probability::ProbabilityComputationTrace;
#[cfg(test)]
use probability::{actionability_confidence_from_probability, fuse_actionability_confidence};
use probability::{build_probabilities, build_probability_trace};

const PROB_MODEL_VERSION: &str = "prob_v1_20260531";
const CALIBRATION_VERSION: &str = "calib_v1_20260531";
const FEATURE_SET_VERSION: &str = "feature_v2_20260531";
const LABEL_VERSION: &str = "label_v1_20260530";
const POSTURE_POLICY_VERSION: &str = "posture_v1_20260530";
const ACTION_PLAYBOOK_VERSION: &str = "action_playbook_v1_20260531";
const PROBABILITY_MODE: &str = "heuristic_mvp";
const RELEASE_STATUS: &str = "degraded";
const PREPARE_P60D_THRESHOLD: f64 = 0.35;
const HEDGE_P20D_THRESHOLD: f64 = 0.30;
const DEFEND_P5D_THRESHOLD: f64 = 0.30;
const FORMAL_MAIN_PREPARE_P60D_THRESHOLD: f64 = 0.10;
const FORMAL_MAIN_HEDGE_P20D_THRESHOLD: f64 = 0.07;
const FORMAL_MAIN_DEFEND_P5D_THRESHOLD: f64 = 0.03;
const FORMAL_MAIN_RUNTIME_PREPARE_P60D_FLOOR: f64 = 0.12;
const FORMAL_MAIN_RUNTIME_HEDGE_P20D_FLOOR: f64 = 0.06;
const FORMAL_MAIN_RUNTIME_DEFEND_P5D_FLOOR: f64 = 0.05;

#[derive(Debug, Clone, Copy)]
struct ProbabilityActionThresholds {
    prepare_p60d: f64,
    hedge_p20d: f64,
    defend_p5d: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeThresholdDiagnostics {
    pub prepare_p60d: f64,
    pub hedge_p20d: f64,
    pub defend_p5d: f64,
    pub severe_now_p20d: f64,
    pub elevated_weeks_p60d: f64,
    pub external_prepare_p20d: f64,
    pub carry_prepare_p60d: f64,
    pub downgrade_prepare_p60d: f64,
    pub downgrade_hedge_p20d: f64,
    pub downgrade_defend_p5d: f64,
    pub history_runtime_policy_version: String,
}

impl ProbabilityActionThresholds {
    fn legacy() -> Self {
        Self {
            prepare_p60d: PREPARE_P60D_THRESHOLD,
            hedge_p20d: HEDGE_P20D_THRESHOLD,
            defend_p5d: DEFEND_P5D_THRESHOLD,
        }
    }

    fn formal_main_runtime() -> Self {
        Self {
            prepare_p60d: probability_threshold_env_override(
                "FC_FORMAL_MAIN_RUNTIME_PREPARE_P60D_FLOOR",
                FORMAL_MAIN_RUNTIME_PREPARE_P60D_FLOOR,
            ),
            hedge_p20d: probability_threshold_env_override(
                "FC_FORMAL_MAIN_RUNTIME_HEDGE_P20D_FLOOR",
                FORMAL_MAIN_RUNTIME_HEDGE_P20D_FLOOR,
            ),
            defend_p5d: probability_threshold_env_override(
                "FC_FORMAL_MAIN_RUNTIME_DEFEND_P5D_FLOOR",
                FORMAL_MAIN_RUNTIME_DEFEND_P5D_FLOOR,
            ),
        }
    }

    fn severe_now_p20d(self) -> f64 {
        (self.hedge_p20d + 0.20).max(self.hedge_p20d * 2.0)
    }

    fn elevated_weeks_p60d(self) -> f64 {
        (self.prepare_p60d + 0.10).max(self.prepare_p60d * 1.6)
    }

    fn external_prepare_p20d(self) -> f64 {
        (self.hedge_p20d * 0.7).max(0.04)
    }

    fn carry_prepare_p60d(self) -> f64 {
        (self.prepare_p60d * 0.8).max(0.05)
    }

    fn downgrade_prepare_p60d(self) -> f64 {
        (self.prepare_p60d * 0.75).max(0.05)
    }

    fn downgrade_hedge_p20d(self) -> f64 {
        (self.hedge_p20d * 0.75).max(0.04)
    }

    fn downgrade_defend_p5d(self) -> f64 {
        (self.defend_p5d * 0.67).max(0.02)
    }

    fn capital_preservation_p5d(self) -> f64 {
        (self.defend_p5d * 1.5).max(self.defend_p5d + 0.02)
    }
}

fn probability_threshold_env_override(name: &str, fallback: f64) -> f64 {
    env::var(name)
        .ok()
        .and_then(|raw| raw.parse::<f64>().ok())
        .map(|value| value.clamp(0.001, 0.90))
        .unwrap_or(fallback)
}

#[derive(Debug, Clone)]
pub struct ServingModelContext {
    pub release: ModelReleaseRecord,
    pub probability_bundle: Option<ProbabilityBundle>,
    pub runtime_probability_mode: String,
    pub runtime_release_status: String,
}

fn probability_action_thresholds(
    serving_model: Option<&ServingModelContext>,
) -> ProbabilityActionThresholds {
    let Some(serving_model) = serving_model else {
        return ProbabilityActionThresholds::legacy();
    };
    let active_release = &serving_model.release;

    if active_release.manifest.feature_set_version == "feature_formal_v1_main_20260531"
        && active_release.manifest.label_version == "formal_label_v1_main"
    {
        if let Some(bundle) = serving_model.probability_bundle.as_ref() {
            ProbabilityActionThresholds {
                prepare_p60d: bundle_horizon_threshold(
                    bundle,
                    60,
                    FORMAL_MAIN_PREPARE_P60D_THRESHOLD,
                )
                .max(FORMAL_MAIN_RUNTIME_PREPARE_P60D_FLOOR),
                hedge_p20d: bundle_horizon_threshold(bundle, 20, FORMAL_MAIN_HEDGE_P20D_THRESHOLD)
                    .max(FORMAL_MAIN_RUNTIME_HEDGE_P20D_FLOOR),
                defend_p5d: bundle_horizon_threshold(bundle, 5, FORMAL_MAIN_DEFEND_P5D_THRESHOLD)
                    .max(FORMAL_MAIN_RUNTIME_DEFEND_P5D_FLOOR),
            }
        } else {
            ProbabilityActionThresholds::formal_main_runtime()
        }
    } else {
        ProbabilityActionThresholds::legacy()
    }
}

pub(crate) fn history_runtime_policy_version(
    serving_model: Option<&ServingModelContext>,
) -> String {
    let thresholds = probability_action_thresholds(serving_model);
    let release_class = if serving_model.is_some_and(|context| {
        context.release.manifest.feature_set_version == "feature_formal_v1_main_20260531"
            && context.release.manifest.label_version == "formal_label_v1_main"
    }) {
        "formal_main"
    } else if serving_model.is_some() {
        "release"
    } else {
        "heuristic"
    };

    // Cached prediction snapshots embed posture/time-bucket outputs. When runtime
    // thresholds are tightened or relaxed, history must be recomputed even if the
    // release manifest itself did not change.
    format!(
        "runtime_history_v2_20260602|class={release_class}|prepare={:.3}|hedge={:.3}|defend={:.3}",
        thresholds.prepare_p60d, thresholds.hedge_p20d, thresholds.defend_p5d
    )
}

pub fn runtime_threshold_diagnostics(
    serving_model: Option<&ServingModelContext>,
) -> RuntimeThresholdDiagnostics {
    let thresholds = probability_action_thresholds(serving_model);
    RuntimeThresholdDiagnostics {
        prepare_p60d: round3(thresholds.prepare_p60d),
        hedge_p20d: round3(thresholds.hedge_p20d),
        defend_p5d: round3(thresholds.defend_p5d),
        severe_now_p20d: round3(thresholds.severe_now_p20d()),
        elevated_weeks_p60d: round3(thresholds.elevated_weeks_p60d()),
        external_prepare_p20d: round3(thresholds.external_prepare_p20d()),
        carry_prepare_p60d: round3(thresholds.carry_prepare_p60d()),
        downgrade_prepare_p60d: round3(thresholds.downgrade_prepare_p60d()),
        downgrade_hedge_p20d: round3(thresholds.downgrade_hedge_p20d()),
        downgrade_defend_p5d: round3(thresholds.downgrade_defend_p5d()),
        history_runtime_policy_version: history_runtime_policy_version(serving_model),
    }
}

fn bundle_horizon_threshold(bundle: &ProbabilityBundle, horizon_days: u32, fallback: f64) -> f64 {
    bundle
        .horizons
        .iter()
        .find(|horizon| horizon.horizon_days == horizon_days)
        .and_then(|horizon| horizon.decision_threshold)
        .map(|threshold| threshold.clamp(0.001, 0.90))
        .unwrap_or(fallback)
}

#[allow(clippy::too_many_arguments)]
pub fn build_assessment_snapshot(
    data_mode: DataMode,
    snapshot: &RiskSnapshot,
    indicator_risks: &[IndicatorRisk],
    observations: &[Observation],
    alerts: &[AlertEvent],
    backtests: &[BacktestScenarioSummary],
    rolling_audit: Option<&BacktestRollingAudit>,
    serving_model: Option<&ServingModelContext>,
    user_preferences: &UserRiskPreferences,
) -> (
    AssessmentSnapshot,
    PostureGuidance,
    ProbabilityComputationTrace,
) {
    let jpy_carry = build_jpy_carry_snapshot(snapshot, indicator_risks, observations);
    let external_dimension_score = snapshot
        .dimensions
        .iter()
        .find(|dimension| dimension.dimension == RiskDimension::ExternalSector)
        .map(|dimension| dimension.score)
        .unwrap_or(0.0);
    let event_dimension_score = snapshot
        .dimensions
        .iter()
        .find(|dimension| dimension.dimension == RiskDimension::EventsSentiment)
        .map(|dimension| dimension.score)
        .unwrap_or(0.0);
    let external_shock_score = round1(
        (external_dimension_score * 0.45 + jpy_carry.score * 0.4 + event_dimension_score * 0.15)
            .clamp(0.0, 100.0),
    );
    let data_trust = build_data_trust(snapshot, indicator_risks, jpy_carry.usdjpy_level.is_some());
    let breadth_score = high_risk_breadth(snapshot);
    let conviction_score = build_conviction_score(snapshot, &data_trust, breadth_score);
    let heuristic_probabilities = build_probabilities(
        snapshot,
        external_shock_score,
        conviction_score,
        breadth_score,
        &data_trust,
        &jpy_carry,
    );
    let runtime = build_runtime_metadata(data_mode, snapshot, observations);
    let key_indicators = build_key_indicator_statuses(observations, snapshot.as_of_date, data_mode);
    let probability_trace = build_probability_trace(
        snapshot,
        observations,
        external_shock_score,
        &data_trust,
        &jpy_carry,
        &heuristic_probabilities,
        &key_indicators,
        serving_model,
    );
    let probabilities = probability_trace.calibrated_probabilities.clone();
    let actionability = probability_trace.actionability.clone();
    let actionability_fusion = probability_trace
        .actionability_enabled
        .then_some(&actionability);
    let active_release = serving_model.map(|context| &context.release);
    let action_thresholds = probability_action_thresholds(serving_model);
    let time_to_risk_bucket = build_time_to_risk_bucket(
        &probabilities,
        actionability_fusion,
        snapshot.structural_score,
        snapshot.trigger_score,
        external_shock_score,
        breadth_score,
        &jpy_carry,
        action_thresholds,
    );
    let top_risk_drivers = snapshot.top_contributors.clone();
    let top_relief_drivers = build_relief_drivers(indicator_risks);
    let historical_analogs = build_historical_analogs(
        snapshot,
        &probabilities,
        external_shock_score,
        backtests,
        action_thresholds,
    );
    let event_assessment = build_event_assessment(snapshot, alerts);
    let backtest_summary = build_backtest_summary(backtests, rolling_audit);
    let posture_guidance = build_posture_guidance(
        snapshot,
        &probabilities,
        actionability_fusion,
        conviction_score,
        &data_trust,
        external_shock_score,
        breadth_score,
        &historical_analogs,
        &jpy_carry,
        &event_assessment,
        user_preferences,
        action_thresholds,
    );
    let position_guidance = build_position_guidance(
        &posture_guidance,
        &probabilities,
        time_to_risk_bucket,
        &data_trust,
        &jpy_carry,
        &event_assessment,
        active_release,
        user_preferences,
        action_thresholds,
    );
    let method = AssessmentMethodVersions {
        score_method_version: snapshot.method_version.clone(),
        prob_model_version: active_release
            .map(|release| release.manifest.prob_model_version.clone())
            .unwrap_or_else(|| PROB_MODEL_VERSION.to_string()),
        calibration_version: active_release
            .map(|release| release.manifest.calibration_version.clone())
            .unwrap_or_else(|| CALIBRATION_VERSION.to_string()),
        actionability_model_version: probability_trace.actionability_model_version.clone(),
        actionability_calibration_version: probability_trace
            .actionability_calibration_version
            .clone(),
        feature_set_version: active_release
            .map(|release| release.manifest.feature_set_version.clone())
            .unwrap_or_else(|| FEATURE_SET_VERSION.to_string()),
        label_version: active_release
            .map(|release| release.manifest.label_version.clone())
            .unwrap_or_else(|| LABEL_VERSION.to_string()),
        posture_policy_version: active_release
            .map(|release| release.manifest.posture_policy_version.clone())
            .unwrap_or_else(|| POSTURE_POLICY_VERSION.to_string()),
        action_playbook_version: active_release
            .map(|release| release.manifest.action_playbook_version.clone())
            .unwrap_or_else(|| ACTION_PLAYBOOK_VERSION.to_string()),
        fusion_policy_version: probability_trace.fusion_policy_version.clone(),
        actionability_enabled: probability_trace.actionability_enabled,
        probability_mode: serving_model
            .map(|context| context.runtime_probability_mode.clone())
            .unwrap_or_else(|| PROBABILITY_MODE.to_string()),
        release_status: serving_model
            .map(|context| context.runtime_release_status.clone())
            .unwrap_or_else(|| RELEASE_STATUS.to_string()),
        release_id: active_release.map(|release| release.manifest.release_id.clone()),
        point_in_time_mode: active_release
            .map(|release| release.manifest.point_in_time_mode.clone())
            .unwrap_or_else(|| "best_effort".to_string()),
    };
    let summary = build_summary(
        snapshot,
        &probabilities,
        time_to_risk_bucket,
        &posture_guidance,
    );

    (
        AssessmentSnapshot {
            as_of_date: snapshot.as_of_date,
            entity_id: snapshot.entity_id.clone(),
            market_scope: snapshot.market_scope.clone(),
            probabilities,
            actionability,
            time_to_risk_bucket,
            posture: posture_guidance.posture,
            conviction_score,
            scores: AssessmentScores {
                overall_score: snapshot.overall_score,
                structural_score: snapshot.structural_score,
                trigger_score: snapshot.trigger_score,
                external_shock_score,
            },
            summary,
            posture_reason: posture_guidance.summary.clone(),
            top_risk_drivers,
            top_relief_drivers,
            historical_analogs,
            data_trust,
            jpy_carry,
            position_guidance,
            runtime,
            key_indicators,
            event_assessment,
            backtest_summary,
            user_preferences: user_preferences.clone(),
            method,
        },
        posture_guidance,
        probability_trace,
    )
}

fn build_runtime_metadata(
    data_mode: DataMode,
    snapshot: &RiskSnapshot,
    observations: &[Observation],
) -> RuntimeMetadata {
    let latest_observation_at = observations
        .iter()
        .filter(|observation| {
            !observation
                .quality_flags
                .iter()
                .any(|flag| flag == "synthetic_zero_fill")
        })
        .map(|observation| observation.as_of_date)
        .max()
        .or_else(|| {
            observations
                .iter()
                .map(|observation| observation.as_of_date)
                .max()
        });
    let latest_observation_lag_days =
        latest_observation_at.map(|date| (snapshot.as_of_date - date).num_days());
    let demo_mode = matches!(data_mode, DataMode::Demo);
    let stale_warning = if demo_mode {
        Some("当前页面运行在 demo 模式，关键指标值是示例数据，不代表真实市场最新状态。".to_string())
    } else if let Some(lag) = latest_observation_lag_days {
        (lag > 5).then(|| format!("当前评估使用的最新观测值滞后 {lag} 天，短期判断需要保守解释。"))
    } else {
        Some("当前缺少最新观测值，不能把面板数字当成实时市场状态。".to_string())
    };

    RuntimeMetadata {
        data_mode,
        generated_at: Utc::now(),
        requested_as_of_date: snapshot.as_of_date,
        latest_observation_at,
        latest_observation_lag_days,
        demo_mode,
        stale_warning,
    }
}

fn build_key_indicator_statuses(
    observations: &[Observation],
    requested_as_of_date: NaiveDate,
    data_mode: DataMode,
) -> Vec<KeyIndicatorStatus> {
    [
        (
            "us_external_usdjpy_level",
            "USDJPY",
            "us",
            "jpy_per_usd",
            3_i64,
        ),
        (
            "jp_rates_call_rate",
            "日本无担保隔夜拆借利率",
            "jp",
            "percent",
            5_i64,
        ),
        (
            "us_liquidity_effr",
            "有效联邦基金利率",
            "us",
            "percent",
            5_i64,
        ),
        ("us_market_vix_close", "VIX 收盘价", "us", "index", 3_i64),
    ]
    .into_iter()
    .map(
        |(indicator_id, display_name, entity_id, unit, stale_threshold_days)| {
            let latest = observations
                .iter()
                .filter(|observation| observation.indicator_id == indicator_id)
                .filter(|observation| observation.entity_id == entity_id)
                .filter(|observation| observation.as_of_date <= requested_as_of_date)
                .max_by_key(|observation| observation.as_of_date);

            let latest_as_of_date = latest.map(|observation| observation.as_of_date);
            let lag_days = latest_as_of_date.map(|date| (requested_as_of_date - date).num_days());
            let status = if matches!(data_mode, DataMode::Demo) {
                FreshnessStatus::Stale
            } else if latest.is_none() {
                FreshnessStatus::Missing
            } else if lag_days.unwrap_or_default() > stale_threshold_days * 3 {
                FreshnessStatus::Stale
            } else if lag_days.unwrap_or_default() > stale_threshold_days {
                FreshnessStatus::Delayed
            } else {
                FreshnessStatus::Fresh
            };

            let note = if matches!(data_mode, DataMode::Demo) {
                "demo 示例数据，不代表真实市场最新值。".to_string()
            } else {
                match status {
                    FreshnessStatus::Fresh => "关键指标处于可接受的新鲜度范围。".to_string(),
                    FreshnessStatus::Delayed => {
                        "指标有一定滞后，近端风险判断要结合其他证据。".to_string()
                    }
                    FreshnessStatus::Stale => {
                        "指标明显陈旧，不能把当前显示值当成实时市场状态。".to_string()
                    }
                    FreshnessStatus::Missing => "缺少该指标最新值。".to_string(),
                }
            };

            KeyIndicatorStatus {
                indicator_id: indicator_id.to_string(),
                display_name: display_name.to_string(),
                entity_id: entity_id.to_string(),
                source_id: latest.map(|observation| observation.source_id.clone()),
                dataset_id: latest.map(|observation| observation.dataset_id.clone()),
                unit: unit.to_string(),
                latest_value: latest.map(|observation| observation.value),
                latest_as_of_date,
                lag_days,
                stale_threshold_days,
                status,
                note,
            }
        },
    )
    .collect()
}

fn build_event_assessment(snapshot: &RiskSnapshot, alerts: &[AlertEvent]) -> EventAssessment {
    let recent_event_count = alerts.len() as u32;
    let recent_events = alerts
        .iter()
        .take(4)
        .map(|alert| EventSignalSummary {
            event_type: alert.event_type,
            level: alert.level,
            triggered_as_of_date: alert.triggered_as_of_date,
            trigger_reason: alert.trigger_reason.clone(),
            related_indicators: alert.related_indicators.clone(),
        })
        .collect::<Vec<_>>();
    let confirmation_score = round1(
        (snapshot
            .dimensions
            .iter()
            .find(|dimension| dimension.dimension == RiskDimension::EventsSentiment)
            .map(|dimension| dimension.score)
            .unwrap_or(0.0)
            * 0.7
            + recent_event_count as f64 * 9.0)
            .clamp(0.0, 100.0),
    );
    let state = if confirmation_score >= 70.0 {
        EventConfirmationState::Escalating
    } else if confirmation_score >= 55.0 {
        EventConfirmationState::Confirmed
    } else if confirmation_score >= 30.0 {
        EventConfirmationState::Watching
    } else {
        EventConfirmationState::Quiet
    };

    let confirmed_signals = alerts
        .iter()
        .map(|alert| alert.trigger_reason.clone())
        .take(3)
        .collect::<Vec<_>>();
    let mut pending_gaps = Vec::new();
    if recent_event_count == 0 {
        pending_gaps.push("事件层还没有给出足够确认，当前更多依赖价格和宏观层信号。".to_string());
    }
    if snapshot.trigger_score >= 60.0 && recent_event_count < 2 {
        pending_gaps.push("触发层已抬升，但银行/公告/新闻事件还没有形成更强共振。".to_string());
    }

    let summary = match state {
        EventConfirmationState::Quiet => {
            "事件层暂时安静，当前风险判断主要来自价格和融资信号。".to_string()
        }
        EventConfirmationState::Watching => {
            "事件层开始出现支持证据，但还不足以单独驱动强结论。".to_string()
        }
        EventConfirmationState::Confirmed => {
            "事件层已经提供了实质性确认，当前风险判断不再只是市场噪声。".to_string()
        }
        EventConfirmationState::Escalating => {
            "事件层与市场层正在同步升级，需优先防范短期风险压缩。".to_string()
        }
    };

    EventAssessment {
        state,
        confirmation_score,
        recent_event_count,
        summary,
        confirmed_signals,
        pending_gaps,
        recent_events,
    }
}

pub fn build_backtest_summary(
    backtests: &[BacktestScenarioSummary],
    rolling_audit: Option<&BacktestRollingAudit>,
) -> BacktestPerformanceSummary {
    let rolling_audit = rolling_audit.cloned().unwrap_or_else(empty_rolling_audit);
    if backtests.is_empty() {
        return BacktestPerformanceSummary {
            scenario_count: 0,
            real_scenario_count: 0,
            fallback_scenario_count: 0,
            structural_warning_rate: 0.0,
            timely_warning_rate: 0.0,
            missed_rate: 1.0,
            avg_structural_lead_time_days: None,
            avg_lead_time_days: None,
            median_lead_time_days: None,
            total_false_positive_count: 0,
            history_start: None,
            history_end: None,
            rolling_audit,
            summary: "当前没有可用回测场景，不能据此评估 posture 的历史可靠性。".to_string(),
        };
    }

    let scenario_count = backtests.len() as u32;
    let real_scenario_count = backtests
        .iter()
        .filter(|scenario| scenario.signal_source == BacktestSignalSource::RealHistory)
        .count() as u32;
    let fallback_scenario_count = scenario_count.saturating_sub(real_scenario_count);
    let structural_warning_count = backtests
        .iter()
        .filter(|scenario| scenario.lead_time_days.unwrap_or_default() >= 7)
        .count() as u32;
    let timely_count = backtests
        .iter()
        .filter(|scenario| {
            !scenario.missed && scenario.actionable_lead_time_days.unwrap_or_default() >= 7
        })
        .count() as u32;
    let missed_count = backtests.iter().filter(|scenario| scenario.missed).count() as u32;
    let mut structural_lead_times = backtests
        .iter()
        .filter_map(|scenario| scenario.lead_time_days.map(|days| days as f64))
        .collect::<Vec<_>>();
    structural_lead_times.sort_by(|left, right| left.total_cmp(right));
    let mut lead_times = backtests
        .iter()
        .filter_map(|scenario| scenario.actionable_lead_time_days.map(|days| days as f64))
        .collect::<Vec<_>>();
    lead_times.sort_by(|left, right| left.total_cmp(right));
    let avg_structural_lead_time_days = (!structural_lead_times.is_empty()).then(|| {
        round1(structural_lead_times.iter().sum::<f64>() / structural_lead_times.len() as f64)
    });
    let avg_lead_time_days = (!lead_times.is_empty())
        .then(|| round1(lead_times.iter().sum::<f64>() / lead_times.len() as f64));
    let median_lead_time_days = if lead_times.is_empty() {
        None
    } else {
        Some(round1(lead_times[lead_times.len() / 2]))
    };
    let total_false_positive_count = backtests
        .iter()
        .map(|scenario| scenario.false_positive_count)
        .sum();
    let structural_warning_rate = round3(structural_warning_count as f64 / scenario_count as f64);
    let timely_warning_rate = round3(timely_count as f64 / scenario_count as f64);
    let missed_rate = round3(missed_count as f64 / scenario_count as f64);
    let history_start = backtests
        .iter()
        .filter_map(|scenario| scenario.history_start)
        .min();
    let history_end = backtests
        .iter()
        .filter_map(|scenario| scenario.history_end)
        .max();
    let summary = if fallback_scenario_count > 0 {
        format!(
            "当前回测共列出 {} 个危机样本，其中 {} 个来自本地真实历史，{} 个仍是模板参考；结构性抬升至少提前 7 天出现的比例约为 {:.0}%，可执行预警至少提前 7 天出现的比例约为 {:.0}%。",
            scenario_count,
            real_scenario_count,
            fallback_scenario_count,
            structural_warning_rate * 100.0,
            timely_warning_rate * 100.0
        )
    } else {
        format!(
            "当前回测覆盖 {} 个真实危机样本；结构性抬升至少提前 7 天出现的比例约为 {:.0}%，可执行预警至少提前 7 天出现的比例约为 {:.0}%。",
            scenario_count,
            structural_warning_rate * 100.0,
            timely_warning_rate * 100.0
        )
    };

    BacktestPerformanceSummary {
        scenario_count,
        real_scenario_count,
        fallback_scenario_count,
        structural_warning_rate,
        timely_warning_rate,
        missed_rate,
        avg_structural_lead_time_days,
        avg_lead_time_days,
        median_lead_time_days,
        total_false_positive_count,
        history_start,
        history_end,
        rolling_audit,
        summary,
    }
}

fn empty_rolling_audit() -> BacktestRollingAudit {
    BacktestRollingAudit {
        history_point_count: 0,
        actionable_signal_count: 0,
        pre_crisis_signal_count: 0,
        in_crisis_signal_count: 0,
        stress_window_signal_count: 0,
        false_positive_signal_count: 0,
        false_positive_episode_count: 0,
        longest_false_positive_episode_days: 0,
        actionable_precision: 0.0,
        classified_episodes: Vec::new(),
        summary: "当前尚未生成全历史滚动审计结果。".to_string(),
    }
}

fn build_historical_analogs(
    snapshot: &RiskSnapshot,
    probabilities: &ProbabilityBlock,
    external_shock_score: f64,
    backtests: &[BacktestScenarioSummary],
    thresholds: ProbabilityActionThresholds,
) -> Vec<HistoricalAnalog> {
    let mut analogs = backtests
        .iter()
        .map(|scenario| {
            let score_distance = (snapshot.overall_score - scenario.max_score).abs();
            let lead_reference = if probabilities.p_5d >= thresholds.defend_p5d
                || probabilities.p_20d >= thresholds.hedge_p20d
            {
                scenario.actionable_lead_time_days.or(scenario.lead_time_days)
            } else {
                scenario.lead_time_days.or(scenario.actionable_lead_time_days)
            };
            let lead_distance = scenario
                .actionable_lead_time_days
                .or(lead_reference)
                .map(|days| ((probabilities.p_20d * 100.0) - days as f64).abs())
                .unwrap_or(35.0);
            let fallback_penalty = match scenario.signal_source {
                BacktestSignalSource::RealHistory => 0.0,
                BacktestSignalSource::FallbackTemplate => 8.0,
            };
            let similarity_score = (100.0 - score_distance * 1.2 - lead_distance * 0.35
                + external_shock_score * 0.08
                - fallback_penalty)
                .clamp(18.0, 96.0);
            HistoricalAnalog {
                scenario_id: scenario.scenario_id.clone(),
                name: scenario.name.clone(),
                similarity_score: round1(similarity_score),
                reference_phase: if probabilities.p_5d >= thresholds.defend_p5d {
                    "acute_window".to_string()
                } else if probabilities.p_20d >= thresholds.hedge_p20d {
                    "pre_break".to_string()
                } else {
                    "fragile_build_up".to_string()
                },
                note: match scenario.signal_source {
                    BacktestSignalSource::RealHistory => match (
                        scenario.lead_time_days,
                        scenario.actionable_lead_time_days,
                    ) {
                        (Some(structural), Some(actionable)) => format!(
                            "{} 的真实历史里，结构性抬升约领先 {} 天，可执行预警约领先 {} 天。",
                            scenario.name, structural, actionable
                        ),
                        (Some(structural), None) => format!(
                            "{} 的真实历史里，结构性抬升约领先 {} 天，但危机前未形成足够强的可执行预警。",
                            scenario.name, structural
                        ),
                        (None, Some(actionable)) => format!(
                            "{} 的真实历史里，约领先 {} 天进入可执行预警，但没有更早的稳定结构抬升。",
                            scenario.name, actionable
                        ),
                        (None, None) => format!(
                            "{} 的真实历史里，危机前没有形成稳定的结构或动作级预警。",
                            scenario.name
                        ),
                    },
                    BacktestSignalSource::FallbackTemplate => {
                        format!("当前分数与 {} 的参考模板较接近；该样本尚未由本地历史库完整覆盖。", scenario.name)
                    }
                },
                peak_score: scenario.max_score,
                lead_time_days: scenario.lead_time_days,
                actionable_lead_time_days: scenario.actionable_lead_time_days,
            }
        })
        .collect::<Vec<_>>();
    analogs.sort_by(|left, right| right.similarity_score.total_cmp(&left.similarity_score));
    analogs.truncate(3);
    analogs
}

fn build_data_trust(
    snapshot: &RiskSnapshot,
    indicator_risks: &[IndicatorRisk],
    has_jpy_data: bool,
) -> DataTrust {
    let (core_total, core_present) = coverage_by_group(indicator_risks, |risk| {
        !is_external_or_event(risk.indicator.dimension)
    });
    let (trigger_total, trigger_present) = coverage_by_group(indicator_risks, |risk| {
        matches!(
            risk.indicator.dimension,
            RiskDimension::MarketStress
                | RiskDimension::LiquidityFunding
                | RiskDimension::EventsSentiment
        )
    });
    let (external_total, external_present) = coverage_by_group(indicator_risks, |risk| {
        risk.indicator.dimension == RiskDimension::ExternalSector
            || risk.indicator.indicator_id.starts_with("us_external_")
    });

    let core_feature_coverage = ratio(core_present, core_total);
    let trigger_feature_coverage = ratio(trigger_present, trigger_total);
    let external_feature_coverage = if external_total == 0 {
        if has_jpy_data {
            1.0
        } else {
            0.0
        }
    } else {
        ratio(external_present, external_total)
    };
    let coverage_score = round3(
        (core_feature_coverage * 0.45
            + trigger_feature_coverage * 0.35
            + external_feature_coverage * 0.2)
            .clamp(0.0, 1.0),
    );

    let mut warnings = Vec::new();
    if snapshot.data_quality_summary.prototype_source_count > 0 {
        warnings.push("部分事件或新闻数据仍是原型源，不能单独触发强结论。".to_string());
    }
    if snapshot.data_quality_summary.stale_indicator_count > 0 {
        warnings.push("存在滞后数据，短期概率需要保守解释。".to_string());
    }
    if !has_jpy_data {
        warnings.push("JPY carry 模块缺少 USDJPY 历史数据，外部冲击识别能力受限。".to_string());
    }
    if snapshot.data_quality_summary.blocked_indicator_count > 0 {
        warnings.push("存在被阻断的核心指标，建议先补齐数据再做强动作。".to_string());
    }

    DataTrust {
        coverage_score,
        core_feature_coverage: round3(core_feature_coverage),
        trigger_feature_coverage: round3(trigger_feature_coverage),
        external_feature_coverage: round3(external_feature_coverage),
        quality_grade: snapshot.data_quality_summary.grade,
        data_quality_summary: snapshot.data_quality_summary.clone(),
        warnings,
    }
}

fn build_jpy_carry_snapshot(
    snapshot: &RiskSnapshot,
    indicator_risks: &[IndicatorRisk],
    observations: &[Observation],
) -> JpyCarrySnapshot {
    let usdjpy_history = observations_for_indicator(
        observations,
        "us_external_usdjpy_level",
        snapshot.as_of_date,
    );
    let usdjpy_level = usdjpy_history.last().map(|observation| observation.value);
    let jp_call_rate_history =
        observations_for_indicator(observations, "jp_rates_call_rate", snapshot.as_of_date);
    let jp_call_rate = jp_call_rate_history
        .last()
        .map(|observation| observation.value);
    let us_short_rate_history =
        observations_for_indicator(observations, "us_liquidity_effr", snapshot.as_of_date);
    let us_short_rate = us_short_rate_history
        .last()
        .map(|observation| observation.value);
    let us_jp_short_rate_diff = match (us_short_rate, jp_call_rate) {
        (Some(us), Some(jp)) => Some(us - jp),
        _ => None,
    };
    let change_5d = difference_from_tail(&usdjpy_history, 5);
    let change_20d = difference_from_tail(&usdjpy_history, 20);
    let realized_vol_20d = realized_volatility(&usdjpy_history, 20);
    let vix_score = find_indicator_score(indicator_risks, "us_market_vix_close");
    let credit_score = find_indicator_score(indicator_risks, "us_credit_high_yield_oas");
    let direction_reversal_score = change_5d
        .map(|change| (change.abs() * 4.0).clamp(0.0, 100.0))
        .unwrap_or(0.0);
    let vol_score = realized_vol_20d
        .map(|value| (value * 8.0).clamp(0.0, 100.0))
        .unwrap_or(0.0);
    let funding_pressure_score = round1(
        us_jp_short_rate_diff
            .map(|diff| (diff * 12.0).clamp(0.0, 100.0))
            .unwrap_or(18.0),
    );
    let vix_coupling_score =
        round1((direction_reversal_score * 0.35 + vix_score * 0.65).clamp(0.0, 100.0));
    let credit_coupling_score = round1((vol_score * 0.35 + credit_score * 0.65).clamp(0.0, 100.0));
    let score = round1(
        (direction_reversal_score * 0.25
            + vol_score * 0.22
            + funding_pressure_score * 0.18
            + vix_coupling_score * 0.2
            + credit_coupling_score * 0.15)
            .clamp(0.0, 100.0),
    );

    let state = if score >= 75.0 {
        JpyCarryState::Unwind
    } else if score >= 58.0 {
        JpyCarryState::Stress
    } else if score >= 35.0 {
        JpyCarryState::Building
    } else {
        JpyCarryState::Quiet
    };

    let reason = match state {
        JpyCarryState::Quiet => {
            if let Some(diff) = us_jp_short_rate_diff {
                format!("USDJPY 波动与美股/信用压力暂未形成明显共振，美日短端利差约 {diff:.2}%。")
            } else {
                "USDJPY 波动与美股/信用压力暂未形成明显共振。".to_string()
            }
        }
        JpyCarryState::Building => {
            if let Some(diff) = us_jp_short_rate_diff {
                format!("USDJPY 开始波动，美日短端利差约 {diff:.2}%，套息吸引力仍在，但还没有与信用和波动率形成全面同步。")
            } else {
                "USDJPY 开始波动，但还没有与信用和波动率形成全面同步。".to_string()
            }
        }
        JpyCarryState::Stress => {
            if let Some(diff) = us_jp_short_rate_diff {
                format!("USDJPY 波动已与 VIX 或信用利差形成联动，美日短端利差约 {diff:.2}%，外部放大器正在增强。")
            } else {
                "USDJPY 波动已与 VIX 或信用利差形成联动，外部放大器正在增强。".to_string()
            }
        }
        JpyCarryState::Unwind => {
            if let Some(diff) = us_jp_short_rate_diff {
                format!("JPY carry 平仓压力进入高位，美日短端利差约 {diff:.2}%，可能把数周风险压缩到数日。")
            } else {
                "JPY carry 平仓压力进入高位，可能把数周风险压缩到数日。".to_string()
            }
        }
    };

    JpyCarrySnapshot {
        state,
        score,
        usdjpy_level,
        jp_call_rate: round_option(jp_call_rate, 3),
        us_short_rate: round_option(us_short_rate, 3),
        us_jp_short_rate_diff: round_option(us_jp_short_rate_diff, 3),
        change_5d: round_option(change_5d, 3),
        change_20d: round_option(change_20d, 3),
        realized_vol_20d: round_option(realized_vol_20d, 3),
        funding_pressure_score,
        vix_coupling_score,
        credit_coupling_score,
        reason,
    }
}

fn build_relief_drivers(indicator_risks: &[IndicatorRisk]) -> Vec<RiskContributor> {
    let mut rows = indicator_risks
        .iter()
        .filter(|risk| risk.latest_observation.is_some())
        .map(|risk| RiskContributor {
            indicator_id: risk.indicator.indicator_id.clone(),
            display_name: risk.indicator.display_name.clone(),
            dimension: risk.indicator.dimension,
            score: round1(risk.score),
            contribution: round1((100.0 - risk.score) * 0.2),
            explanation: format!("{} 当前处于相对低压区。", risk.indicator.display_name),
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.score.total_cmp(&right.score));
    rows.truncate(3);
    rows
}

fn build_conviction_score(
    snapshot: &RiskSnapshot,
    data_trust: &DataTrust,
    breadth_score: f64,
) -> f64 {
    let breadth_component = scaled_pressure(breadth_score, 32.0, 35.0);
    let quality_component = data_trust.coverage_score;
    let agreement_component = if snapshot.structural_score >= 55.0 && snapshot.trigger_score >= 55.0
    {
        0.18
    } else {
        0.05
    };
    round3(
        (quality_component * 0.48 + breadth_component * 0.34 + agreement_component)
            .clamp(0.12, 0.95),
    )
}

fn high_risk_breadth(snapshot: &RiskSnapshot) -> f64 {
    let total = snapshot.dimensions.len();
    if total == 0 {
        return 0.0;
    }
    let elevated = snapshot
        .dimensions
        .iter()
        .filter(|dimension| dimension.score >= 60.0)
        .count();
    elevated as f64 / total as f64 * 100.0
}

fn observations_for_indicator<'a>(
    observations: &'a [Observation],
    indicator_id: &str,
    as_of_date: NaiveDate,
) -> Vec<&'a Observation> {
    let mut rows = observations
        .iter()
        .filter(|observation| observation.indicator_id == indicator_id)
        .filter(|observation| observation.as_of_date <= as_of_date)
        .collect::<Vec<_>>();
    rows.sort_by_key(|observation| observation.as_of_date);
    rows
}

fn difference_from_tail(observations: &[&Observation], lookback: usize) -> Option<f64> {
    let latest = observations.last()?;
    let previous_index = observations.len().checked_sub(lookback + 1)?;
    let previous = observations.get(previous_index)?;
    Some(latest.value - previous.value)
}

fn realized_volatility(observations: &[&Observation], window: usize) -> Option<f64> {
    let start = observations.len().saturating_sub(window + 1);
    let slice = observations.get(start..)?;
    if slice.len() < 3 {
        return None;
    }
    let changes = slice
        .windows(2)
        .filter_map(|pair| {
            let previous = pair.first()?.value;
            let current = pair.get(1)?.value;
            (previous.abs() > f64::EPSILON).then_some((current - previous) / previous.abs())
        })
        .collect::<Vec<_>>();
    if changes.len() < 2 {
        return None;
    }
    let mean = changes.iter().sum::<f64>() / changes.len() as f64;
    let variance = changes
        .iter()
        .map(|change| (change - mean).powi(2))
        .sum::<f64>()
        / changes.len() as f64;
    Some(variance.sqrt())
}

fn coverage_by_group<F>(indicator_risks: &[IndicatorRisk], predicate: F) -> (usize, usize)
where
    F: Fn(&IndicatorRisk) -> bool,
{
    indicator_risks.iter().filter(|risk| predicate(risk)).fold(
        (0_usize, 0_usize),
        |(total, present), risk| {
            (
                total + 1,
                present + usize::from(risk.latest_observation.is_some()),
            )
        },
    )
}

fn is_external_or_event(dimension: RiskDimension) -> bool {
    matches!(
        dimension,
        RiskDimension::ExternalSector | RiskDimension::EventsSentiment
    )
}

fn find_indicator_score(indicator_risks: &[IndicatorRisk], indicator_id: &str) -> f64 {
    indicator_risks
        .iter()
        .find(|risk| risk.indicator.indicator_id == indicator_id)
        .map(|risk| risk.score)
        .unwrap_or(0.0)
}

fn ratio(present: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        present as f64 / total as f64
    }
}

fn scaled_pressure(score: f64, center: f64, width: f64) -> f64 {
    ((score - center) / width).clamp(0.0, 1.0)
}

fn clamp_probability(value: f64) -> f64 {
    value.clamp(0.0, 0.93)
}

fn posture_label(posture: DecisionPosture) -> &'static str {
    match posture {
        DecisionPosture::Normal => "normal",
        DecisionPosture::Prepare => "prepare",
        DecisionPosture::Hedge => "hedge",
        DecisionPosture::Defend => "defend",
    }
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

fn round3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

fn format_probability_threshold(value: f64) -> String {
    format!("{value:.2}")
}

fn round6(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

fn round_option(value: Option<f64>, decimals: i32) -> Option<f64> {
    let scale = 10_f64.powi(decimals);
    value.map(|value| (value * scale).round() / scale)
}

#[cfg(test)]
mod tests {
    use super::{
        actionability_confidence_from_probability, build_posture_guidance,
        build_time_to_risk_bucket, fuse_actionability_confidence, ProbabilityActionThresholds,
    };
    use chrono::{NaiveDate, Utc};
    use fc_domain::{
        ActionabilityLevel, DataQualitySummary, DataTrust, EventAssessment, EventConfirmationState,
        JpyCarrySnapshot, JpyCarryState, ProbabilityBlock, QualityGrade, RiskLevel, RiskSnapshot,
        TimeToRiskBucket, UserRiskPreferences, UserRiskProfile,
    };

    fn neutral_preferences() -> UserRiskPreferences {
        UserRiskPreferences {
            profile: UserRiskProfile::Neutral,
            cash_floor_pct: 15.0,
            max_equity_cap_pct: 70.0,
            max_leverage_pct: 100.0,
            option_overlay_preference_pct: 5.0,
            allow_aggressive_reentry: false,
            note: "test".to_string(),
        }
    }

    fn test_data_trust(quality_grade: QualityGrade) -> DataTrust {
        DataTrust {
            coverage_score: 0.98,
            core_feature_coverage: 1.0,
            trigger_feature_coverage: 0.95,
            external_feature_coverage: 0.95,
            quality_grade,
            data_quality_summary: DataQualitySummary {
                overall_score: 91.0,
                grade: quality_grade,
                stale_indicator_count: 0,
                low_quality_indicator_count: 0,
                prototype_source_count: 0,
                blocked_indicator_count: 0,
            },
            warnings: Vec::new(),
        }
    }

    fn quiet_event_assessment(confirmation_score: f64) -> EventAssessment {
        EventAssessment {
            state: EventConfirmationState::Quiet,
            confirmation_score,
            recent_event_count: 0,
            summary: "test".to_string(),
            confirmed_signals: Vec::new(),
            pending_gaps: Vec::new(),
            recent_events: Vec::new(),
        }
    }

    fn quiet_jpy_carry(funding_pressure_score: f64) -> JpyCarrySnapshot {
        JpyCarrySnapshot {
            state: JpyCarryState::Quiet,
            score: 10.0,
            usdjpy_level: Some(150.0),
            jp_call_rate: Some(0.25),
            us_short_rate: Some(4.0),
            us_jp_short_rate_diff: Some(3.75),
            change_5d: Some(0.2),
            change_20d: Some(1.0),
            realized_vol_20d: Some(0.01),
            funding_pressure_score,
            vix_coupling_score: 15.0,
            credit_coupling_score: 15.0,
            reason: "test".to_string(),
        }
    }

    fn stressed_jpy_carry(score: f64, funding_pressure_score: f64) -> JpyCarrySnapshot {
        JpyCarrySnapshot {
            state: JpyCarryState::Stress,
            score,
            usdjpy_level: Some(159.0),
            jp_call_rate: Some(0.10),
            us_short_rate: Some(5.25),
            us_jp_short_rate_diff: Some(5.15),
            change_5d: Some(2.5),
            change_20d: Some(4.2),
            realized_vol_20d: Some(0.11),
            funding_pressure_score,
            vix_coupling_score: 52.0,
            credit_coupling_score: 48.0,
            reason: "test".to_string(),
        }
    }

    #[test]
    fn actionability_confidence_requires_margin_above_decision_threshold() {
        assert_eq!(actionability_confidence_from_probability(0.05, 0.05), 0.0);
        assert!(actionability_confidence_from_probability(0.20, 0.05) < 0.05);
        assert!(actionability_confidence_from_probability(0.55, 0.05) > 0.25);
    }

    #[test]
    fn fused_actionability_suppresses_high_confidence_without_context() {
        let snapshot = RiskSnapshot {
            as_of_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            entity_id: "us".to_string(),
            market_scope: "financial_system".to_string(),
            overall_score: 33.3,
            overall_level: RiskLevel::Watch,
            structural_score: 39.7,
            trigger_score: 25.4,
            level_reason: "test".to_string(),
            dimensions: Vec::new(),
            top_contributors: Vec::new(),
            data_quality_summary: DataQualitySummary {
                overall_score: 91.0,
                grade: QualityGrade::A,
                stale_indicator_count: 0,
                low_quality_indicator_count: 0,
                prototype_source_count: 0,
                blocked_indicator_count: 0,
            },
            generated_at: Utc::now(),
            method_version: "test".to_string(),
        };
        let probabilities = ProbabilityBlock {
            p_5d: 0.005,
            p_20d: 0.025,
            p_60d: 0.055,
        };
        let thresholds = ProbabilityActionThresholds {
            prepare_p60d: 0.023,
            hedge_p20d: 0.008,
            defend_p5d: 0.005,
        };

        let prepare = fuse_actionability_confidence(
            ActionabilityLevel::Prepare,
            0.954,
            &probabilities,
            &snapshot,
            29.8,
            thresholds,
        );
        let hedge = fuse_actionability_confidence(
            ActionabilityLevel::Hedge,
            0.812,
            &probabilities,
            &snapshot,
            29.8,
            thresholds,
        );

        assert!(prepare < 0.10);
        assert!(hedge < 0.10);
    }

    #[test]
    fn fused_actionability_preserves_supported_prepare_context() {
        let snapshot = RiskSnapshot {
            as_of_date: NaiveDate::from_ymd_opt(2020, 2, 20).unwrap(),
            entity_id: "us".to_string(),
            market_scope: "financial_system".to_string(),
            overall_score: 61.0,
            overall_level: RiskLevel::Stress,
            structural_score: 58.0,
            trigger_score: 54.0,
            level_reason: "test".to_string(),
            dimensions: Vec::new(),
            top_contributors: Vec::new(),
            data_quality_summary: DataQualitySummary {
                overall_score: 91.0,
                grade: QualityGrade::A,
                stale_indicator_count: 0,
                low_quality_indicator_count: 0,
                prototype_source_count: 0,
                blocked_indicator_count: 0,
            },
            generated_at: Utc::now(),
            method_version: "test".to_string(),
        };
        let probabilities = ProbabilityBlock {
            p_5d: 0.018,
            p_20d: 0.052,
            p_60d: 0.118,
        };
        let thresholds = ProbabilityActionThresholds {
            prepare_p60d: 0.023,
            hedge_p20d: 0.008,
            defend_p5d: 0.005,
        };

        let prepare = fuse_actionability_confidence(
            ActionabilityLevel::Prepare,
            0.82,
            &probabilities,
            &snapshot,
            52.0,
            thresholds,
        );

        assert!(prepare > 0.35);
    }

    #[test]
    fn time_to_risk_bucket_requires_confirmation_for_months_bucket() {
        let bucket = build_time_to_risk_bucket(
            &ProbabilityBlock {
                p_5d: 0.004,
                p_20d: 0.018,
                p_60d: 0.14,
            },
            None,
            59.0,
            40.0,
            44.0,
            32.0,
            &quiet_jpy_carry(20.0),
            ProbabilityActionThresholds {
                prepare_p60d: 0.12,
                hedge_p20d: 0.06,
                defend_p5d: 0.05,
            },
        );

        assert_eq!(bucket, TimeToRiskBucket::Normal);
    }

    #[test]
    fn time_to_risk_bucket_allows_months_when_probability_and_context_align() {
        let bucket = build_time_to_risk_bucket(
            &ProbabilityBlock {
                p_5d: 0.004,
                p_20d: 0.05,
                p_60d: 0.14,
            },
            None,
            59.0,
            47.0,
            52.0,
            38.0,
            &quiet_jpy_carry(20.0),
            ProbabilityActionThresholds {
                prepare_p60d: 0.12,
                hedge_p20d: 0.06,
                defend_p5d: 0.05,
            },
        );

        assert_eq!(bucket, TimeToRiskBucket::Months);
    }

    #[test]
    fn posture_guidance_blocks_prepare_external_without_probability_companion() {
        let snapshot = RiskSnapshot {
            as_of_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            entity_id: "us".to_string(),
            market_scope: "financial_system".to_string(),
            overall_score: 57.0,
            overall_level: RiskLevel::Stress,
            structural_score: 54.0,
            trigger_score: 32.0,
            level_reason: "test".to_string(),
            dimensions: Vec::new(),
            top_contributors: Vec::new(),
            data_quality_summary: DataQualitySummary {
                overall_score: 91.0,
                grade: QualityGrade::A,
                stale_indicator_count: 0,
                low_quality_indicator_count: 0,
                prototype_source_count: 0,
                blocked_indicator_count: 0,
            },
            generated_at: Utc::now(),
            method_version: "test".to_string(),
        };
        let probabilities = ProbabilityBlock {
            p_5d: 0.004,
            p_20d: 0.018,
            p_60d: 0.010,
        };
        let posture = build_posture_guidance(
            &snapshot,
            &probabilities,
            None,
            0.60,
            &test_data_trust(QualityGrade::A),
            56.0,
            30.0,
            &[],
            &quiet_jpy_carry(20.0),
            &quiet_event_assessment(20.0),
            &neutral_preferences(),
            ProbabilityActionThresholds {
                prepare_p60d: 0.12,
                hedge_p20d: 0.06,
                defend_p5d: 0.05,
            },
        );

        assert_eq!(posture.posture, fc_domain::DecisionPosture::Normal);
        assert!(posture.trigger_codes.is_empty());
        assert!(posture.blocker_codes.is_empty());
    }

    #[test]
    fn posture_guidance_emits_prepare_external_structural_clause_when_probability_confirms() {
        let snapshot = RiskSnapshot {
            as_of_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            entity_id: "us".to_string(),
            market_scope: "financial_system".to_string(),
            overall_score: 57.0,
            overall_level: RiskLevel::Stress,
            structural_score: 55.0,
            trigger_score: 46.0,
            level_reason: "test".to_string(),
            dimensions: Vec::new(),
            top_contributors: Vec::new(),
            data_quality_summary: DataQualitySummary {
                overall_score: 91.0,
                grade: QualityGrade::A,
                stale_indicator_count: 0,
                low_quality_indicator_count: 0,
                prototype_source_count: 0,
                blocked_indicator_count: 0,
            },
            generated_at: Utc::now(),
            method_version: "test".to_string(),
        };
        let probabilities = ProbabilityBlock {
            p_5d: 0.004,
            p_20d: 0.05,
            p_60d: 0.010,
        };
        let posture = build_posture_guidance(
            &snapshot,
            &probabilities,
            None,
            0.60,
            &test_data_trust(QualityGrade::A),
            59.0,
            38.0,
            &[],
            &quiet_jpy_carry(20.0),
            &quiet_event_assessment(42.0),
            &neutral_preferences(),
            ProbabilityActionThresholds {
                prepare_p60d: 0.12,
                hedge_p20d: 0.06,
                defend_p5d: 0.05,
            },
        );

        assert_eq!(posture.posture, fc_domain::DecisionPosture::Prepare);
        assert_eq!(
            posture.trigger_codes,
            vec!["prepare_external_structural".to_string()]
        );
        assert!(posture.blocker_codes.is_empty());
    }

    #[test]
    fn posture_guidance_marks_quality_blocked_hedge_clause() {
        let snapshot = RiskSnapshot {
            as_of_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            entity_id: "us".to_string(),
            market_scope: "financial_system".to_string(),
            overall_score: 49.0,
            overall_level: RiskLevel::Watch,
            structural_score: 44.0,
            trigger_score: 54.0,
            level_reason: "test".to_string(),
            dimensions: Vec::new(),
            top_contributors: Vec::new(),
            data_quality_summary: DataQualitySummary {
                overall_score: 62.0,
                grade: QualityGrade::F,
                stale_indicator_count: 2,
                low_quality_indicator_count: 0,
                prototype_source_count: 0,
                blocked_indicator_count: 1,
            },
            generated_at: Utc::now(),
            method_version: "test".to_string(),
        };
        let probabilities = ProbabilityBlock {
            p_5d: 0.01,
            p_20d: 0.07,
            p_60d: 0.10,
        };
        let posture = build_posture_guidance(
            &snapshot,
            &probabilities,
            None,
            0.58,
            &test_data_trust(QualityGrade::F),
            52.0,
            41.0,
            &[],
            &quiet_jpy_carry(20.0),
            &quiet_event_assessment(42.0),
            &neutral_preferences(),
            ProbabilityActionThresholds {
                prepare_p60d: 0.12,
                hedge_p20d: 0.06,
                defend_p5d: 0.05,
            },
        );

        assert_eq!(posture.posture, fc_domain::DecisionPosture::Normal);
        assert!(posture.trigger_codes.is_empty());
        assert_eq!(
            posture.blocker_codes,
            vec!["quality_blocked_hedge".to_string()]
        );
    }

    #[test]
    fn posture_guidance_requires_multi_signal_context_for_hedge_clause() {
        let snapshot = RiskSnapshot {
            as_of_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            entity_id: "us".to_string(),
            market_scope: "financial_system".to_string(),
            overall_score: 48.0,
            overall_level: RiskLevel::Watch,
            structural_score: 46.0,
            trigger_score: 53.0,
            level_reason: "test".to_string(),
            dimensions: Vec::new(),
            top_contributors: Vec::new(),
            data_quality_summary: DataQualitySummary {
                overall_score: 88.0,
                grade: QualityGrade::A,
                stale_indicator_count: 0,
                low_quality_indicator_count: 0,
                prototype_source_count: 0,
                blocked_indicator_count: 0,
            },
            generated_at: Utc::now(),
            method_version: "test".to_string(),
        };
        let probabilities = ProbabilityBlock {
            p_5d: 0.01,
            p_20d: 0.18,
            p_60d: 0.03,
        };
        let posture = build_posture_guidance(
            &snapshot,
            &probabilities,
            None,
            0.58,
            &test_data_trust(QualityGrade::A),
            42.0,
            34.0,
            &[],
            &quiet_jpy_carry(18.0),
            &quiet_event_assessment(25.0),
            &neutral_preferences(),
            ProbabilityActionThresholds {
                prepare_p60d: 0.12,
                hedge_p20d: 0.06,
                defend_p5d: 0.05,
            },
        );

        assert_eq!(posture.posture, fc_domain::DecisionPosture::Normal);
        assert!(posture.trigger_codes.is_empty());
    }

    #[test]
    fn posture_guidance_allows_hedge_when_short_and_medium_horizon_context_align() {
        let snapshot = RiskSnapshot {
            as_of_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            entity_id: "us".to_string(),
            market_scope: "financial_system".to_string(),
            overall_score: 56.0,
            overall_level: RiskLevel::Stress,
            structural_score: 52.0,
            trigger_score: 54.0,
            level_reason: "test".to_string(),
            dimensions: Vec::new(),
            top_contributors: Vec::new(),
            data_quality_summary: DataQualitySummary {
                overall_score: 90.0,
                grade: QualityGrade::A,
                stale_indicator_count: 0,
                low_quality_indicator_count: 0,
                prototype_source_count: 0,
                blocked_indicator_count: 0,
            },
            generated_at: Utc::now(),
            method_version: "test".to_string(),
        };
        let probabilities = ProbabilityBlock {
            p_5d: 0.01,
            p_20d: 0.18,
            p_60d: 0.08,
        };
        let posture = build_posture_guidance(
            &snapshot,
            &probabilities,
            None,
            0.60,
            &test_data_trust(QualityGrade::A),
            52.0,
            42.0,
            &[],
            &quiet_jpy_carry(20.0),
            &quiet_event_assessment(45.0),
            &neutral_preferences(),
            ProbabilityActionThresholds {
                prepare_p60d: 0.12,
                hedge_p20d: 0.06,
                defend_p5d: 0.05,
            },
        );

        assert_eq!(posture.posture, fc_domain::DecisionPosture::Hedge);
        assert_eq!(
            posture.trigger_codes,
            vec!["hedge_p20d_context".to_string()]
        );
    }

    #[test]
    fn posture_guidance_blocks_hedge_when_short_horizon_lacks_overall_confirmation() {
        let snapshot = RiskSnapshot {
            as_of_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            entity_id: "us".to_string(),
            market_scope: "financial_system".to_string(),
            overall_score: 56.5,
            overall_level: RiskLevel::Stress,
            structural_score: 52.0,
            trigger_score: 54.0,
            level_reason: "test".to_string(),
            dimensions: Vec::new(),
            top_contributors: Vec::new(),
            data_quality_summary: DataQualitySummary {
                overall_score: 90.0,
                grade: QualityGrade::A,
                stale_indicator_count: 0,
                low_quality_indicator_count: 0,
                prototype_source_count: 0,
                blocked_indicator_count: 0,
            },
            generated_at: Utc::now(),
            method_version: "test".to_string(),
        };
        let probabilities = ProbabilityBlock {
            p_5d: 0.01,
            p_20d: 0.18,
            p_60d: 0.08,
        };
        let posture = build_posture_guidance(
            &snapshot,
            &probabilities,
            None,
            0.60,
            &test_data_trust(QualityGrade::A),
            37.0,
            42.0,
            &[],
            &quiet_jpy_carry(20.0),
            &quiet_event_assessment(25.0),
            &neutral_preferences(),
            ProbabilityActionThresholds {
                prepare_p60d: 0.12,
                hedge_p20d: 0.06,
                defend_p5d: 0.05,
            },
        );

        assert_eq!(posture.posture, fc_domain::DecisionPosture::Normal);
        assert!(posture.trigger_codes.is_empty());
    }

    #[test]
    fn posture_guidance_blocks_prepare_carry_without_noncarry_confirmation() {
        let snapshot = RiskSnapshot {
            as_of_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            entity_id: "us".to_string(),
            market_scope: "financial_system".to_string(),
            overall_score: 56.5,
            overall_level: RiskLevel::Stress,
            structural_score: 57.0,
            trigger_score: 34.0,
            level_reason: "test".to_string(),
            dimensions: Vec::new(),
            top_contributors: Vec::new(),
            data_quality_summary: DataQualitySummary {
                overall_score: 90.0,
                grade: QualityGrade::A,
                stale_indicator_count: 0,
                low_quality_indicator_count: 0,
                prototype_source_count: 0,
                blocked_indicator_count: 0,
            },
            generated_at: Utc::now(),
            method_version: "test".to_string(),
        };
        let probabilities = ProbabilityBlock {
            p_5d: 0.01,
            p_20d: 0.03,
            p_60d: 0.10,
        };
        let posture = build_posture_guidance(
            &snapshot,
            &probabilities,
            None,
            0.60,
            &test_data_trust(QualityGrade::A),
            41.0,
            32.0,
            &[],
            &stressed_jpy_carry(60.0, 52.0),
            &quiet_event_assessment(30.0),
            &neutral_preferences(),
            ProbabilityActionThresholds {
                prepare_p60d: 0.12,
                hedge_p20d: 0.06,
                defend_p5d: 0.05,
            },
        );

        assert_eq!(posture.posture, fc_domain::DecisionPosture::Normal);
        assert!(posture.trigger_codes.is_empty());
    }
}

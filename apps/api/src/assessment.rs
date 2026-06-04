use std::env;

use fc_domain::{
    AlertEvent, AssessmentMethodVersions, AssessmentScores, AssessmentSnapshot,
    BacktestRollingAudit, BacktestScenarioSummary, DataMode, DecisionPosture, IndicatorRisk,
    ModelReleaseRecord, Observation, PostureGuidance, ProbabilityBundle, RiskDimension,
    RiskSnapshot, UserRiskPreferences,
};
use serde::Serialize;

mod context;
mod market_context;
mod posture;
mod probability;

pub(crate) use context::build_backtest_summary;
use context::{
    build_event_assessment, build_historical_analogs, build_key_indicator_statuses,
    build_runtime_metadata,
};
use market_context::{
    build_conviction_score, build_data_trust, build_jpy_carry_snapshot, build_relief_drivers,
    high_risk_breadth,
};
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
    let prepare_reference_p60d = probability_trace
        .probability_diagnostics
        .horizon_overlays
        .iter()
        .find(|horizon| horizon.horizon_days == 60)
        .map(|horizon| horizon.final_probability);
    let active_release = serving_model.map(|context| &context.release);
    let action_thresholds = probability_action_thresholds(serving_model);
    let time_to_risk_bucket = build_time_to_risk_bucket(
        &probabilities,
        prepare_reference_p60d,
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
        prepare_reference_p60d,
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
            probability_diagnostics: probability_trace.probability_diagnostics.clone(),
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
    fn time_to_risk_bucket_ignores_monotonic_only_prepare_crossing() {
        let bucket = build_time_to_risk_bucket(
            &ProbabilityBlock {
                p_5d: 0.004,
                p_20d: 0.09,
                p_60d: 0.14,
            },
            Some(0.09),
            None,
            60.0,
            46.0,
            44.0,
            36.0,
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
    fn posture_guidance_ignores_monotonic_only_prepare_crossing() {
        let snapshot = RiskSnapshot {
            as_of_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            entity_id: "us".to_string(),
            market_scope: "financial_system".to_string(),
            overall_score: 50.0,
            overall_level: RiskLevel::Watch,
            structural_score: 60.0,
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
            p_20d: 0.09,
            p_60d: 0.14,
        };
        let posture = build_posture_guidance(
            &snapshot,
            &probabilities,
            Some(0.09),
            None,
            0.60,
            &test_data_trust(QualityGrade::A),
            44.0,
            36.0,
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

use std::collections::BTreeMap;

use chrono::NaiveDate;
use chrono::Utc;
use fc_domain::{
    ActionabilityBlock, ActionabilityLevel, AlertEvent, AssessmentMethodVersions, AssessmentScores,
    AssessmentSnapshot, BacktestPerformanceSummary, BacktestRollingAudit, BacktestScenarioSummary,
    BacktestSignalSource, DataMode, DataTrust, DecisionPosture, EventAssessment,
    EventConfirmationState, EventSignalSummary, FreshnessStatus, HistoricalAnalog, IndicatorRisk,
    JpyCarrySnapshot, JpyCarryState, KeyIndicatorStatus, LogisticProbabilityModel,
    ModelReleaseRecord, Observation, PlattCalibrationArtifact, PositionGuidance, PostureGuidance,
    ProbabilityBlock, ProbabilityBundle, QualityGrade, RiskContributor, RiskDimension,
    RiskSnapshot, RuntimeMetadata, TimeToRiskBucket, UserRiskPreferences, UserRiskProfile,
    FEATURE_BUCKET_MONTHS_OR_HIGHER, FEATURE_BUCKET_NOW, FEATURE_BUCKET_WEEKS_OR_HIGHER,
    FEATURE_COVERAGE_SCORE, FEATURE_EXTERNAL_DIMENSION_SCORE, FEATURE_EXTERNAL_SHOCK_SCORE,
    FEATURE_FRESHNESS_DELAYED_OR_WORSE, FEATURE_FRESHNESS_STALE_OR_MISSING,
    FEATURE_HEURISTIC_P_20D, FEATURE_HEURISTIC_P_5D, FEATURE_HEURISTIC_P_60D,
    FEATURE_OVERALL_SCORE, FEATURE_STRUCTURAL_SCORE, FEATURE_TRIGGER_SCORE,
    FEATURE_US_BAA_10Y_SPREAD_LEVEL, FEATURE_US_CURVE_10Y2Y_LEVEL, FEATURE_US_FED_FUNDS_LEVEL,
    FEATURE_US_HOUSING_STARTS_LEVEL, FEATURE_US_NFCI_LEVEL, FEATURE_US_STLFSI_LEVEL,
    FEATURE_US_UNEMPLOYMENT_LEVEL, FEATURE_US_USDJPY_CHANGE_20D, FEATURE_US_USDJPY_LEVEL,
    FEATURE_US_VIX_CHANGE_5D, FEATURE_US_VIX_LEVEL,
};

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
            prepare_p60d: FORMAL_MAIN_RUNTIME_PREPARE_P60D_FLOOR,
            hedge_p20d: FORMAL_MAIN_RUNTIME_HEDGE_P20D_FLOOR,
            defend_p5d: FORMAL_MAIN_RUNTIME_DEFEND_P5D_FLOOR,
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
        "runtime_history_v1_20260601|class={release_class}|prepare={:.3}|hedge={:.3}|defend={:.3}",
        thresholds.prepare_p60d, thresholds.hedge_p20d, thresholds.defend_p5d
    )
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

#[derive(Debug, Clone)]
pub struct ProbabilityComputationTrace {
    pub raw_probabilities: ProbabilityBlock,
    pub calibrated_probabilities: ProbabilityBlock,
    pub actionability: ActionabilityBlock,
    pub actionability_enabled: bool,
    pub actionability_model_version: Option<String>,
    pub actionability_calibration_version: Option<String>,
    pub fusion_policy_version: Option<String>,
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

fn build_probabilities(
    snapshot: &RiskSnapshot,
    external_shock_score: f64,
    conviction_score: f64,
    breadth_score: f64,
    data_trust: &DataTrust,
    jpy_carry: &JpyCarrySnapshot,
) -> ProbabilityBlock {
    let structural_pressure = scaled_pressure(snapshot.structural_score, 52.0, 20.0);
    let trigger_pressure = scaled_pressure(snapshot.trigger_score, 55.0, 18.0);
    let external_pressure = scaled_pressure(external_shock_score, 42.0, 18.0);
    let breadth_pressure = scaled_pressure(breadth_score, 38.0, 24.0);
    let carry_funding_pressure = scaled_pressure(jpy_carry.funding_pressure_score, 38.0, 30.0);
    let carry_state_pressure = scaled_pressure(jpy_carry.score, 34.0, 28.0);
    let confidence_penalty = (1.0 - conviction_score) * 0.18;
    let quality_penalty = (1.0 - data_trust.coverage_score) * 0.15;
    let interaction = structural_pressure * trigger_pressure;
    let acute_interaction = trigger_pressure * external_pressure;
    let carry_trigger_interaction = carry_state_pressure * trigger_pressure;

    let p_60d_raw = clamp_probability(
        0.04 + structural_pressure * 0.44
            + trigger_pressure * 0.18
            + external_pressure * 0.08
            + carry_funding_pressure * 0.08
            + breadth_pressure * 0.08
            - quality_penalty * 0.45,
    );
    let p_20d_raw = clamp_probability(
        0.02 + structural_pressure * 0.16
            + trigger_pressure * 0.34
            + external_pressure * 0.14
            + carry_funding_pressure * 0.07
            + interaction * 0.11
            + carry_trigger_interaction * 0.08
            + breadth_pressure * 0.07
            - confidence_penalty * 0.4
            - quality_penalty * 0.2,
    );
    let p_5d = clamp_probability(
        0.01 + trigger_pressure * 0.15
            + external_pressure * 0.16
            + carry_state_pressure * 0.08
            + acute_interaction * 0.18
            + carry_trigger_interaction * 0.12
            + breadth_pressure * 0.05
            - structural_pressure * 0.03
            - confidence_penalty * 0.5
            - quality_penalty * 0.2,
    );
    let p_20d = clamp_probability(p_20d_raw.max((p_5d + 0.03).min(0.93)));
    let p_60d = clamp_probability(p_60d_raw.max((p_20d + 0.05).min(0.93)));

    ProbabilityBlock {
        p_5d: round3(p_5d),
        p_20d: round3(p_20d),
        p_60d: round3(p_60d),
    }
}

fn heuristic_actionability_block(
    snapshot: &RiskSnapshot,
    external_shock_score: f64,
    probabilities: &ProbabilityBlock,
    data_trust: &DataTrust,
    jpy_carry: &JpyCarrySnapshot,
) -> ActionabilityBlock {
    let quality_penalty = (1.0 - data_trust.coverage_score).clamp(0.0, 1.0) * 0.12;
    let prepare = clamp_probability(
        probabilities.p_60d * 0.72
            + scaled_pressure(snapshot.structural_score, 55.0, 18.0) * 0.22
            + scaled_pressure(external_shock_score, 48.0, 18.0) * 0.08
            - quality_penalty,
    );
    let hedge = clamp_probability(
        probabilities.p_20d * 0.74
            + scaled_pressure(snapshot.trigger_score, 52.0, 20.0) * 0.22
            + scaled_pressure(external_shock_score, 50.0, 18.0) * 0.10
            + scaled_pressure(jpy_carry.score, 58.0, 18.0) * 0.06
            - quality_penalty,
    );
    let defend = clamp_probability(
        probabilities.p_5d * 0.78
            + scaled_pressure(snapshot.trigger_score, 60.0, 18.0) * 0.18
            + scaled_pressure(external_shock_score, 55.0, 18.0) * 0.10
            + scaled_pressure(jpy_carry.funding_pressure_score, 55.0, 16.0) * 0.08
            - quality_penalty,
    );

    ActionabilityBlock {
        prepare: round3(prepare),
        hedge: round3(hedge.max((defend + 0.02).min(0.97))),
        defend: round3(defend),
    }
}

#[allow(clippy::too_many_arguments)]
fn build_probability_trace(
    snapshot: &RiskSnapshot,
    observations: &[Observation],
    external_shock_score: f64,
    data_trust: &DataTrust,
    jpy_carry: &JpyCarrySnapshot,
    heuristic_probabilities: &ProbabilityBlock,
    key_indicators: &[KeyIndicatorStatus],
    serving_model: Option<&ServingModelContext>,
) -> ProbabilityComputationTrace {
    let heuristic_actionability = heuristic_actionability_block(
        snapshot,
        external_shock_score,
        heuristic_probabilities,
        data_trust,
        jpy_carry,
    );
    let Some(serving_model) = serving_model else {
        return ProbabilityComputationTrace {
            raw_probabilities: heuristic_probabilities.clone(),
            calibrated_probabilities: heuristic_probabilities.clone(),
            actionability: heuristic_actionability,
            actionability_enabled: false,
            actionability_model_version: None,
            actionability_calibration_version: None,
            fusion_policy_version: None,
        };
    };
    let Some(bundle) = serving_model.probability_bundle.as_ref() else {
        return ProbabilityComputationTrace {
            raw_probabilities: heuristic_probabilities.clone(),
            calibrated_probabilities: heuristic_probabilities.clone(),
            actionability: heuristic_actionability,
            actionability_enabled: false,
            actionability_model_version: None,
            actionability_calibration_version: None,
            fusion_policy_version: None,
        };
    };

    let features = build_probability_feature_map(
        snapshot,
        observations,
        external_shock_score,
        data_trust,
        jpy_carry,
        heuristic_probabilities,
        key_indicators,
    );

    let (raw_p_5d, calibrated_p_5d_raw) = score_bundle_horizon(bundle, 5, &features)
        .unwrap_or((heuristic_probabilities.p_5d, heuristic_probabilities.p_5d));
    let (raw_p_20d, calibrated_p_20d_raw) = score_bundle_horizon(bundle, 20, &features)
        .unwrap_or((heuristic_probabilities.p_20d, heuristic_probabilities.p_20d));
    let (raw_p_60d, calibrated_p_60d_raw) = score_bundle_horizon(bundle, 60, &features)
        .unwrap_or((heuristic_probabilities.p_60d, heuristic_probabilities.p_60d));

    let raw_probabilities = ProbabilityBlock {
        p_5d: round3(raw_p_5d),
        p_20d: round3(raw_p_20d),
        p_60d: round3(raw_p_60d),
    };

    let min_gap_5_to_20 = bundle.monotonic_min_gap_5d_to_20d.max(0.0);
    let min_gap_20_to_60 = bundle.monotonic_min_gap_20d_to_60d.max(0.0);
    let calibrated_p_5d = calibrated_p_5d_raw;
    let calibrated_p_20d =
        clamp_probability(calibrated_p_20d_raw.max((calibrated_p_5d + min_gap_5_to_20).min(0.98)));
    let calibrated_p_60d = clamp_probability(
        calibrated_p_60d_raw.max((calibrated_p_20d + min_gap_20_to_60).min(0.99)),
    );
    let calibrated_probabilities = ProbabilityBlock {
        p_5d: round3(calibrated_p_5d),
        p_20d: round3(calibrated_p_20d),
        p_60d: round3(calibrated_p_60d),
    };
    let action_thresholds = probability_action_thresholds(Some(serving_model));

    let actionability = bundle
        .actionability
        .as_ref()
        .map(|actionability_bundle| ActionabilityBlock {
            prepare: round3(fuse_actionability_confidence(
                ActionabilityLevel::Prepare,
                score_actionability_level(
                    actionability_bundle,
                    ActionabilityLevel::Prepare,
                    &features,
                )
                .unwrap_or(heuristic_actionability.prepare),
                &calibrated_probabilities,
                snapshot,
                external_shock_score,
                action_thresholds,
            )),
            hedge: round3(fuse_actionability_confidence(
                ActionabilityLevel::Hedge,
                score_actionability_level(
                    actionability_bundle,
                    ActionabilityLevel::Hedge,
                    &features,
                )
                .unwrap_or(heuristic_actionability.hedge),
                &calibrated_probabilities,
                snapshot,
                external_shock_score,
                action_thresholds,
            )),
            defend: round3(fuse_actionability_confidence(
                ActionabilityLevel::Defend,
                score_actionability_level(
                    actionability_bundle,
                    ActionabilityLevel::Defend,
                    &features,
                )
                .unwrap_or(heuristic_actionability.defend),
                &calibrated_probabilities,
                snapshot,
                external_shock_score,
                action_thresholds,
            )),
        })
        .unwrap_or_else(|| heuristic_actionability.clone());

    ProbabilityComputationTrace {
        raw_probabilities,
        calibrated_probabilities,
        actionability,
        actionability_enabled: bundle.actionability.is_some(),
        actionability_model_version: bundle
            .actionability
            .as_ref()
            .map(|bundle| bundle.model_version.clone()),
        actionability_calibration_version: bundle
            .actionability
            .as_ref()
            .map(|bundle| bundle.calibration_version.clone()),
        fusion_policy_version: bundle
            .actionability
            .as_ref()
            .map(|_| "fusion_policy_v3_probability_context_gate_20260601".to_string()),
    }
}

fn build_probability_feature_map(
    snapshot: &RiskSnapshot,
    observations: &[Observation],
    external_shock_score: f64,
    data_trust: &DataTrust,
    jpy_carry: &JpyCarrySnapshot,
    heuristic_probabilities: &ProbabilityBlock,
    key_indicators: &[KeyIndicatorStatus],
) -> BTreeMap<String, f64> {
    let heuristic_bucket = build_time_to_risk_bucket(
        heuristic_probabilities,
        None,
        snapshot.structural_score,
        snapshot.trigger_score,
        external_shock_score,
        high_risk_breadth(snapshot),
        jpy_carry,
        ProbabilityActionThresholds::legacy(),
    );
    let freshness_status = worst_key_indicator_freshness(key_indicators);
    let mut features = BTreeMap::from([
        (
            FEATURE_OVERALL_SCORE.to_string(),
            (snapshot.overall_score / 100.0).clamp(0.0, 1.0),
        ),
        (
            FEATURE_EXTERNAL_SHOCK_SCORE.to_string(),
            (external_shock_score / 100.0).clamp(0.0, 1.0),
        ),
        (
            FEATURE_HEURISTIC_P_5D.to_string(),
            clamp_probability(heuristic_probabilities.p_5d),
        ),
        (
            FEATURE_HEURISTIC_P_20D.to_string(),
            clamp_probability(heuristic_probabilities.p_20d),
        ),
        (
            FEATURE_HEURISTIC_P_60D.to_string(),
            clamp_probability(heuristic_probabilities.p_60d),
        ),
        (
            FEATURE_COVERAGE_SCORE.to_string(),
            data_trust.coverage_score.clamp(0.0, 1.0),
        ),
        (
            FEATURE_BUCKET_MONTHS_OR_HIGHER.to_string(),
            matches!(
                heuristic_bucket,
                TimeToRiskBucket::Months | TimeToRiskBucket::Weeks | TimeToRiskBucket::Now
            ) as u8 as f64,
        ),
        (
            FEATURE_BUCKET_WEEKS_OR_HIGHER.to_string(),
            matches!(
                heuristic_bucket,
                TimeToRiskBucket::Weeks | TimeToRiskBucket::Now
            ) as u8 as f64,
        ),
        (
            FEATURE_BUCKET_NOW.to_string(),
            matches!(heuristic_bucket, TimeToRiskBucket::Now) as u8 as f64,
        ),
        (
            FEATURE_FRESHNESS_DELAYED_OR_WORSE.to_string(),
            matches!(
                freshness_status,
                FreshnessStatus::Delayed | FreshnessStatus::Stale | FreshnessStatus::Missing
            ) as u8 as f64,
        ),
        (
            FEATURE_FRESHNESS_STALE_OR_MISSING.to_string(),
            matches!(
                freshness_status,
                FreshnessStatus::Stale | FreshnessStatus::Missing
            ) as u8 as f64,
        ),
    ]);
    features.extend(build_formal_probability_feature_map(
        snapshot,
        observations,
        data_trust,
    ));
    features
}

fn build_formal_probability_feature_map(
    snapshot: &RiskSnapshot,
    observations: &[Observation],
    data_trust: &DataTrust,
) -> BTreeMap<String, f64> {
    let mut features = BTreeMap::from([
        (
            FEATURE_STRUCTURAL_SCORE.to_string(),
            round6((snapshot.structural_score / 100.0).clamp(0.0, 1.0)),
        ),
        (
            FEATURE_TRIGGER_SCORE.to_string(),
            round6((snapshot.trigger_score / 100.0).clamp(0.0, 1.0)),
        ),
        (
            FEATURE_EXTERNAL_DIMENSION_SCORE.to_string(),
            round6(
                (dimension_score(snapshot, RiskDimension::ExternalSector) / 100.0).clamp(0.0, 1.0),
            ),
        ),
        (
            FEATURE_COVERAGE_SCORE.to_string(),
            data_trust.coverage_score.clamp(0.0, 1.0),
        ),
    ]);
    let as_of_date = snapshot.as_of_date;

    insert_formal_latest_feature(
        &mut features,
        FEATURE_US_VIX_LEVEL,
        observations,
        "us_market_vix_close",
        as_of_date,
    );
    insert_formal_derived_feature(
        &mut features,
        FEATURE_US_VIX_CHANGE_5D,
        observation_difference_from_tail(observations, "us_market_vix_close", as_of_date, 5),
    );
    insert_formal_latest_feature(
        &mut features,
        FEATURE_US_CURVE_10Y2Y_LEVEL,
        observations,
        "us_rates_yield_curve_10y2y",
        as_of_date,
    );
    insert_formal_latest_feature(
        &mut features,
        FEATURE_US_BAA_10Y_SPREAD_LEVEL,
        observations,
        "us_credit_baa_10y_spread",
        as_of_date,
    );
    insert_formal_latest_feature(
        &mut features,
        FEATURE_US_FED_FUNDS_LEVEL,
        observations,
        "us_liquidity_effr",
        as_of_date,
    );
    insert_formal_latest_feature(
        &mut features,
        FEATURE_US_NFCI_LEVEL,
        observations,
        "us_liquidity_national_financial_conditions",
        as_of_date,
    );
    insert_formal_latest_feature(
        &mut features,
        FEATURE_US_STLFSI_LEVEL,
        observations,
        "us_liquidity_financial_stress_stl",
        as_of_date,
    );
    insert_formal_latest_feature(
        &mut features,
        FEATURE_US_UNEMPLOYMENT_LEVEL,
        observations,
        "us_macro_unemployment_rate",
        as_of_date,
    );
    insert_formal_latest_feature(
        &mut features,
        FEATURE_US_HOUSING_STARTS_LEVEL,
        observations,
        "us_real_estate_housing_starts",
        as_of_date,
    );
    insert_formal_latest_feature(
        &mut features,
        FEATURE_US_USDJPY_LEVEL,
        observations,
        "us_external_usdjpy_level",
        as_of_date,
    );
    insert_formal_derived_feature(
        &mut features,
        FEATURE_US_USDJPY_CHANGE_20D,
        observation_difference_from_tail(observations, "us_external_usdjpy_level", as_of_date, 20),
    );

    features
}

fn dimension_score(snapshot: &RiskSnapshot, dimension: RiskDimension) -> f64 {
    snapshot
        .dimensions
        .iter()
        .find(|score| score.dimension == dimension)
        .map(|score| score.score)
        .unwrap_or(0.0)
}

fn insert_formal_latest_feature(
    features: &mut BTreeMap<String, f64>,
    feature_name: &str,
    observations: &[Observation],
    indicator_id: &str,
    as_of_date: NaiveDate,
) {
    if let Some(value) = latest_observation_value(observations, indicator_id, as_of_date) {
        features.insert(feature_name.to_string(), round6(value));
    }
}

fn insert_formal_derived_feature(
    features: &mut BTreeMap<String, f64>,
    feature_name: &str,
    value: Option<f64>,
) {
    if let Some(value) = value {
        features.insert(feature_name.to_string(), round6(value));
    }
}

fn latest_observation_value(
    observations: &[Observation],
    indicator_id: &str,
    as_of_date: NaiveDate,
) -> Option<f64> {
    observation_history(observations, indicator_id, as_of_date)
        .last()
        .map(|observation| observation.value)
}

fn observation_difference_from_tail(
    observations: &[Observation],
    indicator_id: &str,
    as_of_date: NaiveDate,
    lookback: usize,
) -> Option<f64> {
    let history = observation_history(observations, indicator_id, as_of_date);
    let latest = history.last()?;
    let previous_index = history.len().checked_sub(lookback + 1)?;
    let previous = history.get(previous_index)?;
    Some(latest.value - previous.value)
}

fn observation_history<'a>(
    observations: &'a [Observation],
    indicator_id: &str,
    as_of_date: NaiveDate,
) -> Vec<&'a Observation> {
    let mut history = observations
        .iter()
        .filter(|observation| {
            observation.indicator_id == indicator_id && observation.as_of_date <= as_of_date
        })
        .collect::<Vec<_>>();
    history.sort_by_key(|observation| observation.as_of_date);
    history
}

fn score_bundle_horizon(
    bundle: &ProbabilityBundle,
    horizon_days: u32,
    features: &BTreeMap<String, f64>,
) -> Option<(f64, f64)> {
    let horizon = bundle
        .horizons
        .iter()
        .find(|horizon| horizon.horizon_days == horizon_days)?;
    let raw_probability = score_logistic_model(&horizon.raw_model, features);
    let calibrated_probability = match horizon.calibration.as_ref() {
        Some(calibration) => apply_platt_calibration(raw_probability, calibration),
        None => raw_probability,
    };
    Some((raw_probability, calibrated_probability))
}

fn score_actionability_level(
    bundle: &fc_domain::ActionabilityBundle,
    level: ActionabilityLevel,
    features: &BTreeMap<String, f64>,
) -> Option<f64> {
    let level_bundle = bundle
        .levels
        .iter()
        .find(|candidate| candidate.level == level)?;
    let raw_probability = score_logistic_model(&level_bundle.raw_model, features);
    let calibrated_probability = match level_bundle.calibration.as_ref() {
        Some(calibration) => apply_platt_calibration(raw_probability, calibration),
        None => raw_probability,
    };
    Some(actionability_confidence_from_probability(
        calibrated_probability,
        level_bundle.decision_threshold,
    ))
}

fn actionability_confidence_from_probability(probability: f64, decision_threshold: f64) -> f64 {
    let threshold = decision_threshold.clamp(0.01, 0.95);
    if probability <= threshold {
        return 0.0;
    }
    let normalized = ((probability - threshold) / (1.0 - threshold)).clamp(0.0, 1.0);
    normalized.powi(2)
}

fn fuse_actionability_confidence(
    level: ActionabilityLevel,
    confidence: f64,
    probabilities: &ProbabilityBlock,
    snapshot: &RiskSnapshot,
    external_shock_score: f64,
    thresholds: ProbabilityActionThresholds,
) -> f64 {
    if confidence <= 0.0 {
        return 0.0;
    }

    let context_support = match level {
        ActionabilityLevel::Prepare => {
            0.55 * normalized_score_support(snapshot.structural_score, 48.0, 62.0)
                + 0.25
                    * normalized_probability_support(
                        probabilities.p_60d,
                        thresholds.prepare_p60d,
                        thresholds.elevated_weeks_p60d(),
                    )
                + 0.20 * normalized_score_support(external_shock_score, 45.0, 60.0)
        }
        ActionabilityLevel::Hedge => {
            0.40 * normalized_score_support(snapshot.trigger_score, 42.0, 60.0)
                + 0.25 * normalized_score_support(external_shock_score, 44.0, 58.0)
                + 0.20 * normalized_score_support(snapshot.structural_score, 45.0, 58.0)
                + 0.15
                    * normalized_probability_support(
                        probabilities.p_20d,
                        thresholds.hedge_p20d,
                        thresholds.severe_now_p20d(),
                    )
        }
        ActionabilityLevel::Defend => {
            0.50 * normalized_score_support(snapshot.trigger_score, 55.0, 68.0)
                + 0.20 * normalized_score_support(external_shock_score, 52.0, 65.0)
                + 0.15 * normalized_score_support(snapshot.structural_score, 50.0, 62.0)
                + 0.15
                    * normalized_probability_support(
                        probabilities.p_5d,
                        thresholds.defend_p5d,
                        thresholds.capital_preservation_p5d(),
                    )
        }
    }
    .clamp(0.0, 1.0);

    round3((confidence * context_support).clamp(0.0, 1.0))
}

fn normalized_score_support(value: f64, start: f64, full: f64) -> f64 {
    if full <= start {
        return f64::from(value >= full);
    }
    ((value - start) / (full - start)).clamp(0.0, 1.0)
}

fn normalized_probability_support(value: f64, threshold: f64, full: f64) -> f64 {
    if full <= threshold {
        return f64::from(value >= full);
    }
    ((value - threshold) / (full - threshold)).clamp(0.0, 1.0)
}

fn score_logistic_model(model: &LogisticProbabilityModel, features: &BTreeMap<String, f64>) -> f64 {
    let mut linear = model.intercept;
    for coefficient in &model.coefficients {
        let stat = model
            .feature_stats
            .iter()
            .find(|stat| stat.name == coefficient.name);
        let raw_value = features
            .get(&coefficient.name)
            .copied()
            .or_else(|| stat.map(|stat| stat.fill_value))
            .unwrap_or(0.0);
        let normalized = stat.map_or(raw_value, |stat| {
            let std_dev = if stat.std_dev.abs() < 1e-9 {
                1.0
            } else {
                stat.std_dev
            };
            (raw_value - stat.mean) / std_dev
        });
        linear += normalized * coefficient.weight;
    }
    sigmoid(linear)
}

fn apply_platt_calibration(raw_probability: f64, calibration: &PlattCalibrationArtifact) -> f64 {
    let clipped = raw_probability.clamp(calibration.min_input, calibration.max_input);
    sigmoid(calibration.alpha * clipped + calibration.beta)
}

fn worst_key_indicator_freshness(key_indicators: &[KeyIndicatorStatus]) -> FreshnessStatus {
    key_indicators
        .iter()
        .map(|indicator| indicator.status)
        .max_by_key(|status| freshness_rank(*status))
        .unwrap_or(FreshnessStatus::Missing)
}

fn freshness_rank(status: FreshnessStatus) -> u8 {
    match status {
        FreshnessStatus::Fresh => 0,
        FreshnessStatus::Delayed => 1,
        FreshnessStatus::Stale => 2,
        FreshnessStatus::Missing => 3,
    }
}

#[allow(clippy::too_many_arguments)]
fn build_time_to_risk_bucket(
    probabilities: &ProbabilityBlock,
    actionability: Option<&ActionabilityBlock>,
    structural_score: f64,
    trigger_score: f64,
    external_shock_score: f64,
    breadth_score: f64,
    jpy_carry: &JpyCarrySnapshot,
    thresholds: ProbabilityActionThresholds,
) -> TimeToRiskBucket {
    let severe_carry = jpy_carry.score >= 70.0 && jpy_carry.funding_pressure_score >= 55.0;
    let stressed_carry = jpy_carry.score >= 58.0 && jpy_carry.funding_pressure_score >= 48.0;
    let defend_head_now = actionability.is_some_and(|scores| {
        scores.defend >= 0.33
            && (trigger_score >= 55.0 || external_shock_score >= 55.0 || breadth_score >= 44.0)
    });
    let hedge_head_weeks = actionability.is_some_and(|scores| {
        scores.hedge >= 0.34
            && (trigger_score >= 48.0 || external_shock_score >= 48.0 || breadth_score >= 38.0)
    });
    let prepare_head_months = actionability.is_some_and(|scores| {
        scores.prepare >= 0.36
            && (structural_score >= 52.0 || external_shock_score >= 50.0 || breadth_score >= 36.0)
    });

    if (probabilities.p_5d >= thresholds.defend_p5d
        && trigger_score >= 62.0
        && breadth_score >= 48.0)
        || (probabilities.p_20d >= thresholds.severe_now_p20d()
            && trigger_score >= 68.0
            && external_shock_score >= 55.0
            && breadth_score >= 45.0)
        || (severe_carry && external_shock_score >= 55.0 && trigger_score >= 50.0)
        || defend_head_now
    {
        TimeToRiskBucket::Now
    } else if (probabilities.p_20d >= thresholds.hedge_p20d
        && (trigger_score >= 50.0 || external_shock_score >= 50.0)
        && breadth_score >= 38.0)
        || (probabilities.p_60d >= thresholds.elevated_weeks_p60d()
            && structural_score >= 55.0
            && trigger_score >= 55.0
            && breadth_score >= 40.0)
        || (stressed_carry && external_shock_score >= 50.0 && structural_score >= 50.0)
        || hedge_head_weeks
    {
        TimeToRiskBucket::Weeks
    } else if (probabilities.p_60d >= thresholds.prepare_p60d && structural_score >= 55.0)
        || (structural_score >= 62.0
            && trigger_score >= 42.0
            && probabilities.p_60d >= thresholds.downgrade_prepare_p60d())
        || (external_shock_score >= 55.0
            && probabilities.p_20d >= thresholds.external_prepare_p20d())
        || prepare_head_months
    {
        TimeToRiskBucket::Months
    } else {
        TimeToRiskBucket::Normal
    }
}

#[allow(clippy::too_many_arguments)]
fn build_posture_guidance(
    snapshot: &RiskSnapshot,
    probabilities: &ProbabilityBlock,
    actionability: Option<&ActionabilityBlock>,
    conviction_score: f64,
    data_trust: &DataTrust,
    external_shock_score: f64,
    breadth_score: f64,
    analogs: &[HistoricalAnalog],
    jpy_carry: &JpyCarrySnapshot,
    event_assessment: &EventAssessment,
    user_preferences: &UserRiskPreferences,
    thresholds: ProbabilityActionThresholds,
) -> PostureGuidance {
    let severe_quality_block =
        matches!(data_trust.quality_grade, QualityGrade::D | QualityGrade::F);
    let defend_quality_gate = matches!(data_trust.quality_grade, QualityGrade::A | QualityGrade::B);
    let confirmation_count = posture_confirmation_count(
        snapshot.trigger_score,
        external_shock_score,
        event_assessment.confirmation_score,
    );
    let severe_carry = jpy_carry.score >= 70.0 && jpy_carry.funding_pressure_score >= 55.0;
    let stressed_carry = jpy_carry.score >= 58.0 && jpy_carry.funding_pressure_score >= 48.0;

    let defend_signal = defend_quality_gate
        && confirmation_count >= 2
        && conviction_score >= 0.62
        && breadth_score >= 48.0
        && ((probabilities.p_5d >= thresholds.defend_p5d && snapshot.trigger_score >= 60.0)
            || (severe_carry && snapshot.trigger_score >= 55.0 && external_shock_score >= 55.0)
            || actionability.is_some_and(|scores| {
                scores.defend >= 0.36
                    && (snapshot.trigger_score >= 55.0 || external_shock_score >= 55.0)
            }));
    let hedge_signal = (probabilities.p_20d >= thresholds.hedge_p20d
        && (snapshot.trigger_score >= 50.0
            || external_shock_score >= 50.0
            || breadth_score >= 40.0
            || event_assessment.confirmation_score >= 40.0))
        || (probabilities.p_60d >= thresholds.elevated_weeks_p60d()
            && snapshot.structural_score >= 55.0
            && snapshot.trigger_score >= 54.0
            && external_shock_score >= 48.0)
        || (stressed_carry
            && external_shock_score >= 50.0
            && snapshot.structural_score >= 50.0
            && snapshot.trigger_score >= 45.0)
        || actionability.is_some_and(|scores| {
            scores.hedge >= 0.36
                && (snapshot.trigger_score >= 46.0
                    || external_shock_score >= 48.0
                    || event_assessment.confirmation_score >= 35.0)
        });
    let prepare_signal = conviction_score >= 0.54
        && ((probabilities.p_60d >= thresholds.prepare_p60d && snapshot.structural_score >= 55.0)
            || (snapshot.structural_score >= 62.0
                && probabilities.p_60d >= thresholds.downgrade_prepare_p60d()
                && (snapshot.trigger_score >= 42.0
                    || external_shock_score >= 50.0
                    || jpy_carry.funding_pressure_score >= 45.0))
            || (external_shock_score >= 55.0 && snapshot.structural_score >= 52.0)
            || (jpy_carry.funding_pressure_score >= 45.0
                && snapshot.structural_score >= 50.0
                && probabilities.p_60d >= thresholds.carry_prepare_p60d())
            || actionability.is_some_and(|scores| {
                scores.prepare >= 0.38
                    && (snapshot.structural_score >= 50.0 || external_shock_score >= 50.0)
            }));

    let base_posture = if defend_signal {
        DecisionPosture::Defend
    } else if !severe_quality_block && hedge_signal {
        DecisionPosture::Hedge
    } else if prepare_signal {
        DecisionPosture::Prepare
    } else {
        DecisionPosture::Normal
    };
    let posture = adjust_posture_for_preferences(base_posture, user_preferences, event_assessment);

    let mut reasons = Vec::new();
    if snapshot.structural_score >= 60.0 {
        reasons.push("结构性脆弱性已抬升，说明风险不是单日噪声。".to_string());
    }
    if snapshot.trigger_score >= 60.0 {
        reasons.push("触发层指标进入高压区，风险窗口正在缩短。".to_string());
    }
    if external_shock_score >= 55.0 {
        reasons.push("外部放大器偏强，JPY carry 或外部冲击可能加速风险传导。".to_string());
    }
    if event_assessment.confirmation_score >= 60.0 {
        reasons.push("事件层已经开始确认压力，不再只是市场价格单侧波动。".to_string());
    }
    if jpy_carry.funding_pressure_score >= 45.0 {
        reasons.push("美日短端利差仍偏高，套息资金在风险释放阶段更容易形成拥挤平仓。".to_string());
    }
    if conviction_score < 0.55 {
        reasons.push("当前信号可信度一般，仓位动作应保留二次确认。".to_string());
    }
    if let Some(analog) = analogs.first() {
        reasons.push(format!(
            "历史上当前形态最接近 {}，但仍需看事件层是否继续确认。",
            analog.name
        ));
    }
    if reasons.is_empty() {
        reasons.push("当前多维度尚未形成高强度共振。".to_string());
    }

    let summary = match posture {
        DecisionPosture::Normal => {
            "系统未看到足够证据支持主动防守，重点是继续观察触发层变化。".to_string()
        }
        DecisionPosture::Prepare => {
            "系统认为中期脆弱性已升高，适合先做流动性检查与对冲准备。".to_string()
        }
        DecisionPosture::Hedge => {
            "系统认为未来数周风险已值得对冲，重点是先保护组合而不是等待事件完全落地。".to_string()
        }
        DecisionPosture::Defend => {
            "系统认为短期风险窗口已经打开，优先资本保全和流动性管理。".to_string()
        }
    };

    let upgrade_condition = match posture {
        DecisionPosture::Normal => {
            format!(
                "若 p_60d 升至 {} 以上且 structural score 抬升，或外部冲击与结构脆弱性同步恶化，则升级为 prepare。",
                format_probability_threshold(thresholds.prepare_p60d)
            )
        }
        DecisionPosture::Prepare => {
            format!(
                "若 p_20d 升至 {} 以上，且 trigger、external、breadth 至少一项同步恶化，则升级为 hedge。",
                format_probability_threshold(thresholds.hedge_p20d)
            )
        }
        DecisionPosture::Hedge => {
            format!(
                "若 p_5d 升至 {} 以上、数据质量不低于 B，且 trigger / external / event 至少两类确认，则升级为 defend。",
                format_probability_threshold(thresholds.defend_p5d)
            )
        }
        DecisionPosture::Defend => "除非 p_5d 明显回落且触发层缓解，否则保持 defend。".to_string(),
    };

    let downgrade_condition = match posture {
        DecisionPosture::Normal => "维持 normal，直到结构与触发层重新抬升。".to_string(),
        DecisionPosture::Prepare => {
            format!(
                "若 p_60d 回落到 {} 以下且 structural score 不再继续抬升，则降回 normal。",
                format_probability_threshold(thresholds.downgrade_prepare_p60d())
            )
        }
        DecisionPosture::Hedge => {
            format!(
                "若 p_20d 连续回落到 {} 以下、外部冲击降温且 trigger score 下降，则降回 prepare。",
                format_probability_threshold(thresholds.downgrade_hedge_p20d())
            )
        }
        DecisionPosture::Defend => {
            format!(
                "若 p_5d 连续回落到 {} 以下、触发层缓和且没有新的高等级事件确认，可先降回 hedge。",
                format_probability_threshold(thresholds.downgrade_defend_p5d())
            )
        }
    };

    PostureGuidance {
        posture,
        summary,
        reasons,
        upgrade_condition,
        downgrade_condition,
    }
}

fn posture_confirmation_count(
    trigger_score: f64,
    external_shock_score: f64,
    event_confirmation_score: f64,
) -> u8 {
    [
        trigger_score >= 60.0,
        external_shock_score >= 55.0,
        event_confirmation_score >= 55.0,
    ]
    .into_iter()
    .filter(|flag| *flag)
    .count() as u8
}

#[allow(clippy::too_many_arguments)]
fn build_position_guidance(
    posture: &PostureGuidance,
    probabilities: &ProbabilityBlock,
    time_to_risk_bucket: TimeToRiskBucket,
    data_trust: &DataTrust,
    jpy_carry: &JpyCarrySnapshot,
    event_assessment: &EventAssessment,
    active_release: Option<&ModelReleaseRecord>,
    user_preferences: &UserRiskPreferences,
    thresholds: ProbabilityActionThresholds,
) -> PositionGuidance {
    let (
        mut target_equity_exposure_pct,
        mut target_cash_pct,
        mut hedge_ratio_pct,
        mut leverage_cap_pct,
        mut option_overlay_pct,
    ): (f64, f64, f64, f64, f64) = match posture.posture {
        DecisionPosture::Normal => (70.0_f64, 10.0_f64, 0.0_f64, 100.0_f64, 0.0_f64),
        DecisionPosture::Prepare => (55.0_f64, 20.0_f64, 10.0_f64, 75.0_f64, 5.0_f64),
        DecisionPosture::Hedge => (40.0_f64, 30.0_f64, 25.0_f64, 45.0_f64, 10.0_f64),
        DecisionPosture::Defend => (25.0_f64, 45.0_f64, 40.0_f64, 20.0_f64, 15.0_f64),
    };

    target_equity_exposure_pct =
        target_equity_exposure_pct.min(user_preferences.max_equity_cap_pct);
    target_cash_pct = target_cash_pct.max(user_preferences.cash_floor_pct);
    leverage_cap_pct = leverage_cap_pct.min(user_preferences.max_leverage_pct);
    option_overlay_pct = option_overlay_pct.max(user_preferences.option_overlay_preference_pct);

    if matches!(user_preferences.profile, UserRiskProfile::Conservative) {
        hedge_ratio_pct = (hedge_ratio_pct + 5.0).clamp(0.0, 100.0);
        target_equity_exposure_pct = (target_equity_exposure_pct - 5.0).max(0.0);
    } else if matches!(user_preferences.profile, UserRiskProfile::Aggressive)
        && user_preferences.allow_aggressive_reentry
        && matches!(
            posture.posture,
            DecisionPosture::Normal | DecisionPosture::Prepare
        )
    {
        target_equity_exposure_pct = (target_equity_exposure_pct + 5.0).min(100.0);
    }

    let mut actions = Vec::new();
    match posture.posture {
        DecisionPosture::Normal => {
            actions.push("维持核心仓位，不主动放大高 beta 暴露。".to_string());
            actions.push("继续监控信用利差、波动率和 JPY carry 是否转入共振。".to_string());
        }
        DecisionPosture::Prepare => {
            actions.push("降低高 beta、低流动性和高杠杆资产比重。".to_string());
            actions.push("预留更多现金或短久期工具，准备必要时快速降仓。".to_string());
            actions.push("评估保护性认沽或波动率保护的成本窗口。".to_string());
        }
        DecisionPosture::Hedge => {
            actions.push("主动收缩高风险敞口，把组合回撤控制放到收益追逐之前。".to_string());
            actions.push("提高现金和短久期资产比例，并建立保护性认沽或指数对冲。".to_string());
            actions.push("避免新增尾部流动性差的仓位。".to_string());
        }
        DecisionPosture::Defend => {
            actions.push("优先降低总风险暴露，把组合切到资本保全模式。".to_string());
            actions.push("保留高流动性头寸，优先兑现低流动性和高弹性风险资产。".to_string());
            actions.push("用指数认沽、波动率或其他保护工具覆盖核心风险敞口。".to_string());
        }
    }

    if jpy_carry.funding_pressure_score >= 50.0 {
        actions.push("外部融资压力偏高，注意全球风险资产同步回撤时的流动性冲击。".to_string());
    }
    actions.push(format!(
        "当前用户配置为 {}，系统已按该风险偏好约束仓位预算。",
        user_profile_label(user_preferences.profile)
    ));

    let mut forbidden_actions = vec![
        "不要把单个概率值当成一键清仓指令。".to_string(),
        "不要只因单日反弹就撤掉全部保护。".to_string(),
    ];
    match posture.posture {
        DecisionPosture::Normal => {
            forbidden_actions.push("不要因为短期平静就盲目放大杠杆或追逐高 beta。".to_string());
        }
        DecisionPosture::Prepare => {
            forbidden_actions
                .push("不要等到流动性明显恶化后才开始腾挪现金和保护工具。".to_string());
        }
        DecisionPosture::Hedge => {
            forbidden_actions.push("不要在尚未满足再入场条件前逆势放大组合净敞口。".to_string());
        }
        DecisionPosture::Defend => {
            forbidden_actions
                .push("不要在短期风险窗口已打开时新增复杂、流动性差的保护结构。".to_string());
        }
    }

    let mut reentry_conditions = match posture.posture {
        DecisionPosture::Normal => vec![
            "当前无需系统性再入场动作，维持常规监控即可。".to_string(),
            "若后续进入 prepare，再按 3-10 个交易日节奏分段调整。".to_string(),
        ],
        DecisionPosture::Prepare => vec![
            format!(
                "当 p_60d 回落到 {} 以下，且 structural score 不再继续抬升时，再考虑恢复常态仓位。",
                format_probability_threshold(thresholds.downgrade_prepare_p60d())
            ),
            "恢复仓位时仍应先回补高流动性核心仓位，再看高 beta 资产。".to_string(),
        ],
        DecisionPosture::Hedge => vec![
            format!(
                "当 p_20d 连续 5 个交易日回落到 {} 以下，并且外部冲击与信用压力同步缓和时，再逐步恢复仓位。",
                format_probability_threshold(thresholds.downgrade_hedge_p20d())
            ),
            "默认按 1/3、1/3、1/3 的分批节奏恢复，不做一次性满仓回补。".to_string(),
        ],
        DecisionPosture::Defend => vec![
            format!(
                "只有当 p_5d 连续 3 个交易日回落到 {} 以下，且没有新的高等级事件确认时，才允许从 defend 降回 hedge。",
                format_probability_threshold(thresholds.downgrade_defend_p5d())
            ),
            "从 defend 撤回防守时先恢复核心流动性仓位，最后再恢复高弹性风险资产。".to_string(),
        ],
    };
    if matches!(
        event_assessment.state,
        EventConfirmationState::Confirmed | EventConfirmationState::Escalating
    ) {
        reentry_conditions
            .push("事件层仍有确认或升级信号时，不应仅凭价格反弹提前撤掉保护。".to_string());
    }

    let mut guardrails = vec![
        "系统 posture 不是自动交易指令，不能替代你自己的风险预算。".to_string(),
        "不要仅凭单个概率值做全清仓动作，必须结合流动性、税务和执行条件。".to_string(),
    ];
    if !matches!(data_trust.quality_grade, QualityGrade::A) {
        guardrails
            .push("当前数据可信度尚可，但事件层仍有原型源，建议保留人工二次确认。".to_string());
    }
    if probabilities.p_5d >= thresholds.defend_p5d {
        guardrails.push("短期窗口已打开，更应优先考虑可快速执行的保护动作。".to_string());
    }

    let execution_urgency = match time_to_risk_bucket {
        TimeToRiskBucket::Normal => "观察为主；当前不需要系统性快速去风险。".to_string(),
        TimeToRiskBucket::Months => {
            "分阶段执行；建议在 3-10 个交易日内先降脆弱仓位、补现金和准备保护工具。".to_string()
        }
        TimeToRiskBucket::Weeks => {
            "尽快执行；建议在 1-5 个交易日内完成主要减仓和第一层组合保护。".to_string()
        }
        TimeToRiskBucket::Now => {
            "立即执行；当日到 2 个交易日内优先去杠杆、补现金并建立核心保护覆盖。".to_string()
        }
    };
    let confidence_gate = match data_trust.quality_grade {
        QualityGrade::A | QualityGrade::B if event_assessment.confirmation_score >= 55.0 => {
            "当前数据可信度和事件确认度足以支持执行主要防守动作。".to_string()
        }
        QualityGrade::D | QualityGrade::F => {
            "数据可信度偏低，先把系统输出当成减震和排查信号，不应直接做极端仓位动作。".to_string()
        }
        _ => "当前更适合先降低组合脆弱性，再结合事件确认和市场流动性决定是否加大保护。".to_string(),
    };
    let capital_preservation_overlay_enabled = matches!(posture.posture, DecisionPosture::Defend)
        && matches!(time_to_risk_bucket, TimeToRiskBucket::Now)
        && probabilities.p_5d >= thresholds.capital_preservation_p5d()
        && matches!(data_trust.quality_grade, QualityGrade::A | QualityGrade::B)
        && matches!(
            event_assessment.state,
            EventConfirmationState::Confirmed | EventConfirmationState::Escalating
        );

    let action_summary = match posture.posture {
        DecisionPosture::Normal => "以观察为主，维持核心仓位，不建议主动大幅防守。".to_string(),
        DecisionPosture::Prepare => "先做减震，不急于极端防守，但要为快速切换做准备。".to_string(),
        DecisionPosture::Hedge => "进入保护性对冲区间，优先减少组合脆弱性。".to_string(),
        DecisionPosture::Defend => "进入资本保全区间，优先流动性、现金和保护覆盖。".to_string(),
    };

    PositionGuidance {
        action_playbook_version: active_release
            .map(|release| release.manifest.action_playbook_version.clone())
            .unwrap_or_else(|| ACTION_PLAYBOOK_VERSION.to_string()),
        execution_urgency,
        confidence_gate,
        target_equity_exposure_pct: round1(target_equity_exposure_pct),
        target_cash_pct: round1(target_cash_pct),
        hedge_ratio_pct: round1(hedge_ratio_pct),
        leverage_cap_pct: round1(leverage_cap_pct),
        option_overlay_pct: round1(option_overlay_pct),
        action_summary,
        actions,
        forbidden_actions,
        reentry_conditions,
        guardrails,
        capital_preservation_overlay_enabled,
    }
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

fn adjust_posture_for_preferences(
    base_posture: DecisionPosture,
    user_preferences: &UserRiskPreferences,
    event_assessment: &EventAssessment,
) -> DecisionPosture {
    match user_preferences.profile {
        UserRiskProfile::Conservative => escalate_posture(base_posture),
        UserRiskProfile::Aggressive => {
            if matches!(
                event_assessment.state,
                EventConfirmationState::Quiet | EventConfirmationState::Watching
            ) {
                deescalate_posture(base_posture)
            } else {
                base_posture
            }
        }
        UserRiskProfile::Neutral => base_posture,
    }
}

fn escalate_posture(posture: DecisionPosture) -> DecisionPosture {
    match posture {
        DecisionPosture::Normal => DecisionPosture::Prepare,
        DecisionPosture::Prepare => DecisionPosture::Hedge,
        DecisionPosture::Hedge | DecisionPosture::Defend => DecisionPosture::Defend,
    }
}

fn deescalate_posture(posture: DecisionPosture) -> DecisionPosture {
    match posture {
        DecisionPosture::Defend => DecisionPosture::Hedge,
        DecisionPosture::Hedge => DecisionPosture::Prepare,
        DecisionPosture::Prepare | DecisionPosture::Normal => DecisionPosture::Normal,
    }
}

fn user_profile_label(profile: UserRiskProfile) -> &'static str {
    match profile {
        UserRiskProfile::Conservative => "保守",
        UserRiskProfile::Neutral => "中性",
        UserRiskProfile::Aggressive => "进取",
    }
}

fn build_summary(
    _snapshot: &RiskSnapshot,
    probabilities: &ProbabilityBlock,
    time_to_risk_bucket: TimeToRiskBucket,
    posture: &PostureGuidance,
) -> String {
    let horizon_text = match time_to_risk_bucket {
        TimeToRiskBucket::Normal => "当前仍偏常态区间",
        TimeToRiskBucket::Months => "未来数月进入高风险阶段的概率已抬升",
        TimeToRiskBucket::Weeks => "未来数周风险窗口已经值得重视",
        TimeToRiskBucket::Now => "短期风险窗口已经打开",
    };
    format!(
        "{}。5d / 20d / 60d 概率分别为 {:.0}% / {:.0}% / {:.0}%，当前 posture 为 {}。",
        horizon_text,
        probabilities.p_5d * 100.0,
        probabilities.p_20d * 100.0,
        probabilities.p_60d * 100.0,
        posture_label(posture.posture)
    )
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

fn sigmoid(value: f64) -> f64 {
    1.0 / (1.0 + (-value).exp())
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
        actionability_confidence_from_probability, fuse_actionability_confidence,
        ProbabilityActionThresholds,
    };
    use chrono::{NaiveDate, Utc};
    use fc_domain::{
        ActionabilityLevel, DataQualitySummary, ProbabilityBlock, QualityGrade, RiskLevel,
        RiskSnapshot,
    };

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
}

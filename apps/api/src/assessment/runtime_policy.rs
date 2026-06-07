use std::env;

use fc_domain::{ModelReleaseRecord, ProbabilityBundle};
use serde::Serialize;

use super::round3;
use crate::demo::is_formal_main_feature_set;

const PREPARE_P60D_THRESHOLD: f64 = 0.35;
const HEDGE_P20D_THRESHOLD: f64 = 0.30;
const DEFEND_P5D_THRESHOLD: f64 = 0.30;
const FORMAL_MAIN_PREPARE_P60D_THRESHOLD: f64 = 0.10;
const FORMAL_MAIN_HEDGE_P20D_THRESHOLD: f64 = 0.07;
const FORMAL_MAIN_DEFEND_P5D_THRESHOLD: f64 = 0.03;
const FORMAL_MAIN_RUNTIME_PREPARE_P60D_FLOOR: f64 = 0.12;
const FORMAL_MAIN_RUNTIME_HEDGE_P20D_FLOOR: f64 = 0.06;
const FORMAL_MAIN_RUNTIME_DEFEND_P5D_FLOOR: f64 = 0.05;
const PREPARE_PLATEAU_P20D_BUFFER: f64 = 0.10;
const PREPARE_PLATEAU_P20D_MIN: f64 = 0.35;
const PREPARE_PLATEAU_P20D_MAX: f64 = 0.45;

#[derive(Debug, Clone, Copy)]
pub(crate) struct ProbabilityActionThresholds {
    pub(crate) prepare_p60d: f64,
    pub(crate) hedge_p20d: f64,
    pub(crate) defend_p5d: f64,
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

#[derive(Debug, Clone)]
pub struct ServingModelContext {
    pub release: ModelReleaseRecord,
    pub probability_bundle: Option<ProbabilityBundle>,
    pub runtime_probability_mode: String,
    pub runtime_release_status: String,
}

impl ProbabilityActionThresholds {
    pub(crate) fn legacy() -> Self {
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

    pub(crate) fn severe_now_p20d(self) -> f64 {
        (self.hedge_p20d + 0.20).max(self.hedge_p20d * 2.0)
    }

    pub(crate) fn elevated_weeks_p60d(self) -> f64 {
        (self.prepare_p60d + 0.10).max(self.prepare_p60d * 1.6)
    }

    pub(crate) fn external_prepare_p20d(self) -> f64 {
        (self.hedge_p20d * 0.7).max(0.04)
    }

    pub(crate) fn carry_prepare_p60d(self) -> f64 {
        (self.prepare_p60d * 0.8).max(0.05)
    }

    pub(crate) fn downgrade_prepare_p60d(self) -> f64 {
        (self.prepare_p60d * 0.75).max(0.05)
    }

    pub(crate) fn downgrade_hedge_p20d(self) -> f64 {
        (self.hedge_p20d * 0.75).max(0.04)
    }

    pub(crate) fn prepare_plateau_p20d(self) -> f64 {
        (self.hedge_p20d + PREPARE_PLATEAU_P20D_BUFFER)
            .clamp(PREPARE_PLATEAU_P20D_MIN, PREPARE_PLATEAU_P20D_MAX)
    }

    pub(crate) fn downgrade_defend_p5d(self) -> f64 {
        (self.defend_p5d * 0.67).max(0.02)
    }

    pub(crate) fn capital_preservation_p5d(self) -> f64 {
        (self.defend_p5d * 1.5).max(self.defend_p5d + 0.02)
    }
}

pub(crate) fn probability_action_thresholds(
    serving_model: Option<&ServingModelContext>,
) -> ProbabilityActionThresholds {
    let Some(serving_model) = serving_model else {
        return ProbabilityActionThresholds::legacy();
    };
    let active_release = &serving_model.release;

    if is_formal_main_feature_set(
        &active_release.manifest.feature_set_version,
        &active_release.manifest.label_version,
    ) {
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
        is_formal_main_feature_set(
            &context.release.manifest.feature_set_version,
            &context.release.manifest.label_version,
        )
    }) {
        "formal_main"
    } else if serving_model.is_some() {
        "release"
    } else {
        "heuristic"
    };

    format!(
        "runtime_history_v4_20260606|class={release_class}|prepare={:.3}|hedge={:.3}|defend={:.3}",
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

fn probability_threshold_env_override(name: &str, fallback: f64) -> f64 {
    env::var(name)
        .ok()
        .and_then(|raw| raw.parse::<f64>().ok())
        .map(|value| value.clamp(0.001, 0.90))
        .unwrap_or(fallback)
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

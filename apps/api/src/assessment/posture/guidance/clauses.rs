use fc_domain::{
    ActionabilityBlock, DataTrust, DecisionPosture, EventAssessment, JpyCarrySnapshot,
    ProbabilityBlock, QualityGrade, RiskSnapshot,
};

use super::super::super::ProbabilityActionThresholds;
use super::counters::{
    posture_confirmation_count, prepare_context_confirmation_count,
    prepare_non_carry_confirmation_count, prepare_non_external_confirmation_count,
};

const PREPARE_CONTINUITY_P20D_FLOOR_RATIO: f64 = 2.0;
const PREPARE_CONTINUITY_P20D_FLOOR_MIN: f64 = 0.12;
const PREPARE_CONTINUITY_P20D_FLOOR_MAX: f64 = 0.18;
const PREPARE_CONTINUITY_LOW_RUNTIME_P20D_FLOOR: f64 = 0.18;
const PREPARE_CONTINUITY_P60D_FLOOR_MIN: f64 = 0.22;
const PREPARE_CONTINUITY_P60D_FLOOR_MAX: f64 = 0.45;
const PREPARE_CONTINUITY_STRUCTURAL_FLOOR: f64 = 60.0;
const PREPARE_CONTINUITY_ACTIONABILITY_FLOOR: f64 = 0.18;
const PREPARE_PROBABILITY_PLATEAU_P60D_FLOOR: f64 = 0.70;
const PREPARE_PROBABILITY_PLATEAU_RELAXED_P20D_BUFFER: f64 = 0.10;
const PREPARE_PROBABILITY_PLATEAU_RELAXED_P20D_FLOOR_MIN: f64 = 0.45;
const PREPARE_PROBABILITY_PLATEAU_RELAXED_P60D_FLOOR: f64 = 0.65;
const PREPARE_PROBABILITY_PLATEAU_OVERALL_FLOOR: f64 = 42.0;
const PREPARE_PROBABILITY_PLATEAU_STRUCTURAL_FLOOR: f64 = 47.0;
const PREPARE_PROBABILITY_PLATEAU_TRIGGER_FLOOR: f64 = 40.0;
const PREPARE_PROBABILITY_PLATEAU_EXTERNAL_FLOOR: f64 = 42.0;
const PREPARE_PROBABILITY_PLATEAU_BREADTH_FLOOR: f64 = 36.0;
const PREPARE_PROBABILITY_PLATEAU_RELAXED_STRUCTURAL_FLOOR: f64 = 44.0;
const PREPARE_PROBABILITY_PLATEAU_RELAXED_TRIGGER_FLOOR: f64 = 36.0;
const PREPARE_PROBABILITY_PLATEAU_RELAXED_EXTERNAL_FLOOR: f64 = 40.0;
const PREPARE_PROBABILITY_PLATEAU_RELAXED_BREADTH_FLOOR: f64 = 34.0;
const PREPARE_TRIGGER_DOMINANT_PLATEAU_P20D_FLOOR: f64 = 0.22;
const PREPARE_TRIGGER_DOMINANT_PLATEAU_P60D_FLOOR: f64 = 0.80;
const PREPARE_TRIGGER_DOMINANT_PLATEAU_OVERALL_FLOOR: f64 = 47.0;
const PREPARE_TRIGGER_DOMINANT_PLATEAU_STRUCTURAL_CEILING: f64 = 48.0;
const PREPARE_TRIGGER_DOMINANT_PLATEAU_TRIGGER_FLOOR: f64 = 53.0;
const PREPARE_TRIGGER_DOMINANT_PLATEAU_EXTERNAL_FLOOR: f64 = 28.0;
const PREPARE_TRIGGER_DOMINANT_PLATEAU_EXTERNAL_CEILING: f64 = 35.0;
const LOW_RUNTIME_PREPARE_FLOOR_CEILING: f64 = 0.30;
const LOW_RUNTIME_HEDGE_FLOOR_CEILING: f64 = 0.12;
const SATURATED_PREPARE_LONG_WINDOW_P60D_FLOOR: f64 = 0.90;
const SATURATED_PREPARE_STRUCTURAL_LONG_WINDOW_P60D_FLOOR: f64 = 0.80;
const SATURATED_PREPARE_P20D_CONFIRMATION_FLOOR: f64 = 0.85;
const SATURATED_PREPARE_TRIGGER_CONFIRMATION_FLOOR: f64 = 55.0;
const SATURATED_HEDGE_LONG_WINDOW_P60D_FLOOR: f64 = 0.80;
const SATURATED_HEDGE_P20D_CONFIRMATION_FLOOR: f64 = 0.45;
const SATURATED_HEDGE_TRIGGER_CONFIRMATION_FLOOR: f64 = 55.0;
const SATURATED_HEDGE_EXTERNAL_CONFIRMATION_FLOOR: f64 = 48.0;

#[derive(Debug, Clone, Default)]
pub(super) struct PostureClauseDiagnostics {
    defend_trigger_codes: Vec<&'static str>,
    hedge_trigger_codes: Vec<&'static str>,
    prepare_trigger_codes: Vec<&'static str>,
    blocker_codes: Vec<&'static str>,
}

impl PostureClauseDiagnostics {
    pub(super) fn has_defend_signal(&self) -> bool {
        !self.defend_trigger_codes.is_empty()
    }

    pub(super) fn has_hedge_signal(&self) -> bool {
        !self.hedge_trigger_codes.is_empty()
    }

    pub(super) fn has_prepare_signal(&self) -> bool {
        !self.prepare_trigger_codes.is_empty()
    }

    pub(super) fn selected_trigger_codes(&self, posture: DecisionPosture) -> Vec<String> {
        match posture {
            DecisionPosture::Defend => self
                .defend_trigger_codes
                .iter()
                .map(|code| (*code).to_string())
                .collect(),
            DecisionPosture::Hedge => self
                .hedge_trigger_codes
                .iter()
                .map(|code| (*code).to_string())
                .collect(),
            DecisionPosture::Prepare => self
                .prepare_trigger_codes
                .iter()
                .map(|code| (*code).to_string())
                .collect(),
            DecisionPosture::Normal => Vec::new(),
        }
    }

    pub(super) fn blocker_code_strings(&self) -> Vec<String> {
        self.blocker_codes
            .iter()
            .map(|code| (*code).to_string())
            .collect()
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn prepare_continuity_bridge_signal(
    probabilities: &ProbabilityBlock,
    prepare_reference_p60d: Option<f64>,
    actionability: Option<&ActionabilityBlock>,
    structural_score: f64,
    trigger_score: f64,
    external_shock_score: f64,
    breadth_score: f64,
    thresholds: ProbabilityActionThresholds,
) -> bool {
    let prepare_p60d = prepare_reference_p60d.unwrap_or(probabilities.p_60d);
    let prepare_continuity_p20d_floor = prepare_continuity_p20d_floor(thresholds);
    let prepare_continuity_p60d_floor = prepare_continuity_p60d_floor(thresholds);
    let saturated_prepare_context_confirmed = saturated_prepare_structural_context_confirmed(
        probabilities,
        prepare_p60d,
        trigger_score,
        external_shock_score,
        0.0,
        thresholds,
    );

    actionability.is_some_and(|scores| {
        scores.prepare >= PREPARE_CONTINUITY_ACTIONABILITY_FLOOR
            && probabilities.p_20d >= prepare_continuity_p20d_floor
            && prepare_p60d >= prepare_continuity_p60d_floor
            && structural_score >= PREPARE_CONTINUITY_STRUCTURAL_FLOOR
            && (trigger_score >= 40.0 || external_shock_score >= 42.0 || breadth_score >= 36.0)
            && saturated_prepare_context_confirmed
    })
}

fn prepare_continuity_p20d_floor(thresholds: ProbabilityActionThresholds) -> f64 {
    let floor = (thresholds.hedge_p20d * PREPARE_CONTINUITY_P20D_FLOOR_RATIO).clamp(
        PREPARE_CONTINUITY_P20D_FLOOR_MIN,
        PREPARE_CONTINUITY_P20D_FLOOR_MAX,
    );
    if low_runtime_thresholds(thresholds) {
        floor.max(PREPARE_CONTINUITY_LOW_RUNTIME_P20D_FLOOR)
    } else {
        floor
    }
}

fn prepare_continuity_p60d_floor(thresholds: ProbabilityActionThresholds) -> f64 {
    thresholds.elevated_weeks_p60d().clamp(
        PREPARE_CONTINUITY_P60D_FLOOR_MIN,
        PREPARE_CONTINUITY_P60D_FLOOR_MAX,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn prepare_probability_plateau_signal(
    probabilities: &ProbabilityBlock,
    prepare_reference_p60d: Option<f64>,
    overall_score: f64,
    structural_score: f64,
    trigger_score: f64,
    external_shock_score: f64,
    breadth_score: f64,
    thresholds: ProbabilityActionThresholds,
) -> bool {
    let prepare_p60d = prepare_reference_p60d.unwrap_or(probabilities.p_60d);
    let plateau_p20d_floor = thresholds.prepare_plateau_p20d();
    let plateau_context_ready = structural_score >= PREPARE_PROBABILITY_PLATEAU_STRUCTURAL_FLOOR
        && (trigger_score >= PREPARE_PROBABILITY_PLATEAU_TRIGGER_FLOOR
            || external_shock_score >= PREPARE_PROBABILITY_PLATEAU_EXTERNAL_FLOOR
            || breadth_score >= PREPARE_PROBABILITY_PLATEAU_BREADTH_FLOOR);
    let relaxed_plateau_p20d_floor = (plateau_p20d_floor
        + PREPARE_PROBABILITY_PLATEAU_RELAXED_P20D_BUFFER)
        .max(PREPARE_PROBABILITY_PLATEAU_RELAXED_P20D_FLOOR_MIN);
    let relaxed_plateau_context_ready = structural_score
        >= PREPARE_PROBABILITY_PLATEAU_RELAXED_STRUCTURAL_FLOOR
        && (trigger_score >= PREPARE_PROBABILITY_PLATEAU_RELAXED_TRIGGER_FLOOR
            || external_shock_score >= PREPARE_PROBABILITY_PLATEAU_RELAXED_EXTERNAL_FLOOR
            || breadth_score >= PREPARE_PROBABILITY_PLATEAU_RELAXED_BREADTH_FLOOR);
    let saturated_prepare_context_confirmed = saturated_prepare_context_confirmed(
        probabilities,
        prepare_p60d,
        trigger_score,
        external_shock_score,
        0.0,
        thresholds,
    );

    (overall_score >= PREPARE_PROBABILITY_PLATEAU_OVERALL_FLOOR
        && probabilities.p_20d >= plateau_p20d_floor
        && prepare_p60d >= PREPARE_PROBABILITY_PLATEAU_P60D_FLOOR
        && plateau_context_ready
        && saturated_prepare_context_confirmed)
        || (overall_score >= PREPARE_PROBABILITY_PLATEAU_OVERALL_FLOOR
            && probabilities.p_20d >= relaxed_plateau_p20d_floor
            && prepare_p60d >= PREPARE_PROBABILITY_PLATEAU_RELAXED_P60D_FLOOR
            && relaxed_plateau_context_ready
            && saturated_prepare_context_confirmed)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn prepare_trigger_dominant_plateau_signal(
    probabilities: &ProbabilityBlock,
    prepare_reference_p60d: Option<f64>,
    overall_score: f64,
    structural_score: f64,
    trigger_score: f64,
    external_shock_score: f64,
    thresholds: ProbabilityActionThresholds,
) -> bool {
    let prepare_p60d = prepare_reference_p60d.unwrap_or(probabilities.p_60d);
    low_runtime_thresholds(thresholds)
        && probabilities.p_20d >= PREPARE_TRIGGER_DOMINANT_PLATEAU_P20D_FLOOR
        && prepare_p60d >= PREPARE_TRIGGER_DOMINANT_PLATEAU_P60D_FLOOR
        && overall_score >= PREPARE_TRIGGER_DOMINANT_PLATEAU_OVERALL_FLOOR
        && structural_score <= PREPARE_TRIGGER_DOMINANT_PLATEAU_STRUCTURAL_CEILING
        && trigger_score >= PREPARE_TRIGGER_DOMINANT_PLATEAU_TRIGGER_FLOOR
        && (PREPARE_TRIGGER_DOMINANT_PLATEAU_EXTERNAL_FLOOR..=PREPARE_TRIGGER_DOMINANT_PLATEAU_EXTERNAL_CEILING).contains(&external_shock_score)
}

fn low_runtime_thresholds(thresholds: ProbabilityActionThresholds) -> bool {
    thresholds.prepare_p60d < LOW_RUNTIME_PREPARE_FLOOR_CEILING
        && thresholds.hedge_p20d < LOW_RUNTIME_HEDGE_FLOOR_CEILING
}

fn saturated_long_window_context(
    prepare_p60d: f64,
    thresholds: ProbabilityActionThresholds,
) -> bool {
    low_runtime_thresholds(thresholds) && prepare_p60d >= SATURATED_PREPARE_LONG_WINDOW_P60D_FLOOR
}

fn saturated_hedge_long_window_context(
    prepare_p60d: f64,
    thresholds: ProbabilityActionThresholds,
) -> bool {
    low_runtime_thresholds(thresholds) && prepare_p60d >= SATURATED_HEDGE_LONG_WINDOW_P60D_FLOOR
}

fn saturated_prepare_structural_long_window_context(
    prepare_p60d: f64,
    thresholds: ProbabilityActionThresholds,
) -> bool {
    low_runtime_thresholds(thresholds)
        && prepare_p60d >= SATURATED_PREPARE_STRUCTURAL_LONG_WINDOW_P60D_FLOOR
}

pub(super) fn saturated_prepare_context_confirmed(
    probabilities: &ProbabilityBlock,
    prepare_p60d: f64,
    trigger_score: f64,
    _external_shock_score: f64,
    event_confirmation_score: f64,
    thresholds: ProbabilityActionThresholds,
) -> bool {
    !saturated_long_window_context(prepare_p60d, thresholds)
        || probabilities.p_20d >= SATURATED_PREPARE_P20D_CONFIRMATION_FLOOR
        || trigger_score >= SATURATED_PREPARE_TRIGGER_CONFIRMATION_FLOOR
        || event_confirmation_score >= SATURATED_PREPARE_TRIGGER_CONFIRMATION_FLOOR
}

pub(super) fn saturated_prepare_structural_context_confirmed(
    probabilities: &ProbabilityBlock,
    prepare_p60d: f64,
    trigger_score: f64,
    _external_shock_score: f64,
    event_confirmation_score: f64,
    thresholds: ProbabilityActionThresholds,
) -> bool {
    !saturated_prepare_structural_long_window_context(prepare_p60d, thresholds)
        || probabilities.p_20d >= SATURATED_PREPARE_P20D_CONFIRMATION_FLOOR
        || trigger_score >= SATURATED_PREPARE_TRIGGER_CONFIRMATION_FLOOR
        || event_confirmation_score >= SATURATED_PREPARE_TRIGGER_CONFIRMATION_FLOOR
}

pub(super) fn saturated_hedge_context_confirmed(
    probabilities: &ProbabilityBlock,
    prepare_p60d: f64,
    trigger_score: f64,
    external_shock_score: f64,
    event_confirmation_score: f64,
    thresholds: ProbabilityActionThresholds,
) -> bool {
    !saturated_hedge_long_window_context(prepare_p60d, thresholds)
        || (probabilities.p_20d >= SATURATED_HEDGE_P20D_CONFIRMATION_FLOOR
            && (trigger_score >= SATURATED_HEDGE_TRIGGER_CONFIRMATION_FLOOR
                || external_shock_score >= SATURATED_HEDGE_EXTERNAL_CONFIRMATION_FLOOR
                || event_confirmation_score >= SATURATED_HEDGE_TRIGGER_CONFIRMATION_FLOOR))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_posture_clause_diagnostics(
    snapshot: &RiskSnapshot,
    probabilities: &ProbabilityBlock,
    prepare_reference_p60d: Option<f64>,
    actionability_trigger: Option<&ActionabilityBlock>,
    actionability_support: Option<&ActionabilityBlock>,
    conviction_score: f64,
    data_trust: &DataTrust,
    external_shock_score: f64,
    breadth_score: f64,
    jpy_carry: &JpyCarrySnapshot,
    event_assessment: &EventAssessment,
    thresholds: ProbabilityActionThresholds,
) -> PostureClauseDiagnostics {
    let severe_quality_block =
        matches!(data_trust.quality_grade, QualityGrade::D | QualityGrade::F);
    let prepare_p60d = prepare_reference_p60d.unwrap_or(probabilities.p_60d);
    let defend_quality_gate = matches!(data_trust.quality_grade, QualityGrade::A | QualityGrade::B);
    let confirmation_count = posture_confirmation_count(
        snapshot.trigger_score,
        external_shock_score,
        event_assessment.confirmation_score,
    );
    let prepare_confirmation_count = prepare_context_confirmation_count(
        snapshot.trigger_score,
        external_shock_score,
        breadth_score,
        event_assessment.confirmation_score,
        jpy_carry.funding_pressure_score,
    );
    let prepare_non_external_confirmation_count = prepare_non_external_confirmation_count(
        snapshot.trigger_score,
        breadth_score,
        event_assessment.confirmation_score,
        jpy_carry.funding_pressure_score,
    );
    let prepare_non_carry_confirmation_count = prepare_non_carry_confirmation_count(
        snapshot.trigger_score,
        external_shock_score,
        breadth_score,
        event_assessment.confirmation_score,
    );
    let saturated_prepare_context_confirmed = saturated_prepare_structural_context_confirmed(
        probabilities,
        prepare_p60d,
        snapshot.trigger_score,
        external_shock_score,
        event_assessment.confirmation_score,
        thresholds,
    );
    let saturated_hedge_context_confirmed = saturated_hedge_context_confirmed(
        probabilities,
        prepare_p60d,
        snapshot.trigger_score,
        external_shock_score,
        event_assessment.confirmation_score,
        thresholds,
    );
    let prepare_continuity_bridge = prepare_continuity_bridge_signal(
        probabilities,
        prepare_reference_p60d,
        actionability_support,
        snapshot.structural_score,
        snapshot.trigger_score,
        external_shock_score,
        breadth_score,
        thresholds,
    );
    let prepare_trigger_dominant_plateau = prepare_trigger_dominant_plateau_signal(
        probabilities,
        prepare_reference_p60d,
        snapshot.overall_score,
        snapshot.structural_score,
        snapshot.trigger_score,
        external_shock_score,
        thresholds,
    );
    let severe_carry = jpy_carry.score >= 70.0 && jpy_carry.funding_pressure_score >= 55.0;
    let stressed_carry = jpy_carry.score >= 58.0 && jpy_carry.funding_pressure_score >= 48.0;

    let mut defend_trigger_codes = Vec::new();
    if defend_quality_gate
        && confirmation_count >= 2
        && conviction_score >= 0.62
        && breadth_score >= 48.0
    {
        if probabilities.p_5d >= thresholds.defend_p5d && snapshot.trigger_score >= 60.0 {
            defend_trigger_codes.push("defend_p5d_trigger");
        }
        if severe_carry && snapshot.trigger_score >= 55.0 && external_shock_score >= 55.0 {
            defend_trigger_codes.push("defend_carry_trigger");
        }
        if actionability_trigger.is_some_and(|scores| {
            scores.defend >= 0.36
                && (snapshot.trigger_score >= 55.0 || external_shock_score >= 55.0)
        }) {
            defend_trigger_codes.push("defend_actionability");
        }
    }

    let mut hedge_trigger_codes = Vec::new();
    let hedge_context_support_count = [
        snapshot.trigger_score >= 50.0,
        external_shock_score >= 50.0,
        breadth_score >= 40.0,
        event_assessment.confirmation_score >= 40.0,
    ]
    .into_iter()
    .filter(|supported| *supported)
    .count();
    let hedge_medium_horizon_support = snapshot.structural_score >= 48.0
        || probabilities.p_60d >= thresholds.downgrade_prepare_p60d()
        || stressed_carry;
    let hedge_context_ready = snapshot.overall_score >= 58.0
        || external_shock_score >= 50.0
        || event_assessment.confirmation_score >= 45.0
        || stressed_carry;
    if probabilities.p_20d >= thresholds.hedge_p20d
        && hedge_context_support_count >= 2
        && hedge_medium_horizon_support
        && hedge_context_ready
        && saturated_hedge_context_confirmed
    {
        hedge_trigger_codes.push("hedge_p20d_context");
    }
    if probabilities.p_60d >= thresholds.elevated_weeks_p60d()
        && snapshot.structural_score >= 55.0
        && snapshot.trigger_score >= 54.0
        && external_shock_score >= 48.0
    {
        hedge_trigger_codes.push("hedge_p60d_elevated");
    }
    if stressed_carry
        && external_shock_score >= 50.0
        && snapshot.structural_score >= 50.0
        && snapshot.trigger_score >= 45.0
    {
        hedge_trigger_codes.push("hedge_carry_structural");
    }
    if actionability_trigger.is_some_and(|scores| {
        scores.hedge >= 0.36
            && (snapshot.trigger_score >= 46.0
                || external_shock_score >= 48.0
                || event_assessment.confirmation_score >= 35.0)
    }) {
        hedge_trigger_codes.push("hedge_actionability");
    }

    let mut prepare_trigger_codes = Vec::new();
    if conviction_score >= 0.54 {
        if prepare_p60d >= thresholds.prepare_p60d
            && snapshot.structural_score >= 58.0
            && prepare_confirmation_count >= 2
            && saturated_prepare_context_confirmed
        {
            prepare_trigger_codes.push("prepare_p60d_structural");
        }
        if snapshot.structural_score >= 64.0
            && prepare_p60d >= thresholds.downgrade_prepare_p60d()
            && prepare_confirmation_count >= 2
            && saturated_prepare_context_confirmed
        {
            prepare_trigger_codes.push("prepare_structural_downgrade");
        }
        if external_shock_score >= 58.0
            && snapshot.structural_score >= 54.0
            && probabilities.p_20d >= thresholds.external_prepare_p20d()
            && prepare_non_external_confirmation_count >= 1
            && saturated_prepare_context_confirmed
        {
            prepare_trigger_codes.push("prepare_external_structural");
        }
        if stressed_carry
            && snapshot.structural_score >= 56.0
            && prepare_p60d >= thresholds.carry_prepare_p60d()
            && prepare_non_carry_confirmation_count >= 1
            && saturated_prepare_context_confirmed
        {
            prepare_trigger_codes.push("prepare_carry_structural");
        }
        if actionability_trigger.is_some_and(|scores| {
            scores.prepare >= 0.40
                && prepare_p60d >= thresholds.downgrade_prepare_p60d()
                && prepare_confirmation_count >= 2
                && (snapshot.structural_score >= 56.0 || external_shock_score >= 55.0)
                && saturated_prepare_context_confirmed
        }) {
            prepare_trigger_codes.push("prepare_actionability");
        }
    }

    // Probability-driven prepare signal: when the formal model's output
    // consistently exceeds its own decision thresholds, treat it as a
    // prepare-level warning even if structural/trigger context is moderate.
    // This fixes posture continuity for scenarios where the model detects
    // elevated near-term risk but traditional score-based context hasn't
    // fully built up yet (e.g. 2022 rate shock scenario).
    if !severe_quality_block
        && conviction_score >= 0.50
        && probabilities.p_20d >= thresholds.hedge_p20d
        && prepare_p60d >= thresholds.prepare_p60d
        && (snapshot.structural_score >= 48.0
            || snapshot.trigger_score >= 40.0
            || external_shock_score >= 40.0
            || breadth_score >= 28.0)
    {
        prepare_trigger_codes.push("prepare_formal_probability");
    }
    if prepare_continuity_bridge {
        prepare_trigger_codes.push("prepare_continuity_bridge");
    }
    if prepare_trigger_dominant_plateau {
        prepare_trigger_codes.push("prepare_trigger_dominant_plateau");
    }
    if prepare_probability_plateau_signal(
        probabilities,
        prepare_reference_p60d,
        snapshot.overall_score,
        snapshot.structural_score,
        snapshot.trigger_score,
        external_shock_score,
        breadth_score,
        thresholds,
    ) {
        prepare_trigger_codes.push("prepare_probability_plateau");
    }

    let mut blocker_codes = Vec::new();
    if severe_quality_block && !hedge_trigger_codes.is_empty() {
        blocker_codes.push("quality_blocked_hedge");
    }

    PostureClauseDiagnostics {
        defend_trigger_codes,
        hedge_trigger_codes,
        prepare_trigger_codes,
        blocker_codes,
    }
}

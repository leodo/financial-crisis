use fc_domain::{
    ActionabilityBlock, DataTrust, DecisionPosture, EventAssessment, JpyCarrySnapshot,
    ProbabilityBlock, QualityGrade, RiskSnapshot,
};

use super::super::super::ProbabilityActionThresholds;
use super::counters::{
    posture_confirmation_count, prepare_context_confirmation_count,
    prepare_non_carry_confirmation_count, prepare_non_external_confirmation_count,
};

const PREPARE_CONTINUITY_P20D_FLOOR: f64 = 0.18;
const PREPARE_CONTINUITY_P60D_FLOOR: f64 = 0.45;
const PREPARE_CONTINUITY_STRUCTURAL_FLOOR: f64 = 60.0;
const PREPARE_CONTINUITY_ACTIONABILITY_FLOOR: f64 = 0.18;

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
) -> bool {
    let prepare_p60d = prepare_reference_p60d.unwrap_or(probabilities.p_60d);

    actionability.is_some_and(|scores| {
        scores.prepare >= PREPARE_CONTINUITY_ACTIONABILITY_FLOOR
            && probabilities.p_20d >= PREPARE_CONTINUITY_P20D_FLOOR
            && prepare_p60d >= PREPARE_CONTINUITY_P60D_FLOOR
            && structural_score >= PREPARE_CONTINUITY_STRUCTURAL_FLOOR
            && (trigger_score >= 40.0 || external_shock_score >= 42.0 || breadth_score >= 36.0)
    })
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
        {
            prepare_trigger_codes.push("prepare_p60d_structural");
        }
        if snapshot.structural_score >= 64.0
            && prepare_p60d >= thresholds.downgrade_prepare_p60d()
            && prepare_confirmation_count >= 2
        {
            prepare_trigger_codes.push("prepare_structural_downgrade");
        }
        if external_shock_score >= 58.0
            && snapshot.structural_score >= 54.0
            && probabilities.p_20d >= thresholds.external_prepare_p20d()
            && prepare_non_external_confirmation_count >= 1
        {
            prepare_trigger_codes.push("prepare_external_structural");
        }
        if stressed_carry
            && snapshot.structural_score >= 56.0
            && prepare_p60d >= thresholds.carry_prepare_p60d()
            && prepare_non_carry_confirmation_count >= 1
        {
            prepare_trigger_codes.push("prepare_carry_structural");
        }
        if actionability_trigger.is_some_and(|scores| {
            scores.prepare >= 0.40
                && prepare_p60d >= thresholds.downgrade_prepare_p60d()
                && prepare_confirmation_count >= 2
                && (snapshot.structural_score >= 56.0 || external_shock_score >= 55.0)
        }) {
            prepare_trigger_codes.push("prepare_actionability");
        }
        if prepare_continuity_bridge_signal(
            probabilities,
            prepare_reference_p60d,
            actionability_support,
            snapshot.structural_score,
            snapshot.trigger_score,
            external_shock_score,
            breadth_score,
        ) {
            prepare_trigger_codes.push("prepare_continuity_bridge");
        }
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

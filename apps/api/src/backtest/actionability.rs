use fc_domain::{
    actionable_warning_point as domain_actionable_warning_point, ActionableGateThresholds,
    AssessmentHistoryPoint, DecisionPosture, TimeToRiskBucket,
};

use crate::{
    assessment::{ProbabilityActionThresholds, ServingModelContext},
    history_replay::is_formal_main_release,
};

const ACTIONABLE_AUDIT_HORIZON_DEFEND_DAYS: i64 = 5;
const ACTIONABLE_AUDIT_HORIZON_HEDGE_DAYS: i64 = 20;
const ACTIONABLE_AUDIT_HORIZON_PREPARE_DAYS: i64 = 60;
pub(crate) fn is_actionable_warning_point(
    point: &AssessmentHistoryPoint,
    use_transitional_bridge: bool,
    strict_thresholds: Option<ProbabilityActionThresholds>,
) -> bool {
    domain_actionable_warning_point(
        point,
        use_transitional_bridge,
        actionable_gate_thresholds(strict_thresholds),
    )
}

fn actionable_gate_thresholds(
    strict_thresholds: Option<ProbabilityActionThresholds>,
) -> Option<ActionableGateThresholds> {
    strict_thresholds.map(|thresholds| ActionableGateThresholds {
        prepare_p60d: thresholds.prepare_p60d,
        hedge_p20d: thresholds.hedge_p20d,
        defend_p5d: thresholds.defend_p5d,
        external_prepare_p20d: thresholds.external_prepare_p20d(),
    })
}

pub(crate) fn use_transitional_actionable_bridge(
    serving_model: Option<&ServingModelContext>,
) -> bool {
    !is_formal_main_release(serving_model)
}

pub(super) fn is_structural_warning_point(point: &AssessmentHistoryPoint) -> bool {
    ((point.p_60d >= 0.35) || !matches!(point.time_to_risk_bucket, TimeToRiskBucket::Normal))
        && point.overall_score >= 54.0
        || (point.overall_score >= 54.0
            && point.p_20d >= 0.12
            && point.external_shock_score >= 42.0)
}

pub(super) fn actionable_audit_horizon_days(point: &AssessmentHistoryPoint) -> i64 {
    match point.posture {
        DecisionPosture::Defend => ACTIONABLE_AUDIT_HORIZON_DEFEND_DAYS,
        DecisionPosture::Hedge => ACTIONABLE_AUDIT_HORIZON_HEDGE_DAYS,
        DecisionPosture::Prepare => ACTIONABLE_AUDIT_HORIZON_PREPARE_DAYS,
        DecisionPosture::Normal => match point.time_to_risk_bucket {
            TimeToRiskBucket::Now => ACTIONABLE_AUDIT_HORIZON_DEFEND_DAYS,
            TimeToRiskBucket::Weeks => ACTIONABLE_AUDIT_HORIZON_HEDGE_DAYS,
            TimeToRiskBucket::Months => ACTIONABLE_AUDIT_HORIZON_PREPARE_DAYS,
            TimeToRiskBucket::Normal => ACTIONABLE_AUDIT_HORIZON_HEDGE_DAYS,
        },
    }
}

use fc_domain::ActionabilityEvaluationSummary;

mod guardrails;
mod summary;
mod threshold;
mod train;

pub(crate) use guardrails::{
    actionability_bundle_quality_regressions, actionability_guardrail_policy,
    actionability_prediction_count_ceiling_from_actual_positive_count, percentage_score,
};
pub(crate) use summary::evaluate_actionability_summary;
pub(crate) use threshold::select_actionability_calibration_strategy;
#[cfg(test)]
pub(crate) use threshold::select_actionability_decision_threshold;
pub(crate) use train::train_actionability_bundle;

fn actionability_precision_floor_score(horizon_days: u32) -> i64 {
    match horizon_days {
        5 => 120,
        20 => 100,
        60 => 80,
        _ => 100,
    }
}

fn actionability_prediction_count_ceiling(
    summary: &ActionabilityEvaluationSummary,
    horizon_days: u32,
) -> u32 {
    guardrails::actionability_prediction_count_ceiling_from_actual_positive_count(
        summary.actual_positive_count,
        horizon_days,
    )
}

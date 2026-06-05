mod actionability;
mod rolling_audit;
mod scenarios;
mod timeline;

use fc_domain::{
    AssessmentHistoryPoint, BacktestRollingAudit, BacktestScenarioSummary, BacktestWindowPoint,
    ProtectedStressWindow, RiskSnapshot,
};

use crate::assessment::ServingModelContext;

pub(crate) fn build_backtests(
    snapshot: &RiskSnapshot,
    history: &[AssessmentHistoryPoint],
    use_transitional_bridge: bool,
) -> Vec<BacktestScenarioSummary> {
    scenarios::build_backtests(snapshot, history, use_transitional_bridge)
}

pub(crate) fn build_backtest_timeline(
    history: &[AssessmentHistoryPoint],
    use_transitional_bridge: bool,
) -> Vec<BacktestWindowPoint> {
    timeline::build_backtest_timeline(history, use_transitional_bridge)
}

pub(crate) fn build_rolling_backtest_audit(
    history: &[AssessmentHistoryPoint],
    stress_windows: &[ProtectedStressWindow],
    use_transitional_bridge: bool,
) -> BacktestRollingAudit {
    rolling_audit::build_rolling_backtest_audit(history, stress_windows, use_transitional_bridge)
}

#[cfg(test)]
pub(crate) fn is_actionable_warning_point(
    point: &AssessmentHistoryPoint,
    use_transitional_bridge: bool,
) -> bool {
    actionability::is_actionable_warning_point(point, use_transitional_bridge)
}

pub(crate) fn use_transitional_actionable_bridge(
    serving_model: Option<&ServingModelContext>,
) -> bool {
    actionability::use_transitional_actionable_bridge(serving_model)
}

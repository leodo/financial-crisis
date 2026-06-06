use fc_domain::{AssessmentHistoryPoint, BacktestWindowPoint};

use crate::assessment::ProbabilityActionThresholds;

use super::actionability::is_actionable_warning_point;

pub(crate) fn build_backtest_timeline(
    history: &[AssessmentHistoryPoint],
    use_transitional_bridge: bool,
    strict_thresholds: Option<ProbabilityActionThresholds>,
) -> Vec<BacktestWindowPoint> {
    history
        .iter()
        .map(|point| BacktestWindowPoint {
            as_of_date: point.as_of_date,
            overall_score: point.overall_score,
            p_5d: point.p_5d,
            p_20d: point.p_20d,
            p_60d: point.p_60d,
            posture: point.posture,
            crisis_window_open: is_actionable_warning_point(
                point,
                use_transitional_bridge,
                strict_thresholds,
            ),
        })
        .collect()
}

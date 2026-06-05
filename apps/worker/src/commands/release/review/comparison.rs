use std::collections::{BTreeMap, BTreeSet};

use fc_domain::AssessmentHistoryPoint;

use super::{
    build_release_review_backtest_scenario_comparisons, release_review_structured_signal_counts,
};

pub(super) struct ReleaseReviewComparisonInput<'a> {
    pub(super) assessment: &'a fc_domain::AssessmentSnapshot,
    pub(super) backtests: &'a [fc_domain::BacktestScenarioSummary],
    pub(super) history: &'a [AssessmentHistoryPoint],
    pub(super) method: &'a crate::AuditMethodResponseWire,
}

pub(super) fn build_release_review_comparison(
    baseline: ReleaseReviewComparisonInput<'_>,
    candidate: ReleaseReviewComparisonInput<'_>,
    baseline_runtime_review: &crate::ReleaseRuntimeReviewDiagnostics,
    candidate_runtime_review: &crate::ReleaseRuntimeReviewDiagnostics,
) -> crate::ReleaseReviewComparisonSummary {
    let (baseline_strict_actionable_point_count, baseline_runtime_floor_hit_count) =
        release_review_structured_signal_counts(
            baseline.backtests,
            baseline.history,
            baseline.method,
        );
    let (candidate_strict_actionable_point_count, candidate_runtime_floor_hit_count) =
        release_review_structured_signal_counts(
            candidate.backtests,
            candidate.history,
            candidate.method,
        );
    crate::ReleaseReviewComparisonSummary {
        timely_warning_rate: scalar_metric(
            baseline.assessment.backtest_summary.timely_warning_rate,
            candidate.assessment.backtest_summary.timely_warning_rate,
        ),
        strict_actionable_point_count: count_metric(
            baseline_strict_actionable_point_count,
            candidate_strict_actionable_point_count,
        ),
        runtime_floor_hit_count: count_metric(
            baseline_runtime_floor_hit_count,
            candidate_runtime_floor_hit_count,
        ),
        actionable_precision: scalar_metric(
            baseline
                .assessment
                .backtest_summary
                .rolling_audit
                .actionable_precision,
            candidate
                .assessment
                .backtest_summary
                .rolling_audit
                .actionable_precision,
        ),
        longest_false_positive_episode_days: count_metric(
            baseline
                .assessment
                .backtest_summary
                .rolling_audit
                .longest_false_positive_episode_days,
            candidate
                .assessment
                .backtest_summary
                .rolling_audit
                .longest_false_positive_episode_days,
        ),
        current_p_5d: scalar_metric(
            baseline.assessment.probabilities.p_5d,
            candidate.assessment.probabilities.p_5d,
        ),
        current_p_20d: scalar_metric(
            baseline.assessment.probabilities.p_20d,
            candidate.assessment.probabilities.p_20d,
        ),
        current_p_60d: scalar_metric(
            baseline.assessment.probabilities.p_60d,
            candidate.assessment.probabilities.p_60d,
        ),
        runtime_separation_summary: build_release_review_runtime_separation_comparisons(
            baseline_runtime_review,
            candidate_runtime_review,
        ),
        backtest_scenarios: build_release_review_backtest_scenario_comparisons(
            baseline.backtests,
            candidate.backtests,
        ),
    }
}

pub(crate) fn build_release_review_runtime_separation_comparisons(
    baseline: &crate::ReleaseRuntimeReviewDiagnostics,
    candidate: &crate::ReleaseRuntimeReviewDiagnostics,
) -> Vec<crate::ReleaseReviewRuntimeSeparationComparison> {
    let baseline_by_horizon = baseline
        .regime_separation_summaries
        .iter()
        .map(|summary| (summary.horizon_days, summary))
        .collect::<BTreeMap<_, _>>();
    let candidate_by_horizon = candidate
        .regime_separation_summaries
        .iter()
        .map(|summary| (summary.horizon_days, summary))
        .collect::<BTreeMap<_, _>>();
    let horizons = baseline_by_horizon
        .keys()
        .chain(candidate_by_horizon.keys())
        .copied()
        .collect::<BTreeSet<_>>();

    horizons
        .into_iter()
        .map(|horizon_days| {
            let baseline_summary = baseline_by_horizon.get(&horizon_days).copied();
            let candidate_summary = candidate_by_horizon.get(&horizon_days).copied();
            let baseline_threshold = release_review_runtime_threshold_for_horizon(
                baseline.runtime_thresholds.as_ref(),
                horizon_days,
            );
            let candidate_threshold = release_review_runtime_threshold_for_horizon(
                candidate.runtime_thresholds.as_ref(),
                horizon_days,
            );
            let baseline_early_warning_avg_probability =
                baseline_summary.and_then(release_review_early_warning_avg_probability);
            let candidate_early_warning_avg_probability =
                candidate_summary.and_then(release_review_early_warning_avg_probability);
            let baseline_normal_avg_probability =
                baseline_summary.map(|summary| summary.normal_avg_probability);
            let candidate_normal_avg_probability =
                candidate_summary.map(|summary| summary.normal_avg_probability);

            crate::ReleaseReviewRuntimeSeparationComparison {
                horizon_days,
                baseline_diagnosis: baseline_summary
                    .map(|summary| summary.diagnosis.clone())
                    .unwrap_or_else(|| "missing".to_string()),
                candidate_diagnosis: candidate_summary
                    .map(|summary| summary.diagnosis.clone())
                    .unwrap_or_else(|| "missing".to_string()),
                baseline_threshold,
                candidate_threshold,
                baseline_early_warning_regime: baseline_summary
                    .map(|summary| summary.early_warning_regime.clone())
                    .unwrap_or_else(|| "—".to_string()),
                candidate_early_warning_regime: candidate_summary
                    .map(|summary| summary.early_warning_regime.clone())
                    .unwrap_or_else(|| "—".to_string()),
                baseline_early_warning_avg_probability,
                candidate_early_warning_avg_probability,
                baseline_normal_avg_probability,
                candidate_normal_avg_probability,
                baseline_early_warning_gap_vs_normal: baseline_summary
                    .and_then(release_review_early_warning_gap_vs_normal),
                candidate_early_warning_gap_vs_normal: candidate_summary
                    .and_then(release_review_early_warning_gap_vs_normal),
                baseline_floor_gap: release_review_runtime_floor_gap(
                    baseline_early_warning_avg_probability,
                    baseline_threshold,
                ),
                candidate_floor_gap: release_review_runtime_floor_gap(
                    candidate_early_warning_avg_probability,
                    candidate_threshold,
                ),
                baseline_early_warning_lift_vs_normal: baseline_summary
                    .and_then(|summary| summary.early_warning_calibrated_lift_vs_normal),
                candidate_early_warning_lift_vs_normal: candidate_summary
                    .and_then(|summary| summary.early_warning_calibrated_lift_vs_normal),
                baseline_threshold_hit_rate: baseline_summary
                    .and_then(|summary| summary.max_non_normal_threshold_hit_rate),
                candidate_threshold_hit_rate: candidate_summary
                    .and_then(|summary| summary.max_non_normal_threshold_hit_rate),
            }
        })
        .collect()
}

fn release_review_runtime_threshold_for_horizon(
    runtime_thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
    horizon_days: u32,
) -> Option<f64> {
    runtime_thresholds.map(|thresholds| match horizon_days {
        5 => thresholds.defend_p5d,
        20 => thresholds.hedge_p20d,
        60 => thresholds.prepare_p60d,
        _ => 1.0,
    })
}

fn release_review_early_warning_avg_probability(
    summary: &crate::ReleaseRuntimeSeparationSummary,
) -> Option<f64> {
    match summary.early_warning_regime.as_str() {
        "positive_window" => Some(summary.positive_window_avg_probability),
        "pre_warning_buffer" => Some(summary.pre_warning_buffer_avg_probability),
        _ => None,
    }
}

fn release_review_early_warning_gap_vs_normal(
    summary: &crate::ReleaseRuntimeSeparationSummary,
) -> Option<f64> {
    release_review_early_warning_avg_probability(summary)
        .map(|value| crate::round6(value - summary.normal_avg_probability))
}

fn release_review_runtime_floor_gap(
    early_warning_avg_probability: Option<f64>,
    threshold: Option<f64>,
) -> Option<f64> {
    match (early_warning_avg_probability, threshold) {
        (Some(early_warning_avg_probability), Some(threshold)) => {
            Some(crate::round6(early_warning_avg_probability - threshold))
        }
        _ => None,
    }
}

fn scalar_metric(baseline: f64, candidate: f64) -> crate::ReleaseReviewScalarMetric {
    crate::ReleaseReviewScalarMetric {
        baseline,
        candidate,
        delta: candidate - baseline,
    }
}

fn count_metric(baseline: u32, candidate: u32) -> crate::ReleaseReviewCountMetric {
    crate::ReleaseReviewCountMetric {
        baseline,
        candidate,
        delta: i64::from(candidate) - i64::from(baseline),
    }
}

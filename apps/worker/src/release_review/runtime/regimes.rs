use std::collections::BTreeMap;

use fc_domain::AssessmentHistoryPoint;

use crate::{
    classify_probability_regime_separation, early_warning_regime_name,
    forward_crisis_training_regime, gap_retention_ratio, lift_vs_baseline,
    probability_training_regime_name, round6, safe_divide, safe_ratio, CrisisScenario,
    RuntimeThresholdDiagnosticsWire,
};

use super::super::{ReleaseRuntimeRegimeProbabilitySummary, ReleaseRuntimeSeparationSummary};

pub(crate) fn summarize_release_runtime_regime_probabilities(
    history: &[AssessmentHistoryPoint],
    scenarios: &[CrisisScenario],
    runtime_thresholds: Option<&RuntimeThresholdDiagnosticsWire>,
) -> Vec<ReleaseRuntimeRegimeProbabilitySummary> {
    #[derive(Default)]
    struct Accumulator {
        row_count: usize,
        raw_probability_sum: f64,
        max_raw_probability: f64,
        calibrated_probability_sum: f64,
        max_calibrated_probability: f64,
        threshold_hit_count: usize,
    }

    let mut buckets = BTreeMap::<(u32, String), Accumulator>::new();
    for point in history {
        for (horizon_days, raw_probability, calibrated_probability) in [
            (5_u32, point.raw_p_5d.unwrap_or(point.p_5d), point.p_5d),
            (20_u32, point.raw_p_20d.unwrap_or(point.p_20d), point.p_20d),
            (60_u32, point.raw_p_60d.unwrap_or(point.p_60d), point.p_60d),
        ] {
            let regime = probability_training_regime_name(forward_crisis_training_regime(
                point.as_of_date,
                scenarios,
                horizon_days,
            ));
            let bucket = buckets
                .entry((horizon_days, regime.to_string()))
                .or_default();
            bucket.row_count += 1;
            bucket.raw_probability_sum += raw_probability;
            bucket.max_raw_probability = bucket.max_raw_probability.max(raw_probability);
            bucket.calibrated_probability_sum += calibrated_probability;
            bucket.max_calibrated_probability = bucket
                .max_calibrated_probability
                .max(calibrated_probability);
            if let Some(threshold) =
                runtime_probability_threshold_for_horizon(runtime_thresholds, horizon_days)
            {
                if calibrated_probability >= threshold {
                    bucket.threshold_hit_count += 1;
                }
            }
        }
    }

    let normal_baselines = buckets
        .iter()
        .filter_map(|((horizon_days, regime), bucket)| {
            if regime != "normal" {
                return None;
            }
            Some((
                *horizon_days,
                (
                    safe_divide(bucket.raw_probability_sum, bucket.row_count as f64),
                    safe_divide(bucket.calibrated_probability_sum, bucket.row_count as f64),
                ),
            ))
        })
        .collect::<BTreeMap<_, _>>();

    buckets
        .into_iter()
        .map(|((horizon_days, regime), bucket)| {
            let avg_raw_probability =
                safe_divide(bucket.raw_probability_sum, bucket.row_count as f64);
            let avg_calibrated_probability =
                safe_divide(bucket.calibrated_probability_sum, bucket.row_count as f64);
            let (
                raw_lift_vs_normal,
                calibrated_lift_vs_normal,
                raw_gap_vs_normal,
                calibrated_gap_vs_normal,
                calibration_gap_retention,
            ) = if let Some((normal_avg_raw, normal_avg_calibrated)) =
                normal_baselines.get(&horizon_days).copied()
            {
                let raw_gap = avg_raw_probability - normal_avg_raw;
                let calibrated_gap = avg_calibrated_probability - normal_avg_calibrated;
                (
                    lift_vs_baseline(avg_raw_probability, normal_avg_raw),
                    lift_vs_baseline(avg_calibrated_probability, normal_avg_calibrated),
                    Some(round6(raw_gap)),
                    Some(round6(calibrated_gap)),
                    gap_retention_ratio(raw_gap, calibrated_gap),
                )
            } else {
                (None, None, None, None, None)
            };

            ReleaseRuntimeRegimeProbabilitySummary {
                horizon_days,
                regime,
                row_count: bucket.row_count,
                row_rate: round6(safe_ratio(bucket.row_count, history.len())),
                avg_raw_probability: round6(avg_raw_probability),
                max_raw_probability: round6(bucket.max_raw_probability),
                avg_probability: round6(avg_calibrated_probability),
                max_probability: round6(bucket.max_calibrated_probability),
                raw_lift_vs_normal,
                calibrated_lift_vs_normal,
                raw_gap_vs_normal,
                calibrated_gap_vs_normal,
                calibration_gap_retention,
                threshold_hit_count: runtime_thresholds.map(|_| bucket.threshold_hit_count),
            }
        })
        .collect()
}

pub(crate) fn summarize_release_runtime_regime_separation(
    summaries: &[ReleaseRuntimeRegimeProbabilitySummary],
) -> Vec<ReleaseRuntimeSeparationSummary> {
    let mut by_horizon = BTreeMap::<u32, Vec<&ReleaseRuntimeRegimeProbabilitySummary>>::new();
    for summary in summaries {
        by_horizon
            .entry(summary.horizon_days)
            .or_default()
            .push(summary);
    }

    by_horizon
        .into_iter()
        .filter_map(|(horizon_days, rows)| {
            let normal = rows.iter().copied().find(|row| row.regime == "normal")?;
            let pre_warning_buffer = rows
                .iter()
                .copied()
                .find(|row| row.regime == "pre_warning_buffer");
            let positive_window = rows
                .iter()
                .copied()
                .find(|row| row.regime == "positive_window");
            let max_non_normal = rows
                .iter()
                .copied()
                .filter(|row| row.regime != "normal")
                .max_by(|left, right| {
                    left.avg_probability
                        .partial_cmp(&right.avg_probability)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })?;
            let early_warning_regime = early_warning_regime_name(horizon_days);
            let early_warning = rows
                .iter()
                .copied()
                .find(|row| row.regime == early_warning_regime);
            let in_crisis = rows.iter().copied().find(|row| row.regime == "in_crisis");
            let post_crisis_cooldown = rows
                .iter()
                .copied()
                .find(|row| row.regime == "post_crisis_cooldown");
            let max_non_normal_threshold_hit_rate = max_non_normal
                .threshold_hit_count
                .map(|count| round6(safe_divide(count as f64, max_non_normal.row_count as f64)));
            let diagnosis = classify_regime_separation(
                horizon_days,
                early_warning
                    .and_then(|row| row.raw_lift_vs_normal)
                    .unwrap_or_default(),
                early_warning
                    .and_then(|row| row.calibrated_lift_vs_normal)
                    .unwrap_or_default(),
                early_warning.and_then(|row| row.calibration_gap_retention),
                positive_window
                    .and_then(|row| row.calibrated_lift_vs_normal)
                    .unwrap_or_default(),
                positive_window
                    .and_then(|row| row.calibrated_gap_vs_normal)
                    .unwrap_or_default(),
                in_crisis
                    .and_then(|row| row.calibrated_lift_vs_normal)
                    .unwrap_or_default(),
                post_crisis_cooldown
                    .and_then(|row| row.calibrated_lift_vs_normal)
                    .unwrap_or_default(),
                post_crisis_cooldown
                    .and_then(|row| row.calibrated_gap_vs_normal)
                    .unwrap_or_default(),
                max_non_normal.calibrated_lift_vs_normal.unwrap_or_default(),
                max_non_normal_threshold_hit_rate.unwrap_or_default(),
            )
            .to_string();

            Some(ReleaseRuntimeSeparationSummary {
                horizon_days,
                early_warning_regime: early_warning_regime.to_string(),
                normal_avg_probability: normal.avg_probability,
                pre_warning_buffer_avg_probability: pre_warning_buffer
                    .map(|row| row.avg_probability)
                    .unwrap_or_default(),
                positive_window_avg_probability: positive_window
                    .map(|row| row.avg_probability)
                    .unwrap_or_default(),
                in_crisis_avg_probability: in_crisis
                    .map(|row| row.avg_probability)
                    .unwrap_or_default(),
                post_crisis_cooldown_avg_probability: post_crisis_cooldown
                    .map(|row| row.avg_probability)
                    .unwrap_or_default(),
                early_warning_raw_lift_vs_normal: early_warning
                    .and_then(|row| row.raw_lift_vs_normal),
                early_warning_calibrated_lift_vs_normal: early_warning
                    .and_then(|row| row.calibrated_lift_vs_normal),
                early_warning_gap_retention: early_warning
                    .and_then(|row| row.calibration_gap_retention),
                positive_window_calibrated_lift_vs_normal: positive_window
                    .and_then(|row| row.calibrated_lift_vs_normal),
                positive_window_gap_vs_normal: positive_window
                    .and_then(|row| row.calibrated_gap_vs_normal),
                in_crisis_raw_lift_vs_normal: in_crisis.and_then(|row| row.raw_lift_vs_normal),
                in_crisis_calibrated_lift_vs_normal: in_crisis
                    .and_then(|row| row.calibrated_lift_vs_normal),
                post_crisis_cooldown_calibrated_lift_vs_normal: post_crisis_cooldown
                    .and_then(|row| row.calibrated_lift_vs_normal),
                post_crisis_cooldown_gap_vs_normal: post_crisis_cooldown
                    .and_then(|row| row.calibrated_gap_vs_normal),
                max_non_normal_calibrated_lift_vs_normal: max_non_normal.calibrated_lift_vs_normal,
                max_non_normal_threshold_hit_rate,
                diagnosis,
            })
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn classify_regime_separation(
    horizon_days: u32,
    early_warning_raw_lift: f64,
    early_warning_calibrated_lift: f64,
    early_warning_gap_retention: Option<f64>,
    positive_window_calibrated_lift: f64,
    positive_window_gap_vs_normal: f64,
    in_crisis_calibrated_lift: f64,
    post_crisis_cooldown_calibrated_lift: f64,
    post_crisis_cooldown_gap_vs_normal: f64,
    max_non_normal_calibrated_lift: f64,
    max_non_normal_threshold_hit_rate: f64,
) -> &'static str {
    if early_warning_raw_lift >= 1.5
        && early_warning_calibrated_lift < 1.15
        && early_warning_gap_retention.unwrap_or_default() < 0.35
    {
        return "calibration_crushed_early_warning";
    }

    let shared_diagnosis = classify_probability_regime_separation(
        horizon_days,
        early_warning_calibrated_lift,
        positive_window_calibrated_lift,
        early_warning_raw_lift,
        in_crisis_calibrated_lift,
        post_crisis_cooldown_calibrated_lift,
        positive_window_gap_vs_normal,
        post_crisis_cooldown_gap_vs_normal,
        max_non_normal_calibrated_lift,
    );

    if matches!(
        shared_diagnosis,
        "cold_across_all_regimes" | "late_only_no_early_warning" | "cooldown_bleed"
    ) {
        return shared_diagnosis;
    }

    if max_non_normal_calibrated_lift >= 1.5 && max_non_normal_threshold_hit_rate <= 0.01 {
        return "separated_but_below_runtime_floor";
    }
    shared_diagnosis
}

fn runtime_probability_threshold_for_horizon(
    runtime_thresholds: Option<&RuntimeThresholdDiagnosticsWire>,
    horizon_days: u32,
) -> Option<f64> {
    runtime_thresholds.map(|thresholds| match horizon_days {
        5 => thresholds.defend_p5d,
        20 => thresholds.hedge_p20d,
        60 => thresholds.prepare_p60d,
        _ => 1.0,
    })
}

use super::super::ProbabilityThresholdRegimeHitSummary;
use super::{
    metrics::probability_threshold_regime_hit_summary,
    selection::{
        probability_prediction_count_ceiling_from_actual_positive_count,
        probability_threshold_beta_sq, probability_threshold_candidates,
    },
};

pub(in super::super) fn regime_aware_threshold_prediction_ceiling(
    actual_positive_count: u32,
    horizon_days: u32,
) -> u32 {
    let base = probability_prediction_count_ceiling_from_actual_positive_count(
        actual_positive_count,
        horizon_days,
    );
    match horizon_days {
        60 => base.saturating_mul(3),
        20 => base.saturating_mul(2),
        _ => base,
    }
}

fn regime_floor_min_hit_rate(horizon_days: u32) -> f64 {
    match horizon_days {
        60 => 0.05,
        20 => 0.03,
        _ => 0.0,
    }
}

fn regime_floor_min_gap_vs_normal(horizon_days: u32) -> f64 {
    match horizon_days {
        60 => 0.02,
        20 => 0.01,
        _ => 0.0,
    }
}

fn regime_floor_over_tight_base_threshold(horizon_days: u32) -> f64 {
    match horizon_days {
        60 => 0.75,
        20 => 0.85,
        _ => 1.0,
    }
}

fn regime_positive_window_min_hit_rate(horizon_days: u32) -> f64 {
    match horizon_days {
        20 => 0.25,
        60 => 0.10,
        _ => regime_floor_min_hit_rate(horizon_days),
    }
}

fn probability_threshold_prediction_counts(
    probabilities: &[f64],
    labels: &[f64],
    threshold: f64,
) -> (u32, u32) {
    let mut true_positive_count = 0_u32;
    let mut predicted_positive_count = 0_u32;
    for (probability, label) in probabilities.iter().zip(labels) {
        if *probability >= threshold {
            predicted_positive_count += 1;
            if *label >= 0.5 {
                true_positive_count += 1;
            }
        }
    }

    (true_positive_count, predicted_positive_count)
}

pub(in super::super) fn threshold_has_usable_early_warning_support(
    hits: ProbabilityThresholdRegimeHitSummary,
    horizon_days: u32,
) -> bool {
    hits.early_warning_hit_count > 0
        && hits.early_warning_hit_rate() >= regime_floor_min_hit_rate(horizon_days)
        && (hits.early_warning_hit_rate() - hits.normal_hit_rate())
            >= regime_floor_min_gap_vs_normal(horizon_days)
}

fn threshold_has_usable_positive_window_support(
    hits: ProbabilityThresholdRegimeHitSummary,
    horizon_days: u32,
) -> bool {
    if !matches!(horizon_days, 20 | 60) || hits.positive_window_row_count == 0 {
        return true;
    }

    let positive_window_hit_rate = hits.positive_window_hit_rate();
    hits.positive_window_hit_count > 0
        && positive_window_hit_rate >= regime_positive_window_min_hit_rate(horizon_days)
        && (positive_window_hit_rate - hits.normal_hit_rate())
            >= regime_floor_min_gap_vs_normal(horizon_days)
        && (hits.cooldown_row_count == 0
            || positive_window_hit_rate + 1e-9 >= hits.cooldown_hit_rate())
}

pub(in super::super) fn threshold_has_usable_forward_crisis_support(
    hits: ProbabilityThresholdRegimeHitSummary,
    horizon_days: u32,
) -> bool {
    threshold_has_usable_early_warning_support(hits, horizon_days)
        && threshold_has_usable_positive_window_support(hits, horizon_days)
}

fn threshold_has_usable_repair_candidate_support(
    hits: ProbabilityThresholdRegimeHitSummary,
    horizon_days: u32,
) -> bool {
    let positive_window_supported =
        threshold_has_usable_positive_window_support(hits, horizon_days);
    if horizon_days == 20 {
        return positive_window_supported;
    }

    threshold_has_usable_early_warning_support(hits, horizon_days) && positive_window_supported
}

fn threshold_has_over_tight_repair_candidate(
    probabilities: &[f64],
    labels: &[f64],
    rows: &[&crate::ProbabilityTrainingRow],
    horizon_days: u32,
    base_threshold: f64,
    relaxed_prediction_ceiling: u32,
) -> bool {
    for threshold in probability_threshold_candidates(probabilities) {
        if threshold >= base_threshold {
            continue;
        }

        let hits =
            probability_threshold_regime_hit_summary(probabilities, rows, horizon_days, threshold);
        if !threshold_has_usable_repair_candidate_support(hits, horizon_days) {
            continue;
        }

        let (true_positive_count, predicted_positive_count) =
            probability_threshold_prediction_counts(probabilities, labels, threshold);
        if true_positive_count > 0
            && predicted_positive_count > 0
            && predicted_positive_count <= relaxed_prediction_ceiling
        {
            return true;
        }
    }

    false
}

pub(crate) fn adjust_probability_decision_threshold_for_regime_support(
    base_threshold: f64,
    probabilities: &[f64],
    labels: &[f64],
    rows: &[&crate::ProbabilityTrainingRow],
    horizon_days: u32,
    label_mode: crate::ProbabilityTargetLabelMode,
) -> f64 {
    if label_mode != crate::ProbabilityTargetLabelMode::ForwardCrisis
        || !matches!(horizon_days, 20 | 60)
        || probabilities.is_empty()
        || rows.is_empty()
        || probabilities.len() != rows.len()
    {
        return base_threshold;
    }

    let Some(regime_summary) = super::super::super::evaluate_regime_separation_summary_refs(
        probabilities,
        rows,
        horizon_days,
        label_mode,
    ) else {
        return base_threshold;
    };

    let base_hits =
        probability_threshold_regime_hit_summary(probabilities, rows, horizon_days, base_threshold);
    if threshold_has_usable_forward_crisis_support(base_hits, horizon_days) {
        return base_threshold;
    }

    let actual_positive_count = labels.iter().filter(|label| **label >= 0.5).count() as u32;
    let positive_count = actual_positive_count as f64;
    if positive_count <= 0.0 {
        return base_threshold;
    }

    let early_warning_regime = super::super::super::probability_early_warning_regime(horizon_days);
    let early_warning_probability_cap = probabilities
        .iter()
        .zip(rows.iter().copied())
        .filter(|(_, row)| row.regime_for_horizon(horizon_days) == early_warning_regime)
        .map(|(probability, _)| *probability)
        .fold(0.0_f64, f64::max);

    let relaxed_prediction_ceiling =
        regime_aware_threshold_prediction_ceiling(actual_positive_count, horizon_days);
    let early_warning_cap_candidate =
        if early_warning_probability_cap > 0.0 && early_warning_probability_cap < base_threshold {
            Some(crate::round3(early_warning_probability_cap).clamp(0.005, base_threshold))
        } else {
            None
        };
    let early_warning_cap_has_usable_support =
        early_warning_cap_candidate.is_some_and(|threshold| {
            let hits = probability_threshold_regime_hit_summary(
                probabilities,
                rows,
                horizon_days,
                threshold,
            );
            threshold_has_usable_early_warning_support(hits, horizon_days)
        });
    let over_tight_base_threshold = base_threshold
        >= regime_floor_over_tight_base_threshold(horizon_days)
        && (early_warning_cap_has_usable_support
            || threshold_has_over_tight_repair_candidate(
                probabilities,
                labels,
                rows,
                horizon_days,
                base_threshold,
                relaxed_prediction_ceiling,
            ));
    if regime_summary
        .early_warning_lift_vs_normal
        .unwrap_or_default()
        < 1.5
        && !over_tight_base_threshold
    {
        return base_threshold;
    }

    let beta_sq = probability_threshold_beta_sq(horizon_days);
    let mut best_score = None::<(bool, bool, i64, i64, i64, i64, i64, i64, i64)>;
    let mut best_threshold = base_threshold;

    for threshold in probability_threshold_candidates(probabilities) {
        if threshold >= base_threshold {
            continue;
        }

        let hits =
            probability_threshold_regime_hit_summary(probabilities, rows, horizon_days, threshold);
        let early_warning_hit_rate = hits.early_warning_hit_rate();
        if hits.early_warning_hit_count == 0 {
            continue;
        }
        if !threshold_has_usable_repair_candidate_support(hits, horizon_days) {
            continue;
        }

        let (true_positive_count, predicted_positive_count) =
            probability_threshold_prediction_counts(probabilities, labels, threshold);
        if predicted_positive_count == 0 || true_positive_count == 0 {
            continue;
        }

        let precision = true_positive_count as f64 / predicted_positive_count as f64;
        let recall = true_positive_count as f64 / positive_count;
        let f_beta = if precision > 0.0 || recall > 0.0 {
            (1.0 + beta_sq) * precision * recall / (beta_sq * precision + recall).max(1e-9)
        } else {
            0.0
        };

        let normal_hit_rate = hits.normal_hit_rate();
        let cooldown_hit_rate = hits.cooldown_hit_rate();
        let score = (
            early_warning_hit_rate >= regime_floor_min_hit_rate(horizon_days),
            predicted_positive_count <= relaxed_prediction_ceiling,
            ((early_warning_hit_rate - normal_hit_rate) * 1_000_000.0).round() as i64,
            ((hits.positive_window_hit_rate() - cooldown_hit_rate) * 1_000_000.0).round() as i64,
            ((hits.in_crisis_hit_rate() - cooldown_hit_rate) * 1_000_000.0).round() as i64,
            (f_beta * 1_000_000.0).round() as i64,
            (precision * 1_000_000.0).round() as i64,
            (recall * 1_000_000.0).round() as i64,
            -((threshold * 1_000.0).round() as i64),
        );
        if best_score.is_none_or(|best| score > best) {
            best_score = Some(score);
            best_threshold = threshold;
        }
    }

    let repaired_threshold =
        if early_warning_probability_cap > 0.0 && early_warning_probability_cap < base_threshold {
            best_threshold.min(early_warning_probability_cap)
        } else {
            best_threshold
        };

    crate::round3(repaired_threshold).clamp(0.005, base_threshold)
}

use super::{
    ProbabilityThresholdDecisionMetrics, ProbabilityThresholdRegimeHitSummary,
    ProbabilityThresholdScoreInputs, ProbabilityThresholdSelection,
};

pub(crate) fn probability_decision_threshold_selection<'a>(
    probabilities: &[f64],
    labels: &[f64],
    rows: &[&'a crate::ProbabilityTrainingRow],
    horizon_days: u32,
    label_mode: crate::ProbabilityTargetLabelMode,
) -> ProbabilityThresholdSelection<'a> {
    let mut filtered_rows = Vec::new();
    let mut filtered_probabilities = Vec::new();
    let mut filtered_labels = Vec::new();
    let mut filtered_positive_count = 0_usize;
    let mut filtered_negative_count = 0_usize;

    for ((probability, label), row) in probabilities.iter().zip(labels).zip(rows.iter().copied()) {
        if !probability_row_is_threshold_eligible(row, horizon_days, label_mode) {
            continue;
        }
        filtered_rows.push(row);
        filtered_probabilities.push(*probability);
        filtered_labels.push(*label);
        if *label >= 0.5 {
            filtered_positive_count += 1;
        } else {
            filtered_negative_count += 1;
        }
    }

    if filtered_positive_count > 0 && filtered_negative_count > 0 {
        ProbabilityThresholdSelection {
            rows: filtered_rows,
            probabilities: filtered_probabilities,
            labels: filtered_labels,
            used_full_split_fallback: false,
        }
    } else {
        ProbabilityThresholdSelection {
            rows: rows.to_vec(),
            probabilities: probabilities.to_vec(),
            labels: labels.to_vec(),
            used_full_split_fallback: true,
        }
    }
}

fn probability_row_is_threshold_eligible(
    row: &crate::ProbabilityTrainingRow,
    horizon_days: u32,
    label_mode: crate::ProbabilityTargetLabelMode,
) -> bool {
    if row.label_for_horizon(label_mode, horizon_days) > 0.0 {
        return true;
    }

    match label_mode {
        crate::ProbabilityTargetLabelMode::ActionWindow
        | crate::ProbabilityTargetLabelMode::ActionEpisode => true,
        crate::ProbabilityTargetLabelMode::ForwardCrisis => match horizon_days {
            20 | 60 => matches!(
                row.regime_for_horizon(horizon_days),
                crate::ProbabilityTrainingRegime::Normal
                    | crate::ProbabilityTrainingRegime::PreWarningBuffer
                    | crate::ProbabilityTrainingRegime::PostCrisisCooldown
            ),
            _ => matches!(
                row.regime_for_horizon(horizon_days),
                crate::ProbabilityTrainingRegime::Normal
            ),
        },
    }
}

pub(crate) fn select_probability_decision_threshold(
    probabilities: &[f64],
    labels: &[f64],
    horizon_days: u32,
) -> f64 {
    let thresholds = probability_decision_threshold_candidates(probabilities);

    let actual_positive_count = labels.iter().filter(|label| **label >= 0.5).count() as u32;
    let positive_count = actual_positive_count as f64;
    let prediction_ceiling = probability_prediction_count_ceiling_from_actual_positive_count(
        actual_positive_count,
        horizon_days,
    );
    let mut best_threshold = 0.3;
    let beta_sq = probability_threshold_beta_sq(horizon_days);
    let mut best_score = None::<(i64, i64, i64, i64, i64)>;
    let mut best_capped_threshold = None::<f64>;
    let mut best_capped_score = None::<(i64, i64, i64, i64, i64)>;
    for threshold in thresholds {
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
        if predicted_positive_count == 0 || positive_count <= 0.0 {
            continue;
        }
        let minimum_true_positives = (positive_count.min(2.0)) as u32;
        if true_positive_count < minimum_true_positives.max(1) {
            continue;
        }
        let precision = true_positive_count as f64 / predicted_positive_count as f64;
        let recall = true_positive_count as f64 / positive_count;
        let f_beta = if precision > 0.0 || recall > 0.0 {
            (1.0 + beta_sq) * precision * recall / (beta_sq * precision + recall).max(1e-9)
        } else {
            0.0
        };
        let score = probability_threshold_score_tuple(ProbabilityThresholdScoreInputs {
            horizon_days,
            precision,
            recall,
            f_beta,
            threshold,
            predicted_positive_count,
            prediction_ceiling,
            actual_positive_count,
        });
        if best_score.is_none_or(|best| score > best) {
            best_score = Some(score);
            best_threshold = threshold;
        }
        if predicted_positive_count <= prediction_ceiling
            && best_capped_score.is_none_or(|best| score > best)
        {
            best_capped_score = Some(score);
            best_capped_threshold = Some(threshold);
        }
    }

    let minimum_threshold = match horizon_days {
        5 => 0.03,
        20 => 0.005,
        60 => 0.01,
        _ => 0.001,
    };

    crate::round3(best_capped_threshold.unwrap_or(best_threshold)).clamp(minimum_threshold, 0.90)
}

fn probability_decision_threshold_candidates(probabilities: &[f64]) -> Vec<f64> {
    let mut thresholds = probabilities
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .filter(|value| (0.001..0.99).contains(value))
        .collect::<Vec<_>>();
    thresholds.extend((1..=20).map(|value| value as f64 / 1_000.0));
    thresholds.extend((2..=90).map(|value| value as f64 / 100.0));
    thresholds.push(0.3);
    thresholds.sort_by(f64::total_cmp);
    thresholds.dedup_by(|left, right| (*left - *right).abs() < 1e-6);
    thresholds
}

fn probability_threshold_beta_sq(horizon_days: u32) -> f64 {
    match horizon_days {
        5 => 0.25,
        20 => 1.0,
        60 => 2.25,
        _ => 1.0,
    }
}

pub(super) fn probability_threshold_score_tuple(
    inputs: ProbabilityThresholdScoreInputs,
) -> (i64, i64, i64, i64, i64) {
    let ProbabilityThresholdScoreInputs {
        horizon_days,
        precision,
        recall,
        f_beta,
        threshold,
        predicted_positive_count,
        prediction_ceiling,
        actual_positive_count,
    } = inputs;
    let precision_score = (precision * 1_000_000.0).round() as i64;
    let recall_score = (recall * 1_000_000.0).round() as i64;
    let f_beta_score = (f_beta * 1_000_000.0).round() as i64;
    let threshold_score = (threshold * 1_000.0).round() as i64;
    let overprediction_score = probability_threshold_overprediction_score(
        horizon_days,
        predicted_positive_count,
        prediction_ceiling,
        actual_positive_count,
    );
    let adjusted_f_beta_score = if horizon_days == 20 {
        f_beta_score + overprediction_score
    } else {
        f_beta_score
    };

    match horizon_days {
        5 => (
            precision_score,
            f_beta_score,
            recall_score,
            overprediction_score,
            threshold_score,
        ),
        20 => (
            adjusted_f_beta_score,
            precision_score,
            recall_score,
            threshold_score,
            overprediction_score,
        ),
        60 => (
            f_beta_score,
            recall_score,
            precision_score,
            overprediction_score,
            threshold_score,
        ),
        _ => (
            f_beta_score,
            precision_score,
            recall_score,
            overprediction_score,
            threshold_score,
        ),
    }
}

fn probability_threshold_overprediction_score(
    horizon_days: u32,
    predicted_positive_count: u32,
    prediction_ceiling: u32,
    actual_positive_count: u32,
) -> i64 {
    if horizon_days != 20 || actual_positive_count == 0 {
        return 0;
    }

    let overflow = predicted_positive_count.saturating_sub(prediction_ceiling) as f64;
    -((overflow / actual_positive_count as f64) * 1_000.0).round() as i64
}

impl ProbabilityThresholdRegimeHitSummary {
    pub(super) fn early_warning_hit_rate(self) -> f64 {
        crate::safe_divide(
            self.early_warning_hit_count as f64,
            self.early_warning_row_count as f64,
        )
    }

    pub(super) fn normal_hit_rate(self) -> f64 {
        crate::safe_divide(self.normal_hit_count as f64, self.normal_row_count as f64)
    }

    pub(super) fn positive_window_hit_rate(self) -> f64 {
        crate::safe_divide(
            self.positive_window_hit_count as f64,
            self.positive_window_row_count as f64,
        )
    }

    pub(super) fn in_crisis_hit_rate(self) -> f64 {
        crate::safe_divide(
            self.in_crisis_hit_count as f64,
            self.in_crisis_row_count as f64,
        )
    }

    pub(super) fn cooldown_hit_rate(self) -> f64 {
        crate::safe_divide(
            self.cooldown_hit_count as f64,
            self.cooldown_row_count as f64,
        )
    }
}

pub(super) fn probability_threshold_regime_hit_summary(
    probabilities: &[f64],
    rows: &[&crate::ProbabilityTrainingRow],
    horizon_days: u32,
    threshold: f64,
) -> ProbabilityThresholdRegimeHitSummary {
    let early_warning_regime = super::super::probability_early_warning_regime(horizon_days);

    let mut summary = ProbabilityThresholdRegimeHitSummary::default();
    for (probability, row) in probabilities.iter().zip(rows.iter().copied()) {
        let regime = row.regime_for_horizon(horizon_days);
        let hit = *probability >= threshold;

        if regime == early_warning_regime {
            summary.early_warning_row_count += 1;
            if hit {
                summary.early_warning_hit_count += 1;
            }
        }

        match regime {
            crate::ProbabilityTrainingRegime::Normal => {
                summary.normal_row_count += 1;
                if hit {
                    summary.normal_hit_count += 1;
                }
            }
            crate::ProbabilityTrainingRegime::PositiveWindow => {
                summary.positive_window_row_count += 1;
                if hit {
                    summary.positive_window_hit_count += 1;
                }
            }
            crate::ProbabilityTrainingRegime::InCrisis => {
                summary.in_crisis_row_count += 1;
                if hit {
                    summary.in_crisis_hit_count += 1;
                }
            }
            crate::ProbabilityTrainingRegime::PostCrisisCooldown => {
                summary.cooldown_row_count += 1;
                if hit {
                    summary.cooldown_hit_count += 1;
                }
            }
            crate::ProbabilityTrainingRegime::PreWarningBuffer => {}
        }
    }

    summary
}

pub(super) fn regime_aware_threshold_prediction_ceiling(
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

pub(super) fn threshold_has_usable_early_warning_support(
    hits: ProbabilityThresholdRegimeHitSummary,
    horizon_days: u32,
) -> bool {
    hits.early_warning_hit_count > 0
        && hits.early_warning_hit_rate() >= regime_floor_min_hit_rate(horizon_days)
        && (hits.early_warning_hit_rate() - hits.normal_hit_rate())
            >= regime_floor_min_gap_vs_normal(horizon_days)
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

    let Some(regime_summary) = super::super::evaluate_regime_separation_summary_refs(
        probabilities,
        rows,
        horizon_days,
        label_mode,
    ) else {
        return base_threshold;
    };

    let base_hits =
        probability_threshold_regime_hit_summary(probabilities, rows, horizon_days, base_threshold);
    if threshold_has_usable_early_warning_support(base_hits, horizon_days) {
        return base_threshold;
    }
    if regime_summary
        .early_warning_lift_vs_normal
        .unwrap_or_default()
        < 1.5
    {
        return base_threshold;
    }

    let actual_positive_count = labels.iter().filter(|label| **label >= 0.5).count() as u32;
    let positive_count = actual_positive_count as f64;
    if positive_count <= 0.0 {
        return base_threshold;
    }

    let early_warning_regime = super::super::probability_early_warning_regime(horizon_days);
    let early_warning_probability_cap = probabilities
        .iter()
        .zip(rows.iter().copied())
        .filter(|(_, row)| row.regime_for_horizon(horizon_days) == early_warning_regime)
        .map(|(probability, _)| *probability)
        .fold(0.0_f64, f64::max);

    let relaxed_prediction_ceiling =
        regime_aware_threshold_prediction_ceiling(actual_positive_count, horizon_days);
    let beta_sq = probability_threshold_beta_sq(horizon_days);
    let mut best_score = None::<(bool, bool, i64, i64, i64, i64, i64, i64, i64)>;
    let mut best_threshold = base_threshold;

    for threshold in probability_decision_threshold_candidates(probabilities) {
        if threshold >= base_threshold {
            continue;
        }

        let hits =
            probability_threshold_regime_hit_summary(probabilities, rows, horizon_days, threshold);
        let early_warning_hit_rate = hits.early_warning_hit_rate();
        if hits.early_warning_hit_count == 0 {
            continue;
        }

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

pub(super) fn probability_threshold_decision_metrics(
    probabilities: &[f64],
    labels: &[f64],
    rows: &[&crate::ProbabilityTrainingRow],
    horizon_days: u32,
    threshold: f64,
) -> ProbabilityThresholdDecisionMetrics {
    let regime_hits =
        probability_threshold_regime_hit_summary(probabilities, rows, horizon_days, threshold);
    let mut predicted_positive_count = 0_u32;
    let mut true_positive_count = 0_u32;
    let positive_count = labels.iter().filter(|label| **label >= 0.5).count() as f64;

    for (probability, label) in probabilities.iter().zip(labels) {
        if *probability >= threshold {
            predicted_positive_count += 1;
            if *label >= 0.5 {
                true_positive_count += 1;
            }
        }
    }

    ProbabilityThresholdDecisionMetrics {
        regime_hits,
        predicted_positive_count,
        true_positive_count,
        precision: crate::safe_divide(true_positive_count as f64, predicted_positive_count as f64),
        recall: crate::safe_divide(true_positive_count as f64, positive_count),
    }
}

pub(super) fn probability_prediction_count_ceiling_from_actual_positive_count(
    actual_positive_count: u32,
    horizon_days: u32,
) -> u32 {
    let multiple = match horizon_days {
        5 => 4_u32,
        20 => 4_u32,
        60 => 5_u32,
        _ => 5_u32,
    };
    actual_positive_count.max(1).saturating_mul(multiple)
}

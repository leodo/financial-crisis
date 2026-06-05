use std::collections::{BTreeMap, HashSet};

use fc_domain::{
    apply_platt_probability_calibration, HorizonEvaluationSummary, PlattCalibrationArtifact,
    ProbabilityCalibrationRegimeEvidence,
    ProbabilityThresholdDecisionSummary as ProbabilityThresholdDecisionSummaryWire,
    ProbabilityThresholdDiagnostics as ProbabilityThresholdDiagnosticsWire,
    RegimeSeparationEvaluationSummary,
};

#[derive(Debug, Clone)]
pub(crate) struct ProbabilityCalibrationSelection<'a> {
    pub(crate) rows: Vec<&'a crate::ProbabilityTrainingRow>,
    pub(crate) eligible_row_count: usize,
    pub(crate) eligible_positive_count: usize,
    pub(crate) eligible_negative_count: usize,
    pub(crate) used_full_split_fallback: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ProbabilityThresholdSelection<'a> {
    pub(crate) rows: Vec<&'a crate::ProbabilityTrainingRow>,
    pub(crate) probabilities: Vec<f64>,
    pub(crate) labels: Vec<f64>,
    pub(crate) used_full_split_fallback: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ProbabilityThresholdDiagnosticsInput<'a> {
    pub(crate) full_calibration_rows: &'a [crate::ProbabilityTrainingRow],
    pub(crate) calibration_selection: &'a ProbabilityCalibrationSelection<'a>,
    pub(crate) threshold_selection: &'a ProbabilityThresholdSelection<'a>,
    pub(crate) horizon_days: u32,
    pub(crate) label_mode: crate::ProbabilityTargetLabelMode,
    pub(crate) base_threshold: f64,
    pub(crate) final_threshold: f64,
}

#[derive(Debug, Clone, Copy, Default)]
struct ProbabilityThresholdDecisionMetrics {
    regime_hits: ProbabilityThresholdRegimeHitSummary,
    predicted_positive_count: u32,
    true_positive_count: u32,
    precision: f64,
    recall: f64,
}

#[derive(Debug, Clone, Copy)]
struct ProbabilityThresholdScoreInputs {
    horizon_days: u32,
    precision: f64,
    recall: f64,
    f_beta: f64,
    threshold: f64,
    predicted_positive_count: u32,
    prediction_ceiling: u32,
    actual_positive_count: u32,
}

#[derive(Debug, Clone, Copy, Default)]
struct ProbabilityThresholdRegimeHitSummary {
    early_warning_row_count: u32,
    early_warning_hit_count: u32,
    normal_row_count: u32,
    normal_hit_count: u32,
    positive_window_row_count: u32,
    positive_window_hit_count: u32,
    in_crisis_row_count: u32,
    in_crisis_hit_count: u32,
    cooldown_row_count: u32,
    cooldown_hit_count: u32,
}

#[derive(Debug, Clone, Copy, Default)]
struct ProbabilityCalibrationRegimeEvidenceBucket {
    full_row_count: u32,
    calibration_eligible_row_count: u32,
    calibration_used_row_count: u32,
    threshold_selected_row_count: u32,
    positive_label_count: u32,
    hard_label_sum: f64,
    training_target_sum: f64,
    objective_weight_sum: f64,
    protected_action_window_count: u32,
}

pub(crate) fn probability_calibration_selection_rows(
    rows: &[crate::ProbabilityTrainingRow],
    horizon_days: u32,
    label_mode: crate::ProbabilityTargetLabelMode,
) -> ProbabilityCalibrationSelection<'_> {
    let filtered = rows
        .iter()
        .filter(|row| probability_row_is_calibration_eligible(row, horizon_days, label_mode))
        .collect::<Vec<_>>();

    let filtered_positive_count = filtered
        .iter()
        .filter(|row| row.label_for_horizon(label_mode, horizon_days) > 0.0)
        .count();
    let filtered_negative_count = filtered.len().saturating_sub(filtered_positive_count);

    if filtered_positive_count > 0 && filtered_negative_count > 0 {
        ProbabilityCalibrationSelection {
            rows: filtered,
            eligible_row_count: filtered_positive_count + filtered_negative_count,
            eligible_positive_count: filtered_positive_count,
            eligible_negative_count: filtered_negative_count,
            used_full_split_fallback: false,
        }
    } else {
        ProbabilityCalibrationSelection {
            rows: rows.iter().collect(),
            eligible_row_count: filtered_positive_count + filtered_negative_count,
            eligible_positive_count: filtered_positive_count,
            eligible_negative_count: filtered_negative_count,
            used_full_split_fallback: true,
        }
    }
}

fn probability_row_is_calibration_eligible(
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
                    | crate::ProbabilityTrainingRegime::InCrisis
                    | crate::ProbabilityTrainingRegime::PostCrisisCooldown
            ),
            _ => matches!(
                row.regime_for_horizon(horizon_days),
                crate::ProbabilityTrainingRegime::Normal
            ),
        },
    }
}

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

pub(crate) fn select_probability_calibration_strategy(
    calibration_raw_probabilities: &[f64],
    calibration_labels: &[f64],
    calibration_rows: &[&crate::ProbabilityTrainingRow],
    horizon_days: u32,
    label_mode: crate::ProbabilityTargetLabelMode,
    evaluation_raw_probabilities: &[f64],
    calibration_candidate: PlattCalibrationArtifact,
) -> (Option<PlattCalibrationArtifact>, Vec<f64>) {
    let raw_summary =
        crate::evaluate_probabilities(calibration_raw_probabilities, calibration_labels);
    let raw_regime_separation = super::evaluate_regime_separation_summary_refs(
        calibration_raw_probabilities,
        calibration_rows,
        horizon_days,
        label_mode,
    );
    let raw_score =
        probability_calibration_selection_score(&raw_summary, raw_regime_separation.as_ref());

    let calibration_probabilities = calibration_raw_probabilities
        .iter()
        .map(|raw_probability| {
            apply_platt_probability_calibration(*raw_probability, &calibration_candidate)
        })
        .collect::<Vec<_>>();
    let calibrated_summary =
        crate::evaluate_probabilities(&calibration_probabilities, calibration_labels);
    let calibrated_regime_separation = super::evaluate_regime_separation_summary_refs(
        &calibration_probabilities,
        calibration_rows,
        horizon_days,
        label_mode,
    );
    let calibrated_score = probability_calibration_selection_score(
        &calibrated_summary,
        calibrated_regime_separation.as_ref(),
    );

    let raw_ranking_reversed =
        probability_raw_ranking_is_reversed(calibration_raw_probabilities, calibration_labels);
    let keep_calibration = calibrated_score > raw_score
        && (calibration_candidate.alpha > 0.0
            || (calibration_candidate.alpha < 0.0 && raw_ranking_reversed));
    if keep_calibration {
        let evaluation_probabilities = evaluation_raw_probabilities
            .iter()
            .map(|raw_probability| {
                apply_platt_probability_calibration(*raw_probability, &calibration_candidate)
            })
            .collect::<Vec<_>>();
        (Some(calibration_candidate), evaluation_probabilities)
    } else {
        (None, evaluation_raw_probabilities.to_vec())
    }
}

fn probability_calibration_selection_score(
    summary: &HorizonEvaluationSummary,
    regime_separation: Option<&RegimeSeparationEvaluationSummary>,
) -> (i64, i64, i64, i64, i64, i64, i64, i64, i64) {
    (
        probability_regime_diagnosis_score(regime_separation),
        probability_regime_positive_window_lift_score(regime_separation),
        probability_regime_positive_window_gap_score(regime_separation),
        probability_regime_positive_window_minus_cooldown_score(regime_separation),
        probability_regime_early_warning_lift_score(regime_separation),
        probability_regime_max_non_normal_lift_score(regime_separation),
        -((summary.log_loss * 1_000_000.0).round() as i64),
        -((summary.brier_score * 1_000_000.0).round() as i64),
        -((summary.ece * 1_000_000.0).round() as i64),
    )
}

fn probability_regime_diagnosis_score(
    regime_separation: Option<&RegimeSeparationEvaluationSummary>,
) -> i64 {
    match regime_separation.map(|summary| summary.diagnosis.as_str()) {
        Some("usable_early_warning_separation") => 6,
        Some("weak_regime_separation") => 5,
        Some("mixed_or_unclear") => 4,
        Some("late_only_no_early_warning") => 3,
        Some("cooldown_bleed") => 2,
        Some("cold_across_all_regimes") => 1,
        Some(_) => 0,
        None => 2,
    }
}

fn probability_regime_positive_window_lift_score(
    regime_separation: Option<&RegimeSeparationEvaluationSummary>,
) -> i64 {
    (regime_separation
        .and_then(|summary| summary.positive_window_lift_vs_normal)
        .unwrap_or_default()
        * 1_000.0)
        .round() as i64
}

fn probability_regime_positive_window_gap_score(
    regime_separation: Option<&RegimeSeparationEvaluationSummary>,
) -> i64 {
    (regime_separation
        .and_then(|summary| summary.positive_window_gap_vs_normal)
        .unwrap_or_default()
        * 1_000_000.0)
        .round() as i64
}

fn probability_regime_positive_window_minus_cooldown_score(
    regime_separation: Option<&RegimeSeparationEvaluationSummary>,
) -> i64 {
    let Some(summary) = regime_separation else {
        return 0;
    };
    let positive_window = summary.positive_window_lift_vs_normal.unwrap_or_default();
    let cooldown = summary
        .post_crisis_cooldown_lift_vs_normal
        .unwrap_or_default();
    ((positive_window - cooldown) * 1_000.0).round() as i64
}

fn probability_regime_early_warning_lift_score(
    regime_separation: Option<&RegimeSeparationEvaluationSummary>,
) -> i64 {
    (regime_separation
        .and_then(|summary| summary.early_warning_lift_vs_normal)
        .unwrap_or_default()
        * 1_000.0)
        .round() as i64
}

fn probability_regime_max_non_normal_lift_score(
    regime_separation: Option<&RegimeSeparationEvaluationSummary>,
) -> i64 {
    (regime_separation
        .and_then(|summary| summary.max_non_normal_lift_vs_normal)
        .unwrap_or_default()
        * 1_000.0)
        .round() as i64
}

fn probability_raw_ranking_is_reversed(probabilities: &[f64], labels: &[f64]) -> bool {
    let mut positive_sum = 0.0;
    let mut positive_count = 0_u32;
    let mut negative_sum = 0.0;
    let mut negative_count = 0_u32;

    for (probability, label) in probabilities.iter().zip(labels) {
        if *label >= 0.5 {
            positive_sum += *probability;
            positive_count += 1;
        } else {
            negative_sum += *probability;
            negative_count += 1;
        }
    }

    if positive_count == 0 || negative_count == 0 {
        return false;
    }

    let positive_mean = positive_sum / positive_count as f64;
    let negative_mean = negative_sum / negative_count as f64;
    positive_mean < negative_mean
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

fn probability_threshold_score_tuple(
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
    fn early_warning_hit_rate(self) -> f64 {
        crate::safe_divide(
            self.early_warning_hit_count as f64,
            self.early_warning_row_count as f64,
        )
    }

    fn normal_hit_rate(self) -> f64 {
        crate::safe_divide(self.normal_hit_count as f64, self.normal_row_count as f64)
    }

    fn positive_window_hit_rate(self) -> f64 {
        crate::safe_divide(
            self.positive_window_hit_count as f64,
            self.positive_window_row_count as f64,
        )
    }

    fn in_crisis_hit_rate(self) -> f64 {
        crate::safe_divide(
            self.in_crisis_hit_count as f64,
            self.in_crisis_row_count as f64,
        )
    }

    fn cooldown_hit_rate(self) -> f64 {
        crate::safe_divide(
            self.cooldown_hit_count as f64,
            self.cooldown_row_count as f64,
        )
    }
}

fn probability_threshold_regime_hit_summary(
    probabilities: &[f64],
    rows: &[&crate::ProbabilityTrainingRow],
    horizon_days: u32,
    threshold: f64,
) -> ProbabilityThresholdRegimeHitSummary {
    let early_warning_regime = super::probability_early_warning_regime(horizon_days);

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

fn regime_aware_threshold_prediction_ceiling(actual_positive_count: u32, horizon_days: u32) -> u32 {
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

fn threshold_has_usable_early_warning_support(
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

    let Some(regime_summary) = super::evaluate_regime_separation_summary_refs(
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

    let early_warning_regime = super::probability_early_warning_regime(horizon_days);
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

fn probability_threshold_decision_metrics(
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

fn probability_threshold_decision_summary_wire(
    metrics: ProbabilityThresholdDecisionMetrics,
) -> ProbabilityThresholdDecisionSummaryWire {
    ProbabilityThresholdDecisionSummaryWire {
        predicted_positive_count: metrics.predicted_positive_count,
        true_positive_count: metrics.true_positive_count,
        precision: crate::round3(metrics.precision),
        recall: crate::round3(metrics.recall),
        early_warning_row_count: metrics.regime_hits.early_warning_row_count,
        early_warning_hit_count: metrics.regime_hits.early_warning_hit_count,
        early_warning_hit_rate: crate::round3(metrics.regime_hits.early_warning_hit_rate()),
        normal_row_count: metrics.regime_hits.normal_row_count,
        normal_hit_count: metrics.regime_hits.normal_hit_count,
        normal_hit_rate: crate::round3(metrics.regime_hits.normal_hit_rate()),
        positive_window_row_count: metrics.regime_hits.positive_window_row_count,
        positive_window_hit_count: metrics.regime_hits.positive_window_hit_count,
        positive_window_hit_rate: crate::round3(metrics.regime_hits.positive_window_hit_rate()),
        in_crisis_row_count: metrics.regime_hits.in_crisis_row_count,
        in_crisis_hit_count: metrics.regime_hits.in_crisis_hit_count,
        in_crisis_hit_rate: crate::round3(metrics.regime_hits.in_crisis_hit_rate()),
        cooldown_row_count: metrics.regime_hits.cooldown_row_count,
        cooldown_hit_count: metrics.regime_hits.cooldown_hit_count,
        cooldown_hit_rate: crate::round3(metrics.regime_hits.cooldown_hit_rate()),
    }
}

pub(crate) fn build_probability_threshold_diagnostics(
    input: ProbabilityThresholdDiagnosticsInput<'_>,
) -> ProbabilityThresholdDiagnosticsWire {
    let ProbabilityThresholdDiagnosticsInput {
        full_calibration_rows,
        calibration_selection,
        threshold_selection,
        horizon_days,
        label_mode,
        base_threshold,
        final_threshold,
    } = input;
    let early_warning_regime = super::probability_early_warning_regime(horizon_days);
    let probabilities = &threshold_selection.probabilities;
    let labels = &threshold_selection.labels;
    let selected_positive_count = labels.iter().filter(|label| **label >= 0.5).count();
    let selected_negative_count = labels.len().saturating_sub(selected_positive_count);
    let actual_positive_count = selected_positive_count as u32;
    let prediction_ceiling = (actual_positive_count > 0).then(|| {
        probability_prediction_count_ceiling_from_actual_positive_count(
            actual_positive_count,
            horizon_days,
        )
    });
    let relaxed_prediction_ceiling = (label_mode
        == crate::ProbabilityTargetLabelMode::ForwardCrisis
        && matches!(horizon_days, 20 | 60)
        && actual_positive_count > 0)
        .then(|| regime_aware_threshold_prediction_ceiling(actual_positive_count, horizon_days));
    let early_warning_probability_cap = probabilities
        .iter()
        .zip(threshold_selection.rows.iter().copied())
        .filter(|(_, row)| row.regime_for_horizon(horizon_days) == early_warning_regime)
        .map(|(probability, _)| *probability)
        .max_by(f64::total_cmp);
    let base_metrics = probability_threshold_decision_metrics(
        probabilities,
        labels,
        &threshold_selection.rows,
        horizon_days,
        base_threshold,
    );
    let final_metrics = probability_threshold_decision_metrics(
        probabilities,
        labels,
        &threshold_selection.rows,
        horizon_days,
        final_threshold,
    );
    let regime_summary = super::evaluate_regime_separation_summary_refs(
        probabilities,
        &threshold_selection.rows,
        horizon_days,
        label_mode,
    );
    let repair_eligible = label_mode == crate::ProbabilityTargetLabelMode::ForwardCrisis
        && matches!(horizon_days, 20 | 60)
        && !probabilities.is_empty()
        && !threshold_selection.rows.is_empty()
        && probabilities.len() == threshold_selection.rows.len();
    let repair_applied = (final_threshold - base_threshold).abs() >= 0.000_5;
    let repair_reason = if !repair_eligible {
        "not_applicable".to_string()
    } else if base_metrics.regime_hits.early_warning_row_count == 0 {
        "no_early_warning_rows".to_string()
    } else if threshold_has_usable_early_warning_support(base_metrics.regime_hits, horizon_days) {
        "base_threshold_has_usable_early_warning_gap".to_string()
    } else if regime_summary
        .as_ref()
        .and_then(|summary| summary.early_warning_lift_vs_normal)
        .unwrap_or_default()
        < 1.5
    {
        "early_warning_lift_below_guardrail".to_string()
    } else if base_metrics.regime_hits.early_warning_hit_count > 0 {
        "base_hits_early_warning_but_gap_is_too_weak".to_string()
    } else if actual_positive_count == 0 {
        "no_positive_labels".to_string()
    } else if !repair_applied {
        "repair_considered_but_no_better_candidate".to_string()
    } else if early_warning_probability_cap
        .is_some_and(|cap| cap < base_threshold && (final_threshold - cap).abs() < 0.000_5)
    {
        "repaired_to_early_warning_cap".to_string()
    } else {
        "repaired_to_regime_support_candidate".to_string()
    };

    ProbabilityThresholdDiagnosticsWire {
        label_mode: label_mode.as_str().to_string(),
        early_warning_regime: crate::probability_training_regime_name(early_warning_regime)
            .to_string(),
        full_calibration_row_count: full_calibration_rows.len(),
        eligible_row_count: calibration_selection.eligible_row_count,
        eligible_positive_count: calibration_selection.eligible_positive_count,
        eligible_negative_count: calibration_selection.eligible_negative_count,
        used_full_split_fallback: calibration_selection.used_full_split_fallback,
        selected_row_count: threshold_selection.rows.len(),
        selected_positive_count,
        selected_negative_count,
        selected_used_full_split_fallback: threshold_selection.used_full_split_fallback,
        base_threshold: crate::round3(base_threshold),
        final_threshold: crate::round3(final_threshold),
        repair_applied,
        repair_eligible,
        repair_reason,
        early_warning_probability_cap: early_warning_probability_cap.map(crate::round3),
        prediction_ceiling,
        relaxed_prediction_ceiling,
        base_summary: probability_threshold_decision_summary_wire(base_metrics),
        final_summary: probability_threshold_decision_summary_wire(final_metrics),
        calibration_regime_evidence: build_probability_calibration_regime_evidence(
            full_calibration_rows,
            calibration_selection,
            threshold_selection,
            horizon_days,
            label_mode,
        ),
    }
}

fn build_probability_calibration_regime_evidence(
    full_calibration_rows: &[crate::ProbabilityTrainingRow],
    calibration_selection: &ProbabilityCalibrationSelection<'_>,
    threshold_selection: &ProbabilityThresholdSelection<'_>,
    horizon_days: u32,
    label_mode: crate::ProbabilityTargetLabelMode,
) -> Vec<ProbabilityCalibrationRegimeEvidence> {
    if full_calibration_rows.is_empty() {
        return Vec::new();
    }

    let calibration_selected_ptrs = calibration_selection
        .rows
        .iter()
        .map(|row| *row as *const crate::ProbabilityTrainingRow)
        .collect::<HashSet<_>>();
    let threshold_selected_ptrs = threshold_selection
        .rows
        .iter()
        .map(|row| *row as *const crate::ProbabilityTrainingRow)
        .collect::<HashSet<_>>();
    let mut buckets = BTreeMap::<
        crate::ProbabilityTrainingRegime,
        ProbabilityCalibrationRegimeEvidenceBucket,
    >::new();

    for row in full_calibration_rows {
        let row_ptr = row as *const crate::ProbabilityTrainingRow;
        let regime = row.regime_for_horizon(horizon_days);
        let hard_label = row.label_for_horizon(label_mode, horizon_days);
        let bucket = buckets.entry(regime).or_default();
        bucket.full_row_count += 1;
        if probability_row_is_calibration_eligible(row, horizon_days, label_mode) {
            bucket.calibration_eligible_row_count += 1;
        }
        if calibration_selected_ptrs.contains(&row_ptr) {
            bucket.calibration_used_row_count += 1;
        }
        if threshold_selected_ptrs.contains(&row_ptr) {
            bucket.threshold_selected_row_count += 1;
        }
        if hard_label > 0.0 {
            bucket.positive_label_count += 1;
        }
        bucket.hard_label_sum += hard_label;
        bucket.training_target_sum +=
            crate::model::probability_training_target_label(row, horizon_days, label_mode);
        bucket.objective_weight_sum +=
            probability_calibration_objective_weight(row, horizon_days, label_mode);
        if row.protected_action_window {
            bucket.protected_action_window_count += 1;
        }
    }

    let full_row_count = full_calibration_rows.len() as f64;
    probability_regime_evidence_order()
        .into_iter()
        .filter_map(|regime| {
            let bucket = buckets.get(&regime).copied().unwrap_or_default();
            if bucket.full_row_count == 0 {
                return None;
            }
            let row_count = bucket.full_row_count as f64;
            Some(ProbabilityCalibrationRegimeEvidence {
                regime: crate::probability_training_regime_name(regime).to_string(),
                full_row_count: bucket.full_row_count,
                full_row_rate: crate::round3(crate::safe_divide(row_count, full_row_count)),
                calibration_eligible_row_count: bucket.calibration_eligible_row_count,
                calibration_eligible_row_rate: crate::round3(crate::safe_divide(
                    bucket.calibration_eligible_row_count as f64,
                    row_count,
                )),
                calibration_used_row_count: bucket.calibration_used_row_count,
                calibration_used_row_rate: crate::round3(crate::safe_divide(
                    bucket.calibration_used_row_count as f64,
                    row_count,
                )),
                threshold_selected_row_count: bucket.threshold_selected_row_count,
                threshold_selected_row_rate: crate::round3(crate::safe_divide(
                    bucket.threshold_selected_row_count as f64,
                    row_count,
                )),
                positive_label_count: bucket.positive_label_count,
                positive_label_rate: crate::round3(crate::safe_divide(
                    bucket.positive_label_count as f64,
                    row_count,
                )),
                avg_hard_label: crate::round3(crate::safe_divide(bucket.hard_label_sum, row_count)),
                avg_training_target: crate::round3(crate::safe_divide(
                    bucket.training_target_sum,
                    row_count,
                )),
                objective_weight_sum: crate::round3(bucket.objective_weight_sum),
                avg_objective_weight: crate::round3(crate::safe_divide(
                    bucket.objective_weight_sum,
                    row_count,
                )),
                protected_action_window_count: bucket.protected_action_window_count,
                protected_action_window_rate: crate::round3(crate::safe_divide(
                    bucket.protected_action_window_count as f64,
                    row_count,
                )),
            })
        })
        .collect()
}

fn probability_regime_evidence_order() -> [crate::ProbabilityTrainingRegime; 5] {
    [
        crate::ProbabilityTrainingRegime::Normal,
        crate::ProbabilityTrainingRegime::PreWarningBuffer,
        crate::ProbabilityTrainingRegime::PositiveWindow,
        crate::ProbabilityTrainingRegime::InCrisis,
        crate::ProbabilityTrainingRegime::PostCrisisCooldown,
    ]
}

fn probability_calibration_objective_weight(
    row: &crate::ProbabilityTrainingRow,
    horizon_days: u32,
    label_mode: crate::ProbabilityTargetLabelMode,
) -> f64 {
    let hard_label = row.label_for_horizon(label_mode, horizon_days);
    if hard_label > 0.0 {
        return match label_mode {
            crate::ProbabilityTargetLabelMode::ForwardCrisis => {
                crate::model::forward_crisis_positive_sample_weight(row, horizon_days)
            }
            crate::ProbabilityTargetLabelMode::ActionWindow
            | crate::ProbabilityTargetLabelMode::ActionEpisode => {
                crate::model::positive_sample_action_weight(row, horizon_days)
            }
        };
    }

    crate::model::negative_sample_weight(row, horizon_days, label_mode)
}

fn probability_prediction_count_ceiling_from_actual_positive_count(
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

#[cfg(test)]
mod tests {
    use super::{probability_threshold_score_tuple, ProbabilityThresholdScoreInputs};

    #[test]
    fn twenty_day_threshold_score_penalizes_overbroad_thresholds() {
        let restrained = probability_threshold_score_tuple(ProbabilityThresholdScoreInputs {
            horizon_days: 20,
            precision: 0.19,
            recall: 0.30,
            f_beta: 0.23,
            threshold: 0.48,
            predicted_positive_count: 80,
            prediction_ceiling: 40,
            actual_positive_count: 10,
        });
        let overbroad = probability_threshold_score_tuple(ProbabilityThresholdScoreInputs {
            horizon_days: 20,
            precision: 0.18,
            recall: 0.35,
            f_beta: 0.232,
            threshold: 0.44,
            predicted_positive_count: 120,
            prediction_ceiling: 40,
            actual_positive_count: 10,
        });

        assert!(restrained > overbroad);
    }
}

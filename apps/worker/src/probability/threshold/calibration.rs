use fc_domain::{
    apply_platt_probability_calibration, HorizonEvaluationSummary, PlattCalibrationArtifact,
    RegimeSeparationEvaluationSummary,
};

use super::ProbabilityCalibrationSelection;

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

pub(super) fn probability_row_is_calibration_eligible(
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
    let raw_regime_separation = super::super::evaluate_regime_separation_summary_refs(
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
    let calibrated_regime_separation = super::super::evaluate_regime_separation_summary_refs(
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

use fc_domain::{
    apply_platt_probability_calibration, ActionabilityEvaluationSummary, PlattCalibrationArtifact,
};

pub(crate) fn select_actionability_decision_threshold(
    probabilities: &[f64],
    rows: &[crate::ProbabilityTrainingRow],
    horizon_days: u32,
) -> f64 {
    let mut thresholds = probabilities
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .filter(|value| (0.01..0.99).contains(value))
        .collect::<Vec<_>>();
    thresholds.extend((5..=60).map(|value| value as f64 / 100.0));
    thresholds.push(0.3);
    thresholds.sort_by(f64::total_cmp);
    thresholds.dedup_by(|left, right| (*left - *right).abs() < 1e-6);

    let mut best_threshold = 0.3;
    let mut best_score = None::<(bool, bool, bool, u32, u32, i64, i64, i64)>;
    for threshold in thresholds {
        let summary = super::summary::evaluate_actionability_summary(
            probabilities,
            rows,
            horizon_days,
            threshold,
        );
        if summary.predicted_positive_count == 0 {
            continue;
        }
        let hit_scenario_count =
            summary.advance_warning_scenario_count + summary.late_confirmation_scenario_count;
        if hit_scenario_count == 0 {
            continue;
        }
        let precision_score =
            (summary.precision_at_threshold.unwrap_or_default() * 1_000.0).round() as i64;
        let false_positive_penalty = -(summary.false_positive_count as i64);
        let threshold_score = (threshold * 1_000.0).round() as i64;
        let meets_precision_floor =
            precision_score >= super::actionability_precision_floor_score(horizon_days);
        let meets_volume_ceiling = summary.predicted_positive_count
            <= super::actionability_prediction_count_ceiling(&summary, horizon_days);
        let score = (
            meets_precision_floor && meets_volume_ceiling,
            meets_precision_floor,
            meets_volume_ceiling,
            hit_scenario_count,
            summary.advance_warning_scenario_count,
            precision_score,
            false_positive_penalty,
            threshold_score,
        );
        if best_score.is_none_or(|best| score > best) {
            best_score = Some(score);
            best_threshold = threshold;
        }
    }

    crate::round3(best_threshold).clamp(0.05, 0.60)
}

pub(crate) fn select_actionability_calibration_strategy(
    calibration_raw_probabilities: &[f64],
    calibration_rows: &[crate::ProbabilityTrainingRow],
    evaluation_raw_probabilities: &[f64],
    horizon_days: u32,
    calibration_candidate: PlattCalibrationArtifact,
) -> (Option<PlattCalibrationArtifact>, Vec<f64>, f64) {
    let raw_threshold = select_actionability_decision_threshold(
        calibration_raw_probabilities,
        calibration_rows,
        horizon_days,
    );
    let raw_summary = super::summary::evaluate_actionability_summary(
        calibration_raw_probabilities,
        calibration_rows,
        horizon_days,
        raw_threshold,
    );
    let raw_score =
        actionability_summary_selection_score(&raw_summary, raw_threshold, horizon_days);

    let calibration_probabilities = calibration_raw_probabilities
        .iter()
        .map(|raw_probability| {
            apply_platt_probability_calibration(*raw_probability, &calibration_candidate)
        })
        .collect::<Vec<_>>();
    let calibrated_threshold = select_actionability_decision_threshold(
        &calibration_probabilities,
        calibration_rows,
        horizon_days,
    );
    let calibrated_summary = super::summary::evaluate_actionability_summary(
        &calibration_probabilities,
        calibration_rows,
        horizon_days,
        calibrated_threshold,
    );
    let calibrated_score = actionability_summary_selection_score(
        &calibrated_summary,
        calibrated_threshold,
        horizon_days,
    );

    let keep_calibration = calibration_candidate.alpha > 0.0 && calibrated_score > raw_score;
    if keep_calibration {
        let evaluation_probabilities = evaluation_raw_probabilities
            .iter()
            .map(|raw_probability| {
                apply_platt_probability_calibration(*raw_probability, &calibration_candidate)
            })
            .collect::<Vec<_>>();
        (
            Some(calibration_candidate),
            evaluation_probabilities,
            calibrated_threshold,
        )
    } else {
        (None, evaluation_raw_probabilities.to_vec(), raw_threshold)
    }
}

fn actionability_summary_selection_score(
    summary: &ActionabilityEvaluationSummary,
    threshold: f64,
    horizon_days: u32,
) -> (bool, bool, bool, u32, u32, i64, i64, i64) {
    let hit_scenario_count =
        summary.advance_warning_scenario_count + summary.late_confirmation_scenario_count;
    let precision_score =
        (summary.precision_at_threshold.unwrap_or_default() * 1_000.0).round() as i64;
    let false_positive_penalty = -(summary.false_positive_count as i64);
    let threshold_score = (threshold * 1_000.0).round() as i64;
    let meets_precision_floor =
        precision_score >= super::actionability_precision_floor_score(horizon_days);
    let meets_volume_ceiling = summary.predicted_positive_count
        <= super::actionability_prediction_count_ceiling(summary, horizon_days);
    (
        meets_precision_floor && meets_volume_ceiling,
        meets_precision_floor,
        meets_volume_ceiling,
        hit_scenario_count,
        summary.advance_warning_scenario_count,
        precision_score,
        false_positive_penalty,
        threshold_score,
    )
}

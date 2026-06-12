use std::collections::BTreeMap;

mod overlay;
mod threshold;

use fc_domain::{
    apply_platt_probability_calibration, HorizonEvaluationSummary, LogisticProbabilityModel,
    PlattCalibrationArtifact, ProbabilityBundleEvaluation, ProbabilityHorizonBundle,
    ProbabilityThresholdDiagnostics as ProbabilityThresholdDiagnosticsWire,
    RegimeSeparationEvaluationSummary,
};

pub(crate) use threshold::{
    adjust_probability_decision_threshold_for_regime_support,
    build_probability_threshold_diagnostics, probability_calibration_selection_rows,
    probability_decision_threshold_selection, select_probability_calibration_strategy,
    select_probability_decision_threshold, ProbabilityCalibrationStrategyInput,
    ProbabilityThresholdDiagnosticsInput,
};
#[cfg(test)]
pub(crate) use threshold::{ProbabilityCalibrationSelection, ProbabilityThresholdSelection};

#[derive(Debug, Clone)]
struct TrainedProbabilityHead {
    raw_model: LogisticProbabilityModel,
    calibration: Option<PlattCalibrationArtifact>,
    evaluation: HorizonEvaluationSummary,
    decision_threshold: f64,
    threshold_diagnostics: ProbabilityThresholdDiagnosticsWire,
}

pub(crate) fn train_horizon_bundle(
    train_rows: &[crate::ProbabilityTrainingRow],
    calibration_rows: &[crate::ProbabilityTrainingRow],
    evaluation_rows: &[crate::ProbabilityTrainingRow],
    base_feature_names: &[String],
    overlay_feature_names: &[String],
    horizon_days: u32,
    label_mode: crate::ProbabilityTargetLabelMode,
) -> anyhow::Result<ProbabilityHorizonBundle> {
    let trained_head = train_probability_head(
        train_rows,
        calibration_rows,
        evaluation_rows,
        base_feature_names,
        horizon_days,
        label_mode,
    )?;
    let family_overlay_audits = overlay::build_family_overlay_audits(
        train_rows,
        calibration_rows,
        evaluation_rows,
        overlay_feature_names,
        horizon_days,
        label_mode,
    );
    let family_overlays = overlay::train_family_overlays(
        train_rows,
        calibration_rows,
        evaluation_rows,
        overlay_feature_names,
        horizon_days,
        label_mode,
        &family_overlay_audits,
    );

    Ok(ProbabilityHorizonBundle {
        horizon_days,
        decision_threshold: Some(trained_head.decision_threshold),
        threshold_diagnostics: Some(trained_head.threshold_diagnostics),
        raw_model: trained_head.raw_model,
        calibration: trained_head.calibration,
        evaluation: trained_head.evaluation,
        family_overlays,
        family_overlay_audits,
    })
}

fn train_probability_head(
    train_rows: &[crate::ProbabilityTrainingRow],
    calibration_rows: &[crate::ProbabilityTrainingRow],
    evaluation_rows: &[crate::ProbabilityTrainingRow],
    feature_names: &[String],
    horizon_days: u32,
    label_mode: crate::ProbabilityTargetLabelMode,
) -> anyhow::Result<TrainedProbabilityHead> {
    crate::ensure_positive_labels(train_rows, horizon_days, "train", label_mode)?;
    crate::ensure_positive_labels(calibration_rows, horizon_days, "calibration", label_mode)?;
    crate::ensure_positive_labels(evaluation_rows, horizon_days, "evaluation", label_mode)?;

    let raw_model = crate::fit_logistic_model(train_rows, feature_names, horizon_days, label_mode);
    let calibration_selection =
        probability_calibration_selection_rows(calibration_rows, horizon_days, label_mode);
    let calibration_inputs = calibration_selection
        .rows
        .iter()
        .map(|row| crate::score_logistic_model_for_dataset(&raw_model, row))
        .collect::<Vec<_>>();
    let calibration_labels = calibration_selection
        .rows
        .iter()
        .map(|row| row.label_for_horizon(label_mode, horizon_days))
        .collect::<Vec<_>>();
    let calibration_candidate =
        crate::fit_platt_calibration(&calibration_inputs, &calibration_labels);
    let evaluation_raw_probabilities = evaluation_rows
        .iter()
        .map(|row| crate::score_logistic_model_for_dataset(&raw_model, row))
        .collect::<Vec<_>>();
    let evaluation_row_refs = evaluation_rows.iter().collect::<Vec<_>>();
    let (calibration, evaluation_probabilities) =
        select_probability_calibration_strategy(ProbabilityCalibrationStrategyInput {
            calibration_raw_probabilities: &calibration_inputs,
            calibration_labels: &calibration_labels,
            calibration_rows: &calibration_selection.rows,
            horizon_days,
            label_mode,
            evaluation_raw_probabilities: &evaluation_raw_probabilities,
            evaluation_rows: &evaluation_row_refs,
            calibration_candidate,
        });
    let calibration_decision_probabilities = calibration.as_ref().map_or_else(
        || calibration_inputs.clone(),
        |calibration| {
            calibration_inputs
                .iter()
                .map(|raw_probability| {
                    apply_platt_probability_calibration(*raw_probability, calibration)
                })
                .collect::<Vec<_>>()
        },
    );
    let threshold_selection = probability_decision_threshold_selection(
        &calibration_decision_probabilities,
        &calibration_labels,
        &calibration_selection.rows,
        horizon_days,
        label_mode,
    );
    let base_decision_threshold = select_probability_decision_threshold(
        &threshold_selection.probabilities,
        &threshold_selection.labels,
        horizon_days,
    );
    let decision_threshold = adjust_probability_decision_threshold_for_regime_support(
        base_decision_threshold,
        &threshold_selection.probabilities,
        &threshold_selection.labels,
        &threshold_selection.rows,
        horizon_days,
        label_mode,
    );
    let threshold_diagnostics =
        build_probability_threshold_diagnostics(ProbabilityThresholdDiagnosticsInput {
            full_calibration_rows: calibration_rows,
            calibration_selection: &calibration_selection,
            threshold_selection: &threshold_selection,
            horizon_days,
            label_mode,
            base_threshold: base_decision_threshold,
            final_threshold: decision_threshold,
        });
    let evaluation = evaluate_probabilities_for_rows(
        &evaluation_probabilities,
        evaluation_rows,
        horizon_days,
        label_mode,
    );

    Ok(TrainedProbabilityHead {
        raw_model,
        calibration,
        evaluation,
        decision_threshold,
        threshold_diagnostics,
    })
}

fn probability_early_warning_regime(horizon_days: u32) -> crate::ProbabilityTrainingRegime {
    match horizon_days {
        5 => crate::ProbabilityTrainingRegime::PositiveWindow,
        20 | 60 => crate::ProbabilityTrainingRegime::PreWarningBuffer,
        _ => crate::ProbabilityTrainingRegime::PositiveWindow,
    }
}

pub(crate) fn evaluate_probabilities_for_rows(
    probabilities: &[f64],
    rows: &[crate::ProbabilityTrainingRow],
    horizon_days: u32,
    label_mode: crate::ProbabilityTargetLabelMode,
) -> HorizonEvaluationSummary {
    let labels = rows
        .iter()
        .map(|row| row.label_for_horizon(label_mode, horizon_days))
        .collect::<Vec<_>>();
    let mut summary = crate::evaluate_probabilities(probabilities, &labels);
    let row_refs = rows.iter().collect::<Vec<_>>();
    summary.regime_separation =
        evaluate_regime_separation_summary_refs(probabilities, &row_refs, horizon_days, label_mode);
    summary
}

pub(crate) fn evaluate_regime_separation_summary_refs(
    probabilities: &[f64],
    rows: &[&crate::ProbabilityTrainingRow],
    horizon_days: u32,
    label_mode: crate::ProbabilityTargetLabelMode,
) -> Option<RegimeSeparationEvaluationSummary> {
    if label_mode != crate::ProbabilityTargetLabelMode::ForwardCrisis
        || probabilities.is_empty()
        || rows.is_empty()
    {
        return None;
    }

    #[derive(Default, Clone, Copy)]
    struct Bucket {
        sample_count: u32,
        probability_sum: f64,
    }

    let mut buckets = BTreeMap::<crate::ProbabilityTrainingRegime, Bucket>::new();
    for (probability, row) in probabilities.iter().zip(rows.iter().copied()) {
        let bucket = buckets
            .entry(row.regime_for_horizon(horizon_days))
            .or_default();
        bucket.sample_count += 1;
        bucket.probability_sum += *probability;
    }

    let average_probability = |regime: crate::ProbabilityTrainingRegime| {
        buckets
            .get(&regime)
            .map(|bucket| crate::safe_divide(bucket.probability_sum, bucket.sample_count as f64))
    };
    let sample_count = |regime: crate::ProbabilityTrainingRegime| {
        buckets.get(&regime).map_or(0, |bucket| bucket.sample_count)
    };

    let early_warning_regime = match horizon_days {
        5 => crate::ProbabilityTrainingRegime::PositiveWindow,
        20 | 60 => crate::ProbabilityTrainingRegime::PreWarningBuffer,
        _ => crate::ProbabilityTrainingRegime::PositiveWindow,
    };
    let normal_avg = average_probability(crate::ProbabilityTrainingRegime::Normal)?;
    let pre_warning_buffer_avg =
        average_probability(crate::ProbabilityTrainingRegime::PreWarningBuffer).unwrap_or(0.0);
    let positive_window_avg =
        average_probability(crate::ProbabilityTrainingRegime::PositiveWindow).unwrap_or(0.0);
    let early_warning_avg = average_probability(early_warning_regime).unwrap_or(0.0);
    let in_crisis_avg =
        average_probability(crate::ProbabilityTrainingRegime::InCrisis).unwrap_or(0.0);
    let post_crisis_cooldown_avg =
        average_probability(crate::ProbabilityTrainingRegime::PostCrisisCooldown).unwrap_or(0.0);
    let max_non_normal_avg = buckets
        .iter()
        .filter(|(regime, _)| **regime != crate::ProbabilityTrainingRegime::Normal)
        .map(|(_, bucket)| crate::safe_divide(bucket.probability_sum, bucket.sample_count as f64))
        .fold(0.0_f64, f64::max);
    let pre_warning_buffer_lift_vs_normal =
        crate::lift_vs_baseline(pre_warning_buffer_avg, normal_avg);
    let positive_window_lift_vs_normal = crate::lift_vs_baseline(positive_window_avg, normal_avg);
    let early_warning_lift_vs_normal = crate::lift_vs_baseline(early_warning_avg, normal_avg);
    let in_crisis_lift_vs_normal = crate::lift_vs_baseline(in_crisis_avg, normal_avg);
    let post_crisis_cooldown_lift_vs_normal =
        crate::lift_vs_baseline(post_crisis_cooldown_avg, normal_avg);
    let positive_window_gap_vs_normal = crate::round6(positive_window_avg - normal_avg);
    let post_crisis_cooldown_gap_vs_normal = crate::round6(post_crisis_cooldown_avg - normal_avg);
    let max_non_normal_lift_vs_normal = crate::lift_vs_baseline(max_non_normal_avg, normal_avg);
    let diagnosis = classify_probability_regime_separation(
        horizon_days,
        pre_warning_buffer_lift_vs_normal.unwrap_or_default(),
        positive_window_lift_vs_normal.unwrap_or_default(),
        early_warning_lift_vs_normal.unwrap_or_default(),
        in_crisis_lift_vs_normal.unwrap_or_default(),
        post_crisis_cooldown_lift_vs_normal.unwrap_or_default(),
        positive_window_gap_vs_normal,
        post_crisis_cooldown_gap_vs_normal,
        max_non_normal_lift_vs_normal.unwrap_or_default(),
    )
    .to_string();

    Some(RegimeSeparationEvaluationSummary {
        horizon_days,
        early_warning_regime: crate::probability_training_regime_name(early_warning_regime)
            .to_string(),
        normal_sample_count: sample_count(crate::ProbabilityTrainingRegime::Normal),
        pre_warning_buffer_sample_count: sample_count(
            crate::ProbabilityTrainingRegime::PreWarningBuffer,
        ),
        positive_window_sample_count: sample_count(
            crate::ProbabilityTrainingRegime::PositiveWindow,
        ),
        early_warning_sample_count: sample_count(early_warning_regime),
        in_crisis_sample_count: sample_count(crate::ProbabilityTrainingRegime::InCrisis),
        post_crisis_cooldown_sample_count: sample_count(
            crate::ProbabilityTrainingRegime::PostCrisisCooldown,
        ),
        normal_avg_probability: crate::round6(normal_avg),
        pre_warning_buffer_avg_probability: crate::round6(pre_warning_buffer_avg),
        positive_window_avg_probability: crate::round6(positive_window_avg),
        early_warning_avg_probability: crate::round6(early_warning_avg),
        in_crisis_avg_probability: crate::round6(in_crisis_avg),
        post_crisis_cooldown_avg_probability: crate::round6(post_crisis_cooldown_avg),
        max_non_normal_avg_probability: crate::round6(max_non_normal_avg),
        pre_warning_buffer_lift_vs_normal,
        positive_window_lift_vs_normal,
        early_warning_lift_vs_normal,
        in_crisis_lift_vs_normal,
        post_crisis_cooldown_lift_vs_normal,
        positive_window_gap_vs_normal: Some(positive_window_gap_vs_normal),
        post_crisis_cooldown_gap_vs_normal: Some(post_crisis_cooldown_gap_vs_normal),
        max_non_normal_lift_vs_normal,
        diagnosis,
    })
}

pub(crate) fn regime_positive_window_gap_floor(horizon_days: u32) -> f64 {
    match horizon_days {
        5 => 0.005,
        20 | 60 => 0.010,
        _ => 0.010,
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn classify_probability_regime_separation(
    horizon_days: u32,
    pre_warning_buffer_lift_vs_normal: f64,
    positive_window_lift_vs_normal: f64,
    early_warning_lift_vs_normal: f64,
    in_crisis_lift_vs_normal: f64,
    post_crisis_cooldown_lift_vs_normal: f64,
    positive_window_gap_vs_normal: f64,
    post_crisis_cooldown_gap_vs_normal: f64,
    max_non_normal_lift_vs_normal: f64,
) -> &'static str {
    if max_non_normal_lift_vs_normal < 1.15
        && positive_window_lift_vs_normal < 1.15
        && early_warning_lift_vs_normal < 1.15
    {
        return "cold_across_all_regimes";
    }
    if positive_window_lift_vs_normal < 1.15 && in_crisis_lift_vs_normal >= 1.5 {
        return "late_only_no_early_warning";
    }
    if positive_window_lift_vs_normal >= 1.15
        && post_crisis_cooldown_lift_vs_normal >= positive_window_lift_vs_normal
        && post_crisis_cooldown_gap_vs_normal + 0.002 >= positive_window_gap_vs_normal
    {
        return "cooldown_bleed";
    }
    if positive_window_lift_vs_normal >= 1.5
        && positive_window_gap_vs_normal >= regime_positive_window_gap_floor(horizon_days)
    {
        return "usable_early_warning_separation";
    }
    if max_non_normal_lift_vs_normal >= 1.15 || pre_warning_buffer_lift_vs_normal >= 1.15 {
        return "weak_regime_separation";
    }
    "mixed_or_unclear"
}

pub(crate) fn lift_vs_baseline(value: f64, baseline: f64) -> Option<f64> {
    if baseline.abs() <= f64::EPSILON {
        return None;
    }
    Some(crate::round6(value / baseline))
}

pub(crate) fn early_warning_regime_name(horizon_days: u32) -> &'static str {
    match horizon_days {
        5 => "positive_window",
        20 | 60 => "pre_warning_buffer",
        _ => "positive_window",
    }
}

pub(crate) fn gap_retention_ratio(raw_gap: f64, calibrated_gap: f64) -> Option<f64> {
    if raw_gap.abs() <= f64::EPSILON {
        return None;
    }
    Some(crate::round6(calibrated_gap / raw_gap))
}

pub(crate) fn summarize_bundle_evaluation(
    horizons: &[ProbabilityHorizonBundle],
) -> ProbabilityBundleEvaluation {
    let total_samples = horizons
        .iter()
        .map(|horizon| horizon.evaluation.sample_count as f64)
        .sum::<f64>()
        .max(1.0);
    let weighted_brier = horizons
        .iter()
        .map(|horizon| horizon.evaluation.brier_score * horizon.evaluation.sample_count as f64)
        .sum::<f64>()
        / total_samples;
    let weighted_log_loss = horizons
        .iter()
        .map(|horizon| horizon.evaluation.log_loss * horizon.evaluation.sample_count as f64)
        .sum::<f64>()
        / total_samples;
    let weighted_ece = horizons
        .iter()
        .map(|horizon| horizon.evaluation.ece * horizon.evaluation.sample_count as f64)
        .sum::<f64>()
        / total_samples;
    let regime_separation_summaries = horizons
        .iter()
        .filter_map(|horizon| horizon.evaluation.regime_separation.clone())
        .collect::<Vec<_>>();
    let usable_early_warning_horizon_count = regime_separation_summaries
        .iter()
        .filter(|summary| summary.diagnosis == "usable_early_warning_separation")
        .count() as u32;
    let insufficient_early_warning_horizon_count = regime_separation_summaries
        .iter()
        .filter(|summary| {
            matches!(
                summary.diagnosis.as_str(),
                "cold_across_all_regimes"
                    | "late_only_no_early_warning"
                    | "mixed_or_unclear"
                    | "cooldown_bleed"
            )
        })
        .count() as u32;
    ProbabilityBundleEvaluation {
        sample_count: total_samples as u32,
        brier_score: weighted_brier,
        log_loss: weighted_log_loss,
        ece: weighted_ece,
        regime_separation_summaries,
        usable_early_warning_horizon_count,
        insufficient_early_warning_horizon_count,
        note: format!(
            "Weighted average across 5d / 20d / 60d evaluation slices. Usable early-warning horizons: {usable_early_warning_horizon_count}. Insufficient or cooldown-bleed horizons: {insufficient_early_warning_horizon_count}."
        ),
    }
}

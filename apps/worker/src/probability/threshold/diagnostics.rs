use std::collections::{BTreeMap, HashSet};

use fc_domain::{
    ProbabilityCalibrationRegimeEvidence,
    ProbabilityThresholdDecisionSummary as ProbabilityThresholdDecisionSummaryWire,
    ProbabilityThresholdDiagnostics as ProbabilityThresholdDiagnosticsWire,
};

use super::{
    calibration::probability_row_is_calibration_eligible,
    decision::{
        probability_prediction_count_ceiling_from_actual_positive_count,
        probability_threshold_decision_metrics, regime_aware_threshold_prediction_ceiling,
        threshold_has_usable_early_warning_support,
    },
    ProbabilityCalibrationRegimeEvidenceBucket, ProbabilityCalibrationSelection,
    ProbabilityThresholdDecisionMetrics, ProbabilityThresholdDiagnosticsInput,
    ProbabilityThresholdSelection,
};

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
    let early_warning_regime = super::super::probability_early_warning_regime(horizon_days);
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
    let regime_summary = super::super::evaluate_regime_separation_summary_refs(
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
        let training_target =
            crate::model::probability_training_target_label(row, horizon_days, label_mode);
        let objective_weight =
            probability_calibration_objective_weight(row, horizon_days, label_mode);
        let uses_episode_native_objective = label_mode
            == crate::ProbabilityTargetLabelMode::ForwardCrisis
            && crate::model::forward_crisis_has_episode_native_objective(row, horizon_days);
        let protected_no_positive_main_row = label_mode
            == crate::ProbabilityTargetLabelMode::ForwardCrisis
            && crate::model::forward_crisis_is_protected_no_positive_main_episode_row(
                row,
                horizon_days,
            );
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
        bucket.training_target_sum += training_target;
        bucket.objective_weight_sum += objective_weight;
        if row.protected_action_window {
            bucket.protected_action_window_count += 1;
        }
        if uses_episode_native_objective {
            bucket.episode_native_objective_row_count += 1;
        }
        if protected_no_positive_main_row {
            bucket.protected_no_positive_main_row_count += 1;
            bucket.protected_no_positive_main_training_target_sum += training_target;
            bucket.protected_no_positive_main_objective_weight_sum += objective_weight;
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
                episode_native_objective_row_count: bucket.episode_native_objective_row_count,
                episode_native_objective_row_rate: crate::round3(crate::safe_divide(
                    bucket.episode_native_objective_row_count as f64,
                    row_count,
                )),
                protected_no_positive_main_row_count: bucket.protected_no_positive_main_row_count,
                protected_no_positive_main_row_rate: crate::round3(crate::safe_divide(
                    bucket.protected_no_positive_main_row_count as f64,
                    row_count,
                )),
                protected_no_positive_main_avg_training_target: crate::round3(
                    crate::safe_divide(
                        bucket.protected_no_positive_main_training_target_sum,
                        bucket.protected_no_positive_main_row_count as f64,
                    ),
                ),
                protected_no_positive_main_avg_objective_weight: crate::round3(
                    crate::safe_divide(
                        bucket.protected_no_positive_main_objective_weight_sum,
                        bucket.protected_no_positive_main_row_count as f64,
                    ),
                ),
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

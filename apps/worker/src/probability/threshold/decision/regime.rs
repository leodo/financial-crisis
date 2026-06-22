use fc_domain::ProbabilityThresholdRepairCandidateDiagnostics;

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
        5 => 0.10,
        _ => 0.0,
    }
}

fn regime_floor_min_gap_vs_normal(horizon_days: u32) -> f64 {
    match horizon_days {
        60 => 0.02,
        20 => 0.01,
        5 => 0.02,
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
        5 => 0.10,
        20 => 0.25,
        60 => 0.10,
        _ => regime_floor_min_hit_rate(horizon_days),
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct ProbabilityThresholdPositiveEvidence {
    actual_positive_count: u32,
    effective_positive_unit_count: u32,
    effective_positive_mass: f64,
    true_positive_count: u32,
    effective_true_positive_mass: f64,
    predicted_positive_count: u32,
}

impl ProbabilityThresholdPositiveEvidence {
    fn effective_positive_count_for_ceiling(self) -> u32 {
        self.actual_positive_count
            .max(self.effective_positive_mass.ceil() as u32)
            .max(1)
    }

    fn has_true_positive_support(self) -> bool {
        self.true_positive_count > 0 || self.effective_true_positive_mass >= 0.5
    }
}

fn probability_threshold_effective_positive_label(
    row: &crate::ProbabilityTrainingRow,
    hard_label: f64,
    horizon_days: u32,
) -> f64 {
    if hard_label >= 0.5 {
        return 1.0;
    }
    if row.regime_for_horizon(horizon_days) == crate::ProbabilityTrainingRegime::PostCrisisCooldown
    {
        return 0.0;
    }

    let target = crate::model::probability_training_target_label(
        row,
        horizon_days,
        crate::ProbabilityTargetLabelMode::ForwardCrisis,
    );
    let floor = match horizon_days {
        20 => 0.18,
        60 => 0.24,
        _ => 0.5,
    };
    if target >= floor {
        target
    } else {
        0.0
    }
}

fn probability_threshold_positive_evidence(
    probabilities: &[f64],
    labels: &[f64],
    rows: &[&crate::ProbabilityTrainingRow],
    horizon_days: u32,
    threshold: f64,
) -> ProbabilityThresholdPositiveEvidence {
    let mut evidence = ProbabilityThresholdPositiveEvidence::default();
    for ((probability, label), row) in probabilities.iter().zip(labels).zip(rows.iter().copied()) {
        let effective_label =
            probability_threshold_effective_positive_label(row, *label, horizon_days);
        if *label >= 0.5 {
            evidence.actual_positive_count += 1;
        }
        if effective_label > 0.0 {
            evidence.effective_positive_unit_count += 1;
            evidence.effective_positive_mass += effective_label;
        }
        if *probability >= threshold {
            evidence.predicted_positive_count += 1;
            if *label >= 0.5 {
                evidence.true_positive_count += 1;
            }
            evidence.effective_true_positive_mass += effective_label;
        }
    }

    evidence
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
    if !matches!(horizon_days, 5 | 20 | 60) || hits.positive_window_row_count == 0 {
        return true;
    }

    let positive_window_hit_rate = hits.positive_window_hit_rate();
    hits.positive_window_hit_count > 0
        && positive_window_hit_rate >= regime_positive_window_min_hit_rate(horizon_days)
        && (positive_window_hit_rate - hits.normal_hit_rate())
            >= regime_floor_min_gap_vs_normal(horizon_days)
        && (hits.cooldown_row_count == 0
            || positive_window_hit_rate
                >= hits.cooldown_hit_rate() + regime_floor_min_gap_vs_normal(horizon_days))
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

#[derive(Clone, Copy, Debug)]
struct ProbabilityThresholdRepairCandidateAssessment {
    threshold: f64,
    reason: &'static str,
    hits: ProbabilityThresholdRegimeHitSummary,
    evidence: ProbabilityThresholdPositiveEvidence,
}

fn assess_probability_threshold_repair_candidate(
    probabilities: &[f64],
    labels: &[f64],
    rows: &[&crate::ProbabilityTrainingRow],
    horizon_days: u32,
    threshold: f64,
    relaxed_prediction_ceiling: u32,
) -> ProbabilityThresholdRepairCandidateAssessment {
    let hits =
        probability_threshold_regime_hit_summary(probabilities, rows, horizon_days, threshold);
    let evidence = probability_threshold_positive_evidence(
        probabilities,
        labels,
        rows,
        horizon_days,
        threshold,
    );
    let reason = if hits.early_warning_hit_count == 0 {
        "no_early_warning_hit"
    } else if !threshold_has_usable_repair_candidate_support(hits, horizon_days) {
        "regime_support_rejected"
    } else if !evidence.has_true_positive_support() || evidence.predicted_positive_count == 0 {
        "no_positive_support"
    } else if evidence.predicted_positive_count > relaxed_prediction_ceiling {
        "prediction_ceiling_exceeded"
    } else {
        "accepted"
    };

    ProbabilityThresholdRepairCandidateAssessment {
        threshold,
        reason,
        hits,
        evidence,
    }
}

pub(in super::super) fn probability_threshold_repair_candidate_diagnostics(
    base_threshold: f64,
    probabilities: &[f64],
    labels: &[f64],
    rows: &[&crate::ProbabilityTrainingRow],
    horizon_days: u32,
    label_mode: crate::ProbabilityTargetLabelMode,
) -> Option<ProbabilityThresholdRepairCandidateDiagnostics> {
    if label_mode != crate::ProbabilityTargetLabelMode::ForwardCrisis
        || !matches!(horizon_days, 20 | 60)
        || probabilities.is_empty()
        || rows.is_empty()
        || probabilities.len() != rows.len()
    {
        return None;
    }

    let positive_pool = probability_threshold_positive_evidence(
        probabilities,
        labels,
        rows,
        horizon_days,
        f64::INFINITY,
    );
    let relaxed_prediction_ceiling = regime_aware_threshold_prediction_ceiling(
        positive_pool.effective_positive_count_for_ceiling(),
        horizon_days,
    );
    let mut candidate_count = 0_u32;
    let mut accepted_candidate_count = 0_u32;
    let mut rejected_no_early_warning_hit_count = 0_u32;
    let mut rejected_regime_support_count = 0_u32;
    let mut rejected_no_positive_support_count = 0_u32;
    let mut rejected_prediction_ceiling_count = 0_u32;
    let mut best_rejected = None::<ProbabilityThresholdRepairCandidateAssessment>;

    for threshold in probability_threshold_candidates(probabilities) {
        if threshold >= base_threshold {
            continue;
        }
        candidate_count += 1;
        let assessment = assess_probability_threshold_repair_candidate(
            probabilities,
            labels,
            rows,
            horizon_days,
            threshold,
            relaxed_prediction_ceiling,
        );
        match assessment.reason {
            "accepted" => accepted_candidate_count += 1,
            "no_early_warning_hit" => rejected_no_early_warning_hit_count += 1,
            "regime_support_rejected" => rejected_regime_support_count += 1,
            "no_positive_support" => rejected_no_positive_support_count += 1,
            "prediction_ceiling_exceeded" => rejected_prediction_ceiling_count += 1,
            _ => {}
        }

        if assessment.reason != "accepted"
            && best_rejected.is_none_or(|best| {
                repair_candidate_diagnostic_score(assessment)
                    > repair_candidate_diagnostic_score(best)
            })
        {
            best_rejected = Some(assessment);
        }
    }

    let (
        best_rejected_reason,
        best_rejected_threshold,
        best_rejected_early_warning_hit_rate,
        best_rejected_positive_window_hit_rate,
        best_rejected_normal_hit_rate,
        best_rejected_cooldown_hit_rate,
        best_rejected_predicted_positive_count,
    ) = if let Some(best) = best_rejected {
        (
            best.reason.to_string(),
            Some(crate::round3(best.threshold)),
            crate::round3(best.hits.early_warning_hit_rate()),
            crate::round3(best.hits.positive_window_hit_rate()),
            crate::round3(best.hits.normal_hit_rate()),
            crate::round3(best.hits.cooldown_hit_rate()),
            best.evidence.predicted_positive_count,
        )
    } else {
        ("none".to_string(), None, 0.0, 0.0, 0.0, 0.0, 0)
    };

    Some(ProbabilityThresholdRepairCandidateDiagnostics {
        candidate_count,
        accepted_candidate_count,
        rejected_no_early_warning_hit_count,
        rejected_regime_support_count,
        rejected_no_positive_support_count,
        rejected_prediction_ceiling_count,
        best_rejected_reason,
        best_rejected_threshold,
        best_rejected_early_warning_hit_rate,
        best_rejected_positive_window_hit_rate,
        best_rejected_normal_hit_rate,
        best_rejected_cooldown_hit_rate,
        best_rejected_predicted_positive_count,
    })
}

fn repair_candidate_diagnostic_score(
    assessment: ProbabilityThresholdRepairCandidateAssessment,
) -> (bool, bool, i64, i64, i64, i64, i64) {
    (
        assessment.hits.early_warning_hit_count > 0,
        assessment.evidence.has_true_positive_support(),
        ((assessment.hits.positive_window_hit_rate() - assessment.hits.cooldown_hit_rate())
            * 1_000_000.0)
            .round() as i64,
        ((assessment.hits.early_warning_hit_rate() - assessment.hits.normal_hit_rate())
            * 1_000_000.0)
            .round() as i64,
        (assessment.evidence.effective_true_positive_mass * 1_000_000.0).round() as i64,
        -((assessment.hits.cooldown_hit_rate() * 1_000_000.0).round() as i64),
        -((assessment.threshold * 1_000.0).round() as i64),
    )
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

        let evidence = probability_threshold_positive_evidence(
            probabilities,
            labels,
            rows,
            horizon_days,
            threshold,
        );
        if evidence.has_true_positive_support()
            && evidence.predicted_positive_count > 0
            && evidence.predicted_positive_count <= relaxed_prediction_ceiling
        {
            return true;
        }
    }

    false
}

fn conservative_forward_crisis_threshold(
    probabilities: &[f64],
    labels: &[f64],
    rows: &[&crate::ProbabilityTrainingRow],
    horizon_days: u32,
    base_threshold: f64,
    relaxed_prediction_ceiling: u32,
) -> f64 {
    let mut best_threshold = None::<f64>;
    for threshold in probability_threshold_candidates(probabilities) {
        if threshold < base_threshold {
            continue;
        }

        let hits =
            probability_threshold_regime_hit_summary(probabilities, rows, horizon_days, threshold);
        if !threshold_has_usable_forward_crisis_support(hits, horizon_days) {
            continue;
        }

        let evidence = probability_threshold_positive_evidence(
            probabilities,
            labels,
            rows,
            horizon_days,
            threshold,
        );
        if !evidence.has_true_positive_support()
            || evidence.predicted_positive_count == 0
            || evidence.predicted_positive_count > relaxed_prediction_ceiling
        {
            continue;
        }

        if best_threshold.is_none_or(|best| threshold < best) {
            best_threshold = Some(threshold);
        }
    }

    best_threshold.unwrap_or(0.99)
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
        || !matches!(horizon_days, 5 | 20 | 60)
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

    let positive_pool = probability_threshold_positive_evidence(
        probabilities,
        labels,
        rows,
        horizon_days,
        f64::INFINITY,
    );
    let actual_positive_count = positive_pool.effective_positive_count_for_ceiling();
    let positive_count = positive_pool.effective_positive_mass;
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
    let conservative_threshold = || {
        conservative_forward_crisis_threshold(
            probabilities,
            labels,
            rows,
            horizon_days,
            base_threshold,
            relaxed_prediction_ceiling,
        )
    };
    if regime_summary
        .early_warning_lift_vs_normal
        .unwrap_or_default()
        < 1.5
        && !over_tight_base_threshold
    {
        return crate::round3(conservative_threshold()).clamp(base_threshold, 0.99);
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

        let evidence = probability_threshold_positive_evidence(
            probabilities,
            labels,
            rows,
            horizon_days,
            threshold,
        );
        if evidence.predicted_positive_count == 0 || !evidence.has_true_positive_support() {
            continue;
        }

        let precision =
            evidence.effective_true_positive_mass / evidence.predicted_positive_count as f64;
        let recall = evidence.effective_true_positive_mass / positive_count;
        let f_beta = if precision > 0.0 || recall > 0.0 {
            (1.0 + beta_sq) * precision * recall / (beta_sq * precision + recall).max(1e-9)
        } else {
            0.0
        };

        let normal_hit_rate = hits.normal_hit_rate();
        let cooldown_hit_rate = hits.cooldown_hit_rate();
        let score = (
            early_warning_hit_rate >= regime_floor_min_hit_rate(horizon_days),
            evidence.predicted_positive_count <= relaxed_prediction_ceiling,
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

    let repaired_threshold = if best_score.is_some()
        && early_warning_probability_cap > 0.0
        && early_warning_probability_cap < base_threshold
    {
        best_threshold.min(early_warning_probability_cap)
    } else if best_score.is_some() {
        best_threshold
    } else {
        conservative_threshold()
    };

    let lower_bound = if best_score.is_some() {
        0.005
    } else {
        base_threshold
    };
    crate::round3(repaired_threshold).clamp(lower_bound, 0.99)
}

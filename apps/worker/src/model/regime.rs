use crate::{
    ProbabilityFeatureStat, ProbabilityTargetLabelMode, ProbabilityTrainingRegime,
    ProbabilityTrainingRow,
};

use super::{dot, normalized_features, sigmoid};

#[derive(Debug, Clone)]
pub(crate) struct RegimePairwiseTarget {
    left_centroid: Vec<f64>,
    right_centroid: Vec<f64>,
    margin: f64,
    weight: f64,
}

pub(crate) fn forward_crisis_regime_pairwise_targets(
    rows: &[ProbabilityTrainingRow],
    feature_stats: &[ProbabilityFeatureStat],
    horizon_days: u32,
    label_mode: ProbabilityTargetLabelMode,
    uses_interaction_tail: bool,
) -> Vec<RegimePairwiseTarget> {
    if !matches!(label_mode, ProbabilityTargetLabelMode::ForwardCrisis) {
        return Vec::new();
    }

    let target_specs = match horizon_days {
        5 if uses_interaction_tail => vec![
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::Normal,
                0.45,
                1.35,
            ),
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::PostCrisisCooldown,
                0.30,
                1.05,
            ),
            (
                ProbabilityTrainingRegime::PreWarningBuffer,
                ProbabilityTrainingRegime::Normal,
                0.15,
                0.50,
            ),
        ],
        20 => vec![
            (
                ProbabilityTrainingRegime::PreWarningBuffer,
                ProbabilityTrainingRegime::Normal,
                if uses_interaction_tail { 1.00 } else { 0.85 },
                if uses_interaction_tail { 1.40 } else { 1.25 },
            ),
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::Normal,
                if uses_interaction_tail { 0.55 } else { 0.40 },
                if uses_interaction_tail { 1.00 } else { 0.85 },
            ),
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::PreWarningBuffer,
                if uses_interaction_tail { 0.40 } else { 0.35 },
                if uses_interaction_tail { 0.80 } else { 0.70 },
            ),
            (
                ProbabilityTrainingRegime::PreWarningBuffer,
                ProbabilityTrainingRegime::PostCrisisCooldown,
                if uses_interaction_tail { 0.90 } else { 0.70 },
                if uses_interaction_tail { 1.25 } else { 1.10 },
            ),
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::PostCrisisCooldown,
                if uses_interaction_tail { 0.70 } else { 0.45 },
                if uses_interaction_tail { 1.05 } else { 0.80 },
            ),
        ],
        60 => vec![
            (
                ProbabilityTrainingRegime::PreWarningBuffer,
                ProbabilityTrainingRegime::Normal,
                if uses_interaction_tail { 1.25 } else { 1.05 },
                if uses_interaction_tail { 1.55 } else { 1.30 },
            ),
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::Normal,
                if uses_interaction_tail { 0.85 } else { 0.65 },
                if uses_interaction_tail { 1.20 } else { 0.95 },
            ),
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::PreWarningBuffer,
                if uses_interaction_tail { 0.60 } else { 0.45 },
                if uses_interaction_tail { 0.80 } else { 0.60 },
            ),
            (
                ProbabilityTrainingRegime::PreWarningBuffer,
                ProbabilityTrainingRegime::PostCrisisCooldown,
                if uses_interaction_tail { 1.15 } else { 0.90 },
                if uses_interaction_tail { 1.60 } else { 1.30 },
            ),
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::PostCrisisCooldown,
                if uses_interaction_tail { 1.20 } else { 0.95 },
                if uses_interaction_tail { 1.30 } else { 1.00 },
            ),
        ],
        _ => Vec::new(),
    };

    target_specs
        .into_iter()
        .filter_map(|(left, right, margin, weight)| {
            let left_centroid = regime_centroid(rows, feature_stats, horizon_days, left)?;
            let right_centroid = regime_centroid(rows, feature_stats, horizon_days, right)?;
            Some(RegimePairwiseTarget {
                left_centroid,
                right_centroid,
                margin,
                weight,
            })
        })
        .collect()
}

fn regime_centroid(
    rows: &[ProbabilityTrainingRow],
    feature_stats: &[ProbabilityFeatureStat],
    horizon_days: u32,
    regime: ProbabilityTrainingRegime,
) -> Option<Vec<f64>> {
    let feature_len = feature_stats.len();
    let mut sum = vec![0.0; feature_len];
    let mut count = 0_usize;
    for row in rows {
        if row.regime_for_horizon(horizon_days) != regime {
            continue;
        }
        let normalized = normalized_features(row, feature_stats);
        for (index, value) in normalized.into_iter().enumerate() {
            sum[index] += value;
        }
        count += 1;
    }
    (count > 0).then(|| {
        sum.into_iter()
            .map(|value| value / count as f64)
            .collect::<Vec<_>>()
    })
}

fn regime_pairwise_strength(horizon_days: u32, uses_interaction_tail: bool) -> f64 {
    match (horizon_days, uses_interaction_tail) {
        (5, true) => 0.70,
        (20, true) => 1.00,
        (60, true) => 1.35,
        (20, false) => 0.80,
        (60, false) => 1.15,
        _ => 0.0,
    }
}

pub(crate) fn apply_regime_pairwise_gradient(
    weight_gradients: &mut [f64],
    weights: &[f64],
    targets: &[RegimePairwiseTarget],
    sample_weight_sum: f64,
    horizon_days: u32,
    uses_interaction_tail: bool,
) {
    if targets.is_empty() {
        return;
    }
    let strength = regime_pairwise_strength(horizon_days, uses_interaction_tail);
    if strength <= 0.0 {
        return;
    }
    let scale = sample_weight_sum * strength / targets.len() as f64;
    for target in targets {
        let left_logit = dot(weights, &target.left_centroid);
        let right_logit = dot(weights, &target.right_centroid);
        let pressure = sigmoid(right_logit + target.margin - left_logit);
        for (index, gradient) in weight_gradients.iter_mut().enumerate() {
            *gradient += target.weight
                * pressure
                * (target.right_centroid[index] - target.left_centroid[index])
                * scale;
        }
    }
}

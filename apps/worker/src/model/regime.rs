use crate::{
    ProbabilityFeatureStat, ProbabilityTargetLabelMode, ProbabilityTrainingRegime,
    ProbabilityTrainingRow,
};

use super::{dot, normalized_features, sigmoid};

#[derive(Debug, Clone)]
pub(crate) struct RegimePairwiseTarget {
    #[cfg(test)]
    pub(crate) left_regime: ProbabilityTrainingRegime,
    #[cfg(test)]
    pub(crate) right_regime: ProbabilityTrainingRegime,
    left_centroid: Vec<f64>,
    right_centroid: Vec<f64>,
    pub(crate) margin: f64,
    pub(crate) weight: f64,
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
                0.60,
                1.60,
            ),
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::PostCrisisCooldown,
                0.40,
                1.20,
            ),
            (
                ProbabilityTrainingRegime::PreWarningBuffer,
                ProbabilityTrainingRegime::Normal,
                0.25,
                0.70,
            ),
            (
                ProbabilityTrainingRegime::PostCrisisCooldown,
                ProbabilityTrainingRegime::Normal,
                0.15,
                0.50,
            ),
        ],
        20 if uses_interaction_tail => vec![
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::Normal,
                0.95,
                1.50,
            ),
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::PostCrisisCooldown,
                1.10,
                1.60,
            ),
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::PreWarningBuffer,
                0.55,
                1.05,
            ),
            (
                ProbabilityTrainingRegime::PreWarningBuffer,
                ProbabilityTrainingRegime::Normal,
                0.85,
                0.90,
            ),
            (
                ProbabilityTrainingRegime::PreWarningBuffer,
                ProbabilityTrainingRegime::PostCrisisCooldown,
                0.75,
                0.85,
            ),
            (
                ProbabilityTrainingRegime::PostCrisisCooldown,
                ProbabilityTrainingRegime::Normal,
                0.30,
                0.60,
            ),
        ],
        20 => vec![
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::Normal,
                0.65,
                1.15,
            ),
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::PostCrisisCooldown,
                0.75,
                1.20,
            ),
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::PreWarningBuffer,
                0.40,
                0.85,
            ),
            (
                ProbabilityTrainingRegime::PreWarningBuffer,
                ProbabilityTrainingRegime::Normal,
                0.70,
                0.75,
            ),
            (
                ProbabilityTrainingRegime::PreWarningBuffer,
                ProbabilityTrainingRegime::PostCrisisCooldown,
                0.55,
                0.70,
            ),
            (
                ProbabilityTrainingRegime::PostCrisisCooldown,
                ProbabilityTrainingRegime::Normal,
                0.25,
                0.50,
            ),
        ],
        60 if uses_interaction_tail => vec![
            (
                ProbabilityTrainingRegime::PreWarningBuffer,
                ProbabilityTrainingRegime::Normal,
                1.45,
                1.80,
            ),
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::Normal,
                1.05,
                1.45,
            ),
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::PreWarningBuffer,
                0.65,
                0.85,
            ),
            (
                ProbabilityTrainingRegime::PreWarningBuffer,
                ProbabilityTrainingRegime::PostCrisisCooldown,
                1.90,
                2.85,
            ),
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::PostCrisisCooldown,
                2.00,
                2.60,
            ),
            (
                ProbabilityTrainingRegime::PostCrisisCooldown,
                ProbabilityTrainingRegime::Normal,
                0.30,
                0.60,
            ),
        ],
        60 => vec![
            (
                ProbabilityTrainingRegime::PreWarningBuffer,
                ProbabilityTrainingRegime::Normal,
                1.10,
                1.35,
            ),
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::Normal,
                0.70,
                1.00,
            ),
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::PreWarningBuffer,
                0.45,
                0.60,
            ),
            (
                ProbabilityTrainingRegime::PreWarningBuffer,
                ProbabilityTrainingRegime::PostCrisisCooldown,
                1.35,
                1.95,
            ),
            (
                ProbabilityTrainingRegime::PositiveWindow,
                ProbabilityTrainingRegime::PostCrisisCooldown,
                1.40,
                1.80,
            ),
            (
                ProbabilityTrainingRegime::PostCrisisCooldown,
                ProbabilityTrainingRegime::Normal,
                0.35,
                0.70,
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
                #[cfg(test)]
                left_regime: left,
                #[cfg(test)]
                right_regime: right,
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
        (5, true) => 0.90,
        (20, true) => 1.05,
        (60, true) => 1.25,
        (20, false) => 0.80,
        (60, false) => 1.35,
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

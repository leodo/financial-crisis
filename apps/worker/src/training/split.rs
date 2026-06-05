use anyhow::bail;

use super::{ProbabilityTargetLabelMode, ProbabilityTrainingRow};

pub(crate) fn chronological_split(
    dataset: &[ProbabilityTrainingRow],
) -> anyhow::Result<(
    Vec<ProbabilityTrainingRow>,
    Vec<ProbabilityTrainingRow>,
    Vec<ProbabilityTrainingRow>,
)> {
    let (train_end, calibration_end) = chronological_split_bounds(dataset.len())?;
    Ok((
        dataset[..train_end].to_vec(),
        dataset[train_end..calibration_end].to_vec(),
        dataset[calibration_end..].to_vec(),
    ))
}

pub(crate) fn validate_split_bounds(
    dataset_len: usize,
    train_end: usize,
    calibration_end: usize,
) -> anyhow::Result<()> {
    if dataset_len < 30 {
        bail!("dataset is too small for chronological split");
    }
    if train_end < 30 || calibration_end <= train_end + 10 || calibration_end >= dataset_len {
        bail!("unable to construct train/calibration/evaluation split");
    }
    if dataset_len.saturating_sub(calibration_end) < 10 {
        bail!("evaluation split would be too small");
    }
    Ok(())
}

pub(crate) fn chronological_split_bounds(dataset_len: usize) -> anyhow::Result<(usize, usize)> {
    let train_end = (dataset_len * 6 / 10)
        .max(30)
        .min(dataset_len.saturating_sub(20));
    let calibration_end = (dataset_len * 8 / 10)
        .max(train_end + 10)
        .min(dataset_len.saturating_sub(10));
    validate_split_bounds(dataset_len, train_end, calibration_end)?;
    Ok((train_end, calibration_end))
}

pub(crate) fn training_rows_support_label_mode(
    train_rows: &[ProbabilityTrainingRow],
    calibration_rows: &[ProbabilityTrainingRow],
    evaluation_rows: &[ProbabilityTrainingRow],
    label_mode: ProbabilityTargetLabelMode,
) -> bool {
    [5_u32, 20_u32, 60_u32].into_iter().all(|horizon_days| {
        train_rows
            .iter()
            .any(|row| row.label_for_horizon(label_mode, horizon_days) > 0.0)
            && calibration_rows
                .iter()
                .any(|row| row.label_for_horizon(label_mode, horizon_days) > 0.0)
            && evaluation_rows
                .iter()
                .any(|row| row.label_for_horizon(label_mode, horizon_days) > 0.0)
    })
}

pub(crate) fn ensure_positive_labels(
    rows: &[ProbabilityTrainingRow],
    horizon_days: u32,
    split_name: &str,
    label_mode: ProbabilityTargetLabelMode,
) -> anyhow::Result<()> {
    let positives = rows
        .iter()
        .filter(|row| row.label_for_horizon(label_mode, horizon_days) > 0.0)
        .count();
    if positives == 0 {
        bail!(
            "no positive {horizon_days}d {} labels found in the {split_name} split",
            label_mode.as_str()
        );
    }
    Ok(())
}

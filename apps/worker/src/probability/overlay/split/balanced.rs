use super::{
    family_overlay_row_flags, family_overlay_split_result_from_parts, FamilyOverlayAuditSpec,
    FamilyOverlayRowFlags, FamilyOverlaySplitBucket, FamilyOverlaySplitResult,
    FamilyOverlaySplitValidationContext,
};

pub(super) fn build_family_overlay_balanced_split_result(
    rows: &[crate::ProbabilityTrainingRow],
    spec: &FamilyOverlayAuditSpec,
    horizon_days: u32,
    label_mode: crate::ProbabilityTargetLabelMode,
) -> anyhow::Result<FamilyOverlaySplitResult> {
    let [train_target, calibration_target, evaluation_target] =
        family_overlay_balanced_targets(rows.len())?;
    let mut buckets = [
        FamilyOverlaySplitBucket::default(),
        FamilyOverlaySplitBucket::default(),
        FamilyOverlaySplitBucket::default(),
    ];

    for row in rows.iter().cloned() {
        let flags = family_overlay_row_flags(&row, spec, horizon_days, label_mode);
        let bucket_index = choose_family_overlay_balanced_bucket(
            &buckets,
            [train_target, calibration_target, evaluation_target],
            flags,
        );
        buckets[bucket_index].push(row, flags);
    }

    family_overlay_split_result_from_parts(
        buckets[0].rows.clone(),
        buckets[1].rows.clone(),
        buckets[2].rows.clone(),
        FamilyOverlaySplitValidationContext {
            strategy: "balanced",
            spec,
            horizon_days,
            label_mode,
        },
        None,
    )
}

fn family_overlay_balanced_targets(row_count: usize) -> anyhow::Result<[usize; 3]> {
    const MIN_TRAIN_ROWS: usize = 3;
    const MIN_CALIBRATION_ROWS: usize = 2;
    const MIN_EVALUATION_ROWS: usize = 2;

    if row_count < MIN_TRAIN_ROWS + MIN_CALIBRATION_ROWS + MIN_EVALUATION_ROWS {
        anyhow::bail!("not enough rows for balanced family overlay split");
    }

    let mut train_target = (row_count * 6 / 10).max(MIN_TRAIN_ROWS);
    if train_target > row_count.saturating_sub(MIN_CALIBRATION_ROWS + MIN_EVALUATION_ROWS) {
        train_target = row_count.saturating_sub(MIN_CALIBRATION_ROWS + MIN_EVALUATION_ROWS);
    }

    let mut calibration_target = (row_count * 2 / 10).max(MIN_CALIBRATION_ROWS);
    let max_calibration_rows = row_count.saturating_sub(train_target + MIN_EVALUATION_ROWS);
    if calibration_target > max_calibration_rows {
        calibration_target = max_calibration_rows;
    }

    let evaluation_target = row_count.saturating_sub(train_target + calibration_target);
    if evaluation_target < MIN_EVALUATION_ROWS {
        anyhow::bail!("balanced family overlay split would leave evaluation too small");
    }

    Ok([train_target, calibration_target, evaluation_target])
}

fn choose_family_overlay_balanced_bucket(
    buckets: &[FamilyOverlaySplitBucket; 3],
    targets: [usize; 3],
    flags: FamilyOverlayRowFlags,
) -> usize {
    if flags.positive {
        for bucket_index in [0_usize, 1, 2] {
            if buckets[bucket_index].counts.positive == 0 {
                return bucket_index;
            }
        }
    }

    if flags.early_warning {
        for bucket_index in [1_usize, 2, 0] {
            if buckets[bucket_index].counts.early_warning == 0 {
                return bucket_index;
            }
        }
    }

    if flags.gate_active {
        for (bucket_index, minimum_gate_active) in [(0_usize, 2_usize), (2, 1), (1, 0)] {
            if buckets[bucket_index].counts.gate_active < minimum_gate_active {
                return bucket_index;
            }
        }
    }

    let mut best_index = 0_usize;
    let mut best_shortage = targets[0].saturating_sub(buckets[0].rows.len());
    let mut best_size = buckets[0].rows.len();
    for bucket_index in 1..3 {
        let shortage = targets[bucket_index].saturating_sub(buckets[bucket_index].rows.len());
        let size = buckets[bucket_index].rows.len();
        if shortage > best_shortage || (shortage == best_shortage && size < best_size) {
            best_index = bucket_index;
            best_shortage = shortage;
            best_size = size;
        }
    }
    best_index
}

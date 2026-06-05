mod balanced;
mod bounds;
mod dataset;

use super::audit::FamilyOverlayAuditSpec;

use balanced::build_family_overlay_balanced_split_result;
use bounds::family_overlay_split_bounds;

#[derive(Debug, Default)]
struct FamilyOverlaySplitSupport {
    positive_counts: Vec<usize>,
    early_warning_counts: Vec<usize>,
    gate_active_counts: Vec<usize>,
}

#[derive(Debug)]
pub(super) struct FamilyOverlaySplitResult {
    pub(super) train_rows: Vec<crate::ProbabilityTrainingRow>,
    pub(super) calibration_rows: Vec<crate::ProbabilityTrainingRow>,
    pub(super) evaluation_rows: Vec<crate::ProbabilityTrainingRow>,
    pub(super) strategy: &'static str,
}

#[derive(Debug, Clone, Copy)]
struct FamilyOverlaySplitValidationContext<'a> {
    strategy: &'static str,
    spec: &'a FamilyOverlayAuditSpec,
    horizon_days: u32,
    label_mode: crate::ProbabilityTargetLabelMode,
}

#[derive(Debug, Default, Clone, Copy)]
struct FamilyOverlayBucketCounts {
    positive: usize,
    early_warning: usize,
    gate_active: usize,
}

#[derive(Debug, Default, Clone)]
struct FamilyOverlaySplitBucket {
    rows: Vec<crate::ProbabilityTrainingRow>,
    counts: FamilyOverlayBucketCounts,
}

#[derive(Debug, Clone, Copy)]
struct FamilyOverlayRowFlags {
    positive: bool,
    early_warning: bool,
    gate_active: bool,
}

impl FamilyOverlaySplitBucket {
    fn push(&mut self, row: crate::ProbabilityTrainingRow, flags: FamilyOverlayRowFlags) {
        self.counts.positive += usize::from(flags.positive);
        self.counts.early_warning += usize::from(flags.early_warning);
        self.counts.gate_active += usize::from(flags.gate_active);
        self.rows.push(row);
    }
}

impl FamilyOverlaySplitSupport {
    fn from_rows(
        rows: &[crate::ProbabilityTrainingRow],
        spec: &FamilyOverlayAuditSpec,
        horizon_days: u32,
        label_mode: crate::ProbabilityTargetLabelMode,
    ) -> Self {
        let early_warning_regime = super::super::probability_early_warning_regime(horizon_days);
        let mut support = Self {
            positive_counts: Vec::with_capacity(rows.len() + 1),
            early_warning_counts: Vec::with_capacity(rows.len() + 1),
            gate_active_counts: Vec::with_capacity(rows.len() + 1),
        };
        support.positive_counts.push(0);
        support.early_warning_counts.push(0);
        support.gate_active_counts.push(0);

        for row in rows {
            let gate_value =
                crate::resolve_probability_feature_value(spec.gate_feature, &row.features)
                    .unwrap_or(0.0);
            support.positive_counts.push(
                support.positive_counts.last().copied().unwrap_or_default()
                    + usize::from(row.label_for_horizon(label_mode, horizon_days) > 0.0),
            );
            support.early_warning_counts.push(
                support
                    .early_warning_counts
                    .last()
                    .copied()
                    .unwrap_or_default()
                    + usize::from(row.regime_for_horizon(horizon_days) == early_warning_regime),
            );
            support.gate_active_counts.push(
                support
                    .gate_active_counts
                    .last()
                    .copied()
                    .unwrap_or_default()
                    + usize::from(gate_value >= spec.gate_active_threshold),
            );
        }

        support
    }

    fn split_has_required_support(
        &self,
        start: usize,
        end: usize,
        min_positive: usize,
        min_early_warning: usize,
        min_gate_active: usize,
    ) -> bool {
        end > start
            && self.positive_count(start, end) >= min_positive
            && self.early_warning_count(start, end) >= min_early_warning
            && self.gate_active_count(start, end) >= min_gate_active
    }

    fn positive_count(&self, start: usize, end: usize) -> usize {
        self.positive_counts[end] - self.positive_counts[start]
    }

    fn early_warning_count(&self, start: usize, end: usize) -> usize {
        self.early_warning_counts[end] - self.early_warning_counts[start]
    }

    fn gate_active_count(&self, start: usize, end: usize) -> usize {
        self.gate_active_counts[end] - self.gate_active_counts[start]
    }
}

pub(super) fn build_family_overlay_dataset_rows(
    train_rows: &[crate::ProbabilityTrainingRow],
    calibration_rows: &[crate::ProbabilityTrainingRow],
    evaluation_rows: &[crate::ProbabilityTrainingRow],
    spec: &FamilyOverlayAuditSpec,
    horizon_days: u32,
    label_mode: crate::ProbabilityTargetLabelMode,
) -> Vec<crate::ProbabilityTrainingRow> {
    dataset::build_family_overlay_dataset_rows(
        train_rows,
        calibration_rows,
        evaluation_rows,
        spec,
        horizon_days,
        label_mode,
    )
}

pub(super) fn split_family_overlay_dataset_rows(
    rows: &[crate::ProbabilityTrainingRow],
    spec: &FamilyOverlayAuditSpec,
    horizon_days: u32,
    label_mode: crate::ProbabilityTargetLabelMode,
) -> anyhow::Result<FamilyOverlaySplitResult> {
    let family_aware_error = match family_overlay_split_bounds(rows, spec, horizon_days, label_mode)
    {
        Ok((train_end, calibration_end)) => match family_overlay_split_result_from_parts(
            rows[..train_end].to_vec(),
            rows[train_end..calibration_end].to_vec(),
            rows[calibration_end..].to_vec(),
            FamilyOverlaySplitValidationContext {
                strategy: "family_aware",
                spec,
                horizon_days,
                label_mode,
            },
            None,
        ) {
            Ok(split) => return Ok(split),
            Err(error) => error.to_string(),
        },
        Err(error) => error.to_string(),
    };

    let balanced_error =
        match build_family_overlay_balanced_split_result(rows, spec, horizon_days, label_mode) {
            Ok(split) => return Ok(split),
            Err(error) => error.to_string(),
        };

    let (train_end, calibration_end) = crate::chronological_split_bounds(rows.len())?;
    family_overlay_split_result_from_parts(
        rows[..train_end].to_vec(),
        rows[train_end..calibration_end].to_vec(),
        rows[calibration_end..].to_vec(),
        FamilyOverlaySplitValidationContext {
            strategy: "chronological",
            spec,
            horizon_days,
            label_mode,
        },
        Some(format!(
            " family_aware_error={family_aware_error} balanced_error={balanced_error}",
        )),
    )
}

fn family_overlay_split_result_from_parts(
    train_rows: Vec<crate::ProbabilityTrainingRow>,
    calibration_rows: Vec<crate::ProbabilityTrainingRow>,
    evaluation_rows: Vec<crate::ProbabilityTrainingRow>,
    context: FamilyOverlaySplitValidationContext<'_>,
    extra_error: Option<String>,
) -> anyhow::Result<FamilyOverlaySplitResult> {
    let train_counts = count_family_overlay_bucket_support(
        &train_rows,
        context.spec,
        context.horizon_days,
        context.label_mode,
    );
    let calibration_counts = count_family_overlay_bucket_support(
        &calibration_rows,
        context.spec,
        context.horizon_days,
        context.label_mode,
    );
    let evaluation_counts = count_family_overlay_bucket_support(
        &evaluation_rows,
        context.spec,
        context.horizon_days,
        context.label_mode,
    );
    if train_rows.is_empty()
        || calibration_rows.is_empty()
        || evaluation_rows.is_empty()
        || train_counts.positive == 0
        || calibration_counts.positive == 0
        || evaluation_counts.positive == 0
        || train_counts.gate_active < 2
        || evaluation_counts.gate_active < 1
    {
        anyhow::bail!(
            "family overlay split lacks label/gate support via {}: train p/e/g={}/{}/{} calib p/e/g={}/{}/{} eval p/e/g={}/{}/{}{}",
            context.strategy,
            train_counts.positive,
            train_counts.early_warning,
            train_counts.gate_active,
            calibration_counts.positive,
            calibration_counts.early_warning,
            calibration_counts.gate_active,
            evaluation_counts.positive,
            evaluation_counts.early_warning,
            evaluation_counts.gate_active,
            extra_error.unwrap_or_default(),
        );
    }

    Ok(FamilyOverlaySplitResult {
        train_rows,
        calibration_rows,
        evaluation_rows,
        strategy: context.strategy,
    })
}

fn count_family_overlay_bucket_support(
    rows: &[crate::ProbabilityTrainingRow],
    spec: &FamilyOverlayAuditSpec,
    horizon_days: u32,
    label_mode: crate::ProbabilityTargetLabelMode,
) -> FamilyOverlayBucketCounts {
    rows.iter()
        .fold(FamilyOverlayBucketCounts::default(), |mut counts, row| {
            let flags = family_overlay_row_flags(row, spec, horizon_days, label_mode);
            counts.positive += usize::from(flags.positive);
            counts.early_warning += usize::from(flags.early_warning);
            counts.gate_active += usize::from(flags.gate_active);
            counts
        })
}

fn family_overlay_row_flags(
    row: &crate::ProbabilityTrainingRow,
    spec: &FamilyOverlayAuditSpec,
    horizon_days: u32,
    label_mode: crate::ProbabilityTargetLabelMode,
) -> FamilyOverlayRowFlags {
    let gate_value =
        crate::resolve_probability_feature_value(spec.gate_feature, &row.features).unwrap_or(0.0);
    FamilyOverlayRowFlags {
        positive: row.label_for_horizon(label_mode, horizon_days) > 0.0,
        early_warning: row.regime_for_horizon(horizon_days)
            == super::super::probability_early_warning_regime(horizon_days),
        gate_active: gate_value >= spec.gate_active_threshold,
    }
}

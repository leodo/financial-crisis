use std::collections::BTreeMap;

use super::audit::FamilyOverlayAuditSpec;

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
    let all_rows = train_rows
        .iter()
        .chain(calibration_rows.iter())
        .chain(evaluation_rows.iter())
        .cloned()
        .collect::<Vec<_>>();
    build_family_overlay_split_rows(&all_rows, spec, horizon_days, label_mode)
}

fn build_family_overlay_split_rows(
    rows: &[crate::ProbabilityTrainingRow],
    spec: &FamilyOverlayAuditSpec,
    horizon_days: u32,
    label_mode: crate::ProbabilityTargetLabelMode,
) -> Vec<crate::ProbabilityTrainingRow> {
    let candidate_rows = rows
        .iter()
        .filter(|row| family_overlay_candidate_row(row, spec))
        .cloned()
        .collect::<Vec<_>>();
    if candidate_rows.is_empty() {
        return Vec::new();
    }

    let background_cap = candidate_rows.len().max(12).saturating_mul(2);
    let gate_active_background = sample_probability_rows_evenly(
        rows.iter()
            .filter(|row| family_overlay_gate_active_background_row(row, spec))
            .cloned()
            .collect(),
        background_cap,
    );
    let normal_background = sample_probability_rows_evenly(
        rows.iter()
            .filter(|row| family_overlay_normal_background_row(row, spec, horizon_days, label_mode))
            .cloned()
            .collect(),
        background_cap,
    );

    dedupe_probability_training_rows(
        candidate_rows
            .into_iter()
            .chain(gate_active_background)
            .chain(normal_background)
            .collect(),
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

fn build_family_overlay_balanced_split_result(
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

fn family_overlay_split_bounds(
    rows: &[crate::ProbabilityTrainingRow],
    spec: &FamilyOverlayAuditSpec,
    horizon_days: u32,
    label_mode: crate::ProbabilityTargetLabelMode,
) -> anyhow::Result<(usize, usize)> {
    if spec.scenario_family.is_none() {
        anyhow::bail!("proxy-only overlays do not use family-aware split bounds");
    }

    let ranges = collect_family_overlay_scenario_ranges(rows, spec);
    if ranges.len() < spec.min_scenario_count as usize {
        anyhow::bail!("not enough family scenario ranges for overlay split");
    }

    let (baseline_train_end, baseline_calibration_end) =
        crate::chronological_split_bounds(rows.len())?;
    let support = FamilyOverlaySplitSupport::from_rows(rows, spec, horizon_days, label_mode);
    let mut best_candidate = None::<(usize, usize, usize, usize, usize, usize, usize)>;

    for first_boundary_scenario in 0..ranges.len().saturating_sub(1) {
        let train_candidates =
            family_overlay_split_boundaries_within_range(&ranges[first_boundary_scenario]);
        for second_boundary_scenario in (first_boundary_scenario + 1)..ranges.len() {
            let calibration_candidates =
                family_overlay_split_boundaries_within_range(&ranges[second_boundary_scenario]);
            for &train_end in &train_candidates {
                for &calibration_end in &calibration_candidates {
                    if crate::validate_split_bounds(rows.len(), train_end, calibration_end).is_err()
                    {
                        continue;
                    }

                    let calibration_scenario_count =
                        crate::scenario_count_for_split_range(&ranges, train_end, calibration_end);
                    let evaluation_scenario_count =
                        crate::scenario_count_for_split_range(&ranges, calibration_end, rows.len());
                    if calibration_scenario_count == 0 || evaluation_scenario_count == 0 {
                        continue;
                    }

                    if !support.split_has_required_support(0, train_end, 1, 0, 2)
                        || !support.split_has_required_support(train_end, calibration_end, 1, 0, 0)
                        || !support.split_has_required_support(calibration_end, rows.len(), 1, 0, 1)
                    {
                        continue;
                    }

                    let scenario_coverage =
                        calibration_scenario_count.saturating_add(evaluation_scenario_count);
                    let early_warning_support_score = support
                        .early_warning_count(train_end, calibration_end)
                        .saturating_add(support.early_warning_count(calibration_end, rows.len()));
                    let gate_support_score = support
                        .gate_active_count(train_end, calibration_end)
                        .min(16)
                        .saturating_add(
                            support
                                .gate_active_count(calibration_end, rows.len())
                                .min(16),
                        );
                    let positive_support_score = support
                        .positive_count(train_end, calibration_end)
                        .saturating_add(support.positive_count(calibration_end, rows.len()));
                    let deviation_from_baseline = train_end.abs_diff(baseline_train_end)
                        + calibration_end.abs_diff(baseline_calibration_end);

                    let replace = match best_candidate {
                        None => true,
                        Some((
                            best_train_end,
                            best_calibration_end,
                            best_coverage,
                            best_early_score,
                            best_gate_score,
                            best_positive_score,
                            best_deviation,
                        )) => {
                            scenario_coverage > best_coverage
                                || (scenario_coverage == best_coverage
                                    && early_warning_support_score > best_early_score)
                                || (scenario_coverage == best_coverage
                                    && early_warning_support_score == best_early_score
                                    && gate_support_score > best_gate_score)
                                || (scenario_coverage == best_coverage
                                    && early_warning_support_score == best_early_score
                                    && gate_support_score == best_gate_score
                                    && positive_support_score > best_positive_score)
                                || (scenario_coverage == best_coverage
                                    && early_warning_support_score == best_early_score
                                    && gate_support_score == best_gate_score
                                    && positive_support_score == best_positive_score
                                    && deviation_from_baseline < best_deviation)
                                || (scenario_coverage == best_coverage
                                    && early_warning_support_score == best_early_score
                                    && gate_support_score == best_gate_score
                                    && positive_support_score == best_positive_score
                                    && deviation_from_baseline == best_deviation
                                    && (train_end > best_train_end
                                        || (train_end == best_train_end
                                            && calibration_end > best_calibration_end)))
                        }
                    };

                    if replace {
                        best_candidate = Some((
                            train_end,
                            calibration_end,
                            scenario_coverage,
                            early_warning_support_score,
                            gate_support_score,
                            positive_support_score,
                            deviation_from_baseline,
                        ));
                    }
                }
            }
        }
    }

    best_candidate
        .map(|(train_end, calibration_end, _, _, _, _, _)| (train_end, calibration_end))
        .ok_or_else(|| {
            anyhow::anyhow!("no family-aware overlay split satisfied support constraints")
        })
}

fn collect_family_overlay_scenario_ranges(
    rows: &[crate::ProbabilityTrainingRow],
    spec: &FamilyOverlayAuditSpec,
) -> Vec<crate::ScenarioRowRange> {
    let mut ranges = BTreeMap::<String, (usize, usize, String)>::new();
    for (index, row) in rows.iter().enumerate() {
        if !family_overlay_candidate_row(row, spec) {
            continue;
        }
        let Some(scenario_id) = row.primary_scenario_id.as_ref() else {
            continue;
        };
        let family = row
            .scenario_family
            .clone()
            .or_else(|| spec.scenario_family.map(str::to_string))
            .unwrap_or_else(|| "unknown".to_string());
        ranges
            .entry(scenario_id.clone())
            .and_modify(|range| range.1 = index)
            .or_insert((index, index, family));
    }

    let mut summaries = ranges
        .into_iter()
        .map(
            |(scenario_id, (start_index, end_index, family))| crate::ScenarioRowRange {
                scenario_id,
                family,
                start_index,
                end_index,
            },
        )
        .collect::<Vec<_>>();
    summaries.sort_by_key(|range| range.start_index);
    summaries
}

fn family_overlay_split_boundaries_within_range(range: &crate::ScenarioRowRange) -> Vec<usize> {
    ((range.start_index + 1)..=range.end_index.saturating_add(1)).collect()
}

fn family_overlay_candidate_row(
    row: &crate::ProbabilityTrainingRow,
    spec: &FamilyOverlayAuditSpec,
) -> bool {
    let gate_value =
        crate::resolve_probability_feature_value(spec.gate_feature, &row.features).unwrap_or(0.0);
    let gate_active = gate_value >= spec.gate_active_threshold;
    match spec.scenario_family {
        Some(family) => row.scenario_family.as_deref() == Some(family),
        None => gate_active || row.protected_action_window,
    }
}

fn family_overlay_gate_active_background_row(
    row: &crate::ProbabilityTrainingRow,
    spec: &FamilyOverlayAuditSpec,
) -> bool {
    let gate_value =
        crate::resolve_probability_feature_value(spec.gate_feature, &row.features).unwrap_or(0.0);
    gate_value >= spec.gate_active_threshold && !family_overlay_candidate_row(row, spec)
}

fn family_overlay_normal_background_row(
    row: &crate::ProbabilityTrainingRow,
    spec: &FamilyOverlayAuditSpec,
    horizon_days: u32,
    label_mode: crate::ProbabilityTargetLabelMode,
) -> bool {
    let gate_value =
        crate::resolve_probability_feature_value(spec.gate_feature, &row.features).unwrap_or(0.0);
    row.regime_for_horizon(horizon_days) == crate::ProbabilityTrainingRegime::Normal
        && row.label_for_horizon(label_mode, horizon_days) <= 0.0
        && gate_value <= spec.inactive_gate_ceiling
}

fn sample_probability_rows_evenly(
    rows: Vec<crate::ProbabilityTrainingRow>,
    cap: usize,
) -> Vec<crate::ProbabilityTrainingRow> {
    if rows.len() <= cap {
        return rows;
    }

    let mut sampled = Vec::with_capacity(cap);
    for index in 0..cap {
        let selected_index = index * rows.len() / cap;
        sampled.push(rows[selected_index].clone());
    }
    sampled
}

fn dedupe_probability_training_rows(
    mut rows: Vec<crate::ProbabilityTrainingRow>,
) -> Vec<crate::ProbabilityTrainingRow> {
    rows.sort_by(|left, right| {
        left.as_of_date
            .cmp(&right.as_of_date)
            .then_with(|| left.primary_scenario_id.cmp(&right.primary_scenario_id))
            .then_with(|| left.action_episode_id.cmp(&right.action_episode_id))
            .then_with(|| left.scenario_family.cmp(&right.scenario_family))
    });
    rows.dedup_by(|left, right| {
        left.as_of_date == right.as_of_date
            && left.primary_scenario_id == right.primary_scenario_id
            && left.action_episode_id == right.action_episode_id
            && left.scenario_family == right.scenario_family
    });
    rows
}

use std::collections::{BTreeMap, HashSet};

use fc_domain::{ProbabilityFamilyOverlayAudit, ProbabilityFamilyOverlayBundle};

#[derive(Debug, Clone, Copy)]
struct FamilyOverlayAuditSpec {
    family_id: &'static str,
    scenario_family: Option<&'static str>,
    gate_feature: &'static str,
    gate_active_threshold: f64,
    inactive_gate_ceiling: f64,
    min_scenario_count: u32,
    gate_slope: f64,
    blend_weight: f64,
    note: &'static str,
}

#[derive(Debug, Default)]
struct FamilyOverlayAuditMetrics {
    row_count: u32,
    gate_active_row_count: u32,
    positive_label_count: u32,
    early_warning_row_count: u32,
    protected_action_window_count: u32,
    gate_value_sum: f64,
    gate_value_count: u32,
    max_gate_value: f64,
    scenario_ids: HashSet<String>,
}

#[derive(Debug, Default)]
struct FamilyOverlaySplitSupport {
    positive_counts: Vec<usize>,
    early_warning_counts: Vec<usize>,
    gate_active_counts: Vec<usize>,
}

#[derive(Debug)]
struct FamilyOverlaySplitResult {
    train_rows: Vec<crate::ProbabilityTrainingRow>,
    calibration_rows: Vec<crate::ProbabilityTrainingRow>,
    evaluation_rows: Vec<crate::ProbabilityTrainingRow>,
    strategy: &'static str,
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

pub(super) fn build_family_overlay_audits(
    train_rows: &[crate::ProbabilityTrainingRow],
    calibration_rows: &[crate::ProbabilityTrainingRow],
    evaluation_rows: &[crate::ProbabilityTrainingRow],
    feature_names: &[String],
    horizon_days: u32,
    label_mode: crate::ProbabilityTargetLabelMode,
) -> Vec<ProbabilityFamilyOverlayAudit> {
    if !feature_names
        .iter()
        .any(|name| name.starts_with("family_proxy__"))
    {
        return Vec::new();
    }

    let early_warning_regime = super::probability_early_warning_regime(horizon_days);
    family_overlay_audit_specs()
        .iter()
        .map(|spec| {
            let train_metrics = collect_family_overlay_audit_metrics(
                train_rows,
                spec,
                horizon_days,
                label_mode,
                early_warning_regime,
            );
            let calibration_metrics = collect_family_overlay_audit_metrics(
                calibration_rows,
                spec,
                horizon_days,
                label_mode,
                early_warning_regime,
            );
            let evaluation_metrics = collect_family_overlay_audit_metrics(
                evaluation_rows,
                spec,
                horizon_days,
                label_mode,
                early_warning_regime,
            );

            let scenario_count = train_metrics
                .scenario_ids
                .iter()
                .chain(calibration_metrics.scenario_ids.iter())
                .chain(evaluation_metrics.scenario_ids.iter())
                .collect::<HashSet<_>>()
                .len() as u32;
            let gate_value_sum = train_metrics.gate_value_sum
                + calibration_metrics.gate_value_sum
                + evaluation_metrics.gate_value_sum;
            let gate_value_count = train_metrics.gate_value_count
                + calibration_metrics.gate_value_count
                + evaluation_metrics.gate_value_count;
            ProbabilityFamilyOverlayAudit {
                family_id: spec.family_id.to_string(),
                gate_feature: spec.gate_feature.to_string(),
                gate_active_threshold: spec.gate_active_threshold,
                scenario_count,
                train_row_count: train_metrics.row_count,
                calibration_row_count: calibration_metrics.row_count,
                evaluation_row_count: evaluation_metrics.row_count,
                train_gate_active_row_count: train_metrics.gate_active_row_count,
                calibration_gate_active_row_count: calibration_metrics.gate_active_row_count,
                evaluation_gate_active_row_count: evaluation_metrics.gate_active_row_count,
                positive_label_count: train_metrics.positive_label_count
                    + calibration_metrics.positive_label_count
                    + evaluation_metrics.positive_label_count,
                early_warning_row_count: train_metrics.early_warning_row_count
                    + calibration_metrics.early_warning_row_count
                    + evaluation_metrics.early_warning_row_count,
                protected_action_window_count: train_metrics.protected_action_window_count
                    + calibration_metrics.protected_action_window_count
                    + evaluation_metrics.protected_action_window_count,
                avg_gate_value: crate::round6(crate::safe_divide(
                    gate_value_sum,
                    gate_value_count as f64,
                )),
                max_gate_value: crate::round6(
                    train_metrics
                        .max_gate_value
                        .max(calibration_metrics.max_gate_value)
                        .max(evaluation_metrics.max_gate_value),
                ),
                note: spec.note.to_string(),
            }
        })
        .collect()
}

fn family_overlay_audit_specs() -> [FamilyOverlayAuditSpec; 5] {
    [
        FamilyOverlayAuditSpec {
            family_id: "systemic_credit",
            scenario_family: Some("systemic_credit_banking_crisis"),
            gate_feature: "family_proxy__systemic_credit",
            gate_active_threshold: 0.50,
            inactive_gate_ceiling: 0.20,
            min_scenario_count: 2,
            gate_slope: 8.0,
            blend_weight: 0.25,
            note: "candidate rows follow systemic_credit_banking_crisis scenario labels",
        },
        FamilyOverlayAuditSpec {
            family_id: "mixed_systemic",
            scenario_family: Some("mixed_systemic_stress"),
            gate_feature: "family_proxy__mixed_systemic",
            gate_active_threshold: 0.50,
            inactive_gate_ceiling: 0.20,
            min_scenario_count: 2,
            gate_slope: 8.0,
            blend_weight: 0.25,
            note: "candidate rows follow mixed_systemic_stress scenario labels",
        },
        FamilyOverlayAuditSpec {
            family_id: "rate_shock",
            scenario_family: Some("rate_shock_or_policy_dislocation"),
            gate_feature: "family_proxy__rate_shock",
            gate_active_threshold: 0.50,
            inactive_gate_ceiling: 0.20,
            min_scenario_count: 2,
            gate_slope: 8.0,
            blend_weight: 0.25,
            note: "candidate rows follow rate_shock_or_policy_dislocation scenario labels",
        },
        FamilyOverlayAuditSpec {
            family_id: "acute_liquidity",
            scenario_family: Some("acute_market_liquidity_crash"),
            gate_feature: "family_proxy__acute_liquidity",
            gate_active_threshold: 0.50,
            inactive_gate_ceiling: 0.20,
            min_scenario_count: 2,
            gate_slope: 8.0,
            blend_weight: 0.25,
            note: "candidate rows follow acute_market_liquidity_crash scenario labels",
        },
        FamilyOverlayAuditSpec {
            family_id: "jpy_carry",
            scenario_family: None,
            gate_feature: "family_proxy__jpy_carry",
            gate_active_threshold: 0.50,
            inactive_gate_ceiling: 0.20,
            min_scenario_count: 1,
            gate_slope: 8.0,
            blend_weight: 0.30,
            note: "proxy-only audit: candidate rows are gate-active rows rather than labeled crisis family rows",
        },
    ]
}

fn collect_family_overlay_audit_metrics(
    rows: &[crate::ProbabilityTrainingRow],
    spec: &FamilyOverlayAuditSpec,
    horizon_days: u32,
    label_mode: crate::ProbabilityTargetLabelMode,
    early_warning_regime: crate::ProbabilityTrainingRegime,
) -> FamilyOverlayAuditMetrics {
    let mut metrics = FamilyOverlayAuditMetrics::default();

    for row in rows {
        let gate_value = crate::resolve_probability_feature_value(spec.gate_feature, &row.features)
            .unwrap_or(0.0);
        let gate_active = gate_value >= spec.gate_active_threshold;
        if gate_active {
            metrics.gate_active_row_count += 1;
        }
        let candidate_row = match spec.scenario_family {
            Some(family) => row.scenario_family.as_deref() == Some(family),
            None => gate_active,
        };
        if !candidate_row {
            continue;
        }

        metrics.row_count += 1;
        metrics.gate_value_sum += gate_value;
        metrics.gate_value_count += 1;
        metrics.max_gate_value = metrics.max_gate_value.max(gate_value);
        if row.label_for_horizon(label_mode, horizon_days) > 0.0 {
            metrics.positive_label_count += 1;
        }
        if row.regime_for_horizon(horizon_days) == early_warning_regime {
            metrics.early_warning_row_count += 1;
        }
        if row.protected_action_window {
            metrics.protected_action_window_count += 1;
        }
        if let Some(scenario_id) = row.primary_scenario_id.as_ref() {
            metrics.scenario_ids.insert(scenario_id.clone());
        }
    }

    metrics
}

impl FamilyOverlaySplitSupport {
    fn from_rows(
        rows: &[crate::ProbabilityTrainingRow],
        spec: &FamilyOverlayAuditSpec,
        horizon_days: u32,
        label_mode: crate::ProbabilityTargetLabelMode,
    ) -> Self {
        let early_warning_regime = super::probability_early_warning_regime(horizon_days);
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

pub(super) fn train_family_overlays(
    train_rows: &[crate::ProbabilityTrainingRow],
    calibration_rows: &[crate::ProbabilityTrainingRow],
    evaluation_rows: &[crate::ProbabilityTrainingRow],
    feature_names: &[String],
    horizon_days: u32,
    label_mode: crate::ProbabilityTargetLabelMode,
    audits: &[ProbabilityFamilyOverlayAudit],
) -> Vec<ProbabilityFamilyOverlayBundle> {
    if !feature_names
        .iter()
        .any(|name| name.starts_with("family_proxy__"))
    {
        return Vec::new();
    }

    family_overlay_audit_specs()
        .iter()
        .filter_map(|spec| {
            let audit = audits.iter().find(|audit| audit.family_id == spec.family_id)?;
            if !family_overlay_has_minimum_support(audit, spec) {
                println!(
                    "  overlay_skip     {:>2}d {} insufficient_audit_support scenarios={} positives={} early_warning_rows={} gate_active_total={}",
                    horizon_days,
                    spec.family_id,
                    audit.scenario_count,
                    audit.positive_label_count,
                    audit.early_warning_row_count,
                    audit.train_gate_active_row_count
                        + audit.calibration_gate_active_row_count
                        + audit.evaluation_gate_active_row_count,
                );
                return None;
            }

            let overlay_dataset_rows = build_family_overlay_dataset_rows(
                train_rows,
                calibration_rows,
                evaluation_rows,
                spec,
                horizon_days,
                label_mode,
            );
            let split = match split_family_overlay_dataset_rows(
                &overlay_dataset_rows,
                spec,
                horizon_days,
                label_mode,
            ) {
                Ok(split) => split,
                Err(error) => {
                    println!(
                        "  overlay_skip     {:>2}d {} split_failed rows={} error={}",
                        horizon_days,
                        spec.family_id,
                        overlay_dataset_rows.len(),
                        error
                    );
                    return None;
                }
            };

            let head = match super::train_probability_head(
                &split.train_rows,
                &split.calibration_rows,
                &split.evaluation_rows,
                feature_names,
                horizon_days,
                label_mode,
            ) {
                Ok(head) => head,
                Err(error) => {
                    println!(
                        "  overlay_skip     {:>2}d {} train_failed strategy={} rows={}/{}/{} error={}",
                        horizon_days,
                        spec.family_id,
                        split.strategy,
                        split.train_rows.len(),
                        split.calibration_rows.len(),
                        split.evaluation_rows.len(),
                        error
                    );
                    return None;
                }
            };

            Some(ProbabilityFamilyOverlayBundle {
                family_id: spec.family_id.to_string(),
                gate_feature: spec.gate_feature.to_string(),
                gate_threshold: spec.gate_active_threshold,
                gate_slope: spec.gate_slope,
                blend_weight: spec.blend_weight,
                raw_model: head.raw_model,
                calibration: head.calibration,
                decision_threshold: Some(head.decision_threshold),
                evaluation: Some(head.evaluation),
                note: format!(
                    "overlay trained from {} / {} / {} selected split rows via {}; audit scenarios={}, positives={}, early_warning_rows={}; {}",
                    split.train_rows.len(),
                    split.calibration_rows.len(),
                    split.evaluation_rows.len(),
                    split.strategy,
                    audit.scenario_count,
                    audit.positive_label_count,
                    audit.early_warning_row_count,
                    spec.note
                ),
            })
        })
        .collect()
}

fn family_overlay_has_minimum_support(
    audit: &ProbabilityFamilyOverlayAudit,
    spec: &FamilyOverlayAuditSpec,
) -> bool {
    if spec.scenario_family.is_some() && audit.scenario_count < spec.min_scenario_count {
        return false;
    }
    if spec.scenario_family.is_none() && audit.protected_action_window_count == 0 {
        return false;
    }

    let total_candidate_rows =
        audit.train_row_count + audit.calibration_row_count + audit.evaluation_row_count;
    let total_gate_active_rows = audit.train_gate_active_row_count
        + audit.calibration_gate_active_row_count
        + audit.evaluation_gate_active_row_count;

    audit.positive_label_count > 0
        && audit.early_warning_row_count > 0
        && total_candidate_rows >= 10
        && total_gate_active_rows >= 4
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

fn build_family_overlay_dataset_rows(
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

fn split_family_overlay_dataset_rows(
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
            == super::probability_early_warning_regime(horizon_days),
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use chrono::NaiveDate;

    use super::{
        family_overlay_audit_specs, family_overlay_has_minimum_support,
        split_family_overlay_dataset_rows, FamilyOverlayAuditSpec,
    };

    fn overlay_row(
        day_index: i64,
        scenario_id: Option<&str>,
        scenario_family: Option<&str>,
        gate_feature: &str,
        gate_value: f64,
        label_20d: u8,
        regime_20d: crate::ProbabilityTrainingRegime,
    ) -> crate::ProbabilityTrainingRow {
        let mut features = BTreeMap::new();
        features.insert(gate_feature.to_string(), gate_value);
        crate::ProbabilityTrainingRow {
            as_of_date: NaiveDate::from_ymd_opt(2000, 1, 1)
                .unwrap()
                .checked_add_signed(chrono::Duration::days(day_index))
                .unwrap(),
            market_scope: "financial_system".to_string(),
            release_id: None,
            probability_mode: Some("formal_bundle_v1".to_string()),
            freshness_status: Some("a".to_string()),
            time_to_risk_bucket: None,
            split_name: None,
            features,
            primary_scenario_id: scenario_id.map(str::to_string),
            scenario_family: scenario_family.map(str::to_string),
            scenario_training_role: scenario_family.map(|_| "mandatory".to_string()),
            days_to_primary_crisis_start: None,
            primary_scenario_supports_5d: true,
            primary_scenario_supports_20d: true,
            primary_scenario_supports_60d: true,
            label_5d: 0,
            label_20d,
            label_60d: 0,
            regime_5d: crate::ProbabilityTrainingRegime::Normal,
            regime_20d,
            regime_60d: crate::ProbabilityTrainingRegime::Normal,
            action_label_5d: 0,
            action_label_20d: 0,
            action_label_60d: 0,
            prepare_episode_label: 0,
            hedge_episode_label: 0,
            defend_episode_label: 0,
            primary_action_level: None,
            action_episode_id: None,
            action_episode_phase: "outside".to_string(),
            protected_action_window: false,
        }
    }

    fn systemic_credit_spec() -> FamilyOverlayAuditSpec {
        family_overlay_audit_specs()
            .into_iter()
            .find(|spec| spec.family_id == "systemic_credit")
            .expect("systemic credit spec exists")
    }

    #[test]
    fn family_overlay_minimum_support_uses_aggregate_support_not_original_split_shape() {
        let spec = systemic_credit_spec();
        let audit = fc_domain::ProbabilityFamilyOverlayAudit {
            family_id: "systemic_credit".to_string(),
            gate_feature: spec.gate_feature.to_string(),
            gate_active_threshold: spec.gate_active_threshold,
            scenario_count: 2,
            train_row_count: 621,
            calibration_row_count: 1,
            evaluation_row_count: 118,
            train_gate_active_row_count: 239,
            calibration_gate_active_row_count: 0,
            evaluation_gate_active_row_count: 484,
            positive_label_count: 40,
            early_warning_row_count: 30,
            protected_action_window_count: 0,
            avg_gate_value: 0.11,
            max_gate_value: 0.64,
            note: "test".to_string(),
        };
        assert!(family_overlay_has_minimum_support(&audit, &spec));

        let zero_gate_audit = fc_domain::ProbabilityFamilyOverlayAudit {
            train_gate_active_row_count: 0,
            calibration_gate_active_row_count: 0,
            evaluation_gate_active_row_count: 0,
            ..audit
        };
        assert!(!family_overlay_has_minimum_support(&zero_gate_audit, &spec));
    }

    #[test]
    fn family_overlay_split_recovers_positive_and_early_warning_support_across_scenarios() {
        let spec = systemic_credit_spec();
        let rows = (0..150)
            .map(|index| match index {
                30..=41 => overlay_row(
                    index,
                    Some("scenario_a"),
                    Some("systemic_credit_banking_crisis"),
                    spec.gate_feature,
                    0.92,
                    0,
                    crate::ProbabilityTrainingRegime::Normal,
                ),
                42..=49 => overlay_row(
                    index,
                    Some("scenario_a"),
                    Some("systemic_credit_banking_crisis"),
                    spec.gate_feature,
                    0.92,
                    0,
                    crate::ProbabilityTrainingRegime::PreWarningBuffer,
                ),
                50..=59 => overlay_row(
                    index,
                    Some("scenario_a"),
                    Some("systemic_credit_banking_crisis"),
                    spec.gate_feature,
                    0.92,
                    1,
                    crate::ProbabilityTrainingRegime::PositiveWindow,
                ),
                70..=75 => overlay_row(
                    index,
                    None,
                    None,
                    spec.gate_feature,
                    0.75,
                    0,
                    crate::ProbabilityTrainingRegime::Normal,
                ),
                90..=101 => overlay_row(
                    index,
                    Some("scenario_b"),
                    Some("systemic_credit_banking_crisis"),
                    spec.gate_feature,
                    0.95,
                    0,
                    crate::ProbabilityTrainingRegime::Normal,
                ),
                102..=109 => overlay_row(
                    index,
                    Some("scenario_b"),
                    Some("systemic_credit_banking_crisis"),
                    spec.gate_feature,
                    0.95,
                    0,
                    crate::ProbabilityTrainingRegime::PreWarningBuffer,
                ),
                110..=119 => overlay_row(
                    index,
                    Some("scenario_b"),
                    Some("systemic_credit_banking_crisis"),
                    spec.gate_feature,
                    0.95,
                    1,
                    crate::ProbabilityTrainingRegime::PositiveWindow,
                ),
                125..=130 => overlay_row(
                    index,
                    None,
                    None,
                    spec.gate_feature,
                    0.72,
                    0,
                    crate::ProbabilityTrainingRegime::Normal,
                ),
                _ => overlay_row(
                    index,
                    None,
                    None,
                    spec.gate_feature,
                    0.02,
                    0,
                    crate::ProbabilityTrainingRegime::Normal,
                ),
            })
            .collect::<Vec<_>>();

        let split = split_family_overlay_dataset_rows(
            &rows,
            &spec,
            20,
            crate::ProbabilityTargetLabelMode::ForwardCrisis,
        )
        .expect("family-aware split should succeed");

        assert_eq!(split.strategy, "family_aware");
        assert!(split.train_rows.iter().any(|row| row
            .label_for_horizon(crate::ProbabilityTargetLabelMode::ForwardCrisis, 20)
            > 0.0));
        assert!(split.calibration_rows.iter().any(|row| row
            .label_for_horizon(crate::ProbabilityTargetLabelMode::ForwardCrisis, 20)
            > 0.0));
        assert!(split.evaluation_rows.iter().any(|row| row
            .label_for_horizon(crate::ProbabilityTargetLabelMode::ForwardCrisis, 20)
            > 0.0));
        assert!(split
            .calibration_rows
            .iter()
            .any(|row| row.regime_for_horizon(20)
                == crate::ProbabilityTrainingRegime::PreWarningBuffer));
        assert!(split
            .calibration_rows
            .iter()
            .chain(split.evaluation_rows.iter())
            .any(|row| row.regime_for_horizon(20)
                == crate::ProbabilityTrainingRegime::PreWarningBuffer));
    }

    #[test]
    fn family_overlay_split_balanced_fallback_recovers_sparse_topology() {
        let spec = systemic_credit_spec();
        let rows = (0..140)
            .map(|index| match index {
                28..=39 => overlay_row(
                    index,
                    Some("scenario_a"),
                    Some("systemic_credit_banking_crisis"),
                    spec.gate_feature,
                    0.91,
                    0,
                    crate::ProbabilityTrainingRegime::Normal,
                ),
                40..=47 => overlay_row(
                    index,
                    Some("scenario_a"),
                    Some("systemic_credit_banking_crisis"),
                    spec.gate_feature,
                    0.91,
                    0,
                    crate::ProbabilityTrainingRegime::PreWarningBuffer,
                ),
                48..=57 => overlay_row(
                    index,
                    Some("scenario_a"),
                    Some("systemic_credit_banking_crisis"),
                    spec.gate_feature,
                    0.91,
                    1,
                    crate::ProbabilityTrainingRegime::PositiveWindow,
                ),
                70..=81 => overlay_row(
                    index,
                    Some("scenario_b"),
                    Some("systemic_credit_banking_crisis"),
                    spec.gate_feature,
                    0.88,
                    0,
                    crate::ProbabilityTrainingRegime::Normal,
                ),
                82..=87 => overlay_row(
                    index,
                    Some("scenario_b"),
                    Some("systemic_credit_banking_crisis"),
                    spec.gate_feature,
                    0.88,
                    0,
                    crate::ProbabilityTrainingRegime::PreWarningBuffer,
                ),
                94..=100 => overlay_row(
                    index,
                    None,
                    None,
                    spec.gate_feature,
                    0.72,
                    0,
                    crate::ProbabilityTrainingRegime::Normal,
                ),
                112..=118 => overlay_row(
                    index,
                    None,
                    None,
                    spec.gate_feature,
                    0.68,
                    0,
                    crate::ProbabilityTrainingRegime::Normal,
                ),
                _ => overlay_row(
                    index,
                    None,
                    None,
                    spec.gate_feature,
                    0.02,
                    0,
                    crate::ProbabilityTrainingRegime::Normal,
                ),
            })
            .collect::<Vec<_>>();

        let split = split_family_overlay_dataset_rows(
            &rows,
            &spec,
            20,
            crate::ProbabilityTargetLabelMode::ForwardCrisis,
        )
        .expect("balanced fallback should succeed");

        assert_eq!(split.strategy, "balanced");
        assert!(split.train_rows.iter().any(|row| row
            .label_for_horizon(crate::ProbabilityTargetLabelMode::ForwardCrisis, 20)
            > 0.0));
        assert!(split.calibration_rows.iter().any(|row| row
            .label_for_horizon(crate::ProbabilityTargetLabelMode::ForwardCrisis, 20)
            > 0.0));
        assert!(split.evaluation_rows.iter().any(|row| row
            .label_for_horizon(crate::ProbabilityTargetLabelMode::ForwardCrisis, 20)
            > 0.0));
    }
}

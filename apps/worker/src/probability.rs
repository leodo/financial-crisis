use std::collections::{BTreeMap, HashSet};

use fc_domain::{
    apply_platt_probability_calibration, HorizonEvaluationSummary, LogisticProbabilityModel,
    PlattCalibrationArtifact, ProbabilityBundleEvaluation, ProbabilityCalibrationRegimeEvidence,
    ProbabilityFamilyOverlayAudit, ProbabilityFamilyOverlayBundle, ProbabilityHorizonBundle,
    ProbabilityThresholdDecisionSummary as ProbabilityThresholdDecisionSummaryWire,
    ProbabilityThresholdDiagnostics as ProbabilityThresholdDiagnosticsWire,
    RegimeSeparationEvaluationSummary,
};

#[derive(Debug, Clone)]
pub(crate) struct ProbabilityCalibrationSelection<'a> {
    pub(crate) rows: Vec<&'a crate::ProbabilityTrainingRow>,
    pub(crate) eligible_row_count: usize,
    pub(crate) eligible_positive_count: usize,
    pub(crate) eligible_negative_count: usize,
    pub(crate) used_full_split_fallback: bool,
}

#[derive(Debug, Clone, Copy, Default)]
struct ProbabilityThresholdDecisionMetrics {
    regime_hits: ProbabilityThresholdRegimeHitSummary,
    predicted_positive_count: u32,
    true_positive_count: u32,
    precision: f64,
    recall: f64,
}

#[derive(Debug, Clone)]
pub(crate) struct ProbabilityThresholdSelection<'a> {
    pub(crate) rows: Vec<&'a crate::ProbabilityTrainingRow>,
    pub(crate) probabilities: Vec<f64>,
    pub(crate) labels: Vec<f64>,
    pub(crate) used_full_split_fallback: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ProbabilityThresholdDiagnosticsInput<'a> {
    pub(crate) full_calibration_rows: &'a [crate::ProbabilityTrainingRow],
    pub(crate) calibration_selection: &'a ProbabilityCalibrationSelection<'a>,
    pub(crate) threshold_selection: &'a ProbabilityThresholdSelection<'a>,
    pub(crate) horizon_days: u32,
    pub(crate) label_mode: crate::ProbabilityTargetLabelMode,
    pub(crate) base_threshold: f64,
    pub(crate) final_threshold: f64,
}

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
    let family_overlay_audits = build_family_overlay_audits(
        train_rows,
        calibration_rows,
        evaluation_rows,
        overlay_feature_names,
        horizon_days,
        label_mode,
    );
    let family_overlays = train_family_overlays(
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
    let (calibration, evaluation_probabilities) = select_probability_calibration_strategy(
        &calibration_inputs,
        &calibration_labels,
        &calibration_selection.rows,
        horizon_days,
        label_mode,
        &evaluation_raw_probabilities,
        calibration_candidate,
    );
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

fn build_family_overlay_audits(
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

    let early_warning_regime = probability_early_warning_regime(horizon_days);
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
        let early_warning_regime = probability_early_warning_regime(horizon_days);
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

fn train_family_overlays(
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

            let head = match train_probability_head(
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
            == probability_early_warning_regime(horizon_days),
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

pub(crate) fn probability_calibration_selection_rows(
    rows: &[crate::ProbabilityTrainingRow],
    horizon_days: u32,
    label_mode: crate::ProbabilityTargetLabelMode,
) -> ProbabilityCalibrationSelection<'_> {
    let filtered = rows
        .iter()
        .filter(|row| probability_row_is_calibration_eligible(row, horizon_days, label_mode))
        .collect::<Vec<_>>();

    let filtered_positive_count = filtered
        .iter()
        .filter(|row| row.label_for_horizon(label_mode, horizon_days) > 0.0)
        .count();
    let filtered_negative_count = filtered.len().saturating_sub(filtered_positive_count);

    if filtered_positive_count > 0 && filtered_negative_count > 0 {
        ProbabilityCalibrationSelection {
            rows: filtered,
            eligible_row_count: filtered_positive_count + filtered_negative_count,
            eligible_positive_count: filtered_positive_count,
            eligible_negative_count: filtered_negative_count,
            used_full_split_fallback: false,
        }
    } else {
        ProbabilityCalibrationSelection {
            rows: rows.iter().collect(),
            eligible_row_count: filtered_positive_count + filtered_negative_count,
            eligible_positive_count: filtered_positive_count,
            eligible_negative_count: filtered_negative_count,
            used_full_split_fallback: true,
        }
    }
}

fn probability_row_is_calibration_eligible(
    row: &crate::ProbabilityTrainingRow,
    horizon_days: u32,
    label_mode: crate::ProbabilityTargetLabelMode,
) -> bool {
    if row.label_for_horizon(label_mode, horizon_days) > 0.0 {
        return true;
    }

    match label_mode {
        crate::ProbabilityTargetLabelMode::ActionWindow
        | crate::ProbabilityTargetLabelMode::ActionEpisode => true,
        crate::ProbabilityTargetLabelMode::ForwardCrisis => match horizon_days {
            20 | 60 => matches!(
                row.regime_for_horizon(horizon_days),
                crate::ProbabilityTrainingRegime::Normal
                    | crate::ProbabilityTrainingRegime::PreWarningBuffer
                    | crate::ProbabilityTrainingRegime::InCrisis
                    | crate::ProbabilityTrainingRegime::PostCrisisCooldown
            ),
            _ => matches!(
                row.regime_for_horizon(horizon_days),
                crate::ProbabilityTrainingRegime::Normal
            ),
        },
    }
}

pub(crate) fn probability_decision_threshold_selection<'a>(
    probabilities: &[f64],
    labels: &[f64],
    rows: &[&'a crate::ProbabilityTrainingRow],
    horizon_days: u32,
    label_mode: crate::ProbabilityTargetLabelMode,
) -> ProbabilityThresholdSelection<'a> {
    let mut filtered_rows = Vec::new();
    let mut filtered_probabilities = Vec::new();
    let mut filtered_labels = Vec::new();
    let mut filtered_positive_count = 0_usize;
    let mut filtered_negative_count = 0_usize;

    for ((probability, label), row) in probabilities.iter().zip(labels).zip(rows.iter().copied()) {
        if !probability_row_is_threshold_eligible(row, horizon_days, label_mode) {
            continue;
        }
        filtered_rows.push(row);
        filtered_probabilities.push(*probability);
        filtered_labels.push(*label);
        if *label >= 0.5 {
            filtered_positive_count += 1;
        } else {
            filtered_negative_count += 1;
        }
    }

    if filtered_positive_count > 0 && filtered_negative_count > 0 {
        ProbabilityThresholdSelection {
            rows: filtered_rows,
            probabilities: filtered_probabilities,
            labels: filtered_labels,
            used_full_split_fallback: false,
        }
    } else {
        ProbabilityThresholdSelection {
            rows: rows.to_vec(),
            probabilities: probabilities.to_vec(),
            labels: labels.to_vec(),
            used_full_split_fallback: true,
        }
    }
}

fn probability_row_is_threshold_eligible(
    row: &crate::ProbabilityTrainingRow,
    horizon_days: u32,
    label_mode: crate::ProbabilityTargetLabelMode,
) -> bool {
    if row.label_for_horizon(label_mode, horizon_days) > 0.0 {
        return true;
    }

    match label_mode {
        crate::ProbabilityTargetLabelMode::ActionWindow
        | crate::ProbabilityTargetLabelMode::ActionEpisode => true,
        crate::ProbabilityTargetLabelMode::ForwardCrisis => match horizon_days {
            20 | 60 => matches!(
                row.regime_for_horizon(horizon_days),
                crate::ProbabilityTrainingRegime::Normal
                    | crate::ProbabilityTrainingRegime::PreWarningBuffer
                    | crate::ProbabilityTrainingRegime::PostCrisisCooldown
            ),
            _ => matches!(
                row.regime_for_horizon(horizon_days),
                crate::ProbabilityTrainingRegime::Normal
            ),
        },
    }
}

pub(crate) fn select_probability_calibration_strategy(
    calibration_raw_probabilities: &[f64],
    calibration_labels: &[f64],
    calibration_rows: &[&crate::ProbabilityTrainingRow],
    horizon_days: u32,
    label_mode: crate::ProbabilityTargetLabelMode,
    evaluation_raw_probabilities: &[f64],
    calibration_candidate: PlattCalibrationArtifact,
) -> (Option<PlattCalibrationArtifact>, Vec<f64>) {
    let raw_summary =
        crate::evaluate_probabilities(calibration_raw_probabilities, calibration_labels);
    let raw_regime_separation = evaluate_regime_separation_summary_refs(
        calibration_raw_probabilities,
        calibration_rows,
        horizon_days,
        label_mode,
    );
    let raw_score =
        probability_calibration_selection_score(&raw_summary, raw_regime_separation.as_ref());

    let calibration_probabilities = calibration_raw_probabilities
        .iter()
        .map(|raw_probability| {
            apply_platt_probability_calibration(*raw_probability, &calibration_candidate)
        })
        .collect::<Vec<_>>();
    let calibrated_summary =
        crate::evaluate_probabilities(&calibration_probabilities, calibration_labels);
    let calibrated_regime_separation = evaluate_regime_separation_summary_refs(
        &calibration_probabilities,
        calibration_rows,
        horizon_days,
        label_mode,
    );
    let calibrated_score = probability_calibration_selection_score(
        &calibrated_summary,
        calibrated_regime_separation.as_ref(),
    );

    let raw_ranking_reversed =
        probability_raw_ranking_is_reversed(calibration_raw_probabilities, calibration_labels);
    let keep_calibration = calibrated_score > raw_score
        && (calibration_candidate.alpha > 0.0
            || (calibration_candidate.alpha < 0.0 && raw_ranking_reversed));
    if keep_calibration {
        let evaluation_probabilities = evaluation_raw_probabilities
            .iter()
            .map(|raw_probability| {
                apply_platt_probability_calibration(*raw_probability, &calibration_candidate)
            })
            .collect::<Vec<_>>();
        (Some(calibration_candidate), evaluation_probabilities)
    } else {
        (None, evaluation_raw_probabilities.to_vec())
    }
}

fn probability_calibration_selection_score(
    summary: &HorizonEvaluationSummary,
    regime_separation: Option<&RegimeSeparationEvaluationSummary>,
) -> (i64, i64, i64, i64, i64, i64, i64, i64, i64) {
    (
        probability_regime_diagnosis_score(regime_separation),
        probability_regime_positive_window_lift_score(regime_separation),
        probability_regime_positive_window_gap_score(regime_separation),
        probability_regime_positive_window_minus_cooldown_score(regime_separation),
        probability_regime_early_warning_lift_score(regime_separation),
        probability_regime_max_non_normal_lift_score(regime_separation),
        -((summary.log_loss * 1_000_000.0).round() as i64),
        -((summary.brier_score * 1_000_000.0).round() as i64),
        -((summary.ece * 1_000_000.0).round() as i64),
    )
}

fn probability_regime_diagnosis_score(
    regime_separation: Option<&RegimeSeparationEvaluationSummary>,
) -> i64 {
    match regime_separation.map(|summary| summary.diagnosis.as_str()) {
        Some("usable_early_warning_separation") => 6,
        Some("weak_regime_separation") => 5,
        Some("mixed_or_unclear") => 4,
        Some("late_only_no_early_warning") => 3,
        Some("cooldown_bleed") => 2,
        Some("cold_across_all_regimes") => 1,
        Some(_) => 0,
        None => 2,
    }
}

fn probability_regime_positive_window_lift_score(
    regime_separation: Option<&RegimeSeparationEvaluationSummary>,
) -> i64 {
    (regime_separation
        .and_then(|summary| summary.positive_window_lift_vs_normal)
        .unwrap_or_default()
        * 1_000.0)
        .round() as i64
}

fn probability_regime_positive_window_gap_score(
    regime_separation: Option<&RegimeSeparationEvaluationSummary>,
) -> i64 {
    (regime_separation
        .and_then(|summary| summary.positive_window_gap_vs_normal)
        .unwrap_or_default()
        * 1_000_000.0)
        .round() as i64
}

fn probability_regime_positive_window_minus_cooldown_score(
    regime_separation: Option<&RegimeSeparationEvaluationSummary>,
) -> i64 {
    let Some(summary) = regime_separation else {
        return 0;
    };
    let positive_window = summary.positive_window_lift_vs_normal.unwrap_or_default();
    let cooldown = summary
        .post_crisis_cooldown_lift_vs_normal
        .unwrap_or_default();
    ((positive_window - cooldown) * 1_000.0).round() as i64
}

fn probability_regime_early_warning_lift_score(
    regime_separation: Option<&RegimeSeparationEvaluationSummary>,
) -> i64 {
    (regime_separation
        .and_then(|summary| summary.early_warning_lift_vs_normal)
        .unwrap_or_default()
        * 1_000.0)
        .round() as i64
}

fn probability_regime_max_non_normal_lift_score(
    regime_separation: Option<&RegimeSeparationEvaluationSummary>,
) -> i64 {
    (regime_separation
        .and_then(|summary| summary.max_non_normal_lift_vs_normal)
        .unwrap_or_default()
        * 1_000.0)
        .round() as i64
}

fn probability_raw_ranking_is_reversed(probabilities: &[f64], labels: &[f64]) -> bool {
    let mut positive_sum = 0.0;
    let mut positive_count = 0_u32;
    let mut negative_sum = 0.0;
    let mut negative_count = 0_u32;

    for (probability, label) in probabilities.iter().zip(labels) {
        if *label >= 0.5 {
            positive_sum += *probability;
            positive_count += 1;
        } else {
            negative_sum += *probability;
            negative_count += 1;
        }
    }

    if positive_count == 0 || negative_count == 0 {
        return false;
    }

    let positive_mean = positive_sum / positive_count as f64;
    let negative_mean = negative_sum / negative_count as f64;
    positive_mean < negative_mean
}

pub(crate) fn select_probability_decision_threshold(
    probabilities: &[f64],
    labels: &[f64],
    horizon_days: u32,
) -> f64 {
    let thresholds = probability_decision_threshold_candidates(probabilities);

    let actual_positive_count = labels.iter().filter(|label| **label >= 0.5).count() as u32;
    let positive_count = actual_positive_count as f64;
    let prediction_ceiling = probability_prediction_count_ceiling_from_actual_positive_count(
        actual_positive_count,
        horizon_days,
    );
    let mut best_threshold = 0.3;
    let beta_sq = probability_threshold_beta_sq(horizon_days);
    let mut best_score = None::<(i64, i64, i64, i64, i64)>;
    let mut best_capped_threshold = None::<f64>;
    let mut best_capped_score = None::<(i64, i64, i64, i64, i64)>;
    for threshold in thresholds {
        let mut true_positive_count = 0_u32;
        let mut predicted_positive_count = 0_u32;
        for (probability, label) in probabilities.iter().zip(labels) {
            if *probability >= threshold {
                predicted_positive_count += 1;
                if *label >= 0.5 {
                    true_positive_count += 1;
                }
            }
        }
        if predicted_positive_count == 0 || positive_count <= 0.0 {
            continue;
        }
        let minimum_true_positives = (positive_count.min(2.0)) as u32;
        if true_positive_count < minimum_true_positives.max(1) {
            continue;
        }
        let precision = true_positive_count as f64 / predicted_positive_count as f64;
        let recall = true_positive_count as f64 / positive_count;
        let f_beta = if precision > 0.0 || recall > 0.0 {
            (1.0 + beta_sq) * precision * recall / (beta_sq * precision + recall).max(1e-9)
        } else {
            0.0
        };
        let score = probability_threshold_score_tuple(ProbabilityThresholdScoreInputs {
            horizon_days,
            precision,
            recall,
            f_beta,
            threshold,
            predicted_positive_count,
            prediction_ceiling,
            actual_positive_count,
        });
        if best_score.is_none_or(|best| score > best) {
            best_score = Some(score);
            best_threshold = threshold;
        }
        if predicted_positive_count <= prediction_ceiling
            && best_capped_score.is_none_or(|best| score > best)
        {
            best_capped_score = Some(score);
            best_capped_threshold = Some(threshold);
        }
    }

    let minimum_threshold = match horizon_days {
        5 => 0.03,
        20 => 0.005,
        60 => 0.01,
        _ => 0.001,
    };

    crate::round3(best_capped_threshold.unwrap_or(best_threshold)).clamp(minimum_threshold, 0.90)
}

fn probability_decision_threshold_candidates(probabilities: &[f64]) -> Vec<f64> {
    let mut thresholds = probabilities
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .filter(|value| (0.001..0.99).contains(value))
        .collect::<Vec<_>>();
    thresholds.extend((1..=20).map(|value| value as f64 / 1_000.0));
    thresholds.extend((2..=90).map(|value| value as f64 / 100.0));
    thresholds.push(0.3);
    thresholds.sort_by(f64::total_cmp);
    thresholds.dedup_by(|left, right| (*left - *right).abs() < 1e-6);
    thresholds
}

fn probability_threshold_beta_sq(horizon_days: u32) -> f64 {
    match horizon_days {
        5 => 0.25,
        20 => 1.0,
        60 => 2.25,
        _ => 1.0,
    }
}

#[derive(Debug, Clone, Copy)]
struct ProbabilityThresholdScoreInputs {
    horizon_days: u32,
    precision: f64,
    recall: f64,
    f_beta: f64,
    threshold: f64,
    predicted_positive_count: u32,
    prediction_ceiling: u32,
    actual_positive_count: u32,
}

fn probability_threshold_score_tuple(
    inputs: ProbabilityThresholdScoreInputs,
) -> (i64, i64, i64, i64, i64) {
    let ProbabilityThresholdScoreInputs {
        horizon_days,
        precision,
        recall,
        f_beta,
        threshold,
        predicted_positive_count,
        prediction_ceiling,
        actual_positive_count,
    } = inputs;
    let precision_score = (precision * 1_000_000.0).round() as i64;
    let recall_score = (recall * 1_000_000.0).round() as i64;
    let f_beta_score = (f_beta * 1_000_000.0).round() as i64;
    let threshold_score = (threshold * 1_000.0).round() as i64;
    let overprediction_score = probability_threshold_overprediction_score(
        horizon_days,
        predicted_positive_count,
        prediction_ceiling,
        actual_positive_count,
    );
    let adjusted_f_beta_score = if horizon_days == 20 {
        f_beta_score + overprediction_score
    } else {
        f_beta_score
    };

    match horizon_days {
        5 => (
            precision_score,
            f_beta_score,
            recall_score,
            overprediction_score,
            threshold_score,
        ),
        20 => (
            adjusted_f_beta_score,
            precision_score,
            recall_score,
            threshold_score,
            overprediction_score,
        ),
        60 => (
            f_beta_score,
            recall_score,
            precision_score,
            overprediction_score,
            threshold_score,
        ),
        _ => (
            f_beta_score,
            precision_score,
            recall_score,
            overprediction_score,
            threshold_score,
        ),
    }
}

fn probability_threshold_overprediction_score(
    horizon_days: u32,
    predicted_positive_count: u32,
    prediction_ceiling: u32,
    actual_positive_count: u32,
) -> i64 {
    if horizon_days != 20 || actual_positive_count == 0 {
        return 0;
    }

    let overflow = predicted_positive_count.saturating_sub(prediction_ceiling) as f64;
    -((overflow / actual_positive_count as f64) * 1_000.0).round() as i64
}

#[derive(Debug, Clone, Copy, Default)]
struct ProbabilityThresholdRegimeHitSummary {
    early_warning_row_count: u32,
    early_warning_hit_count: u32,
    normal_row_count: u32,
    normal_hit_count: u32,
    positive_window_row_count: u32,
    positive_window_hit_count: u32,
    in_crisis_row_count: u32,
    in_crisis_hit_count: u32,
    cooldown_row_count: u32,
    cooldown_hit_count: u32,
}

impl ProbabilityThresholdRegimeHitSummary {
    fn early_warning_hit_rate(self) -> f64 {
        crate::safe_divide(
            self.early_warning_hit_count as f64,
            self.early_warning_row_count as f64,
        )
    }

    fn normal_hit_rate(self) -> f64 {
        crate::safe_divide(self.normal_hit_count as f64, self.normal_row_count as f64)
    }

    fn positive_window_hit_rate(self) -> f64 {
        crate::safe_divide(
            self.positive_window_hit_count as f64,
            self.positive_window_row_count as f64,
        )
    }

    fn in_crisis_hit_rate(self) -> f64 {
        crate::safe_divide(
            self.in_crisis_hit_count as f64,
            self.in_crisis_row_count as f64,
        )
    }

    fn cooldown_hit_rate(self) -> f64 {
        crate::safe_divide(
            self.cooldown_hit_count as f64,
            self.cooldown_row_count as f64,
        )
    }
}

fn probability_early_warning_regime(horizon_days: u32) -> crate::ProbabilityTrainingRegime {
    match horizon_days {
        5 => crate::ProbabilityTrainingRegime::PositiveWindow,
        20 | 60 => crate::ProbabilityTrainingRegime::PreWarningBuffer,
        _ => crate::ProbabilityTrainingRegime::PositiveWindow,
    }
}

fn probability_threshold_regime_hit_summary(
    probabilities: &[f64],
    rows: &[&crate::ProbabilityTrainingRow],
    horizon_days: u32,
    threshold: f64,
) -> ProbabilityThresholdRegimeHitSummary {
    let early_warning_regime = probability_early_warning_regime(horizon_days);

    let mut summary = ProbabilityThresholdRegimeHitSummary::default();
    for (probability, row) in probabilities.iter().zip(rows.iter().copied()) {
        let regime = row.regime_for_horizon(horizon_days);
        let hit = *probability >= threshold;

        if regime == early_warning_regime {
            summary.early_warning_row_count += 1;
            if hit {
                summary.early_warning_hit_count += 1;
            }
        }

        match regime {
            crate::ProbabilityTrainingRegime::Normal => {
                summary.normal_row_count += 1;
                if hit {
                    summary.normal_hit_count += 1;
                }
            }
            crate::ProbabilityTrainingRegime::PositiveWindow => {
                summary.positive_window_row_count += 1;
                if hit {
                    summary.positive_window_hit_count += 1;
                }
            }
            crate::ProbabilityTrainingRegime::InCrisis => {
                summary.in_crisis_row_count += 1;
                if hit {
                    summary.in_crisis_hit_count += 1;
                }
            }
            crate::ProbabilityTrainingRegime::PostCrisisCooldown => {
                summary.cooldown_row_count += 1;
                if hit {
                    summary.cooldown_hit_count += 1;
                }
            }
            crate::ProbabilityTrainingRegime::PreWarningBuffer => {}
        }
    }

    summary
}

fn regime_aware_threshold_prediction_ceiling(actual_positive_count: u32, horizon_days: u32) -> u32 {
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
        _ => 0.0,
    }
}

fn regime_floor_min_gap_vs_normal(horizon_days: u32) -> f64 {
    match horizon_days {
        60 => 0.02,
        20 => 0.01,
        _ => 0.0,
    }
}

fn threshold_has_usable_early_warning_support(
    hits: ProbabilityThresholdRegimeHitSummary,
    horizon_days: u32,
) -> bool {
    hits.early_warning_hit_count > 0
        && hits.early_warning_hit_rate() >= regime_floor_min_hit_rate(horizon_days)
        && (hits.early_warning_hit_rate() - hits.normal_hit_rate())
            >= regime_floor_min_gap_vs_normal(horizon_days)
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
        || !matches!(horizon_days, 20 | 60)
        || probabilities.is_empty()
        || rows.is_empty()
        || probabilities.len() != rows.len()
    {
        return base_threshold;
    }

    let Some(regime_summary) =
        evaluate_regime_separation_summary_refs(probabilities, rows, horizon_days, label_mode)
    else {
        return base_threshold;
    };

    let base_hits =
        probability_threshold_regime_hit_summary(probabilities, rows, horizon_days, base_threshold);
    if threshold_has_usable_early_warning_support(base_hits, horizon_days) {
        return base_threshold;
    }
    if regime_summary
        .early_warning_lift_vs_normal
        .unwrap_or_default()
        < 1.5
    {
        return base_threshold;
    }

    let actual_positive_count = labels.iter().filter(|label| **label >= 0.5).count() as u32;
    let positive_count = actual_positive_count as f64;
    if positive_count <= 0.0 {
        return base_threshold;
    }

    let early_warning_regime = probability_early_warning_regime(horizon_days);
    let early_warning_probability_cap = probabilities
        .iter()
        .zip(rows.iter().copied())
        .filter(|(_, row)| row.regime_for_horizon(horizon_days) == early_warning_regime)
        .map(|(probability, _)| *probability)
        .fold(0.0_f64, f64::max);

    let relaxed_prediction_ceiling =
        regime_aware_threshold_prediction_ceiling(actual_positive_count, horizon_days);
    let beta_sq = probability_threshold_beta_sq(horizon_days);
    let mut best_score = None::<(bool, bool, i64, i64, i64, i64, i64, i64, i64)>;
    let mut best_threshold = base_threshold;

    for threshold in probability_decision_threshold_candidates(probabilities) {
        if threshold >= base_threshold {
            continue;
        }

        let hits =
            probability_threshold_regime_hit_summary(probabilities, rows, horizon_days, threshold);
        let early_warning_hit_rate = hits.early_warning_hit_rate();
        if hits.early_warning_hit_count == 0 {
            continue;
        }

        let mut true_positive_count = 0_u32;
        let mut predicted_positive_count = 0_u32;
        for (probability, label) in probabilities.iter().zip(labels) {
            if *probability >= threshold {
                predicted_positive_count += 1;
                if *label >= 0.5 {
                    true_positive_count += 1;
                }
            }
        }
        if predicted_positive_count == 0 || true_positive_count == 0 {
            continue;
        }

        let precision = true_positive_count as f64 / predicted_positive_count as f64;
        let recall = true_positive_count as f64 / positive_count;
        let f_beta = if precision > 0.0 || recall > 0.0 {
            (1.0 + beta_sq) * precision * recall / (beta_sq * precision + recall).max(1e-9)
        } else {
            0.0
        };

        let normal_hit_rate = hits.normal_hit_rate();
        let cooldown_hit_rate = hits.cooldown_hit_rate();
        let score = (
            early_warning_hit_rate >= regime_floor_min_hit_rate(horizon_days),
            predicted_positive_count <= relaxed_prediction_ceiling,
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

    let repaired_threshold =
        if early_warning_probability_cap > 0.0 && early_warning_probability_cap < base_threshold {
            best_threshold.min(early_warning_probability_cap)
        } else {
            best_threshold
        };

    crate::round3(repaired_threshold).clamp(0.005, base_threshold)
}

fn probability_threshold_decision_metrics(
    probabilities: &[f64],
    labels: &[f64],
    rows: &[&crate::ProbabilityTrainingRow],
    horizon_days: u32,
    threshold: f64,
) -> ProbabilityThresholdDecisionMetrics {
    let regime_hits =
        probability_threshold_regime_hit_summary(probabilities, rows, horizon_days, threshold);
    let mut predicted_positive_count = 0_u32;
    let mut true_positive_count = 0_u32;
    let positive_count = labels.iter().filter(|label| **label >= 0.5).count() as f64;

    for (probability, label) in probabilities.iter().zip(labels) {
        if *probability >= threshold {
            predicted_positive_count += 1;
            if *label >= 0.5 {
                true_positive_count += 1;
            }
        }
    }

    ProbabilityThresholdDecisionMetrics {
        regime_hits,
        predicted_positive_count,
        true_positive_count,
        precision: crate::safe_divide(true_positive_count as f64, predicted_positive_count as f64),
        recall: crate::safe_divide(true_positive_count as f64, positive_count),
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
    let early_warning_regime = probability_early_warning_regime(horizon_days);
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
    let regime_summary = evaluate_regime_separation_summary_refs(
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

#[derive(Debug, Clone, Copy, Default)]
struct ProbabilityCalibrationRegimeEvidenceBucket {
    full_row_count: u32,
    calibration_eligible_row_count: u32,
    calibration_used_row_count: u32,
    threshold_selected_row_count: u32,
    positive_label_count: u32,
    hard_label_sum: f64,
    training_target_sum: f64,
    objective_weight_sum: f64,
    protected_action_window_count: u32,
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
        bucket.training_target_sum +=
            crate::model::probability_training_target_label(row, horizon_days, label_mode);
        bucket.objective_weight_sum +=
            probability_calibration_objective_weight(row, horizon_days, label_mode);
        if row.protected_action_window {
            bucket.protected_action_window_count += 1;
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

fn probability_prediction_count_ceiling_from_actual_positive_count(
    actual_positive_count: u32,
    horizon_days: u32,
) -> u32 {
    let multiple = match horizon_days {
        5 => 4_u32,
        20 => 4_u32,
        60 => 5_u32,
        _ => 5_u32,
    };
    actual_positive_count.max(1).saturating_mul(multiple)
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use chrono::NaiveDate;

    use super::{
        family_overlay_audit_specs, family_overlay_has_minimum_support,
        probability_threshold_score_tuple, split_family_overlay_dataset_rows,
        FamilyOverlayAuditSpec, ProbabilityThresholdScoreInputs,
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

    #[test]
    fn twenty_day_threshold_score_penalizes_overbroad_thresholds() {
        let restrained = probability_threshold_score_tuple(ProbabilityThresholdScoreInputs {
            horizon_days: 20,
            precision: 0.19,
            recall: 0.30,
            f_beta: 0.23,
            threshold: 0.48,
            predicted_positive_count: 80,
            prediction_ceiling: 40,
            actual_positive_count: 10,
        });
        let overbroad = probability_threshold_score_tuple(ProbabilityThresholdScoreInputs {
            horizon_days: 20,
            precision: 0.18,
            recall: 0.35,
            f_beta: 0.232,
            threshold: 0.44,
            predicted_positive_count: 120,
            prediction_ceiling: 40,
            actual_positive_count: 10,
        });

        assert!(restrained > overbroad);
    }
}

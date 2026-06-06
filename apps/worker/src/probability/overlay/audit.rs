use std::collections::HashSet;

use fc_domain::ProbabilityFamilyOverlayAudit;

#[derive(Debug, Clone, Copy)]
pub(super) struct FamilyOverlayAuditSpec {
    pub(super) family_id: &'static str,
    pub(super) scenario_family: Option<&'static str>,
    pub(super) gate_feature: &'static str,
    pub(super) gate_active_threshold: f64,
    pub(super) inactive_gate_ceiling: f64,
    pub(super) min_scenario_count: u32,
    pub(super) gate_slope: f64,
    pub(super) blend_weight: f64,
    pub(super) note: &'static str,
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

    let early_warning_regime = super::super::probability_early_warning_regime(horizon_days);
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

pub(super) fn family_overlay_audit_specs() -> [FamilyOverlayAuditSpec; 5] {
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
            gate_active_threshold: 0.38,
            inactive_gate_ceiling: 0.16,
            min_scenario_count: 2,
            gate_slope: 8.0,
            blend_weight: 0.25,
            note: "candidate rows follow mixed_systemic_stress scenario labels; proxy now anchors on chronic credit/curve/funding stress and uses trigger/vix/external only as confirmation",
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
            gate_active_threshold: 0.38,
            inactive_gate_ceiling: 0.20,
            min_scenario_count: 1,
            gate_slope: 8.0,
            blend_weight: 0.30,
            note: "proxy-only audit: candidate rows include gate-active carry rows plus protected action windows, matching the overlay training dataset builder; gate tuned to the highest protected/pre-warning carry rows currently visible in free-history formal datasets",
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
            None => gate_active || row.protected_action_window,
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

pub(super) fn family_overlay_has_minimum_support(
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

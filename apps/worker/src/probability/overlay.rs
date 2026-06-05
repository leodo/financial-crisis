mod audit;
mod split;

use fc_domain::{ProbabilityFamilyOverlayAudit, ProbabilityFamilyOverlayBundle};

use audit::{family_overlay_audit_specs, family_overlay_has_minimum_support};
use split::{build_family_overlay_dataset_rows, split_family_overlay_dataset_rows};

pub(super) fn build_family_overlay_audits(
    train_rows: &[crate::ProbabilityTrainingRow],
    calibration_rows: &[crate::ProbabilityTrainingRow],
    evaluation_rows: &[crate::ProbabilityTrainingRow],
    feature_names: &[String],
    horizon_days: u32,
    label_mode: crate::ProbabilityTargetLabelMode,
) -> Vec<ProbabilityFamilyOverlayAudit> {
    audit::build_family_overlay_audits(
        train_rows,
        calibration_rows,
        evaluation_rows,
        feature_names,
        horizon_days,
        label_mode,
    )
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

#[cfg(test)]
mod tests;

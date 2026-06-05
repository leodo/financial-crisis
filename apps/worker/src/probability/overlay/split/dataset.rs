use super::FamilyOverlayAuditSpec;

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

pub(super) fn family_overlay_candidate_row(
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

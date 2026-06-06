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
    let mut deduped = Vec::with_capacity(rows.len());
    for row in rows {
        if let Some(existing) = deduped.last_mut() {
            if same_probability_training_identity(existing, &row) {
                merge_probability_training_rows(existing, row);
                continue;
            }
        }
        deduped.push(row);
    }
    deduped
}

fn same_probability_training_identity(
    left: &crate::ProbabilityTrainingRow,
    right: &crate::ProbabilityTrainingRow,
) -> bool {
    left.as_of_date == right.as_of_date
        && left.primary_scenario_id == right.primary_scenario_id
        && left.action_episode_id == right.action_episode_id
        && left.scenario_family == right.scenario_family
}

fn merge_probability_training_rows(
    target: &mut crate::ProbabilityTrainingRow,
    source: crate::ProbabilityTrainingRow,
) {
    target.primary_scenario_id = target
        .primary_scenario_id
        .take()
        .or(source.primary_scenario_id);
    target.scenario_family = target.scenario_family.take().or(source.scenario_family);
    target.scenario_training_role = merge_training_role(
        target.scenario_training_role.take(),
        source.scenario_training_role,
    );
    target.days_to_primary_crisis_start = merge_days_to_primary_crisis_start(
        target.days_to_primary_crisis_start,
        source.days_to_primary_crisis_start,
    );
    target.primary_scenario_supports_5d |= source.primary_scenario_supports_5d;
    target.primary_scenario_supports_20d |= source.primary_scenario_supports_20d;
    target.primary_scenario_supports_60d |= source.primary_scenario_supports_60d;
    target.label_5d = target.label_5d.max(source.label_5d);
    target.label_20d = target.label_20d.max(source.label_20d);
    target.label_60d = target.label_60d.max(source.label_60d);
    target.regime_5d = stronger_regime(target.regime_5d, source.regime_5d);
    target.regime_20d = stronger_regime(target.regime_20d, source.regime_20d);
    target.regime_60d = stronger_regime(target.regime_60d, source.regime_60d);
    target.action_label_5d = target.action_label_5d.max(source.action_label_5d);
    target.action_label_20d = target.action_label_20d.max(source.action_label_20d);
    target.action_label_60d = target.action_label_60d.max(source.action_label_60d);
    target.prepare_episode_label = target
        .prepare_episode_label
        .max(source.prepare_episode_label);
    target.hedge_episode_label = target.hedge_episode_label.max(source.hedge_episode_label);
    target.defend_episode_label = target.defend_episode_label.max(source.defend_episode_label);
    target.primary_action_level = stronger_action_level(
        target.primary_action_level.take(),
        source.primary_action_level,
    );
    target.action_episode_id = target.action_episode_id.take().or(source.action_episode_id);
    target.action_episode_phase = stronger_action_episode_phase(
        target.action_episode_phase.clone(),
        source.action_episode_phase,
    );
    target.protected_action_window |= source.protected_action_window;
    for (feature_name, feature_value) in source.features {
        target.features.entry(feature_name).or_insert(feature_value);
    }
}

fn merge_training_role(left: Option<String>, right: Option<String>) -> Option<String> {
    match (left.as_deref(), right.as_deref()) {
        (Some("mandatory"), _) | (_, Some("mandatory")) => Some("mandatory".to_string()),
        (Some("extension"), _) | (_, Some("extension")) => Some("extension".to_string()),
        (Some(value), None) | (None, Some(value)) => Some(value.to_string()),
        (Some(left), Some(_)) => Some(left.to_string()),
        (None, None) => None,
    }
}

fn merge_days_to_primary_crisis_start(left: Option<i64>, right: Option<i64>) -> Option<i64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

fn stronger_regime(
    left: crate::ProbabilityTrainingRegime,
    right: crate::ProbabilityTrainingRegime,
) -> crate::ProbabilityTrainingRegime {
    if regime_priority(right) > regime_priority(left) {
        right
    } else {
        left
    }
}

fn regime_priority(regime: crate::ProbabilityTrainingRegime) -> u8 {
    match regime {
        crate::ProbabilityTrainingRegime::PositiveWindow => 5,
        crate::ProbabilityTrainingRegime::PreWarningBuffer => 4,
        crate::ProbabilityTrainingRegime::InCrisis => 3,
        crate::ProbabilityTrainingRegime::PostCrisisCooldown => 2,
        crate::ProbabilityTrainingRegime::Normal => 1,
    }
}

fn stronger_action_level(left: Option<String>, right: Option<String>) -> Option<String> {
    match (left, right) {
        (Some(left), Some(right)) => {
            if action_level_priority(&right) > action_level_priority(&left) {
                Some(right)
            } else {
                Some(left)
            }
        }
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

fn action_level_priority(level: &str) -> u8 {
    match level {
        "defend" => 3,
        "hedge" => 2,
        "prepare" => 1,
        _ => 0,
    }
}

fn stronger_action_episode_phase(left: String, right: String) -> String {
    if action_episode_phase_priority(&right) > action_episode_phase_priority(&left) {
        right
    } else {
        left
    }
}

fn action_episode_phase_priority(phase: &str) -> u8 {
    match phase {
        "primary" => 4,
        "late_validation" => 3,
        "cooldown" => 2,
        _ => 1,
    }
}

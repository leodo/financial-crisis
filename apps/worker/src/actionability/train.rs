use fc_domain::{ActionabilityBundle, ActionabilityLevel, ActionabilityLevelBundle};

pub(crate) fn train_actionability_bundle(
    train_rows: &[crate::ProbabilityTrainingRow],
    calibration_rows: &[crate::ProbabilityTrainingRow],
    evaluation_rows: &[crate::ProbabilityTrainingRow],
    feature_names: &[String],
    release_suffix: &str,
) -> anyhow::Result<ActionabilityBundle> {
    let levels = [
        (ActionabilityLevel::Prepare, 60_u32),
        (ActionabilityLevel::Hedge, 20_u32),
        (ActionabilityLevel::Defend, 5_u32),
    ]
    .into_iter()
    .map(|(level, proxy_horizon_days)| {
        train_actionability_level_bundle(
            train_rows,
            calibration_rows,
            evaluation_rows,
            feature_names,
            level,
            proxy_horizon_days,
        )
    })
    .collect::<anyhow::Result<Vec<_>>>()?;

    Ok(ActionabilityBundle {
        model_version: format!("actionability_bundle_{release_suffix}"),
        calibration_version: format!("actionability_platt_{release_suffix}"),
        fusion_policy_version: "fusion_policy_v3_probability_context_gate_20260601".to_string(),
        note: "Separate actionability head trained from episode-native prepare/hedge/defend labels to complement the crisis-prior horizons; runtime consumes threshold-aware confidence instead of treating raw action probabilities as direct posture signals.".to_string(),
        levels,
    })
}

fn train_actionability_level_bundle(
    train_rows: &[crate::ProbabilityTrainingRow],
    calibration_rows: &[crate::ProbabilityTrainingRow],
    evaluation_rows: &[crate::ProbabilityTrainingRow],
    feature_names: &[String],
    level: ActionabilityLevel,
    proxy_horizon_days: u32,
) -> anyhow::Result<ActionabilityLevelBundle> {
    let label_mode = crate::ProbabilityTargetLabelMode::ActionEpisode;
    crate::ensure_positive_labels(train_rows, proxy_horizon_days, "train", label_mode)?;
    crate::ensure_positive_labels(
        calibration_rows,
        proxy_horizon_days,
        "calibration",
        label_mode,
    )?;
    crate::ensure_positive_labels(
        evaluation_rows,
        proxy_horizon_days,
        "evaluation",
        label_mode,
    )?;

    let raw_model =
        crate::fit_logistic_model(train_rows, feature_names, proxy_horizon_days, label_mode);
    let calibration_inputs = calibration_rows
        .iter()
        .map(|row| crate::score_logistic_model_for_dataset(&raw_model, row))
        .collect::<Vec<_>>();
    let calibration_labels = calibration_rows
        .iter()
        .map(|row| row.label_for_horizon(label_mode, proxy_horizon_days))
        .collect::<Vec<_>>();
    let calibration_candidate =
        crate::fit_platt_calibration(&calibration_inputs, &calibration_labels);
    let evaluation_raw_probabilities = evaluation_rows
        .iter()
        .map(|row| crate::score_logistic_model_for_dataset(&raw_model, row))
        .collect::<Vec<_>>();
    let evaluation_labels = evaluation_rows
        .iter()
        .map(|row| row.label_for_horizon(label_mode, proxy_horizon_days))
        .collect::<Vec<_>>();
    let (calibration, evaluation_probabilities, decision_threshold) =
        super::select_actionability_calibration_strategy(
            &calibration_inputs,
            calibration_rows,
            &evaluation_raw_probabilities,
            proxy_horizon_days,
            calibration_candidate,
        );

    let mut evaluation =
        crate::evaluate_probabilities(&evaluation_probabilities, &evaluation_labels);
    evaluation.actionability = Some(super::evaluate_actionability_summary(
        &evaluation_probabilities,
        evaluation_rows,
        proxy_horizon_days,
        decision_threshold,
    ));

    Ok(ActionabilityLevelBundle {
        level,
        proxy_horizon_days,
        target_label_mode: label_mode.as_str().to_string(),
        decision_threshold,
        raw_model,
        calibration,
        evaluation,
    })
}

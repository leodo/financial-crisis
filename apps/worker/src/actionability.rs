use std::collections::BTreeMap;

use fc_domain::{
    apply_platt_probability_calibration, ActionabilityBundle, ActionabilityEvaluationSummary,
    ActionabilityLevel, ActionabilityLevelBundle, PlattCalibrationArtifact,
};

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
        select_actionability_calibration_strategy(
            &calibration_inputs,
            calibration_rows,
            &evaluation_raw_probabilities,
            proxy_horizon_days,
            calibration_candidate,
        );

    let mut evaluation =
        crate::evaluate_probabilities(&evaluation_probabilities, &evaluation_labels);
    evaluation.actionability = Some(evaluate_actionability_summary(
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

pub(crate) fn select_actionability_decision_threshold(
    probabilities: &[f64],
    rows: &[crate::ProbabilityTrainingRow],
    horizon_days: u32,
) -> f64 {
    let mut thresholds = probabilities
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .filter(|value| (0.01..0.99).contains(value))
        .collect::<Vec<_>>();
    thresholds.extend((5..=60).map(|value| value as f64 / 100.0));
    thresholds.push(0.3);
    thresholds.sort_by(f64::total_cmp);
    thresholds.dedup_by(|left, right| (*left - *right).abs() < 1e-6);

    let mut best_threshold = 0.3;
    let mut best_score = None::<(bool, bool, bool, u32, u32, i64, i64, i64)>;
    for threshold in thresholds {
        let summary = evaluate_actionability_summary(probabilities, rows, horizon_days, threshold);
        if summary.predicted_positive_count == 0 {
            continue;
        }
        let hit_scenario_count =
            summary.advance_warning_scenario_count + summary.late_confirmation_scenario_count;
        if hit_scenario_count == 0 {
            continue;
        }
        let precision_score =
            (summary.precision_at_threshold.unwrap_or_default() * 1_000.0).round() as i64;
        let false_positive_penalty = -(summary.false_positive_count as i64);
        let threshold_score = (threshold * 1_000.0).round() as i64;
        let meets_precision_floor =
            precision_score >= actionability_precision_floor_score(horizon_days);
        let meets_volume_ceiling = summary.predicted_positive_count
            <= actionability_prediction_count_ceiling(&summary, horizon_days);
        let score = (
            meets_precision_floor && meets_volume_ceiling,
            meets_precision_floor,
            meets_volume_ceiling,
            hit_scenario_count,
            summary.advance_warning_scenario_count,
            precision_score,
            false_positive_penalty,
            threshold_score,
        );
        if best_score.is_none_or(|best| score > best) {
            best_score = Some(score);
            best_threshold = threshold;
        }
    }

    crate::round3(best_threshold).clamp(0.05, 0.60)
}

fn actionability_precision_floor_score(horizon_days: u32) -> i64 {
    match horizon_days {
        5 => 120,
        20 => 100,
        60 => 80,
        _ => 100,
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ActionabilityGuardrailPolicy {
    pub(crate) min_scenario_count: u32,
    pub(crate) min_precision_score: i64,
    pub(crate) min_advance_warning_rate_score: Option<i64>,
    pub(crate) max_late_confirmation_rate_score: Option<i64>,
    pub(crate) max_missed_rate_score: i64,
}

pub(crate) fn actionability_guardrail_policy(
    level: ActionabilityLevel,
    horizon_days: u32,
) -> ActionabilityGuardrailPolicy {
    match level {
        ActionabilityLevel::Prepare => ActionabilityGuardrailPolicy {
            min_scenario_count: 2,
            min_precision_score: actionability_precision_floor_score(horizon_days),
            min_advance_warning_rate_score: Some(350),
            max_late_confirmation_rate_score: Some(500),
            max_missed_rate_score: 650,
        },
        ActionabilityLevel::Hedge => ActionabilityGuardrailPolicy {
            min_scenario_count: 2,
            min_precision_score: actionability_precision_floor_score(horizon_days),
            min_advance_warning_rate_score: Some(250),
            max_late_confirmation_rate_score: Some(500),
            max_missed_rate_score: 650,
        },
        ActionabilityLevel::Defend => ActionabilityGuardrailPolicy {
            min_scenario_count: 2,
            min_precision_score: actionability_precision_floor_score(horizon_days),
            min_advance_warning_rate_score: None,
            max_late_confirmation_rate_score: Some(400),
            max_missed_rate_score: 500,
        },
    }
}

pub(crate) fn percentage_score(value: Option<f64>) -> Option<i64> {
    value.map(|rate| (rate * 1_000.0).round() as i64)
}

pub(crate) fn actionability_prediction_count_ceiling_from_actual_positive_count(
    actual_positive_count: u32,
    horizon_days: u32,
) -> u32 {
    let multiple = match horizon_days {
        5 => 6_u32,
        20 => 8_u32,
        60 => 10_u32,
        _ => 8_u32,
    };
    actual_positive_count.max(1).saturating_mul(multiple)
}

fn actionability_prediction_count_ceiling(
    summary: &ActionabilityEvaluationSummary,
    horizon_days: u32,
) -> u32 {
    actionability_prediction_count_ceiling_from_actual_positive_count(
        summary.actual_positive_count,
        horizon_days,
    )
}

pub(crate) fn actionability_bundle_quality_regressions(
    bundle: &ActionabilityBundle,
) -> Vec<String> {
    let mut regressions = Vec::new();
    for level in &bundle.levels {
        let Some(summary) = level.evaluation.actionability.as_ref() else {
            regressions.push(format!(
                "{} has no evaluation summary",
                crate::actionability_level_text(level.level)
            ));
            continue;
        };

        let policy = actionability_guardrail_policy(level.level, level.proxy_horizon_days);
        let precision_score =
            (summary.precision_at_threshold.unwrap_or_default() * 1_000.0).round() as i64;
        if precision_score < policy.min_precision_score {
            regressions.push(format!(
                "{} precision {:.1}% is below required {:.1}%",
                crate::actionability_level_text(level.level),
                precision_score as f64 / 10.0,
                policy.min_precision_score as f64 / 10.0
            ));
        }

        if summary.scenario_count < policy.min_scenario_count {
            regressions.push(format!(
                "{} scenario_count {} is below required {}",
                crate::actionability_level_text(level.level),
                summary.scenario_count,
                policy.min_scenario_count
            ));
        }

        let prediction_ceiling =
            actionability_prediction_count_ceiling(summary, level.proxy_horizon_days);
        if summary.predicted_positive_count > prediction_ceiling {
            regressions.push(format!(
                "{} predicted positives {} exceed ceiling {} for {} primary episode rows",
                crate::actionability_level_text(level.level),
                summary.predicted_positive_count,
                prediction_ceiling,
                summary.actual_positive_count
            ));
        }

        if summary.actual_positive_count > 0 {
            if let Some(min_advance_warning_rate_score) = policy.min_advance_warning_rate_score {
                let advance_warning_rate_score =
                    percentage_score(summary.advance_warning_rate).unwrap_or_default();
                if advance_warning_rate_score < min_advance_warning_rate_score {
                    regressions.push(format!(
                        "{} on_time_rate {:.1}% is below required {:.1}%",
                        crate::actionability_level_text(level.level),
                        advance_warning_rate_score as f64 / 10.0,
                        min_advance_warning_rate_score as f64 / 10.0
                    ));
                }
            }

            if let Some(max_late_confirmation_rate_score) = policy.max_late_confirmation_rate_score
            {
                let late_confirmation_rate_score =
                    percentage_score(summary.late_confirmation_rate).unwrap_or_default();
                if late_confirmation_rate_score > max_late_confirmation_rate_score {
                    regressions.push(format!(
                        "{} late_only_rate {:.1}% exceeds ceiling {:.1}%",
                        crate::actionability_level_text(level.level),
                        late_confirmation_rate_score as f64 / 10.0,
                        max_late_confirmation_rate_score as f64 / 10.0
                    ));
                }
            }

            let missed_rate_score = percentage_score(summary.missed_rate).unwrap_or_default();
            if missed_rate_score > policy.max_missed_rate_score {
                regressions.push(format!(
                    "{} missed_rate {:.1}% exceeds ceiling {:.1}%",
                    crate::actionability_level_text(level.level),
                    missed_rate_score as f64 / 10.0,
                    policy.max_missed_rate_score as f64 / 10.0
                ));
            }
        }
    }
    regressions
}

pub(crate) fn select_actionability_calibration_strategy(
    calibration_raw_probabilities: &[f64],
    calibration_rows: &[crate::ProbabilityTrainingRow],
    evaluation_raw_probabilities: &[f64],
    horizon_days: u32,
    calibration_candidate: PlattCalibrationArtifact,
) -> (Option<PlattCalibrationArtifact>, Vec<f64>, f64) {
    let raw_threshold = select_actionability_decision_threshold(
        calibration_raw_probabilities,
        calibration_rows,
        horizon_days,
    );
    let raw_summary = evaluate_actionability_summary(
        calibration_raw_probabilities,
        calibration_rows,
        horizon_days,
        raw_threshold,
    );
    let raw_score =
        actionability_summary_selection_score(&raw_summary, raw_threshold, horizon_days);

    let calibration_probabilities = calibration_raw_probabilities
        .iter()
        .map(|raw_probability| {
            apply_platt_probability_calibration(*raw_probability, &calibration_candidate)
        })
        .collect::<Vec<_>>();
    let calibrated_threshold = select_actionability_decision_threshold(
        &calibration_probabilities,
        calibration_rows,
        horizon_days,
    );
    let calibrated_summary = evaluate_actionability_summary(
        &calibration_probabilities,
        calibration_rows,
        horizon_days,
        calibrated_threshold,
    );
    let calibrated_score = actionability_summary_selection_score(
        &calibrated_summary,
        calibrated_threshold,
        horizon_days,
    );

    let keep_calibration = calibration_candidate.alpha > 0.0 && calibrated_score > raw_score;
    if keep_calibration {
        let evaluation_probabilities = evaluation_raw_probabilities
            .iter()
            .map(|raw_probability| {
                apply_platt_probability_calibration(*raw_probability, &calibration_candidate)
            })
            .collect::<Vec<_>>();
        (
            Some(calibration_candidate),
            evaluation_probabilities,
            calibrated_threshold,
        )
    } else {
        (None, evaluation_raw_probabilities.to_vec(), raw_threshold)
    }
}

fn actionability_summary_selection_score(
    summary: &ActionabilityEvaluationSummary,
    threshold: f64,
    horizon_days: u32,
) -> (bool, bool, bool, u32, u32, i64, i64, i64) {
    let hit_scenario_count =
        summary.advance_warning_scenario_count + summary.late_confirmation_scenario_count;
    let precision_score =
        (summary.precision_at_threshold.unwrap_or_default() * 1_000.0).round() as i64;
    let false_positive_penalty = -(summary.false_positive_count as i64);
    let threshold_score = (threshold * 1_000.0).round() as i64;
    let meets_precision_floor =
        precision_score >= actionability_precision_floor_score(horizon_days);
    let meets_volume_ceiling = summary.predicted_positive_count
        <= actionability_prediction_count_ceiling(summary, horizon_days);
    (
        meets_precision_floor && meets_volume_ceiling,
        meets_precision_floor,
        meets_volume_ceiling,
        hit_scenario_count,
        summary.advance_warning_scenario_count,
        precision_score,
        false_positive_penalty,
        threshold_score,
    )
}

#[derive(Default)]
struct ActionabilityScenarioEvaluationState {
    saw_positive: bool,
    has_pre_start_hit: bool,
    has_post_start_hit: bool,
}

pub(crate) fn evaluate_actionability_summary(
    probabilities: &[f64],
    rows: &[crate::ProbabilityTrainingRow],
    horizon_days: u32,
    threshold: f64,
) -> ActionabilityEvaluationSummary {
    let label_mode = crate::ProbabilityTargetLabelMode::ActionEpisode;
    let mut predicted_positive_count = 0_u32;
    let mut actual_positive_count = 0_u32;
    let mut primary_positive_count = 0_u32;
    let mut late_validation_row_count = 0_u32;
    let mut cooldown_row_count = 0_u32;
    let mut primary_hit_count = 0_u32;
    let mut late_validation_hit_count = 0_u32;
    let mut cooldown_hit_count = 0_u32;
    let mut false_positive_count = 0_u32;
    let mut scenario_states = BTreeMap::<String, ActionabilityScenarioEvaluationState>::new();

    for (probability, row) in probabilities.iter().zip(rows) {
        let predicted_positive = *probability >= threshold;
        let actual_positive = row.label_for_horizon(label_mode, horizon_days) >= 0.5;
        let phase = row.action_episode_phase_for_horizon(horizon_days);

        if predicted_positive {
            predicted_positive_count += 1;
        }

        if actual_positive {
            actual_positive_count += 1;
            primary_positive_count += 1;
            if let Some(scenario_id) = row.primary_scenario_id.as_ref() {
                scenario_states
                    .entry(scenario_id.clone())
                    .or_default()
                    .saw_positive = true;
            }
        } else {
            match phase {
                crate::ActionEpisodePhase::LateValidation => late_validation_row_count += 1,
                crate::ActionEpisodePhase::Cooldown => cooldown_row_count += 1,
                _ => {}
            }
        }

        if predicted_positive {
            if actual_positive {
                primary_hit_count += 1;
                if let Some(scenario_id) = row.primary_scenario_id.as_ref() {
                    let state = scenario_states.entry(scenario_id.clone()).or_default();
                    state.saw_positive = true;
                    state.has_pre_start_hit = true;
                }
            } else if matches!(phase, crate::ActionEpisodePhase::LateValidation) {
                late_validation_hit_count += 1;
                if let Some(scenario_id) = row.primary_scenario_id.as_ref() {
                    let state = scenario_states.entry(scenario_id.clone()).or_default();
                    state.saw_positive = true;
                    state.has_post_start_hit = true;
                }
            } else if matches!(phase, crate::ActionEpisodePhase::Cooldown) {
                cooldown_hit_count += 1;
            } else {
                false_positive_count += 1;
            }
        }
    }

    let mut advance_warning_scenario_count = 0_u32;
    let mut late_confirmation_scenario_count = 0_u32;
    let mut missed_scenario_count = 0_u32;
    for state in scenario_states.values().filter(|state| state.saw_positive) {
        if state.has_pre_start_hit {
            advance_warning_scenario_count += 1;
        } else if state.has_post_start_hit {
            late_confirmation_scenario_count += 1;
        } else {
            missed_scenario_count += 1;
        }
    }

    let hit_count = primary_hit_count + late_validation_hit_count;
    let scenario_count =
        advance_warning_scenario_count + late_confirmation_scenario_count + missed_scenario_count;

    ActionabilityEvaluationSummary {
        threshold: crate::round3(threshold),
        predicted_positive_count,
        actual_positive_count,
        pre_start_positive_count: primary_positive_count,
        post_start_positive_count: late_validation_row_count,
        unclassified_positive_count: cooldown_row_count,
        pre_start_hit_count: primary_hit_count,
        post_start_hit_count: late_validation_hit_count,
        unclassified_hit_count: cooldown_hit_count,
        false_positive_count,
        scenario_count,
        advance_warning_scenario_count,
        late_confirmation_scenario_count,
        missed_scenario_count,
        precision_at_threshold: (predicted_positive_count > 0)
            .then_some(crate::round3(hit_count as f64 / predicted_positive_count as f64)),
        pre_start_recall_at_threshold: (primary_positive_count > 0)
            .then_some(crate::round3(primary_hit_count as f64 / primary_positive_count as f64)),
        post_start_recall_at_threshold: (late_validation_row_count > 0).then_some(crate::round3(
            late_validation_hit_count as f64 / late_validation_row_count as f64,
        )),
        advance_warning_rate: (scenario_count > 0).then_some(crate::round3(
            advance_warning_scenario_count as f64 / scenario_count as f64,
        )),
        late_confirmation_rate: (scenario_count > 0).then_some(crate::round3(
            late_confirmation_scenario_count as f64 / scenario_count as f64,
        )),
        missed_rate: (scenario_count > 0)
            .then_some(crate::round3(missed_scenario_count as f64 / scenario_count as f64)),
        note: "Primary means the episode-native action window fired on time; post-start metrics now represent late-validation tracking rather than crisis-start proxy labels.".to_string(),
    }
}

use std::collections::BTreeMap;

use fc_domain::ActionabilityEvaluationSummary;

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

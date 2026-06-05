use fc_domain::{ActionabilityBundle, ActionabilityLevel};

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
            min_precision_score: super::actionability_precision_floor_score(horizon_days),
            min_advance_warning_rate_score: Some(350),
            max_late_confirmation_rate_score: Some(500),
            max_missed_rate_score: 650,
        },
        ActionabilityLevel::Hedge => ActionabilityGuardrailPolicy {
            min_scenario_count: 2,
            min_precision_score: super::actionability_precision_floor_score(horizon_days),
            min_advance_warning_rate_score: Some(250),
            max_late_confirmation_rate_score: Some(500),
            max_missed_rate_score: 650,
        },
        ActionabilityLevel::Defend => ActionabilityGuardrailPolicy {
            min_scenario_count: 2,
            min_precision_score: super::actionability_precision_floor_score(horizon_days),
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
            super::actionability_prediction_count_ceiling(summary, level.proxy_horizon_days);
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

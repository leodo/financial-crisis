use fc_domain::{AssessmentSnapshot, ModelReleaseRecord};

pub(crate) fn build_release_actionability_review(
    release: &ModelReleaseRecord,
) -> anyhow::Result<crate::ReleaseActionabilityReview> {
    let bundle =
        crate::read_probability_bundle(std::path::Path::new(&release.manifest.bundle_uri))?;
    let Some(actionability) = bundle.actionability.as_ref() else {
        return Ok(crate::ReleaseActionabilityReview {
            release_id: release.manifest.release_id.clone(),
            enabled: false,
            model_version: None,
            calibration_version: None,
            fusion_policy_version: None,
            levels: Vec::new(),
            guard_regressions: Vec::new(),
            guard_passed: true,
            note: "This release has no independent actionability head; release review only applies runtime guardrails.".to_string(),
        });
    };

    let levels = actionability
        .levels
        .iter()
        .map(|level| {
            let evaluation = level
                .evaluation
                .actionability
                .as_ref()
                .cloned()
                .unwrap_or_default();
            crate::ReleaseActionabilityLevelReview {
                level: level.level,
                proxy_horizon_days: level.proxy_horizon_days,
                sample_count: level.evaluation.sample_count,
                positive_rate: level.evaluation.positive_rate,
                threshold: evaluation.threshold,
                predicted_positive_count: evaluation.predicted_positive_count,
                primary_positive_count: evaluation.actual_positive_count,
                late_validation_row_count: evaluation.post_start_positive_count,
                protected_row_count: evaluation.unclassified_positive_count,
                primary_hit_count: evaluation.pre_start_hit_count,
                late_validation_hit_count: evaluation.post_start_hit_count,
                protected_hit_count: evaluation.unclassified_hit_count,
                false_positive_count: evaluation.false_positive_count,
                scenario_count: evaluation.scenario_count,
                on_time_scenario_count: evaluation.advance_warning_scenario_count,
                late_only_scenario_count: evaluation.late_confirmation_scenario_count,
                missed_scenario_count: evaluation.missed_scenario_count,
                precision_at_threshold: evaluation.precision_at_threshold,
                primary_recall_at_threshold: evaluation.pre_start_recall_at_threshold,
                late_validation_capture_rate: evaluation.post_start_recall_at_threshold,
                on_time_rate: evaluation.advance_warning_rate,
                late_only_rate: evaluation.late_confirmation_rate,
                missed_rate: evaluation.missed_rate,
                note: evaluation.note,
            }
        })
        .collect::<Vec<_>>();

    let mut review = crate::ReleaseActionabilityReview {
        release_id: release.manifest.release_id.clone(),
        enabled: true,
        model_version: Some(actionability.model_version.clone()),
        calibration_version: Some(actionability.calibration_version.clone()),
        fusion_policy_version: Some(actionability.fusion_policy_version.clone()),
        levels,
        guard_regressions: Vec::new(),
        guard_passed: true,
        note: actionability.note.clone(),
    };
    review.guard_regressions = compare_actionability_guardrails(&review);
    review.guard_passed = review.guard_regressions.is_empty();
    Ok(review)
}

pub(crate) fn compare_actionability_guardrails(
    review: &crate::ReleaseActionabilityReview,
) -> Vec<String> {
    if !review.enabled {
        return Vec::new();
    }

    let mut regressions = Vec::new();
    for level in &review.levels {
        let level_name = crate::actionability_level_text(level.level);
        let policy = crate::actionability_guardrail_policy(level.level, level.proxy_horizon_days);

        if level.scenario_count < policy.min_scenario_count {
            regressions.push(format!(
                "actionability {level_name} scenario_count is {} (<{}), so the evaluation slice is too narrow for go/no-go",
                level.scenario_count, policy.min_scenario_count
            ));
        }

        let precision_score =
            (level.precision_at_threshold.unwrap_or_default() * 1_000.0).round() as i64;
        if precision_score < policy.min_precision_score {
            regressions.push(format!(
                "actionability {level_name} precision {:.1}% is below required {:.1}%",
                precision_score as f64 / 10.0,
                policy.min_precision_score as f64 / 10.0
            ));
        }

        let prediction_ceiling =
            crate::actionability_prediction_count_ceiling_from_actual_positive_count(
                level.primary_positive_count,
                level.proxy_horizon_days,
            );
        if level.predicted_positive_count > prediction_ceiling {
            regressions.push(format!(
                "actionability {level_name} predicted positives {} exceed ceiling {} for {} primary episode rows",
                level.predicted_positive_count,
                prediction_ceiling,
                level.primary_positive_count
            ));
        }

        if level.primary_positive_count > 0
            && level.primary_hit_count == 0
            && level.late_validation_hit_count == 0
        {
            regressions.push(format!(
                "actionability {level_name} produced no primary or late-validation hits across {} primary episode rows",
                level.primary_positive_count
            ));
        }

        if level.primary_positive_count > 0 {
            if let Some(min_advance_warning_rate_score) = policy.min_advance_warning_rate_score {
                let on_time_rate_score =
                    crate::percentage_score(level.on_time_rate).unwrap_or_default();
                if on_time_rate_score < min_advance_warning_rate_score {
                    regressions.push(format!(
                        "actionability {level_name} on_time_rate {:.1}% is below required {:.1}%",
                        on_time_rate_score as f64 / 10.0,
                        min_advance_warning_rate_score as f64 / 10.0
                    ));
                }
            }

            if let Some(max_late_confirmation_rate_score) = policy.max_late_confirmation_rate_score
            {
                let late_only_rate_score =
                    crate::percentage_score(level.late_only_rate).unwrap_or_default();
                if late_only_rate_score > max_late_confirmation_rate_score {
                    regressions.push(format!(
                        "actionability {level_name} late_only_rate {:.1}% exceeds ceiling {:.1}%",
                        late_only_rate_score as f64 / 10.0,
                        max_late_confirmation_rate_score as f64 / 10.0
                    ));
                }
            }

            let missed_rate_score = crate::percentage_score(level.missed_rate).unwrap_or_default();
            if missed_rate_score > policy.max_missed_rate_score {
                regressions.push(format!(
                    "actionability {level_name} missed_rate {:.1}% exceeds ceiling {:.1}%",
                    missed_rate_score as f64 / 10.0,
                    policy.max_missed_rate_score as f64 / 10.0
                ));
            }
        }
    }
    regressions
}

pub(crate) fn compare_probability_guardrails(
    release: &ModelReleaseRecord,
) -> anyhow::Result<Vec<String>> {
    if release.manifest.probability_mode == "heuristic_mvp" {
        return Ok(vec![format!(
            "release {} has no formal probability bundle evaluation, so it cannot satisfy formal promotion guard",
            release.manifest.release_id
        )]);
    }

    let bundle =
        crate::read_probability_bundle(std::path::Path::new(&release.manifest.bundle_uri))?;
    let Some(summary) = bundle.evaluation.as_ref() else {
        return Ok(vec![format!(
            "release {} bundle is missing aggregate probability evaluation summary",
            release.manifest.release_id
        )]);
    };

    let mut regressions = Vec::new();
    if summary.usable_early_warning_horizon_count == 0 {
        regressions.push(
            "probability head has zero usable early-warning horizons in bundle evaluation"
                .to_string(),
        );
    }

    for horizon in &summary.regime_separation_summaries {
        if horizon.horizon_days == 20
            && horizon.positive_window_avg_probability <= horizon.normal_avg_probability
        {
            regressions.push(format!(
                "20d positive_window avg {} is at or below normal {} in bundle evaluation",
                crate::format_pct(horizon.positive_window_avg_probability),
                crate::format_pct(horizon.normal_avg_probability),
            ));
        }
        if matches!(horizon.horizon_days, 20 | 60)
            && matches!(
                horizon.diagnosis.as_str(),
                "cooldown_bleed" | "cold_across_all_regimes"
            )
        {
            regressions.push(format!(
                "{}d regime diagnosis is {} in bundle evaluation",
                horizon.horizon_days, horizon.diagnosis
            ));
        }
    }

    Ok(regressions)
}

pub(crate) fn compare_operational_guardrails(
    baseline: &AssessmentSnapshot,
    candidate: &AssessmentSnapshot,
) -> Vec<String> {
    let mut regressions = Vec::new();
    let baseline_summary = &baseline.backtest_summary;
    let candidate_summary = &candidate.backtest_summary;
    let baseline_rolling = &baseline_summary.rolling_audit;
    let candidate_rolling = &candidate_summary.rolling_audit;

    if candidate_summary.timely_warning_rate + 0.05 < baseline_summary.timely_warning_rate {
        regressions.push(format!(
            "timely_warning_rate dropped from {:.1}% to {:.1}%",
            baseline_summary.timely_warning_rate * 100.0,
            candidate_summary.timely_warning_rate * 100.0
        ));
    }

    if candidate_rolling.actionable_precision + 0.05 < baseline_rolling.actionable_precision {
        regressions.push(format!(
            "actionable_precision dropped from {:.1}% to {:.1}%",
            baseline_rolling.actionable_precision * 100.0,
            candidate_rolling.actionable_precision * 100.0
        ));
    }

    if candidate_rolling.longest_false_positive_episode_days
        > baseline_rolling.longest_false_positive_episode_days + 7
    {
        regressions.push(format!(
            "longest_false_positive_episode_days increased from {} to {}",
            baseline_rolling.longest_false_positive_episode_days,
            candidate_rolling.longest_false_positive_episode_days
        ));
    }

    regressions
}

pub(crate) fn compare_release_review_count_guardrails(
    comparison: &crate::ReleaseReviewComparisonSummary,
) -> Vec<String> {
    let mut regressions = Vec::new();
    let runtime_floor_hits = &comparison.runtime_floor_hit_count;
    if runtime_floor_hits.delta <= -5 {
        regressions.push(format!(
            "runtime_floor_hit_count dropped from {} to {}",
            runtime_floor_hits.baseline, runtime_floor_hits.candidate
        ));
    }
    regressions
}

pub(crate) fn compare_runtime_sanity_guardrails(
    baseline: &crate::ReleaseRuntimeReviewDiagnostics,
    candidate: &crate::ReleaseRuntimeReviewDiagnostics,
) -> Vec<String> {
    let mut regressions = Vec::new();
    let usable_early_warning_horizon_count = candidate
        .regime_separation_summaries
        .iter()
        .filter(|summary| summary.diagnosis == "usable_early_warning_separation")
        .count();

    if usable_early_warning_horizon_count == 0 {
        regressions.push(format!(
            "candidate {} has zero usable early-warning horizons in runtime regime audit",
            candidate.release_id
        ));
    }

    for summary in &candidate.regime_separation_summaries {
        if summary.horizon_days == 20
            && summary.positive_window_avg_probability <= summary.normal_avg_probability
        {
            regressions.push(format!(
                "candidate {} keeps 20d positive_window avg {} at or below normal {} in runtime history",
                candidate.release_id,
                crate::format_pct(summary.positive_window_avg_probability),
                crate::format_pct(summary.normal_avg_probability),
            ));
        }
        if matches!(summary.horizon_days, 20 | 60) && summary.diagnosis == "cooldown_bleed" {
            regressions.push(format!(
                "candidate {} shows cooldown_bleed on {}d runtime regime audit: cooldown {} vs positive_window {}",
                candidate.release_id,
                summary.horizon_days,
                crate::format_pct(summary.post_crisis_cooldown_avg_probability),
                crate::format_pct(summary.positive_window_avg_probability),
            ));
        }
    }

    if release_has_cold_runtime_history(candidate) {
        regressions.push(format!(
            "candidate {} stayed all-normal across {} history points, hit zero runtime probability floors, and still showed no usable early-warning regime separation",
            candidate.release_id, candidate.history_point_count
        ));
    }

    if release_has_cold_runtime_history(baseline) {
        regressions.push(format!(
            "baseline {} is also all-normal / zero-floor-hit, so relative guardrails alone are not a sufficient promotion test",
            baseline.release_id
        ));
    }

    regressions
}

fn release_has_cold_runtime_history(diagnostics: &crate::ReleaseRuntimeReviewDiagnostics) -> bool {
    let all_normal = diagnostics.posture_distribution.len() == 1
        && diagnostics.posture_distribution.first().is_some_and(|row| {
            row.name == "normal" && row.count == diagnostics.history_point_count
        });
    let zero_floor_hits = diagnostics.runtime_thresholds.is_some()
        && [
            diagnostics
                .points_at_or_above_prepare_p60d
                .unwrap_or_default(),
            diagnostics
                .points_at_or_above_hedge_p20d
                .unwrap_or_default(),
            diagnostics
                .points_at_or_above_defend_p5d
                .unwrap_or_default(),
        ]
        .into_iter()
        .all(|count| count == 0);
    let no_usable_early_warning = !diagnostics
        .regime_separation_summaries
        .iter()
        .any(|summary| {
            matches!(
                summary.diagnosis.as_str(),
                "usable_early_warning_separation" | "separated_but_below_runtime_floor"
            )
        });

    all_normal && zero_floor_hits && no_usable_early_warning
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scalar_metric() -> crate::ReleaseReviewScalarMetric {
        crate::ReleaseReviewScalarMetric {
            baseline: 0.0,
            candidate: 0.0,
            delta: 0.0,
        }
    }

    fn count_metric(baseline: u32, candidate: u32) -> crate::ReleaseReviewCountMetric {
        crate::ReleaseReviewCountMetric {
            baseline,
            candidate,
            delta: candidate as i64 - baseline as i64,
        }
    }

    fn comparison_with_runtime_floor_hits(
        baseline: u32,
        candidate: u32,
    ) -> crate::ReleaseReviewComparisonSummary {
        crate::ReleaseReviewComparisonSummary {
            timely_warning_rate: scalar_metric(),
            strict_actionable_point_count: count_metric(0, 0),
            runtime_floor_hit_count: count_metric(baseline, candidate),
            actionable_precision: scalar_metric(),
            longest_false_positive_episode_days: count_metric(0, 0),
            current_p_5d: scalar_metric(),
            current_p_20d: scalar_metric(),
            current_p_60d: scalar_metric(),
            runtime_separation_summary: Vec::new(),
            backtest_scenarios: Vec::new(),
        }
    }

    #[test]
    fn release_review_count_guardrails_reject_runtime_floor_hit_regression() {
        let regressions =
            compare_release_review_count_guardrails(&comparison_with_runtime_floor_hits(91, 69));

        assert!(regressions
            .iter()
            .any(|item| item.contains("runtime_floor_hit_count dropped from 91 to 69")));
    }

    #[test]
    fn release_review_count_guardrails_allow_small_runtime_floor_noise() {
        let regressions =
            compare_release_review_count_guardrails(&comparison_with_runtime_floor_hits(9, 7));

        assert!(regressions.is_empty());
    }
}

pub(crate) fn print_operational_guardrail_summary(
    baseline: &AssessmentSnapshot,
    candidate: &AssessmentSnapshot,
) {
    println!("Operational guard summary:");
    println!(
        "  timely_warning_rate   {} -> {}",
        crate::format_pct(baseline.backtest_summary.timely_warning_rate),
        crate::format_pct(candidate.backtest_summary.timely_warning_rate)
    );
    println!(
        "  actionable_precision  {} -> {}",
        crate::format_pct(baseline.backtest_summary.rolling_audit.actionable_precision),
        crate::format_pct(
            candidate
                .backtest_summary
                .rolling_audit
                .actionable_precision
        )
    );
    println!(
        "  longest_false_positive_episode_days  {} -> {}",
        baseline
            .backtest_summary
            .rolling_audit
            .longest_false_positive_episode_days,
        candidate
            .backtest_summary
            .rolling_audit
            .longest_false_positive_episode_days
    );
}

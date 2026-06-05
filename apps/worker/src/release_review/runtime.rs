use std::collections::BTreeMap;

use anyhow::Result;
use fc_domain::AssessmentHistoryPoint;

use crate::{
    forward_crisis_training_regime, load_label_set_crisis_scenarios,
    probability_training_regime_name, regime_positive_window_gap_floor, round6, safe_divide,
    safe_ratio, AuditMethodResponseWire, CrisisScenario, RuntimeThresholdDiagnosticsWire,
    DEFAULT_FORMAL_LABEL_VERSION, DEFAULT_FORMAL_SCENARIO_SET_VERSION,
};

use super::{
    ReleaseReviewRuntimeSeparationComparison, ReleaseRuntimeClauseCount, ReleaseRuntimeCount,
    ReleaseRuntimeRegimeProbabilitySummary, ReleaseRuntimeReviewDiagnostics,
    ReleaseRuntimeSeparationSummary,
};

pub(crate) fn release_review_runtime_separation_takeaways(
    rows: &[ReleaseReviewRuntimeSeparationComparison],
) -> Vec<String> {
    let mut takeaways = Vec::new();
    for row in rows {
        if !matches!(row.horizon_days, 20 | 60) {
            continue;
        }
        match row.candidate_diagnosis.as_str() {
            "separated_but_below_runtime_floor" => {
                takeaways.push(format!(
                    "{}d: candidate 的 {} 已经和 normal 拉开，但 early-warning 平均概率 {} 仍低于 runtime floor {}（floor gap {}）。这更像阈值 / runtime policy 瓶颈，不是完全没有信号。",
                    row.horizon_days,
                    row.candidate_early_warning_regime,
                    crate::format_optional_pct(row.candidate_early_warning_avg_probability),
                    crate::format_optional_pct(row.candidate_threshold),
                    crate::format_optional_pct(row.candidate_floor_gap),
                ));
            }
            "usable_early_warning_separation" => {
                if row.baseline_diagnosis == "separated_but_below_runtime_floor" {
                    takeaways.push(format!(
                        "{}d: candidate 已把 early-warning 平均概率 {} 推过 runtime floor {}（floor gap {}），说明这一窗的主瓶颈已不再是 runtime threshold 本身。",
                        row.horizon_days,
                        crate::format_optional_pct(row.candidate_early_warning_avg_probability),
                        crate::format_optional_pct(row.candidate_threshold),
                        crate::format_optional_pct(row.candidate_floor_gap),
                    ));
                }
            }
            "cooldown_bleed" => {
                takeaways.push(format!(
                    "{}d: candidate 仍有 cooldown bleed，说明 post-crisis cooldown 段概率抬得过高，容易把危机后的背景值误当成提前预警。",
                    row.horizon_days
                ));
            }
            "late_only_no_early_warning" => {
                takeaways.push(format!(
                    "{}d: candidate 只有晚到信号，没有形成可用的 early-warning separation，当前还谈不上可执行提前量。",
                    row.horizon_days
                ));
            }
            _ => {}
        }
    }
    takeaways
}

pub(crate) fn build_release_runtime_review_diagnostics(
    release_id: &str,
    label_version: &str,
    method: &AuditMethodResponseWire,
    history: &[AssessmentHistoryPoint],
) -> ReleaseRuntimeReviewDiagnostics {
    let posture_distribution =
        summarize_named_counts(history.iter().map(|point| match point.posture {
            fc_domain::DecisionPosture::Normal => "normal",
            fc_domain::DecisionPosture::Prepare => "prepare",
            fc_domain::DecisionPosture::Hedge => "hedge",
            fc_domain::DecisionPosture::Defend => "defend",
        }));
    let time_bucket_distribution =
        summarize_named_counts(history.iter().map(|point| match point.time_to_risk_bucket {
            fc_domain::TimeToRiskBucket::Normal => "normal",
            fc_domain::TimeToRiskBucket::Months => "months",
            fc_domain::TimeToRiskBucket::Weeks => "weeks",
            fc_domain::TimeToRiskBucket::Now => "now",
        }));
    let posture_trigger_distribution =
        summarize_posture_clause_counts(history, |point| &point.posture_trigger_codes);
    let posture_blocker_distribution =
        summarize_posture_clause_counts(history, |point| &point.posture_blocker_codes);
    let (
        points_at_or_above_prepare_p60d,
        points_at_or_above_hedge_p20d,
        points_at_or_above_defend_p5d,
        mut notes,
    ) = if let Some(thresholds) = method.runtime_thresholds.as_ref() {
        (
            Some(
                history
                    .iter()
                    .filter(|point| point.p_60d >= thresholds.prepare_p60d)
                    .count(),
            ),
            Some(
                history
                    .iter()
                    .filter(|point| point.p_20d >= thresholds.hedge_p20d)
                    .count(),
            ),
            Some(
                history
                    .iter()
                    .filter(|point| point.p_5d >= thresholds.defend_p5d)
                    .count(),
            ),
            vec!["基于运行中 API 返回的 runtime_thresholds 统计历史概率越线次数。".to_string()],
        )
    } else {
        (
            None,
            None,
            None,
            vec![
                "运行中的 API 没有返回 runtime_thresholds；本报告只保留 posture / time bucket 分布。"
                    .to_string(),
            ],
        )
    };
    let regime_probability_summaries = match load_release_review_regime_scenarios(label_version) {
        Ok((scenarios, scenario_note)) => {
            notes.push(scenario_note);
            summarize_release_runtime_regime_probabilities(
                history,
                &scenarios,
                method.runtime_thresholds.as_ref(),
            )
        }
        Err(error) => {
            notes.push(format!(
                "未能加载 release review 所需的 regime scenario catalog，跳过 regime 概率分布：{error:#}"
            ));
            Vec::new()
        }
    };
    let regime_separation_summaries =
        summarize_release_runtime_regime_separation(&regime_probability_summaries);
    if !regime_separation_summaries.is_empty() {
        notes.push(render_release_runtime_separation_note(
            &regime_separation_summaries,
        ));
    }

    ReleaseRuntimeReviewDiagnostics {
        release_id: release_id.to_string(),
        history_point_count: history.len(),
        posture_distribution,
        time_bucket_distribution,
        posture_trigger_distribution,
        posture_blocker_distribution,
        regime_probability_summaries,
        regime_separation_summaries,
        runtime_thresholds: method.runtime_thresholds.clone(),
        points_at_or_above_prepare_p60d,
        points_at_or_above_hedge_p20d,
        points_at_or_above_defend_p5d,
        note: notes.join(" "),
    }
}

pub(crate) fn summarize_release_runtime_regime_probabilities(
    history: &[AssessmentHistoryPoint],
    scenarios: &[CrisisScenario],
    runtime_thresholds: Option<&RuntimeThresholdDiagnosticsWire>,
) -> Vec<ReleaseRuntimeRegimeProbabilitySummary> {
    #[derive(Default)]
    struct Accumulator {
        row_count: usize,
        raw_probability_sum: f64,
        max_raw_probability: f64,
        calibrated_probability_sum: f64,
        max_calibrated_probability: f64,
        threshold_hit_count: usize,
    }

    let mut buckets = BTreeMap::<(u32, String), Accumulator>::new();
    for point in history {
        for (horizon_days, raw_probability, calibrated_probability) in [
            (5_u32, point.raw_p_5d.unwrap_or(point.p_5d), point.p_5d),
            (20_u32, point.raw_p_20d.unwrap_or(point.p_20d), point.p_20d),
            (60_u32, point.raw_p_60d.unwrap_or(point.p_60d), point.p_60d),
        ] {
            let regime = probability_training_regime_name(forward_crisis_training_regime(
                point.as_of_date,
                scenarios,
                horizon_days,
            ));
            let bucket = buckets
                .entry((horizon_days, regime.to_string()))
                .or_default();
            bucket.row_count += 1;
            bucket.raw_probability_sum += raw_probability;
            bucket.max_raw_probability = bucket.max_raw_probability.max(raw_probability);
            bucket.calibrated_probability_sum += calibrated_probability;
            bucket.max_calibrated_probability = bucket
                .max_calibrated_probability
                .max(calibrated_probability);
            if let Some(threshold) =
                runtime_probability_threshold_for_horizon(runtime_thresholds, horizon_days)
            {
                if calibrated_probability >= threshold {
                    bucket.threshold_hit_count += 1;
                }
            }
        }
    }

    let normal_baselines = buckets
        .iter()
        .filter_map(|((horizon_days, regime), bucket)| {
            if regime != "normal" {
                return None;
            }
            Some((
                *horizon_days,
                (
                    safe_divide(bucket.raw_probability_sum, bucket.row_count as f64),
                    safe_divide(bucket.calibrated_probability_sum, bucket.row_count as f64),
                ),
            ))
        })
        .collect::<BTreeMap<_, _>>();

    buckets
        .into_iter()
        .map(|((horizon_days, regime), bucket)| {
            let avg_raw_probability =
                safe_divide(bucket.raw_probability_sum, bucket.row_count as f64);
            let avg_calibrated_probability =
                safe_divide(bucket.calibrated_probability_sum, bucket.row_count as f64);
            let (
                raw_lift_vs_normal,
                calibrated_lift_vs_normal,
                raw_gap_vs_normal,
                calibrated_gap_vs_normal,
                calibration_gap_retention,
            ) = if let Some((normal_avg_raw, normal_avg_calibrated)) =
                normal_baselines.get(&horizon_days).copied()
            {
                let raw_gap = avg_raw_probability - normal_avg_raw;
                let calibrated_gap = avg_calibrated_probability - normal_avg_calibrated;
                (
                    lift_vs_baseline(avg_raw_probability, normal_avg_raw),
                    lift_vs_baseline(avg_calibrated_probability, normal_avg_calibrated),
                    Some(round6(raw_gap)),
                    Some(round6(calibrated_gap)),
                    gap_retention_ratio(raw_gap, calibrated_gap),
                )
            } else {
                (None, None, None, None, None)
            };

            ReleaseRuntimeRegimeProbabilitySummary {
                horizon_days,
                regime,
                row_count: bucket.row_count,
                row_rate: round6(safe_ratio(bucket.row_count, history.len())),
                avg_raw_probability: round6(avg_raw_probability),
                max_raw_probability: round6(bucket.max_raw_probability),
                avg_probability: round6(avg_calibrated_probability),
                max_probability: round6(bucket.max_calibrated_probability),
                raw_lift_vs_normal,
                calibrated_lift_vs_normal,
                raw_gap_vs_normal,
                calibrated_gap_vs_normal,
                calibration_gap_retention,
                threshold_hit_count: runtime_thresholds.map(|_| bucket.threshold_hit_count),
            }
        })
        .collect()
}

pub(crate) fn summarize_release_runtime_regime_separation(
    summaries: &[ReleaseRuntimeRegimeProbabilitySummary],
) -> Vec<ReleaseRuntimeSeparationSummary> {
    let mut by_horizon = BTreeMap::<u32, Vec<&ReleaseRuntimeRegimeProbabilitySummary>>::new();
    for summary in summaries {
        by_horizon
            .entry(summary.horizon_days)
            .or_default()
            .push(summary);
    }

    by_horizon
        .into_iter()
        .filter_map(|(horizon_days, rows)| {
            let normal = rows.iter().copied().find(|row| row.regime == "normal")?;
            let pre_warning_buffer = rows
                .iter()
                .copied()
                .find(|row| row.regime == "pre_warning_buffer");
            let positive_window = rows
                .iter()
                .copied()
                .find(|row| row.regime == "positive_window");
            let max_non_normal = rows
                .iter()
                .copied()
                .filter(|row| row.regime != "normal")
                .max_by(|left, right| {
                    left.avg_probability
                        .partial_cmp(&right.avg_probability)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })?;
            let early_warning_regime_name = early_warning_regime_name(horizon_days);
            let early_warning = rows
                .iter()
                .copied()
                .find(|row| row.regime == early_warning_regime_name);
            let in_crisis = rows.iter().copied().find(|row| row.regime == "in_crisis");
            let post_crisis_cooldown = rows
                .iter()
                .copied()
                .find(|row| row.regime == "post_crisis_cooldown");
            let max_non_normal_threshold_hit_rate = max_non_normal
                .threshold_hit_count
                .map(|count| round6(safe_divide(count as f64, max_non_normal.row_count as f64)));
            let diagnosis = classify_regime_separation(
                horizon_days,
                early_warning
                    .and_then(|row| row.raw_lift_vs_normal)
                    .unwrap_or_default(),
                early_warning
                    .and_then(|row| row.calibrated_lift_vs_normal)
                    .unwrap_or_default(),
                early_warning.and_then(|row| row.calibration_gap_retention),
                positive_window
                    .and_then(|row| row.calibrated_lift_vs_normal)
                    .unwrap_or_default(),
                positive_window
                    .and_then(|row| row.calibrated_gap_vs_normal)
                    .unwrap_or_default(),
                in_crisis
                    .and_then(|row| row.calibrated_lift_vs_normal)
                    .unwrap_or_default(),
                post_crisis_cooldown
                    .and_then(|row| row.calibrated_lift_vs_normal)
                    .unwrap_or_default(),
                post_crisis_cooldown
                    .and_then(|row| row.calibrated_gap_vs_normal)
                    .unwrap_or_default(),
                max_non_normal.calibrated_lift_vs_normal.unwrap_or_default(),
                max_non_normal_threshold_hit_rate.unwrap_or_default(),
            )
            .to_string();

            Some(ReleaseRuntimeSeparationSummary {
                horizon_days,
                early_warning_regime: early_warning_regime_name.to_string(),
                normal_avg_probability: normal.avg_probability,
                pre_warning_buffer_avg_probability: pre_warning_buffer
                    .map(|row| row.avg_probability)
                    .unwrap_or_default(),
                positive_window_avg_probability: positive_window
                    .map(|row| row.avg_probability)
                    .unwrap_or_default(),
                in_crisis_avg_probability: in_crisis
                    .map(|row| row.avg_probability)
                    .unwrap_or_default(),
                post_crisis_cooldown_avg_probability: post_crisis_cooldown
                    .map(|row| row.avg_probability)
                    .unwrap_or_default(),
                early_warning_raw_lift_vs_normal: early_warning
                    .and_then(|row| row.raw_lift_vs_normal),
                early_warning_calibrated_lift_vs_normal: early_warning
                    .and_then(|row| row.calibrated_lift_vs_normal),
                early_warning_gap_retention: early_warning
                    .and_then(|row| row.calibration_gap_retention),
                positive_window_calibrated_lift_vs_normal: positive_window
                    .and_then(|row| row.calibrated_lift_vs_normal),
                positive_window_gap_vs_normal: positive_window
                    .and_then(|row| row.calibrated_gap_vs_normal),
                in_crisis_raw_lift_vs_normal: in_crisis.and_then(|row| row.raw_lift_vs_normal),
                in_crisis_calibrated_lift_vs_normal: in_crisis
                    .and_then(|row| row.calibrated_lift_vs_normal),
                post_crisis_cooldown_calibrated_lift_vs_normal: post_crisis_cooldown
                    .and_then(|row| row.calibrated_lift_vs_normal),
                post_crisis_cooldown_gap_vs_normal: post_crisis_cooldown
                    .and_then(|row| row.calibrated_gap_vs_normal),
                max_non_normal_calibrated_lift_vs_normal: max_non_normal.calibrated_lift_vs_normal,
                max_non_normal_threshold_hit_rate,
                diagnosis,
            })
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn classify_regime_separation(
    horizon_days: u32,
    early_warning_raw_lift: f64,
    early_warning_calibrated_lift: f64,
    early_warning_gap_retention: Option<f64>,
    positive_window_calibrated_lift: f64,
    positive_window_gap_vs_normal: f64,
    in_crisis_calibrated_lift: f64,
    post_crisis_cooldown_calibrated_lift: f64,
    post_crisis_cooldown_gap_vs_normal: f64,
    max_non_normal_calibrated_lift: f64,
    max_non_normal_threshold_hit_rate: f64,
) -> &'static str {
    if max_non_normal_calibrated_lift < 1.15
        && early_warning_raw_lift < 1.15
        && positive_window_calibrated_lift < 1.15
    {
        return "cold_across_all_regimes";
    }
    if early_warning_raw_lift >= 1.5
        && early_warning_calibrated_lift < 1.15
        && early_warning_gap_retention.unwrap_or_default() < 0.35
    {
        return "calibration_crushed_early_warning";
    }
    if positive_window_calibrated_lift < 1.15 && in_crisis_calibrated_lift >= 1.5 {
        return "late_only_no_early_warning";
    }
    if positive_window_calibrated_lift >= 1.15
        && post_crisis_cooldown_calibrated_lift >= positive_window_calibrated_lift
        && post_crisis_cooldown_gap_vs_normal + 0.002 >= positive_window_gap_vs_normal
    {
        return "cooldown_bleed";
    }
    if max_non_normal_calibrated_lift >= 1.5 && max_non_normal_threshold_hit_rate <= 0.01 {
        return "separated_but_below_runtime_floor";
    }
    if positive_window_calibrated_lift >= 1.5
        && positive_window_gap_vs_normal >= regime_positive_window_gap_floor(horizon_days)
    {
        return "usable_early_warning_separation";
    }
    if max_non_normal_calibrated_lift >= 1.15 || early_warning_calibrated_lift >= 1.15 {
        return "weak_regime_separation";
    }
    "mixed_or_unclear"
}

pub(crate) fn lift_vs_baseline(value: f64, baseline: f64) -> Option<f64> {
    if baseline.abs() <= f64::EPSILON {
        return None;
    }
    Some(round6(value / baseline))
}

fn load_release_review_regime_scenarios(
    label_version: &str,
) -> Result<(Vec<CrisisScenario>, String)> {
    match load_label_set_crisis_scenarios(DEFAULT_FORMAL_SCENARIO_SET_VERSION, label_version) {
        Ok(scenarios) => Ok((
            scenarios,
            format!(
                "Regime 概率分布基于 {DEFAULT_FORMAL_SCENARIO_SET_VERSION}/{label_version} 重算。"
            ),
        )),
        Err(primary_error) if label_version == "label_forward_crisis_v1" => {
            let fallback = load_label_set_crisis_scenarios(
                DEFAULT_FORMAL_SCENARIO_SET_VERSION,
                DEFAULT_FORMAL_LABEL_VERSION,
            )?;
            Ok((
                fallback,
                format!(
                    "当前 release label_version={label_version} 不在 scenario catalog 中，Regime 概率分布回退到 {DEFAULT_FORMAL_SCENARIO_SET_VERSION}/{DEFAULT_FORMAL_LABEL_VERSION} 重算（原始错误：{primary_error:#}）。"
                ),
            ))
        }
        Err(error) => Err(error),
    }
}

fn early_warning_regime_name(horizon_days: u32) -> &'static str {
    match horizon_days {
        5 => "positive_window",
        20 | 60 => "pre_warning_buffer",
        _ => "positive_window",
    }
}

fn render_release_runtime_separation_note(summaries: &[ReleaseRuntimeSeparationSummary]) -> String {
    let joined = summaries
        .iter()
        .map(|summary| format!("{}d={}", summary.horizon_days, summary.diagnosis))
        .collect::<Vec<_>>()
        .join(", ");
    format!("Runtime separation summary: {joined}.")
}

fn gap_retention_ratio(raw_gap: f64, calibrated_gap: f64) -> Option<f64> {
    if raw_gap.abs() <= f64::EPSILON {
        return None;
    }
    Some(round6(calibrated_gap / raw_gap))
}

fn runtime_probability_threshold_for_horizon(
    runtime_thresholds: Option<&RuntimeThresholdDiagnosticsWire>,
    horizon_days: u32,
) -> Option<f64> {
    runtime_thresholds.map(|thresholds| match horizon_days {
        5 => thresholds.defend_p5d,
        20 => thresholds.hedge_p20d,
        60 => thresholds.prepare_p60d,
        _ => 1.0,
    })
}

fn summarize_named_counts<'a>(names: impl Iterator<Item = &'a str>) -> Vec<ReleaseRuntimeCount> {
    let mut counts = BTreeMap::<String, usize>::new();
    for name in names {
        *counts.entry(name.to_string()).or_default() += 1;
    }
    let mut rows = counts
        .into_iter()
        .map(|(name, count)| ReleaseRuntimeCount { name, count })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.name.cmp(&right.name))
    });
    rows
}

fn summarize_posture_clause_counts<F>(
    history: &[AssessmentHistoryPoint],
    accessor: F,
) -> Vec<ReleaseRuntimeClauseCount>
where
    F: Fn(&AssessmentHistoryPoint) -> &[String],
{
    let posture_totals = history
        .iter()
        .fold(BTreeMap::<String, usize>::new(), |mut acc, point| {
            *acc.entry(runtime_posture_name(point).to_string())
                .or_default() += 1;
            acc
        });
    let mut counts = BTreeMap::<(String, String), usize>::new();
    for point in history {
        let posture = runtime_posture_name(point).to_string();
        for clause in accessor(point) {
            *counts.entry((posture.clone(), clause.clone())).or_default() += 1;
        }
    }

    let mut rows = counts
        .into_iter()
        .map(|((posture, clause), count)| {
            let posture_total = posture_totals.get(&posture).copied().unwrap_or_default();
            ReleaseRuntimeClauseCount {
                posture,
                clause,
                count,
                share_of_posture: round6(safe_ratio(count, posture_total)),
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.posture.cmp(&right.posture))
            .then_with(|| left.clause.cmp(&right.clause))
    });
    rows
}

fn runtime_posture_name(point: &AssessmentHistoryPoint) -> &'static str {
    match point.posture {
        fc_domain::DecisionPosture::Normal => "normal",
        fc_domain::DecisionPosture::Prepare => "prepare",
        fc_domain::DecisionPosture::Hedge => "hedge",
        fc_domain::DecisionPosture::Defend => "defend",
    }
}

use std::collections::BTreeMap;

use anyhow::Result;
use fc_domain::AssessmentHistoryPoint;

use crate::{
    load_label_set_crisis_scenarios, round6, safe_ratio, AuditMethodResponseWire, CrisisScenario,
    DEFAULT_FORMAL_LABEL_VERSION, DEFAULT_FORMAL_SCENARIO_SET_VERSION,
};

use super::{
    super::{
        ReleaseRuntimeClauseCount, ReleaseRuntimeCount, ReleaseRuntimeReviewDiagnostics,
        ReleaseRuntimeSeparationSummary,
    },
    regimes::{
        summarize_release_runtime_regime_probabilities, summarize_release_runtime_regime_separation,
    },
};

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

fn render_release_runtime_separation_note(summaries: &[ReleaseRuntimeSeparationSummary]) -> String {
    let joined = summaries
        .iter()
        .map(|summary| format!("{}d={}", summary.horizon_days, summary.diagnosis))
        .collect::<Vec<_>>()
        .join(", ");
    format!("Runtime separation summary: {joined}.")
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

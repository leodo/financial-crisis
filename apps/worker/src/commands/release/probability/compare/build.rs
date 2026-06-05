use std::collections::{BTreeMap, BTreeSet};

use anyhow::{bail, Context};
use chrono::Utc;
use fc_domain::LogisticProbabilityModelScoreDiagnostics;

use super::super::formal::{
    release_formal_probability_base_model, release_formal_probability_horizon,
};
use super::{
    ReleaseFormalProbabilityCompareBuildInput, ReleaseFormalProbabilityCompareExport,
    ReleaseFormalProbabilityComparePoint, ReleaseFormalProbabilityCompareSummary,
    ReleaseFormalProbabilityFeatureDelta, ReleaseFormalProbabilityFeatureDeltaAggregate,
    ReleaseFormalProbabilityThresholdSummary, ReleaseFormalProbabilityWindowAggregateSummary,
};

pub(in super::super) fn build_release_formal_probability_compare_export(
    input: ReleaseFormalProbabilityCompareBuildInput<'_>,
) -> anyhow::Result<ReleaseFormalProbabilityCompareExport> {
    let ReleaseFormalProbabilityCompareBuildInput {
        market_scope,
        dataset_key,
        scenario_id,
        from_date,
        to_date,
        baseline_release_id,
        candidate_release_id,
        baseline_bundle,
        candidate_bundle,
        baseline_rows,
        candidate_rows,
    } = input;
    let baseline_thresholds = release_formal_probability_threshold_summaries(baseline_bundle);
    let candidate_thresholds = release_formal_probability_threshold_summaries(candidate_bundle);
    let baseline_threshold_20d = release_formal_probability_threshold(baseline_bundle, 20);
    let candidate_threshold_20d = release_formal_probability_threshold(candidate_bundle, 20);
    let baseline_threshold_60d = release_formal_probability_threshold(baseline_bundle, 60);
    let candidate_threshold_60d = release_formal_probability_threshold(candidate_bundle, 60);
    let candidate_by_date = candidate_rows
        .into_iter()
        .map(|row| (row.as_of_date, row))
        .collect::<BTreeMap<_, _>>();
    let mut rows = Vec::new();
    let mut baseline_hit_count_20d = 0_usize;
    let mut candidate_hit_count_20d = 0_usize;
    let mut baseline_hit_count_60d = 0_usize;
    let mut candidate_hit_count_60d = 0_usize;
    let mut baseline_max_p_20d = f64::NEG_INFINITY;
    let mut baseline_max_p_20d_date = None;
    let mut candidate_max_p_20d = f64::NEG_INFINITY;
    let mut candidate_max_p_20d_date = None;
    let mut baseline_max_p_60d = f64::NEG_INFINITY;
    let mut baseline_max_p_60d_date = None;
    let mut candidate_max_p_60d = f64::NEG_INFINITY;
    let mut candidate_max_p_60d_date = None;

    for baseline_row in baseline_rows {
        let Some(candidate_row) = candidate_by_date.get(&baseline_row.as_of_date) else {
            continue;
        };
        let baseline_horizon_20d = release_formal_probability_horizon(&baseline_row, 20)
            .with_context(|| "baseline slice missing 20d diagnostics")?;
        let candidate_horizon_20d = release_formal_probability_horizon(candidate_row, 20)
            .with_context(|| "candidate slice missing 20d diagnostics")?;
        let baseline_horizon_60d = release_formal_probability_horizon(&baseline_row, 60)
            .with_context(|| "baseline slice missing 60d diagnostics")?;
        let candidate_horizon_60d = release_formal_probability_horizon(candidate_row, 60)
            .with_context(|| "candidate slice missing 60d diagnostics")?;
        let baseline_base_20d = release_formal_probability_base_model(&baseline_row, 20)
            .with_context(|| "baseline slice missing 20d base diagnostics")?;
        let candidate_base_20d = release_formal_probability_base_model(candidate_row, 20)
            .with_context(|| "candidate slice missing 20d base diagnostics")?;
        let baseline_base_60d = release_formal_probability_base_model(&baseline_row, 60)
            .with_context(|| "baseline slice missing 60d base diagnostics")?;
        let candidate_base_60d = release_formal_probability_base_model(candidate_row, 60)
            .with_context(|| "candidate slice missing 60d base diagnostics")?;

        let baseline_hit_20d = baseline_threshold_20d
            .map(|threshold| baseline_horizon_20d.final_probability >= threshold)
            .unwrap_or(false);
        let candidate_hit_20d = candidate_threshold_20d
            .map(|threshold| candidate_horizon_20d.final_probability >= threshold)
            .unwrap_or(false);
        let baseline_hit_60d = baseline_threshold_60d
            .map(|threshold| baseline_horizon_60d.final_probability >= threshold)
            .unwrap_or(false);
        let candidate_hit_60d = candidate_threshold_60d
            .map(|threshold| candidate_horizon_60d.final_probability >= threshold)
            .unwrap_or(false);

        baseline_hit_count_20d += usize::from(baseline_hit_20d);
        candidate_hit_count_20d += usize::from(candidate_hit_20d);
        baseline_hit_count_60d += usize::from(baseline_hit_60d);
        candidate_hit_count_60d += usize::from(candidate_hit_60d);

        if baseline_horizon_20d.final_probability > baseline_max_p_20d {
            baseline_max_p_20d = baseline_horizon_20d.final_probability;
            baseline_max_p_20d_date = Some(baseline_row.as_of_date);
        }
        if candidate_horizon_20d.final_probability > candidate_max_p_20d {
            candidate_max_p_20d = candidate_horizon_20d.final_probability;
            candidate_max_p_20d_date = Some(candidate_row.as_of_date);
        }
        if baseline_horizon_60d.final_probability > baseline_max_p_60d {
            baseline_max_p_60d = baseline_horizon_60d.final_probability;
            baseline_max_p_60d_date = Some(baseline_row.as_of_date);
        }
        if candidate_horizon_60d.final_probability > candidate_max_p_60d {
            candidate_max_p_60d = candidate_horizon_60d.final_probability;
            candidate_max_p_60d_date = Some(candidate_row.as_of_date);
        }

        rows.push(ReleaseFormalProbabilityComparePoint {
            as_of_date: baseline_row.as_of_date,
            split_name: baseline_row.split_name.clone(),
            primary_scenario_id: baseline_row.primary_scenario_id.clone(),
            scenario_family: baseline_row.scenario_family.clone(),
            regime_20d: baseline_row.regime_20d.clone(),
            regime_60d: baseline_row.regime_60d.clone(),
            prepare_episode_label: baseline_row.prepare_episode_label,
            hedge_episode_label: baseline_row.hedge_episode_label,
            defend_episode_label: baseline_row.defend_episode_label,
            primary_action_level: baseline_row.primary_action_level.clone(),
            coverage_score: baseline_row.coverage_score,
            baseline_raw_p_20d: baseline_horizon_20d.raw_probability,
            candidate_raw_p_20d: candidate_horizon_20d.raw_probability,
            baseline_base_linear_20d: baseline_base_20d.base_model.linear_score,
            candidate_base_linear_20d: candidate_base_20d.base_model.linear_score,
            baseline_final_p_20d: baseline_horizon_20d.final_probability,
            candidate_final_p_20d: candidate_horizon_20d.final_probability,
            delta_final_p_20d: candidate_horizon_20d.final_probability
                - baseline_horizon_20d.final_probability,
            baseline_hit_20d,
            candidate_hit_20d,
            baseline_raw_p_60d: baseline_horizon_60d.raw_probability,
            candidate_raw_p_60d: candidate_horizon_60d.raw_probability,
            baseline_base_linear_60d: baseline_base_60d.base_model.linear_score,
            candidate_base_linear_60d: candidate_base_60d.base_model.linear_score,
            baseline_final_p_60d: baseline_horizon_60d.final_probability,
            candidate_final_p_60d: candidate_horizon_60d.final_probability,
            delta_final_p_60d: candidate_horizon_60d.final_probability
                - baseline_horizon_60d.final_probability,
            baseline_hit_60d,
            candidate_hit_60d,
            top_feature_deltas_20d: release_formal_probability_feature_deltas(
                &baseline_base_20d.base_model,
                &candidate_base_20d.base_model,
                8,
            ),
            top_feature_deltas_60d: release_formal_probability_feature_deltas(
                &baseline_base_60d.base_model,
                &candidate_base_60d.base_model,
                8,
            ),
        });
    }

    if rows.is_empty() {
        bail!(
            "no overlapping rows found between baseline {baseline_release_id} and candidate {candidate_release_id} in the selected window"
        );
    }

    let overall_window = build_release_formal_probability_window_aggregate_summary(&rows, |_| true);
    let hedge_window = build_release_formal_probability_window_aggregate_summary(&rows, |row| {
        row.hedge_episode_label == 1
    });
    let positive_window_20d =
        build_release_formal_probability_window_aggregate_summary(&rows, |row| {
            row.regime_20d == "positive_window"
        });

    Ok(ReleaseFormalProbabilityCompareExport {
        exported_at: Utc::now().to_rfc3339(),
        market_scope: market_scope.to_string(),
        baseline_release_id: baseline_release_id.to_string(),
        candidate_release_id: candidate_release_id.to_string(),
        dataset_key: dataset_key.to_string(),
        scenario_id,
        from_date,
        to_date,
        row_count: rows.len(),
        baseline_thresholds,
        candidate_thresholds,
        summary: ReleaseFormalProbabilityCompareSummary {
            baseline_hit_count_20d,
            candidate_hit_count_20d,
            baseline_hit_count_60d,
            candidate_hit_count_60d,
            baseline_max_p_20d: baseline_max_p_20d.max(0.0),
            baseline_max_p_20d_date,
            candidate_max_p_20d: candidate_max_p_20d.max(0.0),
            candidate_max_p_20d_date,
            baseline_max_p_60d: baseline_max_p_60d.max(0.0),
            baseline_max_p_60d_date,
            candidate_max_p_60d: candidate_max_p_60d.max(0.0),
            candidate_max_p_60d_date,
            overall_window,
            hedge_window,
            positive_window_20d,
        },
        rows,
    })
}

fn release_formal_probability_threshold_summaries(
    bundle: &fc_domain::ProbabilityBundle,
) -> Vec<ReleaseFormalProbabilityThresholdSummary> {
    bundle
        .horizons
        .iter()
        .map(|horizon| ReleaseFormalProbabilityThresholdSummary {
            horizon_days: horizon.horizon_days,
            decision_threshold: horizon.decision_threshold,
            overlay_count: horizon.family_overlays.len(),
        })
        .collect()
}

fn release_formal_probability_threshold(
    bundle: &fc_domain::ProbabilityBundle,
    horizon_days: u32,
) -> Option<f64> {
    bundle
        .horizons
        .iter()
        .find(|horizon| horizon.horizon_days == horizon_days)
        .and_then(|horizon| horizon.decision_threshold)
}

fn release_formal_probability_feature_deltas(
    baseline: &LogisticProbabilityModelScoreDiagnostics,
    candidate: &LogisticProbabilityModelScoreDiagnostics,
    limit: usize,
) -> Vec<ReleaseFormalProbabilityFeatureDelta> {
    let baseline_by_name = baseline
        .feature_contributions
        .iter()
        .map(|item| (item.name.clone(), item.clone()))
        .collect::<BTreeMap<_, _>>();
    let candidate_by_name = candidate
        .feature_contributions
        .iter()
        .map(|item| (item.name.clone(), item.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut names = baseline_by_name.keys().cloned().collect::<BTreeSet<_>>();
    names.extend(candidate_by_name.keys().cloned());
    let mut deltas = names
        .into_iter()
        .map(|name| {
            let baseline_item = baseline_by_name.get(&name);
            let candidate_item = candidate_by_name.get(&name);
            let baseline_raw_value = baseline_item.map(|item| item.raw_value).unwrap_or(0.0);
            let candidate_raw_value = candidate_item.map(|item| item.raw_value).unwrap_or(0.0);
            let baseline_normalized_value = baseline_item
                .map(|item| item.normalized_value)
                .unwrap_or(0.0);
            let candidate_normalized_value = candidate_item
                .map(|item| item.normalized_value)
                .unwrap_or(0.0);
            let baseline_weight = baseline_item.map(|item| item.weight).unwrap_or(0.0);
            let candidate_weight = candidate_item.map(|item| item.weight).unwrap_or(0.0);
            let baseline_contribution = baseline_item.map(|item| item.contribution).unwrap_or(0.0);
            let candidate_contribution =
                candidate_item.map(|item| item.contribution).unwrap_or(0.0);
            ReleaseFormalProbabilityFeatureDelta {
                name,
                baseline_raw_value,
                candidate_raw_value,
                baseline_normalized_value,
                candidate_normalized_value,
                baseline_weight,
                candidate_weight,
                baseline_contribution,
                candidate_contribution,
                delta_contribution: candidate_contribution - baseline_contribution,
            }
        })
        .filter(|item| item.delta_contribution.abs() >= 1e-9)
        .collect::<Vec<_>>();
    deltas.sort_by(|left, right| {
        right
            .delta_contribution
            .abs()
            .total_cmp(&left.delta_contribution.abs())
    });
    deltas.truncate(limit);
    deltas
}

fn build_release_formal_probability_window_aggregate_summary<F>(
    rows: &[ReleaseFormalProbabilityComparePoint],
    filter: F,
) -> ReleaseFormalProbabilityWindowAggregateSummary
where
    F: Fn(&ReleaseFormalProbabilityComparePoint) -> bool,
{
    let selected = rows.iter().filter(|row| filter(row)).collect::<Vec<_>>();
    if selected.is_empty() {
        return ReleaseFormalProbabilityWindowAggregateSummary {
            row_count: 0,
            avg_delta_p_20d: 0.0,
            avg_abs_delta_p_20d: 0.0,
            avg_delta_p_60d: 0.0,
            avg_abs_delta_p_60d: 0.0,
            baseline_hit_rate_20d: 0.0,
            candidate_hit_rate_20d: 0.0,
            baseline_hit_rate_60d: 0.0,
            candidate_hit_rate_60d: 0.0,
            top_feature_deltas_20d: Vec::new(),
            top_feature_deltas_60d: Vec::new(),
        };
    }

    let row_count = selected.len();
    let avg_delta_p_20d = selected
        .iter()
        .map(|row| row.delta_final_p_20d)
        .sum::<f64>()
        / row_count as f64;
    let avg_abs_delta_p_20d = selected
        .iter()
        .map(|row| row.delta_final_p_20d.abs())
        .sum::<f64>()
        / row_count as f64;
    let avg_delta_p_60d = selected
        .iter()
        .map(|row| row.delta_final_p_60d)
        .sum::<f64>()
        / row_count as f64;
    let avg_abs_delta_p_60d = selected
        .iter()
        .map(|row| row.delta_final_p_60d.abs())
        .sum::<f64>()
        / row_count as f64;
    let baseline_hit_rate_20d =
        selected.iter().filter(|row| row.baseline_hit_20d).count() as f64 / row_count as f64;
    let candidate_hit_rate_20d =
        selected.iter().filter(|row| row.candidate_hit_20d).count() as f64 / row_count as f64;
    let baseline_hit_rate_60d =
        selected.iter().filter(|row| row.baseline_hit_60d).count() as f64 / row_count as f64;
    let candidate_hit_rate_60d =
        selected.iter().filter(|row| row.candidate_hit_60d).count() as f64 / row_count as f64;

    ReleaseFormalProbabilityWindowAggregateSummary {
        row_count,
        avg_delta_p_20d,
        avg_abs_delta_p_20d,
        avg_delta_p_60d,
        avg_abs_delta_p_60d,
        baseline_hit_rate_20d,
        candidate_hit_rate_20d,
        baseline_hit_rate_60d,
        candidate_hit_rate_60d,
        top_feature_deltas_20d: aggregate_release_formal_probability_feature_deltas(
            selected
                .iter()
                .map(|row| row.top_feature_deltas_20d.as_slice()),
            10,
        ),
        top_feature_deltas_60d: aggregate_release_formal_probability_feature_deltas(
            selected
                .iter()
                .map(|row| row.top_feature_deltas_60d.as_slice()),
            10,
        ),
    }
}

fn aggregate_release_formal_probability_feature_deltas<'a, I>(
    feature_sets: I,
    limit: usize,
) -> Vec<ReleaseFormalProbabilityFeatureDeltaAggregate>
where
    I: IntoIterator<Item = &'a [ReleaseFormalProbabilityFeatureDelta]>,
{
    let mut aggregates = BTreeMap::<String, (f64, f64, usize)>::new();
    for feature_set in feature_sets {
        for item in feature_set {
            let entry = aggregates
                .entry(item.name.clone())
                .or_insert((0.0_f64, 0.0_f64, 0_usize));
            entry.0 += item.delta_contribution;
            entry.1 += item.delta_contribution.abs();
            entry.2 += 1;
        }
    }
    let mut rows = aggregates
        .into_iter()
        .map(
            |(name, (sum_delta_contribution, abs_sum_delta_contribution, count))| {
                ReleaseFormalProbabilityFeatureDeltaAggregate {
                    name,
                    sum_delta_contribution,
                    abs_sum_delta_contribution,
                    mean_delta_contribution: sum_delta_contribution / count as f64,
                    count,
                }
            },
        )
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .abs_sum_delta_contribution
            .total_cmp(&left.abs_sum_delta_contribution)
    });
    rows.truncate(limit);
    rows
}

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
};

use anyhow::{bail, Context};
use chrono::{NaiveDate, Utc};
use fc_domain::LogisticProbabilityModelScoreDiagnostics;
use serde::Serialize;

use super::{
    common::{release_probability_csv_escape, sanitize_release_probability_slice_component},
    formal::{
        release_formal_probability_base_model, release_formal_probability_horizon,
        ReleaseFormalProbabilitySlicePoint,
    },
};

#[derive(Debug, Clone, Serialize)]
pub(super) struct ReleaseFormalProbabilityCompareExport {
    exported_at: String,
    market_scope: String,
    baseline_release_id: String,
    candidate_release_id: String,
    dataset_key: String,
    scenario_id: Option<String>,
    from_date: NaiveDate,
    to_date: NaiveDate,
    row_count: usize,
    baseline_thresholds: Vec<ReleaseFormalProbabilityThresholdSummary>,
    candidate_thresholds: Vec<ReleaseFormalProbabilityThresholdSummary>,
    summary: ReleaseFormalProbabilityCompareSummary,
    rows: Vec<ReleaseFormalProbabilityComparePoint>,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseFormalProbabilityThresholdSummary {
    horizon_days: u32,
    decision_threshold: Option<f64>,
    overlay_count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseFormalProbabilityCompareSummary {
    baseline_hit_count_20d: usize,
    candidate_hit_count_20d: usize,
    baseline_hit_count_60d: usize,
    candidate_hit_count_60d: usize,
    baseline_max_p_20d: f64,
    baseline_max_p_20d_date: Option<NaiveDate>,
    candidate_max_p_20d: f64,
    candidate_max_p_20d_date: Option<NaiveDate>,
    baseline_max_p_60d: f64,
    baseline_max_p_60d_date: Option<NaiveDate>,
    candidate_max_p_60d: f64,
    candidate_max_p_60d_date: Option<NaiveDate>,
    overall_window: ReleaseFormalProbabilityWindowAggregateSummary,
    hedge_window: ReleaseFormalProbabilityWindowAggregateSummary,
    positive_window_20d: ReleaseFormalProbabilityWindowAggregateSummary,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseFormalProbabilityWindowAggregateSummary {
    row_count: usize,
    avg_delta_p_20d: f64,
    avg_abs_delta_p_20d: f64,
    avg_delta_p_60d: f64,
    avg_abs_delta_p_60d: f64,
    baseline_hit_rate_20d: f64,
    candidate_hit_rate_20d: f64,
    baseline_hit_rate_60d: f64,
    candidate_hit_rate_60d: f64,
    top_feature_deltas_20d: Vec<ReleaseFormalProbabilityFeatureDeltaAggregate>,
    top_feature_deltas_60d: Vec<ReleaseFormalProbabilityFeatureDeltaAggregate>,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseFormalProbabilityFeatureDeltaAggregate {
    name: String,
    sum_delta_contribution: f64,
    abs_sum_delta_contribution: f64,
    mean_delta_contribution: f64,
    count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseFormalProbabilityComparePoint {
    as_of_date: NaiveDate,
    split_name: String,
    primary_scenario_id: Option<String>,
    scenario_family: Option<String>,
    regime_20d: String,
    regime_60d: String,
    prepare_episode_label: u8,
    hedge_episode_label: u8,
    defend_episode_label: u8,
    primary_action_level: Option<String>,
    coverage_score: f64,
    baseline_raw_p_20d: f64,
    candidate_raw_p_20d: f64,
    baseline_base_linear_20d: f64,
    candidate_base_linear_20d: f64,
    baseline_final_p_20d: f64,
    candidate_final_p_20d: f64,
    delta_final_p_20d: f64,
    baseline_hit_20d: bool,
    candidate_hit_20d: bool,
    baseline_raw_p_60d: f64,
    candidate_raw_p_60d: f64,
    baseline_base_linear_60d: f64,
    candidate_base_linear_60d: f64,
    baseline_final_p_60d: f64,
    candidate_final_p_60d: f64,
    delta_final_p_60d: f64,
    baseline_hit_60d: bool,
    candidate_hit_60d: bool,
    top_feature_deltas_20d: Vec<ReleaseFormalProbabilityFeatureDelta>,
    top_feature_deltas_60d: Vec<ReleaseFormalProbabilityFeatureDelta>,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseFormalProbabilityFeatureDelta {
    name: String,
    baseline_raw_value: f64,
    candidate_raw_value: f64,
    baseline_normalized_value: f64,
    candidate_normalized_value: f64,
    baseline_weight: f64,
    candidate_weight: f64,
    baseline_contribution: f64,
    candidate_contribution: f64,
    delta_contribution: f64,
}

pub(super) struct ReleaseFormalProbabilityCompareBuildInput<'a> {
    pub(super) market_scope: &'a str,
    pub(super) dataset_key: &'a str,
    pub(super) scenario_id: Option<String>,
    pub(super) from_date: NaiveDate,
    pub(super) to_date: NaiveDate,
    pub(super) baseline_release_id: &'a str,
    pub(super) candidate_release_id: &'a str,
    pub(super) baseline_bundle: &'a fc_domain::ProbabilityBundle,
    pub(super) candidate_bundle: &'a fc_domain::ProbabilityBundle,
    pub(super) baseline_rows: Vec<ReleaseFormalProbabilitySlicePoint>,
    pub(super) candidate_rows: Vec<ReleaseFormalProbabilitySlicePoint>,
}

pub(super) fn build_release_formal_probability_compare_export(
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

pub(super) fn write_release_formal_probability_compare_report(
    output_dir: &PathBuf,
    export: &ReleaseFormalProbabilityCompareExport,
) -> anyhow::Result<()> {
    fs::create_dir_all(output_dir)?;
    let mut stem = format!(
        "{}-vs-{}-{}-{}-formal-probability-compare",
        sanitize_release_probability_slice_component(&export.baseline_release_id),
        sanitize_release_probability_slice_component(&export.candidate_release_id),
        export.from_date,
        export.to_date
    );
    if let Some(scenario_id) = export.scenario_id.as_deref() {
        stem.push('-');
        stem.push_str(&sanitize_release_probability_slice_component(scenario_id));
    }
    let json_path = output_dir.join(format!("{stem}.json"));
    let csv_path = output_dir.join(format!("{stem}.csv"));
    fs::write(&json_path, serde_json::to_string_pretty(export)?)?;
    fs::write(
        &csv_path,
        render_release_formal_probability_compare_csv(export)?,
    )?;
    println!("Release formal probability compare exported.");
    println!("  JSON {}", json_path.display());
    println!("  CSV  {}", csv_path.display());
    Ok(())
}

fn render_release_formal_probability_compare_csv(
    export: &ReleaseFormalProbabilityCompareExport,
) -> anyhow::Result<String> {
    let mut csv = String::from(
        "as_of_date,split_name,primary_scenario_id,scenario_family,regime_20d,regime_60d,prepare_episode_label,hedge_episode_label,defend_episode_label,primary_action_level,coverage_score,baseline_raw_p_20d,candidate_raw_p_20d,baseline_base_linear_20d,candidate_base_linear_20d,baseline_final_p_20d,candidate_final_p_20d,delta_final_p_20d,baseline_hit_20d,candidate_hit_20d,top_feature_deltas_20d_json,baseline_raw_p_60d,candidate_raw_p_60d,baseline_base_linear_60d,candidate_base_linear_60d,baseline_final_p_60d,candidate_final_p_60d,delta_final_p_60d,baseline_hit_60d,candidate_hit_60d,top_feature_deltas_60d_json\n",
    );
    for row in &export.rows {
        let columns = [
            row.as_of_date.to_string(),
            row.split_name.clone(),
            row.primary_scenario_id.clone().unwrap_or_default(),
            row.scenario_family.clone().unwrap_or_default(),
            row.regime_20d.clone(),
            row.regime_60d.clone(),
            row.prepare_episode_label.to_string(),
            row.hedge_episode_label.to_string(),
            row.defend_episode_label.to_string(),
            row.primary_action_level.clone().unwrap_or_default(),
            format!("{:.6}", row.coverage_score),
            format!("{:.6}", row.baseline_raw_p_20d),
            format!("{:.6}", row.candidate_raw_p_20d),
            format!("{:.6}", row.baseline_base_linear_20d),
            format!("{:.6}", row.candidate_base_linear_20d),
            format!("{:.6}", row.baseline_final_p_20d),
            format!("{:.6}", row.candidate_final_p_20d),
            format!("{:.6}", row.delta_final_p_20d),
            row.baseline_hit_20d.to_string(),
            row.candidate_hit_20d.to_string(),
            serde_json::to_string(&row.top_feature_deltas_20d)?,
            format!("{:.6}", row.baseline_raw_p_60d),
            format!("{:.6}", row.candidate_raw_p_60d),
            format!("{:.6}", row.baseline_base_linear_60d),
            format!("{:.6}", row.candidate_base_linear_60d),
            format!("{:.6}", row.baseline_final_p_60d),
            format!("{:.6}", row.candidate_final_p_60d),
            format!("{:.6}", row.delta_final_p_60d),
            row.baseline_hit_60d.to_string(),
            row.candidate_hit_60d.to_string(),
            serde_json::to_string(&row.top_feature_deltas_60d)?,
        ];
        csv.push_str(
            &columns
                .into_iter()
                .map(|value| release_probability_csv_escape(&value))
                .collect::<Vec<_>>()
                .join(","),
        );
        csv.push('\n');
    }
    Ok(csv)
}

pub(super) fn print_release_formal_probability_compare_summary(
    export: &ReleaseFormalProbabilityCompareExport,
) {
    let first_date = export
        .rows
        .first()
        .map(|row| row.as_of_date.to_string())
        .unwrap_or_else(|| "-".to_string());
    let last_date = export
        .rows
        .last()
        .map(|row| row.as_of_date.to_string())
        .unwrap_or_else(|| "-".to_string());
    println!(
        "Release formal probability compare baseline={} candidate={} rows={} range={} -> {} scenario={}",
        export.baseline_release_id,
        export.candidate_release_id,
        export.row_count,
        first_date,
        last_date,
        export.scenario_id.as_deref().unwrap_or("all"),
    );
    println!(
        "  20d hits baseline={} candidate={} max baseline={:.3} ({}) candidate={:.3} ({})",
        export.summary.baseline_hit_count_20d,
        export.summary.candidate_hit_count_20d,
        export.summary.baseline_max_p_20d,
        export
            .summary
            .baseline_max_p_20d_date
            .map(|date| date.to_string())
            .unwrap_or_else(|| "-".to_string()),
        export.summary.candidate_max_p_20d,
        export
            .summary
            .candidate_max_p_20d_date
            .map(|date| date.to_string())
            .unwrap_or_else(|| "-".to_string()),
    );
    println!(
        "  60d hits baseline={} candidate={} max baseline={:.3} ({}) candidate={:.3} ({})",
        export.summary.baseline_hit_count_60d,
        export.summary.candidate_hit_count_60d,
        export.summary.baseline_max_p_60d,
        export
            .summary
            .baseline_max_p_60d_date
            .map(|date| date.to_string())
            .unwrap_or_else(|| "-".to_string()),
        export.summary.candidate_max_p_60d,
        export
            .summary
            .candidate_max_p_60d_date
            .map(|date| date.to_string())
            .unwrap_or_else(|| "-".to_string()),
    );
    println!(
        "  avg delta 20d overall={:.3} hedge={:.3} positive_window={:.3}",
        export.summary.overall_window.avg_delta_p_20d,
        export.summary.hedge_window.avg_delta_p_20d,
        export.summary.positive_window_20d.avg_delta_p_20d,
    );
    println!(
        "  20d hit rate positive_window baseline={:.3} candidate={:.3}",
        export.summary.positive_window_20d.baseline_hit_rate_20d,
        export.summary.positive_window_20d.candidate_hit_rate_20d,
    );
    let top_overall_features = export
        .summary
        .overall_window
        .top_feature_deltas_20d
        .iter()
        .take(3)
        .map(|item| format!("{}:{:.2}", item.name, item.sum_delta_contribution))
        .collect::<Vec<_>>()
        .join(", ");
    let top_hedge_features = export
        .summary
        .hedge_window
        .top_feature_deltas_20d
        .iter()
        .take(3)
        .map(|item| format!("{}:{:.2}", item.name, item.sum_delta_contribution))
        .collect::<Vec<_>>()
        .join(", ");
    println!("  top 20d feature deltas overall={top_overall_features}");
    println!("  top 20d feature deltas hedge={top_hedge_features}");
}

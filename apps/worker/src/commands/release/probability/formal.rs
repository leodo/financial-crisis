use std::{fs, path::PathBuf};

use anyhow::Context;
use chrono::{NaiveDate, Utc};
use fc_domain::{
    FormalDatasetRowRecord, LogisticProbabilityModelScoreDiagnostics, ProbabilityBundle,
    ProbabilityDiagnostics, ProbabilityHorizonOverlayDiagnostics,
};
use serde::Serialize;

use super::common::{release_probability_csv_escape, sanitize_release_probability_slice_component};

#[derive(Debug, Clone, Serialize)]
pub(super) struct ReleaseFormalProbabilitySliceExport {
    exported_at: String,
    market_scope: String,
    release_id: String,
    dataset_key: String,
    scenario_id: Option<String>,
    from_date: NaiveDate,
    to_date: NaiveDate,
    row_count: usize,
    rows: Vec<ReleaseFormalProbabilitySlicePoint>,
}

pub(super) struct ReleaseFormalProbabilitySliceBuildInput<'a> {
    pub(super) market_scope: &'a str,
    pub(super) release_id: &'a str,
    pub(super) dataset_key: &'a str,
    pub(super) scenario_id: Option<String>,
    pub(super) from_date: NaiveDate,
    pub(super) to_date: NaiveDate,
    pub(super) bundle: &'a ProbabilityBundle,
    pub(super) rows: Vec<FormalDatasetRowRecord>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct ReleaseFormalProbabilitySlicePoint {
    pub(super) as_of_date: NaiveDate,
    pub(super) split_name: String,
    pub(super) primary_scenario_id: Option<String>,
    pub(super) scenario_family: Option<String>,
    pub(super) regime_20d: String,
    pub(super) regime_60d: String,
    pub(super) prepare_episode_label: u8,
    pub(super) hedge_episode_label: u8,
    pub(super) defend_episode_label: u8,
    pub(super) primary_action_level: Option<String>,
    pub(super) coverage_score: f64,
    pub(super) probability_diagnostics: ProbabilityDiagnostics,
    pub(super) base_model_diagnostics: Vec<ReleaseFormalProbabilityBaseModelDiagnostics>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct ReleaseFormalProbabilityBaseModelDiagnostics {
    pub(super) horizon_days: u32,
    pub(super) base_model: LogisticProbabilityModelScoreDiagnostics,
}

pub(super) fn build_release_formal_probability_slice_export(
    input: ReleaseFormalProbabilitySliceBuildInput<'_>,
) -> ReleaseFormalProbabilitySliceExport {
    let ReleaseFormalProbabilitySliceBuildInput {
        market_scope,
        release_id,
        dataset_key,
        scenario_id,
        from_date,
        to_date,
        bundle,
        rows,
    } = input;
    let rows = score_release_formal_probability_slice_rows(bundle, rows);
    ReleaseFormalProbabilitySliceExport {
        exported_at: Utc::now().to_rfc3339(),
        market_scope: market_scope.to_string(),
        release_id: release_id.to_string(),
        dataset_key: dataset_key.to_string(),
        scenario_id,
        from_date,
        to_date,
        row_count: rows.len(),
        rows,
    }
}

pub(super) fn score_release_formal_probability_slice_rows(
    bundle: &ProbabilityBundle,
    mut rows: Vec<FormalDatasetRowRecord>,
) -> Vec<ReleaseFormalProbabilitySlicePoint> {
    rows.sort_by(|left, right| left.as_of_date.cmp(&right.as_of_date));
    rows.into_iter()
        .map(|row| {
            let base_model_diagnostics: Vec<ReleaseFormalProbabilityBaseModelDiagnostics> = bundle
                .horizons
                .iter()
                .map(|horizon| {
                    let mut base_model =
                        fc_domain::score_logistic_probability_model_with_diagnostics(
                            &horizon.raw_model,
                            &row.features,
                        );
                    base_model.feature_contributions.sort_by(|left, right| {
                        right.contribution.abs().total_cmp(&left.contribution.abs())
                    });
                    ReleaseFormalProbabilityBaseModelDiagnostics {
                        horizon_days: horizon.horizon_days,
                        base_model,
                    }
                })
                .collect();
            let probability_diagnostics = ProbabilityDiagnostics {
                horizon_overlays: bundle
                    .horizons
                    .iter()
                    .map(|horizon| {
                        let score =
                            fc_domain::score_probability_horizon_bundle(horizon, &row.features);
                        ProbabilityHorizonOverlayDiagnostics {
                            horizon_days: horizon.horizon_days,
                            raw_probability: score.raw_probability,
                            calibrated_probability: score.calibrated_probability,
                            final_probability: score.final_probability,
                            runtime_final_probability: Some(score.final_probability),
                            monotonic_lift: 0.0,
                            configured_overlay_count: horizon.family_overlays.len() as u32,
                            base_contributions: release_formal_probability_base_model_diagnostics(
                                &base_model_diagnostics,
                                horizon.horizon_days,
                            ),
                            contributions: score.overlay_contributions,
                            overlay_audits: Vec::new(),
                        }
                    })
                    .collect(),
            };
            ReleaseFormalProbabilitySlicePoint {
                as_of_date: row.as_of_date,
                split_name: row.split_name,
                primary_scenario_id: row.primary_scenario_id,
                scenario_family: row.scenario_family,
                regime_20d: row.regime_20d,
                regime_60d: row.regime_60d,
                prepare_episode_label: row.prepare_episode_label,
                hedge_episode_label: row.hedge_episode_label,
                defend_episode_label: row.defend_episode_label,
                primary_action_level: row.primary_action_level,
                coverage_score: row.coverage_score,
                probability_diagnostics,
                base_model_diagnostics,
            }
        })
        .collect()
}

pub(super) fn release_formal_probability_base_model(
    row: &ReleaseFormalProbabilitySlicePoint,
    horizon_days: u32,
) -> Option<&ReleaseFormalProbabilityBaseModelDiagnostics> {
    row.base_model_diagnostics
        .iter()
        .find(|item| item.horizon_days == horizon_days)
}

fn release_formal_probability_base_model_diagnostics(
    diagnostics: &[ReleaseFormalProbabilityBaseModelDiagnostics],
    horizon_days: u32,
) -> Vec<fc_domain::LogisticProbabilityFeatureContribution> {
    let mut contributions = diagnostics
        .iter()
        .find(|item| item.horizon_days == horizon_days)
        .map(|item| item.base_model.feature_contributions.clone())
        .unwrap_or_default();
    contributions.truncate(8);
    contributions
}

pub(super) fn release_formal_probability_horizon(
    row: &ReleaseFormalProbabilitySlicePoint,
    horizon_days: u32,
) -> Option<&ProbabilityHorizonOverlayDiagnostics> {
    row.probability_diagnostics
        .horizon_overlays
        .iter()
        .find(|item| item.horizon_days == horizon_days)
}

pub(super) fn write_release_formal_probability_slice_report(
    output_dir: &PathBuf,
    export: &ReleaseFormalProbabilitySliceExport,
) -> anyhow::Result<()> {
    fs::create_dir_all(output_dir)?;
    let mut stem = format!(
        "{}-{}-{}-formal-probability-slice",
        sanitize_release_probability_slice_component(&export.release_id),
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
        render_release_formal_probability_slice_csv(export)?,
    )?;
    println!("Release formal probability slice exported.");
    println!("  JSON {}", json_path.display());
    println!("  CSV  {}", csv_path.display());
    Ok(())
}

pub(super) fn print_release_formal_probability_slice_summary(
    export: &ReleaseFormalProbabilitySliceExport,
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
        "Release formal probability slice release={} dataset_key={} rows={} range={} -> {} scenario={}",
        export.release_id,
        export.dataset_key,
        export.row_count,
        first_date,
        last_date,
        export.scenario_id.as_deref().unwrap_or("all"),
    );
}

fn render_release_formal_probability_slice_csv(
    export: &ReleaseFormalProbabilitySliceExport,
) -> anyhow::Result<String> {
    let mut csv = String::from(
        "as_of_date,split_name,primary_scenario_id,scenario_family,regime_20d,regime_60d,prepare_episode_label,hedge_episode_label,defend_episode_label,primary_action_level,coverage_score,raw_p_5d,base_linear_5d,calibrated_p_5d,final_p_5d,overlay_delta_5d,base_contributions_5d_json,contributions_5d_json,raw_p_20d,base_linear_20d,calibrated_p_20d,final_p_20d,overlay_delta_20d,base_contributions_20d_json,contributions_20d_json,raw_p_60d,base_linear_60d,calibrated_p_60d,final_p_60d,overlay_delta_60d,base_contributions_60d_json,contributions_60d_json\n",
    );
    for row in &export.rows {
        let base_horizon_5d = release_formal_probability_base_model(row, 5)
            .with_context(|| "bundle scoring did not produce 5d base diagnostics")?;
        let base_horizon_20d = release_formal_probability_base_model(row, 20)
            .with_context(|| "bundle scoring did not produce 20d base diagnostics")?;
        let base_horizon_60d = release_formal_probability_base_model(row, 60)
            .with_context(|| "bundle scoring did not produce 60d base diagnostics")?;
        let horizon_5d = release_formal_probability_horizon(row, 5)
            .with_context(|| "bundle scoring did not produce 5d horizon diagnostics")?;
        let horizon_20d = release_formal_probability_horizon(row, 20)
            .with_context(|| "bundle scoring did not produce 20d horizon diagnostics")?;
        let horizon_60d = release_formal_probability_horizon(row, 60)
            .with_context(|| "bundle scoring did not produce 60d horizon diagnostics")?;
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
            format!("{:.6}", horizon_5d.raw_probability),
            format!("{:.6}", base_horizon_5d.base_model.linear_score),
            format!("{:.6}", horizon_5d.calibrated_probability),
            format!("{:.6}", horizon_5d.final_probability),
            format!(
                "{:.6}",
                horizon_5d.final_probability - horizon_5d.calibrated_probability
            ),
            serde_json::to_string(&base_horizon_5d.base_model.feature_contributions)?,
            serde_json::to_string(&horizon_5d.contributions)?,
            format!("{:.6}", horizon_20d.raw_probability),
            format!("{:.6}", base_horizon_20d.base_model.linear_score),
            format!("{:.6}", horizon_20d.calibrated_probability),
            format!("{:.6}", horizon_20d.final_probability),
            format!(
                "{:.6}",
                horizon_20d.final_probability - horizon_20d.calibrated_probability
            ),
            serde_json::to_string(&base_horizon_20d.base_model.feature_contributions)?,
            serde_json::to_string(&horizon_20d.contributions)?,
            format!("{:.6}", horizon_60d.raw_probability),
            format!("{:.6}", base_horizon_60d.base_model.linear_score),
            format!("{:.6}", horizon_60d.calibrated_probability),
            format!("{:.6}", horizon_60d.final_probability),
            format!(
                "{:.6}",
                horizon_60d.final_probability - horizon_60d.calibrated_probability
            ),
            serde_json::to_string(&base_horizon_60d.base_model.feature_contributions)?,
            serde_json::to_string(&horizon_60d.contributions)?,
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

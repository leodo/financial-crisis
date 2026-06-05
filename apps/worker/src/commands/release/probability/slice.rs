use std::{fs, path::PathBuf};

use chrono::{NaiveDate, Utc};
use fc_domain::{
    HistoricalAssessmentPointRecord, ProbabilityDiagnostics, ProbabilityHorizonOverlayDiagnostics,
    ProbabilityOverlayContribution,
};
use serde::Serialize;

use super::common::{release_probability_csv_escape, sanitize_release_probability_slice_component};

#[derive(Debug, Clone, Serialize)]
pub(super) struct ReleaseProbabilitySliceExport {
    exported_at: String,
    market_scope: String,
    release_id: String,
    replay_run_id: String,
    history_mode: String,
    history_limit: usize,
    from_date: NaiveDate,
    to_date: NaiveDate,
    row_count: usize,
    rows: Vec<ReleaseProbabilitySlicePoint>,
}

pub(super) struct ReleaseProbabilitySliceBuildInput<'a> {
    pub(super) market_scope: &'a str,
    pub(super) release_id: &'a str,
    pub(super) replay_run_id: &'a str,
    pub(super) history_mode: &'a str,
    pub(super) history_limit: usize,
    pub(super) from_date: NaiveDate,
    pub(super) to_date: NaiveDate,
    pub(super) points: Vec<HistoricalAssessmentPointRecord>,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseProbabilitySlicePoint {
    as_of_date: NaiveDate,
    overall_score: f64,
    structural_score: f64,
    trigger_score: f64,
    external_shock_score: f64,
    raw_p_5d: f64,
    raw_p_20d: f64,
    raw_p_60d: f64,
    calibrated_p_5d: f64,
    calibrated_p_20d: f64,
    calibrated_p_60d: f64,
    posture: String,
    time_to_risk_bucket: String,
    actionability_prepare: f64,
    actionability_hedge: f64,
    actionability_defend: f64,
    coverage_score: f64,
    freshness_status: String,
    posture_trigger_codes: Vec<String>,
    posture_blocker_codes: Vec<String>,
    probability_diagnostics: ProbabilityDiagnostics,
}

pub(super) fn build_release_probability_slice_export(
    input: ReleaseProbabilitySliceBuildInput<'_>,
) -> ReleaseProbabilitySliceExport {
    let ReleaseProbabilitySliceBuildInput {
        market_scope,
        release_id,
        replay_run_id,
        history_mode,
        history_limit,
        from_date,
        to_date,
        points,
    } = input;
    let rows = points
        .into_iter()
        .map(|point| ReleaseProbabilitySlicePoint {
            as_of_date: point.as_of_date,
            overall_score: point.overall_score,
            structural_score: point.structural_score,
            trigger_score: point.trigger_score,
            external_shock_score: point.external_shock_score,
            raw_p_5d: point.raw_p_5d,
            raw_p_20d: point.raw_p_20d,
            raw_p_60d: point.raw_p_60d,
            calibrated_p_5d: point.calibrated_p_5d,
            calibrated_p_20d: point.calibrated_p_20d,
            calibrated_p_60d: point.calibrated_p_60d,
            posture: point.posture,
            time_to_risk_bucket: point.time_to_risk_bucket,
            actionability_prepare: point.actionability_prepare,
            actionability_hedge: point.actionability_hedge,
            actionability_defend: point.actionability_defend,
            coverage_score: point.coverage_score,
            freshness_status: point.freshness_status,
            posture_trigger_codes: point.posture_trigger_codes,
            posture_blocker_codes: point.posture_blocker_codes,
            probability_diagnostics: point.probability_diagnostics,
        })
        .collect::<Vec<_>>();
    ReleaseProbabilitySliceExport {
        exported_at: Utc::now().to_rfc3339(),
        market_scope: market_scope.to_string(),
        release_id: release_id.to_string(),
        replay_run_id: replay_run_id.to_string(),
        history_mode: history_mode.to_string(),
        history_limit,
        from_date,
        to_date,
        row_count: rows.len(),
        rows,
    }
}

pub(super) fn write_release_probability_slice_report(
    output_dir: &PathBuf,
    export: &ReleaseProbabilitySliceExport,
) -> anyhow::Result<()> {
    fs::create_dir_all(output_dir)?;
    let stem = format!(
        "{}-{}-{}-{}-probability-slice",
        sanitize_release_probability_slice_component(&export.release_id),
        export.from_date,
        export.to_date,
        sanitize_release_probability_slice_component(&export.history_mode),
    );
    let json_path = output_dir.join(format!("{stem}.json"));
    let csv_path = output_dir.join(format!("{stem}.csv"));
    fs::write(&json_path, serde_json::to_string_pretty(export)?)?;
    fs::write(&csv_path, render_release_probability_slice_csv(export)?)?;
    println!("Release probability slice exported.");
    println!("  JSON {}", json_path.display());
    println!("  CSV  {}", csv_path.display());
    Ok(())
}

pub(super) fn print_release_probability_slice_summary(export: &ReleaseProbabilitySliceExport) {
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
        "Release probability slice release={} replay_run={} rows={} range={} -> {} history_mode={} history_limit={}",
        export.release_id,
        export.replay_run_id,
        export.row_count,
        first_date,
        last_date,
        export.history_mode,
        export.history_limit
    );
}

fn render_release_probability_slice_csv(
    export: &ReleaseProbabilitySliceExport,
) -> anyhow::Result<String> {
    let mut csv = String::from(
        "as_of_date,overall_score,structural_score,trigger_score,external_shock_score,posture,time_to_risk_bucket,actionability_prepare,actionability_hedge,actionability_defend,coverage_score,freshness_status,raw_p_5d,calibrated_p_5d,final_p_5d,overlay_delta_5d,monotonic_lift_5d,contributions_5d_json,raw_p_20d,calibrated_p_20d,final_p_20d,overlay_delta_20d,monotonic_lift_20d,contributions_20d_json,raw_p_60d,calibrated_p_60d,final_p_60d,overlay_delta_60d,monotonic_lift_60d,contributions_60d_json,posture_trigger_codes_json,posture_blocker_codes_json\n",
    );
    for row in &export.rows {
        let horizon_5d = release_probability_horizon(row, 5);
        let horizon_20d = release_probability_horizon(row, 20);
        let horizon_60d = release_probability_horizon(row, 60);
        let columns = [
            row.as_of_date.to_string(),
            format!("{:.6}", row.overall_score),
            format!("{:.6}", row.structural_score),
            format!("{:.6}", row.trigger_score),
            format!("{:.6}", row.external_shock_score),
            row.posture.clone(),
            row.time_to_risk_bucket.clone(),
            format!("{:.6}", row.actionability_prepare),
            format!("{:.6}", row.actionability_hedge),
            format!("{:.6}", row.actionability_defend),
            format!("{:.6}", row.coverage_score),
            row.freshness_status.clone(),
            format!("{:.6}", release_raw_probability(row, 5)),
            format!("{:.6}", release_calibrated_probability(row, 5)),
            format!("{:.6}", release_final_probability(row, 5)),
            format!("{:.6}", release_overlay_delta(row, 5)),
            format!("{:.6}", release_monotonic_lift(row, 5)),
            serde_json::to_string(&release_probability_contributions(horizon_5d))?,
            format!("{:.6}", release_raw_probability(row, 20)),
            format!("{:.6}", release_calibrated_probability(row, 20)),
            format!("{:.6}", release_final_probability(row, 20)),
            format!("{:.6}", release_overlay_delta(row, 20)),
            format!("{:.6}", release_monotonic_lift(row, 20)),
            serde_json::to_string(&release_probability_contributions(horizon_20d))?,
            format!("{:.6}", release_raw_probability(row, 60)),
            format!("{:.6}", release_calibrated_probability(row, 60)),
            format!("{:.6}", release_final_probability(row, 60)),
            format!("{:.6}", release_overlay_delta(row, 60)),
            format!("{:.6}", release_monotonic_lift(row, 60)),
            serde_json::to_string(&release_probability_contributions(horizon_60d))?,
            serde_json::to_string(&row.posture_trigger_codes)?,
            serde_json::to_string(&row.posture_blocker_codes)?,
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

fn release_probability_horizon(
    row: &ReleaseProbabilitySlicePoint,
    horizon_days: u32,
) -> Option<&ProbabilityHorizonOverlayDiagnostics> {
    row.probability_diagnostics
        .horizon_overlays
        .iter()
        .find(|horizon| horizon.horizon_days == horizon_days)
}

fn release_probability_contributions(
    horizon: Option<&ProbabilityHorizonOverlayDiagnostics>,
) -> Vec<ProbabilityOverlayContribution> {
    horizon
        .map(|horizon| horizon.contributions.clone())
        .unwrap_or_default()
}

fn release_raw_probability(row: &ReleaseProbabilitySlicePoint, horizon_days: u32) -> f64 {
    release_probability_horizon(row, horizon_days)
        .map(|horizon| horizon.raw_probability)
        .unwrap_or_else(|| match horizon_days {
            5 => row.raw_p_5d,
            20 => row.raw_p_20d,
            60 => row.raw_p_60d,
            _ => 0.0,
        })
}

fn release_calibrated_probability(row: &ReleaseProbabilitySlicePoint, horizon_days: u32) -> f64 {
    release_probability_horizon(row, horizon_days)
        .map(|horizon| horizon.calibrated_probability)
        .unwrap_or_else(|| match horizon_days {
            5 => row.calibrated_p_5d,
            20 => row.calibrated_p_20d,
            60 => row.calibrated_p_60d,
            _ => 0.0,
        })
}

fn release_final_probability(row: &ReleaseProbabilitySlicePoint, horizon_days: u32) -> f64 {
    release_probability_horizon(row, horizon_days)
        .and_then(|horizon| horizon.runtime_final_probability)
        .or_else(|| {
            release_probability_horizon(row, horizon_days).map(|horizon| horizon.final_probability)
        })
        .unwrap_or_else(|| release_calibrated_probability(row, horizon_days))
}

fn release_overlay_delta(row: &ReleaseProbabilitySlicePoint, horizon_days: u32) -> f64 {
    release_probability_horizon(row, horizon_days)
        .map(|horizon| horizon.final_probability - horizon.calibrated_probability)
        .unwrap_or(0.0)
}

fn release_monotonic_lift(row: &ReleaseProbabilitySlicePoint, horizon_days: u32) -> f64 {
    release_probability_horizon(row, horizon_days)
        .map(|horizon| horizon.monotonic_lift)
        .unwrap_or(0.0)
}

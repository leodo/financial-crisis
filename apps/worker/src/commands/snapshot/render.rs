use std::{fmt::Write, fs, path::Path};

use fc_domain::PredictionSnapshotRecord;

use super::options::ExportFormat;

pub(crate) fn write_snapshot_export(
    snapshots: &[PredictionSnapshotRecord],
    format: ExportFormat,
    output_path: Option<&Path>,
) -> anyhow::Result<()> {
    let content = match format {
        ExportFormat::Json => serde_json::to_string_pretty(snapshots)?,
        ExportFormat::Csv => render_snapshot_csv(snapshots),
    };
    write_or_print_export(content, output_path)
}

pub(crate) fn write_dataset_export(
    dataset: &[crate::ProbabilityTrainingRow],
    feature_names: &[String],
    format: ExportFormat,
    output_path: Option<&Path>,
) -> anyhow::Result<()> {
    let content = match format {
        ExportFormat::Json => serde_json::to_string_pretty(dataset)?,
        ExportFormat::Csv => render_dataset_csv(dataset, feature_names),
    };
    write_or_print_export(content, output_path)
}

fn write_or_print_export(content: String, output_path: Option<&Path>) -> anyhow::Result<()> {
    if let Some(path) = output_path {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(path, content)?;
        println!("Exported {}", path.display());
    } else {
        println!("{content}");
    }
    Ok(())
}

fn render_snapshot_csv(snapshots: &[PredictionSnapshotRecord]) -> String {
    let mut csv = String::from(
        "as_of_date,market_scope,release_id,probability_mode,release_status,point_in_time_mode,overall_score,external_shock_score,raw_p_5d,raw_p_20d,raw_p_60d,calibrated_p_5d,calibrated_p_20d,calibrated_p_60d,posture,time_to_risk_bucket,coverage_score,freshness_status,method_version,posture_trigger_codes,posture_blocker_codes,recorded_at\n",
    );
    for snapshot in snapshots {
        let _ = writeln!(
            csv,
            "{},{},{},{},{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{},{},{:.6},{},{},{},{},{}",
            snapshot.as_of_date,
            snapshot.market_scope,
            snapshot.release_id.as_deref().unwrap_or(""),
            snapshot.probability_mode,
            snapshot.release_status,
            snapshot.point_in_time_mode,
            snapshot.overall_score,
            snapshot.external_shock_score,
            snapshot.raw_p_5d,
            snapshot.raw_p_20d,
            snapshot.raw_p_60d,
            snapshot.calibrated_p_5d,
            snapshot.calibrated_p_20d,
            snapshot.calibrated_p_60d,
            snapshot.posture,
            snapshot.time_to_risk_bucket,
            snapshot.coverage_score,
            snapshot.freshness_status,
            snapshot.method_version,
            snapshot.posture_trigger_codes.join("|"),
            snapshot.posture_blocker_codes.join("|"),
            snapshot.recorded_at.to_rfc3339()
        );
    }
    csv
}

pub(crate) fn render_dataset_csv(
    dataset: &[crate::ProbabilityTrainingRow],
    feature_names: &[String],
) -> String {
    let mut header = String::from(
        "as_of_date,market_scope,release_id,probability_mode,freshness_status,time_to_risk_bucket,split_name,primary_scenario_id,scenario_family,scenario_training_role,label_5d,label_20d,label_60d,action_label_5d,action_label_20d,action_label_60d,prepare_episode_label,hedge_episode_label,defend_episode_label,primary_action_level,action_episode_id,action_episode_phase,protected_action_window",
    );
    for feature in feature_names {
        header.push(',');
        header.push_str(feature);
    }
    header.push('\n');

    let mut csv = header;
    for row in dataset {
        let columns = [
            row.as_of_date.to_string(),
            row.market_scope.clone(),
            row.release_id.clone().unwrap_or_default(),
            row.probability_mode.clone().unwrap_or_default(),
            row.freshness_status.clone().unwrap_or_default(),
            row.time_to_risk_bucket.clone().unwrap_or_default(),
            row.split_name.clone().unwrap_or_default(),
            row.primary_scenario_id.clone().unwrap_or_default(),
            row.scenario_family.clone().unwrap_or_default(),
            row.scenario_training_role.clone().unwrap_or_default(),
            row.label_5d.to_string(),
            row.label_20d.to_string(),
            row.label_60d.to_string(),
            row.action_label_5d.to_string(),
            row.action_label_20d.to_string(),
            row.action_label_60d.to_string(),
            row.prepare_episode_label.to_string(),
            row.hedge_episode_label.to_string(),
            row.defend_episode_label.to_string(),
            row.primary_action_level.clone().unwrap_or_default(),
            row.action_episode_id.clone().unwrap_or_default(),
            row.action_episode_phase.clone(),
            (row.protected_action_window as u8).to_string(),
        ];
        csv.push_str(&columns.join(","));
        for feature in feature_names {
            let value = row.features.get(feature).copied().unwrap_or_default();
            let _ = write!(csv, ",{value:.6}");
        }
        csv.push('\n');
    }
    csv
}

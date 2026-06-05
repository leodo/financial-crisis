use std::{collections::BTreeSet, fmt::Write, fs, path::PathBuf};

use anyhow::bail;
use chrono::Utc;
use fc_domain::{FormalDatasetRecord, FormalDatasetRowRecord};

use crate::commands::dataset::options::FormalDatasetSliceOptions;

use super::FormalDatasetSliceExport;

pub(crate) fn build_formal_dataset_slice_export(
    dataset_key: String,
    dataset: FormalDatasetRecord,
    rows: Vec<FormalDatasetRowRecord>,
    options: &FormalDatasetSliceOptions,
) -> anyhow::Result<FormalDatasetSliceExport> {
    let rows = filter_formal_dataset_rows_for_slice(rows, options);
    if rows.is_empty() {
        bail!(
            "formal dataset slice is empty (dataset_key={}, scenario_id={}, split_name={}, from={}, to={})",
            dataset_key,
            options.scenario_id,
            options.split_name.as_deref().unwrap_or("-"),
            options
                .from_date
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
            options
                .to_date
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string())
        );
    }

    let feature_names = collect_formal_dataset_slice_feature_names(&rows);
    Ok(FormalDatasetSliceExport {
        exported_at: Utc::now().to_rfc3339(),
        dataset_key,
        dataset,
        scenario_id: options.scenario_id.clone(),
        split_name: options.split_name.clone(),
        from_date: options.from_date,
        to_date: options.to_date,
        row_count: rows.len(),
        feature_names,
        rows,
    })
}

pub(crate) fn sanitize_filename_component(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => ch,
            _ => '_',
        })
        .collect()
}

pub(crate) fn write_formal_dataset_slice_report(
    output_dir: &PathBuf,
    export: &FormalDatasetSliceExport,
) -> anyhow::Result<()> {
    fs::create_dir_all(output_dir)?;
    let mut stem = format!(
        "{}-{}-slice",
        sanitize_filename_component(&export.dataset_key),
        sanitize_filename_component(&export.scenario_id)
    );
    if let Some(split_name) = export.split_name.as_deref() {
        let _ = write!(stem, "-{}", sanitize_filename_component(split_name));
    }
    if let Some(from_date) = export.from_date {
        let _ = write!(stem, "-from-{from_date}");
    }
    if let Some(to_date) = export.to_date {
        let _ = write!(stem, "-to-{to_date}");
    }
    let json_path = output_dir.join(format!("{stem}.json"));
    let csv_path = output_dir.join(format!("{stem}.csv"));
    fs::write(&json_path, serde_json::to_string_pretty(export)?)?;
    fs::write(
        &csv_path,
        super::render::render_formal_dataset_slice_csv(&export.rows, &export.feature_names),
    )?;
    println!("Formal dataset slice exported.");
    println!("  JSON {}", json_path.display());
    println!("  CSV  {}", csv_path.display());
    Ok(())
}

fn filter_formal_dataset_rows_for_slice(
    rows: Vec<FormalDatasetRowRecord>,
    options: &FormalDatasetSliceOptions,
) -> Vec<FormalDatasetRowRecord> {
    let mut filtered = rows
        .into_iter()
        .filter(|row| row.primary_scenario_id.as_deref() == Some(options.scenario_id.as_str()))
        .filter(|row| {
            options
                .from_date
                .map(|from_date| row.as_of_date >= from_date)
                .unwrap_or(true)
        })
        .filter(|row| {
            options
                .to_date
                .map(|to_date| row.as_of_date <= to_date)
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();
    filtered.sort_by(|left, right| {
        left.as_of_date
            .cmp(&right.as_of_date)
            .then_with(|| left.split_name.cmp(&right.split_name))
    });
    if let Some(limit) = options.limit {
        filtered.truncate(limit);
    }
    filtered
}

fn collect_formal_dataset_slice_feature_names(rows: &[FormalDatasetRowRecord]) -> Vec<String> {
    rows.iter()
        .flat_map(|row| row.features.keys().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

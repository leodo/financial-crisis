use std::{fs, path::PathBuf};

use super::super::common::{
    release_probability_csv_escape, sanitize_release_probability_slice_component,
};
use super::ReleaseFormalProbabilityCompareExport;

pub(in super::super) fn write_release_formal_probability_compare_report(
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

pub(in super::super) fn print_release_formal_probability_compare_summary(
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

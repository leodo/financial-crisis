use std::fmt::Write;

use fc_domain::FormalDatasetRowRecord;

use super::{FormalDatasetSliceExport, FormalDatasetSummaryEnvelope};

pub(crate) fn render_formal_dataset_slice_csv(
    rows: &[FormalDatasetRowRecord],
    feature_names: &[String],
) -> String {
    let mut header = String::from(
        "dataset_key,split_name,as_of_date,entity_id,market_scope,primary_scenario_id,scenario_family,scenario_training_role,label_5d,label_20d,label_60d,regime_5d,regime_20d,regime_60d,action_label_5d,action_label_20d,action_label_60d,prepare_episode_label,hedge_episode_label,defend_episode_label,primary_action_level,action_episode_id,action_episode_phase,protected_action_window,coverage_score,core_feature_coverage,trigger_feature_coverage,external_feature_coverage,sample_quality_grade,latest_visible_at",
    );
    for feature_name in feature_names {
        header.push(',');
        header.push_str(feature_name);
    }
    header.push('\n');

    let mut csv = header;
    for row in rows {
        let columns = [
            row.dataset_key.clone(),
            row.split_name.clone(),
            row.as_of_date.to_string(),
            row.entity_id.clone(),
            row.market_scope.clone(),
            row.primary_scenario_id.clone().unwrap_or_default(),
            row.scenario_family.clone().unwrap_or_default(),
            row.scenario_training_role.clone().unwrap_or_default(),
            row.label_5d.to_string(),
            row.label_20d.to_string(),
            row.label_60d.to_string(),
            row.regime_5d.clone(),
            row.regime_20d.clone(),
            row.regime_60d.clone(),
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
            format!("{:.4}", row.coverage_score),
            format!("{:.4}", row.core_feature_coverage),
            format!("{:.4}", row.trigger_feature_coverage),
            format!("{:.4}", row.external_feature_coverage),
            row.sample_quality_grade.clone(),
            row.latest_visible_at
                .map(|value| value.to_rfc3339())
                .unwrap_or_default(),
        ];
        csv.push_str(&columns.join(","));
        for feature_name in feature_names {
            let value = row.features.get(feature_name).copied().unwrap_or_default();
            let _ = write!(csv, ",{value:.6}");
        }
        csv.push('\n');
    }
    csv
}

pub(crate) fn render_formal_dataset_summary_markdown(
    summary: &FormalDatasetSummaryEnvelope,
) -> String {
    let mut markdown = String::new();
    let manifest = &summary.dataset.manifest;
    let _ = writeln!(markdown, "# Formal Dataset Summary");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "- Generated at: {}", summary.generated_at);
    let _ = writeln!(markdown, "- Dataset key: {}", summary.dataset_key);
    let _ = writeln!(markdown, "- Market scope: {}", manifest.market_scope);
    let _ = writeln!(markdown, "- Feature set: {}", manifest.feature_set_version);
    let _ = writeln!(markdown, "- Label version: {}", manifest.label_version);
    let _ = writeln!(
        markdown,
        "- Scenario set: {}",
        manifest.scenario_set_version
    );
    let _ = writeln!(
        markdown,
        "- Coverage catalog: {}",
        summary.coverage_catalog.catalog_id
    );
    let _ = writeln!(
        markdown,
        "- Dataset intent: {}",
        summary.coverage_catalog.dataset_intent
    );
    let _ = writeln!(markdown, "- PIT mode: {}", manifest.point_in_time_mode);
    let _ = writeln!(markdown, "- Rows: {}", manifest.row_count);
    let _ = writeln!(
        markdown,
        "- Range: {} -> {}",
        manifest
            .from_date
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string()),
        manifest
            .to_date
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string())
    );
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Scenario Data Coverage Alignment");
    let _ = writeln!(markdown);
    let _ = writeln!(
        markdown,
        "- Coverage source: {}",
        summary.coverage_catalog.source
    );
    let _ = writeln!(
        markdown,
        "- Coverage market scope: {}",
        summary.coverage_catalog.market_scope
    );
    let _ = writeln!(
        markdown,
        "- Scenario alignment: {}/{} scenarios match dataset intent {}",
        summary.coverage_catalog.aligned_scenario_count,
        summary.coverage_catalog.total_scenario_count,
        summary.coverage_catalog.dataset_intent
    );
    let _ = writeln!(
        markdown,
        "- Eligibility mix: main={} extension={} protected={} analog={}",
        summary.coverage_catalog.main_training_eligible_count,
        summary.coverage_catalog.extension_training_eligible_count,
        summary.coverage_catalog.protected_stress_eligible_count,
        summary.coverage_catalog.historical_analog_eligible_count
    );
    if let Some(warning) = &summary.coverage_catalog.warning {
        let _ = writeln!(markdown, "- Coverage warning: {}", warning);
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Split Summary");
    let _ = writeln!(markdown);
    let _ = writeln!(
        markdown,
        "| Split | Rows | Forward 5d+ | Forward 20d+ | Forward 60d+ | Prepare Primary | Hedge Primary | Defend Primary | Late Validation | Protected | Avg Coverage | Core | Trigger | External | Scenarios |"
    );
    let _ = writeln!(
        markdown,
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
    );
    for split in &summary.split_summaries {
        let _ = writeln!(
            markdown,
            "| {} | {} | {} ({}) | {} ({}) | {} ({}) | {} ({}) | {} ({}) | {} ({}) | {} ({}) | {} ({}) | {:.1}% | {:.1}% | {:.1}% | {:.1}% | {} |",
            split.split_name,
            split.row_count,
            split.positive_5d_count,
            crate::format_pct(split.positive_5d_rate),
            split.positive_20d_count,
            crate::format_pct(split.positive_20d_rate),
            split.positive_60d_count,
            crate::format_pct(split.positive_60d_rate),
            split.prepare_primary_count,
            crate::format_pct(split.prepare_primary_rate),
            split.hedge_primary_count,
            crate::format_pct(split.hedge_primary_rate),
            split.defend_primary_count,
            crate::format_pct(split.defend_primary_rate),
            split.late_validation_row_count,
            crate::format_pct(split.late_validation_row_rate),
            split.protected_row_count,
            crate::format_pct(split.protected_row_rate),
            split.avg_coverage_score * 100.0,
            split.avg_core_feature_coverage * 100.0,
            split.avg_trigger_feature_coverage * 100.0,
            split.avg_external_feature_coverage * 100.0,
            split.scenario_count
        );
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Scenario Coverage");
    let _ = writeln!(markdown);
    let _ = writeln!(
        markdown,
        "| Scenario | Label | Family | Dataset Role | Coverage Role | Grade | Coverage PIT | Allowed | Status | Gaps | Rows | Splits | Range |"
    );
    let _ = writeln!(
        markdown,
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
    );
    for scenario in &summary.scenario_summaries {
        let allowed_roles = format_allowed_roles(
            scenario.usable_for_main_training,
            scenario.usable_for_extension_training,
            scenario.usable_for_protected_stress,
            scenario.usable_for_historical_analog,
        );
        let coverage_gaps = if scenario.coverage_blocking_gaps.is_empty() {
            "-".to_string()
        } else {
            scenario.coverage_blocking_gaps.join("; ")
        };
        let _ = writeln!(
            markdown,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} -> {} |",
            scenario.scenario_id,
            scenario.label.as_deref().unwrap_or("-"),
            scenario.family.as_deref().unwrap_or("-"),
            scenario.training_role.as_deref().unwrap_or("-"),
            scenario.coverage_recommended_role.as_deref().unwrap_or("-"),
            scenario.coverage_grade.as_deref().unwrap_or("-"),
            scenario
                .coverage_point_in_time_mode
                .as_deref()
                .unwrap_or("-"),
            allowed_roles,
            scenario.coverage_current_status.as_deref().unwrap_or("-"),
            coverage_gaps,
            scenario.row_count,
            scenario.split_count,
            scenario.first_as_of_date,
            scenario.last_as_of_date
        );
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Scenario Episode Metadata");
    let _ = writeln!(markdown);
    let _ = writeln!(
        markdown,
        "| Scenario | Protected | Horizons | Template | Free Sources |"
    );
    let _ = writeln!(markdown, "| --- | --- | --- | --- | --- |");
    for scenario in &summary.scenario_summaries {
        let default_horizon_roles = if scenario.default_horizon_roles.is_empty() {
            "-".to_string()
        } else {
            scenario
                .default_horizon_roles
                .iter()
                .map(|value| format!("{value}d"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let free_sources = if scenario.coverage_free_sources.is_empty() {
            "-".to_string()
        } else {
            scenario.coverage_free_sources.join(", ")
        };
        let _ = writeln!(
            markdown,
            "| {} | {} | {} | {} | {} |",
            scenario.scenario_id,
            scenario
                .protected_window
                .map(|value| if value { "yes" } else { "no" })
                .unwrap_or("-"),
            default_horizon_roles,
            scenario.episode_template_id.as_deref().unwrap_or("-"),
            free_sources
        );
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Quality Mix");
    let _ = writeln!(markdown);
    for quality in &summary.quality_summaries {
        let _ = writeln!(
            markdown,
            "- grade {}: {} rows",
            quality.grade, quality.row_count
        );
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Regime Mix");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "| Split | Horizon | Regime | Rows | Share |");
    let _ = writeln!(markdown, "| --- | --- | --- | --- | --- |");
    for regime in &summary.regime_summaries {
        let _ = writeln!(
            markdown,
            "| {} | {}d | {} | {} | {} |",
            regime.split_name,
            regime.horizon_days,
            regime.regime,
            regime.row_count,
            crate::format_pct(regime.row_rate),
        );
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Recommendation");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "{}", summary.recommendation);
    markdown
}

pub(crate) fn print_formal_dataset_summary(summary: &FormalDatasetSummaryEnvelope) {
    println!(
        "Formal dataset {} rows={} pit={} feature_set={} coverage_catalog={} aligned={}/{}",
        summary.dataset_key,
        summary.dataset.manifest.row_count,
        summary.dataset.manifest.point_in_time_mode,
        summary.dataset.manifest.feature_set_version,
        summary.coverage_catalog.catalog_id,
        summary.coverage_catalog.aligned_scenario_count,
        summary.coverage_catalog.total_scenario_count
    );
    for split in &summary.split_summaries {
        println!(
            "  split={} rows={} forward[5d={}({}) 20d={}({}) 60d={}({})] action[prepare={}({}) hedge={}({}) defend={}({}) late_validation={}({}) protected={}({})] avg_coverage={:.1}%",
            split.split_name,
            split.row_count,
            split.positive_5d_count,
            crate::format_pct(split.positive_5d_rate),
            split.positive_20d_count,
            crate::format_pct(split.positive_20d_rate),
            split.positive_60d_count,
            crate::format_pct(split.positive_60d_rate),
            split.prepare_primary_count,
            crate::format_pct(split.prepare_primary_rate),
            split.hedge_primary_count,
            crate::format_pct(split.hedge_primary_rate),
            split.defend_primary_count,
            crate::format_pct(split.defend_primary_rate),
            split.late_validation_row_count,
            crate::format_pct(split.late_validation_row_rate),
            split.protected_row_count,
            crate::format_pct(split.protected_row_rate),
            split.avg_coverage_score * 100.0
        );
    }
    println!("  recommendation {}", summary.recommendation);
}

fn format_allowed_roles(
    usable_for_main_training: Option<bool>,
    usable_for_extension_training: Option<bool>,
    usable_for_protected_stress: Option<bool>,
    usable_for_historical_analog: Option<bool>,
) -> String {
    let mut roles = Vec::new();
    if usable_for_main_training == Some(true) {
        roles.push("main");
    }
    if usable_for_extension_training == Some(true) {
        roles.push("ext");
    }
    if usable_for_protected_stress == Some(true) {
        roles.push("protected");
    }
    if usable_for_historical_analog == Some(true) {
        roles.push("analog");
    }
    if roles.is_empty() {
        "-".to_string()
    } else {
        roles.join(", ")
    }
}

pub(crate) fn print_formal_dataset_slice_summary(export: &FormalDatasetSliceExport) {
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
        "Formal dataset slice dataset_key={} scenario_id={} rows={} range={} -> {} split={} features={}",
        export.dataset_key,
        export.scenario_id,
        export.row_count,
        first_date,
        last_date,
        export.split_name.as_deref().unwrap_or("all"),
        export.feature_names.len(),
    );
}

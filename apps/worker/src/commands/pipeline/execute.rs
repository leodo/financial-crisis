use std::collections::BTreeMap;

use crate::training::PipelineArtifacts;

use super::{options::PipelineBootstrapOptions, PipelineTrainOptions};

pub(crate) async fn research_pipeline_train_probability(args: &[String]) -> anyhow::Result<()> {
    let options = PipelineTrainOptions::parse(args)?;
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;
    if options.dry_run {
        let training =
            crate::commands::pipeline::load_probability_training_input(&store, &options).await?;
        print_training_dry_run_summary(&training, &options);
        return Ok(());
    }
    let artifacts = crate::train_probability_pipeline(&store, &options).await?;
    print_training_artifacts_summary(&artifacts, &options);
    Ok(())
}

pub(crate) async fn research_pipeline_bootstrap_formal_release(
    args: &[String],
) -> anyhow::Result<()> {
    let options = PipelineBootstrapOptions::parse(args)?;
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;
    let artifacts = crate::train_probability_pipeline(&store, &options.train).await?;
    store.upsert_model_release(&artifacts.release).await?;
    println!(
        "Published formal release {}.",
        artifacts.release.manifest.release_id
    );
    println!("  manifest {}", artifacts.manifest_path.display());
    println!("  bundle   {}", artifacts.bundle_path.display());

    if options.activate {
        super::super::release::activate_release_with_runtime_guard(
            &store,
            &artifacts.release.manifest.market_scope,
            &artifacts.release.manifest.release_id,
            options.reload_api,
            &options.api_reload_url,
            options.skip_operational_guard,
            &options.updated_by,
        )
        .await?;
    }

    Ok(())
}

#[derive(Debug, Default)]
struct TrainingRowTopologySummary {
    row_count: usize,
    topology_repair_rows: usize,
    protected_action_rows: usize,
    mixed_systemic_extension_primary_rows: usize,
    mixed_systemic_extension_primary_repair_rows: usize,
    mixed_systemic_extension_late_validation_rows: usize,
}

fn print_training_dry_run_summary(
    training: &crate::ProbabilityTrainingInput,
    options: &PipelineTrainOptions,
) {
    let train = summarize_training_topology_rows(&training.train_rows);
    let calibration = summarize_training_topology_rows(&training.calibration_rows);
    let evaluation = summarize_training_topology_rows(&training.evaluation_rows);

    println!("Formal probability training dry run.");
    println!("  dataset_source   {}", training.dataset_source.as_str());
    println!("  dataset_label    {}", training.dataset_label);
    println!("  market_scope     {}", training.market_scope);
    println!("  model_shape      {}", options.model_shape.as_str());
    println!("  pit_mode         {}", training.point_in_time_mode);
    println!("  feature_set      {}", training.feature_set_version);
    println!("  label_version    {}", training.label_version);
    println!("  feature_count    {}", training.feature_names.len());
    println!(
        "  rows             train={} calibration={} evaluation={}",
        train.row_count, calibration.row_count, evaluation.row_count,
    );
    println!(
        "  topology_repair  train={} calibration={} evaluation={}",
        train.topology_repair_rows,
        calibration.topology_repair_rows,
        evaluation.topology_repair_rows,
    );
    println!(
        "  protected_rows   train={} calibration={} evaluation={}",
        train.protected_action_rows,
        calibration.protected_action_rows,
        evaluation.protected_action_rows,
    );
    println!(
        "  mixed_sys_primary_ext train={} calibration={} evaluation={}",
        train.mixed_systemic_extension_primary_rows,
        calibration.mixed_systemic_extension_primary_rows,
        evaluation.mixed_systemic_extension_primary_rows,
    );
    println!(
        "  mixed_sys_primary_repair train={} calibration={} evaluation={}",
        train.mixed_systemic_extension_primary_repair_rows,
        calibration.mixed_systemic_extension_primary_repair_rows,
        evaluation.mixed_systemic_extension_primary_repair_rows,
    );
    println!(
        "  mixed_sys_late_ext    train={} calibration={} evaluation={}",
        train.mixed_systemic_extension_late_validation_rows,
        calibration.mixed_systemic_extension_late_validation_rows,
        evaluation.mixed_systemic_extension_late_validation_rows,
    );
    println!(
        "  split_names      train=[{}] calibration=[{}] evaluation=[{}]",
        format_split_name_counts(&training.train_rows),
        format_split_name_counts(&training.calibration_rows),
        format_split_name_counts(&training.evaluation_rows),
    );
}

fn summarize_training_topology_rows(
    rows: &[crate::ProbabilityTrainingRow],
) -> TrainingRowTopologySummary {
    rows.iter()
        .fold(TrainingRowTopologySummary::default(), |mut summary, row| {
            summary.row_count += 1;
            let topology_repair_row = matches!(
                row.split_name.as_deref(),
                Some("train_topology_repair") | Some("calibration_topology_repair")
            );
            if topology_repair_row {
                summary.topology_repair_rows += 1;
            }
            if row.protected_action_window {
                summary.protected_action_rows += 1;
            }
            if row.scenario_training_role.as_deref() == Some("extension_only")
                && row.scenario_family.as_deref() == Some("mixed_systemic_stress")
                && row.protected_action_window
                && row.action_episode_id.is_some()
            {
                match row.action_episode_phase.as_str() {
                    "primary" => {
                        summary.mixed_systemic_extension_primary_rows += 1;
                        if topology_repair_row {
                            summary.mixed_systemic_extension_primary_repair_rows += 1;
                        }
                    }
                    "late_validation" => summary.mixed_systemic_extension_late_validation_rows += 1,
                    _ => {}
                }
            }
            summary
        })
}

fn format_split_name_counts(rows: &[crate::ProbabilityTrainingRow]) -> String {
    if rows.is_empty() {
        return "none".to_string();
    }
    let mut counts = BTreeMap::<&str, usize>::new();
    for row in rows {
        let split_name = row.split_name.as_deref().unwrap_or("unknown");
        *counts.entry(split_name).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .map(|(split_name, count)| format!("{split_name}:{count}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn print_training_artifacts_summary(artifacts: &PipelineArtifacts, options: &PipelineTrainOptions) {
    println!("Formal probability bundle generated.");
    println!("  dataset_source   {}", artifacts.dataset_source);
    println!("  dataset_label    {}", artifacts.dataset_label);
    println!("  model_shape      {}", options.model_shape.as_str());
    println!(
        "  manifest_mode    {}",
        options.release_manifest_mode.as_str()
    );
    println!(
        "  release_state    {}/{}",
        artifacts.release.manifest.status, artifacts.release.manifest.serving_status
    );
    println!(
        "  release_id       {}",
        artifacts.release.manifest.release_id
    );
    println!("  bundle_path      {}", artifacts.bundle_path.display());
    println!("  manifest_path    {}", artifacts.manifest_path.display());
    println!("  evaluation_path  {}", artifacts.evaluation_path.display());
    if let Some(summary) = artifacts.bundle.evaluation.as_ref() {
        println!(
            "  eval             brier={:.4} log_loss={:.4} ece={:.4}",
            summary.brier_score, summary.log_loss, summary.ece
        );
        println!(
            "  regime_eval      usable_early_warning_horizons={} insufficient_early_warning_horizons={}",
            summary.usable_early_warning_horizon_count,
            summary.insufficient_early_warning_horizon_count,
        );
        for regime in &summary.regime_separation_summaries {
            println!(
                "  regime_horizon   {:>2}d early={} positive_window={} cooldown={} in_crisis={} diagnosis={}",
                regime.horizon_days,
                regime.early_warning_regime,
                crate::format_optional_multiplier(regime.positive_window_lift_vs_normal),
                crate::format_optional_multiplier(regime.post_crisis_cooldown_lift_vs_normal),
                crate::format_optional_multiplier(regime.in_crisis_lift_vs_normal),
                regime.diagnosis,
            );
        }
    }
    for horizon in &artifacts.bundle.horizons {
        if let Some(diag) = horizon.threshold_diagnostics.as_ref() {
            println!(
                "  threshold_diag   {:>2}d base={:.3} final={:.3} repair={} reason={} selected_rows={} early_rows={}",
                horizon.horizon_days,
                diag.base_threshold,
                diag.final_threshold,
                diag.repair_applied,
                diag.repair_reason,
                diag.selected_row_count,
                diag.base_summary.early_warning_row_count,
            );
            println!(
                "                   base_hits early={}/{} normal={}/{} final_hits early={}/{} normal={}/{}",
                diag.base_summary.early_warning_hit_count,
                diag.base_summary.early_warning_row_count,
                diag.base_summary.normal_hit_count,
                diag.base_summary.normal_row_count,
                diag.final_summary.early_warning_hit_count,
                diag.final_summary.early_warning_row_count,
                diag.final_summary.normal_hit_count,
                diag.final_summary.normal_row_count,
            );
            println!(
                "                   regime_hits base positive={}/{} cooldown={}/{} in_crisis={}/{} final positive={}/{} cooldown={}/{} in_crisis={}/{}",
                diag.base_summary.positive_window_hit_count,
                diag.base_summary.positive_window_row_count,
                diag.base_summary.cooldown_hit_count,
                diag.base_summary.cooldown_row_count,
                diag.base_summary.in_crisis_hit_count,
                diag.base_summary.in_crisis_row_count,
                diag.final_summary.positive_window_hit_count,
                diag.final_summary.positive_window_row_count,
                diag.final_summary.cooldown_hit_count,
                diag.final_summary.cooldown_row_count,
                diag.final_summary.in_crisis_hit_count,
                diag.final_summary.in_crisis_row_count,
            );
            if let Some(early_evidence) = diag
                .calibration_regime_evidence
                .iter()
                .find(|row| row.regime == diag.early_warning_regime)
            {
                println!(
                    "                   calib_evidence early={} rows={}({}) eligible={} used={} selected={} hard={} soft={} weight={}",
                    early_evidence.regime,
                    early_evidence.full_row_count,
                    crate::format_pct(early_evidence.full_row_rate),
                    early_evidence.calibration_eligible_row_count,
                    early_evidence.calibration_used_row_count,
                    early_evidence.threshold_selected_row_count,
                    crate::format_pct(early_evidence.avg_hard_label),
                    crate::format_pct(early_evidence.avg_training_target),
                    early_evidence.avg_objective_weight,
                );
                if early_evidence.episode_native_objective_row_count > 0 {
                    println!(
                        "                   episode_native rows={}({}) protected_no_positive_main={}({}) target={} weight={}",
                        early_evidence.episode_native_objective_row_count,
                        crate::format_pct(early_evidence.episode_native_objective_row_rate),
                        early_evidence.protected_no_positive_main_row_count,
                        crate::format_pct(early_evidence.protected_no_positive_main_row_rate),
                        crate::format_pct(
                            early_evidence.protected_no_positive_main_avg_training_target
                        ),
                        early_evidence.protected_no_positive_main_avg_objective_weight,
                    );
                }
            }
        }
        let configured_overlay_ids = horizon
            .family_overlays
            .iter()
            .map(|overlay| overlay.family_id.as_str())
            .collect::<Vec<_>>();
        println!(
            "  overlay_diag     {:>2}d configured={} audits={} families={}",
            horizon.horizon_days,
            horizon.family_overlays.len(),
            horizon.family_overlay_audits.len(),
            if configured_overlay_ids.is_empty() {
                "none".to_string()
            } else {
                configured_overlay_ids.join(",")
            }
        );
        for audit in &horizon.family_overlay_audits {
            println!(
                "                   audit {} scenarios={} positives={} rows={}/{}/{} gate_active={}/{}/{} note={}",
                audit.family_id,
                audit.scenario_count,
                audit.positive_label_count,
                audit.train_row_count,
                audit.calibration_row_count,
                audit.evaluation_row_count,
                audit.train_gate_active_row_count,
                audit.calibration_gate_active_row_count,
                audit.evaluation_gate_active_row_count,
                audit.note,
            );
        }
    }
    if let Some(actionability) = artifacts.bundle.actionability.as_ref() {
        for level in &actionability.levels {
            if let Some(summary) = level.evaluation.actionability.as_ref() {
                println!(
                    "  actionability    {:>7} scenarios={} on_time={} late_only={} missed={}",
                    crate::actionability_level_text(level.level),
                    summary.scenario_count,
                    crate::format_pct(summary.advance_warning_rate.unwrap_or(0.0)),
                    crate::format_pct(summary.late_confirmation_rate.unwrap_or(0.0)),
                    crate::format_pct(summary.missed_rate.unwrap_or(0.0)),
                );
            }
        }
    }
}

use crate::training::PipelineArtifacts;

use super::{options::PipelineBootstrapOptions, PipelineTrainOptions};

pub(crate) async fn research_pipeline_train_probability(args: &[String]) -> anyhow::Result<()> {
    let options = PipelineTrainOptions::parse(args)?;
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;
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

fn print_training_artifacts_summary(artifacts: &PipelineArtifacts, options: &PipelineTrainOptions) {
    println!("Formal probability bundle generated.");
    println!("  dataset_source   {}", artifacts.dataset_source);
    println!("  dataset_label    {}", artifacts.dataset_label);
    println!("  model_shape      {}", options.model_shape.as_str());
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

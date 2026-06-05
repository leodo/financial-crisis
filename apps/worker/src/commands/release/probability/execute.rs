use std::{collections::BTreeMap, path::PathBuf};

use anyhow::{bail, Context};
use chrono::NaiveDate;
use fc_domain::{HistoricalAssessmentPointRecord, ModelReleaseRecord, ProbabilityBundle};

use super::super::ReleaseReviewOptions;
use super::compare::{
    build_release_formal_probability_compare_export,
    print_release_formal_probability_compare_summary,
    write_release_formal_probability_compare_report, ReleaseFormalProbabilityCompareBuildInput,
};
use super::formal::{
    build_release_formal_probability_slice_export, print_release_formal_probability_slice_summary,
    score_release_formal_probability_slice_rows, write_release_formal_probability_slice_report,
    ReleaseFormalProbabilitySliceBuildInput,
};
use super::options::{
    ReleaseFormalProbabilityCompareOptions, ReleaseFormalProbabilitySliceOptions,
    ReleaseProbabilitySliceOptions,
};
use super::slice::{
    build_release_probability_slice_export, print_release_probability_slice_summary,
    write_release_probability_slice_report, ReleaseProbabilitySliceBuildInput,
};

pub(crate) async fn research_release_probability_slice(args: &[String]) -> anyhow::Result<()> {
    let options = ReleaseProbabilitySliceOptions::parse(args)?;
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;

    let target_release = store
        .load_model_release(&options.release_id)
        .await?
        .with_context(|| format!("release {} not found", options.release_id))?;
    let market_scope = options
        .market_scope
        .clone()
        .unwrap_or_else(|| target_release.manifest.market_scope.clone());
    if target_release.manifest.market_scope != market_scope {
        bail!(
            "release {} belongs to {}, not {}",
            target_release.manifest.release_id,
            target_release.manifest.market_scope,
            market_scope
        );
    }

    let original_active = store
        .load_active_model_release(&market_scope)
        .await?
        .with_context(|| format!("no active release found for market scope {market_scope}"))?;

    let review_options = ReleaseReviewOptions {
        candidate_release_id: target_release.manifest.release_id.clone(),
        baseline_release_id: None,
        market_scope: Some(market_scope.clone()),
        api_reload_url: options.api_reload_url.clone(),
        output_dir: PathBuf::from(crate::DEFAULT_RELEASE_REVIEW_OUTPUT_DIR),
        history_mode: options.history_mode,
        history_limit: options.history_limit,
        updated_by: options.updated_by.clone(),
    };

    let mut original_records = BTreeMap::<String, ModelReleaseRecord>::new();
    for release in [original_active.clone(), target_release.clone()] {
        original_records.insert(release.manifest.release_id.clone(), release);
    }

    let export_result = async {
        super::super::activate_release_for_review(
            &store,
            &market_scope,
            &target_release.manifest.release_id,
            &review_options,
            "probability-slice",
        )
        .await?;
        let (run, points) = load_release_probability_slice_points(
            &store,
            &market_scope,
            &target_release.manifest.release_id,
            options.from_date,
            options.to_date,
        )
        .await?;
        let export = build_release_probability_slice_export(ReleaseProbabilitySliceBuildInput {
            market_scope: &market_scope,
            release_id: &target_release.manifest.release_id,
            replay_run_id: &run.replay_run_id,
            history_mode: options.history_mode.as_label(),
            history_limit: options.history_limit,
            from_date: options.from_date,
            to_date: options.to_date,
            points,
        });
        write_release_probability_slice_report(&options.output_dir, &export)?;
        print_release_probability_slice_summary(&export);
        Ok::<(), anyhow::Error>(())
    }
    .await;

    let restore_result = super::super::restore_release_review_state(
        &store,
        &market_scope,
        &original_active.manifest.release_id,
        &original_records,
        &options.api_reload_url,
        &options.updated_by,
    )
    .await;

    if let Err(restore_error) = restore_result {
        if let Err(export_error) = export_result {
            bail!(
                "release probability slice export failed and restore also failed:\nexport: {export_error:#}\nrestore: {restore_error:#}"
            );
        }
        bail!("release probability slice export completed but restore failed: {restore_error:#}");
    }

    export_result?;
    println!(
        "Release probability slice restored original active release {}.",
        original_active.manifest.release_id
    );
    Ok(())
}

pub(crate) async fn research_release_formal_probability_slice(
    args: &[String],
) -> anyhow::Result<()> {
    let options = ReleaseFormalProbabilitySliceOptions::parse(args)?;
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;

    let release = store
        .load_model_release(&options.release_id)
        .await?
        .with_context(|| format!("release {} not found", options.release_id))?;
    let market_scope = options
        .market_scope
        .clone()
        .unwrap_or_else(|| release.manifest.market_scope.clone());
    if release.manifest.market_scope != market_scope {
        bail!(
            "release {} belongs to {}, not {}",
            release.manifest.release_id,
            release.manifest.market_scope,
            market_scope
        );
    }

    let dataset_key = super::super::super::pipeline::resolve_formal_dataset_key(
        &store,
        options.dataset_key.as_deref(),
        &options.dataset_id,
        options.dataset_version.as_deref(),
        Some(&market_scope),
    )
    .await?;
    let rows = store
        .list_formal_dataset_rows(&dataset_key, None, None)
        .await?
        .into_iter()
        .filter(|row| row.as_of_date >= options.from_date && row.as_of_date <= options.to_date)
        .filter(|row| {
            options
                .scenario_id
                .as_deref()
                .map(|scenario_id| row.primary_scenario_id.as_deref() == Some(scenario_id))
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();
    if rows.is_empty() {
        bail!(
            "formal dataset {} has no rows in {} -> {} for scenario {}",
            dataset_key,
            options.from_date,
            options.to_date,
            options.scenario_id.as_deref().unwrap_or("all")
        );
    }

    let bundle = read_release_probability_bundle(&release)?;
    let export =
        build_release_formal_probability_slice_export(ReleaseFormalProbabilitySliceBuildInput {
            market_scope: &market_scope,
            release_id: &release.manifest.release_id,
            dataset_key: &dataset_key,
            scenario_id: options.scenario_id.clone(),
            from_date: options.from_date,
            to_date: options.to_date,
            bundle: &bundle,
            rows,
        });
    write_release_formal_probability_slice_report(&options.output_dir, &export)?;
    print_release_formal_probability_slice_summary(&export);
    Ok(())
}

pub(crate) async fn research_release_formal_probability_compare(
    args: &[String],
) -> anyhow::Result<()> {
    let options = ReleaseFormalProbabilityCompareOptions::parse(args)?;
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;

    let baseline_release = store
        .load_model_release(&options.baseline_release_id)
        .await?
        .with_context(|| format!("release {} not found", options.baseline_release_id))?;
    let candidate_release = store
        .load_model_release(&options.candidate_release_id)
        .await?
        .with_context(|| format!("release {} not found", options.candidate_release_id))?;
    let market_scope = options
        .market_scope
        .clone()
        .unwrap_or_else(|| baseline_release.manifest.market_scope.clone());
    for release in [&baseline_release, &candidate_release] {
        if release.manifest.market_scope != market_scope {
            bail!(
                "release {} belongs to {}, not {}",
                release.manifest.release_id,
                release.manifest.market_scope,
                market_scope
            );
        }
    }

    let dataset_key = super::super::super::pipeline::resolve_formal_dataset_key(
        &store,
        options.dataset_key.as_deref(),
        &options.dataset_id,
        options.dataset_version.as_deref(),
        Some(&market_scope),
    )
    .await?;
    let rows = store
        .list_formal_dataset_rows(&dataset_key, None, None)
        .await?
        .into_iter()
        .filter(|row| row.as_of_date >= options.from_date && row.as_of_date <= options.to_date)
        .filter(|row| {
            options
                .scenario_id
                .as_deref()
                .map(|scenario_id| row.primary_scenario_id.as_deref() == Some(scenario_id))
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();
    if rows.is_empty() {
        bail!(
            "formal dataset {} has no rows in {} -> {} for scenario {}",
            dataset_key,
            options.from_date,
            options.to_date,
            options.scenario_id.as_deref().unwrap_or("all")
        );
    }

    let baseline_bundle = read_release_probability_bundle(&baseline_release)?;
    let candidate_bundle = read_release_probability_bundle(&candidate_release)?;
    let baseline_rows = score_release_formal_probability_slice_rows(&baseline_bundle, rows.clone());
    let candidate_rows = score_release_formal_probability_slice_rows(&candidate_bundle, rows);
    let export = build_release_formal_probability_compare_export(
        ReleaseFormalProbabilityCompareBuildInput {
            market_scope: &market_scope,
            dataset_key: &dataset_key,
            scenario_id: options.scenario_id.clone(),
            from_date: options.from_date,
            to_date: options.to_date,
            baseline_release_id: &baseline_release.manifest.release_id,
            candidate_release_id: &candidate_release.manifest.release_id,
            baseline_bundle: &baseline_bundle,
            candidate_bundle: &candidate_bundle,
            baseline_rows,
            candidate_rows,
        },
    )?;
    write_release_formal_probability_compare_report(&options.output_dir, &export)?;
    print_release_formal_probability_compare_summary(&export);
    Ok(())
}

async fn load_release_probability_slice_points(
    store: &fc_storage::SqliteStore,
    market_scope: &str,
    release_id: &str,
    from_date: NaiveDate,
    to_date: NaiveDate,
) -> anyhow::Result<(
    fc_domain::HistoricalReplayRunRecord,
    Vec<HistoricalAssessmentPointRecord>,
)> {
    let run = store
        .list_historical_replay_runs(
            Some(market_scope),
            Some(release_id),
            Some(from_date),
            Some(to_date),
            Some(20),
        )
        .await?
        .into_iter()
        .find(|run| run.from_date <= from_date && run.to_date >= to_date)
        .with_context(|| {
            format!(
                "no historical replay run covering {from_date} -> {to_date} was found for release {release_id} in {market_scope}; reload the API with strict_rebuild first"
            )
        })?;
    let points = store
        .list_historical_assessment_points(
            Some(&run.replay_run_id),
            Some(market_scope),
            Some(release_id),
            Some(from_date),
            Some(to_date),
            None,
        )
        .await?;
    let mut latest_by_date = BTreeMap::<NaiveDate, HistoricalAssessmentPointRecord>::new();
    for point in points {
        latest_by_date
            .entry(point.as_of_date)
            .and_modify(|existing| {
                if point.generated_at > existing.generated_at {
                    *existing = point.clone();
                }
            })
            .or_insert(point);
    }
    let points = latest_by_date.into_values().collect::<Vec<_>>();
    if points.is_empty() {
        bail!(
            "historical replay run {} exists but produced no points in {} -> {} for release {}",
            run.replay_run_id,
            from_date,
            to_date,
            release_id
        );
    }
    Ok((run, points))
}

fn read_release_probability_bundle(
    release: &ModelReleaseRecord,
) -> anyhow::Result<ProbabilityBundle> {
    let bundle_path = release
        .manifest
        .bundle_uri
        .strip_prefix("file://")
        .unwrap_or(&release.manifest.bundle_uri);
    crate::read_probability_bundle(std::path::Path::new(bundle_path))
}

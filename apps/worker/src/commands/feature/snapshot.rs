use std::collections::{BTreeMap, BTreeSet};

use anyhow::bail;
use chrono::{NaiveDate, Utc};
use fc_domain::{
    build_formal_observation_feature_map, formal_feature_snapshot_visibility_status,
    FeatureSnapshotRecord, Frequency, Indicator, Observation, RiskDimension,
};
use fc_scoring::ScoringEngine;
use fc_storage::SqliteStore;

use super::coverage::{coverage_summary, find_dimension_score};
use super::options::{FeatureSnapshotBuildOptions, FeatureSnapshotListOptions, PointInTimeMode};
use super::visibility::observation_is_visible_for_date;

pub(crate) async fn research_feature_snapshot_build(args: &[String]) -> anyhow::Result<()> {
    let options = FeatureSnapshotBuildOptions::parse(args)?;
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;
    let (indicators, observations) = load_formal_feature_inputs(&store, options.to).await?;
    let snapshot_build =
        build_or_load_feature_snapshots(&store, &indicators, &observations, &options).await?;
    let snapshots = snapshot_build.snapshots;
    if snapshots.is_empty() {
        bail!("no feature snapshots were generated for the requested range");
    }
    let ready_count = snapshots
        .iter()
        .filter(|snapshot| snapshot.visibility_status == crate::FEATURE_SNAPSHOT_STATUS_READY)
        .count();
    let blocked_count = snapshots.len().saturating_sub(ready_count);
    store.upsert_feature_snapshots(&snapshots).await?;
    let first_date = snapshots.first().map(|snapshot| snapshot.as_of_date);
    let last_date = snapshots.last().map(|snapshot| snapshot.as_of_date);
    println!(
        "Built {} feature snapshots for {} ({} -> {}, feature_set_version={}, ready={}, blocked={}).",
        snapshots.len(),
        options.market_scope,
        first_date
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string()),
        last_date
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string()),
        options.feature_set_version,
        ready_count,
        blocked_count
    );
    println!(
        "  reused={} recomputed={} pit={} force_rebuild={}",
        snapshot_build.reused_count,
        snapshot_build.recomputed_count,
        options.point_in_time_mode,
        options.force_rebuild
    );
    Ok(())
}

pub(crate) async fn research_feature_snapshot_list(args: &[String]) -> anyhow::Result<()> {
    let options = FeatureSnapshotListOptions::parse(args)?;
    let store = crate::open_sqlite_store().await?;
    store.migrate().await?;
    let snapshots = store
        .list_feature_snapshots(
            options.market_scope.as_deref(),
            options.feature_set_version.as_deref(),
            options.from,
            options.to,
            options.limit,
        )
        .await?;
    if snapshots.is_empty() {
        println!("No feature snapshots found.");
        return Ok(());
    }

    for snapshot in snapshots {
        println!(
            "[{}] {} {} pit={} status={} coverage={:.3} core={:.3} trigger={:.3} external={:.3} features={} latest_visible_at={}",
            snapshot.as_of_date,
            snapshot.market_scope,
            snapshot.feature_set_version,
            snapshot.point_in_time_mode,
            snapshot.visibility_status,
            snapshot.coverage_score,
            snapshot.core_feature_coverage,
            snapshot.trigger_feature_coverage,
            snapshot.external_feature_coverage,
            snapshot.feature_count,
            snapshot
                .latest_visible_at
                .map(|value| value.to_rfc3339())
                .unwrap_or_else(|| "-".to_string())
        );
    }
    Ok(())
}

pub(crate) async fn load_formal_feature_inputs(
    store: &SqliteStore,
    to: Option<NaiveDate>,
) -> anyhow::Result<(Vec<Indicator>, Vec<Observation>)> {
    let indicators = store.load_indicators().await?;
    let upper_bound = to.unwrap_or_else(|| Utc::now().date_naive());
    let observations = store
        .load_observations_for_entities(&["us", "jp"], upper_bound)
        .await?;
    if observations.is_empty() {
        bail!("no observations found in SQLite; run bootstrap/backfill first");
    }
    Ok((indicators, observations))
}

#[derive(Debug, Clone)]
pub(crate) struct FeatureSnapshotBuildResult {
    pub(crate) snapshots: Vec<FeatureSnapshotRecord>,
    pub(crate) reused_count: usize,
    pub(crate) recomputed_count: usize,
}

pub(crate) async fn build_or_load_feature_snapshots(
    store: &SqliteStore,
    indicators: &[Indicator],
    observations: &[Observation],
    options: &FeatureSnapshotBuildOptions,
) -> anyhow::Result<FeatureSnapshotBuildResult> {
    let target_dates = formal_feature_dates(observations, options.from, options.to);
    if target_dates.is_empty() {
        return Ok(FeatureSnapshotBuildResult {
            snapshots: Vec::new(),
            reused_count: 0,
            recomputed_count: 0,
        });
    }

    let reusable = if options.force_rebuild {
        BTreeMap::new()
    } else {
        load_reusable_feature_snapshots(store, options).await?
    };

    let missing_dates = target_dates
        .iter()
        .copied()
        .filter(|date| !reusable.contains_key(date))
        .collect::<Vec<_>>();
    let recomputed = build_formal_feature_snapshots_for_dates(
        indicators,
        observations,
        options,
        &missing_dates,
    )?;

    let mut combined = reusable.into_values().chain(recomputed).collect::<Vec<_>>();
    combined.sort_by_key(|snapshot| snapshot.as_of_date);

    Ok(FeatureSnapshotBuildResult {
        reused_count: combined.len().saturating_sub(missing_dates.len()),
        recomputed_count: missing_dates.len(),
        snapshots: combined,
    })
}

async fn load_reusable_feature_snapshots(
    store: &SqliteStore,
    options: &FeatureSnapshotBuildOptions,
) -> anyhow::Result<BTreeMap<NaiveDate, FeatureSnapshotRecord>> {
    let rows = store
        .list_feature_snapshots_for_mode(
            &options.market_scope,
            &options.feature_set_version,
            &options.point_in_time_mode,
            options.from,
            options.to,
        )
        .await?;
    let reusable = rows
        .into_iter()
        .filter(feature_snapshot_status_is_current)
        .fold(BTreeMap::new(), |mut acc, snapshot| {
            acc.entry(snapshot.as_of_date).or_insert(snapshot);
            acc
        });
    Ok(reusable)
}

fn feature_snapshot_status_is_current(snapshot: &FeatureSnapshotRecord) -> bool {
    matches!(
        snapshot.visibility_status.as_str(),
        crate::FEATURE_SNAPSHOT_STATUS_READY
            | crate::FEATURE_SNAPSHOT_STATUS_COVERAGE_OR_VISIBILITY_FAILED
    )
}

fn build_formal_feature_snapshots_for_dates(
    indicators: &[Indicator],
    observations: &[Observation],
    options: &FeatureSnapshotBuildOptions,
    dates: &[NaiveDate],
) -> anyhow::Result<Vec<FeatureSnapshotRecord>> {
    let scoring = ScoringEngine::default();
    let mut snapshots = Vec::with_capacity(dates.len());
    for as_of_date in dates.iter().copied() {
        snapshots.push(build_formal_feature_snapshot_for_date(
            indicators,
            observations,
            &scoring,
            as_of_date,
            options,
        )?);
    }
    Ok(snapshots)
}

pub(crate) fn build_formal_feature_snapshot_for_date(
    indicators: &[Indicator],
    observations: &[Observation],
    scoring: &ScoringEngine,
    as_of_date: NaiveDate,
    options: &FeatureSnapshotBuildOptions,
) -> anyhow::Result<FeatureSnapshotRecord> {
    let point_in_time_mode = PointInTimeMode::parse(&options.point_in_time_mode)?;
    let output = scoring.score_with_observation_filter(
        indicators,
        observations,
        as_of_date,
        "us",
        &options.market_scope,
        |observation| observation_is_visible_for_date(observation, as_of_date, point_in_time_mode),
    );
    let observation_feature_map =
        build_formal_observation_feature_map(observations, as_of_date, point_in_time_mode.as_str());
    let latest_visible_at = observation_feature_map.latest_visible_at;
    let mut features = observation_feature_map.features;

    features.insert(
        "overall_score".to_string(),
        crate::round6((output.snapshot.overall_score / 100.0).clamp(0.0, 1.0)),
    );
    features.insert(
        "structural_score".to_string(),
        crate::round6((output.snapshot.structural_score / 100.0).clamp(0.0, 1.0)),
    );
    features.insert(
        "trigger_score".to_string(),
        crate::round6((output.snapshot.trigger_score / 100.0).clamp(0.0, 1.0)),
    );
    features.insert(
        "external_dimension_score".to_string(),
        crate::round6(
            (find_dimension_score(&output.indicator_risks, RiskDimension::ExternalSector) / 100.0)
                .clamp(0.0, 1.0),
        ),
    );

    let (
        core_feature_coverage,
        trigger_feature_coverage,
        external_feature_coverage,
        coverage_score,
    ) = coverage_summary(&output.indicator_risks, as_of_date);
    let visibility_status =
        formal_feature_snapshot_visibility_status(&features, coverage_score, latest_visible_at);

    Ok(FeatureSnapshotRecord {
        as_of_date,
        entity_id: "us".to_string(),
        market_scope: options.market_scope.clone(),
        feature_set_version: options.feature_set_version.clone(),
        point_in_time_mode: options.point_in_time_mode.clone(),
        visibility_status: visibility_status.to_string(),
        latest_visible_at,
        coverage_score,
        core_feature_coverage,
        trigger_feature_coverage,
        external_feature_coverage,
        feature_count: features.len(),
        features,
        created_at: Utc::now(),
    })
}

pub(crate) fn formal_feature_dates(
    observations: &[Observation],
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
) -> Vec<NaiveDate> {
    let mut dates = observations
        .iter()
        .filter(|observation| {
            matches!(observation.frequency, Frequency::Daily | Frequency::Event)
                && (observation.entity_id == "us"
                    || matches!(
                        observation.indicator_id.as_str(),
                        "us_external_usdjpy_level" | "jp_rates_call_rate"
                    ))
        })
        .map(|observation| observation.as_of_date)
        .collect::<BTreeSet<_>>();
    if dates.is_empty() {
        dates.extend(
            observations
                .iter()
                .filter(|observation| observation.entity_id == "us")
                .map(|observation| observation.as_of_date),
        );
    }
    let mut dates = dates.into_iter().collect::<Vec<_>>();
    if let Some(from) = from {
        dates.retain(|date| *date >= from);
    }
    if let Some(to) = to {
        dates.retain(|date| *date <= to);
    }
    dates.sort();
    dates
}

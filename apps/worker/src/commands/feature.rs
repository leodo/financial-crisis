use std::collections::BTreeMap;

use anyhow::{bail, Context};
use chrono::{NaiveDate, Utc};
use fc_domain::{FeatureSnapshotRecord, Indicator, Observation};
use fc_scoring::ScoringEngine;
use fc_storage::SqliteStore;

#[derive(Debug, Clone)]
pub(crate) struct FeatureSnapshotBuildOptions {
    pub(crate) market_scope: String,
    pub(crate) from: Option<NaiveDate>,
    pub(crate) to: Option<NaiveDate>,
    pub(crate) feature_set_version: String,
    pub(crate) point_in_time_mode: String,
    pub(crate) force_rebuild: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PointInTimeMode {
    BestEffort,
    Strict,
}

impl PointInTimeMode {
    pub(crate) fn parse(raw: &str) -> anyhow::Result<Self> {
        match raw {
            "best_effort" => Ok(Self::BestEffort),
            "strict" => Ok(Self::Strict),
            other => bail!(
                "unsupported --point-in-time-mode value: {other}; supported values are best_effort and strict"
            ),
        }
    }
}

impl FeatureSnapshotBuildOptions {
    pub(crate) fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut market_scope = "financial_system".to_string();
        let mut from = None;
        let mut to = None;
        let mut feature_set_version = crate::DEFAULT_FORMAL_FEATURE_SET_VERSION.to_string();
        let mut point_in_time_mode = "best_effort".to_string();
        let mut force_rebuild = false;
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--market-scope" => {
                    index += 1;
                    market_scope = args
                        .get(index)
                        .with_context(|| "--market-scope requires a value")?
                        .clone();
                }
                "--from" => {
                    index += 1;
                    from = Some(crate::parse_date_arg(args.get(index), "--from")?);
                }
                "--to" => {
                    index += 1;
                    to = Some(crate::parse_date_arg(args.get(index), "--to")?);
                }
                "--feature-set-version" => {
                    index += 1;
                    feature_set_version = args
                        .get(index)
                        .with_context(|| "--feature-set-version requires a value")?
                        .clone();
                }
                "--point-in-time-mode" => {
                    index += 1;
                    point_in_time_mode = args
                        .get(index)
                        .with_context(|| "--point-in-time-mode requires a value")?
                        .clone();
                }
                "--force-rebuild" => {
                    force_rebuild = true;
                }
                other => bail!("unknown feature snapshot build option: {other}"),
            }
            index += 1;
        }
        PointInTimeMode::parse(&point_in_time_mode)?;
        Ok(Self {
            market_scope,
            from,
            to,
            feature_set_version,
            point_in_time_mode,
            force_rebuild,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FeatureSnapshotListOptions {
    pub(crate) market_scope: Option<String>,
    pub(crate) feature_set_version: Option<String>,
    pub(crate) from: Option<NaiveDate>,
    pub(crate) to: Option<NaiveDate>,
    pub(crate) limit: Option<usize>,
}

impl FeatureSnapshotListOptions {
    pub(crate) fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut market_scope = None;
        let mut feature_set_version = None;
        let mut from = None;
        let mut to = None;
        let mut limit = Some(20_usize);
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--market-scope" => {
                    index += 1;
                    market_scope = Some(
                        args.get(index)
                            .with_context(|| "--market-scope requires a value")?
                            .clone(),
                    );
                }
                "--feature-set-version" => {
                    index += 1;
                    feature_set_version = Some(
                        args.get(index)
                            .with_context(|| "--feature-set-version requires a value")?
                            .clone(),
                    );
                }
                "--from" => {
                    index += 1;
                    from = Some(crate::parse_date_arg(args.get(index), "--from")?);
                }
                "--to" => {
                    index += 1;
                    to = Some(crate::parse_date_arg(args.get(index), "--to")?);
                }
                "--limit" => {
                    index += 1;
                    limit = Some(
                        args.get(index)
                            .with_context(|| "--limit requires a number")?
                            .parse::<usize>()
                            .context("--limit must be an integer")?,
                    );
                }
                other => bail!("unknown feature snapshot list option: {other}"),
            }
            index += 1;
        }
        Ok(Self {
            market_scope,
            feature_set_version,
            from,
            to,
            limit,
        })
    }
}

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
    let target_dates = crate::formal_feature_dates(observations, options.from, options.to);
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
        snapshots.push(crate::build_formal_feature_snapshot_for_date(
            indicators,
            observations,
            &scoring,
            as_of_date,
            options,
        )?);
    }
    Ok(snapshots)
}

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{bail, Context};
use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc, Weekday};
use fc_domain::{
    formal_observation_feature_value_from_history, observation_history_for_indicator_where,
    FeatureSnapshotRecord, FormalObservationFeatureTransform, Frequency, Indicator, IndicatorRisk,
    Observation, RiskDimension, FORMAL_OBSERVATION_FEATURE_SPECS,
};
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
    let mut features = BTreeMap::new();
    let mut visible_candidates = Vec::new();

    insert_formal_observation_features(
        &mut features,
        &mut visible_candidates,
        observations,
        as_of_date,
        point_in_time_mode,
    );

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
    ) = coverage_summary(&output.indicator_risks);
    let latest_visible_at = visible_candidates.into_iter().max();
    let visibility_status =
        feature_snapshot_visibility_status(&features, coverage_score, latest_visible_at);

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

fn observations_for_indicator<'a>(
    observations: &'a [Observation],
    indicator_id: &str,
    as_of_date: NaiveDate,
    point_in_time_mode: PointInTimeMode,
) -> Vec<&'a Observation> {
    observation_history_for_indicator_where(observations, indicator_id, as_of_date, |observation| {
        observation_is_visible_for_date(observation, as_of_date, point_in_time_mode)
    })
}

fn insert_formal_observation_features(
    features: &mut BTreeMap<String, f64>,
    visible_candidates: &mut Vec<DateTime<Utc>>,
    observations: &[Observation],
    as_of_date: NaiveDate,
    point_in_time_mode: PointInTimeMode,
) {
    for spec in FORMAL_OBSERVATION_FEATURE_SPECS {
        let history = observations_for_indicator(
            observations,
            spec.indicator_id,
            as_of_date,
            point_in_time_mode,
        );
        if let Some(value) = formal_observation_feature_value_from_history(&history, spec.transform)
        {
            features.insert(spec.feature_name.to_string(), crate::round6(value));
        }
        if matches!(spec.transform, FormalObservationFeatureTransform::Latest) {
            if let Some(latest) = history.last() {
                if let Some(visible_at) =
                    observation_visible_at_for_mode(latest, point_in_time_mode)
                {
                    visible_candidates.push(visible_at);
                }
            }
        }
    }
}

fn feature_snapshot_visibility_status(
    features: &BTreeMap<String, f64>,
    coverage_score: f64,
    latest_visible_at: Option<DateTime<Utc>>,
) -> &'static str {
    if latest_visible_at.is_none()
        || coverage_score < 0.70
        || !has_main_dataset_core_features(features)
    {
        crate::FEATURE_SNAPSHOT_STATUS_COVERAGE_OR_VISIBILITY_FAILED
    } else {
        crate::FEATURE_SNAPSHOT_STATUS_READY
    }
}

pub(crate) fn observation_is_visible_for_date(
    observation: &Observation,
    as_of_date: NaiveDate,
    point_in_time_mode: PointInTimeMode,
) -> bool {
    observation_visible_at_for_mode(observation, point_in_time_mode)
        .map(|visible_at| visible_at <= assessment_cutoff_utc(as_of_date))
        .unwrap_or(false)
}

fn observation_visible_at_for_mode(
    observation: &Observation,
    point_in_time_mode: PointInTimeMode,
) -> Option<DateTime<Utc>> {
    match point_in_time_mode {
        PointInTimeMode::BestEffort => best_effort_visible_at(observation),
        PointInTimeMode::Strict => strict_visible_at(observation),
    }
}

fn best_effort_visible_at(observation: &Observation) -> Option<DateTime<Utc>> {
    let anchor_date = observation.period_end.unwrap_or(observation.as_of_date);
    match observation.source_id.as_str() {
        "treasury" => Some(new_york_time_to_utc(anchor_date, 18, 0)),
        "world_bank" => anchor_date
            .checked_add_signed(Duration::days(270))
            .map(|date| new_york_time_to_utc(date, 17, 30)),
        "boj" => Some(tokyo_time_to_utc(anchor_date, 17, 0)),
        "sec_edgar" => Some(
            observation
                .publication_time
                .unwrap_or_else(|| new_york_time_to_utc(anchor_date, 18, 0)),
        ),
        "gdelt" => None,
        "mock" => Some(
            observation
                .publication_time
                .unwrap_or_else(|| new_york_time_to_utc(anchor_date, 17, 30)),
        ),
        _ => anchor_date
            .checked_add_signed(Duration::days(default_visibility_lag_days(
                observation.frequency,
            )))
            .map(|date| new_york_time_to_utc(date, 17, 30)),
    }
}

fn strict_visible_at(observation: &Observation) -> Option<DateTime<Utc>> {
    match observation.source_id.as_str() {
        "sec_edgar" | "mock" => observation.publication_time,
        _ => None,
    }
}

fn default_visibility_lag_days(frequency: Frequency) -> i64 {
    match frequency {
        Frequency::Daily | Frequency::Event => 0,
        Frequency::Weekly => 3,
        Frequency::Monthly => 15,
        Frequency::Quarterly => 45,
        Frequency::Annual => 270,
    }
}

fn assessment_cutoff_utc(as_of_date: NaiveDate) -> DateTime<Utc> {
    new_york_time_to_utc(as_of_date, 17, 30)
}

fn new_york_time_to_utc(date: NaiveDate, hour: u32, minute: u32) -> DateTime<Utc> {
    let utc_offset_hours = if is_new_york_dst(date) { 4 } else { 5 };
    let local = date
        .and_hms_opt(hour, minute, 0)
        .expect("local wall-clock timestamp must be valid");
    DateTime::<Utc>::from_naive_utc_and_offset(local + Duration::hours(utc_offset_hours), Utc)
}

fn tokyo_time_to_utc(date: NaiveDate, hour: u32, minute: u32) -> DateTime<Utc> {
    let local = date
        .and_hms_opt(hour, minute, 0)
        .expect("tokyo wall-clock timestamp must be valid");
    DateTime::<Utc>::from_naive_utc_and_offset(local - Duration::hours(9), Utc)
}

fn is_new_york_dst(date: NaiveDate) -> bool {
    let year = date.year();
    let (start, end) = if year >= 2007 {
        (
            nth_weekday_of_month(year, 3, Weekday::Sun, 2),
            nth_weekday_of_month(year, 11, Weekday::Sun, 1),
        )
    } else {
        (
            nth_weekday_of_month(year, 4, Weekday::Sun, 1),
            last_weekday_of_month(year, 10, Weekday::Sun),
        )
    };
    date >= start && date < end
}

fn nth_weekday_of_month(year: i32, month: u32, weekday: Weekday, nth: u32) -> NaiveDate {
    let first_day = NaiveDate::from_ymd_opt(year, month, 1).expect("valid calendar date");
    let first_weekday_offset = (7 + weekday.num_days_from_monday() as i64
        - first_day.weekday().num_days_from_monday() as i64)
        % 7;
    first_day
        .checked_add_signed(Duration::days(
            first_weekday_offset + 7 * i64::from(nth - 1),
        ))
        .expect("nth weekday must be representable")
}

fn last_weekday_of_month(year: i32, month: u32, weekday: Weekday) -> NaiveDate {
    let next_month = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).expect("valid calendar date")
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).expect("valid calendar date")
    };
    let last_day = next_month
        .checked_sub_signed(Duration::days(1))
        .expect("previous day must be valid");
    let backward_offset = (7 + last_day.weekday().num_days_from_monday() as i64
        - weekday.num_days_from_monday() as i64)
        % 7;
    last_day
        .checked_sub_signed(Duration::days(backward_offset))
        .expect("last weekday must be representable")
}

fn coverage_summary(indicator_risks: &[IndicatorRisk]) -> (f64, f64, f64, f64) {
    const FORMAL_CORE_INDICATORS: &[&str] = &[
        "us_market_vix_close",
        "us_rates_yield_curve_10y2y",
        "us_credit_baa_10y_spread",
        "us_liquidity_effr",
        "us_liquidity_national_financial_conditions",
        "us_liquidity_financial_stress_stl",
        "us_macro_unemployment_rate",
        "us_real_estate_housing_starts",
    ];
    const FORMAL_TRIGGER_INDICATORS: &[&str] = &[
        "us_market_vix_close",
        "us_rates_yield_curve_10y2y",
        "us_credit_baa_10y_spread",
        "us_liquidity_effr",
        "us_liquidity_national_financial_conditions",
        "us_liquidity_financial_stress_stl",
    ];
    const FORMAL_EXTERNAL_INDICATORS: &[&str] = &["us_external_usdjpy_level", "jp_rates_call_rate"];

    let (core_total, core_present) =
        coverage_by_indicator_ids(indicator_risks, FORMAL_CORE_INDICATORS);
    let (trigger_total, trigger_present) =
        coverage_by_indicator_ids(indicator_risks, FORMAL_TRIGGER_INDICATORS);
    let (external_total, external_present) =
        coverage_by_indicator_ids(indicator_risks, FORMAL_EXTERNAL_INDICATORS);

    let core_feature_coverage = ratio(core_present, core_total);
    let trigger_feature_coverage = ratio(trigger_present, trigger_total);
    let external_feature_coverage = ratio(external_present, external_total);
    let coverage_score = crate::round3(
        (core_feature_coverage * 0.45
            + trigger_feature_coverage * 0.35
            + external_feature_coverage * 0.2)
            .clamp(0.0, 1.0),
    );
    (
        crate::round3(core_feature_coverage),
        crate::round3(trigger_feature_coverage),
        crate::round3(external_feature_coverage),
        coverage_score,
    )
}

fn coverage_by_indicator_ids(
    indicator_risks: &[IndicatorRisk],
    indicator_ids: &[&str],
) -> (usize, usize) {
    indicator_risks
        .iter()
        .filter(|risk| indicator_ids.contains(&risk.indicator.indicator_id.as_str()))
        .fold((0_usize, 0_usize), |(total, present), risk| {
            (
                total + 1,
                present + usize::from(risk.latest_observation.is_some()),
            )
        })
}

fn ratio(present: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        present as f64 / total as f64
    }
}

fn find_dimension_score(indicator_risks: &[IndicatorRisk], dimension: RiskDimension) -> f64 {
    let scores = indicator_risks
        .iter()
        .filter(|risk| risk.indicator.dimension == dimension)
        .filter(|risk| risk.latest_observation.is_some())
        .map(|risk| risk.score)
        .collect::<Vec<_>>();
    if scores.is_empty() {
        0.0
    } else {
        scores.iter().sum::<f64>() / scores.len() as f64
    }
}

pub(crate) fn has_main_dataset_core_features(features: &BTreeMap<String, f64>) -> bool {
    [
        "us_vix_level",
        "us_curve_10y2y_level",
        "us_baa_10y_spread_level",
        "us_fed_funds_level",
    ]
    .into_iter()
    .all(|feature| features.contains_key(feature))
}

pub(crate) fn has_extension_acute_core_features(features: &BTreeMap<String, f64>) -> bool {
    [
        "us_curve_10y2y_level",
        "us_baa_10y_spread_level",
        "us_fed_funds_level",
        "us_usdjpy_level",
    ]
    .into_iter()
    .all(|feature| features.contains_key(feature))
}

pub(crate) fn feature_quality_grade(coverage_score: f64) -> &'static str {
    if coverage_score >= 0.9 {
        "a"
    } else if coverage_score >= 0.8 {
        "b"
    } else if coverage_score >= 0.7 {
        "c"
    } else if coverage_score >= 0.6 {
        "d"
    } else {
        "f"
    }
}

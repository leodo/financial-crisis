use std::collections::BTreeSet;

use chrono::NaiveDate;
use fc_domain::{
    AlertEvent, BacktestWindowPoint, DataMode, Indicator, Observation, UserRiskPreferences,
};
use fc_scoring::ScoringEngine;
use fc_storage::SqliteStore;

use crate::assessment::{build_assessment_snapshot, ServingModelContext};
use crate::backtest::{build_backtests, use_transitional_actionable_bridge};
use crate::data_source::AssessmentHistoryBuildMode;
use crate::demo_seed::{build_alerts, select_recent_alerts_for_date};
use crate::history_replay::{
    assessment_history_point_from_assessment, historical_output_from_prediction_snapshots,
    historical_replay_point_draft_from_assessment, load_cached_historical_replay_output,
    merge_historical_outputs, persist_historical_replay_output,
    prediction_snapshot_from_assessment, should_refresh_full_release_history,
    HistoricalAssessmentOutput,
};

#[derive(Debug, Clone, Copy)]
pub struct HistoryQueryWindow {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub limit: Option<usize>,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_assessment_history(
    data_mode: DataMode,
    scoring: &ScoringEngine,
    indicators: &[Indicator],
    observations: &[Observation],
    stored_alerts: Option<&[AlertEvent]>,
    serving_model: Option<&ServingModelContext>,
    user_preferences: &UserRiskPreferences,
    window: HistoryQueryWindow,
) -> HistoricalAssessmentOutput {
    let mut dates = observations
        .iter()
        .filter(|observation| observation.entity_id == "us")
        .map(|observation| observation.as_of_date)
        .collect::<Vec<_>>();
    dates.sort();
    dates.dedup();
    if let Some(from) = window.from {
        dates.retain(|date| *date >= from);
    }
    if let Some(to) = window.to {
        dates.retain(|date| *date <= to);
    }
    if let Some(limit) = window.limit {
        if dates.len() > limit {
            dates = dates[dates.len().saturating_sub(limit)..].to_vec();
        }
    }

    build_assessment_history_for_dates(
        data_mode,
        scoring,
        indicators,
        observations,
        stored_alerts,
        serving_model,
        user_preferences,
        &dates,
    )
}

#[allow(clippy::too_many_arguments)]
fn build_assessment_history_for_dates(
    data_mode: DataMode,
    scoring: &ScoringEngine,
    indicators: &[Indicator],
    observations: &[Observation],
    stored_alerts: Option<&[AlertEvent]>,
    serving_model: Option<&ServingModelContext>,
    user_preferences: &UserRiskPreferences,
    dates: &[NaiveDate],
) -> HistoricalAssessmentOutput {
    let mut history_points = Vec::with_capacity(dates.len());
    let mut prediction_snapshots = Vec::with_capacity(dates.len());
    let mut replay_point_drafts = Vec::with_capacity(dates.len());
    for as_of_date in dates.iter().copied() {
        let output = scoring.score(
            indicators,
            observations,
            as_of_date,
            "us",
            "financial_system",
        );
        let point_alerts = stored_alerts
            .map(|alerts| select_recent_alerts_for_date(alerts, as_of_date))
            .unwrap_or_else(|| build_alerts(&output.snapshot));
        let point_backtests = build_backtests(
            &output.snapshot,
            &[],
            use_transitional_actionable_bridge(serving_model),
        );
        let (assessment, posture_guidance, probability_trace) = build_assessment_snapshot(
            data_mode,
            &output.snapshot,
            &output.indicator_risks,
            observations,
            &point_alerts,
            &point_backtests,
            None,
            serving_model,
            user_preferences,
        );
        history_points.push(assessment_history_point_from_assessment(
            &assessment,
            &posture_guidance,
            &probability_trace,
        ));
        replay_point_drafts.push(historical_replay_point_draft_from_assessment(
            &assessment,
            &posture_guidance,
            &probability_trace,
            serving_model,
        ));
        prediction_snapshots.push(prediction_snapshot_from_assessment(
            &assessment,
            &posture_guidance,
            &probability_trace,
            serving_model,
        ));
    }

    HistoricalAssessmentOutput {
        history_points,
        prediction_snapshots,
        replay_point_drafts,
    }
}

#[allow(clippy::too_many_arguments)]
async fn rebuild_full_release_history_from_raw(
    store: &SqliteStore,
    indicators: &[Indicator],
    observations: &[Observation],
    alerts: &[AlertEvent],
    serving_model: Option<&ServingModelContext>,
    user_preferences: &UserRiskPreferences,
    rebuild_dates: &[NaiveDate],
    persist_prediction_snapshots: bool,
) -> anyhow::Result<HistoricalAssessmentOutput> {
    let rebuilt = build_assessment_history_for_dates(
        DataMode::Sqlite,
        &ScoringEngine::default(),
        indicators,
        observations,
        Some(alerts),
        serving_model,
        user_preferences,
        rebuild_dates,
    );
    if persist_prediction_snapshots {
        store
            .upsert_prediction_snapshots(&rebuilt.prediction_snapshots)
            .await?;
    }
    persist_historical_replay_output(store, observations, serving_model, &rebuilt).await?;
    Ok(rebuilt)
}

fn uses_bundle_backed_history(serving_model: Option<&ServingModelContext>) -> bool {
    serving_model.is_some_and(|context| context.probability_bundle.is_some())
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn load_sqlite_assessment_history(
    store: &SqliteStore,
    indicators: &[Indicator],
    observations: &[Observation],
    alerts: &[AlertEvent],
    serving_model: Option<&ServingModelContext>,
    user_preferences: &UserRiskPreferences,
    as_of_date: NaiveDate,
    _max_history_points: usize,
    history_build_mode: AssessmentHistoryBuildMode,
) -> anyhow::Result<Vec<fc_domain::AssessmentHistoryPoint>> {
    let release_filter = serving_model.map(|context| context.release.manifest.release_id.as_str());
    let persisted_rows = store
        .list_prediction_snapshots(Some("financial_system"), release_filter, None, None, None)
        .await?;
    let target_dates = observations
        .iter()
        .filter(|observation| observation.entity_id == "us")
        .map(|observation| observation.as_of_date)
        .chain(std::iter::once(as_of_date))
        .collect::<BTreeSet<_>>();
    let existing_dates = persisted_rows
        .iter()
        .map(|snapshot| snapshot.as_of_date)
        .collect::<BTreeSet<_>>();
    let missing_dates = target_dates
        .difference(&existing_dates)
        .copied()
        .collect::<Vec<_>>();
    let bundle_backed_history = uses_bundle_backed_history(serving_model);
    let persist_prediction_snapshots = !bundle_backed_history;
    let full_history_refresh = should_refresh_full_release_history(
        serving_model,
        &persisted_rows,
        !missing_dates.is_empty(),
    );

    if matches!(
        history_build_mode,
        AssessmentHistoryBuildMode::StrictRebuild
    ) {
        if let Some(cached_replay) =
            load_cached_historical_replay_output(store, serving_model, observations, &target_dates)
                .await?
        {
            tracing::info!(
                release_id = release_filter.unwrap_or("heuristic"),
                cached_dates = cached_replay.history_points.len(),
                "reusing cached strict-rebuild historical replay for current reload"
            );
            return Ok(cached_replay.history_points);
        }
        let rebuild_dates = target_dates.into_iter().collect::<Vec<_>>();
        tracing::info!(
            release_id = release_filter.unwrap_or("heuristic"),
            rebuild_dates = rebuild_dates.len(),
            "strictly rebuilding full release history from raw observations for current reload"
        );
        let rebuilt = rebuild_full_release_history_from_raw(
            store,
            indicators,
            observations,
            alerts,
            serving_model,
            user_preferences,
            &rebuild_dates,
            persist_prediction_snapshots,
        )
        .await?;
        return Ok(rebuilt.history_points);
    }

    if let Some(cached_replay) =
        load_cached_historical_replay_output(store, serving_model, observations, &target_dates)
            .await?
    {
        return Ok(cached_replay.history_points);
    }

    if bundle_backed_history {
        let rebuild_dates = target_dates.iter().copied().collect::<Vec<_>>();
        let reason = if full_history_refresh {
            "cached prediction snapshots are stale or incomplete"
        } else {
            "no reusable historical replay cache was found"
        };
        tracing::info!(
            release_id = release_filter.unwrap_or("heuristic"),
            rebuild_dates = rebuild_dates.len(),
            reason,
            "rebuilding full release history from raw observations for bundle-backed release"
        );
        let rebuilt = rebuild_full_release_history_from_raw(
            store,
            indicators,
            observations,
            alerts,
            serving_model,
            user_preferences,
            &rebuild_dates,
            persist_prediction_snapshots,
        )
        .await?;
        return Ok(rebuilt.history_points);
    }

    let mut historical =
        historical_output_from_prediction_snapshots(persisted_rows.clone(), release_filter);

    if !missing_dates.is_empty() {
        let computed = build_assessment_history_for_dates(
            DataMode::Sqlite,
            &ScoringEngine::default(),
            indicators,
            observations,
            Some(alerts),
            serving_model,
            user_preferences,
            &missing_dates,
        );
        store
            .upsert_prediction_snapshots(&computed.prediction_snapshots)
            .await?;
        let mut combined_snapshots = persisted_rows;
        combined_snapshots.extend(computed.prediction_snapshots.clone());
        historical = merge_historical_outputs(
            historical_output_from_prediction_snapshots(combined_snapshots, release_filter),
            computed,
        );
    }

    Ok(historical.history_points)
}

pub(crate) fn select_assessment_history(
    points: &[fc_domain::AssessmentHistoryPoint],
    window: HistoryQueryWindow,
) -> Vec<fc_domain::AssessmentHistoryPoint> {
    select_points_by_window(points, window, |point| point.as_of_date)
}

pub(crate) fn select_backtest_timeline(
    points: &[BacktestWindowPoint],
    window: HistoryQueryWindow,
) -> Vec<BacktestWindowPoint> {
    select_points_by_window(points, window, |point| point.as_of_date)
}

fn select_points_by_window<T: Clone>(
    points: &[T],
    window: HistoryQueryWindow,
    date_of: impl Fn(&T) -> NaiveDate,
) -> Vec<T> {
    let mut filtered = points
        .iter()
        .filter(|point| window.from.is_none_or(|from| date_of(point) >= from))
        .filter(|point| window.to.is_none_or(|to| date_of(point) <= to))
        .cloned()
        .collect::<Vec<_>>();
    if let Some(limit) = window.limit {
        if filtered.len() > limit {
            filtered = filtered[filtered.len().saturating_sub(limit)..].to_vec();
        }
    }
    filtered
}

#[cfg(test)]
mod tests;

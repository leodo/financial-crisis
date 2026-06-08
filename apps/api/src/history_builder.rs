use std::collections::BTreeSet;

use chrono::NaiveDate;
use fc_domain::{
    AlertEvent, AssessmentSnapshot, BacktestWindowPoint, DataMode, DecisionPosture,
    FeatureSnapshotRecord, Indicator, Observation, PostureGuidance, TimeToRiskBucket,
    UserRiskPreferences,
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
    prediction_snapshot_from_assessment, HistoricalAssessmentOutput,
};

const HISTORY_PREPARE_HYSTERESIS_PLATEAU_P20D_FLOOR: f64 = 0.45;
const HISTORY_PREPARE_HYSTERESIS_PLATEAU_P60D_FLOOR: f64 = 0.65;
const HISTORY_PREPARE_HYSTERESIS_LONG_WINDOW_P20D_FLOOR: f64 = 0.35;
const HISTORY_PREPARE_HYSTERESIS_LONG_WINDOW_P60D_FLOOR: f64 = 0.80;
const HISTORY_PREPARE_HYSTERESIS_OVERALL_FLOOR: f64 = 40.0;
const HISTORY_PREPARE_HYSTERESIS_STRUCTURAL_FLOOR: f64 = 44.0;
const HISTORY_PREPARE_HYSTERESIS_EXTREME_CARRY_P20D_FLOOR: f64 = 0.49;
const HISTORY_PREPARE_HYSTERESIS_EXTREME_CARRY_P60D_FLOOR: f64 = 0.75;
const HISTORY_PREPARE_HYSTERESIS_EXTREME_CARRY_STRUCTURAL_FLOOR: f64 = 36.0;
const HISTORY_PREPARE_HYSTERESIS_EXTREME_CARRY_EXTERNAL_FLOOR: f64 = 43.0;
const HISTORY_PREPARE_HYSTERESIS_STRUCTURAL_CARRY_P20D_FLOOR: f64 = 0.25;
const HISTORY_PREPARE_HYSTERESIS_STRUCTURAL_CARRY_P60D_FLOOR: f64 = 0.65;
const HISTORY_PREPARE_HYSTERESIS_STRUCTURAL_CARRY_OVERALL_FLOOR: f64 = 43.5;
const HISTORY_PREPARE_HYSTERESIS_STRUCTURAL_CARRY_STRUCTURAL_FLOOR: f64 = 58.0;
const HISTORY_PREPARE_HYSTERESIS_STRUCTURAL_CARRY_TRIGGER_CEILING: f64 = 30.0;
const HISTORY_PREPARE_HYSTERESIS_CARRY_GRACE_DAYS: u8 = 1;
const HISTORY_PREPARE_HYSTERESIS_CARRY_GRACE_P60D_FLOOR: f64 = 0.75;
const HISTORY_PREPARE_HYSTERESIS_CARRY_GRACE_OVERALL_FLOOR: f64 = 42.0;
const HISTORY_PREPARE_HYSTERESIS_CARRY_GRACE_STRUCTURAL_FLOOR: f64 = 55.0;
const HISTORY_PREPARE_HYSTERESIS_CARRY_GRACE_TRIGGER_CEILING: f64 = 28.0;
const HISTORY_PREPARE_HYSTERESIS_TRIGGER_CODE: &str = "prepare_history_hysteresis";
const HISTORY_SOURCE_RAW_OBSERVATION_REBUILD: &str = "raw_observation_rebuild";
const HISTORY_SOURCE_RAW_PIT_FEATURE_REPLAY: &str = "raw_pit_feature_replay";

#[derive(Debug, Clone, Copy, Default)]
struct HistoricalPrepareHysteresisState {
    active: bool,
    carry_grace_days_remaining: u8,
}

impl HistoricalPrepareHysteresisState {
    fn apply(
        &mut self,
        enabled: bool,
        assessment: &mut AssessmentSnapshot,
        posture_guidance: &mut PostureGuidance,
    ) {
        if !enabled {
            self.active = false;
            self.carry_grace_days_remaining = 0;
            return;
        }

        let was_active = self.active;
        let continuation_supported = history_prepare_hysteresis_supported(assessment)
            || (was_active && history_prepare_hysteresis_extreme_carry_supported(assessment))
            || (was_active && history_prepare_hysteresis_structural_carry_supported(assessment));
        if was_active && continuation_supported {
            if matches!(assessment.posture, DecisionPosture::Normal) {
                assessment.posture = DecisionPosture::Prepare;
                posture_guidance.posture = DecisionPosture::Prepare;
            }
            if matches!(assessment.time_to_risk_bucket, TimeToRiskBucket::Normal) {
                assessment.time_to_risk_bucket = TimeToRiskBucket::Months;
            }
            if !matches!(assessment.posture, DecisionPosture::Normal)
                && !posture_guidance
                    .trigger_codes
                    .iter()
                    .any(|code| code == HISTORY_PREPARE_HYSTERESIS_TRIGGER_CODE)
            {
                posture_guidance
                    .trigger_codes
                    .push(HISTORY_PREPARE_HYSTERESIS_TRIGGER_CODE.to_string());
            }
        }

        let anchored = history_prepare_hysteresis_anchor(assessment, posture_guidance);
        let grace_supported = was_active
            && !continuation_supported
            && self.carry_grace_days_remaining > 0
            && history_prepare_hysteresis_carry_grace_supported(assessment);

        self.active = anchored || (was_active && continuation_supported) || grace_supported;
        self.carry_grace_days_remaining = if anchored || (was_active && continuation_supported) {
            HISTORY_PREPARE_HYSTERESIS_CARRY_GRACE_DAYS
        } else if grace_supported {
            self.carry_grace_days_remaining.saturating_sub(1)
        } else {
            0
        };
    }
}

fn history_prepare_hysteresis_anchor(
    assessment: &AssessmentSnapshot,
    posture_guidance: &PostureGuidance,
) -> bool {
    matches!(assessment.posture, DecisionPosture::Prepare)
        && !matches!(assessment.time_to_risk_bucket, TimeToRiskBucket::Normal)
        && posture_guidance.trigger_codes.iter().any(|code| {
            matches!(
                code.as_str(),
                "prepare_p60d_structural"
                    | "prepare_structural_downgrade"
                    | "prepare_carry_structural"
                    | "prepare_external_structural"
                    | "prepare_continuity_bridge"
                    | "prepare_probability_plateau"
                    | HISTORY_PREPARE_HYSTERESIS_TRIGGER_CODE
            )
        })
}

fn history_prepare_hysteresis_supported(assessment: &AssessmentSnapshot) -> bool {
    let plateau_probability_support = assessment.probabilities.p_20d
        >= HISTORY_PREPARE_HYSTERESIS_PLATEAU_P20D_FLOOR
        && assessment.probabilities.p_60d >= HISTORY_PREPARE_HYSTERESIS_PLATEAU_P60D_FLOOR;
    let long_window_probability_support = assessment.probabilities.p_20d
        >= HISTORY_PREPARE_HYSTERESIS_LONG_WINDOW_P20D_FLOOR
        && assessment.probabilities.p_60d >= HISTORY_PREPARE_HYSTERESIS_LONG_WINDOW_P60D_FLOOR;

    assessment.scores.overall_score >= HISTORY_PREPARE_HYSTERESIS_OVERALL_FLOOR
        && assessment.scores.structural_score >= HISTORY_PREPARE_HYSTERESIS_STRUCTURAL_FLOOR
        && (plateau_probability_support || long_window_probability_support)
}

fn history_prepare_hysteresis_extreme_carry_supported(assessment: &AssessmentSnapshot) -> bool {
    assessment.probabilities.p_20d >= HISTORY_PREPARE_HYSTERESIS_EXTREME_CARRY_P20D_FLOOR
        && assessment.probabilities.p_60d >= HISTORY_PREPARE_HYSTERESIS_EXTREME_CARRY_P60D_FLOOR
        && assessment.scores.structural_score
            >= HISTORY_PREPARE_HYSTERESIS_EXTREME_CARRY_STRUCTURAL_FLOOR
        && assessment.scores.external_shock_score
            >= HISTORY_PREPARE_HYSTERESIS_EXTREME_CARRY_EXTERNAL_FLOOR
}

fn history_prepare_hysteresis_structural_carry_supported(assessment: &AssessmentSnapshot) -> bool {
    assessment.probabilities.p_20d >= HISTORY_PREPARE_HYSTERESIS_STRUCTURAL_CARRY_P20D_FLOOR
        && assessment.probabilities.p_60d >= HISTORY_PREPARE_HYSTERESIS_STRUCTURAL_CARRY_P60D_FLOOR
        && assessment.scores.overall_score
            >= HISTORY_PREPARE_HYSTERESIS_STRUCTURAL_CARRY_OVERALL_FLOOR
        && assessment.scores.structural_score
            >= HISTORY_PREPARE_HYSTERESIS_STRUCTURAL_CARRY_STRUCTURAL_FLOOR
        && assessment.scores.trigger_score
            <= HISTORY_PREPARE_HYSTERESIS_STRUCTURAL_CARRY_TRIGGER_CEILING
}

fn history_prepare_hysteresis_carry_grace_supported(assessment: &AssessmentSnapshot) -> bool {
    assessment.probabilities.p_60d >= HISTORY_PREPARE_HYSTERESIS_CARRY_GRACE_P60D_FLOOR
        && assessment.scores.overall_score >= HISTORY_PREPARE_HYSTERESIS_CARRY_GRACE_OVERALL_FLOOR
        && assessment.scores.structural_score
            >= HISTORY_PREPARE_HYSTERESIS_CARRY_GRACE_STRUCTURAL_FLOOR
        && assessment.scores.trigger_score <= HISTORY_PREPARE_HYSTERESIS_CARRY_GRACE_TRIGGER_CEILING
}

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
    let mut prepare_hysteresis = HistoricalPrepareHysteresisState::default();
    let enable_history_hysteresis = serving_model.is_some();
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
        let mut assessment = assessment;
        let mut posture_guidance = posture_guidance;
        prepare_hysteresis.apply(
            enable_history_hysteresis,
            &mut assessment,
            &mut posture_guidance,
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
    let mut rebuilt = build_assessment_history_for_dates(
        DataMode::Sqlite,
        &ScoringEngine::default(),
        indicators,
        observations,
        Some(alerts),
        serving_model,
        user_preferences,
        rebuild_dates,
    );
    attach_feature_snapshot_context(store, serving_model, rebuild_dates, &mut rebuilt).await?;
    if persist_prediction_snapshots {
        store
            .upsert_prediction_snapshots(&rebuilt.prediction_snapshots)
            .await?;
    }
    persist_historical_replay_output(store, observations, serving_model, &rebuilt).await?;
    Ok(rebuilt)
}

async fn attach_feature_snapshot_context(
    store: &SqliteStore,
    serving_model: Option<&ServingModelContext>,
    rebuild_dates: &[NaiveDate],
    output: &mut HistoricalAssessmentOutput,
) -> anyhow::Result<()> {
    let Some(serving_model) = serving_model else {
        return Ok(());
    };
    let Some(_) = serving_model.probability_bundle.as_ref() else {
        return Ok(());
    };
    let Some(from_date) = rebuild_dates.first().copied() else {
        return Ok(());
    };
    let Some(to_date) = rebuild_dates.last().copied() else {
        return Ok(());
    };

    let snapshot_ids = load_feature_snapshot_ids_for_history_range(
        store,
        serving_model,
        from_date,
        to_date,
        rebuild_dates,
    )
    .await?;

    for point in &mut output.history_points {
        point.feature_snapshot_id = snapshot_ids.get(&point.as_of_date).cloned();
        point.history_source = Some(
            if point.feature_snapshot_id.is_some() {
                HISTORY_SOURCE_RAW_PIT_FEATURE_REPLAY
            } else {
                HISTORY_SOURCE_RAW_OBSERVATION_REBUILD
            }
            .to_string(),
        );
    }

    for point in &mut output.replay_point_drafts {
        point.feature_snapshot_id = snapshot_ids.get(&point.as_of_date).cloned();
    }

    Ok(())
}

async fn load_feature_snapshot_ids_for_history_range(
    store: &SqliteStore,
    serving_model: &ServingModelContext,
    from_date: NaiveDate,
    to_date: NaiveDate,
    rebuild_dates: &[NaiveDate],
) -> anyhow::Result<std::collections::BTreeMap<NaiveDate, String>> {
    let target_dates = rebuild_dates.iter().copied().collect::<BTreeSet<_>>();
    let snapshots = store
        .list_feature_snapshots_for_mode(
            &serving_model.release.manifest.market_scope,
            &serving_model.release.manifest.feature_set_version,
            &serving_model.release.manifest.point_in_time_mode,
            Some(from_date),
            Some(to_date),
        )
        .await?;

    let mut latest_by_date = std::collections::BTreeMap::<NaiveDate, FeatureSnapshotRecord>::new();
    for snapshot in snapshots {
        if snapshot.entity_id != "us" {
            continue;
        }
        match latest_by_date.get(&snapshot.as_of_date) {
            Some(existing) if existing.created_at >= snapshot.created_at => {}
            _ => {
                latest_by_date.insert(snapshot.as_of_date, snapshot);
            }
        }
    }

    let effective_snapshot_ids = latest_by_date
        .into_iter()
        .map(|(date, snapshot)| {
            (
                date,
                feature_snapshot_id(
                    &snapshot.entity_id,
                    &snapshot.market_scope,
                    snapshot.as_of_date,
                    &snapshot.feature_set_version,
                    &snapshot.point_in_time_mode,
                ),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();

    let mut bound_snapshot_ids = std::collections::BTreeMap::new();
    let mut current_snapshot_id = None::<String>;
    for target_date in target_dates {
        if let Some(snapshot_id) = effective_snapshot_ids.get(&target_date) {
            current_snapshot_id = Some(snapshot_id.clone());
        }
        if let Some(snapshot_id) = current_snapshot_id.clone() {
            bound_snapshot_ids.insert(target_date, snapshot_id);
        }
    }

    Ok(bound_snapshot_ids)
}

fn feature_snapshot_id(
    entity_id: &str,
    market_scope: &str,
    as_of_date: NaiveDate,
    feature_set_version: &str,
    point_in_time_mode: &str,
) -> String {
    format!("{market_scope}:{entity_id}:{as_of_date}:{feature_set_version}:{point_in_time_mode}")
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
    max_history_points: usize,
    history_build_mode: AssessmentHistoryBuildMode,
) -> anyhow::Result<Vec<fc_domain::AssessmentHistoryPoint>> {
    let release_filter = serving_model.map(|context| context.release.manifest.release_id.as_str());
    let mut target_dates = observations
        .iter()
        .filter(|observation| observation.entity_id == "us")
        .map(|observation| observation.as_of_date)
        .chain(std::iter::once(as_of_date))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if max_history_points > 0 && target_dates.len() > max_history_points {
        target_dates =
            target_dates[target_dates.len().saturating_sub(max_history_points)..].to_vec();
    }
    let target_dates = target_dates.into_iter().collect::<BTreeSet<_>>();
    let bundle_backed_history = uses_bundle_backed_history(serving_model);
    let persist_prediction_snapshots = !bundle_backed_history;

    if matches!(
        history_build_mode,
        AssessmentHistoryBuildMode::StrictRebuild
    ) {
        let rebuild_dates = target_dates.into_iter().collect::<Vec<_>>();
        tracing::info!(
            release_id = release_filter.unwrap_or("heuristic"),
            rebuild_dates = rebuild_dates.len(),
            "strictly rebuilding full release history from raw observations and bypassing replay cache for current reload"
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
        // Formal bundle history now treats replay cache as the only reusable history source.
        // If no matching replay run exists for the target dates, fall back directly to raw replay.
        tracing::info!(
            release_id = release_filter.unwrap_or("heuristic"),
            rebuild_dates = rebuild_dates.len(),
            "rebuilding full release history from raw observations for bundle-backed release because replay cache is unavailable for the current target dates"
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

    let persisted_rows = store
        .list_prediction_snapshots(
            Some("financial_system"),
            release_filter,
            target_dates.iter().next().copied(),
            target_dates.iter().next_back().copied(),
            None,
        )
        .await?;
    let existing_dates = persisted_rows
        .iter()
        .map(|snapshot| snapshot.as_of_date)
        .collect::<BTreeSet<_>>();
    let missing_dates = target_dates
        .difference(&existing_dates)
        .copied()
        .collect::<Vec<_>>();
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

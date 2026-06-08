use std::collections::BTreeSet;

use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc, Weekday};
use fc_domain::{
    formal_observation_feature_value_from_history, observation_history_for_indicator_where,
    AlertEvent, AssessmentSnapshot, BacktestWindowPoint, DataMode, DecisionPosture,
    FeatureSnapshotRecord, FormalObservationFeatureTransform, Frequency, Indicator, IndicatorRisk,
    Observation, PostureGuidance, RiskDimension, TimeToRiskBucket, UserRiskPreferences,
    FORMAL_OBSERVATION_FEATURE_SPECS,
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
    merge_historical_outputs, persist_historical_replay_output, pit_feature_history_source,
    prediction_snapshot_from_assessment, HistoricalAssessmentOutput,
    HISTORY_SOURCE_RAW_OBSERVATION_REBUILD, HISTORY_SOURCE_RAW_OBSERVATION_REPLAY,
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
const FEATURE_SNAPSHOT_STATUS_READY: &str = "ready";
const FEATURE_SNAPSHOT_STATUS_COVERAGE_OR_VISIBILITY_FAILED: &str = "coverage_or_visibility_failed";
const FORMAL_STLFSI_REQUIRED_FROM: (i32, u32, u32) = (1993, 12, 31);
const FORMAL_CORE_INDICATORS_PRE_STLFSI: &[&str] = &[
    "us_market_vix_close",
    "us_rates_yield_curve_10y2y",
    "us_credit_baa_10y_spread",
    "us_liquidity_effr",
    "us_liquidity_national_financial_conditions",
    "us_macro_unemployment_rate",
    "us_real_estate_housing_starts",
];
const FORMAL_CORE_INDICATORS_POST_STLFSI: &[&str] = &[
    "us_market_vix_close",
    "us_rates_yield_curve_10y2y",
    "us_credit_baa_10y_spread",
    "us_liquidity_effr",
    "us_liquidity_national_financial_conditions",
    "us_liquidity_financial_stress_stl",
    "us_macro_unemployment_rate",
    "us_real_estate_housing_starts",
];
const FORMAL_TRIGGER_INDICATORS_PRE_STLFSI: &[&str] = &[
    "us_market_vix_close",
    "us_rates_yield_curve_10y2y",
    "us_credit_baa_10y_spread",
    "us_liquidity_effr",
    "us_liquidity_national_financial_conditions",
];
const FORMAL_TRIGGER_INDICATORS_POST_STLFSI: &[&str] = &[
    "us_market_vix_close",
    "us_rates_yield_curve_10y2y",
    "us_credit_baa_10y_spread",
    "us_liquidity_effr",
    "us_liquidity_national_financial_conditions",
    "us_liquidity_financial_stress_stl",
];
const FORMAL_EXTERNAL_INDICATORS: &[&str] = &["us_external_usdjpy_level"];

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
    attach_feature_snapshot_context(
        store,
        serving_model,
        indicators,
        observations,
        rebuild_dates,
        &mut rebuilt,
    )
    .await?;
    if persist_prediction_snapshots {
        store
            .upsert_prediction_snapshots(&rebuilt.prediction_snapshots)
            .await?;
    }
    if let Some(replay_run_id) =
        persist_historical_replay_output(store, observations, serving_model, &rebuilt).await?
    {
        for point in &mut rebuilt.history_points {
            point.replay_run_id = Some(replay_run_id.clone());
            point.history_source = Some(
                pit_feature_history_source(
                    point.feature_snapshot_id.as_deref(),
                    point.as_of_date,
                    HISTORY_SOURCE_RAW_OBSERVATION_REPLAY,
                )
                .to_string(),
            );
        }
    }
    Ok(rebuilt)
}

async fn attach_feature_snapshot_context(
    store: &SqliteStore,
    serving_model: Option<&ServingModelContext>,
    indicators: &[Indicator],
    observations: &[Observation],
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
        indicators,
        observations,
        from_date,
        to_date,
        rebuild_dates,
    )
    .await?;

    for point in &mut output.history_points {
        point.feature_snapshot_id = snapshot_ids.get(&point.as_of_date).cloned();
        point.history_source = Some(
            pit_feature_history_source(
                point.feature_snapshot_id.as_deref(),
                point.as_of_date,
                HISTORY_SOURCE_RAW_OBSERVATION_REBUILD,
            )
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
    indicators: &[Indicator],
    observations: &[Observation],
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

    let mut materialized_snapshots = Vec::new();
    let mut rebuilt_snapshots = Vec::new();
    let mut latest_snapshot = latest_by_date
        .iter()
        .next()
        .map(|(_, snapshot)| snapshot.clone());
    for target_date in target_dates.iter().copied() {
        if let Some(snapshot) = latest_by_date.get(&target_date).cloned() {
            latest_snapshot = Some(snapshot);
            continue;
        }

        let Some(prior_snapshot) = latest_snapshot.as_ref() else {
            continue;
        };
        if can_materialize_exact_carry_forward_snapshot(
            prior_snapshot,
            observations,
            target_date,
            &serving_model.release.manifest.point_in_time_mode,
        ) {
            let exact_snapshot = materialize_carry_forward_snapshot(prior_snapshot, target_date);
            latest_by_date.insert(target_date, exact_snapshot.clone());
            latest_snapshot = Some(exact_snapshot.clone());
            materialized_snapshots.push(exact_snapshot);
        }
    }

    if !materialized_snapshots.is_empty() {
        store
            .upsert_feature_snapshots(&materialized_snapshots)
            .await?;
    }

    for target_date in target_dates.iter().copied() {
        if latest_by_date.contains_key(&target_date) {
            continue;
        }
        let exact_snapshot = build_exact_feature_snapshot_for_date(
            indicators,
            observations,
            serving_model,
            target_date,
        )?;
        latest_by_date.insert(target_date, exact_snapshot.clone());
        rebuilt_snapshots.push(exact_snapshot);
    }

    if !rebuilt_snapshots.is_empty() {
        store.upsert_feature_snapshots(&rebuilt_snapshots).await?;
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

fn build_exact_feature_snapshot_for_date(
    indicators: &[Indicator],
    observations: &[Observation],
    serving_model: &ServingModelContext,
    as_of_date: NaiveDate,
) -> anyhow::Result<FeatureSnapshotRecord> {
    let point_in_time_mode = serving_model.release.manifest.point_in_time_mode.as_str();
    let scoring = ScoringEngine::default();
    let scoring_output = scoring.score_with_observation_filter(
        indicators,
        observations,
        as_of_date,
        "us",
        &serving_model.release.manifest.market_scope,
        |observation| {
            observation_is_visible_for_date_for_mode(observation, as_of_date, point_in_time_mode)
        },
    );
    let mut features = std::collections::BTreeMap::new();
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
        round6((scoring_output.snapshot.overall_score / 100.0).clamp(0.0, 1.0)),
    );
    features.insert(
        "structural_score".to_string(),
        round6((scoring_output.snapshot.structural_score / 100.0).clamp(0.0, 1.0)),
    );
    features.insert(
        "trigger_score".to_string(),
        round6((scoring_output.snapshot.trigger_score / 100.0).clamp(0.0, 1.0)),
    );
    features.insert(
        "external_dimension_score".to_string(),
        round6(
            (find_dimension_score(
                &scoring_output.indicator_risks,
                RiskDimension::ExternalSector,
            ) / 100.0)
                .clamp(0.0, 1.0),
        ),
    );

    let (
        core_feature_coverage,
        trigger_feature_coverage,
        external_feature_coverage,
        coverage_score,
    ) = coverage_summary(&scoring_output.indicator_risks, as_of_date);
    let latest_visible_at = visible_candidates.into_iter().max();
    let visibility_status =
        feature_snapshot_visibility_status(&features, coverage_score, latest_visible_at);

    Ok(FeatureSnapshotRecord {
        as_of_date,
        entity_id: "us".to_string(),
        market_scope: serving_model.release.manifest.market_scope.clone(),
        feature_set_version: serving_model.release.manifest.feature_set_version.clone(),
        point_in_time_mode: serving_model.release.manifest.point_in_time_mode.clone(),
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

fn materialize_carry_forward_snapshot(
    prior_snapshot: &FeatureSnapshotRecord,
    as_of_date: NaiveDate,
) -> FeatureSnapshotRecord {
    let mut snapshot = prior_snapshot.clone();
    snapshot.as_of_date = as_of_date;
    snapshot.created_at = Utc::now();
    snapshot
}

fn can_materialize_exact_carry_forward_snapshot(
    prior_snapshot: &FeatureSnapshotRecord,
    observations: &[Observation],
    as_of_date: NaiveDate,
    point_in_time_mode: &str,
) -> bool {
    if as_of_date <= prior_snapshot.as_of_date {
        return false;
    }
    let Some(prior_latest_visible_at) = prior_snapshot.latest_visible_at else {
        return false;
    };

    observations
        .iter()
        .filter_map(|observation| observation_visible_at_for_mode(observation, point_in_time_mode))
        .all(|visible_at| {
            visible_at <= prior_latest_visible_at || visible_at > assessment_cutoff_utc(as_of_date)
        })
}

fn observation_is_visible_for_date_for_mode(
    observation: &Observation,
    as_of_date: NaiveDate,
    point_in_time_mode: &str,
) -> bool {
    observation_visible_at_for_mode(observation, point_in_time_mode)
        .map(|visible_at| visible_at <= assessment_cutoff_utc(as_of_date))
        .unwrap_or(false)
}

fn insert_formal_observation_features(
    features: &mut std::collections::BTreeMap<String, f64>,
    visible_candidates: &mut Vec<DateTime<Utc>>,
    observations: &[Observation],
    as_of_date: NaiveDate,
    point_in_time_mode: &str,
) {
    for spec in FORMAL_OBSERVATION_FEATURE_SPECS {
        let history = observation_history_for_indicator_where(
            observations,
            spec.indicator_id,
            as_of_date,
            |observation| {
                observation_is_visible_for_date_for_mode(
                    observation,
                    as_of_date,
                    point_in_time_mode,
                )
            },
        );
        if let Some(value) = formal_observation_feature_value_from_history(&history, spec.transform)
        {
            features.insert(spec.feature_name.to_string(), round6(value));
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
    features: &std::collections::BTreeMap<String, f64>,
    coverage_score: f64,
    latest_visible_at: Option<DateTime<Utc>>,
) -> &'static str {
    if latest_visible_at.is_none()
        || coverage_score < 0.70
        || !has_main_dataset_core_features(features)
    {
        FEATURE_SNAPSHOT_STATUS_COVERAGE_OR_VISIBILITY_FAILED
    } else {
        FEATURE_SNAPSHOT_STATUS_READY
    }
}

fn coverage_summary(
    indicator_risks: &[IndicatorRisk],
    as_of_date: NaiveDate,
) -> (f64, f64, f64, f64) {
    let (core_total, core_present) =
        coverage_by_indicator_ids(indicator_risks, formal_core_indicator_ids(as_of_date));
    let (trigger_total, trigger_present) =
        coverage_by_indicator_ids(indicator_risks, formal_trigger_indicator_ids(as_of_date));
    let (external_total, external_present) =
        coverage_by_indicator_ids(indicator_risks, FORMAL_EXTERNAL_INDICATORS);

    let core_feature_coverage = ratio(core_present, core_total);
    let trigger_feature_coverage = ratio(trigger_present, trigger_total);
    let external_feature_coverage = ratio(external_present, external_total);
    let coverage_score = round3(
        (core_feature_coverage * 0.45
            + trigger_feature_coverage * 0.35
            + external_feature_coverage * 0.2)
            .clamp(0.0, 1.0),
    );
    (
        round3(core_feature_coverage),
        round3(trigger_feature_coverage),
        round3(external_feature_coverage),
        coverage_score,
    )
}

fn formal_core_indicator_ids(as_of_date: NaiveDate) -> &'static [&'static str] {
    if as_of_date >= formal_stlfsi_required_from() {
        FORMAL_CORE_INDICATORS_POST_STLFSI
    } else {
        FORMAL_CORE_INDICATORS_PRE_STLFSI
    }
}

fn formal_trigger_indicator_ids(as_of_date: NaiveDate) -> &'static [&'static str] {
    if as_of_date >= formal_stlfsi_required_from() {
        FORMAL_TRIGGER_INDICATORS_POST_STLFSI
    } else {
        FORMAL_TRIGGER_INDICATORS_PRE_STLFSI
    }
}

fn formal_stlfsi_required_from() -> NaiveDate {
    NaiveDate::from_ymd_opt(
        FORMAL_STLFSI_REQUIRED_FROM.0,
        FORMAL_STLFSI_REQUIRED_FROM.1,
        FORMAL_STLFSI_REQUIRED_FROM.2,
    )
    .expect("valid stlfsi activation date")
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

fn has_main_dataset_core_features(features: &std::collections::BTreeMap<String, f64>) -> bool {
    [
        "us_vix_level",
        "us_curve_10y2y_level",
        "us_baa_10y_spread_level",
        "us_fed_funds_level",
    ]
    .into_iter()
    .all(|feature| features.contains_key(feature))
}

fn round3(value: f64) -> f64 {
    (value * 1_000.0).round() / 1_000.0
}

fn round6(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

fn observation_visible_at_for_mode(
    observation: &Observation,
    point_in_time_mode: &str,
) -> Option<DateTime<Utc>> {
    match point_in_time_mode {
        "best_effort" => best_effort_visible_at(observation),
        "strict" => strict_visible_at(observation),
        _ => None,
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

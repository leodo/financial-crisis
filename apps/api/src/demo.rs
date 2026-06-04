use std::{collections::BTreeSet, env};

use chrono::{Duration, NaiveDate, Utc};
use fc_domain::{
    load_protected_stress_window_catalog, AlertEvent, AlertStatus, AlertType, BacktestWindowPoint,
    DataMode, DataSource, Frequency, Indicator, Observation, PredictionSnapshotRecord,
    RiskDimension, RiskDirection, RiskLevel, SourceHealth, SourcePriority, SourceStatus,
    UserRiskPreferences, UserRiskProfile,
};
use fc_scoring::ScoringEngine;
use fc_storage::SqliteStore;
use uuid::Uuid;

use crate::assessment::{
    build_assessment_snapshot, build_backtest_summary, runtime_threshold_diagnostics,
    ServingModelContext,
};
#[cfg(test)]
use crate::backtest::is_actionable_warning_point;
use crate::backtest::{
    build_backtest_timeline, build_backtests, build_rolling_backtest_audit,
    use_transitional_actionable_bridge,
};
use crate::data_source::AssessmentHistoryBuildMode;
#[cfg(test)]
pub(crate) use crate::history_replay::expected_prediction_snapshot_method_version;
pub(crate) use crate::history_replay::{
    assessment_history_point_from_assessment, historical_output_from_prediction_snapshots,
    historical_replay_point_draft_from_assessment, load_cached_historical_replay_output,
    merge_historical_outputs, persist_historical_replay_output,
    prediction_snapshot_from_assessment, should_refresh_full_release_history,
    HistoricalAssessmentOutput,
};
use crate::AppData;

const EVENT_LOOKBACK_DAYS: i64 = 30;
pub(crate) const FORMAL_MAIN_FEATURE_SET_VERSION: &str = "feature_formal_v1_main_20260531";
pub(crate) const FORMAL_MAIN_LABEL_VERSION: &str = "formal_label_v1_main";

#[derive(Debug, Clone, Copy)]
pub struct HistoryQueryWindow {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub limit: Option<usize>,
}

#[derive(Debug)]
pub(crate) struct BuiltAppData {
    pub(crate) app_data: AppData,
    pub(crate) prediction_snapshots: Vec<PredictionSnapshotRecord>,
}

pub fn build_demo_data(_max_history_points: usize) -> AppData {
    let as_of_date = NaiveDate::from_ymd_opt(2026, 5, 30).expect("valid date");
    let indicators = indicators();
    let observations = observations(as_of_date);
    let user_preferences = load_user_preferences();
    let historical = build_assessment_history(
        DataMode::Demo,
        &ScoringEngine::default(),
        &indicators,
        &observations,
        None,
        None,
        &user_preferences,
        HistoryQueryWindow {
            from: None,
            to: None,
            limit: None,
        },
    );
    build_app_data_from_inputs(
        DataMode::Demo,
        indicators,
        observations,
        None,
        None,
        as_of_date,
        historical.history_points,
        user_preferences,
    )
    .app_data
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_app_data_from_inputs(
    data_mode: DataMode,
    indicators: Vec<Indicator>,
    observations: Vec<Observation>,
    stored_alerts: Option<Vec<AlertEvent>>,
    serving_model: Option<ServingModelContext>,
    as_of_date: NaiveDate,
    mut assessment_history: Vec<fc_domain::AssessmentHistoryPoint>,
    user_preferences: UserRiskPreferences,
) -> BuiltAppData {
    let use_transitional_bridge = use_transitional_actionable_bridge(serving_model.as_ref());
    let scoring = ScoringEngine::default();
    let protected_stress_window_catalog = load_protected_stress_window_catalog();
    let threshold_diagnostics = runtime_threshold_diagnostics(serving_model.as_ref());
    let output = scoring.score(
        &indicators,
        &observations,
        as_of_date,
        "us",
        "financial_system",
    );
    let backtests = build_backtests(
        &output.snapshot,
        &assessment_history,
        use_transitional_bridge,
    );
    let rolling_audit = build_rolling_backtest_audit(
        &assessment_history,
        &protected_stress_window_catalog.windows,
        use_transitional_bridge,
    );
    let alerts = stored_alerts
        .map(|alerts| select_recent_alerts_for_date(&alerts, as_of_date))
        .unwrap_or_else(|| build_alerts(&output.snapshot));
    let backtest_summary = build_backtest_summary(&backtests, Some(&rolling_audit));
    let (assessment, posture_guidance, probability_trace) = build_assessment_snapshot(
        data_mode,
        &output.snapshot,
        &output.indicator_risks,
        &observations,
        &alerts,
        &backtests,
        Some(&rolling_audit),
        serving_model.as_ref(),
        &user_preferences,
    );
    let assessment = fc_domain::AssessmentSnapshot {
        backtest_summary,
        ..assessment
    };
    let current_history_point = assessment_history_point_from_assessment(
        &assessment,
        &posture_guidance,
        &probability_trace,
    );
    match assessment_history.last_mut() {
        Some(last) if last.as_of_date == current_history_point.as_of_date => {
            *last = current_history_point;
        }
        _ => assessment_history.push(current_history_point),
    }
    let backtest_timeline = build_backtest_timeline(&assessment_history, use_transitional_bridge);
    let current_prediction_snapshot = prediction_snapshot_from_assessment(
        &assessment,
        &posture_guidance,
        &probability_trace,
        serving_model.as_ref(),
    );
    BuiltAppData {
        app_data: AppData {
            data_mode,
            user_preferences,
            overview: output.snapshot,
            indicators: output.indicator_risks,
            alerts,
            sources: if matches!(data_mode, DataMode::Demo) {
                sources_demo()
            } else {
                sources_runtime(&observations, as_of_date)
            },
            backtests,
            backtest_timeline,
            assessment,
            assessment_history,
            posture_guidance,
            protected_stress_window_catalog,
            runtime_thresholds: threshold_diagnostics,
        },
        prediction_snapshots: vec![current_prediction_snapshot],
    }
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
        let rebuilt = build_assessment_history_for_dates(
            DataMode::Sqlite,
            &ScoringEngine::default(),
            indicators,
            observations,
            Some(alerts),
            serving_model,
            user_preferences,
            &rebuild_dates,
        );
        store
            .upsert_prediction_snapshots(&rebuilt.prediction_snapshots)
            .await?;
        persist_historical_replay_output(store, observations, serving_model, &rebuilt).await?;
        return Ok(rebuilt.history_points);
    }

    if full_history_refresh {
        let rebuild_dates = target_dates.into_iter().collect::<Vec<_>>();
        tracing::info!(
            release_id = release_filter.unwrap_or("heuristic"),
            rebuild_dates = rebuild_dates.len(),
            "rebuilding full release history from raw observations because cached prediction snapshots are stale or incomplete"
        );
        let rebuilt = build_assessment_history_for_dates(
            DataMode::Sqlite,
            &ScoringEngine::default(),
            indicators,
            observations,
            Some(alerts),
            serving_model,
            user_preferences,
            &rebuild_dates,
        );
        store
            .upsert_prediction_snapshots(&rebuilt.prediction_snapshots)
            .await?;
        persist_historical_replay_output(store, observations, serving_model, &rebuilt).await?;
        return Ok(rebuilt.history_points);
    }

    if let Some(cached_replay) =
        load_cached_historical_replay_output(store, serving_model, observations, &target_dates)
            .await?
    {
        return Ok(cached_replay.history_points);
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

    let should_refresh_recent_formal_history = serving_model
        .and_then(|context| context.probability_bundle.as_ref())
        .is_some()
        && max_history_points > 0;
    if should_refresh_recent_formal_history {
        let recent_dates = target_dates
            .iter()
            .copied()
            .rev()
            .take(max_history_points)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>();
        let recomputed = build_assessment_history_for_dates(
            DataMode::Sqlite,
            &ScoringEngine::default(),
            indicators,
            observations,
            Some(alerts),
            serving_model,
            user_preferences,
            &recent_dates,
        );
        store
            .upsert_prediction_snapshots(&recomputed.prediction_snapshots)
            .await?;
        let mut combined_snapshots = historical.prediction_snapshots.clone();
        combined_snapshots.extend(recomputed.prediction_snapshots.clone());
        historical = merge_historical_outputs(
            historical_output_from_prediction_snapshots(combined_snapshots, release_filter),
            recomputed,
        );
    }

    Ok(historical.history_points)
}

pub(crate) fn select_assessment_history(
    points: &[fc_domain::AssessmentHistoryPoint],
    window: HistoryQueryWindow,
) -> Vec<fc_domain::AssessmentHistoryPoint> {
    let mut filtered = points
        .iter()
        .filter(|point| window.from.is_none_or(|from| point.as_of_date >= from))
        .filter(|point| window.to.is_none_or(|to| point.as_of_date <= to))
        .cloned()
        .collect::<Vec<_>>();
    if let Some(limit) = window.limit {
        if filtered.len() > limit {
            filtered = filtered[filtered.len().saturating_sub(limit)..].to_vec();
        }
    }
    filtered
}

pub(crate) fn select_backtest_timeline(
    points: &[BacktestWindowPoint],
    window: HistoryQueryWindow,
) -> Vec<BacktestWindowPoint> {
    let mut filtered = points
        .iter()
        .filter(|point| window.from.is_none_or(|from| point.as_of_date >= from))
        .filter(|point| window.to.is_none_or(|to| point.as_of_date <= to))
        .cloned()
        .collect::<Vec<_>>();
    if let Some(limit) = window.limit {
        if filtered.len() > limit {
            filtered = filtered[filtered.len().saturating_sub(limit)..].to_vec();
        }
    }
    filtered
}

pub(crate) fn load_user_preferences() -> UserRiskPreferences {
    let profile = match env::var("FC_USER_RISK_PROFILE")
        .unwrap_or_else(|_| "neutral".to_string())
        .to_lowercase()
        .as_str()
    {
        "conservative" => UserRiskProfile::Conservative,
        "aggressive" => UserRiskProfile::Aggressive,
        _ => UserRiskProfile::Neutral,
    };
    let cash_floor_pct = env::var("FC_USER_CASH_FLOOR_PCT")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(15.0);
    let max_equity_cap_pct = env::var("FC_USER_MAX_EQUITY_CAP_PCT")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(70.0);
    let max_leverage_pct = env::var("FC_USER_MAX_LEVERAGE_PCT")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(100.0);
    let option_overlay_preference_pct = env::var("FC_USER_OPTION_OVERLAY_PCT")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(5.0);
    let allow_aggressive_reentry = env::var("FC_USER_ALLOW_AGGRESSIVE_REENTRY")
        .ok()
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "True"))
        .unwrap_or(false);

    let note = format!(
        "profile={}, cash_floor={}%, max_equity={}%, max_leverage={}%, option_overlay={}%",
        match profile {
            UserRiskProfile::Conservative => "conservative",
            UserRiskProfile::Neutral => "neutral",
            UserRiskProfile::Aggressive => "aggressive",
        },
        cash_floor_pct,
        max_equity_cap_pct,
        max_leverage_pct,
        option_overlay_preference_pct
    );

    UserRiskPreferences {
        profile,
        cash_floor_pct,
        max_equity_cap_pct,
        max_leverage_pct,
        option_overlay_preference_pct,
        allow_aggressive_reentry,
        note,
    }
}

fn indicators() -> Vec<Indicator> {
    vec![
        indicator(
            "us_market_vix_close",
            "VIX 收盘价",
            RiskDimension::MarketStress,
            "美国市场隐含波动率。",
            "index",
            Frequency::Daily,
            RiskDirection::HigherIsRiskier,
            "fred",
        ),
        indicator(
            "us_credit_high_yield_oas",
            "高收益债 OAS",
            RiskDimension::LeverageCredit,
            "美国高收益债期权调整利差。",
            "percent",
            Frequency::Daily,
            RiskDirection::HigherIsRiskier,
            "fred",
        ),
        indicator(
            "us_rates_yield_curve_10y2y",
            "10Y-2Y 期限利差",
            RiskDimension::MarketStress,
            "美国 10 年期和 2 年期国债收益率利差。",
            "percent",
            Frequency::Daily,
            RiskDirection::LowerIsRiskier,
            "fred",
        ),
        indicator(
            "us_liquidity_national_financial_conditions",
            "NFCI 金融条件指数",
            RiskDimension::LiquidityFunding,
            "Chicago Fed National Financial Conditions Index。",
            "index",
            Frequency::Weekly,
            RiskDirection::HigherIsRiskier,
            "fred",
        ),
        indicator(
            "us_liquidity_effr",
            "有效联邦基金利率",
            RiskDimension::LiquidityFunding,
            "美国有效联邦基金利率。",
            "percent",
            Frequency::Daily,
            RiskDirection::RisingFastIsRiskier,
            "fred",
        ),
        indicator(
            "us_macro_unemployment_rate",
            "失业率",
            RiskDimension::MacroFragility,
            "美国失业率。",
            "percent",
            Frequency::Monthly,
            RiskDirection::HigherIsRiskier,
            "fred",
        ),
        indicator(
            "us_banking_deposits_growth",
            "银行存款增速",
            RiskDimension::BankingSystem,
            "银行存款同比或近似增速。",
            "percent",
            Frequency::Weekly,
            RiskDirection::LowerIsRiskier,
            "fred",
        ),
        indicator(
            "us_real_estate_home_price_yoy",
            "房价同比",
            RiskDimension::RealEstate,
            "全国房价同比变化。",
            "percent",
            Frequency::Monthly,
            RiskDirection::TwoSided,
            "fred",
        ),
        indicator(
            "global_external_current_account_gdp",
            "经常账户/GDP",
            RiskDimension::ExternalSector,
            "经常账户余额占 GDP 比重。",
            "percent",
            Frequency::Annual,
            RiskDirection::LowerIsRiskier,
            "world_bank",
        ),
        indicator(
            "us_external_usdjpy_level",
            "USDJPY 汇率",
            RiskDimension::ExternalSector,
            "美元兑日元水平，用于识别日元套息平仓风险放大器。",
            "jpy_per_usd",
            Frequency::Daily,
            RiskDirection::TwoSided,
            "boj",
        ),
        indicator(
            "jp_rates_call_rate",
            "日本无担保隔夜拆借利率",
            RiskDimension::ExternalSector,
            "日本无担保隔夜拆借利率，作为日元融资成本代理。",
            "percent",
            Frequency::Daily,
            RiskDirection::RisingFastIsRiskier,
            "boj",
        ),
        indicator(
            "global_news_financial_stress_count",
            "金融压力新闻数量",
            RiskDimension::EventsSentiment,
            "金融压力相关新闻数量。",
            "count",
            Frequency::Daily,
            RiskDirection::HigherIsRiskier,
            "gdelt",
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn indicator(
    indicator_id: &str,
    display_name: &str,
    dimension: RiskDimension,
    description: &str,
    unit: &str,
    frequency: Frequency,
    risk_direction: RiskDirection,
    default_source_id: &str,
) -> Indicator {
    Indicator {
        indicator_id: indicator_id.to_string(),
        display_name: display_name.to_string(),
        dimension,
        description: description.to_string(),
        unit: unit.to_string(),
        frequency,
        risk_direction,
        default_source_id: default_source_id.to_string(),
        quality_tier: "core".to_string(),
    }
}

fn observations(as_of_date: NaiveDate) -> Vec<Observation> {
    let mut rows = Vec::new();
    rows.extend(series(
        "us_market_vix_close",
        "fred",
        Frequency::Daily,
        "index",
        as_of_date,
        &[
            18.0, 21.0, 79.0, 32.0, 20.0, 15.0, 17.0, 66.0, 28.0, 20.0, 18.0, 25.0, 24.0,
        ],
        96.0,
        &[],
    ));
    rows.extend(series(
        "us_credit_high_yield_oas",
        "fred",
        Frequency::Daily,
        "percent",
        as_of_date,
        &[
            3.1, 4.2, 10.8, 7.9, 3.8, 3.4, 4.6, 8.7, 4.1, 3.7, 4.5, 5.8, 5.2,
        ],
        95.0,
        &[],
    ));
    rows.extend(series(
        "us_rates_yield_curve_10y2y",
        "fred",
        Frequency::Daily,
        "percent",
        as_of_date,
        &[
            1.2, 0.8, -0.8, -0.2, 0.5, 0.1, -1.05, -0.6, -0.1, 0.0, -0.35, -0.55, -0.45,
        ],
        94.0,
        &[],
    ));
    rows.extend(series(
        "us_liquidity_national_financial_conditions",
        "fred",
        Frequency::Weekly,
        "index",
        as_of_date,
        &[
            -0.4, -0.2, 4.0, 1.2, -0.3, -0.4, 0.1, 1.6, 0.2, -0.1, 0.25, 0.7, 0.55,
        ],
        92.0,
        &[],
    ));
    rows.extend(series(
        "us_liquidity_effr",
        "fred",
        Frequency::Daily,
        "percent",
        as_of_date,
        &[
            0.15, 0.18, 0.12, 0.09, 4.85, 5.10, 5.30, 5.32, 5.31, 5.30, 5.28, 5.20, 5.12,
        ],
        94.0,
        &[],
    ));
    rows.extend(series(
        "us_macro_unemployment_rate",
        "fred",
        Frequency::Monthly,
        "percent",
        as_of_date,
        &[
            4.6, 5.8, 10.0, 7.8, 4.2, 3.7, 3.5, 14.7, 6.2, 4.0, 3.8, 4.3, 4.1,
        ],
        91.0,
        &[],
    ));
    rows.extend(series(
        "us_banking_deposits_growth",
        "fred",
        Frequency::Weekly,
        "percent",
        as_of_date,
        &[
            7.0, 5.5, -3.5, -1.4, 4.0, 5.2, 2.3, -2.1, 1.1, 2.0, 0.2, -1.2, -0.8,
        ],
        86.0,
        &[],
    ));
    rows.extend(series(
        "us_real_estate_home_price_yoy",
        "fred",
        Frequency::Monthly,
        "percent",
        as_of_date,
        &[
            7.0, 12.5, -8.2, -4.1, 3.2, 5.6, 6.8, 13.5, 10.1, 4.8, 3.2, 5.2, 4.5,
        ],
        87.0,
        &[],
    ));
    rows.extend(series(
        "global_external_current_account_gdp",
        "world_bank",
        Frequency::Annual,
        "percent",
        as_of_date,
        &[
            -2.0, -4.2, -6.1, -3.5, -1.5, -1.8, -2.1, -4.8, -3.2, -2.0, -1.7, -3.1, -2.7,
        ],
        82.0,
        &[],
    ));
    rows.extend(series(
        "us_external_usdjpy_level",
        "boj",
        Frequency::Daily,
        "jpy_per_usd",
        as_of_date,
        &[
            106.0, 110.0, 93.0, 101.0, 115.0, 130.0, 151.0, 141.0, 144.0, 149.0, 156.0, 151.0,
            148.0,
        ],
        92.0,
        &[],
    ));
    rows.extend(series_for_entity(
        "jp_rates_call_rate",
        "jp",
        "boj",
        Frequency::Daily,
        "percent",
        as_of_date,
        &[
            -0.08, -0.07, -0.1, -0.09, 0.03, 0.08, 0.12, 0.18, 0.22, 0.29, 0.38, 0.44, 0.48,
        ],
        97.0,
        &[],
    ));
    rows.extend(series(
        "global_news_financial_stress_count",
        "gdelt",
        Frequency::Daily,
        "count",
        as_of_date,
        &[
            40.0, 72.0, 210.0, 128.0, 52.0, 44.0, 61.0, 180.0, 82.0, 70.0, 65.0, 110.0, 96.0,
        ],
        78.0,
        &["prototype_source"],
    ));
    rows
}

#[allow(clippy::too_many_arguments)]
fn series(
    indicator_id: &str,
    source_id: &str,
    frequency: Frequency,
    unit: &str,
    as_of_date: NaiveDate,
    values: &[f64],
    quality_score: f64,
    flags: &[&str],
) -> Vec<Observation> {
    series_for_entity(
        indicator_id,
        "us",
        source_id,
        frequency,
        unit,
        as_of_date,
        values,
        quality_score,
        flags,
    )
}

#[allow(clippy::too_many_arguments)]
fn series_for_entity(
    indicator_id: &str,
    entity_id: &str,
    source_id: &str,
    frequency: Frequency,
    unit: &str,
    as_of_date: NaiveDate,
    values: &[f64],
    quality_score: f64,
    flags: &[&str],
) -> Vec<Observation> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let days_back = (values.len() - index - 1) as i64 * 30;
            let date = as_of_date - Duration::days(days_back);
            Observation {
                indicator_id: indicator_id.to_string(),
                entity_id: entity_id.to_string(),
                as_of_date: date,
                period_start: Some(date),
                period_end: Some(date),
                frequency,
                value: *value,
                unit: unit.to_string(),
                source_id: source_id.to_string(),
                dataset_id: "demo".to_string(),
                revision_time: None,
                publication_time: Some(Utc::now()),
                quality_score,
                quality_flags: flags.iter().map(|flag| (*flag).to_string()).collect(),
            }
        })
        .collect()
}

fn select_recent_alerts_for_date(alerts: &[AlertEvent], as_of_date: NaiveDate) -> Vec<AlertEvent> {
    let floor = as_of_date - Duration::days(EVENT_LOOKBACK_DAYS);
    let mut filtered = alerts
        .iter()
        .filter(|alert| alert.triggered_as_of_date >= floor)
        .filter(|alert| alert.triggered_as_of_date <= as_of_date)
        .cloned()
        .collect::<Vec<_>>();
    filtered.sort_by(|a, b| {
        b.triggered_as_of_date
            .cmp(&a.triggered_as_of_date)
            .then_with(|| b.score.total_cmp(&a.score))
    });
    filtered
}

fn sources_demo() -> Vec<DataSource> {
    vec![
        source(
            "fred",
            "FRED",
            "macro_financial_timeseries",
            SourcePriority::P0,
            SourceStatus::Healthy,
            96.0,
            true,
            "FRED graph CSV is the default no-key source; official API remains optional.",
        ),
        source(
            "treasury",
            "U.S. Treasury",
            "government_timeseries",
            SourcePriority::P0,
            SourceStatus::Healthy,
            96.0,
            true,
            "Official no-key Treasury yield curve data.",
        ),
        source(
            "sec_edgar",
            "SEC EDGAR",
            "filings_events",
            SourcePriority::P0,
            SourceStatus::Prototype,
            72.0,
            false,
            "Official SEC JSON APIs. Runtime connector is available in SQLite mode; this demo source only marks the capability shape.",
        ),
        source(
            "world_bank",
            "World Bank Indicators",
            "global_macro",
            SourcePriority::P0,
            SourceStatus::Healthy,
            90.0,
            true,
            "Official World Bank Indicators API.",
        ),
        source(
            "boj",
            "Bank of Japan",
            "fx_rates_timeseries",
            SourcePriority::P1,
            SourceStatus::Delayed,
            84.0,
            true,
            "Official BOJ FX and time-series endpoints are tracked as the preferred JPY carry enhancement source.",
        ),
        source(
            "gdelt",
            "GDELT",
            "news_events",
            SourcePriority::P1,
            SourceStatus::Prototype,
            66.0,
            false,
            "News-event prototype source. Optional runtime backfill is available, but noise filtering and production validation are still pending.",
        ),
        source(
            "yfinance",
            "yfinance",
            "market_price_prototype",
            SourcePriority::P1,
            SourceStatus::Prototype,
            62.0,
            false,
            "Development-only market data prototype; not a production dependency.",
        ),
    ]
}

fn sources_runtime(observations: &[Observation], as_of_date: NaiveDate) -> Vec<DataSource> {
    let gdelt_has_data = observations
        .iter()
        .any(|observation| observation.source_id == "gdelt");
    vec![
        live_source(
            observations,
            as_of_date,
            "fred",
            "FRED",
            "macro_financial_timeseries",
            SourcePriority::P0,
            7,
            96.0,
            true,
            "FRED graph CSV is the default no-key source; official API remains optional.",
        ),
        live_source(
            observations,
            as_of_date,
            "treasury",
            "U.S. Treasury",
            "government_timeseries",
            SourcePriority::P0,
            7,
            96.0,
            true,
            "Official no-key Treasury yield curve data.",
        ),
        live_source(
            observations,
            as_of_date,
            "sec_edgar",
            "SEC EDGAR",
            "filings_events",
            SourcePriority::P0,
            7,
            88.0,
            true,
            "Official SEC JSON filings metadata aggregated into daily event features. No paid key is required.",
        ),
        live_source(
            observations,
            as_of_date,
            "world_bank",
            "World Bank Indicators",
            "global_macro",
            SourcePriority::P0,
            730,
            90.0,
            true,
            "Official World Bank Indicators API.",
        ),
        live_source(
            observations,
            as_of_date,
            "boj",
            "Bank of Japan",
            "fx_rates_timeseries",
            SourcePriority::P1,
            3,
            84.0,
            true,
            "Official BOJ FX and money-market endpoints are used for the JPY carry monitor.",
        ),
        if gdelt_has_data {
            live_source(
                observations,
                as_of_date,
                "gdelt",
                "GDELT",
                "news_events",
                SourcePriority::P1,
                3,
                66.0,
                false,
                "GDELT 聚合新闻压力序列已支持本地回填和运行时展示，但仍属于 prototype 辅助信号。",
            )
        } else {
            source(
                "gdelt",
                "GDELT",
                "news_events",
                SourcePriority::P1,
                SourceStatus::Prototype,
                66.0,
                false,
                "GDELT 聚合新闻压力序列可选接入；当前本地库尚未回填该源。",
            )
        },
        source(
            "yfinance",
            "yfinance",
            "market_price_prototype",
            SourcePriority::P1,
            SourceStatus::Prototype,
            62.0,
            false,
            "Development-only market data prototype; not a production dependency.",
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn live_source(
    observations: &[Observation],
    as_of_date: NaiveDate,
    source_id: &str,
    display_name: &str,
    source_type: &str,
    priority: SourcePriority,
    stale_days: i64,
    fallback_quality_score: f64,
    production_allowed: bool,
    license_note: &str,
) -> DataSource {
    let latest = observations
        .iter()
        .filter(|observation| observation.source_id == source_id)
        .max_by_key(|observation| observation.as_of_date);
    match latest {
        Some(observation) => {
            let lag_days = (as_of_date - observation.as_of_date).num_days();
            let status = if lag_days > stale_days * 3 {
                SourceStatus::PartialFailure
            } else if lag_days > stale_days {
                SourceStatus::Delayed
            } else {
                SourceStatus::Healthy
            };
            let message = format!(
                "latest observation {} (lag {}d, dataset={})",
                observation.as_of_date, lag_days, observation.dataset_id
            );
            runtime_source(
                source_id,
                display_name,
                source_type,
                priority,
                status,
                if observation.quality_score > 0.0 {
                    observation.quality_score
                } else {
                    fallback_quality_score
                },
                production_allowed,
                license_note,
                observation.publication_time.or(Some(Utc::now())),
                Some(lag_days.saturating_mul(86_400)),
                message,
            )
        }
        None => runtime_source(
            source_id,
            display_name,
            source_type,
            priority,
            SourceStatus::Delayed,
            fallback_quality_score,
            production_allowed,
            license_note,
            None,
            None,
            "connector available, but no local observations are loaded yet".to_string(),
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn source(
    source_id: &str,
    display_name: &str,
    source_type: &str,
    priority: SourcePriority,
    status: SourceStatus,
    quality_score: f64,
    production_allowed: bool,
    license_note: &str,
) -> DataSource {
    DataSource {
        source_id: source_id.to_string(),
        display_name: display_name.to_string(),
        source_type: source_type.to_string(),
        priority,
        access_method: "api".to_string(),
        documentation_url: None,
        production_allowed,
        license_note: license_note.to_string(),
        health: SourceHealth {
            status,
            last_success_at: Some(Utc::now()),
            lag_seconds: Some(if status == SourceStatus::Delayed {
                14_400
            } else {
                0
            }),
            consecutive_failures: 0,
            quality_score,
            message: match status {
                SourceStatus::Healthy => "source healthy".to_string(),
                SourceStatus::Delayed => "source delayed but usable".to_string(),
                SourceStatus::Prototype => "prototype source, not for production".to_string(),
                SourceStatus::PartialFailure => "partial failure".to_string(),
                SourceStatus::Failed => "source failed".to_string(),
                SourceStatus::Disabled => "source disabled".to_string(),
            },
        },
    }
}

#[allow(clippy::too_many_arguments)]
fn runtime_source(
    source_id: &str,
    display_name: &str,
    source_type: &str,
    priority: SourcePriority,
    status: SourceStatus,
    quality_score: f64,
    production_allowed: bool,
    license_note: &str,
    last_success_at: Option<chrono::DateTime<Utc>>,
    lag_seconds: Option<i64>,
    message: String,
) -> DataSource {
    DataSource {
        source_id: source_id.to_string(),
        display_name: display_name.to_string(),
        source_type: source_type.to_string(),
        priority,
        access_method: "api".to_string(),
        documentation_url: None,
        production_allowed,
        license_note: license_note.to_string(),
        health: SourceHealth {
            status,
            last_success_at,
            lag_seconds,
            consecutive_failures: 0,
            quality_score,
            message,
        },
    }
}

fn build_alerts(snapshot: &fc_domain::RiskSnapshot) -> Vec<AlertEvent> {
    let top = snapshot.top_contributors.clone();
    let credit_alert = AlertEvent {
        alert_id: Uuid::new_v4(),
        event_type: AlertType::RiskStress,
        scope: "dimension".to_string(),
        entity_id: "us".to_string(),
        dimension: Some(RiskDimension::LeverageCredit),
        level: RiskLevel::Stress,
        status: AlertStatus::Open,
        triggered_at: Utc::now(),
        triggered_as_of_date: snapshot.as_of_date,
        resolved_at: None,
        score: snapshot
            .dimensions
            .iter()
            .find(|dimension| dimension.dimension == RiskDimension::LeverageCredit)
            .map(|dimension| dimension.score)
            .unwrap_or(snapshot.overall_score),
        previous_score: Some(48.0),
        trigger_reason: "高收益债 OAS 和期限结构信号同时恶化。".to_string(),
        top_contributors: top.iter().take(3).cloned().collect(),
        related_indicators: vec![
            "us_credit_high_yield_oas".to_string(),
            "us_rates_yield_curve_10y2y".to_string(),
        ],
        method_version: snapshot.method_version.clone(),
    };

    let source_alert = AlertEvent {
        alert_id: Uuid::new_v4(),
        event_type: AlertType::SourceHealthIssue,
        scope: "data_source".to_string(),
        entity_id: "gdelt".to_string(),
        dimension: None,
        level: RiskLevel::Watch,
        status: AlertStatus::Monitoring,
        triggered_at: Utc::now(),
        triggered_as_of_date: snapshot.as_of_date,
        resolved_at: None,
        score: 35.0,
        previous_score: Some(20.0),
        trigger_reason: "GDELT 事件源仍处于 prototype 状态，事件维度质量降级。".to_string(),
        top_contributors: Vec::new(),
        related_indicators: vec!["global_news_financial_stress_count".to_string()],
        method_version: snapshot.method_version.clone(),
    };

    vec![credit_alert, source_alert]
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, TimeZone, Utc};
    use fc_domain::{
        load_protected_stress_window_catalog, DecisionPosture, ModelReleaseManifest,
        ModelReleaseRecord, PredictionSnapshotRecord, ProbabilityBundle, TimeToRiskBucket,
    };

    use super::{
        build_rolling_backtest_audit, expected_prediction_snapshot_method_version,
        historical_output_from_prediction_snapshots, is_actionable_warning_point,
        should_refresh_full_release_history, use_transitional_actionable_bridge,
        ServingModelContext,
    };

    fn history_point(
        as_of_date: NaiveDate,
        overall_score: f64,
        posture: DecisionPosture,
        time_to_risk_bucket: TimeToRiskBucket,
        external_shock_score: f64,
    ) -> fc_domain::AssessmentHistoryPoint {
        fc_domain::AssessmentHistoryPoint {
            as_of_date,
            overall_score,
            p_5d: 0.026,
            p_20d: 0.026,
            p_60d: 0.056,
            raw_p_5d: Some(0.012),
            raw_p_20d: Some(0.028),
            raw_p_60d: Some(0.081),
            posture,
            time_to_risk_bucket,
            external_shock_score,
            posture_trigger_codes: Vec::new(),
            posture_blocker_codes: Vec::new(),
        }
    }

    fn snapshot(
        as_of_date: NaiveDate,
        release_id: Option<&str>,
        p_20d: f64,
        posture: &str,
        recorded_at_hour: u32,
    ) -> PredictionSnapshotRecord {
        PredictionSnapshotRecord {
            as_of_date,
            entity_id: "us".to_string(),
            market_scope: "financial_system".to_string(),
            release_id: release_id.map(str::to_string),
            probability_mode: "heuristic_mvp".to_string(),
            release_status: "degraded".to_string(),
            point_in_time_mode: "best_effort".to_string(),
            overall_score: 42.0,
            external_shock_score: 25.0,
            raw_p_5d: 0.01,
            raw_p_20d: p_20d,
            raw_p_60d: 0.08,
            calibrated_p_5d: 0.01,
            calibrated_p_20d: p_20d,
            calibrated_p_60d: 0.08,
            posture: posture.to_string(),
            time_to_risk_bucket: "weeks".to_string(),
            feature_set_version: "feature_v2".to_string(),
            label_version: "label_v1".to_string(),
            coverage_score: 0.95,
            freshness_status: "fresh".to_string(),
            method_version: "score_v1".to_string(),
            posture_trigger_codes: vec!["prepare_p60d_structural".to_string()],
            posture_blocker_codes: Vec::new(),
            recorded_at: Utc
                .with_ymd_and_hms(2026, 5, 31, recorded_at_hour, 0, 0)
                .single()
                .unwrap(),
        }
    }

    fn formal_serving_model_context() -> ServingModelContext {
        ServingModelContext {
            release: ModelReleaseRecord {
                manifest: ModelReleaseManifest {
                    release_id: "formal-release".to_string(),
                    market_scope: "financial_system".to_string(),
                    status: "active".to_string(),
                    probability_mode: "formal_bundle_v1".to_string(),
                    serving_status: "healthy".to_string(),
                    bundle_uri: "bundle.json".to_string(),
                    feature_set_version: super::FORMAL_MAIN_FEATURE_SET_VERSION.to_string(),
                    label_version: super::FORMAL_MAIN_LABEL_VERSION.to_string(),
                    prob_model_version: "prob_bundle_test".to_string(),
                    calibration_version: "platt_test".to_string(),
                    posture_policy_version: "posture_test".to_string(),
                    action_playbook_version: "action_test".to_string(),
                    point_in_time_mode: "best_effort".to_string(),
                    training_range_start: None,
                    training_range_end: None,
                    calibration_range_start: None,
                    calibration_range_end: None,
                    evaluation_range_start: None,
                    evaluation_range_end: None,
                    brier_score: None,
                    log_loss: None,
                    ece: None,
                    note: String::new(),
                },
                created_at: Utc::now(),
                activated_at: None,
                retired_at: None,
            },
            probability_bundle: Some(ProbabilityBundle {
                bundle_id: "bundle".to_string(),
                market_scope: "financial_system".to_string(),
                probability_mode: "formal_bundle_v1".to_string(),
                model_family: "linear_v1".to_string(),
                feature_transform: "identity_v1".to_string(),
                created_at: Utc::now(),
                feature_names: Vec::new(),
                monotonic_min_gap_5d_to_20d: 0.0,
                monotonic_min_gap_20d_to_60d: 0.0,
                note: String::new(),
                horizons: Vec::new(),
                evaluation: None,
                actionability: None,
            }),
            runtime_probability_mode: "formal_bundle_v1".to_string(),
            runtime_release_status: "healthy".to_string(),
        }
    }

    #[test]
    fn prediction_history_filters_by_release_and_keeps_latest_daily_snapshot() {
        let as_of_date = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();
        let output = historical_output_from_prediction_snapshots(
            vec![
                snapshot(as_of_date, Some("release-a"), 0.12, "normal", 1),
                snapshot(as_of_date, Some("release-a"), 0.27, "hedge", 3),
                snapshot(as_of_date, Some("release-b"), 0.88, "defend", 4),
            ],
            Some("release-a"),
        );

        assert_eq!(output.history_points.len(), 1);
        assert_eq!(output.prediction_snapshots.len(), 1);
        assert_eq!(output.history_points[0].p_20d, 0.27);
        assert_eq!(
            output.history_points[0].posture,
            fc_domain::DecisionPosture::Hedge
        );
        assert_eq!(
            output.history_points[0].posture_trigger_codes,
            vec!["prepare_p60d_structural".to_string()]
        );
    }

    #[test]
    fn actionable_warning_point_accepts_prepare_bridge_for_persisted_snapshots() {
        let point = history_point(
            NaiveDate::from_ymd_opt(2008, 7, 25).unwrap(),
            58.0,
            DecisionPosture::Prepare,
            TimeToRiskBucket::Normal,
            46.0,
        );

        assert!(is_actionable_warning_point(&point, true));
    }

    #[test]
    fn actionable_warning_point_rejects_weak_prepare_bridge() {
        let point = history_point(
            NaiveDate::from_ymd_opt(2008, 7, 25).unwrap(),
            57.9,
            DecisionPosture::Prepare,
            TimeToRiskBucket::Normal,
            45.9,
        );

        assert!(!is_actionable_warning_point(&point, true));
    }

    #[test]
    fn actionable_warning_point_disables_prepare_bridge_for_formal_main() {
        let point = history_point(
            NaiveDate::from_ymd_opt(2008, 7, 25).unwrap(),
            58.0,
            DecisionPosture::Prepare,
            TimeToRiskBucket::Normal,
            46.0,
        );

        assert!(!is_actionable_warning_point(&point, false));
    }

    #[test]
    fn actionable_warning_point_accepts_strong_prepare_clause_for_formal_main() {
        let point = fc_domain::AssessmentHistoryPoint {
            as_of_date: NaiveDate::from_ymd_opt(2007, 3, 1).unwrap(),
            overall_score: 53.4,
            p_5d: 0.03,
            p_20d: 0.70,
            p_60d: 0.73,
            raw_p_5d: Some(0.02),
            raw_p_20d: Some(0.68),
            raw_p_60d: Some(0.70),
            posture: DecisionPosture::Prepare,
            time_to_risk_bucket: TimeToRiskBucket::Months,
            external_shock_score: 38.5,
            posture_trigger_codes: vec!["prepare_p60d_structural".to_string()],
            posture_blocker_codes: Vec::new(),
        };

        assert!(is_actionable_warning_point(&point, false));
    }

    #[test]
    fn actionable_warning_point_rejects_weak_prepare_clause_for_formal_main() {
        let point = fc_domain::AssessmentHistoryPoint {
            as_of_date: NaiveDate::from_ymd_opt(2007, 3, 1).unwrap(),
            overall_score: 52.9,
            p_5d: 0.03,
            p_20d: 0.70,
            p_60d: 0.73,
            raw_p_5d: Some(0.02),
            raw_p_20d: Some(0.68),
            raw_p_60d: Some(0.70),
            posture: DecisionPosture::Prepare,
            time_to_risk_bucket: TimeToRiskBucket::Months,
            external_shock_score: 38.5,
            posture_trigger_codes: vec!["prepare_p60d_structural".to_string()],
            posture_blocker_codes: Vec::new(),
        };

        assert!(!is_actionable_warning_point(&point, false));
    }

    #[test]
    fn rolling_audit_counts_catalog_protected_windows_as_stress() {
        let stress_windows = load_protected_stress_window_catalog();
        let history = vec![history_point(
            NaiveDate::from_ymd_opt(2015, 9, 1).unwrap(),
            60.0,
            DecisionPosture::Prepare,
            TimeToRiskBucket::Months,
            46.0,
        )];

        let audit = build_rolling_backtest_audit(&history, &stress_windows.windows, true);

        assert_eq!(audit.actionable_signal_count, 1);
        assert_eq!(audit.stress_window_signal_count, 1);
        assert_eq!(audit.pre_crisis_signal_count, 0);
        assert_eq!(audit.false_positive_signal_count, 0);
        assert_eq!(audit.classified_episodes.len(), 1);
        assert_eq!(audit.classified_episodes[0].classification, "stress_window");
    }

    #[test]
    fn rolling_audit_counts_prepare_signal_within_sixty_days_as_pre_crisis() {
        let history = vec![fc_domain::AssessmentHistoryPoint {
            as_of_date: NaiveDate::from_ymd_opt(2000, 1, 31).unwrap(),
            overall_score: 63.0,
            p_5d: 0.03,
            p_20d: 0.19,
            p_60d: 0.48,
            raw_p_5d: Some(0.02),
            raw_p_20d: Some(0.18),
            raw_p_60d: Some(0.45),
            posture: DecisionPosture::Prepare,
            time_to_risk_bucket: TimeToRiskBucket::Months,
            external_shock_score: 49.0,
            posture_trigger_codes: vec!["prepare_p60d_structural".to_string()],
            posture_blocker_codes: Vec::new(),
        }];

        let audit = build_rolling_backtest_audit(&history, &[], false);

        assert_eq!(audit.actionable_signal_count, 1);
        assert_eq!(audit.pre_crisis_signal_count, 1);
        assert_eq!(audit.false_positive_signal_count, 0);
    }

    #[test]
    fn bundle_backed_history_refreshes_when_cached_method_version_is_stale() {
        let serving_model = formal_serving_model_context();
        let mut persisted = vec![snapshot(
            NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
            Some("formal-release"),
            0.27,
            "hedge",
            3,
        )];
        persisted[0].method_version = "legacy-cache".to_string();

        assert!(should_refresh_full_release_history(
            Some(&serving_model),
            &persisted,
            false,
        ));
    }

    #[test]
    fn bundle_backed_history_keeps_cache_when_method_version_matches() {
        let serving_model = formal_serving_model_context();
        let expected_method_version =
            expected_prediction_snapshot_method_version(Some(&serving_model));
        let mut persisted = vec![snapshot(
            NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
            Some("formal-release"),
            0.27,
            "hedge",
            3,
        )];
        persisted[0].method_version = expected_method_version;

        assert!(!should_refresh_full_release_history(
            Some(&serving_model),
            &persisted,
            false,
        ));
    }

    #[test]
    fn formal_main_disables_transitional_actionable_bridge() {
        let serving_model = formal_serving_model_context();

        assert!(!use_transitional_actionable_bridge(Some(&serving_model)));
        assert!(use_transitional_actionable_bridge(None));
    }

    #[test]
    fn formal_main_method_version_carries_runtime_policy_cache_key() {
        let serving_model = formal_serving_model_context();
        let method_version = expected_prediction_snapshot_method_version(Some(&serving_model));

        assert!(method_version.contains("runtime_policy="));
        assert!(method_version.contains("class=formal_main"));
    }
}

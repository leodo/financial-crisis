use std::env;

use anyhow::Context;
use chrono::{Duration, NaiveDate, Utc};
use fc_domain::{
    AlertEvent, AlertStatus, AlertType, BacktestScenarioSummary, BacktestSignalSource,
    BacktestWindowPoint, DataMode, DataSource, Frequency, Indicator, Observation, RiskContributor,
    RiskDimension, RiskDirection, RiskLevel, SourceHealth, SourcePriority, SourceStatus,
    UserRiskPreferences, UserRiskProfile,
};
use fc_scoring::ScoringEngine;
use fc_storage::{PostgresStore, SqliteStore};
use uuid::Uuid;

use crate::assessment::{build_assessment_history_point, build_assessment_snapshot};
use crate::AppData;

const EVENT_LOOKBACK_DAYS: i64 = 30;

#[derive(Debug, Clone)]
pub enum AppDataSource {
    Demo,
    Sqlite { path: String },
    Postgres { database_url: String },
}

#[derive(Debug, Clone, Copy)]
pub struct HistoryQueryWindow {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub limit: Option<usize>,
}

pub fn source_from_env() -> anyhow::Result<AppDataSource> {
    match env::var("FC_DATA_MODE").ok().as_deref() {
        Some("postgres") => {
            let database_url = env::var("DATABASE_URL").context("DATABASE_URL is required")?;
            Ok(AppDataSource::Postgres { database_url })
        }
        Some("sqlite") => Ok(AppDataSource::Sqlite {
            path: env::var("FC_SQLITE_PATH").unwrap_or_else(|_| "data/fc-local.sqlite".to_string()),
        }),
        _ => Ok(AppDataSource::Demo),
    }
}

pub async fn load_app_data(
    source: &AppDataSource,
    max_history_points: usize,
) -> anyhow::Result<AppData> {
    match source {
        AppDataSource::Demo => Ok(build_demo_data(max_history_points)),
        AppDataSource::Sqlite { path } => load_sqlite_app_data(path, max_history_points).await,
        AppDataSource::Postgres { database_url } => {
            load_postgres_app_data(database_url, max_history_points).await
        }
    }
}

pub fn build_demo_data(max_history_points: usize) -> AppData {
    let as_of_date = NaiveDate::from_ymd_opt(2026, 5, 30).expect("valid date");
    let indicators = indicators();
    let observations = observations(as_of_date);
    build_app_data_from_inputs(
        DataMode::Demo,
        indicators,
        observations,
        None,
        as_of_date,
        max_history_points,
    )
}

async fn load_postgres_app_data(
    database_url: &str,
    max_history_points: usize,
) -> anyhow::Result<AppData> {
    let as_of_date = Utc::now().date_naive();
    let store = PostgresStore::connect(database_url).await?;
    let indicators = store.load_indicators().await?;
    if indicators.is_empty() {
        anyhow::bail!("metadata.indicators is empty");
    }
    let observations = store
        .load_observations_for_entities(&["us", "jp"], as_of_date)
        .await?;
    if observations.is_empty() {
        anyhow::bail!("ts.indicator_observations has no rows for entity us");
    }
    Ok(build_app_data_from_inputs(
        DataMode::Postgres,
        indicators,
        observations,
        Some(Vec::new()),
        as_of_date,
        max_history_points,
    ))
}

async fn load_sqlite_app_data(
    sqlite_path: &str,
    max_history_points: usize,
) -> anyhow::Result<AppData> {
    let as_of_date = Utc::now().date_naive();
    let store = SqliteStore::connect(sqlite_path).await?;
    store.migrate().await?;
    let indicators = store.load_indicators().await?;
    if indicators.is_empty() {
        anyhow::bail!("metadata_indicators is empty; run `just db-seed` first");
    }
    let observations = store
        .load_observations_for_entities(&["us", "jp"], as_of_date)
        .await?;
    if observations.is_empty() {
        anyhow::bail!(
            "ts_indicator_observations has no rows for entity us; run at least one backfill such as `just backfill-fred`, `just backfill-treasury-yield`, or `just backfill-world-bank` first"
        );
    }
    let alerts = store
        .load_alerts_recent(as_of_date - Duration::days(EVENT_LOOKBACK_DAYS), as_of_date)
        .await?;
    Ok(build_app_data_from_inputs(
        DataMode::Sqlite,
        indicators,
        observations,
        Some(alerts),
        as_of_date,
        max_history_points,
    ))
}

fn build_app_data_from_inputs(
    data_mode: DataMode,
    indicators: Vec<Indicator>,
    observations: Vec<Observation>,
    stored_alerts: Option<Vec<AlertEvent>>,
    as_of_date: NaiveDate,
    _max_history_points: usize,
) -> AppData {
    let scoring = ScoringEngine::default();
    let output = scoring.score(
        &indicators,
        &observations,
        as_of_date,
        "us",
        "financial_system",
    );
    let user_preferences = load_user_preferences();
    let assessment_history = build_assessment_history(
        data_mode,
        &scoring,
        &indicators,
        &observations,
        stored_alerts.as_deref(),
        &user_preferences,
        HistoryQueryWindow {
            from: None,
            to: None,
            limit: None,
        },
    );
    let backtests = build_backtests(&output.snapshot, &assessment_history);
    let alerts = stored_alerts
        .map(|alerts| select_recent_alerts_for_date(&alerts, as_of_date))
        .unwrap_or_else(|| build_alerts(&output.snapshot));
    let (assessment, posture_guidance) = build_assessment_snapshot(
        data_mode,
        &output.snapshot,
        &output.indicator_risks,
        &observations,
        &alerts,
        &backtests,
        &user_preferences,
    );
    let backtest_timeline = build_backtest_timeline(&assessment_history);
    AppData {
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
    }
}

fn build_assessment_history(
    data_mode: DataMode,
    scoring: &ScoringEngine,
    indicators: &[Indicator],
    observations: &[Observation],
    stored_alerts: Option<&[AlertEvent]>,
    user_preferences: &UserRiskPreferences,
    window: HistoryQueryWindow,
) -> Vec<fc_domain::AssessmentHistoryPoint> {
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

    dates
        .into_iter()
        .map(|as_of_date| {
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
            let point_backtests = build_backtests(&output.snapshot, &[]);
            build_assessment_history_point(
                data_mode,
                &output.snapshot,
                &output.indicator_risks,
                observations,
                &point_alerts,
                &point_backtests,
                user_preferences,
            )
        })
        .collect()
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

fn load_user_preferences() -> UserRiskPreferences {
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

fn build_backtests(
    snapshot: &fc_domain::RiskSnapshot,
    history: &[fc_domain::AssessmentHistoryPoint],
) -> Vec<BacktestScenarioSummary> {
    let history_start = history.first().map(|point| point.as_of_date);
    let history_end = history.last().map(|point| point.as_of_date);
    scenario_catalog()
        .into_iter()
        .map(|scenario| {
            scenario_summary_from_history(
                snapshot,
                history,
                &scenario,
                snapshot.top_contributors.iter().take(3).cloned().collect(),
            )
            .unwrap_or_else(|| fallback_backtest(snapshot, &scenario, history_start, history_end))
        })
        .collect()
}

#[derive(Debug, Clone)]
struct ScenarioDefinition {
    scenario_id: &'static str,
    name: &'static str,
    region: &'static str,
    crisis_start: NaiveDate,
    crisis_end: NaiveDate,
    fallback_first_l2_date: Option<NaiveDate>,
    fallback_first_l3_date: Option<NaiveDate>,
    fallback_max_level: RiskLevel,
    fallback_max_score: f64,
    fallback_lead_time_days: Option<i64>,
    fallback_false_positive_count: u32,
}

fn scenario_catalog() -> Vec<ScenarioDefinition> {
    vec![
        ScenarioDefinition {
            scenario_id: "us_gfc_2008",
            name: "2007-2009 全球金融危机",
            region: "US",
            crisis_start: NaiveDate::from_ymd_opt(2007, 8, 1).expect("valid date"),
            crisis_end: NaiveDate::from_ymd_opt(2009, 3, 31).expect("valid date"),
            fallback_first_l2_date: Some(NaiveDate::from_ymd_opt(2007, 6, 15).expect("valid date")),
            fallback_first_l3_date: Some(NaiveDate::from_ymd_opt(2007, 8, 9).expect("valid date")),
            fallback_max_level: RiskLevel::Crisis,
            fallback_max_score: 92.0,
            fallback_lead_time_days: Some(47),
            fallback_false_positive_count: 2,
        },
        ScenarioDefinition {
            scenario_id: "us_covid_liquidity_2020",
            name: "2020 疫情流动性冲击",
            region: "US",
            crisis_start: NaiveDate::from_ymd_opt(2020, 2, 24).expect("valid date"),
            crisis_end: NaiveDate::from_ymd_opt(2020, 4, 30).expect("valid date"),
            fallback_first_l2_date: Some(NaiveDate::from_ymd_opt(2020, 2, 25).expect("valid date")),
            fallback_first_l3_date: Some(NaiveDate::from_ymd_opt(2020, 3, 9).expect("valid date")),
            fallback_max_level: RiskLevel::Crisis,
            fallback_max_score: 88.0,
            fallback_lead_time_days: Some(13),
            fallback_false_positive_count: 1,
        },
        ScenarioDefinition {
            scenario_id: "us_regional_banks_2023",
            name: "2023 美国区域银行危机",
            region: "US",
            crisis_start: NaiveDate::from_ymd_opt(2023, 3, 8).expect("valid date"),
            crisis_end: NaiveDate::from_ymd_opt(2023, 5, 1).expect("valid date"),
            fallback_first_l2_date: Some(NaiveDate::from_ymd_opt(2023, 2, 15).expect("valid date")),
            fallback_first_l3_date: Some(NaiveDate::from_ymd_opt(2023, 3, 10).expect("valid date")),
            fallback_max_level: RiskLevel::Warning,
            fallback_max_score: 78.0,
            fallback_lead_time_days: Some(21),
            fallback_false_positive_count: 1,
        },
    ]
}

fn scenario_summary_from_history(
    snapshot: &fc_domain::RiskSnapshot,
    history: &[fc_domain::AssessmentHistoryPoint],
    scenario: &ScenarioDefinition,
    top_contributors: Vec<RiskContributor>,
) -> Option<BacktestScenarioSummary> {
    let crisis_points = history
        .iter()
        .filter(|point| {
            point.as_of_date >= scenario.crisis_start && point.as_of_date <= scenario.crisis_end
        })
        .cloned()
        .collect::<Vec<_>>();
    if crisis_points.is_empty() {
        return None;
    }

    let warmup_start = scenario.crisis_start - Duration::days(90);
    let warmup_points = history
        .iter()
        .filter(|point| {
            point.as_of_date >= warmup_start && point.as_of_date <= scenario.crisis_start
        })
        .cloned()
        .collect::<Vec<_>>();

    let first_l2_date = warmup_points
        .iter()
        .find(|point| {
            point.p_20d >= 0.35 || !matches!(point.posture, fc_domain::DecisionPosture::Normal)
        })
        .map(|point| point.as_of_date);
    let first_l3_date = warmup_points
        .iter()
        .find(|point| {
            point.p_5d >= 0.30
                || matches!(
                    point.posture,
                    fc_domain::DecisionPosture::Hedge | fc_domain::DecisionPosture::Defend
                )
        })
        .map(|point| point.as_of_date);

    let max_point = crisis_points
        .iter()
        .max_by(|left, right| left.overall_score.total_cmp(&right.overall_score))
        .expect("crisis_points is not empty");
    let earliest_signal = first_l3_date.or(first_l2_date);
    let lead_time_days = earliest_signal
        .map(|date| (scenario.crisis_start - date).num_days())
        .filter(|days| *days >= 0);
    let false_positive_count = warmup_points
        .iter()
        .filter(|point| point.as_of_date < scenario.crisis_start)
        .filter(|point| point.p_5d >= 0.30 || point.p_20d >= 0.50)
        .count() as u32;

    Some(BacktestScenarioSummary {
        scenario_id: scenario.scenario_id.to_string(),
        name: scenario.name.to_string(),
        region: scenario.region.to_string(),
        signal_source: BacktestSignalSource::RealHistory,
        crisis_start: scenario.crisis_start,
        crisis_end: scenario.crisis_end,
        first_l2_date,
        first_l3_date,
        max_level: RiskLevel::from_score(max_point.overall_score),
        max_score: max_point.overall_score,
        lead_time_days,
        false_positive_count,
        missed: earliest_signal.is_none(),
        history_start: crisis_points.first().map(|point| point.as_of_date),
        history_end: crisis_points.last().map(|point| point.as_of_date),
        history_point_count: crisis_points.len() as u32,
        note: format!(
            "该场景来自本地历史库真实评估窗口，共使用 {} 个历史点。",
            crisis_points.len()
        ),
        top_contributors,
        method_version: snapshot.method_version.clone(),
    })
}

fn fallback_backtest(
    snapshot: &fc_domain::RiskSnapshot,
    scenario: &ScenarioDefinition,
    history_start: Option<NaiveDate>,
    history_end: Option<NaiveDate>,
) -> BacktestScenarioSummary {
    BacktestScenarioSummary {
        scenario_id: scenario.scenario_id.to_string(),
        name: scenario.name.to_string(),
        region: scenario.region.to_string(),
        signal_source: BacktestSignalSource::FallbackTemplate,
        crisis_start: scenario.crisis_start,
        crisis_end: scenario.crisis_end,
        first_l2_date: scenario.fallback_first_l2_date,
        first_l3_date: scenario.fallback_first_l3_date,
        max_level: scenario.fallback_max_level,
        max_score: scenario.fallback_max_score,
        lead_time_days: scenario.fallback_lead_time_days,
        false_positive_count: scenario.fallback_false_positive_count,
        missed: false,
        history_start,
        history_end,
        history_point_count: 0,
        note: match (history_start, history_end) {
            (Some(start), Some(end)) => format!(
                "本地历史库当前只覆盖 {start} 到 {end}，尚未覆盖该危机窗口，当前结果来自内置参考模板。"
            ),
            _ => "本地历史库尚未覆盖该危机窗口，当前结果来自内置参考模板。".to_string(),
        },
        top_contributors: snapshot.top_contributors.iter().take(3).cloned().collect(),
        method_version: snapshot.method_version.clone(),
    }
}

fn build_backtest_timeline(
    history: &[fc_domain::AssessmentHistoryPoint],
) -> Vec<BacktestWindowPoint> {
    history
        .iter()
        .map(|point| BacktestWindowPoint {
            as_of_date: point.as_of_date,
            overall_score: point.overall_score,
            p_5d: point.p_5d,
            p_20d: point.p_20d,
            p_60d: point.p_60d,
            posture: point.posture,
            crisis_window_open: point.p_5d >= 0.30 || point.p_20d >= 0.50,
        })
        .collect()
}

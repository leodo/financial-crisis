use std::env;

use anyhow::Context;
use chrono::{Duration, NaiveDate, Utc};
use fc_domain::{
    AlertEvent, AlertStatus, AlertType, BacktestScenarioSummary, DataSource, Frequency, Indicator,
    Observation, RiskDimension, RiskDirection, RiskLevel, SourceHealth, SourcePriority,
    SourceStatus,
};
use fc_scoring::ScoringEngine;
use fc_storage::{PostgresStore, SqliteStore};
use uuid::Uuid;

use crate::AppData;

pub async fn load_app_data() -> AppData {
    match env::var("FC_DATA_MODE").ok().as_deref() {
        Some("postgres") => match load_postgres_app_data().await {
            Ok(data) => return data,
            Err(error) => panic!("postgres data mode failed: {error:#}"),
        },
        Some("sqlite") => match load_sqlite_app_data().await {
            Ok(data) => return data,
            Err(error) => panic!("sqlite data mode failed: {error:#}"),
        },
        _ => {
            // Demo mode remains the default so a fresh checkout can start without a database.
        }
    }
    build_demo_data()
}

pub fn build_demo_data() -> AppData {
    let as_of_date = NaiveDate::from_ymd_opt(2026, 5, 30).expect("valid date");
    let indicators = indicators();
    let observations = observations(as_of_date);
    let scoring = ScoringEngine::default();
    let output = scoring.score(
        &indicators,
        &observations,
        as_of_date,
        "us",
        "financial_system",
    );
    let alerts = build_alerts(&output.snapshot);
    let backtests = build_backtests(&output.snapshot);
    AppData {
        overview: output.snapshot,
        indicators: output.indicator_risks,
        alerts,
        sources: sources(),
        backtests,
    }
}

async fn load_postgres_app_data() -> anyhow::Result<AppData> {
    let database_url = env::var("DATABASE_URL").context("DATABASE_URL is required")?;
    let as_of_date = Utc::now().date_naive();
    let store = PostgresStore::connect(&database_url).await?;
    let indicators = store.load_indicators().await?;
    if indicators.is_empty() {
        anyhow::bail!("metadata.indicators is empty");
    }
    let observations = store.load_observations("us", as_of_date).await?;
    if observations.is_empty() {
        anyhow::bail!("ts.indicator_observations has no rows for entity us");
    }
    let scoring = ScoringEngine::default();
    let output = scoring.score(
        &indicators,
        &observations,
        as_of_date,
        "us",
        "financial_system",
    );
    Ok(AppData {
        alerts: build_alerts(&output.snapshot),
        backtests: build_backtests(&output.snapshot),
        sources: sources(),
        overview: output.snapshot,
        indicators: output.indicator_risks,
    })
}

async fn load_sqlite_app_data() -> anyhow::Result<AppData> {
    let sqlite_path =
        env::var("FC_SQLITE_PATH").unwrap_or_else(|_| "data/fc-local.sqlite".to_string());
    let as_of_date = Utc::now().date_naive();
    let store = SqliteStore::connect(sqlite_path).await?;
    store.migrate().await?;
    let indicators = store.load_indicators().await?;
    if indicators.is_empty() {
        anyhow::bail!("metadata_indicators is empty; run `just db-seed` first");
    }
    let observations = store.load_observations("us", as_of_date).await?;
    if observations.is_empty() {
        anyhow::bail!(
            "ts_indicator_observations has no rows for entity us; run at least one backfill such as `just backfill-fred`, `just backfill-treasury-yield`, or `just backfill-world-bank` first"
        );
    }
    let scoring = ScoringEngine::default();
    let output = scoring.score(
        &indicators,
        &observations,
        as_of_date,
        "us",
        "financial_system",
    );
    Ok(AppData {
        alerts: build_alerts(&output.snapshot),
        backtests: build_backtests(&output.snapshot),
        sources: sources(),
        overview: output.snapshot,
        indicators: output.indicator_risks,
    })
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
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let days_back = (values.len() - index - 1) as i64 * 30;
            let date = as_of_date - Duration::days(days_back);
            Observation {
                indicator_id: indicator_id.to_string(),
                entity_id: "us".to_string(),
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

fn sources() -> Vec<DataSource> {
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
            SourceStatus::Healthy,
            94.0,
            true,
            "Official SEC JSON APIs. Respect fair access and User-Agent requirements.",
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
            "gdelt",
            "GDELT",
            "news_events",
            SourcePriority::P1,
            SourceStatus::Delayed,
            78.0,
            true,
            "News-event prototype source. Requires noise filtering.",
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
        trigger_reason: "GDELT 新闻数据延迟，事件维度质量降级。".to_string(),
        top_contributors: Vec::new(),
        related_indicators: vec!["global_news_financial_stress_count".to_string()],
        method_version: snapshot.method_version.clone(),
    };

    vec![credit_alert, source_alert]
}

fn build_backtests(snapshot: &fc_domain::RiskSnapshot) -> Vec<BacktestScenarioSummary> {
    vec![
        BacktestScenarioSummary {
            scenario_id: "us_gfc_2008".to_string(),
            name: "2007-2009 全球金融危机".to_string(),
            region: "US".to_string(),
            crisis_start: NaiveDate::from_ymd_opt(2007, 8, 1).expect("valid date"),
            crisis_end: NaiveDate::from_ymd_opt(2009, 3, 31).expect("valid date"),
            first_l2_date: Some(NaiveDate::from_ymd_opt(2007, 6, 15).expect("valid date")),
            first_l3_date: Some(NaiveDate::from_ymd_opt(2007, 8, 9).expect("valid date")),
            max_level: RiskLevel::Crisis,
            max_score: 92.0,
            lead_time_days: Some(47),
            false_positive_count: 2,
            missed: false,
            top_contributors: snapshot.top_contributors.iter().take(3).cloned().collect(),
            method_version: snapshot.method_version.clone(),
        },
        BacktestScenarioSummary {
            scenario_id: "us_covid_liquidity_2020".to_string(),
            name: "2020 疫情流动性冲击".to_string(),
            region: "US".to_string(),
            crisis_start: NaiveDate::from_ymd_opt(2020, 2, 24).expect("valid date"),
            crisis_end: NaiveDate::from_ymd_opt(2020, 4, 30).expect("valid date"),
            first_l2_date: Some(NaiveDate::from_ymd_opt(2020, 2, 25).expect("valid date")),
            first_l3_date: Some(NaiveDate::from_ymd_opt(2020, 3, 9).expect("valid date")),
            max_level: RiskLevel::Crisis,
            max_score: 88.0,
            lead_time_days: Some(13),
            false_positive_count: 1,
            missed: false,
            top_contributors: snapshot.top_contributors.iter().take(3).cloned().collect(),
            method_version: snapshot.method_version.clone(),
        },
        BacktestScenarioSummary {
            scenario_id: "us_regional_banks_2023".to_string(),
            name: "2023 美国区域银行危机".to_string(),
            region: "US".to_string(),
            crisis_start: NaiveDate::from_ymd_opt(2023, 3, 8).expect("valid date"),
            crisis_end: NaiveDate::from_ymd_opt(2023, 5, 1).expect("valid date"),
            first_l2_date: Some(NaiveDate::from_ymd_opt(2023, 2, 15).expect("valid date")),
            first_l3_date: Some(NaiveDate::from_ymd_opt(2023, 3, 10).expect("valid date")),
            max_level: RiskLevel::Warning,
            max_score: 78.0,
            lead_time_days: Some(21),
            false_positive_count: 1,
            missed: false,
            top_contributors: snapshot.top_contributors.iter().take(3).cloned().collect(),
            method_version: snapshot.method_version.clone(),
        },
    ]
}

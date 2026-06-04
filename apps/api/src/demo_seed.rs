use chrono::{Duration, NaiveDate, Utc};
use fc_domain::{
    AlertEvent, AlertStatus, AlertType, DataSource, Frequency, Indicator, Observation,
    RiskDimension, RiskDirection, RiskLevel, SourceHealth, SourcePriority, SourceStatus,
};
use uuid::Uuid;

const EVENT_LOOKBACK_DAYS: i64 = 30;

pub(crate) fn indicators() -> Vec<Indicator> {
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

pub(crate) fn observations(as_of_date: NaiveDate) -> Vec<Observation> {
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

pub(crate) fn select_recent_alerts_for_date(
    alerts: &[AlertEvent],
    as_of_date: NaiveDate,
) -> Vec<AlertEvent> {
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

pub(crate) fn sources_demo() -> Vec<DataSource> {
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

pub(crate) fn sources_runtime(
    observations: &[Observation],
    as_of_date: NaiveDate,
) -> Vec<DataSource> {
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
            "Development-only market data prototype; not a production dependency。",
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

pub(crate) fn build_alerts(snapshot: &fc_domain::RiskSnapshot) -> Vec<AlertEvent> {
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

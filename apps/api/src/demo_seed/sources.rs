use chrono::{NaiveDate, Utc};
use fc_domain::{DataSource, Observation, SourceHealth, SourcePriority, SourceStatus};

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

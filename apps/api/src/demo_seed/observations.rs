use chrono::{Duration, NaiveDate, Utc};
use fc_domain::{Frequency, Observation};

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

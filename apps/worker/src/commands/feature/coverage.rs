use std::collections::BTreeMap;

use fc_domain::{IndicatorRisk, RiskDimension};

pub(super) fn coverage_summary(indicator_risks: &[IndicatorRisk]) -> (f64, f64, f64, f64) {
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

pub(super) fn find_dimension_score(
    indicator_risks: &[IndicatorRisk],
    dimension: RiskDimension,
) -> f64 {
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

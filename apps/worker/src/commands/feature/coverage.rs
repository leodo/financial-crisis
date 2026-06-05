use std::collections::BTreeMap;

use chrono::NaiveDate;
use fc_domain::{IndicatorRisk, RiskDimension};

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

pub(super) fn coverage_summary(
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

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use fc_domain::{
        Frequency, Indicator, IndicatorRisk, Observation, QualityGrade, RiskDimension,
        RiskDirection, RiskLevel,
    };

    use super::coverage_summary;

    fn indicator_risk(
        indicator_id: &str,
        dimension: RiskDimension,
        as_of_date: NaiveDate,
        present: bool,
    ) -> IndicatorRisk {
        IndicatorRisk {
            indicator: Indicator {
                indicator_id: indicator_id.to_string(),
                display_name: indicator_id.to_string(),
                dimension,
                description: "test".to_string(),
                unit: "value".to_string(),
                frequency: Frequency::Daily,
                risk_direction: RiskDirection::HigherIsRiskier,
                default_source_id: "test".to_string(),
                quality_tier: "best_effort".to_string(),
            },
            latest_observation: present.then(|| Observation {
                indicator_id: indicator_id.to_string(),
                entity_id: if indicator_id == "jp_rates_call_rate" {
                    "jp".to_string()
                } else {
                    "us".to_string()
                },
                as_of_date,
                period_start: Some(as_of_date),
                period_end: Some(as_of_date),
                frequency: Frequency::Daily,
                value: 1.0,
                unit: "value".to_string(),
                source_id: "test".to_string(),
                dataset_id: "test".to_string(),
                revision_time: None,
                publication_time: None,
                quality_score: 100.0,
                quality_flags: Vec::new(),
            }),
            score: 50.0,
            level: RiskLevel::Normal,
            percentile: Some(0.5),
            change_30d: None,
            score_basis: "test".to_string(),
            score_input_value: Some(1.0),
            score_input_unit: Some("value".to_string()),
            quality_grade: QualityGrade::A,
            contribution: 0.0,
        }
    }

    #[test]
    fn pre_stlfsi_main_coverage_ignores_stlfsi_and_jpy_call_rate() {
        let as_of_date = NaiveDate::from_ymd_opt(1993, 1, 5).unwrap();
        let risks = vec![
            indicator_risk(
                "us_market_vix_close",
                RiskDimension::MarketStress,
                as_of_date,
                true,
            ),
            indicator_risk(
                "us_rates_yield_curve_10y2y",
                RiskDimension::LeverageCredit,
                as_of_date,
                true,
            ),
            indicator_risk(
                "us_credit_baa_10y_spread",
                RiskDimension::LeverageCredit,
                as_of_date,
                true,
            ),
            indicator_risk(
                "us_liquidity_effr",
                RiskDimension::LiquidityFunding,
                as_of_date,
                true,
            ),
            indicator_risk(
                "us_liquidity_national_financial_conditions",
                RiskDimension::LiquidityFunding,
                as_of_date,
                true,
            ),
            indicator_risk(
                "us_liquidity_financial_stress_stl",
                RiskDimension::LiquidityFunding,
                as_of_date,
                false,
            ),
            indicator_risk(
                "us_macro_unemployment_rate",
                RiskDimension::MacroFragility,
                as_of_date,
                true,
            ),
            indicator_risk(
                "us_real_estate_housing_starts",
                RiskDimension::RealEstate,
                as_of_date,
                true,
            ),
            indicator_risk(
                "us_external_usdjpy_level",
                RiskDimension::ExternalSector,
                as_of_date,
                true,
            ),
            indicator_risk(
                "jp_rates_call_rate",
                RiskDimension::ExternalSector,
                as_of_date,
                false,
            ),
        ];

        assert_eq!(coverage_summary(&risks, as_of_date), (1.0, 1.0, 1.0, 1.0));
    }

    #[test]
    fn post_stlfsi_main_coverage_requires_stlfsi_but_not_jpy_call_rate() {
        let as_of_date = NaiveDate::from_ymd_opt(1998, 1, 5).unwrap();
        let risks = vec![
            indicator_risk(
                "us_market_vix_close",
                RiskDimension::MarketStress,
                as_of_date,
                true,
            ),
            indicator_risk(
                "us_rates_yield_curve_10y2y",
                RiskDimension::LeverageCredit,
                as_of_date,
                true,
            ),
            indicator_risk(
                "us_credit_baa_10y_spread",
                RiskDimension::LeverageCredit,
                as_of_date,
                true,
            ),
            indicator_risk(
                "us_liquidity_effr",
                RiskDimension::LiquidityFunding,
                as_of_date,
                true,
            ),
            indicator_risk(
                "us_liquidity_national_financial_conditions",
                RiskDimension::LiquidityFunding,
                as_of_date,
                true,
            ),
            indicator_risk(
                "us_liquidity_financial_stress_stl",
                RiskDimension::LiquidityFunding,
                as_of_date,
                false,
            ),
            indicator_risk(
                "us_macro_unemployment_rate",
                RiskDimension::MacroFragility,
                as_of_date,
                true,
            ),
            indicator_risk(
                "us_real_estate_housing_starts",
                RiskDimension::RealEstate,
                as_of_date,
                true,
            ),
            indicator_risk(
                "us_external_usdjpy_level",
                RiskDimension::ExternalSector,
                as_of_date,
                true,
            ),
            indicator_risk(
                "jp_rates_call_rate",
                RiskDimension::ExternalSector,
                as_of_date,
                false,
            ),
        ];

        assert_eq!(
            coverage_summary(&risks, as_of_date),
            (0.875, 0.833, 1.0, 0.885)
        );
    }
}

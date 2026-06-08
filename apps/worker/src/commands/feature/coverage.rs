use std::collections::BTreeMap;

use chrono::NaiveDate;
use fc_domain::{
    formal_feature_coverage_summary, formal_feature_dimension_score, formal_feature_quality_grade,
    formal_has_extension_acute_core_features, formal_has_main_dataset_core_features, IndicatorRisk,
    RiskDimension,
};

pub(super) fn coverage_summary(
    indicator_risks: &[IndicatorRisk],
    as_of_date: NaiveDate,
) -> (f64, f64, f64, f64) {
    let summary = formal_feature_coverage_summary(indicator_risks, as_of_date);
    (
        summary.core_feature_coverage,
        summary.trigger_feature_coverage,
        summary.external_feature_coverage,
        summary.coverage_score,
    )
}

pub(super) fn find_dimension_score(
    indicator_risks: &[IndicatorRisk],
    dimension: RiskDimension,
) -> f64 {
    formal_feature_dimension_score(indicator_risks, dimension)
}

pub(crate) fn has_main_dataset_core_features(features: &BTreeMap<String, f64>) -> bool {
    formal_has_main_dataset_core_features(features)
}

pub(crate) fn has_extension_acute_core_features(features: &BTreeMap<String, f64>) -> bool {
    formal_has_extension_acute_core_features(features)
}

pub(crate) fn feature_quality_grade(coverage_score: f64) -> &'static str {
    formal_feature_quality_grade(coverage_score)
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

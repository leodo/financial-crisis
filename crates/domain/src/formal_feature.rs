use chrono::NaiveDate;

use crate::observation_window::{
    observation_history_for_indicator, observation_value_difference_from_tail,
};
use crate::probability_bundle::{
    FEATURE_US_BAA_10Y_SPREAD_LEVEL, FEATURE_US_CURVE_10Y2Y_LEVEL, FEATURE_US_FED_FUNDS_LEVEL,
    FEATURE_US_HOUSING_STARTS_LEVEL, FEATURE_US_NFCI_LEVEL, FEATURE_US_STLFSI_LEVEL,
    FEATURE_US_UNEMPLOYMENT_LEVEL, FEATURE_US_USDJPY_CHANGE_20D, FEATURE_US_USDJPY_LEVEL,
    FEATURE_US_VIX_CHANGE_5D, FEATURE_US_VIX_LEVEL,
};
use crate::Observation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormalObservationFeatureTransform {
    Latest,
    DifferenceFromTail { lookback: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormalObservationFeatureSpec {
    pub feature_name: &'static str,
    pub indicator_id: &'static str,
    pub transform: FormalObservationFeatureTransform,
}

pub const FORMAL_OBSERVATION_FEATURE_SPECS: &[FormalObservationFeatureSpec] = &[
    FormalObservationFeatureSpec {
        feature_name: FEATURE_US_VIX_LEVEL,
        indicator_id: "us_market_vix_close",
        transform: FormalObservationFeatureTransform::Latest,
    },
    FormalObservationFeatureSpec {
        feature_name: FEATURE_US_VIX_CHANGE_5D,
        indicator_id: "us_market_vix_close",
        transform: FormalObservationFeatureTransform::DifferenceFromTail { lookback: 5 },
    },
    FormalObservationFeatureSpec {
        feature_name: FEATURE_US_CURVE_10Y2Y_LEVEL,
        indicator_id: "us_rates_yield_curve_10y2y",
        transform: FormalObservationFeatureTransform::Latest,
    },
    FormalObservationFeatureSpec {
        feature_name: FEATURE_US_BAA_10Y_SPREAD_LEVEL,
        indicator_id: "us_credit_baa_10y_spread",
        transform: FormalObservationFeatureTransform::Latest,
    },
    FormalObservationFeatureSpec {
        feature_name: FEATURE_US_FED_FUNDS_LEVEL,
        indicator_id: "us_liquidity_effr",
        transform: FormalObservationFeatureTransform::Latest,
    },
    FormalObservationFeatureSpec {
        feature_name: FEATURE_US_NFCI_LEVEL,
        indicator_id: "us_liquidity_national_financial_conditions",
        transform: FormalObservationFeatureTransform::Latest,
    },
    FormalObservationFeatureSpec {
        feature_name: FEATURE_US_STLFSI_LEVEL,
        indicator_id: "us_liquidity_financial_stress_stl",
        transform: FormalObservationFeatureTransform::Latest,
    },
    FormalObservationFeatureSpec {
        feature_name: FEATURE_US_UNEMPLOYMENT_LEVEL,
        indicator_id: "us_macro_unemployment_rate",
        transform: FormalObservationFeatureTransform::Latest,
    },
    FormalObservationFeatureSpec {
        feature_name: FEATURE_US_HOUSING_STARTS_LEVEL,
        indicator_id: "us_real_estate_housing_starts",
        transform: FormalObservationFeatureTransform::Latest,
    },
    FormalObservationFeatureSpec {
        feature_name: FEATURE_US_USDJPY_LEVEL,
        indicator_id: "us_external_usdjpy_level",
        transform: FormalObservationFeatureTransform::Latest,
    },
    FormalObservationFeatureSpec {
        feature_name: FEATURE_US_USDJPY_CHANGE_20D,
        indicator_id: "us_external_usdjpy_level",
        transform: FormalObservationFeatureTransform::DifferenceFromTail { lookback: 20 },
    },
];

pub fn formal_observation_feature_value(
    observations: &[Observation],
    spec: &FormalObservationFeatureSpec,
    as_of_date: NaiveDate,
) -> Option<f64> {
    let history = observation_history_for_indicator(observations, spec.indicator_id, as_of_date);
    formal_observation_feature_value_from_history(&history, spec.transform)
}

pub fn formal_observation_feature_value_from_history(
    history: &[&Observation],
    transform: FormalObservationFeatureTransform,
) -> Option<f64> {
    match transform {
        FormalObservationFeatureTransform::Latest => {
            history.last().map(|observation| observation.value)
        }
        FormalObservationFeatureTransform::DifferenceFromTail { lookback } => {
            observation_value_difference_from_tail(history, lookback)
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use crate::{Frequency, Observation};

    use super::{
        formal_observation_feature_value, formal_observation_feature_value_from_history,
        FormalObservationFeatureSpec, FormalObservationFeatureTransform, FEATURE_US_VIX_CHANGE_5D,
        FEATURE_US_VIX_LEVEL, FORMAL_OBSERVATION_FEATURE_SPECS,
    };

    fn observation(indicator_id: &str, day: u32, value: f64) -> Observation {
        Observation {
            indicator_id: indicator_id.to_string(),
            entity_id: "us".to_string(),
            as_of_date: NaiveDate::from_ymd_opt(2026, 1, day).unwrap(),
            period_start: None,
            period_end: None,
            frequency: Frequency::Daily,
            value,
            unit: "index".to_string(),
            source_id: "test".to_string(),
            dataset_id: "test".to_string(),
            revision_time: None,
            publication_time: None,
            quality_score: 1.0,
            quality_flags: Vec::new(),
        }
    }

    #[test]
    fn formal_observation_registry_contains_level_and_tail_specs() {
        assert!(FORMAL_OBSERVATION_FEATURE_SPECS.iter().any(|spec| {
            spec.feature_name == FEATURE_US_VIX_LEVEL
                && spec.indicator_id == "us_market_vix_close"
                && spec.transform == FormalObservationFeatureTransform::Latest
        }));
        assert!(FORMAL_OBSERVATION_FEATURE_SPECS.iter().any(|spec| {
            spec.feature_name == FEATURE_US_VIX_CHANGE_5D
                && spec.indicator_id == "us_market_vix_close"
                && spec.transform
                    == FormalObservationFeatureTransform::DifferenceFromTail { lookback: 5 }
        }));
    }

    #[test]
    fn formal_observation_feature_value_resolves_latest_and_tail_change() {
        let observations = vec![
            observation("vix", 1, 10.0),
            observation("vix", 2, 14.0),
            observation("vix", 3, 18.0),
        ];
        let latest = FormalObservationFeatureSpec {
            feature_name: "vix_level",
            indicator_id: "vix",
            transform: FormalObservationFeatureTransform::Latest,
        };
        let change = FormalObservationFeatureSpec {
            feature_name: "vix_change_2d",
            indicator_id: "vix",
            transform: FormalObservationFeatureTransform::DifferenceFromTail { lookback: 2 },
        };

        assert_eq!(
            formal_observation_feature_value(
                &observations,
                &latest,
                NaiveDate::from_ymd_opt(2026, 1, 3).unwrap()
            ),
            Some(18.0)
        );
        assert_eq!(
            formal_observation_feature_value(
                &observations,
                &change,
                NaiveDate::from_ymd_opt(2026, 1, 3).unwrap()
            ),
            Some(8.0)
        );
    }

    #[test]
    fn formal_observation_feature_value_from_history_requires_full_lookback() {
        let observations = vec![observation("vix", 1, 10.0), observation("vix", 2, 12.0)];
        let history = observations.iter().collect::<Vec<_>>();

        assert_eq!(
            formal_observation_feature_value_from_history(
                &history,
                FormalObservationFeatureTransform::DifferenceFromTail { lookback: 2 }
            ),
            None
        );
    }
}

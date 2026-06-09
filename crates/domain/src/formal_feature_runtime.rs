use std::collections::BTreeMap;

use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc, Weekday};

use crate::{
    formal_observation_feature_value_from_history, observation_history_for_indicator_where,
    FeatureSnapshotRecord, FormalObservationFeatureTransform, Frequency, IndicatorRisk,
    Observation, RiskDimension, FORMAL_OBSERVATION_FEATURE_SPECS,
};

pub const FEATURE_SNAPSHOT_STATUS_READY: &str = "ready";
pub const FEATURE_SNAPSHOT_STATUS_COVERAGE_OR_VISIBILITY_FAILED: &str =
    "coverage_or_visibility_failed";

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FormalCoverageSummary {
    pub core_feature_coverage: f64,
    pub trigger_feature_coverage: f64,
    pub external_feature_coverage: f64,
    pub coverage_score: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FormalObservationFeatureMap {
    pub features: BTreeMap<String, f64>,
    pub latest_visible_at: Option<DateTime<Utc>>,
}

pub fn build_formal_observation_feature_map(
    observations: &[Observation],
    as_of_date: NaiveDate,
    point_in_time_mode: &str,
) -> FormalObservationFeatureMap {
    let mut features = BTreeMap::new();
    let mut latest_visible_at: Option<DateTime<Utc>> = None;

    for spec in FORMAL_OBSERVATION_FEATURE_SPECS {
        let history = observation_history_for_indicator_where(
            observations,
            spec.indicator_id,
            as_of_date,
            |observation| {
                observation_is_visible_for_date_for_point_in_time_mode(
                    observation,
                    as_of_date,
                    point_in_time_mode,
                )
            },
        );
        if let Some(value) = formal_observation_feature_value_from_history(&history, spec.transform)
        {
            features.insert(spec.feature_name.to_string(), round6(value));
        }
        if matches!(spec.transform, FormalObservationFeatureTransform::Latest) {
            if let Some(latest) = history.last() {
                if let Some(visible_at) =
                    observation_visible_at_for_point_in_time_mode(latest, point_in_time_mode)
                {
                    latest_visible_at = Some(match latest_visible_at {
                        Some(current) => current.max(visible_at),
                        None => visible_at,
                    });
                }
            }
        }
    }

    FormalObservationFeatureMap {
        features,
        latest_visible_at,
    }
}

pub fn formal_feature_coverage_summary(
    indicator_risks: &[IndicatorRisk],
    as_of_date: NaiveDate,
) -> FormalCoverageSummary {
    let (core_total, core_present) =
        coverage_by_indicator_ids(indicator_risks, formal_core_indicator_ids(as_of_date));
    let (trigger_total, trigger_present) =
        coverage_by_indicator_ids(indicator_risks, formal_trigger_indicator_ids(as_of_date));
    let (external_total, external_present) =
        coverage_by_indicator_ids(indicator_risks, FORMAL_EXTERNAL_INDICATORS);

    let core_feature_coverage = ratio(core_present, core_total);
    let trigger_feature_coverage = ratio(trigger_present, trigger_total);
    let external_feature_coverage = ratio(external_present, external_total);
    let coverage_score = round3(
        (core_feature_coverage * 0.45
            + trigger_feature_coverage * 0.35
            + external_feature_coverage * 0.2)
            .clamp(0.0, 1.0),
    );

    FormalCoverageSummary {
        core_feature_coverage: round3(core_feature_coverage),
        trigger_feature_coverage: round3(trigger_feature_coverage),
        external_feature_coverage: round3(external_feature_coverage),
        coverage_score,
    }
}

pub fn formal_feature_dimension_score(
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

pub fn formal_has_main_dataset_core_features(features: &BTreeMap<String, f64>) -> bool {
    [
        "us_vix_level",
        "us_curve_10y2y_level",
        "us_baa_10y_spread_level",
        "us_fed_funds_level",
    ]
    .into_iter()
    .all(|feature| features.contains_key(feature))
}

pub fn formal_has_extension_acute_core_features(features: &BTreeMap<String, f64>) -> bool {
    [
        "us_curve_10y2y_level",
        "us_baa_10y_spread_level",
        "us_fed_funds_level",
        "us_usdjpy_level",
    ]
    .into_iter()
    .all(|feature| features.contains_key(feature))
}

pub fn formal_feature_quality_grade(coverage_score: f64) -> &'static str {
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

pub fn formal_feature_snapshot_visibility_status(
    features: &BTreeMap<String, f64>,
    coverage_score: f64,
    latest_visible_at: Option<DateTime<Utc>>,
) -> &'static str {
    if latest_visible_at.is_none()
        || coverage_score < 0.70
        || !formal_has_main_dataset_core_features(features)
    {
        FEATURE_SNAPSHOT_STATUS_COVERAGE_OR_VISIBILITY_FAILED
    } else {
        FEATURE_SNAPSHOT_STATUS_READY
    }
}

#[allow(clippy::too_many_arguments)]
pub fn build_formal_feature_snapshot_record(
    as_of_date: NaiveDate,
    entity_id: &str,
    market_scope: &str,
    feature_set_version: &str,
    point_in_time_mode: &str,
    indicator_risks: &[IndicatorRisk],
    features: BTreeMap<String, f64>,
    latest_visible_at: Option<DateTime<Utc>>,
    overall_score: f64,
    structural_score: f64,
    trigger_score: f64,
    created_at: DateTime<Utc>,
) -> FeatureSnapshotRecord {
    let mut features = features;
    features.insert(
        "overall_score".to_string(),
        round6((overall_score / 100.0).clamp(0.0, 1.0)),
    );
    features.insert(
        "structural_score".to_string(),
        round6((structural_score / 100.0).clamp(0.0, 1.0)),
    );
    features.insert(
        "trigger_score".to_string(),
        round6((trigger_score / 100.0).clamp(0.0, 1.0)),
    );
    features.insert(
        "external_dimension_score".to_string(),
        round6(
            (formal_feature_dimension_score(indicator_risks, RiskDimension::ExternalSector)
                / 100.0)
                .clamp(0.0, 1.0),
        ),
    );

    let coverage = formal_feature_coverage_summary(indicator_risks, as_of_date);
    let visibility_status = formal_feature_snapshot_visibility_status(
        &features,
        coverage.coverage_score,
        latest_visible_at,
    );

    FeatureSnapshotRecord {
        as_of_date,
        entity_id: entity_id.to_string(),
        market_scope: market_scope.to_string(),
        feature_set_version: feature_set_version.to_string(),
        point_in_time_mode: point_in_time_mode.to_string(),
        visibility_status: visibility_status.to_string(),
        latest_visible_at,
        coverage_score: coverage.coverage_score,
        core_feature_coverage: coverage.core_feature_coverage,
        trigger_feature_coverage: coverage.trigger_feature_coverage,
        external_feature_coverage: coverage.external_feature_coverage,
        feature_count: features.len(),
        features,
        created_at,
    }
}

pub fn observation_is_visible_for_date_for_point_in_time_mode(
    observation: &Observation,
    as_of_date: NaiveDate,
    point_in_time_mode: &str,
) -> bool {
    observation_visible_at_for_point_in_time_mode(observation, point_in_time_mode)
        .map(|visible_at| visible_at <= assessment_cutoff_utc(as_of_date))
        .unwrap_or(false)
}

pub fn observation_visible_at_for_point_in_time_mode(
    observation: &Observation,
    point_in_time_mode: &str,
) -> Option<DateTime<Utc>> {
    match point_in_time_mode {
        "best_effort" => best_effort_visible_at(observation),
        "strict" => strict_visible_at(observation),
        _ => None,
    }
}

pub fn assessment_cutoff_utc(as_of_date: NaiveDate) -> DateTime<Utc> {
    new_york_time_to_utc(as_of_date, 17, 30)
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

fn round3(value: f64) -> f64 {
    (value * 1_000.0).round() / 1_000.0
}

fn round6(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

fn best_effort_visible_at(observation: &Observation) -> Option<DateTime<Utc>> {
    let anchor_date = observation.period_end.unwrap_or(observation.as_of_date);
    match observation.source_id.as_str() {
        "treasury" => Some(new_york_time_to_utc(anchor_date, 18, 0)),
        "world_bank" => anchor_date
            .checked_add_signed(Duration::days(270))
            .map(|date| new_york_time_to_utc(date, 17, 30)),
        "boj" => Some(tokyo_time_to_utc(anchor_date, 17, 0)),
        "sec_edgar" => Some(
            observation
                .publication_time
                .unwrap_or_else(|| new_york_time_to_utc(anchor_date, 18, 0)),
        ),
        "gdelt" => None,
        "mock" => Some(
            observation
                .publication_time
                .unwrap_or_else(|| new_york_time_to_utc(anchor_date, 17, 30)),
        ),
        _ => anchor_date
            .checked_add_signed(Duration::days(default_visibility_lag_days(
                observation.frequency,
            )))
            .map(|date| new_york_time_to_utc(date, 17, 30)),
    }
}

fn strict_visible_at(observation: &Observation) -> Option<DateTime<Utc>> {
    match observation.source_id.as_str() {
        "sec_edgar" | "mock" => observation.publication_time,
        _ => None,
    }
}

fn default_visibility_lag_days(frequency: Frequency) -> i64 {
    match frequency {
        Frequency::Daily | Frequency::Event => 0,
        Frequency::Weekly => 3,
        Frequency::Monthly => 15,
        Frequency::Quarterly => 45,
        Frequency::Annual => 270,
    }
}

fn new_york_time_to_utc(date: NaiveDate, hour: u32, minute: u32) -> DateTime<Utc> {
    let utc_offset_hours = if is_new_york_dst(date) { 4 } else { 5 };
    let local = date
        .and_hms_opt(hour, minute, 0)
        .expect("local wall-clock timestamp must be valid");
    DateTime::<Utc>::from_naive_utc_and_offset(local + Duration::hours(utc_offset_hours), Utc)
}

fn tokyo_time_to_utc(date: NaiveDate, hour: u32, minute: u32) -> DateTime<Utc> {
    let local = date
        .and_hms_opt(hour, minute, 0)
        .expect("tokyo wall-clock timestamp must be valid");
    DateTime::<Utc>::from_naive_utc_and_offset(local - Duration::hours(9), Utc)
}

fn is_new_york_dst(date: NaiveDate) -> bool {
    let year = date.year();
    let (start, end) = if year >= 2007 {
        (
            nth_weekday_of_month(year, 3, Weekday::Sun, 2),
            nth_weekday_of_month(year, 11, Weekday::Sun, 1),
        )
    } else {
        (
            nth_weekday_of_month(year, 4, Weekday::Sun, 1),
            last_weekday_of_month(year, 10, Weekday::Sun),
        )
    };
    date >= start && date < end
}

fn nth_weekday_of_month(year: i32, month: u32, weekday: Weekday, nth: u32) -> NaiveDate {
    let first_day = NaiveDate::from_ymd_opt(year, month, 1).expect("valid calendar date");
    let first_weekday_offset = (7 + weekday.num_days_from_monday() as i64
        - first_day.weekday().num_days_from_monday() as i64)
        % 7;
    first_day
        .checked_add_signed(Duration::days(
            first_weekday_offset + 7 * i64::from(nth - 1),
        ))
        .expect("nth weekday must be representable")
}

fn last_weekday_of_month(year: i32, month: u32, weekday: Weekday) -> NaiveDate {
    let next_month = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).expect("valid calendar date")
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).expect("valid calendar date")
    };
    let last_day = next_month
        .checked_sub_signed(Duration::days(1))
        .expect("previous day must be valid");
    let backward_offset = (7 + last_day.weekday().num_days_from_monday() as i64
        - weekday.num_days_from_monday() as i64)
        % 7;
    last_day
        .checked_sub_signed(Duration::days(backward_offset))
        .expect("last weekday must be representable")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use chrono::{NaiveDate, Utc};

    use super::{
        build_formal_feature_snapshot_record, FEATURE_SNAPSHOT_STATUS_COVERAGE_OR_VISIBILITY_FAILED,
    };
    use crate::{
        Frequency, Indicator, IndicatorRisk, Observation, QualityGrade, RiskDimension,
        RiskDirection, RiskLevel,
    };

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
                entity_id: "us".to_string(),
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
    fn build_snapshot_record_adds_runtime_scores_and_coverage() {
        let as_of_date = NaiveDate::from_ymd_opt(1993, 1, 5).unwrap();
        let indicator_risks = vec![
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
        ];

        let latest_visible_at = Some(Utc::now());
        let record = build_formal_feature_snapshot_record(
            as_of_date,
            "us",
            "financial_system",
            "formal_v1",
            "best_effort",
            &indicator_risks,
            BTreeMap::from([("us_vix_level".to_string(), 0.12)]),
            latest_visible_at,
            82.0,
            71.0,
            65.0,
            Utc::now(),
        );

        assert_eq!(record.coverage_score, 1.0);
        assert_eq!(record.core_feature_coverage, 1.0);
        assert_eq!(record.trigger_feature_coverage, 1.0);
        assert_eq!(record.external_feature_coverage, 1.0);
        assert_eq!(record.features.get("overall_score"), Some(&0.82));
        assert_eq!(record.features.get("structural_score"), Some(&0.71));
        assert_eq!(record.features.get("trigger_score"), Some(&0.65));
        assert_eq!(record.features.get("external_dimension_score"), Some(&0.5));
        assert_eq!(record.feature_count, 5);
        assert_eq!(
            record.visibility_status,
            FEATURE_SNAPSHOT_STATUS_COVERAGE_OR_VISIBILITY_FAILED
        );
        assert_eq!(record.latest_visible_at, latest_visible_at);
    }
}

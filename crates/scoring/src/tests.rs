use chrono::{NaiveDate, Utc};
use fc_domain::{
    Frequency, Indicator, IndicatorRisk, Observation, QualityGrade, RiskDimension, RiskDirection,
    RiskLevel,
};

use crate::{compute_signal, explain_indicator, score_value};

#[test]
fn higher_is_riskier_uses_percentile() {
    let (score, percentile) =
        score_value(&[1.0, 2.0, 3.0, 4.0], 4.0, RiskDirection::HigherIsRiskier);
    assert_eq!(score, 100.0);
    assert_eq!(percentile, Some(100.0));
}

#[test]
fn lower_is_riskier_inverts_percentile() {
    let (score, percentile) =
        score_value(&[1.0, 2.0, 3.0, 4.0], 1.0, RiskDirection::LowerIsRiskier);
    assert_eq!(score, 75.0);
    assert_eq!(percentile, Some(25.0));
}

#[test]
fn home_price_uses_yoy_signal_not_raw_level() {
    let indicator = Indicator {
        indicator_id: "us_real_estate_home_price".to_string(),
        display_name: "Case-Shiller 房价指数".to_string(),
        dimension: RiskDimension::RealEstate,
        description: String::new(),
        unit: "index".to_string(),
        frequency: Frequency::Monthly,
        risk_direction: RiskDirection::TwoSided,
        default_source_id: "fred".to_string(),
        quality_tier: "core".to_string(),
    };
    let history = vec![
        observation("us_real_estate_home_price", 2024, 1, 1, 200.0),
        observation("us_real_estate_home_price", 2025, 1, 1, 210.0),
        observation("us_real_estate_home_price", 2026, 1, 1, 220.5),
    ];
    let refs = history.iter().collect::<Vec<_>>();
    let signal = compute_signal(&indicator, &refs, history.last());
    assert_eq!(signal.score_basis, "12m同比");
    assert_eq!(signal.score_input_unit.as_deref(), Some("%"));
    assert!(signal.score_input_value.is_some());
    assert!(signal.score_input_value.unwrap() < 10.0);
}

#[test]
fn indicator_explanation_names_two_sided_tail() {
    let risk = IndicatorRisk {
        indicator: Indicator {
            indicator_id: "us_real_estate_home_price".to_string(),
            display_name: "Case-Shiller 房价指数".to_string(),
            dimension: RiskDimension::RealEstate,
            description: String::new(),
            unit: "index".to_string(),
            frequency: Frequency::Monthly,
            risk_direction: RiskDirection::TwoSided,
            default_source_id: "fred".to_string(),
            quality_tier: "core".to_string(),
        },
        latest_observation: None,
        score: 80.6,
        level: RiskLevel::Warning,
        percentile: Some(19.4),
        change_30d: None,
        score_basis: "12m同比".to_string(),
        score_input_value: Some(0.66),
        score_input_unit: Some("%".to_string()),
        quality_grade: QualityGrade::A,
        contribution: 40.3,
    };

    let explanation = explain_indicator(&risk);

    assert!(explanation.contains("低尾异常"));
    assert!(explanation.contains("历史分位 19.4"));
}

#[test]
fn rising_fast_series_scores_off_change_not_level() {
    let indicator = Indicator {
        indicator_id: "us_liquidity_effr".to_string(),
        display_name: "有效联邦基金利率".to_string(),
        dimension: RiskDimension::LiquidityFunding,
        description: String::new(),
        unit: "percent".to_string(),
        frequency: Frequency::Daily,
        risk_direction: RiskDirection::RisingFastIsRiskier,
        default_source_id: "fred".to_string(),
        quality_tier: "core".to_string(),
    };
    let history = vec![
        observation("us_liquidity_effr", 2026, 1, 1, 3.0),
        observation("us_liquidity_effr", 2026, 1, 31, 3.1),
        observation("us_liquidity_effr", 2026, 3, 2, 3.7),
    ];
    let refs = history.iter().collect::<Vec<_>>();
    let signal = compute_signal(&indicator, &refs, history.last());
    assert_eq!(signal.score_basis, "变化幅度");
    assert_eq!(signal.score_input_unit.as_deref(), Some("percent"));
    assert!(signal.score_input_value.unwrap() > 0.0);
}

fn observation(indicator_id: &str, year: i32, month: u32, day: u32, value: f64) -> Observation {
    Observation {
        indicator_id: indicator_id.to_string(),
        entity_id: "us".to_string(),
        as_of_date: NaiveDate::from_ymd_opt(year, month, day).unwrap(),
        period_start: None,
        period_end: None,
        frequency: Frequency::Daily,
        value,
        unit: "source_unit".to_string(),
        source_id: "fred".to_string(),
        dataset_id: "fred_series_observations".to_string(),
        revision_time: None,
        publication_time: Some(Utc::now()),
        quality_score: 92.0,
        quality_flags: Vec::new(),
    }
}

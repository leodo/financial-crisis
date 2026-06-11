use super::*;

fn snapshot_with_scores(structural_score: f64, trigger_score: f64) -> RiskSnapshot {
    RiskSnapshot {
        as_of_date: NaiveDate::from_ymd_opt(2026, 6, 9).unwrap(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        overall_score: 35.0,
        overall_level: RiskLevel::Watch,
        structural_score,
        trigger_score,
        level_reason: "test".to_string(),
        dimensions: Vec::new(),
        top_contributors: Vec::new(),
        data_quality_summary: DataQualitySummary {
            overall_score: 91.0,
            grade: QualityGrade::A,
            stale_indicator_count: 0,
            low_quality_indicator_count: 0,
            prototype_source_count: 0,
            blocked_indicator_count: 0,
        },
        generated_at: Utc::now(),
        method_version: "test".to_string(),
    }
}

#[test]
fn action_evidence_breakdown_does_not_mistake_data_quality_for_risk_evidence() {
    let snapshot = snapshot_with_scores(44.0, 37.0);
    let evidence =
        build_action_evidence_breakdown(&snapshot, &test_data_trust(QualityGrade::A), 32.0);

    assert_eq!(evidence.score, 0.178);
    assert_eq!(evidence.data_quality_component, 0.098);
    assert_eq!(evidence.breadth_component, 0.08);
    assert_eq!(evidence.risk_pressure_component, 0.0);
    assert_eq!(evidence.agreement_component, 0.0);
    assert!(!evidence.structural_trigger_agreement);
}

#[test]
fn action_evidence_rises_when_breadth_and_agreement_confirm() {
    let snapshot = snapshot_with_scores(61.0, 59.0);
    let evidence =
        build_action_evidence_breakdown(&snapshot, &test_data_trust(QualityGrade::A), 67.0);

    assert_eq!(evidence.score, 0.712);
    assert_eq!(evidence.breadth_component, 0.3);
    assert_eq!(evidence.risk_pressure_component, 0.194);
    assert_eq!(evidence.agreement_component, 0.12);
    assert!(evidence.structural_trigger_agreement);
}

#[test]
fn derived_driver_explanation_does_not_call_delta_the_current_level() {
    let risk = IndicatorRisk {
        indicator: Indicator {
            indicator_id: "us_liquidity_sofr".to_string(),
            display_name: "SOFR".to_string(),
            dimension: RiskDimension::LiquidityFunding,
            description: "Secured Overnight Financing Rate".to_string(),
            unit: "percent".to_string(),
            frequency: Frequency::Daily,
            risk_direction: RiskDirection::RisingFastIsRiskier,
            default_source_id: "fred".to_string(),
            quality_tier: "core".to_string(),
        },
        latest_observation: Some(Observation {
            indicator_id: "us_liquidity_sofr".to_string(),
            entity_id: "us".to_string(),
            as_of_date: NaiveDate::from_ymd_opt(2026, 6, 8).unwrap(),
            period_start: None,
            period_end: None,
            frequency: Frequency::Daily,
            value: 3.63,
            unit: "source_unit".to_string(),
            source_id: "fred".to_string(),
            dataset_id: "fred_series_observations".to_string(),
            revision_time: None,
            publication_time: None,
            quality_score: 92.0,
            quality_flags: Vec::new(),
        }),
        score: 71.7,
        level: RiskLevel::Warning,
        percentile: Some(71.7),
        change_30d: Some(0.03),
        score_basis: "变化幅度".to_string(),
        score_input_value: Some(0.03),
        score_input_unit: Some("percent".to_string()),
        quality_grade: QualityGrade::A,
        contribution: 14.3,
    };

    let explanation = build_synthetic_driver_explanation(&risk);

    assert!(
        explanation.contains("评分输入 +0.03 %（变化幅度，不是 SOFR 当前水平；最新水平 3.63 %）")
    );
    assert!(!explanation.contains("当前读数 0.03"));
}

#[test]
fn top_driver_enrichment_rewrites_derived_base_current_signal_copy() {
    let risk = IndicatorRisk {
        indicator: Indicator {
            indicator_id: "us_real_estate_home_price".to_string(),
            display_name: "Case-Shiller 房价指数".to_string(),
            dimension: RiskDimension::RealEstate,
            description: "Case-Shiller home price index".to_string(),
            unit: "index".to_string(),
            frequency: Frequency::Monthly,
            risk_direction: RiskDirection::FallingFastIsRiskier,
            default_source_id: "fred".to_string(),
            quality_tier: "core".to_string(),
        },
        latest_observation: Some(Observation {
            indicator_id: "us_real_estate_home_price".to_string(),
            entity_id: "us".to_string(),
            as_of_date: NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            period_start: None,
            period_end: None,
            frequency: Frequency::Monthly,
            value: 322.0,
            unit: "source_unit".to_string(),
            source_id: "fred".to_string(),
            dataset_id: "fred_series_observations".to_string(),
            revision_time: None,
            publication_time: None,
            quality_score: 94.0,
            quality_flags: Vec::new(),
        }),
        score: 80.6,
        level: RiskLevel::Warning,
        percentile: Some(19.4),
        change_30d: Some(0.66),
        score_basis: "12m同比".to_string(),
        score_input_value: Some(0.66),
        score_input_unit: Some("percent".to_string()),
        quality_grade: QualityGrade::A,
        contribution: 40.3,
    };
    let base_driver = RiskContributor {
        indicator_id: "us_real_estate_home_price".to_string(),
        display_name: "Case-Shiller 房价指数".to_string(),
        dimension: RiskDimension::RealEstate,
        score: 80.6,
        contribution: 40.3,
        explanation:
            "Case-Shiller 房价指数 按12m同比评分，当前信号 0.66%，历史分位 19.4（低尾异常），风险分 80.6。"
                .to_string(),
    };

    let drivers = build_top_risk_drivers(
        &[base_driver],
        &[risk],
        NaiveDate::from_ymd_opt(2026, 6, 10).unwrap(),
    );
    let explanation = &drivers[0].explanation;

    assert!(explanation.contains(
        "评分输入 +0.66 %（12m同比，不是 Case-Shiller 房价指数 当前水平；最新水平 322.00 指数）"
    ));
    assert!(explanation.contains("历史分位 19.4（低尾异常）"));
    assert!(explanation.contains("月频慢变量，更偏结构背景"));
    assert!(!explanation.contains("当前信号 0.66"));
}

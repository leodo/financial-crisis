use super::*;
use crate::assessment::{build_assessment_snapshot, prepare_reference_p60d, ServingModelContext};
use fc_domain::ActionabilityLevel;

#[test]
fn actionability_confidence_requires_margin_above_decision_threshold() {
    assert_eq!(actionability_confidence_from_probability(0.05, 0.05), 0.0);
    assert!(actionability_confidence_from_probability(0.20, 0.05) < 0.05);
    assert!(actionability_confidence_from_probability(0.55, 0.05) > 0.25);
}

#[test]
fn fused_actionability_suppresses_high_confidence_without_context() {
    let snapshot = RiskSnapshot {
        as_of_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        overall_score: 33.3,
        overall_level: RiskLevel::Watch,
        structural_score: 39.7,
        trigger_score: 25.4,
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
    };
    let probabilities = ProbabilityBlock {
        p_5d: 0.005,
        p_20d: 0.025,
        p_60d: 0.055,
    };
    let thresholds = ProbabilityActionThresholds {
        prepare_p60d: 0.023,
        hedge_p20d: 0.008,
        defend_p5d: 0.005,
    };

    let prepare = fuse_actionability_confidence(
        ActionabilityLevel::Prepare,
        0.954,
        &probabilities,
        &snapshot,
        29.8,
        thresholds,
    );
    let hedge = fuse_actionability_confidence(
        ActionabilityLevel::Hedge,
        0.812,
        &probabilities,
        &snapshot,
        29.8,
        thresholds,
    );

    assert!(prepare < 0.10);
    assert!(hedge < 0.10);
}

#[test]
fn fused_actionability_preserves_supported_prepare_context() {
    let snapshot = RiskSnapshot {
        as_of_date: NaiveDate::from_ymd_opt(2020, 2, 20).unwrap(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        overall_score: 61.0,
        overall_level: RiskLevel::Stress,
        structural_score: 58.0,
        trigger_score: 54.0,
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
    };
    let probabilities = ProbabilityBlock {
        p_5d: 0.018,
        p_20d: 0.052,
        p_60d: 0.118,
    };
    let thresholds = ProbabilityActionThresholds {
        prepare_p60d: 0.023,
        hedge_p20d: 0.008,
        defend_p5d: 0.005,
    };

    let prepare = fuse_actionability_confidence(
        ActionabilityLevel::Prepare,
        0.82,
        &probabilities,
        &snapshot,
        52.0,
        thresholds,
    );

    assert!(prepare > 0.35);
}

#[test]
fn assessment_snapshot_uses_support_actionability_for_prepare_continuity_without_head() {
    use fc_domain::{
        DataMode, DimensionScore, Frequency, HorizonEvaluationSummary, Indicator, IndicatorRisk,
        LogisticProbabilityModel, ModelReleaseManifest, ModelReleaseRecord, Observation,
        ProbabilityBundle, ProbabilityHorizonBundle, QualityGrade, RiskContributor, RiskDimension,
        RiskDirection,
    };

    fn indicator(indicator_id: &str, dimension: RiskDimension) -> Indicator {
        Indicator {
            indicator_id: indicator_id.to_string(),
            display_name: indicator_id.to_string(),
            dimension,
            description: "test".to_string(),
            unit: "index".to_string(),
            frequency: Frequency::Daily,
            risk_direction: RiskDirection::HigherIsRiskier,
            default_source_id: "test".to_string(),
            quality_tier: "gold".to_string(),
        }
    }

    fn observation(indicator: &Indicator, value: f64) -> Observation {
        Observation {
            indicator_id: indicator.indicator_id.clone(),
            entity_id: "us".to_string(),
            as_of_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            period_start: None,
            period_end: None,
            frequency: indicator.frequency,
            value,
            unit: indicator.unit.clone(),
            source_id: "test".to_string(),
            dataset_id: "test".to_string(),
            revision_time: None,
            publication_time: None,
            quality_score: 1.0,
            quality_flags: Vec::new(),
        }
    }

    fn indicator_risk(indicator: Indicator, latest_observation: Observation) -> IndicatorRisk {
        IndicatorRisk {
            indicator,
            latest_observation: Some(latest_observation),
            score: 60.0,
            level: RiskLevel::Stress,
            percentile: Some(0.6),
            change_30d: None,
            score_basis: "test".to_string(),
            score_input_value: Some(1.0),
            score_input_unit: Some("index".to_string()),
            quality_grade: QualityGrade::A,
            contribution: 10.0,
        }
    }

    fn serving_model() -> ServingModelContext {
        ServingModelContext {
            release: ModelReleaseRecord {
                manifest: ModelReleaseManifest {
                    release_id: "formal-runtime-test".to_string(),
                    market_scope: "financial_system".to_string(),
                    status: "active".to_string(),
                    probability_mode: "formal_bundle_v1".to_string(),
                    serving_status: "healthy".to_string(),
                    bundle_uri: "bundle.json".to_string(),
                    feature_set_version: "feature_formal_v1_main_20260531".to_string(),
                    label_version: "formal_label_v1_main".to_string(),
                    prob_model_version: "prob_bundle_test".to_string(),
                    calibration_version: "platt_test".to_string(),
                    posture_policy_version: "posture_test".to_string(),
                    action_playbook_version: "action_test".to_string(),
                    point_in_time_mode: "best_effort".to_string(),
                    training_range_start: None,
                    training_range_end: None,
                    calibration_range_start: None,
                    calibration_range_end: None,
                    evaluation_range_start: None,
                    evaluation_range_end: None,
                    brier_score: None,
                    log_loss: None,
                    ece: None,
                    note: String::new(),
                },
                created_at: Utc::now(),
                activated_at: None,
                retired_at: None,
            },
            probability_bundle: Some(ProbabilityBundle {
                bundle_id: "bundle".to_string(),
                market_scope: "financial_system".to_string(),
                probability_mode: "formal_bundle_v1".to_string(),
                model_family: "linear_v1".to_string(),
                feature_transform: "identity_v1".to_string(),
                created_at: Utc::now(),
                feature_names: Vec::new(),
                monotonic_min_gap_5d_to_20d: 0.0,
                monotonic_min_gap_20d_to_60d: 0.0,
                note: String::new(),
                horizons: [5_u32, 20, 60]
                    .into_iter()
                    .map(|horizon_days| ProbabilityHorizonBundle {
                        horizon_days,
                        decision_threshold: None,
                        threshold_diagnostics: None,
                        raw_model: LogisticProbabilityModel {
                            intercept: 6.0,
                            feature_transform: "identity_v1".to_string(),
                            feature_stats: Vec::new(),
                            coefficients: Vec::new(),
                        },
                        calibration: None,
                        evaluation: HorizonEvaluationSummary::default(),
                        family_overlays: Vec::new(),
                        family_overlay_audits: Vec::new(),
                    })
                    .collect(),
                evaluation: None,
                actionability: None,
            }),
            runtime_probability_mode: "formal_bundle_v1".to_string(),
            runtime_release_status: "healthy".to_string(),
        }
    }

    let core_indicator = indicator("core", RiskDimension::MacroFragility);
    let trigger_indicator = indicator("trigger", RiskDimension::MarketStress);
    let external_indicator = indicator("external", RiskDimension::ExternalSector);
    let indicator_risks = vec![
        indicator_risk(core_indicator.clone(), observation(&core_indicator, 1.0)),
        indicator_risk(
            trigger_indicator.clone(),
            observation(&trigger_indicator, 1.0),
        ),
        indicator_risk(
            external_indicator.clone(),
            observation(&external_indicator, 1.0),
        ),
    ];
    let snapshot = RiskSnapshot {
        as_of_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        overall_score: 53.5,
        overall_level: RiskLevel::Stress,
        structural_score: 62.6,
        trigger_score: 42.3,
        level_reason: "test".to_string(),
        dimensions: vec![
            DimensionScore {
                dimension: RiskDimension::MacroFragility,
                label: "macro".to_string(),
                score: 65.0,
                level: RiskLevel::Stress,
                change_30d: None,
                quality_score: 1.0,
                top_contributors: Vec::new(),
            },
            DimensionScore {
                dimension: RiskDimension::MarketStress,
                label: "trigger".to_string(),
                score: 61.0,
                level: RiskLevel::Stress,
                change_30d: None,
                quality_score: 1.0,
                top_contributors: Vec::new(),
            },
            DimensionScore {
                dimension: RiskDimension::ExternalSector,
                label: "external".to_string(),
                score: 45.0,
                level: RiskLevel::Watch,
                change_30d: None,
                quality_score: 1.0,
                top_contributors: Vec::new(),
            },
        ],
        top_contributors: vec![RiskContributor {
            indicator_id: "core".to_string(),
            display_name: "core".to_string(),
            dimension: RiskDimension::MacroFragility,
            score: 80.0,
            contribution: 12.0,
            explanation: "test".to_string(),
        }],
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
    };

    let (assessment, posture, probability_trace) = build_assessment_snapshot(
        DataMode::Sqlite,
        &snapshot,
        &indicator_risks,
        &[],
        &[],
        &[],
        None,
        Some(&serving_model()),
        &neutral_preferences(),
    );

    assert!(!probability_trace.actionability_enabled);
    assert!(!assessment.method.actionability_enabled);
    assert_eq!(assessment.time_to_risk_bucket, TimeToRiskBucket::Months);
    assert_eq!(posture.posture, DecisionPosture::Prepare);
    assert!(posture
        .trigger_codes
        .iter()
        .any(|code| code == "prepare_continuity_bridge"));
}

#[test]
fn assessment_snapshot_uses_runtime_final_p60d_for_prepare_plateau() {
    use fc_domain::{
        DataMode, DimensionScore, Frequency, HorizonEvaluationSummary, Indicator, IndicatorRisk,
        LogisticProbabilityModel, ModelReleaseManifest, ModelReleaseRecord, Observation,
        ProbabilityBundle, ProbabilityHorizonBundle, QualityGrade, RiskContributor, RiskDimension,
        RiskDirection,
    };

    fn indicator(indicator_id: &str, dimension: RiskDimension) -> Indicator {
        Indicator {
            indicator_id: indicator_id.to_string(),
            display_name: indicator_id.to_string(),
            dimension,
            description: "test".to_string(),
            unit: "index".to_string(),
            frequency: Frequency::Daily,
            risk_direction: RiskDirection::HigherIsRiskier,
            default_source_id: "test".to_string(),
            quality_tier: "gold".to_string(),
        }
    }

    fn observation(indicator: &Indicator, value: f64) -> Observation {
        Observation {
            indicator_id: indicator.indicator_id.clone(),
            entity_id: "us".to_string(),
            as_of_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            period_start: None,
            period_end: None,
            frequency: indicator.frequency,
            value,
            unit: indicator.unit.clone(),
            source_id: "test".to_string(),
            dataset_id: "test".to_string(),
            revision_time: None,
            publication_time: None,
            quality_score: 1.0,
            quality_flags: Vec::new(),
        }
    }

    fn indicator_risk(indicator: Indicator, latest_observation: Observation) -> IndicatorRisk {
        IndicatorRisk {
            indicator,
            latest_observation: Some(latest_observation),
            score: 60.0,
            level: RiskLevel::Stress,
            percentile: Some(0.6),
            change_30d: None,
            score_basis: "test".to_string(),
            score_input_value: Some(1.0),
            score_input_unit: Some("index".to_string()),
            quality_grade: QualityGrade::A,
            contribution: 10.0,
        }
    }

    fn serving_model() -> ServingModelContext {
        ServingModelContext {
            release: ModelReleaseRecord {
                manifest: ModelReleaseManifest {
                    release_id: "formal-runtime-plateau".to_string(),
                    market_scope: "financial_system".to_string(),
                    status: "active".to_string(),
                    probability_mode: "formal_bundle_v1".to_string(),
                    serving_status: "healthy".to_string(),
                    bundle_uri: "bundle.json".to_string(),
                    feature_set_version: "feature_formal_v1_main_20260607_plateau".to_string(),
                    label_version: "formal_label_v1_main".to_string(),
                    prob_model_version: "prob_bundle_test".to_string(),
                    calibration_version: "platt_test".to_string(),
                    posture_policy_version: "posture_test".to_string(),
                    action_playbook_version: "action_test".to_string(),
                    point_in_time_mode: "best_effort".to_string(),
                    training_range_start: None,
                    training_range_end: None,
                    calibration_range_start: None,
                    calibration_range_end: None,
                    evaluation_range_start: None,
                    evaluation_range_end: None,
                    brier_score: None,
                    log_loss: None,
                    ece: None,
                    note: String::new(),
                },
                created_at: Utc::now(),
                activated_at: None,
                retired_at: None,
            },
            probability_bundle: Some(ProbabilityBundle {
                bundle_id: "bundle".to_string(),
                market_scope: "financial_system".to_string(),
                probability_mode: "formal_bundle_v1".to_string(),
                model_family: "linear_v1".to_string(),
                feature_transform: "identity_v1".to_string(),
                created_at: Utc::now(),
                feature_names: Vec::new(),
                monotonic_min_gap_5d_to_20d: 0.0,
                monotonic_min_gap_20d_to_60d: 0.25,
                note: String::new(),
                horizons: vec![
                    ProbabilityHorizonBundle {
                        horizon_days: 5,
                        decision_threshold: None,
                        threshold_diagnostics: None,
                        raw_model: LogisticProbabilityModel {
                            intercept: -5.0,
                            feature_transform: "identity_v1".to_string(),
                            feature_stats: Vec::new(),
                            coefficients: Vec::new(),
                        },
                        calibration: None,
                        evaluation: HorizonEvaluationSummary::default(),
                        family_overlays: Vec::new(),
                        family_overlay_audits: Vec::new(),
                    },
                    ProbabilityHorizonBundle {
                        horizon_days: 20,
                        decision_threshold: None,
                        threshold_diagnostics: None,
                        raw_model: LogisticProbabilityModel {
                            intercept: 0.0,
                            feature_transform: "identity_v1".to_string(),
                            feature_stats: Vec::new(),
                            coefficients: Vec::new(),
                        },
                        calibration: None,
                        evaluation: HorizonEvaluationSummary::default(),
                        family_overlays: Vec::new(),
                        family_overlay_audits: Vec::new(),
                    },
                    ProbabilityHorizonBundle {
                        horizon_days: 60,
                        decision_threshold: None,
                        threshold_diagnostics: None,
                        raw_model: LogisticProbabilityModel {
                            intercept: 1.38629436112,
                            feature_transform: "identity_v1".to_string(),
                            feature_stats: Vec::new(),
                            coefficients: Vec::new(),
                        },
                        calibration: None,
                        evaluation: HorizonEvaluationSummary::default(),
                        family_overlays: Vec::new(),
                        family_overlay_audits: Vec::new(),
                    },
                ],
                evaluation: None,
                actionability: None,
            }),
            runtime_probability_mode: "formal_bundle_v1".to_string(),
            runtime_release_status: "healthy".to_string(),
        }
    }

    let core_indicator = indicator("core", RiskDimension::MacroFragility);
    let trigger_indicator = indicator("trigger", RiskDimension::MarketStress);
    let external_indicator = indicator("external", RiskDimension::ExternalSector);
    let indicator_risks = vec![
        indicator_risk(core_indicator.clone(), observation(&core_indicator, 1.0)),
        indicator_risk(
            trigger_indicator.clone(),
            observation(&trigger_indicator, 1.0),
        ),
        indicator_risk(
            external_indicator.clone(),
            observation(&external_indicator, 1.0),
        ),
    ];
    let snapshot = RiskSnapshot {
        as_of_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        overall_score: 55.0,
        overall_level: RiskLevel::Stress,
        structural_score: 57.0,
        trigger_score: 42.0,
        level_reason: "test".to_string(),
        dimensions: vec![
            DimensionScore {
                dimension: RiskDimension::MacroFragility,
                label: "macro".to_string(),
                score: 57.0,
                level: RiskLevel::Stress,
                change_30d: None,
                quality_score: 1.0,
                top_contributors: Vec::new(),
            },
            DimensionScore {
                dimension: RiskDimension::MarketStress,
                label: "trigger".to_string(),
                score: 42.0,
                level: RiskLevel::Watch,
                change_30d: None,
                quality_score: 1.0,
                top_contributors: Vec::new(),
            },
            DimensionScore {
                dimension: RiskDimension::ExternalSector,
                label: "external".to_string(),
                score: 46.0,
                level: RiskLevel::Stress,
                change_30d: None,
                quality_score: 1.0,
                top_contributors: Vec::new(),
            },
        ],
        top_contributors: vec![RiskContributor {
            indicator_id: "core".to_string(),
            display_name: "core".to_string(),
            dimension: RiskDimension::MacroFragility,
            score: 80.0,
            contribution: 12.0,
            explanation: "test".to_string(),
        }],
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
    };

    let (assessment, posture, probability_trace) = build_assessment_snapshot(
        DataMode::Sqlite,
        &snapshot,
        &indicator_risks,
        &[],
        &[],
        &[],
        None,
        Some(&serving_model()),
        &neutral_preferences(),
    );

    let prepare_reference = prepare_reference_p60d(&probability_trace);
    assert_eq!(
        probability_trace
            .probability_diagnostics
            .horizon_overlays
            .len(),
        3
    );
    let horizon_60d = probability_trace
        .probability_diagnostics
        .horizon_overlays
        .iter()
        .find(|horizon| horizon.horizon_days == 60)
        .expect("60d diagnostics should be present");

    assert_eq!(horizon_60d.final_probability, 0.8);
    assert_eq!(horizon_60d.runtime_final_probability, Some(0.8));
    assert_eq!(horizon_60d.monotonic_lift, 0.0);
    assert_eq!(prepare_reference, Some(0.8));
    assert_eq!(assessment.time_to_risk_bucket, TimeToRiskBucket::Months);
    assert_eq!(posture.posture, DecisionPosture::Prepare);
    assert!(posture
        .trigger_codes
        .iter()
        .any(|code| code == "prepare_probability_plateau"));
}

#[test]
fn formal_runtime_does_not_apply_cross_horizon_monotonic_gap() {
    use fc_domain::{
        DataMode, DimensionScore, Frequency, HorizonEvaluationSummary, Indicator, IndicatorRisk,
        LogisticProbabilityModel, ModelReleaseManifest, ModelReleaseRecord, Observation,
        ProbabilityBundle, ProbabilityHorizonBundle, QualityGrade, RiskContributor, RiskDimension,
        RiskDirection,
    };

    fn indicator(indicator_id: &str, dimension: RiskDimension) -> Indicator {
        Indicator {
            indicator_id: indicator_id.to_string(),
            display_name: indicator_id.to_string(),
            dimension,
            description: "test".to_string(),
            unit: "index".to_string(),
            frequency: Frequency::Daily,
            risk_direction: RiskDirection::HigherIsRiskier,
            default_source_id: "test".to_string(),
            quality_tier: "gold".to_string(),
        }
    }

    fn observation(indicator: &Indicator, value: f64) -> Observation {
        Observation {
            indicator_id: indicator.indicator_id.clone(),
            entity_id: "us".to_string(),
            as_of_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            period_start: None,
            period_end: None,
            frequency: indicator.frequency,
            value,
            unit: indicator.unit.clone(),
            source_id: "test".to_string(),
            dataset_id: "test".to_string(),
            revision_time: None,
            publication_time: None,
            quality_score: 1.0,
            quality_flags: Vec::new(),
        }
    }

    fn indicator_risk(indicator: Indicator, latest_observation: Observation) -> IndicatorRisk {
        IndicatorRisk {
            indicator,
            latest_observation: Some(latest_observation),
            score: 60.0,
            level: RiskLevel::Stress,
            percentile: Some(0.6),
            change_30d: None,
            score_basis: "test".to_string(),
            score_input_value: Some(1.0),
            score_input_unit: Some("index".to_string()),
            quality_grade: QualityGrade::A,
            contribution: 10.0,
        }
    }

    fn serving_model() -> ServingModelContext {
        ServingModelContext {
            release: ModelReleaseRecord {
                manifest: ModelReleaseManifest {
                    release_id: "formal-runtime-no-gap".to_string(),
                    market_scope: "financial_system".to_string(),
                    status: "active".to_string(),
                    probability_mode: "formal_bundle_v1".to_string(),
                    serving_status: "healthy".to_string(),
                    bundle_uri: "bundle.json".to_string(),
                    feature_set_version: "feature_formal_v1_main_20260607_plateau".to_string(),
                    label_version: "formal_label_v1_main".to_string(),
                    prob_model_version: "prob_bundle_test".to_string(),
                    calibration_version: "platt_test".to_string(),
                    posture_policy_version: "posture_test".to_string(),
                    action_playbook_version: "action_test".to_string(),
                    point_in_time_mode: "best_effort".to_string(),
                    training_range_start: None,
                    training_range_end: None,
                    calibration_range_start: None,
                    calibration_range_end: None,
                    evaluation_range_start: None,
                    evaluation_range_end: None,
                    brier_score: None,
                    log_loss: None,
                    ece: None,
                    note: String::new(),
                },
                created_at: Utc::now(),
                activated_at: None,
                retired_at: None,
            },
            probability_bundle: Some(ProbabilityBundle {
                bundle_id: "bundle".to_string(),
                market_scope: "financial_system".to_string(),
                probability_mode: "formal_bundle_v1".to_string(),
                model_family: "linear_v1".to_string(),
                feature_transform: "identity_v1".to_string(),
                created_at: Utc::now(),
                feature_names: Vec::new(),
                monotonic_min_gap_5d_to_20d: 0.0,
                monotonic_min_gap_20d_to_60d: 0.25,
                note: String::new(),
                horizons: vec![
                    ProbabilityHorizonBundle {
                        horizon_days: 5,
                        decision_threshold: None,
                        threshold_diagnostics: None,
                        raw_model: LogisticProbabilityModel {
                            intercept: -5.0,
                            feature_transform: "identity_v1".to_string(),
                            feature_stats: Vec::new(),
                            coefficients: Vec::new(),
                        },
                        calibration: None,
                        evaluation: HorizonEvaluationSummary::default(),
                        family_overlays: Vec::new(),
                        family_overlay_audits: Vec::new(),
                    },
                    ProbabilityHorizonBundle {
                        horizon_days: 20,
                        decision_threshold: None,
                        threshold_diagnostics: None,
                        raw_model: LogisticProbabilityModel {
                            intercept: 0.0,
                            feature_transform: "identity_v1".to_string(),
                            feature_stats: Vec::new(),
                            coefficients: Vec::new(),
                        },
                        calibration: None,
                        evaluation: HorizonEvaluationSummary::default(),
                        family_overlays: Vec::new(),
                        family_overlay_audits: Vec::new(),
                    },
                    ProbabilityHorizonBundle {
                        horizon_days: 60,
                        decision_threshold: None,
                        threshold_diagnostics: None,
                        raw_model: LogisticProbabilityModel {
                            intercept: -1.38629436112,
                            feature_transform: "identity_v1".to_string(),
                            feature_stats: Vec::new(),
                            coefficients: Vec::new(),
                        },
                        calibration: None,
                        evaluation: HorizonEvaluationSummary::default(),
                        family_overlays: Vec::new(),
                        family_overlay_audits: Vec::new(),
                    },
                ],
                evaluation: None,
                actionability: None,
            }),
            runtime_probability_mode: "formal_bundle_v1".to_string(),
            runtime_release_status: "healthy".to_string(),
        }
    }

    let core_indicator = indicator("core", RiskDimension::MacroFragility);
    let trigger_indicator = indicator("trigger", RiskDimension::MarketStress);
    let external_indicator = indicator("external", RiskDimension::ExternalSector);
    let indicator_risks = vec![
        indicator_risk(core_indicator.clone(), observation(&core_indicator, 1.0)),
        indicator_risk(
            trigger_indicator.clone(),
            observation(&trigger_indicator, 1.0),
        ),
        indicator_risk(
            external_indicator.clone(),
            observation(&external_indicator, 1.0),
        ),
    ];
    let snapshot = RiskSnapshot {
        as_of_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
        entity_id: "us".to_string(),
        market_scope: "financial_system".to_string(),
        overall_score: 55.0,
        overall_level: RiskLevel::Stress,
        structural_score: 57.0,
        trigger_score: 42.0,
        level_reason: "test".to_string(),
        dimensions: vec![
            DimensionScore {
                dimension: RiskDimension::MacroFragility,
                label: "macro".to_string(),
                score: 57.0,
                level: RiskLevel::Stress,
                change_30d: None,
                quality_score: 1.0,
                top_contributors: Vec::new(),
            },
            DimensionScore {
                dimension: RiskDimension::MarketStress,
                label: "trigger".to_string(),
                score: 42.0,
                level: RiskLevel::Watch,
                change_30d: None,
                quality_score: 1.0,
                top_contributors: Vec::new(),
            },
            DimensionScore {
                dimension: RiskDimension::ExternalSector,
                label: "external".to_string(),
                score: 46.0,
                level: RiskLevel::Stress,
                change_30d: None,
                quality_score: 1.0,
                top_contributors: Vec::new(),
            },
        ],
        top_contributors: vec![RiskContributor {
            indicator_id: "core".to_string(),
            display_name: "core".to_string(),
            dimension: RiskDimension::MacroFragility,
            score: 80.0,
            contribution: 12.0,
            explanation: "test".to_string(),
        }],
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
    };

    let (assessment, posture, probability_trace) = build_assessment_snapshot(
        DataMode::Sqlite,
        &snapshot,
        &indicator_risks,
        &[],
        &[],
        &[],
        None,
        Some(&serving_model()),
        &neutral_preferences(),
    );

    let horizon_20d = probability_trace
        .probability_diagnostics
        .horizon_overlays
        .iter()
        .find(|horizon| horizon.horizon_days == 20)
        .expect("20d diagnostics should be present");
    let horizon_60d = probability_trace
        .probability_diagnostics
        .horizon_overlays
        .iter()
        .find(|horizon| horizon.horizon_days == 60)
        .expect("60d diagnostics should be present");

    assert_eq!(assessment.probabilities.p_20d, 0.5);
    assert_eq!(assessment.probabilities.p_60d, 0.2);
    assert_eq!(horizon_20d.runtime_final_probability, Some(0.5));
    assert_eq!(horizon_60d.runtime_final_probability, Some(0.2));
    assert_eq!(horizon_20d.monotonic_lift, 0.0);
    assert_eq!(horizon_60d.monotonic_lift, 0.0);
    assert_eq!(posture.posture, DecisionPosture::Normal);
    assert_eq!(assessment.time_to_risk_bucket, TimeToRiskBucket::Normal);
}

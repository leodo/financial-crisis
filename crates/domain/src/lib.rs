pub mod actionability_gate;
pub mod alert;
pub mod assessment;
pub mod backtest;
pub mod feature_snapshot;
pub mod formal_dataset;
pub mod formal_feature;
pub mod formal_feature_runtime;
pub mod free_data_source;
pub mod historical_replay;
pub mod indicator;
pub mod model_release;
pub mod observation_window;
pub mod prediction_snapshot;
pub mod probability_bundle;
pub mod quality;
pub mod risk;
pub mod scenario_catalog;
pub mod scenario_data_coverage;
pub mod source;
pub mod stress_window;

pub use actionability_gate::*;
pub use alert::*;
pub use assessment::*;
pub use backtest::*;
pub use feature_snapshot::*;
pub use formal_dataset::*;
pub use formal_feature::*;
pub use formal_feature_runtime::*;
pub use free_data_source::*;
pub use historical_replay::*;
pub use indicator::*;
pub use model_release::*;
pub use observation_window::*;
pub use prediction_snapshot::*;
pub use probability_bundle::*;
pub use quality::*;
pub use risk::*;
pub use scenario_catalog::*;
pub use scenario_data_coverage::*;
pub use source::*;
pub use stress_window::*;

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, TimeZone, Utc};

    use super::*;

    #[test]
    fn risk_level_serde_round_trip() {
        let cases = [
            RiskLevel::Normal,
            RiskLevel::Watch,
            RiskLevel::Stress,
            RiskLevel::Warning,
            RiskLevel::Crisis,
        ];
        for level in cases {
            let json = serde_json::to_string(&level).unwrap();
            let deserialized: RiskLevel = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, level);
        }
    }

    #[test]
    fn risk_level_ordering() {
        assert!(RiskLevel::Normal < RiskLevel::Watch);
        assert!(RiskLevel::Watch < RiskLevel::Stress);
        assert!(RiskLevel::Stress < RiskLevel::Warning);
        assert!(RiskLevel::Warning < RiskLevel::Crisis);
    }

    #[test]
    fn risk_level_from_score() {
        assert_eq!(RiskLevel::from_score(20.0), RiskLevel::Normal);
        assert_eq!(RiskLevel::from_score(30.0), RiskLevel::Watch);
        assert_eq!(RiskLevel::from_score(50.0), RiskLevel::Stress);
        assert_eq!(RiskLevel::from_score(70.0), RiskLevel::Warning);
        assert_eq!(RiskLevel::from_score(85.0), RiskLevel::Crisis);
    }

    #[test]
    fn risk_level_code_and_label() {
        assert_eq!(RiskLevel::Normal.code(), "L0");
        assert_eq!(RiskLevel::Crisis.code(), "L4");
        assert_eq!(RiskLevel::Warning.label(), "Warning");
    }

    #[test]
    fn quality_grade_serde_round_trip() {
        let cases = [
            QualityGrade::A,
            QualityGrade::B,
            QualityGrade::C,
            QualityGrade::D,
            QualityGrade::F,
        ];
        for grade in cases {
            let json = serde_json::to_string(&grade).unwrap();
            let deserialized: QualityGrade = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, grade);
        }
    }

    #[test]
    fn data_quality_summary_serde_round_trip() {
        let summary = DataQualitySummary {
            overall_score: 85.0,
            grade: QualityGrade::B,
            stale_indicator_count: 2,
            low_quality_indicator_count: 1,
            prototype_source_count: 0,
            blocked_indicator_count: 0,
        };
        let json = serde_json::to_string(&summary).unwrap();
        let deserialized: DataQualitySummary = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.overall_score, 85.0);
        assert_eq!(deserialized.grade, QualityGrade::B);
    }

    #[test]
    fn observation_serde_round_trip() {
        let obs = Observation {
            indicator_id: "us_gdp".to_string(),
            entity_id: "us".to_string(),
            as_of_date: NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
            period_start: None,
            period_end: None,
            frequency: Frequency::Daily,
            value: 42.5,
            unit: "index".to_string(),
            source_id: "fred".to_string(),
            dataset_id: "fred_series_observations".to_string(),
            revision_time: None,
            publication_time: None,
            quality_score: 92.0,
            quality_flags: vec![],
        };
        let json = serde_json::to_string(&obs).unwrap();
        let deserialized: Observation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.indicator_id, "us_gdp");
        assert_eq!(deserialized.value, 42.5);
    }

    #[test]
    fn observation_with_all_optionals() {
        let obs = Observation {
            indicator_id: "us_gdp_q".to_string(),
            entity_id: "us".to_string(),
            as_of_date: NaiveDate::from_ymd_opt(2026, 3, 31).unwrap(),
            period_start: Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
            period_end: Some(NaiveDate::from_ymd_opt(2026, 3, 31).unwrap()),
            frequency: Frequency::Quarterly,
            value: 2.5,
            unit: "percent".to_string(),
            source_id: "fred".to_string(),
            dataset_id: "fred_series_observations".to_string(),
            revision_time: Some(Utc.with_ymd_and_hms(2026, 4, 15, 0, 0, 0).unwrap()),
            publication_time: Some(Utc.with_ymd_and_hms(2026, 4, 1, 12, 0, 0).unwrap()),
            quality_score: 95.0,
            quality_flags: vec!["revised".to_string()],
        };
        let json = serde_json::to_string(&obs).unwrap();
        let deserialized: Observation = serde_json::from_str(&json).unwrap();
        assert!(deserialized.period_start.is_some());
        assert!(deserialized.revision_time.is_some());
        assert_eq!(deserialized.quality_flags.len(), 1);
    }

    #[test]
    fn frequency_serde_round_trip() {
        let cases = [
            Frequency::Daily,
            Frequency::Weekly,
            Frequency::Monthly,
            Frequency::Quarterly,
            Frequency::Annual,
            Frequency::Event,
        ];
        for freq in cases {
            let json = serde_json::to_string(&freq).unwrap();
            let deserialized: Frequency = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, freq);
        }
    }

    #[test]
    fn risk_dimension_labels() {
        assert_eq!(RiskDimension::MacroFragility.label(), "宏观脆弱性");
        assert_eq!(RiskDimension::MarketStress.label(), "市场压力");
        assert_eq!(RiskDimension::EventsSentiment.label(), "事件与情绪");
    }

    #[test]
    fn risk_dimension_is_structural() {
        assert!(RiskDimension::MacroFragility.is_structural());
        assert!(RiskDimension::LeverageCredit.is_structural());
        assert!(!RiskDimension::MarketStress.is_structural());
        assert!(!RiskDimension::EventsSentiment.is_structural());
    }

    #[test]
    fn risk_contributor_serde_round_trip() {
        let contributor = RiskContributor {
            indicator_id: "us_gdp".to_string(),
            display_name: "GDP Growth".to_string(),
            dimension: RiskDimension::MacroFragility,
            score: 65.0,
            contribution: 0.25,
            explanation: "Below trend growth".to_string(),
        };
        let json = serde_json::to_string(&contributor).unwrap();
        let deserialized: RiskContributor = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.dimension, RiskDimension::MacroFragility);
        assert_eq!(deserialized.contribution, 0.25);
    }

    #[test]
    fn alert_event_serde_round_trip() {
        let event = AlertEvent {
            alert_id: uuid::Uuid::new_v4(),
            event_type: AlertType::RiskWarning,
            scope: "test_scope".to_string(),
            entity_id: "us".to_string(),
            dimension: Some(RiskDimension::MacroFragility),
            level: RiskLevel::Warning,
            status: AlertStatus::Open,
            triggered_at: Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap(),
            triggered_as_of_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            resolved_at: None,
            score: 75.0,
            previous_score: Some(50.0),
            trigger_reason: "Macro fragility elevated".to_string(),
            top_contributors: vec![],
            related_indicators: vec!["us_gdp".to_string()],
            method_version: "v1".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: AlertEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.alert_id, event.alert_id);
        assert_eq!(deserialized.level, RiskLevel::Warning);
        assert_eq!(deserialized.status, AlertStatus::Open);
    }

    #[test]
    fn risk_snapshot_serde_round_trip() {
        let snapshot = RiskSnapshot {
            as_of_date: NaiveDate::from_ymd_opt(2026, 6, 30).unwrap(),
            entity_id: "us".to_string(),
            market_scope: "us".to_string(),
            overall_score: 36.0,
            overall_level: RiskLevel::Watch,
            structural_score: 38.1,
            trigger_score: 33.5,
            level_reason: "Elevated structural risk".to_string(),
            dimensions: vec![],
            top_contributors: vec![],
            data_quality_summary: DataQualitySummary {
                overall_score: 85.0,
                grade: QualityGrade::B,
                stale_indicator_count: 1,
                low_quality_indicator_count: 0,
                prototype_source_count: 0,
                blocked_indicator_count: 0,
            },
            generated_at: Utc.with_ymd_and_hms(2026, 6, 30, 12, 0, 0).unwrap(),
            method_version: "v1".to_string(),
        };
        let json = serde_json::to_string(&snapshot).unwrap();
        let deserialized: RiskSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.overall_score, 36.0);
        assert_eq!(deserialized.overall_level, RiskLevel::Watch);
    }
}

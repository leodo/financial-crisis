use std::{collections::HashMap, fs, path::Path as FsPath};

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

use super::read_json_artifact;

const SUMMARY_DATASET_IDS: [&str; 3] = [
    "formal_v1_main_1990_daily",
    "formal_v1_ext_stress_1990_daily",
    "formal_v1_ext_acute_pre1990",
];

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct DatasetSummaryArtifactDatasetRecord {
    pub(super) dataset_id: String,
    pub(super) dataset_version: String,
    pub(super) market_scope: String,
    pub(super) feature_set_version: String,
    pub(super) label_version: String,
    pub(super) scenario_set_version: String,
    pub(super) point_in_time_mode: String,
    pub(super) from_date: String,
    pub(super) to_date: String,
    pub(super) train_end_date: String,
    pub(super) calibration_end_date: String,
    pub(super) evaluation_start_date: String,
    pub(super) row_count: usize,
    pub(super) note: Option<String>,
    pub(super) created_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct DatasetSummaryArtifactSplitSummary {
    pub(super) split_name: String,
    pub(super) row_count: usize,
    pub(super) positive_5d_count: usize,
    pub(super) positive_20d_count: usize,
    pub(super) positive_60d_count: usize,
    pub(super) prepare_primary_count: usize,
    pub(super) hedge_primary_count: usize,
    pub(super) defend_primary_count: usize,
    pub(super) late_validation_row_count: usize,
    pub(super) protected_row_count: usize,
    pub(super) avg_coverage_score: f64,
    pub(super) scenario_count: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct DatasetSummaryArtifactScenarioSummary {
    pub(super) scenario_id: String,
    pub(super) label: Option<String>,
    pub(super) row_count: usize,
    pub(super) split_count: usize,
    pub(super) first_as_of_date: String,
    pub(super) last_as_of_date: String,
    pub(super) family: Option<String>,
    pub(super) training_role: Option<String>,
    pub(super) protected_window: Option<bool>,
    pub(super) episode_template_id: Option<String>,
    #[serde(default)]
    pub(super) default_horizon_roles: Vec<u32>,
    pub(super) coverage_recommended_role: Option<String>,
    pub(super) coverage_grade: Option<String>,
    pub(super) coverage_point_in_time_mode: Option<String>,
    pub(super) coverage_current_status: Option<String>,
    #[serde(default)]
    pub(super) coverage_blocking_gaps: Vec<String>,
    #[serde(default)]
    pub(super) coverage_free_sources: Vec<String>,
    pub(super) usable_for_main_training: Option<bool>,
    pub(super) usable_for_extension_training: Option<bool>,
    pub(super) usable_for_protected_stress: Option<bool>,
    pub(super) usable_for_historical_analog: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct DatasetSummaryArtifactCoverageCatalog {
    pub(super) catalog_id: String,
    pub(super) scenario_catalog_id: String,
    pub(super) market_scope: String,
    pub(super) source: String,
    pub(super) warning: Option<String>,
    pub(super) dataset_intent: String,
    pub(super) aligned_scenario_count: usize,
    pub(super) total_scenario_count: usize,
    pub(super) main_training_eligible_count: usize,
    pub(super) extension_training_eligible_count: usize,
    pub(super) protected_stress_eligible_count: usize,
    pub(super) historical_analog_eligible_count: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct DatasetSummaryArtifactWire {
    generated_at: String,
    dataset_key: String,
    dataset: DatasetSummaryArtifactDatasetRecord,
    #[serde(default)]
    split_summaries: Vec<DatasetSummaryArtifactSplitSummary>,
    #[serde(default)]
    scenario_summaries: Vec<DatasetSummaryArtifactScenarioSummary>,
    coverage_catalog: DatasetSummaryArtifactCoverageCatalog,
    recommendation: String,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct DatasetSummaryArtifactSummary {
    pub(super) generated_at: String,
    pub(super) source: String,
    pub(super) dataset_key: String,
    pub(super) dataset: DatasetSummaryArtifactDatasetRecord,
    pub(super) split_summaries: Vec<DatasetSummaryArtifactSplitSummary>,
    pub(super) scenario_summaries: Vec<DatasetSummaryArtifactScenarioSummary>,
    pub(super) coverage_catalog: DatasetSummaryArtifactCoverageCatalog,
    pub(super) recommendation: String,
}

pub(super) fn load_latest_dataset_summaries(
    market_scope: &str,
) -> Vec<DatasetSummaryArtifactSummary> {
    let mut latest = HashMap::<
        String,
        (
            usize,
            Option<DateTime<FixedOffset>>,
            DatasetSummaryArtifactSummary,
        ),
    >::new();
    for directory in [
        "artifacts/research/dataset-summary-check",
        "reports/formal-dataset",
    ] {
        let path = FsPath::new(directory);
        let Ok(entries) = fs::read_dir(path) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                continue;
            }
            let Some(body) = read_json_artifact(&path) else {
                continue;
            };
            let wire = match serde_json::from_str::<DatasetSummaryArtifactWire>(&body) {
                Ok(wire) => wire,
                Err(error) => {
                    tracing::warn!(
                        path = %path.to_string_lossy(),
                        %error,
                        "failed to parse formal dataset summary artifact"
                    );
                    continue;
                }
            };
            if wire.dataset.market_scope != market_scope {
                continue;
            }
            if !SUMMARY_DATASET_IDS.contains(&wire.dataset.dataset_id.as_str()) {
                continue;
            }
            let summary = DatasetSummaryArtifactSummary {
                generated_at: wire.generated_at.clone(),
                source: path.to_string_lossy().into_owned(),
                dataset_key: wire.dataset_key,
                dataset: wire.dataset,
                split_summaries: wire.split_summaries,
                scenario_summaries: wire.scenario_summaries,
                coverage_catalog: wire.coverage_catalog,
                recommendation: wire.recommendation,
            };
            let generated_at = DateTime::parse_from_rfc3339(&wire.generated_at).ok();
            latest
                .entry(summary.dataset.dataset_id.clone())
                .and_modify(|current| {
                    let replace = summary.dataset.row_count > current.0
                        || (summary.dataset.row_count == current.0 && generated_at > current.1);
                    if replace {
                        *current = (summary.dataset.row_count, generated_at, summary.clone());
                    }
                })
                .or_insert((summary.dataset.row_count, generated_at, summary));
        }
    }

    SUMMARY_DATASET_IDS
        .iter()
        .filter_map(|dataset_id| latest.remove(*dataset_id).map(|(_, _, summary)| summary))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::DatasetSummaryArtifactWire;

    #[test]
    fn dataset_summary_wire_allows_missing_optional_arrays() {
        let body = r#"
        {
          "generated_at": "2026-06-09T00:00:00+00:00",
          "dataset_key": "formal_v1_main_1990_daily:latest",
          "dataset": {
            "dataset_id": "formal_v1_main_1990_daily",
            "dataset_version": "latest",
            "market_scope": "financial_system",
            "feature_set_version": "feature_formal_v1",
            "label_version": "formal_label_v1_main",
            "scenario_set_version": "scenario_v1_main",
            "point_in_time_mode": "best_effort",
            "from_date": "1990-01-02",
            "to_date": "2026-05-31",
            "train_end_date": "2009-08-13",
            "calibration_end_date": "2020-03-06",
            "evaluation_start_date": "2020-03-07",
            "row_count": 100,
            "created_at": "2026-06-09T00:00:00Z"
          },
          "coverage_catalog": {
            "catalog_id": "scenario_data_coverage_v1",
            "scenario_catalog_id": "scenario_v1_main",
            "market_scope": "financial_system",
            "source": "embedded:test",
            "dataset_intent": "main_training",
            "aligned_scenario_count": 1,
            "total_scenario_count": 1,
            "main_training_eligible_count": 1,
            "extension_training_eligible_count": 0,
            "protected_stress_eligible_count": 0,
            "historical_analog_eligible_count": 1
          },
          "recommendation": "ok"
        }
        "#;

        let wire: DatasetSummaryArtifactWire =
            serde_json::from_str(body).expect("wire should deserialize");
        assert!(wire.split_summaries.is_empty());
        assert!(wire.scenario_summaries.is_empty());
        assert_eq!(wire.coverage_catalog.dataset_intent, "main_training");
    }
}

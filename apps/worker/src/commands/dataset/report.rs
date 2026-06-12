use chrono::NaiveDate;
use fc_domain::{FormalDatasetRecord, FormalDatasetRowRecord};
use serde::Serialize;

mod render;
mod slice;
mod summary;

pub(crate) use render::print_formal_dataset_slice_summary;
#[cfg(test)]
pub(crate) use render::render_formal_dataset_slice_csv;
pub(crate) use render::{print_formal_dataset_summary, render_formal_dataset_summary_markdown};
#[cfg(test)]
pub(crate) use slice::sanitize_filename_component;
pub(crate) use slice::{build_formal_dataset_slice_export, write_formal_dataset_slice_report};
pub(crate) use summary::build_formal_dataset_summary;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct FormalDatasetSplitSummary {
    split_name: String,
    row_count: usize,
    positive_5d_count: usize,
    positive_5d_rate: f64,
    positive_20d_count: usize,
    positive_20d_rate: f64,
    positive_60d_count: usize,
    positive_60d_rate: f64,
    prepare_primary_count: usize,
    prepare_primary_rate: f64,
    hedge_primary_count: usize,
    hedge_primary_rate: f64,
    defend_primary_count: usize,
    defend_primary_rate: f64,
    late_validation_row_count: usize,
    late_validation_row_rate: f64,
    protected_row_count: usize,
    protected_row_rate: f64,
    avg_coverage_score: f64,
    avg_core_feature_coverage: f64,
    avg_trigger_feature_coverage: f64,
    avg_external_feature_coverage: f64,
    scenario_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct FormalDatasetScenarioSummary {
    scenario_id: String,
    label: Option<String>,
    row_count: usize,
    split_count: usize,
    first_as_of_date: NaiveDate,
    last_as_of_date: NaiveDate,
    family: Option<String>,
    training_role: Option<String>,
    protected_window: Option<bool>,
    episode_template_id: Option<String>,
    default_horizon_roles: Vec<u32>,
    coverage_recommended_role: Option<String>,
    coverage_grade: Option<String>,
    coverage_point_in_time_mode: Option<String>,
    coverage_current_status: Option<String>,
    coverage_blocking_gaps: Vec<String>,
    coverage_free_sources: Vec<String>,
    usable_for_main_training: Option<bool>,
    usable_for_extension_training: Option<bool>,
    usable_for_protected_stress: Option<bool>,
    usable_for_historical_analog: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct FormalDatasetFamilySummary {
    family: String,
    row_count: usize,
    scenario_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct FormalDatasetQualitySummary {
    grade: String,
    row_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct FormalDatasetRegimeSummary {
    split_name: String,
    horizon_days: u32,
    regime: String,
    row_count: usize,
    row_rate: f64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct FormalDatasetCoverageCatalogSummary {
    catalog_id: String,
    scenario_catalog_id: String,
    market_scope: String,
    source: String,
    warning: Option<String>,
    dataset_intent: String,
    aligned_scenario_count: usize,
    total_scenario_count: usize,
    main_training_eligible_count: usize,
    extension_training_eligible_count: usize,
    protected_stress_eligible_count: usize,
    historical_analog_eligible_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct FormalDatasetSummaryEnvelope {
    pub(crate) generated_at: String,
    pub(crate) dataset_key: String,
    pub(crate) dataset: FormalDatasetRecord,
    pub(crate) split_summaries: Vec<FormalDatasetSplitSummary>,
    pub(crate) scenario_summaries: Vec<FormalDatasetScenarioSummary>,
    pub(crate) family_summaries: Vec<FormalDatasetFamilySummary>,
    pub(crate) quality_summaries: Vec<FormalDatasetQualitySummary>,
    pub(crate) regime_summaries: Vec<FormalDatasetRegimeSummary>,
    pub(crate) coverage_catalog: FormalDatasetCoverageCatalogSummary,
    pub(crate) recommendation: String,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct FormalDatasetSliceExport {
    pub(super) exported_at: String,
    pub(super) dataset_key: String,
    pub(super) dataset: FormalDatasetRecord,
    pub(super) scenario_id: String,
    pub(super) split_name: Option<String>,
    pub(super) from_date: Option<NaiveDate>,
    pub(super) to_date: Option<NaiveDate>,
    pub(super) row_count: usize,
    pub(super) feature_names: Vec<String>,
    pub(super) rows: Vec<FormalDatasetRowRecord>,
}

#[derive(Debug, Clone)]
struct ScenarioSummaryMetadata {
    label: String,
    family: String,
    training_role: String,
    protected_window: bool,
    episode_template_id: String,
    default_horizon_roles: Vec<u32>,
}

#[derive(Debug, Clone)]
struct ScenarioCoverageMetadata {
    recommended_role: String,
    coverage_grade: String,
    point_in_time_mode: String,
    current_status: String,
    blocking_gaps: Vec<String>,
    free_sources: Vec<String>,
    usable_for_main_training: bool,
    usable_for_extension_training: bool,
    usable_for_protected_stress: bool,
    usable_for_historical_analog: bool,
}

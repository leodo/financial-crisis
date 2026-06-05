mod build;
mod execute;
mod options;
mod report;
mod scenarios;
mod split;

#[cfg(test)]
pub(crate) use build::{formal_dataset_min_date, formal_dataset_snapshot_is_usable};
pub(crate) use execute::{
    research_formal_dataset_build_main, research_formal_dataset_list_main,
    research_formal_dataset_slice_main, research_formal_dataset_summarize_main,
};
#[cfg(test)]
pub(crate) use options::{
    FormalDatasetBuildOptions, FormalDatasetSliceOptions, FormalDatasetSummaryOptions,
};
#[cfg(test)]
pub(crate) use report::{render_formal_dataset_slice_csv, sanitize_filename_component};
pub(crate) use report::{render_formal_dataset_summary_markdown, FormalDatasetSummaryEnvelope};
pub(crate) use scenarios::{
    action_episode_template_code, load_formal_dataset_scenario_sets,
    load_label_set_crisis_scenarios, scenario_family_code, scenario_training_role_code,
};
#[cfg(test)]
pub(crate) use split::scenario_count_for_index_range;
pub(crate) use split::{
    collect_formal_dataset_scenario_ranges, formal_dataset_split_profile,
    row_has_action_episode_label, scenario_count_for_split_range, FormalDatasetSplitProfile,
    ScenarioRowRange,
};
#[cfg(test)]
pub(crate) use split::{
    formal_dataset_split_requirements, scenario_aware_formal_split_bounds, FormalSplitLabelSupport,
};

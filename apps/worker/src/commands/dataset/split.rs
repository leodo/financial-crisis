use std::collections::BTreeMap;
#[cfg(test)]
use std::collections::BTreeSet;

use anyhow::bail;
use fc_domain::{ActionabilityLevel, FormalDatasetRowRecord};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FormalDatasetSplitProfile {
    Main,
    ExtensionAcute,
    ExtensionStress,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct FormalDatasetSplitRequirements {
    pub(crate) minimum_scenario_ranges: usize,
    pub(crate) minimum_calibration_scenarios: usize,
    pub(crate) minimum_evaluation_scenarios: usize,
    pub(crate) require_forward_5d: bool,
    pub(crate) require_forward_20d: bool,
    pub(crate) require_forward_60d: bool,
    pub(crate) require_prepare: bool,
    pub(crate) require_hedge: bool,
    pub(crate) require_defend: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ScenarioRowRange {
    pub(crate) scenario_id: String,
    pub(crate) family: String,
    pub(crate) start_index: usize,
    pub(crate) end_index: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct FormalSplitLabelSupport {
    forward_5d: Vec<usize>,
    forward_20d: Vec<usize>,
    forward_60d: Vec<usize>,
    prepare_primary: Vec<usize>,
    hedge_primary: Vec<usize>,
    defend_primary: Vec<usize>,
}

pub(crate) fn formal_dataset_split_profile(label_version: &str) -> FormalDatasetSplitProfile {
    match label_version {
        "formal_label_v1_ext_acute" => FormalDatasetSplitProfile::ExtensionAcute,
        "formal_label_v1_ext_stress" => FormalDatasetSplitProfile::ExtensionStress,
        _ => FormalDatasetSplitProfile::Main,
    }
}

pub(crate) fn formal_dataset_split_requirements(
    label_version: &str,
) -> FormalDatasetSplitRequirements {
    match formal_dataset_split_profile(label_version) {
        FormalDatasetSplitProfile::Main => FormalDatasetSplitRequirements {
            minimum_scenario_ranges: 3,
            minimum_calibration_scenarios: 2,
            minimum_evaluation_scenarios: 2,
            require_forward_5d: true,
            require_forward_20d: true,
            require_forward_60d: true,
            require_prepare: true,
            require_hedge: true,
            require_defend: true,
        },
        FormalDatasetSplitProfile::ExtensionAcute => FormalDatasetSplitRequirements {
            minimum_scenario_ranges: 2,
            minimum_calibration_scenarios: 2,
            minimum_evaluation_scenarios: 1,
            require_forward_5d: true,
            require_forward_20d: true,
            require_forward_60d: false,
            require_prepare: false,
            require_hedge: false,
            require_defend: true,
        },
        FormalDatasetSplitProfile::ExtensionStress => FormalDatasetSplitRequirements {
            minimum_scenario_ranges: 3,
            minimum_calibration_scenarios: 2,
            minimum_evaluation_scenarios: 2,
            require_forward_5d: false,
            require_forward_20d: true,
            require_forward_60d: true,
            require_prepare: true,
            require_hedge: true,
            require_defend: false,
        },
    }
}

pub(crate) fn assign_formal_dataset_splits(
    rows: &mut [FormalDatasetRowRecord],
    scenarios: &[crate::CrisisScenario],
    label_version: &str,
) {
    let ranges = collect_formal_dataset_scenario_ranges(rows, scenarios);
    let split_requirements = formal_dataset_split_requirements(label_version);
    let Ok((train_end, calibration_end)) =
        scenario_aware_formal_split_bounds(rows, &ranges, split_requirements)
            .or_else(|_| crate::chronological_split_bounds(rows.len()))
    else {
        return;
    };
    for (index, row) in rows.iter_mut().enumerate() {
        row.split_name = split_name_for_index(index, train_end, calibration_end).to_string();
    }
}

pub(crate) fn scenario_aware_formal_split_bounds(
    rows: &[FormalDatasetRowRecord],
    ranges: &[ScenarioRowRange],
    split_requirements: FormalDatasetSplitRequirements,
) -> anyhow::Result<(usize, usize)> {
    if ranges.len() < split_requirements.minimum_scenario_ranges {
        bail!(
            "fewer than {} scenario ranges available for scenario-aware split",
            split_requirements.minimum_scenario_ranges
        );
    }
    let (baseline_train_end, baseline_calibration_end) =
        crate::chronological_split_bounds(rows.len())?;
    let label_support = FormalSplitLabelSupport::from_rows(rows);
    let mut best_candidate = None::<(usize, usize, usize, usize, usize)>;

    for first_boundary_scenario in 0..ranges.len().saturating_sub(1) {
        let train_candidates = split_boundaries_within_scenario(&ranges[first_boundary_scenario]);
        for second_boundary_scenario in (first_boundary_scenario + 1)..ranges.len() {
            let calibration_candidates =
                split_boundaries_within_scenario(&ranges[second_boundary_scenario]);
            for &train_end in &train_candidates {
                for &calibration_end in &calibration_candidates {
                    if crate::validate_split_bounds(rows.len(), train_end, calibration_end).is_err()
                    {
                        continue;
                    }

                    let calibration_scenario_count =
                        scenario_count_for_split_range(ranges, train_end, calibration_end);
                    let evaluation_scenario_count =
                        scenario_count_for_split_range(ranges, calibration_end, rows.len());
                    if calibration_scenario_count < split_requirements.minimum_calibration_scenarios
                        || evaluation_scenario_count
                            < split_requirements.minimum_evaluation_scenarios
                    {
                        continue;
                    }

                    if !label_support.split_has_required_label_support(
                        0,
                        train_end,
                        split_requirements,
                    ) || !label_support.split_has_required_label_support(
                        train_end,
                        calibration_end,
                        split_requirements,
                    ) || !label_support.split_has_required_label_support(
                        calibration_end,
                        rows.len(),
                        split_requirements,
                    ) {
                        continue;
                    }

                    let scenario_coverage =
                        calibration_scenario_count.saturating_add(evaluation_scenario_count);
                    let evaluation_actionability_support_score =
                        split_actionability_scenario_support_score(
                            rows,
                            ranges,
                            calibration_end,
                            rows.len(),
                            split_requirements,
                        );
                    let deviation_from_baseline = train_end.abs_diff(baseline_train_end)
                        + calibration_end.abs_diff(baseline_calibration_end);
                    let replace_candidate = match best_candidate {
                        None => true,
                        Some((
                            best_train_end,
                            best_calibration_end,
                            best_coverage,
                            best_actionability_support_score,
                            best_deviation,
                        )) => {
                            scenario_coverage > best_coverage
                                || (scenario_coverage == best_coverage
                                    && evaluation_actionability_support_score
                                        > best_actionability_support_score)
                                || (scenario_coverage == best_coverage
                                    && evaluation_actionability_support_score
                                        == best_actionability_support_score
                                    && deviation_from_baseline < best_deviation)
                                || (scenario_coverage == best_coverage
                                    && evaluation_actionability_support_score
                                        == best_actionability_support_score
                                    && deviation_from_baseline == best_deviation
                                    && (train_end > best_train_end
                                        || (train_end == best_train_end
                                            && calibration_end > best_calibration_end)))
                        }
                    };
                    if replace_candidate {
                        best_candidate = Some((
                            train_end,
                            calibration_end,
                            scenario_coverage,
                            evaluation_actionability_support_score,
                            deviation_from_baseline,
                        ));
                    }
                }
            }
        }
    }

    best_candidate
        .map(|(train_end, calibration_end, _, _, _)| (train_end, calibration_end))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "no scenario-aware split preserves multi-scenario calibration/evaluation coverage together with forward/action-episode label support"
            )
        })
}

pub(crate) fn collect_formal_dataset_scenario_ranges(
    rows: &[FormalDatasetRowRecord],
    scenarios: &[crate::CrisisScenario],
) -> Vec<ScenarioRowRange> {
    let family_by_id = scenarios
        .iter()
        .map(|scenario| (scenario.scenario_id.as_str(), scenario.family.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut ranges = BTreeMap::<String, (usize, usize)>::new();
    for (index, row) in rows.iter().enumerate() {
        let Some(scenario_id) = row.primary_scenario_id.as_ref() else {
            continue;
        };
        ranges
            .entry(scenario_id.clone())
            .and_modify(|range| range.1 = index)
            .or_insert((index, index));
    }

    let mut summaries = ranges
        .into_iter()
        .map(|(scenario_id, (start_index, end_index))| ScenarioRowRange {
            family: family_by_id
                .get(scenario_id.as_str())
                .cloned()
                .or_else(|| rows[start_index].scenario_family.clone())
                .unwrap_or_else(|| "unknown".to_string()),
            scenario_id,
            start_index,
            end_index,
        })
        .collect::<Vec<_>>();
    summaries.sort_by_key(|range| range.start_index);
    summaries
}

impl FormalSplitLabelSupport {
    pub(crate) fn from_rows(rows: &[FormalDatasetRowRecord]) -> Self {
        let mut support = Self {
            forward_5d: Vec::with_capacity(rows.len() + 1),
            forward_20d: Vec::with_capacity(rows.len() + 1),
            forward_60d: Vec::with_capacity(rows.len() + 1),
            prepare_primary: Vec::with_capacity(rows.len() + 1),
            hedge_primary: Vec::with_capacity(rows.len() + 1),
            defend_primary: Vec::with_capacity(rows.len() + 1),
        };
        support.forward_5d.push(0);
        support.forward_20d.push(0);
        support.forward_60d.push(0);
        support.prepare_primary.push(0);
        support.hedge_primary.push(0);
        support.defend_primary.push(0);
        for row in rows {
            support.forward_5d.push(
                support.forward_5d.last().copied().unwrap_or_default()
                    + usize::from(row.label_5d > 0),
            );
            support.forward_20d.push(
                support.forward_20d.last().copied().unwrap_or_default()
                    + usize::from(row.label_20d > 0),
            );
            support.forward_60d.push(
                support.forward_60d.last().copied().unwrap_or_default()
                    + usize::from(row.label_60d > 0),
            );
            support.prepare_primary.push(
                support.prepare_primary.last().copied().unwrap_or_default()
                    + usize::from(row.prepare_episode_label > 0),
            );
            support.hedge_primary.push(
                support.hedge_primary.last().copied().unwrap_or_default()
                    + usize::from(row.hedge_episode_label > 0),
            );
            support.defend_primary.push(
                support.defend_primary.last().copied().unwrap_or_default()
                    + usize::from(row.defend_episode_label > 0),
            );
        }
        support
    }

    pub(crate) fn split_has_required_label_support(
        &self,
        start: usize,
        end: usize,
        split_requirements: FormalDatasetSplitRequirements,
    ) -> bool {
        end > start
            && (!split_requirements.require_forward_5d
                || self.has_positive(&self.forward_5d, start, end))
            && (!split_requirements.require_forward_20d
                || self.has_positive(&self.forward_20d, start, end))
            && (!split_requirements.require_forward_60d
                || self.has_positive(&self.forward_60d, start, end))
            && (!split_requirements.require_prepare
                || self.has_positive(&self.prepare_primary, start, end))
            && (!split_requirements.require_hedge
                || self.has_positive(&self.hedge_primary, start, end))
            && (!split_requirements.require_defend
                || self.has_positive(&self.defend_primary, start, end))
    }

    fn has_positive(&self, prefix: &[usize], start: usize, end: usize) -> bool {
        prefix[end] > prefix[start]
    }
}

fn split_boundaries_within_scenario(range: &ScenarioRowRange) -> Vec<usize> {
    ((range.start_index + 1)..=range.end_index.saturating_add(1)).collect()
}

pub(crate) fn scenario_count_for_split_range(
    ranges: &[ScenarioRowRange],
    start: usize,
    end: usize,
) -> usize {
    ranges
        .iter()
        .filter(|range| start <= range.end_index && end > range.start_index)
        .count()
}

fn split_actionability_scenario_support_score(
    rows: &[FormalDatasetRowRecord],
    ranges: &[ScenarioRowRange],
    start: usize,
    end: usize,
    split_requirements: FormalDatasetSplitRequirements,
) -> usize {
    let mut score = 0;
    if split_requirements.require_prepare {
        score += actionability_positive_scenario_count_for_split_range(
            rows,
            ranges,
            start,
            end,
            ActionabilityLevel::Prepare,
        )
        .min(2);
    }
    if split_requirements.require_hedge {
        score += actionability_positive_scenario_count_for_split_range(
            rows,
            ranges,
            start,
            end,
            ActionabilityLevel::Hedge,
        )
        .min(2);
    }
    if split_requirements.require_defend {
        score += actionability_positive_scenario_count_for_split_range(
            rows,
            ranges,
            start,
            end,
            ActionabilityLevel::Defend,
        )
        .min(2);
    }
    score
}

fn actionability_positive_scenario_count_for_split_range(
    rows: &[FormalDatasetRowRecord],
    ranges: &[ScenarioRowRange],
    start: usize,
    end: usize,
    level: ActionabilityLevel,
) -> usize {
    ranges
        .iter()
        .filter(|range| {
            let overlap_start = start.max(range.start_index);
            let overlap_end = end.min(range.end_index.saturating_add(1));
            overlap_start < overlap_end
                && rows[overlap_start..overlap_end].iter().any(|row| {
                    row.primary_scenario_id.as_deref() == Some(range.scenario_id.as_str())
                        && row_has_action_episode_label(row, level)
                })
        })
        .count()
}

pub(crate) fn row_has_action_episode_label(
    row: &FormalDatasetRowRecord,
    level: ActionabilityLevel,
) -> bool {
    match level {
        ActionabilityLevel::Prepare => row.prepare_episode_label > 0,
        ActionabilityLevel::Hedge => row.hedge_episode_label > 0,
        ActionabilityLevel::Defend => row.defend_episode_label > 0,
    }
}

#[cfg(test)]
pub(crate) fn scenario_count_for_index_range(
    rows: &[FormalDatasetRowRecord],
    start: usize,
    end: usize,
) -> usize {
    rows[start.min(rows.len())..end.min(rows.len())]
        .iter()
        .filter_map(|row| row.primary_scenario_id.as_ref())
        .collect::<BTreeSet<_>>()
        .len()
}

fn split_name_for_index(index: usize, train_end: usize, calibration_end: usize) -> &'static str {
    if index < train_end {
        "train"
    } else if index < calibration_end {
        "calibration"
    } else {
        "evaluation"
    }
}

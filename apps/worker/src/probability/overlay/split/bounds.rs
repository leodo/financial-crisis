use std::collections::BTreeMap;

use super::{
    dataset::family_overlay_candidate_row, FamilyOverlayAuditSpec, FamilyOverlaySplitSupport,
};

pub(super) fn family_overlay_split_bounds(
    rows: &[crate::ProbabilityTrainingRow],
    spec: &FamilyOverlayAuditSpec,
    horizon_days: u32,
    label_mode: crate::ProbabilityTargetLabelMode,
) -> anyhow::Result<(usize, usize)> {
    if spec.scenario_family.is_none() {
        anyhow::bail!("proxy-only overlays do not use family-aware split bounds");
    }

    let ranges = collect_family_overlay_scenario_ranges(rows, spec);
    if ranges.len() < spec.min_scenario_count as usize {
        anyhow::bail!("not enough family scenario ranges for overlay split");
    }

    let (baseline_train_end, baseline_calibration_end) =
        crate::chronological_split_bounds(rows.len())?;
    let support = FamilyOverlaySplitSupport::from_rows(rows, spec, horizon_days, label_mode);
    let mut best_candidate = None::<(usize, usize, usize, usize, usize, usize, usize)>;

    for first_boundary_scenario in 0..ranges.len().saturating_sub(1) {
        let train_candidates =
            family_overlay_split_boundaries_within_range(&ranges[first_boundary_scenario]);
        for second_boundary_scenario in (first_boundary_scenario + 1)..ranges.len() {
            let calibration_candidates =
                family_overlay_split_boundaries_within_range(&ranges[second_boundary_scenario]);
            for &train_end in &train_candidates {
                for &calibration_end in &calibration_candidates {
                    if crate::validate_split_bounds(rows.len(), train_end, calibration_end).is_err()
                    {
                        continue;
                    }

                    let calibration_scenario_count =
                        crate::scenario_count_for_split_range(&ranges, train_end, calibration_end);
                    let evaluation_scenario_count =
                        crate::scenario_count_for_split_range(&ranges, calibration_end, rows.len());
                    if calibration_scenario_count == 0 || evaluation_scenario_count == 0 {
                        continue;
                    }

                    if !support.split_has_required_support(0, train_end, 1, 0, 2)
                        || !support.split_has_required_support(train_end, calibration_end, 1, 0, 0)
                        || !support.split_has_required_support(calibration_end, rows.len(), 1, 0, 1)
                    {
                        continue;
                    }

                    let scenario_coverage =
                        calibration_scenario_count.saturating_add(evaluation_scenario_count);
                    let early_warning_support_score = support
                        .early_warning_count(train_end, calibration_end)
                        .saturating_add(support.early_warning_count(calibration_end, rows.len()));
                    let gate_support_score = support
                        .gate_active_count(train_end, calibration_end)
                        .min(16)
                        .saturating_add(
                            support
                                .gate_active_count(calibration_end, rows.len())
                                .min(16),
                        );
                    let positive_support_score = support
                        .positive_count(train_end, calibration_end)
                        .saturating_add(support.positive_count(calibration_end, rows.len()));
                    let deviation_from_baseline = train_end.abs_diff(baseline_train_end)
                        + calibration_end.abs_diff(baseline_calibration_end);

                    let replace = match best_candidate {
                        None => true,
                        Some((
                            best_train_end,
                            best_calibration_end,
                            best_coverage,
                            best_early_score,
                            best_gate_score,
                            best_positive_score,
                            best_deviation,
                        )) => {
                            scenario_coverage > best_coverage
                                || (scenario_coverage == best_coverage
                                    && early_warning_support_score > best_early_score)
                                || (scenario_coverage == best_coverage
                                    && early_warning_support_score == best_early_score
                                    && gate_support_score > best_gate_score)
                                || (scenario_coverage == best_coverage
                                    && early_warning_support_score == best_early_score
                                    && gate_support_score == best_gate_score
                                    && positive_support_score > best_positive_score)
                                || (scenario_coverage == best_coverage
                                    && early_warning_support_score == best_early_score
                                    && gate_support_score == best_gate_score
                                    && positive_support_score == best_positive_score
                                    && deviation_from_baseline < best_deviation)
                                || (scenario_coverage == best_coverage
                                    && early_warning_support_score == best_early_score
                                    && gate_support_score == best_gate_score
                                    && positive_support_score == best_positive_score
                                    && deviation_from_baseline == best_deviation
                                    && (train_end > best_train_end
                                        || (train_end == best_train_end
                                            && calibration_end > best_calibration_end)))
                        }
                    };

                    if replace {
                        best_candidate = Some((
                            train_end,
                            calibration_end,
                            scenario_coverage,
                            early_warning_support_score,
                            gate_support_score,
                            positive_support_score,
                            deviation_from_baseline,
                        ));
                    }
                }
            }
        }
    }

    best_candidate
        .map(|(train_end, calibration_end, _, _, _, _, _)| (train_end, calibration_end))
        .ok_or_else(|| {
            anyhow::anyhow!("no family-aware overlay split satisfied support constraints")
        })
}

fn collect_family_overlay_scenario_ranges(
    rows: &[crate::ProbabilityTrainingRow],
    spec: &FamilyOverlayAuditSpec,
) -> Vec<crate::ScenarioRowRange> {
    let mut ranges = BTreeMap::<String, (usize, usize, String)>::new();
    for (index, row) in rows.iter().enumerate() {
        if !family_overlay_candidate_row(row, spec) {
            continue;
        }
        let Some(scenario_id) = row.primary_scenario_id.as_ref() else {
            continue;
        };
        let family = row
            .scenario_family
            .clone()
            .or_else(|| spec.scenario_family.map(str::to_string))
            .unwrap_or_else(|| "unknown".to_string());
        ranges
            .entry(scenario_id.clone())
            .and_modify(|range| range.1 = index)
            .or_insert((index, index, family));
    }

    let mut summaries = ranges
        .into_iter()
        .map(
            |(scenario_id, (start_index, end_index, family))| crate::ScenarioRowRange {
                scenario_id,
                family,
                start_index,
                end_index,
            },
        )
        .collect::<Vec<_>>();
    summaries.sort_by_key(|range| range.start_index);
    summaries
}

fn family_overlay_split_boundaries_within_range(range: &crate::ScenarioRowRange) -> Vec<usize> {
    ((range.start_index + 1)..=range.end_index.saturating_add(1)).collect()
}

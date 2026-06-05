use std::collections::{BTreeMap, BTreeSet};

use super::super::{ReleaseReviewFailureModeSummary, ReleaseReviewScenarioFocusDiagnostic};

pub(crate) fn summarize_release_review_failure_modes(
    scenarios: &[ReleaseReviewScenarioFocusDiagnostic],
) -> Vec<ReleaseReviewFailureModeSummary> {
    let mut map = BTreeMap::<String, (BTreeSet<String>, BTreeSet<String>)>::new();
    for scenario in scenarios {
        if let Some(failure_mode) = scenario.baseline_primary_failure_mode.as_ref() {
            map.entry(failure_mode.clone())
                .or_default()
                .0
                .insert(scenario.name.clone());
        }
        if let Some(failure_mode) = scenario.candidate_primary_failure_mode.as_ref() {
            map.entry(failure_mode.clone())
                .or_default()
                .1
                .insert(scenario.name.clone());
        }
    }

    let mut rows = map
        .into_iter()
        .map(
            |(failure_mode, (baseline_scenarios, candidate_scenarios))| {
                ReleaseReviewFailureModeSummary {
                    failure_mode,
                    baseline_count: baseline_scenarios.len() as u32,
                    candidate_count: candidate_scenarios.len() as u32,
                    baseline_scenarios: baseline_scenarios.into_iter().collect(),
                    candidate_scenarios: candidate_scenarios.into_iter().collect(),
                }
            },
        )
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .baseline_count
            .max(right.candidate_count)
            .cmp(&left.baseline_count.max(left.candidate_count))
            .then_with(|| left.failure_mode.cmp(&right.failure_mode))
    });
    rows
}

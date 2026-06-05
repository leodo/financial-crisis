use std::collections::{BTreeMap, BTreeSet};

use fc_domain::AssessmentHistoryPoint;

use super::facets::release_review_runtime_continuity_facets;
use super::gating::release_review_runtime_actionable_block_category;

pub(in super::super) fn release_review_runtime_block_counts(
    baseline_points: &[&AssessmentHistoryPoint],
    baseline_use_transitional_bridge: bool,
    baseline_thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
    candidate_points: &[&AssessmentHistoryPoint],
    candidate_use_transitional_bridge: bool,
    candidate_thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> Vec<crate::ReleaseReviewRuntimeBlockCount> {
    let baseline_counts = collect_block_counts(
        baseline_points,
        baseline_use_transitional_bridge,
        baseline_thresholds,
    );
    let candidate_counts = collect_block_counts(
        candidate_points,
        candidate_use_transitional_bridge,
        candidate_thresholds,
    );
    render_count_rows(baseline_counts, candidate_counts)
}

pub(in super::super) fn release_review_runtime_dominant_categories(
    counts: &[crate::ReleaseReviewRuntimeBlockCount],
) -> crate::ReleaseReviewRuntimeDominantCategories {
    let baseline_count = counts
        .iter()
        .map(|row| row.baseline_count)
        .max()
        .unwrap_or(0);
    let candidate_count = counts
        .iter()
        .map(|row| row.candidate_count)
        .max()
        .unwrap_or(0);

    crate::ReleaseReviewRuntimeDominantCategories {
        baseline_categories: if baseline_count == 0 {
            Vec::new()
        } else {
            counts
                .iter()
                .filter(|row| row.baseline_count == baseline_count)
                .map(|row| row.category.clone())
                .collect()
        },
        baseline_count,
        candidate_categories: if candidate_count == 0 {
            Vec::new()
        } else {
            counts
                .iter()
                .filter(|row| row.candidate_count == candidate_count)
                .map(|row| row.category.clone())
                .collect()
        },
        candidate_count,
    }
}

pub(in super::super) fn release_review_runtime_continuity_facet_counts(
    baseline_points: &[&AssessmentHistoryPoint],
    baseline_use_transitional_bridge: bool,
    baseline_thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
    candidate_points: &[&AssessmentHistoryPoint],
    candidate_use_transitional_bridge: bool,
    candidate_thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> Vec<crate::ReleaseReviewRuntimeBlockCount> {
    let baseline_counts = collect_facet_counts(
        baseline_points,
        baseline_use_transitional_bridge,
        baseline_thresholds,
    );
    let candidate_counts = collect_facet_counts(
        candidate_points,
        candidate_use_transitional_bridge,
        candidate_thresholds,
    );
    render_count_rows(baseline_counts, candidate_counts)
}

fn collect_block_counts(
    points: &[&AssessmentHistoryPoint],
    use_transitional_bridge: bool,
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> BTreeMap<String, u32> {
    points
        .iter()
        .fold(BTreeMap::<String, u32>::new(), |mut acc, point| {
            if let Some(category) = release_review_runtime_actionable_block_category(
                point,
                use_transitional_bridge,
                thresholds,
            ) {
                *acc.entry(category.to_string()).or_default() += 1;
            }
            acc
        })
}

fn collect_facet_counts(
    points: &[&AssessmentHistoryPoint],
    use_transitional_bridge: bool,
    thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> BTreeMap<String, u32> {
    points
        .iter()
        .fold(BTreeMap::<String, u32>::new(), |mut acc, point| {
            for facet in
                release_review_runtime_continuity_facets(point, use_transitional_bridge, thresholds)
            {
                *acc.entry(facet).or_default() += 1;
            }
            acc
        })
}

fn render_count_rows(
    baseline_counts: BTreeMap<String, u32>,
    candidate_counts: BTreeMap<String, u32>,
) -> Vec<crate::ReleaseReviewRuntimeBlockCount> {
    let categories = baseline_counts
        .keys()
        .chain(candidate_counts.keys())
        .cloned()
        .collect::<BTreeSet<_>>();

    categories
        .into_iter()
        .map(|category| {
            let baseline_count = baseline_counts.get(&category).copied().unwrap_or_default();
            let candidate_count = candidate_counts.get(&category).copied().unwrap_or_default();
            crate::ReleaseReviewRuntimeBlockCount {
                category,
                baseline_count,
                candidate_count,
                delta: i64::from(candidate_count) - i64::from(baseline_count),
            }
        })
        .collect()
}

use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;
use fc_domain::AssessmentHistoryPoint;

use super::super::{
    blocks::{
        release_review_actionable_diagnostic, release_review_posture_name,
        release_review_runtime_actionable_block_category,
        release_review_runtime_actionable_block_reason, release_review_time_bucket_name,
    },
    signals::{release_review_hits_runtime_floor, release_review_is_actionable_warning_point},
};
use super::window::PreparedScenarioWindow;

pub(super) fn build_interesting_points(
    prepared: &PreparedScenarioWindow<'_>,
    baseline_points_by_date: &BTreeMap<NaiveDate, &AssessmentHistoryPoint>,
    candidate_points_by_date: &BTreeMap<NaiveDate, &AssessmentHistoryPoint>,
    baseline_use_transitional_bridge: bool,
    candidate_use_transitional_bridge: bool,
    baseline_runtime_thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
    candidate_runtime_thresholds: Option<&crate::RuntimeThresholdDiagnosticsWire>,
) -> Vec<crate::ReleaseReviewScenarioPointComparison> {
    collect_interesting_dates(
        prepared,
        baseline_points_by_date,
        candidate_points_by_date,
        baseline_use_transitional_bridge,
        candidate_use_transitional_bridge,
    )
    .into_iter()
    .filter_map(|date| {
        let baseline_point = baseline_points_by_date.get(&date).copied();
        let candidate_point = candidate_points_by_date.get(&date).copied();
        if baseline_point.is_none() && candidate_point.is_none() {
            return None;
        }
        let baseline_strict_review_actionable = baseline_point.is_some_and(|point| {
            release_review_is_actionable_warning_point(point, baseline_use_transitional_bridge)
        });
        let candidate_strict_review_actionable = candidate_point.is_some_and(|point| {
            release_review_is_actionable_warning_point(point, candidate_use_transitional_bridge)
        });
        let baseline_runtime_floor_hit = baseline_point.is_some_and(|point| {
            release_review_hits_runtime_floor(point, baseline_runtime_thresholds)
        });
        let candidate_runtime_floor_hit = candidate_point.is_some_and(|point| {
            release_review_hits_runtime_floor(point, candidate_runtime_thresholds)
        });
        Some(crate::ReleaseReviewScenarioPointComparison {
            as_of_date: date,
            baseline_p20d: baseline_point.map(|point| point.p_20d),
            candidate_p20d: candidate_point.map(|point| point.p_20d),
            baseline_p60d: baseline_point.map(|point| point.p_60d),
            candidate_p60d: candidate_point.map(|point| point.p_60d),
            baseline_posture: baseline_point
                .map(release_review_posture_name)
                .map(str::to_string),
            candidate_posture: candidate_point
                .map(release_review_posture_name)
                .map(str::to_string),
            baseline_time_bucket: baseline_point
                .map(release_review_time_bucket_name)
                .map(str::to_string),
            candidate_time_bucket: candidate_point
                .map(release_review_time_bucket_name)
                .map(str::to_string),
            baseline_strict_review_actionable,
            candidate_strict_review_actionable,
            baseline_runtime_floor_hit,
            candidate_runtime_floor_hit,
            baseline_actionable: baseline_strict_review_actionable,
            candidate_actionable: candidate_strict_review_actionable,
            baseline_actionable_forward_5d_hits: prepared
                .baseline_actionable_hits
                .get(&date)
                .map(|(hit_count, _)| *hit_count),
            candidate_actionable_forward_5d_hits: prepared
                .candidate_actionable_hits
                .get(&date)
                .map(|(hit_count, _)| *hit_count),
            baseline_actionable_sustained: prepared
                .baseline_actionable_hits
                .get(&date)
                .map(|(_, sustained)| *sustained),
            candidate_actionable_sustained: prepared
                .candidate_actionable_hits
                .get(&date)
                .map(|(_, sustained)| *sustained),
            baseline_trigger_codes: baseline_point
                .map(|point| point.posture_trigger_codes.clone())
                .unwrap_or_default(),
            candidate_trigger_codes: candidate_point
                .map(|point| point.posture_trigger_codes.clone())
                .unwrap_or_default(),
            baseline_runtime_actionable_block_category: baseline_point.and_then(|point| {
                release_review_runtime_actionable_block_category(
                    point,
                    baseline_use_transitional_bridge,
                    baseline_runtime_thresholds,
                )
                .map(str::to_string)
            }),
            candidate_runtime_actionable_block_category: candidate_point.and_then(|point| {
                release_review_runtime_actionable_block_category(
                    point,
                    candidate_use_transitional_bridge,
                    candidate_runtime_thresholds,
                )
                .map(str::to_string)
            }),
            baseline_runtime_actionable_block_reason: baseline_point.and_then(|point| {
                release_review_runtime_actionable_block_reason(
                    point,
                    baseline_use_transitional_bridge,
                    baseline_runtime_thresholds,
                )
            }),
            candidate_runtime_actionable_block_reason: candidate_point.and_then(|point| {
                release_review_runtime_actionable_block_reason(
                    point,
                    candidate_use_transitional_bridge,
                    candidate_runtime_thresholds,
                )
            }),
            baseline_actionable_diagnostic: baseline_point.map(|point| {
                release_review_actionable_diagnostic(
                    point,
                    baseline_use_transitional_bridge,
                    baseline_runtime_thresholds,
                )
            }),
            candidate_actionable_diagnostic: candidate_point.map(|point| {
                release_review_actionable_diagnostic(
                    point,
                    candidate_use_transitional_bridge,
                    candidate_runtime_thresholds,
                )
            }),
        })
    })
    .collect()
}

fn collect_interesting_dates(
    prepared: &PreparedScenarioWindow<'_>,
    baseline_points_by_date: &BTreeMap<NaiveDate, &AssessmentHistoryPoint>,
    candidate_points_by_date: &BTreeMap<NaiveDate, &AssessmentHistoryPoint>,
    baseline_use_transitional_bridge: bool,
    candidate_use_transitional_bridge: bool,
) -> BTreeSet<NaiveDate> {
    let mut interesting_dates = BTreeSet::new();
    for date in [
        Some(prepared.baseline.crisis_start),
        Some(prepared.baseline.crisis_end),
        prepared.baseline.first_l2_date,
        prepared
            .candidate
            .and_then(|scenario| scenario.first_l2_date),
        prepared.baseline.first_l3_date,
        prepared
            .candidate
            .and_then(|scenario| scenario.first_l3_date),
        prepared.baseline_first_non_normal_date,
        prepared.candidate_first_non_normal_date,
        prepared
            .baseline_first_runtime_floor_hit_without_l3
            .as_ref()
            .map(|(date, _)| *date),
        prepared
            .candidate_first_runtime_floor_hit_without_l3
            .as_ref()
            .map(|(date, _)| *date),
    ]
    .into_iter()
    .flatten()
    {
        if date >= prepared.window_start && date <= prepared.window_end {
            interesting_dates.insert(date);
        }
    }

    for date in prepared
        .baseline_window_points
        .iter()
        .map(|point| point.as_of_date)
        .chain(
            prepared
                .candidate_window_points
                .iter()
                .map(|point| point.as_of_date),
        )
    {
        let baseline_point = baseline_points_by_date.get(&date).copied();
        let candidate_point = candidate_points_by_date.get(&date).copied();
        if release_review_point_is_interesting(
            baseline_point,
            candidate_point,
            baseline_use_transitional_bridge,
            candidate_use_transitional_bridge,
        ) {
            interesting_dates.insert(date);
        }
    }

    interesting_dates
}

pub(super) fn release_review_point_is_non_normal(point: &AssessmentHistoryPoint) -> bool {
    !matches!(point.posture, fc_domain::DecisionPosture::Normal)
        || !matches!(
            point.time_to_risk_bucket,
            fc_domain::TimeToRiskBucket::Normal
        )
}

fn release_review_point_is_interesting(
    baseline_point: Option<&AssessmentHistoryPoint>,
    candidate_point: Option<&AssessmentHistoryPoint>,
    baseline_use_transitional_bridge: bool,
    candidate_use_transitional_bridge: bool,
) -> bool {
    let baseline_actionable = baseline_point.is_some_and(|point| {
        release_review_is_actionable_warning_point(point, baseline_use_transitional_bridge)
    });
    let candidate_actionable = candidate_point.is_some_and(|point| {
        release_review_is_actionable_warning_point(point, candidate_use_transitional_bridge)
    });
    if baseline_actionable
        || candidate_actionable
        || baseline_point.is_some_and(release_review_point_is_non_normal)
        || candidate_point.is_some_and(release_review_point_is_non_normal)
    {
        return true;
    }

    match (baseline_point, candidate_point) {
        (Some(baseline), Some(candidate)) => {
            baseline.posture != candidate.posture
                || baseline.time_to_risk_bucket != candidate.time_to_risk_bucket
                || baseline.posture_trigger_codes != candidate.posture_trigger_codes
                || (baseline.p_20d - candidate.p_20d).abs() >= 0.05
                || (baseline.p_60d - candidate.p_60d).abs() >= 0.05
        }
        _ => false,
    }
}

use std::collections::{BTreeMap, BTreeSet};

use super::super::{
    ReleaseReviewHistoricalAuditActionSummary, ReleaseReviewHistoricalAuditAttributionSummary,
    ReleaseReviewHistoricalAuditPriority,
};
use super::{
    release_review_failure_mode_matches_workstream, release_review_historical_action_priority,
    release_review_historical_attribution_priority,
    release_review_historical_audit_action_recommendation,
    release_review_historical_audit_action_type,
    release_review_historical_audit_attribution_explanation,
    release_review_historical_audit_attribution_label,
    release_review_historical_workstream_priority,
};

pub(crate) fn summarize_release_review_historical_audit_attribution(
    priorities: &[ReleaseReviewHistoricalAuditPriority],
) -> Vec<ReleaseReviewHistoricalAuditAttributionSummary> {
    let mut rows = BTreeMap::<
        (String, String),
        (
            BTreeSet<String>,
            u32,
            u32,
            u32,
            BTreeSet<String>,
            BTreeSet<String>,
        ),
    >::new();
    for priority in priorities {
        let baseline_matches = release_review_failure_mode_matches_workstream(
            &priority.baseline_failure_mode,
            &priority.primary_workstream,
        );
        let candidate_matches = release_review_failure_mode_matches_workstream(
            &priority.candidate_failure_mode,
            &priority.primary_workstream,
        );
        let attribution = release_review_historical_audit_attribution_label(
            &priority.baseline_failure_mode,
            baseline_matches,
            candidate_matches,
            &priority.outcome,
            priority.baseline_runtime_floor_hit_point_count,
            priority.candidate_runtime_floor_hit_point_count,
        );
        let entry = rows
            .entry((priority.primary_workstream.clone(), attribution.to_string()))
            .or_insert_with(|| (BTreeSet::new(), 0, 0, 0, BTreeSet::new(), BTreeSet::new()));
        entry.0.insert(priority.scenario_name.clone());
        if priority.protected_window {
            entry.1 += 1;
        }
        if baseline_matches {
            entry.2 += 1;
            entry.4.insert(priority.scenario_name.clone());
        }
        if candidate_matches {
            entry.3 += 1;
            entry.5.insert(priority.scenario_name.clone());
        }
    }

    let mut rows = rows
        .into_iter()
        .map(
            |(
                (workstream, attribution),
                (
                    scenarios,
                    protected_count,
                    baseline_count,
                    candidate_count,
                    baseline_scenarios,
                    candidate_scenarios,
                ),
            )| {
                let scenario_count = scenarios.len() as u32;
                let scenario_names = scenarios.into_iter().collect::<Vec<_>>();
                ReleaseReviewHistoricalAuditAttributionSummary {
                    explanation: release_review_historical_audit_attribution_explanation(
                        &workstream,
                        &attribution,
                        scenario_count,
                        &scenario_names,
                    ),
                    workstream,
                    attribution,
                    scenario_count,
                    protected_count,
                    baseline_count,
                    candidate_count,
                    baseline_scenarios: baseline_scenarios.into_iter().collect(),
                    candidate_scenarios: candidate_scenarios.into_iter().collect(),
                }
            },
        )
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        release_review_historical_workstream_priority(&left.workstream)
            .cmp(&release_review_historical_workstream_priority(
                &right.workstream,
            ))
            .then_with(|| {
                release_review_historical_attribution_priority(&left.attribution).cmp(
                    &release_review_historical_attribution_priority(&right.attribution),
                )
            })
            .then_with(|| right.scenario_count.cmp(&left.scenario_count))
            .then_with(|| left.workstream.cmp(&right.workstream))
    });
    rows
}

pub(crate) fn summarize_release_review_historical_audit_actions(
    rows: &[ReleaseReviewHistoricalAuditAttributionSummary],
) -> Vec<ReleaseReviewHistoricalAuditActionSummary> {
    let mut actions = rows
        .iter()
        .map(|row| ReleaseReviewHistoricalAuditActionSummary {
            workstream: row.workstream.clone(),
            attribution: row.attribution.clone(),
            action_type: release_review_historical_audit_action_type(&row.attribution).to_string(),
            scenario_count: row.scenario_count,
            protected_count: row.protected_count,
            recommendation: release_review_historical_audit_action_recommendation(
                &row.workstream,
                &row.attribution,
                row.scenario_count,
            ),
        })
        .collect::<Vec<_>>();
    actions.sort_by(|left, right| {
        release_review_historical_workstream_priority(&left.workstream)
            .cmp(&release_review_historical_workstream_priority(
                &right.workstream,
            ))
            .then_with(|| {
                release_review_historical_action_priority(&left.action_type).cmp(
                    &release_review_historical_action_priority(&right.action_type),
                )
            })
            .then_with(|| right.scenario_count.cmp(&left.scenario_count))
            .then_with(|| left.workstream.cmp(&right.workstream))
    });
    actions
}

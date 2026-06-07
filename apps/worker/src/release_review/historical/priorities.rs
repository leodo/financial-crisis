use std::collections::BTreeMap;

use fc_domain::{load_crisis_scenario_catalog, CrisisScenarioTrainingRole};

use super::super::{ReleaseReviewHistoricalAuditPriority, ReleaseReviewScenarioFocusDiagnostic};
use super::{
    release_review_gate_gap_profile_for_scenario, release_review_historical_workstream_priority,
    release_review_primary_workstream, release_review_scenario_family_name,
    release_review_scenario_training_role_name, release_review_suggested_historical_audit,
};

pub(crate) fn summarize_release_review_historical_audit_priorities(
    scenarios: &[ReleaseReviewScenarioFocusDiagnostic],
) -> Vec<ReleaseReviewHistoricalAuditPriority> {
    let catalog = load_crisis_scenario_catalog();
    let scenarios_by_id = catalog
        .scenarios
        .iter()
        .map(|scenario| (scenario.scenario_id.as_str(), scenario))
        .collect::<BTreeMap<_, _>>();

    let mut rows = scenarios
        .iter()
        .filter_map(|scenario| {
            let definition = scenarios_by_id
                .get(scenario.scenario_id.as_str())
                .copied()?;
            if !definition.protected_window
                && definition.training_role == CrisisScenarioTrainingRole::Mandatory
            {
                return None;
            }

            let baseline_failure_mode = scenario
                .baseline_primary_failure_mode
                .clone()
                .unwrap_or_else(|| "unclassified".to_string());
            let candidate_failure_mode = scenario
                .candidate_primary_failure_mode
                .clone()
                .unwrap_or_else(|| "—".to_string());
            let primary_workstream = release_review_primary_workstream(
                scenario.baseline_primary_failure_mode.as_deref(),
                scenario.candidate_primary_failure_mode.as_deref(),
            )
            .to_string();
            let (baseline_gate_gap_profile, candidate_gate_gap_profile) =
                if primary_workstream == "strict_review_vs_runtime_mapping" {
                    (
                        release_review_gate_gap_profile_for_scenario(scenario, false),
                        release_review_gate_gap_profile_for_scenario(scenario, true),
                    )
                } else {
                    (None, None)
                };

            Some(ReleaseReviewHistoricalAuditPriority {
                scenario_id: scenario.scenario_id.clone(),
                scenario_name: scenario.name.clone(),
                outcome: scenario.outcome.clone(),
                scenario_family: release_review_scenario_family_name(definition.family).to_string(),
                training_role: release_review_scenario_training_role_name(definition.training_role)
                    .to_string(),
                protected_window: definition.protected_window,
                baseline_failure_mode,
                candidate_failure_mode,
                baseline_actionable_point_count: scenario.baseline_actionable_point_count,
                candidate_actionable_point_count: scenario.candidate_actionable_point_count,
                baseline_runtime_floor_hit_point_count: scenario
                    .baseline_runtime_floor_hit_point_count,
                candidate_runtime_floor_hit_point_count: scenario
                    .candidate_runtime_floor_hit_point_count,
                baseline_gate_gap_profile,
                candidate_gate_gap_profile,
                primary_workstream: primary_workstream.clone(),
                suggested_review: release_review_suggested_historical_audit(
                    definition,
                    scenario,
                    &primary_workstream,
                )
                .to_string(),
            })
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        release_review_historical_workstream_priority(&left.primary_workstream)
            .cmp(&release_review_historical_workstream_priority(
                &right.primary_workstream,
            ))
            .then_with(|| left.scenario_id.cmp(&right.scenario_id))
    });
    rows
}

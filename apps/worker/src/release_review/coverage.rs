use std::collections::{BTreeMap, BTreeSet};

use fc_domain::{load_crisis_scenario_catalog, load_scenario_data_coverage_catalog};

pub(crate) fn build_release_review_scenario_coverage(
    backtest_scenarios: &[crate::ReleaseReviewBacktestScenarioComparison],
    focus_scenarios: &[crate::ReleaseReviewScenarioFocusDiagnostic],
) -> (
    crate::ReleaseReviewScenarioCoverageCatalogSummary,
    Vec<crate::ReleaseReviewScenarioCoverage>,
) {
    let crisis_catalog = load_crisis_scenario_catalog();
    let coverage_catalog = load_scenario_data_coverage_catalog();
    let backtests_by_id = backtest_scenarios
        .iter()
        .map(|scenario| (scenario.scenario_id.as_str(), scenario))
        .collect::<BTreeMap<_, _>>();
    let focus_ids = focus_scenarios
        .iter()
        .map(|scenario| scenario.scenario_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut scenario_ids = backtest_scenarios
        .iter()
        .map(|scenario| scenario.scenario_id.clone())
        .collect::<BTreeSet<_>>();
    scenario_ids.extend(
        focus_scenarios
            .iter()
            .map(|scenario| scenario.scenario_id.clone()),
    );

    let rows = scenario_ids
        .into_iter()
        .filter_map(|scenario_id| {
            let scenario_definition = crisis_catalog
                .scenarios
                .iter()
                .find(|scenario| scenario.scenario_id == scenario_id)?;
            let coverage = coverage_catalog.record_for_scenario(&scenario_id)?;
            let backtest = backtests_by_id.get(scenario_id.as_str()).copied();
            Some(crate::ReleaseReviewScenarioCoverage {
                scenario_id: scenario_id.clone(),
                scenario_name: backtest
                    .map(|scenario| scenario.name.clone())
                    .unwrap_or_else(|| scenario_definition.label.clone()),
                scenario_family: scenario_family_code(scenario_definition.family).to_string(),
                training_role: scenario_training_role_code(scenario_definition.training_role)
                    .to_string(),
                protected_window: scenario_definition.protected_window,
                in_backtest_comparison: backtest.is_some(),
                in_focus_review: focus_ids.contains(scenario_id.as_str()),
                recommended_role: coverage.recommended_role.clone(),
                coverage_grade: coverage.coverage_grade.clone(),
                point_in_time_mode: coverage.point_in_time_mode.clone(),
                current_status: coverage.current_status.clone(),
                blocking_gaps: coverage.blocking_gaps.clone(),
                free_sources: coverage.free_sources.clone(),
                usable_for_main_training: coverage.usable_for_main_training,
                usable_for_extension_training: coverage.usable_for_extension_training,
                usable_for_protected_stress: coverage.usable_for_protected_stress,
                usable_for_historical_analog: coverage.usable_for_historical_analog,
            })
        })
        .collect::<Vec<_>>();

    let backtest_ids = backtest_scenarios
        .iter()
        .map(|scenario| scenario.scenario_id.as_str())
        .collect::<BTreeSet<_>>();
    let covered_backtest_scenario_count = rows
        .iter()
        .filter(|row| backtest_ids.contains(row.scenario_id.as_str()))
        .count();
    let covered_focus_scenario_count = rows.iter().filter(|row| row.in_focus_review).count();

    (
        crate::ReleaseReviewScenarioCoverageCatalogSummary {
            catalog_id: coverage_catalog.catalog_id,
            scenario_catalog_id: coverage_catalog.scenario_catalog_id,
            market_scope: coverage_catalog.market_scope,
            source: coverage_catalog.source,
            warning: coverage_catalog.warning,
            backtest_scenario_count: backtest_scenarios.len(),
            covered_backtest_scenario_count,
            focus_scenario_count: focus_scenarios.len(),
            covered_focus_scenario_count,
            main_training_eligible_count: rows
                .iter()
                .filter(|row| row.usable_for_main_training)
                .count(),
            extension_training_eligible_count: rows
                .iter()
                .filter(|row| row.usable_for_extension_training)
                .count(),
            protected_stress_eligible_count: rows
                .iter()
                .filter(|row| row.usable_for_protected_stress)
                .count(),
            historical_analog_eligible_count: rows
                .iter()
                .filter(|row| row.usable_for_historical_analog)
                .count(),
        },
        rows,
    )
}

fn scenario_family_code(family: fc_domain::CrisisScenarioFamily) -> &'static str {
    match family {
        fc_domain::CrisisScenarioFamily::AcuteMarketLiquidityCrash => {
            "acute_market_liquidity_crash"
        }
        fc_domain::CrisisScenarioFamily::SystemicCreditBankingCrisis => {
            "systemic_credit_banking_crisis"
        }
        fc_domain::CrisisScenarioFamily::MixedSystemicStress => "mixed_systemic_stress",
        fc_domain::CrisisScenarioFamily::RateShockOrPolicyDislocation => {
            "rate_shock_or_policy_dislocation"
        }
    }
}

fn scenario_training_role_code(role: fc_domain::CrisisScenarioTrainingRole) -> &'static str {
    match role {
        fc_domain::CrisisScenarioTrainingRole::Mandatory => "mandatory",
        fc_domain::CrisisScenarioTrainingRole::CandidateOptional => "candidate_optional",
        fc_domain::CrisisScenarioTrainingRole::ExtensionOnly => "extension_only",
        fc_domain::CrisisScenarioTrainingRole::NoPositiveMain => "no_positive_main",
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    #[test]
    fn build_release_review_scenario_coverage_marks_focus_and_backtest_counts() {
        let backtests = vec![
            crate::ReleaseReviewBacktestScenarioComparison {
                scenario_id: "us_gfc_2008".to_string(),
                name: "2007-2009 全球金融危机".to_string(),
                signal_source: "real_history".to_string(),
                crisis_start: NaiveDate::from_ymd_opt(2008, 9, 15).unwrap(),
                crisis_end: NaiveDate::from_ymd_opt(2009, 3, 9).unwrap(),
                baseline_first_l2_date: None,
                candidate_first_l2_date: None,
                baseline_first_l3_date: None,
                candidate_first_l3_date: None,
                baseline_lead_time_days: None,
                candidate_lead_time_days: None,
                baseline_actionable_lead_time_days: None,
                candidate_actionable_lead_time_days: None,
                baseline_false_positive_count: 0,
                candidate_false_positive_count: 0,
                actionable_delta_days: None,
                outcome: "missed_to_missed".to_string(),
            },
            crate::ReleaseReviewBacktestScenarioComparison {
                scenario_id: "us_bond_massacre_1994".to_string(),
                name: "1994 联储加息与债市暴跌".to_string(),
                signal_source: "real_history".to_string(),
                crisis_start: NaiveDate::from_ymd_opt(1994, 2, 1).unwrap(),
                crisis_end: NaiveDate::from_ymd_opt(1994, 12, 1).unwrap(),
                baseline_first_l2_date: None,
                candidate_first_l2_date: None,
                baseline_first_l3_date: None,
                candidate_first_l3_date: None,
                baseline_lead_time_days: None,
                candidate_lead_time_days: None,
                baseline_actionable_lead_time_days: None,
                candidate_actionable_lead_time_days: None,
                baseline_false_positive_count: 0,
                candidate_false_positive_count: 0,
                actionable_delta_days: None,
                outcome: "missed_to_missed".to_string(),
            },
        ];
        let focus = vec![crate::ReleaseReviewScenarioFocusDiagnostic {
            scenario_id: "us_bond_massacre_1994".to_string(),
            name: "1994 联储加息与债市暴跌".to_string(),
            outcome: "late_only_to_missed".to_string(),
            window_start: NaiveDate::from_ymd_opt(1993, 10, 1).unwrap(),
            window_end: NaiveDate::from_ymd_opt(1995, 3, 17).unwrap(),
            crisis_start: NaiveDate::from_ymd_opt(1994, 2, 1).unwrap(),
            crisis_end: NaiveDate::from_ymd_opt(1994, 12, 1).unwrap(),
            baseline_first_l2_date: None,
            candidate_first_l2_date: None,
            baseline_first_l3_date: None,
            candidate_first_l3_date: None,
            baseline_first_non_normal_date: None,
            candidate_first_non_normal_date: None,
            baseline_actionable_point_count: 0,
            candidate_actionable_point_count: 0,
            baseline_runtime_floor_hit_point_count: 0,
            candidate_runtime_floor_hit_point_count: 0,
            baseline_max_p20d: None,
            candidate_max_p20d: None,
            baseline_max_p60d: None,
            candidate_max_p60d: None,
            baseline_first_runtime_floor_hit_without_l3_date: None,
            candidate_first_runtime_floor_hit_without_l3_date: None,
            baseline_first_runtime_floor_hit_without_l3_reason: None,
            candidate_first_runtime_floor_hit_without_l3_reason: None,
            baseline_primary_failure_mode: None,
            candidate_primary_failure_mode: None,
            dominant_runtime_blocks: crate::ReleaseReviewRuntimeDominantCategories {
                baseline_categories: Vec::new(),
                baseline_count: 0,
                candidate_categories: Vec::new(),
                candidate_count: 0,
            },
            dominant_runtime_continuity_facets: crate::ReleaseReviewRuntimeDominantCategories {
                baseline_categories: Vec::new(),
                baseline_count: 0,
                candidate_categories: Vec::new(),
                candidate_count: 0,
            },
            runtime_block_counts: Vec::new(),
            runtime_continuity_facet_counts: Vec::new(),
            interesting_points: Vec::new(),
        }];

        let (catalog, rows) = crate::build_release_review_scenario_coverage(&backtests, &focus);

        assert_eq!(catalog.backtest_scenario_count, 2);
        assert_eq!(catalog.covered_backtest_scenario_count, 2);
        assert_eq!(catalog.focus_scenario_count, 1);
        assert_eq!(catalog.covered_focus_scenario_count, 1);
        assert_eq!(rows.len(), 2);
        assert!(rows
            .iter()
            .find(|row| row.scenario_id == "us_bond_massacre_1994")
            .is_some_and(|row| row.in_focus_review && row.usable_for_protected_stress));
        assert!(rows
            .iter()
            .find(|row| row.scenario_id == "us_gfc_2008")
            .is_some_and(|row| row.usable_for_main_training && !row.in_focus_review));
    }
}

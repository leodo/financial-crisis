use anyhow::{bail, Context};

#[derive(Debug, Clone)]
pub(crate) struct FormalDatasetScenarioSets {
    pub(crate) positive_scenarios: Vec<crate::CrisisScenario>,
    pub(crate) context_scenarios: Vec<crate::CrisisScenario>,
}

pub(crate) fn scenario_family_code(family: fc_domain::CrisisScenarioFamily) -> &'static str {
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

pub(crate) fn scenario_training_role_code(
    role: fc_domain::CrisisScenarioTrainingRole,
) -> &'static str {
    match role {
        fc_domain::CrisisScenarioTrainingRole::Mandatory => "mandatory",
        fc_domain::CrisisScenarioTrainingRole::CandidateOptional => "candidate_optional",
        fc_domain::CrisisScenarioTrainingRole::ExtensionOnly => "extension_only",
        fc_domain::CrisisScenarioTrainingRole::NoPositiveMain => "no_positive_main",
    }
}

pub(crate) fn action_episode_template_code(
    template: fc_domain::ActionEpisodeTemplateId,
) -> &'static str {
    match template {
        fc_domain::ActionEpisodeTemplateId::AcuteMarketLiquidityCrash => {
            "acute_market_liquidity_crash"
        }
        fc_domain::ActionEpisodeTemplateId::SystemicCreditBankingCrisis => {
            "systemic_credit_banking_crisis"
        }
        fc_domain::ActionEpisodeTemplateId::MixedSystemicStress => "mixed_systemic_stress",
        fc_domain::ActionEpisodeTemplateId::RateShockOrPolicyDislocation => {
            "rate_shock_or_policy_dislocation"
        }
    }
}

pub(crate) fn load_label_set_crisis_scenarios(
    scenario_set_version: &str,
    label_set_id: &str,
) -> anyhow::Result<Vec<crate::CrisisScenario>> {
    let catalog = crate::load_crisis_scenario_catalog();
    if catalog.catalog_id != scenario_set_version {
        bail!(
            "scenario set version {} is not available in the active catalog {}; set FC_SCENARIO_CATALOG_PATH to another catalog or use {}",
            scenario_set_version,
            catalog.catalog_id,
            catalog.catalog_id
        );
    }

    load_label_set_crisis_scenarios_from_catalog(&catalog, label_set_id)
}

pub(crate) fn load_formal_dataset_scenario_sets(
    scenario_set_version: &str,
    label_set_id: &str,
) -> anyhow::Result<FormalDatasetScenarioSets> {
    let catalog = crate::load_crisis_scenario_catalog();
    if catalog.catalog_id != scenario_set_version {
        bail!(
            "scenario set version {} is not available in the active catalog {}; set FC_SCENARIO_CATALOG_PATH to another catalog or use {}",
            scenario_set_version,
            catalog.catalog_id,
            catalog.catalog_id
        );
    }

    let positive_scenarios = load_label_set_crisis_scenarios_from_catalog(&catalog, label_set_id)?;
    let mut context_scenarios = positive_scenarios.clone();
    if label_set_id == crate::DEFAULT_FORMAL_LABEL_VERSION {
        let protected_context_scenarios = load_window_set_crisis_scenarios_from_catalog(
            &catalog,
            crate::DEFAULT_FORMAL_MAIN_CONTEXT_WINDOW_SET_ID,
        )?;
        for scenario in protected_context_scenarios {
            if context_scenarios
                .iter()
                .any(|existing| existing.scenario_id == scenario.scenario_id)
            {
                continue;
            }
            context_scenarios.push(scenario);
        }
        context_scenarios.sort_by_key(|scenario| scenario.crisis_start);
    }

    Ok(FormalDatasetScenarioSets {
        positive_scenarios,
        context_scenarios,
    })
}

fn load_label_set_crisis_scenarios_from_catalog(
    catalog: &fc_domain::CrisisScenarioCatalog,
    label_set_id: &str,
) -> anyhow::Result<Vec<crate::CrisisScenario>> {
    let scenarios = catalog
        .scenarios_for_label_set(label_set_id)
        .with_context(|| format!("label set {label_set_id} was not found in scenario catalog"))?;
    Ok(scenarios
        .into_iter()
        .map(crisis_scenario_from_definition)
        .collect())
}

fn load_window_set_crisis_scenarios_from_catalog(
    catalog: &fc_domain::CrisisScenarioCatalog,
    window_set_id: &str,
) -> anyhow::Result<Vec<crate::CrisisScenario>> {
    let scenario_ids = catalog
        .scenario_ids_for_window_set(window_set_id)
        .with_context(|| format!("window set {window_set_id} was not found in scenario catalog"))?;
    let mut scenarios = Vec::with_capacity(scenario_ids.len());
    for scenario_id in scenario_ids {
        let scenario = catalog
            .scenarios
            .iter()
            .find(|scenario| scenario.scenario_id == *scenario_id)
            .with_context(|| {
                format!("window set {window_set_id} references unknown scenario {scenario_id}")
            })?;
        scenarios.push(crisis_scenario_from_definition(scenario));
    }
    Ok(scenarios)
}

fn crisis_scenario_from_definition(
    scenario: &fc_domain::CrisisScenarioDefinition,
) -> crate::CrisisScenario {
    crate::CrisisScenario {
        scenario_id: scenario.scenario_id.clone(),
        family: scenario_family_code(scenario.family).to_string(),
        training_role: scenario_training_role_code(scenario.training_role).to_string(),
        pre_warning_start: scenario.pre_warning_start,
        crisis_start: scenario.crisis_start,
        acute_start: scenario.acute_start,
        crisis_end: scenario.crisis_end,
        default_horizon_roles: scenario.default_horizon_roles.clone(),
        protected_window: scenario.protected_window,
        protected_action_levels: scenario.protected_action_levels.clone(),
        episode_template_id: scenario
            .episode_template_id
            .expect("validated scenario catalog must include episode_template_id"),
        action_episode_overrides: scenario.action_episode_overrides.clone(),
    }
}

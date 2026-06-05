use crate::ActionabilityLevel;

use super::{embedded_crisis_scenario_catalog, CrisisScenarioTrainingRole};

#[test]
fn embedded_catalog_contains_main_and_extension_sets() {
    let catalog = embedded_crisis_scenario_catalog();
    assert_eq!(catalog.catalog_id, "scenario_v1_main");
    assert!(catalog
        .scenarios
        .iter()
        .any(|scenario| scenario.scenario_id == "us_black_monday_1987"));
    assert!(catalog
        .scenarios
        .iter()
        .any(|scenario| scenario.training_role == CrisisScenarioTrainingRole::Mandatory));
    let main = catalog
        .scenarios_for_label_set("formal_label_v1_main")
        .expect("main label set");
    assert_eq!(main.len(), 3);
    let ext = catalog
        .scenarios_for_label_set("formal_label_v1_ext_acute")
        .expect("extension label set");
    assert_eq!(ext.len(), 2);
    let protected_ext = catalog
        .scenarios_for_label_set("formal_label_v1_ext_stress")
        .expect("protected extension label set");
    assert_eq!(protected_ext.len(), 4);
    let protected = catalog
        .scenarios
        .iter()
        .find(|scenario| scenario.scenario_id == "us_funding_stress_2011")
        .expect("protected scenario");
    assert_eq!(
        protected.protected_action_levels,
        vec![ActionabilityLevel::Prepare, ActionabilityLevel::Hedge]
    );
    assert!(protected.episode_template_id.is_some());
}

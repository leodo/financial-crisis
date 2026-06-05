use std::collections::HashSet;

use super::{
    ActionEpisodeWindowOverride, CrisisScenarioDefinition, CrisisScenarioLabelSet,
    CrisisScenarioWindowSet,
};

pub(super) fn validate_scenarios(scenarios: &[CrisisScenarioDefinition]) -> Result<(), String> {
    if scenarios.is_empty() {
        return Err("危机场景目录不能为空，至少需要一个场景。".to_string());
    }

    let mut ids = HashSet::new();
    for scenario in scenarios {
        if !ids.insert(scenario.scenario_id.clone()) {
            return Err(format!("场景 {} 重复定义。", scenario.scenario_id));
        }
        if scenario.pre_warning_start > scenario.crisis_start {
            return Err(format!(
                "场景 {} 的 pre_warning_start 晚于 crisis_start。",
                scenario.scenario_id
            ));
        }
        if scenario.crisis_start > scenario.crisis_end {
            return Err(format!(
                "场景 {} 的 crisis_start 晚于 crisis_end。",
                scenario.scenario_id
            ));
        }
        if let Some(acute_start) = scenario.acute_start {
            if acute_start < scenario.crisis_start || acute_start > scenario.crisis_end {
                return Err(format!(
                    "场景 {} 的 acute_start 不在 crisis_start 与 crisis_end 之间。",
                    scenario.scenario_id
                ));
            }
        }
        if let Some(crisis_peak) = scenario.crisis_peak {
            if crisis_peak < scenario.crisis_start || crisis_peak > scenario.crisis_end {
                return Err(format!(
                    "场景 {} 的 crisis_peak 不在 crisis_start 与 crisis_end 之间。",
                    scenario.scenario_id
                ));
            }
        }
        if scenario.default_horizon_roles.is_empty() {
            return Err(format!(
                "场景 {} 的 default_horizon_roles 不能为空。",
                scenario.scenario_id
            ));
        }
        if scenario
            .default_horizon_roles
            .iter()
            .any(|role| !matches!(role, 5 | 20 | 60))
        {
            return Err(format!(
                "场景 {} 的 default_horizon_roles 只能包含 5、20、60。",
                scenario.scenario_id
            ));
        }
        if scenario.episode_template_id.is_none() {
            return Err(format!(
                "场景 {} 缺少 episode_template_id。",
                scenario.scenario_id
            ));
        }
        let mut protected_levels = HashSet::new();
        for level in &scenario.protected_action_levels {
            if !protected_levels.insert(*level) {
                return Err(format!(
                    "场景 {} 的 protected_action_levels 存在重复动作层级。",
                    scenario.scenario_id
                ));
            }
        }
        if let Some(overrides) = scenario.action_episode_overrides.as_ref() {
            validate_action_episode_override(
                &scenario.scenario_id,
                "prepare",
                overrides.prepare.as_ref(),
            )?;
            validate_action_episode_override(
                &scenario.scenario_id,
                "hedge",
                overrides.hedge.as_ref(),
            )?;
            validate_action_episode_override(
                &scenario.scenario_id,
                "defend",
                overrides.defend.as_ref(),
            )?;
        }
    }

    Ok(())
}

fn validate_action_episode_override(
    scenario_id: &str,
    level_name: &str,
    override_window: Option<&ActionEpisodeWindowOverride>,
) -> Result<(), String> {
    let Some(override_window) = override_window else {
        return Ok(());
    };

    if let (Some(primary_start), Some(primary_end)) =
        (override_window.primary_start, override_window.primary_end)
    {
        if primary_start > primary_end {
            return Err(format!(
                "场景 {scenario_id} 的 {level_name} action episode override 中 primary_start 晚于 primary_end。"
            ));
        }
    }

    if override_window.primary_start.is_some() ^ override_window.primary_end.is_some() {
        return Err(format!(
            "场景 {scenario_id} 的 {level_name} action episode override 必须同时提供 primary_start 和 primary_end。"
        ));
    }

    if let (Some(primary_end), Some(late_validation_end)) = (
        override_window.primary_end,
        override_window.late_validation_end,
    ) {
        if late_validation_end < primary_end {
            return Err(format!(
                "场景 {scenario_id} 的 {level_name} action episode override 中 late_validation_end 早于 primary_end。"
            ));
        }
    }

    if let (Some(late_validation_end), Some(cooldown_end)) = (
        override_window.late_validation_end,
        override_window.cooldown_end,
    ) {
        if cooldown_end < late_validation_end {
            return Err(format!(
                "场景 {scenario_id} 的 {level_name} action episode override 中 cooldown_end 早于 late_validation_end。"
            ));
        }
    }

    Ok(())
}

pub(super) fn validate_label_sets(
    label_sets: &[CrisisScenarioLabelSet],
    scenarios: &[CrisisScenarioDefinition],
) -> Result<(), String> {
    validate_scenario_refs(
        label_sets.iter().map(|label_set| {
            (
                label_set.label_set_id.as_str(),
                label_set.scenario_ids.as_slice(),
                "label_set",
            )
        }),
        scenarios,
    )
}

pub(super) fn validate_window_sets(
    window_sets: &[CrisisScenarioWindowSet],
    scenarios: &[CrisisScenarioDefinition],
) -> Result<(), String> {
    validate_scenario_refs(
        window_sets.iter().map(|window_set| {
            (
                window_set.window_set_id.as_str(),
                window_set.scenario_ids.as_slice(),
                "window_set",
            )
        }),
        scenarios,
    )
}

fn validate_scenario_refs<'a>(
    sets: impl Iterator<Item = (&'a str, &'a [String], &'a str)>,
    scenarios: &[CrisisScenarioDefinition],
) -> Result<(), String> {
    let known_ids = scenarios
        .iter()
        .map(|scenario| scenario.scenario_id.as_str())
        .collect::<HashSet<_>>();
    let mut set_ids = HashSet::new();

    for (set_id, scenario_ids, set_kind) in sets {
        if !set_ids.insert(format!("{set_kind}:{set_id}")) {
            return Err(format!("{set_kind} {set_id} 重复定义。"));
        }
        if scenario_ids.is_empty() {
            return Err(format!("{set_kind} {set_id} 不能为空。"));
        }

        let mut local_ids = HashSet::new();
        for scenario_id in scenario_ids {
            if !known_ids.contains(scenario_id.as_str()) {
                return Err(format!(
                    "{set_kind} {set_id} 引用了不存在的场景 {scenario_id}。"
                ));
            }
            if !local_ids.insert(scenario_id.as_str()) {
                return Err(format!(
                    "{set_kind} {set_id} 内部重复引用场景 {scenario_id}。"
                ));
            }
        }
    }

    Ok(())
}

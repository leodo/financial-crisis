use chrono::{Duration, NaiveDate};

use super::CrisisScenario;

pub(crate) fn scenario_supports_horizon(scenario: &CrisisScenario, horizon_days: u32) -> bool {
    scenario.default_horizon_roles.contains(&horizon_days)
}

pub(crate) fn label_anchor_date(scenario: &CrisisScenario, horizon_days: u32) -> NaiveDate {
    if horizon_days == 5 {
        scenario.acute_start.unwrap_or(scenario.crisis_start)
    } else {
        scenario.crisis_start
    }
}

fn action_window_lead_days(horizon_days: u32) -> i64 {
    match horizon_days {
        5 => 10,
        20 => 35,
        60 => 90,
        _ => horizon_days as i64,
    }
}

pub(crate) fn action_window_start_date(scenario: &CrisisScenario, horizon_days: u32) -> NaiveDate {
    let anchor_date = label_anchor_date(scenario, horizon_days);
    let buffered_start = anchor_date
        .checked_sub_signed(Duration::days(action_window_lead_days(horizon_days)))
        .unwrap_or(anchor_date);
    scenario.pre_warning_start.max(buffered_start)
}

fn action_window_end_days(horizon_days: u32) -> i64 {
    match horizon_days {
        5 => 7,
        20 => 20,
        60 => 30,
        _ => horizon_days as i64,
    }
}

fn action_window_end_date(scenario: &CrisisScenario, horizon_days: u32) -> NaiveDate {
    let anchor_date = label_anchor_date(scenario, horizon_days);
    let buffered_end = anchor_date
        .checked_add_signed(Duration::days(action_window_end_days(horizon_days)))
        .unwrap_or(scenario.crisis_end);
    scenario.crisis_end.min(buffered_end)
}

pub(crate) fn action_window_label(
    as_of_date: NaiveDate,
    scenarios: &[CrisisScenario],
    horizon_days: i64,
) -> u8 {
    let horizon_days_u32 = horizon_days as u32;
    scenarios.iter().any(|scenario| {
        as_of_date >= action_window_start_date(scenario, horizon_days_u32)
            && as_of_date <= action_window_end_date(scenario, horizon_days_u32)
    }) as u8
}

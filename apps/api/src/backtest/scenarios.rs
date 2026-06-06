use chrono::{Duration, NaiveDate};
use fc_domain::{
    load_crisis_scenario_catalog, AssessmentHistoryPoint, BacktestScenarioSummary,
    BacktestSignalSource, RiskContributor, RiskLevel, RiskSnapshot,
};

use crate::assessment::ProbabilityActionThresholds;

use super::actionability::{is_actionable_warning_point, is_structural_warning_point};

const BACKTEST_SIGNAL_WINDOW: usize = 5;
const BACKTEST_SIGNAL_MIN_HITS: usize = 3;

#[derive(Debug, Clone)]
pub(super) struct ScenarioDefinition {
    pub(super) scenario_id: String,
    pub(super) name: String,
    pub(super) region: String,
    pub(super) pre_warning_start: NaiveDate,
    pub(super) crisis_start: NaiveDate,
    pub(super) crisis_end: NaiveDate,
    pub(super) protected_window: bool,
    pub(super) fallback_first_l2_date: Option<NaiveDate>,
    pub(super) fallback_first_l3_date: Option<NaiveDate>,
    pub(super) fallback_max_level: RiskLevel,
    pub(super) fallback_max_score: f64,
    pub(super) fallback_lead_time_days: Option<i64>,
    pub(super) fallback_false_positive_count: u32,
}

#[derive(Debug, Clone, Copy)]
struct ScenarioFallbackProfile {
    fallback_first_l2_date: Option<NaiveDate>,
    fallback_first_l3_date: Option<NaiveDate>,
    fallback_max_level: RiskLevel,
    fallback_max_score: f64,
    fallback_lead_time_days: Option<i64>,
    fallback_false_positive_count: u32,
}

pub(crate) fn build_backtests(
    snapshot: &RiskSnapshot,
    history: &[AssessmentHistoryPoint],
    use_transitional_bridge: bool,
    strict_thresholds: Option<ProbabilityActionThresholds>,
) -> Vec<BacktestScenarioSummary> {
    let history_start = history.first().map(|point| point.as_of_date);
    let history_end = history.last().map(|point| point.as_of_date);
    scenario_catalog()
        .into_iter()
        .map(|scenario| {
            scenario_summary_from_history(
                snapshot,
                history,
                &scenario,
                use_transitional_bridge,
                strict_thresholds,
                snapshot.top_contributors.iter().take(3).cloned().collect(),
            )
            .unwrap_or_else(|| fallback_backtest(snapshot, &scenario, history_start, history_end))
        })
        .collect()
}

pub(super) fn scenario_catalog() -> Vec<ScenarioDefinition> {
    let catalog = load_crisis_scenario_catalog();
    catalog
        .scenarios
        .into_iter()
        .map(|scenario| {
            let fallback = scenario_fallback_profile(&scenario.scenario_id);
            ScenarioDefinition {
                scenario_id: scenario.scenario_id,
                name: scenario.label,
                region: "US".to_string(),
                pre_warning_start: scenario.pre_warning_start,
                crisis_start: scenario.crisis_start,
                crisis_end: scenario.crisis_end,
                protected_window: scenario.protected_window,
                fallback_first_l2_date: fallback.fallback_first_l2_date,
                fallback_first_l3_date: fallback.fallback_first_l3_date,
                fallback_max_level: fallback.fallback_max_level,
                fallback_max_score: fallback.fallback_max_score,
                fallback_lead_time_days: fallback.fallback_lead_time_days,
                fallback_false_positive_count: fallback.fallback_false_positive_count,
            }
        })
        .collect()
}

fn scenario_fallback_profile(scenario_id: &str) -> ScenarioFallbackProfile {
    match scenario_id {
        "us_black_monday_1987" => ScenarioFallbackProfile {
            fallback_first_l2_date: Some(NaiveDate::from_ymd_opt(1987, 10, 8).expect("valid date")),
            fallback_first_l3_date: Some(
                NaiveDate::from_ymd_opt(1987, 10, 16).expect("valid date"),
            ),
            fallback_max_level: RiskLevel::Crisis,
            fallback_max_score: 95.0,
            fallback_lead_time_days: Some(6),
            fallback_false_positive_count: 0,
        },
        "us_ltcm_1998" => ScenarioFallbackProfile {
            fallback_first_l2_date: Some(NaiveDate::from_ymd_opt(1998, 8, 10).expect("valid date")),
            fallback_first_l3_date: Some(NaiveDate::from_ymd_opt(1998, 8, 27).expect("valid date")),
            fallback_max_level: RiskLevel::Crisis,
            fallback_max_score: 84.0,
            fallback_lead_time_days: Some(7),
            fallback_false_positive_count: 1,
        },
        "us_dotcom_unwind_2000" => ScenarioFallbackProfile {
            fallback_first_l2_date: Some(NaiveDate::from_ymd_opt(2000, 2, 10).expect("valid date")),
            fallback_first_l3_date: None,
            fallback_max_level: RiskLevel::Warning,
            fallback_max_score: 68.0,
            fallback_lead_time_days: Some(29),
            fallback_false_positive_count: 1,
        },
        "us_gfc_2008" => ScenarioFallbackProfile {
            fallback_first_l2_date: Some(NaiveDate::from_ymd_opt(2007, 6, 15).expect("valid date")),
            fallback_first_l3_date: Some(NaiveDate::from_ymd_opt(2007, 8, 9).expect("valid date")),
            fallback_max_level: RiskLevel::Crisis,
            fallback_max_score: 92.0,
            fallback_lead_time_days: Some(47),
            fallback_false_positive_count: 2,
        },
        "us_funding_stress_2011" => ScenarioFallbackProfile {
            fallback_first_l2_date: Some(NaiveDate::from_ymd_opt(2011, 7, 18).expect("valid date")),
            fallback_first_l3_date: None,
            fallback_max_level: RiskLevel::Warning,
            fallback_max_score: 71.0,
            fallback_lead_time_days: Some(11),
            fallback_false_positive_count: 1,
        },
        "us_covid_liquidity_2020" => ScenarioFallbackProfile {
            fallback_first_l2_date: Some(NaiveDate::from_ymd_opt(2020, 2, 25).expect("valid date")),
            fallback_first_l3_date: Some(NaiveDate::from_ymd_opt(2020, 3, 9).expect("valid date")),
            fallback_max_level: RiskLevel::Crisis,
            fallback_max_score: 88.0,
            fallback_lead_time_days: Some(13),
            fallback_false_positive_count: 1,
        },
        "us_rate_shock_2022" => ScenarioFallbackProfile {
            fallback_first_l2_date: Some(NaiveDate::from_ymd_opt(2022, 4, 29).expect("valid date")),
            fallback_first_l3_date: Some(NaiveDate::from_ymd_opt(2022, 6, 13).expect("valid date")),
            fallback_max_level: RiskLevel::Warning,
            fallback_max_score: 74.0,
            fallback_lead_time_days: Some(35),
            fallback_false_positive_count: 1,
        },
        "us_regional_banks_2023" => ScenarioFallbackProfile {
            fallback_first_l2_date: Some(NaiveDate::from_ymd_opt(2023, 2, 15).expect("valid date")),
            fallback_first_l3_date: Some(NaiveDate::from_ymd_opt(2023, 3, 10).expect("valid date")),
            fallback_max_level: RiskLevel::Warning,
            fallback_max_score: 78.0,
            fallback_lead_time_days: Some(21),
            fallback_false_positive_count: 1,
        },
        _ => ScenarioFallbackProfile {
            fallback_first_l2_date: None,
            fallback_first_l3_date: None,
            fallback_max_level: RiskLevel::Watch,
            fallback_max_score: 60.0,
            fallback_lead_time_days: None,
            fallback_false_positive_count: 0,
        },
    }
}

fn scenario_summary_from_history(
    snapshot: &RiskSnapshot,
    history: &[AssessmentHistoryPoint],
    scenario: &ScenarioDefinition,
    use_transitional_bridge: bool,
    strict_thresholds: Option<ProbabilityActionThresholds>,
    top_contributors: Vec<RiskContributor>,
) -> Option<BacktestScenarioSummary> {
    let crisis_points = history
        .iter()
        .filter(|point| {
            point.as_of_date >= scenario.crisis_start && point.as_of_date <= scenario.crisis_end
        })
        .cloned()
        .collect::<Vec<_>>();
    if crisis_points.is_empty() {
        return None;
    }

    let warmup_start = scenario.crisis_start - Duration::days(90);
    let warmup_points = history
        .iter()
        .filter(|point| {
            point.as_of_date >= warmup_start && point.as_of_date < scenario.crisis_start
        })
        .cloned()
        .collect::<Vec<_>>();

    let first_l2_date = first_sustained_signal_date(&warmup_points, is_structural_warning_point);
    let first_l3_date = first_sustained_signal_date(&warmup_points, |point| {
        is_actionable_warning_point(point, use_transitional_bridge, strict_thresholds)
    });

    let max_point = crisis_points
        .iter()
        .max_by(|left, right| left.overall_score.total_cmp(&right.overall_score))
        .expect("crisis_points is not empty");
    let lead_time_days = lead_time_from_date(scenario.crisis_start, first_l2_date);
    let actionable_lead_time_days = lead_time_from_date(scenario.crisis_start, first_l3_date);
    let false_positive_count = count_false_positive_actionable_episodes(
        &warmup_points,
        use_transitional_bridge,
        strict_thresholds,
    );

    Some(BacktestScenarioSummary {
        scenario_id: scenario.scenario_id.clone(),
        name: scenario.name.clone(),
        region: scenario.region.clone(),
        signal_source: BacktestSignalSource::RealHistory,
        crisis_start: scenario.crisis_start,
        crisis_end: scenario.crisis_end,
        first_l2_date,
        first_l3_date,
        max_level: RiskLevel::from_score(max_point.overall_score),
        max_score: max_point.overall_score,
        lead_time_days,
        actionable_lead_time_days,
        false_positive_count,
        missed: actionable_lead_time_days.is_none(),
        history_start: crisis_points.first().map(|point| point.as_of_date),
        history_end: crisis_points.last().map(|point| point.as_of_date),
        history_point_count: crisis_points.len() as u32,
        note: build_real_history_backtest_note(
            lead_time_days,
            actionable_lead_time_days,
            crisis_points.len(),
        ),
        top_contributors,
        method_version: snapshot.method_version.clone(),
    })
}

fn fallback_backtest(
    snapshot: &RiskSnapshot,
    scenario: &ScenarioDefinition,
    history_start: Option<NaiveDate>,
    history_end: Option<NaiveDate>,
) -> BacktestScenarioSummary {
    let structural_lead_time_days = scenario
        .fallback_first_l2_date
        .and_then(|date| lead_time_from_date(scenario.crisis_start, Some(date)))
        .or(scenario.fallback_lead_time_days);
    let actionable_lead_time_days = scenario
        .fallback_first_l3_date
        .and_then(|date| lead_time_from_date(scenario.crisis_start, Some(date)));
    BacktestScenarioSummary {
        scenario_id: scenario.scenario_id.clone(),
        name: scenario.name.clone(),
        region: scenario.region.clone(),
        signal_source: BacktestSignalSource::FallbackTemplate,
        crisis_start: scenario.crisis_start,
        crisis_end: scenario.crisis_end,
        first_l2_date: scenario.fallback_first_l2_date,
        first_l3_date: scenario.fallback_first_l3_date,
        max_level: scenario.fallback_max_level,
        max_score: scenario.fallback_max_score,
        lead_time_days: structural_lead_time_days,
        actionable_lead_time_days,
        false_positive_count: scenario.fallback_false_positive_count,
        missed: actionable_lead_time_days.is_none(),
        history_start,
        history_end,
        history_point_count: 0,
        note: match (history_start, history_end) {
            (Some(start), Some(end)) => format!(
                "本地历史库当前只覆盖 {start} 到 {end}，尚未覆盖该危机窗口，当前结果来自内置参考模板。"
            ),
            _ => "本地历史库尚未覆盖该危机窗口，当前结果来自内置参考模板。".to_string(),
        },
        top_contributors: snapshot.top_contributors.iter().take(3).cloned().collect(),
        method_version: snapshot.method_version.clone(),
    }
}

fn first_sustained_signal_date<F>(
    points: &[AssessmentHistoryPoint],
    predicate: F,
) -> Option<NaiveDate>
where
    F: Fn(&AssessmentHistoryPoint) -> bool,
{
    points.iter().enumerate().find_map(|(index, point)| {
        if !predicate(point) {
            return None;
        }
        let end = (index + BACKTEST_SIGNAL_WINDOW).min(points.len());
        let window = &points[index..end];
        let hit_count = window
            .iter()
            .filter(|candidate| predicate(candidate))
            .count();
        let required_hits = BACKTEST_SIGNAL_MIN_HITS.min(window.len());
        (hit_count >= required_hits).then_some(point.as_of_date)
    })
}

fn lead_time_from_date(crisis_start: NaiveDate, signal_date: Option<NaiveDate>) -> Option<i64> {
    signal_date
        .map(|date| (crisis_start - date).num_days())
        .filter(|days| *days >= 0)
}

fn count_false_positive_actionable_episodes(
    points: &[AssessmentHistoryPoint],
    use_transitional_bridge: bool,
    strict_thresholds: Option<ProbabilityActionThresholds>,
) -> u32 {
    let actionable_flags = points
        .iter()
        .map(|point| is_actionable_warning_point(point, use_transitional_bridge, strict_thresholds))
        .collect::<Vec<_>>();
    let mut episode_count = 0_u32;
    let mut index = 0_usize;

    while index < actionable_flags.len() {
        if !actionable_flags[index] {
            index += 1;
            continue;
        }

        let start = index;
        while index < actionable_flags.len() && actionable_flags[index] {
            index += 1;
        }

        if start + 1 < actionable_flags.len() {
            episode_count += 1;
        }
    }

    episode_count.saturating_sub(1)
}

fn build_real_history_backtest_note(
    structural_lead_time_days: Option<i64>,
    actionable_lead_time_days: Option<i64>,
    history_point_count: usize,
) -> String {
    match (structural_lead_time_days, actionable_lead_time_days) {
        (Some(structural), Some(actionable)) => format!(
            "本地真实历史共 {history_point_count} 个评估点；结构性抬升约提前 {structural} 天出现，可执行预警约提前 {actionable} 天形成。"
        ),
        (Some(structural), None) => format!(
            "本地真实历史共 {history_point_count} 个评估点；结构性抬升约提前 {structural} 天出现，但危机开始前未形成足够强的可执行预警。"
        ),
        (None, Some(actionable)) => format!(
            "本地真实历史共 {history_point_count} 个评估点；危机前未见稳定的结构抬升，但约提前 {actionable} 天进入可执行预警。"
        ),
        (None, None) => format!(
            "本地真实历史共 {history_point_count} 个评估点；危机开始前未形成稳定的结构抬升或可执行预警。"
        ),
    }
}

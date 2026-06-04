use chrono::{Duration, NaiveDate};
use fc_domain::{
    load_crisis_scenario_catalog, AssessmentHistoryPoint, BacktestRollingAudit,
    BacktestRollingAuditEpisode, BacktestScenarioSummary, BacktestSignalSource,
    BacktestWindowPoint, DecisionPosture, ProtectedStressWindow, RiskContributor, RiskLevel,
    RiskSnapshot, TimeToRiskBucket,
};

use crate::{assessment::ServingModelContext, history_replay::is_formal_main_release};

const BACKTEST_SIGNAL_WINDOW: usize = 5;
const BACKTEST_SIGNAL_MIN_HITS: usize = 3;
const ACTIONABLE_AUDIT_HORIZON_DEFEND_DAYS: i64 = 5;
const ACTIONABLE_AUDIT_HORIZON_HEDGE_DAYS: i64 = 20;
const ACTIONABLE_AUDIT_HORIZON_PREPARE_DAYS: i64 = 60;
const ROLLING_AUDIT_EPISODE_LIMIT: usize = 12;
const ROLLING_AUDIT_MIN_DATE: (i32, u32, u32) = (1990, 1, 2);

#[derive(Debug, Clone)]
struct ScenarioDefinition {
    scenario_id: String,
    name: String,
    region: String,
    pre_warning_start: NaiveDate,
    crisis_start: NaiveDate,
    crisis_end: NaiveDate,
    protected_window: bool,
    fallback_first_l2_date: Option<NaiveDate>,
    fallback_first_l3_date: Option<NaiveDate>,
    fallback_max_level: RiskLevel,
    fallback_max_score: f64,
    fallback_lead_time_days: Option<i64>,
    fallback_false_positive_count: u32,
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

#[derive(Debug, Clone)]
struct RollingAuditEpisodeBuilder {
    start_date: NaiveDate,
    end_date: NaiveDate,
    signal_count: u32,
    classification: &'static str,
    note: String,
}

pub(crate) fn build_backtests(
    snapshot: &RiskSnapshot,
    history: &[AssessmentHistoryPoint],
    use_transitional_bridge: bool,
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
                snapshot.top_contributors.iter().take(3).cloned().collect(),
            )
            .unwrap_or_else(|| fallback_backtest(snapshot, &scenario, history_start, history_end))
        })
        .collect()
}

pub(crate) fn build_backtest_timeline(
    history: &[AssessmentHistoryPoint],
    use_transitional_bridge: bool,
) -> Vec<BacktestWindowPoint> {
    history
        .iter()
        .map(|point| BacktestWindowPoint {
            as_of_date: point.as_of_date,
            overall_score: point.overall_score,
            p_5d: point.p_5d,
            p_20d: point.p_20d,
            p_60d: point.p_60d,
            posture: point.posture,
            crisis_window_open: is_actionable_warning_point(point, use_transitional_bridge),
        })
        .collect()
}

pub(crate) fn build_rolling_backtest_audit(
    history: &[AssessmentHistoryPoint],
    stress_windows: &[ProtectedStressWindow],
    use_transitional_bridge: bool,
) -> BacktestRollingAudit {
    let catalog_window_start = scenario_catalog()
        .iter()
        .map(|scenario| scenario.crisis_start - Duration::days(90))
        .min();
    let min_supported_date = NaiveDate::from_ymd_opt(
        ROLLING_AUDIT_MIN_DATE.0,
        ROLLING_AUDIT_MIN_DATE.1,
        ROLLING_AUDIT_MIN_DATE.2,
    )
    .expect("valid rolling audit min date");
    let audit_window_start = Some(
        catalog_window_start
            .map(|date| date.max(min_supported_date))
            .unwrap_or(min_supported_date),
    );
    let filtered_history = history
        .iter()
        .filter(|point| audit_window_start.is_none_or(|start| point.as_of_date >= start))
        .cloned()
        .collect::<Vec<_>>();

    if filtered_history.is_empty() {
        return BacktestRollingAudit {
            history_point_count: 0,
            actionable_signal_count: 0,
            pre_crisis_signal_count: 0,
            in_crisis_signal_count: 0,
            stress_window_signal_count: 0,
            false_positive_signal_count: 0,
            false_positive_episode_count: 0,
            longest_false_positive_episode_days: 0,
            actionable_precision: 0.0,
            classified_episodes: Vec::new(),
            summary: "当前没有历史评估序列，无法生成全历史滚动审计。".to_string(),
        };
    }

    let scenarios = scenario_catalog();
    let mut actionable_signal_count = 0_u32;
    let mut pre_crisis_signal_count = 0_u32;
    let mut in_crisis_signal_count = 0_u32;
    let mut stress_window_signal_count = 0_u32;
    let mut false_positive_signal_count = 0_u32;
    let mut false_positive_episode_count = 0_u32;
    let mut longest_false_positive_episode_days = 0_u32;
    let mut classified_episodes = Vec::new();
    let mut current_episode: Option<RollingAuditEpisodeBuilder> = None;

    for point in &filtered_history {
        let is_actionable = is_actionable_warning_point(point, use_transitional_bridge);
        let in_crisis = scenarios.iter().any(|scenario| {
            point.as_of_date >= scenario.crisis_start && point.as_of_date <= scenario.crisis_end
        });
        let next_crisis_lead_days = scenarios
            .iter()
            .filter_map(|scenario| {
                (scenario.crisis_start >= point.as_of_date)
                    .then_some((scenario.crisis_start - point.as_of_date).num_days())
            })
            .min();
        let actionable_horizon_days = actionable_audit_horizon_days(point);
        let within_actionable_horizon = next_crisis_lead_days
            .map(|days| days <= actionable_horizon_days)
            .unwrap_or(false);

        if is_actionable {
            actionable_signal_count += 1;
            if in_crisis {
                in_crisis_signal_count += 1;
            } else if within_actionable_horizon {
                pre_crisis_signal_count += 1;
            } else if let Some(note) =
                protected_stress_window_note(point.as_of_date, stress_windows, &scenarios)
            {
                stress_window_signal_count += 1;
                advance_classified_episode(
                    &mut current_episode,
                    Some(("stress_window", note)),
                    point.as_of_date,
                    &mut classified_episodes,
                    &mut false_positive_episode_count,
                    &mut longest_false_positive_episode_days,
                );
                continue;
            } else {
                false_positive_signal_count += 1;
                advance_classified_episode(
                    &mut current_episode,
                    Some((
                        "false_positive",
                        format!(
                            "未落入姿态对应的危机前 {actionable_horizon_days} 日窗口，也不在受保护压力窗口内。"
                        ),
                    )),
                    point.as_of_date,
                    &mut classified_episodes,
                    &mut false_positive_episode_count,
                    &mut longest_false_positive_episode_days,
                );
                continue;
            }
        }

        advance_classified_episode(
            &mut current_episode,
            None,
            point.as_of_date,
            &mut classified_episodes,
            &mut false_positive_episode_count,
            &mut longest_false_positive_episode_days,
        );
    }

    close_classified_episode(
        &mut current_episode,
        &mut classified_episodes,
        &mut false_positive_episode_count,
        &mut longest_false_positive_episode_days,
    );

    let actionable_precision_denominator =
        pre_crisis_signal_count + stress_window_signal_count + false_positive_signal_count;
    let actionable_precision = if actionable_precision_denominator == 0 {
        0.0
    } else {
        ((pre_crisis_signal_count + stress_window_signal_count) as f64
            / actionable_precision_denominator as f64)
            .clamp(0.0, 1.0)
    };
    let history_start = filtered_history.first().map(|point| point.as_of_date);
    let history_end = filtered_history.last().map(|point| point.as_of_date);
    classified_episodes.sort_by(|left, right| {
        right
            .duration_days
            .cmp(&left.duration_days)
            .then_with(|| right.signal_count.cmp(&left.signal_count))
            .then_with(|| right.start_date.cmp(&left.start_date))
    });
    classified_episodes.truncate(ROLLING_AUDIT_EPISODE_LIMIT);
    let summary = format!(
        "全历史滚动审计覆盖 {} 到 {}；动作级信号共 {} 个评估点，其中危机前 {} 个、危机中 {} 个、受保护压力窗口 {} 个、纯误报 {} 个，形成 {} 段纯误报区间，动作信号精度约为 {:.0}%。",
        history_start
            .map(|date| date.to_string())
            .unwrap_or_else(|| "未知起点".to_string()),
        history_end
            .map(|date| date.to_string())
            .unwrap_or_else(|| "未知终点".to_string()),
        actionable_signal_count,
        pre_crisis_signal_count,
        in_crisis_signal_count,
        stress_window_signal_count,
        false_positive_signal_count,
        false_positive_episode_count,
        actionable_precision * 100.0
    );

    BacktestRollingAudit {
        history_point_count: filtered_history.len() as u32,
        actionable_signal_count,
        pre_crisis_signal_count,
        in_crisis_signal_count,
        stress_window_signal_count,
        false_positive_signal_count,
        false_positive_episode_count,
        longest_false_positive_episode_days,
        actionable_precision: round3(actionable_precision),
        classified_episodes,
        summary,
    }
}

pub(crate) fn is_actionable_warning_point(
    point: &AssessmentHistoryPoint,
    use_transitional_bridge: bool,
) -> bool {
    let strict_short_horizon_signal =
        matches!(
            point.posture,
            DecisionPosture::Hedge | DecisionPosture::Defend
        ) || (matches!(point.time_to_risk_bucket, TimeToRiskBucket::Now)
            && point.overall_score >= 60.0
            && point.p_5d >= 0.18)
            || (matches!(point.time_to_risk_bucket, TimeToRiskBucket::Weeks)
                && point.overall_score >= 58.0
                && point.p_20d >= 0.25
                && point.external_shock_score >= 44.0);

    let high_probability_prepare_signal = matches!(point.posture, DecisionPosture::Prepare)
        && point.p_20d >= 0.18
        && point.p_60d >= 0.45
        && ((point.overall_score >= 60.0 && point.external_shock_score >= 46.0)
            || (point.overall_score >= 53.0
                && !matches!(point.time_to_risk_bucket, TimeToRiskBucket::Normal)
                && has_strong_prepare_trigger_code(point)));
    let high_probability_months_signal =
        matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
            && point.overall_score >= 62.0
            && point.p_20d >= 0.18
            && point.p_60d >= 0.45
            && point.external_shock_score >= 48.0;

    // Persisted historical snapshots still carry a transitional posture/bucket view:
    // probabilities are often floor-bound, while overall/external stress capture the
    // elevated state. Until the raw point-in-time feature store replaces that archive,
    // rolling audit needs a bridge rule for strong prepare/months phases.
    let prepare_bridge_signal = use_transitional_bridge
        && matches!(point.posture, DecisionPosture::Prepare)
        && point.overall_score >= 58.0
        && point.external_shock_score >= 46.0;
    let months_bridge_signal = use_transitional_bridge
        && matches!(point.time_to_risk_bucket, TimeToRiskBucket::Months)
        && point.overall_score >= 58.0
        && point.external_shock_score >= 42.0;

    strict_short_horizon_signal
        || high_probability_prepare_signal
        || high_probability_months_signal
        || prepare_bridge_signal
        || months_bridge_signal
}

pub(crate) fn use_transitional_actionable_bridge(
    serving_model: Option<&ServingModelContext>,
) -> bool {
    !is_formal_main_release(serving_model)
}

fn scenario_catalog() -> Vec<ScenarioDefinition> {
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
        is_actionable_warning_point(point, use_transitional_bridge)
    });

    let max_point = crisis_points
        .iter()
        .max_by(|left, right| left.overall_score.total_cmp(&right.overall_score))
        .expect("crisis_points is not empty");
    let lead_time_days = lead_time_from_date(scenario.crisis_start, first_l2_date);
    let actionable_lead_time_days = lead_time_from_date(scenario.crisis_start, first_l3_date);
    let false_positive_count =
        count_false_positive_actionable_episodes(&warmup_points, use_transitional_bridge);

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

fn advance_classified_episode(
    current_episode: &mut Option<RollingAuditEpisodeBuilder>,
    next_episode: Option<(&'static str, String)>,
    as_of_date: NaiveDate,
    classified_episodes: &mut Vec<BacktestRollingAuditEpisode>,
    false_positive_episode_count: &mut u32,
    longest_false_positive_episode_days: &mut u32,
) {
    match next_episode {
        Some((classification, note)) => {
            let continue_existing = current_episode.as_ref().is_some_and(|episode| {
                episode.classification == classification && episode.note == note
            });
            if continue_existing {
                if let Some(episode) = current_episode.as_mut() {
                    episode.end_date = as_of_date;
                    episode.signal_count += 1;
                }
            } else {
                close_classified_episode(
                    current_episode,
                    classified_episodes,
                    false_positive_episode_count,
                    longest_false_positive_episode_days,
                );
                *current_episode = Some(RollingAuditEpisodeBuilder {
                    start_date: as_of_date,
                    end_date: as_of_date,
                    signal_count: 1,
                    classification,
                    note,
                });
            }
        }
        None => close_classified_episode(
            current_episode,
            classified_episodes,
            false_positive_episode_count,
            longest_false_positive_episode_days,
        ),
    }
}

fn close_classified_episode(
    current_episode: &mut Option<RollingAuditEpisodeBuilder>,
    classified_episodes: &mut Vec<BacktestRollingAuditEpisode>,
    false_positive_episode_count: &mut u32,
    longest_false_positive_episode_days: &mut u32,
) {
    let Some(episode) = current_episode.take() else {
        return;
    };

    let duration_days = (episode.end_date - episode.start_date).num_days().max(0) as u32 + 1;
    if episode.classification == "false_positive" {
        *false_positive_episode_count += 1;
        *longest_false_positive_episode_days =
            (*longest_false_positive_episode_days).max(duration_days);
    }
    classified_episodes.push(BacktestRollingAuditEpisode {
        start_date: episode.start_date,
        end_date: episode.end_date,
        duration_days,
        signal_count: episode.signal_count,
        classification: episode.classification.to_string(),
        note: episode.note,
    });
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

fn is_structural_warning_point(point: &AssessmentHistoryPoint) -> bool {
    ((point.p_60d >= 0.35) || !matches!(point.time_to_risk_bucket, TimeToRiskBucket::Normal))
        && point.overall_score >= 54.0
        || (point.overall_score >= 54.0
            && point.p_20d >= 0.12
            && point.external_shock_score >= 42.0)
}

fn has_strong_prepare_trigger_code(point: &AssessmentHistoryPoint) -> bool {
    point.posture_trigger_codes.iter().any(|code| {
        matches!(
            code.as_str(),
            "prepare_p60d_structural"
                | "prepare_structural_downgrade"
                | "prepare_carry_structural"
                | "prepare_external_structural"
        )
    })
}

fn actionable_audit_horizon_days(point: &AssessmentHistoryPoint) -> i64 {
    match point.posture {
        DecisionPosture::Defend => ACTIONABLE_AUDIT_HORIZON_DEFEND_DAYS,
        DecisionPosture::Hedge => ACTIONABLE_AUDIT_HORIZON_HEDGE_DAYS,
        DecisionPosture::Prepare => ACTIONABLE_AUDIT_HORIZON_PREPARE_DAYS,
        DecisionPosture::Normal => match point.time_to_risk_bucket {
            TimeToRiskBucket::Now => ACTIONABLE_AUDIT_HORIZON_DEFEND_DAYS,
            TimeToRiskBucket::Weeks => ACTIONABLE_AUDIT_HORIZON_HEDGE_DAYS,
            TimeToRiskBucket::Months => ACTIONABLE_AUDIT_HORIZON_PREPARE_DAYS,
            TimeToRiskBucket::Normal => ACTIONABLE_AUDIT_HORIZON_HEDGE_DAYS,
        },
    }
}

fn protected_stress_window_note(
    as_of_date: NaiveDate,
    explicit_windows: &[ProtectedStressWindow],
    scenarios: &[ScenarioDefinition],
) -> Option<String> {
    if let Some(window) = explicit_windows
        .iter()
        .find(|window| as_of_date >= window.start_date && as_of_date <= window.end_date)
    {
        return Some(format!("{}：{}", window.label, window.note));
    }

    scenarios
        .iter()
        .find(|scenario| {
            scenario.protected_window
                && as_of_date >= scenario.pre_warning_start
                && as_of_date <= scenario.crisis_end
        })
        .map(|scenario| {
            format!(
                "{}：场景目录将该阶段标记为受保护压力窗口，用于 posture 审计而不是主正例。",
                scenario.name
            )
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
) -> u32 {
    let actionable_flags = points
        .iter()
        .map(|point| is_actionable_warning_point(point, use_transitional_bridge))
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

fn round3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

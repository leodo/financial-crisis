use chrono::{Duration, NaiveDate};
use fc_domain::{
    AssessmentHistoryPoint, BacktestRollingAudit, BacktestRollingAuditEpisode,
    ProtectedStressWindow,
};

use super::{
    actionability::{actionable_audit_horizon_days, is_actionable_warning_point},
    scenarios::{scenario_catalog, ScenarioDefinition},
};

const ROLLING_AUDIT_EPISODE_LIMIT: usize = 12;
const ROLLING_AUDIT_MIN_DATE: (i32, u32, u32) = (1990, 1, 2);

#[derive(Debug, Clone)]
struct RollingAuditEpisodeBuilder {
    start_date: NaiveDate,
    end_date: NaiveDate,
    signal_count: u32,
    classification: &'static str,
    note: String,
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

fn round3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

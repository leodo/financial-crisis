use std::collections::{BTreeMap, BTreeSet};

use super::super::{
    format_runtime_category_list, ReleaseReviewHistoricalAuditPriority,
    ReleaseReviewHistoricalAuditWorkstreamSummary, ReleaseReviewRuntimeBlockCount,
    ReleaseReviewScenarioFocusDiagnostic,
};
use super::{release_review_gate_gap_profile_label, release_review_historical_workstream_priority};

fn format_gate_gap_profiles(profiles: &[String]) -> String {
    if profiles.is_empty() {
        "—".to_string()
    } else {
        profiles
            .iter()
            .map(|profile| release_review_gate_gap_profile_label(profile).to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn gate_gap_focus_summary(profiles: &[String]) -> Option<&'static str> {
    if profiles.is_empty() {
        return None;
    }
    let has_p20d_only = profiles.iter().any(|profile| profile == "p20d_only");
    let has_p60d_only = profiles.iter().any(|profile| profile == "p60d_only");
    let has_both = profiles.iter().any(|profile| profile == "p20d_and_p60d");

    if has_both || (has_p20d_only && has_p60d_only) {
        Some("下一轮应同时复核 p20d / p60d strict gate。")
    } else if has_p20d_only {
        Some("下一轮应先复核 p20d strict gate。")
    } else if has_p60d_only {
        Some("下一轮应先复核 p60d strict gate。")
    } else {
        None
    }
}

fn format_gate_gap_point_counts(
    counts: &[ReleaseReviewRuntimeBlockCount],
    for_candidate: bool,
) -> String {
    let rendered = counts
        .iter()
        .filter_map(|count| {
            let value = if for_candidate {
                count.candidate_count
            } else {
                count.baseline_count
            };
            (value > 0).then(|| {
                format!(
                    "{}={}",
                    release_review_gate_gap_profile_label(&count.category),
                    value
                )
            })
        })
        .collect::<Vec<_>>();
    if rendered.is_empty() {
        "—".to_string()
    } else {
        rendered.join(", ")
    }
}

fn gate_gap_point_count_focus_summary(
    counts: &[ReleaseReviewRuntimeBlockCount],
) -> Option<&'static str> {
    if counts.is_empty() {
        return None;
    }
    let p20d_only = counts
        .iter()
        .find(|count| count.category == "p20d_only")
        .map(|count| count.candidate_count.max(count.baseline_count))
        .unwrap_or(0);
    let p60d_only = counts
        .iter()
        .find(|count| count.category == "p60d_only")
        .map(|count| count.candidate_count.max(count.baseline_count))
        .unwrap_or(0);
    let both = counts
        .iter()
        .find(|count| count.category == "p20d_and_p60d")
        .map(|count| count.candidate_count.max(count.baseline_count))
        .unwrap_or(0);
    let total = p20d_only + p60d_only + both;
    if total == 0 {
        None
    } else if p20d_only > p60d_only + both {
        Some("下一轮应先复核 p20d strict gate。")
    } else if p60d_only > p20d_only + both {
        Some("下一轮应先复核 p60d strict gate。")
    } else {
        Some("下一轮应同时复核 p20d / p60d strict gate。")
    }
}

fn summarize_release_review_historical_audit_workstreams_internal(
    priorities: &[ReleaseReviewHistoricalAuditPriority],
    scenarios: &[ReleaseReviewScenarioFocusDiagnostic],
) -> Vec<ReleaseReviewHistoricalAuditWorkstreamSummary> {
    let scenarios_by_id = scenarios
        .iter()
        .map(|scenario| (scenario.scenario_id.as_str(), scenario))
        .collect::<BTreeMap<_, _>>();
    let mut workstreams = BTreeMap::<
        String,
        (
            BTreeSet<String>,
            u32,
            BTreeSet<String>,
            BTreeSet<String>,
            BTreeSet<String>,
            BTreeSet<String>,
            BTreeSet<String>,
            BTreeSet<String>,
            BTreeMap<String, (u32, u32)>,
        ),
    >::new();
    for priority in priorities {
        let entry = workstreams
            .entry(priority.primary_workstream.clone())
            .or_insert_with(|| {
                (
                    BTreeSet::new(),
                    0,
                    BTreeSet::new(),
                    BTreeSet::new(),
                    BTreeSet::new(),
                    BTreeSet::new(),
                    BTreeSet::new(),
                    BTreeSet::new(),
                    BTreeMap::new(),
                )
            });
        entry.0.insert(priority.scenario_name.clone());
        if priority.protected_window {
            entry.1 += 1;
        }
        entry.2.insert(priority.scenario_family.clone());
        entry.3.insert(priority.training_role.clone());
        entry.4.insert(priority.suggested_review.clone());
        entry.5.insert(priority.scenario_id.clone());
        if let Some(profile) = &priority.baseline_gate_gap_profile {
            entry.6.insert(profile.clone());
        }
        if let Some(profile) = &priority.candidate_gate_gap_profile {
            entry.7.insert(profile.clone());
        }
        if let Some(scenario) = scenarios_by_id.get(priority.scenario_id.as_str()) {
            for facet in &scenario.runtime_continuity_facet_counts {
                let Some(category) = facet.category.strip_prefix("gate_gap:") else {
                    continue;
                };
                if category == "none" {
                    continue;
                }
                let count_entry = entry.8.entry(category.to_string()).or_insert((0, 0));
                count_entry.0 += facet.baseline_count;
                count_entry.1 += facet.candidate_count;
            }
        }
    }

    let mut rows = workstreams
        .into_iter()
        .map(
            |(
                workstream,
                (
                    scenarios,
                    protected_count,
                    scenario_families,
                    training_roles,
                    suggested_reviews,
                    scenario_ids,
                    baseline_gate_gap_profiles,
                    candidate_gate_gap_profiles,
                    gate_gap_point_counts,
                ),
            )| {
                ReleaseReviewHistoricalAuditWorkstreamSummary {
                    workstream,
                    scenario_count: scenario_ids.len() as u32,
                    protected_count,
                    scenarios: scenarios.into_iter().collect(),
                    scenario_families: scenario_families.into_iter().collect(),
                    training_roles: training_roles.into_iter().collect(),
                    baseline_gate_gap_profiles: baseline_gate_gap_profiles.into_iter().collect(),
                    candidate_gate_gap_profiles: candidate_gate_gap_profiles.into_iter().collect(),
                    gate_gap_point_counts: gate_gap_point_counts
                        .into_iter()
                        .map(|(category, (baseline_count, candidate_count))| {
                            ReleaseReviewRuntimeBlockCount {
                                delta: candidate_count as i64 - baseline_count as i64,
                                category,
                                baseline_count,
                                candidate_count,
                            }
                        })
                        .collect(),
                    suggested_review: suggested_reviews
                        .into_iter()
                        .collect::<Vec<_>>()
                        .join(" / "),
                }
            },
        )
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        release_review_historical_workstream_priority(&left.workstream)
            .cmp(&release_review_historical_workstream_priority(
                &right.workstream,
            ))
            .then_with(|| right.scenario_count.cmp(&left.scenario_count))
            .then_with(|| left.workstream.cmp(&right.workstream))
    });
    rows
}

#[cfg(test)]
pub(crate) fn summarize_release_review_historical_audit_workstreams(
    priorities: &[ReleaseReviewHistoricalAuditPriority],
) -> Vec<ReleaseReviewHistoricalAuditWorkstreamSummary> {
    summarize_release_review_historical_audit_workstreams_internal(priorities, &[])
}

pub(crate) fn summarize_release_review_historical_audit_workstreams_with_focus(
    priorities: &[ReleaseReviewHistoricalAuditPriority],
    scenarios: &[ReleaseReviewScenarioFocusDiagnostic],
) -> Vec<ReleaseReviewHistoricalAuditWorkstreamSummary> {
    summarize_release_review_historical_audit_workstreams_internal(priorities, scenarios)
}

pub(crate) fn release_review_historical_audit_takeaways(
    rows: &[ReleaseReviewHistoricalAuditWorkstreamSummary],
) -> Vec<String> {
    let mut takeaways = Vec::new();
    for row in rows {
        match row.workstream.as_str() {
            "strict_review_vs_runtime_mapping" => {
                let focus_text = gate_gap_point_count_focus_summary(&row.gate_gap_point_counts)
                    .or_else(|| gate_gap_focus_summary(&row.candidate_gate_gap_profiles))
                    .or_else(|| gate_gap_focus_summary(&row.baseline_gate_gap_profiles))
                    .unwrap_or("下一轮应继续逐点复核 strict gate。");
                takeaways.push(format!(
                    "{} 个历史样本首先应复核 strict review gate 与 runtime floor 的映射，涉及 {}。当前更像 runtime 已经看到风险，但 strict gate 仍比运行时口径更严。strict gate gap 画像：baseline={}，candidate={}；点位计数：baseline={}，candidate={}。{}",
                    row.scenario_count,
                    format_runtime_category_list(&row.scenarios),
                    format_gate_gap_profiles(&row.baseline_gate_gap_profiles),
                    format_gate_gap_profiles(&row.candidate_gate_gap_profiles),
                    format_gate_gap_point_counts(&row.gate_gap_point_counts, false),
                    format_gate_gap_point_counts(&row.gate_gap_point_counts, true),
                    focus_text
                ));
            }
            "posture_continuity" => {
                takeaways.push(format!(
                    "{} 个历史样本主要卡在 posture continuity，涉及 {}。下一步应优先解释为什么高 p20d/p60d 仍长期停在 normal，以及 3/5 sustained 命中为什么建不起来。",
                    row.scenario_count,
                    format_runtime_category_list(&row.scenarios)
                ));
            }
            "score_confirmation" => {
                takeaways.push(format!(
                    "{} 个历史样本主要卡在 score confirmation，涉及 {}。这更像 months/prepare 的确认门槛过严，不是完全没有 pre-warning separation。",
                    row.scenario_count,
                    format_runtime_category_list(&row.scenarios)
                ));
            }
            "transitional_bridge" => {
                takeaways.push(format!(
                    "{} 个历史样本主要卡在 transitional bridge，涉及 {}。下一步应确认 bridge 只是过渡模型遗留问题，还是正式策略本身缺少连续触发。",
                    row.scenario_count,
                    format_runtime_category_list(&row.scenarios)
                ));
            }
            "prewarning_signal_gap" => {
                takeaways.push(format!(
                    "{} 个历史样本主要缺少可用的 pre-warning signal，涉及 {}。下一步应先回到训练样本、特征覆盖与标签窗口本身，确认为什么连 non-normal / runtime floor 都没有稳定形成。",
                    row.scenario_count,
                    format_runtime_category_list(&row.scenarios)
                ));
            }
            "weak_signal_continuity" => {
                takeaways.push(format!(
                    "{} 个历史样本已经出现弱 pre-warning 信号但没有形成可执行延续，涉及 {}。下一步应优先复核 feature separation、months/prepare continuity 与阈值前置量。",
                    row.scenario_count,
                    format_runtime_category_list(&row.scenarios)
                ));
            }
            _ => {
                takeaways.push(format!(
                    "{} 个历史样本仍落在 residual release-review audit，涉及 {}。{}",
                    row.scenario_count,
                    format_runtime_category_list(&row.scenarios),
                    row.suggested_review,
                ));
            }
        }
    }
    takeaways
}

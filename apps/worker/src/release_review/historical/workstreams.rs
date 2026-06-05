use std::collections::{BTreeMap, BTreeSet};

use super::super::{
    format_runtime_category_list, ReleaseReviewHistoricalAuditPriority,
    ReleaseReviewHistoricalAuditWorkstreamSummary,
};
use super::release_review_historical_workstream_priority;

pub(crate) fn summarize_release_review_historical_audit_workstreams(
    priorities: &[ReleaseReviewHistoricalAuditPriority],
) -> Vec<ReleaseReviewHistoricalAuditWorkstreamSummary> {
    let mut workstreams = BTreeMap::<
        String,
        (
            BTreeSet<String>,
            u32,
            BTreeSet<String>,
            BTreeSet<String>,
            BTreeSet<String>,
            BTreeSet<String>,
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
                ),
            )| {
                ReleaseReviewHistoricalAuditWorkstreamSummary {
                    workstream,
                    scenario_count: scenario_ids.len() as u32,
                    protected_count,
                    scenarios: scenarios.into_iter().collect(),
                    scenario_families: scenario_families.into_iter().collect(),
                    training_roles: training_roles.into_iter().collect(),
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

pub(crate) fn release_review_historical_audit_takeaways(
    rows: &[ReleaseReviewHistoricalAuditWorkstreamSummary],
) -> Vec<String> {
    let mut takeaways = Vec::new();
    for row in rows {
        match row.workstream.as_str() {
            "strict_review_vs_runtime_mapping" => {
                takeaways.push(format!(
                    "{} 个历史样本首先应复核 strict review gate 与 runtime floor 的映射，涉及 {}。当前更像 runtime 已经看到风险，但 strict gate 仍比运行时口径更严。",
                    row.scenario_count,
                    format_runtime_category_list(&row.scenarios)
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
            _ => {
                takeaways.push(format!(
                    "{} 个历史样本仍落在 residual release-review audit，涉及 {}。需要继续回到 block mix 与 continuity facets 做逐点复核。",
                    row.scenario_count,
                    format_runtime_category_list(&row.scenarios)
                ));
            }
        }
    }
    takeaways
}

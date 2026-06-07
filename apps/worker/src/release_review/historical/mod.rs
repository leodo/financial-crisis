use fc_domain::{CrisisScenarioDefinition, CrisisScenarioFamily, CrisisScenarioTrainingRole};

use super::ReleaseReviewScenarioFocusDiagnostic;

mod attribution;
mod failure_modes;
mod priorities;
mod workstreams;

pub(crate) use attribution::{
    summarize_release_review_historical_audit_actions,
    summarize_release_review_historical_audit_attribution,
};
pub(crate) use failure_modes::summarize_release_review_failure_modes;
pub(crate) use priorities::summarize_release_review_historical_audit_priorities;
#[cfg(test)]
pub(crate) use workstreams::summarize_release_review_historical_audit_workstreams;
pub(crate) use workstreams::{
    release_review_historical_audit_takeaways,
    summarize_release_review_historical_audit_workstreams_with_focus,
};

fn release_review_historical_audit_attribution_label(
    baseline_failure_mode: &str,
    baseline_matches: bool,
    candidate_matches: bool,
    outcome: &str,
    baseline_runtime_floor_hit_point_count: u32,
    candidate_runtime_floor_hit_point_count: u32,
) -> &'static str {
    match (baseline_matches, candidate_matches) {
        (true, true) => "both_baseline_and_candidate",
        (true, false) => "baseline_shared_weakness",
        (false, true)
            if release_review_candidate_revealed_next_blocker(
                baseline_failure_mode,
                outcome,
                baseline_runtime_floor_hit_point_count,
                candidate_runtime_floor_hit_point_count,
            ) =>
        {
            "candidate_revealed_next_blocker"
        }
        (false, true) => "candidate_regression",
        (false, false) => "unclassified",
    }
}

fn release_review_historical_audit_action_type(attribution: &str) -> &'static str {
    match attribution {
        "candidate_regression" => "candidate_reject_or_retrain",
        "candidate_revealed_next_blocker" => "next_blocker_fix_before_promotion",
        "both_baseline_and_candidate" => "shared_blocker_fix_before_promotion",
        "baseline_shared_weakness" => "baseline_research_fix",
        _ => "manual_review",
    }
}

fn release_review_historical_attribution_priority(attribution: &str) -> u8 {
    match attribution {
        "candidate_regression" => 0,
        "both_baseline_and_candidate" => 1,
        "candidate_revealed_next_blocker" => 2,
        "baseline_shared_weakness" => 3,
        _ => 4,
    }
}

fn release_review_historical_action_priority(action_type: &str) -> u8 {
    match action_type {
        "candidate_reject_or_retrain" => 0,
        "shared_blocker_fix_before_promotion" => 1,
        "next_blocker_fix_before_promotion" => 2,
        "baseline_research_fix" => 3,
        _ => 4,
    }
}

fn release_review_historical_audit_attribution_explanation(
    workstream: &str,
    attribution: &str,
    scenario_count: u32,
    scenarios: &[String],
) -> String {
    let workstream_label = match workstream {
        "strict_review_vs_runtime_mapping" => "strict gate vs runtime floor",
        "posture_continuity" => "posture continuity",
        "score_confirmation" => "score confirmation",
        "transitional_bridge" => "transitional bridge",
        _ => "residual release-review audit",
    };
    let scenario_text = super::format_runtime_category_list(scenarios);
    match attribution {
        "both_baseline_and_candidate" => format!(
            "{scenario_count} 个样本在 baseline 和 candidate 都落在 {workstream_label}，涉及 {scenario_text}。这说明它既是 formal main 的共性短板，也是 candidate 当前仍未修复的问题。"
        ),
        "candidate_regression" => format!(
            "{scenario_count} 个样本主要是 candidate 新落入 {workstream_label}，涉及 {scenario_text}。baseline 在同一条线没有对应失败，这更像这次候选版本自己退化出来的问题。"
        ),
        "candidate_revealed_next_blocker" => format!(
            "{scenario_count} 个样本当前主要落在 candidate 的 {workstream_label}，涉及 {scenario_text}。baseline 虽然没有同一条失败，但这些样本的动作结局没有比 baseline 更差，且 runtime floor 命中没有回退，因此更像 candidate 先修掉了上游阻塞，暴露出下一层 blocker，而不是纯粹退化。"
        ),
        "baseline_shared_weakness" => format!(
            "{scenario_count} 个样本在 baseline 已经落在 {workstream_label}，涉及 {scenario_text}。candidate 这版没有新增同类失败，因此这更像 formal main 既有短板。"
        ),
        _ => format!(
            "{scenario_count} 个样本目前仍需要继续人工复核 {workstream_label}，涉及 {scenario_text}。"
        ),
    }
}

fn release_review_historical_audit_action_recommendation(
    workstream: &str,
    attribution: &str,
    scenario_count: u32,
) -> String {
    let workstream_label = match workstream {
        "strict_review_vs_runtime_mapping" => "strict gate vs runtime floor",
        "posture_continuity" => "posture continuity",
        "score_confirmation" => "score confirmation",
        "transitional_bridge" => "transitional bridge",
        _ => "residual release-review audit",
    };
    match attribution {
        "candidate_regression" => format!(
            "{scenario_count} 个样本显示 candidate 在 {workstream_label} 上新增退化。当前更合适的动作是先判定该候选不具备晋升条件，回到训练 / 阈值 / policy 改动复核。"
        ),
        "candidate_revealed_next_blocker" => format!(
            "{scenario_count} 个样本显示 candidate 在不恶化历史动作结局的前提下，把阻塞暴露到了 {workstream_label}。当前不应把它按纯退化判退，但仍应把这条线视为晋升前必须修掉的下一层 blocker。"
        ),
        "both_baseline_and_candidate" => format!(
            "{scenario_count} 个样本显示 {workstream_label} 同时是 baseline 共性短板和 candidate 未修复阻塞。当前应先把这条线视为晋升前置 blocker。"
        ),
        "baseline_shared_weakness" => format!(
            "{scenario_count} 个样本显示 {workstream_label} 主要是 baseline 主线既有短板。当前更合适的动作是纳入 formal main 研究修复，而不是把责任归到 candidate 本轮。"
        ),
        _ => format!(
            "{scenario_count} 个样本在 {workstream_label} 上仍需继续人工复核，暂不适合自动给出晋升或判退结论。"
        ),
    }
}

fn release_review_failure_mode_matches_workstream(failure_mode: &str, workstream: &str) -> bool {
    failure_mode != "—" && release_review_workstream_for_failure_mode(failure_mode) == workstream
}

fn release_review_candidate_revealed_next_blocker(
    baseline_failure_mode: &str,
    outcome: &str,
    baseline_runtime_floor_hit_point_count: u32,
    candidate_runtime_floor_hit_point_count: u32,
) -> bool {
    release_review_has_known_failure_mode(baseline_failure_mode)
        && release_review_candidate_outcome_not_worse(outcome)
        && candidate_runtime_floor_hit_point_count >= baseline_runtime_floor_hit_point_count
}

fn release_review_has_known_failure_mode(failure_mode: &str) -> bool {
    !failure_mode.is_empty() && failure_mode != "—" && failure_mode != "unclassified"
}

fn release_review_candidate_outcome_not_worse(outcome: &str) -> bool {
    let Some((baseline, candidate)) = outcome.split_once("_to_") else {
        return false;
    };
    release_review_warning_state_rank(candidate) >= release_review_warning_state_rank(baseline)
}

fn release_review_warning_state_rank(state: &str) -> i8 {
    match state {
        "timely" => 2,
        "late_only" => 1,
        "missed" => 0,
        _ => -1,
    }
}

fn release_review_primary_workstream(
    baseline_failure_mode: Option<&str>,
    candidate_failure_mode: Option<&str>,
) -> &'static str {
    release_review_workstream_for_failure_mode(
        candidate_failure_mode
            .or(baseline_failure_mode)
            .unwrap_or_default(),
    )
}

fn release_review_gate_gap_profile_for_scenario(
    scenario: &ReleaseReviewScenarioFocusDiagnostic,
    for_candidate: bool,
) -> Option<String> {
    let mut best: Option<(&str, u32)> = None;
    for facet in &scenario.runtime_continuity_facet_counts {
        let category = facet.category.as_str();
        if !category.starts_with("gate_gap:") || category == "gate_gap:none" {
            continue;
        }
        let count = if for_candidate {
            facet.candidate_count
        } else {
            facet.baseline_count
        };
        if count == 0 {
            continue;
        }
        let should_replace = match best {
            Some((best_category, best_count)) => {
                count > best_count || (count == best_count && category < best_category)
            }
            None => true,
        };
        if should_replace {
            best = Some((category, count));
        }
    }

    best.map(|(category, _)| category.trim_start_matches("gate_gap:").to_string())
        .or_else(|| {
            let categories = if for_candidate {
                &scenario
                    .dominant_runtime_continuity_facets
                    .candidate_categories
            } else {
                &scenario
                    .dominant_runtime_continuity_facets
                    .baseline_categories
            };
            categories
                .iter()
                .find(|category| {
                    category.starts_with("gate_gap:") && category.as_str() != "gate_gap:none"
                })
                .map(|category| category.trim_start_matches("gate_gap:").to_string())
        })
}

fn release_review_gate_gap_profile_label(profile: &str) -> &'static str {
    match profile {
        "p20d_only" => "p20d only",
        "p60d_only" => "p60d only",
        "p20d_and_p60d" => "p20d + p60d",
        _ => "mixed gate gap",
    }
}

fn release_review_workstream_for_failure_mode(failure_mode: &str) -> &'static str {
    match failure_mode {
        "strict_gate_mismatch" => "strict_review_vs_runtime_mapping",
        "posture_continuity_failure" => "posture_continuity",
        "score_confirmation_failure" => "score_confirmation",
        "transitional_bridge_failure" => "transitional_bridge",
        _ => "residual_release_review_audit",
    }
}

fn release_review_historical_workstream_priority(workstream: &str) -> u8 {
    match workstream {
        "strict_review_vs_runtime_mapping" => 0,
        "posture_continuity" => 1,
        "score_confirmation" => 2,
        "transitional_bridge" => 3,
        _ => 4,
    }
}

fn release_review_suggested_historical_audit(
    scenario: &CrisisScenarioDefinition,
    focus: &ReleaseReviewScenarioFocusDiagnostic,
    primary_workstream: &str,
) -> &'static str {
    match primary_workstream {
        "strict_review_vs_runtime_mapping" => {
            "复核 strict review gate 与 runtime floor 的映射，先确认长窗结构性样本是否被过高的 p20d/p60d 硬门槛过早挡住。"
        }
        "posture_continuity" => {
            "复核 prepare/months 连续性，重点看高 p20d/p60d 日期为何仍长期停在 normal，以及 3/5 sustained 命中为何建不起来。"
        }
        "score_confirmation" => {
            "复核 months/prepare 的 score confirmation，确认 overall_score 与 external_shock_score 的确认门槛是否对结构性样本过严。"
        }
        "transitional_bridge" => {
            "复核 prepare/months bridge 的启用条件，确认 bridge 只是在过渡模型中失效，还是正式策略本身就缺少连续触发。"
        }
        "residual_release_review_audit"
            if release_review_is_prewarning_signal_gap(focus) =>
        {
            "当前更像 pre-warning signal gap：窗口里几乎没有 non-normal、runtime floor 或 actionable evidence，先回到训练样本、特征覆盖与标签窗口本身，确认为什么连可诊断 blocker 都没有形成。"
        }
        "residual_release_review_audit"
            if release_review_is_non_normal_without_actionable_followthrough(focus) =>
        {
            "当前更像弱连续性信号：窗口里已经出现 non-normal 或零星 runtime floor 提示，但还没形成可执行 pre-warning，先复核 feature separation、months/prepare continuity 与阈值前置量。"
        }
        _ if scenario.family == CrisisScenarioFamily::MixedSystemicStress => {
            "继续逐点复核 mixed_systemic_stress 的 runtime block mix，确认问题更像 gate、continuity，还是 residual review clause。"
        }
        _ => {
            "继续逐点复核 runtime block mix 与 continuity facets，确认未分类阻塞到底落在 gate、continuity 还是 residual review clause。"
        }
    }
}

fn release_review_has_runtime_block_or_facet_evidence(
    scenario: &ReleaseReviewScenarioFocusDiagnostic,
) -> bool {
    scenario
        .runtime_block_counts
        .iter()
        .any(|count| count.baseline_count > 0 || count.candidate_count > 0)
        || scenario
            .runtime_continuity_facet_counts
            .iter()
            .any(|count| count.baseline_count > 0 || count.candidate_count > 0)
}

fn release_review_is_prewarning_signal_gap(
    scenario: &ReleaseReviewScenarioFocusDiagnostic,
) -> bool {
    !release_review_has_runtime_block_or_facet_evidence(scenario)
        && scenario.baseline_actionable_point_count == 0
        && scenario.candidate_actionable_point_count == 0
        && scenario.baseline_runtime_floor_hit_point_count == 0
        && scenario.candidate_runtime_floor_hit_point_count == 0
        && scenario.baseline_first_non_normal_date.is_none()
        && scenario.candidate_first_non_normal_date.is_none()
}

fn release_review_is_non_normal_without_actionable_followthrough(
    scenario: &ReleaseReviewScenarioFocusDiagnostic,
) -> bool {
    !release_review_has_runtime_block_or_facet_evidence(scenario)
        && scenario.baseline_actionable_point_count == 0
        && scenario.candidate_actionable_point_count == 0
        && (scenario.baseline_first_non_normal_date.is_some()
            || scenario.candidate_first_non_normal_date.is_some()
            || scenario.baseline_runtime_floor_hit_point_count > 0
            || scenario.candidate_runtime_floor_hit_point_count > 0)
}

fn release_review_scenario_family_name(family: CrisisScenarioFamily) -> &'static str {
    match family {
        CrisisScenarioFamily::AcuteMarketLiquidityCrash => "acute_market_liquidity_crash",
        CrisisScenarioFamily::SystemicCreditBankingCrisis => "systemic_credit_banking_crisis",
        CrisisScenarioFamily::MixedSystemicStress => "mixed_systemic_stress",
        CrisisScenarioFamily::RateShockOrPolicyDislocation => "rate_shock_or_policy_dislocation",
    }
}

fn release_review_scenario_training_role_name(role: CrisisScenarioTrainingRole) -> &'static str {
    match role {
        CrisisScenarioTrainingRole::Mandatory => "mandatory",
        CrisisScenarioTrainingRole::CandidateOptional => "candidate_optional",
        CrisisScenarioTrainingRole::ExtensionOnly => "extension_only",
        CrisisScenarioTrainingRole::NoPositiveMain => "no_positive_main",
    }
}

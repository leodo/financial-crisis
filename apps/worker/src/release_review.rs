use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use chrono::NaiveDate;
use fc_domain::{
    load_crisis_scenario_catalog, ActionabilityLevel, AssessmentHistoryPoint, AssessmentSnapshot,
    CrisisScenarioDefinition, CrisisScenarioFamily, CrisisScenarioTrainingRole, ModelReleaseRecord,
};
use serde::Serialize;

use crate::{
    forward_crisis_training_regime, load_label_set_crisis_scenarios,
    probability_training_regime_name, regime_positive_window_gap_floor, reporting, round6,
    safe_divide, safe_ratio, AuditMethodResponseWire, CrisisScenario,
    RuntimeThresholdDiagnosticsWire, DEFAULT_FORMAL_LABEL_VERSION,
    DEFAULT_FORMAL_SCENARIO_SET_VERSION,
};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseReviewScalarMetric {
    pub(crate) baseline: f64,
    pub(crate) candidate: f64,
    pub(crate) delta: f64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseReviewCountMetric {
    pub(crate) baseline: u32,
    pub(crate) candidate: u32,
    pub(crate) delta: i64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseReviewBacktestScenarioComparison {
    pub(crate) scenario_id: String,
    pub(crate) name: String,
    pub(crate) signal_source: String,
    pub(crate) crisis_start: NaiveDate,
    pub(crate) crisis_end: NaiveDate,
    pub(crate) baseline_first_l2_date: Option<NaiveDate>,
    pub(crate) candidate_first_l2_date: Option<NaiveDate>,
    pub(crate) baseline_first_l3_date: Option<NaiveDate>,
    pub(crate) candidate_first_l3_date: Option<NaiveDate>,
    pub(crate) baseline_lead_time_days: Option<i64>,
    pub(crate) candidate_lead_time_days: Option<i64>,
    pub(crate) baseline_actionable_lead_time_days: Option<i64>,
    pub(crate) candidate_actionable_lead_time_days: Option<i64>,
    pub(crate) baseline_false_positive_count: u32,
    pub(crate) candidate_false_positive_count: u32,
    pub(crate) actionable_delta_days: Option<i64>,
    pub(crate) outcome: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseReviewScenarioPointComparison {
    pub(crate) as_of_date: NaiveDate,
    pub(crate) baseline_p20d: Option<f64>,
    pub(crate) candidate_p20d: Option<f64>,
    pub(crate) baseline_p60d: Option<f64>,
    pub(crate) candidate_p60d: Option<f64>,
    pub(crate) baseline_posture: Option<String>,
    pub(crate) candidate_posture: Option<String>,
    pub(crate) baseline_time_bucket: Option<String>,
    pub(crate) candidate_time_bucket: Option<String>,
    pub(crate) baseline_strict_review_actionable: bool,
    pub(crate) candidate_strict_review_actionable: bool,
    pub(crate) baseline_runtime_floor_hit: bool,
    pub(crate) candidate_runtime_floor_hit: bool,
    pub(crate) baseline_actionable: bool,
    pub(crate) candidate_actionable: bool,
    pub(crate) baseline_actionable_forward_5d_hits: Option<u32>,
    pub(crate) candidate_actionable_forward_5d_hits: Option<u32>,
    pub(crate) baseline_actionable_sustained: Option<bool>,
    pub(crate) candidate_actionable_sustained: Option<bool>,
    pub(crate) baseline_trigger_codes: Vec<String>,
    pub(crate) candidate_trigger_codes: Vec<String>,
    pub(crate) baseline_runtime_actionable_block_category: Option<String>,
    pub(crate) candidate_runtime_actionable_block_category: Option<String>,
    pub(crate) baseline_runtime_actionable_block_reason: Option<String>,
    pub(crate) candidate_runtime_actionable_block_reason: Option<String>,
    pub(crate) baseline_actionable_diagnostic: Option<String>,
    pub(crate) candidate_actionable_diagnostic: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseReviewRuntimeBlockCount {
    pub(crate) category: String,
    pub(crate) baseline_count: u32,
    pub(crate) candidate_count: u32,
    pub(crate) delta: i64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseReviewRuntimeDominantCategories {
    pub(crate) baseline_categories: Vec<String>,
    pub(crate) baseline_count: u32,
    pub(crate) candidate_categories: Vec<String>,
    pub(crate) candidate_count: u32,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseReviewScenarioFocusDiagnostic {
    pub(crate) scenario_id: String,
    pub(crate) name: String,
    pub(crate) outcome: String,
    pub(crate) window_start: NaiveDate,
    pub(crate) window_end: NaiveDate,
    pub(crate) crisis_start: NaiveDate,
    pub(crate) crisis_end: NaiveDate,
    pub(crate) baseline_first_l2_date: Option<NaiveDate>,
    pub(crate) candidate_first_l2_date: Option<NaiveDate>,
    pub(crate) baseline_first_l3_date: Option<NaiveDate>,
    pub(crate) candidate_first_l3_date: Option<NaiveDate>,
    pub(crate) baseline_first_non_normal_date: Option<NaiveDate>,
    pub(crate) candidate_first_non_normal_date: Option<NaiveDate>,
    pub(crate) baseline_actionable_point_count: u32,
    pub(crate) candidate_actionable_point_count: u32,
    pub(crate) baseline_runtime_floor_hit_point_count: u32,
    pub(crate) candidate_runtime_floor_hit_point_count: u32,
    pub(crate) baseline_max_p20d: Option<f64>,
    pub(crate) candidate_max_p20d: Option<f64>,
    pub(crate) baseline_max_p60d: Option<f64>,
    pub(crate) candidate_max_p60d: Option<f64>,
    pub(crate) baseline_first_runtime_floor_hit_without_l3_date: Option<NaiveDate>,
    pub(crate) candidate_first_runtime_floor_hit_without_l3_date: Option<NaiveDate>,
    pub(crate) baseline_first_runtime_floor_hit_without_l3_reason: Option<String>,
    pub(crate) candidate_first_runtime_floor_hit_without_l3_reason: Option<String>,
    pub(crate) baseline_primary_failure_mode: Option<String>,
    pub(crate) candidate_primary_failure_mode: Option<String>,
    pub(crate) dominant_runtime_blocks: ReleaseReviewRuntimeDominantCategories,
    pub(crate) dominant_runtime_continuity_facets: ReleaseReviewRuntimeDominantCategories,
    pub(crate) runtime_block_counts: Vec<ReleaseReviewRuntimeBlockCount>,
    pub(crate) runtime_continuity_facet_counts: Vec<ReleaseReviewRuntimeBlockCount>,
    pub(crate) interesting_points: Vec<ReleaseReviewScenarioPointComparison>,
}

#[derive(Debug, Clone)]
pub(crate) struct ReleaseReviewFailureModeSummary {
    pub(crate) failure_mode: String,
    pub(crate) baseline_count: u32,
    pub(crate) candidate_count: u32,
    pub(crate) baseline_scenarios: Vec<String>,
    pub(crate) candidate_scenarios: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseReviewHistoricalAuditPriority {
    pub(crate) scenario_id: String,
    pub(crate) scenario_name: String,
    pub(crate) scenario_family: String,
    pub(crate) training_role: String,
    pub(crate) protected_window: bool,
    pub(crate) baseline_failure_mode: String,
    pub(crate) candidate_failure_mode: String,
    pub(crate) primary_workstream: String,
    pub(crate) suggested_review: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseReviewHistoricalAuditWorkstreamSummary {
    pub(crate) workstream: String,
    pub(crate) scenario_count: u32,
    pub(crate) protected_count: u32,
    pub(crate) scenarios: Vec<String>,
    pub(crate) scenario_families: Vec<String>,
    pub(crate) training_roles: Vec<String>,
    pub(crate) suggested_review: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseReviewHistoricalAuditAttributionSummary {
    pub(crate) workstream: String,
    pub(crate) attribution: String,
    pub(crate) scenario_count: u32,
    pub(crate) protected_count: u32,
    pub(crate) baseline_count: u32,
    pub(crate) candidate_count: u32,
    pub(crate) baseline_scenarios: Vec<String>,
    pub(crate) candidate_scenarios: Vec<String>,
    pub(crate) explanation: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseReviewHistoricalAuditActionSummary {
    pub(crate) workstream: String,
    pub(crate) attribution: String,
    pub(crate) action_type: String,
    pub(crate) scenario_count: u32,
    pub(crate) protected_count: u32,
    pub(crate) recommendation: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseReviewComparisonSummary {
    pub(crate) timely_warning_rate: ReleaseReviewScalarMetric,
    pub(crate) strict_actionable_point_count: ReleaseReviewCountMetric,
    pub(crate) runtime_floor_hit_count: ReleaseReviewCountMetric,
    pub(crate) actionable_precision: ReleaseReviewScalarMetric,
    pub(crate) longest_false_positive_episode_days: ReleaseReviewCountMetric,
    pub(crate) current_p_5d: ReleaseReviewScalarMetric,
    pub(crate) current_p_20d: ReleaseReviewScalarMetric,
    pub(crate) current_p_60d: ReleaseReviewScalarMetric,
    pub(crate) runtime_separation_summary: Vec<ReleaseReviewRuntimeSeparationComparison>,
    pub(crate) backtest_scenarios: Vec<ReleaseReviewBacktestScenarioComparison>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseReviewRuntimeSeparationComparison {
    pub(crate) horizon_days: u32,
    pub(crate) baseline_diagnosis: String,
    pub(crate) candidate_diagnosis: String,
    pub(crate) baseline_threshold: Option<f64>,
    pub(crate) candidate_threshold: Option<f64>,
    pub(crate) baseline_early_warning_regime: String,
    pub(crate) candidate_early_warning_regime: String,
    pub(crate) baseline_early_warning_avg_probability: Option<f64>,
    pub(crate) candidate_early_warning_avg_probability: Option<f64>,
    pub(crate) baseline_normal_avg_probability: Option<f64>,
    pub(crate) candidate_normal_avg_probability: Option<f64>,
    pub(crate) baseline_early_warning_gap_vs_normal: Option<f64>,
    pub(crate) candidate_early_warning_gap_vs_normal: Option<f64>,
    pub(crate) baseline_floor_gap: Option<f64>,
    pub(crate) candidate_floor_gap: Option<f64>,
    pub(crate) baseline_early_warning_lift_vs_normal: Option<f64>,
    pub(crate) candidate_early_warning_lift_vs_normal: Option<f64>,
    pub(crate) baseline_threshold_hit_rate: Option<f64>,
    pub(crate) candidate_threshold_hit_rate: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseActionabilityLevelReview {
    pub(crate) level: ActionabilityLevel,
    pub(crate) proxy_horizon_days: u32,
    pub(crate) sample_count: u32,
    pub(crate) positive_rate: f64,
    pub(crate) threshold: f64,
    pub(crate) predicted_positive_count: u32,
    pub(crate) primary_positive_count: u32,
    pub(crate) late_validation_row_count: u32,
    pub(crate) protected_row_count: u32,
    pub(crate) primary_hit_count: u32,
    pub(crate) late_validation_hit_count: u32,
    pub(crate) protected_hit_count: u32,
    pub(crate) false_positive_count: u32,
    pub(crate) scenario_count: u32,
    pub(crate) on_time_scenario_count: u32,
    pub(crate) late_only_scenario_count: u32,
    pub(crate) missed_scenario_count: u32,
    pub(crate) precision_at_threshold: Option<f64>,
    pub(crate) primary_recall_at_threshold: Option<f64>,
    pub(crate) late_validation_capture_rate: Option<f64>,
    pub(crate) on_time_rate: Option<f64>,
    pub(crate) late_only_rate: Option<f64>,
    pub(crate) missed_rate: Option<f64>,
    pub(crate) note: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseActionabilityReview {
    pub(crate) release_id: String,
    pub(crate) enabled: bool,
    pub(crate) model_version: Option<String>,
    pub(crate) calibration_version: Option<String>,
    pub(crate) fusion_policy_version: Option<String>,
    pub(crate) levels: Vec<ReleaseActionabilityLevelReview>,
    pub(crate) guard_regressions: Vec<String>,
    pub(crate) guard_passed: bool,
    pub(crate) note: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseRuntimeCount {
    pub(crate) name: String,
    pub(crate) count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseRuntimeReviewDiagnostics {
    pub(crate) release_id: String,
    pub(crate) history_point_count: usize,
    pub(crate) posture_distribution: Vec<ReleaseRuntimeCount>,
    pub(crate) time_bucket_distribution: Vec<ReleaseRuntimeCount>,
    pub(crate) posture_trigger_distribution: Vec<ReleaseRuntimeClauseCount>,
    pub(crate) posture_blocker_distribution: Vec<ReleaseRuntimeClauseCount>,
    pub(crate) regime_probability_summaries: Vec<ReleaseRuntimeRegimeProbabilitySummary>,
    pub(crate) regime_separation_summaries: Vec<ReleaseRuntimeSeparationSummary>,
    pub(crate) runtime_thresholds: Option<RuntimeThresholdDiagnosticsWire>,
    pub(crate) points_at_or_above_prepare_p60d: Option<usize>,
    pub(crate) points_at_or_above_hedge_p20d: Option<usize>,
    pub(crate) points_at_or_above_defend_p5d: Option<usize>,
    pub(crate) note: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseRuntimeClauseCount {
    pub(crate) posture: String,
    pub(crate) clause: String,
    pub(crate) count: usize,
    pub(crate) share_of_posture: f64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseRuntimeRegimeProbabilitySummary {
    pub(crate) horizon_days: u32,
    pub(crate) regime: String,
    pub(crate) row_count: usize,
    pub(crate) row_rate: f64,
    pub(crate) avg_raw_probability: f64,
    pub(crate) max_raw_probability: f64,
    pub(crate) avg_probability: f64,
    pub(crate) max_probability: f64,
    pub(crate) raw_lift_vs_normal: Option<f64>,
    pub(crate) calibrated_lift_vs_normal: Option<f64>,
    pub(crate) raw_gap_vs_normal: Option<f64>,
    pub(crate) calibrated_gap_vs_normal: Option<f64>,
    pub(crate) calibration_gap_retention: Option<f64>,
    pub(crate) threshold_hit_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseRuntimeSeparationSummary {
    pub(crate) horizon_days: u32,
    pub(crate) early_warning_regime: String,
    pub(crate) normal_avg_probability: f64,
    pub(crate) pre_warning_buffer_avg_probability: f64,
    pub(crate) positive_window_avg_probability: f64,
    pub(crate) in_crisis_avg_probability: f64,
    pub(crate) post_crisis_cooldown_avg_probability: f64,
    pub(crate) early_warning_raw_lift_vs_normal: Option<f64>,
    pub(crate) early_warning_calibrated_lift_vs_normal: Option<f64>,
    pub(crate) early_warning_gap_retention: Option<f64>,
    pub(crate) positive_window_calibrated_lift_vs_normal: Option<f64>,
    pub(crate) positive_window_gap_vs_normal: Option<f64>,
    pub(crate) in_crisis_raw_lift_vs_normal: Option<f64>,
    pub(crate) in_crisis_calibrated_lift_vs_normal: Option<f64>,
    pub(crate) post_crisis_cooldown_calibrated_lift_vs_normal: Option<f64>,
    pub(crate) post_crisis_cooldown_gap_vs_normal: Option<f64>,
    pub(crate) max_non_normal_calibrated_lift_vs_normal: Option<f64>,
    pub(crate) max_non_normal_threshold_hit_rate: Option<f64>,
    pub(crate) diagnosis: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseReviewEnvelope {
    pub(crate) reviewed_at: String,
    pub(crate) market_scope: String,
    pub(crate) api_reload_url: String,
    pub(crate) history_mode: String,
    pub(crate) history_limit: usize,
    pub(crate) original_active_release_id: String,
    pub(crate) restored_release_id: String,
    pub(crate) baseline_release: ModelReleaseRecord,
    pub(crate) candidate_release: ModelReleaseRecord,
    pub(crate) baseline_assessment: AssessmentSnapshot,
    pub(crate) candidate_assessment: AssessmentSnapshot,
    pub(crate) baseline_runtime_review: ReleaseRuntimeReviewDiagnostics,
    pub(crate) candidate_runtime_review: ReleaseRuntimeReviewDiagnostics,
    pub(crate) baseline_actionability_review: ReleaseActionabilityReview,
    pub(crate) candidate_actionability_review: ReleaseActionabilityReview,
    pub(crate) scenario_focus: Vec<ReleaseReviewScenarioFocusDiagnostic>,
    pub(crate) historical_audit_workstreams: Vec<ReleaseReviewHistoricalAuditWorkstreamSummary>,
    pub(crate) historical_audit_priorities: Vec<ReleaseReviewHistoricalAuditPriority>,
    pub(crate) historical_audit_attribution: Vec<ReleaseReviewHistoricalAuditAttributionSummary>,
    pub(crate) historical_audit_actions: Vec<ReleaseReviewHistoricalAuditActionSummary>,
    pub(crate) comparison: ReleaseReviewComparisonSummary,
    pub(crate) probability_guard_regressions: Vec<String>,
    pub(crate) probability_guard_passed: bool,
    pub(crate) operational_guard_regressions: Vec<String>,
    pub(crate) operational_guard_passed: bool,
    pub(crate) actionability_guard_regressions: Vec<String>,
    pub(crate) actionability_guard_passed: bool,
    pub(crate) runtime_sanity_regressions: Vec<String>,
    pub(crate) runtime_sanity_passed: bool,
    pub(crate) overall_guard_regressions: Vec<String>,
    pub(crate) overall_guard_passed: bool,
    pub(crate) recommendation: String,
}

pub(crate) fn release_review_runtime_separation_takeaways(
    rows: &[crate::ReleaseReviewRuntimeSeparationComparison],
) -> Vec<String> {
    let mut takeaways = Vec::new();
    for row in rows {
        if !matches!(row.horizon_days, 20 | 60) {
            continue;
        }
        match row.candidate_diagnosis.as_str() {
            "separated_but_below_runtime_floor" => {
                takeaways.push(format!(
                    "{}d: candidate 的 {} 已经和 normal 拉开，但 early-warning 平均概率 {} 仍低于 runtime floor {}（floor gap {}）。这更像阈值 / runtime policy 瓶颈，不是完全没有信号。",
                    row.horizon_days,
                    row.candidate_early_warning_regime,
                    crate::format_optional_pct(row.candidate_early_warning_avg_probability),
                    crate::format_optional_pct(row.candidate_threshold),
                    crate::format_optional_pct(row.candidate_floor_gap),
                ));
            }
            "usable_early_warning_separation" => {
                if row.baseline_diagnosis == "separated_but_below_runtime_floor" {
                    takeaways.push(format!(
                        "{}d: candidate 已把 early-warning 平均概率 {} 推过 runtime floor {}（floor gap {}），说明这一窗的主瓶颈已不再是 runtime threshold 本身。",
                        row.horizon_days,
                        crate::format_optional_pct(row.candidate_early_warning_avg_probability),
                        crate::format_optional_pct(row.candidate_threshold),
                        crate::format_optional_pct(row.candidate_floor_gap),
                    ));
                }
            }
            "cooldown_bleed" => {
                takeaways.push(format!(
                    "{}d: candidate 仍有 cooldown bleed，说明 post-crisis cooldown 段概率抬得过高，容易把危机后的背景值误当成提前预警。",
                    row.horizon_days
                ));
            }
            "late_only_no_early_warning" => {
                takeaways.push(format!(
                    "{}d: candidate 只有晚到信号，没有形成可用的 early-warning separation，当前还谈不上可执行提前量。",
                    row.horizon_days
                ));
            }
            _ => {}
        }
    }
    takeaways
}

pub(crate) fn summarize_release_review_failure_modes(
    scenarios: &[crate::ReleaseReviewScenarioFocusDiagnostic],
) -> Vec<crate::ReleaseReviewFailureModeSummary> {
    let mut map = BTreeMap::<String, (BTreeSet<String>, BTreeSet<String>)>::new();
    for scenario in scenarios {
        if let Some(failure_mode) = scenario.baseline_primary_failure_mode.as_ref() {
            map.entry(failure_mode.clone())
                .or_default()
                .0
                .insert(scenario.name.clone());
        }
        if let Some(failure_mode) = scenario.candidate_primary_failure_mode.as_ref() {
            map.entry(failure_mode.clone())
                .or_default()
                .1
                .insert(scenario.name.clone());
        }
    }

    let mut rows = map
        .into_iter()
        .map(
            |(failure_mode, (baseline_scenarios, candidate_scenarios))| {
                crate::ReleaseReviewFailureModeSummary {
                    failure_mode,
                    baseline_count: baseline_scenarios.len() as u32,
                    candidate_count: candidate_scenarios.len() as u32,
                    baseline_scenarios: baseline_scenarios.into_iter().collect(),
                    candidate_scenarios: candidate_scenarios.into_iter().collect(),
                }
            },
        )
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .baseline_count
            .max(right.candidate_count)
            .cmp(&left.baseline_count.max(left.candidate_count))
            .then_with(|| left.failure_mode.cmp(&right.failure_mode))
    });
    rows
}

pub(crate) fn summarize_release_review_historical_audit_priorities(
    scenarios: &[crate::ReleaseReviewScenarioFocusDiagnostic],
) -> Vec<crate::ReleaseReviewHistoricalAuditPriority> {
    let catalog = load_crisis_scenario_catalog();
    let scenarios_by_id = catalog
        .scenarios
        .iter()
        .map(|scenario| (scenario.scenario_id.as_str(), scenario))
        .collect::<BTreeMap<_, _>>();

    let mut rows = scenarios
        .iter()
        .filter_map(|scenario| {
            let definition = scenarios_by_id
                .get(scenario.scenario_id.as_str())
                .copied()?;
            if !definition.protected_window
                && definition.training_role == CrisisScenarioTrainingRole::Mandatory
            {
                return None;
            }

            let baseline_failure_mode = scenario
                .baseline_primary_failure_mode
                .clone()
                .unwrap_or_else(|| "unclassified".to_string());
            let candidate_failure_mode = scenario
                .candidate_primary_failure_mode
                .clone()
                .unwrap_or_else(|| "—".to_string());
            let primary_workstream = release_review_primary_workstream(
                scenario.baseline_primary_failure_mode.as_deref(),
                scenario.candidate_primary_failure_mode.as_deref(),
            )
            .to_string();

            Some(crate::ReleaseReviewHistoricalAuditPriority {
                scenario_id: scenario.scenario_id.clone(),
                scenario_name: scenario.name.clone(),
                scenario_family: release_review_scenario_family_name(definition.family).to_string(),
                training_role: release_review_scenario_training_role_name(definition.training_role)
                    .to_string(),
                protected_window: definition.protected_window,
                baseline_failure_mode,
                candidate_failure_mode,
                primary_workstream: primary_workstream.clone(),
                suggested_review: release_review_suggested_historical_audit(
                    definition,
                    &primary_workstream,
                )
                .to_string(),
            })
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        release_review_historical_workstream_priority(&left.primary_workstream)
            .cmp(&release_review_historical_workstream_priority(
                &right.primary_workstream,
            ))
            .then_with(|| left.scenario_id.cmp(&right.scenario_id))
    });
    rows
}

pub(crate) fn summarize_release_review_historical_audit_attribution(
    priorities: &[crate::ReleaseReviewHistoricalAuditPriority],
) -> Vec<crate::ReleaseReviewHistoricalAuditAttributionSummary> {
    let mut rows = BTreeMap::<
        (String, String),
        (
            BTreeSet<String>,
            u32,
            u32,
            u32,
            BTreeSet<String>,
            BTreeSet<String>,
        ),
    >::new();
    for priority in priorities {
        let baseline_matches = release_review_failure_mode_matches_workstream(
            &priority.baseline_failure_mode,
            &priority.primary_workstream,
        );
        let candidate_matches = release_review_failure_mode_matches_workstream(
            &priority.candidate_failure_mode,
            &priority.primary_workstream,
        );
        let attribution =
            release_review_historical_audit_attribution_label(baseline_matches, candidate_matches);
        let entry = rows
            .entry((priority.primary_workstream.clone(), attribution.to_string()))
            .or_insert_with(|| (BTreeSet::new(), 0, 0, 0, BTreeSet::new(), BTreeSet::new()));
        entry.0.insert(priority.scenario_name.clone());
        if priority.protected_window {
            entry.1 += 1;
        }
        if baseline_matches {
            entry.2 += 1;
            entry.4.insert(priority.scenario_name.clone());
        }
        if candidate_matches {
            entry.3 += 1;
            entry.5.insert(priority.scenario_name.clone());
        }
    }

    let mut rows = rows
        .into_iter()
        .map(
            |(
                (workstream, attribution),
                (
                    scenarios,
                    protected_count,
                    baseline_count,
                    candidate_count,
                    baseline_scenarios,
                    candidate_scenarios,
                ),
            )| {
                let scenario_count = scenarios.len() as u32;
                let scenario_names = scenarios.into_iter().collect::<Vec<_>>();
                crate::ReleaseReviewHistoricalAuditAttributionSummary {
                    explanation: release_review_historical_audit_attribution_explanation(
                        &workstream,
                        &attribution,
                        scenario_count,
                        &scenario_names,
                    ),
                    workstream,
                    attribution,
                    scenario_count,
                    protected_count,
                    baseline_count,
                    candidate_count,
                    baseline_scenarios: baseline_scenarios.into_iter().collect(),
                    candidate_scenarios: candidate_scenarios.into_iter().collect(),
                }
            },
        )
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        release_review_historical_workstream_priority(&left.workstream)
            .cmp(&release_review_historical_workstream_priority(
                &right.workstream,
            ))
            .then_with(|| {
                release_review_historical_attribution_priority(&left.attribution).cmp(
                    &release_review_historical_attribution_priority(&right.attribution),
                )
            })
            .then_with(|| right.scenario_count.cmp(&left.scenario_count))
            .then_with(|| left.workstream.cmp(&right.workstream))
    });
    rows
}

pub(crate) fn summarize_release_review_historical_audit_actions(
    rows: &[crate::ReleaseReviewHistoricalAuditAttributionSummary],
) -> Vec<crate::ReleaseReviewHistoricalAuditActionSummary> {
    let mut actions = rows
        .iter()
        .map(|row| crate::ReleaseReviewHistoricalAuditActionSummary {
            workstream: row.workstream.clone(),
            attribution: row.attribution.clone(),
            action_type: release_review_historical_audit_action_type(&row.attribution).to_string(),
            scenario_count: row.scenario_count,
            protected_count: row.protected_count,
            recommendation: release_review_historical_audit_action_recommendation(
                &row.workstream,
                &row.attribution,
                row.scenario_count,
            ),
        })
        .collect::<Vec<_>>();
    actions.sort_by(|left, right| {
        release_review_historical_workstream_priority(&left.workstream)
            .cmp(&release_review_historical_workstream_priority(
                &right.workstream,
            ))
            .then_with(|| {
                release_review_historical_action_priority(&left.action_type).cmp(
                    &release_review_historical_action_priority(&right.action_type),
                )
            })
            .then_with(|| right.scenario_count.cmp(&left.scenario_count))
            .then_with(|| left.workstream.cmp(&right.workstream))
    });
    actions
}

pub(crate) fn summarize_release_review_historical_audit_workstreams(
    priorities: &[crate::ReleaseReviewHistoricalAuditPriority],
) -> Vec<crate::ReleaseReviewHistoricalAuditWorkstreamSummary> {
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
                crate::ReleaseReviewHistoricalAuditWorkstreamSummary {
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
    rows: &[crate::ReleaseReviewHistoricalAuditWorkstreamSummary],
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

pub(crate) fn format_runtime_category_list(categories: &[String]) -> String {
    if categories.is_empty() {
        "—".to_string()
    } else {
        categories.join(", ")
    }
}

fn release_review_historical_audit_attribution_label(
    baseline_matches: bool,
    candidate_matches: bool,
) -> &'static str {
    match (baseline_matches, candidate_matches) {
        (true, true) => "both_baseline_and_candidate",
        (true, false) => "baseline_shared_weakness",
        (false, true) => "candidate_regression",
        (false, false) => "unclassified",
    }
}

fn release_review_historical_audit_action_type(attribution: &str) -> &'static str {
    match attribution {
        "candidate_regression" => "candidate_reject_or_retrain",
        "both_baseline_and_candidate" => "shared_blocker_fix_before_promotion",
        "baseline_shared_weakness" => "baseline_research_fix",
        _ => "manual_review",
    }
}

fn release_review_historical_attribution_priority(attribution: &str) -> u8 {
    match attribution {
        "both_baseline_and_candidate" => 0,
        "candidate_regression" => 1,
        "baseline_shared_weakness" => 2,
        _ => 3,
    }
}

fn release_review_historical_action_priority(action_type: &str) -> u8 {
    match action_type {
        "candidate_reject_or_retrain" => 0,
        "shared_blocker_fix_before_promotion" => 1,
        "baseline_research_fix" => 2,
        _ => 3,
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
    let scenario_text = format_runtime_category_list(scenarios);
    match attribution {
        "both_baseline_and_candidate" => format!(
            "{scenario_count} 个样本在 baseline 和 candidate 都落在 {workstream_label}，涉及 {scenario_text}。这说明它既是 formal main 的共性短板，也是 candidate 当前仍未修复的问题。"
        ),
        "candidate_regression" => format!(
            "{scenario_count} 个样本主要是 candidate 新落入 {workstream_label}，涉及 {scenario_text}。baseline 在同一条线没有对应失败，这更像这次候选版本自己退化出来的问题。"
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
        _ if scenario.family == CrisisScenarioFamily::MixedSystemicStress => {
            "继续逐点复核 mixed_systemic_stress 的 runtime block mix，确认问题更像 gate、continuity，还是 residual review clause。"
        }
        _ => {
            "继续逐点复核 runtime block mix 与 continuity facets，确认未分类阻塞到底落在 gate、continuity 还是 residual review clause。"
        }
    }
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

pub(crate) fn build_release_runtime_review_diagnostics(
    release_id: &str,
    label_version: &str,
    method: &AuditMethodResponseWire,
    history: &[AssessmentHistoryPoint],
) -> ReleaseRuntimeReviewDiagnostics {
    let posture_distribution =
        summarize_named_counts(history.iter().map(|point| match point.posture {
            fc_domain::DecisionPosture::Normal => "normal",
            fc_domain::DecisionPosture::Prepare => "prepare",
            fc_domain::DecisionPosture::Hedge => "hedge",
            fc_domain::DecisionPosture::Defend => "defend",
        }));
    let time_bucket_distribution =
        summarize_named_counts(history.iter().map(|point| match point.time_to_risk_bucket {
            fc_domain::TimeToRiskBucket::Normal => "normal",
            fc_domain::TimeToRiskBucket::Months => "months",
            fc_domain::TimeToRiskBucket::Weeks => "weeks",
            fc_domain::TimeToRiskBucket::Now => "now",
        }));
    let posture_trigger_distribution =
        summarize_posture_clause_counts(history, |point| &point.posture_trigger_codes);
    let posture_blocker_distribution =
        summarize_posture_clause_counts(history, |point| &point.posture_blocker_codes);
    let (
        points_at_or_above_prepare_p60d,
        points_at_or_above_hedge_p20d,
        points_at_or_above_defend_p5d,
        mut notes,
    ) = if let Some(thresholds) = method.runtime_thresholds.as_ref() {
        (
            Some(
                history
                    .iter()
                    .filter(|point| point.p_60d >= thresholds.prepare_p60d)
                    .count(),
            ),
            Some(
                history
                    .iter()
                    .filter(|point| point.p_20d >= thresholds.hedge_p20d)
                    .count(),
            ),
            Some(
                history
                    .iter()
                    .filter(|point| point.p_5d >= thresholds.defend_p5d)
                    .count(),
            ),
            vec!["基于运行中 API 返回的 runtime_thresholds 统计历史概率越线次数。".to_string()],
        )
    } else {
        (
            None,
            None,
            None,
            vec![
                "运行中的 API 没有返回 runtime_thresholds；本报告只保留 posture / time bucket 分布。"
                    .to_string(),
            ],
        )
    };
    let regime_probability_summaries = match load_release_review_regime_scenarios(label_version) {
        Ok((scenarios, scenario_note)) => {
            notes.push(scenario_note);
            summarize_release_runtime_regime_probabilities(
                history,
                &scenarios,
                method.runtime_thresholds.as_ref(),
            )
        }
        Err(error) => {
            notes.push(format!(
                "未能加载 release review 所需的 regime scenario catalog，跳过 regime 概率分布：{error:#}"
            ));
            Vec::new()
        }
    };
    let regime_separation_summaries =
        summarize_release_runtime_regime_separation(&regime_probability_summaries);
    if !regime_separation_summaries.is_empty() {
        notes.push(render_release_runtime_separation_note(
            &regime_separation_summaries,
        ));
    }

    ReleaseRuntimeReviewDiagnostics {
        release_id: release_id.to_string(),
        history_point_count: history.len(),
        posture_distribution,
        time_bucket_distribution,
        posture_trigger_distribution,
        posture_blocker_distribution,
        regime_probability_summaries,
        regime_separation_summaries,
        runtime_thresholds: method.runtime_thresholds.clone(),
        points_at_or_above_prepare_p60d,
        points_at_or_above_hedge_p20d,
        points_at_or_above_defend_p5d,
        note: notes.join(" "),
    }
}

fn load_release_review_regime_scenarios(
    label_version: &str,
) -> Result<(Vec<CrisisScenario>, String)> {
    match load_label_set_crisis_scenarios(DEFAULT_FORMAL_SCENARIO_SET_VERSION, label_version) {
        Ok(scenarios) => Ok((
            scenarios,
            format!(
                "Regime 概率分布基于 {DEFAULT_FORMAL_SCENARIO_SET_VERSION}/{label_version} 重算。"
            ),
        )),
        Err(primary_error) if label_version == "label_forward_crisis_v1" => {
            let fallback = load_label_set_crisis_scenarios(
                DEFAULT_FORMAL_SCENARIO_SET_VERSION,
                DEFAULT_FORMAL_LABEL_VERSION,
            )?;
            Ok((
                fallback,
                format!(
                    "当前 release label_version={label_version} 不在 scenario catalog 中，Regime 概率分布回退到 {DEFAULT_FORMAL_SCENARIO_SET_VERSION}/{DEFAULT_FORMAL_LABEL_VERSION} 重算（原始错误：{primary_error:#}）。"
                ),
            ))
        }
        Err(error) => Err(error),
    }
}

pub(crate) fn summarize_release_runtime_regime_probabilities(
    history: &[AssessmentHistoryPoint],
    scenarios: &[CrisisScenario],
    runtime_thresholds: Option<&RuntimeThresholdDiagnosticsWire>,
) -> Vec<ReleaseRuntimeRegimeProbabilitySummary> {
    #[derive(Default)]
    struct Accumulator {
        row_count: usize,
        raw_probability_sum: f64,
        max_raw_probability: f64,
        calibrated_probability_sum: f64,
        max_calibrated_probability: f64,
        threshold_hit_count: usize,
    }

    let mut buckets = BTreeMap::<(u32, String), Accumulator>::new();
    for point in history {
        for (horizon_days, raw_probability, calibrated_probability) in [
            (5_u32, point.raw_p_5d.unwrap_or(point.p_5d), point.p_5d),
            (20_u32, point.raw_p_20d.unwrap_or(point.p_20d), point.p_20d),
            (60_u32, point.raw_p_60d.unwrap_or(point.p_60d), point.p_60d),
        ] {
            let regime = probability_training_regime_name(forward_crisis_training_regime(
                point.as_of_date,
                scenarios,
                horizon_days,
            ));
            let bucket = buckets
                .entry((horizon_days, regime.to_string()))
                .or_default();
            bucket.row_count += 1;
            bucket.raw_probability_sum += raw_probability;
            bucket.max_raw_probability = bucket.max_raw_probability.max(raw_probability);
            bucket.calibrated_probability_sum += calibrated_probability;
            bucket.max_calibrated_probability = bucket
                .max_calibrated_probability
                .max(calibrated_probability);
            if let Some(threshold) =
                runtime_probability_threshold_for_horizon(runtime_thresholds, horizon_days)
            {
                if calibrated_probability >= threshold {
                    bucket.threshold_hit_count += 1;
                }
            }
        }
    }

    let normal_baselines = buckets
        .iter()
        .filter_map(|((horizon_days, regime), bucket)| {
            if regime != "normal" {
                return None;
            }
            Some((
                *horizon_days,
                (
                    safe_divide(bucket.raw_probability_sum, bucket.row_count as f64),
                    safe_divide(bucket.calibrated_probability_sum, bucket.row_count as f64),
                ),
            ))
        })
        .collect::<BTreeMap<_, _>>();

    buckets
        .into_iter()
        .map(|((horizon_days, regime), bucket)| {
            let avg_raw_probability =
                safe_divide(bucket.raw_probability_sum, bucket.row_count as f64);
            let avg_calibrated_probability =
                safe_divide(bucket.calibrated_probability_sum, bucket.row_count as f64);
            let (
                raw_lift_vs_normal,
                calibrated_lift_vs_normal,
                raw_gap_vs_normal,
                calibrated_gap_vs_normal,
                calibration_gap_retention,
            ) = if let Some((normal_avg_raw, normal_avg_calibrated)) =
                normal_baselines.get(&horizon_days).copied()
            {
                let raw_gap = avg_raw_probability - normal_avg_raw;
                let calibrated_gap = avg_calibrated_probability - normal_avg_calibrated;
                (
                    lift_vs_baseline(avg_raw_probability, normal_avg_raw),
                    lift_vs_baseline(avg_calibrated_probability, normal_avg_calibrated),
                    Some(round6(raw_gap)),
                    Some(round6(calibrated_gap)),
                    gap_retention_ratio(raw_gap, calibrated_gap),
                )
            } else {
                (None, None, None, None, None)
            };

            ReleaseRuntimeRegimeProbabilitySummary {
                horizon_days,
                regime,
                row_count: bucket.row_count,
                row_rate: round6(safe_ratio(bucket.row_count, history.len())),
                avg_raw_probability: round6(avg_raw_probability),
                max_raw_probability: round6(bucket.max_raw_probability),
                avg_probability: round6(avg_calibrated_probability),
                max_probability: round6(bucket.max_calibrated_probability),
                raw_lift_vs_normal,
                calibrated_lift_vs_normal,
                raw_gap_vs_normal,
                calibrated_gap_vs_normal,
                calibration_gap_retention,
                threshold_hit_count: runtime_thresholds.map(|_| bucket.threshold_hit_count),
            }
        })
        .collect()
}

pub(crate) fn summarize_release_runtime_regime_separation(
    summaries: &[ReleaseRuntimeRegimeProbabilitySummary],
) -> Vec<ReleaseRuntimeSeparationSummary> {
    let mut by_horizon = BTreeMap::<u32, Vec<&ReleaseRuntimeRegimeProbabilitySummary>>::new();
    for summary in summaries {
        by_horizon
            .entry(summary.horizon_days)
            .or_default()
            .push(summary);
    }

    by_horizon
        .into_iter()
        .filter_map(|(horizon_days, rows)| {
            let normal = rows.iter().copied().find(|row| row.regime == "normal")?;
            let pre_warning_buffer = rows
                .iter()
                .copied()
                .find(|row| row.regime == "pre_warning_buffer");
            let positive_window = rows
                .iter()
                .copied()
                .find(|row| row.regime == "positive_window");
            let max_non_normal = rows
                .iter()
                .copied()
                .filter(|row| row.regime != "normal")
                .max_by(|left, right| {
                    left.avg_probability
                        .partial_cmp(&right.avg_probability)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })?;
            let early_warning_regime_name = early_warning_regime_name(horizon_days);
            let early_warning = rows
                .iter()
                .copied()
                .find(|row| row.regime == early_warning_regime_name);
            let in_crisis = rows.iter().copied().find(|row| row.regime == "in_crisis");
            let post_crisis_cooldown = rows
                .iter()
                .copied()
                .find(|row| row.regime == "post_crisis_cooldown");
            let max_non_normal_threshold_hit_rate = max_non_normal
                .threshold_hit_count
                .map(|count| round6(safe_divide(count as f64, max_non_normal.row_count as f64)));
            let diagnosis = classify_regime_separation(
                horizon_days,
                early_warning
                    .and_then(|row| row.raw_lift_vs_normal)
                    .unwrap_or_default(),
                early_warning
                    .and_then(|row| row.calibrated_lift_vs_normal)
                    .unwrap_or_default(),
                early_warning.and_then(|row| row.calibration_gap_retention),
                positive_window
                    .and_then(|row| row.calibrated_lift_vs_normal)
                    .unwrap_or_default(),
                positive_window
                    .and_then(|row| row.calibrated_gap_vs_normal)
                    .unwrap_or_default(),
                in_crisis
                    .and_then(|row| row.calibrated_lift_vs_normal)
                    .unwrap_or_default(),
                post_crisis_cooldown
                    .and_then(|row| row.calibrated_lift_vs_normal)
                    .unwrap_or_default(),
                post_crisis_cooldown
                    .and_then(|row| row.calibrated_gap_vs_normal)
                    .unwrap_or_default(),
                max_non_normal.calibrated_lift_vs_normal.unwrap_or_default(),
                max_non_normal_threshold_hit_rate.unwrap_or_default(),
            )
            .to_string();

            Some(ReleaseRuntimeSeparationSummary {
                horizon_days,
                early_warning_regime: early_warning_regime_name.to_string(),
                normal_avg_probability: normal.avg_probability,
                pre_warning_buffer_avg_probability: pre_warning_buffer
                    .map(|row| row.avg_probability)
                    .unwrap_or_default(),
                positive_window_avg_probability: positive_window
                    .map(|row| row.avg_probability)
                    .unwrap_or_default(),
                in_crisis_avg_probability: in_crisis
                    .map(|row| row.avg_probability)
                    .unwrap_or_default(),
                post_crisis_cooldown_avg_probability: post_crisis_cooldown
                    .map(|row| row.avg_probability)
                    .unwrap_or_default(),
                early_warning_raw_lift_vs_normal: early_warning
                    .and_then(|row| row.raw_lift_vs_normal),
                early_warning_calibrated_lift_vs_normal: early_warning
                    .and_then(|row| row.calibrated_lift_vs_normal),
                early_warning_gap_retention: early_warning
                    .and_then(|row| row.calibration_gap_retention),
                positive_window_calibrated_lift_vs_normal: positive_window
                    .and_then(|row| row.calibrated_lift_vs_normal),
                positive_window_gap_vs_normal: positive_window
                    .and_then(|row| row.calibrated_gap_vs_normal),
                in_crisis_raw_lift_vs_normal: in_crisis.and_then(|row| row.raw_lift_vs_normal),
                in_crisis_calibrated_lift_vs_normal: in_crisis
                    .and_then(|row| row.calibrated_lift_vs_normal),
                post_crisis_cooldown_calibrated_lift_vs_normal: post_crisis_cooldown
                    .and_then(|row| row.calibrated_lift_vs_normal),
                post_crisis_cooldown_gap_vs_normal: post_crisis_cooldown
                    .and_then(|row| row.calibrated_gap_vs_normal),
                max_non_normal_calibrated_lift_vs_normal: max_non_normal.calibrated_lift_vs_normal,
                max_non_normal_threshold_hit_rate,
                diagnosis,
            })
        })
        .collect()
}

fn early_warning_regime_name(horizon_days: u32) -> &'static str {
    match horizon_days {
        5 => "positive_window",
        20 | 60 => "pre_warning_buffer",
        _ => "positive_window",
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn classify_regime_separation(
    horizon_days: u32,
    early_warning_raw_lift: f64,
    early_warning_calibrated_lift: f64,
    early_warning_gap_retention: Option<f64>,
    positive_window_calibrated_lift: f64,
    positive_window_gap_vs_normal: f64,
    in_crisis_calibrated_lift: f64,
    post_crisis_cooldown_calibrated_lift: f64,
    post_crisis_cooldown_gap_vs_normal: f64,
    max_non_normal_calibrated_lift: f64,
    max_non_normal_threshold_hit_rate: f64,
) -> &'static str {
    if max_non_normal_calibrated_lift < 1.15
        && early_warning_raw_lift < 1.15
        && positive_window_calibrated_lift < 1.15
    {
        return "cold_across_all_regimes";
    }
    if early_warning_raw_lift >= 1.5
        && early_warning_calibrated_lift < 1.15
        && early_warning_gap_retention.unwrap_or_default() < 0.35
    {
        return "calibration_crushed_early_warning";
    }
    if positive_window_calibrated_lift < 1.15 && in_crisis_calibrated_lift >= 1.5 {
        return "late_only_no_early_warning";
    }
    if positive_window_calibrated_lift >= 1.15
        && post_crisis_cooldown_calibrated_lift >= positive_window_calibrated_lift
        && post_crisis_cooldown_gap_vs_normal + 0.002 >= positive_window_gap_vs_normal
    {
        return "cooldown_bleed";
    }
    if max_non_normal_calibrated_lift >= 1.5 && max_non_normal_threshold_hit_rate <= 0.01 {
        return "separated_but_below_runtime_floor";
    }
    if positive_window_calibrated_lift >= 1.5
        && positive_window_gap_vs_normal >= regime_positive_window_gap_floor(horizon_days)
    {
        return "usable_early_warning_separation";
    }
    if max_non_normal_calibrated_lift >= 1.15 || early_warning_calibrated_lift >= 1.15 {
        return "weak_regime_separation";
    }
    "mixed_or_unclear"
}

fn render_release_runtime_separation_note(summaries: &[ReleaseRuntimeSeparationSummary]) -> String {
    let joined = summaries
        .iter()
        .map(|summary| format!("{}d={}", summary.horizon_days, summary.diagnosis))
        .collect::<Vec<_>>()
        .join(", ");
    format!("Runtime separation summary: {joined}.")
}

pub(crate) fn lift_vs_baseline(value: f64, baseline: f64) -> Option<f64> {
    if baseline.abs() <= f64::EPSILON {
        return None;
    }
    Some(round6(value / baseline))
}

fn gap_retention_ratio(raw_gap: f64, calibrated_gap: f64) -> Option<f64> {
    if raw_gap.abs() <= f64::EPSILON {
        return None;
    }
    Some(round6(calibrated_gap / raw_gap))
}

fn runtime_probability_threshold_for_horizon(
    runtime_thresholds: Option<&RuntimeThresholdDiagnosticsWire>,
    horizon_days: u32,
) -> Option<f64> {
    runtime_thresholds.map(|thresholds| match horizon_days {
        5 => thresholds.defend_p5d,
        20 => thresholds.hedge_p20d,
        60 => thresholds.prepare_p60d,
        _ => 1.0,
    })
}

fn summarize_named_counts<'a>(names: impl Iterator<Item = &'a str>) -> Vec<ReleaseRuntimeCount> {
    let mut counts = BTreeMap::<String, usize>::new();
    for name in names {
        *counts.entry(name.to_string()).or_default() += 1;
    }
    let mut rows = counts
        .into_iter()
        .map(|(name, count)| ReleaseRuntimeCount { name, count })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.name.cmp(&right.name))
    });
    rows
}

fn summarize_posture_clause_counts<F>(
    history: &[AssessmentHistoryPoint],
    accessor: F,
) -> Vec<ReleaseRuntimeClauseCount>
where
    F: Fn(&AssessmentHistoryPoint) -> &[String],
{
    let posture_totals = history
        .iter()
        .fold(BTreeMap::<String, usize>::new(), |mut acc, point| {
            *acc.entry(runtime_posture_name(point).to_string())
                .or_default() += 1;
            acc
        });
    let mut counts = BTreeMap::<(String, String), usize>::new();
    for point in history {
        let posture = runtime_posture_name(point).to_string();
        for clause in accessor(point) {
            *counts.entry((posture.clone(), clause.clone())).or_default() += 1;
        }
    }

    let mut rows = counts
        .into_iter()
        .map(|((posture, clause), count)| {
            let posture_total = posture_totals.get(&posture).copied().unwrap_or_default();
            ReleaseRuntimeClauseCount {
                posture,
                clause,
                count,
                share_of_posture: round6(safe_ratio(count, posture_total)),
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.posture.cmp(&right.posture))
            .then_with(|| left.clause.cmp(&right.clause))
    });
    rows
}

fn runtime_posture_name(point: &AssessmentHistoryPoint) -> &'static str {
    match point.posture {
        fc_domain::DecisionPosture::Normal => "normal",
        fc_domain::DecisionPosture::Prepare => "prepare",
        fc_domain::DecisionPosture::Hedge => "hedge",
        fc_domain::DecisionPosture::Defend => "defend",
    }
}

pub(crate) fn render_release_review_markdown(report: &ReleaseReviewEnvelope) -> String {
    reporting::render_release_review_markdown_impl(report)
}

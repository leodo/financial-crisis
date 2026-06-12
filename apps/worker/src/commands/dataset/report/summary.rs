use std::collections::{BTreeMap, BTreeSet};

use anyhow::bail;
use chrono::Utc;
use fc_domain::{
    load_scenario_data_coverage_catalog, ActionabilityLevel, FormalDatasetRecord,
    FormalDatasetRowRecord, ScenarioDataCoverageCatalog,
};

use super::{
    FormalDatasetCoverageCatalogSummary, FormalDatasetFamilySummary, FormalDatasetQualitySummary,
    FormalDatasetRegimeSummary, FormalDatasetScenarioSummary, FormalDatasetSplitSummary,
    FormalDatasetSummaryEnvelope, ScenarioCoverageMetadata, ScenarioSummaryMetadata,
};

pub(crate) fn build_formal_dataset_summary(
    dataset_key: &str,
    dataset: FormalDatasetRecord,
    rows: &[FormalDatasetRowRecord],
) -> anyhow::Result<FormalDatasetSummaryEnvelope> {
    let scenarios = super::super::load_label_set_crisis_scenarios(
        &dataset.manifest.scenario_set_version,
        &dataset.manifest.label_version,
    )?;
    let scenario_metadata =
        load_formal_dataset_scenario_metadata(&dataset.manifest.scenario_set_version)?;
    let coverage_catalog = load_scenario_data_coverage_catalog();
    let scenario_coverage_metadata =
        load_formal_dataset_scenario_coverage_metadata(&coverage_catalog);
    let scenario_ranges = super::super::collect_formal_dataset_scenario_ranges(rows, &scenarios);
    let split_summaries = summarize_formal_dataset_splits(rows, &scenario_ranges);
    let scenario_summaries = summarize_formal_dataset_scenarios(
        rows,
        &scenario_ranges,
        &scenario_metadata,
        &scenario_coverage_metadata,
    );
    let family_summaries = summarize_formal_dataset_families(rows);
    let quality_summaries = summarize_formal_dataset_quality(rows);
    let regime_summaries = summarize_formal_dataset_regimes(rows, &scenarios);
    let coverage_catalog = summarize_coverage_catalog(
        &dataset.manifest.label_version,
        &coverage_catalog,
        &scenario_summaries,
    );
    let recommendation = build_formal_dataset_recommendation(
        &dataset.manifest.label_version,
        &split_summaries,
        &scenario_summaries,
        rows.len(),
    );

    Ok(FormalDatasetSummaryEnvelope {
        generated_at: Utc::now().to_rfc3339(),
        dataset_key: dataset_key.to_string(),
        dataset,
        split_summaries,
        scenario_summaries,
        family_summaries,
        quality_summaries,
        regime_summaries,
        coverage_catalog,
        recommendation,
    })
}

fn summarize_formal_dataset_splits(
    rows: &[FormalDatasetRowRecord],
    scenario_ranges: &[super::super::ScenarioRowRange],
) -> Vec<FormalDatasetSplitSummary> {
    ["train", "calibration", "evaluation"]
        .into_iter()
        .filter_map(|split_name| {
            let split_rows = rows
                .iter()
                .filter(|row| row.split_name == split_name)
                .collect::<Vec<_>>();
            let split_start = rows
                .iter()
                .position(|row| row.split_name == split_name)
                .unwrap_or_default();
            let split_end = rows
                .iter()
                .rposition(|row| row.split_name == split_name)
                .map(|index| index + 1)
                .unwrap_or_default();
            (!split_rows.is_empty()).then(|| FormalDatasetSplitSummary {
                split_name: split_name.to_string(),
                row_count: split_rows.len(),
                positive_5d_count: split_rows.iter().filter(|row| row.label_5d > 0).count(),
                positive_5d_rate: crate::round6(forward_label_rate(&split_rows, 5)),
                positive_20d_count: split_rows.iter().filter(|row| row.label_20d > 0).count(),
                positive_20d_rate: crate::round6(forward_label_rate(&split_rows, 20)),
                positive_60d_count: split_rows.iter().filter(|row| row.label_60d > 0).count(),
                positive_60d_rate: crate::round6(forward_label_rate(&split_rows, 60)),
                prepare_primary_count: split_rows
                    .iter()
                    .filter(|row| row.prepare_episode_label > 0)
                    .count(),
                prepare_primary_rate: crate::round6(action_episode_primary_rate(
                    &split_rows,
                    ActionabilityLevel::Prepare,
                )),
                hedge_primary_count: split_rows
                    .iter()
                    .filter(|row| row.hedge_episode_label > 0)
                    .count(),
                hedge_primary_rate: crate::round6(action_episode_primary_rate(
                    &split_rows,
                    ActionabilityLevel::Hedge,
                )),
                defend_primary_count: split_rows
                    .iter()
                    .filter(|row| row.defend_episode_label > 0)
                    .count(),
                defend_primary_rate: crate::round6(action_episode_primary_rate(
                    &split_rows,
                    ActionabilityLevel::Defend,
                )),
                late_validation_row_count: split_rows
                    .iter()
                    .filter(|row| row.action_episode_phase == "late_validation")
                    .count(),
                late_validation_row_rate: crate::round6(late_validation_row_rate(&split_rows)),
                protected_row_count: split_rows
                    .iter()
                    .filter(|row| row.protected_action_window)
                    .count(),
                protected_row_rate: crate::round6(protected_action_window_rate(&split_rows)),
                avg_coverage_score: crate::round3(avg_metric(&split_rows, |row| {
                    row.coverage_score
                })),
                avg_core_feature_coverage: crate::round3(avg_metric(&split_rows, |row| {
                    row.core_feature_coverage
                })),
                avg_trigger_feature_coverage: crate::round3(avg_metric(&split_rows, |row| {
                    row.trigger_feature_coverage
                })),
                avg_external_feature_coverage: crate::round3(avg_metric(&split_rows, |row| {
                    row.external_feature_coverage
                })),
                scenario_count: super::super::scenario_count_for_split_range(
                    scenario_ranges,
                    split_start,
                    split_end,
                ),
            })
        })
        .collect()
}

fn summarize_formal_dataset_scenarios(
    rows: &[FormalDatasetRowRecord],
    scenario_ranges: &[super::super::ScenarioRowRange],
    scenario_metadata: &BTreeMap<String, ScenarioSummaryMetadata>,
    scenario_coverage_metadata: &BTreeMap<String, ScenarioCoverageMetadata>,
) -> Vec<FormalDatasetScenarioSummary> {
    scenario_ranges
        .iter()
        .map(|range| {
            let metadata = scenario_metadata.get(&range.scenario_id);
            let coverage = scenario_coverage_metadata.get(&range.scenario_id);
            FormalDatasetScenarioSummary {
                scenario_id: range.scenario_id.clone(),
                label: metadata.map(|item| item.label.clone()),
                row_count: range.end_index.saturating_sub(range.start_index) + 1,
                split_count: rows[range.start_index..=range.end_index]
                    .iter()
                    .map(|row| row.split_name.as_str())
                    .collect::<BTreeSet<_>>()
                    .len(),
                first_as_of_date: rows[range.start_index].as_of_date,
                last_as_of_date: rows[range.end_index].as_of_date,
                family: metadata
                    .map(|item| item.family.clone())
                    .or_else(|| Some(range.family.clone())),
                training_role: metadata.map(|item| item.training_role.clone()),
                protected_window: metadata.map(|item| item.protected_window),
                episode_template_id: metadata.map(|item| item.episode_template_id.clone()),
                default_horizon_roles: metadata
                    .map(|item| item.default_horizon_roles.clone())
                    .unwrap_or_default(),
                coverage_recommended_role: coverage.map(|item| item.recommended_role.clone()),
                coverage_grade: coverage.map(|item| item.coverage_grade.clone()),
                coverage_point_in_time_mode: coverage.map(|item| item.point_in_time_mode.clone()),
                coverage_current_status: coverage.map(|item| item.current_status.clone()),
                coverage_blocking_gaps: coverage
                    .map(|item| item.blocking_gaps.clone())
                    .unwrap_or_default(),
                coverage_free_sources: coverage
                    .map(|item| item.free_sources.clone())
                    .unwrap_or_default(),
                usable_for_main_training: coverage.map(|item| item.usable_for_main_training),
                usable_for_extension_training: coverage
                    .map(|item| item.usable_for_extension_training),
                usable_for_protected_stress: coverage.map(|item| item.usable_for_protected_stress),
                usable_for_historical_analog: coverage
                    .map(|item| item.usable_for_historical_analog),
            }
        })
        .collect()
}

fn summarize_formal_dataset_families(
    rows: &[FormalDatasetRowRecord],
) -> Vec<FormalDatasetFamilySummary> {
    let mut buckets = BTreeMap::<String, Vec<&FormalDatasetRowRecord>>::new();
    for row in rows.iter().filter(|row| row.scenario_family.is_some()) {
        let family = row.scenario_family.clone().unwrap_or_default();
        buckets.entry(family).or_default().push(row);
    }

    buckets
        .into_iter()
        .map(|(family, family_rows)| FormalDatasetFamilySummary {
            row_count: family_rows.len(),
            scenario_count: family_rows
                .iter()
                .filter_map(|row| row.primary_scenario_id.as_ref())
                .collect::<BTreeSet<_>>()
                .len(),
            family,
        })
        .collect()
}

fn summarize_formal_dataset_quality(
    rows: &[FormalDatasetRowRecord],
) -> Vec<FormalDatasetQualitySummary> {
    let mut buckets = BTreeMap::<String, usize>::new();
    for row in rows {
        *buckets.entry(row.sample_quality_grade.clone()).or_default() += 1;
    }
    buckets
        .into_iter()
        .map(|(grade, row_count)| FormalDatasetQualitySummary { grade, row_count })
        .collect()
}

fn summarize_formal_dataset_regimes(
    rows: &[FormalDatasetRowRecord],
    scenarios: &[crate::CrisisScenario],
) -> Vec<FormalDatasetRegimeSummary> {
    let split_totals = rows
        .iter()
        .fold(BTreeMap::<String, usize>::new(), |mut acc, row| {
            *acc.entry(row.split_name.clone()).or_default() += 1;
            acc
        });
    let mut buckets = BTreeMap::<(String, u32, String), usize>::new();
    for row in rows {
        for horizon_days in [5_u32, 20_u32, 60_u32] {
            let regime = crate::probability_training_regime_name(
                crate::forward_crisis_training_regime(row.as_of_date, scenarios, horizon_days),
            );
            *buckets
                .entry((row.split_name.clone(), horizon_days, regime.to_string()))
                .or_default() += 1;
        }
    }

    buckets
        .into_iter()
        .map(|((split_name, horizon_days, regime), row_count)| {
            let split_total = split_totals.get(&split_name).copied().unwrap_or_default();
            FormalDatasetRegimeSummary {
                split_name,
                horizon_days,
                regime,
                row_count,
                row_rate: crate::round6(crate::safe_ratio(row_count, split_total)),
            }
        })
        .collect()
}

fn build_formal_dataset_recommendation(
    label_version: &str,
    split_summaries: &[FormalDatasetSplitSummary],
    scenario_summaries: &[FormalDatasetScenarioSummary],
    total_rows: usize,
) -> String {
    let evaluation = split_summaries
        .iter()
        .find(|split| split.split_name == "evaluation");
    let coverage_mismatches = scenario_coverage_mismatches(label_version, scenario_summaries);
    if !coverage_mismatches.is_empty() {
        return format!(
            "当前数据集包含 {} 个与 {} 覆盖口径不一致的场景：{}。应先修正场景角色或数据覆盖配置，再拿这版数据集训练或做 release review。",
            coverage_mismatches.len(),
            dataset_intent_text(label_version),
            coverage_mismatches.join("、")
        );
    }
    if total_rows < 5_000 {
        return "样本量仍偏小，先继续补历史数据，再用这版数据集训练正式候选版。".to_string();
    }
    match super::super::formal_dataset_split_profile(label_version) {
        super::super::FormalDatasetSplitProfile::ExtensionAcute => {
            let Some(evaluation) = evaluation else {
                return "缺少 evaluation split，当前还不能稳定比较 1987/1998 的急性冲击表现。"
                    .to_string();
            };
            if evaluation.scenario_count < 1 || evaluation.defend_primary_count == 0 {
                return "evaluation 仍未覆盖足够的 acute 尾段主正例，先继续重做 split 或补齐 1987/1998 proxy 覆盖。".to_string();
            }
            if evaluation.prepare_primary_count == 0 || evaluation.hedge_primary_count == 0 {
                return "这套扩展 acute 数据集已经能用于 1987/1998 的 5d/20d 与急性尾段类比，但 evaluation 还不足以单独评估完整的 prepare/hedge/defend episode 头。".to_string();
            }
            return "这套扩展 acute 数据集已经可以用于 1987/1998 的 5d/20d 历史类比与短窗研究；它是研究包，不应用作正式主模型上线判断。".to_string();
        }
        super::super::FormalDatasetSplitProfile::ExtensionStress => {
            let Some(evaluation) = evaluation else {
                return "缺少 evaluation split，当前还不能稳定比较 protected stress 扩展场景。"
                    .to_string();
            };
            if evaluation.scenario_count < 1
                || evaluation.prepare_primary_count == 0
                || evaluation.hedge_primary_count == 0
            {
                return "evaluation 的 protected stress / extension 主正例仍偏少，先继续重做 split，再把它用于扩展研究和 posture 对照。".to_string();
            }
            if evaluation.protected_row_count < 1 {
                return "evaluation 还没有 protected stress 尾段样本，当前不适合拿它判断受保护压力窗口是否稳定。".to_string();
            }
            return "这套扩展 stress 数据集已经可以用于 protected stress、历史对照和扩展训练研究；它不是正式主模型 go/no-go 的单独依据。".to_string();
        }
        super::super::FormalDatasetSplitProfile::Main => {}
    }
    if let Some(evaluation) = evaluation {
        if evaluation.hedge_primary_count < 10 || evaluation.prepare_primary_count < 10 {
            return "evaluation 的 episode-native 主正例仍偏少，当前更适合作研究版比较，不适合直接给正式模型做上线判断。".to_string();
        }
        if evaluation.late_validation_row_count < 5 {
            return "evaluation 的 late-validation 样本仍偏少，动作头很难判断“过晚确认”到底是偶然还是系统性问题。".to_string();
        }
        if evaluation.protected_row_count < 5 {
            return "evaluation 的 protected stress 样本仍偏少，当前还不适合把 protected/cooldown 行为当成稳定结论。".to_string();
        }
        if evaluation.scenario_count < 2 {
            return format!(
                "evaluation split 的 episode-native 动作标签目前只覆盖 {} 个场景，动作头评估很不稳；应先扩历史场景或重做 split，再用它判断 formal 候选版优劣。",
                evaluation.scenario_count
            );
        }
        if evaluation.avg_coverage_score < 0.85 {
            return "evaluation 覆盖率偏低，应先补可见性/覆盖率，再看训练结果。".to_string();
        }
    }
    "样本量、split 和覆盖率已具备基础研究条件，可以进入正式训练与 release review。".to_string()
}

fn summarize_coverage_catalog(
    label_version: &str,
    coverage_catalog: &ScenarioDataCoverageCatalog,
    scenario_summaries: &[FormalDatasetScenarioSummary],
) -> FormalDatasetCoverageCatalogSummary {
    FormalDatasetCoverageCatalogSummary {
        catalog_id: coverage_catalog.catalog_id.clone(),
        scenario_catalog_id: coverage_catalog.scenario_catalog_id.clone(),
        market_scope: coverage_catalog.market_scope.clone(),
        source: coverage_catalog.source.clone(),
        warning: coverage_catalog.warning.clone(),
        dataset_intent: dataset_intent_text(label_version).to_string(),
        aligned_scenario_count: scenario_summaries
            .iter()
            .filter(|scenario| scenario_is_aligned_with_dataset(label_version, scenario))
            .count(),
        total_scenario_count: scenario_summaries.len(),
        main_training_eligible_count: scenario_summaries
            .iter()
            .filter(|scenario| scenario.usable_for_main_training == Some(true))
            .count(),
        extension_training_eligible_count: scenario_summaries
            .iter()
            .filter(|scenario| scenario.usable_for_extension_training == Some(true))
            .count(),
        protected_stress_eligible_count: scenario_summaries
            .iter()
            .filter(|scenario| scenario.usable_for_protected_stress == Some(true))
            .count(),
        historical_analog_eligible_count: scenario_summaries
            .iter()
            .filter(|scenario| scenario.usable_for_historical_analog == Some(true))
            .count(),
    }
}

fn forward_label_rate(rows: &[&FormalDatasetRowRecord], horizon_days: u32) -> f64 {
    let positives = rows
        .iter()
        .filter(|row| match horizon_days {
            5 => row.label_5d > 0,
            20 => row.label_20d > 0,
            60 => row.label_60d > 0,
            _ => false,
        })
        .count();
    positives as f64 / rows.len() as f64
}

fn action_episode_primary_rate(rows: &[&FormalDatasetRowRecord], level: ActionabilityLevel) -> f64 {
    let positives = rows
        .iter()
        .filter(|row| super::super::row_has_action_episode_label(row, level))
        .count();
    positives as f64 / rows.len() as f64
}

fn late_validation_row_rate(rows: &[&FormalDatasetRowRecord]) -> f64 {
    let positives = rows
        .iter()
        .filter(|row| row.action_episode_phase == "late_validation")
        .count();
    positives as f64 / rows.len() as f64
}

fn protected_action_window_rate(rows: &[&FormalDatasetRowRecord]) -> f64 {
    let positives = rows
        .iter()
        .filter(|row| row.protected_action_window)
        .count();
    positives as f64 / rows.len() as f64
}

fn avg_metric<F>(rows: &[&FormalDatasetRowRecord], accessor: F) -> f64
where
    F: Fn(&FormalDatasetRowRecord) -> f64,
{
    rows.iter().map(|row| accessor(row)).sum::<f64>() / rows.len() as f64
}

fn dataset_intent_text(label_version: &str) -> &'static str {
    match super::super::formal_dataset_split_profile(label_version) {
        super::super::FormalDatasetSplitProfile::Main => "main_training + protected_context",
        super::super::FormalDatasetSplitProfile::ExtensionAcute
        | super::super::FormalDatasetSplitProfile::ExtensionStress => "extension_training",
    }
}

fn scenario_is_aligned_with_dataset(
    label_version: &str,
    scenario: &FormalDatasetScenarioSummary,
) -> bool {
    match super::super::formal_dataset_split_profile(label_version) {
        super::super::FormalDatasetSplitProfile::Main => {
            scenario.usable_for_main_training.unwrap_or(false)
                || scenario.usable_for_protected_stress.unwrap_or(false)
        }
        super::super::FormalDatasetSplitProfile::ExtensionAcute
        | super::super::FormalDatasetSplitProfile::ExtensionStress => {
            scenario.usable_for_extension_training.unwrap_or(false)
        }
    }
}

fn scenario_coverage_mismatches(
    label_version: &str,
    scenario_summaries: &[FormalDatasetScenarioSummary],
) -> Vec<String> {
    scenario_summaries
        .iter()
        .filter(|scenario| {
            let has_coverage = scenario.usable_for_main_training.is_some()
                || scenario.usable_for_extension_training.is_some();
            has_coverage && !scenario_is_aligned_with_dataset(label_version, scenario)
        })
        .map(|scenario| {
            format!(
                "{} ({})",
                scenario.label.as_deref().unwrap_or(&scenario.scenario_id),
                scenario.scenario_id
            )
        })
        .collect()
}

fn load_formal_dataset_scenario_metadata(
    scenario_set_version: &str,
) -> anyhow::Result<BTreeMap<String, ScenarioSummaryMetadata>> {
    let catalog = crate::load_crisis_scenario_catalog();
    if catalog.catalog_id != scenario_set_version {
        bail!(
            "scenario set version {} is not available in the active catalog {}; set FC_SCENARIO_CATALOG_PATH to another catalog or use {}",
            scenario_set_version,
            catalog.catalog_id,
            catalog.catalog_id
        );
    }

    Ok(catalog
        .scenarios
        .into_iter()
        .map(|scenario| {
            (
                scenario.scenario_id.clone(),
                ScenarioSummaryMetadata {
                    label: scenario.label,
                    family: super::super::scenario_family_code(scenario.family).to_string(),
                    training_role: super::super::scenario_training_role_code(
                        scenario.training_role,
                    )
                    .to_string(),
                    protected_window: scenario.protected_window,
                    episode_template_id: super::super::action_episode_template_code(
                        scenario
                            .episode_template_id
                            .expect("validated scenario catalog must include episode_template_id"),
                    )
                    .to_string(),
                    default_horizon_roles: scenario.default_horizon_roles,
                },
            )
        })
        .collect())
}

fn load_formal_dataset_scenario_coverage_metadata(
    coverage_catalog: &ScenarioDataCoverageCatalog,
) -> BTreeMap<String, ScenarioCoverageMetadata> {
    coverage_catalog
        .records
        .iter()
        .map(|record| {
            (
                record.scenario_id.clone(),
                ScenarioCoverageMetadata {
                    recommended_role: record.recommended_role.clone(),
                    coverage_grade: record.coverage_grade.clone(),
                    point_in_time_mode: record.point_in_time_mode.clone(),
                    current_status: record.current_status.clone(),
                    blocking_gaps: record.blocking_gaps.clone(),
                    free_sources: record.free_sources.clone(),
                    usable_for_main_training: record.usable_for_main_training,
                    usable_for_extension_training: record.usable_for_extension_training,
                    usable_for_protected_stress: record.usable_for_protected_stress,
                    usable_for_historical_analog: record.usable_for_historical_analog,
                },
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        build_formal_dataset_recommendation, scenario_coverage_mismatches,
        scenario_is_aligned_with_dataset,
    };
    use crate::commands::dataset::report::{
        FormalDatasetScenarioSummary, FormalDatasetSplitSummary,
    };
    use chrono::NaiveDate;

    fn sample_split_summary() -> FormalDatasetSplitSummary {
        FormalDatasetSplitSummary {
            split_name: "evaluation".to_string(),
            row_count: 100,
            positive_5d_count: 10,
            positive_5d_rate: 0.1,
            positive_20d_count: 12,
            positive_20d_rate: 0.12,
            positive_60d_count: 15,
            positive_60d_rate: 0.15,
            prepare_primary_count: 12,
            prepare_primary_rate: 0.12,
            hedge_primary_count: 12,
            hedge_primary_rate: 0.12,
            defend_primary_count: 6,
            defend_primary_rate: 0.06,
            late_validation_row_count: 8,
            late_validation_row_rate: 0.08,
            protected_row_count: 10,
            protected_row_rate: 0.10,
            avg_coverage_score: 0.9,
            avg_core_feature_coverage: 0.9,
            avg_trigger_feature_coverage: 0.9,
            avg_external_feature_coverage: 0.9,
            scenario_count: 3,
        }
    }

    fn sample_scenario_summary(
        scenario_id: &str,
        label: &str,
        usable_for_main_training: Option<bool>,
        usable_for_extension_training: Option<bool>,
    ) -> FormalDatasetScenarioSummary {
        FormalDatasetScenarioSummary {
            scenario_id: scenario_id.to_string(),
            label: Some(label.to_string()),
            row_count: 100,
            split_count: 2,
            first_as_of_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
            last_as_of_date: NaiveDate::from_ymd_opt(2020, 2, 1).unwrap(),
            family: Some("mixed_systemic_stress".to_string()),
            training_role: Some("main".to_string()),
            protected_window: Some(false),
            episode_template_id: Some("generic".to_string()),
            default_horizon_roles: vec![20, 60],
            coverage_recommended_role: Some("main_training".to_string()),
            coverage_grade: Some("A".to_string()),
            coverage_point_in_time_mode: Some("best_effort".to_string()),
            coverage_current_status: Some("ready".to_string()),
            coverage_blocking_gaps: Vec::new(),
            coverage_free_sources: vec!["FRED".to_string()],
            usable_for_main_training,
            usable_for_extension_training,
            usable_for_protected_stress: Some(false),
            usable_for_historical_analog: Some(true),
        }
    }

    #[test]
    fn main_dataset_accepts_protected_context_coverage() {
        let mut scenario = sample_scenario_summary(
            "us_bond_massacre_1994",
            "1994 联储加息与债市暴跌",
            Some(false),
            Some(true),
        );
        scenario.usable_for_protected_stress = Some(true);
        assert!(scenario_is_aligned_with_dataset(
            "formal_label_v1_main",
            &scenario
        ));
        let recommendation = build_formal_dataset_recommendation(
            "formal_label_v1_main",
            &[sample_split_summary()],
            &[scenario],
            10_000,
        );
        assert!(!recommendation.contains("覆盖口径不一致"));
    }

    #[test]
    fn main_dataset_rejects_extension_only_scenario_without_protected_context() {
        let scenario = sample_scenario_summary(
            "us_black_monday_1987",
            "1987 黑色星期一",
            Some(false),
            Some(true),
        );
        assert!(!scenario_is_aligned_with_dataset(
            "formal_label_v1_main",
            &scenario
        ));
        let mismatches = scenario_coverage_mismatches("formal_label_v1_main", &[scenario.clone()]);
        assert_eq!(mismatches.len(), 1);
        let recommendation = build_formal_dataset_recommendation(
            "formal_label_v1_main",
            &[sample_split_summary()],
            &[scenario],
            10_000,
        );
        assert!(recommendation.contains("覆盖口径不一致"));
        assert!(recommendation.contains("us_black_monday_1987"));
    }

    #[test]
    fn extension_dataset_accepts_extension_training_coverage() {
        let scenario = sample_scenario_summary(
            "us_black_monday_1987",
            "1987 黑色星期一",
            Some(false),
            Some(true),
        );
        assert!(scenario_is_aligned_with_dataset(
            "formal_label_v1_ext_acute",
            &scenario
        ));
        let recommendation = build_formal_dataset_recommendation(
            "formal_label_v1_ext_acute",
            &[sample_split_summary()],
            &[scenario],
            10_000,
        );
        assert!(!recommendation.contains("覆盖口径不一致"));
    }
}

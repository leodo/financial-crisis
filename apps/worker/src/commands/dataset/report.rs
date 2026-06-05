use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
    fs,
    path::PathBuf,
};

use anyhow::bail;
use chrono::{NaiveDate, Utc};
use fc_domain::{ActionabilityLevel, FormalDatasetRecord, FormalDatasetRowRecord};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct FormalDatasetSplitSummary {
    split_name: String,
    row_count: usize,
    positive_5d_count: usize,
    positive_5d_rate: f64,
    positive_20d_count: usize,
    positive_20d_rate: f64,
    positive_60d_count: usize,
    positive_60d_rate: f64,
    prepare_primary_count: usize,
    prepare_primary_rate: f64,
    hedge_primary_count: usize,
    hedge_primary_rate: f64,
    defend_primary_count: usize,
    defend_primary_rate: f64,
    late_validation_row_count: usize,
    late_validation_row_rate: f64,
    protected_row_count: usize,
    protected_row_rate: f64,
    avg_coverage_score: f64,
    avg_core_feature_coverage: f64,
    avg_trigger_feature_coverage: f64,
    avg_external_feature_coverage: f64,
    scenario_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct FormalDatasetScenarioSummary {
    scenario_id: String,
    label: Option<String>,
    row_count: usize,
    split_count: usize,
    first_as_of_date: NaiveDate,
    last_as_of_date: NaiveDate,
    family: Option<String>,
    training_role: Option<String>,
    protected_window: Option<bool>,
    episode_template_id: Option<String>,
    default_horizon_roles: Vec<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct FormalDatasetFamilySummary {
    family: String,
    row_count: usize,
    scenario_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct FormalDatasetQualitySummary {
    grade: String,
    row_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct FormalDatasetRegimeSummary {
    split_name: String,
    horizon_days: u32,
    regime: String,
    row_count: usize,
    row_rate: f64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct FormalDatasetSummaryEnvelope {
    pub(crate) generated_at: String,
    pub(crate) dataset_key: String,
    pub(crate) dataset: FormalDatasetRecord,
    pub(crate) split_summaries: Vec<FormalDatasetSplitSummary>,
    pub(crate) scenario_summaries: Vec<FormalDatasetScenarioSummary>,
    pub(crate) family_summaries: Vec<FormalDatasetFamilySummary>,
    pub(crate) quality_summaries: Vec<FormalDatasetQualitySummary>,
    pub(crate) regime_summaries: Vec<FormalDatasetRegimeSummary>,
    pub(crate) recommendation: String,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct FormalDatasetSliceExport {
    pub(super) exported_at: String,
    pub(super) dataset_key: String,
    pub(super) dataset: FormalDatasetRecord,
    pub(super) scenario_id: String,
    pub(super) split_name: Option<String>,
    pub(super) from_date: Option<NaiveDate>,
    pub(super) to_date: Option<NaiveDate>,
    pub(super) row_count: usize,
    pub(super) feature_names: Vec<String>,
    pub(super) rows: Vec<FormalDatasetRowRecord>,
}

#[derive(Debug, Clone)]
struct ScenarioSummaryMetadata {
    label: String,
    family: String,
    training_role: String,
    protected_window: bool,
    episode_template_id: String,
    default_horizon_roles: Vec<u32>,
}

pub(crate) fn build_formal_dataset_summary(
    dataset_key: &str,
    dataset: FormalDatasetRecord,
    rows: &[FormalDatasetRowRecord],
) -> anyhow::Result<FormalDatasetSummaryEnvelope> {
    let scenarios = super::load_label_set_crisis_scenarios(
        &dataset.manifest.scenario_set_version,
        &dataset.manifest.label_version,
    )?;
    let scenario_metadata =
        load_formal_dataset_scenario_metadata(&dataset.manifest.scenario_set_version)?;
    let scenario_ranges = super::collect_formal_dataset_scenario_ranges(rows, &scenarios);
    let split_summaries = summarize_formal_dataset_splits(rows, &scenario_ranges);
    let scenario_summaries =
        summarize_formal_dataset_scenarios(rows, &scenario_ranges, &scenario_metadata);
    let family_summaries = summarize_formal_dataset_families(rows);
    let quality_summaries = summarize_formal_dataset_quality(rows);
    let regime_summaries = summarize_formal_dataset_regimes(rows, &scenarios);
    let recommendation = build_formal_dataset_recommendation(
        &dataset.manifest.label_version,
        &split_summaries,
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
        recommendation,
    })
}

fn summarize_formal_dataset_splits(
    rows: &[FormalDatasetRowRecord],
    scenario_ranges: &[super::ScenarioRowRange],
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
                scenario_count: super::scenario_count_for_split_range(
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
    scenario_ranges: &[super::ScenarioRowRange],
    scenario_metadata: &BTreeMap<String, ScenarioSummaryMetadata>,
) -> Vec<FormalDatasetScenarioSummary> {
    scenario_ranges
        .iter()
        .map(|range| {
            let metadata = scenario_metadata.get(&range.scenario_id);
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
    total_rows: usize,
) -> String {
    let evaluation = split_summaries
        .iter()
        .find(|split| split.split_name == "evaluation");
    if total_rows < 5_000 {
        return "样本量仍偏小，先继续补历史数据，再用这版数据集训练正式候选版。".to_string();
    }
    match super::formal_dataset_split_profile(label_version) {
        super::FormalDatasetSplitProfile::ExtensionAcute => {
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
        super::FormalDatasetSplitProfile::ExtensionStress => {
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
        super::FormalDatasetSplitProfile::Main => {}
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
        .filter(|row| super::row_has_action_episode_label(row, level))
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

pub(super) fn build_formal_dataset_slice_export(
    dataset_key: String,
    dataset: FormalDatasetRecord,
    rows: Vec<FormalDatasetRowRecord>,
    options: &super::FormalDatasetSliceOptions,
) -> anyhow::Result<FormalDatasetSliceExport> {
    let rows = filter_formal_dataset_rows_for_slice(rows, options);
    if rows.is_empty() {
        bail!(
            "formal dataset slice is empty (dataset_key={}, scenario_id={}, split_name={}, from={}, to={})",
            dataset_key,
            options.scenario_id,
            options.split_name.as_deref().unwrap_or("-"),
            options
                .from_date
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
            options
                .to_date
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string())
        );
    }

    let feature_names = collect_formal_dataset_slice_feature_names(&rows);
    Ok(FormalDatasetSliceExport {
        exported_at: Utc::now().to_rfc3339(),
        dataset_key,
        dataset,
        scenario_id: options.scenario_id.clone(),
        split_name: options.split_name.clone(),
        from_date: options.from_date,
        to_date: options.to_date,
        row_count: rows.len(),
        feature_names,
        rows,
    })
}

fn filter_formal_dataset_rows_for_slice(
    rows: Vec<FormalDatasetRowRecord>,
    options: &super::FormalDatasetSliceOptions,
) -> Vec<FormalDatasetRowRecord> {
    let mut filtered = rows
        .into_iter()
        .filter(|row| row.primary_scenario_id.as_deref() == Some(options.scenario_id.as_str()))
        .filter(|row| {
            options
                .from_date
                .map(|from_date| row.as_of_date >= from_date)
                .unwrap_or(true)
        })
        .filter(|row| {
            options
                .to_date
                .map(|to_date| row.as_of_date <= to_date)
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();
    filtered.sort_by(|left, right| {
        left.as_of_date
            .cmp(&right.as_of_date)
            .then_with(|| left.split_name.cmp(&right.split_name))
    });
    if let Some(limit) = options.limit {
        filtered.truncate(limit);
    }
    filtered
}

fn collect_formal_dataset_slice_feature_names(rows: &[FormalDatasetRowRecord]) -> Vec<String> {
    rows.iter()
        .flat_map(|row| row.features.keys().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub(crate) fn render_formal_dataset_slice_csv(
    rows: &[FormalDatasetRowRecord],
    feature_names: &[String],
) -> String {
    let mut header = String::from(
        "dataset_key,split_name,as_of_date,entity_id,market_scope,primary_scenario_id,scenario_family,scenario_training_role,label_5d,label_20d,label_60d,regime_5d,regime_20d,regime_60d,action_label_5d,action_label_20d,action_label_60d,prepare_episode_label,hedge_episode_label,defend_episode_label,primary_action_level,action_episode_id,action_episode_phase,protected_action_window,coverage_score,core_feature_coverage,trigger_feature_coverage,external_feature_coverage,sample_quality_grade,latest_visible_at",
    );
    for feature_name in feature_names {
        header.push(',');
        header.push_str(feature_name);
    }
    header.push('\n');

    let mut csv = header;
    for row in rows {
        let columns = [
            row.dataset_key.clone(),
            row.split_name.clone(),
            row.as_of_date.to_string(),
            row.entity_id.clone(),
            row.market_scope.clone(),
            row.primary_scenario_id.clone().unwrap_or_default(),
            row.scenario_family.clone().unwrap_or_default(),
            row.scenario_training_role.clone().unwrap_or_default(),
            row.label_5d.to_string(),
            row.label_20d.to_string(),
            row.label_60d.to_string(),
            row.regime_5d.clone(),
            row.regime_20d.clone(),
            row.regime_60d.clone(),
            row.action_label_5d.to_string(),
            row.action_label_20d.to_string(),
            row.action_label_60d.to_string(),
            row.prepare_episode_label.to_string(),
            row.hedge_episode_label.to_string(),
            row.defend_episode_label.to_string(),
            row.primary_action_level.clone().unwrap_or_default(),
            row.action_episode_id.clone().unwrap_or_default(),
            row.action_episode_phase.clone(),
            (row.protected_action_window as u8).to_string(),
            format!("{:.4}", row.coverage_score),
            format!("{:.4}", row.core_feature_coverage),
            format!("{:.4}", row.trigger_feature_coverage),
            format!("{:.4}", row.external_feature_coverage),
            row.sample_quality_grade.clone(),
            row.latest_visible_at
                .map(|value| value.to_rfc3339())
                .unwrap_or_default(),
        ];
        csv.push_str(&columns.join(","));
        for feature_name in feature_names {
            let value = row.features.get(feature_name).copied().unwrap_or_default();
            let _ = write!(csv, ",{value:.6}");
        }
        csv.push('\n');
    }
    csv
}

pub(crate) fn sanitize_filename_component(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => ch,
            _ => '_',
        })
        .collect()
}

pub(super) fn write_formal_dataset_slice_report(
    output_dir: &PathBuf,
    export: &FormalDatasetSliceExport,
) -> anyhow::Result<()> {
    fs::create_dir_all(output_dir)?;
    let mut stem = format!(
        "{}-{}-slice",
        sanitize_filename_component(&export.dataset_key),
        sanitize_filename_component(&export.scenario_id)
    );
    if let Some(split_name) = export.split_name.as_deref() {
        let _ = write!(stem, "-{}", sanitize_filename_component(split_name));
    }
    if let Some(from_date) = export.from_date {
        let _ = write!(stem, "-from-{from_date}");
    }
    if let Some(to_date) = export.to_date {
        let _ = write!(stem, "-to-{to_date}");
    }
    let json_path = output_dir.join(format!("{stem}.json"));
    let csv_path = output_dir.join(format!("{stem}.csv"));
    fs::write(&json_path, serde_json::to_string_pretty(export)?)?;
    fs::write(
        &csv_path,
        render_formal_dataset_slice_csv(&export.rows, &export.feature_names),
    )?;
    println!("Formal dataset slice exported.");
    println!("  JSON {}", json_path.display());
    println!("  CSV  {}", csv_path.display());
    Ok(())
}

pub(crate) fn render_formal_dataset_summary_markdown(
    summary: &FormalDatasetSummaryEnvelope,
) -> String {
    let mut markdown = String::new();
    let manifest = &summary.dataset.manifest;
    let _ = writeln!(markdown, "# Formal Dataset Summary");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "- Generated at: {}", summary.generated_at);
    let _ = writeln!(markdown, "- Dataset key: {}", summary.dataset_key);
    let _ = writeln!(markdown, "- Market scope: {}", manifest.market_scope);
    let _ = writeln!(markdown, "- Feature set: {}", manifest.feature_set_version);
    let _ = writeln!(markdown, "- Label version: {}", manifest.label_version);
    let _ = writeln!(
        markdown,
        "- Scenario set: {}",
        manifest.scenario_set_version
    );
    let _ = writeln!(markdown, "- PIT mode: {}", manifest.point_in_time_mode);
    let _ = writeln!(markdown, "- Rows: {}", manifest.row_count);
    let _ = writeln!(
        markdown,
        "- Range: {} -> {}",
        manifest
            .from_date
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string()),
        manifest
            .to_date
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string())
    );
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Split Summary");
    let _ = writeln!(markdown);
    let _ = writeln!(
        markdown,
        "| Split | Rows | Forward 5d+ | Forward 20d+ | Forward 60d+ | Prepare Primary | Hedge Primary | Defend Primary | Late Validation | Protected | Avg Coverage | Core | Trigger | External | Scenarios |"
    );
    let _ = writeln!(
        markdown,
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
    );
    for split in &summary.split_summaries {
        let _ = writeln!(
            markdown,
            "| {} | {} | {} ({}) | {} ({}) | {} ({}) | {} ({}) | {} ({}) | {} ({}) | {} ({}) | {} ({}) | {:.1}% | {:.1}% | {:.1}% | {:.1}% | {} |",
            split.split_name,
            split.row_count,
            split.positive_5d_count,
            crate::format_pct(split.positive_5d_rate),
            split.positive_20d_count,
            crate::format_pct(split.positive_20d_rate),
            split.positive_60d_count,
            crate::format_pct(split.positive_60d_rate),
            split.prepare_primary_count,
            crate::format_pct(split.prepare_primary_rate),
            split.hedge_primary_count,
            crate::format_pct(split.hedge_primary_rate),
            split.defend_primary_count,
            crate::format_pct(split.defend_primary_rate),
            split.late_validation_row_count,
            crate::format_pct(split.late_validation_row_rate),
            split.protected_row_count,
            crate::format_pct(split.protected_row_rate),
            split.avg_coverage_score * 100.0,
            split.avg_core_feature_coverage * 100.0,
            split.avg_trigger_feature_coverage * 100.0,
            split.avg_external_feature_coverage * 100.0,
            split.scenario_count
        );
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Scenario Coverage");
    let _ = writeln!(markdown);
    let _ = writeln!(
        markdown,
        "| Scenario | Label | Family | Role | Protected | Horizons | Template | Rows | Splits | Range |"
    );
    let _ = writeln!(
        markdown,
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
    );
    for scenario in &summary.scenario_summaries {
        let default_horizon_roles = if scenario.default_horizon_roles.is_empty() {
            "-".to_string()
        } else {
            scenario
                .default_horizon_roles
                .iter()
                .map(|value| format!("{value}d"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let _ = writeln!(
            markdown,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} -> {} |",
            scenario.scenario_id,
            scenario.label.as_deref().unwrap_or("-"),
            scenario.family.as_deref().unwrap_or("-"),
            scenario.training_role.as_deref().unwrap_or("-"),
            scenario
                .protected_window
                .map(|value| if value { "yes" } else { "no" })
                .unwrap_or("-"),
            default_horizon_roles,
            scenario.episode_template_id.as_deref().unwrap_or("-"),
            scenario.row_count,
            scenario.split_count,
            scenario.first_as_of_date,
            scenario.last_as_of_date
        );
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Quality Mix");
    let _ = writeln!(markdown);
    for quality in &summary.quality_summaries {
        let _ = writeln!(
            markdown,
            "- grade {}: {} rows",
            quality.grade, quality.row_count
        );
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Regime Mix");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "| Split | Horizon | Regime | Rows | Share |");
    let _ = writeln!(markdown, "| --- | --- | --- | --- | --- |");
    for regime in &summary.regime_summaries {
        let _ = writeln!(
            markdown,
            "| {} | {}d | {} | {} | {} |",
            regime.split_name,
            regime.horizon_days,
            regime.regime,
            regime.row_count,
            crate::format_pct(regime.row_rate),
        );
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Recommendation");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "{}", summary.recommendation);
    markdown
}

pub(crate) fn print_formal_dataset_summary(summary: &FormalDatasetSummaryEnvelope) {
    println!(
        "Formal dataset {} rows={} pit={} feature_set={}",
        summary.dataset_key,
        summary.dataset.manifest.row_count,
        summary.dataset.manifest.point_in_time_mode,
        summary.dataset.manifest.feature_set_version
    );
    for split in &summary.split_summaries {
        println!(
            "  split={} rows={} forward[5d={}({}) 20d={}({}) 60d={}({})] action[prepare={}({}) hedge={}({}) defend={}({}) late_validation={}({}) protected={}({})] avg_coverage={:.1}%",
            split.split_name,
            split.row_count,
            split.positive_5d_count,
            crate::format_pct(split.positive_5d_rate),
            split.positive_20d_count,
            crate::format_pct(split.positive_20d_rate),
            split.positive_60d_count,
            crate::format_pct(split.positive_60d_rate),
            split.prepare_primary_count,
            crate::format_pct(split.prepare_primary_rate),
            split.hedge_primary_count,
            crate::format_pct(split.hedge_primary_rate),
            split.defend_primary_count,
            crate::format_pct(split.defend_primary_rate),
            split.late_validation_row_count,
            crate::format_pct(split.late_validation_row_rate),
            split.protected_row_count,
            crate::format_pct(split.protected_row_rate),
            split.avg_coverage_score * 100.0
        );
    }
    println!("  recommendation {}", summary.recommendation);
}

pub(super) fn print_formal_dataset_slice_summary(export: &FormalDatasetSliceExport) {
    let first_date = export
        .rows
        .first()
        .map(|row| row.as_of_date.to_string())
        .unwrap_or_else(|| "-".to_string());
    let last_date = export
        .rows
        .last()
        .map(|row| row.as_of_date.to_string())
        .unwrap_or_else(|| "-".to_string());
    println!(
        "Formal dataset slice dataset_key={} scenario_id={} rows={} range={} -> {} split={} features={}",
        export.dataset_key,
        export.scenario_id,
        export.row_count,
        first_date,
        last_date,
        export.split_name.as_deref().unwrap_or("all"),
        export.feature_names.len(),
    );
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
                    family: super::scenario_family_code(scenario.family).to_string(),
                    training_role: super::scenario_training_role_code(scenario.training_role)
                        .to_string(),
                    protected_window: scenario.protected_window,
                    episode_template_id: super::action_episode_template_code(
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

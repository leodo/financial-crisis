import {
  compactFileReference,
  formatDate,
  formatDateTime,
  formatPercent,
  releaseIdLabel,
  releaseReviewScenarioFamilyLabel,
  releaseReviewScenarioTrainingRoleLabel
} from "../../format";
import type { ResearchAuditResponse } from "../../types";
import type { DetailRowItem, MetricItem } from "../shared/panelHelpers";

function workstreamAuditLabel(label: string): string {
  return (
    {
      prewarning_signal_gap: "预警信号缺口",
      weak_signal_continuity: "弱连续性"
    }[label] ?? label
  );
}

function workstreamDatasetReasonLabel(label: string): string {
  return (
    {
      "main baseline coverage": "主数据集基线覆盖",
      "protected or extension coverage": "protected / 扩展数据集覆盖",
      "acute extension coverage": "急性冲击扩展数据集覆盖",
      "stress fallback coverage": "压力扩展回退覆盖",
      "acute fallback coverage": "急性扩展回退覆盖"
    }[label] ?? label
  );
}

function compactList(items: string[], fallback: string): string {
  return items.length > 0 ? items.join(" / ") : fallback;
}

export function buildWorkstreamAuditSection(audit: ResearchAuditResponse) {
  const latestWorkstreamAudit = audit.latest_workstream_audit;
  const latestWorkstreamAuditSource = latestWorkstreamAudit
    ? compactFileReference(latestWorkstreamAudit.source)
    : null;
  const latestWorkstreamAuditReport = latestWorkstreamAudit
    ? compactFileReference(latestWorkstreamAudit.review_report_path)
    : null;
  const matchesLatestReleaseReview =
    audit.latest_release_review !== null &&
    audit.latest_release_review.baseline_release_id === latestWorkstreamAudit?.baseline_release_id &&
    audit.latest_release_review.candidate_release_id === latestWorkstreamAudit?.candidate_release_id &&
    audit.latest_release_review.history_mode === latestWorkstreamAudit?.history_mode;

  if (!latestWorkstreamAudit) {
    return {
      latestWorkstreamAudit,
      latestWorkstreamAuditSource,
      latestWorkstreamAuditReport,
      latestWorkstreamAuditMetrics: [] as MetricItem[],
      latestWorkstreamAuditContextRows: [] as DetailRowItem[],
      latestWorkstreamSummaryRows: [] as DetailRowItem[],
      latestWorkstreamScenarioRows: [] as Array<{
        id: string;
        scenarioLabel: string;
        scenarioDetails: string[];
        datasetSummary: string;
        datasetDetails: string[];
        splitSummary: string;
        regimeSummary: string;
        labelSummary: string;
        actionSummary: string;
        coverageSummary: string;
        coverageDetails: string[];
        takeaway: string;
      }>
    };
  }

  const workstreamOrder = new Map(
    latestWorkstreamAudit.requested_workstreams.map((workstream, index) => [workstream, index])
  );
  const totalScenarioCount = latestWorkstreamAudit.workstream_summaries.reduce(
    (sum, row) => sum + row.scenario_count,
    0
  );
  const coveredScenarioCount = latestWorkstreamAudit.workstream_summaries.reduce(
    (sum, row) => sum + row.covered_scenario_count,
    0
  );
  const totalRows = latestWorkstreamAudit.workstream_summaries.reduce(
    (sum, row) => sum + row.total_rows,
    0
  );
  const totalPositive20d = latestWorkstreamAudit.workstream_summaries.reduce(
    (sum, row) => sum + row.total_positive_label_20d_count,
    0
  );
  const totalPositive60d = latestWorkstreamAudit.workstream_summaries.reduce(
    (sum, row) => sum + row.total_positive_label_60d_count,
    0
  );
  const totalProtectedRows = latestWorkstreamAudit.workstream_summaries.reduce(
    (sum, row) => sum + row.total_protected_row_count,
    0
  );

  const latestWorkstreamAuditMetrics: MetricItem[] = [
    {
      label: "审计时间",
      value: formatDateTime(latestWorkstreamAudit.generated_at)
    },
    {
      label: "工作流",
      value: `${latestWorkstreamAudit.workstream_summaries.length}`,
      hint: compactList(
        latestWorkstreamAudit.requested_workstreams.map(workstreamAuditLabel),
        "未登记请求工作流"
      )
    },
    {
      label: "覆盖场景",
      value: `${coveredScenarioCount}/${totalScenarioCount}`
    },
    {
      label: "总样本行",
      value: `${totalRows}`
    },
    {
      label: "20d 正标签",
      value: `${totalPositive20d}`
    },
    {
      label: "60d 正标签",
      value: `${totalPositive60d}`,
      hint: `受保护样本 ${totalProtectedRows}`
    }
  ];

  const latestWorkstreamAuditContextRows: DetailRowItem[] = [
    {
      id: "workstream-audit-releases",
      title: "基线 / 候选",
      detail: `${releaseIdLabel(latestWorkstreamAudit.baseline_release_id).value} vs ${releaseIdLabel(latestWorkstreamAudit.candidate_release_id).value}`,
      note: matchesLatestReleaseReview
        ? `history mode: ${latestWorkstreamAudit.history_mode}`
        : `当前优先展示 residual 覆盖更完整的专项工件；history mode: ${latestWorkstreamAudit.history_mode}`
    },
    {
      id: "workstream-audit-report",
      title: "关联 release review",
      detail: latestWorkstreamAuditReport?.value ?? "未登记",
      note: latestWorkstreamAuditReport?.hint
    },
    {
      id: "workstream-audit-dataset",
      title: "数据集选择",
      detail:
        latestWorkstreamAudit.dataset_key ||
        latestWorkstreamAudit.dataset_id ||
        "按场景自动选择 dataset",
      note:
        latestWorkstreamAudit.dataset_version || latestWorkstreamAudit.market_scope
          ? [latestWorkstreamAudit.dataset_version, latestWorkstreamAudit.market_scope]
              .filter((item) => item && item.length > 0)
              .join(" / ")
          : "当前以场景级 selector 自动匹配 dataset。"
    }
  ];

  const latestWorkstreamSummaryRows: DetailRowItem[] = latestWorkstreamAudit.workstream_summaries
    .slice()
    .sort((left, right) => {
      return (
        (workstreamOrder.get(left.workstream) ?? 99) - (workstreamOrder.get(right.workstream) ?? 99)
      );
    })
    .map((row) => ({
      id: `workstream-${row.workstream}`,
      title: workstreamAuditLabel(row.workstream),
      detail: `${row.covered_scenario_count}/${row.scenario_count} 个场景；${compactList(row.scenarios, "未登记场景")}`,
      note: `rows ${row.total_rows}；20d 正标签 ${row.total_positive_label_20d_count}；60d 正标签 ${row.total_positive_label_60d_count}；training role ${compactList(row.training_roles, "未登记")}；family ${compactList(row.scenario_families.map(releaseReviewScenarioFamilyLabel), "未登记")}`,
      meta: formatPercent(row.avg_coverage_score)
    }));

  const latestWorkstreamScenarioRows = latestWorkstreamAudit.scenario_summaries
    .slice()
    .sort((left, right) => {
      return (
        (workstreamOrder.get(left.workstream) ?? 99) - (workstreamOrder.get(right.workstream) ?? 99) ||
        left.scenario_name.localeCompare(right.scenario_name, "zh-CN")
      );
    })
    .map((row) => ({
      id: row.scenario_id,
      scenarioLabel: row.scenario_name,
      scenarioDetails: [
        row.scenario_id,
        workstreamAuditLabel(row.workstream),
        releaseReviewScenarioFamilyLabel(row.scenario_family),
        releaseReviewScenarioTrainingRoleLabel(row.training_role),
        row.protected_window ? "受保护窗口" : "非 protected"
      ],
      datasetSummary: row.dataset_key || "未命中 dataset",
      datasetDetails: [
        workstreamDatasetReasonLabel(row.slice_selector_reason),
        `${formatDate(row.window_start)} - ${formatDate(row.window_end)}`,
        row.attempted_datasets.length > 0 ? `Tried: ${row.attempted_datasets.join(" / ")}` : "未登记 dataset 尝试链路"
      ],
      splitSummary: compactList(row.split_counts, "未登记 split"),
      regimeSummary: `20d ${compactList(row.regime20_counts, "—")} / 60d ${compactList(row.regime60_counts, "—")}`,
      labelSummary: `5d ${row.positive_label_5d_count} / 20d ${row.positive_label_20d_count} / 60d ${row.positive_label_60d_count}`,
      actionSummary: `prepare ${row.prepare_primary_count} / hedge ${row.hedge_primary_count} / defend ${row.defend_primary_count}；phase ${compactList(row.action_phase_counts, "—")}；level ${compactList(row.primary_action_level_counts, "—")}`,
      coverageSummary: `${formatPercent(row.avg_coverage_score)} / 核心 ${formatPercent(row.avg_core_feature_coverage)} / 触发 ${formatPercent(row.avg_trigger_feature_coverage)}`,
      coverageDetails: [
        `行数 ${row.row_count} / protected ${row.protected_row_count}`,
        `特征 ${row.feature_name_count}：${compactList(row.feature_names.slice(0, 4), "未登记")}${row.feature_names.length > 4 ? " ..." : ""}`,
        latestWorkstreamAuditSource?.value
          ? `slice: ${compactFileReference(row.slice_json_path).value}`
          : compactFileReference(row.slice_json_path).value
      ],
      takeaway: row.suggested_review ?? row.slice_status
    }));

  return {
    latestWorkstreamAudit,
    latestWorkstreamAuditSource,
    latestWorkstreamAuditReport,
    latestWorkstreamAuditMetrics,
    latestWorkstreamAuditContextRows,
    latestWorkstreamSummaryRows,
    latestWorkstreamScenarioRows
  };
}

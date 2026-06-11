import {
  compactFileReference,
  formatDate,
  formatDateTime,
  pointInTimeModeLabel,
  releaseReviewScenarioCoveragePitLabel,
  releaseReviewScenarioFamilyLabel,
  releaseReviewScenarioRoleLabel,
  releaseReviewScenarioTrainingRoleLabel
} from "../../format";
import type { ResearchAuditResponse } from "../../types";
import type { MetricItem } from "../shared/panelHelpers";

function datasetLabel(datasetId: string): string {
  return (
    {
      formal_v1_main_1990_daily: "正式主数据集",
      formal_v1_ext_stress_1990_daily: "压力扩展数据集",
      formal_v1_ext_acute_pre1990: "急性扩展数据集"
    }[datasetId] ?? datasetId
  );
}

function datasetIntentLabel(intent: string): string {
  return (
    {
      "main_training + protected_context": "主训练 + protected context",
      extension_training: "扩展训练研究"
    }[intent] ?? intent
  );
}

function boolSummary(value: boolean | null | undefined, label: string): string | null {
  return value ? label : null;
}

export function buildDatasetSummarySection(audit: ResearchAuditResponse) {
  const datasetOrder = new Map([
    ["formal_v1_main_1990_daily", 0],
    ["formal_v1_ext_stress_1990_daily", 1],
    ["formal_v1_ext_acute_pre1990", 2]
  ]);
  const latestDatasetSummaries = audit.latest_dataset_summaries
    .slice()
    .sort((left, right) => {
      return (
        (datasetOrder.get(left.dataset.dataset_id) ?? 99) -
          (datasetOrder.get(right.dataset.dataset_id) ?? 99) ||
        right.dataset.row_count - left.dataset.row_count ||
        right.generated_at.localeCompare(left.generated_at)
      );
    });

  if (latestDatasetSummaries.length === 0) {
    return {
      latestDatasetSummaries,
      latestDatasetSummaryMetrics: [] as MetricItem[],
      latestDatasetSummaryRows: [] as Array<{
        id: string;
        datasetLabel: string;
        datasetDetails: string[];
        rangeSummary: string;
        rangeDetails: string[];
        splitSummary: string;
        labelSummary: string;
        coverageSummary: string;
        coverageDetails: string[];
        recommendation: string;
      }>,
      latestDatasetScenarioRows: [] as Array<{
        id: string;
        datasetLabel: string;
        datasetDetails: string[];
        scenarioLabel: string;
        scenarioDetails: string[];
        windowSummary: string;
        windowDetails: string[];
        labelSummary: string;
        coverageSummary: string;
        coverageDetails: string[];
        statusSummary: string;
      }>
    };
  }

  const uniqueScenarioIds = new Set<string>();
  const uniqueMainScenarioIds = new Set<string>();
  const uniqueExtensionScenarioIds = new Set<string>();
  const uniqueProtectedScenarioIds = new Set<string>();
  const uniqueAnalogScenarioIds = new Set<string>();
  for (const summary of latestDatasetSummaries) {
    for (const row of summary.scenario_summaries) {
      uniqueScenarioIds.add(row.scenario_id);
      if (row.usable_for_main_training) {
        uniqueMainScenarioIds.add(row.scenario_id);
      }
      if (row.usable_for_extension_training) {
        uniqueExtensionScenarioIds.add(row.scenario_id);
      }
      if (row.usable_for_protected_stress) {
        uniqueProtectedScenarioIds.add(row.scenario_id);
      }
      if (row.usable_for_historical_analog) {
        uniqueAnalogScenarioIds.add(row.scenario_id);
      }
    }
  }

  const latestDatasetSummaryMetrics: MetricItem[] = [
    {
      label: "已导出数据集（审计）",
      value: `${latestDatasetSummaries.length}/3`,
      hint: "已落库/导出的 formal dataset evidence，不代表模型已经完成训练或上线。"
    },
    {
      label: "总样本行（历史）",
      value: `${latestDatasetSummaries.reduce((sum, row) => sum + row.dataset.row_count, 0)}`,
      hint: "三套历史 dataset 的行数合计，不是当前线上模型训练样本承诺。"
    },
    {
      label: "覆盖场景（目录）",
      value: `${uniqueScenarioIds.size}`,
      hint: "历史场景目录覆盖数，不是当前风险事件数。"
    },
    {
      label: "主训练可用（目录）",
      value: `${uniqueMainScenarioIds.size}`,
      hint: "目录层判断的可用场景数，不等于 candidate 已通过 release review。"
    },
    {
      label: "扩展 / Protected（目录）",
      value: `${uniqueExtensionScenarioIds.size} / ${uniqueProtectedScenarioIds.size}`,
      hint: "扩展训练和 protected stress 场景目录数，不是自动放行条件。"
    },
    {
      label: "历史类比可用（目录）",
      value: `${uniqueAnalogScenarioIds.size}`,
      hint: "可用于类比的历史场景数，不是当前危机概率。"
    }
  ];

  const latestDatasetSummaryRows = latestDatasetSummaries.map((summary) => {
    const splitCounts = summary.split_summaries.map(
      (row) => `${row.split_name}=${row.row_count}`
    );
    const totalPositive5d = summary.split_summaries.reduce(
      (sum, row) => sum + row.positive_5d_count,
      0
    );
    const totalPositive20d = summary.split_summaries.reduce(
      (sum, row) => sum + row.positive_20d_count,
      0
    );
    const totalPositive60d = summary.split_summaries.reduce(
      (sum, row) => sum + row.positive_60d_count,
      0
    );
    const totalPrepare = summary.split_summaries.reduce(
      (sum, row) => sum + row.prepare_primary_count,
      0
    );
    const totalHedge = summary.split_summaries.reduce(
      (sum, row) => sum + row.hedge_primary_count,
      0
    );
    const totalProtected = summary.split_summaries.reduce(
      (sum, row) => sum + row.protected_row_count,
      0
    );
    const summarySource = compactFileReference(summary.source);

    return {
      id: summary.dataset_key,
      datasetLabel: datasetLabel(summary.dataset.dataset_id),
      datasetDetails: [
        summary.dataset_key,
        pointInTimeModeLabel(summary.dataset.point_in_time_mode),
        summarySource.value
      ],
      rangeSummary: `${formatDate(summary.dataset.from_date)} - ${formatDate(summary.dataset.to_date)}`,
      rangeDetails: [
        `feature: ${summary.dataset.feature_set_version}`,
        `label: ${summary.dataset.label_version}`,
        `生成: ${formatDateTime(summary.generated_at)}`
      ],
      splitSummary: splitCounts.join(" / "),
      labelSummary: `历史标签 5d ${totalPositive5d} / 20d ${totalPositive20d} / 60d ${totalPositive60d}；动作标签 prepare ${totalPrepare} / hedge ${totalHedge}；protected ${totalProtected}`,
      coverageSummary: `目录覆盖 ${summary.coverage_catalog.aligned_scenario_count}/${summary.coverage_catalog.total_scenario_count} 场景 / ${datasetIntentLabel(summary.coverage_catalog.dataset_intent)}`,
      coverageDetails: [
        `主训练 ${summary.coverage_catalog.main_training_eligible_count} / 扩展 ${summary.coverage_catalog.extension_training_eligible_count} / protected ${summary.coverage_catalog.protected_stress_eligible_count}`,
        `历史类比 ${summary.coverage_catalog.historical_analog_eligible_count}`,
        summary.coverage_catalog.warning ?? "当前没有额外 catalog 告警。"
      ],
      recommendation: summary.recommendation
    };
  });

  const latestDatasetScenarioRows = latestDatasetSummaries
    .flatMap((summary) =>
      summary.scenario_summaries.map((row) => ({
        id: `${summary.dataset.dataset_id}:${row.scenario_id}`,
        datasetOrder: datasetOrder.get(summary.dataset.dataset_id) ?? 99,
        datasetLabel: datasetLabel(summary.dataset.dataset_id),
        datasetDetails: [
          summary.dataset.dataset_version,
          `历史行数 ${summary.dataset.row_count}`,
          releaseReviewScenarioCoveragePitLabel(row.coverage_point_in_time_mode ?? "best_effort")
        ],
        scenarioLabel: row.label ?? row.scenario_id,
        scenarioDetails: [
          row.scenario_id,
          row.family ? releaseReviewScenarioFamilyLabel(row.family) : null,
          row.training_role
            ? releaseReviewScenarioTrainingRoleLabel(row.training_role)
            : null,
          row.protected_window ? "受保护窗口" : null
        ].filter((item): item is string => item !== null),
        windowSummary: `${formatDate(row.first_as_of_date)} - ${formatDate(row.last_as_of_date)}`,
        windowDetails: [
          `行数 ${row.row_count} / split ${row.split_count}`,
          row.coverage_recommended_role
            ? releaseReviewScenarioRoleLabel(row.coverage_recommended_role)
            : "未登记推荐角色",
          row.episode_template_id ? `episode: ${row.episode_template_id}` : null
        ].filter((item): item is string => item !== null),
        labelSummary: `训练用途 horizon ${row.default_horizon_roles.join("/") || "—"}；主训练 ${boolSummary(
          row.usable_for_main_training,
          "是"
        ) ?? "否"} / 扩展 ${boolSummary(row.usable_for_extension_training, "是") ?? "否"}`,
        coverageSummary: `覆盖等级 ${row.coverage_grade ?? "未评级"} / ${releaseReviewScenarioCoveragePitLabel(
          row.coverage_point_in_time_mode ?? "best_effort"
        )}`,
        coverageDetails: [
          row.coverage_current_status ?? "未登记当前状态",
          row.coverage_free_sources.length > 0
            ? `免费主源: ${row.coverage_free_sources.join("、")}`
            : "未登记免费主源",
          [
            boolSummary(row.usable_for_protected_stress, "protected"),
            boolSummary(row.usable_for_historical_analog, "analog")
          ]
            .filter((item): item is string => item !== null)
            .join(" / ") || "未登记附加用途"
        ],
        statusSummary:
          row.coverage_blocking_gaps.length > 0
            ? row.coverage_blocking_gaps.join("；")
            : "当前没有额外 blocking gap。"
      }))
    )
    .sort((left, right) => {
      return (
        left.scenarioLabel.localeCompare(right.scenarioLabel, "zh-CN") ||
        left.datasetOrder - right.datasetOrder
      );
    });

  return {
    latestDatasetSummaries,
    latestDatasetSummaryMetrics,
    latestDatasetSummaryRows,
    latestDatasetScenarioRows
  };
}

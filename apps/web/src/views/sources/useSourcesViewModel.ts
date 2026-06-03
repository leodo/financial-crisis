import {
  datasetLabel,
  formatDateTime,
  formatNumber,
  formatPercent,
  qualityDetailLabel,
  sourceAccessMethodLabel,
  sourceHealthStatusLabel,
  sourcePriorityLabel,
  sourceTypeLabel
} from "../../format";
import type { AssessmentSnapshot, DataSource } from "../../types";
import type { MetricItem } from "../shared/panelHelpers";

function qualityBandLabel(score: number) {
  if (score >= 90) {
    return "高";
  }
  if (score >= 80) {
    return "可用";
  }
  if (score >= 70) {
    return "一般";
  }
  return "偏弱";
}

function extractDatasetId(message: string) {
  const match = message.match(/dataset=([^)]+)/);
  return match?.[1] ?? null;
}

function formatLagText(seconds: number | null | undefined) {
  if (seconds === null || seconds === undefined) {
    return "滞后未知";
  }

  const days = Math.round(seconds / 86_400);
  if (days >= 1) {
    return `滞后 ${days} 天`;
  }

  const hours = Math.round(seconds / 3_600);
  if (hours >= 1) {
    return `滞后 ${hours} 小时`;
  }

  return "近实时";
}

function humanizeSourceCopy(text: string) {
  return text
    .replaceAll("Official no-key Treasury yield curve data.", "官方免 key 的美债收益率曲线数据。")
    .replaceAll("FRED graph CSV is the default no-key source; official API remains optional.", "FRED Graph CSV 是默认免 key 路径，官方 API 只是可选增强。")
    .replaceAll("Official SEC JSON filings metadata aggregated into daily event features. No paid key is required.", "SEC 官方 JSON 公告元数据已聚合成日频事件特征，无需付费 key。")
    .replaceAll("Official World Bank Indicators API.", "World Bank 官方指标 API。")
    .replaceAll("Official BOJ FX and money-market endpoints are used for the JPY carry monitor.", "BOJ 官方汇率和货币市场接口用于日元套息监控。")
    .replaceAll("Development-only market data prototype; not a production dependency.", "仅开发期市场数据原型，不属于正式依赖。")
    .replaceAll("prototype source, not for production", "原型源，不进入正式评估");
}

export function useSourcesViewModel({
  assessment,
  sources
}: {
  assessment: AssessmentSnapshot;
  sources: DataSource[];
}) {
  const coverageMetrics = [
    ["总覆盖", formatPercent(assessment.data_trust.coverage_score)],
    ["核心特征", formatPercent(assessment.data_trust.core_feature_coverage)],
    ["触发特征", formatPercent(assessment.data_trust.trigger_feature_coverage)],
    ["外部特征", formatPercent(assessment.data_trust.external_feature_coverage)]
  ] as Array<[string, string]>;

  const delayedOrMissingCount = sources.filter((source) =>
    ["delayed", "stale", "missing", "failing"].includes(source.health.status)
  ).length;
  const researchOnlyCount = sources.filter((source) => !source.production_allowed).length;

  const summaryMetrics: MetricItem[] = [
    {
      label: "总体等级",
      value: qualityDetailLabel(assessment.data_trust.quality_grade),
      hint: `总覆盖 ${formatPercent(assessment.data_trust.coverage_score)}`
    },
    {
      label: "受阻核心",
      value: `${assessment.data_trust.data_quality_summary.blocked_indicator_count}`,
      hint: "建议先补齐这些指标，再提高动作强度。"
    },
    {
      label: "延迟/缺失源",
      value: `${delayedOrMissingCount}`,
      hint: "这部分会拖慢部分维度的确认速度。"
    },
    {
      label: "仅辅助源",
      value: `${researchOnlyCount}`,
      hint: "这些源默认不直接进入正式评估。"
    }
  ];

  const sourceRows = sources.map((source) => {
    const dataset = extractDatasetId(source.health.message);
    const datasetName = datasetLabel(dataset);

    return {
      id: source.source_id,
      displayName: source.display_name,
      sourceMeta: `${sourceTypeLabel(source.source_type)} · ${sourcePriorityLabel(source.priority)}`,
      sourceMetaHint: dataset ? `数据集 ${datasetName}` : source.source_id,
      status: sourceHealthStatusLabel(source.health.status),
      statusDetail: [
        source.health.last_success_at
          ? `最近成功 ${formatDateTime(source.health.last_success_at)}`
          : "暂时没有成功抓取记录",
        formatLagText(source.health.lag_seconds)
      ],
      qualityScore: formatNumber(source.health.quality_score),
      qualityDetail: qualityBandLabel(source.health.quality_score),
      productionAllowed: source.production_allowed ? "可进入正式评估" : "仅研究参考",
      productionDetail: source.production_allowed
        ? sourceAccessMethodLabel(source.access_method)
        : "原型源或开发辅助源",
      healthMessage:
        source.health.status === "prototype"
          ? "当前仍按原型辅助信号处理，不直接进入正式评估。"
          : `当前使用 ${datasetName}`,
      licenseNote: humanizeSourceCopy(source.license_note)
    };
  });

  return {
    summaryMetrics,
    coverageMetrics,
    warnings: assessment.data_trust.warnings,
    sourceRows
  };
}

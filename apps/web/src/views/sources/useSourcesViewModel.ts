import {
  datasetLabel,
  formatDateTime,
  formatNumber,
  formatPercent,
  humanizeSourceLicenseNote,
  qualityDetailLabel,
  sourceLagLabel,
  sourceAccessMethodLabel,
  sourceHealthStatusLabel,
  sourcePriorityLabel,
  sourceQualityBandLabel,
  sourceTypeLabel
} from "../../format";
import type { AssessmentSnapshot, DataSource } from "../../types";
import type { MetricItem } from "../shared/panelHelpers";

function extractDatasetId(message: string) {
  const match = message.match(/dataset=([^)]+)/);
  return match?.[1] ?? null;
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
    ["delayed", "partial_failure", "failed", "stale", "missing", "failing"].includes(
      source.health.status
    )
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
        sourceLagLabel(source.health.lag_seconds)
      ],
      qualityScore: formatNumber(source.health.quality_score),
      qualityDetail: sourceQualityBandLabel(source.health.quality_score),
      productionAllowed: source.production_allowed ? "可进入正式评估" : "仅研究参考",
      productionDetail: source.production_allowed
        ? sourceAccessMethodLabel(source.access_method)
        : "原型源或开发辅助源",
      healthMessage:
        source.health.status === "prototype"
          ? "当前仍按原型辅助信号处理，不直接进入正式评估。"
          : `当前使用 ${datasetName}`,
      licenseNote: humanizeSourceLicenseNote(source.license_note)
    };
  });

  return {
    summaryMetrics,
    coverageMetrics,
    warnings: assessment.data_trust.warnings,
    sourceRows
  };
}

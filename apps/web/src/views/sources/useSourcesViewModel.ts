import {
  datasetLabel,
  formatDateTime,
  formatNumber,
  formatPercent,
  humanizeNarrativeCopy,
  humanizeSourceLicenseNote,
  qualityDetailLabel,
  sourceAccessMethodLabel,
  sourceHealthStatusLabel,
  sourcePriorityLabel,
  sourceQualityBandLabel,
  sourceTypeLabel
} from "../../format";
import type { AssessmentSnapshot, DataSource } from "../../types";
import type { MetricItem } from "../shared/panelHelpers";

function extractDatasetId(message: string) {
  const match = message.match(/dataset=([^;)]+)/);
  return match?.[1] ?? null;
}

function extractLatestObservationDate(message: string) {
  const match = message.match(/latest observation (\d{4}-\d{2}-\d{2})/);
  return match?.[1] ?? null;
}

function extractWatermarkPeriod(message: string) {
  const match =
    message.match(/抓取水位[= ](\d{4}-\d{2}-\d{2})/) ??
    message.match(/watermark_period=(\d{4}-\d{2}-\d{2})/) ??
    message.match(/data_period=(\d{4}-\d{2}-\d{2})/) ??
    message.match(/last successful data period (\d{4}-\d{2}-\d{2})/);
  return match?.[1] ?? null;
}

function sourceObservationLagLabel(seconds: number | null | undefined) {
  if (seconds === null || seconds === undefined) {
    return "观测滞后未知";
  }

  const days = Math.max(0, Math.round(seconds / 86_400));
  return `观测滞后 ${days} 天`;
}

function sourceLagDetail(source: DataSource, latestObservationDate: string | null) {
  if (latestObservationDate || source.health.last_success_at) {
    return sourceObservationLagLabel(source.health.lag_seconds);
  }

  if (!source.production_allowed) {
    return "未进入正式刷新监控";
  }

  return sourceObservationLagLabel(source.health.lag_seconds);
}

function sourceHealthMessage(
  source: DataSource,
  datasetName: string,
  latestObservationDate: string | null,
  watermarkPeriod: string | null
): string {
  if (source.health.status === "prototype") {
    return "当前仍按原型辅助信号处理，不直接进入正式评估。";
  }

  if (["delayed", "partial_failure", "failed"].includes(source.health.status)) {
    return `源状态${sourceHealthStatusLabel(source.health.status)}：${humanizeNarrativeCopy(
      source.health.message
    )}`;
  }

  if (latestObservationDate && watermarkPeriod && latestObservationDate !== watermarkPeriod) {
    return `当前使用 ${datasetName}；最新观测 ${latestObservationDate}，抓取水位 ${watermarkPeriod}。`;
  }

  if (latestObservationDate) {
    return `当前使用 ${datasetName}；最新观测 ${latestObservationDate}。`;
  }

  if (watermarkPeriod) {
    return `当前使用 ${datasetName}；抓取水位 ${watermarkPeriod}。`;
  }

  return `当前使用 ${datasetName}。`;
}

function sourceHealthWarning(source: DataSource): string | null {
  if (
    !source.production_allowed ||
    !["delayed", "partial_failure", "failed"].includes(source.health.status)
  ) {
    return null;
  }

  return `${source.display_name} 当前${sourceHealthStatusLabel(
    source.health.status
  )}：${humanizeNarrativeCopy(source.health.message)}`;
}

function sourceUsageRecommendation(source: DataSource, latestObservationDate: string | null) {
  if (["failed", "failing", "missing"].includes(source.health.status)) {
    return {
      label: "先不要依赖",
      detail: "当前源不可用或缺失；相关维度只能等待兜底源或补抓后再解释。"
    };
  }

  if (!source.production_allowed || source.health.status === "prototype") {
    return {
      label: "仅作辅助背景",
      detail: "不参与当前主结论，也不能单独触发动作升级。"
    };
  }

  if (["delayed", "partial_failure", "stale"].includes(source.health.status)) {
    return {
      label: "降级使用",
      detail: "只能作为背景或复核输入，短期动作升级要等源恢复或其他近端信号确认。"
    };
  }

  if (source.source_type === "global_macro") {
    return {
      label: "慢变量背景",
      detail: "可解释结构脆弱性，不代表今天刚发生触发。"
    };
  }

  if (source.source_type === "filings_events") {
    return {
      label: "事件确认输入",
      detail: latestObservationDate
        ? "可参与事件层确认，但仍需与市场压力共振后才支持动作升级。"
        : "可参与事件层确认；当前没有解析到最新观测日期，先按保守口径解释。"
    };
  }

  return {
    label: "可参与当前评估",
    detail: "可用于当前日频评估；仍不是盘中实时行情，也不是结论可信度本身。"
  };
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
  const sourceWarnings = sources
    .map(sourceHealthWarning)
    .filter((warning): warning is string => warning !== null);

  const summaryMetrics: MetricItem[] = [
    {
      label: "关键覆盖等级",
      value: qualityDetailLabel(assessment.data_trust.quality_grade),
      hint: `关键指标覆盖 ${formatPercent(assessment.data_trust.coverage_score)}，不等同于全部源健康。`
    },
    {
      label: "受阻核心",
      value: `${assessment.data_trust.data_quality_summary.blocked_indicator_count}`,
      hint: "建议先补齐这些指标，再提高动作强度。"
    },
    {
      label: "源健康降级",
      value: `${delayedOrMissingCount}`,
      hint: "这部分单独反映源状态，会拖慢部分维度的确认速度。"
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
    const latestObservationDate = extractLatestObservationDate(source.health.message);
    const watermarkPeriod = extractWatermarkPeriod(source.health.message);
    const usageRecommendation = sourceUsageRecommendation(source, latestObservationDate);
    const statusDetail = [
      source.health.last_success_at
        ? `最近成功刷新 ${formatDateTime(source.health.last_success_at)}`
        : "暂时没有成功抓取记录",
      latestObservationDate ? `最新观测 ${latestObservationDate}` : null,
      watermarkPeriod ? `抓取水位 ${watermarkPeriod}` : null,
      sourceLagDetail(source, latestObservationDate)
    ].filter((detail): detail is string => detail !== null);

    return {
      id: source.source_id,
      displayName: source.display_name,
      sourceMeta: `${sourceTypeLabel(source.source_type)} · ${sourcePriorityLabel(source.priority)}`,
      sourceMetaHint: dataset ? `数据集 ${datasetName}` : source.source_id,
      status: sourceHealthStatusLabel(source.health.status),
      statusDetail,
      qualityScore: `源健康分 ${formatNumber(source.health.quality_score)}`,
      qualityDetail: [
        sourceQualityBandLabel(source.health.quality_score),
        "抓取/源状态分，不是当前结论可信度"
      ],
      usageLabel: usageRecommendation.label,
      usageDetail: usageRecommendation.detail,
      productionAllowed: source.production_allowed ? "可进入正式评估" : "仅研究参考",
      productionDetail: source.production_allowed
        ? sourceAccessMethodLabel(source.access_method)
        : "原型源或开发辅助源",
      healthMessage: sourceHealthMessage(
        source,
        datasetName,
        latestObservationDate,
        watermarkPeriod
      ),
      licenseNote: humanizeSourceLicenseNote(source.license_note)
    };
  });

  return {
    summaryMetrics,
    coverageMetrics,
    warnings: [...assessment.data_trust.warnings, ...sourceWarnings],
    sourceRows
  };
}

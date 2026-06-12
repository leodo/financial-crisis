import {
  formatDateTime,
  humanizeNarrativeCopy,
  sourceLabel
} from "../../format";
import type { AssessmentSnapshot, FreeDataSourceCatalog } from "../../types";

type KeyIndicator = AssessmentSnapshot["key_indicators"][number];

export function sourceAccessTag(sourceId: string | null): string {
  const officialFreeSources = new Set(["fred", "treasury", "world_bank", "boj", "sec_edgar"]);
  if (!sourceId) {
    return "来源缺失";
  }
  if (officialFreeSources.has(sourceId)) {
    return "免费官方";
  }
  if (sourceId === "gdelt") {
    return "免费公开";
  }
  return "需复核授权";
}

export function keyIndicatorSourceTimingCopy(indicator: KeyIndicator): string {
  if (indicator.indicator_id === "us_external_usdjpy_level" && indicator.source_id === "boj") {
    return "BOJ 免费官方日频点位，适合日频风险评估，不等同盘中实时价。";
  }
  if (indicator.source_id === "fred") {
    return "FRED 免费公开图表/序列数据，适合日频或低频风险评估。";
  }
  return `${sourceLabel(indicator.source_id)} 来源用于当前关键指标。`;
}

export function keyIndicatorFallbackCopy(
  indicator: KeyIndicator,
  catalog?: FreeDataSourceCatalog
): string {
  const record = catalog?.records.find(
    (item) => item.indicator_id === indicator.indicator_id
  );
  if (record) {
    if (record.alternatives.length > 0) {
      const paths = record.alternatives
        .map((alt) => `${sourceLabel(alt.source_id)} ${alt.dataset}（${alt.note}）`)
        .join("；");
      return `替代路径：${paths}`;
    }
    return `替代路径：当前未配置专门兜底源。${record.missing_impact}`;
  }

  // 目录缺失时的静态兜底，保持与历史口径一致
  switch (indicator.indicator_id) {
    case "us_external_usdjpy_level":
      return "替代路径：FRED DEXJPUS 可做免费日频兜底，但同样不是盘中价。";
    case "jp_rates_call_rate":
      return "替代路径：当前以 BOJ 为主，若缺失只能降低日元套息风险可信度。";
    case "us_liquidity_effr":
      return "替代路径：FRED DFF 可作为长历史联邦基金利率补丁。";
    case "us_market_vix_close":
      return "替代路径：FRED VIXCLS 是当前免费主路径；商业行情只作为未来增强。";
    default:
      return "替代路径：当前未配置专门兜底源，缺失时应降低结论可信度。";
  }
}

export function keyIndicatorLineageCopy(
  lineage: KeyIndicator["lineage"],
  options: { includeEvidenceLevel?: boolean } = {}
): string {
  if (!lineage) {
    return "当前缺少 lineage 追溯信息。";
  }

  const evidence = options.includeEvidenceLevel
    ? `追溯级别 ${lineage.evidence_level}；`
    : "";
  const fetchedAt = lineage.fetched_at ? ` 抓取 ${formatDateTime(lineage.fetched_at)}。` : "";
  const recordsWritten =
    lineage.records_written !== null ? ` 写入 ${lineage.records_written} 条。` : "";
  const raw = lineage.raw_payload_id ? ` raw=${lineage.raw_payload_id.slice(0, 8)}。` : "";
  return `${evidence}${humanizeNarrativeCopy(lineage.note)}${fetchedAt}${recordsWritten}${raw}`;
}

export function keyIndicatorDecisionImpact(indicator: KeyIndicator): string {
  if (indicator.latest_value === null || indicator.latest_as_of_date === null) {
    return "缺失时降级结论";
  }
  if (indicator.status === "stale") {
    return "陈旧时降级结论";
  }
  if (indicator.lineage?.evidence_level === "missing") {
    return "追溯缺失需复核";
  }
  return "可参与当前评估";
}

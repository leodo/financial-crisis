import {
  formatDate,
  formatNumber,
  freshnessLabel,
  sourceLabel,
  unitLabel
} from "../../format";
import type { AssessmentSnapshot, FreeDataSourceCatalog } from "../../types";
import type { DecisionNumberAuditRow } from "./numberAudit";
import {
  keyIndicatorDecisionImpact,
  keyIndicatorFallbackCopy,
  keyIndicatorLineageCopy,
  keyIndicatorSourceTimingCopy,
  sourceAccessTag
} from "./dataSourceReliability";

export function buildFreeDataReliabilityRows(
  keyIndicators: AssessmentSnapshot["key_indicators"],
  freeDataSourceCatalog?: FreeDataSourceCatalog
): DecisionNumberAuditRow[] {
  return keyIndicators.map((indicator) => ({
    id: `free-data-${indicator.entity_id}-${indicator.indicator_id}`,
    title: `${indicator.display_name} · ${sourceAccessTag(indicator.source_id)}`,
    detail: `${formatNumber(indicator.latest_value)} ${unitLabel(indicator.unit)} · ${
      indicator.latest_as_of_date ? formatDate(indicator.latest_as_of_date) : "—"
    } · ${sourceLabel(indicator.source_id)} / ${indicator.dataset_id ?? "dataset 缺失"} · ${freshnessLabel(
      indicator.status
    )}`,
    meta: keyIndicatorDecisionImpact(indicator),
    note: [
      keyIndicatorSourceTimingCopy(indicator),
      keyIndicatorFallbackCopy(indicator, freeDataSourceCatalog),
      keyIndicatorLineageCopy(indicator.lineage, { includeEvidenceLevel: true })
    ].join(" ")
  }));
}

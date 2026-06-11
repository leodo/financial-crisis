import {
  formatNumber,
  humanizeNarrativeCopy,
  levelLabel,
  levelPlainText,
  postureLabel,
  timeBucketLabel
} from "../../format";
import type { AssessmentSnapshot, PostureGuidance, RiskSnapshot } from "../../types";
import type { MetricItem } from "../shared/panelHelpers";
import {
  currentMvpRiskState,
  mvpProbabilityInputIsAuditOnly,
  mvpRiskStateDetail,
  mvpRiskStateDisplayLabel
} from "../decision/mvpRiskState";
import { driversContent } from "./content";

export function useDriversViewModel({
  assessment,
  overview,
  posture
}: {
  assessment: AssessmentSnapshot;
  overview: RiskSnapshot;
  posture: PostureGuidance;
}) {
  const topDimension = [...overview.dimensions].sort((left, right) => right.score - left.score)[0];
  const elevatedDimensions = overview.dimensions.filter((dimension) => dimension.score >= 50).length;
  const auditOnly = mvpProbabilityInputIsAuditOnly(assessment);
  const mvpState = currentMvpRiskState(assessment);

  const summaryMetrics: MetricItem[] = [
    {
      label: "高压驱动",
      value: `${assessment.top_risk_drivers.filter((item) => item.score >= 60).length}`,
      hint: assessment.top_risk_drivers[0]
        ? `最强的是 ${humanizeNarrativeCopy(assessment.top_risk_drivers[0].display_name)}`
        : "当前没有进入高压区的驱动。"
    },
    {
      label: "缓冲因素",
      value: `${assessment.top_relief_drivers.length}`,
      hint: assessment.top_relief_drivers[0]
        ? `最强缓冲是 ${humanizeNarrativeCopy(assessment.top_relief_drivers[0].display_name)}`
        : "当前没有明显缓冲项。"
    },
    {
      label: "敏感维度",
      value: topDimension?.label ?? "—",
      hint: topDimension
        ? `${formatNumber(topDimension.score)} / ${levelPlainText(topDimension.level)}区`
        : "—"
    },
    {
      label: "执行结论",
      value: auditOnly ? mvpRiskStateDisplayLabel(mvpState.label) : postureLabel(assessment.posture),
      hint: auditOnly
        ? "当前主结论先按 MVP 规则层解释；正式概率和 posture 只作背景参考。"
        : timeBucketLabel(assessment.time_to_risk_bucket)
    }
  ];

  const dimensionRows = overview.dimensions.map((dimension) => ({
    id: dimension.dimension,
    label: dimension.label,
    score: formatNumber(dimension.score),
    caption: `${levelLabel(dimension.level)} · 30d ${formatNumber(dimension.change_30d)} · 已抬升维度 ${elevatedDimensions} 个`,
    focus: `${driversContent.dimensionCaptionPrefix} ${humanizeNarrativeCopy(dimension.top_contributors[0]?.display_name ?? "—")}${driversContent.dimensionCaptionSuffix}`,
    contributors: dimension.top_contributors.map((item) => ({
      id: `${dimension.dimension}-${item.indicator_id}`,
      displayName: humanizeNarrativeCopy(item.display_name),
      explanation: humanizeNarrativeCopy(item.explanation),
      score: formatNumber(item.score)
    }))
  }));

  const summaryRows = [
    [driversContent.summaryTitles.system, humanizeNarrativeCopy(assessment.summary)],
    [driversContent.summaryTitles.legacy, humanizeNarrativeCopy(overview.level_reason)],
    [
      driversContent.summaryTitles.posture,
      auditOnly ? humanizeNarrativeCopy(mvpRiskStateDetail(assessment)) : humanizeNarrativeCopy(posture.summary)
    ]
  ] as Array<[string, string]>;

  return {
    summaryMetrics,
    dimensionRows,
    summaryRows
  };
}

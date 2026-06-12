import {
  formatNumber,
  humanizeNarrativeCopy,
  levelLabel,
  levelPlainText,
  postureLabel,
  timeBucketLabel
} from "../../format";
import type { AssessmentSnapshot, IndicatorRisk, PostureGuidance, RiskSnapshot } from "../../types";
import type { MetricItem } from "../shared/panelHelpers";
import {
  currentMvpRiskState,
  mvpProbabilityInputIsAuditOnly,
  mvpRiskStateDetail,
  mvpRiskStateDisplayLabel
} from "../decision/mvpRiskState";
import {
  buildNearTermRiskDrivers,
  buildTimedRiskDrivers,
  driverTimingLabel,
  driverTimingPriority,
  stripDriverTiming
} from "../shared/driverTiming";
import { driversContent } from "./content";

export function useDriversViewModel({
  assessment,
  indicators,
  overview,
  posture
}: {
  assessment: AssessmentSnapshot;
  indicators: IndicatorRisk[];
  overview: RiskSnapshot;
  posture: PostureGuidance;
}) {
  const topDimension = [...overview.dimensions].sort((left, right) => right.score - left.score)[0];
  const elevatedDimensions = overview.dimensions.filter((dimension) => dimension.score >= 50).length;
  const auditOnly = mvpProbabilityInputIsAuditOnly(assessment);
  const mvpState = currentMvpRiskState(assessment);
  const timedRiskDrivers = buildTimedRiskDrivers(assessment, indicators);
  const nearTermTimedDrivers = buildNearTermRiskDrivers(assessment, indicators);
  const nearTermDrivers = nearTermTimedDrivers.slice(0, 5).map(stripDriverTiming);
  const backgroundDrivers = timedRiskDrivers
    .filter((driver) => driverTimingPriority(driver.timingBucket) > 1)
    .map(stripDriverTiming);
  const strongestBackgroundDriver = timedRiskDrivers.find(
    (driver) => driverTimingPriority(driver.timingBucket) > 1
  );

  const summaryMetrics: MetricItem[] = [
    {
      label: "近端高压驱动",
      value: `${nearTermTimedDrivers.filter((item) => item.score >= 60).length}`,
      hint: nearTermTimedDrivers[0]
        ? `最强近端项是 ${humanizeNarrativeCopy(nearTermTimedDrivers[0].display_name)}`
        : "当前没有日频/周频近端驱动进入高压区。"
    },
    {
      label: "背景高分驱动",
      value: `${backgroundDrivers.filter((item) => item.score >= 60).length}`,
      hint: strongestBackgroundDriver
        ? `${humanizeNarrativeCopy(strongestBackgroundDriver.display_name)} 属于 ${driverTimingLabel(
            strongestBackgroundDriver.timingBucket
          )}，不能当成今天刚发生的触发。`
        : "当前没有需要单独标注的慢变量背景高分项。"
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
    nearTermDrivers,
    backgroundDrivers,
    summaryMetrics,
    dimensionRows,
    summaryRows
  };
}

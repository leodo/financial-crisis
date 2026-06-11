import {
  formatDate,
  formatNumber,
  formatSignedNumber,
  scoreBasisLabel,
  unitLabel
} from "../../format";
import type { AssessmentSnapshot, IndicatorRisk } from "../../types";

export type DriverTimingBucket = "near_term" | "recent" | "structural" | "stale" | "missing";

export type TimedRiskDriver = AssessmentSnapshot["top_risk_drivers"][number] & {
  timingBucket: DriverTimingBucket;
};

export function buildTimedRiskDrivers(
  assessment: AssessmentSnapshot,
  indicators: IndicatorRisk[]
): TimedRiskDriver[] {
  const indicatorById = new Map(
    indicators.map((risk) => [risk.indicator.indicator_id, risk] as const)
  );

  return assessment.top_risk_drivers
    .map((driver) => enrichDriver(driver, indicatorById.get(driver.indicator_id), assessment.as_of_date))
    .filter((driver): driver is TimedRiskDriver => driver !== null)
    .sort((left, right) => {
      const priorityDelta = driverTimingPriority(left.timingBucket) - driverTimingPriority(right.timingBucket);
      if (priorityDelta !== 0) {
        return priorityDelta;
      }
      return right.contribution - left.contribution;
    });
}

export function buildNearTermRiskDrivers(
  assessment: AssessmentSnapshot,
  indicators: IndicatorRisk[]
): TimedRiskDriver[] {
  return buildIndicatorRiskDrivers(assessment, indicators)
    .filter((driver) => driverTimingPriority(driver.timingBucket) <= 1)
    .sort(compareTimedDrivers);
}

export function buildWhyNowRiskDrivers(
  assessment: AssessmentSnapshot,
  indicators: IndicatorRisk[],
  limit = 3
): AssessmentSnapshot["top_risk_drivers"] {
  const indicatorById = new Map(
    indicators.map((risk) => [risk.indicator.indicator_id, risk] as const)
  );
  const baseDrivers = assessment.top_risk_drivers
    .map((driver) => enrichDriver(driver, indicatorById.get(driver.indicator_id), assessment.as_of_date))
    .filter((driver): driver is TimedRiskDriver => driver !== null);
  const nearTermCandidates = buildNearTermRiskDrivers(assessment, indicators);

  const selected = new Map<string, TimedRiskDriver>();
  for (const driver of nearTermCandidates) {
    selected.set(driver.indicator_id, driver);
    if (selected.size >= limit) {
      break;
    }
  }
  for (const driver of baseDrivers) {
    if (selected.size >= limit) {
      break;
    }
    if (!selected.has(driver.indicator_id)) {
      selected.set(driver.indicator_id, driver);
    }
  }

  return Array.from(selected.values()).sort(compareTimedDrivers).map(stripDriverTiming);
}

export function driverTimingPriority(timingBucket: DriverTimingBucket) {
  switch (timingBucket) {
    case "near_term":
      return 0;
    case "recent":
      return 1;
    case "structural":
      return 2;
    case "stale":
      return 3;
    case "missing":
      return 4;
  }
}

export function driverTimingLabel(timingBucket: DriverTimingBucket) {
  switch (timingBucket) {
    case "near_term":
      return "近端触发";
    case "recent":
      return "近期背景";
    case "structural":
      return "结构背景";
    case "stale":
      return "偏旧背景";
    case "missing":
      return "待补日期";
  }
}

export function stripDriverTiming(driver: TimedRiskDriver): AssessmentSnapshot["top_risk_drivers"][number] {
  const { timingBucket: _timingBucket, ...rest } = driver;
  return rest;
}

function compareTimedDrivers(left: TimedRiskDriver, right: TimedRiskDriver) {
  const priorityDelta = driverTimingPriority(left.timingBucket) - driverTimingPriority(right.timingBucket);
  if (priorityDelta !== 0) {
    return priorityDelta;
  }
  return right.contribution - left.contribution;
}

function buildIndicatorRiskDrivers(
  assessment: AssessmentSnapshot,
  indicators: IndicatorRisk[]
): TimedRiskDriver[] {
  const baseExplanationById = new Map(
    assessment.top_risk_drivers.map((driver) => [driver.indicator_id, driver.explanation] as const)
  );

  return indicators
    .filter((risk) => risk.latest_observation !== null && risk.score > 0)
    .map((risk) =>
      enrichDriver(
        {
          indicator_id: risk.indicator.indicator_id,
          display_name: risk.indicator.display_name,
          dimension: risk.indicator.dimension,
          score: risk.score,
          contribution: risk.contribution,
          explanation:
            baseExplanationById.get(risk.indicator.indicator_id) ??
            buildSyntheticDriverExplanation(risk)
        },
        risk,
        assessment.as_of_date
      )
    )
    .filter((driver): driver is TimedRiskDriver => driver !== null);
}

function enrichDriver(
  driver: AssessmentSnapshot["top_risk_drivers"][number],
  risk: IndicatorRisk | undefined,
  asOfDate: string
): TimedRiskDriver | null {
  const baseExplanation = normalizeDriverExplanation(driver.explanation.trim(), risk);
  if (!risk?.latest_observation) {
    return {
      ...driver,
      timingBucket: "missing",
      explanation: baseExplanation.includes("当前缺少可核对的最新观测日期")
        ? baseExplanation
        : `${baseExplanation} 当前缺少可核对的最新观测日期。`.trim()
    };
  }

  const latestDate = risk.latest_observation.as_of_date;
  const frequency = risk.indicator.frequency;
  const lagDays = daysBetween(latestDate, asOfDate);
  const timingBucket = classifyDriverTiming(frequency, lagDays);
  const timingNote = buildTimingNote(latestDate, frequency, timingBucket);
  const explanation = baseExplanation.includes("最近观测 ")
    ? baseExplanation
    : `${baseExplanation} ${timingNote}`.trim();

  return {
    ...driver,
    timingBucket,
    explanation
  };
}

function buildSyntheticDriverExplanation(risk: IndicatorRisk): string {
  const basis = scoreBasisLabel(risk.score_basis);
  const value = buildDriverScoreInputCopy(risk, basis);
  const percentile =
    risk.percentile === null
      ? ""
      : `，历史分位 ${formatNumber(
          risk.percentile > 1 ? risk.percentile : risk.percentile * 100,
          "%"
        )}`;
  return `${risk.indicator.display_name} 按${basis}评分，${value}${percentile}，风险分 ${formatNumber(
    risk.score
  )}。`;
}

function normalizeDriverExplanation(baseExplanation: string, risk: IndicatorRisk | undefined): string {
  if (!risk || baseExplanation.length === 0) {
    return baseExplanation;
  }
  const basis = scoreBasisLabel(risk.score_basis);
  if (
    !isDerivedScoreBasis(basis) ||
    baseExplanation.includes("评分输入") ||
    (!baseExplanation.includes("当前信号") && !baseExplanation.includes("当前读数"))
  ) {
    return baseExplanation;
  }

  return (
    replaceCurrentSignalClause(baseExplanation, buildDriverScoreInputCopy(risk, basis)) ??
    baseExplanation
  );
}

function replaceCurrentSignalClause(explanation: string, replacement: string): string | null {
  for (const marker of ["当前信号", "当前读数"]) {
    const start = explanation.indexOf(marker);
    if (start < 0) {
      continue;
    }

    const tail = explanation.slice(start);
    const end = ["，历史分位", "，风险分"]
      .map((delimiter) => tail.indexOf(delimiter))
      .filter((index) => index >= 0)
      .sort((left, right) => left - right)[0];
    if (end === undefined) {
      continue;
    }

    return `${explanation.slice(0, start)}${replacement}${tail.slice(end)}`;
  }

  return null;
}

function buildDriverScoreInputCopy(risk: IndicatorRisk, basis: string): string {
  if (risk.score_input_value === null) {
    return isDerivedScoreBasis(basis) ? "评分输入缺失" : "当前读数缺失";
  }

  if (isDerivedScoreBasis(basis)) {
    const input = formatDriverValue(risk.score_input_value, risk.score_input_unit, true);
    if (risk.latest_observation) {
      const latestValue = formatDriverValue(
        risk.latest_observation.value,
        risk.indicator.unit,
        false
      );
      return `评分输入 ${input}（${basis}，不是 ${risk.indicator.display_name} 当前水平；最新水平 ${latestValue}）`;
    }
    return `评分输入 ${input}（${basis}，不是当前水平）`;
  }

  return `当前读数 ${formatDriverValue(risk.score_input_value, risk.score_input_unit, false)}`;
}

function formatDriverValue(
  value: number | null | undefined,
  unit: string | null | undefined,
  signed: boolean
) {
  const suffix = unitLabel(unit);
  const numeric = signed ? formatSignedNumber(value, 2) : formatNumber(value);
  return suffix ? `${numeric} ${suffix}` : numeric;
}

function isDerivedScoreBasis(basis: string): boolean {
  const lower = basis.toLowerCase();
  return (
    basis.includes("变化") ||
    basis.includes("同比") ||
    basis.includes("振幅") ||
    lower.includes("change") ||
    lower.includes("delta") ||
    lower.includes("yoy")
  );
}

function daysBetween(from: string, to: string) {
  const fromDate = Date.parse(`${from}T00:00:00Z`);
  const toDate = Date.parse(`${to}T00:00:00Z`);
  if (!Number.isFinite(fromDate) || !Number.isFinite(toDate)) {
    return Number.MAX_SAFE_INTEGER;
  }
  return Math.max(0, Math.round((toDate - fromDate) / 86_400_000));
}

function classifyDriverTiming(frequency: string, lagDays: number): DriverTimingBucket {
  switch (frequency) {
    case "daily":
      return lagDays <= 7 ? "near_term" : lagDays <= 14 ? "recent" : "stale";
    case "weekly":
      return lagDays <= 14 ? "near_term" : lagDays <= 28 ? "recent" : "stale";
    case "monthly":
      return lagDays <= 45 ? "recent" : lagDays <= 120 ? "structural" : "stale";
    case "quarterly":
      return lagDays <= 180 ? "structural" : "stale";
    case "annual":
      return lagDays <= 420 ? "structural" : "stale";
    default:
      return lagDays <= 30 ? "recent" : "stale";
  }
}

function buildTimingNote(
  latestDate: string,
  frequency: string,
  timingBucket: DriverTimingBucket
) {
  const frequencyLabel = frequencyToLabel(frequency);
  switch (timingBucket) {
    case "near_term":
      return `最近观测 ${formatDate(latestDate)}（${frequencyLabel}，属于近端驱动）。`;
    case "recent":
      return `最近观测 ${formatDate(latestDate)}（${frequencyLabel}，属于近月背景，解读要结合近端市场信号）。`;
    case "structural":
      return `最近观测 ${formatDate(latestDate)}（${frequencyLabel}慢变量，更偏结构背景，不表示今天市场刚出现这个变化）。`;
    case "stale":
      return `最近观测 ${formatDate(latestDate)}（${frequencyLabel}，相对当前请求日已偏旧，只能作为背景参照）。`;
    case "missing":
      return "当前缺少可核对的最新观测日期。";
  }
}

function frequencyToLabel(frequency: string) {
  switch (frequency) {
    case "daily":
      return "日频";
    case "weekly":
      return "周频";
    case "monthly":
      return "月频";
    case "quarterly":
      return "季频";
    case "annual":
      return "年频";
    default:
      return frequency;
  }
}

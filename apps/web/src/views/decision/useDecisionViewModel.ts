import { useMemo } from "react";
import {
  formatDate,
  formatNumber,
  formatProbabilityPercentExact,
  formatSignedNumber,
  historyEvidenceTierLabel
} from "../../format";
import type {
  AssessmentMethodResponse,
  AssessmentHistoryPoint,
  AssessmentSnapshot,
  BacktestScenarioSummary,
  IndicatorRisk,
  PostureGuidance
} from "../../types";
import {
  buildAnalogChart,
  buildLayerScoreChart,
  buildProbabilityTrendModel
} from "./charts";
import { buildHeroMetrics } from "./heroMetrics";
import { buildFreeDataReliabilityRows } from "./freeDataReliability";
import { buildNumberAuditRows } from "./numberAudit";
import {
  buildActionPlanMetrics,
  buildBacktestCoverageScopeText,
  buildAnalogRows,
  buildBacktestHistoryCoverageText,
  buildBacktestSummaryMetrics,
  buildBlockerClauses,
  buildDataTrustMetrics,
  buildJpyCarryMetrics,
  buildPostureThresholdMetrics,
  buildRiskHorizonActionMetrics,
  buildRollingAuditBoundaryText,
  buildRollingAuditEpisodes,
  buildRollingAuditHistoryText,
  buildRollingAuditMetrics,
  buildRollingAuditScopeText,
  buildRuntimeCards,
  buildRuntimeChipLabel,
  buildRuntimeNotice,
  buildScoreBandRows,
  buildSignalLayerRows,
  buildTriggerClauses
} from "./builders";
import {
  describeAnalogWindow,
  describeRiskScoreBand,
  describeTimeBucket
} from "./logic";
import { buildWhyNowRiskDrivers } from "../shared/driverTiming";

function decisionHistoryEvidenceCopy(note: string) {
  return note
    .replaceAll("formal history 审计的正式证据层", "正式历史证据层")
    .replaceAll("formal history 审计证据", "正式历史证据")
    .replaceAll("formal history 审计", "正式历史证据复核");
}

const DAY_MS = 86_400_000;

function parseHistoryDate(value: string) {
  const timestamp = Date.parse(`${value}T00:00:00Z`);
  return Number.isFinite(timestamp) ? timestamp : null;
}

function formatProbabilityDelta(value: number | null) {
  if (value === null || Number.isNaN(value)) {
    return "—";
  }
  const sign = value > 0 ? "+" : value < 0 ? "-" : "";
  return `${sign}${formatProbabilityPercentExact(Math.abs(value))}`;
}

function buildCurrentHistoryPoint(assessment: AssessmentSnapshot): AssessmentHistoryPoint {
  return {
    as_of_date: assessment.as_of_date,
    overall_score: assessment.scores.overall_score,
    p_5d: assessment.probabilities.p_5d,
    p_20d: assessment.probabilities.p_20d,
    p_60d: assessment.probabilities.p_60d,
    posture: assessment.posture,
    time_to_risk_bucket: assessment.time_to_risk_bucket,
    external_shock_score: assessment.scores.external_shock_score,
    posture_trigger_codes: [],
    posture_blocker_codes: [],
    history_source: "current_assessment"
  };
}

function buildRecentChangeSummary(assessment: AssessmentSnapshot, history: AssessmentHistoryPoint[]) {
  const sortedHistory = [...history].sort((left, right) =>
    left.as_of_date.localeCompare(right.as_of_date)
  );
  const currentPoint = buildCurrentHistoryPoint(assessment);
  if (sortedHistory.at(-1)?.as_of_date !== currentPoint.as_of_date) {
    sortedHistory.push(currentPoint);
  }
  const latest = sortedHistory.at(-1) ?? currentPoint;
  const previous = sortedHistory.length >= 2 ? sortedHistory.at(-2) ?? null : null;
  const latestDate = parseHistoryDate(latest.as_of_date);
  const weeklyCutoff = latestDate === null ? null : latestDate - 7 * DAY_MS;
  const weeklyReference =
    weeklyCutoff === null
      ? sortedHistory.at(0) ?? null
      : [...sortedHistory]
          .reverse()
          .find((point) => {
            const pointDate = parseHistoryDate(point.as_of_date);
            return point.as_of_date !== latest.as_of_date && pointDate !== null && pointDate <= weeklyCutoff;
          }) ?? sortedHistory.find((point) => point.as_of_date !== latest.as_of_date) ?? null;

  const dailyScoreDelta = previous ? latest.overall_score - previous.overall_score : null;
  const weeklyScoreDelta = weeklyReference ? latest.overall_score - weeklyReference.overall_score : null;
  const externalDelta = weeklyReference
    ? latest.external_shock_score - weeklyReference.external_shock_score
    : null;
  const p20dDelta = weeklyReference ? latest.p_20d - weeklyReference.p_20d : null;
  const topRiskDriver = assessment.top_risk_drivers[0];
  const topReliefDriver = assessment.top_relief_drivers[0];

  return {
    metrics: [
      {
        label: "今日总风险变化",
        value: formatSignedNumber(dailyScoreDelta, 1, " 分"),
        hint: previous
          ? `相对 ${formatDate(previous.as_of_date)}；当前总风险 ${formatNumber(latest.overall_score)} 分。`
          : "历史点不足，暂不能计算今日变化。"
      },
      {
        label: "近一周总风险变化",
        value: formatSignedNumber(weeklyScoreDelta, 1, " 分"),
        hint: weeklyReference
          ? `相对 ${formatDate(weeklyReference.as_of_date)}；用于判断本周风险是升温还是降温。`
          : "历史点不足，暂不能计算近一周变化。"
      },
      {
        label: "外部冲击变化",
        value: formatSignedNumber(externalDelta, 1, " 分"),
        hint: weeklyReference
          ? `相对 ${formatDate(weeklyReference.as_of_date)}；外部冲击包括 USDJPY、日元套息和跨市场压力。`
          : "历史点不足，暂不能计算外部冲击变化。"
      },
      {
        label: "20日参考概率变化",
        value: formatProbabilityDelta(p20dDelta),
        hint: weeklyReference
          ? `相对 ${formatDate(weeklyReference.as_of_date)}；当前正式概率仍只作参考，不参与 MVP 主结论。`
          : "历史点不足，暂不能计算概率变化。"
      }
    ],
    note: previous
      ? `最新历史点 ${formatDate(latest.as_of_date)}，今日对比 ${formatDate(previous.as_of_date)}；近一周对比 ${
          weeklyReference ? formatDate(weeklyReference.as_of_date) : "历史首点不足"
        }。这些变化来自历史截面，不是逐指标因果归因。`
      : "当前历史截面不足，页面先展示静态风险状态；等本地历史点累积后再给出今日和近一周变化。这不是逐指标因果归因。",
    driverNote: `当前主要上行解释：${topRiskDriver?.display_name ?? "暂无"}${
      topRiskDriver ? `（${topRiskDriver.explanation}）` : ""
    }。当前主要缓冲解释：${topReliefDriver?.display_name ?? "暂无"}${
      topReliefDriver ? `（${topReliefDriver.explanation}）` : ""
    }。`
  };
}

export type {
  DecisionAnalogRow,
  DecisionKeyIndicatorRow,
  DecisionRollingAuditEpisodeRow,
  DecisionRuntimeCard,
  DecisionRuntimeNotice,
  DecisionScoreBandRow,
  DecisionSignalLayerRowModel
} from "./builders";

export function useDecisionViewModel({
  assessment,
  method,
  history,
  posture,
  backtests,
  indicators
}: {
  assessment: AssessmentSnapshot;
  method: AssessmentMethodResponse;
  history: AssessmentHistoryPoint[];
  posture: PostureGuidance;
  backtests: BacktestScenarioSummary[];
  indicators: IndicatorRisk[];
}) {
  const probabilityTrend = useMemo(() => buildProbabilityTrendModel(history), [history]);
  const recentChangeSummary = useMemo(
    () => buildRecentChangeSummary(assessment, history),
    [assessment, history]
  );
  const layerScoreChart = useMemo(() => buildLayerScoreChart(assessment), [assessment]);
  const analogChart = useMemo(
    () => buildAnalogChart(assessment, backtests),
    [assessment, backtests]
  );
  const nearestAnalog = assessment.historical_analogs[0];
  const currentRiskBand = useMemo(
    () => describeRiskScoreBand(assessment.scores.overall_score),
    [assessment.scores.overall_score]
  );
  const usdJpyIndicator = useMemo(
    () =>
      assessment.key_indicators.find((item) => item.indicator_id === "us_external_usdjpy_level"),
    [assessment.key_indicators]
  );
  const triggerClauses = useMemo(() => buildTriggerClauses(posture), [posture]);
  const blockerClauses = useMemo(() => buildBlockerClauses(posture), [posture]);
  const runtimeNotice = useMemo(
    () => buildRuntimeNotice(assessment.runtime),
    [assessment.runtime]
  );
  const runtimeChipLabel = useMemo(
    () => buildRuntimeChipLabel(assessment.runtime),
    [assessment.runtime]
  );
  const runtimeCards = useMemo(
    () => buildRuntimeCards(assessment, usdJpyIndicator),
    [assessment, usdJpyIndicator]
  );
  const heroMetrics = useMemo(() => buildHeroMetrics(assessment), [assessment]);
  const numberAuditRows = useMemo(() => buildNumberAuditRows(assessment), [assessment]);
  const riskHorizonActionMetrics = useMemo(
    () => buildRiskHorizonActionMetrics(assessment),
    [assessment]
  );
  const timeBucketDescription = useMemo(
    () => describeTimeBucket(assessment.time_to_risk_bucket),
    [assessment.time_to_risk_bucket]
  );
  const analogWindowDescription = useMemo(
    () => describeAnalogWindow(nearestAnalog, assessment.time_to_risk_bucket),
    [assessment.time_to_risk_bucket, nearestAnalog]
  );
  const overallScoreText = useMemo(
    () => formatNumber(assessment.scores.overall_score),
    [assessment.scores.overall_score]
  );
  const scoreBandRows = useMemo(
    () => buildScoreBandRows(currentRiskBand.label),
    [currentRiskBand.label]
  );
  const dataTrustMetrics = useMemo(() => buildDataTrustMetrics(assessment), [assessment]);
  const historyEvidenceMetrics = useMemo(
    () => [
      {
        label: "历史证据等级",
        value: historyEvidenceTierLabel(method.history_provenance.evidence_tier),
        hint: decisionHistoryEvidenceCopy(method.history_provenance.note)
      },
      {
        label: "PIT 快照支撑",
        value: `${method.history_provenance.feature_backed_points}/${method.history_provenance.total_points}`
      },
      {
        label: "沿用旧 PIT",
        value: `${method.history_provenance.reused_feature_snapshot_points}`
      },
      {
        label: "旧快照桥接",
        value: `${method.history_provenance.snapshot_bridge_points}`
      }
    ],
    [method.history_provenance]
  );
  const historyEvidenceNote = useMemo(() => {
    const evidenceNote = decisionHistoryEvidenceCopy(method.history_provenance.note);
    const latestFeatureBackedDate = method.history_provenance.latest_feature_backed_date;
    const latestReusedSnapshotDate =
      method.history_provenance.latest_reused_feature_snapshot_date;
    if (latestFeatureBackedDate && latestReusedSnapshotDate) {
      return `${evidenceNote} 最近一条当天 PIT 快照支撑点日期 ${formatDate(latestFeatureBackedDate)}，最近一条沿用旧 PIT 的点日期 ${formatDate(latestReusedSnapshotDate)}。`;
    }
    if (latestFeatureBackedDate) {
      return `${evidenceNote} 最近一条当天 PIT 快照支撑点日期 ${formatDate(latestFeatureBackedDate)}。`;
    }
    if (latestReusedSnapshotDate) {
      return `${evidenceNote} 最近一条沿用旧 PIT 的点日期 ${formatDate(latestReusedSnapshotDate)}。`;
    }
    return evidenceNote;
  }, [method.history_provenance]);
  const postureThresholdMetrics = useMemo(
    () => buildPostureThresholdMetrics(method),
    [method]
  );
  const freeDataReliabilityRows = useMemo(
    () => buildFreeDataReliabilityRows(assessment.key_indicators, method.free_data_source_catalog),
    [assessment.key_indicators, method.free_data_source_catalog]
  );
  const whyNowDrivers = useMemo(
    () => buildWhyNowRiskDrivers(assessment, indicators),
    [assessment, indicators]
  );
  const signalLayerRows = useMemo(
    () => buildSignalLayerRows(assessment, method, posture),
    [assessment, method, posture]
  );
  const analogRows = useMemo(
    () => buildAnalogRows(assessment),
    [assessment]
  );
  const actionPlanMetrics = useMemo(
    () => buildActionPlanMetrics(assessment),
    [assessment]
  );
  const jpyCarryMetrics = useMemo(
    () => buildJpyCarryMetrics(assessment, usdJpyIndicator),
    [assessment, usdJpyIndicator]
  );
  const backtestSummaryMetrics = useMemo(
    () => buildBacktestSummaryMetrics(assessment),
    [assessment]
  );
  const backtestHistoryCoverageText = useMemo(
    () => buildBacktestHistoryCoverageText(assessment.backtest_summary),
    [assessment.backtest_summary]
  );
  const backtestCoverageScopeText = useMemo(
    () => buildBacktestCoverageScopeText(assessment.backtest_summary),
    [assessment.backtest_summary]
  );
  const rollingAuditMetrics = useMemo(
    () => buildRollingAuditMetrics(assessment),
    [assessment]
  );
  const rollingAuditHistoryText = useMemo(
    () => buildRollingAuditHistoryText(assessment.backtest_summary.rolling_audit),
    [assessment.backtest_summary.rolling_audit]
  );
  const rollingAuditScopeText = useMemo(
    () => buildRollingAuditScopeText(assessment.backtest_summary.rolling_audit),
    [assessment.backtest_summary.rolling_audit]
  );
  const rollingAuditBoundaryText = useMemo(
    () => buildRollingAuditBoundaryText(assessment, method),
    [assessment, method]
  );
  const rollingAuditEpisodes = useMemo(
    () => buildRollingAuditEpisodes(assessment.backtest_summary.rolling_audit),
    [assessment.backtest_summary.rolling_audit]
  );

  return {
    probabilityTrend,
    recentChangeSummary,
    layerScoreChart,
    analogChart,
    currentRiskBand,
    triggerClauses,
    blockerClauses,
    runtimeNotice,
    runtimeChipLabel,
    runtimeCards,
    heroMetrics,
    numberAuditRows,
    riskHorizonActionMetrics,
    timeBucketDescription,
    analogWindowDescription,
    overallScoreText,
    scoreBandRows,
    dataTrustMetrics,
    historyEvidenceMetrics,
    historyEvidenceNote,
    postureThresholdMetrics,
    freeDataReliabilityRows,
    whyNowDrivers,
    signalLayerRows,
    analogRows,
    actionPlanMetrics,
    jpyCarryMetrics,
    backtestSummaryMetrics,
    backtestHistoryCoverageText,
    backtestCoverageScopeText,
    rollingAuditMetrics,
    rollingAuditHistoryText,
    rollingAuditScopeText,
    rollingAuditBoundaryText,
    rollingAuditEpisodes
  };
}

import { useMemo } from "react";
import { formatDate, formatNumber, historyEvidenceTierLabel } from "../../format";
import type {
  AssessmentMethodResponse,
  AssessmentHistoryPoint,
  AssessmentSnapshot,
  BacktestScenarioSummary,
  PostureGuidance
} from "../../types";
import {
  buildAnalogChart,
  buildLayerScoreChart,
  buildProbabilityTrendModel
} from "./charts";
import { buildHeroMetrics } from "./heroMetrics";
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
  buildKeyIndicatorRows,
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
  backtests
}: {
  assessment: AssessmentSnapshot;
  method: AssessmentMethodResponse;
  history: AssessmentHistoryPoint[];
  posture: PostureGuidance;
  backtests: BacktestScenarioSummary[];
}) {
  const probabilityTrend = useMemo(() => buildProbabilityTrendModel(history), [history]);
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
        hint: method.history_provenance.note
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
    const latestFeatureBackedDate = method.history_provenance.latest_feature_backed_date;
    const latestReusedSnapshotDate =
      method.history_provenance.latest_reused_feature_snapshot_date;
    if (latestFeatureBackedDate && latestReusedSnapshotDate) {
      return `${method.history_provenance.note} 最近一条当天 PIT 快照支撑点日期 ${formatDate(latestFeatureBackedDate)}，最近一条沿用旧 PIT 的点日期 ${formatDate(latestReusedSnapshotDate)}。`;
    }
    if (latestFeatureBackedDate) {
      return `${method.history_provenance.note} 最近一条当天 PIT 快照支撑点日期 ${formatDate(latestFeatureBackedDate)}。`;
    }
    if (latestReusedSnapshotDate) {
      return `${method.history_provenance.note} 最近一条沿用旧 PIT 的点日期 ${formatDate(latestReusedSnapshotDate)}。`;
    }
    return method.history_provenance.note;
  }, [method.history_provenance]);
  const postureThresholdMetrics = useMemo(
    () => buildPostureThresholdMetrics(method),
    [method]
  );
  const keyIndicatorRows = useMemo(
    () => buildKeyIndicatorRows(assessment.key_indicators),
    [assessment.key_indicators]
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
    keyIndicatorRows,
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

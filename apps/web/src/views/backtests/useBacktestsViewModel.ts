import {
  auditEpisodeClass,
  auditEpisodeLabel,
  backtestSignalSourceLabel,
  formatCount,
  formatDate,
  formatNumber,
  formatPercent,
  humanizeNarrativeCopy,
  postureLabel
} from "../../format";
import type {
  AssessmentSnapshot,
  BacktestScenarioSummary,
  BacktestWindowPoint
} from "../../types";
import {
  buildBacktestSummaryMetrics,
  buildRollingAuditMetrics
} from "../decision/buildersBacktests";
import { buildProbabilityAxisMax, type LineChartModel } from "../decision/charts";
import type { MetricItem } from "../shared/panelHelpers";
import {
  backtestReviewCopy,
  buildBacktestCoverageScopeText,
  buildBacktestHistoryCoverageText,
  buildRollingAuditHistoryText,
  buildRollingAuditScopeText
} from "../shared/backtestCopy";
import {
  currentMvpRiskState,
  mvpProbabilityInputIsAuditOnly,
  mvpRiskStateDisplayLabel
} from "../decision/mvpRiskState";
import { backtestsContent } from "./content";

function formatOptionalDays(value: number | null | undefined) {
  return value === null || value === undefined ? "—" : `${value}d`;
}

export function useBacktestsViewModel({
  assessment,
  backtests,
  timeline
}: {
  assessment: AssessmentSnapshot;
  backtests: BacktestScenarioSummary[];
  timeline: BacktestWindowPoint[];
}) {
  const probabilityValues = timeline.flatMap((point) => [point.p_5d, point.p_20d, point.p_60d]);
  const probabilityMax = probabilityValues.length > 0 ? Math.max(...probabilityValues) : 0;
  const chart: LineChartModel = {
    categories: timeline.map((point) => formatDate(point.as_of_date)),
    maxValue: buildProbabilityAxisMax(probabilityMax),
    valueType: "percent",
    series: [
      { label: "5日窗口", color: "#b45309", values: timeline.map((point) => point.p_5d) },
      { label: "20日窗口", color: "#2563eb", values: timeline.map((point) => point.p_20d) },
      {
        label: "60日窗口",
        color: "#115e59",
        fillColor: "rgba(17, 94, 89, 0.08)",
        values: timeline.map((point) => point.p_60d)
      }
    ]
  };

  const summaryMetrics = buildBacktestSummaryMetrics(assessment);
  const rollingMetrics = buildRollingAuditMetrics(assessment);

  const historyRange = buildBacktestHistoryCoverageText(assessment.backtest_summary);
  const coverageScopeText = buildBacktestCoverageScopeText(assessment.backtest_summary);
  const rollingAuditHistoryRange = buildRollingAuditHistoryText(
    assessment.backtest_summary.rolling_audit
  );
  const rollingAuditScopeText = buildRollingAuditScopeText(
    assessment.backtest_summary.rolling_audit
  );
  const auditOnly = mvpProbabilityInputIsAuditOnly(assessment);
  const mvpState = currentMvpRiskState(assessment);

  const currentPosture = auditOnly
    ? mvpRiskStateDisplayLabel(mvpState.label)
    : postureLabel(assessment.posture);
  const hasActionSignals = assessment.backtest_summary.rolling_audit.actionable_signal_count > 0;
  const historicalReplayCountHint =
    "这是全历史滚动回放里的评估点/区间数量，不是今天新增事件数，也不是当前正式概率准确率。";
  const headlineMetrics: MetricItem[] = [
    {
      label: "动作命中（场景回测）",
      value:
        assessment.backtest_summary.timely_warning_rate === 0
          ? "未形成动作预警"
          : formatPercent(assessment.backtest_summary.timely_warning_rate),
      hint:
        assessment.backtest_summary.timely_warning_rate === 0
          ? "当前回测口径没有形成满足提前量要求的动作级预警，不等于采集失败。"
          : "这是历史场景回测命中率，用来评估过去危机前是否提前亮灯；不是当前危机概率。"
    },
    {
      label: "动作信号点（历史）",
      value: hasActionSignals
        ? formatCount(assessment.backtest_summary.rolling_audit.actionable_signal_count)
        : "无",
      hint: hasActionSignals
        ? historicalReplayCountHint
        : "当前滚动窗口没有准备/对冲/防守动作信号，精度没有可评估分母。"
    },
    {
      label: "纯误报区间（历史）",
      value: hasActionSignals
        ? formatCount(assessment.backtest_summary.rolling_audit.false_positive_episode_count)
        : "无",
      hint: hasActionSignals
        ? historicalReplayCountHint
        : "当前滚动窗口没有准备/对冲/防守动作信号，因此没有可评估的纯误报区间。"
    },
    {
      label: "当前执行节奏",
      value: currentPosture,
      hint: auditOnly
        ? "当前主结论先按 MVP 规则层解释；回测页中的正式概率轨迹只保留为模型复核参考。"
        : undefined
    }
  ];

  const scenarioRows = backtests.map((scenario) => ({
    id: scenario.scenario_id,
    name: scenario.name,
    signalSource: backtestSignalSourceLabel(scenario.signal_source),
    crisisRange: `${formatDate(scenario.crisis_start)} - ${formatDate(scenario.crisis_end)}`,
    leadTime: formatOptionalDays(scenario.lead_time_days),
    actionableLeadTime: formatOptionalDays(scenario.actionable_lead_time_days),
    peakScore: formatNumber(scenario.max_score),
    falsePositives: formatCount(scenario.false_positive_count),
    note: humanizeNarrativeCopy(scenario.note)
  }));

  const episodeRows = assessment.backtest_summary.rolling_audit.classified_episodes.map((episode) => ({
    id: `${episode.classification}-${episode.start_date}-${episode.end_date}`,
    badgeClass: auditEpisodeClass(episode.classification),
    badgeLabel: auditEpisodeLabel(episode.classification),
    startDate: formatDate(episode.start_date),
    endDate: formatDate(episode.end_date),
    duration: formatCount(episode.duration_days, "d"),
    signalCount: formatCount(episode.signal_count),
    note: humanizeNarrativeCopy(backtestReviewCopy(episode.note))
  }));

  return {
    chart,
    headlineMetrics,
    summaryMetrics,
    rollingMetrics,
    historyRange,
    coverageScopeText,
    rollingAuditHistoryRange,
    rollingAuditScopeText,
    currentPosture,
    auditOnly,
    scenarioRows,
    episodeRows
  };
}

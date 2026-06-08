import {
  auditEpisodeClass,
  auditEpisodeLabel,
  backtestSignalSourceLabel,
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
import type { LineChartModel } from "../decision/charts";
import {
  buildBacktestCoverageScopeText,
  buildBacktestHistoryCoverageText,
  buildRollingAuditHistoryText,
  buildRollingAuditScopeText
} from "../shared/backtestCopy";
import { backtestsContent } from "./content";

export function useBacktestsViewModel({
  assessment,
  backtests,
  timeline
}: {
  assessment: AssessmentSnapshot;
  backtests: BacktestScenarioSummary[];
  timeline: BacktestWindowPoint[];
}) {
  const chart: LineChartModel = {
    categories: timeline.map((point) => formatDate(point.as_of_date)),
    maxValue: 1,
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

  const summaryMetrics = [
    ["结构抬升率", formatPercent(assessment.backtest_summary.structural_warning_rate)],
    ["可执行预警率", formatPercent(assessment.backtest_summary.timely_warning_rate)],
    ["漏报率", formatPercent(assessment.backtest_summary.missed_rate)],
    ["平均结构提前量", formatNumber(assessment.backtest_summary.avg_structural_lead_time_days, "d")],
    ["平均动作提前量", formatNumber(assessment.backtest_summary.avg_lead_time_days, "d")],
    ["预警折返", formatNumber(assessment.backtest_summary.total_false_positive_count)],
    ["本地覆盖场景", formatNumber(assessment.backtest_summary.real_scenario_count)],
    ["模板参照场景", formatNumber(assessment.backtest_summary.fallback_scenario_count)]
  ] as Array<[string, string]>;

  const rollingMetrics = [
    ["动作信号精度", formatPercent(assessment.backtest_summary.rolling_audit.actionable_precision)],
    ["动作信号点", formatNumber(assessment.backtest_summary.rolling_audit.actionable_signal_count)],
    ["危机前命中点", formatNumber(assessment.backtest_summary.rolling_audit.pre_crisis_signal_count)],
    ["危机中信号点", formatNumber(assessment.backtest_summary.rolling_audit.in_crisis_signal_count)],
    ["受保护压力点", formatNumber(assessment.backtest_summary.rolling_audit.stress_window_signal_count)],
    ["纯误报点", formatNumber(assessment.backtest_summary.rolling_audit.false_positive_signal_count)],
    ["误报区间", formatNumber(assessment.backtest_summary.rolling_audit.false_positive_episode_count)],
    [
      "最长误报区间",
      formatNumber(assessment.backtest_summary.rolling_audit.longest_false_positive_episode_days, "d")
    ]
  ] as Array<[string, string]>;

  const historyRange = buildBacktestHistoryCoverageText(assessment.backtest_summary);
  const coverageScopeText = buildBacktestCoverageScopeText(assessment.backtest_summary);
  const rollingAuditHistoryRange = buildRollingAuditHistoryText(
    assessment.backtest_summary.rolling_audit
  );
  const rollingAuditScopeText = buildRollingAuditScopeText(
    assessment.backtest_summary.rolling_audit
  );

  const currentPosture = postureLabel(assessment.posture);
  const headlineMetrics = [
    ["动作命中", formatPercent(assessment.backtest_summary.timely_warning_rate)],
    ["纯误报区间", formatNumber(assessment.backtest_summary.rolling_audit.false_positive_episode_count)],
    [
      "最长误报",
      formatNumber(assessment.backtest_summary.rolling_audit.longest_false_positive_episode_days, "d")
    ],
    ["当前执行节奏", currentPosture]
  ] as Array<[string, string]>;

  const scenarioRows = backtests.map((scenario) => ({
    id: scenario.scenario_id,
    name: scenario.name,
    signalSource: backtestSignalSourceLabel(scenario.signal_source),
    crisisRange: `${formatDate(scenario.crisis_start)} - ${formatDate(scenario.crisis_end)}`,
    leadTime: `${scenario.lead_time_days ?? "—"}d`,
    actionableLeadTime: `${scenario.actionable_lead_time_days ?? "—"}d`,
    peakScore: formatNumber(scenario.max_score),
    falsePositives: formatNumber(scenario.false_positive_count),
    note: humanizeNarrativeCopy(scenario.note)
  }));

  const episodeRows = assessment.backtest_summary.rolling_audit.classified_episodes.map((episode) => ({
    id: `${episode.classification}-${episode.start_date}-${episode.end_date}`,
    badgeClass: auditEpisodeClass(episode.classification),
    badgeLabel: auditEpisodeLabel(episode.classification),
    startDate: formatDate(episode.start_date),
    endDate: formatDate(episode.end_date),
    duration: formatNumber(episode.duration_days, "d"),
    signalCount: formatNumber(episode.signal_count),
    note: humanizeNarrativeCopy(episode.note)
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
    scenarioRows,
    episodeRows
  };
}

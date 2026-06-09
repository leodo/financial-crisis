import {
  auditEpisodeClass,
  auditEpisodeLabel,
  formatCount,
  formatDate,
  formatNumber,
  formatPercent,
  userProfileLabel
} from "../../format";
import type { AssessmentMethodResponse, AssessmentSnapshot } from "../../types";
import type { MetricItem } from "../shared/panelHelpers";
import {
  buildBacktestCoverageScopeText,
  buildBacktestHistoryCoverageText,
  buildRollingAuditHistoryText,
  buildRollingAuditScopeText
} from "../shared/backtestCopy";
import type { DecisionRollingAuditEpisodeRow } from "./builderTypes";
import { describeRollingAuditBoundary } from "./logic";

function zeroAwareRate(value: number, zeroLabel: string) {
  return value === 0 ? zeroLabel : formatPercent(value);
}

function noActionSignalText(hasActionSignals: boolean, fallback = "无可评估") {
  return hasActionSignals ? undefined : fallback;
}

export function buildBacktestSummaryMetrics(
  assessment: AssessmentSnapshot
): MetricItem[] {
  const summary = assessment.backtest_summary;
  const noLocalCoverage = summary.real_scenario_count === 0;
  const noActionLeadSample = summary.avg_lead_time_days === null;
  const localCoverageHint =
    "本地覆盖场景为 0 表示当前 SQLite 历史窗口还没有直接覆盖这些危机场景，不是采集失败；页面仍会用模板参照场景辅助解释。";
  const noActionWarningHint =
    "当前回测口径没有形成满足提前量要求的动作级预警，所以这里不能理解成数据错了。";

  return [
    {
      label: "结构抬升率",
      value: formatPercent(summary.structural_warning_rate),
      hint: noLocalCoverage ? "当前主要来自模板参照场景，不能当成本地 PIT 命中率。" : undefined
    },
    {
      label: "可执行预警率",
      value: zeroAwareRate(summary.timely_warning_rate, "未形成动作预警"),
      hint: summary.timely_warning_rate === 0 ? noActionWarningHint : undefined
    },
    {
      label: "漏报率",
      value: summary.missed_rate >= 1 && summary.timely_warning_rate === 0 ? "动作未命中" : formatPercent(summary.missed_rate),
      hint: summary.missed_rate >= 1 && summary.timely_warning_rate === 0 ? noActionWarningHint : undefined
    },
    {
      label: "平均结构提前量",
      value: formatNumber(summary.avg_structural_lead_time_days, "d")
    },
    {
      label: "平均动作提前量",
      value: noActionLeadSample ? "暂无动作样本" : formatNumber(summary.avg_lead_time_days, "d"),
      hint: noActionLeadSample ? noActionWarningHint : undefined
    },
    {
      label: "预警折返",
      value: formatCount(summary.total_false_positive_count)
    },
    {
      label: "本地覆盖场景",
      value: noLocalCoverage ? "暂无覆盖" : formatCount(summary.real_scenario_count),
      hint: noLocalCoverage ? localCoverageHint : undefined
    },
    {
      label: "模板参照场景",
      value: formatCount(summary.fallback_scenario_count),
      hint: noLocalCoverage ? "当前历史类比和提前量更多依赖模板参照，后续要继续补长历史 PIT 数据。" : undefined
    },
    {
      label: "用户风险档位",
      value: userProfileLabel(assessment.user_preferences.profile)
    }
  ];
}

export { buildBacktestCoverageScopeText, buildBacktestHistoryCoverageText };
export { buildRollingAuditHistoryText, buildRollingAuditScopeText };

export function buildRollingAuditMetrics(
  assessment: AssessmentSnapshot
): MetricItem[] {
  const rollingAudit = assessment.backtest_summary.rolling_audit;
  const hasActionSignals = rollingAudit.actionable_signal_count > 0;
  const noSignalHint =
    "当前滚动窗口没有发出准备/对冲/防守动作信号，所以这里不是命中率为 0，而是样本分母为 0。";

  return [
    {
      label: "动作信号精度",
      value: hasActionSignals ? formatPercent(rollingAudit.actionable_precision) : "无动作信号",
      hint: hasActionSignals ? undefined : noSignalHint
    },
    {
      label: "动作信号点",
      value: noActionSignalText(hasActionSignals, "无") ?? formatCount(rollingAudit.actionable_signal_count),
      hint: hasActionSignals ? undefined : noSignalHint
    },
    {
      label: "危机前命中点",
      value: noActionSignalText(hasActionSignals) ?? formatCount(rollingAudit.pre_crisis_signal_count)
    },
    {
      label: "危机中信号点",
      value: noActionSignalText(hasActionSignals) ?? formatCount(rollingAudit.in_crisis_signal_count)
    },
    {
      label: "受保护压力点",
      value: noActionSignalText(hasActionSignals) ?? formatCount(rollingAudit.stress_window_signal_count)
    },
    {
      label: "纯误报点",
      value: noActionSignalText(hasActionSignals) ?? formatCount(rollingAudit.false_positive_signal_count)
    },
    {
      label: "误报区间",
      value: noActionSignalText(hasActionSignals, "无") ?? formatCount(rollingAudit.false_positive_episode_count)
    },
    {
      label: "最长误报区间",
      value: noActionSignalText(hasActionSignals, "无") ?? formatCount(rollingAudit.longest_false_positive_episode_days, "d")
    }
  ];
}

export function buildRollingAuditBoundaryText(
  assessment: AssessmentSnapshot,
  method: AssessmentMethodResponse
) {
  return describeRollingAuditBoundary(method, assessment.backtest_summary.rolling_audit);
}

export function buildRollingAuditEpisodes(
  rollingAudit: AssessmentSnapshot["backtest_summary"]["rolling_audit"]
): DecisionRollingAuditEpisodeRow[] {
  return rollingAudit.classified_episodes.slice(0, 5).map((episode) => ({
    key: `${episode.classification}-${episode.start_date}-${episode.end_date}`,
    classificationClass: auditEpisodeClass(episode.classification),
    classificationLabel: auditEpisodeLabel(episode.classification),
    interval: `${formatDate(episode.start_date)} - ${formatDate(episode.end_date)}`,
    duration: formatCount(episode.duration_days, "d"),
    signalCount: formatCount(episode.signal_count),
    note: episode.note
  }));
}

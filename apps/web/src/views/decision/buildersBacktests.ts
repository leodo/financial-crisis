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

export function buildBacktestSummaryMetrics(
  assessment: AssessmentSnapshot
): MetricItem[] {
  return [
    {
      label: "结构抬升率",
      value: formatPercent(assessment.backtest_summary.structural_warning_rate)
    },
    {
      label: "可执行预警率",
      value: formatPercent(assessment.backtest_summary.timely_warning_rate)
    },
    { label: "漏报率", value: formatPercent(assessment.backtest_summary.missed_rate) },
    {
      label: "平均结构提前量",
      value: formatNumber(assessment.backtest_summary.avg_structural_lead_time_days, "d")
    },
    {
      label: "平均动作提前量",
      value: formatNumber(assessment.backtest_summary.avg_lead_time_days, "d")
    },
    {
      label: "预警折返",
      value: formatCount(assessment.backtest_summary.total_false_positive_count)
    },
    {
      label: "本地覆盖场景",
      value: formatCount(assessment.backtest_summary.real_scenario_count)
    },
    {
      label: "模板参照场景",
      value: formatCount(assessment.backtest_summary.fallback_scenario_count)
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
      value: formatCount(rollingAudit.actionable_signal_count),
      hint: hasActionSignals ? undefined : noSignalHint
    },
    {
      label: "危机前命中点",
      value: formatCount(rollingAudit.pre_crisis_signal_count)
    },
    {
      label: "危机中信号点",
      value: formatCount(rollingAudit.in_crisis_signal_count)
    },
    {
      label: "受保护压力点",
      value: formatCount(rollingAudit.stress_window_signal_count)
    },
    {
      label: "纯误报点",
      value: formatCount(rollingAudit.false_positive_signal_count)
    },
    {
      label: "误报区间",
      value: formatCount(rollingAudit.false_positive_episode_count)
    },
    {
      label: "最长误报区间",
      value: formatCount(rollingAudit.longest_false_positive_episode_days, "d")
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

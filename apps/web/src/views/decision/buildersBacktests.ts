import {
  auditEpisodeClass,
  auditEpisodeLabel,
  formatDate,
  formatNumber,
  formatPercent,
  userProfileLabel
} from "../../format";
import type { AssessmentMethodResponse, AssessmentSnapshot } from "../../types";
import type { MetricItem } from "../shared/panelHelpers";
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
      value: formatNumber(assessment.backtest_summary.total_false_positive_count)
    },
    {
      label: "真实样本",
      value: formatNumber(assessment.backtest_summary.real_scenario_count)
    },
    {
      label: "模板样本",
      value: formatNumber(assessment.backtest_summary.fallback_scenario_count)
    },
    {
      label: "用户风险档位",
      value: userProfileLabel(assessment.user_preferences.profile)
    }
  ];
}

export function buildBacktestHistoryCoverageText(
  backtestSummary: AssessmentSnapshot["backtest_summary"]
) {
  return backtestSummary.history_start && backtestSummary.history_end
    ? `${formatDate(backtestSummary.history_start)} - ${formatDate(backtestSummary.history_end)}`
    : "当前没有可用历史区间。";
}

export function buildRollingAuditMetrics(
  assessment: AssessmentSnapshot
): MetricItem[] {
  return [
    {
      label: "动作信号精度",
      value: formatPercent(assessment.backtest_summary.rolling_audit.actionable_precision)
    },
    {
      label: "动作信号点",
      value: formatNumber(assessment.backtest_summary.rolling_audit.actionable_signal_count)
    },
    {
      label: "危机前命中点",
      value: formatNumber(assessment.backtest_summary.rolling_audit.pre_crisis_signal_count)
    },
    {
      label: "危机中信号点",
      value: formatNumber(assessment.backtest_summary.rolling_audit.in_crisis_signal_count)
    },
    {
      label: "受保护压力点",
      value: formatNumber(assessment.backtest_summary.rolling_audit.stress_window_signal_count)
    },
    {
      label: "纯误报点",
      value: formatNumber(assessment.backtest_summary.rolling_audit.false_positive_signal_count)
    },
    {
      label: "误报区间",
      value: formatNumber(assessment.backtest_summary.rolling_audit.false_positive_episode_count)
    },
    {
      label: "最长误报区间",
      value: formatNumber(
        assessment.backtest_summary.rolling_audit.longest_false_positive_episode_days,
        "d"
      )
    }
  ];
}

export function buildRollingAuditBoundaryText(method: AssessmentMethodResponse) {
  return describeRollingAuditBoundary(method);
}

export function buildRollingAuditEpisodes(
  rollingAudit: AssessmentSnapshot["backtest_summary"]["rolling_audit"]
): DecisionRollingAuditEpisodeRow[] {
  return rollingAudit.classified_episodes.slice(0, 5).map((episode) => ({
    key: `${episode.classification}-${episode.start_date}-${episode.end_date}`,
    classificationClass: auditEpisodeClass(episode.classification),
    classificationLabel: auditEpisodeLabel(episode.classification),
    interval: `${formatDate(episode.start_date)} - ${formatDate(episode.end_date)}`,
    duration: formatNumber(episode.duration_days, "d"),
    signalCount: formatNumber(episode.signal_count),
    note: episode.note
  }));
}

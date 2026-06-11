import { formatDate } from "../../format";
import type { BacktestPerformanceSummary, BacktestRollingAudit } from "../../types";

export function backtestReviewCopy(text: string) {
  return text
    .replaceAll("全历史滚动审计", "全历史滚动复核")
    .replaceAll("滚动审计历史窗口", "滚动复核历史窗口")
    .replaceAll("滚动审计", "滚动历史复核")
    .replaceAll("posture 审计", "执行节奏复核")
    .replaceAll("执行节奏 审计", "执行节奏复核")
    .replaceAll("审计口径", "复核口径")
    .replaceAll("审计", "复核");
}

export function buildBacktestHistoryCoverageText(
  backtestSummary: BacktestPerformanceSummary
) {
  return backtestSummary.history_start && backtestSummary.history_end
    ? `${formatDate(backtestSummary.history_start)} - ${formatDate(backtestSummary.history_end)}`
    : "当前没有可用历史区间。";
}

export function buildBacktestCoverageScopeText(
  backtestSummary: BacktestPerformanceSummary
) {
  const coverageScopeNote = backtestSummary.coverage_scope_note?.trim();
  if (coverageScopeNote) {
    return backtestReviewCopy(coverageScopeNote);
  }

  if (backtestSummary.history_start && backtestSummary.history_end) {
    return `这里的“本地覆盖场景 / 模板参照场景”按场景回测历史窗口 ${formatDate(backtestSummary.history_start)} 到 ${formatDate(backtestSummary.history_end)} 统计；它回答的是危机场景目录里有多少样本能直接落在这段本地历史上，不等于上面默认历史轨迹是否已经进入 PIT 正式证据层。`;
  }

  return "这里的“本地覆盖场景 / 模板参照场景”按当前场景回测历史窗口统计；它回答的是危机场景目录里有多少样本能直接落在本地历史上，不等于上面默认历史轨迹是否已经进入 PIT 正式证据层。";
}

export function buildRollingAuditHistoryText(rollingAudit: BacktestRollingAudit) {
  return rollingAudit.history_start && rollingAudit.history_end
    ? `${formatDate(rollingAudit.history_start)} - ${formatDate(rollingAudit.history_end)}`
    : "当前没有可用滚动复核历史区间。";
}

export function buildRollingAuditScopeText(rollingAudit: BacktestRollingAudit) {
  const scopeNote = rollingAudit.scope_note?.trim();
  if (scopeNote) {
    return backtestReviewCopy(scopeNote);
  }

  if (rollingAudit.history_start && rollingAudit.history_end) {
    return `这里的滚动历史复核按 ${formatDate(rollingAudit.history_start)} 到 ${formatDate(rollingAudit.history_end)} 统计，用于观察动作规则在这段历史里的命中、受保护压力窗口和纯误报分布。`;
  }

  return "这里的滚动历史复核按当前历史窗口统计，用于观察动作规则在这段历史里的命中、受保护压力窗口和纯误报分布。";
}

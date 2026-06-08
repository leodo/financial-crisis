import { formatDate } from "../../format";
import type { BacktestPerformanceSummary } from "../../types";

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
    return coverageScopeNote;
  }

  if (backtestSummary.history_start && backtestSummary.history_end) {
    return `这里的“本地覆盖场景 / 模板参照场景”按场景回测历史窗口 ${formatDate(backtestSummary.history_start)} 到 ${formatDate(backtestSummary.history_end)} 统计；它回答的是危机场景目录里有多少样本能直接落在这段本地历史上，不等于上面默认历史轨迹是否已经进入 PIT 正式证据层。`;
  }

  return "这里的“本地覆盖场景 / 模板参照场景”按当前场景回测历史窗口统计；它回答的是危机场景目录里有多少样本能直接落在本地历史上，不等于上面默认历史轨迹是否已经进入 PIT 正式证据层。";
}

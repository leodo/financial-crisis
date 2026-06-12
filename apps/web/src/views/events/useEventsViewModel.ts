import {
  eventSignalListLabel,
  eventStateLabel,
  eventTypeLabel,
  formatDate,
  formatNumber,
  humanizeNarrativeCopy,
  indicatorRefLabel,
  levelLabel
} from "../../format";
import type { AlertEvent, AssessmentSnapshot } from "../../types";
import type { MetricItem } from "../shared/panelHelpers";

export function useEventsViewModel({
  assessment,
  events
}: {
  assessment: AssessmentSnapshot;
  events: AlertEvent[];
}) {
  const eventState = eventStateLabel(assessment.event_assessment.state);
  const eventSignalLabel = eventSignalListLabel(assessment.event_assessment.state);
  const confirmationScore = formatNumber(assessment.event_assessment.confirmation_score);
  const summaryMetrics: MetricItem[] = [
    {
      label: "事件状态",
      value: eventState,
      hint: "事件层状态只说明外部事件是否共振，不是危机概率。"
    },
    {
      label: "确认分数",
      value: confirmationScore,
      hint: "0-100 分事件确认输入；低分表示尚未形成动作升级证据。"
    },
    {
      label: eventSignalLabel,
      value: `${assessment.event_assessment.confirmed_signals.length}`,
      hint:
        assessment.event_assessment.state === "quiet"
          ? "当前只是近期观察信号，不按已确认事件解释。"
          : "已进入事件确认口径，仍需与市场压力共振。"
    },
    {
      label: "待补缺口",
      value: `${assessment.event_assessment.pending_gaps.length}`,
      hint: "这些是升级前还缺的确认条件，不是当前新增事件数。"
    }
  ];
  const latestEventDate = events
    .map((event) => event.triggered_as_of_date)
    .sort((left, right) => right.localeCompare(left))[0];
  const eventTypeCounts = new Map<string, number>();
  const dimensions = new Set<string>();
  const relatedIndicators = new Set<string>();

  for (const event of events) {
    eventTypeCounts.set(event.event_type, (eventTypeCounts.get(event.event_type) ?? 0) + 1);
    if (event.dimension) {
      dimensions.add(event.dimension);
    }
    for (const indicator of event.related_indicators) {
      relatedIndicators.add(indicator);
    }
  }

  const dominantEventType =
    [...eventTypeCounts.entries()].sort((left, right) => right[1] - left[1])[0]?.[0] ?? null;
  const structureMetrics: MetricItem[] = [
    {
      label: "最近事件日",
      value: latestEventDate ? formatDate(latestEventDate) : "—",
      hint: "按事件触发日期统计，用来判断事件层是否仍然新鲜。"
    },
    {
      label: "最常见事件",
      value: dominantEventType ? eventTypeLabel(dominantEventType) : "—",
      hint: "只是当前列表里的事件类型分布，不代表风险等级。"
    },
    {
      label: "涉及维度",
      value: `${dimensions.size}`,
      hint: "统计近期事件覆盖的风险维度数量。"
    },
    {
      label: "关联指标",
      value: `${relatedIndicators.size}`,
      hint: "统计事件映射到的指标数量，不等同于指标触发数量。"
    }
  ];
  const eventRows = events.map((event) => ({
    id: event.alert_id,
    triggeredDate: formatDate(event.triggered_as_of_date),
    eventType: eventTypeLabel(event.event_type),
    level: levelLabel(event.level),
    reason: humanizeNarrativeCopy(event.trigger_reason),
    relatedIndicators: event.related_indicators.map(indicatorRefLabel).join(" / ")
  }));

  return {
    summaryMetrics,
    eventState,
    eventSignalLabel,
    confirmationScore,
    confirmedSignals: assessment.event_assessment.confirmed_signals.map(humanizeNarrativeCopy),
    pendingGaps: assessment.event_assessment.pending_gaps.map(humanizeNarrativeCopy),
    structureMetrics,
    summary: humanizeNarrativeCopy(assessment.event_assessment.summary),
    eventRows
  };
}

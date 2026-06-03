import {
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
  const confirmationScore = formatNumber(assessment.event_assessment.confirmation_score);
  const summaryMetrics: MetricItem[] = [
    { label: "事件状态", value: eventState },
    { label: "确认分数", value: confirmationScore },
    {
      label: "已确认信号",
      value: `${assessment.event_assessment.confirmed_signals.length}`
    },
    {
      label: "待补缺口",
      value: `${assessment.event_assessment.pending_gaps.length}`
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
      value: latestEventDate ? formatDate(latestEventDate) : "—"
    },
    {
      label: "最常见事件",
      value: dominantEventType ? eventTypeLabel(dominantEventType) : "—"
    },
    {
      label: "涉及维度",
      value: `${dimensions.size}`
    },
    {
      label: "关联指标",
      value: `${relatedIndicators.size}`
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
    confirmationScore,
    confirmedSignals: assessment.event_assessment.confirmed_signals.map(humanizeNarrativeCopy),
    pendingGaps: assessment.event_assessment.pending_gaps.map(humanizeNarrativeCopy),
    structureMetrics,
    summary: humanizeNarrativeCopy(assessment.event_assessment.summary),
    eventRows
  };
}

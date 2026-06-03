import { Activity, Radar } from "lucide-react";
import type { AlertEvent, AssessmentSnapshot } from "../../types";
import {
  BulletList,
  GuideList,
  MetricGrid,
  ResponsiveTable,
  StackedTableCell,
  StateSummary,
  SurfaceHeader
} from "../shared/panelHelpers";
import { eventsContent } from "./content";
import { useEventsViewModel } from "./useEventsViewModel";

export default function EventsView({
  assessment,
  events
}: {
  assessment: AssessmentSnapshot;
  events: AlertEvent[];
}) {
  const {
    summaryMetrics,
    eventState,
    confirmationScore,
    confirmedSignals,
    pendingGaps,
    structureMetrics,
    summary,
    eventRows
  } =
    useEventsViewModel({
      assessment,
      events
    });

  return (
    <section className="workspace">
      <section className="band-grid">
        <section className="surface">
          <SurfaceHeader title="事件摘要" icon={Radar} />
          <MetricGrid items={summaryMetrics} />
        </section>

        <section className="surface">
          <SurfaceHeader title="怎么看这页" icon={Activity} />
          <GuideList rows={eventsContent.guideRows} />
        </section>
      </section>

      <section className="band-grid">
        <section className="surface">
          <SurfaceHeader title="事件层结论" icon={Radar} />
          <StateSummary
            pillLabel={eventState}
            score={confirmationScore}
            summary={summary}
          />
          <BulletList items={confirmedSignals} compact />
        </section>

        <section className="surface">
          <SurfaceHeader title="待补确认" icon={Activity} />
          <BulletList items={pendingGaps} emptyText={eventsContent.pendingEmpty} compact />
          <MetricGrid items={structureMetrics} />
        </section>
      </section>

      <section className="surface">
        <SurfaceHeader title="最近事件" icon={Radar} />
        <ResponsiveTable
          className="wide-table"
          columns={["日期", "事件", "触发说明", "相关指标"]}
          note={eventsContent.tableNote}
        >
          {eventRows.map((event) => (
            <tr key={event.id}>
              <td className="table-nowrap">{event.triggeredDate}</td>
              <StackedTableCell title={event.eventType} details={event.level} />
              <td>{event.reason}</td>
              <td>{event.relatedIndicators}</td>
            </tr>
          ))}
        </ResponsiveTable>
      </section>
    </section>
  );
}

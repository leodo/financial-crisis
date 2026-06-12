import { Database, ShieldCheck } from "lucide-react";
import type { AssessmentSnapshot, DataSource } from "../../types";
import {
  BulletList,
  GuideList,
  MetricGrid,
  MetricPairsGrid,
  ResponsiveTable,
  StackedTableCell,
  SurfaceHeader
} from "../shared/panelHelpers";
import { sourcesContent } from "./content";
import { useSourcesViewModel } from "./useSourcesViewModel";

export default function SourcesView({
  assessment,
  sources
}: {
  assessment: AssessmentSnapshot;
  sources: DataSource[];
}) {
  const { summaryMetrics, coverageMetrics, warnings, sourceRows } = useSourcesViewModel({
    assessment,
    sources
  });

  return (
    <section className="workspace">
      <section className="sources-top-grid">
        <section className="surface">
          <SurfaceHeader title="数据覆盖与源健康摘要" icon={Database} />
          <MetricGrid items={summaryMetrics} />
          <MetricPairsGrid pairs={coverageMetrics} />
          <BulletList items={warnings} emptyText={sourcesContent.warningsEmpty} />
        </section>

        <section className="surface">
          <SurfaceHeader title="免费数据源策略" icon={ShieldCheck} />
          <GuideList rows={sourcesContent.sourceGuideRows} />
        </section>
      </section>

      <section className="surface">
        <SurfaceHeader title="源状态" icon={Database} />
        <ResponsiveTable
          className="wide-table"
          columns={["数据源", "最新状态", "源健康分", "使用建议", "说明"]}
          note={sourcesContent.tableNote}
        >
          {sourceRows.map((source) => (
            <tr key={source.id}>
              <StackedTableCell
                title={source.displayName}
                details={[
                  source.sourceMeta,
                  <span key={`${source.id}-meta`} title={source.sourceMetaHint}>
                    {source.sourceMetaHint}
                  </span>
                ]}
              />
              <StackedTableCell title={source.status} details={source.statusDetail} />
              <StackedTableCell title={source.qualityScore} details={source.qualityDetail} />
              <StackedTableCell
                title={source.usageLabel}
                details={[source.usageDetail, `${source.productionAllowed} · ${source.productionDetail}`]}
              />
              <StackedTableCell title={source.healthMessage} details={source.licenseNote} />
            </tr>
          ))}
        </ResponsiveTable>
        <p className="legend-note">{sourcesContent.summaryNote}</p>
      </section>
    </section>
  );
}

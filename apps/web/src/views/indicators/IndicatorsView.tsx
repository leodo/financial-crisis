import { BadgeInfo, Table2 } from "lucide-react";
import type { IndicatorRisk } from "../../types";
import {
  DetailRows,
  GuideList,
  MetricGrid,
  ResponsiveTable,
  StackedTableCell,
  SurfaceHeader
} from "../shared/panelHelpers";
import { indicatorsContent } from "./content";
import { useIndicatorsViewModel } from "./useIndicatorsViewModel";

export default function IndicatorsView({ indicators }: { indicators: IndicatorRisk[] }) {
  const { summaryMetrics, focusRows, tableRows } = useIndicatorsViewModel({ indicators });

  return (
    <section className="workspace">
      <section className="band-grid">
        <section className="surface">
          <SurfaceHeader title="当前指标摘要" icon={Table2} />
          <MetricGrid items={summaryMetrics} />
          <div className="driver-preview">
            <strong>近端最需盯的指标</strong>
            <DetailRows items={focusRows} compact />
          </div>
        </section>

        <section className="surface">
          <SurfaceHeader title="怎么看这页" icon={BadgeInfo} />
          <GuideList rows={indicatorsContent.guideRows} />
        </section>
      </section>

      <section className="surface">
        <SurfaceHeader title="指标细项" icon={Table2} />
        <ResponsiveTable
          className="wide-table xwide-table"
          columns={["指标", "最近读数", "评分依据", "风险分", "历史位置", "指标级质量"]}
          note={indicatorsContent.tableNote}
        >
          {tableRows.map((risk) => (
            <tr key={risk.id}>
              <StackedTableCell title={risk.indicatorTitle} details={risk.indicatorDetails} />
              <StackedTableCell title={risk.latestValueTitle} details={risk.latestValueDetail} />
              <StackedTableCell title={risk.basisTitle} details={risk.basisDetails} />
              <StackedTableCell title={risk.scoreTitle} details={risk.scoreDetail} />
              <StackedTableCell
                title={risk.percentileTitle}
                details={risk.percentileDetail}
              />
              <StackedTableCell title={risk.qualityTitle} details={risk.qualityDetails} />
            </tr>
          ))}
        </ResponsiveTable>
      </section>
    </section>
  );
}

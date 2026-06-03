import { BadgeInfo, Layers3, ShieldCheck, Siren } from "lucide-react";
import type { AssessmentSnapshot, PostureGuidance, RiskSnapshot } from "../../types";
import {
  DetailRows,
  DriverList,
  GuideList,
  MetricGrid,
  RuleBox,
  SurfaceHeader
} from "../shared/panelHelpers";
import { driversContent } from "./content";
import { useDriversViewModel } from "./useDriversViewModel";

export default function DriversView({
  assessment,
  overview,
  posture
}: {
  assessment: AssessmentSnapshot;
  overview: RiskSnapshot;
  posture: PostureGuidance;
}) {
  const { summaryMetrics, dimensionRows, summaryRows } = useDriversViewModel({
    assessment,
    overview,
    posture
  });

  return (
    <section className="workspace">
      <section className="band-grid">
        <section className="surface">
          <SurfaceHeader title="当前驱动摘要" icon={BadgeInfo} />
          <MetricGrid items={summaryMetrics} />
        </section>

        <section className="surface">
          <SurfaceHeader title="怎么看这页" icon={BadgeInfo} />
          <GuideList rows={driversContent.guideRows} />
        </section>
      </section>

      <section className="band-grid">
        <section className="surface">
          <SurfaceHeader title="上行风险驱动" icon={Siren} />
          <DriverList rows={assessment.top_risk_drivers} />
        </section>

        <section className="surface">
          <SurfaceHeader title="缓冲因素" icon={ShieldCheck} />
          <DriverList rows={assessment.top_relief_drivers} reverse />
        </section>
      </section>

      <section className="surface">
        <SurfaceHeader title="维度解释" icon={Layers3} />
        <div className="dimension-detail-grid">
          {dimensionRows.map((dimension) => (
            <div className="dimension-detail" key={dimension.id}>
              <div className="dimension-row-head">
                <strong>{dimension.label}</strong>
                <b>{dimension.score}</b>
              </div>
              <span className="dimension-caption">{dimension.caption}</span>
              <span className="dimension-caption">{dimension.focus}</span>
              <DetailRows
                compact
                items={dimension.contributors.map((item) => ({
                  id: item.id,
                  title: item.displayName,
                  detail: item.explanation,
                  meta: item.score
                }))}
              />
            </div>
          ))}
        </div>
      </section>

      <section className="surface">
        <SurfaceHeader title="当前结论" icon={BadgeInfo} />
        {summaryRows.map(([title, text]) => (
          <RuleBox key={title} label={title}>
            {text}
          </RuleBox>
        ))}
      </section>
    </section>
  );
}

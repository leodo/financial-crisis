import { BadgeInfo, Layers3, ShieldCheck, Siren } from "lucide-react";
import type { AssessmentSnapshot, IndicatorRisk, PostureGuidance, RiskSnapshot } from "../../types";
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
  indicators,
  overview,
  posture
}: {
  assessment: AssessmentSnapshot;
  indicators: IndicatorRisk[];
  overview: RiskSnapshot;
  posture: PostureGuidance;
}) {
  const { nearTermDrivers, backgroundDrivers, summaryMetrics, dimensionRows, summaryRows } = useDriversViewModel({
    assessment,
    indicators,
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
          <SurfaceHeader title="近端风险驱动" icon={Siren} />
          {nearTermDrivers.length > 0 ? (
            <DriverList rows={nearTermDrivers} />
          ) : (
            <RuleBox label="当前状态">当前没有日频/周频近端高分驱动；先看结构背景和缓冲因素。</RuleBox>
          )}
        </section>

        <section className="surface">
          <SurfaceHeader title="缓冲因素" icon={ShieldCheck} />
          <DriverList rows={assessment.top_relief_drivers} reverse />
        </section>
      </section>

      <section className="surface">
        <SurfaceHeader title="结构背景驱动" icon={Siren} />
        <RuleBox label="怎么读">
          月频、季频、年频或偏旧数据可以解释风险底色，但不代表今天刚出现触发；当前执行动作仍要看近端驱动、事件确认和数据新鲜度是否共振。
        </RuleBox>
        {backgroundDrivers.length > 0 ? (
          <DriverList rows={backgroundDrivers} />
        ) : (
          <RuleBox label="当前状态">当前没有需要单独标注的慢变量背景驱动。</RuleBox>
        )}
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

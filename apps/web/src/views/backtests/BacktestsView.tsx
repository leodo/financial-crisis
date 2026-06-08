import { BadgeInfo, History, ShieldCheck } from "lucide-react";
import { SimpleLineChart } from "../../simpleCharts";
import { humanizeNarrativeCopy } from "../../format";
import type {
  AssessmentSnapshot,
  BacktestScenarioSummary,
  BacktestWindowPoint
} from "../../types";
import {
  GuideList,
  MetricGrid,
  MetricPairsGrid,
  PillTableCell,
  ResponsiveTable,
  RuleBox,
  SurfaceHeader
} from "../shared/panelHelpers";
import { backtestsContent } from "./content";
import { useBacktestsViewModel } from "./useBacktestsViewModel";

export default function BacktestsView({
  assessment,
  backtests,
  timeline
}: {
  assessment: AssessmentSnapshot;
  backtests: BacktestScenarioSummary[];
  timeline: BacktestWindowPoint[];
}) {
  const {
    chart,
    headlineMetrics,
    summaryMetrics,
    rollingMetrics,
    historyRange,
    coverageScopeText,
    currentPosture,
    scenarioRows,
    episodeRows
  } = useBacktestsViewModel({
    assessment,
    backtests,
    timeline
  });

  return (
    <section className="workspace">
      <section className="band-grid">
        <section className="surface">
          <SurfaceHeader title="怎么看这页" icon={BadgeInfo} />
          <MetricGrid
            items={headlineMetrics.map(([label, value]) => ({
              label,
              value
            }))}
          />
        </section>

        <section className="surface">
          <SurfaceHeader title="解读顺序" icon={BadgeInfo} />
          <GuideList rows={backtestsContent.guideRows} />
        </section>
      </section>

      <section className="band-grid backtests-band-grid">
        <section className="surface">
          <SurfaceHeader title="回测摘要" icon={History} />
          <p className="body-copy">{humanizeNarrativeCopy(assessment.backtest_summary.summary)}</p>
          <MetricPairsGrid pairs={summaryMetrics} />
          <RuleBox label="口径区分">{humanizeNarrativeCopy(coverageScopeText)}</RuleBox>
          <RuleBox label="场景回测历史窗口">{historyRange}</RuleBox>
        </section>

        <section className="surface">
          <SurfaceHeader title="滚动审计" icon={ShieldCheck} />
          <p className="body-copy">
            {humanizeNarrativeCopy(assessment.backtest_summary.rolling_audit.summary)}
          </p>
          <MetricPairsGrid pairs={rollingMetrics} />
          <RuleBox label="审计口径">{backtestsContent.auditDefinition}</RuleBox>
          <RuleBox label="区间展示规则">{backtestsContent.episodeDisplayRule}</RuleBox>
        </section>

        <section className="surface">
          <SurfaceHeader title="执行节奏解释" icon={ShieldCheck} />
          <RuleBox label="当前执行节奏">{currentPosture}</RuleBox>
          <RuleBox label="回测用途">{backtestsContent.postureUse}</RuleBox>
        </section>
      </section>

      <section className="surface">
        <SurfaceHeader title="历史轨迹" icon={History} />
        <SimpleLineChart model={chart} height={280} />
      </section>

      <section className="surface">
        <SurfaceHeader title="场景样本" icon={History} />
        <ResponsiveTable
          className="wide-table xwide-table"
          columns={[
            "场景",
            "样本来源",
            "危机区间",
            "结构抬升",
            "动作预警",
            "峰值",
            "折返",
            "说明"
          ]}
          note={backtestsContent.scenariosTableNote}
        >
          {scenarioRows.map((scenario) => (
            <tr key={scenario.id}>
              <td>{scenario.name}</td>
              <td>{scenario.signalSource}</td>
              <td>{scenario.crisisRange}</td>
              <td>{scenario.leadTime}</td>
              <td>{scenario.actionableLeadTime}</td>
              <td>{scenario.peakScore}</td>
              <td>{scenario.falsePositives}</td>
              <td>{scenario.note}</td>
            </tr>
          ))}
        </ResponsiveTable>
      </section>

      <section className="surface">
        <SurfaceHeader title="非危机动作区间" icon={ShieldCheck} />
        {episodeRows.length > 0 ? (
          <>
            <ResponsiveTable
              className="wide-table"
              columns={["类型", "开始", "结束", "持续", "信号点", "说明"]}
              note={backtestsContent.episodesTableNote}
            >
              {episodeRows.map((episode) => (
                <tr key={episode.id}>
                  <PillTableCell className={episode.badgeClass} label={episode.badgeLabel} />
                  <td>{episode.startDate}</td>
                  <td>{episode.endDate}</td>
                  <td>{episode.duration}</td>
                  <td>{episode.signalCount}</td>
                  <td>{episode.note}</td>
                </tr>
              ))}
            </ResponsiveTable>
          </>
        ) : (
          <p className="body-copy">{backtestsContent.noEpisodes}</p>
        )}
      </section>
    </section>
  );
}

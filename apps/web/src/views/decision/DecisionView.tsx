import {
  BadgeInfo,
  ChartColumnIncreasing,
  Database,
  History,
  Layers3,
  ShieldCheck
} from "lucide-react";
import type {
  AssessmentHistoryPoint,
  AssessmentMethodResponse,
  AssessmentSnapshot,
  BacktestScenarioSummary,
  PostureGuidance,
  RiskSnapshot
} from "../../types";
import { compactTechnicalId } from "../../format";
import {
  ClauseList,
  SignalLayerRows
} from "./components";
import { decisionContent } from "./content";
import { SimpleHorizontalBarChart, SimpleLineChart } from "../../simpleCharts";
import {
  BulletList,
  DetailRows,
  MetricGrid,
  RuleBox,
  SurfaceHeader
} from "../shared/panelHelpers";
import {
  DecisionActionPlanPanel,
  DecisionAnalogPanel,
  DecisionBacktestSummaryPanel,
  DecisionEventPanel,
  DecisionJpyCarryPanel,
  DecisionReliefPanel,
  DecisionRollingAuditPanel,
  DecisionWhyNowPanel
} from "./panels";
import {
  DecisionHeroSummary,
  DecisionPosturePlaybook,
  DecisionPrelude,
  DecisionRiskHorizon
} from "./sections";
import { useDecisionViewModel } from "./useDecisionViewModel";

export default function DecisionView({
  assessment,
  history,
  method,
  posture,
  overview,
  backtests
}: {
  assessment: AssessmentSnapshot;
  history: AssessmentHistoryPoint[];
  method: AssessmentMethodResponse;
  posture: PostureGuidance;
  overview: RiskSnapshot;
  backtests: BacktestScenarioSummary[];
}) {
  const {
    probabilityTrend,
    layerScoreChart,
    analogChart,
    currentRiskBand,
    triggerClauses,
    blockerClauses,
    runtimeNotice,
    runtimeChipLabel,
    runtimeCards,
    heroMetrics,
    riskHorizonActionMetrics,
    timeBucketDescription,
    analogWindowDescription,
    overallScoreText,
    scoreBandRows,
    dataTrustMetrics,
    historyEvidenceMetrics,
    historyEvidenceNote,
    postureThresholdMetrics,
    keyIndicatorRows,
    signalLayerRows,
    analogRows,
    actionPlanMetrics,
    jpyCarryMetrics,
    backtestSummaryMetrics,
    backtestHistoryCoverageText,
    rollingAuditMetrics,
    rollingAuditBoundaryText,
    rollingAuditEpisodes
  } = useDecisionViewModel({
    assessment,
    method,
    history,
    posture,
    backtests
  });

  return (
    <section className="workspace">
      <DecisionPrelude
        runtimeNotice={runtimeNotice}
        runtimeChipLabel={runtimeChipLabel}
        runtimeCards={runtimeCards}
      />

      <section className="dashboard-columns">
        <div className="dashboard-column">
          <DecisionHeroSummary
            assessment={assessment}
            posture={posture}
            heroMetrics={heroMetrics}
          />

          <section className="surface">
            <SurfaceHeader title="当前结论怎么来的" icon={BadgeInfo} />
            <SignalLayerRows rows={signalLayerRows} />
          </section>

          <section className="surface">
            <SurfaceHeader title="总风险强度怎么读" icon={ChartColumnIncreasing} />
            <div className="score-summary">
              <div className="score-summary-head">
                <span className="kicker">当前总风险强度</span>
                <div className="score-value">{overallScoreText}</div>
                <p>{currentRiskBand.description}</p>
              </div>
              <div className="score-band-list">
                {scoreBandRows.map((band) => {
                  return (
                    <div className={band.active ? "score-band active" : "score-band"} key={band.label}>
                      <div>
                        <strong>{band.label}</strong>
                        <span>{band.rangeText}</span>
                      </div>
                      <span>{band.note}</span>
                    </div>
                  );
                })}
              </div>
            </div>
            <div className="legend-note">
              `0-100` 强度分只是指标组合所处的历史压力位置。即使接近 `100`，也不等于危机已经发生，
              更不等于 `100%` 会发生。
            </div>
          </section>

          <section className="surface">
            <SurfaceHeader title="关键指标是否最新" icon={Database} />
            <DetailRows
              items={keyIndicatorRows.map((item) => ({
                id: item.id,
                title: item.title,
                detail: item.detail,
                note: item.note
              }))}
            />
          </section>

          <section className="surface">
            <SurfaceHeader title="概率轨迹" icon={History} />
            <SimpleLineChart model={probabilityTrend.chart} height={320} />
            <div className="legend-note">{probabilityTrend.note}</div>
          </section>

          <DecisionWhyNowPanel assessment={assessment} posture={posture} />

          <DecisionReliefPanel assessment={assessment} posture={posture} overview={overview} />

          <section className="surface">
            <SurfaceHeader title="风险层拆解" icon={Layers3} />
            <SimpleHorizontalBarChart model={layerScoreChart} />
            <div className="legend-note">
              结构性风险决定脆弱性底色，触发性风险决定窗口压缩速度，外部冲击决定是否出现共振放大。
            </div>
          </section>

          <DecisionAnalogPanel analogChart={analogChart} analogRows={analogRows} />

          <section className="surface">
            <SurfaceHeader title="可信度与数据缺口" icon={Database} />
            <MetricGrid items={dataTrustMetrics} />
            <BulletList items={assessment.data_trust.warnings} compact />
            <RuleBox label="历史轨迹证据层">{historyEvidenceNote}</RuleBox>
            <MetricGrid items={historyEvidenceMetrics} />
          </section>

          <DecisionBacktestSummaryPanel
            assessment={assessment}
            backtestSummaryMetrics={backtestSummaryMetrics}
            historyCoverageText={backtestHistoryCoverageText}
          />
        </div>

        <div className="dashboard-column">
          <DecisionRiskHorizon
            assessment={assessment}
            actionMetrics={riskHorizonActionMetrics}
            timeBucketDescription={timeBucketDescription}
            analogWindowDescription={analogWindowDescription}
          />

          <DecisionPosturePlaybook assessment={assessment} />

          <section className="surface">
            <SurfaceHeader title="当前执行条款" icon={ShieldCheck} />
            <ClauseList
              title="已触发"
              emptyText={decisionContent.clauses.triggeredEmpty}
              clauses={triggerClauses}
            />
            <ClauseList
              title="被阻断"
              emptyText={decisionContent.clauses.blockedEmpty}
              clauses={blockerClauses}
            />
            <MetricGrid items={postureThresholdMetrics} />
            <RuleBox label="历史审计策略">
              {compactTechnicalId(method.runtime_thresholds.history_runtime_policy_version).value}
            </RuleBox>
          </section>

          <DecisionActionPlanPanel
            assessment={assessment}
            actionPlanMetrics={actionPlanMetrics}
          />

          <DecisionEventPanel assessment={assessment} />

          <DecisionJpyCarryPanel assessment={assessment} jpyCarryMetrics={jpyCarryMetrics} />

          <DecisionRollingAuditPanel
            assessment={assessment}
            rollingAuditMetrics={rollingAuditMetrics}
            rollingAuditBoundaryText={rollingAuditBoundaryText}
            rollingAuditEpisodes={rollingAuditEpisodes}
          />
        </div>
      </section>
    </section>
  );
}

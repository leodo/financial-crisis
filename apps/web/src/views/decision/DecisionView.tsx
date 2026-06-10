import {
  BadgeInfo,
  ChartColumnIncreasing,
  ClipboardCheck,
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
import {
  compactTechnicalId,
  formatProbabilityBasisPoints,
  formatProbabilityDecimal,
  formatProbabilityPercentExact
} from "../../format";
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
import { probabilityDiagnosticAnomalyHorizons } from "./probabilityDiagnostics";
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
    numberAuditRows,
    riskHorizonActionMetrics,
    timeBucketDescription,
    analogWindowDescription,
    overallScoreText,
    scoreBandRows,
    dataTrustMetrics,
    historyEvidenceMetrics,
    historyEvidenceNote,
    postureThresholdMetrics,
    freeDataReliabilityRows,
    signalLayerRows,
    analogRows,
    actionPlanMetrics,
    jpyCarryMetrics,
    backtestSummaryMetrics,
    backtestHistoryCoverageText,
    backtestCoverageScopeText,
    rollingAuditMetrics,
    rollingAuditHistoryText,
    rollingAuditScopeText,
    rollingAuditBoundaryText,
    rollingAuditEpisodes
  } = useDecisionViewModel({
    assessment,
    method,
    history,
    posture,
    backtests
  });
  const probabilityTrajectoryAuditNote = buildProbabilityTrajectoryAuditNote(assessment);

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
            <SurfaceHeader title="当前数字可信度清单" icon={ClipboardCheck} />
            <DetailRows
              items={numberAuditRows.map((item) => ({
                id: item.id,
                title: item.title,
                detail: item.detail,
                meta: item.meta,
                note: item.note
              }))}
            />
          </section>

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
            <SurfaceHeader title="关键免费数据源是否可信" icon={Database} />
            <DetailRows
              items={freeDataReliabilityRows.map((item) => ({
                id: item.id,
                title: item.title,
                detail: item.detail,
                meta: item.meta,
                note: item.note
              }))}
            />
          </section>

          <section className="surface">
            <SurfaceHeader title="概率轨迹" icon={History} />
            {probabilityTrajectoryAuditNote ? (
              <RuleBox label="概率轨迹复核">{probabilityTrajectoryAuditNote}</RuleBox>
            ) : null}
            <MetricGrid className="probability-trend-metrics" items={probabilityTrend.summaryMetrics} />
            <SimpleLineChart model={probabilityTrend.chart} height={320} />
            <div className="legend-note">{probabilityTrend.note}</div>
            <div className="probability-trend-drilldowns">
              <div className="probability-trend-relative">
                <div className="section-subhead">
                  <strong>20日局部放大</strong>
                  <span>只重画 20d，使用 20d 自身范围的纵轴；用来判断它是不是一条真正的直线。</span>
                </div>
                <SimpleLineChart model={probabilityTrend.twentyDayZoomChart} height={190} />
              </div>
              <div className="probability-trend-relative">
                <div className="section-subhead">
                  <strong>近期相对变化</strong>
                  <span>每条线按自身近期区间归一，专门用来看 20d 这类低位线是否真的没有变化。</span>
                </div>
                <SimpleLineChart model={probabilityTrend.relativeChart} height={190} />
              </div>
            </div>
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
            coverageScopeText={backtestCoverageScopeText}
          />
        </div>

        <div className="dashboard-column">
          <DecisionRiskHorizon
            assessment={assessment}
            method={method}
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
            rollingAuditHistoryText={rollingAuditHistoryText}
            rollingAuditScopeText={rollingAuditScopeText}
            rollingAuditBoundaryText={rollingAuditBoundaryText}
            rollingAuditEpisodes={rollingAuditEpisodes}
          />
        </div>
      </section>
    </section>
  );
}

function buildProbabilityTrajectoryAuditNote(assessment: AssessmentSnapshot): string | null {
  const { p_5d: p5d, p_20d: p20d, p_60d: p60d } = assessment.probabilities;
  const anomalyHorizons = probabilityDiagnosticAnomalyHorizons(assessment);
  const twentyDayIsCold = p20d > 0 && p20d < p5d * 0.25 && p20d < p60d * 0.25;

  if (anomalyHorizons.length === 0 && !twentyDayIsCold) {
    return null;
  }

  const twentyDayCopy = `20d 当前是 ${formatProbabilityPercentExact(
    p20d
  )}（${formatProbabilityBasisPoints(p20d)}，接口 ${formatProbabilityDecimal(p20d)}）`;
  const comparisonCopy = `5d 是 ${formatProbabilityPercentExact(
    p5d
  )}，60d 是 ${formatProbabilityPercentExact(p60d)}`;

  if (anomalyHorizons.length > 0) {
    return `${twentyDayCopy}，明显低于 ${comparisonCopy}。这不是图表渲染把 20d 画错，而是 active release 的 ${anomalyHorizons.join(
      " / "
    )} 概率命中 USDJPY 高位 tail 压低读数的语义异常；折线和极小概率只作为模型审计证据保留，不参与“离风险还有多远”、减仓或对冲时距判断。`;
  }

  return `${twentyDayCopy}，明显低于 ${comparisonCopy}。主图使用统一纵轴时 20d 会贴近底部；下方 20d 局部放大图用于确认它是否真的在变化。当前先按 20d head 偏冷处理，不在运行时硬抬概率。`;
}

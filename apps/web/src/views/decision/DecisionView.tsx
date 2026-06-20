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
  IndicatorRisk,
  PostureGuidance,
  RiskSnapshot
} from "../../types";
import {
  compactTechnicalId,
  formatPercentPrecise,
  formatProbabilityBasisPoints,
  formatProbabilityDecimal,
  formatProbabilityPercentExact,
  timeBucketLabel
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
import type { MetricItem } from "../shared/panelHelpers";
import {
  DecisionActionPlanPanel,
  DecisionAnalogPanel,
  DecisionBacktestSummaryPanel,
  DecisionEventPanel,
  DecisionJpyCarryPanel,
  DecisionReliefPanel,
  DecisionWhyNowPanel
} from "./panels";
import {
  DecisionHeroSummary,
  DecisionPosturePlaybook,
  DecisionPrelude,
  DecisionRiskHorizon
} from "./sections";
import {
  currentMvpRiskState,
  mvpProbabilityInputIsAuditOnly,
  mvpRiskStateDisplayLabel
} from "./mvpRiskState";
import {
  probabilityDiagnosticAnomalyHorizons,
  probabilityModelFinalHorizonValues,
  probabilityModelTwentyDayIsCold
} from "./probabilityDiagnostics";
import type { DecisionNumberAuditRow } from "./numberAudit";
import {
  hasRuntimeProbabilityOverride,
  probabilityModelFinalSnapshotValue,
  probabilityRuntimeReferenceNote
} from "./signalLayerBuilders";
import { useDecisionViewModel } from "./useDecisionViewModel";

export default function DecisionView({
  assessment,
  history,
  method,
  posture,
  overview,
  backtests,
  indicators
}: {
  assessment: AssessmentSnapshot;
  history: AssessmentHistoryPoint[];
  method: AssessmentMethodResponse;
  posture: PostureGuidance;
  overview: RiskSnapshot;
  backtests: BacktestScenarioSummary[];
  indicators: IndicatorRisk[];
}) {
  const {
    probabilityTrend,
    recentChangeSummary,
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
    whyNowDrivers,
    analogRows,
    actionPlanMetrics,
    jpyCarryMetrics,
    backtestSummaryMetrics,
    backtestHistoryCoverageText,
    backtestCoverageScopeText,
  } = useDecisionViewModel({
    assessment,
    method,
    history,
    posture,
    backtests,
    indicators
  });
  const probabilityTrajectoryAuditNote = buildProbabilityTrajectoryAuditNote(assessment);
  const probabilityReferenceOnly = mvpProbabilityInputIsAuditOnly(assessment);
  const runtimeProbabilityOverride = hasRuntimeProbabilityOverride(assessment);

  return (
    <section className="workspace">
      <DecisionOperatorBrief
        assessment={assessment}
        recentChangeMetrics={recentChangeSummary.metrics}
        overallScoreText={overallScoreText}
      />

      <DecisionHeroSummary
        assessment={assessment}
        posture={posture}
        heroMetrics={heroMetrics}
      />

      <DecisionPrelude
        runtimeNotice={runtimeNotice}
        runtimeChipLabel={runtimeChipLabel}
        runtimeCards={runtimeCards}
      />

      <section className="dashboard-columns">
        <div className="dashboard-column">
          <section className="surface number-audit-surface">
            <SurfaceHeader title="当前数字说明" icon={ClipboardCheck} />
            <NumberAuditRows rows={numberAuditRows} />
          </section>

          <section className="surface">
            <SurfaceHeader title="当前结论怎么来的" icon={BadgeInfo} />
            <SignalLayerRows rows={signalLayerRows} />
          </section>

          <section className="surface">
            <SurfaceHeader title="今日与本周变化" icon={History} />
            <MetricGrid items={recentChangeSummary.metrics} />
            <div className="change-insight-grid" aria-label="趋势变化行动解读">
              <article className="change-insight-card change-insight-card-primary">
                <span>趋势判断</span>
                <strong>{recentChangeSummary.interpretation.tone}</strong>
                <p>
                  <HighlightedAuditText
                    className="inline-number-value"
                    text={recentChangeSummary.interpretation.summary}
                  />
                </p>
              </article>
              <article className="change-insight-card">
                <span>动作含义</span>
                <p>
                  <HighlightedAuditText
                    className="inline-number-value"
                    text={recentChangeSummary.interpretation.actionMeaning}
                  />
                </p>
              </article>
              <article className="change-insight-card">
                <span>升温 / 缓冲来源</span>
                <p>
                  <HighlightedAuditText
                    className="inline-number-value"
                    text={recentChangeSummary.interpretation.riskDriver}
                  />
                </p>
                <p>
                  <HighlightedAuditText
                    className="inline-number-value"
                    text={recentChangeSummary.interpretation.reliefDriver}
                  />
                </p>
              </article>
              <article className="change-insight-card">
                <span>接下来盯什么</span>
                <BulletList
                  compact
                  items={recentChangeSummary.interpretation.nextWatchItems.map((item) => (
                    <HighlightedAuditText className="inline-number-value" text={item} />
                  ))}
                />
              </article>
            </div>
            <RuleBox label="变化口径">{recentChangeSummary.note}</RuleBox>
            <RuleBox label="当前主要解释">{recentChangeSummary.driverNote}</RuleBox>
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
            <SurfaceHeader
              title={probabilityReferenceOnly ? "概率轨迹（参考）" : "概率轨迹"}
              icon={History}
            />
            {probabilityTrajectoryAuditNote ? (
              <RuleBox label="概率轨迹说明">{probabilityTrajectoryAuditNote}</RuleBox>
            ) : null}
            {probabilityReferenceOnly ? (
              <>
                <MetricGrid
                  className="probability-trend-metrics"
                  items={[
                    {
                      label: runtimeProbabilityOverride ? "5日参考值（运行口径）" : "5日参考值",
                      value: formatProbabilityPercentExact(assessment.probabilities.p_5d),
                      hint: buildReferenceProbabilityHint(assessment, 5)
                    },
                    {
                      label: runtimeProbabilityOverride ? "20日参考值（运行口径）" : "20日参考值",
                      value: formatProbabilityPercentExact(assessment.probabilities.p_20d),
                      hint: buildReferenceProbabilityHint(assessment, 20)
                    },
                    {
                      label: runtimeProbabilityOverride ? "60日参考值（运行口径）" : "60日参考值",
                      value: formatProbabilityPercentExact(assessment.probabilities.p_60d),
                      hint: buildReferenceProbabilityHint(assessment, 60)
                    }
                  ]}
                />
                <RuleBox label="为什么这里不展开细轨迹">
                  正式概率当前只作为参考输入。为了避免把极小概率的坐标压缩、局部放大或相对变化误读成可执行时距，
                  当前不展示这组三期限的细轨迹图；如需排查模型链路，请优先看上方“当前数字说明”和每张概率卡底部
                  默认收起的技术细节。
                </RuleBox>
              </>
            ) : (
              <>
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
              </>
            )}
          </section>

          <DecisionWhyNowPanel
            assessment={assessment}
            posture={posture}
            drivers={whyNowDrivers}
          />

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
            <RuleBox label="历史评估策略">
              {compactTechnicalId(method.runtime_thresholds.history_runtime_policy_version).value}
            </RuleBox>
          </section>

          <DecisionActionPlanPanel
            assessment={assessment}
            actionPlanMetrics={actionPlanMetrics}
          />

          <DecisionEventPanel assessment={assessment} />

          <DecisionJpyCarryPanel assessment={assessment} jpyCarryMetrics={jpyCarryMetrics} />
        </div>
      </section>
    </section>
  );
}

function DecisionOperatorBrief({
  assessment,
  recentChangeMetrics,
  overallScoreText
}: {
  assessment: AssessmentSnapshot;
  recentChangeMetrics: MetricItem[];
  overallScoreText: string;
}) {
  const probabilityReferenceOnly = mvpProbabilityInputIsAuditOnly(assessment);
  const mvpState = currentMvpRiskState(assessment);
  const todayChange = recentChangeMetrics.find((item) => item.label.includes("今日总风险"));
  const weekChange = recentChangeMetrics.find((item) => item.label.includes("近一周总风险"));
  const externalChange = recentChangeMetrics.find((item) => item.label.includes("外部冲击"));
  const actionCopy = probabilityReferenceOnly
    ? "观察，不自动交易"
    : assessment.position_guidance.governance.auto_execution_allowed
      ? "可执行前复核"
      : "人工复核后执行";

  return (
    <section className="surface operator-brief-surface" aria-label="当前操作摘要">
      <SurfaceHeader title="当前操作摘要" icon={ShieldCheck} />
      <div className="operator-brief-grid">
        <article className="operator-brief-card operator-brief-card-primary">
          <span>主判断</span>
          <strong>
            {probabilityReferenceOnly
              ? mvpRiskStateDisplayLabel(mvpState.label)
              : timeBucketLabel(assessment.time_to_risk_bucket)}
          </strong>
          <small>
            <HighlightedAuditText
              className="inline-number-value"
              text={`总风险 ${overallScoreText} 分；正式概率当前${
                probabilityReferenceOnly ? "仅作参考" : "可参与判断"
              }。`}
            />
          </small>
        </article>
        <article className="operator-brief-card">
          <span>今日 / 本周</span>
          <strong>
            <HighlightedChangeValue value={todayChange?.value ?? "—"} />
            <span className="operator-change-separator">/</span>
            <HighlightedChangeValue value={weekChange?.value ?? "—"} />
          </strong>
          <small>
            <HighlightedAuditText
              className="inline-number-value"
              text={`${externalChange ? `外部冲击 ${externalChange.value}；` : ""}用来判断风险是升温还是降温。`}
            />
          </small>
        </article>
        <article className="operator-brief-card">
          <span>离动作窗口</span>
          <strong>
            {probabilityReferenceOnly ? "未进入动作窗口" : timeBucketLabel(assessment.time_to_risk_bucket)}
          </strong>
          <small>
            {probabilityReferenceOnly
              ? "不按低概率判断风险很远，主看规则层、事件和关键数据是否共振。"
              : "按正式概率、事件确认和数据新鲜度综合判断。"}
          </small>
        </article>
        <article className="operator-brief-card operator-brief-card-action">
          <span>现在做什么</span>
          <strong>{actionCopy}</strong>
          <small>
            <HighlightedAuditText
              className="inline-number-value"
              text={`风险资产 ${formatPercentPrecise(
                assessment.position_guidance.target_equity_exposure_pct / 100
              )} / 现金 ${formatPercentPrecise(
                assessment.position_guidance.target_cash_pct / 100
              )} / 对冲 ${formatPercentPrecise(
                assessment.position_guidance.hedge_ratio_pct / 100
              )} / 期权 ${formatPercentPrecise(
                assessment.position_guidance.option_overlay_pct / 100
              )}`}
            />
          </small>
        </article>
      </div>
    </section>
  );
}

function HighlightedChangeValue({ value }: { value: string }) {
  return <span className={`operator-change-value ${changeValueToneClassName(value)}`}>{value}</span>;
}

function NumberAuditRows({ rows }: { rows: DecisionNumberAuditRow[] }) {
  return (
    <div className="number-audit-list">
      {rows.map((item) => (
        <article
          className={`number-audit-card ${numberAuditToneClass(item.id)}`}
          data-audit-detail={item.detail}
          data-audit-meta={item.meta}
          data-audit-note={item.note}
          data-audit-title={item.title}
          key={item.id}
        >
          <div className="number-audit-card-main">
            <strong>
              <HighlightedAuditText text={item.title} />
            </strong>
            <div className="number-audit-detail">
              {splitAuditDetail(item.detail).map((segment, index) => (
                <span className={auditSegmentClassName(segment)} key={`${item.id}-${index}`}>
                  <HighlightedAuditText text={segment} />
                </span>
              ))}
            </div>
            <details className="number-audit-note">
              <summary>口径与限制</summary>
              <span className="number-audit-note-copy">
                <HighlightedAuditText text={item.note} />
              </span>
            </details>
          </div>
          <span className="number-audit-meta">
            <HighlightedAuditText text={item.meta} />
          </span>
        </article>
      ))}
    </div>
  );
}

function HighlightedAuditText({
  text,
  className = "number-audit-value"
}: {
  text: string;
  className?: string;
}) {
  return (
    <>
      {text.split(NUMBER_TOKEN_PATTERN).map((part, index) => {
        if (part.length === 0) {
          return null;
        }
        if (NUMBER_TOKEN_EXACT_PATTERN.test(part)) {
          return (
            <span className={`${className} ${changeValueToneClassName(part)}`} key={`${part}-${index}`}>
              {part}
            </span>
          );
        }
        return part;
      })}
    </>
  );
}

function changeValueToneClassName(value: string) {
  const trimmed = value.trim();
  if (trimmed.startsWith("+")) {
    return "numeric-value-up";
  }
  if (trimmed.startsWith("-")) {
    return "numeric-value-down";
  }
  return "numeric-value-flat";
}

const NUMBER_TOKEN_PATTERN =
  /(\d{4}-\d{2}-\d{2}(?:\s+\d{2}:\d{2}(?::\d{2})?)?|[+-]?\d+(?:\.\d+)?\s?-\s?[+-]?\d+(?:\.\d+)?|[+-]?\d{1,3}(?:,\d{3})*(?:\.\d+)?\s?(?:%|bp|bps|d|分|条|天)?)/g;

const NUMBER_TOKEN_EXACT_PATTERN =
  /^(\d{4}-\d{2}-\d{2}(?:\s+\d{2}:\d{2}(?::\d{2})?)?|[+-]?\d+(?:\.\d+)?\s?-\s?[+-]?\d+(?:\.\d+)?|[+-]?\d{1,3}(?:,\d{3})*(?:\.\d+)?\s?(?:%|bp|bps|d|分|条|天)?)$/;

function splitAuditDetail(detail: string): string[] {
  const segments = detail
    .split(/\s+\/\s+|[；，]| · /)
    .map((segment) => segment.trim())
    .filter(Boolean);
  return segments.length > 0 ? segments : [detail];
}

function auditSegmentClassName(segment: string): string {
  const hasHighlightedValue = segment
    .split(NUMBER_TOKEN_PATTERN)
    .some((part) => NUMBER_TOKEN_EXACT_PATTERN.test(part));
  return hasHighlightedValue
    ? "number-audit-segment"
    : "number-audit-segment number-audit-segment-copy";
}

function numberAuditToneClass(rowId: string): string {
  switch (rowId) {
    case "mvp-state":
      return "number-audit-tone-primary";
    case "risk-score-snapshot":
    case "event-confirmation":
    case "jpy-carry":
      return "number-audit-tone-risk";
    case "decision-reliability":
    case "probability-snapshot":
      return "number-audit-tone-reference";
    case "usdjpy":
    case "freshness":
      return "number-audit-tone-data";
    case "position-guidance":
      return "number-audit-tone-action";
    default:
      return "number-audit-tone-neutral";
  }
}

function buildProbabilityTrajectoryAuditNote(assessment: AssessmentSnapshot): string | null {
  const { p_5d: p5d, p_20d: p20d, p_60d: p60d } = assessment.probabilities;
  const anomalyHorizons = probabilityDiagnosticAnomalyHorizons(assessment);
  const auditOnly = mvpProbabilityInputIsAuditOnly(assessment);
  const twentyDayIsCold = probabilityModelTwentyDayIsCold(assessment);
  const runtimeReferenceNote = probabilityRuntimeReferenceNote(assessment);

  if (!auditOnly && anomalyHorizons.length === 0 && !twentyDayIsCold) {
    return null;
  }

  const pageReferenceCopy = `页面当前显示的运行口径参考值：5d ${formatProbabilityPercentExact(
    p5d
  )} / 20d ${formatProbabilityPercentExact(p20d)} / 60d ${formatProbabilityPercentExact(
    p60d
  )}`;
  const sourceCopy = runtimeReferenceNote ? `${runtimeReferenceNote} ` : `${pageReferenceCopy}。`;
  const modelColdCopy = probabilityModelTwentyDayColdCopy(assessment);

  if (anomalyHorizons.length > 0) {
    return `${sourceCopy}${modelColdCopy ? `${modelColdCopy}；` : ""}这不是图表渲染把 20d 画错，而是 active release 的 ${anomalyHorizons.join(
      " / "
    )} 概率命中 USDJPY 高位 tail 压低读数的语义异常；折线和极小概率当前作为参考证据保留，不单独参与“离风险还有多远”、减仓或对冲时距判断。`;
  }

  if (auditOnly) {
    return `正式概率当前已被 MVP 降为参考输入；${sourceCopy}折线保留用于复核模型和数据链路，不单独参与“离风险还有多远”、减仓或对冲时距判断。`;
  }

  return `${modelColdCopy ?? "模型原始 20d head 当前偏冷"}。主图使用统一纵轴时 20d 会贴近底部；下方 20d 局部放大图用于确认它是否真的在变化。当前先按 20d head 偏冷处理，不在运行时硬抬概率。`;
}

function probabilityModelTwentyDayColdCopy(assessment: AssessmentSnapshot): string | null {
  const values = probabilityModelFinalHorizonValues(assessment);
  if (!values || !probabilityModelTwentyDayIsCold(assessment)) {
    return null;
  }

  return `模型原始 20d ${formatProbabilityPercentExact(
    values.p20d
  )} 明显低于模型原始 5d ${formatProbabilityPercentExact(
    values.p5d
  )} 和 60d ${formatProbabilityPercentExact(values.p60d)}`;
}

function buildReferenceProbabilityHint(
  assessment: AssessmentSnapshot,
  horizonDays: 5 | 20 | 60
): string {
  const diagnostic = assessment.probability_diagnostics.horizon_overlays.find(
    (item) => item.horizon_days === horizonDays
  );
  const pageValue =
    horizonDays === 5
      ? assessment.probabilities.p_5d
      : horizonDays === 20
        ? assessment.probabilities.p_20d
        : assessment.probabilities.p_60d;
  const modelFinal = diagnostic?.final_probability;
  const hasOverride =
    diagnostic?.runtime_final_probability !== undefined &&
    modelFinal !== undefined &&
    Math.abs(diagnostic.runtime_final_probability - modelFinal) > 1e-9;

  if (hasOverride && modelFinal !== undefined) {
    return `${formatProbabilityBasisPoints(pageValue)} · 页面 ${formatProbabilityDecimal(
      pageValue
    )} · 模型原始 ${formatProbabilityPercentExact(modelFinal)}`;
  }

  return `${formatProbabilityBasisPoints(pageValue)} · 页面 ${formatProbabilityDecimal(pageValue)}`;
}

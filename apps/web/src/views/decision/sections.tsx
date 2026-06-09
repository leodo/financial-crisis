import { BadgeInfo, ShieldCheck, Siren } from "lucide-react";
import {
  formatPercentPrecise,
  formatProbabilityPercentExact,
  postureClass,
  postureLabel,
  timeBucketLabel
} from "../../format";
import type { AssessmentMethodResponse, AssessmentSnapshot, PostureGuidance } from "../../types";
import type { MetricItem } from "../shared/panelHelpers";
import { MetricGrid, RuleBox, SurfaceHeader } from "../shared/panelHelpers";
import { PostureLadder, ProbabilityTile } from "./components";
import { decisionContent } from "./content";
import type {
  DecisionRuntimeCard,
  DecisionRuntimeNotice
} from "./useDecisionViewModel";

export function DecisionPrelude({
  runtimeNotice,
  runtimeChipLabel,
  runtimeCards
}: {
  runtimeNotice?: DecisionRuntimeNotice;
  runtimeChipLabel: string;
  runtimeCards: DecisionRuntimeCard[];
}) {
  return (
    <>
      <section className="callout">
        <BadgeInfo size={18} />
        <div>
          <strong>{decisionContent.prelude.calloutTitle}</strong>
          <span>{decisionContent.prelude.calloutBody}</span>
        </div>
      </section>

      {runtimeNotice ? (
        <section className={runtimeNotice.tone}>
          <strong>{runtimeNotice.title}</strong>
          <div>{runtimeNotice.body}</div>
        </section>
      ) : null}

      <section className="runtime-surface">
        <div className="runtime-header">
          <div>
            <strong>当前数据状态</strong>
            <span>{decisionContent.prelude.runtimeSummary}</span>
          </div>
          <span className={runtimeChipLabel === "Demo 样例" ? "runtime-chip runtime-chip-demo" : "runtime-chip"}>
            {runtimeChipLabel}
          </span>
        </div>

        <div className="runtime-card-grid">
          {runtimeCards.map((card) => (
            <div className="runtime-card" key={card.label}>
              <span>{card.label}</span>
              <strong>{card.value}</strong>
              <small>{card.detail}</small>
            </div>
          ))}
        </div>
        <div className="runtime-data-check">{decisionContent.prelude.dataCheckHint}</div>
      </section>
    </>
  );
}

export function DecisionHeroSummary({
  assessment,
  posture,
  heroMetrics
}: {
  assessment: AssessmentSnapshot;
  posture: PostureGuidance;
  heroMetrics: MetricItem[];
}) {
  return (
    <section className={`hero-surface ${postureClass(assessment.posture)}`}>
      <span className="kicker">当前执行节奏</span>
      <div className="hero-value">{postureLabel(assessment.posture)}</div>
      <div className="hero-subtitle">风险窗口判断：{timeBucketLabel(assessment.time_to_risk_bucket)}</div>
      <p>{posture.summary}</p>
      <MetricGrid className="hero-metrics" items={heroMetrics} />
    </section>
  );
}

export function DecisionRiskHorizon({
  assessment,
  method,
  actionMetrics,
  timeBucketDescription,
  analogWindowDescription
}: {
  assessment: AssessmentSnapshot;
  method: AssessmentMethodResponse;
  actionMetrics: MetricItem[];
  timeBucketDescription: string;
  analogWindowDescription: string;
}) {
  const horizonDiagnostic = (horizonDays: number) =>
    assessment.probability_diagnostics.horizon_overlays.find(
      (diagnostic) => diagnostic.horizon_days === horizonDays
    );
  const riskHorizonSanityNote = buildRiskHorizonSanityNote(assessment, method);

  return (
    <section className="surface">
      <SurfaceHeader title="离风险还有多远" icon={Siren} />
      <div className="probability-grid">
        <ProbabilityTile
          label="5 个交易日"
          value={assessment.probabilities.p_5d}
          hint={decisionContent.riskHorizon.tileHints.p5d}
          threshold={method.runtime_thresholds.defend_p5d}
          thresholdLabel="防守线"
          diagnostic={horizonDiagnostic(5)}
        />
        <ProbabilityTile
          label="20 个交易日"
          value={assessment.probabilities.p_20d}
          hint={decisionContent.riskHorizon.tileHints.p20d}
          threshold={method.runtime_thresholds.hedge_p20d}
          thresholdLabel="对冲线"
          diagnostic={horizonDiagnostic(20)}
        />
        <ProbabilityTile
          label="60 个交易日"
          value={assessment.probabilities.p_60d}
          hint={decisionContent.riskHorizon.tileHints.p60d}
          threshold={method.runtime_thresholds.prepare_p60d}
          thresholdLabel="准备线"
          diagnostic={horizonDiagnostic(60)}
        />
      </div>
      {riskHorizonSanityNote ? (
        <RuleBox label="模型一致性复核">{riskHorizonSanityNote}</RuleBox>
      ) : null}
      <div className="legend-note">{decisionContent.riskHorizon.bandLegend}</div>
      <RuleBox label={decisionContent.riskHorizon.priorVsAction.title}>
        {decisionContent.riskHorizon.priorVsAction.body}
      </RuleBox>
      <MetricGrid items={actionMetrics} />
      <RuleBox label="时距判断">{timeBucketDescription}</RuleBox>
      <RuleBox label="历史参照">{analogWindowDescription}</RuleBox>
    </section>
  );
}

function buildRiskHorizonSanityNote(
  assessment: AssessmentSnapshot,
  method: AssessmentMethodResponse
): string | null {
  const { p_5d: p5d, p_20d: p20d, p_60d: p60d } = assessment.probabilities;
  const thresholds = method.runtime_thresholds;
  const thresholdShares = [
    thresholds.defend_p5d > 0 ? p5d / thresholds.defend_p5d : null,
    thresholds.hedge_p20d > 0 ? p20d / thresholds.hedge_p20d : null,
    thresholds.prepare_p60d > 0 ? p60d / thresholds.prepare_p60d : null
  ].filter((value): value is number => value !== null);
  const allFarBelowEntry =
    thresholdShares.length === 3 && thresholdShares.every((share) => share < 0.03);
  const twentyDayIsCold = p20d > 0 && p20d < p5d * 0.25 && p20d < p60d * 0.25;

  if (twentyDayIsCold && allFarBelowEntry) {
    return `当前三条正式概率都远低于进入线，且 20日窗口 ${formatProbabilityPercentExact(
      p20d
    )} 明显低于 5日 ${formatProbabilityPercentExact(p5d)} 和 60日 ${formatProbabilityPercentExact(
      p60d
    )}。这不是“风险被证明为 0”，而是活跃正式模型当前没有捕捉到临近危机信号，同时 20d head 输出偏冷；决策上仍要结合关键指标、事件确认、历史类比和动作层。进入线占比约为 5d ${formatPercentPrecise(
      thresholdShares[0]
    )} / 20d ${formatPercentPrecise(thresholdShares[1])} / 60d ${formatPercentPrecise(
      thresholdShares[2]
    )}。`;
  }

  if (twentyDayIsCold) {
    return `20日窗口 ${formatProbabilityPercentExact(
      p20d
    )} 明显低于 5日和 60日，这说明当前 20d head 输出偏冷；它不是画图错误，后续应通过训练和 release review 修复模型，而不是运行时硬抬概率。`;
  }

  if (allFarBelowEntry) {
    return `当前三条正式概率都远低于进入线，系统因此判断风险时距仍在 normal 区间；这表示当前模型没有看到临近危机证据，不等于市场风险为 0。`;
  }

  return null;
}

export function DecisionPosturePlaybook({
  assessment
}: {
  assessment: AssessmentSnapshot;
}) {
  return (
    <section className="surface">
      <SurfaceHeader title="四档执行节奏在做什么" icon={ShieldCheck} />
      <p className="body-copy">{decisionContent.posture.intro}</p>
      <PostureLadder current={assessment.posture} />
    </section>
  );
}

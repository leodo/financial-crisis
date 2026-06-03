import { BadgeInfo, ShieldCheck, Siren } from "lucide-react";
import {
  postureClass,
  postureLabel,
  timeBucketLabel
} from "../../format";
import type { AssessmentSnapshot, PostureGuidance } from "../../types";
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
  actionMetrics,
  timeBucketDescription,
  analogWindowDescription
}: {
  assessment: AssessmentSnapshot;
  actionMetrics: MetricItem[];
  timeBucketDescription: string;
  analogWindowDescription: string;
}) {
  return (
    <section className="surface">
      <SurfaceHeader title="离风险还有多远" icon={Siren} />
      <div className="probability-grid">
        <ProbabilityTile
          label="5 个交易日"
          value={assessment.probabilities.p_5d}
          hint={decisionContent.riskHorizon.tileHints.p5d}
        />
        <ProbabilityTile
          label="20 个交易日"
          value={assessment.probabilities.p_20d}
          hint={decisionContent.riskHorizon.tileHints.p20d}
        />
        <ProbabilityTile
          label="60 个交易日"
          value={assessment.probabilities.p_60d}
          hint={decisionContent.riskHorizon.tileHints.p60d}
        />
      </div>
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

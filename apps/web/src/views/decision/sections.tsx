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
import {
  findProbabilityDiagnosticAnomaly,
  formatPercentagePointGap,
  formatThresholdMultiple,
  PostureLadder,
  ProbabilityTile
} from "./components";
import type { ProbabilityDiagnosticAnomaly } from "./components";
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
  const riskDistanceSummary = buildRiskDistanceSummary(assessment, method);
  const riskHorizonSanityNote = buildRiskHorizonSanityNote(assessment, method);

  return (
    <section className="surface">
      <SurfaceHeader title="离风险还有多远" icon={Siren} />
      <div className="risk-horizon-summary">
        <div>
          <span>风险时距</span>
          <strong>{riskDistanceSummary.bucketLabel}</strong>
          <small>{riskDistanceSummary.bucketDetail}</small>
        </div>
        <div>
          <span>最接近的动作线</span>
          <strong>{riskDistanceSummary.nearestValue}</strong>
          <small>{riskDistanceSummary.nearestDetail}</small>
        </div>
        <div>
          <span>模型读数状态</span>
          <strong>{riskDistanceSummary.modelStatus}</strong>
          <small>{riskDistanceSummary.modelDetail}</small>
        </div>
      </div>
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

function buildRiskDistanceSummary(
  assessment: AssessmentSnapshot,
  method: AssessmentMethodResponse
) {
  const { p_5d: p5d, p_20d: p20d, p_60d: p60d } = assessment.probabilities;
  const thresholds = method.runtime_thresholds;
  const anomalyHorizons = assessment.probability_diagnostics.horizon_overlays
    .map((diagnostic) => ({
      horizonDays: diagnostic.horizon_days,
      anomaly: findProbabilityDiagnosticAnomaly(diagnostic)
    }))
    .filter((row): row is { horizonDays: number; anomaly: ProbabilityDiagnosticAnomaly } =>
      row.anomaly !== null
    );
  const rows = [
    { label: "5d 防守线", value: p5d, threshold: thresholds.defend_p5d },
    { label: "20d 对冲线", value: p20d, threshold: thresholds.hedge_p20d },
    { label: "60d 准备线", value: p60d, threshold: thresholds.prepare_p60d }
  ].map((row) => ({
    ...row,
    gap: Math.max(0, row.threshold - row.value),
    share: row.threshold > 0 ? row.value / row.threshold : null,
    multiple: row.threshold > 0 && row.value > 0 ? row.threshold / row.value : null
  }));
  const rankedRows = rows
    .filter((row): row is typeof row & { share: number } => row.share !== null)
    .sort((left, right) => right.share - left.share);
  const nearest = rankedRows[0];
  const mechanicalDistanceCopy = rows
    .map((row) => {
      if (row.share === null) {
        return `${row.label} 未配置`;
      }
      const multipleCopy = row.multiple ? ` / 需 ${formatThresholdMultiple(row.multiple)}` : "";
      return `${row.label} 审计比例 ${formatProbabilityPercentExact(row.share)}${multipleCopy}`;
    })
    .join("；");
  const allShares = rows
    .map((row) => row.share)
    .filter((value): value is number => value !== null);
  const allFarBelowEntry = allShares.length === 3 && allShares.every((share) => share < 0.03);
  const twentyDayIsCold = p20d > 0 && p20d < p5d * 0.25 && p20d < p60d * 0.25;

  const bucketDetail: Record<AssessmentSnapshot["time_to_risk_bucket"], string> = {
    normal: "当前没有形成数月、数周或当下风险窗口；这不是零风险证明。",
    months: "风险更像数月级脆弱性，适合先准备现金、执行顺序和保护工具。",
    weeks: "风险已经压缩到数周级别，应重点看对冲和减仓执行节奏。",
    now: "近端窗口已经打开，应优先确认流动性、杠杆和保护动作。"
  };

  const modelStatus = twentyDayIsCold
    ? "20d 偏冷待审计"
    : allFarBelowEntry
      ? "未捕捉临近窗口"
      : "读数可解释";
  const modelStatusWithAnomaly =
    anomalyHorizons.length > 0 ? "读数待审计" : modelStatus;
  const modelDetail =
    anomalyHorizons.length > 0
      ? `${anomalyHorizons
          .map((row) => `${row.horizonDays}d`)
          .join(" / ")} 命中 USDJPY 高位 tail 压低概率的语义异常；这些小数只能说明 active release 当前输出偏冷，不能当成风险已远离。`
      : twentyDayIsCold
        ? `20d 只有 ${formatProbabilityPercentExact(p20d)}，明显低于 5d ${formatProbabilityPercentExact(
            p5d
          )} 和 60d ${formatProbabilityPercentExact(p60d)}；先按模型审计处理，不在运行时硬抬概率。`
        : allFarBelowEntry
          ? "三期限都远低于动作进入线，系统因此给出常态观察；仍需结合关键指标和事件确认复核。"
          : "当前概率和动作进入线之间没有明显显示层异常。";

  return {
    bucketLabel: timeBucketLabel(assessment.time_to_risk_bucket),
    bucketDetail: bucketDetail[assessment.time_to_risk_bucket],
    nearestValue:
      anomalyHorizons.length > 0
        ? "不作距离结论"
        : nearest
          ? nearest.gap === 0
            ? "已触线"
            : nearest.multiple
              ? `需 ${formatThresholdMultiple(nearest.multiple)}`
              : "无法计算"
          : "未配置",
    nearestDetail:
      anomalyHorizons.length > 0
        ? `当前存在模型方向异常，不能用机械触线比例判断“离风险还有多远”。机械值只保留为审计证据：${mechanicalDistanceCopy}。`
        : nearest
          ? `${nearest.label} 最接近；还差 ${formatPercentagePointGap(
              nearest.gap
            )}。机械完成度 ${formatProbabilityPercentExact(nearest.share)}，不是剩余天数。${
              nearest.multiple && nearest.gap > 0
                ? ` 触线仍需约 ${formatThresholdMultiple(nearest.multiple)}。`
                : ""
            }`
          : "当前 release 没有返回可比较的动作进入线。",
    modelStatus: modelStatusWithAnomaly,
    modelDetail
  };
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
  const thresholdMultiples = [
    p5d > 0 ? thresholds.defend_p5d / p5d : null,
    p20d > 0 ? thresholds.hedge_p20d / p20d : null,
    p60d > 0 ? thresholds.prepare_p60d / p60d : null
  ].filter((value): value is number => value !== null);
  const allFarBelowEntry =
    thresholdShares.length === 3 && thresholdShares.every((share) => share < 0.03);
  const twentyDayIsCold = p20d > 0 && p20d < p5d * 0.25 && p20d < p60d * 0.25;
  const hasAllThresholdMultiples = thresholdMultiples.length === 3;
  const anomalyHorizons = assessment.probability_diagnostics.horizon_overlays
    .map((diagnostic) => ({
      horizonDays: diagnostic.horizon_days,
      anomaly: findProbabilityDiagnosticAnomaly(diagnostic)
    }))
    .filter((row): row is { horizonDays: number; anomaly: ProbabilityDiagnosticAnomaly } =>
      row.anomaly !== null
    );

  if (anomalyHorizons.length > 0) {
    return `当前 ${anomalyHorizons
      .map((row) => `${row.horizonDays}日`)
      .join(" / ")} 概率命中模型语义异常：高 USDJPY tail 在 active release 中反而压低概率。页面保留正式输出用于审计，但这些极小数不应被解释成“离风险很远”；下一步应修训练约束和 release review，而不是在运行时硬抬概率。${
      hasAllThresholdMultiples
        ? `按当前进入线机械反推，触线仍需 5d ${formatThresholdMultiple(
            thresholdMultiples[0]
          )}、20d ${formatThresholdMultiple(thresholdMultiples[1])}、60d ${formatThresholdMultiple(
            thresholdMultiples[2]
          )} 的同期限概率放大。`
        : ""
    }`;
  }

  if (twentyDayIsCold && allFarBelowEntry) {
    return `当前三条正式概率都远低于进入线，且 20日窗口 ${formatProbabilityPercentExact(
      p20d
    )} 明显低于 5日 ${formatProbabilityPercentExact(p5d)} 和 60日 ${formatProbabilityPercentExact(
      p60d
    )}。这不是“风险被证明为 0”，而是活跃正式模型当前没有捕捉到临近危机信号，同时 20d head 输出偏冷；决策上仍要结合关键指标、事件确认、历史类比和动作层。${
      hasAllThresholdMultiples
        ? `按当前进入线反推，触线大约还需要 5d ${formatThresholdMultiple(
            thresholdMultiples[0]
          )}、20d ${formatThresholdMultiple(thresholdMultiples[1])}、60d ${formatThresholdMultiple(
            thresholdMultiples[2]
          )} 的同期限概率放大，这比“占比小于多少”更适合判断距离。`
        : ""
    }`;
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

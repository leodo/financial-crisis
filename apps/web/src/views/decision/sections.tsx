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
  formatPercentagePointGap,
  PostureLadder,
  ProbabilityTile
} from "./components";
import { decisionContent } from "./content";
import {
  findProbabilityDiagnosticAnomaly,
  probabilityDiagnosticAnomalyHorizons,
  type ProbabilityDiagnosticAnomaly
} from "./probabilityDiagnostics";
import type {
  DecisionRuntimeCard,
  DecisionRuntimeNotice
} from "./useDecisionViewModel";
import {
  currentMvpRiskState,
  mvpProbabilityInputIsAuditOnly,
  mvpRiskStateDetail
} from "./mvpRiskState";

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
  const anomalyHorizons = probabilityDiagnosticAnomalyHorizons(assessment);
  const mvpRiskState = currentMvpRiskState(assessment);
  const probabilityAuditActive = mvpProbabilityInputIsAuditOnly(assessment);
  const auditReason =
    anomalyHorizons.length > 0
      ? `${anomalyHorizons.join(" / ")} 正式概率读数命中模型方向异常`
      : "后端已将正式概率降级为审计态";
  const heroClassName = `hero-surface ${postureClass(assessment.posture)}${
    probabilityAuditActive ? " probability-audit" : ""
  }`;
  const heroValue = probabilityAuditActive ? mvpRiskState.label : postureLabel(assessment.posture);
  const heroSubtitle = probabilityAuditActive
    ? `当前 MVP 风险状态；正式风险窗口待审计（原模型桶：${timeBucketLabel(assessment.time_to_risk_bucket)}）`
    : `风险窗口判断：${timeBucketLabel(assessment.time_to_risk_bucket)}`;
  const heroSummary = probabilityAuditActive
    ? `${mvpRiskState.summary}${auditReason}；当前不输出“离风险还有多远”的仓位结论，原执行节奏只作为非概率层参考。`
    : posture.summary;
  const decisionAnswers = buildDecisionAnswerItems(
    assessment,
    posture,
    probabilityAuditActive,
    mvpRiskState
  );

  return (
    <section className={heroClassName}>
      <span className="kicker">{probabilityAuditActive ? "当前 MVP 风险状态" : "当前执行节奏"}</span>
      <div className="hero-value">{heroValue}</div>
      <div className="hero-subtitle">{heroSubtitle}</div>
      <div className="decision-answer-grid" aria-label="首屏四问摘要">
        {decisionAnswers.map((item) => (
          <div className={`decision-answer ${item.tone}`} key={item.id}>
            <span>{item.question}</span>
            <strong>{item.answer}</strong>
            <small>{item.detail}</small>
          </div>
        ))}
      </div>
      <p>{heroSummary}</p>
      <MetricGrid className="hero-metrics" items={heroMetrics} />
    </section>
  );
}

interface DecisionAnswerItem {
  id: string;
  question: string;
  answer: string;
  detail: string;
  tone: "answer-observe" | "answer-prepare" | "answer-hedge" | "answer-defend" | "answer-audit";
}

function buildDecisionAnswerItems(
  assessment: AssessmentSnapshot,
  posture: PostureGuidance,
  probabilityAuditActive: boolean,
  mvpRiskState: ReturnType<typeof currentMvpRiskState>
): DecisionAnswerItem[] {
  const postureTone = decisionAnswerTone(assessment.posture);
  const action = decisionActionCopy(probabilityAuditActive ? mvpRiskState.code : assessment.posture);
  const distanceAnswer = probabilityAuditActive
    ? "未知（概率待审计）"
    : timeBucketLabel(assessment.time_to_risk_bucket);
  const distanceDetail = probabilityAuditActive
    ? "正式 5d / 20d / 60d 暂不参与时距，先看规则层和关键数据是否共振。"
    : "按正式概率、事件确认和数据新鲜度合成，不等于自动交易倒计时。";
  const whyDetail = probabilityAuditActive
    ? firstReadableBlocker(mvpRiskState.blockers) ??
      "正式概率审计中，当前主结论只按规则层、数据新鲜度和事件确认解释。"
    : posture.summary;

  return [
    {
      id: "danger-now",
      question: "当前是否危险",
      answer: probabilityAuditActive ? mvpRiskState.label : postureLabel(assessment.posture),
      detail: probabilityAuditActive
        ? "当前未把 formal 小概率当成低风险证明。"
        : "这是当前执行节奏，不是危机发生概率。",
      tone: probabilityAuditActive ? "answer-audit" : postureTone
    },
    {
      id: "risk-distance",
      question: "离风险多远",
      answer: distanceAnswer,
      detail: distanceDetail,
      tone: probabilityAuditActive ? "answer-audit" : postureTone
    },
    {
      id: "primary-reason",
      question: "为什么",
      answer: probabilityAuditActive ? "正式概率不作主输入" : "证据分层合成",
      detail: whyDetail,
      tone: probabilityAuditActive ? "answer-audit" : postureTone
    },
    {
      id: "current-action",
      question: "现在做什么",
      answer: action.label,
      detail: `${action.detail} 当前预算：${positionBudgetCopy(assessment)}。`,
      tone: action.tone
    }
  ];
}

function decisionAnswerTone(
  posture: AssessmentSnapshot["posture"]
): DecisionAnswerItem["tone"] {
  if (posture === "prepare") {
    return "answer-prepare";
  }
  if (posture === "hedge") {
    return "answer-hedge";
  }
  if (posture === "defend") {
    return "answer-defend";
  }
  return "answer-observe";
}

function decisionActionCopy(code: string): {
  label: string;
  detail: string;
  tone: DecisionAnswerItem["tone"];
} {
  if (code === "prepare") {
    return {
      label: "准备",
      detail: "补现金、降脆弱性，先准备保护工具",
      tone: "answer-prepare"
    };
  }
  if (code === "hedge") {
    return {
      label: "对冲",
      detail: "开始落地保护性对冲和净敞口收缩",
      tone: "answer-hedge"
    };
  }
  if (code === "defend") {
    return {
      label: "防守",
      detail: "资本保全、流动性和去杠杆优先",
      tone: "answer-defend"
    };
  }
  return {
    label: "观察",
    detail: "维持核心仓位，重点监控触发层和数据复核",
    tone: "answer-observe"
  };
}

function firstReadableBlocker(blockers: string[]): string | null {
  const blocker = blockers.find((item) => !item.includes("正式 5d/20d/60d"));
  return blocker ?? blockers[0] ?? null;
}

function positionBudgetCopy(assessment: AssessmentSnapshot): string {
  const guidance = assessment.position_guidance;
  return [
    `风险资产 ${formatPercentPrecise(guidance.target_equity_exposure_pct / 100)}`,
    `现金 ${formatPercentPrecise(guidance.target_cash_pct / 100)}`,
    `对冲 ${formatPercentPrecise(guidance.hedge_ratio_pct / 100)}`,
    `期权 ${formatPercentPrecise(guidance.option_overlay_pct / 100)}`
  ].join(" / ");
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
  const probabilityAuditActive = mvpProbabilityInputIsAuditOnly(assessment);

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
          <span>{riskDistanceSummary.nearestLabel}</span>
          <strong>{riskDistanceSummary.nearestValue}</strong>
          <small>{riskDistanceSummary.nearestDetail}</small>
        </div>
        <div>
          <span>模型读数状态</span>
          <strong>{riskDistanceSummary.modelStatus}</strong>
          <small>{riskDistanceSummary.modelDetail}</small>
        </div>
      </div>
      {probabilityAuditActive ? (
        <RuleBox label="MVP 决策口径">{mvpRiskStateDetail(assessment)}</RuleBox>
      ) : null}
      <div className="probability-grid">
        <ProbabilityTile
          label="5 个交易日"
          value={assessment.probabilities.p_5d}
          hint={decisionContent.riskHorizon.tileHints.p5d}
          threshold={method.runtime_thresholds.defend_p5d}
          thresholdLabel="防守线"
          diagnostic={horizonDiagnostic(5)}
          forceAuditOnly={probabilityAuditActive}
        />
        <ProbabilityTile
          label="20 个交易日"
          value={assessment.probabilities.p_20d}
          hint={decisionContent.riskHorizon.tileHints.p20d}
          threshold={method.runtime_thresholds.hedge_p20d}
          thresholdLabel="对冲线"
          diagnostic={horizonDiagnostic(20)}
          forceAuditOnly={probabilityAuditActive}
        />
        <ProbabilityTile
          label="60 个交易日"
          value={assessment.probabilities.p_60d}
          hint={decisionContent.riskHorizon.tileHints.p60d}
          threshold={method.runtime_thresholds.prepare_p60d}
          thresholdLabel="准备线"
          diagnostic={horizonDiagnostic(60)}
          forceAuditOnly={probabilityAuditActive}
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
      <RuleBox label="时距判断">
        {probabilityAuditActive
          ? "正式概率当前处于审计状态，页面暂停输出“数月 / 数周 / 当下”的仓位时距结论；当前只按 MVP 规则层判断是否需要观察、准备、对冲或防守。"
          : timeBucketDescription}
      </RuleBox>
      <RuleBox label="历史参照">
        {probabilityAuditActive
          ? `${analogWindowDescription} 当前历史类比只作为结构参照，不把异常偏低的 formal 概率解释成风险已经远离。`
          : analogWindowDescription}
      </RuleBox>
    </section>
  );
}

function buildRiskDistanceSummary(
  assessment: AssessmentSnapshot,
  method: AssessmentMethodResponse
) {
  const mvpRiskState = currentMvpRiskState(assessment);
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
  const allShares = rows
    .map((row) => row.share)
    .filter((value): value is number => value !== null);
  const allFarBelowEntry = allShares.length === 3 && allShares.every((share) => share < 0.03);
  const twentyDayIsCold = p20d > 0 && p20d < p5d * 0.25 && p20d < p60d * 0.25;
  const auditOnly = mvpProbabilityInputIsAuditOnly(assessment);
  const mvpBlockers = mvpRiskState.blockers.length
    ? mvpRiskState.blockers.join("；")
    : "MVP 规则层尚未看到足够证据支持动作升级。";
  const mvpNextActions = mvpRiskState.next_actions.length
    ? mvpRiskState.next_actions.join("；")
    : "继续观察关键指标、事件确认和历史类比是否共振。";

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
  const modelStatusWithAnomaly = auditOnly ? "正式概率待审计" : modelStatus;
  const modelDetail =
    auditOnly && anomalyHorizons.length > 0
      ? `${anomalyHorizons
          .map((row) => `${row.horizonDays}d`)
          .join(" / ")} 命中 USDJPY 高位 tail 压低概率的语义异常；这些小数只能说明 active release 当前输出偏冷，不能当成风险已远离，也不能用于离场/对冲时距结论。`
      : auditOnly
        ? "后端 MVP 状态已把正式概率降级为审计读数；这些小数不能当成风险时距、减仓或对冲结论。"
      : twentyDayIsCold
        ? `20d 只有 ${formatProbabilityPercentExact(p20d)}，明显低于 5d ${formatProbabilityPercentExact(
            p5d
          )} 和 60d ${formatProbabilityPercentExact(p60d)}；先按模型审计处理，不在运行时硬抬概率。`
        : allFarBelowEntry
          ? "三期限都远低于动作进入线，系统因此给出常态观察；仍需结合关键指标和事件确认复核。"
          : "当前概率和动作进入线之间没有明显显示层异常。";

  return {
    bucketLabel: auditOnly ? mvpRiskState.label : timeBucketLabel(assessment.time_to_risk_bucket),
    bucketDetail:
      auditOnly
        ? `${mvpRiskState.summary} 原模型桶是 ${timeBucketLabel(
            assessment.time_to_risk_bucket
          )}，但 ${
            anomalyHorizons.length > 0
              ? `${anomalyHorizons.map((row) => `${row.horizonDays}d`).join(" / ")} 概率读数命中方向异常`
              : "正式概率已被后端标记为审计态"
          }，系统暂停输出“数月/数周/当下”的距离判断。`
        : bucketDetail[assessment.time_to_risk_bucket],
    nearestLabel: auditOnly ? "下一阶段还差" : "最接近的动作线",
    nearestValue:
      auditOnly
        ? "证据共振"
        : nearest
          ? nearest.gap === 0
            ? "已触线"
            : `占线 ${formatProbabilityPercentExact(nearest.share)}`
          : "未配置",
    nearestDetail:
      auditOnly
        ? `当前不计算阈值占比、放大倍数或仓位时距。阻断项：${mvpBlockers} 下一步：${mvpNextActions}`
        : nearest
          ? `${nearest.label} 最接近；离动作线差 ${formatPercentagePointGap(
              nearest.gap
            )}，当前读数约为动作线的 ${formatProbabilityPercentExact(
              nearest.share
            )}。这只是阈值相对位置，不代表剩余天数，动作升级还要看事件确认、数据新鲜度和历史类比。`
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
  const allFarBelowEntry =
    thresholdShares.length === 3 && thresholdShares.every((share) => share < 0.03);
  const twentyDayIsCold = p20d > 0 && p20d < p5d * 0.25 && p20d < p60d * 0.25;
  const auditOnly = mvpProbabilityInputIsAuditOnly(assessment);
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
      thresholdShares.length === 3
        ? "阈值占比在异常状态下已从主结论隐藏，避免把模型偏冷误读成可决策时距。"
        : ""
    }`;
  }

  if (auditOnly) {
    return "正式概率已被 MVP 降级为审计读数；页面不把低概率、小数或阈值占比解释成“离风险很远”，当前主结论看规则层、关键数据、事件确认和历史类比。";
  }

  if (twentyDayIsCold && allFarBelowEntry) {
    return `当前三条正式概率都远低于进入线，且 20日窗口 ${formatProbabilityPercentExact(
      p20d
    )} 明显低于 5日 ${formatProbabilityPercentExact(p5d)} 和 60日 ${formatProbabilityPercentExact(
      p60d
    )}。这不是“风险被证明为 0”，而是活跃正式模型当前没有捕捉到临近危机信号，同时 20d head 输出偏冷；决策上仍要结合关键指标、事件确认、历史类比和动作层。`;
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

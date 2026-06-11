import {
  compactTechnicalId,
  formatDate,
  formatPercent,
  formatPercentPrecise,
  formatProbabilityPercent,
  formatProbabilityPercentExact,
  postureLabel,
  timeBucketLabel
} from "../../format";
import type {
  AssessmentMethodResponse,
  AssessmentSnapshot,
  PostureGuidance
} from "../../types";
import type { DecisionSignalLayerRowModel } from "./builderTypes";
import {
  currentMvpRiskState,
  mvpProbabilityInputIsAuditOnly,
  mvpRiskStateDetail,
  mvpRiskStateDisplayLabel
} from "./mvpRiskState";
import { probabilityDiagnosticAnomalyHorizons } from "./probabilityDiagnostics";

const PROBABILITY_HORIZONS = [5, 20, 60] as const;
const PROBABILITY_EPSILON = 1e-9;

export function actionSourceSummary(assessment: AssessmentSnapshot) {
  if (!assessment.method.actionability_enabled) {
    return {
      label: "过渡动作映射",
      detail:
        "当前线上版本还没有独立动作模型，准备/对冲/防守仍由危机先验和评分层过渡映射而来，只适合辅助执行节奏，不应当成正式校准后的独立动作概率。"
    };
  }

  const actionModel =
    assessment.method.actionability_model_version
      ? compactTechnicalId(assessment.method.actionability_model_version).value
      : "动作模型";
  const fusionPolicy =
    assessment.method.fusion_policy_version
      ? compactTechnicalId(assessment.method.fusion_policy_version).value
      : "融合层";

  return {
    label: "双层动作模型",
    detail: `当前已启用独立动作模型和融合层：${actionModel} / ${fusionPolicy}`
  };
}

function probabilityDisplayNote(assessment: AssessmentSnapshot): string | null {
  const anomalyHorizons = probabilityDiagnosticAnomalyHorizons(assessment);
  const auditOnly = mvpProbabilityInputIsAuditOnly(assessment);
  const runtimeReferenceNote = probabilityRuntimeReferenceNote(assessment);
  if (anomalyHorizons.length > 0) {
    return `当前 ${anomalyHorizons.join(
      " / "
    )} 概率命中 USDJPY 高位 tail 压低读数的语义异常；页面当前显示 ${probabilitySnapshotValue(
      assessment.probabilities
    )} 只作为运行口径参考值。${runtimeReferenceNote ? `${runtimeReferenceNote} ` : ""}它不能解释成“风险很远”，也不能单独用于离场/对冲时距结论。`;
  }
  if (auditOnly) {
    return `当前正式先验 ${probabilitySnapshotValue(
      assessment.probabilities
    )} 已被后端 MVP 状态降为参考输入；${runtimeReferenceNote ? `${runtimeReferenceNote} ` : ""}它不单独回答“离风险还有多远”，主结论看 ${mvpRiskStateDisplayLabel(
      currentMvpRiskState(assessment).label
    )}。`;
  }

  const peakProbability = Math.max(
    assessment.probabilities.p_5d,
    assessment.probabilities.p_20d,
    assessment.probabilities.p_60d
  );
  if (peakProbability >= 0.01) {
    return null;
  }
  const staleDays =
    assessment.runtime.latest_key_indicator_lag_business_days ??
    assessment.runtime.latest_observation_lag_business_days ??
    assessment.runtime.latest_key_indicator_lag_days ??
    assessment.runtime.latest_observation_lag_days;
  if (peakProbability === 0) {
    return staleDays !== null && staleDays >= 7
      ? `当前正式先验低于展示精度，且关键观测按工作日口径已滞后约 ${staleDays} 天；这代表“暂未看到足够证据支持主动防守”，不代表市场风险被证明为零。`
      : "当前正式先验低于展示精度；这代表“风险很低”，不代表市场风险被证明为零。";
  }
  return staleDays !== null && staleDays >= 7
    ? `当前正式先验仍低于 1%，且关键观测按工作日口径已滞后约 ${staleDays} 天；短期判断应保守解释。`
    : "当前正式先验仍低于 1%，属于低位区间，而不是零风险断言。";
}

export function probabilitySnapshotValue(
  probabilities: AssessmentSnapshot["probabilities"]
): string {
  return [
    formatProbabilityPercentExact(probabilities.p_5d),
    formatProbabilityPercentExact(probabilities.p_20d),
    formatProbabilityPercentExact(probabilities.p_60d)
  ].join(" / ");
}

function probabilityValueForHorizon(
  probabilities: AssessmentSnapshot["probabilities"],
  horizonDays: (typeof PROBABILITY_HORIZONS)[number]
): number {
  switch (horizonDays) {
    case 5:
      return probabilities.p_5d;
    case 20:
      return probabilities.p_20d;
    case 60:
      return probabilities.p_60d;
  }
}

function probabilityDiagnosticForHorizon(
  assessment: Pick<AssessmentSnapshot, "probabilities" | "probability_diagnostics">,
  horizonDays: (typeof PROBABILITY_HORIZONS)[number]
) {
  return assessment.probability_diagnostics.horizon_overlays.find(
    (diagnostic) => diagnostic.horizon_days === horizonDays
  );
}

export function hasRuntimeProbabilityOverride(
  assessment: Pick<AssessmentSnapshot, "probability_diagnostics">
): boolean {
  return assessment.probability_diagnostics.horizon_overlays.some((diagnostic) => {
    const runtimeFinal = diagnostic.runtime_final_probability;
    return (
      runtimeFinal !== undefined &&
      Math.abs(runtimeFinal - diagnostic.final_probability) > PROBABILITY_EPSILON
    );
  });
}

export function probabilityModelFinalSnapshotValue(
  assessment: Pick<AssessmentSnapshot, "probabilities" | "probability_diagnostics">
): string {
  return PROBABILITY_HORIZONS.map((horizonDays) => {
    const diagnostic = probabilityDiagnosticForHorizon(assessment, horizonDays);
    const modelFinal =
      diagnostic?.final_probability ??
      probabilityValueForHorizon(assessment.probabilities, horizonDays);
    return formatProbabilityPercentExact(modelFinal);
  }).join(" / ");
}

export function probabilityRuntimeReferenceNote(
  assessment: Pick<AssessmentSnapshot, "probabilities" | "probability_diagnostics">
): string | null {
  if (!hasRuntimeProbabilityOverride(assessment)) {
    return null;
  }
  return `模型原始输出 ${probabilityModelFinalSnapshotValue(
    assessment
  )}；页面当前显示的运行口径参考值为 ${probabilitySnapshotValue(assessment.probabilities)}。`;
}

export function probabilitySnapshotDetail(assessment: AssessmentSnapshot): string {
  const anomalyHorizons = probabilityDiagnosticAnomalyHorizons(assessment);
  const auditOnly = mvpProbabilityInputIsAuditOnly(assessment);
  const runtimeReferenceNote = probabilityRuntimeReferenceNote(assessment);
  if (anomalyHorizons.length > 0) {
    return `当前线上 ${formatDate(
      assessment.as_of_date
    )} · 5d / 20d / 60d 页面显示的是运行口径参考值 ${probabilitySnapshotValue(
      assessment.probabilities
    )}。${runtimeReferenceNote ? `${runtimeReferenceNote} ` : ""}${anomalyHorizons.join(
      " / "
    )} 命中模型方向异常，这些数值不作为风险时距或离场/对冲主结论。${mvpRiskStateDisplayLabel(
      currentMvpRiskState(assessment).label
    )}是当前主显示口径。`;
  }
  if (auditOnly) {
    return `当前线上 ${formatDate(
      assessment.as_of_date
    )} · 5d / 20d / 60d 当前按参考口径展示为 ${probabilitySnapshotValue(
      assessment.probabilities
    )}；${runtimeReferenceNote ? `${runtimeReferenceNote} ` : ""}后端当前将正式概率作为参考输入，主显示口径为 ${mvpRiskStateDisplayLabel(
      currentMvpRiskState(assessment).label
    )}。`;
  }

  const allZero =
    assessment.probabilities.p_5d === 0 &&
    assessment.probabilities.p_20d === 0 &&
    assessment.probabilities.p_60d === 0;
  const currentScope = `当前线上 ${formatDate(assessment.as_of_date)} · 5d / 20d / 60d`;
  if (allZero) {
    return `${currentScope}；三个期限均为精确 0，需要先检查正式概率包、关键观测日期和 release 状态。`;
  }
  return `${currentScope}；历史/候选旧快照可能保留 0 值，不代表当前线上结论。`;
}

export function actionEvidenceScore(assessment: AssessmentSnapshot): number {
  return assessment.action_evidence?.score ?? assessment.conviction_score;
}

export function actionEvidenceStatus(score: number): string {
  if (score >= 0.82) {
    return "强升级证据";
  }
  if (score >= 0.68) {
    return "可升级证据";
  }
  if (score >= 0.42) {
    return "接近观察线";
  }
  if (score >= 0.18) {
    return "初步观察证据";
  }
  return "仅数据底座";
}

function actionEvidenceBreakdownCopy(assessment: AssessmentSnapshot): string {
  const evidence = assessment.action_evidence;
  if (!evidence) {
    return `动作升级证据分 ${formatPercent(
      actionEvidenceScore(assessment)
    )}，当前缺少后端拆解，只能作为过渡动作证据。`;
  }

  const breadthCopy =
    evidence.breadth_component <= 0
      ? "风险广度尚未贡献"
      : `风险广度贡献 ${formatPercent(evidence.breadth_component)}`;
  const riskPressureComponent = evidence.risk_pressure_component ?? 0;
  const riskPressureCopy =
    riskPressureComponent <= 0
      ? "整体/结构/触发压力尚未贡献"
      : `整体/结构/触发压力贡献 ${formatPercent(riskPressureComponent)}`;
  const agreementCopy = evidence.structural_trigger_agreement
    ? `结构/触发共振贡献 ${formatPercent(evidence.agreement_component)}`
    : "结构/触发未共振，未给共振加分";

  return `动作升级证据分 ${formatPercent(evidence.score)} = 数据可信底座 ${formatPercent(evidence.data_quality_component)} + ${breadthCopy} + ${riskPressureCopy} + ${agreementCopy}。`;
}

export function actionEvidenceHint(assessment: AssessmentSnapshot): string {
  const evidence = assessment.action_evidence;
  if (!evidence) {
    return `${actionEvidenceBreakdownCopy(assessment)} 这不是模型结论置信概率，而是当前证据是否足以升级仓位动作。`;
  }

  return [
    actionEvidenceBreakdownCopy(assessment),
    `当前状态为 ${actionEvidenceStatus(evidence.score)}。`,
    "这不是模型结论置信概率，也不是危机发生概率；危机概率看 5/20/60 天三项。",
    "如果风险广度没有打开、整体/结构/触发压力没有抬升，它会停在低位；含义是“数据可用，但还不足以升级仓位动作”。"
  ].join(" ");
}

export function formatActionProbability(value: number, actionabilityEnabled: boolean): string {
  if (value === 0) {
    return actionabilityEnabled ? "0%" : "未触发";
  }
  return formatProbabilityPercent(value);
}

function formatActionDetailValue(
  label: string,
  value: number,
  actionabilityEnabled: boolean
): string {
  if (value === 0 && !actionabilityEnabled) {
    return `${label} 未触发`;
  }
  return `${label} ${formatPercentPrecise(value)}`;
}

export function buildSignalLayerRows(
  assessment: AssessmentSnapshot,
  method: AssessmentMethodResponse,
  posture: PostureGuidance
): DecisionSignalLayerRowModel[] {
  const actionabilitySource = actionSourceSummary(assessment).detail;
  const actionEvidence = assessment.action_evidence;
  const actionEvidenceDetail = actionEvidence
    ? `${actionEvidenceBreakdownCopy(assessment)} 它不是模型结论置信概率；结论可靠性请看关键指标覆盖、模型服务状态、源健康状态和关键数据日期。`
    : `${actionEvidenceBreakdownCopy(assessment)} 它不是模型结论置信概率；结论可靠性请看关键指标覆盖、模型服务状态、源健康状态和关键数据日期。`;
  const priorDetail = probabilityDisplayNote(assessment);
  const priorAnomalyHorizons = probabilityDiagnosticAnomalyHorizons(assessment);
  const priorAuditOnly = mvpProbabilityInputIsAuditOnly(assessment);
  const actionValuesAreAuxiliary =
    priorAuditOnly || !assessment.method.actionability_enabled;
  const priorThresholdSummary = `当前进入线：准备 ${formatPercent(method.runtime_thresholds.prepare_p60d)} / 对冲 ${formatPercent(method.runtime_thresholds.hedge_p20d)} / 防守 ${formatPercent(method.runtime_thresholds.defend_p5d)}`;
  const state = currentMvpRiskState(assessment);

  return [
    {
      id: "prior",
      title: priorAuditOnly ? "危机先验（参考）" : "危机先验",
      description:
        priorAuditOnly
          ? "正式概率当前只保留为参考输入，主结论优先看规则层和关键数据是否共振。"
          : "先看未来 5d / 20d / 60d 进入风险窗口的概率，回答“离风险还有多远”。",
      value: priorAuditOnly
        ? "参考输入"
        : `${formatProbabilityPercentExact(
            assessment.probabilities.p_5d
          )} / ${formatProbabilityPercentExact(
            assessment.probabilities.p_20d
          )} / ${formatProbabilityPercentExact(assessment.probabilities.p_60d)}`,
      detail:
        priorAuditOnly
          ? `当前页面参考值 ${probabilitySnapshotValue(assessment.probabilities)}。${
              priorDetail ?? "正式概率当前只作为参考值。"
            }`
          : priorDetail
            ? `${priorThresholdSummary} · ${priorDetail}`
            : priorThresholdSummary
    },
    {
      id: "actionability",
      title: actionValuesAreAuxiliary ? "动作信号（辅助）" : "动作概率",
      description: actionValuesAreAuxiliary
        ? "准备 / 对冲 / 防守当前只保留为辅助执行信号，不能压过 MVP 规则层主结论。"
        : "再看准备 / 对冲 / 防守，回答“现在该不该开始准备、加保护、保流动性”。",
      value: actionValuesAreAuxiliary
        ? "辅助信号"
        : `${formatActionProbability(
            assessment.actionability.prepare,
            assessment.method.actionability_enabled
          )} / ${formatActionProbability(
            assessment.actionability.hedge,
            assessment.method.actionability_enabled
          )} / ${formatActionProbability(
            assessment.actionability.defend,
            assessment.method.actionability_enabled
          )}`,
      detail: `${actionabilitySource} 当前显示：${formatActionDetailValue(
        "准备",
        assessment.actionability.prepare,
        assessment.method.actionability_enabled
      )} / ${formatActionDetailValue(
        "对冲",
        assessment.actionability.hedge,
        assessment.method.actionability_enabled
      )} / ${formatActionDetailValue(
        "防守",
        assessment.actionability.defend,
        assessment.method.actionability_enabled
      )}。`
    },
    {
      id: "action-evidence",
      title: "动作升级证据",
      description: "看当前证据是否足以把仓位动作从观察推向准备、对冲或防守；它不是模型结论置信概率。",
      value: actionEvidenceStatus(actionEvidenceScore(assessment)),
      detail: actionEvidenceDetail
    },
    {
      id: "posture",
      title: priorAuditOnly ? "MVP 主结论" : "最终执行节奏",
      description:
        priorAuditOnly
          ? "正式概率作为参考输入时，先用规则层、数据质量、事件确认和日元套息状态给出保守 MVP 结论。"
          : "最后再叠加数据可信度、事件确认、日元套息放大器和用户偏好，压成一档执行节奏。",
      value:
        priorAuditOnly
          ? mvpRiskStateDisplayLabel(state.label)
          : `${postureLabel(assessment.posture)} / ${timeBucketLabel(assessment.time_to_risk_bucket)}`,
      detail:
        priorAuditOnly
          ? `${
              priorAnomalyHorizons.length > 0
                ? `${priorAnomalyHorizons.join(" / ")} 正式概率读数命中模型方向异常`
                : "正式概率已被后端降为参考输入"
            }；当前执行节奏按 MVP 风险状态展示，不能把正式低概率直接理解成风险已经远离。${mvpRiskStateDetail(assessment)}`
          : posture.summary
    }
  ];
}

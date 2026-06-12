import { formatPercent } from "../../format";
import type { AssessmentSnapshot } from "../../types";
import type { MetricItem } from "../shared/panelHelpers";
import {
  decisionFreshnessReliabilityHint,
  decisionFreshnessReliabilityLabel,
  decisionModelReliabilityHint,
  decisionModelReliabilityLabel,
  decisionReliabilityHint,
  decisionReliabilityLabel
} from "./decisionReliability";
import {
  currentMvpRiskState,
  mvpRiskStateDetail,
  mvpRiskStateDisplayLabel
} from "./mvpRiskState";

function actionEvidenceScore(assessment: AssessmentSnapshot): number {
  return assessment.action_evidence?.score ?? assessment.conviction_score;
}

function actionEvidenceStatus(score: number): string {
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

  return `动作升级证据分 ${formatPercent(
    evidence.score
  )} = 数据可信底座 ${formatPercent(
    evidence.data_quality_component
  )} + ${breadthCopy} + ${riskPressureCopy} + ${agreementCopy}。`;
}

function actionEvidenceHint(assessment: AssessmentSnapshot): string {
  const evidence = assessment.action_evidence;
  if (!evidence) {
    return `${actionEvidenceBreakdownCopy(
      assessment
    )} 这不是模型结论置信概率，而是当前证据是否足以升级仓位动作。`;
  }

  return [
    actionEvidenceBreakdownCopy(assessment),
    `当前状态为 ${actionEvidenceStatus(evidence.score)}。`,
    "这不是模型结论置信概率，也不是危机发生概率；危机概率看 5/20/60 天三项。",
    "如果风险广度没有打开、整体/结构/触发压力没有抬升，它会停在低位；含义是“数据可用，但还不足以升级仓位动作”。"
  ].join(" ");
}

export function buildHeroMetrics(assessment: AssessmentSnapshot): MetricItem[] {
  const evidenceScore = actionEvidenceScore(assessment);
  const state = currentMvpRiskState(assessment);
  return [
    {
      label: "MVP 风险状态",
      value: mvpRiskStateDisplayLabel(state.label),
      hint: mvpRiskStateDetail(assessment),
      valueClassName: "metric-value-token"
    },
    {
      label: "结论可信度",
      value: decisionReliabilityLabel(assessment),
      hint: decisionReliabilityHint(assessment),
      valueClassName: "metric-value-token"
    },
    {
      label: "模型可信度",
      value: decisionModelReliabilityLabel(assessment),
      hint: decisionModelReliabilityHint(assessment),
      valueClassName: "metric-value-token"
    },
    {
      label: "数据新鲜度",
      value: decisionFreshnessReliabilityLabel(assessment),
      hint: decisionFreshnessReliabilityHint(assessment),
      valueClassName: "metric-value-token"
    },
    {
      label: "动作升级证据",
      value: actionEvidenceStatus(evidenceScore),
      hint: actionEvidenceHint(assessment),
      valueClassName: "metric-value-token"
    }
  ];
}

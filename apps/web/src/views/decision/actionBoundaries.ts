import type { AssessmentSnapshot, DecisionPosture, MvpRiskStateCode } from "../../types";
import { currentMvpRiskState, mvpProbabilityInputIsAuditOnly } from "./mvpRiskState";

export interface ActionBoundaryRow {
  id: DecisionPosture;
  label: string;
  summary: string;
  riskAssetRange: string;
  cashRange: string;
  hedgeRange: string;
  optionRange: string;
  leverageCap: string;
  executionWindow: string;
  currentBudget?: string;
}

const BASE_ACTION_BOUNDARIES: Array<Omit<ActionBoundaryRow, "currentBudget">> = [
  {
    id: "normal",
    label: "观察",
    summary: "风险处于常态，不主动大幅防守，重点是确认数据和监控触发层。",
    riskAssetRange: "65% - 85%",
    cashRange: "5% - 15%",
    hedgeRange: "0% - 5%",
    optionRange: "0% - 3%",
    leverageCap: "≤100%",
    executionWindow: "常规监控"
  },
  {
    id: "prepare",
    label: "准备",
    summary: "脆弱性升高，先降组合脆弱性、补现金，并准备保护工具。",
    riskAssetRange: "50% - 70%",
    cashRange: "15% - 25%",
    hedgeRange: "5% - 15%",
    optionRange: "0% - 5%",
    leverageCap: "≤75%",
    executionWindow: "3-10 个交易日"
  },
  {
    id: "hedge",
    label: "对冲",
    summary: "风险进入几周尺度，保护性对冲和净敞口收缩要开始落地。",
    riskAssetRange: "30% - 50%",
    cashRange: "25% - 40%",
    hedgeRange: "20% - 35%",
    optionRange: "5% - 12%",
    leverageCap: "≤50%",
    executionWindow: "1-5 个交易日"
  },
  {
    id: "defend",
    label: "防守",
    summary: "短期风险窗口已打开，资本保全、流动性和去杠杆优先。",
    riskAssetRange: "10% - 25%",
    cashRange: "45% - 65%",
    hedgeRange: "35% - 60%",
    optionRange: "10% - 20%",
    leverageCap: "0% - 20%",
    executionWindow: "当日到 2 个交易日"
  }
];

function postureFromMvpState(code: MvpRiskStateCode): DecisionPosture {
  if (code === "observe") {
    return "normal";
  }
  return code;
}

export function currentActionBoundaryPosture(assessment: AssessmentSnapshot): DecisionPosture {
  const mvpRiskState = currentMvpRiskState(assessment);
  if (mvpProbabilityInputIsAuditOnly(assessment)) {
    return postureFromMvpState(mvpRiskState.code);
  }
  return assessment.posture;
}

function currentBudgetSummary(assessment: AssessmentSnapshot): string {
  const guidance = assessment.position_guidance;
  const hedge =
    guidance.hedge_ratio_pct === 0 ? "暂不对冲" : formatBudgetPercent(guidance.hedge_ratio_pct);

  return [
    `风险资产 ${formatBudgetPercent(guidance.target_equity_exposure_pct)}`,
    `现金 ${formatBudgetPercent(guidance.target_cash_pct)}`,
    `对冲 ${hedge}`,
    `期权 ${formatBudgetPercent(guidance.option_overlay_pct)}`,
    `杠杆 ${formatBudgetPercent(guidance.leverage_cap_pct)}`
  ].join(" / ");
}

function formatBudgetPercent(value: number): string {
  if (!Number.isFinite(value)) {
    return "—";
  }
  const rounded = Math.round(value);
  if (Math.abs(value - rounded) < 0.05) {
    return `${rounded}%`;
  }
  return `${value.toFixed(1)}%`;
}

export function buildActionBoundaryRows(assessment: AssessmentSnapshot): ActionBoundaryRow[] {
  const current = currentActionBoundaryPosture(assessment);
  const currentBudget = currentBudgetSummary(assessment);

  return BASE_ACTION_BOUNDARIES.map((row) => ({
    ...row,
    currentBudget: row.id === current ? currentBudget : undefined
  }));
}

export function actionBoundarySourceCopy(assessment: AssessmentSnapshot): string {
  if (mvpProbabilityInputIsAuditOnly(assessment)) {
    return "当前正式概率待审计，四档高亮按 MVP 规则层决定；正式 5d/20d/60d 只保留为模型审计读数。";
  }
  return "当前四档高亮按正式 posture 决定，并已叠加用户风险偏好生成下方预算条。";
}

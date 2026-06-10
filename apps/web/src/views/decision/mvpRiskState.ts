import type { AssessmentSnapshot, MvpRiskState } from "../../types";
import { probabilityDiagnosticAnomalyHorizons } from "./probabilityDiagnostics";

function stripTerminalPunctuation(value: string): string {
  return value.trim().replace(/[。；;]+$/u, "");
}

function sentence(value: string): string {
  const stripped = stripTerminalPunctuation(value);
  return stripped ? `${stripped}。` : "";
}

function labeledSentence(label: string, items: string[]): string {
  const values = items.map(stripTerminalPunctuation).filter(Boolean);
  return values.length > 0 ? `${label}：${values.join("；")}。` : "";
}

export function currentMvpRiskState(assessment: AssessmentSnapshot): MvpRiskState {
  return assessment.mvp_risk_state ?? {
    code: "observe",
    label: "观察为主（MVP 未返回）",
    probability_input_status: probabilityDiagnosticAnomalyHorizons(assessment).length > 0
      ? "audit_only"
      : "usable",
    summary: "当前 API 未返回 MVP 风险状态，页面仅保留兼容显示；主结论仍应先复核数据和模型状态。",
    primary_evidence: [],
    blockers: ["API 未返回 mvp_risk_state。"],
    next_actions: ["先刷新 API 并确认后端版本已经包含 MVP 风险状态。"]
  };
}

export function mvpRiskStateDetail(assessment: AssessmentSnapshot): string {
  const state = currentMvpRiskState(assessment);
  return [
    sentence(state.summary),
    labeledSentence("主要证据", state.primary_evidence),
    labeledSentence("限制", state.blockers),
    labeledSentence("下一步", state.next_actions)
  ]
    .filter(Boolean)
    .join(" ");
}

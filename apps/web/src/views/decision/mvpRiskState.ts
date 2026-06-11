import type { AssessmentSnapshot, MvpRiskState } from "../../types";
import { probabilityDiagnosticAnomalyHorizons } from "./probabilityDiagnostics";

function stripTerminalPunctuation(value: string): string {
  return value.trim().replace(/[。；;]+$/u, "");
}

export function mvpRiskStateDisplayCopy(value: string): string {
  return value
    .replaceAll("概率待审计", "概率参考")
    .replaceAll("审计读数", "参考值")
    .replaceAll("审计输入", "参考输入")
    .replaceAll("审计态", "参考态")
    .replaceAll("待审计", "参考");
}

function sentence(value: string): string {
  const stripped = stripTerminalPunctuation(mvpRiskStateDisplayCopy(value));
  return stripped ? `${stripped}。` : "";
}

function labeledSentence(label: string, items: string[]): string {
  const values = items
    .map((item) => stripTerminalPunctuation(mvpRiskStateDisplayCopy(item)))
    .filter(Boolean);
  return values.length > 0 ? `${label}：${values.join("；")}。` : "";
}

export function currentMvpRiskState(assessment: AssessmentSnapshot): MvpRiskState {
  return assessment.mvp_risk_state ?? {
    code: "observe",
    label: "观察为主（MVP 未返回）",
    probability_input_status: probabilityDiagnosticAnomalyHorizons(assessment).length > 0
      ? "reference_only"
      : "usable",
    summary: "当前 API 未返回 MVP 风险状态，页面仅保留兼容显示；主结论仍应先复核数据和模型状态。",
    primary_evidence: [],
    blockers: ["API 未返回 mvp_risk_state。"],
    next_actions: ["先刷新 API 并确认后端版本已经包含 MVP 风险状态。"]
  };
}

export function mvpRiskStateDisplayLabel(label: string): string {
  return label.replace(/（[^）]*(?:待审计|未返回)[^）]*）/gu, "").trim();
}

export function mvpProbabilityInputIsAuditOnly(assessment: AssessmentSnapshot): boolean {
  return (
    currentMvpRiskState(assessment).probability_input_status === "reference_only" ||
    probabilityDiagnosticAnomalyHorizons(assessment).length > 0
  );
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

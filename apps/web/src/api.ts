import type {
  AlertEvent,
  AssessmentHistoryPoint,
  AssessmentMethodResponse,
  AssessmentSnapshot,
  BacktestScenarioSummary,
  BacktestWindowPoint,
  DataSource,
  DimensionScore,
  IndicatorRisk,
  PostureGuidance,
  ResearchAuditResponse,
  RiskSnapshot
} from "./types";

const API_BASE = import.meta.env.VITE_API_BASE_URL ?? "";

const DEFAULT_POSITION_GUIDANCE_GOVERNANCE = {
  system_budget_only: true,
  auto_execution_allowed: false,
  manual_confirmation_required: true,
  policy_change_requires_release_review: true,
  policy_change_requires_go_no_go: true,
  required_operator_checks: []
} satisfies AssessmentSnapshot["position_guidance"]["governance"];

async function getJson<T>(path: string): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`, {
    cache: "no-store",
    headers: {
      "Cache-Control": "no-cache"
    }
  });
  if (!response.ok) {
    throw new Error(`Request failed: ${response.status} ${response.statusText}`);
  }
  return (await response.json()) as T;
}

function normalizeAssessmentSnapshot(
  assessment: AssessmentSnapshot
): AssessmentSnapshot {
  return {
    ...assessment,
    position_guidance: {
      ...assessment.position_guidance,
      governance: assessment.position_guidance.governance
        ? {
            ...DEFAULT_POSITION_GUIDANCE_GOVERNANCE,
            ...assessment.position_guidance.governance,
            required_operator_checks:
              assessment.position_guidance.governance.required_operator_checks ??
              DEFAULT_POSITION_GUIDANCE_GOVERNANCE.required_operator_checks
          }
        : DEFAULT_POSITION_GUIDANCE_GOVERNANCE
    }
  };
}

async function sendJson<T>(path: string, method: string): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`, {
    method,
    cache: "no-store",
    headers: {
      "Cache-Control": "no-cache"
    }
  });
  if (!response.ok) {
    throw new Error(`Request failed: ${response.status} ${response.statusText}`);
  }
  return (await response.json()) as T;
}

export const api = {
  overview: () => getJson<RiskSnapshot>("/api/overview"),
  dimensions: () => getJson<DimensionScore[]>("/api/dimensions"),
  indicators: () => getJson<IndicatorRisk[]>("/api/indicators"),
  eventsRecent: () => getJson<AlertEvent[]>("/api/events/recent"),
  sources: () => getJson<DataSource[]>("/api/sources"),
  backtests: () => getJson<BacktestScenarioSummary[]>("/api/backtests"),
  backtestTimeline: () => getJson<BacktestWindowPoint[]>("/api/backtests/timeline"),
  assessmentCurrent: async () =>
    normalizeAssessmentSnapshot(
      await getJson<AssessmentSnapshot>("/api/assessment/current")
    ),
  assessmentHistory: () => getJson<AssessmentHistoryPoint[]>("/api/assessment/history"),
  assessmentPosture: () => getJson<PostureGuidance>("/api/assessment/posture"),
  assessmentMethod: () => getJson<AssessmentMethodResponse>("/api/assessment/method"),
  researchAudit: () => getJson<ResearchAuditResponse>("/api/research/audit"),
  systemReload: () =>
    sendJson<{ status: string; data_mode: string; as_of_date: string; generated_at: string }>(
      "/api/system/reload",
      "POST"
    )
};

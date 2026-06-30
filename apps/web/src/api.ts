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
  RiskAlertThresholds,
  RiskSnapshot,
  ServiceHealthResponse
} from "./types";

const API_BASE = import.meta.env.VITE_API_BASE_URL ?? "";
const DEFAULT_REQUEST_TIMEOUT_MS = 15_000;
const MUTATION_REQUEST_TIMEOUT_MS = 30_000;

const DEFAULT_POSITION_GUIDANCE_GOVERNANCE = {
  system_budget_only: true,
  auto_execution_allowed: false,
  manual_confirmation_required: true,
  policy_change_requires_release_review: true,
  policy_change_requires_go_no_go: true,
  required_operator_checks: []
} satisfies AssessmentSnapshot["position_guidance"]["governance"];

async function requestJson<T>(
  path: string,
  options: {
    method?: string;
    timeoutMs?: number;
  } = {}
): Promise<T> {
  const timeoutMs = options.timeoutMs ?? DEFAULT_REQUEST_TIMEOUT_MS;
  const controller = new AbortController();
  let didTimeout = false;
  const timeoutId = window.setTimeout(() => {
    didTimeout = true;
    controller.abort();
  }, timeoutMs);

  try {
    const response = await fetch(`${API_BASE}${path}`, {
      method: options.method,
      cache: "no-store",
      headers: {
        "Cache-Control": "no-cache"
      },
      signal: controller.signal
    });
    if (!response.ok) {
      throw new Error(`Request failed: ${response.status} ${response.statusText}`);
    }
    return (await response.json()) as T;
  } catch (error) {
    if (didTimeout) {
      throw new Error(
        `请求 ${path} 超过 ${Math.round(
          timeoutMs / 1000
        )} 秒未返回；本地 API 可能已卡住。请先执行 just status，必要时执行 just stop 后再执行 just dev-sqlite。`
      );
    }
    throw error;
  } finally {
    window.clearTimeout(timeoutId);
  }
}

async function getJson<T>(path: string): Promise<T> {
  return requestJson<T>(path);
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
  return requestJson<T>(path, {
    method,
    timeoutMs: MUTATION_REQUEST_TIMEOUT_MS
  });
}

export const api = {
  systemHealth: () => getJson<ServiceHealthResponse>("/health"),
  riskAlertThresholds: () => getJson<RiskAlertThresholds>("/api/system/risk-thresholds"),
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

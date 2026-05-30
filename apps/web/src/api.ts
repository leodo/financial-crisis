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
  RiskSnapshot
} from "./types";

const API_BASE = import.meta.env.VITE_API_BASE_URL ?? "";

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
  assessmentCurrent: () => getJson<AssessmentSnapshot>("/api/assessment/current"),
  assessmentHistory: () => getJson<AssessmentHistoryPoint[]>("/api/assessment/history"),
  assessmentPosture: () => getJson<PostureGuidance>("/api/assessment/posture"),
  assessmentMethod: () => getJson<AssessmentMethodResponse>("/api/assessment/method"),
  systemReload: () =>
    sendJson<{ status: string; data_mode: string; as_of_date: string; generated_at: string }>(
      "/api/system/reload",
      "POST"
    )
};

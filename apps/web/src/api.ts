import type {
  AlertEvent,
  BacktestScenarioSummary,
  DataSource,
  DimensionScore,
  IndicatorRisk,
  RiskSnapshot
} from "./types";

const API_BASE = import.meta.env.VITE_API_BASE_URL ?? "";

async function getJson<T>(path: string): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`);
  if (!response.ok) {
    throw new Error(`Request failed: ${response.status} ${response.statusText}`);
  }
  return (await response.json()) as T;
}

export const api = {
  overview: () => getJson<RiskSnapshot>("/api/overview"),
  dimensions: () => getJson<DimensionScore[]>("/api/dimensions"),
  indicators: () => getJson<IndicatorRisk[]>("/api/indicators"),
  alerts: () => getJson<AlertEvent[]>("/api/alerts"),
  sources: () => getJson<DataSource[]>("/api/sources"),
  backtests: () => getJson<BacktestScenarioSummary[]>("/api/backtests")
};


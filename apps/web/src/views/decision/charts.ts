import { formatDate, wrapTimelineLabel } from "../../format";
import type {
  AssessmentHistoryPoint,
  AssessmentSnapshot,
  BacktestScenarioSummary
} from "../../types";

export interface LineChartSeriesModel {
  label: string;
  color: string;
  values: number[];
  fillColor?: string;
}

export interface LineChartModel {
  categories: string[];
  maxValue: number;
  series: LineChartSeriesModel[];
  valueType: "percent" | "score";
}

export interface HorizontalBarRowModel {
  label: string;
  value: number;
  color: string;
}

export interface HorizontalBarChartModel {
  maxValue: number;
  rows: HorizontalBarRowModel[];
}

export interface GroupedBarSeriesModel {
  label: string;
  color: string;
  values: number[];
}

export interface GroupedBarChartModel {
  categories: string[];
  maxValue: number;
  series: GroupedBarSeriesModel[];
}

type ProbabilityTrendMode = "calibrated" | "raw";

export function buildProbabilityTrendModel(history: AssessmentHistoryPoint[]) {
  const mode = selectProbabilityTrendMode(history);
  const baseNote =
    mode === "raw"
      ? "当前发布版正式概率被校准下限压得很平，这里改为展示原始概率轨迹，用来看风险是在升温还是降温；上方当前评估卡片仍以正式概率为准。"
      : "这里展示的是发布版正式概率轨迹；若三条线长期贴平，通常表示当前仍在低风险区，或正式概率暂时受校准下限约束。";
  const sourceNote = buildProbabilityTrendSourceNote(history);

  return {
    chart: buildProbabilityTrendChart(history, mode),
    note: sourceNote ? `${baseNote} ${sourceNote}` : baseNote
  };
}

function selectProbabilityTrendMode(history: AssessmentHistoryPoint[]): ProbabilityTrendMode {
  const hasRawProbability = history.every(
    (point) =>
      point.raw_p_5d !== undefined &&
      point.raw_p_20d !== undefined &&
      point.raw_p_60d !== undefined
  );
  if (!hasRawProbability || history.length < 2) {
    return "calibrated";
  }

  const calibratedSpread = Math.max(
    probabilitySpread(history.map((point) => point.p_5d)),
    probabilitySpread(history.map((point) => point.p_20d)),
    probabilitySpread(history.map((point) => point.p_60d))
  );
  const rawSpread = Math.max(
    probabilitySpread(history.map((point) => point.raw_p_5d ?? point.p_5d)),
    probabilitySpread(history.map((point) => point.raw_p_20d ?? point.p_20d)),
    probabilitySpread(history.map((point) => point.raw_p_60d ?? point.p_60d))
  );

  return calibratedSpread <= 0.001 && rawSpread >= 0.004 ? "raw" : "calibrated";
}

function probabilitySpread(values: number[]) {
  if (values.length === 0) {
    return 0;
  }

  return Math.max(...values) - Math.min(...values);
}

function buildProbabilityTrendSourceNote(history: AssessmentHistoryPoint[]) {
  if (history.length === 0) {
    return "";
  }

  const bridgeCount = history.filter(
    (point) => point.history_source === "transitional_snapshot_bridge"
  ).length;
  if (bridgeCount > 0) {
    return `这段轨迹里有 ${bridgeCount}/${history.length} 个点仍来自过渡 snapshot bridge，只适合辅助观察，不应直接当成正式 Go/No-Go 历史证据。`;
  }

  const rawObservationCount = history.filter(
    (point) => point.history_source === "raw_observation_rebuild"
  ).length;
  if (rawObservationCount > 0) {
    return `这段轨迹已经避开旧 snapshot bridge，但其中 ${rawObservationCount}/${history.length} 个点还没有对上已落库的 PIT feature snapshot，当前仍属于 raw rebuild 过渡口径。`;
  }

  return "";
}

function probabilityValue(
  point: AssessmentHistoryPoint,
  horizon: 5 | 20 | 60,
  mode: ProbabilityTrendMode
) {
  if (mode === "raw") {
    if (horizon === 5) {
      return point.raw_p_5d ?? point.p_5d;
    }
    if (horizon === 20) {
      return point.raw_p_20d ?? point.p_20d;
    }
    return point.raw_p_60d ?? point.p_60d;
  }

  if (horizon === 5) {
    return point.p_5d;
  }
  if (horizon === 20) {
    return point.p_20d;
  }
  return point.p_60d;
}

function buildProbabilityTrendChart(
  history: AssessmentHistoryPoint[],
  mode: ProbabilityTrendMode
): LineChartModel {
  const probabilityValues = history.flatMap((point) => [
    probabilityValue(point, 5, mode),
    probabilityValue(point, 20, mode),
    probabilityValue(point, 60, mode)
  ]);
  const probabilityMax = probabilityValues.length > 0 ? Math.max(...probabilityValues) : 0;
  const yAxisMax = Math.min(
    1,
    Math.max(0.08, Math.ceil((probabilityMax * 1.35) / 0.02) * 0.02)
  );

  return {
    categories: history.map((point) => formatDate(point.as_of_date)),
    maxValue: yAxisMax,
    valueType: "percent",
    series: [
      {
        label: mode === "raw" ? "5日窗口（原始）" : "5日窗口",
        color: "#b45309",
        values: history.map((point) => probabilityValue(point, 5, mode))
      },
      {
        label: mode === "raw" ? "20日窗口（原始）" : "20日窗口",
        color: "#2563eb",
        values: history.map((point) => probabilityValue(point, 20, mode))
      },
      {
        label: mode === "raw" ? "60日窗口（原始）" : "60日窗口",
        color: "#115e59",
        fillColor: "rgba(17, 94, 89, 0.08)",
        values: history.map((point) => probabilityValue(point, 60, mode))
      }
    ]
  };
}

export function buildLayerScoreChart(assessment: AssessmentSnapshot): HorizontalBarChartModel {
  return {
    maxValue: 100,
    rows: [
      { label: "结构性", value: assessment.scores.structural_score, color: "#115e59" },
      { label: "触发性", value: assessment.scores.trigger_score, color: "#2563eb" },
      { label: "外部冲击", value: assessment.scores.external_shock_score, color: "#8b5cf6" },
      { label: "总风险强度", value: assessment.scores.overall_score, color: "#b45309" }
    ]
  };
}

export function buildAnalogChart(
  assessment: AssessmentSnapshot,
  backtests: BacktestScenarioSummary[]
): GroupedBarChartModel {
  const analogNames = assessment.historical_analogs.map((analog) => wrapTimelineLabel(analog.name));
  const peakScores = assessment.historical_analogs.map((analog) => analog.peak_score);
  const scenarioPeaks = assessment.historical_analogs.map((analog) => {
    const match = backtests.find((scenario) => scenario.name === analog.name);
    return match?.max_score ?? analog.peak_score;
  });

  return {
    categories: analogNames,
    maxValue: 100,
    series: [
      {
        label: "当前总风险强度",
        color: "#1d4ed8",
        values: analogNames.map(() => assessment.scores.overall_score)
      },
      {
        label: "历史峰值",
        color: "#b45309",
        values: scenarioPeaks.length > 0 ? scenarioPeaks : peakScores
      }
    ]
  };
}

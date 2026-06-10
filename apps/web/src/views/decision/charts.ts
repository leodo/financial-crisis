import {
  formatDate,
  formatPercentPrecise,
  formatProbabilityBasisPoints,
  formatProbabilityDecimal,
  formatProbabilityPercentExact,
  postureLabel,
  timeBucketLabel,
  wrapTimelineLabel
} from "../../format";
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
  pointDetails?: LineChartPointDetail[];
}

export interface LineChartPointDetail {
  valueLabel: string;
  detail?: string;
}

export interface LineChartModel {
  categories: string[];
  minValue?: number;
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

export interface ProbabilityTrendMetric {
  label: string;
  value: string;
  hint: string;
}

type ProbabilityTrendMode = "calibrated" | "raw";
const RECENT_PROBABILITY_WINDOW_POINTS = 90;

export function buildProbabilityTrendModel(history: AssessmentHistoryPoint[]) {
  const mode = selectProbabilityTrendMode(history);
  const chartHistory = selectProbabilityTrendWindow(history, mode);
  const baseNote =
    mode === "raw"
      ? "当前发布版正式概率被校准下限压得很平，这里改为展示原始概率轨迹，用来看风险是在升温还是降温；上方当前评估卡片仍以正式概率为准。"
      : "这里展示的是发布版正式概率轨迹；若三条线长期贴平，通常表示当前仍在低风险区，或正式概率暂时受校准下限约束。";
  const windowNote =
    chartHistory.length < history.length
      ? `当前图表已缩放到最近 ${chartHistory.length} 条评估点，避免历史高峰把当前低位压成贴地线；完整历史仍用于回测和发布审计。`
      : "";
  const sourceNote = buildProbabilityTrendSourceNote(history);
  const sanityNote = buildProbabilityTrendSanityNote(chartHistory, mode);
  const scaleNote = buildProbabilityTrendScaleNote(chartHistory, mode);

  return {
    chart: buildProbabilityTrendChart(chartHistory, mode),
    twentyDayZoomChart: buildTwentyDayZoomChart(chartHistory, mode),
    relativeChart: buildProbabilityRelativeTrendChart(chartHistory, mode),
    summaryMetrics: buildProbabilityTrendSummaryMetrics(chartHistory, mode),
    note: [baseNote, windowNote, scaleNote, sanityNote, sourceNote].filter(Boolean).join(" ")
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

function selectProbabilityTrendWindow(
  history: AssessmentHistoryPoint[],
  mode: ProbabilityTrendMode
) {
  if (history.length <= RECENT_PROBABILITY_WINDOW_POINTS) {
    return history;
  }

  const recentHistory = history.slice(-RECENT_PROBABILITY_WINDOW_POINTS);
  const fullMax = maxProbabilityValue(history, mode);
  const recentMax = maxProbabilityValue(recentHistory, mode);
  if (fullMax >= 0.05 && recentMax <= 0.02) {
    return recentHistory;
  }

  return history;
}

function maxProbabilityValue(
  history: AssessmentHistoryPoint[],
  mode: ProbabilityTrendMode
) {
  const values = history.flatMap((point) => [
    probabilityValue(point, 5, mode),
    probabilityValue(point, 20, mode),
    probabilityValue(point, 60, mode)
  ]);
  return values.length > 0 ? Math.max(...values) : 0;
}

export function buildProbabilityAxisMax(probabilityMax: number) {
  if (probabilityMax <= 0) {
    return 0.02;
  }
  if (probabilityMax < 0.001) {
    return Math.max(0.0004, Math.ceil((probabilityMax * 1.35) / 0.0001) * 0.0001);
  }
  if (probabilityMax < 0.01) {
    return Math.max(0.004, Math.ceil((probabilityMax * 1.35) / 0.001) * 0.001);
  }
  if (probabilityMax < 0.08) {
    return Math.max(0.02, Math.ceil((probabilityMax * 1.35) / 0.01) * 0.01);
  }
  return Math.min(1, Math.max(0.08, Math.ceil((probabilityMax * 1.35) / 0.02) * 0.02));
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
    (point) =>
      point.history_source === "raw_observation_rebuild" ||
      point.history_source === "raw_observation_replay"
  ).length;
  if (rawObservationCount > 0) {
    return `这段轨迹已经避开旧 snapshot bridge，但其中 ${rawObservationCount}/${history.length} 个点还没有对上已落库的 PIT feature snapshot，当前仍属于 raw observation 过渡口径。`;
  }

  return "";
}

function buildProbabilityTrendSanityNote(
  history: AssessmentHistoryPoint[],
  mode: ProbabilityTrendMode
) {
  const latest = history.at(-1);
  if (!latest) {
    return "";
  }

  const latest5d = probabilityValue(latest, 5, mode);
  const latest20d = probabilityValue(latest, 20, mode);
  const latest60d = probabilityValue(latest, 60, mode);
  const twentyDayIsCold =
    latest20d > 0 &&
    latest20d < latest5d * 0.25 &&
    latest20d < latest60d * 0.25;

  if (!twentyDayIsCold) {
    return "";
  }

  return "当前 20日窗口明显低于 5日和 60日，不是画图错误；它表示活跃正式模型的 20d head 在当前样本上输出偏冷，后续需要通过训练和 release review 修复，而不是在运行时硬抬概率。";
}

function buildProbabilityTrendScaleNote(
  history: AssessmentHistoryPoint[],
  mode: ProbabilityTrendMode
) {
  if (history.length < 2) {
    return "";
  }

  const seriesStats = [
    probabilitySeriesStats("5日窗口", history.map((point) => probabilityValue(point, 5, mode))),
    probabilitySeriesStats("20日窗口", history.map((point) => probabilityValue(point, 20, mode))),
    probabilitySeriesStats("60日窗口", history.map((point) => probabilityValue(point, 60, mode)))
  ];
  const yAxisMax = buildProbabilityAxisMax(
    Math.max(...seriesStats.map((series) => series.maxValue), 0)
  );
  const compressedSeries = seriesStats.find(
    (series) => series.label === "20日窗口" && series.maxValue > 0 && series.maxValue < yAxisMax * 0.12
  );

  if (!compressedSeries) {
    return "";
  }

  const axisShare = yAxisMax > 0 ? compressedSeries.maxValue / yAxisMax : 0;
  const rangeCopy = `${formatProbabilityPercentExact(
    compressedSeries.minValue
  )} 到 ${formatProbabilityPercentExact(compressedSeries.maxValue)}`;
  const spreadCopy = formatProbabilityPercentExact(compressedSeries.spread);

  if (isProbabilitySeriesEffectivelyFlat(compressedSeries)) {
    return `${compressedSeries.label}最近窗口几乎没有变化，范围是 ${rangeCopy}，波动只有 ${spreadCopy}；这时局部放大图也会接近直线，说明数据本身近似常数。`;
  }

  return `${compressedSeries.label}不是一条真正的 0 线，最近窗口范围是 ${rangeCopy}，波动 ${spreadCopy}；但它在主图线性坐标上只占纵轴 ${formatPercentPrecise(
    axisShare
  )}，所以视觉上会贴近底部。`;
}

function probabilitySeriesStats(label: string, values: number[]) {
  return {
    label,
    minValue: Math.min(...values),
    maxValue: Math.max(...values),
    latestValue: values.at(-1) ?? 0,
    spread: probabilitySpread(values)
  };
}

function isProbabilitySeriesEffectivelyFlat(series: ReturnType<typeof probabilitySeriesStats>) {
  if (series.maxValue <= 0) {
    return true;
  }

  return series.spread <= Math.max(series.maxValue * 0.03, 0.000005);
}

function probabilityTrendCompressionCopy(
  series: ReturnType<typeof probabilitySeriesStats>,
  axisShare: number
) {
  if (isProbabilitySeriesEffectivelyFlat(series)) {
    return "波动近似为常数，局部放大也会很平";
  }

  if (axisShare > 0 && axisShare < 0.12) {
    return `纵轴占比 ${formatPercentPrecise(axisShare)}，主图视觉上会贴底`;
  }

  return `纵轴占比 ${formatPercentPrecise(axisShare)}`;
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

function formatProbabilityDelta(value: number): string {
  if (value === 0) {
    return "持平";
  }

  const sign = value > 0 ? "+" : "";
  return `${sign}${formatProbabilityPercentExact(value)}`;
}

function historySourceLabel(source: AssessmentHistoryPoint["history_source"]): string {
  if (!source) {
    return "未返回";
  }
  const labels: Record<string, string> = {
    raw_pit_feature_replay: "当天 PIT 特征回放",
    raw_pit_feature_reuse: "沿用 PIT 特征",
    raw_observation_rebuild: "原始观测重建",
    raw_observation_replay: "原始观测回放",
    transitional_snapshot_bridge: "过渡快照桥接"
  };
  return labels[source] ?? source;
}

function buildProbabilityPointDetails(
  history: AssessmentHistoryPoint[],
  horizon: 5 | 20 | 60,
  mode: ProbabilityTrendMode
): LineChartPointDetail[] {
  return history.map((point, index) => {
    const value = probabilityValue(point, horizon, mode);
    const formalValue = probabilityValue(point, horizon, "calibrated");
    const rawValue = probabilityValue(point, horizon, "raw");
    const previous = index > 0 ? probabilityValue(history[index - 1], horizon, mode) : null;
    const deltaCopy =
      previous === null ? "首个点" : `较前点 ${formatProbabilityDelta(value - previous)}`;
    const modeCopy = mode === "raw" ? "原始" : "正式";
    const counterpartCopy =
      mode === "raw"
        ? `正式 ${formatProbabilityPercentExact(formalValue)}`
        : `原始 ${formatProbabilityPercentExact(rawValue)}`;

    return {
      valueLabel: `${formatProbabilityPercentExact(value)} · ${formatProbabilityBasisPoints(value)}`,
      detail: `${formatDate(point.as_of_date)} · ${timeBucketLabel(
        point.time_to_risk_bucket
      )} · ${postureLabel(point.posture)} · ${modeCopy} ${formatProbabilityDecimal(
        value
      )} · ${counterpartCopy} · ${deltaCopy} · 来源 ${historySourceLabel(point.history_source)}`
    };
  });
}

function buildLocalProbabilityAxis(values: number[]) {
  if (values.length === 0) {
    return { minValue: 0, maxValue: 0.02 };
  }

  const minValue = Math.min(...values);
  const maxValue = Math.max(...values);
  const spread = maxValue - minValue;
  if (spread <= 0) {
    const padding = Math.max(maxValue * 0.35, 0.00002);
    return {
      minValue: Math.max(0, minValue - padding),
      maxValue: Math.max(0.00004, maxValue + padding)
    };
  }

  const padding = Math.max(spread * 0.25, maxValue * 0.08, 0.00001);
  return {
    minValue: Math.max(0, minValue - padding),
    maxValue: maxValue + padding
  };
}

function normalizedProbabilityValues(values: number[]): number[] {
  if (values.length === 0) {
    return [];
  }

  const minValue = Math.min(...values);
  const maxValue = Math.max(...values);
  const spread = maxValue - minValue;
  if (spread <= 0) {
    return values.map(() => 50);
  }

  return values.map((value) => ((value - minValue) / spread) * 100);
}

function buildRelativePointDetails(
  history: AssessmentHistoryPoint[],
  horizon: 5 | 20 | 60,
  mode: ProbabilityTrendMode,
  normalizedValues: number[]
): LineChartPointDetail[] {
  return history.map((point, index) => {
    const value = probabilityValue(point, horizon, mode);
    return {
      valueLabel: `${normalizedValues[index].toFixed(0)} / 100`,
      detail: `${formatDate(point.as_of_date)} · ${timeBucketLabel(
        point.time_to_risk_bucket
      )} · ${postureLabel(point.posture)} · 原始概率 ${formatProbabilityPercentExact(
        value
      )} · ${formatProbabilityBasisPoints(value)} · 来源 ${historySourceLabel(point.history_source)}`
    };
  });
}

function buildProbabilityTrendSummaryMetrics(
  history: AssessmentHistoryPoint[],
  mode: ProbabilityTrendMode
): ProbabilityTrendMetric[] {
  if (history.length === 0) {
    return [];
  }

  const stats = [
    probabilitySeriesStats("5日窗口", history.map((point) => probabilityValue(point, 5, mode))),
    probabilitySeriesStats("20日窗口", history.map((point) => probabilityValue(point, 20, mode))),
    probabilitySeriesStats("60日窗口", history.map((point) => probabilityValue(point, 60, mode)))
  ];
  const yAxisMax = buildProbabilityAxisMax(Math.max(...stats.map((series) => series.maxValue), 0));

  return stats.map((series) => {
    const axisShare = yAxisMax > 0 ? series.maxValue / yAxisMax : 0;
    const compressionCopy = probabilityTrendCompressionCopy(series, axisShare);

    return {
      label: series.label,
      value: formatProbabilityPercentExact(series.latestValue),
      hint: `近期 ${formatProbabilityPercentExact(series.minValue)} - ${formatProbabilityPercentExact(
        series.maxValue
      )} · 波动 ${formatProbabilityPercentExact(series.spread)} · ${compressionCopy}`
    };
  });
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
  const yAxisMax = buildProbabilityAxisMax(probabilityMax);

  return {
    categories: history.map((point) => formatDate(point.as_of_date)),
    maxValue: yAxisMax,
    valueType: "percent",
    series: [
      {
        label: mode === "raw" ? "5日窗口（原始）" : "5日窗口",
        color: "#b45309",
        values: history.map((point) => probabilityValue(point, 5, mode)),
        pointDetails: buildProbabilityPointDetails(history, 5, mode)
      },
      {
        label: mode === "raw" ? "20日窗口（原始）" : "20日窗口",
        color: "#2563eb",
        values: history.map((point) => probabilityValue(point, 20, mode)),
        pointDetails: buildProbabilityPointDetails(history, 20, mode)
      },
      {
        label: mode === "raw" ? "60日窗口（原始）" : "60日窗口",
        color: "#115e59",
        fillColor: "rgba(17, 94, 89, 0.08)",
        values: history.map((point) => probabilityValue(point, 60, mode)),
        pointDetails: buildProbabilityPointDetails(history, 60, mode)
      }
    ]
  };
}

function buildTwentyDayZoomChart(
  history: AssessmentHistoryPoint[],
  mode: ProbabilityTrendMode
): LineChartModel {
  const values = history.map((point) => probabilityValue(point, 20, mode));
  const axis = buildLocalProbabilityAxis(values);

  return {
    categories: history.map((point) => formatDate(point.as_of_date)),
    minValue: axis.minValue,
    maxValue: axis.maxValue,
    valueType: "percent",
    series: [
      {
        label: mode === "raw" ? "20日窗口局部放大（原始）" : "20日窗口局部放大",
        color: "#2563eb",
        fillColor: "rgba(37, 99, 235, 0.08)",
        values,
        pointDetails: buildProbabilityPointDetails(history, 20, mode)
      }
    ]
  };
}

function buildProbabilityRelativeTrendChart(
  history: AssessmentHistoryPoint[],
  mode: ProbabilityTrendMode
): LineChartModel {
  const values5d = history.map((point) => probabilityValue(point, 5, mode));
  const values20d = history.map((point) => probabilityValue(point, 20, mode));
  const values60d = history.map((point) => probabilityValue(point, 60, mode));
  const relative5d = normalizedProbabilityValues(values5d);
  const relative20d = normalizedProbabilityValues(values20d);
  const relative60d = normalizedProbabilityValues(values60d);

  return {
    categories: history.map((point) => formatDate(point.as_of_date)),
    maxValue: 100,
    valueType: "score",
    series: [
      {
        label: mode === "raw" ? "5日相对（原始）" : "5日相对",
        color: "#b45309",
        values: relative5d,
        pointDetails: buildRelativePointDetails(history, 5, mode, relative5d)
      },
      {
        label: mode === "raw" ? "20日相对（原始）" : "20日相对",
        color: "#2563eb",
        values: relative20d,
        pointDetails: buildRelativePointDetails(history, 20, mode, relative20d)
      },
      {
        label: mode === "raw" ? "60日相对（原始）" : "60日相对",
        color: "#115e59",
        values: relative60d,
        pointDetails: buildRelativePointDetails(history, 60, mode, relative60d)
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

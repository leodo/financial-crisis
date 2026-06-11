import {
  dimensionLabel,
  formatDate,
  formatNumber,
  frequencyLabel,
  humanizeNarrativeCopy,
  indicatorRefLabel,
  indicatorQualityTierLabel,
  levelLabel,
  qualityDetailLabel,
  riskDirectionLabel,
  scoreBasisLabel,
  sourceLabel,
  formatSignedNumber,
  unitLabel
} from "../../format";
import type { IndicatorRisk } from "../../types";
import type { DetailRowItem, MetricItem } from "../shared/panelHelpers";

function formatValue(value: number | null | undefined, unit: string | null | undefined) {
  const numeric = formatNumber(value);
  const suffix = unitLabel(unit);
  return suffix ? `${numeric} ${suffix}` : numeric;
}

function formatSignedValue(value: number | null | undefined, unit: string | null | undefined) {
  const numeric = formatSignedNumber(value, 2);
  const suffix = unitLabel(unit);
  return suffix ? `${numeric} ${suffix}` : numeric;
}

function isDerivedScoreBasis(scoreBasis: string | null | undefined) {
  const basis = scoreBasisLabel(scoreBasis);
  const lower = basis.toLowerCase();
  return (
    basis.includes("变化") ||
    basis.includes("同比") ||
    basis.includes("振幅") ||
    lower.includes("change") ||
    lower.includes("delta") ||
    lower.includes("yoy")
  );
}

function scoreInputTitle(risk: IndicatorRisk) {
  const basis = scoreBasisLabel(risk.score_basis);
  if (risk.score_input_value === null) {
    return isDerivedScoreBasis(risk.score_basis) ? "评分输入缺失" : "当前读数缺失";
  }
  if (isDerivedScoreBasis(risk.score_basis)) {
    return `评分输入 ${formatSignedValue(risk.score_input_value, risk.score_input_unit)}`;
  }
  return `当前读数 ${formatValue(risk.score_input_value, risk.score_input_unit)}`;
}

function scoreInputDetails(risk: IndicatorRisk) {
  const basis = scoreBasisLabel(risk.score_basis);
  if (!isDerivedScoreBasis(risk.score_basis)) {
    return [basis, riskDirectionLabel(risk.indicator.risk_direction)];
  }

  return [
    `${basis}，不是 ${humanizeNarrativeCopy(risk.indicator.display_name)} 当前水平`,
    "当前水平请看左侧最近读数",
    riskDirectionLabel(risk.indicator.risk_direction)
  ];
}

export function useIndicatorsViewModel({ indicators }: { indicators: IndicatorRisk[] }) {
  const rows = [...indicators].sort((left, right) => right.score - left.score);
  const highRiskRows = rows.filter((risk) => risk.score >= 75);
  const stressRows = rows.filter((risk) => risk.score >= 60 && risk.score < 75);
  const missingRows = rows.filter(
    (risk) => risk.latest_observation === null || risk.quality_grade === "f"
  );
  const indicatorTitle = (risk: IndicatorRisk) => {
    const label = indicatorRefLabel(risk.indicator.indicator_id);
    return label !== risk.indicator.indicator_id
      ? label
      : humanizeNarrativeCopy(risk.indicator.display_name);
  };

  const summaryMetrics: MetricItem[] = [
    { label: "跟踪指标", value: `${rows.length}` },
    {
      label: "高压指标",
      value: `${highRiskRows.length}`,
      hint:
        highRiskRows.length > 0
          ? `最靠前的是 ${indicatorTitle(highRiskRows[0])}`
          : "当前没有指标落在 75 分以上。"
    },
    {
      label: "压力积累",
      value: `${stressRows.length}`,
      hint: "60-75 分表示更像风险在积累，而不是已经进入极端高压。"
    },
    {
      label: "缺测/降级",
      value: `${missingRows.length}`,
      hint: "这部分指标更适合当作观察项，不宜单独支撑强动作。"
    }
  ];

  const focusRows: DetailRowItem[] = rows.slice(0, 3).map((risk) => ({
    id: risk.indicator.indicator_id,
    title: indicatorTitle(risk),
    detail: `${scoreInputTitle(risk)} · ${scoreBasisLabel(risk.score_basis)}`,
    meta: formatNumber(risk.score),
    note: `${
      isDerivedScoreBasis(risk.score_basis)
        ? `最近读数 ${formatValue(risk.latest_observation?.value, risk.indicator.unit)} · `
        : ""
    }${formatDate(risk.latest_observation?.as_of_date)} · ${sourceLabel(
      risk.latest_observation?.source_id ?? risk.indicator.default_source_id
    )}`
  }));

  const tableRows = rows.map((risk) => ({
    id: risk.indicator.indicator_id,
    indicatorTitle: indicatorTitle(risk),
    indicatorDetails: [
      dimensionLabel(risk.indicator.dimension),
      `${frequencyLabel(risk.indicator.frequency)} · ${indicatorQualityTierLabel(
        risk.indicator.quality_tier
      )}`
    ],
    latestValueTitle:
      risk.latest_observation !== null
        ? formatValue(risk.latest_observation.value, risk.indicator.unit)
        : "暂无观测",
    latestValueDetail:
      risk.latest_observation !== null
        ? `${formatDate(risk.latest_observation.as_of_date)} · ${sourceLabel(
            risk.latest_observation.source_id
          )}`
        : "这项指标当前没有落库观测。",
    basisTitle: scoreInputTitle(risk),
    basisDetails: scoreInputDetails(risk),
    scoreTitle: formatNumber(risk.score),
    scoreDetail: levelLabel(risk.level),
    percentileTitle:
      risk.percentile !== null ? formatNumber(risk.percentile, "%") : "—",
    percentileDetail:
      risk.change_30d !== null ? `30d ${formatNumber(risk.change_30d)}` : "30d 无可比变化",
    qualityTitle: `指标级 ${qualityDetailLabel(risk.quality_grade)}`,
    qualityDetails: [
      sourceLabel(risk.indicator.default_source_id),
      risk.latest_observation !== null
        ? `单项观测质量分 ${formatNumber(risk.latest_observation.quality_score)}`
        : "当前无单项观测质量分",
      "不是整体结论可信度"
    ]
  }));

  return {
    summaryMetrics,
    focusRows,
    tableRows
  };
}

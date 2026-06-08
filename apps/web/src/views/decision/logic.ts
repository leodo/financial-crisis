import {
  pointInTimeModeLabel,
  probabilityModeLabel,
  releaseServingStatusLabel
} from "../../format";
import type { AssessmentMethodResponse, AssessmentSnapshot } from "../../types";

export function describeProbabilityMode(method: AssessmentSnapshot["method"]) {
  if (method.probability_mode === "heuristic_mvp") {
    return {
      label: probabilityModeLabel(method.probability_mode),
      hint: `${probabilityModeLabel(method.probability_mode)} / ${pointInTimeModeLabel(method.point_in_time_mode)}`
    };
  }

  if (method.probability_mode.startsWith("formal_bundle")) {
    const label = method.actionability_enabled
      ? method.point_in_time_mode === "strict"
        ? "正式概率包·双头候选"
        : "正式概率包·双头过渡"
      : method.point_in_time_mode === "strict"
        ? "正式概率包·候选"
        : "正式概率包·过渡";
    return {
      label,
      hint: `${probabilityModeLabel(method.probability_mode)} / ${pointInTimeModeLabel(method.point_in_time_mode)}`
    };
  }

  return {
    label: "研究模式",
    hint: `${probabilityModeLabel(method.probability_mode)} / ${pointInTimeModeLabel(method.point_in_time_mode)}`
  };
}

export function describeReleaseHealth(status: string) {
  return releaseServingStatusLabel(status);
}

export function describeRollingAuditBoundary(method: AssessmentMethodResponse) {
  if (method.method.probability_mode === "heuristic_mvp") {
    return "当前滚动审计主要用于解释启发式动作层在历史上的表现，不能把它当成正式概率模型命中率。";
  }

  if (method.history_provenance.snapshot_bridge_points > 0) {
    return `当前默认历史轨迹里仍有 ${method.history_provenance.snapshot_bridge_points}/${method.history_provenance.total_points} 个点来自旧 snapshot bridge。它更适合比较哪些阶段风险在升温、误报有没有收缩，还不能直接当成最终正式模型的命中率。`;
  }

  if (method.history_provenance.raw_observation_points > 0) {
    return `当前默认历史轨迹已经避开旧 snapshot bridge，但仍有 ${method.history_provenance.raw_observation_points}/${method.history_provenance.total_points} 个点只是 raw observation 过渡口径。它已经比旧 bridge 更接近正式历史证据，但仍要结合 PIT 特征落库覆盖一起解释。`;
  }

  if (
    method.history_provenance.feature_backed_points > 0 &&
    method.history_provenance.feature_backed_points === method.history_provenance.total_points
  ) {
    return `当前默认历史轨迹 ${method.history_provenance.feature_backed_points}/${method.history_provenance.total_points} 个点都绑定到已落库 PIT feature snapshot，滚动审计已经进入 formal history 审计的正式证据层；后续重点应放在模型本体命中率和样本覆盖，而不是再把它当成旧 bridge 兼容结果。`;
  }

  return "当前滚动审计可以帮助比较模型在不同历史阶段是否稳定，但仍要结合数据覆盖、PIT 可见性和 release review 一起解释。";
}

export const RISK_SCORE_BANDS = [
  {
    label: "常态区",
    min: 0,
    maxExclusive: 45,
    rangeText: "0 - 45",
    note: "缓冲因素占优，系统通常不会给出高执行档位。"
  },
  {
    label: "积累区",
    min: 45,
    maxExclusive: 60,
    rangeText: "45 - 60",
    note: "脆弱性开始积累，需要盯触发因子。"
  },
  {
    label: "高压区",
    min: 60,
    maxExclusive: 75,
    rangeText: "60 - 75",
    note: "系统会结合概率和数据可信度考虑保护动作。"
  },
  {
    label: "危机样态区",
    min: 75,
    maxExclusive: 101,
    rangeText: "75 - 100",
    note: "更接近历史危机高压样态，但仍不等于危机已经发生。"
  }
] as const;

export function describeRiskScoreBand(score: number) {
  const band =
    RISK_SCORE_BANDS.find((item) => score >= item.min && score < item.maxExclusive) ??
    RISK_SCORE_BANDS[RISK_SCORE_BANDS.length - 1];

  return {
    label: band.label,
    description: `当前位于${band.label}。${band.note}`
  };
}

export function describeTimeBucket(bucket: AssessmentSnapshot["time_to_risk_bucket"]) {
  const mapping: Record<AssessmentSnapshot["time_to_risk_bucket"], string> = {
    normal: "系统还没有看到可交易的近端风险窗口，更偏向常态监控。",
    months: "脆弱性在积累，但更像数月级风险，而不是马上发生的冲击。",
    weeks: "风险已经压缩到数周级别，应该提前准备流动性和保护动作。",
    now: "短期风险窗口已经打开，更接近历史危机前 1 到 4 周或当下冲击区间。"
  };

  return mapping[bucket];
}

export function describeAnalogWindow(
  analog: AssessmentSnapshot["historical_analogs"][number] | undefined,
  bucket: AssessmentSnapshot["time_to_risk_bucket"]
) {
  if (!analog) {
    return describeTimeBucket(bucket);
  }

  if (analog.lead_time_days === null && analog.actionable_lead_time_days === null) {
    return `当前最接近 ${analog.name} 的压力阶段，但该历史样本没有可用提前量估计。`;
  }

  if (analog.actionable_lead_time_days === null) {
    return `当前最接近 ${analog.name} 的结构脆弱阶段，历史上大约提前 ${analog.lead_time_days} 天先出现类似压力，但危机前未形成足够强的可执行预警。`;
  }

  if (analog.lead_time_days === null) {
    return `当前最接近 ${analog.name} 的风险窗口，历史上大约提前 ${analog.actionable_lead_time_days} 天进入可执行预警。`;
  }

  return `当前最接近 ${analog.name} 的风险窗口，历史上大约提前 ${analog.lead_time_days} 天进入结构抬升，并在约提前 ${analog.actionable_lead_time_days} 天进入可执行预警。`;
}

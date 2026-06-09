import {
  compactTechnicalId,
  dataModeLabel,
  describePostureClause,
  formatDate,
  formatDateTime,
  formatNumber,
  formatPercent,
  formatPercentPrecise,
  formatPreciseNumber,
  formatProbabilityPercent,
  formatSignedNumber,
  freshnessLabel,
  pointInTimeModeLabel,
  postureLabel,
  releaseIdLabel,
  releaseServingStatusLabel,
  runtimeThresholdLabel,
  sourceLabel,
  timeBucketLabel,
  unitLabel,
  userProfileLabel,
  humanizeNarrativeCopy
} from "../../format";
import type {
  AssessmentMethodResponse,
  AssessmentSnapshot,
  PostureGuidance
} from "../../types";
import type { MetricItem } from "../shared/panelHelpers";
import type {
  DecisionAnalogRow,
  DecisionKeyIndicatorRow,
  DecisionRuntimeCard,
  DecisionRuntimeNotice,
  DecisionScoreBandRow,
  DecisionSignalLayerRowModel
} from "./builderTypes";
import { decisionContent } from "./content";
import {
  RISK_SCORE_BANDS,
  describeProbabilityMode,
  describeReleaseHealth
} from "./logic";

function formatOptionalNumber(value: number | null, unit?: string) {
  return value === null ? "—" : formatNumber(value, unit);
}

function formatLagSummary(
  calendarLagDays: number | null | undefined,
  businessLagDays: number | null | undefined
) {
  if (calendarLagDays === null || calendarLagDays === undefined) {
    return "当前没有可用滞后信息。";
  }
  if (businessLagDays === null || businessLagDays === undefined) {
    return `自然日滞后 ${calendarLagDays} 天。`;
  }
  return `自然日滞后 ${calendarLagDays} 天；按工作日口径约 ${businessLagDays} 天。`;
}

function actionSourceSummary(assessment: AssessmentSnapshot) {
  if (!assessment.method.actionability_enabled) {
    return {
      label: "过渡动作映射",
      detail:
        "当前线上版本还没有独立动作模型，准备/对冲/防守仍由危机先验和评分层过渡映射而来，只适合辅助执行节奏，不应当成正式校准后的独立动作概率。"
    };
  }

  const actionModel =
    assessment.method.actionability_model_version
      ? compactTechnicalId(assessment.method.actionability_model_version).value
      : "动作模型";
  const fusionPolicy =
    assessment.method.fusion_policy_version
      ? compactTechnicalId(assessment.method.fusion_policy_version).value
      : "融合层";

  return {
    label: "双层动作模型",
    detail: `当前已启用独立动作模型和融合层：${actionModel} / ${fusionPolicy}`
  };
}

function probabilityDisplayNote(assessment: AssessmentSnapshot): string | null {
  const peakProbability = Math.max(
    assessment.probabilities.p_5d,
    assessment.probabilities.p_20d,
    assessment.probabilities.p_60d
  );
  if (peakProbability >= 0.01) {
    return null;
  }
  const staleDays =
    assessment.runtime.latest_key_indicator_lag_business_days ??
    assessment.runtime.latest_observation_lag_business_days ??
    assessment.runtime.latest_key_indicator_lag_days ??
    assessment.runtime.latest_observation_lag_days;
  if (peakProbability === 0) {
    return staleDays !== null && staleDays >= 7
      ? `当前 formal 先验低于展示精度，且关键观测按工作日口径已滞后约 ${staleDays} 天；这代表“暂未看到足够证据支持主动防守”，不代表市场风险被证明为零。`
      : "当前 formal 先验低于展示精度；这代表“风险很低”，不代表市场风险被证明为零。";
  }
  return staleDays !== null && staleDays >= 7
    ? `当前 formal 先验仍低于 1%，且关键观测按工作日口径已滞后约 ${staleDays} 天；短期判断应保守解释。`
    : "当前 formal 先验仍低于 1%，属于低位区间，而不是零风险断言。";
}

function formatActionProbability(value: number, actionabilityEnabled: boolean): string {
  if (value === 0) {
    return actionabilityEnabled ? "0%" : "未触发";
  }
  return formatProbabilityPercent(value);
}

function formatActionCurrentValue(value: number, actionabilityEnabled: boolean): string {
  if (value === 0 && !actionabilityEnabled) {
    return `当前未触发（原始值 ${formatPercentPrecise(value)}）。`;
  }
  return `当前值 ${formatPercentPrecise(value)}。`;
}

function formatActionDetailValue(label: string, value: number, actionabilityEnabled: boolean): string {
  if (value === 0 && !actionabilityEnabled) {
    return `${label} 未触发（原始值 ${formatPercentPrecise(value)}）`;
  }
  return `${label} ${formatPercentPrecise(value)}`;
}

export function buildTriggerClauses(posture: PostureGuidance) {
  return posture.trigger_codes.map(describePostureClause);
}

export function buildBlockerClauses(posture: PostureGuidance) {
  return posture.blocker_codes.map(describePostureClause);
}

export function buildRuntimeNotice(
  runtime: AssessmentSnapshot["runtime"]
): DecisionRuntimeNotice | undefined {
  if (!runtime.stale_warning) {
    return undefined;
  }

  return {
    tone: runtime.demo_mode ? "notice error" : "notice",
    title: runtime.demo_mode ? "当前是 Demo 数据" : "当前数据存在时效性提醒",
    body: runtime.stale_warning
  };
}

export function buildRuntimeChipLabel(runtime: AssessmentSnapshot["runtime"]) {
  return runtime.demo_mode ? "Demo 样例" : `${dataModeLabel(runtime.data_mode)} 已加载`;
}

export function buildRuntimeCards(
  assessment: AssessmentSnapshot,
  usdJpyIndicator: AssessmentSnapshot["key_indicators"][number] | undefined
): DecisionRuntimeCard[] {
  return [
    {
      label: "最新关键观测",
      value: formatDate(assessment.runtime.latest_key_indicator_at ?? assessment.runtime.latest_observation_at),
      detail: formatLagSummary(
        assessment.runtime.latest_key_indicator_lag_days ??
          assessment.runtime.latest_observation_lag_days,
        assessment.runtime.latest_key_indicator_lag_business_days ??
          assessment.runtime.latest_observation_lag_business_days
      )
    },
    {
      label: "本次评估生成",
      value: formatDateTime(assessment.runtime.generated_at),
      detail: decisionContent.prelude.generatedHint
    },
    {
      label: "当前 USDJPY",
      value: formatNumber(usdJpyIndicator?.latest_value),
      detail: usdJpyIndicator?.latest_as_of_date
        ? `${formatDate(usdJpyIndicator.latest_as_of_date)} · ${sourceLabel(usdJpyIndicator.source_id)} · ${freshnessLabel(usdJpyIndicator.status)}`
        : "缺少 USDJPY 最新观测。"
    },
    {
      label: "系统节奏",
      value: decisionContent.prelude.cadenceTitle,
      detail: decisionContent.prelude.cadenceHint
    }
  ];
}

export function buildHeroMetrics(assessment: AssessmentSnapshot): MetricItem[] {
  return [
    { label: "结论把握度", value: formatPercent(assessment.conviction_score) },
    { label: "数据覆盖", value: formatPercent(assessment.data_trust.coverage_score) },
    { label: "风险强度", value: formatNumber(assessment.scores.overall_score) }
  ];
}

export function buildRiskHorizonActionMetrics(
  assessment: AssessmentSnapshot
): MetricItem[] {
  const actionSource = actionSourceSummary(assessment);
  const prepareLabel = assessment.method.actionability_enabled ? "准备动作" : "准备动作（过渡）";
  const hedgeLabel = assessment.method.actionability_enabled ? "对冲动作" : "对冲动作（过渡）";
  const defendLabel = assessment.method.actionability_enabled ? "防守动作" : "防守动作（过渡）";

  return [
    {
      label: prepareLabel,
      value: formatActionProbability(
        assessment.actionability.prepare,
        assessment.method.actionability_enabled
      ),
      hint: `${decisionContent.riskHorizon.actionHints.prepare} ${formatActionCurrentValue(
        assessment.actionability.prepare,
        assessment.method.actionability_enabled
      )}`
    },
    {
      label: hedgeLabel,
      value: formatActionProbability(
        assessment.actionability.hedge,
        assessment.method.actionability_enabled
      ),
      hint: `${decisionContent.riskHorizon.actionHints.hedge} ${formatActionCurrentValue(
        assessment.actionability.hedge,
        assessment.method.actionability_enabled
      )}`
    },
    {
      label: defendLabel,
      value: formatActionProbability(
        assessment.actionability.defend,
        assessment.method.actionability_enabled
      ),
      hint: `${decisionContent.riskHorizon.actionHints.defend} ${formatActionCurrentValue(
        assessment.actionability.defend,
        assessment.method.actionability_enabled
      )}`
    },
    {
      label: "动作头来源",
      value: actionSource.label,
      hint: actionSource.detail
    }
  ];
}

export function buildScoreBandRows(currentRiskBandLabel: string): DecisionScoreBandRow[] {
  return RISK_SCORE_BANDS.map((band) => ({
    label: band.label,
    rangeText: band.rangeText,
    note: band.note,
    active: currentRiskBandLabel === band.label
  }));
}

export function buildDataTrustMetrics(assessment: AssessmentSnapshot): MetricItem[] {
  return [
    { label: "总覆盖", value: formatPercent(assessment.data_trust.coverage_score) },
    { label: "核心特征", value: formatPercent(assessment.data_trust.core_feature_coverage) },
    { label: "触发特征", value: formatPercent(assessment.data_trust.trigger_feature_coverage) },
    { label: "外部特征", value: formatPercent(assessment.data_trust.external_feature_coverage) }
  ];
}

export function buildPostureThresholdMetrics(
  method: AssessmentMethodResponse
): MetricItem[] {
  return [
    {
      label: runtimeThresholdLabel("prepare floor"),
      value: formatPercent(method.runtime_thresholds.prepare_p60d),
      hint: "这是系统进入准备档的运行底线之一，不是准备动作概率本身。"
    },
    {
      label: runtimeThresholdLabel("hedge floor"),
      value: formatPercent(method.runtime_thresholds.hedge_p20d),
      hint: "这是系统进入对冲档的运行底线之一，不是对冲动作概率本身。"
    },
    {
      label: runtimeThresholdLabel("defend floor"),
      value: formatPercent(method.runtime_thresholds.defend_p5d),
      hint: "这是系统进入防守档的运行底线之一，不是防守动作概率本身。"
    }
  ];
}

export function buildKeyIndicatorRows(
  keyIndicators: AssessmentSnapshot["key_indicators"]
): DecisionKeyIndicatorRow[] {
  return keyIndicators.map((item) => ({
    id: `${item.entity_id}-${item.indicator_id}`,
    title: `${item.display_name} · ${freshnessLabel(item.status)}`,
    detail: `${formatNumber(item.latest_value)} ${unitLabel(item.unit)} · 日期 ${
      item.latest_as_of_date ? formatDate(item.latest_as_of_date) : "—"
    } · 来源 ${sourceLabel(item.source_id)}${
      item.lag_days !== null
        ? ` · ${formatLagSummary(item.lag_days, item.lag_business_days)}`
        : ""
    }`,
    note: humanizeNarrativeCopy(item.note)
  }));
}

export function buildSignalLayerRows(
  assessment: AssessmentSnapshot,
  method: AssessmentMethodResponse,
  posture: PostureGuidance
): DecisionSignalLayerRowModel[] {
  const actionabilitySource = actionSourceSummary(assessment).detail;
  const priorDetail = probabilityDisplayNote(assessment);
  const priorThresholdSummary = `当前进入线：准备 ${formatPercent(method.runtime_thresholds.prepare_p60d)} / 对冲 ${formatPercent(method.runtime_thresholds.hedge_p20d)} / 防守 ${formatPercent(method.runtime_thresholds.defend_p5d)}`;

  return [
    {
      id: "prior",
      title: "危机先验",
      description: "先看未来 5d / 20d / 60d 进入风险窗口的概率，回答“离风险还有多远”。",
      value: `${formatProbabilityPercent(assessment.probabilities.p_5d, { zeroLabel: "<0.01%" })} / ${formatProbabilityPercent(assessment.probabilities.p_20d, { zeroLabel: "<0.01%" })} / ${formatProbabilityPercent(assessment.probabilities.p_60d, { zeroLabel: "<0.01%" })}`,
      detail: priorDetail ? `${priorThresholdSummary} · ${priorDetail}` : priorThresholdSummary
    },
    {
      id: "actionability",
      title: "动作概率",
      description: "再看准备 / 对冲 / 防守，回答“现在该不该开始准备、加保护、保流动性”。",
      value: `${formatActionProbability(
        assessment.actionability.prepare,
        assessment.method.actionability_enabled
      )} / ${formatActionProbability(
        assessment.actionability.hedge,
        assessment.method.actionability_enabled
      )} / ${formatActionProbability(
        assessment.actionability.defend,
        assessment.method.actionability_enabled
      )}`,
      detail: `${actionabilitySource} 当前显示：${formatActionDetailValue(
        "准备",
        assessment.actionability.prepare,
        assessment.method.actionability_enabled
      )} / ${formatActionDetailValue(
        "对冲",
        assessment.actionability.hedge,
        assessment.method.actionability_enabled
      )} / ${formatActionDetailValue(
        "防守",
        assessment.actionability.defend,
        assessment.method.actionability_enabled
      )}。`
    },
    {
      id: "posture",
      title: "最终执行节奏",
      description: "最后再叠加数据可信度、事件确认、日元套息放大器和用户偏好，压成一档执行节奏。",
      value: `${postureLabel(assessment.posture)} / ${timeBucketLabel(assessment.time_to_risk_bucket)}`,
      detail: posture.summary
    }
  ];
}

export function buildAnalogRows(
  historicalAnalogs: AssessmentSnapshot["historical_analogs"]
): DecisionAnalogRow[] {
  return historicalAnalogs.map((analog) => ({
    id: analog.scenario_id,
    title: analog.name,
    detail: humanizeNarrativeCopy(
      `相似度 ${formatNumber(analog.similarity_score)} · 结构抬升 ${formatOptionalNumber(analog.lead_time_days, "d")} · 可执行预警 ${formatOptionalNumber(analog.actionable_lead_time_days, "d")} · ${analog.note}`
    ),
    score: formatNumber(analog.similarity_score)
  }));
}

export function buildActionPlanMetrics(
  assessment: AssessmentSnapshot
): MetricItem[] {
  const probabilityMode = describeProbabilityMode(assessment.method);
  const releaseHealth = describeReleaseHealth(assessment.method.release_status);
  const compactReleaseId = releaseIdLabel(assessment.method.release_id);
  const governance = assessment.position_guidance.governance;

  return [
    { label: "概率模式", value: probabilityMode.label, hint: probabilityMode.hint },
    {
      label: "运行状态",
      value: releaseHealth,
      hint: assessment.method.release_id
        ? `${compactReleaseId.value} · ${pointInTimeModeLabel(assessment.method.point_in_time_mode)}`
        : releaseServingStatusLabel(assessment.method.release_status)
    },
    {
      label: "动作框架",
      value: assessment.position_guidance.capital_preservation_overlay_enabled
        ? "资本保全"
        : "分层防守",
      hint: compactTechnicalId(assessment.position_guidance.action_playbook_version).value
    },
    {
      label: "建议性质",
      value: governance.system_budget_only ? "系统预算建议" : "可执行指令",
      hint: governance.system_budget_only
        ? "只回答系统层面的减震、对冲和现金预算，不替代个性化投资建议。"
        : "当前版本允许直接执行。"
    },
    {
      label: "自动执行",
      value: governance.auto_execution_allowed ? "允许" : "禁止",
      hint: governance.auto_execution_allowed
        ? "当前版本允许自动执行动作。"
        : "当前面板不下交易指令，仍需人工确认。"
    },
    {
      label: "规则治理",
      value:
        governance.policy_change_requires_release_review && governance.policy_change_requires_go_no_go
          ? "需评审 + Go/No-Go"
          : "普通变更",
      hint:
        "任何动作规则升级都应先经过 release review，再满足正式 Go/No-Go，不能只凭页面观感放行。"
    }
  ];
}

export function buildJpyCarryMetrics(
  assessment: AssessmentSnapshot,
  usdJpyIndicator: AssessmentSnapshot["key_indicators"][number] | undefined
): MetricItem[] {
  return [
    {
      label: "USDJPY",
      value: formatNumber(assessment.jpy_carry.usdjpy_level),
      hint: usdJpyIndicator?.latest_as_of_date
        ? `${formatDate(usdJpyIndicator.latest_as_of_date)} · ${sourceLabel(usdJpyIndicator.source_id)}`
        : "无最新日期"
    },
    { label: "5d 变化", value: formatNumber(assessment.jpy_carry.change_5d) },
    { label: "日短端", value: formatNumber(assessment.jpy_carry.jp_call_rate, "%") },
    { label: "美短端", value: formatNumber(assessment.jpy_carry.us_short_rate, "%") },
    {
      label: "美日利差",
      value: formatSignedNumber(assessment.jpy_carry.us_jp_short_rate_diff, 2, "%")
    },
    { label: "20d 波动", value: formatPreciseNumber(assessment.jpy_carry.realized_vol_20d) },
    {
      label: "融资压力",
      value: formatNumber(assessment.jpy_carry.funding_pressure_score)
    },
    { label: "VIX 联动", value: formatNumber(assessment.jpy_carry.vix_coupling_score) }
  ];
}

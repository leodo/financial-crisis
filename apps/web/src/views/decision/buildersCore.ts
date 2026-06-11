import {
  compactTechnicalId,
  dataModeLabel,
  describePostureClause,
  formatDate,
  formatDateTime,
  formatNumber,
  formatPercent,
  formatPercentPrecise,
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
  DecisionScoreBandRow
} from "./builderTypes";
import { decisionContent } from "./content";
import { decisionReliabilityHint, decisionReliabilityLabel } from "./decisionReliability";
import {
  RISK_SCORE_BANDS,
  describeProbabilityMode,
  describeReleaseHealth
} from "./logic";
import {
  currentMvpRiskState,
  mvpProbabilityInputIsAuditOnly,
  mvpRiskStateDetail,
  mvpRiskStateDisplayLabel
} from "./mvpRiskState";
import {
  actionEvidenceHint,
  actionEvidenceScore,
  actionEvidenceStatus,
  actionSourceSummary,
  formatActionProbability,
  hasRuntimeProbabilityOverride,
  probabilitySnapshotDetail,
  probabilitySnapshotValue
} from "./signalLayerBuilders";

export { buildSignalLayerRows } from "./signalLayerBuilders";

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

function indicatorSourceTimingLabel(
  item: AssessmentSnapshot["key_indicators"][number]
): string {
  if (item.indicator_id === "us_external_usdjpy_level") {
    if (item.source_id === "boj") {
      return "BOJ 9:00 JST 官方点位，非盘中价";
    }
    if (item.source_id === "fred") {
      return "FRED DEXJPUS 日频，非盘中价";
    }
  }
  return sourceLabel(item.source_id);
}

function formatActionCurrentValue(value: number, actionabilityEnabled: boolean): string {
  if (value === 0 && !actionabilityEnabled) {
    return "当前未触发；过渡动作层没有形成可执行动作信号。";
  }
  return `当前值 ${formatPercentPrecise(value)}。`;
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
  const probabilityIsReferenceOnly = mvpProbabilityInputIsAuditOnly(assessment);
  const hasRuntimeOverride = hasRuntimeProbabilityOverride(assessment);
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
      label:
        probabilityIsReferenceOnly || hasRuntimeOverride ? "运行口径参考概率" : "当前概率快照",
      value: probabilitySnapshotValue(assessment.probabilities),
      detail: probabilitySnapshotDetail(assessment)
    },
    {
      label: "当前 USDJPY",
      value: formatNumber(usdJpyIndicator?.latest_value),
      detail: usdJpyIndicator?.latest_as_of_date
        ? `${formatDate(usdJpyIndicator.latest_as_of_date)} · ${indicatorSourceTimingLabel(usdJpyIndicator)} · ${freshnessLabel(usdJpyIndicator.status)}`
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
  const evidenceScore = actionEvidenceScore(assessment);
  const state = currentMvpRiskState(assessment);
  return [
    {
      label: "MVP 风险状态",
      value: mvpRiskStateDisplayLabel(state.label),
      hint: mvpRiskStateDetail(assessment),
      valueClassName: "metric-value-token"
    },
    {
      label: "结论可靠性",
      value: decisionReliabilityLabel(assessment),
      hint: decisionReliabilityHint(assessment),
      valueClassName: "metric-value-token"
    },
    {
      label: "动作升级证据",
      value: actionEvidenceStatus(evidenceScore),
      hint: actionEvidenceHint(assessment),
      valueClassName: "metric-value-token"
    },
    {
      label: "关键指标覆盖",
      value: formatPercent(assessment.data_trust.coverage_score),
      hint: "衡量当前关键指标覆盖度；不等同于全部免费源都健康。"
    }
  ];
}

export function buildRiskHorizonActionMetrics(
  assessment: AssessmentSnapshot
): MetricItem[] {
  const actionSource = actionSourceSummary(assessment);
  const actionValuesAreAuxiliary =
    !assessment.method.actionability_enabled || mvpProbabilityInputIsAuditOnly(assessment);
  const prepareLabel = assessment.method.actionability_enabled ? "准备动作" : "准备动作（过渡）";
  const hedgeLabel = assessment.method.actionability_enabled ? "对冲动作" : "对冲动作（过渡）";
  const defendLabel = assessment.method.actionability_enabled ? "防守动作" : "防守动作（过渡）";

  return [
    {
      label: prepareLabel,
      value: actionValuesAreAuxiliary
        ? "辅助信号"
        : formatActionProbability(
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
      value: actionValuesAreAuxiliary
        ? "辅助信号"
        : formatActionProbability(
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
      value: actionValuesAreAuxiliary
        ? "辅助信号"
        : formatActionProbability(
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
    } · 来源 ${indicatorSourceTimingLabel(item)}${
      item.lag_days !== null
        ? ` · ${formatLagSummary(item.lag_days, item.lag_business_days)}`
        : ""
    }`,
    meta: keyIndicatorLineageMeta(item.lineage),
    note: keyIndicatorLineageNote(item.note, item.lineage)
  }));
}

function keyIndicatorLineageMeta(
  lineage: AssessmentSnapshot["key_indicators"][number]["lineage"]
): string | undefined {
  if (!lineage) {
    return undefined;
  }
  const rawRef = lineage.raw_payload_id ? `raw ${lineage.raw_payload_id.slice(0, 8)}` : undefined;
  const runRef = lineage.run_status ? `run ${runStatusLabel(lineage.run_status)}` : undefined;
  const evidenceLabel = lineageEvidenceLabel(lineage.evidence_level);
  return [evidenceLabel, runRef, rawRef].filter(Boolean).join(" · ");
}

function keyIndicatorLineageNote(
  note: string,
  lineage: AssessmentSnapshot["key_indicators"][number]["lineage"]
): string {
  const baseNote = humanizeNarrativeCopy(note);
  if (!lineage) {
    return baseNote;
  }
  const fetchedAt = lineage.fetched_at ? `抓取时间 ${formatDateTime(lineage.fetched_at)}` : null;
  const recordsWritten =
    lineage.records_written !== null ? `写入 ${lineage.records_written} 条` : null;
  return [baseNote, `追溯：${lineage.note}`, fetchedAt, recordsWritten]
    .filter(Boolean)
    .join(" ");
}

function lineageEvidenceLabel(
  evidenceLevel: NonNullable<
    AssessmentSnapshot["key_indicators"][number]["lineage"]
  >["evidence_level"]
) {
  const labels: Record<
    NonNullable<AssessmentSnapshot["key_indicators"][number]["lineage"]>["evidence_level"],
    string
  > = {
    run_raw_observation: "run+raw",
    raw_observation: "raw",
    observation_only: "仅观测",
    missing: "无追溯"
  };
  return labels[evidenceLevel];
}

function runStatusLabel(status: string) {
  const labels: Record<string, string> = {
    success: "成功",
    failed: "失败",
    running: "运行中",
    skipped: "跳过"
  };
  return labels[status] ?? status;
}

export function analogLeadText(leadDays: number | null, actionableDays: number | null): string {
  if (leadDays !== null && actionableDays !== null) {
    return `结构抬升约领先 ${Math.round(leadDays).toLocaleString("zh-CN")} 天，可执行预警约领先 ${Math.round(actionableDays).toLocaleString("zh-CN")} 天`;
  }
  if (leadDays !== null) {
    return `结构抬升约领先 ${Math.round(leadDays).toLocaleString("zh-CN")} 天，但未形成可执行预警`;
  }
  if (actionableDays !== null) {
    return `可执行预警约领先 ${Math.round(actionableDays).toLocaleString("zh-CN")} 天`;
  }
  return "无可用提前量估计";
}

export function buildAnalogRows(assessment: AssessmentSnapshot): DecisionAnalogRow[] {
  return assessment.historical_analogs.map((analog) => ({
    id: analog.scenario_id,
    title: analog.name,
    similarity: `${formatNumber(analog.similarity_score)} /100`,
    historicalLead: analogLeadText(analog.lead_time_days, analog.actionable_lead_time_days),
    gap: historicalEvidenceDifference(assessment.scores.overall_score, analog.peak_score),
    detail: humanizeNarrativeCopy(analog.note)
  }));
}

function historicalEvidenceDifference(currentScore: number, historicalPeakScore: number): string {
  const gap = historicalPeakScore - currentScore;
  const absoluteGap = formatNumber(Math.abs(gap));
  if (Math.abs(gap) < 0.05) {
    return `接近历史峰值 ${formatNumber(historicalPeakScore)}`;
  }
  if (gap > 0) {
    return `低于历史峰值 ${absoluteGap} 分`;
  }
  return `高于历史峰值 ${absoluteGap} 分`;
}

export function buildActionPlanMetrics(
  assessment: AssessmentSnapshot
): MetricItem[] {
  const probabilityMode = describeProbabilityMode(assessment.method);
  const releaseHealth = describeReleaseHealth(assessment.method.release_status);
  const compactReleaseId = releaseIdLabel(assessment.method.release_id);
  const governance = assessment.position_guidance.governance;
  const budgetIsReference =
    mvpProbabilityInputIsAuditOnly(assessment) || !assessment.method.actionability_enabled;

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
        ? budgetIsReference
          ? "资本保全（参考）"
          : "资本保全"
        : budgetIsReference
          ? "分层防守（参考）"
          : "分层防守",
      hint: `${compactTechnicalId(assessment.position_guidance.action_playbook_version).value}${
        budgetIsReference ? " · 当前先按预算参考解读" : ""
      }`
    },
    {
      label: "建议性质",
      value: governance.system_budget_only
        ? budgetIsReference
          ? "系统预算参考"
          : "系统预算建议"
        : "可执行指令",
      hint: governance.system_budget_only
        ? budgetIsReference
          ? "当前预算数字只作为系统层参考边界，不替代个性化投资建议，也不应直接照抄执行。"
          : "只回答系统层面的减震、对冲和现金预算，不替代个性化投资建议。"
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
    {
      label: "20d 日收益波动",
      value: formatPercentPrecise(assessment.jpy_carry.realized_vol_20d),
      hint: "USDJPY 20 日窗口日变化率标准差；后端返回小数口径，页面按百分比展示。"
    },
    {
      label: "融资压力",
      value: formatNumber(assessment.jpy_carry.funding_pressure_score)
    },
    { label: "VIX 联动", value: formatNumber(assessment.jpy_carry.vix_coupling_score) }
  ];
}

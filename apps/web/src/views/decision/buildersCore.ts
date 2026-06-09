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
  formatProbabilityPercentExact,
  formatProbabilityPercent,
  formatSignedNumber,
  freshnessLabel,
  pointInTimeModeLabel,
  postureLabel,
  qualityDetailLabel,
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

function probabilitySnapshotValue(probabilities: AssessmentSnapshot["probabilities"]): string {
  return [
    formatProbabilityPercentExact(probabilities.p_5d),
    formatProbabilityPercentExact(probabilities.p_20d),
    formatProbabilityPercentExact(probabilities.p_60d)
  ].join(" / ");
}

function probabilitySnapshotDetail(assessment: AssessmentSnapshot): string {
  const allZero =
    assessment.probabilities.p_5d === 0 &&
    assessment.probabilities.p_20d === 0 &&
    assessment.probabilities.p_60d === 0;
  const currentScope = `当前线上 ${formatDate(assessment.as_of_date)} · 5d / 20d / 60d`;
  if (allZero) {
    return `${currentScope}；三个期限均为精确 0，需要先检查正式概率包、关键观测日期和 release 状态。`;
  }
  return `${currentScope}；历史/候选旧快照可能保留 0 值，不代表当前线上结论。`;
}

function actionEvidenceScore(assessment: AssessmentSnapshot): number {
  return assessment.action_evidence?.score ?? assessment.conviction_score;
}

function actionEvidenceStatus(score: number): string {
  if (score >= 0.82) {
    return "强升级证据";
  }
  if (score >= 0.68) {
    return "可升级证据";
  }
  if (score >= 0.42) {
    return "接近观察线";
  }
  if (score >= 0.18) {
    return "初步观察证据";
  }
  return "仅数据底座";
}

function actionEvidenceBreakdownCopy(assessment: AssessmentSnapshot): string {
  const evidence = assessment.action_evidence;
  if (!evidence) {
    return `动作升级证据分 ${formatPercent(actionEvidenceScore(assessment))}，当前缺少后端拆解，只能作为过渡动作证据。`;
  }

  const breadthCopy =
    evidence.breadth_component <= 0
      ? "风险广度尚未贡献"
      : `风险广度贡献 ${formatPercent(evidence.breadth_component)}`;
  const riskPressureComponent = evidence.risk_pressure_component ?? 0;
  const riskPressureCopy =
    riskPressureComponent <= 0
      ? "整体/结构/触发压力尚未贡献"
      : `整体/结构/触发压力贡献 ${formatPercent(riskPressureComponent)}`;
  const agreementCopy = evidence.structural_trigger_agreement
    ? `结构/触发共振贡献 ${formatPercent(evidence.agreement_component)}`
    : "结构/触发未共振，未给共振加分";

  return `动作升级证据分 ${formatPercent(evidence.score)} = 数据可信底座 ${formatPercent(evidence.data_quality_component)} + ${breadthCopy} + ${riskPressureCopy} + ${agreementCopy}。`;
}

function actionEvidenceHint(assessment: AssessmentSnapshot): string {
  const evidence = assessment.action_evidence;
  if (!evidence) {
    return `${actionEvidenceBreakdownCopy(assessment)} 这不是模型结论置信概率，而是当前证据是否足以升级仓位动作。`;
  }

  return [
    actionEvidenceBreakdownCopy(assessment),
    `当前状态为 ${actionEvidenceStatus(evidence.score)}。`,
    "这不是模型结论置信概率，也不是危机发生概率；危机概率看 5/20/60 天三项。",
    "如果风险广度没有打开、整体/结构/触发压力没有抬升，它会停在低位；含义是“数据可用，但还不足以升级仓位动作”。"
  ].join(" ");
}

function decisionReliabilityLabel(assessment: AssessmentSnapshot): string {
  if (assessment.runtime.demo_mode) {
    return "演示数据";
  }
  if (assessment.method.release_status === "degraded") {
    return "已降级";
  }
  if (assessment.data_trust.coverage_score >= 0.9 && !assessment.runtime.stale_warning) {
    return qualityDetailLabel(assessment.data_trust.quality_grade);
  }
  if (assessment.data_trust.coverage_score >= 0.75) {
    return "可用但需复核";
  }
  return "低覆盖复核";
}

function decisionReliabilityHint(assessment: AssessmentSnapshot): string {
  const probabilityMode = describeProbabilityMode(assessment.method);
  const releaseHealth = describeReleaseHealth(assessment.method.release_status);
  const latestDataDate =
    assessment.runtime.latest_key_indicator_at ?? assessment.runtime.latest_observation_at ?? "无最新日期";
  const staleCopy = assessment.runtime.stale_warning
    ? `存在滞后告警：${assessment.runtime.stale_warning}`
    : "关键数据新鲜度未触发滞后告警。";

  return [
    `结论可靠性看数据覆盖、模型服务状态和关键数据日期，不看动作升级证据分。`,
    `当前覆盖 ${formatPercent(assessment.data_trust.coverage_score)}，模型层 ${probabilityMode.label}，服务 ${releaseHealth}，最新关键数据 ${latestDataDate}。`,
    staleCopy
  ].join(" ");
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

function formatActionProbability(value: number, actionabilityEnabled: boolean): string {
  if (value === 0) {
    return actionabilityEnabled ? "0%" : "未触发";
  }
  return formatProbabilityPercent(value);
}

function formatActionCurrentValue(value: number, actionabilityEnabled: boolean): string {
  if (value === 0 && !actionabilityEnabled) {
    return "当前未触发；过渡动作层没有形成可执行动作信号。";
  }
  return `当前值 ${formatPercentPrecise(value)}。`;
}

function formatActionDetailValue(label: string, value: number, actionabilityEnabled: boolean): string {
  if (value === 0 && !actionabilityEnabled) {
    return `${label} 未触发`;
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
      label: "当前概率快照",
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
  return [
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
      label: "数据覆盖",
      value: formatPercent(assessment.data_trust.coverage_score),
      hint: "衡量当前免费数据源覆盖度；这个才更接近数据可信程度。"
    },
    {
      label: "风险强度",
      value: formatNumber(assessment.scores.overall_score),
      hint: "0-100 压力位置分，不等于危机概率。"
    }
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

export function buildSignalLayerRows(
  assessment: AssessmentSnapshot,
  method: AssessmentMethodResponse,
  posture: PostureGuidance
): DecisionSignalLayerRowModel[] {
  const actionabilitySource = actionSourceSummary(assessment).detail;
  const actionEvidence = assessment.action_evidence;
  const actionEvidenceDetail = actionEvidence
    ? `${actionEvidenceBreakdownCopy(assessment)} 它不是模型结论置信概率；结论可靠性请看数据覆盖、模型服务状态和关键数据日期。`
    : `${actionEvidenceBreakdownCopy(assessment)} 它不是模型结论置信概率；结论可靠性请看数据覆盖、模型服务状态和关键数据日期。`;
  const priorDetail = probabilityDisplayNote(assessment);
  const priorThresholdSummary = `当前进入线：准备 ${formatPercent(method.runtime_thresholds.prepare_p60d)} / 对冲 ${formatPercent(method.runtime_thresholds.hedge_p20d)} / 防守 ${formatPercent(method.runtime_thresholds.defend_p5d)}`;

  return [
    {
      id: "prior",
      title: "危机先验",
      description: "先看未来 5d / 20d / 60d 进入风险窗口的概率，回答“离风险还有多远”。",
      value: `${formatProbabilityPercentExact(assessment.probabilities.p_5d)} / ${formatProbabilityPercentExact(assessment.probabilities.p_20d)} / ${formatProbabilityPercentExact(assessment.probabilities.p_60d)}`,
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
      id: "action-evidence",
      title: "动作升级证据",
      description: "看当前证据是否足以把仓位动作从观察推向准备、对冲或防守；它不是模型结论置信概率。",
      value: actionEvidenceStatus(actionEvidenceScore(assessment)),
      detail: actionEvidenceDetail
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

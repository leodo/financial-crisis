import { formatPercent } from "../../format";
import type { AssessmentSnapshot } from "../../types";
import { describeProbabilityMode, describeReleaseHealth } from "./logic";
import { currentMvpRiskState } from "./mvpRiskState";
import { probabilityDiagnosticAnomalyHorizons } from "./probabilityDiagnostics";

function clamp01(value: number): number {
  return Math.max(0, Math.min(1, value));
}

function modelReliabilityComponent(assessment: AssessmentSnapshot): number {
  const auditOnly =
    currentMvpRiskState(assessment).probability_input_status === "audit_only" ||
    probabilityDiagnosticAnomalyHorizons(assessment).length > 0;
  if (assessment.runtime.demo_mode) {
    return 0.1;
  }
  if (assessment.method.release_status === "degraded") {
    return 0.25;
  }
  if (auditOnly) {
    return 0.35;
  }
  if (assessment.method.release_status === "healthy") {
    return 0.9;
  }
  return 0.65;
}

function freshnessReliabilityComponent(assessment: AssessmentSnapshot): number {
  if (assessment.runtime.stale_warning) {
    return 0.35;
  }
  const businessLag =
    assessment.runtime.latest_key_indicator_lag_business_days ??
    assessment.runtime.latest_observation_lag_business_days;
  if (businessLag === null || businessLag === undefined) {
    return 0.55;
  }
  if (businessLag <= 2) {
    return 1;
  }
  if (businessLag <= 5) {
    return 0.75;
  }
  if (businessLag <= 10) {
    return 0.45;
  }
  return 0.25;
}

export function decisionModelReliabilityLabel(assessment: AssessmentSnapshot): string {
  const score = modelReliabilityComponent(assessment);
  const auditOnly =
    currentMvpRiskState(assessment).probability_input_status === "audit_only" ||
    probabilityDiagnosticAnomalyHorizons(assessment).length > 0;
  if (assessment.runtime.demo_mode) {
    return `演示 ${formatPercent(score)}`;
  }
  if (assessment.method.release_status === "degraded") {
    return `降级 ${formatPercent(score)}`;
  }
  if (auditOnly) {
    return `待审计 ${formatPercent(score)}`;
  }
  if (assessment.method.release_status === "healthy") {
    return `健康 ${formatPercent(score)}`;
  }
  return `需复核 ${formatPercent(score)}`;
}

export function decisionModelReliabilityHint(assessment: AssessmentSnapshot): string {
  const probabilityMode = describeProbabilityMode(assessment.method);
  const releaseHealth = describeReleaseHealth(assessment.method.release_status);
  const auditOnly =
    currentMvpRiskState(assessment).probability_input_status === "audit_only" ||
    probabilityDiagnosticAnomalyHorizons(assessment).length > 0;
  return [
    `模型层当前是 ${probabilityMode.label}，服务状态 ${releaseHealth}。`,
    auditOnly
      ? "正式概率已经被 MVP 降级为审计读数，不能把低概率解释成风险已经远离。"
      : "正式概率可以作为主输入之一，但仍需要事件确认、数据新鲜度和历史类比共同支持。",
    `模型可信度组件当前为 ${formatPercent(modelReliabilityComponent(assessment))}，它只说明模型读数可用程度，不是危机概率。`
  ].join(" ");
}

export function decisionFreshnessReliabilityLabel(assessment: AssessmentSnapshot): string {
  const score = freshnessReliabilityComponent(assessment);
  if (assessment.runtime.stale_warning) {
    return `滞后 ${formatPercent(score)}`;
  }
  if (score >= 0.95) {
    return `新鲜 ${formatPercent(score)}`;
  }
  if (score >= 0.7) {
    return `可用 ${formatPercent(score)}`;
  }
  if (score >= 0.45) {
    return `需复核 ${formatPercent(score)}`;
  }
  return `陈旧 ${formatPercent(score)}`;
}

export function decisionFreshnessReliabilityHint(assessment: AssessmentSnapshot): string {
  const latestDataDate =
    assessment.runtime.latest_key_indicator_at ?? assessment.runtime.latest_observation_at ?? "无最新日期";
  const businessLag =
    assessment.runtime.latest_key_indicator_lag_business_days ??
    assessment.runtime.latest_observation_lag_business_days;
  const lagCopy =
    businessLag === null || businessLag === undefined
      ? "当前没有可用工作日滞后信息。"
      : `关键数据工作日滞后约 ${businessLag} 天。`;
  return [
    `最新关键数据日期 ${latestDataDate}，${lagCopy}`,
    assessment.runtime.stale_warning
      ? `滞后告警：${assessment.runtime.stale_warning}`
      : "关键数据新鲜度未触发滞后告警。",
    `数据新鲜度组件当前为 ${formatPercent(freshnessReliabilityComponent(assessment))}，它和模型可信度分开计算。`
  ].join(" ");
}

function historicalAnalogComponent(assessment: AssessmentSnapshot): number {
  const maxSimilarity = Math.max(
    0,
    ...assessment.historical_analogs.map((analog) => analog.similarity_score)
  );
  return clamp01(maxSimilarity / 100);
}

function rawReliabilityScore(assessment: AssessmentSnapshot): number {
  const dataCoverage = clamp01(assessment.data_trust.coverage_score);
  const model = modelReliabilityComponent(assessment);
  const eventConfirmation = clamp01(assessment.event_assessment.confirmation_score / 100);
  const analog = historicalAnalogComponent(assessment);
  const freshness = freshnessReliabilityComponent(assessment);
  return (
    dataCoverage * 0.35 +
    model * 0.25 +
    eventConfirmation * 0.2 +
    analog * 0.1 +
    freshness * 0.1
  );
}

export function decisionReliabilityScore(assessment: AssessmentSnapshot): number {
  const auditOnly = currentMvpRiskState(assessment).probability_input_status === "audit_only";
  const score = rawReliabilityScore(assessment);
  if (assessment.runtime.demo_mode) {
    return Math.min(score, 0.4);
  }
  if (assessment.method.release_status === "degraded") {
    return Math.min(score, 0.5);
  }
  if (auditOnly) {
    return Math.min(score, 0.62);
  }
  if (assessment.runtime.stale_warning) {
    return Math.min(score, 0.65);
  }
  return score;
}

export function decisionReliabilityLabel(assessment: AssessmentSnapshot): string {
  const score = decisionReliabilityScore(assessment);
  if (assessment.runtime.demo_mode) {
    return `演示 ${formatPercent(score)}`;
  }
  if (assessment.method.release_status === "degraded") {
    return `降级 ${formatPercent(score)}`;
  }
  if (currentMvpRiskState(assessment).probability_input_status === "audit_only") {
    return `审计中 ${formatPercent(score)}`;
  }
  if (score >= 0.8) {
    return `高可信 ${formatPercent(score)}`;
  }
  if (score >= 0.65) {
    return `可用 ${formatPercent(score)}`;
  }
  if (score >= 0.45) {
    return `需复核 ${formatPercent(score)}`;
  }
  return `低可信 ${formatPercent(score)}`;
}

export function decisionReliabilityHint(assessment: AssessmentSnapshot): string {
  const probabilityMode = describeProbabilityMode(assessment.method);
  const releaseHealth = describeReleaseHealth(assessment.method.release_status);
  const mvpState = currentMvpRiskState(assessment);
  const latestDataDate =
    assessment.runtime.latest_key_indicator_at ?? assessment.runtime.latest_observation_at ?? "无最新日期";
  const staleCopy = assessment.runtime.stale_warning
    ? `存在滞后告警：${assessment.runtime.stale_warning}`
    : "关键数据新鲜度未触发滞后告警。";
  const maxAnalog = Math.max(
    0,
    ...assessment.historical_analogs.map((analog) => analog.similarity_score)
  );
  const capCopy =
    mvpState.probability_input_status === "audit_only"
      ? "正式概率当前待审计，可靠性分数会被封顶，不能解释成模型结论已经很有把握。"
      : "正式概率当前可作为主输入之一，但仍需结合事件确认和数据新鲜度。";

  return [
    "结论可靠性不是危机发生概率，也不是动作升级证据。",
    "它按数据覆盖 35%、模型状态 25%、事件确认 20%、历史相似度 10%、关键数据新鲜度 10% 汇总；页面已把模型可信度和数据新鲜度拆开显示。",
    `当前覆盖 ${formatPercent(assessment.data_trust.coverage_score)}，模型层 ${probabilityMode.label}，服务 ${releaseHealth}，MVP 状态 ${mvpState.label}。`,
    `事件确认 ${formatPercent(assessment.event_assessment.confirmation_score / 100)}，最高历史相似度 ${formatPercent(maxAnalog / 100)}，最新关键数据 ${latestDataDate}。`,
    staleCopy,
    capCopy
  ].join(" ");
}

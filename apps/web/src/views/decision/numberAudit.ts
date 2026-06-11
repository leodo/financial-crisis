import {
  dataModeLabel,
  formatDate,
  formatDateTime,
  formatDateTimeWithLocal,
  formatNumber,
  formatPercent,
  formatPercentPrecise,
  formatProbabilityBasisPoints,
  formatProbabilityDecimal,
  formatProbabilityPercentExact,
  formatSignedNumber,
  eventSignalListEmptyText,
  eventSignalListLabel,
  eventStateLabel,
  freshnessLabel,
  jpyStateLabel,
  levelLabel,
  sourceLabel,
  unitLabel
} from "../../format";
import type { AssessmentSnapshot } from "../../types";
import {
  keyIndicatorLineageCopy,
  keyIndicatorSourceTimingCopy,
  sourceAccessTag
} from "./dataSourceReliability";
import {
  decisionFreshnessReliabilityHint,
  decisionFreshnessReliabilityLabel,
  decisionModelReliabilityHint,
  decisionModelReliabilityLabel,
  decisionReliabilityHint,
  decisionReliabilityLabel
} from "./decisionReliability";
import {
  currentMvpRiskState,
  mvpProbabilityInputIsAuditOnly,
  mvpRiskStateDisplayCopy,
  mvpRiskStateDisplayLabel
} from "./mvpRiskState";
import { probabilityDiagnosticAnomalyHorizons } from "./probabilityDiagnostics";
import {
  actionEvidenceHint,
  actionEvidenceScore,
  actionEvidenceStatus,
  hasRuntimeProbabilityOverride,
  probabilityModelFinalSnapshotValue,
  probabilityRuntimeReferenceNote
} from "./signalLayerBuilders";

export interface DecisionNumberAuditRow {
  id: string;
  title: string;
  detail: string;
  meta: string;
  note: string;
}

export function buildNumberAuditRows(assessment: AssessmentSnapshot): DecisionNumberAuditRow[] {
  const mvpState = currentMvpRiskState(assessment);
  const auditOnly = mvpProbabilityInputIsAuditOnly(assessment);
  const anomalyHorizons = probabilityDiagnosticAnomalyHorizons(assessment);
  const runtimeOverride = hasRuntimeProbabilityOverride(assessment);
  const modelFinalSnapshot = probabilityModelFinalSnapshotValue(assessment);
  const runtimeReferenceNote = probabilityRuntimeReferenceNote(assessment);
  const usdJpy = assessment.key_indicators.find(
    (indicator) => indicator.indicator_id === "us_external_usdjpy_level"
  );

  return [
    {
      id: "mvp-state",
      title: `MVP 风险状态 · ${mvpRiskStateDisplayLabel(mvpState.label)}`,
      detail: mvpRiskStateDisplayCopy(mvpState.summary),
      meta: "主结论",
      note: `${auditTrailCopy(
        "API mvp_risk_state / MVP 规则层",
        "风险状态",
        assessment.as_of_date,
        mvpState.probability_input_status === "reference_only" ? "正式概率仅作参考" : "正式概率可参与"
      )} ${compactListCopy(
        "阻断项",
        mvpState.blockers,
        "当前没有阻断项；仍需按数据新鲜度、事件确认和历史类比复核。"
      )}`
    },
    {
      id: "risk-score-snapshot",
      title: "规则层风险分数 · MVP 主输入",
      detail: `总风险 ${formatNumber(assessment.scores.overall_score)} / 结构 ${formatNumber(
        assessment.scores.structural_score
      )} / 触发 ${formatNumber(assessment.scores.trigger_score)} / 外部 ${formatNumber(
        assessment.scores.external_shock_score
      )}`,
      meta: "0-100 分",
      note: `${auditTrailCopy(
        "API scores / scoring engine",
        "0-100 分",
        assessment.as_of_date,
        "规则层主结论输入"
      )} 风险分数反映压力位置和证据共振，不是危机发生概率；正式概率处于参考态时，MVP 主结论优先按这组规则层分数、关键数据新鲜度和事件确认解释。`
    },
    {
      id: "event-confirmation",
      title: `事件确认 · ${eventStateLabel(assessment.event_assessment.state)}`,
      detail: `确认分 ${formatNumber(
        assessment.event_assessment.confirmation_score
      )} / 近期事件 ${assessment.event_assessment.recent_event_count} 条`,
      meta: "0-100 分",
      note: `${auditTrailCopy(
        "API event_assessment / event rules",
        "0-100 分 / 事件条数",
        assessment.as_of_date,
        "MVP 辅助确认输入"
      )} ${mvpRiskStateDisplayCopy(
        assessment.event_assessment.summary
      )} 事件确认分衡量外部事件是否与市场压力共振，不是危机发生概率，也不是自动动作指令。${compactListCopy(
        eventSignalListLabel(assessment.event_assessment.state),
        eventSignalAuditItems(assessment),
        eventSignalListEmptyText(assessment.event_assessment.state)
      )} ${compactListCopy(
        "待补缺口",
        assessment.event_assessment.pending_gaps,
        "当前没有额外待补缺口。"
      )}`
    },
    {
      id: "jpy-carry",
      title: `日元套息放大器 · ${jpyStateLabel(assessment.jpy_carry.state)}`,
      detail: `放大器 ${formatNumber(assessment.jpy_carry.score)} / 融资压力 ${formatNumber(
        assessment.jpy_carry.funding_pressure_score
      )} / VIX 联动 ${formatNumber(assessment.jpy_carry.vix_coupling_score)}`,
      meta: "0-100 分",
      note: `${auditTrailCopy(
        "API jpy_carry / key indicators",
        "0-100 分 / JPY per USD / 利率",
        assessment.as_of_date,
        "外部放大器辅助输入"
      )} ${mvpRiskStateDisplayCopy(
        assessment.jpy_carry.reason
      )} JPY carry 分数衡量日元融资环境是否可能放大美国风险资产同步回撤，不是在预测日本危机，也不是正式危机概率。USDJPY ${formatNumber(
        assessment.jpy_carry.usdjpy_level
      )}，5d 变化 ${formatNumber(assessment.jpy_carry.change_5d)}，美日短端利差 ${formatSignedNumber(
        assessment.jpy_carry.us_jp_short_rate_diff,
        2,
        "%"
      )}，20d 日收益波动 ${formatPercentPrecise(assessment.jpy_carry.realized_vol_20d)}。`
    },
    {
      id: "decision-reliability",
      title: `结论可信度 · ${decisionReliabilityLabel(assessment)}`,
      detail: `模型 ${decisionModelReliabilityLabel(assessment)} / 数据 ${decisionFreshnessReliabilityLabel(
        assessment
      )} / 事件 ${formatPercent(assessment.event_assessment.confirmation_score / 100)}`,
      meta: "可靠性，不是概率",
      note: `${auditTrailCopy(
        "frontend decisionReliability / API data_trust + method + event_assessment",
        "0-100% 组件分",
        assessment.as_of_date,
        auditOnly ? "reference_only 封顶" : "按组件合成"
      )} ${decisionReliabilityHint(assessment)} ${decisionModelReliabilityHint(
        assessment
      )} ${decisionFreshnessReliabilityHint(assessment)}`
    },
    {
      id: "action-evidence",
      title: `动作升级证据 · ${actionEvidenceStatus(actionEvidenceScore(assessment))}`,
      detail: `总分 ${formatPercent(actionEvidenceScore(assessment))} / 数据底座 ${formatPercent(
        assessment.action_evidence?.data_quality_component ?? 0
      )} / 风险广度 ${formatPercent(
        assessment.action_evidence?.breadth_component ?? 0
      )} / 压力 ${formatPercent(assessment.action_evidence?.risk_pressure_component ?? 0)}`,
      meta: "辅助，不是概率",
      note: `${auditTrailCopy(
        "API action_evidence / scoring engine",
        "0-100% 证据分",
        assessment.as_of_date,
        "动作升级辅助输入"
      )} ${actionEvidenceHint(assessment)}`
    },
    {
      id: "probability-snapshot",
      title:
        auditOnly || runtimeOverride
          ? "5/20/60 日运行口径概率 · 参考值"
          : "5/20/60 日正式概率 · 可用",
      detail: `5d ${formatProbabilityPercentExact(
        assessment.probabilities.p_5d
      )}（${formatProbabilityBasisPoints(
        assessment.probabilities.p_5d
      )}） / 20d ${formatProbabilityPercentExact(
        assessment.probabilities.p_20d
      )}（${formatProbabilityBasisPoints(
        assessment.probabilities.p_20d
      )}） / 60d ${formatProbabilityPercentExact(
        assessment.probabilities.p_60d
      )}（${formatProbabilityBasisPoints(assessment.probabilities.p_60d)}）`,
      meta: auditOnly ? "辅助参考" : "可参与判断",
      note: auditOnly
        ? `${auditTrailCopy(
            runtimeOverride ? "runtime reference probabilities" : "active release formal probabilities",
            "概率 / bp / 接口小数",
            assessment.as_of_date,
            "参考值"
          )} 页面当前值 ${formatProbabilityDecimal(
            assessment.probabilities.p_5d
          )} / ${formatProbabilityDecimal(
            assessment.probabilities.p_20d
          )} / ${formatProbabilityDecimal(
            assessment.probabilities.p_60d
          )}。${runtimeReferenceNote ? `${runtimeReferenceNote} ` : ""}异常期限 ${anomalyHorizons.join(
            " / "
          ) || "已由 MVP 降为参考输入"}；这些小概率当前作为参考值保留，不单独决定风险时距、减仓或对冲结论。`
        : `${auditTrailCopy(
            "active release formal probabilities",
            "概率 / bp / 接口小数",
            assessment.as_of_date,
            "可参与判断"
          )} ${
            runtimeOverride
              ? `模型原始输出 ${modelFinalSnapshot}；页面当前值 ${formatProbabilityDecimal(
                  assessment.probabilities.p_5d
                )} / ${formatProbabilityDecimal(
                  assessment.probabilities.p_20d
                )} / ${formatProbabilityDecimal(
                  assessment.probabilities.p_60d
                )} 为运行口径参考值。`
              : "正式概率可作为风险时距输入之一，但仍需要事件确认、数据新鲜度和历史类比共同支持。"
          }`
    },
    {
      id: "usdjpy",
      title: `USDJPY · ${usdJpy ? sourceLabel(usdJpy.source_id) : "缺失"}`,
      detail: usdJpy
        ? `${formatNumber(usdJpy.latest_value)} ${unitLabel(usdJpy.unit)} · 日期 ${
            usdJpy.latest_as_of_date ? formatDate(usdJpy.latest_as_of_date) : "—"
          } · ${freshnessLabel(usdJpy.status)}`
        : "当前接口没有返回 USDJPY 关键指标。",
      meta: usdJpy ? sourceAccessTag(usdJpy.source_id) : "缺失",
      note: usdJpy
        ? `${keyIndicatorSourceTimingCopy(usdJpy)} dataset=${usdJpy.dataset_id ?? "—"}；${
            usdJpy.note
          } ${keyIndicatorLineageCopy(usdJpy.lineage, { includeEvidenceLevel: true })}`
        : "缺少 USDJPY 时，日元套息风险只能降级解释，不能给出高置信结论。"
    },
    {
      id: "freshness",
      title: `数据新鲜度 · ${dataModeLabel(assessment.runtime.data_mode)}`,
      detail: `最新关键观测 ${
        assessment.runtime.latest_key_indicator_at
          ? formatDate(assessment.runtime.latest_key_indicator_at)
          : "—"
      }；本次生成 ${formatDateTimeWithLocal(assessment.runtime.generated_at)}`,
      meta: assessment.runtime.stale_warning ? "需降级解释" : "可用",
      note:
        assessment.runtime.stale_warning ??
        `${auditTrailCopy(
          "runtime freshness guard",
          "日期 / 滞后天数 / 覆盖率",
          assessment.runtime.generated_at,
          assessment.runtime.stale_warning ? "需降级解释" : "可用"
        )} 关键指标自然日滞后 ${assessment.runtime.latest_key_indicator_lag_days ?? "—"} 天，工作日滞后 ${
          assessment.runtime.latest_key_indicator_lag_business_days ?? "—"
        } 天；关键指标覆盖 ${formatPercent(assessment.data_trust.coverage_score)}，覆盖等级 ${
          assessment.data_trust.quality_grade.toUpperCase()
        }；这不等同于全部免费源都健康，源状态请看顶部降级提示和“数据可信度”页。`
    },
    {
      id: "position-guidance",
      title: "持仓动作建议 · 系统预算",
      detail: `风险资产目标 ${formatPercentPrecise(
        assessment.position_guidance.target_equity_exposure_pct / 100
      )}，现金目标 ${formatPercentPrecise(
        assessment.position_guidance.target_cash_pct / 100
      )}，对冲覆盖 ${formatPercentPrecise(
        assessment.position_guidance.hedge_ratio_pct / 100
      )}，期权保护 ${formatPercentPrecise(
        assessment.position_guidance.option_overlay_pct / 100
      )}`,
      meta: assessment.position_guidance.governance.auto_execution_allowed ? "需复核" : "非自动交易",
      note: `${auditTrailCopy(
        "API position_guidance",
        "百分比预算",
        assessment.as_of_date,
        assessment.position_guidance.governance.auto_execution_allowed ? "需人工复核" : "禁止自动执行"
      )} ${assessment.position_guidance.action_summary} 该建议只给仓位预算边界，执行前仍需人工确认流动性、税务、账户和持仓结构。`
    }
  ];
}

function auditTrailCopy(source: string, unit: string, date: string, status: string): string {
  return `来源：${source}；单位：${unit}；日期：${formatDate(date)}；状态：${status}。`;
}

function compactListCopy(label: string, items: string[], emptyText: string): string {
  if (items.length === 0) {
    return emptyText;
  }
  return `${label}：${items.map((item) => mvpRiskStateDisplayCopy(item)).join("；")}`;
}

function eventSignalAuditItems(assessment: AssessmentSnapshot): string[] {
  if (assessment.event_assessment.recent_events.length > 0) {
    return assessment.event_assessment.recent_events.map(
      (event) =>
        `${formatDate(event.triggered_as_of_date)} · ${levelLabel(event.level)} · ${event.trigger_reason}`
    );
  }
  return assessment.event_assessment.confirmed_signals;
}

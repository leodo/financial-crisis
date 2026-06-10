import {
  dataModeLabel,
  formatDate,
  formatDateTime,
  formatNumber,
  formatPercent,
  formatPercentPrecise,
  formatProbabilityBasisPoints,
  formatProbabilityDecimal,
  formatProbabilityPercentExact,
  freshnessLabel,
  humanizeNarrativeCopy,
  sourceLabel,
  unitLabel
} from "../../format";
import type { AssessmentSnapshot } from "../../types";
import { currentMvpRiskState } from "./mvpRiskState";
import { probabilityDiagnosticAnomalyHorizons } from "./probabilityDiagnostics";

export interface DecisionNumberAuditRow {
  id: string;
  title: string;
  detail: string;
  meta: string;
  note: string;
}

export function buildNumberAuditRows(assessment: AssessmentSnapshot): DecisionNumberAuditRow[] {
  const mvpState = currentMvpRiskState(assessment);
  const auditOnly = mvpState.probability_input_status === "audit_only";
  const anomalyHorizons = probabilityDiagnosticAnomalyHorizons(assessment);
  const usdJpy = assessment.key_indicators.find(
    (indicator) => indicator.indicator_id === "us_external_usdjpy_level"
  );

  return [
    {
      id: "mvp-state",
      title: `MVP 风险状态 · ${mvpState.label}`,
      detail: mvpState.summary,
      meta: "主结论",
      note: compactListCopy(
        "阻断项",
        mvpState.blockers,
        "当前没有阻断项；仍需按数据新鲜度、事件确认和历史类比复核。"
      )
    },
    {
      id: "probability-snapshot",
      title: auditOnly ? "5/20/60 日正式概率 · 待审计" : "5/20/60 日正式概率 · 可用",
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
      meta: auditOnly ? "不作主决策输入" : "可参与判断",
      note: auditOnly
        ? `接口原始值 ${formatProbabilityDecimal(
            assessment.probabilities.p_5d
          )} / ${formatProbabilityDecimal(
            assessment.probabilities.p_20d
          )} / ${formatProbabilityDecimal(
            assessment.probabilities.p_60d
          )}。异常期限 ${anomalyHorizons.join(
            " / "
          ) || "已由 MVP 降级"}；这些小概率只用于模型审计，不参与风险时距、减仓或对冲结论。`
        : "正式概率可作为风险时距输入之一，但仍需要事件确认、数据新鲜度和历史类比共同支持。"
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
        ? `${sourceTimingCopy(usdJpy)} dataset=${usdJpy.dataset_id ?? "—"}；${humanizeNarrativeCopy(
            usdJpy.note
          )}${lineageCopy(usdJpy.lineage)}`
        : "缺少 USDJPY 时，日元套息风险只能降级解释，不能给出高置信结论。"
    },
    {
      id: "freshness",
      title: `数据新鲜度 · ${dataModeLabel(assessment.runtime.data_mode)}`,
      detail: `最新关键观测 ${
        assessment.runtime.latest_key_indicator_at
          ? formatDate(assessment.runtime.latest_key_indicator_at)
          : "—"
      }；本次生成 ${formatDateTime(assessment.runtime.generated_at)}`,
      meta: assessment.runtime.stale_warning ? "需降级解释" : "可用",
      note:
        assessment.runtime.stale_warning ??
        `关键指标自然日滞后 ${assessment.runtime.latest_key_indicator_lag_days ?? "—"} 天，工作日滞后 ${
          assessment.runtime.latest_key_indicator_lag_business_days ?? "—"
        } 天；数据覆盖 ${formatPercent(assessment.data_trust.coverage_score)}，质量等级 ${
          assessment.data_trust.quality_grade.toUpperCase()
        }。`
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
      note: `${assessment.position_guidance.action_summary} 该建议只给仓位预算边界，执行前仍需人工确认流动性、税务、账户和持仓结构。`
    }
  ];
}

function compactListCopy(label: string, items: string[], emptyText: string): string {
  if (items.length === 0) {
    return emptyText;
  }
  return `${label}：${items.map(humanizeNarrativeCopy).join("；")}`;
}

function sourceAccessTag(sourceId: string | null): string {
  const officialFreeSources = new Set(["fred", "treasury", "world_bank", "boj", "sec_edgar"]);
  if (!sourceId) {
    return "来源缺失";
  }
  if (officialFreeSources.has(sourceId)) {
    return "免费官方";
  }
  if (sourceId === "gdelt") {
    return "免费公开";
  }
  return "需复核授权";
}

function sourceTimingCopy(indicator: AssessmentSnapshot["key_indicators"][number]): string {
  if (indicator.indicator_id === "us_external_usdjpy_level" && indicator.source_id === "boj") {
    return "BOJ 免费官方日频点位，适合日频风险评估，不等同盘中实时价。";
  }
  if (indicator.source_id === "fred") {
    return "FRED 免费公开图表/序列数据，适合日频或低频风险评估。";
  }
  return `${sourceLabel(indicator.source_id)} 来源用于当前关键指标。`;
}

function lineageCopy(
  lineage: AssessmentSnapshot["key_indicators"][number]["lineage"]
): string {
  if (!lineage) {
    return "";
  }
  const fetchedAt = lineage.fetched_at ? ` 抓取 ${formatDateTime(lineage.fetched_at)}。` : "";
  const raw = lineage.raw_payload_id ? ` raw=${lineage.raw_payload_id.slice(0, 8)}。` : "";
  return ` 追溯级别 ${lineage.evidence_level}；${humanizeNarrativeCopy(
    lineage.note
  )}${fetchedAt}${raw}`;
}

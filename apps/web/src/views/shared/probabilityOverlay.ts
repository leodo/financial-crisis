import {
  formatPercent,
  formatProbabilityPercentExact,
  formatSignedNumber
} from "../../format";
import type { AssessmentSnapshot } from "../../types";
import type { DetailRowItem, MetricItem } from "./panelHelpers";

export interface ProbabilityOverlayAuditRow {
  id: string;
  horizonLabel: string;
  familyLabel: string;
  scenarioSummary: string;
  splitSummary: string;
  gateSummary: string;
  note: string;
}

export function overlayHorizonLabel(horizonDays: number) {
  switch (horizonDays) {
    case 5:
      return "5 日窗口";
    case 20:
      return "20 日窗口";
    case 60:
      return "60 日窗口";
    default:
      return `${horizonDays} 日窗口`;
  }
}

export function overlayFamilyLabel(familyId: string) {
  switch (familyId) {
    case "systemic_credit":
      return "系统信用";
    case "mixed_systemic":
      return "混合系统性";
    case "rate_shock":
      return "利率冲击";
    case "acute_liquidity":
      return "急性流动性";
    case "jpy_carry":
      return "日元套息";
    default:
      return familyId;
  }
}

export function buildProbabilityOverlayViewModel(assessment: AssessmentSnapshot): {
  overlayHeadlineMetrics: MetricItem[];
  overlayHorizonRows: DetailRowItem[];
  overlayAuditRows: ProbabilityOverlayAuditRow[];
  configuredOverlayCount: number;
  activeContributionCount: number;
  auditedFamilyCount: number;
} {
  const overlayDiagnostics = (assessment.probability_diagnostics?.horizon_overlays ?? [])
    .slice()
    .sort((left, right) => left.horizon_days - right.horizon_days);
  const configuredOverlayCount = overlayDiagnostics.reduce(
    (total, horizon) => total + horizon.configured_overlay_count,
    0
  );
  const activeContributionCount = overlayDiagnostics.reduce(
    (total, horizon) => total + horizon.contributions.length,
    0
  );
  const auditedFamilyCount = overlayDiagnostics.reduce(
    (total, horizon) => total + horizon.overlay_audits.length,
    0
  );

  const overlayHeadlineMetrics: MetricItem[] = [
    {
      label: "诊断窗口",
      value: `${overlayDiagnostics.length}`
    },
    {
      label: "已训练 overlay",
      value: `${configuredOverlayCount}`
    },
    {
      label: "当前参与头数",
      value: `${activeContributionCount}`
    },
    {
      label: "审计 family",
      value: `${auditedFamilyCount}`
    }
  ];

  const overlayHorizonRows: DetailRowItem[] = overlayDiagnostics.map((horizon) => {
    const monotonicLift = horizon.monotonic_lift ?? 0;
    const runtimeFinalProbability =
      horizon.runtime_final_probability ?? horizon.final_probability;
    const runtimeContributionSummary =
      horizon.contributions.length === 0
        ? "当前快照没有 family overlay 直接改写这个窗口的最终概率。"
        : horizon.contributions
            .map(
              (contribution) =>
                `${overlayFamilyLabel(contribution.family_id)} ${formatSignedNumber(contribution.contribution * 100, 1, "%")}（gate ${formatPercent(contribution.gate, 0)}，blend ${formatPercent(contribution.blend, 0)}）`
            )
            .join("；");
    return {
      id: `overlay-${horizon.horizon_days}`,
      title: `${overlayHorizonLabel(horizon.horizon_days)} · ${horizon.configured_overlay_count > 0 ? `${horizon.configured_overlay_count} 个已挂载 overlay` : "当前仅保留审计元数据"}`,
      detail: `base raw ${formatProbabilityPercentExact(horizon.raw_probability)}，base calibrated ${formatProbabilityPercentExact(horizon.calibrated_probability)}，overlay final ${formatProbabilityPercentExact(horizon.final_probability)}，runtime final ${formatProbabilityPercentExact(runtimeFinalProbability)}。${monotonicLift > 0 ? `单调约束额外抬升 ${formatProbabilityPercentExact(monotonicLift)}。` : ""}${runtimeContributionSummary}`,
      meta: formatSignedNumber(
        monotonicLift > 0
          ? monotonicLift * 100
          : (horizon.final_probability - horizon.calibrated_probability) * 100,
        1,
        "%"
      ),
      note:
        monotonicLift > 0
          ? "当前 runtime 对这个窗口施加了跨 horizon 单调 gap 约束。"
          : horizon.overlay_audits.length > 0
          ? `训练审计覆盖 ${horizon.overlay_audits.length} 个 family。`
          : "当前没有 family-level 训练审计。"
    };
  });

  const overlayAuditRows: ProbabilityOverlayAuditRow[] = overlayDiagnostics.flatMap((horizon) =>
    horizon.overlay_audits.map((audit) => ({
      id: `${horizon.horizon_days}-${audit.family_id}`,
      horizonLabel: overlayHorizonLabel(horizon.horizon_days),
      familyLabel: overlayFamilyLabel(audit.family_id),
      scenarioSummary: `${audit.scenario_count} 个场景 / 正例 ${audit.positive_label_count}`,
      splitSummary: `${audit.train_row_count} / ${audit.calibration_row_count} / ${audit.evaluation_row_count}`,
      gateSummary: `${audit.train_gate_active_row_count} / ${audit.calibration_gate_active_row_count} / ${audit.evaluation_gate_active_row_count}`,
      note: audit.note
    }))
  );

  return {
    overlayHeadlineMetrics,
    overlayHorizonRows,
    overlayAuditRows,
    configuredOverlayCount,
    activeContributionCount,
    auditedFamilyCount
  };
}

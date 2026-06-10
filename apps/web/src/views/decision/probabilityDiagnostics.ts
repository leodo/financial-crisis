import { formatNumber, formatSignedNumber } from "../../format";
import type { AssessmentSnapshot, ProbabilityHorizonOverlayDiagnostics } from "../../types";

const USDJPY_HIGH_TAIL_SUPPRESSOR_FEATURE = "tail_pos__us_usdjpy_level__145";

export interface ProbabilityDiagnosticAnomaly {
  title: string;
  detail: string;
}

export function findProbabilityDiagnosticAnomaly(
  diagnostic?: ProbabilityHorizonOverlayDiagnostics
): ProbabilityDiagnosticAnomaly | null {
  const suppressor = diagnostic?.base_contributions?.find(
    (contribution) =>
      contribution.name === USDJPY_HIGH_TAIL_SUPPRESSOR_FEATURE &&
      contribution.raw_value > 0 &&
      contribution.contribution <= -1
  );

  if (!suppressor) {
    return null;
  }

  const usdJpyLevel = diagnostic?.base_contributions?.find(
    (contribution) => contribution.name === "us_usdjpy_level"
  )?.raw_value;
  const levelCopy =
    usdJpyLevel === undefined ? "USDJPY 高于 145" : `USDJPY ${formatNumber(usdJpyLevel)}`;

  return {
    title: "USDJPY 高位 tail 正在压低读数",
    detail: `${levelCopy} 时，高位 tail 特征对 ${diagnostic?.horizon_days ?? "当前"}d 概率贡献 ${formatSignedNumber(
      suppressor.contribution,
      2
    )}，方向和“日元套息/外部冲击风险升温”的解释冲突；这个正式概率应先按模型待审计读数处理。`
  };
}

export function probabilityDiagnosticAnomalyHorizons(
  assessment: Pick<AssessmentSnapshot, "probability_diagnostics">
): string[] {
  return assessment.probability_diagnostics.horizon_overlays
    .filter((diagnostic) => findProbabilityDiagnosticAnomaly(diagnostic) !== null)
    .map((diagnostic) => `${diagnostic.horizon_days}d`);
}

export function hasProbabilityDiagnosticAnomaly(
  assessment: Pick<AssessmentSnapshot, "probability_diagnostics">
): boolean {
  return probabilityDiagnosticAnomalyHorizons(assessment).length > 0;
}

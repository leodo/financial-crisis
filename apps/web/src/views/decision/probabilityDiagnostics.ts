import { formatNumber, formatSignedNumber } from "../../format";
import type { AssessmentSnapshot, ProbabilityHorizonOverlayDiagnostics } from "../../types";

const USDJPY_HIGH_TAIL_SUPPRESSOR_FEATURE = "tail_pos__us_usdjpy_level__145";

export interface ProbabilityDiagnosticAnomaly {
  title: string;
  detail: string;
}

export interface ProbabilityHorizonModelValues {
  p5d: number;
  p20d: number;
  p60d: number;
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
    )}，方向和“日元套息/外部冲击风险升温”的解释冲突；这个正式概率当前应先按参考值处理。`
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

export function probabilityModelFinalHorizonValues(
  assessment: Pick<AssessmentSnapshot, "probability_diagnostics">
): ProbabilityHorizonModelValues | null {
  const p5d = probabilityModelFinalForHorizon(assessment, 5);
  const p20d = probabilityModelFinalForHorizon(assessment, 20);
  const p60d = probabilityModelFinalForHorizon(assessment, 60);

  if (p5d === null || p20d === null || p60d === null) {
    return null;
  }

  return { p5d, p20d, p60d };
}

export function probabilityModelTwentyDayIsCold(
  assessment: Pick<AssessmentSnapshot, "probability_diagnostics">
): boolean {
  const values = probabilityModelFinalHorizonValues(assessment);
  if (!values) {
    return false;
  }

  return values.p20d > 0 && values.p20d < values.p5d * 0.25 && values.p20d < values.p60d * 0.25;
}

function probabilityModelFinalForHorizon(
  assessment: Pick<AssessmentSnapshot, "probability_diagnostics">,
  horizonDays: 5 | 20 | 60
): number | null {
  return (
    assessment.probability_diagnostics.horizon_overlays.find(
      (diagnostic) => diagnostic.horizon_days === horizonDays
    )?.final_probability ?? null
  );
}

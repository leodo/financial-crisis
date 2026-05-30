import type { QualityGrade, RiskLevel } from "./types";

export function levelLabel(level: RiskLevel): string {
  const labels: Record<RiskLevel, string> = {
    normal: "L0 Normal",
    watch: "L1 Watch",
    stress: "L2 Stress",
    warning: "L3 Warning",
    crisis: "L4 Crisis"
  };
  return labels[level];
}

export function levelClass(level: RiskLevel): string {
  return `level-${level}`;
}

export function qualityLabel(grade: QualityGrade): string {
  return grade.toUpperCase();
}

export function formatNumber(value: number | null | undefined, suffix = ""): string {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "—";
  }
  return `${value.toFixed(1)}${suffix}`;
}

export function formatDate(value: string | null | undefined): string {
  if (!value) {
    return "—";
  }
  return value.slice(0, 10);
}


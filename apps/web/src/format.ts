import type { QualityGrade, RiskLevel } from "./types";

export function levelLabel(level: RiskLevel): string {
  const labels: Record<RiskLevel, string> = {
    normal: "L0 正常",
    watch: "L1 观察",
    stress: "L2 压力",
    warning: "L3 预警",
    crisis: "L4 危机态"
  };
  return labels[level];
}

export function levelPlainText(level: RiskLevel): string {
  const labels: Record<RiskLevel, string> = {
    normal: "正常",
    watch: "观察",
    stress: "压力",
    warning: "预警",
    crisis: "危机态"
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

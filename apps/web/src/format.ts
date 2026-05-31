import type {
  BacktestSignalSource,
  DataMode,
  DecisionPosture,
  EventConfirmationState,
  FreshnessStatus,
  JpyCarryState,
  QualityGrade,
  RiskLevel,
  TimeToRiskBucket,
  UserRiskProfile
} from "./types";

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

export function postureLabel(posture: DecisionPosture): string {
  const labels: Record<DecisionPosture, string> = {
    normal: "正常观察",
    prepare: "提前准备",
    hedge: "保护性对冲",
    defend: "防守优先"
  };
  return labels[posture];
}

export function postureClass(posture: DecisionPosture): string {
  return `posture-${posture}`;
}

export function timeBucketLabel(bucket: TimeToRiskBucket): string {
  const labels: Record<TimeToRiskBucket, string> = {
    normal: "常态",
    months: "数月",
    weeks: "数周",
    now: "当下"
  };
  return labels[bucket];
}

export function jpyStateLabel(state: JpyCarryState): string {
  const labels: Record<JpyCarryState, string> = {
    quiet: "平稳",
    building: "积累中",
    stress: "高压",
    unwind: "平仓风险"
  };
  return labels[state];
}

export function qualityLabel(grade: QualityGrade): string {
  return grade.toUpperCase();
}

export function dataModeLabel(mode: DataMode): string {
  const labels: Record<DataMode, string> = {
    demo: "Demo",
    sqlite: "SQLite",
    postgres: "Postgres"
  };
  return labels[mode];
}

export function backtestSignalSourceLabel(source: BacktestSignalSource): string {
  const labels: Record<BacktestSignalSource, string> = {
    real_history: "真实历史",
    fallback_template: "模板参考"
  };
  return labels[source];
}

export function freshnessLabel(status: FreshnessStatus): string {
  const labels: Record<FreshnessStatus, string> = {
    fresh: "新鲜",
    delayed: "延迟",
    stale: "陈旧",
    missing: "缺失"
  };
  return labels[status];
}

export function eventStateLabel(state: EventConfirmationState): string {
  const labels: Record<EventConfirmationState, string> = {
    quiet: "安静",
    watching: "观察中",
    confirmed: "已确认",
    escalating: "升级中"
  };
  return labels[state];
}

export function userProfileLabel(profile: UserRiskProfile): string {
  const labels: Record<UserRiskProfile, string> = {
    conservative: "保守",
    neutral: "中性",
    aggressive: "进取"
  };
  return labels[profile];
}

export function formatNumber(value: number | null | undefined, suffix = ""): string {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "—";
  }
  return `${value.toFixed(1)}${suffix}`;
}

export function formatSignedNumber(value: number | null | undefined, digits = 1, suffix = ""): string {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "—";
  }
  const prefix = value > 0 ? "+" : "";
  return `${prefix}${value.toFixed(digits)}${suffix}`;
}

export function formatPercent(value: number | null | undefined, digits = 0): string {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "—";
  }
  return `${(value * 100).toFixed(digits)}%`;
}

export function formatDate(value: string | null | undefined): string {
  if (!value) {
    return "—";
  }
  return value.slice(0, 10);
}

export function formatDateTime(value: string | null | undefined): string {
  if (!value) {
    return "—";
  }

  const normalized = value.replace("T", " ");
  return `${normalized.slice(0, 16)} UTC`;
}

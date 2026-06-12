import type {
  BacktestRollingAuditEpisodeClassification,
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
} from "../../types";

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

export function qualityDetailLabel(grade: QualityGrade): string {
  const labels: Record<QualityGrade, string> = {
    a: "A 可靠",
    b: "B 可用",
    c: "C 降级",
    d: "D 偏弱",
    f: "F 缺测"
  };
  return labels[grade];
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
    real_history: "本地窗口覆盖",
    fallback_template: "模板参照"
  };
  return labels[source];
}

export function auditEpisodeLabel(classification: BacktestRollingAuditEpisodeClassification): string {
  const labels: Record<BacktestRollingAuditEpisodeClassification, string> = {
    stress_window: "受保护压力",
    false_positive: "纯误报"
  };
  return labels[classification];
}

export function auditEpisodeClass(classification: BacktestRollingAuditEpisodeClassification): string {
  return classification === "stress_window" ? "state-protected" : "state-false-positive";
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

export function eventSignalListLabel(state: EventConfirmationState): string {
  return state === "confirmed" || state === "escalating" ? "已确认信号" : "近期观察信号";
}

export function eventSignalListEmptyText(state: EventConfirmationState): string {
  return state === "confirmed" || state === "escalating"
    ? "当前没有新增确认信号。"
    : "当前没有近期观察信号；事件层暂不支持动作升级。";
}

export function userProfileLabel(profile: UserRiskProfile): string {
  const labels: Record<UserRiskProfile, string> = {
    conservative: "保守",
    neutral: "中性",
    aggressive: "进取"
  };
  return labels[profile];
}

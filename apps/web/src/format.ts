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

const DIMENSION_LABELS: Record<string, string> = {
  market_stress: "市场压力",
  external_sector: "外部部门与汇率",
  real_estate: "房地产与资产泡沫",
  macro_fragility: "宏观脆弱性",
  liquidity_funding: "流动性与融资",
  leverage_credit: "杠杆与信用",
  events_sentiment: "事件与公告"
};

export function dimensionLabel(dimension: string | null | undefined): string {
  if (!dimension) {
    return "—";
  }
  return DIMENSION_LABELS[dimension] ?? dimension;
}

const SOURCE_LABELS: Record<string, string> = {
  fred: "FRED",
  treasury: "U.S. Treasury",
  world_bank: "World Bank",
  boj: "BOJ",
  sec_edgar: "SEC EDGAR",
  gdelt: "GDELT",
  yfinance: "yfinance"
};

export function sourceLabel(sourceId: string | null | undefined): string {
  if (!sourceId) {
    return "—";
  }
  return SOURCE_LABELS[sourceId] ?? sourceId;
}

const SOURCE_TYPE_LABELS: Record<string, string> = {
  macro_financial_timeseries: "宏观金融时序",
  government_timeseries: "官方时序",
  filings_events: "公告事件",
  global_macro: "全球宏观",
  fx_rates_timeseries: "汇率与利率时序",
  news_events: "新闻事件",
  market_price_prototype: "行情原型"
};

export function sourceTypeLabel(sourceType: string): string {
  return SOURCE_TYPE_LABELS[sourceType] ?? sourceType;
}

const SOURCE_HEALTH_STATUS_LABELS: Record<string, string> = {
  healthy: "正常",
  delayed: "延迟",
  stale: "陈旧",
  degraded: "降级",
  prototype: "原型",
  failing: "失败",
  missing: "缺失"
};

export function sourceHealthStatusLabel(status: string): string {
  return SOURCE_HEALTH_STATUS_LABELS[status] ?? status;
}

const SOURCE_PRIORITY_LABELS: Record<string, string> = {
  p0: "核心主路径",
  p1: "扩展/辅助"
};

export function sourcePriorityLabel(priority: string | null | undefined): string {
  if (!priority) {
    return "—";
  }
  return SOURCE_PRIORITY_LABELS[priority] ?? priority;
}

const SOURCE_ACCESS_METHOD_LABELS: Record<string, string> = {
  api: "API 拉取",
  file: "文件导入",
  scrape: "网页抓取"
};

export function sourceAccessMethodLabel(accessMethod: string | null | undefined): string {
  if (!accessMethod) {
    return "—";
  }
  return SOURCE_ACCESS_METHOD_LABELS[accessMethod] ?? accessMethod;
}

export function backtestSignalSourceLabel(source: BacktestSignalSource): string {
  const labels: Record<BacktestSignalSource, string> = {
    real_history: "真实历史",
    fallback_template: "模板参考"
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

export function userProfileLabel(profile: UserRiskProfile): string {
  const labels: Record<UserRiskProfile, string> = {
    conservative: "保守",
    neutral: "中性",
    aggressive: "进取"
  };
  return labels[profile];
}

const EVENT_TYPE_LABELS: Record<string, string> = {
  risk_watch: "风险观察",
  risk_warning: "风险预警",
  risk_confirmed: "风险确认",
  filing_cluster: "公告聚集",
  funding_stress: "融资压力"
};

export function eventTypeLabel(eventType: string): string {
  return EVENT_TYPE_LABELS[eventType] ?? eventType;
}

const INDICATOR_REF_LABELS: Record<string, string> = {
  us_event_bank_8k_count: "白名单银行 8-K 数量",
  us_event_risk_keyword_count: "SEC 风险关键词命中数",
  us_banking_filing_stress_count: "银行公告压力计数",
  us_event_official_filing_severity: "SEC 官方公告严重度",
  global_news_financial_stress_count: "金融压力新闻数量"
};

export function indicatorRefLabel(indicatorId: string): string {
  return INDICATOR_REF_LABELS[indicatorId] ?? indicatorId;
}

const DATASET_LABELS: Record<string, string> = {
  fred_series_observations: "FRED 序列观测",
  treasury_daily_yield_curve: "美债收益率曲线",
  sec_filing_events: "SEC 公告事件",
  world_bank_country_indicators: "World Bank 国家指标",
  boj_fx_daily: "BOJ 汇率日序列",
  boj_money_market_rates: "BOJ 货币市场利率"
};

export function datasetLabel(datasetId: string | null | undefined): string {
  if (!datasetId) {
    return "—";
  }
  return DATASET_LABELS[datasetId] ?? datasetId;
}

const FREQUENCY_LABELS: Record<string, string> = {
  daily: "日频",
  weekly: "周频",
  monthly: "月频",
  quarterly: "季频",
  annual: "年频"
};

export function frequencyLabel(frequency: string | null | undefined): string {
  if (!frequency) {
    return "—";
  }
  return FREQUENCY_LABELS[frequency] ?? frequency;
}

const RISK_DIRECTION_LABELS: Record<string, string> = {
  higher_is_riskier: "越高越危险",
  lower_is_riskier: "越低越危险",
  two_sided: "偏离过大都危险",
  rising_fast_is_riskier: "上升过快危险",
  falling_fast_is_riskier: "下跌过快危险",
  manual_rule: "规则触发型"
};

export function riskDirectionLabel(direction: string | null | undefined): string {
  if (!direction) {
    return "—";
  }
  return RISK_DIRECTION_LABELS[direction] ?? direction;
}

const INDICATOR_QUALITY_TIER_LABELS: Record<string, string> = {
  core: "核心",
  extended: "扩展",
  supplemental: "补充"
};

export function indicatorQualityTierLabel(tier: string | null | undefined): string {
  if (!tier) {
    return "—";
  }
  return INDICATOR_QUALITY_TIER_LABELS[tier] ?? tier;
}

const UNIT_LABELS: Record<string, string> = {
  percent: "%",
  index: "指数",
  jpy_per_usd: "JPY/USD",
  count: "次",
  score: "分",
  billions: "十亿",
  thousands: "千"
};

export function unitLabel(unit: string | null | undefined): string {
  if (!unit) {
    return "";
  }
  return UNIT_LABELS[unit] ?? unit;
}

const SCORE_BASIS_LABELS: Record<string, string> = {
  原始水平: "原始水平",
  "12m同比": "12个月同比",
  "20d振幅": "20日振幅",
  变化幅度: "变化幅度",
  人工规则: "规则触发",
  缺少观测: "缺少观测"
};

export function scoreBasisLabel(scoreBasis: string | null | undefined): string {
  if (!scoreBasis) {
    return "—";
  }
  return SCORE_BASIS_LABELS[scoreBasis] ?? scoreBasis;
}

const METHOD_VERSION_FIELD_LABELS: Record<string, string> = {
  score: "评分规则版本",
  prob: "概率模型版本",
  calibration: "概率校准版本",
  feature: "特征集版本",
  label: "标签口径版本",
  posture: "执行节奏规则版本",
  playbook: "仓位动作框架版本",
  "prob mode": "概率模式",
  release: "运行状态",
  "release id": "当前生效版本",
  "pit mode": "点位可见性"
};

export function methodVersionFieldLabel(field: string): string {
  return METHOD_VERSION_FIELD_LABELS[field] ?? field;
}

const PROBABILITY_MODE_LABELS: Record<string, string> = {
  heuristic_mvp: "启发式过渡层"
};

export function probabilityModeLabel(mode: string): string {
  if (PROBABILITY_MODE_LABELS[mode]) {
    return PROBABILITY_MODE_LABELS[mode];
  }
  if (mode.startsWith("formal_bundle")) {
    return "正式概率包";
  }
  return mode;
}

const POINT_IN_TIME_MODE_LABELS: Record<string, string> = {
  strict: "严格 PIT",
  best_effort: "过渡 PIT"
};

export function pointInTimeModeLabel(mode: string): string {
  return POINT_IN_TIME_MODE_LABELS[mode] ?? mode;
}

const RELEASE_SERVING_STATUS_LABELS: Record<string, string> = {
  healthy: "运行正常",
  degraded: "降级运行"
};

export function releaseServingStatusLabel(status: string): string {
  return RELEASE_SERVING_STATUS_LABELS[status] ?? status;
}

const RELEASE_MANIFEST_STATUS_LABELS: Record<string, string> = {
  active: "当前生效",
  approved: "已批准",
  archived: "已归档",
  rolled_back: "已回退",
  retired: "已退役"
};

export function releaseManifestStatusLabel(status: string): string {
  return RELEASE_MANIFEST_STATUS_LABELS[status] ?? status;
}

const RUNTIME_THRESHOLD_LABELS: Record<string, string> = {
  "prepare floor": "准备档进入线",
  "hedge floor": "对冲档进入线",
  "defend floor": "防守档进入线",
  "weeks bridge": "数周窗口桥接线",
  "external bridge": "外部冲击桥接线",
  "carry bridge": "日元套息桥接线"
};

export function runtimeThresholdLabel(label: string): string {
  return RUNTIME_THRESHOLD_LABELS[label] ?? label;
}

export function technicalWithLabel(label: string, technical: string | null | undefined): string {
  if (!technical) {
    return label;
  }
  return `${label}（${technical}）`;
}

export function humanizeNarrativeCopy(text: string): string {
  return text
    .replaceAll(/\bposture\b/g, "执行节奏")
    .replaceAll(/\bp_60d\b/g, "60日危机先验")
    .replaceAll(/\bp_20d\b/g, "20日危机先验")
    .replaceAll(/\bp_5d\b/g, "5日危机先验")
    .replaceAll(/\bstructural score\b/g, "结构性风险强度")
    .replaceAll(/\bhigh beta\b/g, "高波动")
    .replaceAll(/高 beta/g, "高波动")
    .replaceAll(/\bprepare\b/g, "准备档")
    .replaceAll(/\bhedge\b/g, "对冲档")
    .replaceAll(/\bdefend\b/g, "防守档")
    .replaceAll(/\bnormal\b/g, "正常观察")
    .replaceAll(/\bL1 Watch\b/g, "L1 观察")
    .replaceAll(/\bL2 Stress\b/g, "L2 压力")
    .replaceAll(/\bL3 Warning\b/g, "L3 预警")
    .replaceAll(/\bbeta\b/g, "高波动")
    .replaceAll(/\bfiling\b/g, "公告")
    .replaceAll(/\bJPY carry\b/g, "日元套息")
    .replaceAll("12m同比", "12个月同比")
    .replaceAll("20d振幅", "20日振幅")
    .replaceAll("当前信号", "当前读数")
    .replaceAll("按人工规则评分", "按规则触发评分")
    .replaceAll("当前处于相对低压区。", "当前处在低压区，更像缓冲项。")
    .replaceAll("压力 公告", "压力公告");
}

function humanizeTechnicalFamily(family: string) {
  const mappings: Array<[RegExp, (...groups: string[]) => string]> = [
    [/^scoring_v(\d+)$/, (version) => `评分规则 v${version}`],
    [/^interaction_tail_v(\d+)$/, (version) => `概率模型 v${version}`],
    [/^interaction_tail_extmix(\d+)$/, (variant) => `候选版本 ${variant}`],
    [/^platt$/, () => "Platt 校准"],
    [/^formal_v(\d+)_main$/, (version) => `正式特征主线 v${version}`],
    [/^formal_label_v(\d+)$/, (version) => `正式标签口径 v${version}`],
    [/^posture_v(\d+)$/, (version) => `执行节奏规则 v${version}`],
    [/^action_playbook_v(\d+)$/, (version) => `动作框架 v${version}`],
    [/^runtime_history_v(\d+)$/, (version) => `历史审计策略 v${version}`],
    [/^protected_stress_windows$/, () => "受保护窗口目录"]
  ];

  for (const [pattern, formatter] of mappings) {
    const match = family.match(pattern);
    if (match) {
      return formatter(...match.slice(1));
    }
  }

  return family;
}

export function compactTechnicalId(
  value: string | null | undefined,
  familySegmentCount = 3
) {
  if (!value) {
    return {
      value: "none",
      hint: undefined
    };
  }

  const [head] = value.split("|");
  const parts = head.split("_").filter(Boolean);
  if (parts.length === 2 && /^\d{8}(T\d+)?$/i.test(parts[1])) {
    return {
      value: `${humanizeTechnicalFamily(parts[0])} · ${parts[1]}`,
      hint: value
    };
  }

  if (parts.length < 3) {
    return {
      value: head,
      hint: head === value ? undefined : value
    };
  }

  const timestamp = parts.at(-1);
  const family = humanizeTechnicalFamily(
    parts
    .slice(Math.max(0, parts.length - (familySegmentCount + 1)), parts.length - 1)
    .join("_")
  );
  return {
    value: `${family} · ${timestamp}`,
    hint: value
  };
}

export function releaseIdLabel(value: string | null | undefined) {
  if (!value) {
    return {
      value: "未绑定版本",
      hint: undefined
    };
  }

  const [head] = value.split("|");
  const extmixMatch = head.match(/(?:^|_)(?:main_)?extmix(\d*)_(\d{8})(?:T(\d{2})(\d{2})(\d{2}))?$/);
  const mainMatch = head.match(/(?:^|_)main_(\d{8})(?:T(\d{2})(\d{2})(\d{2}))?$/);
  const formatTimestamp = (date: string, hour?: string, minute?: string) => {
    const formattedDate = `${date.slice(0, 4)}-${date.slice(4, 6)}-${date.slice(6, 8)}`;
    return `${formattedDate}${hour && minute ? ` ${hour}:${minute}` : ""}`;
  };

  if (extmixMatch) {
    const [, version, date, hour, minute] = extmixMatch;
    return {
      value: `${version ? `候选版本 ${version}` : "候选版本"} · ${formatTimestamp(date, hour, minute)}`,
      hint: value
    };
  }

  if (mainMatch) {
    const [, date, hour, minute] = mainMatch;
    return {
      value: `主线版本 · ${formatTimestamp(date, hour, minute)}`,
      hint: value
    };
  }

  return compactTechnicalId(value, 1);
}

export function compactFileReference(value: string | null | undefined, segments = 3) {
  if (!value) {
    return {
      value: "none",
      hint: undefined
    };
  }

  const normalized = value.replaceAll("\\", "/");
  const parts = normalized.split("/").filter(Boolean);
  if (parts.length <= segments) {
    return {
      value: normalized,
      hint: undefined
    };
  }

  return {
    value: parts.slice(-segments).join("/"),
    hint: normalized
  };
}

export interface PostureClauseDescriptor {
  label: string;
  summary: string;
  kind: "trigger" | "blocker" | "preference";
}

const POSTURE_CLAUSE_DESCRIPTORS: Record<string, PostureClauseDescriptor> = {
  defend_p5d_trigger: {
    label: "5日危机先验触发 defend",
    summary: "5日危机先验超过 defend 阈值，且触发层已经进入高压区。",
    kind: "trigger"
  },
  defend_carry_trigger: {
    label: "套息平仓压力触发 defend",
    summary: "JPY carry 压力和外部冲击共振，系统把短端窗口视为已打开。",
    kind: "trigger"
  },
  defend_actionability: {
    label: "动作头确认 defend",
    summary: "独立动作头认为近端保护优先级已经足够高。",
    kind: "trigger"
  },
  hedge_p20d_context: {
    label: "20日危机先验触发 hedge",
    summary: "20日危机先验越过 hedge 阈值，且触发层、外部层或事件层已经给出上下文确认。",
    kind: "trigger"
  },
  hedge_p60d_elevated: {
    label: "60日高位挤压到数周",
    summary: "60日先验已升高，结构脆弱性和外部冲击同步恶化，系统认为风险开始压缩到数周。",
    kind: "trigger"
  },
  hedge_carry_structural: {
    label: "JPY carry 叠加结构脆弱性",
    summary: "日元套息融资压力偏高，足以把原本的中期风险推到 hedge 档位。",
    kind: "trigger"
  },
  hedge_actionability: {
    label: "动作头确认 hedge",
    summary: "独立动作头提示未来几周的保护动作需要前置。",
    kind: "trigger"
  },
  prepare_p60d_structural: {
    label: "60日危机先验触发 prepare",
    summary: "60日危机先验超过 prepare 阈值，同时结构脆弱性已经明显抬升。",
    kind: "trigger"
  },
  prepare_structural_downgrade: {
    label: "结构脆弱性提前进入 prepare",
    summary: "即使 60 日先验还没到主阈值，但结构风险已经够高，系统先切到 prepare。",
    kind: "trigger"
  },
  prepare_external_structural: {
    label: "外部冲击放大 prepare",
    summary: "结构脆弱性还没到短端窗口，但外部放大器已经足够强，需要先做准备。",
    kind: "trigger"
  },
  prepare_carry_structural: {
    label: "JPY carry 提前进入 prepare",
    summary: "日元融资环境开始变紧，系统把它作为中期风险积累的放大器。",
    kind: "trigger"
  },
  prepare_actionability: {
    label: "动作头确认 prepare",
    summary: "独立动作头提示应该先准备现金、对冲工具和执行顺序。",
    kind: "trigger"
  },
  quality_blocked_hedge: {
    label: "数据质量阻断 hedge",
    summary: "原本存在 hedge 级信号，但当前数据质量太差，系统拒绝直接升级档位。",
    kind: "blocker"
  },
  preference_conservative_escalation: {
    label: "保守偏好上调档位",
    summary: "用户偏好更保守，系统把基础执行节奏再上调一档处理。",
    kind: "preference"
  },
  preference_aggressive_deescalation: {
    label: "进取偏好下调档位",
    summary: "用户偏好更进取，系统把基础执行节奏适度下调后再给出建议。",
    kind: "preference"
  },
  preference_neutral_no_adjustment: {
    label: "中性偏好未调整",
    summary: "用户偏好没有改变基础执行节奏。",
    kind: "preference"
  }
};

export function describePostureClause(code: string): PostureClauseDescriptor {
  return (
    POSTURE_CLAUSE_DESCRIPTORS[code] ?? {
      label: code,
      summary: "当前版本还没有为这个条款补充中文解释。",
      kind: "trigger"
    }
  );
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

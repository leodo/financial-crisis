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
} from "../types";

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

export function sourceQualityBandLabel(score: number): string {
  if (score >= 90) {
    return "高";
  }
  if (score >= 80) {
    return "可用";
  }
  if (score >= 70) {
    return "一般";
  }
  return "偏弱";
}

export function sourceLagLabel(seconds: number | null | undefined): string {
  if (seconds === null || seconds === undefined) {
    return "滞后未知";
  }

  const days = Math.round(seconds / 86_400);
  if (days >= 1) {
    return `滞后 ${days} 天`;
  }

  const hours = Math.round(seconds / 3_600);
  if (hours >= 1) {
    return `滞后 ${hours} 小时`;
  }

  return "近实时";
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

const RELEASE_REVIEW_HISTORY_MODE_LABELS: Record<string, string> = {
  strict_rebuild: "严格重放",
  default: "默认历史缓存"
};

export function releaseReviewHistoryModeLabel(mode: string): string {
  return RELEASE_REVIEW_HISTORY_MODE_LABELS[mode] ?? mode ?? "—";
}

const RELEASE_REVIEW_WORKSTREAM_LABELS: Record<string, string> = {
  strict_review_vs_runtime_mapping: "严格评审 vs 运行映射",
  posture_continuity: "执行节奏连续性",
  score_confirmation: "评分确认层",
  transitional_bridge: "过渡桥接层"
};

export function releaseReviewWorkstreamLabel(workstream: string): string {
  return RELEASE_REVIEW_WORKSTREAM_LABELS[workstream] ?? workstream;
}

const RELEASE_REVIEW_ATTRIBUTION_LABELS: Record<string, string> = {
  candidate_regression: "候选版新增退化",
  both_baseline_and_candidate: "主线已有短板，候选未修复",
  baseline_shared_weakness: "主线既有短板"
};

export function releaseReviewAttributionLabel(attribution: string): string {
  return RELEASE_REVIEW_ATTRIBUTION_LABELS[attribution] ?? attribution;
}

const RELEASE_REVIEW_ACTION_TYPE_LABELS: Record<string, string> = {
  candidate_reject_or_retrain: "判退 / 重训",
  shared_blocker_fix_before_promotion: "晋升前先修",
  baseline_research_fix: "主线研究修复",
  manual_review: "继续人工复核"
};

export function releaseReviewActionTypeLabel(actionType: string): string {
  return RELEASE_REVIEW_ACTION_TYPE_LABELS[actionType] ?? actionType;
}

export function releaseReviewVerdictLabel(passed: boolean): string {
  return passed ? "通过当前 guard" : "存在 guard blocker";
}

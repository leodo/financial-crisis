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

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
  partial_failure: "部分失败",
  failed: "失败",
  prototype: "原型",
  disabled: "停用",
  stale: "陈旧",
  degraded: "降级",
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

export function technicalWithLabel(label: string, technical: string | null | undefined): string {
  if (!technical) {
    return label;
  }
  return `${label}（${technical}）`;
}

export function humanizeSourceLicenseNote(text: string): string {
  return text
    .replaceAll("Official no-key Treasury yield curve data.", "官方免 key 的美债收益率曲线数据。")
    .replaceAll(
      "FRED graph CSV is the default no-key source; official API remains optional.",
      "FRED Graph CSV 是默认免 key 路径，官方 API 只是可选增强。"
    )
    .replaceAll(
      "Official SEC JSON filings metadata aggregated into daily event features. No paid key is required.",
      "SEC 官方 JSON 公告元数据已聚合成日频事件特征，无需付费 key。"
    )
    .replaceAll("Official World Bank Indicators API.", "World Bank 官方指标 API。")
    .replaceAll(
      "Official BOJ FX and money-market endpoints are used for the JPY carry monitor.",
      "BOJ 官方汇率和货币市场接口用于日元套息监控。"
    )
    .replaceAll(
      "Development-only market data prototype; not a production dependency.",
      "仅开发期市场数据原型，不属于正式依赖。"
    )
    .replaceAll("prototype source, not for production", "原型源，不进入正式评估");
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

export function humanizeMethodNote(text: string): string {
  return humanizeNarrativeCopy(text)
    .replaceAll("protected stress window", "受保护压力窗口")
    .replaceAll("stress window catalog", "压力窗口目录");
}

export function humanizeAuditNote(text: string): string {
  return humanizeNarrativeCopy(text)
    .replaceAll("release registry", "版本登记册")
    .replaceAll("historical replay run / point", "历史回放结果")
    .replaceAll("prediction snapshot", "预测快照")
    .replaceAll("runtime probability mode", "运行中的概率层")
    .replaceAll("release manifest", "版本登记状态")
    .replaceAll("heuristic", "启发式过渡层");
}

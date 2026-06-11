export const sourcesContent = {
  sourceGuideRows: [
    ["FRED Graph CSV", "默认无 key 路径，适合宏观和部分市场序列。"],
    ["U.S. Treasury", "官方收益率曲线兜底，不依赖第三方包装。"],
    ["World Bank", "年频慢变量补充结构脆弱性。"],
    ["BOJ + JPY carry", "BOJ 官方 USDJPY 和日本隔夜拆借利率已接入，用于免费跟踪套息融资环境。"],
    ["SEC EDGAR", "已接入官方公告 JSON，并聚合为银行公告事件特征与告警。"],
    ["GDELT", "已支持可选回填和运行时展示，但默认仍按原型辅助信号处理。"]
  ] as Array<[string, string]>,
  summaryNote: "先看关键指标覆盖、生产源健康降级和日期口径，再进入下面的数据源明细；源健康分只说明抓取/源状态，不等同于当前结论可信度。",
  warningsEmpty: "当前没有额外数据可信度告警。",
  tableNote: "小屏幕下这张表支持横向滚动；最新观测、观测滞后、抓取水位和最近成功刷新是不同口径，源健康分也不是当前结论可信度。"
} as const;

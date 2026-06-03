export const methodContent = {
  layerGuideRows: [
    ["风险强度", "0-100 分只表示指标组合位于历史压力区的什么位置，不等于危机发生概率。"],
    ["危机概率", "告诉你未来 5d / 20d / 60d 进入风险窗口的可能性。"],
    [
      "动作概率",
      "准备 / 对冲 / 防守是独立动作层，只回答是否该准备、对冲或防守，不等于危机先验，也不是 60d / 20d / 5d 的直接改名。"
    ],
    ["风险时距", "告诉你当前更像是数月、数周还是当下风险。"],
    ["执行节奏", "把概率和可信度转换成可执行的风险处理节奏。"]
  ] as Array<[string, string]>,
  clauseTriggerEmpty: "当前没有额外触发条款。",
  blockerLead: "以下条款阻断了更激进的执行节奏升级：",
  tableNote: "小屏幕下这张表支持横向滚动，先看窗口名称和日期，再看备注。",
  protectedCatalogTitle: "受保护压力窗口目录",
  runtimeBoundarySummary:
    "这个页面把危机概率、动作概率、风险时距和执行节奏拆开解释。历史回放会优先复用同口径缓存；如果缓存口径不匹配，系统才会回退到已落库快照或按原始观测重建。",
  limitationModeFormal:
    "已经切到正式概率包，但仍要结合数据新鲜度、回测审计和事件确认一起解释。",
  limitationModeHeuristic: "这仍是启发式过渡层，不能当成校准后的正式危机概率。",
  limitationReleaseDegraded:
    "因此页面上的仓位预算更适合当作执行节奏和保护框架，而不是自动交易指令。",
  limitationReleaseHealthy:
    "表示当前线上版本处于正式服务状态，但仓位建议仍应配合你的账户约束和流动性条件执行。"
} as const;

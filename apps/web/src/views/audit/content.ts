export const auditContent = {
  guideRows: [
    ["版本登记册", "看线上当前登记了哪些候选版、正式版和历史版。"],
    ["运行模式", "看 API 当前实际正在使用启发式过渡层，还是正式概率包。"],
    ["Overlay 审计", "看当前 active release 是只有 family 审计元数据，还是已经有 overlay head 真正参与 runtime。"],
    ["快照历史", "看每天落库的概率快照是否和当前生效版本对得上。"],
    ["降级识别", "如果版本登记已经是正式版，但运行态退回启发式层，通常说明 bundle 加载或服务检查失败。"]
  ] as Array<[string, string]>,
  noteSummary:
    "这页主要用来核对当前线上版本、版本登记历史和历史预测快照是否一致；如果运行中的概率层和登记状态对不上，通常表示系统已经自动降级。",
  summaryNote: "先看版本总量、当前/已批准和快照覆盖，再进入下面两张明细表。",
  overlaySummary:
    "这里只看当前 active release 的 runtime overlay 证据，不替代离线 release review。先看当前有没有真正挂载 overlay，再看每个 horizon 的 final 概率是否被改写。",
  overlayEmpty:
    "当前 active release 没有输出 family overlay 诊断；这通常说明还在旧 bundle，或者该版本没有挂载 overlay head。",
  overlayTableNote:
    "先看场景数和三段 split 行数，再看 gate-active 行数，最后结合 note 判断这个 family 是真正可训，还是只有候选审计。",
  unsupportedPrefix: "当前数据存储模式为",
  unsupportedSuffix: "，暂时没有可展示的本地版本 / 快照审计数据。",
  releaseTableNote: "小屏幕下这张表支持横向滚动，优先看版本、登记状态和训练区间。",
  snapshotTableNote: "先看日期、版本和 5d/20d/60d，再看执行档位、新鲜度和覆盖度。"
} as const;

export const driversContent = {
  dimensionCaptionPrefix: "这个维度当前最敏感的是",
  dimensionCaptionSuffix: "。",
  guideRows: [
    ["近端驱动", "优先看日频/周频且最近更新的指标，它们更能解释“为什么是现在”。"],
    ["结构背景", "月频、季频、年频或偏旧高分项只解释风险底色，不等于今天刚发生触发。"],
    ["缓冲因素", "看哪些指标仍停留在低压区，为什么系统还没进入更激进档位。"],
    ["维度解释", "先看维度总分，再看该维度里最敏感的 2-3 个指标。"],
    ["当前结论", "把系统摘要、旧引擎解释和执行节奏结论放在一起，方便交叉核对。"]
  ] as Array<[string, string]>,
  summaryTitles: {
    system: "系统摘要",
    legacy: "旧版评分层解释",
    posture: "执行节奏结论"
  }
} as const;

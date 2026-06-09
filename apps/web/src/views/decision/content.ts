export const decisionContent = {
  prelude: {
    calloutTitle: "风险强度分不是危机概率。",
    calloutBody:
      "总风险、结构脆弱性、触发压力和外部冲击分反映的是压力位置；真正用于决策的是 5d / 20d / 60d 危机先验、准备 / 对冲 / 防守动作层、风险时距和执行节奏。",
    runtimeSummary:
      "这是基于免费日频/周频数据的危机预警面板，不是逐笔行情终端。先看日期和模式，再解读数值。",
    generatedHint: "点击右上角刷新按钮可以立即重新载入本地库。",
    cadenceTitle: "日频预警",
    cadenceHint: "更适合判断未来几天到数周的风险窗口，不适合替代盘中报价软件。"
  },
  posture: {
    intro: "执行节奏是系统建议的风险处理方式，从观察到防守一共四档，当前高亮的是系统结论。"
  },
  riskHorizon: {
    tileHints: {
      p5d: "用于判断是不是已经接近急性风险窗口。",
      p20d: "用于判断未来几周是否应考虑保护性对冲。",
      p60d: "用于判断中期脆弱性是否已经积累。"
    },
    bandLegend:
      "5d 看急性冲击，20d 看未来几周是否需要离场和保护，60d 看数月级脆弱性；卡片里的距离进入线用于判断离准备/对冲/防守阈值还有多远。",
    priorVsAction: {
      title: "先验和动作概率不是一回事",
      body:
        "上面 5d / 20d / 60d 是危机先验，回答“风险窗口离现在有多近”；下面 准备 / 对冲 / 防守 是独立动作概率，回答“现在该不该开始准备、对冲或防守”，不是把 60d / 20d / 5d 直接换了名字。"
    },
    actionHints: {
      prepare: "回答是否该先准备现金、执行顺序和保护工具，通常早于真正的离场动作。",
      hedge: "回答是否该把保护性对冲提前到未来几周内执行。",
      defend: "回答近端保护和去杠杆是否已经优先于继续冒险。"
    },
    actionSourceFallback:
      "当前生效版本尚未内置独立动作模型，先用危机先验和评分层做过渡映射。"
  },
  clauses: {
    triggeredEmpty: "当前执行节奏没有额外条款触发，仍处于常态观察。",
    blockedEmpty: "当前没有阻断条款。"
  },
  panels: {
    whyNowTopDrivers: "当前最强的上行驱动",
    reliefBody:
      "这些缓冲因素解释了为什么当前评估还没有被推到更高执行档位，也提醒你不要只盯着单个高分指标。",
    actionPlanCapitalPreservation:
      "当前已满足防守档、当下风险窗口、高可信度和事件确认，不必默认清仓，但应把去杠杆、现金和核心保护放在收益追逐之前。",
    actionPlanGovernance:
      "下面这组边界回答的是这套建议能做到什么、不能做到什么。它给的是系统层预算和执行顺序，不会替你下单。",
    actionPlanChecks:
      "执行前先按这份清单做人工复核，确认当前输出没有绕开动作手册、release review 和 Go/No-Go 边界。",
    eventConfirmedTitle: "已确认信号",
    eventPendingTitle: "待补缺口",
    eventConfirmedEmpty: "当前没有新增确认信号。",
    eventPendingEmpty: "当前没有额外待补缺口。",
    jpyCarryLegend:
      "这张卡不是在预测日本危机，而是在看日元融资环境是否可能放大美国风险资产的同步回撤。",
    rollingAuditDefinition:
      "危机前命中表示系统在危机前 20 日内发出动作信号；受保护压力表示虽然没有落入定义危机，但处在应允许保护性减仓或对冲的系统压力阶段；纯误报才是需要继续压缩的噪声。"
  }
} as const;

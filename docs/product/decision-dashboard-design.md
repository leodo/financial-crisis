# 决策面板设计

状态：`Draft`

最后更新：2026-05-31

## 1. 目标

把系统从“风险打分看板”升级成“危机距离和处置准备面板”。

首屏必须让用户在几秒内回答：

1. 当前危险吗？
2. 风险更像是数月、数周还是当下？
3. 为什么？
4. 和 `2008`、`2020`、`2023` 比处在哪？

## 2. 首屏结构

```text
顶部摘要
  当前 posture
  p_5d / p_20d / p_60d
  time_to_risk_bucket
  conviction
  data_mode
  stale warning

第二层
  关键指标 freshness
  事件层确认
  为什么高
  为什么没更高
  posture 梯子
  风险强度区间解释

第三层
  组合动作预算
  历史危机对照
  结构 / 触发 / 外部 三分图
  JPY carry 专题卡

第四层
  指标下钻
  事件下钻
  回测视图
```

## 3. 核心组件

### 3.1 Current Assessment Hero

显示：

- `p_5d`
- `p_20d`
- `p_60d`
- `time_to_risk_bucket`
- 当前 posture
- 简短结论
- `data_mode`
- stale warning（如果当前是 demo 或关键指标滞后）

示例结论：

- 风险更接近“数周”而不是“当下”
- 短期概率已升高，建议提高防守准备

### 3.2 Why Now

显示推升当前概率的主要因素：

- VIX 和信用利差同步上升
- 曲线和融资条件恶化
- 银行业事件增加
- JPY carry 波动放大外部冲击

### 3.3 Why Not Higher

这个模块很重要，用于抑制误判。

显示：

- 哪些维度还没有共振
- 哪些数据缺失
- 为什么系统还没有进入 `defend`

### 3.4 Historical Analogs

展示当前与历史压力阶段的对照：

- 当前 vs `2008` 预警前 1 个月
- 当前 vs `2020` 冲击前 2 周
- 当前 vs `2023` 区域银行事件前

### 3.5 Time-to-Risk

把概率翻译成时距提示：

- `months`
- `weeks`
- `now`

并解释映射依据。

### 3.6 Position Guidance

把 posture 翻译成系统级动作预算：

- 风险资产上限
- 现金目标
- 对冲覆盖
- 杠杆上限
- 期权保护

要求：

- 用预算条而不是只放数字
- 每个预算旁边都有一句“为什么”
- 明确写这是系统预算，不是自动交易

### 3.7 Data Trust

显示：

- 关键数据覆盖率
- 最新更新时间
- 是否使用代理变量
- 当前可信度评级

### 3.8 Key Indicator Freshness

必须覆盖：

- `USDJPY`
- 日本隔夜拆借利率
- `EFFR`
- `VIX`

每项都要显示：

- 最新值
- 最新日期
- 来源
- 滞后天数
- `fresh / delayed / stale / missing`

目标：

- 避免把 demo 值误读成真实市场值
- 避免把旧 SQLite 值误读成当前值

### 3.9 JPY Carry Card

专题卡展示：

- 当前状态
- 近期变化
- 是否在放大美国风险

## 4. 页面结构

### 4.1 总览页

面向“现在该不该防守”。

### 4.2 指标页

面向“哪些指标在驱动概率变化”。

### 4.3 事件页

面向“最近有哪些银行/公告/新闻在支持这个判断”。

### 4.4 回测页

面向“系统历史上是否真的有提前量”。

### 4.5 方法页

面向“概率是怎么来的，可信度如何”。

## 5. 文案原则

- 明确写“概率”还是“风险强度”。
- 明确写“这是系统 posture，不是自动交易指令”。
- 明确写“当前离危险多远”。
- 明确写“哪些因素缺失或降低可信度”。
- 明确写“当前看到的是不是实时值”。

## 6. 视觉原则

- 首屏优先数字结论，不先堆表格。
- 颜色用于区分 posture 和 horizon probability，不再只围绕 `0-100` 分。
- 历史对照必须直观，避免用户自己换算。
- 所有关键卡片都要能一眼区分“现在危险”和“值得继续观察”。

## 7. API 需求

```text
/api/assessment/current
/api/assessment/history
/api/assessment/analogs
/api/assessment/data-trust
/api/assessment/posture
/api/events/recent
/api/backtests
```

## 8. 第一阶段落地清单

1. 用真实 SQLite 评估结果替换 demo 顶部结论。
2. 新增三 horizon probability 卡片。
3. 新增 time-to-risk 组件。
4. 新增 posture 梯子和风险强度区间解释。
5. 新增 historical analogs 组件。
6. 新增 JPY carry 专题卡。
7. 新增组合动作预算组件。
8. 新增关键指标 freshness 与 demo/stale 强提示。
9. 回测页切换到真实历史曲线。

## 9. 不再推荐的旧表达

- 只放一个大号总分
- 大量 `风险 100`
- 不区分概率和强度
- 不告诉用户离风险有多远

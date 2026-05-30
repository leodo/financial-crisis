# Web 面板信息架构

状态：`Draft`

最后更新：2026-05-30

## 1. 目标

设计金融危机预警系统的网页面板信息架构。面板需要让用户快速理解当前整体风险、主要驱动因素、数据质量和可追溯指标细项。

## 2. 用户任务

核心用户任务：

- 判断当前整体风险等级。
- 理解风险为什么变化。
- 查看哪些维度和指标贡献最大。
- 下钻到单个指标的历史走势、阈值和数据源。
- 查看数据抓取和质量状态。
- 查看历史预警记录。
- 用回测理解系统可靠性。

## 3. 页面结构

```text
Dashboard
  Overview              总览
  Dimensions            分项风险
  Indicators            指标库
  Indicator Detail      指标详情
  Alerts                预警记录
  Data Sources          数据源状态
  Backtesting           回测
  Settings              配置和权重，后续
```

## 4. 总览页

目标：

- 10 秒内回答“现在风险有多高、为什么”。

主要模块：

- 整体风险等级。
- 整体风险分。
- 结构性风险分。
- 触发性风险分。
- Top 风险贡献。
- 最近 30/90/365 天趋势。
- 数据质量摘要。
- 最新预警事件。

布局建议：

```text
顶部：范围选择器 + as_of_date + 数据质量灯号
主区：整体风险等级 + 分数趋势
中区：维度热力条 + Top contributors
下区：最新预警 + 数据源异常 + 关键指标走势
```

## 5. 分项风险页

目标：

- 展示每个风险维度的状态和变化。

维度：

- 宏观脆弱性。
- 杠杆与信用。
- 市场压力。
- 流动性与融资。
- 银行体系。
- 房地产与资产泡沫。
- 外部部门与汇率。
- 事件与情绪。

每个维度展示：

- 当前分数和等级。
- 7/30/90 天变化。
- Top 指标贡献。
- 数据质量。
- 历史走势。

## 6. 指标库页

目标：

- 让用户浏览所有指标，并快速筛选异常项。

表格字段：

```text
indicator_id
display_name
dimension
entity
latest_value
unit
as_of_date
risk_score
risk_level
change_30d
percentile
quality_grade
source
frequency
```

筛选：

- 区域。
- 维度。
- 风险等级。
- 数据源。
- 频率。
- 质量等级。
- 只看最近恶化。

## 7. 指标详情页

目标：

- 解释单个指标如何影响风险评分。

模块：

- 指标说明。
- 最新值、风险分、历史分位。
- 时间序列图。
- 阈值线和风险区间。
- 派生特征图，例如同比、Z-score、滚动波动率。
- 评分公式说明。
- 数据质量检查结果。
- 数据源和原始响应追溯。

## 8. 预警记录页

目标：

- 管理和回顾系统生成的预警事件。

字段：

```text
alert_id
level
status
scope
dimension
triggered_at
resolved_at
score
reason
top_contributors
acknowledged_by
```

支持：

- 按等级、状态、时间筛选。
- 查看事件升级/降级历史。
- 查看触发时刻的风险快照。

## 9. 数据源状态页

目标：

- 让用户判断系统当前数据是否可信。

模块：

- 数据源健康列表。
- 最近成功抓取时间。
- 延迟。
- 连续失败次数。
- 限流次数。
- 隔离任务数。
- 质量分趋势。
- 数据源授权状态。

关键原则：

- 数据源异常不能隐藏。
- 预警分数应显示是否受到数据质量影响。

## 10. 回测页

目标：

- 展示系统历史表现。

模块：

- 历史危机时间轴。
- 回测期间风险分走势。
- 触发点、解除点、提前量。
- 误报和漏报统计。
- 不同方法版本对比。

## 11. 全局导航

建议侧边栏：

```text
Overview
Dimensions
Indicators
Alerts
Data Sources
Backtesting
Settings
```

顶部全局控件：

- Region。
- Market scope。
- As-of date。
- Method version。
- Data quality mode。

## 12. 信息优先级

首页不要堆满所有指标。优先级：

1. 整体风险等级和变化。
2. 为什么变化。
3. 哪些维度最危险。
4. 数据是否可信。
5. 能否下钻验证。

## 13. 技术建议

前端建议：

- React + TypeScript + Vite。
- ECharts 负责趋势、热力、贡献图。
- TanStack Table 负责指标和预警表格。
- TanStack Query 负责 API 缓存和刷新。

API 应提供页面级聚合接口，避免前端拼接大量底层查询。


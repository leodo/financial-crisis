# 回测设计

状态：`Draft`

最后更新：2026-05-30

## 1. 目标

设计金融危机预警系统的历史回测方法，用来评估规则评分、指标体系和预警等级是否能在历史危机前给出有用信号。

回测目标不是证明系统能精确预测危机日期，而是验证：

- 是否能提前发现压力积累。
- 是否误报过多。
- 哪些指标在历史上有效。
- 数据修订和滞后是否会污染结果。
- 不同方法版本表现如何。

## 2. 回测原则

- 使用 point-in-time 数据优先，避免未来函数。
- 没有 vintage 的数据必须标记修订风险。
- 所有评分使用当时可得数据，而不是今天修订后的完整数据。
- 危机窗口定义必须独立于模型输出。
- 既评估提前预警，也评估误报成本。

## 3. 回测对象

第一批美国场景：

| 场景 | 危机窗口 | 目标 |
|---|---|---|
| 2007-2009 全球金融危机 | 2007-08 到 2009-03 | 信用、流动性、银行压力 |
| 2020 新冠流动性冲击 | 2020-02 到 2020-04 | 市场压力、流动性压力 |
| 2022 通胀和利率冲击 | 2022-01 到 2022-12 | 利率、期限结构、资产重估 |
| 2023 区域银行危机 | 2023-03 到 2023-05 | 银行公告、利率、存款、流动性 |

后续全球场景：

- 亚洲金融危机。
- 欧债危机。
- 主要新兴市场汇率危机。
- 房地产泡沫破裂案例。

## 4. 时间轴定义

每个场景定义：

```text
scenario_id
name
region
crisis_start
crisis_peak
crisis_end
pre_warning_window_start
pre_warning_window_end
post_crisis_window_end
source_note
```

建议窗口：

- 预警窗口：危机开始前 3 到 18 个月。
- 危机窗口：已知压力或事件集中期。
- 恢复窗口：危机结束后 3 到 12 个月。

## 5. 回测数据快照

每次回测必须记录：

```text
backtest_id
scenario_id
method_version
data_snapshot_id
indicator_set_version
started_at
finished_at
config
```

数据快照应记录：

- 指标观测值版本。
- 原始数据版本。
- 数据质量规则版本。
- 指标映射版本。
- 是否使用 point-in-time。

## 6. 评估指标

### 6.1 预警提前量

```text
lead_time = crisis_start - first_alert_date
```

第二阶段起，需要显式区分两类提前量：

- `structural_lead_time`
  - 第一次进入持续结构性抬升状态的时间。
  - 含义是“系统开始看到脆弱性积累”，不等于已经应该立刻清仓。
- `actionable_lead_time`
  - 第一次进入持续可执行预警状态的时间。
  - 含义是“系统已经给出足够强的动作级风险信号”，更接近是否要减仓、上保护、降杠杆。

场景摘要仍保留：

- `first_l1_date`
- `first_l2_date`
- `first_l3_date`

其中建议语义：

- `first_l2_date` 对应结构性抬升。
- `first_l3_date` 对应可执行预警。

### 6.2 命中率

在预警窗口内触发可执行预警即视为命中。

```text
hit = alert_date in pre_warning_window
```

如果只出现结构性抬升，但没有进入可执行预警，应单列为：

- `structural_only_hit`
- 不能直接计入动作级命中率。

### 6.3 误报率

非危机窗口内触发 L3/L4 视为高严重度误报。

需要区分：

- 合理压力但未演变为危机。
- 纯噪声误报。
- 数据质量导致误报。

### 6.4 稳定性

衡量等级是否频繁跳变：

```text
level_change_count
average_level_duration
upgrade_downgrade_ratio
```

### 6.5 解释质量

人工检查：

- Top contributors 是否符合历史叙事。
- 指标恶化是否有合理经济含义。
- 数据质量是否影响结论。

## 7. 回测输出

回测结果输出：

```text
scenario_summary
  first_l1_date
  first_l2_date
  first_l3_date
  max_level
  max_score
  lead_time_days
  actionable_lead_time_days
  false_positive_count
  missed
  top_contributors
  data_quality_notes
```

前端回测页展示：

- 风险分时间序列。
- 危机窗口阴影。
- 预警触发点。
- 结构性抬升提前量 vs 可执行预警提前量。
- 全历史滚动审计：动作信号精度、危机前命中、受保护压力点、纯误报点、纯误报区间，以及最长的非危机动作区间列表。
- Top contributors 随时间变化。
- 数据质量事件。

受保护压力窗口不再硬编码在页面或 API 内部，而是统一来自：

```text
config/protected_stress_windows.us.json
```

如果需要临时实验不同口径，可通过环境变量覆盖：

```text
FC_PROTECTED_STRESS_WINDOWS_PATH=<custom-json>
```

## 8. 避免未来函数

高风险点：

- 使用修订后的 GDP、CPI、信贷数据。
- 使用后来才知道的危机标签调参。
- 使用全样本分位数计算历史分位。
- 使用未来价格计算当前回撤。

处理规则：

- 分位数只能使用当前日期之前的数据。
- 宏观数据以 publication_time 控制可见性。
- 没有 publication_time 时按保守 lag 处理。
- 方法版本定稿后再跑独立场景。

## 9. 回测阶段计划

第一阶段：

- 用 FRED 数据完成 2008、2020、2022、2023 美国场景回测。
- 只评估规则评分卡。

第二阶段：

- 加入 SEC 事件和 GDELT 新闻。
- 检查事件信号是否提升提前量。

第三阶段：

- 加入全球宏观和 BIS 信贷指标。
- 扩展到非美国危机场景。

## 10. 回测结论记录

每次回测应形成报告：

```text
docs/research/backtests/{scenario_id}-{method_version}.md
```

报告包含：

- 数据快照。
- 方法版本。
- 图表。
- 指标表现。
- 误报分析。
- 需要调整的指标或权重。

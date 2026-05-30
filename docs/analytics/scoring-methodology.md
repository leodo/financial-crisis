# 风险评分方法

状态：`Draft`

最后更新：2026-05-30

## 1. 目标

定义第一版金融危机预警评分方法。第一版优先采用可解释规则评分卡，不追求黑盒预测概率。

系统输出：

- 单指标风险分。
- 分项风险分。
- 整体风险分。
- 风险等级。
- 主要贡献因子。
- 数据质量影响说明。

## 2. 评分原则

- 可解释优先。
- 慢变量和快变量分别评分。
- 数据质量影响评分权重。
- 风险等级需要防抖，避免单日噪声频繁跳变。
- 所有评分方法必须带版本号。

## 3. 评分流程

```mermaid
flowchart LR
    A["指标观测值"] --> B["特征变换"]
    B --> C["单指标风险分"]
    C --> D["质量权重调整"]
    D --> E["维度聚合"]
    E --> F["整体聚合"]
    F --> G["风险等级"]
    G --> H["解释与贡献"]
```

## 4. 特征变换

常用变换：

| 变换 | 说明 | 适用指标 |
|---|---|---|
| `level` | 原始水平 | 利差、VIX、失业率 |
| `yoy` | 同比 | CPI、房价、贷款 |
| `mom` | 环比 | 工业产出、信贷 |
| `change_n` | N 期变化 | 利率、汇率 |
| `pct_change_n` | N 期百分比变化 | 股票、商品、货币 |
| `rolling_vol` | 滚动波动率 | 股指、汇率 |
| `drawdown` | 相对滚动高点回撤 | 股指、房价 |
| `spread` | 利差 | 信用、期限结构 |
| `percentile` | 历史分位 | 所有稳定序列 |
| `zscore` | 标准化偏离 | 近似稳定序列 |

## 5. 单指标风险分

单指标风险分范围 0 到 100。

### 5.1 越高越危险

```text
score = percentile(value, history_window)
```

示例：

- VIX
- 高收益债 OAS
- 金融压力指数

### 5.2 越低越危险

```text
score = 100 - percentile(value, history_window)
```

示例：

- 期限利差
- 外储覆盖
- GDP 增速

### 5.3 双向偏离危险

```text
score = max(
  percentile(value),
  100 - percentile(value)
)
```

或者使用目标值偏离：

```text
score = min(100, abs(value - target) / tolerance * 100)
```

示例：

- 通胀偏离目标。
- 汇率异常升值或贬值。

### 5.4 快速变化危险

```text
score = percentile(abs(change_n), history_window)
```

方向明确时：

```text
score = percentile(change_n)          # 快速上升危险
score = 100 - percentile(change_n)    # 快速下降危险
```

示例：

- 20 日国债收益率变化。
- 20 日汇率贬值。
- 银行存款快速下降。

## 6. 历史窗口

默认窗口：

| 频率 | 默认窗口 |
|---|---|
| 日频 | 5 年，最少 2 年 |
| 周频 | 8 年，最少 3 年 |
| 月频 | 15 年，最少 5 年 |
| 季频 | 20 年，最少 8 年 |
| 年频 | 30 年，最少 10 年 |

窗口不足时：

- 标记 `short_history`。
- 降低指标质量。
- 不允许作为高权重核心指标。

## 7. 维度聚合

维度分数采用加权平均和极端风险补偿：

```text
base_score = weighted_average(indicator_scores, quality_adjusted_weights)
tail_boost = max_indicator_score * tail_weight
dimension_score = min(100, base_score * (1 - tail_weight) + tail_boost)
```

默认 `tail_weight = 0.2`。

这样可以避免单个极端指标被完全平均掉。

## 8. 整体聚合

整体风险分建议分两部分：

```text
structural_score = aggregate(macro_fragility, leverage_credit, banking_system, real_estate, external_sector)
trigger_score = aggregate(market_stress, liquidity_funding, events_sentiment)
overall_score = 0.55 * structural_score + 0.45 * trigger_score
```

如果结构性风险高且触发信号也高，应增加交互项：

```text
interaction_boost = max(0, structural_score - 60) * max(0, trigger_score - 60) / 100
overall_score = min(100, overall_score + interaction_boost * 0.25)
```

## 9. 数据质量权重

质量权重：

| 质量等级 | 权重系数 |
|---|---|
| A | 1.0 |
| B | 0.9 |
| C | 0.6 |
| D | 0.0，默认不参与 |
| F | 阻断 |

如果某个维度可用指标少于最低数量，维度评分应标记 `insufficient_data`，不能假装精确。

## 10. 风险贡献

每次评分要输出贡献因子：

```text
contribution = normalized_weight * indicator_score
```

前端展示：

- Top 5 推升风险指标。
- Top 5 缓和风险指标。
- 最近变化最大的指标。
- 数据质量影响最大的指标。

## 11. 防抖和持续性

风险等级升级规则：

- L2 以上要求至少 2 个连续评分周期满足阈值，除非单日分数超过紧急阈值。
- L3/L4 需要至少 2 个维度同时升高，或者一个核心维度达到极端水平。

降级规则：

- 需要连续 3 个评分周期低于阈值。
- 预警事件解除前保留观察状态。

## 12. 方法版本

评分方法必须版本化：

```text
method_version = scoring_v1_YYYYMMDD
```

版本变化包括：

- 指标增删。
- 权重变化。
- 阈值变化。
- 聚合公式变化。
- 质量权重变化。

回测和实时结果必须记录方法版本。

## 13. 后续模型扩展

规则评分卡稳定后再加入：

- Logit/Probit 危机概率模型。
- Markov regime switching。
- Survival analysis。
- Tree-based 模型。
- 图模型和传染风险模型。

机器学习模型不能替代解释层。即使后续输出概率，也必须同时输出贡献和可解释信号。


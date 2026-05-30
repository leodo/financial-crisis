# 数据质量模型

状态：`Draft`

最后更新：2026-05-30

## 1. 目标

数据质量模型用于评估每个数据源、每个指标和每次评分结果的可信度。金融危机预警系统不能只展示风险分，还必须告诉用户这个风险分所依赖的数据是否新鲜、完整、一致、可追溯。

## 2. 质量维度

| 维度 | 说明 | 示例 |
|---|---|---|
| 新鲜度 | 数据是否按预期更新 | 日频指标超过 2 个交易日未更新 |
| 完整性 | 预期区间是否缺失 | 月度宏观数据缺少最近一期 |
| 有效性 | 值域、类型和单位是否合理 | 利率出现非数字或极端错误值 |
| 一致性 | 与历史、替代源或派生关系是否一致 | 10Y-2Y 与 DGS10-DGS2 差异过大 |
| 可追溯性 | 是否能追溯到原始响应 | 缺少 raw_payload_id |
| 修订风险 | 是否存在历史修订且未标记 | GDP 被修订但未记录 revision |
| 源健康 | 数据源抓取是否稳定 | 连续 5 次抓取失败 |

## 3. 质量等级

质量分范围：0 到 100。

| 分数 | 等级 | 含义 |
|---|---|---|
| 90-100 | `A` | 数据可靠，可直接参与评分 |
| 75-89 | `B` | 有轻微问题，可参与评分并显示提示 |
| 60-74 | `C` | 有明显风险，降低评分权重 |
| 40-59 | `D` | 不建议参与核心评分 |
| 0-39 | `F` | 阻断发布或进入隔离 |

## 4. 质量 flags

标准 flags：

```text
stale
missing_expected_period
partial_response
out_of_range
unit_mismatch
duplicate_observation
revision_detected
calendar_gap
source_rate_limited
source_failed_recently
schema_warning
raw_trace_missing
prototype_source
license_review_required
```

flags 应该写入指标观测值和评分结果，前端可以展示。

## 5. 检查规则

### 5.1 新鲜度检查

每个指标定义 `freshness_slo`：

| 频率 | 默认 SLO |
|---|---|
| 日频 | 2 个交易日 |
| 周频 | 10 个自然日 |
| 月频 | 45 个自然日 |
| 季频 | 120 个自然日 |
| 年频 | 450 个自然日 |

超过 SLO：

- 轻微超期：质量降级。
- 严重超期：阻断该指标对整体评分的新增贡献。

### 5.2 完整性检查

检查内容：

- 请求区间内是否有预期观测。
- 同一批次记录数量是否异常下降。
- 是否缺少关键实体或关键指标。

对于宏观数据，不应把未发布的新一期误判为缺失。需要结合发布日历和指标频率。

### 5.3 有效性检查

检查内容：

- 数值类型。
- 单位。
- 值域。
- 是否为无穷、空值或明显占位值。

示例：

- 失业率小于 0 或大于 100 阻断。
- VIX 小于 0 阻断。
- 利差极端值进入警告或阻断，阈值按历史分布设定。

### 5.4 一致性检查

检查内容：

- 派生指标与源指标是否一致。
- 同源不同 endpoint 数据是否冲突。
- 近似替代源是否方向一致。

示例：

- `T10Y2Y` 与 `DGS10 - DGS2` 的差异超过容忍阈值。
- CPI 同比与 CPI 指数派生结果差异异常。

### 5.5 可追溯性检查

所有标准化记录必须有：

- `source_id`
- `dataset_id`
- `raw_payload_id`
- `publication_time` 或合理的空值原因
- `config_version`

缺少 `raw_payload_id` 的记录不能进入正式指标表。

## 6. 质量分计算

建议默认公式：

```text
quality_score =
  100
  - freshness_penalty
  - completeness_penalty
  - validity_penalty
  - consistency_penalty
  - traceability_penalty
  - source_health_penalty
```

最低为 0，最高为 100。

关键阻断规则优先于分数：

- 授权禁止。
- 原始响应缺失。
- 必需字段无法解析。
- 单位不明。
- 值域不可能。

## 7. 质量对评分的影响

风险评分聚合时：

- A/B 数据正常参与。
- C 数据权重乘以 0.5 到 0.8。
- D 数据默认不参与整体评分，但可展示。
- F 数据阻断发布或进入隔离。

整体风险输出需要包含：

```text
overall_data_quality_score
low_quality_indicator_count
stale_indicator_count
prototype_source_count
blocked_indicator_count
```

## 8. 前端展示

面板需要展示：

- 总体数据质量灯号。
- 每个数据源最近成功时间。
- 每个指标质量等级。
- 风险评分是否受低质量数据影响。
- 原型数据源提示。

用户看到风险升高时，应能判断这是风险真实变化，还是数据质量异常导致。

## 9. 质量事件

质量问题应生成事件：

- `data_source_delay`
- `indicator_stale`
- `schema_change_detected`
- `quality_gate_blocked`
- `prototype_source_used`
- `license_review_required`

这些事件不等同于金融风险预警，但应该显示在数据源和运维面板。


# 特征库设计

状态：`Draft`

最后更新：2026-05-30

## 1. 目标

定义危机概率系统的 feature store，把观测值和解释性风险分转换为 `5d / 20d / 60d` 模型可直接使用的特征快照。

## 2. 设计原则

- 特征必须可重建
- 特征必须版本化
- 特征生成必须支持 point-in-time
- 同时服务实时评估和历史回测

## 3. 特征层级

### 3.1 L1 原始值

- VIX
- OAS
- 10Y2Y
- STLFSI
- NFCI
- unemployment
- USDJPY
- filing event count

### 3.2 L2 基础派生

- `change_5d`
- `change_20d`
- `change_60d`
- `pct_change_n`
- `rolling_vol_20d`
- `drawdown_252d`
- `yoy`
- `mom`
- `spread`

### 3.3 L3 解释性评分

- `indicator_score`
- `dimension_score`
- `structural_score`
- `trigger_score`
- `external_shock_score`

### 3.4 L4 共振和交互

- `market_credit_coupling`
- `rates_liquidity_coupling`
- `banking_event_market_coupling`
- `us_internal_x_jpy_carry`

## 4. 特征实体

第一阶段固定：

```text
entity_id = us
market_scope = financial_system
```

按交易日生成一行特征快照。

## 5. 表结构

### 5.1 长表

```text
feature_id
entity_id
as_of_date
feature_name
feature_value
feature_group
lookback_window
source_indicator_ids
method_version
quality_score
quality_flags
created_at
```

### 5.2 宽表快照

```text
feature_snapshot_id
entity_id
as_of_date
feature_set_version
prediction_horizon
features_json
coverage_score
created_at
```

## 6. 命名规则

```text
{entity}_{domain}_{concept}_{transform}_{window}
```

示例：

- `us_market_vix_level`
- `us_market_vix_change_5d`
- `us_credit_hy_oas_level`
- `us_external_usdjpy_realized_vol_20d`
- `us_system_trigger_score`

## 7. 生成流程

```mermaid
flowchart LR
    A["indicator observations"] --> B["base transforms"]
    B --> C["score features"]
    C --> D["interaction features"]
    D --> E["feature snapshots"]
```

## 8. Point-in-time 规则

每个 `as_of_date=t`：

- 只能使用 `t` 当天前可见数据
- 宏观数据使用 `publication_time`
- 无发布日期时按保守 lag
- 禁止用未来值

## 9. 缺失值策略

允许：

- 慢变量有限前值保持并打 flag
- 缺失即缺失
- 使用代理变量时显式标记

质量 flags：

- `feature_missing_source`
- `feature_short_history`
- `feature_proxy_used`

## 10. 覆盖率

每个 snapshot 输出：

```text
coverage_score
core_feature_coverage
trigger_feature_coverage
external_feature_coverage
```

## 11. 版本

```text
feature_set_version = feature_v1_YYYYMMDD
```

变化包括：

- 特征增删
- lookback 变化
- 交互公式变化
- 缺失值策略变化

## 12. 运行模式

- 实时评估：生成最新快照
- 历史回测：逐日重放
- 受影响回刷：局部重算

## 13. 与概率模型的接口

模型输入：

```text
feature_snapshot(entity_id, as_of_date, horizon)
```

## 14. 第一阶段优先级

P0：

- VIX / OAS / 10Y2Y / NFCI / STLFSI / unemployment
- banking filing count
- USDJPY / JPY vol / US-JP short rate diff
- structural / trigger / external scores

P1：

- historical analog distance
- richer event severity

## 15. 实现顺序

1. 基础派生特征
2. 评分特征
3. SEC / JPY carry 交互项
4. 宽表 snapshot

## 16. 风险

- 特征过多会过拟合
- 无 point-in-time 控制会产生未来函数
- 事件特征时点可能不稳定

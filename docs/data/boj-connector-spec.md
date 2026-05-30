# BOJ / USDJPY 连接器实现规格

状态：`Draft`

最后更新：2026-05-30

## 1. 目标

实现一个服务于 `JPY carry` 外部风险模块的 `BOJ` 连接器，提供：

- USDJPY / JPYUSD 汇率历史
- 日本短端利率与政策代理
- JPY carry 风险特征输入

第一阶段目标是支撑日频危机概率模型，不是覆盖日本宏观全量数据。

## 2. 范围

### 2.1 第一阶段纳入

- BOJ 日频汇率入口
- BOJ time-series 数据搜索 API
- 日本短端利率 / call rate / 官方近似代理
- 与美国利率构成美日利差特征

### 2.2 第一阶段不纳入

- 日本全量收益率曲线
- 高频 FX tick
- 日本银行体系全量统计

## 3. 数据源结构

核心源：

- BOJ Foreign Exchange Rates (Daily)
- BOJ Time-Series Data Search API

补充源：

- FRED 的日本汇率序列

## 4. 连接器注册

```text
source_id: boj
dataset_id:
  boj_fx_daily
  boj_money_market_rates
```

能力：

- `backfill`
- `incremental`
- `parse_raw`
- `normalize`
- `validate`

## 5. 第一批指标

### 5.1 FX

| 指标 ID | 说明 |
|---|---|
| `us_external_usdjpy_level` | USDJPY 日频水平 |
| `us_external_usdjpy_change_5d` | 派生 |
| `us_external_usdjpy_change_20d` | 派生 |
| `us_external_usdjpy_realized_vol_20d` | 派生 |

### 5.2 日本短端

| 指标 ID | 说明 |
|---|---|
| `jp_rates_call_rate` | 日本无担保隔夜拆借利率或官方代理 |
| `jp_policy_shift_proxy` | BOJ 政策变化代理 |

### 5.3 利差代理

后续特征层派生：

- `us_external_us_jp_short_rate_diff`
- `us_external_us_jp_2y_rate_diff`

## 6. 抓取模式

### 6.1 历史回填

流程：

1. 拉 BOJ FX 历史
2. 拉 BOJ 利率时序
3. 保存原始响应
4. 标准化为日频观测值

### 6.2 增量更新

建议 lookback：

- FX：`10` 个交易日
- 利率：`30` 个自然日

## 7. 原始响应保存

同通用契约：

```text
raw_payload_id
source_id=boj
dataset_id
request_url
response_hash
raw_object_uri
http_status
fetched_at
```

## 8. 标准化规则

### 8.1 频率

- 统一落日频
- 周末和日本节假日保留缺口

### 8.2 汇率方向

系统内部统一为：

```text
USDJPY = JPY per 1 USD
```

若源为 `JPYUSD`，则转换：

```text
USDJPY = 1 / JPYUSD
```

### 8.3 时区

- 按日本本地发布日期解释
- 入库使用 `as_of_date`

## 9. 数据质量规则

FX：

- 汇率必须大于 `0`
- 大幅跳变标记，不自动判错

利率：

- 利率范围合理性检查
- 负利率允许

质量 flags：

- `boj_quote_inverted`
- `boj_holiday_gap`
- `boj_large_fx_move`
- `boj_partial_history`
- `boj_rate_jump`

## 10. 回退策略

当 BOJ FX 数据不可用时：

- 可回退到 FRED FX 代理
- 必须显式切换 source metadata

## 11. 存储落点

标准化观测值写入：

```text
ts_indicator_observations
```

后续由 feature store 派生：

- `change_5d`
- `change_20d`
- `realized_vol_20d`
- 美日利差
- USDJPY 与 VIX / OAS 联动特征

## 12. Worker 命令建议

```text
backfill boj --dataset fx-daily --start YYYY-MM-DD --end YYYY-MM-DD
backfill boj --dataset money-market --start YYYY-MM-DD --end YYYY-MM-DD
backfill jpy-carry
```

## 13. 测试要求

- FX 正常样本
- 报价方向转换样本
- 日本节假日缺口样本
- 利率负值样本
- `404 / 429 / schema_changed` 样本

## 14. 实现顺序

1. 先完成 FX 数据接入
2. 再完成日本短端利率
3. 最后交给特征层派生 carry 特征

## 15. 风险

- BOJ 数据入口结构可能变化
- 不同接口的报价方向可能不同
- 长历史字段稳定性需要单独验证

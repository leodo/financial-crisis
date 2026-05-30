# FRED 首个真实连接器实现规格

状态：`Draft`

最后更新：2026-05-30

## 1. 目标

FRED 是第一个要打通的真实免费历史数据源。目标是用官方 API 建立可重复的历史回填链路，让系统从 demo 分数进入真实历史分位评分。

## 2. 前置条件

- 用户申请 FRED 免费 API key。
- 本地设置环境变量：

```text
FRED_API_KEY=<key>
FC_DATA_MODE=sqlite
FC_SQLITE_PATH=data/fc-local.sqlite
```

- 已执行 SQLite migration 和 metadata seed。

## 3. API 使用

使用 endpoint：

```text
https://api.stlouisfed.org/fred/series/observations
```

请求参数：

```text
series_id
api_key
file_type=json
observation_start
observation_end
```

保留字段：

```text
realtime_start
realtime_end
date
value
```

解析规则：

- `value = "."` 视为缺失，不写入有效观测。
- `date` 解析为 `as_of_date`。
- `realtime_end` 第一版写入 `revision_time`。
- `publication_time` 第一版使用 payload fetched time。
- `raw_payload_id` 或 `raw_response_id` 必须可回链原始响应。

## 4. 第一批 series mapping

第一批只选择免费、历史较长、解释清楚、和金融危机风险机制直接相关的 FRED 序列。

| 内部指标 ID | FRED series | 频率 | 变换 | 风险方向 | 说明 |
|---|---|---|---|---|---|
| `us_market_vix_close` | `VIXCLS` | daily | level | higher_is_riskier | 市场恐慌和波动率 |
| `us_credit_high_yield_oas` | `BAMLH0A0HYM2` | daily | level | higher_is_riskier | 高收益债信用压力 |
| `us_credit_baa_10y_spread` | `BAA10Y` | daily | level | higher_is_riskier | 投资级信用压力代理 |
| `us_rates_yield_curve_10y2y` | `T10Y2Y` | daily | level | lower_is_riskier | 期限结构倒挂 |
| `us_liquidity_financial_stress_stl` | `STLFSI4` | weekly | level | higher_is_riskier | 圣路易斯金融压力指数 |
| `us_liquidity_national_financial_conditions` | `NFCI` | weekly | level | higher_is_riskier | 全国金融条件指数 |
| `us_macro_unemployment_rate` | `UNRATE` | monthly | level | higher_is_riskier | 宏观脆弱性 |
| `us_macro_cpi_yoy` | `CPIAUCSL` | monthly | yoy | two_sided | 通胀偏离 |
| `us_macro_industrial_production_mom` | `INDPRO` | monthly | mom | lower_is_riskier | 实体经济下行 |
| `us_macro_real_gdp_growth` | `GDPC1` | quarterly | yoy | lower_is_riskier | 实际 GDP 增速 |
| `us_liquidity_sofr` | `SOFR` | daily | level_or_change_20d | rising_fast_is_riskier | 短端融资利率压力 |
| `us_liquidity_effr` | `EFFR` | daily | level_or_change_20d | rising_fast_is_riskier | 有效联邦基金利率 |
| `us_liquidity_fed_balance_sheet` | `WALCL` | weekly | pct_change_13w | falling_fast_is_riskier | Fed 资产负债表收缩 |
| `us_liquidity_money_supply_m2` | `M2SL` | monthly | yoy | falling_fast_is_riskier | 货币供给收缩 |
| `us_real_estate_housing_starts_yoy` | `HOUST` | monthly | yoy | falling_fast_is_riskier | 住宅开工下滑 |
| `us_real_estate_home_price_yoy` | `CSUSHPISA` | monthly | yoy | two_sided | 房价过热或下跌 |
| `us_banking_commercial_real_estate_loans` | `CREACBW027SBOG` | weekly | yoy | rising_fast_is_riskier | 商业地产贷款扩张压力 |
| `us_banking_deposits_growth` | `DPSACBW027SBOG` | weekly | yoy | falling_fast_is_riskier | 银行存款流失压力 |

注意：

- `transform != level` 的指标需要 feature builder，不应直接用原始值评分。
- 如果某个 series 不存在或权限变化，连接器应将其标记为 `failed_terminal`，不影响其他 series。
- 房价数据可能有授权说明，进入生产前需要单独检查 FRED/source note。

## 5. 数据库写入

### 5.1 Metadata seed

必须写入：

```text
metadata_sources(source_id='fred')
metadata_datasets(dataset_id='fred_series_observations')
metadata_indicators(...)
metadata_external_indicator_mappings(...)
metadata_entities(entity_id='us')
```

### 5.2 Raw response

每次请求保存：

```text
raw_file_path
request_url
request_params_hash
response_hash
content_type
fetched_at
```

文件路径建议：

```text
data/raw/fred/<series_id>/<yyyy>/<request_hash>.json.gz
```

### 5.3 Observations

写入 `ts_indicator_observations`。

唯一键：

```text
indicator_id + entity_id + as_of_date + source_id + vintage_date
```

如果第一版没有显式 `vintage_date` 列，可以临时用 `revision_time` 参与唯一键，但 SQLite 设计应预留 `vintage_date`。

## 6. Feature builder

第一版至少支持：

| transform | 计算 |
|---|---|
| `level` | 直接使用原始值 |
| `yoy` | 当前值 / 12 个月前值 - 1 |
| `mom` | 当前值 / 1 个月前值 - 1 |
| `pct_change_13w` | 当前值 / 13 周前值 - 1 |
| `level_or_change_20d` | 保留 level，同时后续可增加 20 日变化 |

feature 输出要记录：

```text
feature_name
lookback_window
method_version
source_observation_count
quality_score
```

## 7. 错误处理

| 情况 | 处理 |
|---|---|
| 缺少 `FRED_API_KEY` | 命令失败并提示配置方式 |
| HTTP 429 | `rate_limited`，指数退避 |
| HTTP 401/403 | `auth_failed`，停止该源 |
| series 不存在 | `invalid_request`，该 mapping 禁用或进入告警 |
| value 为 `.` | 跳过观测并记录 warning |
| JSON schema 变化 | `schema_changed`，保留 raw 并阻止 publish |
| 单个 series 失败 | 不阻塞其他 series |

## 8. 测试要求

单元测试：

- URL 构造包含 `series_id`、`file_type=json`、start/end。
- FRED JSON fixture 能解析为 observations。
- `.` 缺失值会跳过并产生 warning。
- transform `yoy`、`mom`、`pct_change_13w` 计算正确。

集成测试：

- SQLite 临时库 migration 成功。
- seed 后 mapping 数量正确。
- fixture payload 写入 raw index 和 observations。
- 重复写入不增加重复行。

手工验证：

```text
just db-init
just db-seed
just backfill-fred
FC_DATA_MODE=sqlite just api
```

## 9. 完成标准

- 至少 10 个 FRED series 成功回填。
- 每个已回填指标在 SQLite 中有最新值和历史序列。
- `/api/overview` 在 SQLite 模式下不再使用 demo 数据。
- Web 面板能展示真实数据源状态和真实历史分位。
- README 包含 FRED API key 配置说明。

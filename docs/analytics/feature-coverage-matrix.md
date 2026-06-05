# 特征覆盖矩阵

状态：`Draft`

最后更新：2026-05-31

## 1. 目标

把“哪些特征能进入正式模型、最早能回到哪一年、point-in-time 能力到什么程度”一次性说清楚。

这份文档解决三个问题：

1. `formal_v1` 到底能用哪些核心特征。
2. `1990+` 主面板和 `1987 / 1998` 扩展场景，哪些特征可以共用，哪些只能降级代理。
3. 哪些特征只适合 UI 解释，不适合进正式训练。

## 2. 设计原则

- 先按“可训练性”分层，再按“研究价值”分层。
- `formal_v1` 只收 `1990+` 可持续、可解释、可回填的特征。
- `1987` 等早期场景允许用代理特征包，但不和 `1990+` 主面板混成同一套完整训练输入。
- 没有 point-in-time 方案的特征，不进入 `strict` 训练集。
- 历史覆盖不稳定的特征，只能做增强或解释，不做主轴。

## 3. 覆盖等级

### 3.1 `core_1990`

满足以下条件：

- 最迟在 `1990-01-02` 前后可稳定覆盖；
- 免费官方源可持续回填；
- 至少具备 `best_effort` point-in-time；
- 经济含义清楚，能长期解释。

这类特征可进入 `formal_v1_main_1990_daily`。

### 3.2 `extension_pre1990`

满足以下条件：

- 可回到 `1987` 或更早；
- 但频率、代理关系或 point-in-time 能力不如主面板；
- 适合扩展场景回放、历史类比和急性冲击研究。

这类特征进入扩展场景包，不直接并入 `formal_v1_main_1990_daily`。

### 3.3 `explain_only`

满足以下任一条件：

- 历史覆盖太短；
- 源站返回窗口不稳定；
- 噪声高；
- 只能解释，不能稳定训练。

这类特征只用于 UI、审计和人工复核。

## 4. Point-in-time 能力等级

| 等级 | 含义 |
|---|---|
| `strict` | 有官方发布时间或接受时间，可精确恢复当时是否可见 |
| `best_effort` | 能根据官方日期、固定 cutover 或保守 lag 近似恢复 |
| `weak` | 只能大概知道历史值，严格可见性弱 |
| `none` | 只能做解释或原型，不进入正式训练 |

## 5. `formal_v1` 主面板核心矩阵

| 特征 ID | 经济含义 | 主源 | 频率 | 已确认最早日期 | PIT 等级 | `1990+` 主面板 | `1987` 扩展 | 角色 | 备注 |
|---|---|---|---|---|---|---|---|---|---|
| `us_vix_level` | 市场恐慌水平 | FRED `VIXCLS` | 日 | `1990-01-02` | `best_effort` | 是 | 否 | `core_1990` | 急性冲击最关键快变量之一 |
| `us_vix_change_5d` | 恐慌抬升速度 | `VIXCLS` 派生 | 日 | `1990-01-09` | `best_effort` | 是 | 否 | `core_1990` | `p_5d` 核心 |
| `us_treasury_10y_level` | 长端利率水平 | Treasury 日频曲线 | 日 | `1990-01-02` | `best_effort` | 是 | 代理可用 | `core_1990` | `1987` 用 `GS10` 月频代理 |
| `us_treasury_2y_level` | 短端利率水平 | Treasury 日频曲线 | 日 | `1990-01-02` | `best_effort` | 是 | 代理可用 | `core_1990` | `1987` 用 `GS2` 月频代理 |
| `us_curve_10y2y` | 期限利差 | 10Y/2Y 派生 | 日 | `1990-01-02` | `best_effort` | 是 | 代理可用 | `core_1990` | 结构脆弱性主轴 |
| `us_baa_10y_spread` | 信用压力代理 | FRED `BAA10Y` | 日 | `1986-01-02` | `best_effort` | 是 | 是 | `core_1990` | `1987` 可直接使用 |
| `us_fed_funds_level` | 政策/资金价格 | FRED `DFF` 或 `FEDFUNDS` | 日 / 月 | `1954-07-01` | `best_effort` | 是 | 是 | `core_1990` | `1987` 可用，频率按源选择 |
| `us_nfci_level` | 综合金融条件 | FRED `NFCI` | 周 | `1971-01-08` | `best_effort` | 是 | 是 | `core_1990` | 主面板与扩展场景均有价值 |
| `us_stlfsi_level` | 金融压力指数 | FRED `STLFSI4` | 周 | `1993-12-31` | `best_effort` | 是 | 否 | `core_1990` | 不能覆盖 `1987`；formal main 仅从 `1993-12-31` 起把它计入核心/触发硬覆盖 |
| `us_unemployment_level` | 劳动力脆弱性 | FRED `UNRATE` | 月 | `1948-01-01` | `best_effort` | 是 | 是 | `core_1990` | 慢变量 |
| `us_industrial_production_level` | 周期动能 | FRED `INDPRO` | 月 | `1919-01-01` | `best_effort` | 是 | 是 | `core_1990` | 慢变量 |
| `us_housing_starts_level` | 房地产动能 | FRED `HOUST` | 月 | `1959-01-01` | `best_effort` | 是 | 是 | `core_1990` | 慢变量 |
| `us_home_price_case_shiller_level` | 房价周期 | FRED `CSUSHPISA` | 月 | `1987-01-01` | `best_effort` | 是 | 部分 | `core_1990` | 仅能覆盖 `1987` 年内末段 |
| `us_usdjpy_level` | 外部融资与套息环境 | FRED `DEXJPUS` / BOJ | 日 | `1971-01-04` | `best_effort` | 是 | 是 | `core_1990` | JPY carry 主轴 |
| `us_usdjpy_change_20d` | 汇率冲击速度 | `DEXJPUS` / BOJ 派生 | 日 | `1971-02-01` 左右 | `best_effort` | 是 | 是 | `core_1990` | 外部冲击放大器 |

## 6. 增强特征矩阵

| 特征 ID | 主源 | 频率 | 已确认最早日期 | PIT 等级 | 训练角色 | 说明 |
|---|---|---|---|---|---|---|
| `us_high_yield_oas_level` | FRED `BAMLH0A0HYM2` | 日 | 当前免费 CSV 仅见 `2023-05-30` 起 | `weak` | `explain_only` / 后续复核 | 历史窗口不稳定，不能作为 `formal_v1` 主轴 |
| `us_sec_banking_event_count` | SEC EDGAR | 事件 / 日聚合 | `1994+` | `strict` | `enhancement` | 适合 `2008 / 2023`，不覆盖 `1987` |
| `us_sec_liquidity_keyword_score` | SEC EDGAR | 事件 / 日聚合 | `1994+` | `strict` | `enhancement` | 事件确认层 |
| `us_gdelt_financial_stress_count` | GDELT | 日 | `2015+` | `none` | `explain_only` | 只能做辅助原型，不进正式模型 |
| `us_jpy_carry_rate_diff` | FRED + BOJ | 日 / 月 | `待按 BOJ/FRED 对齐验证` | `best_effort` | `enhancement` | 有研究价值，但先不列为主面板硬依赖；formal main 覆盖门槛不以 `jp_rates_call_rate` 为必要条件 |
| `us_banking_event_market_coupling` | SEC + 市场特征派生 | 日 | 取决于底层 | 取决于底层 | `enhancement` | 第二阶段交互项 |

## 7. `1987 / 1998` 扩展场景代理包

这些特征用于早期场景，不和 `formal_v1_main_1990_daily` 混做一个统一宽表。

| 代理特征 | 主源 | 最早日期 | 用途 | 限制 |
|---|---|---|---|---|
| `proxy_gs10_level` | FRED `GS10` | `1953-04-01` | 长端利率代理 | 月频，不等价于日频 Treasury 曲线 |
| `proxy_gs2_level` | FRED `GS2` | `1976-06-01` | 短端利率代理 | 月频 |
| `proxy_curve_10y2y_monthly` | `GS10 - GS2` | `1976-06-01` | 期限利差代理 | 月频 |
| `proxy_baa_10y_spread` | FRED `BAA10Y` | `1986-01-02` | 信用压力代理 | 可直接覆盖 `1987` |
| `proxy_fed_funds_level` | `DFF` / `FEDFUNDS` | `1954-07-01` | 资金价格代理 | 无事件确认层 |
| `proxy_nfci_level` | `NFCI` | `1971-01-08` | 综合金融条件 | 周频 |
| `proxy_usdjpy_level` | `DEXJPUS` | `1971-01-04` | 外部融资压力 | 汇率维度可用 |
| `proxy_unemployment_level` | `UNRATE` | `1948-01-01` | 慢变量背景 | 不适合急性冲击短窗单独驱动 |

## 8. `formal_v1` 的硬门槛

只有同时满足以下条件的特征，才允许进入第一版正式训练：

1. 分类为 `core_1990`
2. `1990+` 历史覆盖稳定
3. PIT 等级至少 `best_effort`
4. 经济含义可解释
5. 当前免费源不会只返回最近两三年窗口

因此第一版建议的最小核心集是：

- `us_vix_level`
- `us_vix_change_5d`
- `us_treasury_10y_level`
- `us_treasury_2y_level`
- `us_curve_10y2y`
- `us_baa_10y_spread`
- `us_fed_funds_level`
- `us_nfci_level`
- `us_stlfsi_level`
- `us_unemployment_level`
- `us_industrial_production_level`
- `us_housing_starts_level`
- `us_usdjpy_level`

## 9. 当前不该做的事

- 不要把 `BAMLH0A0HYM2` 这种当前免费窗口很短的序列硬塞进正式训练。
- 不要为了让 `1987` 进样本，就把 `1990+` 主面板全部降级成月频。
- 不要把 `SEC / GDELT` 这种晚起步事件层当成全时期统一主特征。
- 不要把“能画在前端上”误当成“能稳定进训练”。

## 10. 对实现的直接要求

实现 `raw feature store` 时，至少要有这几个字段：

```text
feature_name
source_id
frequency
earliest_reliable_date
point_in_time_grade
coverage_class
usable_for_formal_v1
usable_for_pre1990_extension
proxy_of nullable
```

## 11. 下一步

按这份矩阵，后续开发顺序应该是：

1. 先落 `core_1990` 原始观测与派生特征；
2. 再落 `1987 / 1998` 扩展场景代理包；
3. 最后再考虑事件层交互特征是否进入正式训练。

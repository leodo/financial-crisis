# 指标体系设计

状态：`Draft`

最后更新：2026-05-30

## 1. 目标

定义金融危机预警系统的指标树、指标命名、分组、初始指标清单和扩展规则。指标体系要同时支持整体风险评估、分项风险评估、指标下钻和历史回测。

## 2. 指标体系原则

- 指标按风险机制分组，而不是按数据源分组。
- 内部指标 ID 独立于外部数据源代码。
- 每个指标必须声明风险方向、频率、单位、默认来源和质量等级。
- 慢变量和快变量分开建模，最后再汇总。
- 第一版优先选择可解释、历史较长、免费数据源可覆盖的指标。

## 3. 一级风险维度

```text
overall
  macro_fragility       宏观脆弱性
  leverage_credit       杠杆与信用
  market_stress         市场压力
  liquidity_funding     流动性与融资
  banking_system        银行体系
  real_estate           房地产与资产泡沫
  external_sector       外部部门与汇率
  events_sentiment      事件与情绪
  data_quality          数据质量，不直接代表金融风险
```

## 4. 指标命名规则

内部指标 ID：

```text
{region}_{dimension}_{concept}_{variant}
```

示例：

- `us_market_vix_close`
- `us_credit_high_yield_oas`
- `us_rates_yield_curve_10y2y`
- `us_macro_unemployment_rate`
- `global_external_current_account_gdp`

外部代码放入映射表：

```text
indicator_id: us_market_vix_close
source_id: fred
external_code: VIXCLS
```

## 5. 初始 MVP 指标树

### 5.1 宏观脆弱性

| 指标 ID | 说明 | 频率 | 首选来源 | 风险方向 |
|---|---|---|---|---|
| `us_macro_real_gdp_growth` | 实际 GDP 增速 | 季 | FRED | 越低越危险 |
| `us_macro_unemployment_rate` | 失业率 | 月 | FRED | 越高越危险 |
| `us_macro_cpi_yoy` | CPI 同比 | 月 | FRED 派生 | 双向偏离危险 |
| `us_macro_industrial_production_mom` | 工业产出环比 | 月 | FRED 派生 | 越低越危险 |
| `global_macro_gdp_growth` | 全球/国家 GDP 增速 | 年 | World Bank | 越低越危险 |
| `global_macro_inflation_yoy` | 全球/国家通胀 | 年 | World Bank | 双向偏离危险 |

### 5.2 杠杆与信用

| 指标 ID | 说明 | 频率 | 首选来源 | 风险方向 |
|---|---|---|---|---|
| `us_credit_high_yield_oas` | 高收益债 OAS | 日 | FRED | 越高越危险 |
| `us_credit_baa_10y_spread` | Baa 与 10Y 国债利差 | 日 | FRED | 越高越危险 |
| `us_credit_bank_loans_growth` | 银行贷款增速 | 周/月 | FRED | 过快上升或快速下滑危险 |
| `global_credit_private_nonfinancial_gdp` | 私营非金融部门信贷/GDP | 季 | BIS | 越高越危险 |
| `global_credit_gap` | 信贷/GDP 缺口 | 季 | BIS | 越高越危险 |

### 5.3 市场压力

| 指标 ID | 说明 | 频率 | 首选来源 | 风险方向 |
|---|---|---|---|---|
| `us_market_vix_close` | VIX 收盘价 | 日 | FRED | 越高越危险 |
| `us_market_equity_drawdown` | 股指回撤 | 日 | 市场数据源 | 越低越危险 |
| `us_market_equity_realized_vol` | 股指实现波动率 | 日 | 市场数据源 | 越高越危险 |
| `us_rates_yield_curve_10y2y` | 10Y-2Y 期限利差 | 日 | FRED | 越低越危险 |
| `us_rates_10y_change_20d` | 10Y 收益率 20 日变化 | 日 | FRED 派生 | 快速上升危险 |

### 5.4 流动性与融资

| 指标 ID | 说明 | 频率 | 首选来源 | 风险方向 |
|---|---|---|---|---|
| `us_liquidity_sofr` | SOFR | 日 | FRED | 快速上升危险 |
| `us_liquidity_effr` | 有效联邦基金利率 | 日 | FRED | 快速上升危险 |
| `us_liquidity_fed_balance_sheet` | Fed 总资产 | 周 | FRED | 快速收缩需关注 |
| `us_liquidity_money_supply_m2` | M2 | 月 | FRED | 快速收缩危险 |
| `us_liquidity_financial_stress_stl` | 圣路易斯金融压力指数 | 周 | FRED | 越高越危险 |
| `us_liquidity_national_financial_conditions` | NFCI | 周 | FRED | 越高越危险 |

### 5.5 银行体系

| 指标 ID | 说明 | 频率 | 首选来源 | 风险方向 |
|---|---|---|---|---|
| `us_banking_deposits_growth` | 银行存款增速 | 周/月 | FRED | 快速下降危险 |
| `us_banking_commercial_real_estate_loans` | 商业地产贷款 | 周/月 | FRED | 过快上升危险 |
| `us_banking_filing_stress_count` | 银行风险公告数量 | 日/周 | SEC | 越高越危险 |
| `global_banking_cross_border_claims` | 跨境银行债权 | 季 | BIS | 过快扩张危险 |

### 5.6 房地产与资产泡沫

| 指标 ID | 说明 | 频率 | 首选来源 | 风险方向 |
|---|---|---|---|---|
| `us_real_estate_home_price_yoy` | 房价同比 | 月 | FRED 派生 | 过快上升或快速下跌危险 |
| `us_real_estate_housing_starts_yoy` | 新屋开工同比 | 月 | FRED 派生 | 快速下跌危险 |
| `global_real_estate_price_growth` | 房地产价格增速 | 季 | BIS | 过快上升或快速下跌危险 |

### 5.7 外部部门与汇率

| 指标 ID | 说明 | 频率 | 首选来源 | 风险方向 |
|---|---|---|---|---|
| `global_external_current_account_gdp` | 经常账户/GDP | 年/季 | World Bank/IMF | 越低越危险 |
| `global_external_reserves_months_imports` | 外储覆盖进口月数 | 月/年 | IMF/World Bank | 越低越危险 |
| `global_fx_usd_change_20d` | 本币兑美元 20 日变化 | 日 | 市场数据源 | 快速贬值危险 |
| `global_fx_real_effective_exchange_rate` | 实际有效汇率 | 月 | BIS/IMF | 快速升贬值需关注 |

### 5.8 事件与情绪

| 指标 ID | 说明 | 频率 | 首选来源 | 风险方向 |
|---|---|---|---|---|
| `us_event_bank_8k_count` | 银行 8-K 公告数量 | 日/周 | SEC | 越高越危险 |
| `us_event_risk_keyword_count` | 风险关键词公告数量 | 日/周 | SEC | 越高越危险 |
| `global_news_financial_stress_count` | 金融压力新闻数量 | 日 | GDELT | 越高越危险 |
| `global_news_negative_tone` | 负面新闻语调 | 日 | GDELT | 越高越危险 |

## 6. 指标元数据要求

每个指标必须定义：

```text
indicator_id
display_name
dimension
subdimension
description
unit
frequency
risk_direction
default_transform
default_source
fallback_source
quality_tier
release_lag
history_start_target
```

## 7. 派生指标

派生指标要记录公式和依赖：

```text
derived_indicator_id
formula
input_indicators
lookback_window
calendar_policy
missing_value_policy
method_version
```

示例：

- `us_macro_cpi_yoy = pct_change(CPIAUCSL, 12 months)`
- `us_market_equity_drawdown = price / rolling_max(price, 252d) - 1`
- `us_rates_10y_change_20d = DGS10 - lag(DGS10, 20 trading days)`

## 8. 频率对齐

评分日期通常为日频。

处理规则：

- 日频指标：按交易日更新。
- 周频指标：最近一期向前填充，但超过 freshness SLO 后降权。
- 月频/季频指标：最近一期向前填充，明确标记 `slow_variable`。
- 年频指标：只用于结构性脆弱性，不用于短期触发。

慢变量不能因为当天没有更新而重复触发预警，只能影响风险底座。

## 9. 扩展规则

新增指标前必须回答：

- 该指标对应哪个风险机制？
- 是否有稳定数据源？
- 历史长度是否足够回测？
- 是否与已有指标高度重复？
- 评分方向是否明确？
- 数据质量是否可检查？
- 是否需要授权？

不满足这些问题的指标可以进入研究层，但不要进入生产评分。


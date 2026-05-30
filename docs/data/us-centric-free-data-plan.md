# 美国主线免费数据方案

状态：`Draft`

最后更新：2026-05-30

## 1. 目标

定义一套以美国金融危机概率评估为目标的免费数据方案，满足三个要求：

1. 能支撑真实历史回测。
2. 能支撑日频持续更新。
3. 尽量依赖官方或稳定公开源，不依赖脆弱爬虫。

## 2. 设计原则

- 官方优先，免费优先，历史可回填优先。
- 先满足美国主线，再补全球辅助。
- 快变量优先覆盖风险触发，慢变量优先覆盖结构脆弱性。

## 3. 核心数据源

### 3.1 FRED

用途：

- 美国宏观
- 利率
- 信用利差
- 金融压力
- 部分汇率和国际金融序列

接入方式：

- 首选 FRED Graph CSV
- 可选 FRED API 作为增强

适用模块：

- 规则评分层
- 概率模型特征
- 历史回测

### 3.2 U.S. Treasury

用途：

- 官方国债收益率曲线
- 期限利差兜底

接入方式：

- 官方 XML / Daily Treasury Rates

适用模块：

- 利率冲击
- 曲线倒挂
- 利率变化率

### 3.3 SEC EDGAR

用途：

- 银行与金融机构事件
- 8-K、10-Q、10-K、风险因子变化

接入方式：

- 官方 submissions / companyfacts / filing APIs

适用模块：

- `banking_funding_stress`
- 事件层
- `p_5d` / `p_20d`

### 3.4 World Bank

用途：

- 年频慢变量
- 美国与全球背景对比

适用模块：

- 结构脆弱性
- 历史背景补充

### 3.5 BOJ

用途：

- 日元汇率
- 日本货币市场与利率
- 日元套息专题

接入方式：

- BOJ Time-Series Data Search API
- BOJ FX Daily 页面和长历史数据

适用模块：

- 外部冲击
- JPY carry stress

## 4. 数据层级

### 4.1 P0：必须先接入

| Source | 目的 |
|---|---|
| FRED | 美国主线大部分指标 |
| Treasury | 利率和收益率曲线 |
| SEC EDGAR | 银行和金融机构事件 |
| BOJ / FRED FX | USDJPY 和日本利率/FX专题 |

### 4.2 P1：补强

| Source | 目的 |
|---|---|
| World Bank | 年频背景与对比 |
| BIS | 信贷/GDP、跨境银行数据 |
| IMF | 外储、国际收支、外部脆弱性 |

### 4.3 P2：原型或补充

| Source | 目的 |
|---|---|
| Stooq | 免费市场价格补充 |
| Alpha Vantage | 补充市场价格 |
| yfinance | 原型验证，不作为核心源 |

## 5. 数据集分组

### 5.1 结构脆弱性

- GDP、失业率、工业产出
- 银行贷款、存款
- 房价、新屋开工
- 全球/美国慢变量

### 5.2 触发压力

- VIX
- 高收益债 OAS
- Baa-10Y spread
- 10Y、2Y、10Y-2Y
- SOFR、EFFR、NFCI、STLFSI

### 5.3 事件层

- 银行业 8-K
- 风险关键词
- 资本、流动性、融资、存款相关披露

### 5.4 外部冲击层

- USDJPY
- 美日 2Y 利差
- JPY FX 波动
- BOJ 短端利率

## 6. SQLite 是否足够

结论：第一阶段足够。

SQLite 可以承担：

- 指标元数据
- 观测值
- 原始抓取日志
- 特征快照
- 标签表
- 回测结果摘要

不适合的场景：

- 高频 tick 级行情
- 大规模并行研究
- 超大多资产分钟级历史库

## 7. 数据新鲜度目标

| 层级 | 目标 |
|---|---|
| 日频核心市场指标 | T+0 或 T+1 |
| 公告事件 | 当日或次日 |
| 周频/宏观 | 按官方发布日期 |

## 8. 数据质量要求

核心字段必须带：

```text
source_id
external_code
as_of_date
observed_at
published_at
ingested_at
quality_score
quality_flags
revision_time
```

## 9. 开发优先顺序

1. FRED + Treasury 继续扩充。
2. 补 SEC EDGAR 真实连接器。
3. 接入 USDJPY 和 BOJ 利率。
4. 再补 BIS / IMF / World Bank。

## 10. 主要风险

- 免费市场价格源在极少数指标上不稳定。
- 事件层解析成本高于时序层。
- 部分信用序列历史覆盖不完整，需要代理变量。

## 11. 参考入口

- [FRED Graph / Export](https://fredhelp.stlouisfed.org/fred/graphs/share-my-fred-graph/export-options/)
- [U.S. Treasury Daily Rates](https://home.treasury.gov/resource-center/data-chart-center/interest-rates/TextView?type=daily_treasury_yield_curve)
- [Treasury XML Feed](https://home.treasury.gov/treasury-daily-interest-rate-xml-feed)
- [SEC EDGAR APIs](https://www.sec.gov/edgar/sec-api-documentation)
- [World Bank Indicators API](https://datahelpdesk.worldbank.org/knowledgebase/articles/889392)
- [BOJ FX Daily](https://www.boj.or.jp/en/statistics/market/forex/fxdaily/index.htm)
- [BOJ Time-Series API notice](https://www.boj.or.jp/en/statistics/outline/notice_2026/not260218a.htm)

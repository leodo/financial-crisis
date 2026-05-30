# 免费数据源目录

状态：`Draft`

最后更新：2026-05-30

## 1. 目标

建立第一版免费或低成本数据源目录，支撑金融危机预警系统的 MVP。目录只记录候选数据源和接入策略，不代表所有数据都已经完成授权审查或生产可用验证。

## 2. 数据源准入标准

P0 数据源必须尽量满足：

- 有官方 API、官方下载入口或稳定批量文件。
- 有明确的数据字段、更新时间和错误响应。
- 可以支撑历史回填。
- 不需要绕过反爬机制。
- 可在连接器层记录授权和使用限制。

P1 数据源可以用于原型，但进入生产前必须额外审查：

- 免费层限流。
- 商业使用限制。
- 是否为非官方封装。
- 数据完整性和延迟。

P2 数据源只作为后续增强或商业替代方案。

## 3. 第一批数据源

| 优先级 | Source ID | 数据源 | 类型 | 主要用途 | 接入方式 | 生产判断 |
|---|---|---|---|---|---|---|
| P0 | `fred` | FRED | 宏观/金融时序 | 美国利率、信用、宏观、压力指标 | 官方 REST API | 可作为核心源 |
| P0 | `sec_edgar` | SEC EDGAR | 公告/事件 | 上市公司申报、风险事件 | 官方 JSON API | 可作为核心源 |
| P0 | `world_bank` | World Bank Indicators | 全球宏观 | GDP、通胀、失业、外部指标 | 官方 API | 可作为核心源 |
| P0 | `imf` | IMF Data | 全球宏观金融 | 国际收支、外储、财政、金融统计 | 官方 API/SDMX | 可作为核心源 |
| P0 | `bis` | BIS Statistics | 银行/信贷/房地产 | 信贷、债务、房地产、银行统计 | 官方 API/SDMX | 可作为核心源 |
| P1 | `ecb` | ECB Data Portal | 欧元区 | 货币、利率、金融和宏观数据 | 官方 API/SDMX | 欧洲模块核心源 |
| P1 | `oecd` | OECD Data | OECD 宏观 | OECD 国家补充指标 | 官方 API/SDMX | 补充源 |
| P1 | `gdelt` | GDELT | 新闻/事件 | 全球新闻检索、情绪和主题 | DOC API | 事件原型源 |
| P1 | `alpha_vantage` | Alpha Vantage | 市场价格 | 股票、FX、部分宏观补充 | 官方 API | 原型或低频补充 |
| P1 | `yfinance` | yfinance | 市场价格 | Yahoo Finance 数据下载 | 非官方封装 | 仅研发原型 |
| P2 | `nasdaq_data_link` | Nasdaq Data Link | 多类数据集 | 扩展数据集 | 官方 API | 按数据集审查 |
| P2 | `treasury_fiscaldata` | Fiscal Data | 美国财政 | 债务、财政和国债相关数据 | 官方 API | 补充源 |

官方资料入口：

- FRED API: <https://fred.stlouisfed.org/docs/api/fred/>
- SEC EDGAR APIs: <https://www.sec.gov/search-filings/edgar-application-programming-interfaces>
- World Bank Indicators API: <https://datahelpdesk.worldbank.org/knowledgebase/articles/889392>
- IMF API: <https://data.imf.org/en/Resource-Pages/IMF-API>
- BIS Statistics API: <https://stats.bis.org/api-doc/v1/>
- ECB Data API: <https://data.ecb.europa.eu/help/api/data>
- OECD API: <https://www.oecd.org/en/data/insights/data-explainers/2024/09/api.html>
- Alpha Vantage API: <https://www.alphavantage.co/documentation/>
- GDELT DOC API: <https://blog.gdeltproject.org/gdelt-doc-2-0-api-debuts/amp/>
- Nasdaq Data Link Docs: <https://docs.data.nasdaq.com/>

## 4. P0 数据源细分

### 4.1 FRED

用途：

- 美国宏观慢变量。
- 市场和金融压力日频指标。
- 利率、期限利差、信用利差、货币和银行类指标。

第一批候选指标：

| 指标 ID | FRED series | 说明 | 频率 | 风险方向 |
|---|---|---|---|---|
| `us_yield_10y` | `DGS10` | 10 年期美国国债收益率 | 日 | 双向，快速上升或倒挂相关 |
| `us_yield_2y` | `DGS2` | 2 年期美国国债收益率 | 日 | 双向 |
| `us_yield_curve_10y2y` | `T10Y2Y` | 10Y-2Y 期限利差 | 日 | 越低越危险 |
| `us_high_yield_oas` | `BAMLH0A0HYM2` | 美国高收益债 OAS | 日 | 越高越危险 |
| `us_baa_10y_spread` | `BAA10Y` | Baa 企业债与 10Y 国债利差 | 日 | 越高越危险 |
| `us_vix` | `VIXCLS` | VIX 收盘价 | 日 | 越高越危险 |
| `us_financial_stress_stl` | `STLFSI4` | St. Louis Financial Stress Index | 周 | 越高越危险 |
| `us_national_financial_conditions` | `NFCI` | Chicago Fed NFCI | 周 | 越高越危险 |
| `us_unemployment` | `UNRATE` | 失业率 | 月 | 越高越危险 |
| `us_cpi_yoy_source` | `CPIAUCSL` | CPI 指数，系统内转同比 | 月 | 偏离目标越危险 |
| `us_industrial_production` | `INDPRO` | 工业产出指数 | 月 | 快速下行危险 |
| `us_real_gdp` | `GDPC1` | 实际 GDP | 季 | 快速下行危险 |
| `us_housing_starts` | `HOUST` | 新屋开工 | 月 | 快速下行危险 |
| `us_home_price_case_shiller` | `CSUSHPISA` | Case-Shiller 房价指数 | 月 | 过快上升或下跌危险 |

接入策略：

- 使用 `series/observations` 拉取观测值。
- 使用 `series` 或 `series/search` 获取元数据。
- 增量抓取以 `observation_start` 和最后发布日期为水位。
- 保存 FRED 的 `realtime_start`、`realtime_end`，为后续 point-in-time 回测预留空间。

主要风险：

- 部分序列有发布滞后。
- 部分数据存在历史修订。
- API key 和限流需要在连接器配置中显式管理。

### 4.2 SEC EDGAR

用途：

- 公司公告和事件监测。
- 银行、金融机构、系统重要公司申报跟踪。
- 10-K、10-Q、8-K 等申报中的风险事件抽取。

第一批对象：

- 系统重要银行和经纪商。
- 大型保险、资产管理和交易所。
- 高杠杆行业龙头。

接入策略：

- 使用 company submissions JSON 获取公司申报列表。
- 使用 company facts JSON 获取结构化财务 facts。
- 保存 accession number、CIK、form type、filing date、report date、primary document URL。
- 原始 JSON 和关键 HTML 文档都要可追溯。

主要风险：

- SEC 要求合理 User-Agent 和访问频率控制。
- HTML 文档结构差异大，事件抽取不能依赖单一模板。

### 4.3 World Bank

用途：

- 全球宏观慢变量。
- 国家级对比和外部脆弱性评分。

第一批候选指标：

| 指标 | 说明 |
|---|---|
| `NY.GDP.MKTP.KD.ZG` | GDP 实际增速 |
| `FP.CPI.TOTL.ZG` | CPI 通胀 |
| `SL.UEM.TOTL.ZS` | 失业率 |
| `BN.CAB.XOKA.GD.ZS` | 经常账户占 GDP |
| `NY.GDP.MKTP.CD` | 名义 GDP |

接入策略：

- 按国家和指标分页抓取。
- 保存 World Bank country code、indicator code、date 和 value。
- 年频数据只能作为慢变量，不参与日频触发。

### 4.4 IMF

用途：

- 外储、国际收支、财政、债务、金融统计。

候选数据域：

- International Financial Statistics。
- Balance of Payments。
- Government Finance Statistics。
- Financial Soundness Indicators。

接入策略：

- 先做数据流发现，再确认具体 dataset 和 key。
- 连接器需要支持 SDMX 维度组合。
- 指标映射表必须独立维护，不把 IMF key 写死在业务逻辑里。

### 4.5 BIS

用途：

- 跨境银行敞口。
- 信贷缺口。
- 房地产价格。
- 债务证券。

接入策略：

- 优先使用 BIS 官方 API 或批量下载。
- 先接信贷、房地产和债务相关数据。
- 保留国家、部门、借款人类型、货币和频率维度。

## 5. P1/P2 数据源策略

### 5.1 ECB 和 OECD

用于扩展欧洲和 OECD 国家覆盖。实现上应复用 SDMX 连接器，不为每个源写完全不同的解析逻辑。

### 5.2 GDELT

用于新闻事件原型。

第一版只做：

- 指定关键词。
- 指定实体白名单。
- 指定地区和语言过滤。
- 输出事件数量、情绪均值、负面新闻占比。

不做：

- 直接让新闻情绪决定整体风险。
- 未经验证的复杂主题模型。

### 5.3 Alpha Vantage 和 yfinance

Alpha Vantage 可作为低频市场数据补充，但免费层有限流。yfinance 只用于研发原型和图表验证，不作为生产依赖。

生产替代接口必须预留：

- provider id。
- symbol mapping。
- 调整价格策略。
- 交易日历。
- 授权说明。

## 6. 数据源元数据字段

每个数据源进入系统前必须登记：

```text
source_id
display_name
owner
official_url
documentation_url
access_method
auth_required
auth_secret_ref
rate_limit
license_note
commercial_use_status
expected_latency
supported_frequencies
supported_regions
backfill_support
point_in_time_support
production_allowed
replacement_strategy
```

## 7. 参考开源项目

优先参考 [Equibles](https://github.com/daniel3303/Equibles) 的数据源覆盖和自托管思路。它已经把 SEC、FRED、FINRA、CFTC、CBOE、Yahoo Finance 等公开或免费数据源组织在一个金融数据平台中。

参考方式：

- 借鉴数据源覆盖清单。
- 借鉴抓取状态和本地存储思路。
- 不直接复制连接器实现。
- 不跳过授权审查。

## 8. 进入实现前的确认项

- 每个 P0 数据源完成一次手工 API 调用验证。
- 每个 P0 数据源补齐限流和授权说明。
- FRED 首批 series code 确认存在且历史长度足够。
- SEC 首批 CIK 白名单确认。
- IMF/BIS 的具体 dataset 和 key 完成映射。
- 市场价格原型源和生产替代源分离。


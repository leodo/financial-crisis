# 免费历史数据可落地性与需求分析

状态：`Draft`

最后更新：2026-05-30

## 1. 结论

可以先用免费历史数据落地一个可解释的金融危机预警 MVP，但边界要清楚：

- 可以覆盖：美国宏观、利率、期限利差、信用利差、波动率、金融压力、全球宏观慢变量、上市公司公告、新闻事件聚合。
- 暂不能免费稳定覆盖：实时逐笔行情、Level-2 订单簿、全量 CDS、商业信用违约数据库、机构级资金流和银行内部流动性。
- 第一版目标应是“日频风险评估 + 小时/日级事件补充”，不是高频交易级实时风控。
- FRED 适合作为 P0 主源；默认采用公开图表 CSV 下载口，不需要 API key。官方 API 仅作为可选增强，用于后续需要 realtime/vintage 字段的场景。
- FRED 图表 CSV 的少数授权序列可能只返回最近窗口，不能假设每个 series 都覆盖 2008/2020；首批回测需要为信用利差准备 BAA10Y 等长历史代理。
- SQLite 可以作为本地个人版和开发版主数据库；生产多人访问或高并发抓取再迁移 PostgreSQL/TimescaleDB。

## 2. 需求范围

### 2.1 功能需求

| 编号 | 需求 | MVP 标准 |
|---|---|---|
| R1 | 免费历史回填 | P0 指标能回填 10-20 年以上，慢变量尽量覆盖 1960 年以后 |
| R2 | 增量更新 | 每个数据源有 watermark，重复运行幂等 |
| R3 | 原始数据可追溯 | 保存 request、response hash、原始响应位置、解析版本 |
| R4 | 本地可运行 | 不依赖云数据库，个人电脑可完成回填、评分、面板展示 |
| R5 | 授权可审查 | 每个连接器记录官方文档、限流、商业使用状态和风险 |
| R6 | 历史回测 | 能重放历史危机窗口，区分真实预警、误报和漏报 |
| R7 | 持仓决策支持 | 输出风险等级、置信度、主要驱动和建议动作，不直接自动清仓 |

### 2.2 非功能需求

- 抓取失败不污染已发布指标。
- 低质量数据不能悄悄参与高置信度评分。
- 所有评分结果能追溯到指标、观测值、数据源和抓取批次。
- 本地库可备份、可重建、可迁移到生产数据库。
- 不使用需要绕过反爬、登录会话或非授权下载的数据源。

## 3. 免费数据源可落地性

| Source ID | 官方入口 | 历史数据可用性 | 认证 | 适合用途 | MVP 判断 |
|---|---|---|---|---|---|
| `fred` | [FRED Graph CSV](https://fred.stlouisfed.org/graph/fredgraph.csv?id=FEDFUNDS) / [FRED API](https://fred.stlouisfed.org/docs/api/fred/) | 多数美国宏观和金融序列可回填多年；CSV 无 vintage，API 支持 realtime/vintage 字段 | CSV 无需 key；API 需要免费 key | 利率、信用、VIX、金融压力、宏观 | P0，可落地 |
| `treasury` | [U.S. Treasury Yield Curve](https://home.treasury.gov/resource-center/data-chart-center/interest-rates/pages/xml?data=daily_treasury_yield_curve) | 每日美国国债收益率曲线，可作为 FRED 利率序列兜底 | 无需 key | 10Y、2Y、期限利差 | P0，可落地 |
| `world_bank` | [World Bank Indicators API](https://datahelpdesk.worldbank.org/knowledgebase/articles/889392) | 国家级年频指标历史长，适合 1960 年后慢变量 | 通常无需 key | GDP、通胀、失业、经常账户 | P0，可落地 |
| `sec_edgar` | [SEC EDGAR APIs](https://www.sec.gov/search-filings/edgar-application-programming-interfaces) | 公司申报历史可追溯，事件抽取需要 CIK 白名单 | 无 key，但要合理 User-Agent 和限流 | 8-K、10-Q、10-K、银行风险事件 | P0，可落地 |
| `imf` | [IMF Data API](https://data.imf.org/en/Resource-Pages/IMF-API) | 全球宏观金融统计覆盖广，接入复杂度高于 FRED | 以官方 API 为准 | 外储、国际收支、金融稳健指标 | P0，需先做 dataset/key 映射 |
| `bis` | [BIS Statistics API](https://stats.bis.org/api-doc/v1/) | 信贷、银行、房地产、债务等长周期数据有价值 | 以官方 API 为准 | 信贷缺口、跨境银行、房价 | P0，需 SDMX 连接器 |
| `gdelt` | [GDELT DOC API](https://blog.gdeltproject.org/gdelt-doc-2-0-api-debuts/amp/) | 新闻检索和时间线可构造日频事件信号 | 无 key | 新闻数量、负面情绪、主题热度 | P1，可做事件原型 |
| `alpha_vantage` | [Alpha Vantage Docs](https://www.alphavantage.co/documentation/) | 免费层有限流，历史覆盖按接口而定 | 免费 key | 价格和 FX 原型补充 | P1，只作原型 |
| `nasdaq_data_link` | [Nasdaq Data Link Docs](https://docs.data.nasdaq.com/) | 部分免费数据集可用，很多高价值集付费 | 按数据集 | 扩展数据集 | P2，逐数据集审查 |

## 4. 手工连通性验证

2026-05-30 本地手工验证结果：

| 数据源 | 验证方式 | 结果 |
|---|---|---|
| World Bank | `US/NY.GDP.MKTP.KD.ZG?date=1960:2024` | 返回 65 个年度点，覆盖 1960-2024 |
| SEC EDGAR | `data.sec.gov/submissions/CIK0000320193.json` | 使用普通 User-Agent 可返回 Apple filing 元数据，recent filing 数量约 1000 |
| GDELT | `financial crisis` 2020-01 timeline volume | 返回 31 个日度时间线点 |
| FRED Graph CSV | `fredgraph.csv?id=FEDFUNDS` | 无需 key，返回 FEDFUNDS 长历史 CSV |
| Treasury Yield XML | `daily_treasury_yield_curve&field_tdr_date_value_month=202605` | 无需 key，返回 2026-05 每日收益率曲线 XML |

该验证只证明“官方接口可访问并能返回历史样本”，不等于完成字段级授权审查和生产 SLA 验证。

## 5. 无法免费保证的部分

| 需求 | 免费源问题 | 建议 |
|---|---|---|
| 分钟级全市场实时价格 | 免费 API 限流和稳定性不足 | 第一版日频；后续接商业行情或券商 API |
| CDS 和信用违约明细 | 多为商业数据 | 用高收益 OAS、BAA spread、金融压力指数替代 |
| 银行内部流动性 | 非公开 | 用公开资产负债、存款、同业、公告事件代理 |
| 全球股指完整历史 | 免费源授权不一 | 原型使用低频源，生产预留 provider 替换 |
| 完整新闻语义理解 | GDELT 噪声高 | 第一版只做聚合信号和白名单实体，不让新闻单独决定危机等级 |

## 6. SQLite 可行性

SQLite 适合当前阶段，因为：

- 单用户本地运行，数据量主要是日频/月频时间序列。
- 抓取任务可以串行落库，读取由 Web 面板和分析任务完成。
- 原始大响应可以放文件系统，SQLite 只存索引、hash、元数据和标准化数据。
- 后续通过 repository/store trait 迁移到 PostgreSQL，不应把业务逻辑绑定到 SQLite SQL 方言。

SQLite 不适合：

- 多 worker 高并发写入。
- 多用户共享生产环境。
- 需要复杂 OLAP、长期分区和大规模并行回测。

详细方案见 [本地 SQLite 历史数据总体设计](../architecture/local-sqlite-historical-data-design.md)。

## 7. 落地判断

可以落地，但必须分阶段：

1. 第一阶段：SQLite + FRED + World Bank + SEC + GDELT，完成本地历史库、日频评分和面板展示。
2. 第二阶段：增加 IMF/BIS 的 SDMX 连接器，补齐全球慢变量和信贷/房地产长周期指标。
3. 第三阶段：做完整 historical backfill、point-in-time 回测和持仓风险动作表。
4. 第四阶段：按需要迁移 PostgreSQL/TimescaleDB，并替换市场价格生产源。

## 8. 需要补足的细分设计

本轮需要补两个细分文档：

- [历史回填设计](historical-backfill-design.md)：解决如何批量抓历史、增量更新、幂等和质量检查。
- [SQLite 本地存储方案](sqlite-local-storage-design.md)：解决表命名、写入模式、WAL、raw 文件、迁移路径。

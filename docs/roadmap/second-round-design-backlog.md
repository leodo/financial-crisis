# 第二轮细分设计清单

状态：`Draft`

最后更新：2026-05-30

本清单用于安排下一轮细分设计。优先级按是否阻塞 MVP 闭环排序。

执行状态见 [设计 TODO 总索引](design-todo.md)。截至 2026-05-30，本清单列出的细分设计文档已全部落地为初稿。

## P0：必须先设计

### 1. 免费数据源目录与连接器规范

目标：

- 确定第一批免费数据源。
- 明确每个数据源的 API、认证、限流、授权、频率、字段和失败策略。
- 定义 Rust 连接器接口和抓取任务状态模型。

关键问题：

- FRED、SEC、World Bank、IMF、BIS 分别先接哪些数据集？
- 哪些市场价格源只允许原型使用？
- 如何保存原始响应和数据修订？
- 如何做限流、重试和幂等？

输出文档：

- `docs/data/source-catalog.md`
- `docs/data/connector-contract.md`
- `docs/data/ingestion-state-machine.md`

### 2. 指标体系与风险评分方法

目标：

- 定义危机预警指标树。
- 定义每个指标的风险方向、变换方式、标准化方法和权重。
- 定义整体评分和分项评分。

关键问题：

- 宏观、市场、信用、银行、房地产、事件各放哪些指标？
- 每个指标是越高越危险、越低越危险，还是偏离正常区间危险？
- 如何处理不同频率的数据？
- 阈值用历史分位数、Z-score，还是固定监管阈值？

输出文档：

- `docs/analytics/indicator-taxonomy.md`
- `docs/analytics/scoring-methodology.md`
- `docs/analytics/risk-levels.md`

### 3. 数据库 schema 与数据质量模型

目标：

- 设计指标元数据、时间序列、原始数据、抓取任务、质量检查和预警事件表。
- 明确 TimescaleDB、PostgreSQL 普通表和 Parquet 的边界。

关键问题：

- 指标代码如何命名？
- 如何支持多国家、多市场、多频率？
- 原始响应保存在哪里？
- 数据质量如何评分并暴露给前端？

输出文档：

- `docs/data/storage-schema.md`
- `docs/data/data-quality-model.md`

### 4. Web 面板信息架构

目标：

- 设计用户看到的页面、组件和下钻路径。
- 明确总览页、指标页、数据源页、预警页、回测页的核心内容。

关键问题：

- 首页最重要的 5 个信息是什么？
- 用户如何从整体风险下钻到具体指标？
- 数据质量问题如何提示？
- 图表和表格如何组合？

输出文档：

- `docs/product/dashboard-information-architecture.md`
- `docs/product/dashboard-wireframe-notes.md`

## P1：MVP 后半段设计

### 5. 回测与评估方法

目标：

- 设计如何用历史危机评估预警系统。
- 定义误报、漏报、提前量和稳定性指标。

输出文档：

- `docs/analytics/backtesting-design.md`

### 6. 预警事件与通知机制

目标：

- 定义何时触发预警。
- 定义预警升级、降级、确认、解除和归档流程。

输出文档：

- `docs/alerts/alert-event-model.md`

### 7. 部署与运维

目标：

- 设计本地开发、Docker Compose、生产部署、监控和日志。

输出文档：

- `docs/ops/deployment-design.md`
- `docs/ops/observability-design.md`

## P2：后续增强

### 8. 机器学习和研究环境

目标：

- 定义何时引入 Python、Polars、DuckDB、XGBoost 或其他模型工具。

输出文档：

- `docs/research/modeling-workbench.md`

### 9. 新闻、公告和 LLM 事件分析

目标：

- 设计新闻实体识别、事件分类、情绪评分和解释生成。

输出文档：

- `docs/events/news-and-filing-analysis.md`

### 10. 权限、安全和审计

目标：

- 设计用户登录、权限、API key 管理、审计日志和敏感配置管理。

输出文档：

- `docs/security/security-design.md`

## 推荐第二轮顺序

1. 先做 `source-catalog.md` 和 `connector-contract.md`，解决免费数据源可用性。
2. 再做 `indicator-taxonomy.md` 和 `scoring-methodology.md`，定义系统评估逻辑。
3. 然后做 `storage-schema.md`，保证数据能长期沉淀。
4. 最后做面板信息架构，把数据和评分映射到用户界面。

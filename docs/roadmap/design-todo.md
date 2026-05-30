# 设计 TODO 总索引

状态：`Done`

最后更新：2026-05-30

本文档是后续细分设计的执行清单。当前所有列出的设计文档已经落地为初稿。

## 执行原则

- 先完成阻塞 MVP 的 P0 设计，再补齐 P1 和 P2。
- 每个设计文档都要能独立回答“目标、边界、核心结构、关键流程、风险、后续实现提示”。
- 文档先于代码，当前阶段不写实现代码。
- 免费数据源相关设计优先落地，后续实现时可参考开源项目，但不能依赖未经确认授权的数据抓取方式。

## P0：MVP 前置设计

- [x] `docs/data/source-catalog.md`：免费数据源目录
- [x] `docs/data/connector-contract.md`：数据连接器契约
- [x] `docs/data/ingestion-state-machine.md`：抓取任务状态机
- [x] `docs/analytics/indicator-taxonomy.md`：指标体系
- [x] `docs/analytics/scoring-methodology.md`：评分方法
- [x] `docs/analytics/risk-levels.md`：风险等级
- [x] `docs/data/storage-schema.md`：数据库 schema
- [x] `docs/data/data-quality-model.md`：数据质量模型
- [x] `docs/product/dashboard-information-architecture.md`：面板信息架构
- [x] `docs/product/dashboard-wireframe-notes.md`：面板草图说明

## P1：MVP 后半段设计

- [x] `docs/analytics/backtesting-design.md`：回测设计
- [x] `docs/alerts/alert-event-model.md`：预警事件模型
- [x] `docs/ops/deployment-design.md`：部署设计
- [x] `docs/ops/observability-design.md`：可观测性设计

## P2：增强设计

- [x] `docs/research/modeling-workbench.md`：研究和模型工作台
- [x] `docs/events/news-and-filing-analysis.md`：新闻、公告和 LLM 事件分析
- [x] `docs/security/security-design.md`：权限、安全和审计

## 专项补充设计：免费历史数据与本地 SQLite

- [x] `docs/data/free-historical-data-feasibility.md`：免费历史数据可落地性与需求分析
- [x] `docs/architecture/local-sqlite-historical-data-design.md`：本地 SQLite 历史数据总体设计
- [x] `docs/data/historical-backfill-design.md`：历史回填设计
- [x] `docs/data/sqlite-local-storage-design.md`：SQLite 本地存储方案
- [x] `docs/roadmap/sqlite-historical-data-implementation-plan.md`：SQLite 历史数据实现路线
- [x] `docs/data/fred-first-connector-implementation-spec.md`：FRED 首个真实连接器实现规格

## 完成定义

本轮设计完成需要满足：

- 上述所有文档已创建。
- `README.md` 和 `docs/README.md` 已更新导航。
- `second-round-design-backlog.md` 与本 TODO 不冲突。
- 文件路径可通过 `rg --files` 检查到。
- 文档中没有明显未解决占位标记。

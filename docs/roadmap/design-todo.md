# 设计 TODO 总索引

状态：`Review`

最后更新：2026-05-30

本文档是第一阶段设计清单，用于记录风险强度看板和基础数据层设计的落地情况。

自 2026-05-30 起，系统主线已经升级为“危机概率评估系统”。与概率、时距判断、决策 posture 直接相关的新一轮设计，统一由 [危机概率评估设计 TODO](crisis-probability-design-todo.md) 管理。

## 执行原则

- 先完成阻塞 MVP 的 P0 设计，再补齐 P1 和 P2。
- 每个设计文档都要能独立回答“目标、边界、核心结构、关键流程、风险、后续实现提示”。
- 文档先于代码，当前阶段不写实现代码。
- 免费数据源相关设计优先落地，后续实现时可参考开源项目，但不能依赖未经确认授权的数据抓取方式。
- 本文档覆盖的是第一阶段基线；若涉及危机概率、标签、时距和决策支持，以新 TODO 为准。

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

## 与新主线的关系

第一阶段已完成项主要提供：

- 可解释风险强度层
- 基础数据连接器和本地 SQLite 落地
- 旧版风险面板和回测骨架

若后续开发目标是：

- `5d / 20d / 60d` 危机概率
- “离风险还有多远”的时距判断
- 决策 posture
- JPY carry 外部风险增强

则应优先参考：

- `docs/architecture/global-design.md`
- `docs/analytics/horizon-label-design.md`
- `docs/analytics/probability-engine-design.md`
- `docs/analytics/decision-support-policy.md`
- `docs/data/us-centric-free-data-plan.md`
- `docs/data/jpy-carry-risk-module-design.md`
- `docs/product/decision-dashboard-design.md`
- `docs/roadmap/crisis-probability-design-todo.md`

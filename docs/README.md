# 文档总览

本文档系统用于沉淀金融危机预警系统的设计、取舍、数据源研究和后续实现计划。

## 目录结构

```text
docs/
  architecture/        全局架构、模块边界、系统流程
  alerts/              预警事件、通知和状态模型
  analytics/           指标体系、评分、风险等级、回测
  data/                数据源、抓取、清洗、质量控制
  decisions/           架构决策记录
  events/              新闻、公告和事件分析
  ops/                 部署、运维和可观测性
  product/             Web 面板信息架构和页面草图
  references/          开源项目、外部系统、官方资料参考
  research/            模型研究和实验设计
  roadmap/             细分设计清单、阶段规划、待办事项
  security/            权限、安全和审计
```

## 当前文档

- [全局设计](architecture/global-design.md)
- [设计 TODO 总索引](roadmap/design-todo.md)

### 数据层

- [免费数据源与抓取设计](data/free-data-ingestion.md)
- [免费数据源目录](data/source-catalog.md)
- [数据连接器契约](data/connector-contract.md)
- [抓取任务状态机](data/ingestion-state-machine.md)
- [数据库 Schema 设计](data/storage-schema.md)
- [数据质量模型](data/data-quality-model.md)

### 分析层

- [指标体系设计](analytics/indicator-taxonomy.md)
- [风险评分方法](analytics/scoring-methodology.md)
- [风险等级设计](analytics/risk-levels.md)
- [回测设计](analytics/backtesting-design.md)

### 产品层

- [Web 面板信息架构](product/dashboard-information-architecture.md)
- [Web 面板草图说明](product/dashboard-wireframe-notes.md)
- [风险面板 UX 改造设计](product/dashboard-ux-redesign.md)

### 预警、事件、研究和运维

- [预警事件模型](alerts/alert-event-model.md)
- [新闻、公告和 LLM 事件分析设计](events/news-and-filing-analysis.md)
- [研究和模型工作台设计](research/modeling-workbench.md)
- [部署设计](ops/deployment-design.md)
- [可观测性设计](ops/observability-design.md)
- [权限、安全和审计设计](security/security-design.md)

### 参考和决策

- [开源项目参考](references/open-source-projects.md)
- [第二轮细分设计清单](roadmap/second-round-design-backlog.md)
- [ADR-0001 初始架构方向](decisions/0001-initial-architecture.md)

## 设计原则

- 先做可解释预警，再做复杂模型。
- 先用免费和官方数据源验证系统闭环，再接商业实时数据。
- 慢变量判断系统脆弱性，快变量捕捉危机触发。
- 原始数据必须可追溯，评分结果必须可解释。
- Rust 优先承担后端服务、抓取、分析和任务调度，前端优先采用成熟 Web 生态。

## 实现入口

- Rust workspace: `Cargo.toml`
- API 服务: `apps/api`
- Worker 入口: `apps/worker`
- 前端面板: `apps/web`
- 存储层: `crates/storage`
- 数据库迁移: `migrations/0001_init.sql`
- 部署草案: `deploy/docker-compose.yml`

## 文档状态约定

- `Draft`：初稿，可大幅调整。
- `Review`：等待讨论或确认。
- `Accepted`：作为后续实现依据。
- `Superseded`：已被新文档替代。

大部分设计文档当前为 `Draft`，表示已经落地初稿，但仍可在实现前继续评审和修订。

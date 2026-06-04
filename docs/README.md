# 文档总览

本文档系统用于沉淀金融危机概率评估系统的设计、取舍、数据源研究和后续实现计划。

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
- [系统可行性分析](architecture/system-feasibility-analysis.md)
- [本地 SQLite 历史数据总体设计](architecture/local-sqlite-historical-data-design.md)
- [代码结构与可维护性评审](architecture/codebase-maintainability-review.md)
- [工程治理方案](architecture/engineering-governance-plan.md)
- [开发质量门禁](architecture/development-quality-gates.md)
- [设计 TODO 总索引](roadmap/design-todo.md)
- [危机概率评估设计 TODO](roadmap/crisis-probability-design-todo.md)
- [工程维护性 TODO](roadmap/engineering-maintainability-todo.md)
- [SQLite 历史数据实现路线](roadmap/sqlite-historical-data-implementation-plan.md)

当前活跃真相源只有两份：

- 模型/数据/回测主线：`roadmap/crisis-probability-design-todo.md`
- 工程结构/质量治理主线：`roadmap/engineering-maintainability-todo.md`

### 数据层

- [免费数据源与抓取设计](data/free-data-ingestion.md)
- [免费数据源目录](data/source-catalog.md)
- [免费历史数据可落地性与需求分析](data/free-historical-data-feasibility.md)
- [Point-in-Time 可见性规范](data/point-in-time-visibility-spec.md)
- [FRED 首个真实连接器实现规格](data/fred-first-connector-implementation-spec.md)
- [数据连接器契约](data/connector-contract.md)
- [抓取任务状态机](data/ingestion-state-machine.md)
- [历史回填设计](data/historical-backfill-design.md)
- [数据库 Schema 设计](data/storage-schema.md)
- [SQLite 本地存储方案](data/sqlite-local-storage-design.md)
- [数据质量模型](data/data-quality-model.md)
- [美国主线免费数据方案](data/us-centric-free-data-plan.md)
- [日元套息外部风险模块设计](data/jpy-carry-risk-module-design.md)
- [SEC EDGAR 连接器实现规格](data/sec-edgar-connector-spec.md)
- [BOJ / USDJPY 连接器实现规格](data/boj-connector-spec.md)

### 分析层

- [指标体系设计](analytics/indicator-taxonomy.md)
- [特征覆盖矩阵](analytics/feature-coverage-matrix.md)
- [风险评分方法](analytics/scoring-methodology.md)
- [风险等级设计](analytics/risk-levels.md)
- [回测设计](analytics/backtesting-design.md)
- [危机窗口与标签设计](analytics/horizon-label-design.md)
- [危机场景目录](analytics/scenario-catalog.md)
- [危机概率引擎设计](analytics/probability-engine-design.md)
- [决策支持策略设计](analytics/decision-support-policy.md)
- [持仓动作手册设计](analytics/portfolio-action-playbook.md)
- [特征库设计](analytics/feature-store-design.md)
- [正式训练数据集规格](analytics/formal-dataset-spec.md)
- [正式危机概率模型下一代设计](analytics/formal-nextgen-model-design.md)
- [概率校准设计](analytics/probability-calibration-design.md)
- [真实回测执行设计](analytics/real-backtest-execution-design.md)
- [模型发布与在线评分设计](analytics/model-release-and-serving-design.md)
- [正式模型准入与 Go/No-Go](analytics/model-go-no-go.md)
- [历史相似阶段设计](analytics/historical-analog-design.md)
- [Posture 阈值调优设计](analytics/posture-threshold-tuning.md)
- [2023 区域银行危机 L3 修复设计](analytics/regional-banks-2023-l3-repair-design.md)
- [Release Review Runtime 对齐设计](analytics/release-review-runtime-alignment-design.md)

### 产品层

- [Web 面板信息架构](product/dashboard-information-architecture.md)
- [Web 面板草图说明](product/dashboard-wireframe-notes.md)
- [风险面板 UX 改造设计](product/dashboard-ux-redesign.md)
- [决策面板设计](product/decision-dashboard-design.md)
- [Assessment API Contract](product/assessment-api-contract.md)
- [方法页设计](product/methodology-page-design.md)

### 预警、事件、研究和运维

- [预警事件模型](alerts/alert-event-model.md)
- [银行业风险事件分类设计](events/banking-event-taxonomy.md)
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

- 先做可解释风险层，再做可校准概率层。
- 先用免费和官方数据源验证系统闭环，再接商业实时数据。
- 慢变量判断系统脆弱性，快变量捕捉危机触发。
- 原始数据必须可追溯，评分结果必须可解释。
- 概率、风险强度和决策 posture 必须明确区分。
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

# 工程维护性 TODO

状态：`Draft`

最后更新：2026-06-02

本文档只管理“工程结构、模块边界、共用代码收敛、生成工件治理、维护性约束”，不替代概率模型主线 TODO。

相关评审见：

- [代码结构与可维护性评审](../architecture/codebase-maintainability-review.md)
- [危机概率评估设计 TODO](crisis-probability-design-todo.md)

## 1. 目标

目标不是追求形式上的“优雅”，而是降低以下风险：

- 改一个研究逻辑误伤运行逻辑；
- 训练与线上评分口径漂移；
- review 成本持续上升；
- 生成物淹没有效提交；
- 文件过大导致理解和测试成本失控。

## 2. P0：立即补齐的治理项

- [x] 明确生成工件分级：
  - 正式发布与基线对照工件可以入库；
  - 临时实验工件必须有独立归档或清理策略；
  - 同目录不能长期混放“正式工件”和“草稿工件”。
- [x] 定义 `apps/worker` 模块拆分边界：
  - `commands/release`
  - `commands/research`
  - `commands/backfill`
  - `commands/db`
  - `reporting`
  - `pipeline`
- [x] 定义 `apps/api` 模块拆分边界：
  - `assessment`
  - `history_replay`
  - `data_source`
  - `demo_seed`
  - `backtest`
- [x] 盘点 API 与 worker 的重复逻辑，列出必须收敛到共享模块的函数清单。
- [x] 为维护性治理补一条实施原则：模型主线优先，但新增功能不再允许直接塞进现有超大文件。

以上 P0 事项已由 [工程治理方案](../architecture/engineering-governance-plan.md) 落地，并已同步收紧 worker 的默认实验输出目录。

## 3. P1：优先重构项

### 3.1 Worker

- [x] 把 `apps/worker/src/main.rs` 的 CLI 参数解析与命令分发拆出去。
- [x] 把 release review / publish / activate / rollback 收敛到独立模块。
- [x] 把 feature snapshot / formal dataset / pipeline train 拆成独立研究模块。
- [x] 把 backfill / refresh 免费数据路径拆成独立命令模块。
- [x] 把 markdown/json 报告渲染抽到专门 reporting 模块。

当前进展：

- 已先抽出 `apps/worker/src/output_paths.rs`，统一实验/追踪输出路径策略。
- 已先抽出 `apps/worker/src/reporting.rs`，收走 release review / formal dataset summary 的写盘逻辑。
- 已新增 `apps/worker/src/commands/mod.rs`，收走顶层 CLI 参数匹配、帮助文本和一级命令分发。
- 已新增 `apps/worker/src/commands/audit.rs` 与 `commands/research.rs`，把 `audit` / `research` 一级命令入口从路由文件中继续分层。
- 已新增 `apps/worker/src/commands/release.rs`，把 release 选项解析与 `publish/list/show/activate/rollback/review` handler 从 `main.rs` 中移出。
- 已新增 `apps/worker/src/commands/snapshot.rs`、`commands/feature.rs`、`commands/dataset.rs`、`commands/pipeline.rs`，把 research 下的 snapshot / feature / dataset / pipeline CLI 选项解析与入口 handler 从 `main.rs` 中继续剥离。
- snapshot 导出写盘、heuristic snapshot 训练样本装配、formal dataset 训练集解析等 research helper 也已开始跟随迁移到对应子模块，不再继续堆在 `main.rs` 的同一层里。
- formal feature snapshot 的观测可见性判断、时区截止规则、覆盖率汇总、核心特征门槛与单日快照构建实现，已继续迁入 `commands/feature.rs`，`main.rs` 不再直接承载这一整块特征工程细节。
- formal dataset summary 的 envelope 结构、split/scenario/regime 汇总、Markdown 渲染与 CLI 打印，也已迁入 `commands/dataset.rs`，`main.rs` 只保留场景切分与共享训练 helper。
- formal dataset 的主样本装配、场景集加载/切分要求、scenario metadata 编码 helper 也已继续迁入 `commands/dataset.rs`，`main.rs` 进一步缩回到 actionability / 概率训练共享逻辑。
- 已新增 `apps/worker/src/formal.rs`，把 snapshot/formal dataset 共用的场景标签推导收敛成单一 helper，避免两条训练输入链路各自维护一套 crisis/actionability 标注逻辑。
- 已新增 `apps/worker/src/training.rs`，把 `ProbabilityTrainingRow/Input`、chronological split 与 label-mode 支持检查从 `main.rs` 中抽离，固定训练数据 contract 的归属边界。
- 已新增 `apps/worker/src/actionability.rs`，把 actionability bundle 训练、阈值选择、校准策略、guardrail 与 actionability evaluation summary 从 `main.rs` 中拆出，供训练与 release review 共用。
- release 相关的 `activate_release_with_runtime_guard`、review stage activate/restore、market scope resolve 也已迁到 `commands/release.rs`。
- `release review` 的 runtime snapshot 抓取与 orchestration 也已迁到 `commands/release.rs`。
- `release review` 专属的 probability/actionability/runtime sanity guardrail、recommendation、summary helper 也已开始跟随迁移。
- 已新增 `apps/worker/src/commands/db.rs`，把 `db init/seed/check` 从超大入口文件中拆出。
- 已新增 `apps/worker/src/commands/refresh.rs` 与 `commands/backfill.rs`，开始把免费数据刷新与回填入口从 `main.rs` 中剥离。
- 当前 `main.rs` 已主要保留底层 research/helper/训练实现；下一步继续按实现体拆出 formal dataset / pipeline 内部 helper，以及剩余 release review runtime 细节。

### 3.2 API

- [ ] 把 `apps/api/src/demo.rs` 中的 demo seed 与真实数据源加载拆开。
- [ ] 把 historical replay / prediction snapshot bridge / cache key 逻辑拆开。
- [ ] 把 `apps/api/src/assessment.rs` 中的特征构造、概率评分、posture 判定、position guidance、analogs 分模块。

### 3.3 Shared Logic

- [ ] 收敛 `apply_platt_calibration`、观测值窗口切片、`difference_from_tail` 等重复函数。
- [ ] 明确概率数学、特征派生、runtime threshold 诊断哪些属于共享领域逻辑，哪些属于 app-specific glue code。
- [ ] 为共享逻辑补单元测试，避免训练侧和运行侧未来再次分叉。

当前进展：

- 已把 logistic 概率打分与 `Platt` 校准收敛到 `crates/domain/src/probability_bundle.rs`。
- `apps/api` 与 `apps/worker` 已开始复用同一套共享概率函数。
- 观测窗口 / `difference_from_tail` 仍待继续收敛，但涉及 PIT 可见性过滤，下一步需要先划清 generic helper 与 runtime-specific 过滤边界。

## 4. P2：次级重构项

### 4.1 Storage

- [ ] 把 `crates/storage/src/sqlite.rs` 按聚合拆分：
  - metadata / mappings
  - observations
  - alerts
  - releases
  - snapshots
  - formal datasets
  - historical replay

### 4.2 Web

- [ ] 把 `apps/web/src/App.tsx` 拆成按页面和卡片组织的 view/container/component。
- [ ] 把领域解释文本和格式化逻辑继续从页面组件中剥离。
- [ ] 为决策面板、方法页、审计页明确单独的数据装配层。

## 5. 约束机制

- [ ] 新增或显著扩展功能时，如果目标文件已经是当前仓库前几位的大文件，优先先拆模块，再加功能。
- [ ] 生成工件进入 Git 前，必须说明它属于：
  - 正式 release 工件；
  - 基线对照证据；
  - 还是临时研究副产物。
- [ ] 任何影响训练口径和运行口径的修改，都要检查共享函数是否已统一。
- [ ] 任何新的仓位建议或动作规则，都不能绕开现有 `playbook`、`Go/No-Go` 和 “非自动交易指令”边界。

## 6. 完成定义

以下条件满足时，才可以认为工程维护性从“高风险”进入“可持续”：

- [ ] `apps/worker/src/main.rs` 不再承担所有主线职责。
- [ ] `apps/api/src/demo.rs` 不再同时承载 demo seed、真实历史回放和 runtime bridge。
- [ ] API / worker 的重复概率数学与观测窗口逻辑已收敛。
- [ ] `crates/storage/src/sqlite.rs` 已按聚合拆开。
- [ ] `apps/web/src/App.tsx` 已拆成稳定组件层次。
- [ ] 生成工件治理规则已落文档并落实到提交流程。

## 7. 执行顺序

建议按以下顺序推进：

1. 先完成 P0 治理定义；
2. 再拆 worker；
3. 再收敛 API / worker 共用逻辑；
4. 再拆 API history/runtime/demo；
5. 再拆 storage 与 web。

原因很简单：

- worker 和 API 共用逻辑是当前最容易继续恶化的耦合点；
- web 与 storage 的问题也真实存在，但对当前模型主线的阻塞更小。

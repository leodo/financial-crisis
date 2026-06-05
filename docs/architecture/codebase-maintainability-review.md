# 代码结构与可维护性评审

状态：`Review`

最后更新：2026-06-05

## 1. 目的

本文档回答四个问题：

1. 当前项目的大结构是否还合理。
2. 当前代码是否已经出现明显的维护性风险。
3. 哪些地方必须先治理，哪些地方可以延后。
4. 后续开发是否已经有足够的文档和边界约束支撑。

## 2. 总体结论

结论分两层：

- **仓库级结构是合理的**：`apps/` + `crates/` + `docs/` + `config/` 的分层方向没有问题，符合当前 Rust 多应用工作区的主流做法。
- **文件级结构已经开始失衡**：多个核心文件同时承载“入口、业务编排、领域逻辑、数据变换、演示数据、审计输出、训练流程”等多类职责，后续继续堆功能会明显提高回归风险。

因此，当前不建议推翻整体目录结构，也不建议重做框架；正确做法是：

1. 保留现有仓库级结构。
2. 先补工程治理文档和 TODO。
3. 再按优先级把几个超大文件拆回模块边界。

对应治理落地见：

- [工程治理方案](engineering-governance-plan.md)
- [工程维护性 TODO](../roadmap/engineering-maintainability-todo.md)

## 3. 当前结构中做对的部分

### 3.1 仓库层级基本正确

当前主结构：

```text
apps/
  api/
  web/
  worker/
crates/
  domain/
  ingestion/
  scoring/
  storage/
docs/
config/
```

这个结构的优点是：

- `apps/api`、`apps/worker`、`apps/web` 的运行边界清楚；
- `crates/domain`、`crates/ingestion`、`crates/storage` 已经把部分共用能力抽离出来；
- 文档系统较完整，研究、产品、数据、模型、回测都有单独设计；
- 对“免费数据、解释性、Go/No-Go、非自动交易指令”这些产品边界已有明确文档约束。

### 3.2 文档约束比一般原型项目成熟

当前已经具备以下约束文档：

- `docs/analytics/model-go-no-go.md`
- `docs/analytics/formal-nextgen-model-design.md`
- `docs/analytics/portfolio-action-playbook.md`
- `docs/product/assessment-api-contract.md`
- `docs/roadmap/crisis-probability-design-todo.md`

这说明项目不是“先乱写代码再回头补文档”，而是已经有相对清晰的研究和产品边界。

## 4. 主要维护性风险

### 4.1 `apps/worker/src/main.rs` 已经超出单文件可维护范围

当前 `apps/worker/src/main.rs` 已经同时承载：

- CLI 入口；
- SQLite 初始化与巡检；
- release publish / activate / rollback / review；
- feature snapshot 构建；
- formal dataset 构建；
- probability / actionability 训练；
- backfill 与 refresh；
- runtime guard；
- 报告渲染与导出。

这类集中式实现短期推进快，但长期风险很高：

- 改一个训练逻辑，容易误伤 CLI 或 review；
- 改一个 backfill 行为，容易影响 research pipeline；
- review 难度持续上升；
- 测试粒度难以下沉。

最新进展：

- 顶层 CLI 分发、research 子命令、release 主流程、db/refresh/backfill 入口已先后从 `main.rs` 中拆出；
- `apps/worker/src/commands/release.rs` 又进一步把概率切片研究工具拆到 `apps/worker/src/commands/release/probability.rs`，并把 release review 的 CLI 选项、runtime snapshot、比较诊断、建议与总结继续拆到 `apps/worker/src/commands/release/review.rs`；当前 `release.rs` 主要保留 publish/list/show/activate/rollback 与共享 runtime guard helper；
- `apps/worker/src/commands/release/probability/slice.rs` 已收走 runtime probability slice 的 JSON/CSV 导出、horizon overlay 展开与 CLI 摘要打印；`commands/release/probability/formal.rs` 已收走 formal dataset slice 的 bundle 打分、base model diagnostics、CSV/JSON 导出与 CLI 摘要打印；`commands/release/probability/common.rs` 负责文件名与 CSV 转义这类共享小工具；`commands/release/probability/compare.rs` 则继续只保留 formal probability compare 的阈值摘要、feature delta 聚合、窗口汇总、CSV/JSON 导出与 CLI 摘要打印，`commands/release/probability.rs` 现已收缩到“slice CLI 编排 + release/bundle 装配 + formal compare orchestration”边界；
- `apps/worker/src/commands/release/review/focus.rs` 已继续收走 structured signal counts、backtest scenario compare、scenario focus diagnostics、runtime actionable block/facet 汇总与 primary failure mode 判定，`commands/release/review.rs` 已回到“review 编排 + runtime separation compare + recommendation / summary”边界；
- `apps/worker/src/training.rs` 已进一步收走 formal bundle 训练管线、`forward_crisis` 标签 / regime helper；`apps/worker/src/release_review.rs` 也已继续收走 release review 专属 report wire structs、historical audit helper、runtime regime diagnostics 与 Markdown 渲染入口；
- `apps/worker/src/release_review/historical.rs` 本轮继续收走 failure mode、historical audit priority / attribution / action / workstream 汇总与 takeaways；`apps/worker/src/release_review/runtime.rs` 继续收走 runtime regime probability / separation diagnostics 与 runtime takeaways，`release_review.rs` 开始回到“report wire structs + shared formatter + Markdown 壳层”边界；
- `apps/worker/src/commands/dataset/report.rs` 已继续收走 formal dataset summary/slice 的 envelope、split/scenario/regime 汇总、Markdown/CSV/JSON 渲染与 CLI 摘要打印；本轮 `apps/worker/src/commands/dataset/split.rs` 又继续收走 split profile、scenario-aware split bounds、label support 与 scenario range helper，`apps/worker/src/commands/dataset/scenarios.rs` 收走 scenario catalog 装配与 metadata 编码 helper，`commands/dataset.rs` 已从约 `1218` 行收缩到约 `676` 行，边界开始回到“数据构建 + 研究命令编排”；
- `apps/api/src/assessment/runtime_policy.rs` 本轮继续收走 runtime threshold、serving model policy、history runtime policy version 与 diagnostics；`apps/api/src/assessment/common.rs` 收走 rounding/format/pressure 这类跨子模块共享 helper，随后又把 `assessment.rs` 底部内联测试整体外移到 `apps/api/src/assessment/tests.rs`，`assessment.rs` 已从约 `1174` 行收缩到约 `241` 行，主文件进一步回到“assessment 总装配 + method version / snapshot envelope”边界；
- `apps/worker/src/probability/overlay.rs` 已继续收走 family overlay 的 audit spec、样本筛选、family-aware/balanced split 与 overlay 训练；`apps/worker/src/probability/threshold.rs` 也已继续收走 calibration selection、threshold selection、regime-support threshold repair 与 threshold diagnostics / calibration evidence，`probability.rs` 开始回到“主头训练 + regime evaluation / bundle summary”边界；
- `apps/worker/src/scenario.rs` 已继续收走 `CrisisScenario`、action episode window、protected context、primary/forward scenario 选择和 action window label；
- `apps/worker/src/support.rs` 已继续收走 `ApiReloadHistoryMode`、demo run、API fetch/reload、SQLite/raw payload IO、格式化 helper 和通用 rounding/hash/path helper；
- 原先内联在 `main.rs` 的超大测试块已整体迁到 `apps/worker/src/tests.rs`，共享测试构造器已继续下沉到 `apps/worker/src/tests/fixtures.rs`；其中原本约 `1344` 行的 `apps/worker/src/tests/training.rs` 本轮又继续拆成 `apps/worker/src/tests/training/{visibility,scenario_regimes,weighting,sign_constraints,family_constraints}.rs` 与聚合 `mod.rs`，option parsing / training / quality / review / split requirement 也已切成真实测试子模块而不再依赖 `include!` 聚合；
- `main.rs` 体量已从约 `7.6k` 行进一步降到约 `165` 行；
- 因此，worker 当前的主要维护风险已从“所有 release 能力都堆在一个文件里”，下降为“运行时代码已基本按边界收口，测试层也开始具备稳定模块边界；后续仍可继续把 cross-topic fixture 与少量共享导入继续收窄”。

### 4.2 API runtime、demo、history replay 曾有明显耦合

治理前，`apps/api/src/demo.rs` 同时处理：

- 数据源模式切换；
- SQLite / Postgres 加载；
- demo 数据；
- prediction snapshot 转换；
- historical replay 缓存；
- backtest timeline / rolling audit；
- scenario fallback。

这会带来两个问题：

1. “演示数据”与“真实历史重建”混在一处，理解成本很高。
2. history replay 继续演进时，很容易把 demo 兼容逻辑一起拖着走。

最新进展：

- `FC_DATA_MODE` 解析与 SQLite/Postgres 装载已拆到 `apps/api/src/data_source.rs`；
- historical replay / prediction snapshot bridge 已拆到 `apps/api/src/history_replay.rs`；
- backtest timeline、rolling audit、scenario fallback 和动作级历史判定规则已拆到 `apps/api/src/backtest.rs`。
- 静态 demo 指标样本、观测样本、源状态样本和 demo alert 构造已拆到 `apps/api/src/demo_seed.rs`；
- assessment history 装配、SQLite prediction snapshot 重建和窗口筛选已拆到 `apps/api/src/history_builder.rs`。

因此，`demo.rs` 的风险已明显下降，当前已主要收缩为 demo 当前截面装配、runtime assessment snapshot 组装与用户偏好加载；API 侧后续重点已从 `demo.rs` 主拆分，转向观察这些 runtime helper 是否值得继续下沉到 shared/runtime config 层。

### 4.3 训练侧与运行侧已有重复实现，后续容易漂移

目前 API 与 worker 中已经出现重复的底层函数，例如：

- 概率打分/校准；
- 观测值切片；
- `difference_from_tail`；
- 部分特征衍生逻辑。

如果后续只修一边，就会出现：

- 训练结果与线上评分口径不一致；
- release review 看起来通过，但线上解释变味；
- debug 成本明显上升。

### 4.4 SQLite store 过大，存储职责还可以继续按聚合拆分

`crates/storage/src/sqlite.rs` 目前不仅处理：

- 观测值；
- 数据源与映射；
- alerts；
- release registry；
- prediction snapshots；
- feature snapshots；
- formal datasets；
- historical replay。

这说明存储层虽然已经抽到 crate，但仍然是“大仓库对象”风格，不利于后续按子域演进。

### 4.5 Web `App.tsx` 已开始变成页面总控 + 领域解释器

`apps/web/src/App.tsx` 同时承担：

- 整体布局；
- tab 导航；
- 决策说明；
- 指标解释；
- posture 与 position guidance 展示；
- analog / probability / JPY carry 卡片编排。

这说明前端虽然没有选错框架，但组件层次还不够细，继续增加方法页、研究页、审计页后会继续膨胀。

### 4.6 生成物治理还不够清晰

当前仓库里：

- 一部分 `config/model-bundles/generated/*`、`config/model-releases/generated/*`、`reports/release-review/*` 已被纳入版本控制；
- 新一轮实验生成物又会继续落在同一目录；
- `.gitignore` 只忽略了部分 `reports/*` 临时文件，没有定义“哪些生成物应该长期追踪，哪些只是实验草稿”。

这会导致：

- 工作区很容易长期脏；
- review 中掺入大量非核心文件；
- 以后更难看出“正式发布工件”和“临时研究副产物”的边界。

## 5. 目前是否需要重做大结构

**不需要。**

当前问题不在于：

- 选错 Rust；
- 选错 React；
- 目录分层完全错误；
- 文档体系缺失。

当前问题在于：

- **实现推进速度已经超过了模块回收速度**；
- 需要把“先能跑”回收到“可持续维护”。

所以，不建议现在推倒重来，也不建议改成全新框架。  
应当在现有结构上做 **受控拆分**。

## 6. 后续开发的文档支撑是否足够

### 6.1 对“模型主线”来说，文档基本够用

模型与研究主线已有较完整的设计与回顾：

- 标签、场景、PIT、回测、release review、Go/No-Go、动作层、下一代模型、相似历史阶段等文档都已存在；
- 走不通的路径也已有明确记录，例如一些 sample-weight、soft-label、runtime floor 放宽方案没有带来真正提前量提升。

因此，“继续做什么”和“哪些路径已经证伪”在研究主线里已经比较清楚。

### 6.2 对“工程可维护性”来说，文档还不够

当前缺少的是一套专门面向工程维护的约束：

- 哪些文件必须优先拆；
- 共用逻辑该收敛到哪个 crate；
- 生成工件如何分级管理；
- API / worker / web 各自允许承载哪些职责；
- 什么时候必须补测试而不是继续加分支。

这个缺口现在由以下文档共同补齐：

- [工程治理方案](engineering-governance-plan.md)
- [开发质量门禁](development-quality-gates.md)
- [工程维护性 TODO](../roadmap/engineering-maintainability-todo.md)

## 7. 已有边界约束是否足够

### 7.1 产品边界约束：基本够用

当前已经明确约束：

- `position_guidance` 不是自动交易指令；
- release review 不能只看相对 guardrail；
- bridge snapshot 不能直接当正式命中率依据；
- 免费数据存在延迟和样本缺口；
- 只有达成 Go/No-Go 条件的 candidate 才能作为正式主模型。

这部分约束是够的，后续主要问题不在“没有边界”，而在“要持续执行边界”。

### 7.2 工程边界约束：需要补强

目前还没有足够明确地限制：

- 单文件体量；
- 重复逻辑允许停留多久；
- 生成工件目录的生命周期；
- demo 路径与正式 runtime 路径的分离程度；
- storage / worker / api 各自的抽象上限。

当前补强方式已经明确：

- 活跃任务真相源收口到两份 TODO；
- 本地统一门禁收口到 `just verify`；
- CI 自动执行 Rust fmt/test/clippy 与 Web build；
- 研究证据与 release review 必须回写文档，而不是只留在临时沟通里。

## 8. 建议的重构顺序

### 第一阶段：先补治理，不先大拆

1. 建立工程维护 TODO。
2. 明确生成工件治理规则。
3. 明确 `apps/worker`、`apps/api`、`apps/web` 的模块拆分目标。

### 第二阶段：优先拆 worker 和 API 共用逻辑

优先级最高：

1. 把 worker 中的 research / release / backfill / report 渲染拆成模块。
2. 把 API 与 worker 共用的概率数学、校准、观测切片、特征派生收敛到共享模块。
3. 把 `demo.rs` 中的 demo seed、history replay、sqlite source loader 拆开。该项已基本完成，后续转为观察 runtime helper 是否继续下沉。

### 第三阶段：再拆前端和 SQLite store

1. 拆 `App.tsx` 为按页面职责组织的 view/container/component。
2. 拆 `sqlite.rs` 为按聚合分组的 repository / store 模块。

## 9. 不建议现在做的事

- 不建议现在把整个项目改成微服务。
- 不建议现在换掉 React 或 Rust。
- 不建议为了“优雅”重写训练链路。
- 不建议在模型主线尚未稳定时，同时进行大规模行为重构。

这些动作收益低，且会直接打断当前最重要的“提前预警能力”攻关。

## 10. 结论

当前项目的 **方向和仓库级结构是对的**，但 **实现层已经进入需要治理的阶段**。

最关键的判断是：

- 现在还不是“推倒重来”的时机；
- 但也绝不是“继续往几个超大文件里堆功能”的阶段。

正确动作是：

1. 保持现有顶层架构；
2. 先补工程治理文档和 TODO；
3. 再按优先级拆 `worker`、`api`、共享逻辑和前端大文件；
4. 在不打断模型主线的前提下，把工程结构拉回可维护状态。

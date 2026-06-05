# 工程维护性 TODO

状态：`Draft`

最后更新：2026-06-05

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

- [x] 收口活跃 TODO 真相源：
  - 模型主线统一进 `crisis-probability-design-todo.md`
  - 工程治理主线统一进 `engineering-maintainability-todo.md`
  - 旧 `design/backlog/sqlite-plan` 文档只保留索引或专项背景角色
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
- [x] 增加统一质量门禁文档，明确：
  - 哪类改动必须先补设计；
  - 哪类改动必须跑 `just verify`；
  - 哪类候选必须补 `release-review-fast` / `release-review`。
- [x] 把本地与 CI 的基础检查收口到同一套门禁：
  - 本地统一入口 `just verify`
  - CI 自动执行 Rust fmt/test/clippy 与 Web build
- [x] 固化提交流程 checklist：
  - PR 模板要求填写活跃 TODO 归属
  - 要求说明 `just verify`、release review 与 artifact 归属证据

以上 P0 事项已由 [工程治理方案](../architecture/engineering-governance-plan.md) 落地，并已同步收紧 worker 的默认实验输出目录。
质量门禁细则见 [开发质量门禁](../architecture/development-quality-gates.md)。

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
- formal dataset summary 的 envelope 结构、split/scenario/regime 汇总、Markdown 渲染与 CLI 打印，已先迁入 `commands/dataset.rs`，再继续下沉到 `apps/worker/src/commands/dataset/report.rs`；`dataset.rs` 现在主要保留样本装配、split/scenario 约束与研究命令编排。
- formal dataset 的主样本装配、场景集加载/切分要求、scenario metadata 编码 helper 也已继续迁入 `commands/dataset.rs`，`main.rs` 进一步缩回到 actionability / 概率训练共享逻辑。
- formal dataset slice 的过滤、feature 列收集、CSV/JSON 导出与摘要打印也已一并收口到 `commands/dataset/report.rs`，避免 `dataset.rs` 同时持有“数据构建”和“报表输出”两类职责。
- 已新增 `apps/worker/src/formal.rs`，把 snapshot/formal dataset 共用的场景标签推导收敛成单一 helper，避免两条训练输入链路各自维护一套 crisis/actionability 标注逻辑。
- 已新增 `apps/worker/src/training.rs`，把 `ProbabilityTrainingRow/Input`、chronological split、label-mode 支持检查、formal bundle 训练管线以及 `forward_crisis` 标签 / regime helper 从 `main.rs` 中抽离，固定训练数据 contract 与训练编排的归属边界。
- 已新增 `apps/worker/src/actionability.rs`，把 actionability bundle 训练、阈值选择、校准策略、guardrail 与 actionability evaluation summary 从 `main.rs` 中拆出，供训练与 release review 共用。
- 已新增 `apps/worker/src/probability.rs`，把 probability bundle 训练、Platt 校准择优、threshold 选择、regime separation 诊断与 evaluation summary 从 `main.rs` 中拆出，收拢概率训练主链路的模块边界。
- `apps/worker/src/probability.rs` 本轮已继续瘦身：family overlay 的 audit spec、候选样本装配、family-aware/balanced split 与 overlay 训练已经拆到 `apps/worker/src/probability/overlay.rs`，主文件开始回到“概率主头 + 阈值/诊断/评估”主链路。
- `apps/worker/src/probability.rs` 本轮继续瘦身：calibration selection、threshold selection、regime-support threshold repair、threshold diagnostics / calibration evidence 已拆到 `apps/worker/src/probability/threshold.rs`，主文件进一步回到“概率主头训练 + regime evaluation / bundle summary”边界。
- 已新增 `apps/worker/src/model.rs`，把 logistic 拟合、样本加权、sign / regime pairwise 约束、Platt 校准、runtime 打分与基础概率评估从 `main.rs` 中拆出，避免训练数学细节继续和命令编排混在一起。
- release 相关的 `activate_release_with_runtime_guard`、review stage activate/restore、market scope resolve 也已迁到 `commands/release.rs`。
- `release review` 的 runtime snapshot 抓取、CLI 选项解析、对比 orchestration 与建议/总结 helper 已继续迁到 `commands/release/review.rs`，`commands/release.rs` 重新收缩为 release 生命周期壳层与共享 runtime guard 入口。
- `release review` 专属的 probability/actionability/runtime sanity guardrail、recommendation、summary helper 现已随主流程收口到 `commands/release/review.rs`。
- 已新增 `apps/worker/src/commands/release/probability.rs`，把 `probability-slice`、`formal-probability-slice`、`formal-probability-compare` 的 CLI 选项解析、bundle 评分、CSV/JSON 导出与摘要打印从 `commands/release.rs` 中拆出，release 主模块重新收缩到 publish / activate / rollback / review 主流程。
- `apps/worker/src/commands/release/probability/compare.rs` 本轮继续收走 formal probability compare 的阈值摘要、feature delta 聚合、窗口汇总、CSV/JSON 导出与 CLI 摘要打印，`commands/release/probability.rs` 已重新收缩到“slice CLI + bundle 评分 + formal compare 编排”主链路。
- `apps/worker/src/commands/release/probability/slice.rs` 本轮继续收走 runtime probability slice 的 JSON/CSV 导出、overlay 展开与 CLI 摘要打印；`apps/worker/src/commands/release/probability/formal.rs` 继续收走 formal dataset slice 的 bundle 打分、base model diagnostics、CSV/JSON 导出与 CLI 摘要打印；`apps/worker/src/commands/release/probability/common.rs` 负责共享的文件名/CSV helper。当前 `commands/release/probability.rs` 已从约 `1,153` 行收缩到约 `677` 行，边界回到“CLI 编排 + release/bundle 装配 + formal compare orchestration”。
- 已新增 `apps/worker/src/commands/release/review.rs`，把 `research release review` 的 CLI 编排、runtime snapshot 抓取、阶段切换/恢复、比较诊断、recommendation 与 summary 从 `commands/release.rs` 中拆出，减少 release 主模块的职责混杂。
- `apps/worker/src/commands/release/review/focus.rs` 本轮继续收走 structured signal counts、backtest scenario compare、scenario focus diagnostics、runtime actionable block/facet 统计与 primary failure mode 判定，`commands/release/review.rs` 已重新收缩到“review 编排 + runtime separation compare + recommendation / summary”主链路。
- 已新增 `apps/worker/src/release_review.rs`，把 release review 专属的 report wire structs、historical audit takeaways、failure mode / attribution / action / workstream 汇总、runtime regime probability / separation diagnostics，以及 review Markdown 渲染入口从 `main.rs` 中拆出，统一 release review helper 与报告数据结构的归属边界。
- `apps/worker/src/release_review.rs` 本轮继续瘦身：historical audit 的 failure mode / priority / attribution / action / workstream 汇总与 takeaways 已拆到 `apps/worker/src/release_review/historical.rs`，runtime regime probability / separation diagnostics、分类 helper 与 runtime takeaways 已拆到 `apps/worker/src/release_review/runtime.rs`；主文件开始回到“report wire structs + shared formatter + Markdown 壳层”边界。
- 已新增 `apps/worker/src/commands/db.rs`，把 `db init/seed/check` 从超大入口文件中拆出。
- 已新增 `apps/worker/src/commands/refresh.rs` 与 `commands/backfill.rs`，开始把免费数据刷新与回填入口从 `main.rs` 中剥离。
- 已新增 `apps/worker/src/scenario.rs`，把 `CrisisScenario`、action episode window、protected context、primary/forward scenario 选择和 action window label 这组场景时间窗逻辑从 `main.rs` 中拆出，固定场景标签与动作窗口 helper 的归属边界。
- 已新增 `apps/worker/src/support.rs`，把 `ApiReloadHistoryMode`、demo run、API fetch/reload、SQLite/raw payload IO、格式化 helper、解析 helper 和通用 rounding/hash/path helper 从 `main.rs` 中拆出，统一 worker 顶层支撑函数的归属边界。
- 已新增 `apps/worker/src/tests.rs`，把原先内联在 `main.rs` 的超大测试模块整体迁出，先把测试代码和运行时入口彻底解耦。
- `tests.rs` 已进一步收口为第一层测试聚合壳层，并按 `options / training / quality / review / split_requirements` 拆到 `apps/worker/src/tests/*.rs`；共享测试构造器已继续收敛到 `apps/worker/src/tests/fixtures.rs`，专题测试也已改成真实子模块，避免继续依赖 `include!` 共享词法作用域。
- 这一轮之后，`apps/worker/src/main.rs` 已从约 `7,573` 行继续收缩到约 `165` 行，当前已基本收口为“顶层模块声明 + 共享导出 + 常量 + 入口壳层”。
- 当前 `main.rs` 已不再直接承载训练管线、release review 报告结构、动作 episode / scenario 时间窗逻辑、通用 API/IO helper，也不再直接承载超大测试块；下一步可优先继续评估是否还需要把跨专题的少量测试导入/夹具进一步下沉，避免测试聚合壳层重新长胖。
- `apps/worker/src/commands/dataset.rs` 本轮继续瘦身：formal dataset 的报表结构、切片导出和 CLI 摘要打印已经拆到 `apps/worker/src/commands/dataset/report.rs`；split profile、scenario-aware split bounds、label support 与 scenario range helper 现已继续拆到 `apps/worker/src/commands/dataset/split.rs`，scenario catalog 装配与 metadata 编码 helper 已拆到 `apps/worker/src/commands/dataset/scenarios.rs`。当前 `dataset.rs` 已从约 `1218` 行收缩到约 `676` 行，后续可优先把注意力转向下一个超大热点模块。

### 3.2 API

- [x] 把 `apps/api/src/demo.rs` 中的 demo seed 与真实数据源加载拆开。
- [x] 把 historical replay / prediction snapshot bridge / cache key 逻辑拆开。
- [x] 把 `apps/api/src/assessment.rs` 中的特征构造、概率评分、posture 判定、position guidance、analogs 分模块。

当前进展：

- 已新增 `apps/api/src/data_source.rs`，把 `FC_DATA_MODE` 解析、SQLite/Postgres 装载、active release bundle 装配与 API reload 入口依赖的 `AppDataSource` / `AssessmentHistoryBuildMode` 从 `demo.rs` 中拆出。
- 已新增 `apps/api/src/history_replay.rs`，把 historical replay run 持久化、prediction snapshot cache、history cache key/method version、history point 转换和 release-aware cache refresh 判定从 `demo.rs` 中拆出。
- 已新增 `apps/api/src/backtest.rs`，把 scenario fallback、backtest timeline、rolling audit 和动作级历史判定规则从 `demo.rs` 中拆出，避免 demo seed 文件继续承载历史回测规则。
- 已新增 `apps/api/src/demo_seed.rs`，把静态 demo 指标样本、观测样本、源状态样本和 demo alert 构造从 `demo.rs` 中拆出，避免示例数据继续和历史装配/缓存逻辑耦合。
- 已新增 `apps/api/src/history_builder.rs`，把 assessment history 装配、SQLite prediction snapshot 重建与时间窗口筛选从 `demo.rs` 中拆出，`handlers.rs` / `data_source.rs` 已直接依赖新模块。
- `demo.rs` 当前已进一步收缩为 demo 当前截面装配、runtime assessment snapshot 组装与用户偏好加载；后续可以优先继续观察 `load_user_preferences` 是否值得下沉到 shared/runtime config 层。
- 已新增 `apps/api/src/assessment/posture.rs`，把 `time_to_risk_bucket`、posture clause、position guidance、用户偏好升降级和 summary 这条姿态决策链从 `assessment.rs` 中拆出，主装配逻辑只保留调用点。
- 已新增 `apps/api/src/assessment/probability.rs`，把 heuristic probability、bundle scoring、formal feature map、actionability 融合与相关测试依赖的 helper 从 `assessment.rs` 中拆出，避免模型评分逻辑继续和 assessment orchestration 混在一起。
- 已新增 `apps/api/src/assessment/context.rs`，把 runtime freshness、关键指标状态、事件确认、历史类比和 backtest summary 从 `assessment.rs` 中拆出，让解释层上下文与概率/姿态决策链解耦。
- 已新增 `apps/api/src/assessment/market_context.rs`，把 data trust、JPY carry、conviction、risk breadth 和相关观测窗口 helper 从 `assessment.rs` 中拆出，assessment 主文件已基本收缩为 runtime threshold 与总装配层。
- `apps/api/src/assessment/runtime_policy.rs` 本轮继续收走 runtime threshold、serving model policy、history runtime policy version 与 diagnostics；`apps/api/src/assessment/common.rs` 收走 rounding/format/pressure 这类共享 helper，`assessment.rs` 已从约 `1174` 行收缩到约 `982` 行。
- `assessment.rs` 当前剩余逻辑已主要是 assessment 总装配与内联测试模块；下一步可优先把 assessment 测试块外移到独立测试子模块，并继续观察 common helper 中哪些值得下沉到 shared crate。

### 3.3 Shared Logic

- [x] 收敛 `apply_platt_calibration`、观测值窗口切片、`difference_from_tail` 等重复函数。
- [x] 明确概率数学、特征派生、runtime threshold 诊断哪些属于共享领域逻辑，哪些属于 app-specific glue code。
- [x] 为共享逻辑补单元测试，避免训练侧和运行侧未来再次分叉。

当前进展：

- 已把 logistic 概率打分与 `Platt` 校准收敛到 `crates/domain/src/probability_bundle.rs`。
- `apps/api` 与 `apps/worker` 已开始复用同一套共享概率函数。
- 已新增 `crates/domain/src/observation_window.rs`，把按指标取历史窗口、按 as-of 排序、尾部 lookback 差值计算收敛到共享领域层。
- `apps/api` 与 `apps/worker` 已改用 `observation_history_for_indicator*` / `observation_value_difference_*`，避免训练侧与运行侧各写一套窗口切片和尾部差值。
- PIT 可见性过滤暂时保留在 worker 的 `observations_for_indicator` 包装函数内，因为它绑定 source publication timing、cutoff timezone 和 `PointInTimeMode`，不应在未完成边界设计前强行下沉到 domain。
- `crates/domain` 已补观测窗口排序、过滤、lookback 差值单测；`probability_bundle` 已覆盖共享 Platt 校准和派生特征 resolver。
- 共享边界已补到 [工程治理方案](../architecture/engineering-governance-plan.md)：纯概率打分、Platt 应用、观测窗口和纯 transform 进 `crates/domain`；训练拟合、阈值选择、release review 留 `apps/worker`；active release / 用户偏好 / response 装配留 `apps/api`；Web 只做展示翻译。
- 已新增 `crates/domain/src/formal_feature.rs`，把正式观测特征的 `feature id -> source indicator -> transform/lookback` 注册表收敛到 domain；API runtime 与 worker PIT feature snapshot 已共用该注册表。
- 后续新增免费数据指标或调整 lookback，应先改 `FORMAL_OBSERVATION_FEATURE_SPECS`，再由 API/worker 自动沿用同一口径。

## 4. P2：次级重构项

### 4.1 Storage

- [x] 把 `crates/storage/src/sqlite.rs` 按聚合拆分：
  - metadata / mappings
  - observations
  - alerts
  - releases
  - snapshots
  - formal datasets
  - historical replay

当前进展：

- 已新增 `crates/storage/src/sqlite/` 子模块目录，按聚合拆出 `metadata.rs`、`observations.rs`、`operational.rs`、`releases.rs`、`prediction_snapshots.rs`、`feature_snapshots.rs`、`formal_datasets.rs`、`historical_replay.rs`。
- `sqlite.rs` 当前保留连接/迁移、底层 mapper/id/parser helper、`RiskStore` trait 转接、seed struct 和 storage round-trip tests。
- 本轮是低风险机械拆分：SQL、表结构、public method 签名和调用方不变。

### 4.2 Web

- [x] 把 `apps/web/src/App.tsx` 拆成按页面和卡片组织的 view/container/component。
- [x] 把领域解释文本和格式化逻辑继续从页面组件中剥离。
- [x] 为决策面板、方法页、审计页明确单独的数据装配层。

当前进展（2026-06-03）：
- 已把决策页主体拆到 `apps/web/src/views/decision/DecisionView.tsx`。
- 已把决策页展示组件拆到 `apps/web/src/views/decision/components.tsx`。
- 已把决策页重型业务面板拆到 `apps/web/src/views/decision/panels.tsx`。
- 已把决策页解释逻辑和图表构造拆到 `apps/web/src/views/decision/logic.ts` 与 `charts.ts`。
- 已新增 `apps/web/src/views/decision/useDecisionViewModel.ts`，把决策页派生数据、图表模型、条款映射和关键指标定位集中到页面数据装配层。
- 已新增 `apps/web/src/views/decision/content.ts`，把长段解释文本、风险提示和空态文案从页面 TSX 中外提。
- 已新增 `apps/web/src/views/method/useMethodViewModel.ts` 与 `apps/web/src/views/method/content.ts`，开始把 method 页的解释文本、版本清单、阈值展示和限制说明从页面 TSX 中抽离。
- 已新增 `apps/web/src/views/audit/useAuditViewModel.ts` 与 `apps/web/src/views/audit/content.ts`，开始把 audit 页的运行态摘要、release/snapshot 行模型和说明文案从页面 TSX 中抽离。
- 已新增 `apps/web/src/views/backtests/useBacktestsViewModel.ts` 与 `apps/web/src/views/backtests/content.ts`，把回测页的轨迹图、摘要指标、审计指标、场景行和 episode 行模型从页面 TSX 中抽离。
- 已新增 `apps/web/src/views/sources/useSourcesViewModel.ts` 与 `apps/web/src/views/sources/content.ts`，把可信度指标、告警列表、数据源行模型和免费源策略说明从页面 TSX 中抽离。
- 已新增 `apps/web/src/views/drivers/useDriversViewModel.ts` 与 `apps/web/src/views/drivers/content.ts`，把维度卡片和结论摘要的派生展示数据从页面 TSX 中抽离。
- 已新增 `apps/web/src/views/events/useEventsViewModel.ts` 与 `apps/web/src/views/events/content.ts`，把事件层状态、缺口列表和最近事件表行模型从页面 TSX 中抽离。
- 已把主应用查询装配抽到 `apps/web/src/useConsoleData.ts`。
- 已把 drivers / indicators / sources / method / events / backtests / audit 全部拆到独立 view 文件，`lazyViews.tsx` 已删除。
- `apps/web/src/App.tsx` 已收缩到壳层（194 行），不再承载页面实现。
- 已把决策页首屏 prelude 拆到 `apps/web/src/views/decision/sections.tsx`。
- 已继续把决策页首屏 hero / 风险时距 / posture playbook 从主体文件拆到 `sections.tsx`，并把重复的 bullet list 抽到 `components.tsx`。
- 已把 why-now / relief / analog / action plan / event / JPY carry / backtest summary / rolling audit 等重型面板迁入 `panels.tsx`，`DecisionView.tsx` 目前约 249 行。
- 已重排决策页列布局：移除顶部三列同排的拉高结构，改为双列独立工作台，并把“历史类比”“为什么还没更糟”“回测摘要与用户参数”按主题重新分配到列内，降低大面积留白和超长右列。
- 已把“组合动作建议”重排为预算、建议动作、禁止动作、再入场条件、执行护栏分块展示；已把“回测与用户参数”拆成“回测摘要与用户参数”与“滚动审计与误报”两块。
- 当前决策页文件边界已变为：`DecisionView.tsx`（249 行壳层编排）/ `sections.tsx`（195 行首屏摘要）/ `panels.tsx`（458 行重型业务面板）/ `components.tsx`（265 行通用展示组件）/ `useDecisionViewModel.ts`（74 行数据装配层）/ `content.ts`（55 行页面文案）。
- 已去掉 Web 运行时对 `echarts` 的依赖，改为 `apps/web/src/simpleCharts.tsx` 里的轻量图表组件；原先的 500k+ 图表 chunk 警告已消失。
- 已补一轮窄屏治理：平板端保留更多双列信息密度，移动端顶部导航改为横向滚动，避免过早全部堆成单列。
- 已补 method / audit 表格的窄屏可读性：宽表最小宽度、横向滚动容器、滚动提示、窄屏字体收缩已就位。
- 已补 backtests / sources / events 的宽表滚动提示与最小宽度约束，移动端优先保证“可滚、可扫、可定位”。
- `apps/web/src/useConsoleData.ts` 已新增 `readyData` 聚合出口，`App.tsx` 不再手工串接一长串 `query.data` 判定；主壳层当前约 173 行，继续保持在壳层编排职责内。
- 已新增 `apps/web/src/views/shared/panelHelpers.tsx` 里的 `ResponsiveTable`，把 audit / method / backtests / sources / events / indicators 与决策页局部表格的响应式壳层收敛到一处，减少重复的 `table-wrap + thead/tbody` JSX。
- 已继续把 `SurfaceHeader` / `RuleBox` / `MetricGrid` 收敛到 `apps/web/src/views/shared/panelHelpers.tsx`，并覆盖 decision / audit / method / backtests / sources / events / drivers / indicators 等页面，减少重复的 `surface-head`、`rule-box` 与 `mini-metrics` 壳层。
- decision 页已改为复用 shared `Metric`，不再在 `views/decision/components.tsx` 维护第二套指标卡实现。
- decision 页已进一步复用 shared `BulletList` / `DriverList`，并通过 `emptyVariant` 兼容卡片内联空态；`views/decision/components.tsx` 已收缩为概率 tile、posture ladder、signal layer 和 budget bar 等真正 decision-specific 组件。
- decision 页的大块指标组已统一改用 `MetricGrid`，减少 `DecisionView.tsx` / `sections.tsx` / `panels.tsx` 里成片重复的 `Metric` JSX。
- 已新增 shared `DetailRows`，把 `GuideList` / `DriverList` 以及 decision/drivers 页局部 `list-row` 壳层收敛到一处，减少重复的“标题 + 说明 + 右侧分值” JSX。
- 已新增 shared `renderClauseBulletRows`，把 method 页 runtime/posture 条款的 bullet 映射收敛到共享层，避免继续手写 `bullet-row` + `describePostureClause` 展示循环。
- 已新增 shared `MetricPairsGrid`，把 audit / backtests / method / sources 页里反复出现的 `[label, value] -> MetricGrid items` 样板代码收敛到一处。
- 已新增 shared `StackedTableCell`，把 indicators / sources / audit 表格里反复出现的 `td > strong + span` 堆栈单元格收敛到共享层。
- 已新增 shared `PillTableCell`，把 decision / backtests 表格里反复出现的 `state-pill` 单元格收敛到共享层。
- method 页的“受保护压力窗口目录”也已同步改用 `MetricPairsGrid` 与 `StackedTableCell`，不再保留同类旧写法。
- 当前 shared table/list 展示积木已形成 `DetailRows` / `MetricPairsGrid` / `StackedTableCell` / `PillTableCell` / `renderClauseBulletRows` 这一组基础层，可继续优先承接 Web 端后续展示治理。
- 已新增 `apps/web/src/views/decision/builders.ts`，把 runtime prelude、风险档位、signal layer、历史类比、回测摘要、滚动审计 episode 行等纯展示拼装从 hook/面板层继续外提；`useDecisionViewModel.ts` 当前重新收缩为编排层。
- 决策页当前更多由 “`logic.ts` 解释规则 + `charts.ts` 图表模型 + `builders.ts` 纯展示行模型 + `useDecisionViewModel.ts` hook 编排 + `sections/panels/components` 渲染层” 组成，页面层已较少再直接拼字符串或拼表格行。
- 已新增 `apps/web/src/viewRegistry.tsx`，把导航定义、懒加载页面注册和 `readyData` 到各页面 props 的装配从 `App.tsx` 中继续下沉；主壳层当前只保留导航、状态提示和活动视图渲染。
- 已把各视图的顶栏标题/说明并入 `viewRegistry.tsx` 的导航元数据，切换到方法页、事件页、回测页时不再错误地继续显示“风险决策面板”标题。
- 当前决策页文件边界已变为：`DecisionView.tsx`（222 行壳层编排）/ `sections.tsx`（135 行首屏摘要）/ `panels.tsx`（281 行重型业务面板）/ `components.tsx`（163 行 decision-specific 小组件）/ `builders.ts`（405 行纯展示拼装）/ `useDecisionViewModel.ts`（189 行 hook 编排）/ `content.ts`（55 行页面文案）。
- 已把移动端/平板导航从横向滚动改为 4 列网格，并把顶部刷新按钮并回标题行，减少首屏空白和滚动条占位。
- 已把 `sources` 页改成“顶部摘要双列 + 下方全宽源状态表”，避免短说明卡与长表绑成同一栅格后在桌面端留下大块空白。
- 已把 `backtests` 页顶部摘要区改成桌面 3 列、宽屏退化 2 列、窄屏 1 列，去掉 3 张卡套 2 列时留下的半屏空洞。
- shared `MetricGrid` / `Metric` 已补 `hint` 与长 token value class 支持，像 release id、bundle id 这类长字符串不再轻易把指标卡撑坏。
- 已把 `format.ts` 扩成前端显示标签层，统一兜底 `dimension` / `source_id` / `source_type` / `event_type` / 事件相关指标 ID 的人话映射，减少页面直接暴露内部枚举和值域代码。
- 已继续把 `unit` 的机器值（如 `percent` / `index` / `count` / `jpy_per_usd`）映射成更适合面板阅读的展示文本，避免指标页和决策页继续出现底层单位代码。
- 已继续把 `audit / method / decision` 的 runtime / release / PIT / 概率模式等内部英文术语压到统一显示层，审计页表头、状态值和说明文案已改成中文可读版本，并保留必要的技术码作括号提示。
- 已继续把 `App` 顶栏元信息、视图标题说明、决策页动态说明文本和用户约束摘要做前端“人话翻译”层，减少 `As of / Mode / profile=neutral / structural score / prepare / normal` 这类直接暴露给用户的内部字样。
- 已把 method 页的版本清单、历史阈值策略版本、受保护压力窗口目录版本和配置来源改为压缩展示，长技术串默认不再整段铺在页面上。
- 已给 audit 页补“审计摘要”概览，并把 release bundle 路径压缩成短引用，降低进入明细表前的认知负担。
- 已把 sources 页补成“总体可信度摘要 + 免费源策略 + 源状态明细”的三层结构，并把 dataset/raw health message 收敛成中文可读说明。
- 已给 indicators 页新增 view model 与阅读导引，把裸表改成“指标摘要 + 阅读规则 + 压缩明细表”，移除原始 indicator id 的直接暴露。
- 已继续把 drivers 页的摘要卡、维度焦点和缓冲项名称接入统一“人话翻译”层，清掉前端直接暴露的 `filing` 等原始 display name。
- 已继续把 decision 页下半区的人机边界重新命名并解释，例如把“旧逻辑回推”改成“过渡动作映射”、把“旧风险引擎解释”降为“旧版评分层辅助解释”，并给历史类比补充可视化阅读提示。
- 已把 backtests 页补成“顶部速览 + 解读顺序 + 回测/滚动审计明细”的三层结构，并把场景说明与误报区间说明接入统一文案人话化。
- 已给 events 页补充“最近事件日 / 最常见事件 / 涉及维度 / 关联指标”结构摘要，减少待补确认为空时的桌面端留白。
- 已给 method 页补“当前方法摘要”首屏，把概率模式、动作层、PIT 状态和运行状态先翻成人话，再把版本清单下沉到单独的“版本与边界”区块。
- 已继续清理 audit / method / decision 的 release id 展示，把 `extmix`、完整 bundle 文件名和原始模型路径从正文显示中移出，统一通过 `releaseIdLabel` 输出“候选版本 / 主线版本 + 日期”。
- 已把 indicators 页摘要、焦点指标和明细表接入统一指标名映射，避免不同页面继续各自暴露后端 `display_name` 或内部 indicator id。
- 已继续把 sources / audit / method / decision / charts 里残留的页面私有 helper（如 source lag/quality、release review 标签、人话翻译、时间轴换行）抽到 `apps/web/src/format.ts`，页面层不再持有这些解释/格式化细节。
- Web 页面当前剩余重点：视需要继续把 `builders.ts` 中的高复用行模型向 shared 层沉淀。

## 5. 约束机制

- [ ] 新增或显著扩展功能时，如果目标文件已经是当前仓库前几位的大文件，优先先拆模块，再加功能。
- [x] 生成工件进入 Git 前，必须说明它属于：
  - 正式 release 工件；
  - 基线对照证据；
  - 还是临时研究副产物。
- [ ] 任何影响训练口径和运行口径的修改，都要检查共享函数是否已统一。
- [x] 任何新的仓位建议或动作规则，都不能绕开现有 `playbook`、`Go/No-Go` 和 “非自动交易指令”边界。
- [x] 本地提交前统一运行 `just verify`，不要继续靠人工记忆零散执行 `fmt/test/lint/web-build`。
- [x] CI 自动执行与本地一致的核心检查，不再只靠本地手工自觉。

## 6. 完成定义

以下条件满足时，才可以认为工程维护性从“高风险”进入“可持续”：

- [ ] `apps/worker/src/main.rs` 不再承担所有主线职责。
- [x] `apps/api/src/demo.rs` 不再同时承载 demo seed、真实历史回放和 runtime bridge。
- [x] API / worker 的重复概率数学与观测窗口逻辑已收敛。
- [x] `crates/storage/src/sqlite.rs` 已按聚合拆开。
- [x] `apps/web/src/App.tsx` 已拆成稳定组件层次。
- [x] 生成工件治理规则已落文档并落实到提交流程。
- [x] 活跃 TODO 真相源已收口，不再多份 roadmap 并行承载当前任务。
- [x] 本地质量门禁与 CI 自动检查已落地。

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

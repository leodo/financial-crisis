# 工程维护性 TODO

状态：`Draft`

最后更新：2026-06-03

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
- 已新增 `apps/worker/src/probability.rs`，把 probability bundle 训练、Platt 校准择优、threshold 选择、regime separation 诊断与 evaluation summary 从 `main.rs` 中拆出，收拢概率训练主链路的模块边界。
- 已新增 `apps/worker/src/model.rs`，把 logistic 拟合、样本加权、sign / regime pairwise 约束、Platt 校准、runtime 打分与基础概率评估从 `main.rs` 中拆出，避免训练数学细节继续和命令编排混在一起。
- release 相关的 `activate_release_with_runtime_guard`、review stage activate/restore、market scope resolve 也已迁到 `commands/release.rs`。
- `release review` 的 runtime snapshot 抓取与 orchestration 也已迁到 `commands/release.rs`。
- `release review` 专属的 probability/actionability/runtime sanity guardrail、recommendation、summary helper 也已开始跟随迁移。
- 已新增 `apps/worker/src/commands/db.rs`，把 `db init/seed/check` 从超大入口文件中拆出。
- 已新增 `apps/worker/src/commands/refresh.rs` 与 `commands/backfill.rs`，开始把免费数据刷新与回填入口从 `main.rs` 中剥离。
- 当前 `main.rs` 已主要保留底层 research/helper/训练实现；下一步继续按实现体拆出 formal dataset / pipeline 内部 helper，以及剩余 release review runtime 细节。

### 3.2 API

- [x] 把 `apps/api/src/demo.rs` 中的 demo seed 与真实数据源加载拆开。
- [x] 把 historical replay / prediction snapshot bridge / cache key 逻辑拆开。
- [ ] 把 `apps/api/src/assessment.rs` 中的特征构造、概率评分、posture 判定、position guidance、analogs 分模块。

当前进展：

- 已新增 `apps/api/src/data_source.rs`，把 `FC_DATA_MODE` 解析、SQLite/Postgres 装载、active release bundle 装配与 API reload 入口依赖的 `AppDataSource` / `AssessmentHistoryBuildMode` 从 `demo.rs` 中拆出。
- 已新增 `apps/api/src/history_replay.rs`，把 historical replay run 持久化、prediction snapshot cache、history cache key/method version、history point 转换和 release-aware cache refresh 判定从 `demo.rs` 中拆出。
- `demo.rs` 当前主要收缩为 demo seed、assessment history/backtest 生成、historical replay 缓存与审计辅助逻辑，后续可以继续沿 replay/cache 边界拆分。
- 已新增 `apps/api/src/assessment/posture.rs`，把 `time_to_risk_bucket`、posture clause、position guidance、用户偏好升降级和 summary 这条姿态决策链从 `assessment.rs` 中拆出，主装配逻辑只保留调用点。
- 已新增 `apps/api/src/assessment/probability.rs`，把 heuristic probability、bundle scoring、formal feature map、actionability 融合与相关测试依赖的 helper 从 `assessment.rs` 中拆出，避免模型评分逻辑继续和 assessment orchestration 混在一起。
- 已新增 `apps/api/src/assessment/context.rs`，把 runtime freshness、关键指标状态、事件确认、历史类比和 backtest summary 从 `assessment.rs` 中拆出，让解释层上下文与概率/姿态决策链解耦。
- 已新增 `apps/api/src/assessment/market_context.rs`，把 data trust、JPY carry、conviction、risk breadth 和相关观测窗口 helper 从 `assessment.rs` 中拆出，assessment 主文件已基本收缩为 runtime threshold 与总装配层。
- `assessment.rs` 当前剩余逻辑已主要是 runtime threshold / serving policy 与少量通用格式化 helper；后续如果再扩展，可以优先考虑把共用小工具继续下沉到 shared crate。

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

- [x] 把 `apps/web/src/App.tsx` 拆成按页面和卡片组织的 view/container/component。
- [ ] 把领域解释文本和格式化逻辑继续从页面组件中剥离。
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
- Web 页面当前剩余重点：继续抽共享文案/格式化 helper，并视需要继续把 `builders.ts` 中的高复用行模型向 shared 层沉淀。

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

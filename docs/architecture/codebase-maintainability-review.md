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
- `apps/worker/src/commands/release.rs` 又进一步把概率切片研究工具拆到 `apps/worker/src/commands/release/probability.rs`，并把 release review 的 CLI 选项、runtime snapshot、比较诊断、建议与总结继续拆到 `apps/worker/src/commands/release/review.rs`；本轮又继续拆成 `release.rs` + `release/options.rs` + `release/lifecycle.rs` + `release/guardrails.rs`，父文件已从约 `774` 行收缩到约 `35` 行；CLI 选项解析、publish/list/show/activate/rollback 生命周期命令，以及 actionability/probability/runtime/operational guardrail helper 已各自落回独立边界；
- `apps/worker/src/commands/release/probability/slice.rs` 已收走 runtime probability slice 的 JSON/CSV 导出、horizon overlay 展开与 CLI 摘要打印；`commands/release/probability/formal.rs` 已收走 formal dataset slice 的 bundle 打分、base model diagnostics、CSV/JSON 导出与 CLI 摘要打印；`commands/release/probability/common.rs` 负责文件名与 CSV 转义这类共享小工具；`commands/release/probability/compare.rs` 则继续只保留 formal probability compare 的阈值摘要、feature delta 聚合、窗口汇总、CSV/JSON 导出与 CLI 摘要打印；本轮又继续拆成 `probability.rs` + `probability/options.rs` + `probability/execute.rs`，父文件已从约 `679` 行收缩到约 `15` 行；3 组 CLI 选项解析、3 个 research orchestration 入口，以及 historical replay / bundle 读取 helper 已各自落回独立边界；
- `apps/worker/src/commands/release/review/focus.rs` 本轮已进一步从约 `1072` 行收缩到约 `6` 行；backtest scenario compare / focus 入口 helper 已拆到 `apps/worker/src/commands/release/review/focus/backtest.rs`，scenario focus diagnostics 与 runtime actionable block/facet 汇总先拆到 `apps/worker/src/commands/release/review/focus/runtime.rs`，随后 `runtime.rs` 又继续从约 `987` 行收缩到约 `5` 行，structured signal / actionable signal helper 已拆到 `apps/worker/src/commands/release/review/focus/runtime/signals.rs`；本轮原先约 `432` 行的 `runtime/blocks.rs` 也已继续细分为 `runtime/blocks/{gating,facets,counts,failure}.rs`，把 runtime block 分类与诊断文案、continuity facet 构造、block/facet 计数汇总与 primary failure mode 判定重新收回独立边界，父文件收口到约 `14` 行；此前原先约 `451` 行的 `runtime/scenario.rs` 也已继续细分为 `runtime/scenario/{mod,window,points,summary}.rs`，把场景窗口准备、interesting point 比较与最终诊断汇总重新收回独立边界，父文件收口到约 `75` 行；
- `apps/worker/src/commands/release/review.rs` 本轮也已继续从约 `893` 行收缩到约 `258` 行；CLI 选项解析已拆到 `apps/worker/src/commands/release/review/options.rs`，runtime snapshot 抓取与 activate/restore helper 已拆到 `apps/worker/src/commands/release/review/snapshot.rs`，release compare / runtime separation compare 已拆到 `apps/worker/src/commands/release/review/comparison.rs`，recommendation / CLI summary 打印已拆到 `apps/worker/src/commands/release/review/summary.rs`；主文件重新回到“review orchestration + 子模块导出”边界；
- `apps/worker/src/commands/snapshot.rs` 本轮也已继续从约 `401` 行收缩到约 `10` 行；prediction snapshot query / export 参数解析已拆到 `apps/worker/src/commands/snapshot/options.rs`，snapshot list/export/dataset research orchestration 与默认 heuristic training release 选择已拆到 `apps/worker/src/commands/snapshot/execute.rs`，CSV/JSON 渲染与写盘 helper 已拆到 `apps/worker/src/commands/snapshot/render.rs`；父文件重新回到 “snapshot 子模块导出壳层”边界；
- `apps/worker/src/training.rs` 已进一步收走 formal bundle 训练管线、`forward_crisis` 标签 / regime helper；本轮又继续拆成 `training.rs` + `training/{types,split,regimes,pipeline}.rs`，把训练数据 contract、chronological split、regime/label helper 与 bundle 训练写盘重新收回独立边界；`apps/worker/src/release_review.rs` 也已继续收走 release review 专属 report wire structs、historical audit helper、runtime regime diagnostics 与 Markdown 渲染入口；
- `apps/worker/src/release_review/historical.rs` 先继续收走 failure mode、historical audit priority / attribution / action / workstream 汇总与 takeaways，随后又进一步拆成 `apps/worker/src/release_review/historical/{mod,failure_modes,priorities,attribution,workstreams}.rs`，把 failure summary、priority 排序、attribution/action 汇总和 workstream/takeaways 重新收回独立边界；`apps/worker/src/release_review/runtime.rs` 先继续收走 runtime regime probability / separation diagnostics 与 runtime takeaways，随后又进一步拆成 `apps/worker/src/release_review/runtime/{mod,diagnostics,regimes,takeaways}.rs`，把 runtime review 装配、regime 分析和 takeaways 重新收回独立边界，`release_review.rs` 开始回到“report wire structs + shared formatter + Markdown 壳层”边界；
- `apps/worker/src/reporting.rs` 本轮已从约 `1131` 行收缩到约 `73` 行；release review Markdown 已拆到 `apps/worker/src/reporting/release_review.rs`，rolling audit Markdown 已拆到 `apps/worker/src/reporting/audit.rs`，主文件重新回到“写盘入口 + 模块导出”边界；
- `apps/worker/src/reporting/release_review.rs` 本轮也已继续从约 `854` 行收缩到约 `54` 行；release review 总览已拆到 `apps/worker/src/reporting/release_review/overview.rs`，historical audit 渲染已拆到 `apps/worker/src/reporting/release_review/historical.rs`，focus scenarios 渲染已拆到 `apps/worker/src/reporting/release_review/focus.rs`，runtime/actionability/guardrail/recommendation 渲染已拆到 `apps/worker/src/reporting/release_review/diagnostics.rs`；父文件重新回到“报告总装配 + 子模块导出”边界；
- `apps/worker/src/model.rs` 本轮已进一步从约 `706` 行收缩到约 `192` 行；样本标签/权重策略已拆到 `apps/worker/src/model/weighting.rs`，`Platt` 校准、runtime scoring 与评估 helper 已拆到 `apps/worker/src/model/calibration.rs`，主文件已回到“拟合主循环 + 少量共享数学 helper”边界；
- `apps/worker/src/commands/dataset/report.rs` 已继续收走 formal dataset summary/slice 的 envelope、split/scenario/regime 汇总、Markdown/CSV/JSON 渲染与 CLI 摘要打印；本轮又继续从约 `834` 行收缩到约 `105` 行，formal dataset summary 汇总已拆到 `apps/worker/src/commands/dataset/report/summary.rs`，slice 过滤/导出与文件名 helper 已拆到 `apps/worker/src/commands/dataset/report/slice.rs`，Markdown/CSV/CLI 渲染已拆到 `apps/worker/src/commands/dataset/report/render.rs`；此前 `apps/worker/src/commands/dataset/split.rs` 已继续收走 split profile、scenario-aware split bounds、label support 与 scenario range helper，`apps/worker/src/commands/dataset/scenarios.rs` 收走 scenario catalog 装配与 metadata 编码 helper；本轮又把 `commands/dataset.rs` 进一步拆成 `dataset.rs` + `dataset/options.rs` + `dataset/build.rs` + `dataset/execute.rs`，父文件已从约 `676` 行收缩到约 `35` 行，CLI 选项解析、formal dataset 主样本装配与 `build/list/summarize/slice` 研究命令编排现已各自落回独立边界；
- `apps/worker/src/commands/backfill.rs` 本轮也已继续从约 `738` 行收缩到约 `11` 行；`commands/backfill/options.rs` 负责免费数据回填的通用时间窗/过滤/水位回退选项、FRED 模式切换与 BOJ dataset 解析；原先约 `500` 行的 `commands/backfill/execute.rs` 现已进一步收口到约 `10` 行，`commands/backfill/execute/dispatch.rs` 负责 CLI source 分发，`execute/market.rs` 负责 FRED/Treasury/World Bank/BOJ/JPY carry 这类 mapping 驱动的市场数据回填，`execute/events.rs` 负责 GDELT/SEC EDGAR 这类带 payload/alert 的事件源回填，`execute/shared.rs` 负责共享 seeded store 打开与 chunked mapping 执行器；父文件重新回到“backfill execute 子模块导出壳层”边界；
- `apps/worker/src/commands/release/probability/compare.rs` 本轮也已继续从约 `712` 行收缩到约 `141` 行；`commands/release/probability/compare/build.rs` 负责 formal probability compare 的窗口对齐、阈值/命中统计、feature delta 聚合与窗口汇总，`commands/release/probability/compare/render.rs` 负责 JSON/CSV 写盘和 CLI 摘要打印；父文件重新回到“compare 结构定义 + 子模块导出壳层”边界；
- `apps/worker/src/commands/pipeline.rs` 本轮也已继续从约 `830` 行收缩到约 `13` 行；CLI 选项解析与默认 release prefix 规则已拆到 `apps/worker/src/commands/pipeline/options.rs`，训练/发布命令执行与控制台摘要打印已拆到 `apps/worker/src/commands/pipeline/execute.rs`；随后原先约 `411` 行的 `apps/worker/src/commands/pipeline/dataset.rs` 又继续细分为 `dataset/{features,formal,snapshot}.rs`，把 transitional/formal feature helper、formal dataset 读取/校验与 snapshot 训练数据装配重新收回独立边界；父文件现已回到“dataset 子模块导出壳层”边界；
- `apps/worker/src/commands/feature.rs` 本轮也已继续从约 `727` 行收缩到约 `16` 行；CLI 选项解析已拆到 `apps/worker/src/commands/feature/options.rs`，feature snapshot build/list 主流程与 snapshot 复用/重建逻辑已拆到 `apps/worker/src/commands/feature/snapshot.rs`，PIT 可见性与时区截止规则已拆到 `apps/worker/src/commands/feature/visibility.rs`，coverage / core-feature gate / quality grade helper 已拆到 `apps/worker/src/commands/feature/coverage.rs`；父文件重新回到“feature 子模块导出壳层”边界；
- `apps/api/src/assessment/runtime_policy.rs` 本轮继续收走 runtime threshold、serving model policy、history runtime policy version 与 diagnostics；`apps/api/src/assessment/common.rs` 收走 rounding/format/pressure 这类跨子模块共享 helper，随后又把 `assessment.rs` 底部内联测试整体外移到 `apps/api/src/assessment/tests.rs`，`assessment.rs` 已从约 `1174` 行收缩到约 `241` 行，主文件进一步回到“assessment 总装配 + method version / snapshot envelope”边界；本轮又把 `apps/api/src/assessment/posture.rs` 继续拆成 `posture.rs` + `posture/guidance.rs` + `posture/position.rs`，父文件已从约 `901` 行收缩到约 `5` 行；风险时距与 posture 条款判定、用户偏好升降级已落到 `guidance.rs`，仓位预算、动作手册、执行护栏与治理条款已落到 `position.rs`；随后 `apps/api/src/assessment/tests.rs` 也继续拆成 `assessment/tests/mod.rs` + `actionability.rs` + `time_bucket.rs` + `posture.rs` + `position.rs`，原约 `740` 行的聚合测试文件已收口为主题测试模块；本轮又把 `assessment/tests/posture.rs` 继续细分为 `assessment/tests/posture/{mod,prepare,hedge}.rs`，让 prepare / hedge 场景断言重新回到独立测试边界；
- `apps/api/src/assessment/context.rs` 本轮也已继续从约 `406` 行收缩到约 `10` 行；runtime freshness metadata 已拆到 `apps/api/src/assessment/context/runtime.rs`，关键指标新鲜度卡片已拆到 `assessment/context/indicators.rs`，事件确认解释已拆到 `assessment/context/events.rs`，backtest summary 与空 rolling-audit fallback 已拆到 `assessment/context/backtests.rs`，历史类比与相似度说明已拆到 `assessment/context/analogs.rs`；父文件重新回到 “context 子模块导出壳层”边界；
- `apps/worker/src/probability/overlay.rs` 本轮已进一步从约 `1260` 行收缩到约 `438` 行；family overlay 的 audit spec / audit 汇总已拆到 `apps/worker/src/probability/overlay/audit.rs`，样本筛选、family-aware/balanced split 与 support helper 先拆到 `apps/worker/src/probability/overlay/split.rs`，随后 `split.rs` 又继续细分为 `apps/worker/src/probability/overlay/split/{mod,dataset,balanced,bounds}.rs`，把数据集构建、balanced fallback、family-aware boundary search 和共享 support 结构重新收回独立边界；`apps/worker/src/probability/threshold.rs` 本轮也已进一步从约 `1080` 行收缩到约 `108` 行，calibration selection / calibration strategy 已拆到 `apps/worker/src/probability/threshold/calibration.rs`，threshold diagnostics / calibration evidence 已拆到 `apps/worker/src/probability/threshold/diagnostics.rs`；随后原先约 `496` 行的 `apps/worker/src/probability/threshold/decision.rs` 又继续细分为 `decision/{selection,regime,metrics}.rs`，把阈值候选/打分、regime-support 修复与 decision metrics/regime hit 汇总重新收回独立边界；本轮又把原先内联在 `overlay.rs` 里的 family overlay 测试整体外移到 `apps/worker/src/probability/overlay/tests.rs`，让运行时代码重新收口到约 `131` 行纯训练编排壳层；`probability.rs` 进一步回到“主头训练 + regime evaluation / bundle summary”边界；
- `apps/worker/src/scenario.rs` 本轮也已继续从约 `489` 行收缩到约 `15` 行；`scenario/models.rs` 负责 `CrisisScenario`、`ActionEpisodePhase`、`ActionEpisodeSelection`，`scenario/timing.rs` 负责 action episode window / phase 与共享日期 helper，`scenario/episodes.rs` 负责 actionability level 映射、action window label、dominant episode 与 protected context phase，`scenario/horizon.rs` 负责 horizon support、anchor/start label helper，`scenario/selection.rs` 负责 primary / forward scenario 选择；父文件重新回到“scenario 子模块导出壳层”边界；
- `apps/worker/src/support.rs` 已继续收走 `ApiReloadHistoryMode`、demo run、API fetch/reload、SQLite/raw payload IO、格式化 helper 和通用 rounding/hash/path helper；
- 原先内联在 `main.rs` 的超大测试块已整体迁到 `apps/worker/src/tests.rs`，共享测试构造器已继续下沉到 `apps/worker/src/tests/fixtures.rs`；其中原本约 `1344` 行的 `apps/worker/src/tests/training.rs` 本轮又继续拆成 `apps/worker/src/tests/training/{visibility,scenario_regimes,sign_constraints,family_constraints}.rs` 与聚合 `mod.rs`，同时把 `training/weighting.rs` 进一步细分为 `training/weighting/mod.rs` 与 `training/weighting/{negative_weights,pairwise,target_labels,positive_weights}.rs`，把样本权重/目标测试再按职责拆细；原本约 `624` 行的 `apps/worker/src/tests/options.rs` 也继续拆成 `apps/worker/src/tests/options/mod.rs` 与 `options/{refresh,release,snapshots,dataset,pipeline}.rs`，把 CLI 参数、dataset gate 与 pipeline 形状测试按职责收回独立边界；原本约 `1254` 行的 `apps/worker/src/tests/review.rs` 也继续拆成 `apps/worker/src/tests/review/{historical_audit,runtime}.rs`、`review/focus/mod.rs` 与 `review/focus/{comparison,diagnostics,continuity,failure_modes}.rs`，把 scenario focus 测试进一步按主题拆细；原本约 `983` 行的 `apps/worker/src/tests/quality.rs` 也继续拆成 `apps/worker/src/tests/quality/{render,actionability,probability_thresholds,regime_guardrails}.rs` 与聚合 `mod.rs`，option parsing / training / quality / review / split requirement 也已切成真实测试子模块而不再依赖 `include!` 聚合；
- `main.rs` 体量已从约 `7.6k` 行进一步降到约 `165` 行；
- 因此，worker 当前的主要维护风险已从“所有 release 能力都堆在一个文件里”，下降为“运行时代码已基本按边界收口，测试层也开始具备稳定模块边界；后续仍可继续把 cross-topic fixture 与少量共享导入继续收窄”。随着 `overlay`、`overlay/split`、`threshold`、`focus`、`focus/runtime`、`review`、`release_review/runtime`、`release_review/historical`、`reporting/release_review`、`dataset/report`、`pipeline`、`release`、`feature`、`snapshot`、`release/probability`、`dataset`、`backfill`、`release/probability/compare`、`assessment/posture`、`assessment/context`、`assessment/probability`、`web/format`、`assessment/tests`、`demo_seed`、`history_replay`、`backtest` 主文件进一步收口之后，当前更真实的下一阶段热点已转向仍偏大的 shared model / connector 文件，例如 `crates/domain/src/probability_bundle.rs`、`crates/ingestion/src/connectors/sec_edgar.rs` 与 `crates/domain/src/scenario_catalog.rs`。

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
- 静态 demo 指标样本、观测样本、源状态样本和 demo alert 构造已拆到 `apps/api/src/demo_seed.rs`；本轮又继续拆成 `demo_seed.rs` + `demo_seed/{indicators,observations,sources,alerts}.rs`，父文件回到纯导出壳层；
- `history_replay.rs` 本轮又继续拆成 `history_replay.rs` + `history_replay/cache.rs` + `history_replay/transform.rs`，历史重放缓存持久化、水位校验、cache key/method version 与 history point / prediction snapshot 转换已各自落回独立边界；
- `backtest.rs` 本轮又继续拆成 `backtest.rs` + `backtest/{actionability,scenarios,rolling_audit,timeline}.rs`，动作级历史判定、场景目录/回退模板、滚动审计 episode 分类与 timeline 构造已各自落回独立边界；
- `assessment/probability.rs` 本轮又继续拆成 `assessment/probability.rs` + `assessment/probability/{heuristic,features,actionability,trace}.rs`，启发式概率、formal/runtime 特征映射、动作置信度融合与 bundle trace / overlay diagnostics 已各自落回独立边界；
- assessment history 装配、SQLite prediction snapshot 重建和窗口筛选已拆到 `apps/api/src/history_builder.rs`。

因此，`demo.rs` 的风险已明显下降，当前已主要收缩为 demo 当前截面装配、runtime assessment snapshot 组装与用户偏好加载；本轮又把 demo/runtime bridge 相关测试整体外移到 `apps/api/src/demo/tests.rs`，让运行时代码和测试边界彻底分开。与此同时，`assessment/posture/guidance.rs` 也已继续拆成 `guidance.rs` + `guidance/{clauses,counters,preferences}.rs`，把 posture clause、确认计数和用户偏好升降级从主文件里收口出来；`apps/api/src/lib.rs` 底部的端点集成测试也已整体外移到 `apps/api/src/tests.rs`，入口文件重新回到 router / middleware / refresh loop / tracing 壳层；`apps/api/src/assessment/tests/posture.rs` 也继续拆成 `assessment/tests/posture/{mod,prepare,hedge}.rs`，让 prepare / hedge 场景断言重新回到独立测试边界；`apps/api/src/handlers.rs` 又继续把 research audit 相关的 release-review wire type、文件扫描与 SQLite audit 装配外移到 `apps/api/src/handlers/research_audit.rs`，主文件回到轻量 query type、日期解析、常规只读 handler 与 reload 入口。`demo_seed.rs`、`history_replay.rs`、`backtest.rs` 与 `assessment/probability.rs` 也不再继续把示例样本、历史缓存、动作级历史判定、滚动审计与评分融合逻辑堆在单文件里。API 侧当前的大块运行时代码已基本按边界收口；前端侧 `apps/web/src/types.ts` 本轮也已继续拆成 `types.ts` + `types/{common,risk,backtest,assessment,research}.ts`，`apps/web/src/styles.css` 也已继续拆成 `styles.css` + `styles/{base,surfaces,analysis,responsive}.css`，把 600 行级别的类型聚合和 1k+ 行样式聚合分别按主题收回独立边界。下一阶段重点转向少数剩余 worker / web 运行时代码热点。

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

这一轮已经把原本 1600+ 行的 `crates/storage/src/sqlite.rs` 拆成：

- 壳层入口 `sqlite.rs`；
- 聚合子模块 `metadata.rs`、`observations.rs`、`operational.rs`、`releases.rs`、`prediction_snapshots.rs`、`feature_snapshots.rs`、`formal_datasets.rs`、`historical_replay.rs`；
- 共享底层子模块 `helpers.rs`、`rows.rs`、`migrations.rs`、`seeds.rs` / `seeds/*`、`tests/mod.rs` / `tests/*`。

这让 `sqlite.rs` 本身已经回到“连接壳层 + 常量/record type + trait 转接”的轻量边界；同时 `sqlite/metadata.rs` 也已继续收口为模块壳层，并把实现拆到 `sqlite/metadata/catalog.rs` 与 `sqlite/metadata/mappings.rs`；本轮又把 `sqlite/metadata/catalog.rs` 继续细分成 `catalog.rs` + `catalog/seeds.rs` + `catalog/upsert.rs`，让 metadata seed 定义、upsert SQL 与初始化编排重新落回独立边界；`sqlite/seeds.rs` 已进一步收口为模块壳层，seed 定义与 mapping helper 下沉到 `sqlite/seeds/indicator_catalog.rs` 与 `sqlite/seeds/mappings.rs`；本轮又把 `sqlite/seeds/indicator_catalog.rs` 继续细分成 `indicator_catalog.rs` + `indicator_catalog/{fred,boj,world_bank,sec_events,gdelt}.rs`，让各免费数据源的 indicator seed 列表重新落回独立边界；`sqlite/formal_datasets.rs` 本轮也继续拆成 `formal_datasets.rs` + `formal_datasets/{datasets,rows}.rs`，把 dataset manifest upsert/load/list 与 dataset row replace/list 重新收回独立边界；`sqlite/historical_replay.rs` 本轮也继续拆成 `historical_replay.rs` + `historical_replay/{runs,points}.rs`，把 replay run upsert/load/list 与 replay assessment point replace/list 重新收回独立边界；`sqlite/tests.rs` 也已拆成 `sqlite/tests/mod.rs` 与多份主题测试模块。当前存储层仍有二级维护风险：

- `sqlite/seeds/indicator_catalog.rs` 虽已收口为共享 seed type + wrapper 壳层，但若后续继续扩展来源或复用规则，仍可能再次堆入跨来源共享逻辑；
- 存储测试虽然已按主题拆开，但若 replay / dataset 断言继续扩展，仍应优先沿着 `tests/historical_replay/{fixtures,runs,points}.rs` 与 `tests/formal_datasets/{fixtures,snapshots,rows}.rs` 继续收窄，而不是重新回到单文件堆叠；
- metadata catalog 的 SQL/seed 边界已经收口，但后续若继续扩展数据源，仍应优先落到 `catalog/seeds.rs` 与 `catalog/upsert.rs`，避免重新把编排壳层堆大。

因此，存储层风险已经从“单个超大入口文件”转为“个别剩余运行时子模块需继续观察增长，以及测试主题需要沿现有 fixture / topic 边界持续收窄”。

### 4.5 Web `App.tsx` 已开始变成页面总控 + 领域解释器

`apps/web/src/App.tsx` 同时承担：

- 整体布局；
- tab 导航；
- 决策说明；
- 指标解释；
- posture 与 position guidance 展示；
- analog / probability / JPY carry 卡片编排。

这说明前端虽然没有选错框架，但组件层次还不够细，继续增加方法页、研究页、审计页后会继续膨胀。

最新进展：

- `apps/web/src/App.tsx` 已较早收缩回壳层；
- 本轮又把 `apps/web/src/format.ts` 继续拆成 `format.ts` + `format/{labels,narrative,technical,posture,value}.ts`，原入口只保留 re-export；
- `apps/web/src/format/labels.ts` 本轮继续拆成 `labels.ts` + `labels/{risk,source,indicator,release}.ts`，父文件已收缩到约 `4` 行；风险/姿态、数据源、指标元数据与 release/review 标签映射已各自落回独立边界；
- `apps/web/src/views/decision/builders.ts` 本轮继续拆成 `builders.ts` + `builderTypes.ts` + `buildersCore.ts` + `buildersBacktests.ts`，父文件已收缩到约 `3` 行；纯展示行模型、runtime/hero/analog/action plan 拼装与 backtest/rolling audit 拼装已各自落回独立边界；
- `crates/scoring/src/lib.rs` 本轮继续拆成 `lib.rs` + `engine.rs` + `signal.rs` + `aggregation.rs` + `narrative.rs` + `tests.rs`，主文件已收缩到约 `16` 行；评分引擎、信号构造、维度聚合、解释文案与测试已各自落回独立边界；
- 标签映射、解释文案、人话化 technical id、posture clause 说明和数值/时间格式化职责已经分离，后续 UI 解释层扩展不再继续堆进单文件。

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
2. 继续细分 SQLite store 剩余大子模块，优先在 `sqlite/seeds/indicator_catalog.rs` 中继续限制跨来源共享逻辑的堆积，并视测试继续增长情况再收窄 `sqlite/tests/historical_replay.rs`、`sqlite/tests/formal_datasets.rs`。

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

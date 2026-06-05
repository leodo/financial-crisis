# 工程治理方案

状态：`Review`

最后更新：2026-06-05

## 1. 目的

本文档不是讨论模型好坏，而是约束代码库如何继续增长而不失控。

当前治理目标只有四个：

1. 实验生成物默认不污染 Git 工作区。
2. `worker`、`api` 的后续拆分边界先讲清楚，再动刀。
3. 训练侧与运行侧重复逻辑先建立收敛清单。
4. 后续新增功能不能继续直接塞进几个超大文件。

本文档与以下文档配套使用：

- [开发质量门禁](development-quality-gates.md)
- [工程维护性 TODO](../roadmap/engineering-maintainability-todo.md)
- [危机概率评估设计 TODO](../roadmap/crisis-probability-design-todo.md)

## 1.1 活跃真相源

从治理角度，当前只允许两份活跃 TODO 持续承载新增任务：

1. 模型/数据/回测主线：`docs/roadmap/crisis-probability-design-todo.md`
2. 工程结构/质量治理主线：`docs/roadmap/engineering-maintainability-todo.md`

`design-todo.md`、`second-round-design-backlog.md`、`sqlite-historical-data-implementation-plan.md`
保留为索引或专项背景，不再单独承载新的活跃主线任务。

## 2. 生成工件分级

### 2.1 A 类：临时实验工件

用途：

- 本地训练候选版
- 临时 release review
- formal dataset summary
- 还没有进入文档、结论或发布流程的研究副产物

规则：

- 默认输出到 `artifacts/research/**`
- 由 `.gitignore` 忽略
- 可以重复覆盖
- 不作为仓库内长期证据

默认目录：

```text
artifacts/research/model-bundles/generated
artifacts/research/model-releases/generated
artifacts/research/release-review
artifacts/research/formal-dataset
```

### 2.2 B 类：可追踪研究证据

用途：

- 已被文档、TODO、结论直接引用的 release review
- 已决定保留的 formal dataset summary
- 需要在仓库内长期可追溯的对照证据

规则：

- 显式输出到版本化目录
- 进入 Git 前，必须能回答“为什么这份证据值得长期保留”
- 不允许把整轮实验的所有副产物都直接推进来

目录：

```text
reports/release-review
reports/formal-dataset
```

### 2.3 C 类：长期基线工件

用途：

- 被明确保留的 bundle / manifest
- 作为基线对照、回归复核、历史研究样本的正式候选工件

规则：

- 只有被明确 promotion 的候选版才进入版本化目录
- SQLite release registry 才是运行态 source of truth
- 仓库里的 generated manifest/bundle 只承担“可追溯证据”和“对照基线”角色

目录：

```text
config/model-bundles/generated
config/model-releases/generated
```

## 3. CLI 与 just 约束

从本轮治理开始：

- `research release review` 默认输出到 `artifacts/research/release-review`
- `research dataset summarize-main` 默认输出到 `artifacts/research/formal-dataset`
- `research pipeline train-probability` 默认输出到 `artifacts/research/model-*/generated`
- 只有显式传 `--output-dir` / `--manifest-dir`，才把结果送入版本化目录
- 提交前运行 `just artifact-status`；如果版本化 artifact 已被暂存，命令会要求先写清它属于正式 release、基线证据还是临时研究副产物。
- 版本化 artifact 目录默认忽略新生成文件；需要长期保留的证据必须在文档/TODO 中说明用途后，用 `git add -f` 显式纳入。

`just` 层同步提供两类入口：

- 默认命令：面向本地实验，不污染 Git
- `*-tracked` 命令：面向长期证据保留

同时，本地提交前统一质量门禁收口到：

```text
just verify
```

它应始终覆盖：

- artifact 审计；
- Rust 格式检查；
- Rust 测试；
- clippy；
- Web 构建。

这套约束的细则由 [开发质量门禁](development-quality-gates.md) 统一维护。

## 4. Worker 拆分边界

`apps/worker/src/main.rs` 后续拆分目标如下：

```text
apps/worker/src/
  main.rs                  # 顶层入口 + 少量共享导出
  actionability.rs
  formal.rs
  model.rs
  output_paths.rs
  probability.rs
  probability/
    overlay.rs
    threshold.rs
  release_review.rs
  reporting.rs
  scenario.rs
  support.rs
  tests.rs                 # 第一层测试聚合壳层
  tests/
    fixtures.rs
    options.rs
    training.rs
    quality.rs
    review.rs
    split_requirements.rs
  training.rs
  commands/
    audit.rs
    backfill.rs
    dataset.rs
    dataset/
      report.rs
    db.rs
    feature.rs
    pipeline.rs
    refresh.rs
    release.rs
    release/
      probability.rs
      probability/
        compare.rs
      review.rs
      review/
        focus.rs
    research.rs
    snapshot.rs
```

边界要求：

- `main.rs` 继续朝“只保留顶层分发”收缩，但在完全拆干净之前，可以暂时承载少量共享导出与底层 helper；不要为了形式上只留一个入口，先制造空目录或额外跳转层。
- `reporting.rs` 负责结构化报告渲染与导出；当前主文件应保持在“写盘入口 + 模块导出”边界，release review Markdown 细节下沉到 `reporting/release_review.rs`，rolling audit Markdown 细节下沉到 `reporting/audit.rs`。
- `reporting/release_review.rs` 负责 release review Markdown 总装配；`reporting/release_review/overview.rs` 负责 release rows、runtime snapshot、runtime separation、backtest/failure summary，`historical.rs` 负责 historical audit 各表段，`focus.rs` 负责 focus scenarios 明细，`diagnostics.rs` 负责 runtime/actionability diagnostics、guardrail 与 recommendation。
- `model.rs` 负责训练数学、样本标签/权重、校准拟合与 runtime scoring 的主链路；前向危机符号/边界约束应下沉到 `model/constraints.rs`，regime pairwise 目标与梯度应下沉到 `model/regime.rs`，样本标签/权重策略应下沉到 `model/weighting.rs`，`Platt` 校准、runtime scoring 与概率评估 helper 应下沉到 `model/calibration.rs`，主文件尽量回到“拟合主循环 + 少量共享数学 helper”边界。
- `scenario.rs` 负责 `CrisisScenario`、action episode window、protected context、primary/forward scenario 选择和 action window label 这组场景时间窗逻辑。
- `support.rs` 负责 `ApiReloadHistoryMode`、demo run、API fetch/reload、SQLite/raw payload IO、格式化 helper 和通用 rounding/hash/path helper。
- `tests.rs` 作为第一层测试聚合壳层；共享测试构造器下沉到 `tests/fixtures.rs`，主题测试以真实子模块形式落在 `tests/*.rs`，避免继续依赖 `include!` 共享词法作用域。
- `actionability.rs`、`probability.rs`、`model.rs`、`training.rs`、`formal.rs` 负责训练、特征、数据集构建与共享数学/标签逻辑；其中 `training.rs` 还承接 formal bundle 训练管线和 `forward_crisis` 标签 / regime helper。
- `probability.rs` 负责概率主头训练、regime evaluation 与 bundle summary；`probability/threshold.rs` 负责 threshold 子模块导出与测试壳层，`probability/threshold/calibration.rs` 负责 calibration sample selection / calibration strategy，`probability/threshold/decision.rs` 负责 threshold selection / regime-support threshold repair，`probability/threshold/diagnostics.rs` 负责 threshold diagnostics / calibration evidence；`probability/overlay.rs` 负责 family overlay 训练编排，`probability/overlay/audit.rs` 负责 overlay audit spec / audit 汇总，`probability/overlay/split/mod.rs` 负责 split 壳层与共享 support 结构，`probability/overlay/split/dataset.rs` 负责样本筛选与背景样本去重，`probability/overlay/split/balanced.rs` 负责 balanced fallback，`probability/overlay/split/bounds.rs` 负责 family-aware boundary search。
- `commands/dataset.rs` 负责 formal dataset 子模块导出；`commands/dataset/options.rs` 负责 build/list/summarize/slice 的 CLI 选项解析，`commands/dataset/build.rs` 负责 formal dataset 主样本装配与最小覆盖/可用性门槛，`commands/dataset/execute.rs` 负责 `build/list/summarize/slice` 研究命令编排；`commands/dataset/report.rs` 负责 formal dataset report 壳层与共享数据结构导出，`commands/dataset/report/summary.rs` 负责 split/scenario/family/quality/regime 汇总与 recommendation，`commands/dataset/report/slice.rs` 负责 scenario slice 过滤、feature 列收集、JSON/CSV 导出与文件名 helper，`commands/dataset/report/render.rs` 负责 Markdown/CSV 渲染与 CLI 摘要打印；`commands/dataset/split.rs` 负责 split profile、scenario-aware split bounds、label support 与 scenario range helper；`commands/dataset/scenarios.rs` 负责 scenario catalog 装配与 metadata 编码 helper。
- `commands/backfill.rs` 负责 backfill 子模块导出；`commands/backfill/options.rs` 负责免费数据回填的通用时间窗/过滤/水位回退选项，以及 FRED 模式切换与 BOJ dataset 解析，`commands/backfill/execute.rs` 负责 FRED / Treasury / World Bank / GDELT / SEC / BOJ / JPY carry 的回填编排与共享 chunked mapping 执行器。
- `commands/feature.rs` 负责 feature 子模块导出；`commands/feature/options.rs` 负责 feature snapshot build/list 的 CLI 选项解析，`commands/feature/snapshot.rs` 负责 feature snapshot build/list 主流程、snapshot 重用/重建与 formal feature row 装配，`commands/feature/visibility.rs` 负责 PIT 可见性、publication timing 与时区截止规则，`commands/feature/coverage.rs` 负责 coverage / core-feature gate / quality grade helper。
- `commands/pipeline.rs` 负责 pipeline 子模块导出；`commands/pipeline/options.rs` 负责 `--dataset-source` / `--model-shape` / bundle 输出目录 / release prefix 解析，`commands/pipeline/dataset.rs` 负责 formal/snapshot 训练数据装配、dataset key 解析、training row 映射与 transitional feature helper，`commands/pipeline/execute.rs` 负责 `train-probability` / `bootstrap-formal-release` 命令执行与控制台摘要打印。
- `commands/release.rs` 负责 release 子模块导出；`commands/release/options.rs` 负责 publish/list/show/activate/rollback 的 CLI 选项解析，`commands/release/lifecycle.rs` 负责 publish/list/show/activate/rollback 生命周期命令与 active release 切换，`commands/release/guardrails.rs` 负责 actionability/probability/runtime/operational guardrail helper 与 actionability review 装配。
- `commands/release/review.rs` 负责 release review orchestration 与子模块导出；`commands/release/review/options.rs` 负责 CLI 选项解析，`commands/release/review/snapshot.rs` 负责 runtime snapshot 抓取与 activate/restore helper，`commands/release/review/comparison.rs` 负责 release compare / runtime separation compare，`commands/release/review/summary.rs` 负责 recommendation 与 CLI summary 打印；`commands/release/review/focus.rs` 负责 focus 子模块导出，`commands/release/review/focus/backtest.rs` 负责 backtest scenario compare 辅助边界，`commands/release/review/focus/runtime.rs` 负责 runtime focus 子模块导出，`commands/release/review/focus/runtime/signals.rs` 负责 structured signal / actionable signal helper，`commands/release/review/focus/runtime/blocks.rs` 负责 runtime actionable block/facet 统计与 primary failure mode 判定，`commands/release/review/focus/runtime/scenario.rs` 负责 scenario focus diagnostics 主装配。
- `release_review.rs` 负责 release review 报告结构、共享 formatter 与 Markdown 壳层；`release_review/historical.rs` 负责 failure mode、historical audit priority / attribution / action / workstream 汇总与 takeaways；`release_review/runtime.rs` 负责 runtime regime probability / separation diagnostics、分类 helper 与 runtime takeaways。
- `commands/release/probability.rs` 负责 release probability 子模块导出；`commands/release/probability/options.rs` 负责 probability slice / formal slice / formal compare 的 CLI 选项解析，`commands/release/probability/execute.rs` 负责 release lookup、runtime activate/restore、historical replay 读取与三类 research orchestration；
- `commands/release/probability/slice.rs` 负责 runtime probability slice 的 JSON/CSV 导出、overlay 展开与 CLI 摘要打印；
- `commands/release/probability/formal.rs` 负责 formal dataset slice 的 bundle 打分、base model diagnostics、CSV/JSON 导出与 CLI 摘要打印；
- `commands/release/probability/compare.rs` 负责 formal probability compare 的结构定义与子模块导出；`commands/release/probability/compare/build.rs` 负责窗口对齐、阈值/命中统计、feature delta 聚合与窗口汇总，`commands/release/probability/compare/render.rs` 负责 JSON/CSV 导出与 CLI 摘要打印；
- `commands/release/probability/common.rs` 负责文件名清洗与 CSV 转义等共享小工具。
- `assessment.rs` 负责 assessment 总装配、method version 与 snapshot envelope；`assessment/posture.rs` 负责 posture 子模块导出，`assessment/posture/guidance.rs` 负责风险时距、posture clause、用户偏好升降级与姿态摘要，`assessment/posture/position.rs` 负责仓位预算、动作手册、执行护栏和治理条款；`assessment/runtime_policy.rs` 负责 runtime threshold / serving policy / history runtime policy version；`assessment/common.rs` 负责 rounding、formatting、pressure 这类跨 assessment 子模块共享 helper；`assessment/tests/mod.rs` 负责 assessment 测试聚合，`assessment/tests/actionability.rs`、`time_bucket.rs`、`posture.rs`、`position.rs` 分别承接动作置信度、风险时距、posture 条款和 position guidance 测试，避免测试层重新回到单文件堆叠。
- `assessment/probability.rs` 只负责概率子模块导出与共享 trace 结构；`assessment/probability/heuristic.rs` 负责启发式概率与动作层基线，`features.rs` 负责 formal/runtime 概率特征映射与 freshness helper，`actionability.rs` 负责动作置信度校准与上下文融合，`trace.rs` 负责 bundle scoring、monotonic repair、overlay diagnostics 与 actionability trace 装配；避免评分融合和特征映射继续堆在单文件里。
- `demo_seed.rs` 只负责 demo seed 子模块导出；`demo_seed/indicators.rs` 负责静态指标定义，`observations.rs` 负责演示观测序列与共享 series helper，`sources.rs` 负责 demo/runtime 数据源健康样本，`alerts.rs` 负责 demo alert 构造与最近告警筛选；`demo.rs` 只消费这些 seed API，不再内联样本细节。
- `history_replay.rs` 只负责 historical replay 子模块导出与共享记录结构；`history_replay/cache.rs` 负责 replay run 持久化、source watermark 校验、history cache key / method version 与 release-aware cache refresh 判定，`history_replay/transform.rs` 负责 history point / prediction snapshot / replay draft 转换与 merge helper；避免历史缓存、水位校验与纯转换继续堆在单文件里。
- `backtest.rs` 只负责 backtest 子模块导出；`backtest/actionability.rs` 负责动作级历史判定、过渡 bridge 与姿态对应 horizon helper，`backtest/scenarios.rs` 负责场景目录映射、fallback 模板与真实历史场景汇总，`backtest/rolling_audit.rs` 负责滚动审计 episode 分类、受保护压力窗口说明与 summary 生成，`backtest/timeline.rs` 负责 timeline 点位构造；避免场景回退、滚动审计和动作级历史规则继续堆在单文件里。
- `apps/web/src/format.ts` 只负责前端格式化子模块导出；`format/labels.ts` 负责风险等级、source、dataset、review status 等人话标签，`format/narrative.ts` 负责人话解释文案与 license/method/audit note 翻译，`format/technical.ts` 负责 technical id / release id / file reference 压缩显示，`format/posture.ts` 负责 posture clause 中文说明，`format/value.ts` 负责数值、百分比、日期与时间轴标签格式化；页面层只消费格式化 API，不再内联这些映射细节。
- `apps/web/src/types.ts` 只负责前端类型子模块导出；`types/common.ts` 负责基础枚举、通用质量摘要与概率/动作块，`risk.ts` 负责风险快照、指标、数据源、告警和 backtest scenario summary 类型，`backtest.ts` 负责 backtest window / rolling audit 类型，`assessment.ts` 负责 assessment snapshot、overlay diagnostics、posture/runtime/data trust 类型，`research.ts` 负责 release/replay/research audit 类型；前端页面与 view model 继续只从 `types.ts` 取类型，不再把全部响应结构堆在单文件。
- `apps/web/src/styles.css` 只负责前端样式入口；`styles/base.css` 负责全局 reset、sidebar/topbar、runtime strip 与基础布局，`styles/surfaces.css` 负责 surface/hero/probability/list/pill 等通用面板组件，`styles/analysis.css` 负责 audit/posture/budget/chart/table 这类分析视图样式，`styles/responsive.css` 负责各断点响应式规则；`main.tsx` 继续只引入 `styles.css`，避免页面层分散样式入口。
- `apps/worker/src/tests/training/mod.rs` 负责训练测试聚合；`training/visibility.rs`、`scenario_regimes.rs`、`sign_constraints.rs`、`family_constraints.rs` 分别承接 PIT 可见性、危机标签/训练 regime、符号约束和 family cap 约束测试；`training/weighting/mod.rs` 负责样本权重/训练目标测试聚合，`training/weighting/negative_weights.rs`、`pairwise.rs`、`target_labels.rs`、`positive_weights.rs` 分别承接负样本权重、regime pairwise 目标、训练 target label 与正样本权重测试，避免训练测试再次回到单文件堆叠。
- `apps/worker/src/tests/options/mod.rs` 负责 CLI / dataset / release 选项测试聚合；`options/refresh.rs` 承接 refresh / audit export 参数测试，`release.rs` 承接 release publish/switch/review/probability slice/compare 参数测试，`snapshots.rs` 承接 prediction/feature snapshot 查询参数测试，`dataset.rs` 承接 formal dataset build/summary/slice 与 pre-1990 coverage gate 测试，`pipeline.rs` 承接 `train-probability` dataset source / model shape / aux dataset key 解析测试，避免 option parsing 再次堆回单文件。
- `apps/worker/src/tests/review/mod.rs` 负责 release review 测试聚合；`review/focus/mod.rs` 负责 scenario focus 测试聚合，`review/focus/comparison.rs`、`diagnostics.rs`、`continuity.rs`、`failure_modes.rs` 分别承接 backtest compare、focus diagnostics、posture continuity 与 failure summary 场景测试；`historical_audit.rs`、`runtime.rs` 继续承接 historical audit、runtime separation 相关测试，避免 release review 测试再次回到单文件堆叠。
- `apps/worker/src/tests/quality/mod.rs` 负责质量门禁测试聚合；`quality/render.rs`、`actionability.rs`、`probability_thresholds.rs`、`regime_guardrails.rs` 分别承接 CSV 渲染、actionability 质量门禁、概率校准/阈值与 regime/guardrail 测试，避免质量测试再次回到单文件堆叠。
- `commands/*` 其余模块只负责 CLI 层 glue code。

## 5. API 拆分边界

`apps/api` 后续拆分目标如下：

```text
apps/api/src/
  assessment/
    features.rs
    probability.rs
    posture.rs
    guidance.rs
    analogs.rs
  data_source/
    demo_seed.rs
    sqlite.rs
    postgres.rs
  history/
    replay.rs
    snapshot_bridge.rs
    cache.rs
  backtest/
    timeline.rs
    rolling_audit.rs
```

边界要求：

- `demo_seed` 与真实数据加载分离。
- history replay、snapshot bridge、cache key 分离。
- assessment 的特征构造、概率评分、posture、仓位建议分离。

## 6. 重复逻辑收敛清单

当前已经明确需要收敛的函数/职责：

### 6.1 概率数学

- `apply_platt_calibration`
- `score_logistic_model`

目标：

- 训练侧和运行侧共用同一实现
- 不允许继续一边修、一边忘

当前落地：

- `apply_platt_calibration` 已以 `apply_platt_probability_calibration` 形式进入 `crates/domain/src/probability_bundle.rs`。
- bundle runtime scoring 已以 `score_logistic_probability_model` 形式进入 `crates/domain/src/probability_bundle.rs`。
- `Platt` 拟合、样本权重、sign/regime 约束仍留在 `apps/worker/src/model.rs`，因为它们依赖训练样本、切分策略和候选实验，不属于 API 运行时必需能力。

### 6.2 观测窗口与派生特征

- `observations_for_indicator`
- `difference_from_tail`
- `insert_derived_feature`
- 一部分 `latest feature` / `derived feature` 派生逻辑

目标：

- 同一特征名在 worker 训练与 API 运行中使用同一派生规则

当前落地：

- 通用观测窗口、as-of 过滤、排序、尾部 lookback 差值已进入 `crates/domain/src/observation_window.rs`。
- worker 的 PIT 可见性包装仍保留在 `apps/worker/src/commands/feature.rs`，因为它绑定 `PointInTimeMode`、publication timing 和时区截止规则。
- 下一步若要下沉更多正式特征派生，必须先定义“feature id -> source indicator -> transform”的注册表，而不是继续在 API/worker 两边手写映射。

### 6.3 Runtime / training 解释口径

- threshold diagnostics
- horizon probability 解释字段
- posture 相关 runtime metadata

目标：

- 前台解释、release review、训练导出三者不能各说各话

当前边界：

- `threshold selection`、calibration selection、regime separation 诊断当前仍属于 worker 训练/release review 侧。
- API 运行时只消费已发布 bundle 中的阈值、metadata 和 serving policy，不应在请求路径重新训练或重新选择阈值。
- 如果未来 dashboard 要展示与 release review 完全一致的 threshold diagnostic 明细，应先把纯诊断结构和渲染无关计算抽到 shared crate，再让 worker/API 共用。

### 6.4 共享边界判定矩阵

| 归属 | 可以放什么 | 不能放什么 | 当前代表文件 |
| --- | --- | --- | --- |
| `crates/domain` | 纯领域模型、bundle schema、纯概率打分、Platt 应用、特征 transform resolver、观测窗口排序/差值、静态场景目录 | IO、环境变量、HTTP、数据库、缓存、当前时间、用户 profile、source-specific PIT 发布规则 | `probability_bundle.rs`、`observation_window.rs`、`stress_window.rs` |
| `apps/worker` | 数据刷新/回填命令、训练样本构建、PIT feature snapshot、模型拟合、阈值选择、release review、候选实验 guardrail | API response shape、前端展示文案、请求级用户偏好、运行时重新训练 | `commands/feature.rs`、`model.rs`、`probability.rs`、`commands/release.rs` |
| `apps/api` | 当前评估装配、active release 加载、runtime cache、用户偏好升降级、posture/position guidance、历史回放与回测 API DTO | 训练样本切分、候选模型搜索、离线实验输出、UI 文案硬编码 | `assessment/*.rs`、`data_source.rs`、`history_replay.rs`、`backtest.rs` |
| `apps/web` | 人话标签、格式化、页面 view model、图表和交互状态 | 概率计算、阈值选择、仓位规则事实来源、数据抓取 | `format.ts`、`format/**`、`views/**` |

判定规则：

1. 同一函数如果同时被训练侧和运行侧需要，且不依赖 IO/缓存/当前时间/用户请求，优先进入 `crates/domain`。
2. 依赖训练切分、候选发布、guardrail review 的逻辑先留在 `apps/worker`；只有 API 需要同一语义时才抽共享层。
3. 依赖 active release、runtime cache、用户 profile、HTTP response 的逻辑留在 `apps/api`。
4. PIT 可见性分两层处理：通用观测窗口可以进 domain，source publication timing 与 cutoff timezone 暂留 worker/data 规范，避免把数据源时效假设伪装成纯领域逻辑。
5. Web 只能翻译和呈现，不承接新的风险判断事实来源；任何新增仓位动作必须先进入 API playbook/Go-No-Go 边界。

## 7. 实施原则

从现在开始，以下规则生效：

1. 模型主线优先，但**新增非小修功能不再允许直接塞进**：
   - `apps/worker/src/main.rs`
   - `apps/api/src/demo.rs`
   - `apps/api/src/assessment.rs`
   - `apps/web/src/App.tsx`
   - `crates/storage/src/sqlite.rs`
2. 如果改动会同时影响训练口径和运行口径，先检查是否应抽共享函数。
3. 如果生成物只是本地实验副产物，默认留在 `artifacts/research/**`。
4. 进入版本化目录的工件，必须能被某个文档、TODO、release 结论直接引用。

## 8. 本轮治理完成定义

本轮 P0 治理完成，指的是以下事项已经落地：

- [x] 工件分级规则已定义
- [x] 默认实验输出目录已切到 ignored artifacts
- [x] 提交前 artifact 审计入口已落到 `just artifact-status`
- [x] worker 模块拆分边界已定义
- [x] api 模块拆分边界已定义
- [x] API / worker 重复逻辑收敛清单已建立
- [x] “不再继续往超大文件堆功能”的实施原则已落文档

之后再进入下一阶段：

- 先拆 `worker`
- 再收敛共享概率/特征逻辑
- 再拆 `api history/demo/runtime`

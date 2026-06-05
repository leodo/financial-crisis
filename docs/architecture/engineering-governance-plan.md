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
- `reporting.rs` 负责结构化报告渲染与导出，`release_review.rs` 负责 release review 专属报告结构、诊断与解释 helper。
- `scenario.rs` 负责 `CrisisScenario`、action episode window、protected context、primary/forward scenario 选择和 action window label 这组场景时间窗逻辑。
- `support.rs` 负责 `ApiReloadHistoryMode`、demo run、API fetch/reload、SQLite/raw payload IO、格式化 helper 和通用 rounding/hash/path helper。
- `tests.rs` 作为第一层测试聚合壳层；共享测试构造器下沉到 `tests/fixtures.rs`，主题测试以真实子模块形式落在 `tests/*.rs`，避免继续依赖 `include!` 共享词法作用域。
- `actionability.rs`、`probability.rs`、`model.rs`、`training.rs`、`formal.rs` 负责训练、特征、数据集构建与共享数学/标签逻辑；其中 `training.rs` 还承接 formal bundle 训练管线和 `forward_crisis` 标签 / regime helper。
- `probability.rs` 负责概率主头训练、regime evaluation 与 bundle summary；`probability/threshold.rs` 负责 calibration sample selection、threshold selection、regime-support threshold repair 与 threshold diagnostics / evidence；`probability/overlay.rs` 负责 family overlay 的审计、样本筛选、split 策略与 overlay 子模型训练。
- `commands/dataset.rs` 负责 formal dataset build、split/scenario 约束与研究命令编排；`commands/dataset/report.rs` 负责 formal dataset summary、slice export、Markdown/CSV/JSON 渲染与 CLI 摘要打印。
- `commands/release.rs` 负责 release 生命周期总入口、共享 activate/runtime guard 与 market scope resolve。
- `commands/release/review.rs` 负责 release review 的 CLI 选项解析、runtime snapshot、runtime separation compare、建议与总结；`commands/release/review/focus.rs` 负责 structured signal counts、backtest scenario compare、scenario focus diagnostics、runtime actionable block/facet 统计与 primary failure mode 判定。
- `release_review.rs` 负责 release review 报告结构、共享 formatter 与 Markdown 壳层；`release_review/historical.rs` 负责 failure mode、historical audit priority / attribution / action / workstream 汇总与 takeaways；`release_review/runtime.rs` 负责 runtime regime probability / separation diagnostics、分类 helper 与 runtime takeaways。
- `commands/release/probability.rs` 负责 probability slice / formal slice / formal compare 的 CLI 解析、release lookup 与 orchestration；
- `commands/release/probability/slice.rs` 负责 runtime probability slice 的 JSON/CSV 导出、overlay 展开与 CLI 摘要打印；
- `commands/release/probability/formal.rs` 负责 formal dataset slice 的 bundle 打分、base model diagnostics、CSV/JSON 导出与 CLI 摘要打印；
- `commands/release/probability/compare.rs` 负责 formal probability compare 的阈值摘要、feature delta 聚合、窗口汇总、CSV/JSON 导出与 CLI 摘要打印；
- `commands/release/probability/common.rs` 负责文件名清洗与 CSV 转义等共享小工具。
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
| `apps/api` | 当前评估装配、active release 加载、runtime cache、用户偏好升降级、posture/position guidance、API DTO | 训练样本切分、候选模型搜索、离线实验输出、UI 文案硬编码 | `assessment/*.rs`、`data_source.rs`、`history_replay.rs` |
| `apps/web` | 人话标签、格式化、页面 view model、图表和交互状态 | 概率计算、阈值选择、仓位规则事实来源、数据抓取 | `format.ts`、`views/**` |

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

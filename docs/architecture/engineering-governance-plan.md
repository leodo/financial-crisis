# 工程治理方案

状态：`Review`

最后更新：2026-06-02

## 1. 目的

本文档不是讨论模型好坏，而是约束代码库如何继续增长而不失控。

当前治理目标只有四个：

1. 实验生成物默认不污染 Git 工作区。
2. `worker`、`api` 的后续拆分边界先讲清楚，再动刀。
3. 训练侧与运行侧重复逻辑先建立收敛清单。
4. 后续新增功能不能继续直接塞进几个超大文件。

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

`just` 层同步提供两类入口：

- 默认命令：面向本地实验，不污染 Git
- `*-tracked` 命令：面向长期证据保留

## 4. Worker 拆分边界

`apps/worker/src/main.rs` 后续拆分目标如下：

```text
apps/worker/src/
  main.rs                  # 只保留顶层分发
  commands/
    db.rs
    backfill.rs
    audit.rs
    release.rs
    research.rs
  pipeline/
    feature_build.rs
    formal_dataset.rs
    probability_train.rs
    actionability_train.rs
  reporting/
    release_review.rs
    formal_dataset_summary.rs
    audit_export.rs
  output_paths.rs
```

边界要求：

- `main.rs` 只做参数分发，不再直接承载完整流程。
- `reporting/*` 只负责结构化报告渲染与导出。
- `pipeline/*` 只负责训练、特征、数据集构建。
- `commands/*` 只负责 CLI 层 glue code。

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

### 6.2 观测窗口与派生特征

- `observations_for_indicator`
- `difference_from_tail`
- `insert_derived_feature`
- 一部分 `latest feature` / `derived feature` 派生逻辑

目标：

- 同一特征名在 worker 训练与 API 运行中使用同一派生规则

### 6.3 Runtime / training 解释口径

- threshold diagnostics
- horizon probability 解释字段
- posture 相关 runtime metadata

目标：

- 前台解释、release review、训练导出三者不能各说各话

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
- [x] worker 模块拆分边界已定义
- [x] api 模块拆分边界已定义
- [x] API / worker 重复逻辑收敛清单已建立
- [x] “不再继续往超大文件堆功能”的实施原则已落文档

之后再进入下一阶段：

- 先拆 `worker`
- 再收敛共享概率/特征逻辑
- 再拆 `api history/demo/runtime`

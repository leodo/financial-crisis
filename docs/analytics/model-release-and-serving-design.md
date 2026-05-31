# 模型发布与在线评分设计

状态：`Draft`

最后更新：2026-05-31

## 1. 目标

定义从 `label -> feature -> model -> calibration -> release -> serving` 的完整工程链路，把当前启发式概率层升级为可版本化、可回滚、可在线加载的正式危机概率系统。

本设计回答三个工程问题：

1. 模型和校准结果如何固化为可发布 artifact。
2. API / worker 如何加载当前有效版本并在线评分。
3. 当 point-in-time 数据不足或模型缺失时，系统如何进入降级模式。

## 2. 非目标

第一阶段不追求：

- 自动化超参数搜索平台
- 多市场多租户模型编排
- 深度学习或复杂集成模型
- 完整 MLOps SaaS 化

## 3. 设计原则

- 在线评分必须由 Rust 服务直接执行，不能依赖外部 notebook。
- 只有被激活的 release 才能作为“正式概率”展示给用户。
- 模型版本、校准版本、特征版本和标签版本必须显式绑定。
- point-in-time 能力必须作为 release 元数据的一部分，而不是隐式假设。
- 无法满足发布门槛时，系统应明确退回 `heuristic_mvp`，而不是伪装成正式概率模型。

## 4. 总体链路

```text
raw observations
  -> point-in-time feature snapshots
  -> horizon labels
  -> train raw models
  -> calibrate probabilities
  -> evaluate against backtests and rolling history
  -> publish candidate release
  -> activate release
  -> online scoring
```

## 5. Release Bundle

每次可上线的模型发布物应是一个完整 bundle，而不是零散文件。

```text
release_bundle/
  release_manifest.json
  feature_manifest.json
  label_manifest.json
  posture_policy.json
  p5d_model.json
  p20d_model.json
  p60d_model.json
  p5d_calibration.json
  p20d_calibration.json
  p60d_calibration.json
  evaluation_summary.json
```

### 5.1 `release_manifest.json`

至少包含：

```text
release_id
market_scope
feature_set_version
label_version
prob_model_version
calibration_version
posture_policy_version
point_in_time_mode
training_range
calibration_range
evaluation_range
created_at
created_by
git_revision
status
```

### 5.2 模型 artifact 格式

第一阶段建议优先使用简单、可解释、Rust 易加载的格式：

- 逻辑回归 / Probit 系数
- 特征名与标准化参数
- 缺失值处理规则
- 截距与单调性修正配置

建议 JSON 或 MessagePack 序列化，避免先把在线评分绑定到 Python 运行时。

### 5.3 校准 artifact 格式

至少支持：

- `platt`
- `isotonic`

每个 horizon 独立保存：

```text
calibration_method
input_score_field
parameters
valid_score_range
calibration_version
```

## 6. Release 状态机

```text
draft -> candidate -> approved -> active -> retired
                     \-> rejected
active -> rolled_back
```

状态含义：

- `draft`：训练或校准刚完成，尚未评估
- `candidate`：已完成评估，等待人工审阅
- `approved`：通过门槛，可随时激活
- `active`：当前线上正式版本
- `retired`：历史保留，不再用于线上
- `rolled_back`：曾被激活，但已被回滚

## 7. 离线训练与发布流程

### 7.1 标签阶段

输入：

- `label_version`
- 场景表
- point-in-time 日期范围

输出：

- `research_horizon_labels`
- `research_label_generation_runs`

### 7.2 特征阶段

输入：

- `feature_set_version`
- observation snapshot
- point-in-time mode

输出：

- 特征快照宽表
- 特征清单和统计摘要

### 7.3 训练阶段

每个 horizon 独立产出：

- 原始模型
- 训练段指标
- 特征重要性或系数摘要

### 7.4 校准阶段

每个 horizon 独立产出：

- calibrated probability
- calibration metrics
- calibration artifact

### 7.5 评估阶段

必须同时看：

- 概率质量
- 场景提前量
- posture 误报
- 保护性压力窗口表现

### 7.6 发布阶段

只有当评估达标时，才允许生成 `candidate release`。

## 8. 发布门槛

候选 release 至少需要满足：

1. `Brier score`、`log loss`、`ECE` 不显著差于当前 active release
2. `5d / 20d / 60d` 三个 horizon 没有明显失真
3. 回测中 `hedge / defend` 的首次触发提前量不恶化
4. 纯误报区间长度与频次可接受
5. point-in-time 覆盖率达到声明门槛

第一阶段建议的默认门槛：

- `ECE <= 0.05`
- 关键场景的 `first_hedge_date` 不晚于当前 active release
- 最长纯误报区间不得显著恶化

这里的门槛不是永久固定值，但必须显式记录在评估摘要中。

## 9. 在线评分链路

```text
load active release
  -> build current feature snapshot
  -> check freshness / coverage / point-in-time mode
  -> run p5d/p20d/p60d raw model
  -> apply calibration
  -> apply monotonicity guard
  -> derive time bucket / posture / position guidance
  -> persist assessment snapshot
```

### 9.1 active release 加载

API 服务启动时：

- 读取当前 `market_scope` 的 active release 指针
- 加载 bundle 到内存
- 记录 `release_id` 和各 version

刷新时：

- worker 完成新 release 激活后，调用 reload
- API 原子替换当前 bundle

### 9.2 数据门禁

在线评分前必须检查：

- 关键指标是否新鲜
- 特征覆盖率是否达标
- 是否满足 release 声明的 `point_in_time_mode`

若失败：

- 正式概率标记为不可用，或
- 退回 `heuristic_mvp`

但退回时必须在 API 明确标记：

```text
probability_mode = heuristic_mvp
release_status = degraded
```

### 9.3 单调性保护

由于三个 horizon 独立建模，校准后可能出现：

```text
p_5d > p_20d
or
p_20d > p_60d
```

在线层必须做轻量修正，确保：

```text
p_5d <= p_20d <= p_60d
```

修正规则需要写入 method note，避免前后端误以为模型天然满足该性质。

## 10. 降级模式

以下情况进入降级：

- 没有 active release
- active release artifact 损坏
- point-in-time 或覆盖率不满足最低门槛
- 新鲜度严重失效

降级策略：

1. 概率层退回启发式
2. `method` 明确返回 `heuristic_mvp`
3. 在数据质量差时禁止仅凭启发式概率直接升级到最高 posture
4. UI 必须突出显示“正式概率不可用”

## 11. 落库对象

```text
research_training_runs
research_model_artifacts
research_calibration_runs
analytics_model_releases
analytics_active_model_pointers
analytics_prediction_snapshots
```

建议字段：

### 11.1 `analytics_model_releases`

```text
release_id
market_scope
feature_set_version
label_version
prob_model_version
calibration_version
posture_policy_version
point_in_time_mode
status
bundle_uri
evaluation_summary_uri
created_at
activated_at nullable
retired_at nullable
```

### 11.2 `analytics_prediction_snapshots`

```text
as_of_date
market_scope
release_id
probability_mode
raw_p_5d
raw_p_20d
raw_p_60d
calibrated_p_5d
calibrated_p_20d
calibrated_p_60d
feature_set_version
label_version
point_in_time_mode
coverage_score
freshness_status
```

## 12. Worker 命令建议

命令名可后续细化，但流程应支持：

```text
fc-worker research build-labels
fc-worker research build-features
fc-worker research train-probability
fc-worker research calibrate-probability
fc-worker research evaluate-release
fc-worker research publish-release
fc-worker research activate-release
fc-worker research rollback-release
```

第一阶段可以先把训练、校准和发布做成串行命令，不需要上来就做任务编排平台。

## 13. 与现有文档的关系

- 概率模型结构：见 `probability-engine-design.md`
- 校准方法：见 `probability-calibration-design.md`
- 特征快照：见 `feature-store-design.md`
- 标签生成：见 `horizon-label-design.md`
- 历史评估：见 `real-backtest-execution-design.md`

本文件补的是“如何把这些设计发布成线上可加载版本”这一层。

## 14. 实现顺序

1. 先固化 bundle 和 manifest 格式
2. 再固化 `analytics_model_releases` 和 active pointer
3. 实现 Rust 侧 bundle loader
4. 接入在线评分与 snapshot 落库
5. 最后实现 activate / rollback 和 UI 方法页展示

## 15. 风险

- FRED CSV 默认没有 vintage，`strict point-in-time` 覆盖会受限
- 事件发布时间和市场交易日对齐存在噪声
- 若过早引入复杂模型，Rust 在线服务和 artifact 兼容性会迅速变差
- 没有清晰 release 门槛时，版本号会看起来很完整，但本质仍是实验模型

## 16. 2026-05-31 已落地的实现补充

当前仓库已经把第一条可运行的 formal bundle 工程链路接通：

- `prediction snapshots -> 特征/标签数据集`
- `chronological split -> logistic raw model -> Platt calibration`
- `bundle JSON + evaluation JSON + release manifest JSON`
- `SQLite release publish/activate -> API online load`
- `bundle load failure -> heuristic_mvp + degraded fallback`
- `/api/research/audit` 与前端“发布审计”页

对应命令：

```text
just snapshot-export
just snapshot-dataset
just formal-train
just formal-bootstrap
```

这条链路的定位是“过渡版 formal serving”：

- 训练特征来自已落库 `prediction snapshots`
- 标签来自美国危机起点目录
- 目标是先打通可发布、可回滚、可解释的线上 bundle 机制

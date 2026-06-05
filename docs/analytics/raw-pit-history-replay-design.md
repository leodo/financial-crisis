# Raw PIT History Replay 设计

状态：`Draft`

最后更新：2026-06-01

已落地进展（2026-06-01）：

- API `/api/system/reload` 已支持 `history_mode=strict_rebuild`；
- release review 已默认通过 `strict_rebuild` 触发 raw-history rebuild，再抓 baseline / candidate runtime snapshot；
- `historical replay run / point` 已落到 SQLite / domain store，strict/full rebuild 会把历史点级结果写入 replay store；
- API 在命中同 `history_cache_key + date range + release_id` 的成功 replay run 时，已经会优先读取 replay points，而不是先退回旧 `prediction snapshots`；
- API 默认历史路径对 `bundle-backed release` 也已改为 `replay-first`：若无可复用 replay cache，会直接基于原始观测全量重建并写回 replay store，而不是静默复用旧 `prediction snapshots`；
- 但 `analytics_prediction_snapshots` 仍保留较大桥接职责，尚未完全退回到“运行审计 + 兼容视图”的次要角色。

## 1. 目标

把历史 assessment / rolling audit / release review 的数据来源，从“复用旧的 `prediction snapshots` 过渡缓存”推进到：

```text
raw observations
-> point-in-time feature snapshots
-> release bundle scoring
-> historical assessment replay
-> rolling audit / release review
```

这份文档解决五件事：

1. 历史评估的真源头到底是什么。
2. replay 应该缓存什么，缓存何时失效。
3. `analytics_prediction_snapshots` 在新体系里还保留什么角色。
4. runtime posture clause、threshold policy、release metadata 如何一起进入历史回放。
5. 何时可以说“historical audit 已经不再依赖 persisted snapshot bridge”。

## 2. 为什么现在必须做

截至 `2026-06-01`，已经确认两个现实问题：

1. 只看旧 `prediction snapshots` 会被历史缓存污染；
2. 即使补了 bundle-backed cache invalidation，当前体系仍然是“以 snapshot 为主、以 raw replay 为补”。

这会带来三个风险：

- release review 容易混入旧 runtime policy 的历史 posture；
- `posture_trigger_codes / blocker_codes` 的解释链不一定来自当前 active release 真正重放；
- formal main 很难从工程上证明自己已经脱离过渡链。

## 3. 设计原则

- `prediction snapshots` 可以保留，但不再作为正式历史审计的 source of truth。
- 历史 replay 的最小单位是：`release_id x as_of_date x market_scope`。
- replay 必须显式绑定 `feature_set_version`、`point_in_time_mode`、`runtime_policy_version`。
- 任何会改变 posture / time bucket / actionability 的策略变化，都必须能触发历史缓存失效。
- 若无法满足 replay 所需的 PIT 特征覆盖，应该明确标记失败，而不是静默退回旧 snapshot。

## 4. 新的历史审计链路

## 4.1 Source of Truth

正式历史审计的 source of truth 定义为：

1. `raw observations`
2. `research_feature_snapshots`
3. `analytics_model_releases` 中的 active / candidate bundle
4. `protected_stress_window_catalog`
5. 当前 runtime threshold / posture policy / action playbook 版本

### 4.2 不再作为 source of truth 的对象

以下对象可以保留，但只能作为派生产物：

- `analytics_prediction_snapshots`
- API 当前历史曲线缓存
- 前端图表直接读取的旧历史序列

## 5. Replay 的产物

建议把 replay 产物分成两层。

### 5.1 点级产物

```text
analytics_historical_assessment_points
```

每个点至少包含：

```text
release_id
market_scope
as_of_date
feature_snapshot_id
point_in_time_mode
runtime_policy_version
action_playbook_version
overall_score
structural_score
trigger_score
external_shock_score
raw_p_5d
raw_p_20d
raw_p_60d
calibrated_p_5d
calibrated_p_20d
calibrated_p_60d
posture
time_to_risk_bucket
actionability_prepare
actionability_hedge
actionability_defend
posture_trigger_codes_json
posture_blocker_codes_json
coverage_score
freshness_status
generated_at
replay_run_id
```

### 5.2 运行级产物

```text
analytics_historical_replay_runs
```

每次 replay 至少包含：

```text
replay_run_id
release_id
market_scope
from_date
to_date
history_cache_key
feature_set_version
label_version
point_in_time_mode
runtime_policy_version
action_playbook_version
protected_window_catalog_id
source_watermark
status
point_count
failure_reason nullable
created_at
```

## 6. `analytics_prediction_snapshots` 的新角色

在新体系里，`analytics_prediction_snapshots` 只保留三个角色：

1. 当前线上运行的滚动快照审计；
2. 前端“最近一段时间”查看当前线上行为；
3. 当 raw replay 还未覆盖全部日期时，作为临时桥接视图。

明确禁止：

- 不再把它作为 formal dataset builder 的正式输入；
- 不再把它作为 release review 的默认历史依据；
- 不再以“snapshot 已存在”为理由跳过 raw PIT replay。

## 7. 历史缓存键

新的历史缓存键建议至少包含：

```text
release_id
market_scope
feature_set_version
label_version
point_in_time_mode
runtime_policy_version
action_playbook_version
protected_window_catalog_id
history_range
feature_snapshot_watermark
observation_watermark
```

### 7.1 必须触发失效的场景

只要以下任一变化，必须重放：

1. `release_id` 变化
2. `feature_set_version` 变化
3. `point_in_time_mode` 变化
4. posture threshold 或 action fusion policy 变化
5. protected stress window catalog 变化
6. 原始观测新增或修正，影响历史 `visible_at`
7. 动作头启用状态或 bundle 内容变化

## 8. Replay 模式

### 8.1 `strict_rebuild`

用于：

- release review
- baseline vs candidate 对照
- 研究报告导出

规则：

- 不读 `analytics_prediction_snapshots`
- 完全依赖 raw observations + PIT features

### 8.2 `incremental_refresh`

用于：

- active release 日常延伸最新几天历史
- 新数据刷新后补最近窗口

规则：

- 优先复用已有 replay points
- 只重放受水位影响的日期范围

### 8.3 `bridge_fallback`

只在过渡期允许：

- 当某些老日期尚无 feature snapshots
- 但需要前端保留最小历史曲线时

规则：

- 明确打标 `history_source = transitional_snapshot_bridge`
- release review 禁止把 bridge 段计入正式晋升依据

## 9. 与 PIT feature store 的接口

历史 replay 不应直接从 observation 临时拼特征，而应优先读取：

```text
research_feature_snapshots
```

理由：

- replay 与训练输入必须共享同一份特征口径；
- 否则会出现“训练时一套特征，历史评估时另一套特征”的伪差异。

### 9.1 最低要求

每个 `as_of_date` 的 feature snapshot 至少要暴露：

```text
feature_snapshot_id
entity_id
market_scope
as_of_date
feature_set_version
point_in_time_mode
latest_visible_at
coverage_score
features_json
```

## 10. Release Review 的新默认规则

从这份设计开始，release review 的默认顺序应改成：

1. 读取 baseline/candidate release metadata
2. 检查 replay cache 是否命中且口径一致
3. 若不一致，先做 `strict_rebuild`
4. 用 replay points 生成：
   - posture distribution
   - runtime threshold hits
   - regime probability summary
   - rolling audit
5. 最后才渲染 Markdown / JSON 报告

## 11. Failure Handling

如果 raw PIT replay 失败，必须显式区分失败原因：

| failure_reason | 含义 |
|---|---|
| `missing_feature_snapshots` | 某段日期还没有 PIT 特征快照 |
| `coverage_below_minimum` | 特征覆盖率不够 |
| `visibility_mode_mismatch` | release 声明的 PIT 模式高于当前数据可见性 |
| `bundle_load_failed` | release bundle 无法加载 |
| `runtime_policy_missing` | posture / action policy 元数据不完整 |

处理原则：

- release review 可失败；
- 不允许因为 replay 失败就静默退回旧 snapshot 并继续给 candidate 通过。

## 12. 分阶段迁移计划

### Phase 1

- 保留 `analytics_prediction_snapshots`
- 新增 `analytics_historical_replay_runs`
- release review 默认优先 raw replay

### Phase 2

- 前端历史曲线优先切到 replay points
- snapshot bridge 只保留最近运行审计

### Phase 3

- formal dataset builder 完全不再依赖 `prediction snapshots`
- `bridge_fallback` 只服务旧历史兼容，不服务正式评审

### Phase 4

- 当 `1990+` 主要区间都已具备 replay coverage 后
- 才能把 `rolling audit no longer depends on persisted snapshot bridge` 在 Go/No-Go 文档里勾掉

## 13. 完成定义

只有以下条件同时满足，才算该专项完成：

1. candidate release review 默认使用 raw PIT replay
2. `analytics_prediction_snapshots` 不再是 formal dataset 或 release review 的主输入
3. 历史缓存键已纳入 runtime policy / actionability / protected window catalog
4. replay 失败时会显式报错，而不是静默退回旧桥接链
5. `1990+` 主区间的大部分历史 assessment 可从 PIT feature snapshots 直接重放

## 14. 下一步实施顺序

1. 先固化 replay run / point 表结构与 cache key；
2. 再让 worker 支持 `strict_rebuild`；
3. 再把 release review 全部切到 replay points；
4. 最后再收缩 snapshot bridge 的职责。

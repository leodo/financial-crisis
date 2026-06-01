# 动作层概率设计

状态：`Draft`

最后更新：2026-06-01

## 1. 目标

把“危机概率”与“现在是否已经该动作”拆开，避免单一概率头同时承担：

1. 中期脆弱性排序；
2. 危机前数周可执行离场；
3. 危机期间持续高压状态；
4. UI 上的 posture / 仓位动作映射。

## 2. 为什么需要单独设计

截至 2026-06-01，已经验证过四条路：

1. `forward_crisis` 单头概率
2. `scenario-aware weighting` 单头概率
3. `action_window` 单头概率
4. `dual-head proxy actionability`（动作头接入 serving 诊断融合）

结论一致：

- 离线校准指标可以还不错；
- runtime `timely_warning_rate` 仍明显不够；
- 一旦把动作标签铺得太宽，`actionable_precision` 和误报持续天数会明显恶化。

这说明“危机先验”和“动作触发”不是同一个学习目标。

## 3. 建议架构

### 3.1 Head A: Crisis Prior

含义：

- 评估未来 `20d/60d` 进入危机或高压状态的先验概率。

建议标签：

- 继续保留 `label_20d`
- 继续保留 `label_60d`
- `5d` 只作为辅助短窗头，不承担主要动作决策

用途：

- 排序
- 长周期脆弱性积累
- 历史类比和月级风险提示

### 3.2 Head B: Actionability

含义：

- 评估“现在是否已经进入准备 / 对冲 / 防守的动作窗口”。

建议标签：

- `prepare_window`
- `hedge_window`
- `defend_window`

或最小版：

- `action_label_20d`
- `action_label_5d`

但应进一步改成 episode 目标，而不是继续只做 horizon 二分类。

用途：

- posture 升级
- `time_to_risk_bucket`
- `position guidance`
- rolling audit 的动作级命中统计

### 3.3 Runtime Fusion

最终前端和 API 不直接暴露单一模型输出，而是融合：

```text
crisis_prior
+ actionability
+ trigger confirmation
+ data trust
= posture / time bucket / execution urgency
```

原则：

- `crisis_prior` 高，不等于立刻 `defend`
- `actionability` 高，但 `data_trust` 差，也不能直接升级
- posture 应由多块联合决定，而不是只看某个 horizon 的单点概率

## 4. 需要的数据

新增或强化以下对象：

```text
action_window_start
action_window_end
episode_id
episode_phase
action_phase_score
```

至少要能回答：

1. 当前样本处在危机前多少期。
2. 这个动作窗口是 `prepare`、`hedge` 还是 `defend`。
3. 该窗口是否只是受保护压力期，而不是主危机正例。

## 5. 最小实现顺序

1. 在 formal dataset 中保留 `forward labels + bounded action labels`
2. 新增 `actionability` 训练入口，不替换现有 `formal_bundle_v1`
3. 先做双头离线导出与评估，不直接切线上
4. 更新 API method metadata，明确返回：
   - `crisis_prior_source`
   - `actionability_source`
   - `fusion_policy_version`
5. 最后再改 posture / time bucket 的融合逻辑

### 5.1 训练侧专属评估口径

动作头不能只沿用普通 horizon 的 `brier / log_loss / ece / precision_at_30pct`。

从 2026-06-01 这一轮开始，训练侧还应额外导出：

- `pre_start_recall_at_threshold`
  - 在动作窗口正样本里，有多少命中发生在 `crisis_start` 之前；
- `post_start_recall_at_threshold`
  - 有多少命中直到 `crisis_start` 之后才出现；
- `advance_warning_rate`
  - 按场景统计，至少有一次在危机前触发动作信号的比例；
- `late_confirmation_rate`
  - 按场景统计，危机前没有命中，但危机后才首次确认的比例；
- `missed_rate`
  - 按场景统计，整个动作窗口都没有形成信号的比例。

这些指标的目的不是替代 runtime guard，而是先把“模型是提前看见了，还是等危机已经开始才补确认”拆开。

## 6. 当前结论

当前代码已经证明：

- action label 是必要的；
- 但 action label 单独替换掉原有概率标签还不够；
- `separate actionability layer` 的工程链路已经打通，但第一版 `proxy actionability head` 仍没有解决动作级提前量不足；
- 后续开发应该进入 `episode-native actionability target`，而不是继续在单头 bundle 或 proxy dual-head 上做小修小补。

### 6.1 2026-06-01 实施检查点

本轮已经完成：

1. 双头 bundle 导出
2. API `actionability` 字段与方法元数据补充
3. web 面板动作概率展示
4. runtime posture / time bucket 的诊断性融合
5. 训练侧 `actionability` 专属评估口径

对应候选版：

- `us_formal_pit_dualhead_20260601T003145`

但首轮 review 结果仍然是：

- `timely_warning_rate`: `37.5% -> 12.5%`
- `actionable_precision`: `29.6% -> 20.6%`
- `longest_false_positive_episode_days`: `9 -> 18`

因此这份设计文档的当前结论要再往前收敛一步：

- `dual-head plumbing` 不是问题；
- 当前失败点主要在于动作头标签仍然只是 `bounded action window` 的 horizon proxy；
- 当前训练报告已经能区分“提前命中 / 过晚确认 / 完全漏报”；随后这一层指标也已经接入 release review 护栏；
- 下一轮不应优先改融合阈值，而应先改 `prepare / hedge / defend` 的目标定义、样本构造和 episode 评估口径。

### 6.2 2026-06-01 actionability guard 已进入 release review

本轮继续推进后，`actionability` 已不再只是训练输出里的诊断字段，而是正式进入 `release review`：

- review 报告会展示每个动作层级的 `scenario_count / advance_warning_rate / late_confirmation_rate / missed_rate`
- review guard 会直接拦截：
  - `scenario_count < 2`
  - evaluation 正样本存在，但动作头完全零命中

在这个前提下重新复核：

- `us_formal_pit_dualheadguard_20260601T012122`

结果仍然是 No-Go，而且原因比之前更清楚：

- 三个动作层级都只落在 `1` 个 evaluation 场景上；
- 三个层级在各自 evaluation 正样本上都是 `0` 命中；
- 这说明当前问题已经可以明确归因到：
  - 动作头目标定义还不对；
  - split / 场景覆盖太窄；
  - 不是简单调阈值能解决的。

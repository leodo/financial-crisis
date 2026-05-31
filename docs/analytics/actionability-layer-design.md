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

截至 2026-06-01，已经验证过三条路：

1. `forward_crisis` 单头概率
2. `scenario-aware weighting` 单头概率
3. `action_window` 单头概率

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

## 6. 当前结论

当前代码已经证明：

- action label 是必要的；
- 但 action label 单独替换掉原有概率标签还不够；
- 后续开发应该进入 `separate actionability layer`，而不是继续在单头 bundle 上做小修小补。

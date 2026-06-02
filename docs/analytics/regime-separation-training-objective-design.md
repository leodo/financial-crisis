# Regime Separation 训练目标设计

状态：`Draft`

最后更新：2026-06-02

## 1. 目标

定义 formal main 主概率头下一阶段真正要优化什么，避免继续出现：

- `normal / positive_window / in_crisis / cooldown` 概率几乎一样；
- `p_5d / p_20d / p_60d` 被校准后压成近似常数；
- runtime floor 命中始终为 `0`，但候选版仍因为“和 baseline 一样冷”而看起来没那么差。

这份文档解决三件事：

1. `forward_crisis` 主概率头应该如何显式约束 regime separation；
2. release review 应该用什么口径判断“模型是真的看见了 pre-warning”，而不是只会做冷概率排序；
3. 下一轮实现应该优先改哪里，而不是继续微调 serving 阈值。

## 2. 当前问题的人话解释

截至 `2026-06-01`，当前 formal main 的主问题已经不是：

- 缓存错了；
- UI 没解释清楚；
- 动作头阈值没调好。

真正的问题是：

```text
主概率头没有把 positive_window 稳定拉离 normal
```

具体表现为：

- `5d / 20d` 很容易被压成近乎常数；
- `60d` 虽然有波动，但对 runtime posture floor 仍不够；
- 候选版看上去“没有明显更差”，其实只是 baseline 和 candidate 都偏冷。

## 3. 设计原则

- 第一优先级不是追求更低 `Brier / ECE`，而是先证明模型能分出“平时”和“危机前窗口”。
- `positive_window` 必须显式优于 `normal`，不能只靠后处理阈值碰运气。
- `in_crisis` 与 `post_crisis_cooldown` 不应和 `positive_window` 完全混成一个概率水平。
- 训练目标、模型选择、release guard 要用同一套 regime 语义。
- 在 Rust 线上可加载约束下，优先采用简单、可解释、可逐步实现的方法。

## 4. Regime 定义

第一版统一使用这五类：

| regime | 含义 |
|---|---|
| `normal` | 不在危机前缓冲、正例窗口、危机中或余震期 |
| `pre_warning_buffer` | 距离正式正例窗口还有一段时间，但已进入脆弱性预热区 |
| `positive_window` | 本 horizon 认为最值得提前识别的窗口 |
| `in_crisis` | 危机已进入主要释放阶段 |
| `post_crisis_cooldown` | 危机后余震期，高压仍可能存在，但不等于新的前瞻窗口 |

## 5. 各 horizon 的理想排序

### 5.1 `p_60d`

目标不是短端精确 timing，而是：

```text
positive_window > normal
pre_warning_buffer >= normal
in_crisis 不必远高于 positive_window
post_crisis_cooldown 不应长期高于 positive_window
```

人话解释：

- `p_60d` 要能看见中期脆弱性积累；
- 但不能在危机已经发生后还比危机前窗口更有“前瞻性”。

### 5.2 `p_20d`

目标是：

```text
positive_window 明显高于 normal
in_crisis 可以略高，但不能与 positive_window 完全无差别
cooldown 应开始回落
```

人话解释：

- `p_20d` 是最关键的“未来几周风险是否临近”头；
- 这条线如果拉不开，`prepare / hedge` 就只能靠 serving 语义硬推。

### 5.3 `p_5d`

目标是：

```text
acute pre-warning / positive_window > normal
in_crisis 不需要比 positive_window 高很多
cooldown 应快速回落
```

人话解释：

- `p_5d` 不是要长期漂高，而是要在急性冲击前后形成短促、可解释的抬升。

## 6. 目标函数设计

## 6.1 第一阶段：保持二元输出，增加 regime-aware 约束

第一阶段不强行换复杂模型，先保留当前二元 horizon 输出：

```text
label_5d
label_20d
label_60d
```

但训练和选模不再只看普通分类损失，而是增加三层 regime-aware 约束。

### A. 样本权重

建议按 `(horizon, regime)` 给权重：

| horizon | `normal` | `pre_warning_buffer` | `positive_window` | `in_crisis` | `post_crisis_cooldown` |
|---|---:|---:|---:|---:|---:|
| `5d` | 1.0 | 1.3 | 2.0 | 1.2 | 0.8 |
| `20d` | 1.0 | 1.4 | 2.2 | 1.1 | 0.7 |
| `60d` | 1.0 | 1.3 | 1.8 | 0.9 | 0.7 |

解释：

- `positive_window` 必须被明确抬权；
- `in_crisis` 不再和正例窗口等权；
- `cooldown` 应该对“仍然高分”形成负面约束。

### B. 候选模型选择分

除了 `brier / log_loss / ece`，候选选择分必须额外包含：

```text
regime_lift_score
+ early_warning_support_score
- cooldown_bleed_penalty
```

### C. Release Guard

即使离线校准指标不错，只要 regime separation 不成立，也不能晋升。

## 6.2 第二阶段：若第一阶段仍偏冷，再加 pairwise separation

如果第一阶段后仍然出现：

- `positive_window_avg_probability <= normal_avg_probability`
- `runtime floor hits = 0`
- `diagnosis = weak_regime_separation`

再进入第二阶段：

- 对 `(positive_window, normal)` 加 pairwise ranking penalty；
- 对 `(cooldown, positive_window)` 加 cooldown suppression penalty；
- 但仍保持最终输出为 Rust 可加载的简单模型或近似方案。

这一步不是当前第一优先级，第一优先级仍是把第一阶段的 regime-aware objective 明确落地。

## 7. 训练输出必须新增的诊断

每个 horizon 至少输出：

```text
normal_avg_probability
pre_warning_avg_probability
positive_window_avg_probability
in_crisis_avg_probability
post_crisis_cooldown_avg_probability
positive_window_lift_vs_normal
in_crisis_lift_vs_normal
cooldown_lift_vs_normal
diagnosis
```

### 7.1 `diagnosis`

第一版建议固定几类：

| diagnosis | 含义 |
|---|---|
| `usable_early_warning` | `positive_window` 对 `normal` 有明确抬升 |
| `weak_regime_separation` | 抬升太弱，不能支撑动作层 |
| `cold_across_all_regimes` | 所有 regime 概率都很冷、差异极小 |
| `calibration_crushed_early_warning` | raw ranking 有抬升，但校准后被压平 |
| `cooldown_bleed` | 危机后余震长时间和正例窗口一样高 |

## 8. Release Review 护栏

### 8.1 最低门槛

一个 horizon 要被视为“可用 early-warning horizon”，至少同时满足：

1. `positive_window_avg_probability > normal_avg_probability`
2. `positive_window_lift_vs_normal >= 1.5`
3. `positive_window_avg_probability - normal_avg_probability >= 0.010`
4. `diagnosis != cold_across_all_regimes`

说明：

- `0.010` 指 1 个百分点绝对差，是第一版默认研究门槛；
- 对 `5d` 急性头可允许更低绝对差，但不能低到接近常数噪声。

### 8.2 明确 No-Go 条件

任一候选版若出现以下任一情况，直接 No-Go：

1. 三个 horizon 的 `usable_early_warning_horizon_count = 0`
2. `20d` 满足 `positive_window_avg_probability <= normal_avg_probability`
3. `20d / 60d` 的 `cooldown_lift_vs_normal` 明显高于 `positive_window_lift_vs_normal`
4. runtime history 中 `prepare / hedge / defend` 三个 floor 命中数全为 `0`

## 9. runtime 与训练的关系

runtime posture floor 仍然保留，但它不再是主要救火工具。

规则改成：

1. 训练负责学出 `positive_window > normal`；
2. 校准负责保持概率有可解释尺度；
3. runtime floor 只负责把已有信号映射成动作节奏；
4. 若训练阶段没有 separation，runtime 不应用更激进阈值硬制造 posture。

## 10. 实现落点

### 10.1 数据集

`research_formal_dataset_rows` 至少要保留：

```text
probability_training_regime
primary_scenario_id
scenario_family
protected_action_window
```

### 10.2 训练侧

新增或固化：

```text
regime_aware_sample_weight(...)
probability_regime_separation_summary(...)
candidate_selection_score(...)
```

### 10.3 Review 侧

release review 必须继续暴露：

- `regime mix`
- `runtime regime probability`
- `usable_early_warning_horizon_count`
- `insufficient_early_warning_horizon_count`

## 11. 实现顺序

1. 先把 regime-aware 权重和诊断固化到 dataset / training output；
2. 再把 candidate selection score 正式纳入选模；
3. 然后把 `positive_window vs normal` 的最低门槛接入 release guard；
4. 如果仍然全局偏冷，再评估是否需要 pairwise separation 或更复杂目标。

## 12. 当前建议

下一轮不应继续优先做：

- serving 阈值再微调一轮；
- 只换校准策略不动训练目标；
- 指望动作头单独救回主概率头 separation。

下一轮必须优先做：

1. `positive_window > normal` 显式训练约束
2. `cooldown_bleed` 抑制
3. candidate selection 与 release guard 统一口径
4. 让 `20d / 60d` 至少出现可用 early-warning separation，再讨论默认上线

## 13. 2026-06-01 已落地进展

当前代码已经先落了第一批可执行骨架：

- `RegimeSeparationEvaluationSummary` 已显式输出 `pre_warning_buffer / positive_window / post_crisis_cooldown` 的样本数、平均概率与相对 `normal` 的 lift；
- calibration candidate selection 已开始优先比较 `positive_window lift / gap`，并对 `cooldown` 抬升做惩罚；
- runtime regime audit 已新增 `positive_window` 与 `cooldown` 对比，并把 `20d positive_window <= normal`、`cooldown_bleed` 接入 runtime sanity guard；
- bundle-level probability guard 已接入 release review，并把 `zero usable early-warning horizons`、`20d positive_window <= normal`、`cooldown_bleed / cold_across_all_regimes` 作为正式晋升拦截项；
- `cold_across_all_regimes / calibration_crushed_early_warning / cooldown_bleed` 已具备明确 diagnosis。

仍未完成的关键部分：

1. 用重建后的 dataset / candidate release 跑一轮完整复核，验证 `20d / 60d` 是否真正拉开；
2. 把 release review 默认历史来源切到 `strict_rebuild` raw PIT replay；
3. 如果仍然偏冷，再评估是否需要更强的 pairwise separation 或更复杂目标。

## 14. 2026-06-02 重建复核结果

本轮已经用最新口径完成了一次主线闭环：

1. 重建 `formal_v1_main_1990_daily:20260601T163337`
2. 训练 `us_formal_main_20260601T163415`
3. 对比 active baseline `us_formal_transitional_20260531T094603` 跑 release review

结果不是“代码还没连起来”，而是主线训练目标仍未过关：

- bundle evaluation 仍然是 `usable_early_warning_horizons=0`
- `5d=cold_across_all_regimes`
- `20d=weak_regime_separation`
- `60d=cooldown_bleed`
- release review `guard_passed=false`

这次结果同时说明两件事：

1. 扩展历史样本缺失已经不再是当前主阻塞，`1987 / 1994 / 1998 / 2000-2001 / 2011` 的扩展包已经落地；
2. formal main 仍然需要更强的样本治理与训练目标，而不是继续在 serving 阈值上打转。

下一轮真正该改的是：

1. 判断 formal main 是否要吸收一部分 `candidate_optional / protected stress` 场景，改善 `20d/60d` 的 pre-warning 分离；
2. 明确 protected stress 在 formal main 中是 `actionability context only` 还是 `regime-aware negative / protected sample`；
3. 在完成这一步之前，不应继续以当前 formal main 候选版讨论默认上线。

## 15. 2026-06-02 第二轮训练与审计进展

在上一轮 `formal main` 仍然 `usable_early_warning_horizons=0` 之后，本轮已经连续补上四个关键改动：

1. `ForwardCrisis` 20d/60d 加入方向约束，避免信用利差、金融压力、失业率这类特征继续学出反向系数；
2. 把 `pre_warning_buffer / positive_window / cooldown` 负样本改成软标签，而不是一刀切当作纯 `0`；
3. 把 pairwise separation 从“略高即可”改成带 margin 的约束；
4. calibration selection 不再只看 `positive + normal`，而是把 `pre_warning_buffer / in_crisis / cooldown` 一起纳入。

这四步合起来之后，候选版：

- `us_formal_main_20260601T184003`

已经出现明确变化：

- bundle evaluation 不再是 `zero usable horizons`
- `20d=usable_early_warning_separation`
- `60d=usable_early_warning_separation`
- release review 的 probability guard 已经不再报错

但新的瓶颈也非常明确：

- candidate runtime 触发过宽；
- `timely_warning_rate` 与 `actionable_precision` 已明显抬升，但 `longest_false_positive_episode_days` 也被拉长；
- 说明当前已经不是“没有信号”，而是“信号可以看见了，但 runtime 映射仍然太松”。

因此下一阶段不该回头继续证明 `20d / 60d` 能不能拉开，而应转向两件更具体的事：

1. 收紧 formal main 的 runtime posture 触发条件，优先压缩超长误报段；
2. 在不破坏 `20d / 60d usable separation` 的前提下，重新校准 `hedge / prepare` 的 floor 与上下文门控。

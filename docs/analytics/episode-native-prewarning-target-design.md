# Episode-Native 60d Pre-Warning Target Design

状态：`Experimental`

最后更新：2026-06-03

## 1. 背景

`us_formal_interaction_tail_extmix_20260603T062837` 的新增 evidence 已经把 `60d` 瓶颈定位清楚：

- `60d pre_warning_buffer` 没有被过滤，150 行全部进入 calibration 与 threshold selection；
- 但这些行的 hard label 仍是 `0.0`，soft target 只有 `26.0%`，objective weight 只有约 `0.63`；
- calibration hit rate 是 `8.7%`，低于 normal 的 `13.7%`。

这说明问题不是免费历史样本没有进来，也不是 threshold repair 没走到，而是训练目标仍把真正应该提前准备的 `60d pre_warning_buffer` 当成弱负样本。

## 2. 目标

本设计只解决一个窄问题：

```text
让 ForwardCrisis 60d 头能识别“已经进入 prepare episode、但还没有落入 60d hard positive window”的可执行提前准备样本。
```

它不是把 `ForwardCrisis` 头改成动作头，也不是把所有 `pre_warning_buffer` 都当成危机正例。动作层 `prepare / hedge / defend` 仍然由 episode-native actionability 目标定义。

## 3. Eligibility

只有同时满足以下条件的样本，才进入 `prepare_p60d_episode_native_v1`：

1. `label_mode = ForwardCrisis`
2. `horizon_days = 60`
3. `regime_60d = pre_warning_buffer`
4. `label_60d = 0`
5. `prepare_episode_label = 1` 或 `primary_action_level = prepare`
6. `days_to_primary_crisis_start > 0`
7. `primary_scenario_supports_60d = true`
8. 场景家族不是 `acute_market_liquidity_crash`

原因：

- `prepare_episode_label` 用来确认这不是普通背景噪声；
- `days_to_primary_crisis_start > 0` 避免把危机中或危机后样本误抬成提前预警；
- `primary_scenario_supports_60d` 避免把本来不该支持长提前量的急性冲击硬塞进 60d；
- 排除 `acute_market_liquidity_crash`，避免把 `1987 / 2020` 这类短促急性样本包装成数月级提前窗口。

## 4. Target 与 Weight

第一版采用保守软目标：

| 样本类型 | target | objective weight |
| --- | ---: | ---: |
| mandatory / candidate optional prepare buffer | `0.64` | `1.35` |
| extension / protected prepare buffer | `0.58` | `1.10` |
| 其他 `60d pre_warning_buffer` | `0.26` | 沿用现有 regime weight |

解释：

- `0.64` 的目标是让 eligible prepare buffer 有机会穿过当前 `prepare_p60d` runtime floor；
- `0.58` 用于 extension/protected 样本，承认它们有训练价值，但不让它们支配主模型；
- 非 eligible buffer 仍保持旧口径，避免误报段重新变宽。

## 5. Go / No-Go

训练候选必须先跑 `just release-review-fast <candidate>` 做方向性 triage。

Go 条件：

- `timely_warning_rate` 高于 active；
- `longest_false_positive_episode_days <= active + 2`；
- `actionable_precision` 不低于 active 的 `90%`；
- `threshold_diagnostics.calibration_regime_evidence` 中 `60d pre_warning_buffer` 的 `avg_training_target` 和 `avg_objective_weight` 明确高于旧版。

No-Go 条件：

- `timely_warning_rate` 仍停在 `10.0%`；
- 或者提前量靠大面积误报换来；
- 或者 `60d normal / cooldown` months bucket 重新扩散。

若本轮 No-Go，则停止继续调同类 target/weight，进入 `family_conditional_v1` 细分设计与 PoC。

## 6. 边界

这项改动不会解决所有问题：

- 它不补新的免费数据；
- 它不替代 raw point-in-time feature store；
- 它不保证 one-week 级别的提前命中；
- 它只是验证“episode-native prepare 监督是否能让 60d 头恢复可执行提前量”。

## 7. 实测结果

候选：

- `us_formal_interaction_tail_prepare_20260603T081710`

训练输入：

- `formal_v1_main_1990_daily:20260601T172759`
- `formal_v1_ext_stress_1990_daily:20260601T162655`
- `formal_v1_ext_acute_pre1990:20260601T163102`

训练侧 evidence 的变化：

| Horizon | Early regime | Old soft target | New soft target | Old weight | New weight |
| --- | --- | ---: | ---: | ---: | ---: |
| `60d` | `pre_warning_buffer` | `26.0%` | `45.2%` | `0.630` | `0.900` |

快速 release review：

- 命令：`just release-review-fast us_formal_interaction_tail_prepare_20260603T081710`
- active 已恢复：`us_formal_interaction_tail_extmix10_20260602T061401`

| Metric | Active | Candidate | 结论 |
| --- | ---: | ---: | --- |
| `timely_warning_rate` | `10.0%` | `10.0%` | 无改善 |
| `actionable_precision` | `55.9%` | `54.8%` | 轻微变差 |
| `longest_false_positive_episode_days` | `5` | `5` | 未扩散 |
| `prepare_p60d` floor | `65.6%` | `66.1%` | 反而略升 |
| `p_60d>=prepare` history hits | `112` | `97` | 更窄但没有更早 |

结论：

- `prepare_p60d_episode_native_v1` 证明了目标增强能改变 bundle evidence；
- 但它没有改善真正重要的 runtime 提前命中；
- 因此本轮是 No-Go，不晋升；
- 下一步不应继续调同类 soft target / objective weight，应进入 `family_conditional_v1`。

# Family Overlay Bundle Schema Design

状态：`Draft`

最后更新：2026-06-04

## 1. 触发原因

`family_conditional_v1` derived-feature PoC 已经失败：

- 离线 bundle 指标变好；
- 但 runtime `timely_warning_rate` 从 `10.0%` 降到 `0.0%`；
- `60d` runtime diagnosis 从 `usable_early_warning_separation` 退化为 `late_only_no_early_warning`。

这说明单一 logistic head 即使增加 family proxy 特征，也仍然会把不同风险家族混在一起。下一步需要真正的 family overlay / multi-head schema。

## 2. 目标

在保持旧 bundle 兼容的前提下，为每个 horizon 增加可选的 family overlay：

```text
base horizon head
+ family overlay heads
+ family gate / blend policy
= final calibrated probability
```

第一版先服务研究候选，不直接晋升 active。

## 3. Schema 草案

### 3.1 `ProbabilityHorizonBundle`

新增可选字段：

```text
family_overlays: Vec<ProbabilityFamilyOverlayBundle>
```

旧 bundle JSON 没有这个字段时，默认空数组，serving 行为不变。

### 3.2 `ProbabilityFamilyOverlayBundle`

建议字段：

```text
family_id
gate_feature
gate_threshold
gate_slope
blend_weight
raw_model
calibration
decision_threshold
evaluation
note
```

解释：

- `family_id`：例如 `systemic_credit / rate_shock / jpy_carry`；
- `gate_feature`：线上可计算的 `family_proxy__*`；
- `gate_threshold`：超过多少才认为该 family overlay 有资格参与；
- `gate_slope`：gate 过渡斜率，避免 hard switch；
- `blend_weight`：overlay 对 final probability 的最大影响；
- `raw_model / calibration`：该 family 的局部模型；
- `decision_threshold / evaluation`：局部模型自身的审计证据；
- `note`：解释和训练来源。

## 4. Scoring 规则

第一版建议使用概率空间 blend，而不是直接替换 base：

```text
base_p = calibrated(base_head)
overlay_p = calibrated(overlay_head)
gate = sigmoid((gate_feature - gate_threshold) * gate_slope)
final_p = base_p * (1 - gate * blend_weight) + overlay_p * gate * blend_weight
```

边界：

- `gate_feature` 缺失时 overlay 不参与；
- `blend_weight <= 0.50`，避免局部头完全覆盖 base；
- final probability 仍要过原来的 monotonic 与 runtime guard；
- release review 必须输出 overlay contribution，否则 UI 不允许展示成黑盒结论。

## 5. Training 规则

第一版每个 family overlay 只允许用以下样本：

1. 对应 `scenario_family` 的 positive/context rows；
2. 同 family 的 protected stress rows；
3. 足量 normal rows 作为背景负样本；
4. 不允许只用一个 scenario 训练一个 overlay。

最低训练门槛：

| Overlay | 最低 scenario count | 备注 |
| --- | ---: | --- |
| `systemic_credit` | `2` | 2008 + 2023 起步 |
| `mixed_systemic` | `2` | 2000 + 2011 起步 |
| `rate_shock` | `2` | 1994 + 2022 起步 |
| `acute_liquidity` | `2` | 1987/1998/2020 中至少两个 |
| `jpy_carry` | `1 protected + proxy evidence` | 目前更像风险因子 overlay，不应直接当 crisis family |

## 6. Go / No-Go

必须先过 bundle 级审计：

- 每个 overlay 输出 scenario count；
- 每个 overlay 输出 train/calibration/evaluation row count；
- 每个 overlay 输出 gate hit rate；
- 每个 overlay 输出 `base_p -> final_p` 的贡献分布。

再过 runtime fast review：

- `timely_warning_rate` 必须高于 active；
- `actionable_precision` 不低于 active 的 `90%`；
- `longest_false_positive_episode_days <= active + 2`；
- 若某个 overlay 只让当前快照升分，但历史回放没有提前命中，不允许晋升。

## 7. 实施顺序

1. 先在 domain bundle schema 增加 `family_overlays` 兼容字段；
2. 增加共享 scoring helper，旧 bundle 无 overlay 时输出不变；
3. worker 训练侧先生成 overlay metadata 和局部训练样本统计，不急着训练局部 head；
4. 再实现第一版 overlay training；
5. API runtime 加 overlay contribution diagnostics；
6. release review 和 UI 再显示 overlay contribution。

## 8. 当前边界

本设计是下一阶段真正 multi-head 的基础，不应误解成“已经实现多头模型”。

如果 schema 骨架先落地，仍然只能说明旧 bundle 兼容性已准备好；要产生候选，还必须继续实现 overlay training 和 runtime diagnostics。

## 9. 2026-06-03 当前实现结果

本轮已经落地：

1. `ProbabilityHorizonBundle.family_overlays` / `family_overlay_audits` schema；
2. shared scoring helper 与 runtime overlay contribution diagnostics；
3. worker 端 family overlay audit 与第一版 overlay training 逻辑；
4. 方法页与发布审计页对 overlay diagnostics 的解释展示。

先用单一 `formal_v1_main_1990_daily:20260601T172759` 实测时，训练出的 `family_conditional_v1` 候选仍然是：

```text
family_overlay_audits != empty
family_overlays == []
```

直接原因不是代码缺失，而是数据支持不足：

- 某些 family 只覆盖 `train + eval`，几乎没有 `calibration`；
- 某些 family 只落在 `train + calibration`，没有 `evaluation`；
- `jpy_carry` 这条线的真实卡点后来被拆清了：不是“完全没有样本”，而是
  `proxy-only audit`、overlay dataset 去重和 gate 阈值三层口径叠在一起，导致
  有意义的 protected/pre-warning carry rows 在训练前就被弱化掉了。

随后把 `ext_stress / ext_acute` 与 formal main 组合后，候选 `us_formal_family_conditional_20260603T114855` 与后续脚本复跑产物 `us_formal_family_conditional_20260603T121703` 都稳定复现了同一结构：

```text
5d  configured overlays = 0
20d configured overlays = 1 (acute_liquidity)
60d configured overlays = 0
```

这说明第一版 overlay training 已经真正训出过可服务的 family head，代码链路和免费数据拼装链路都不是阻塞点。

之后又补了一轮 family-level split 治理：

- `just formal-train-family-overlay` 固化为正式复跑入口；
- worker 新增 overlay audit / split 失败原因输出；
- family overlay split 新增 `balanced` fallback，在连续时间切分会把 calibration / evaluation 切成“无正例死区”时，允许按 support 约束做受控分层切分。

这轮重训后的候选 `us_formal_family_conditional_20260603T135847` 已经不再是“只训出一个 20d acute_liquidity head”，而是：

```text
5d  configured overlays = 3 (systemic_credit, rate_shock, acute_liquidity)
20d configured overlays = 3 (systemic_credit, rate_shock, acute_liquidity)
60d configured overlays = 2 (systemic_credit, rate_shock)
```

也就是说，样本拓扑这条线已经从“训不出来”推进到“能训出多个 family head 并进入 serving schema”。

但 `release review` 对 `us_formal_family_conditional_20260603T114855` 以及后续 `us_formal_family_conditional_20260603T135847` 的结论都仍然是 `FAIL`：

- `timely_warning_rate` 从 `10.0%` 降到 `0.0%`；
- `20d` 仍保留 usable early-warning separation；
- `60d` runtime 从基线的 usable separation 退化为 `weak_regime_separation`；
- `prepare_p60d` runtime floor 从 `65.6%` 抬到 `70.8%`，历史 `p_60d>=prepare` 命中从 `112` 收窄到 `24`；
- 更关键的是，candidate runtime replay 里 `60d positive_window` 的 raw probability 均值只有 `2.3%`，反而低于 `60d normal` 的 `3.7%`，说明问题已经不是“overlay 数量不够”，而是 `family_conditional_v1` 的 `60d` 主头本身在 runtime 回放里学出了错误排序。

也就是说，当前问题已经从“overlay 能不能训出来”切换成“两件更具体的事”：

1. family overlay 的样本切分治理是否已经足够代表真实历史回放；
2. `family_conditional_v1` 的 `60d` base head 是否应该继续独立存在，还是改成更保守的 hybrid / fallback 结构。

随后又验证了第一版 hybrid：

- 新增 `family_hybrid_v1`：`5d/20d` 继续使用 family conditional base + overlay，`60d` base head 退回 `interaction_tail_v1`；
- 候选 `us_formal_family_hybrid_20260603T142649` 的 runtime `60d` 不再是“正例反向”，但仍然失败：
  - `timely_warning_rate` 仍是 `0.0%`；
  - `actionable_precision` 从 `55.9%` 降到 `44.4%`；
  - `60d` diagnosis 从 `weak_regime_separation` 变成 `cooldown_bleed`；
  - `60d normal calibrated P = 10.8%`
  - `60d positive_window calibrated P = 16.3%`
  - `60d cooldown calibrated P = 16.9%`
  - `prepare_p60d = 66.1%`

这说明“把 60d 从 family base head 回退到 interaction_tail 基座”是有方向性的，但还不足以恢复 runtime `timely_warning_rate`；当前真正要继续拆的是 `60d cooldown` 为什么会重新盖过 `positive_window`。

我随后又把 `family_hybrid_v1` 收得更保守，直接关闭 `60d` overlay，只保留 `5d/20d` overlay，得到候选 `us_formal_family_hybrid_20260603T144814`。正式 review 的结果几乎没有变化：

- `timely_warning_rate` 仍是 `0.0%`
- `actionable_precision` 仍是 `44.4%`
- `60d` 仍是 `cooldown_bleed`，`cooldown 17.4% > positive_window 16.5%`

这一步很重要，因为它基本排除了“60d overlay 本身把 runtime 拉坏”这个假设。当前更像是：

- `60d interaction_tail + episode-native 目标` 这条 base head 本身还没有把 `cooldown` 压下去；
- 或者 runtime posture / threshold policy 仍然把本就不够强的 `60d` 提前窗口进一步收窄成 `timely_warning_rate=0.0%`。

我随后又试过一轮更激进的 60d 训练目标微调：

- 提高 `60d positive_window` 的 pairwise margin / weight；
- 提高 `60d cooldown` 的负样本惩罚；
- 轻微放宽 `60d pre_warning_buffer` 的负样本压制。

对应候选 `us_formal_family_hybrid_20260603T151447` 的结果更差：

- `prepare_p60d` 又被抬到 `73.0%`
- `timely_warning_rate` 仍是 `0.0%`
- `actionable_precision` 从 `55.9%` 降到 `37.5%`
- `longest_false_positive_episode_days` 从 `5` 升到 `7`

因此这条“继续加大 60d 权重/惩罚”的路线已经明确是 No-Go，代码没有保留这组更激进权重，而是回退到了前一版稳定实现。

因此下一阶段重点进一步收敛为：

1. 针对 `60d` 审计为什么 runtime replay 会从“正例反向”切换成“cooldown_bleed”，并区分到底是 base head、overlay，还是 calibration / floor 在主导；
2. 既然“关闭 60d overlay”也没有改变 review 结论，而“继续加大 60d 权重/惩罚”又明显变差，下一步应直接审计 `60d interaction_tail + episode-native target + runtime threshold policy` 的耦合问题，而不是继续盲调权重；
3. 对 `mixed_systemic` 先重做 proxy 定义（当前 `gate_active_total=0`，继续训练没有意义）；
   - `2026-06-06` 已把 proxy 从“overall/trigger/external/VIX”泛化分数改成
     “credit spread / curve inversion / NFCI”作为慢性压力锚点，`trigger / VIX / external`
     只做确认，同时把 overlay gate 从 `0.50` 下调到 `0.38`；
   - 下一步不再继续拍脑袋调权重，而是直接用真实 formal overlay audit 复核
     `2000 / 2011` 是否已经出现足够的 gate-active rows。
4. 对 `jpy_carry` 单独补 family proxy / protected stress 样本后再决定是否进入正式 overlay 训练；
   - `2026-06-06` 已把这条线继续前推到“真实可训练”：
     1. `proxy-only audit` 现在把 `protected_action_window` 和 gate-active carry rows
        一起视为候选支持，和 overlay dataset builder 保持同一口径；
     2. overlay dataset builder 在 formal main / ext_stress / ext_acute 叠加时，
        不再简单 `dedup` 掉重复 identity 行，而会合并并保留更强的
        `label / regime / protected_action_window`；
     3. 基于真实 free-history formal dataset 分布，`jpy_carry` 的 gate 从 `0.50`
        下调到 `0.38`。这是数据驱动的，而不是拍脑袋调参：当前 protected /
        pre-warning carry rows 的 proxy 最高约 `0.389`，旧 gate 在数据上根本不可能点亮；
     4. 候选 `us_formal_family_hybrid_20260606T104037` 已证明 `jpy_carry`
        真实进入 `5d/20d` overlay：`configured=5`，并且 fast review
        `guard_passed=true`，`actionable_precision 75.2% -> 75.8%`，
        没有引入新的 bundle-level probability guard regression。
   - 因此这条 TODO 的状态已经从“先补样本、还没训起来”推进到
     “已经能训起来，下一步转向 scenario-level attribution audit”。
5. 继续保留 `just formal-train-family-overlay` / `just formal-train-family-hybrid` 作为主复跑入口，避免后续实验再次退回手工拼 dataset key。

随后我又验证了一轮“只在 API runtime 对 `prepare_p60d` 做软收敛”的假设：把 candidate `us_formal_family_hybrid_20260603T144814` 的 `prepare_p60d` 从 `66.1%` 压到 `61.0%`，希望用更宽的 `60d` 动作门槛恢复提前量。结果是：

- `prepare` posture 次数从 `16` 增到 `27`；
- `months` bucket 次数从 `22` 增到 `34`；
- 但 `timely_warning_rate` 仍是 `0.0%`；
- `actionable_precision` 反而从 `44.4%` 降到 `39.1%`；
- `longest_false_positive_episode_days` 从 `5` 升到 `7`。

这说明“单纯把 runtime `prepare_p60d` 放宽”只会增加非关键窗口里的动作信号，并没有把 candidate 的动作级预警拉回真正的危机前窗口。因此这条 runtime threshold soft-cap 路线已经被否决，代码没有保留该策略。

为避免后续继续盲打，我把 `release review` 补成了场景级 backtest 对比输出。对 `us_formal_family_hybrid_20260603T144814` 的最新 fast review，新增的 `Scenario-Level Backtests` 表已经明确指出：

- 真正从 baseline 丢掉的不是全部样本，而是唯一的 timely 样本：`2023 美国区域银行危机`；
- baseline 仍保留 `L2=83d`、`L3=70d`；
- candidate 仍保留 `L2=83d`，但 `L3` 已经消失；
- 其他样本大多本来就是 `missed_to_missed`。

所以现在问题的最小闭环已经从“60d 阈值是否太高”收缩成更精确的问题：

1. 为什么 candidate 仍能在 `2023` 场景上保留同样的结构性 `L2=83d`，却丢掉唯一一次 `L3=70d` 的动作级预警；
2. 这个缺口究竟来自 `prepare/hedge` 姿态条件、`actionable` 桥接逻辑，还是 `2023` 场景在 family-hybrid 概率分布里的时间定位已经偏离可执行窗口；
3. 下一轮实验必须直接围绕 `2023 regional banks` 这条唯一 timely 样本做场景级复盘，而不是继续做全局阈值微调。

在 `2026-06-03` 新增 `Focus Scenarios` 逐日复盘后，这个问题已经可以再收窄一层：

- `family_hybrid` 并不是“从来没打到动作级条件”，而是只在危机前打到了 **2 个零星 actionable points**；
- baseline 同一窗口有 **13 个 pre-crisis actionable points**，因此可以满足 backtest `5 天窗口至少 3 次命中` 的 sustained rule，candidate 则完全不满足，所以 `L3` 记为 `—`；
- `2022-12-28` 这一天 candidate 其实短暂满足了 `prepare` 动作级条件，但从 `2022-12-31` 开始 `p_20d` 已重新跌破 `18%`，`2023-01-03` 之后又大量退回 `normal` posture；
- 到 `2023-02-20 ~ 2023-02-27`，baseline 已连续进入 `hedge + weeks + hedge_p20d_context`，candidate 只有 `2023-02-22` 一天真正进入同口径 `hedge`，其余日期都掉回 `normal`；
- 因此当前主故障更像是 **`20d / posture context` 的持续性塌缩**，而不是单独一个 `60d` 首次越线没出现。

这也意味着下一轮不应再继续做“放宽 `prepare_p60d` 阈值”这类单点修补，而应直接审计：

1. `family_hybrid` 为什么把 `2023` 场景里的 `20d` 抬升和 `hedge` 姿态连续性压扁；
2. `prepare_structural_downgrade` / `hedge_p20d_context` 在 candidate 中为什么只能零星出现；
3. `release review` 已继续前推一层，新增“`L3` sustained hits 证据列”，现在能把 `3/5` 命中条件直接显式输出；下一步不再争论“要不要加”，而是要继续下钻导致连续性塌缩的训练样本与 family feature 根因。

为避免这条主线继续只停留在 review 结论里，后续实验入口统一收口到：

- [2023 区域银行危机 L3 修复设计](regional-banks-2023-l3-repair-design.md)

后续如果继续推进 family-hybrid / overlay 修复，应优先满足该文档里定义的诊断产物、实验边界和 Go/No-Go 条件，而不是重新回到全局阈值微调。

## 10. 2026-06-04 补充：20d-only derived tail 约束验证通过

在上面的 `regional banks 2023` 复盘基础上，我随后验证了一个更收敛的假设：

- 不是继续盲调 `60d`；
- 而是只修 `20d` 上那些已经有明确风险方向、却被 family-hybrid 学成错误惩罚方向的 derived tail 特征。

第一轮我把 derived tail 单调约束同时施加到 `20d/60d`，得到候选 `us_formal_family_hybrid_20260603T191209`。结果很说明问题：

- `20d hits` 从 baseline 的 `13` 抬到 `29`
- 但 `60d hits` 从 `0` 爆到 `62`

这说明“方向约束本身”不是错的，错的是把它粗暴复制到了 `60d`。

我随后把这条约束收窄成 **只对 `20d` 生效**，重训得到 `us_formal_family_hybrid_20260603T192249`。这版结果才符合当前设计目标：

- 相比 baseline `us_formal_interaction_tail_extmix10_20260602T061401`
  - `20d hits`: `13 -> 29`
  - `positive_window 20d hit rate`: `40% -> 80%`
  - `60d hits`: `0 -> 0`
- 相比错误的中间候选 `191209`
  - `20d` 改善完全保留
  - `60d hits` 从 `62 -> 0`

更关键的是，这版不只通过了离线切片 compare，也通过了运行态 review：

- `just release-review-fast us_formal_family_hybrid_20260603T192249`
- `just release-review us_formal_family_hybrid_20260603T192249`

两轮结果一致：

- `timely_warning_rate`: `10.0% -> 10.0%`
- `actionable_precision`: `55.9% -> 57.9%`
- `longest_false_positive_episode_days`: `5 -> 7`
- `guard_passed=true`

所以当前 family-hybrid 主线的结论已经更新为：

1. `60d` fallback 到 `interaction_tail_v1` 是对的；
2. `20d` 上需要做更定点的符号/方向约束，而不是继续泛化调 `60d`；
3. derived tail 方向约束已经证明要按 horizon 区分，**20d 可用，60d 暂不允许直接套用**；
4. 下一步不是重新大改 schema，而是继续围绕 `tail_neg__us_curve_10y2y_level__0`、`rate_shock family context`、`USDJPY/jpy carry` 这些已定位的特征做定点治理，并审计为什么误报最长段从 `5` 增到 `7`。

## 11. 2026-06-04 补充：interaction sign constraint 让 family-hybrid 再前进一步

在 `192249` 之后，我先验证了一轮“继续压 `curve/fed-funds` cap”：

- `us_formal_family_hybrid_20260604T022954`
- `us_formal_family_hybrid_20260604T031738`

结论都一样：

- `2023-02` 的额外 `20d hits` 仍是 `4`
- `2023-07` 的额外 `20d hits` 仍接近 `17`
- formal fast review 仍停在 `actionable_precision=60.9%`

这说明单纯继续压 `curve inversion / fed funds / rate_shock cap` 已经接近收益上限。

随后我把方向从“继续硬压系数”改成“修正交互项的风险语义”：

- 对几类明确应当同向放大风险的 interaction 纳入 `forward_crisis sign constraint`
  - `interaction__overall_score__us_vix_level`
  - `interaction__structural_score__trigger_score`
  - `interaction__trigger_score__us_vix_level`
  - `interaction__external_dimension_score__us_usdjpy_level`
  - `interaction__us_nfci_level__us_stlfsi_level`
  - `interaction__us_baa_10y_spread_level__us_vix_level`

这样做的原因很直接：

- 这些交互项在 `031738` 里有一部分被学成了负权重；
- 结果就是 `low-vix / low-spread` 这类正常窗口，反而会因为“低于均值”被负权重抬高 `20d` 风险；
- 这不是“系数太大”，而是“方向学错了”。

新候选 `us_formal_family_hybrid_20260604T034053` 的结果明显更好：

- 局部 compare：
  - `2023-02-01 ~ 2023-02-15`：`avg delta p20d +0.111 -> +0.085`
  - `2023-07-01 ~ 2023-07-20`：额外 `20d hits 17 -> 13`
  - `us_regional_banks_2023`：`20d hits 13 -> 28`、`positive_window hit rate 40% -> 80%`
- formal strict review：
  - `timely_warning_rate`: `10.0% -> 10.0%`
  - `actionable_precision`: `54.8% -> 67.3%`
  - `longest_false_positive_episode_days`: `5 -> 5`
  - `guard_passed=true`

所以当前 family-hybrid 主线的最新判断应更新为：

1. `20d-only derived tail` 方向约束是第一步；
2. `monotonic positive interaction` 的 sign constraint 是第二步，而且已经把候选推进到目前最优；
3. 当前剩余误报的主导项，已经从 `curve/fed-funds` 进一步转移到 `USDJPY level / jpy carry proxy`；
4. 下一步如果继续推进，不该再回到泛化调 `60d` 或重复压同一组 cap，而应重点审计 `USDJPY level`、`jpy carry proxy/context` 与 `20d threshold` 的收口空间。

随后我又验证了两轮“继续把 `USDJPY / jpy_carry / rate_shock` 压成辅助上下文”的收口实验：

- `us_formal_family_hybrid_20260604T043437`
  - 新增 `20d` family-context 条件下的：
    - `us_usdjpy_level <= 0.22`
    - `family_proxy__jpy_carry <= 0.06`
    - `family_context__jpy_carry__external_dimension_score <= 0.10`
  - 局部 compare 更干净：
    - `2023-02 avg delta p20d +0.085 -> +0.055`
    - `2023-07 avg delta p20d +0.256 -> +0.222`
  - 但 fast review 只有：
    - `actionable_precision 54.8% -> 66.7%`
- `us_formal_family_hybrid_20260604T045257`
  - 在此基础上进一步把 `rate_shock` cap 收到：
    - `family_proxy__rate_shock <= 0.06`
    - `family_context__rate_shock__external_dimension_score <= 0.12`
  - 局部 compare 再进一步：
    - `2023-07 extra 20d hits 13 -> 12`
    - `2023-07 avg delta p20d +0.222 -> +0.209`
  - 但 fast review 反而进一步回落到：
    - `actionable_precision 54.8% -> 66.0%`

所以这里的结论也要更新：

1. `034053` 仍然是当前 family-hybrid 主线的最好候选；
2. 继续堆 `USDJPY / jpy_carry / rate_shock` coefficient cap，局部窗口还能变干净，但已经很难再转化成更好的 runtime review；
3. 原因大概率不再是“哪个系数还不够小”，而是：
   - `20d threshold selection` 会在局部误报收缩后重新下探；
   - `USDJPY level` 作为 base level 特征本身仍然过宽；
   - `jpy_carry` 更适合被改成 tail/context 结构，而不是继续在线性 base head 里承载广义水平语义。

因此，family-hybrid 下一轮不应继续沿这条 coefficient cap 线往前走，而应把重点切到：

- `20d threshold selection`
- `jpy_carry proxy/context` 重构
- `USDJPY level -> tail/context` 的语义迁移

随后我又补了两轮更贴近这三个方向的验证，结论进一步收窄了可行边界：

- 候选 `us_formal_family_hybrid_20260604T055652`
  - 改动：
    - `20d threshold` 增加“soft overprediction penalty”，避免因为局部窗口稍变干净就继续往更低阈值下探；
    - `jpy_carry proxy` 改成“高位水平 + 20d 变化 + 外部维度确认”的更保守组合，而不是继续让绝对水平单独主导。
  - 结果：
    - `20d threshold` 回到 `0.451`，没有像极端版本那样把 `regional_banks` 直接打穿；
    - `2023-02-01 ~ 2023-02-15` 仍有 `4` 个额外 `20d hits`；
    - `2023-07-01 ~ 2023-07-20` 仍有 `12` 个额外 `20d hits`；
    - `us_regional_banks_2023` 仍保持 `20d hits 13 -> 28`、`positive_window hit rate 40% -> 80%`；
    - 但 fast review 只做到 `actionable_precision 54.8% -> 65.5%`，仍低于 `034053` 的 `67.3%`。
  - 结论：
    - `soft threshold` 这条思路本身可保留；
    - 但“只改 `jpy_carry proxy`”还不足以超过 `034053`。

- 候选 `us_formal_family_hybrid_20260604T061852`
  - 改动：
    - 继续把 `USDJPY level -> tail/context` 往前推，直接把
      `interaction__external_dimension_score__us_usdjpy_level`
      改成更窄的 tail 交互语义。
  - 结果：
    - `20d threshold` 直接塌到 `0.294`；
    - `predicted_positive_count` 膨胀到 `1196`；
    - `normal hit rate` 升到 `14.2%`，明显重新打开了常态误报面。
  - 结论：
    - “直接替换 raw interaction 为 tail interaction” 这条具体实现不可取；
    - 这不是正确的 `USDJPY level -> tail/context` 落地方式，至少当前不该继续沿这个改法前进。

- 候选 `us_formal_family_hybrid_20260604T064930`
  - 改动：
    - 继续沿 `curve/bond-spread + USDJPY` 这条线压 `20d` 常态误报，
      把 `tail_neg__us_curve_10y2y_level__0` 与 `USDJPY level` 的放大效应继续往下收。
  - 结果：
    - 相比 `034053`，`2023-02-01 ~ 2023-02-15` 的 `20d hits` 从 `4` 压到 `1`；
    - `2023-07-01 ~ 2023-07-20` 的 `20d hits` 从 `12` 压到 `2`；
    - 但 `regional_banks` 的 `20d` 连续性同步回落到 `20d hits 27 -> 19`、
      `positive_window hit rate 75% -> 60%`；
    - runtime fast review 结果是：
      - `timely_warning_rate 10.0% -> 10.0%`
      - `actionable_precision 54.8% -> 65.1%`
      - `longest_false_positive_episode_days 5 -> 5`
  - 结论：
    - 这版已经证明“继续靠压 `curve/USDJPY` 来换常态窗口干净度”会开始直接吃掉
      `regional_banks` 的 `20d` 连续性；
    - 虽然它仍通过 fast review 护栏，但没有超过 `034053` 的 `67.3%`，
      因此不能成为新的正式主线。

- 候选 `us_formal_family_hybrid_20260604T064040`
  - 改动：
    - 在 `064930` 同一方向上继续加深 `tail_neg__us_curve_10y2y_level__0` 负权，
      同时维持更低的 `USDJPY level` 基础权重，目标是把 `2023-02 / 2023-07`
      的 `20d` 常态误报进一步压到接近归零。
  - 结果：
    - `2023-02-01 ~ 2023-02-15` 的 `20d hits` 从 `4` 直接压到 `0`；
    - `2023-07-01 ~ 2023-07-20` 的 `20d hits` 也从 `12` 压到 `0`；
    - 但 `regional_banks` 的 `20d` 连续性同步塌到 `20d hits 27 -> 7`、
      `positive_window hit rate 75% -> 25%`；
    - 特征审计已经说明这不是阈值单独导致的：
      - `20d threshold` 只从 `0.477` 变到 `0.471`
      - 但 `tail_neg__us_curve_10y2y_level__0` 从 `0.00` 直接压到 `-0.12`
      - `positive_window_avg_probability` 也被系统性压低 `0.237 -> 0.157`
  - 结论：
    - 这类“继续加深 `tail_neg__curve` 负权，再压 `USDJPY level`”的候选，
      已经可以直接离线 No-Go，不值得再跑 runtime review。

所以截至目前，主线判断进一步明确为：

1. `034053` 仍然是当前 family-hybrid 主线的最好候选；
2. `055652` 证明了 `20d threshold` 的软约束可以保留，但 `jpy_carry proxy` 重构单独拿出来还不够；
3. `061852` 证明了 `USDJPY raw interaction` 不能用“简单替换成 tail interaction”来处理；
4. `064930` 进一步证明“继续硬压 `curve/USDJPY`”的边际收益已经见顶，且会反向侵蚀危机窗口连续性；
5. `064040` 进一步证明这条线的更激进版本会直接把 `regional_banks` 的正窗口压穿；
6. 下一轮真正该优先追的，不再是继续围绕 `USDJPY` 单点做微调，而是：
   - `tail_neg__us_curve_10y2y_level__0`
   - `tail_pos__us_baa_10y_spread_level__2`
   - `us_usdjpy_level`
   - 以及这三组特征与 `20d threshold` 之间为什么仍能共同放大 `2023-02 / 2023-07` 常态窗口，
     但一旦继续下压又会吃掉 `regional_banks` 的 hedge 连续性。

为避免后面重复手工拼命令，当前这条主线也已经固定了两个标准入口：

- `just formal-candidate-window-audit <baseline> <candidate>`
  - 固定输出 `regional_banks`、`2023-02`、`2023-07` 三段窗口 compare；
- `just formal-candidate-feature-audit <baseline> <candidate>`
  - 固定输出 `20d threshold`、regime 概率分布，以及 `curve / spread / USDJPY / family context`
    的关键权重差异。

后续 family-hybrid 候选应先过这两层离线审计，再决定是否值得进入 `release-review-fast`。

## 13. 2026-06-04 补充：20d 联合审计后的下一轮训练硬约束

`034053 / 064930 / 064040` 这组三连 compare 已经把下一轮实验边界收得足够窄，后面不该再回到“继续堆 cap 看看会不会更好”的试法。

当前最关键的结论只有一条：`20d threshold` 不是 `064930 / 064040` 失去 `regional_banks`
连续性的主因，真正的问题是 `20d` 原始概率在 `positive_window` 被系统性压低了。

证据很直接：

- `034053 -> 064930`
  - `20d threshold: 0.477 -> 0.459`
  - `positive_window_avg_probability: 0.237 -> 0.200`
  - `regional_banks 20d hits: 27 -> 19`
- `034053 -> 064040`
  - `20d threshold: 0.477 -> 0.471`
  - `positive_window_avg_probability: 0.237 -> 0.157`
  - `regional_banks 20d hits: 27 -> 7`

也就是说，即便阈值没有明显抬高，`regional_banks` 还是被打穿；因此下一轮不能再把“lower threshold”
当成主要补救手段。

基于这组证据，下一轮训练必须遵守下面四条硬约束：

1. `tail_neg__us_curve_10y2y_level__0` 不允许继续往负方向加深
   - 当前最好候选 `034053` 的该权重是 `0.00`；
   - `064930` 压到 `-0.05` 后，`regional_banks` 连续性已明显回落；
   - `064040` 压到 `-0.12` 后，正窗口几乎被直接压穿。
   - 因此下一轮只允许两种做法：
     - 保持 `0`；
     - 或改成更温和、可解释的单调非负 / protected-context 语义。
   - 不再接受“继续负向加深，再靠 threshold 补回来”的候选。

2. `us_usdjpy_level` 不允许继续走“下压 base weight + 加强 interaction”的组合
   - `034053 -> 064930 / 064040` 里，`us_usdjpy_level` 都从 `0.382` 被压到 `0.22`；
   - 同时 `interaction__external_dimension_score__us_usdjpy_level`
     从 `0.552` 又被推到 `0.660 / 0.631`；
   - 结果不是只清掉 `2023-02 / 2023-07` 的误报，也连带压掉了 `regional_banks`
     真正需要的 `20d` 连续性。
   - 因此下一轮应把 `USDJPY` 更多迁到“高位 + 20d 变化/波动 + 外部维度确认”的
     proxy/context 结构，而不是继续用 blunt base suppression。

3. `tail_pos__us_baa_10y_spread_level__2` 目前不应被拉进新的硬压分实验
   - `034053 / 064930 / 064040` 三版里，这个权重都仍是 `0`；
   - 当前证据并没有证明“把它也做成新的负向 suppressor”会带来更好的权衡。
   - 如果下一轮要动它，只允许朝“单调非负 / protected-context 正向确认”的方向试，
     不允许把它变成第二条 `curve tail` 式的负向压分线。

4. `20d threshold` 只允许作为软策略约束，不再作为主修复手段
   - 后续候选仍要继续输出 threshold compare，但评审优先级必须改成：
     1. 先看 `positive_window_avg_probability` 是否被压坏；
     2. 再看 `regional_banks` 的 `20d hits / positive_window hit rate` 是否守住；
     3. 最后才看 `20d threshold` 是否需要小幅微调。
   - 凡是出现“原始概率被压平，但想靠 lowering threshold 补救”的候选，直接视为方向错误。

据此，下一轮 candidate 的离线 Go / No-Go 也同步收紧：

- `No-Go offline`
  - 再次出现 `tail_neg__us_curve_10y2y_level__0 < 0` 的明显加深；
  - 再次出现 `us_usdjpy_level` base weight 明显下压，同时 `USDJPY interaction`
    继续被放大；
  - `positive_window_avg_probability` 相比 `034053` 明显继续下滑，且 `regional_banks`
    连续性同步恶化。
- `Worth fast review`
  - 局部误报窗口继续收敛，但 `regional_banks` 连续性只出现可解释、可控的轻微回落；
  - 且候选至少在一个关键维度上有超过 `034053` 的明确证据。
- `Promote mainline`
  - 不只通过 fast review 护栏；
  - 还必须在不牺牲 `regional_banks` `20d` 连续性的前提下，实质超过 `034053`
    当前的 `actionable_precision 67.3%`。

## 14. 2026-06-04 补充：约束已下沉到训练与离线筛选

上面第 13 节不再只是文档结论，当前代码已经先把最确定的两条约束落地：

1. `apps/worker/src/model.rs`
   - `20d` 上的 `tail_neg__us_curve_10y2y_level__0` 已收紧为 `min=0 / max=0`，
     不再允许继续学成负向压分；
   - `us_usdjpy_level` 在 family-context 形态下已改成更窄的正向区间
     `0.30 ~ 0.40`；
   - `interaction__external_dimension_score__us_usdjpy_level` 也新增 `20d`
     上界 `0.58`，避免继续替代 base level 语义。

2. `scripts/formal-candidate-screen.ps1`
   - 已新增 `positive_window_avg_probability` 的硬性离线拦截；
   - 已新增“curve tail 加深 + USDJPY base 下压 + USDJPY interaction 放大”组合的
     `No-Go offline` 规则。

这意味着下一轮 family-hybrid 候选从训练开始就不该再自然长成 `064930 / 064040`
那条分支；后面真正还没做完的，是验证这些约束是否足够把下一版候选稳定推向
“保住 `regional_banks` 连续性，同时继续收口常态误报”的方向。

## 14.5 2026-06-06 补充：已新增可重复的语义审计入口

为了不再只靠文档结论人工复核，当前仓库已新增：

- `scripts/formal-candidate-semantics-audit.ps1`
- `just formal-candidate-semantics-audit <baseline> <candidate>`

这条入口会固定输出三层信息：

1. `20d threshold role`
   - `decision/base/final threshold`
   - `positive_window_avg_probability`
   - `normal_avg_probability`
   - `positive minus threshold gap`
2. `curve / bond-spread / USDJPY / jpy carry / rate_shock` 权重
   - 直接看 base weight、interaction、tail 与 family context 的差异
3. `guardrail coverage`
   - 哪些约束已经在训练层实现
   - 哪些还只是 `doc_only`
   - 真正要改代码时最小入口在哪

当前已用 `034053 -> 064930` 这条旧坏分支做过一次验证：

- 脚本能够直接指出 `curve tail negative suppression`、
  `USDJPY base-level 下压`、`USDJPY external interaction 放大` 这三类问题；
- 也能直接显示 `jpy_carry` 仍是 `proxy-only`，还不是正式主 family；
- 同时把当前仍未自动化落实的残余项缩到两类：
  - `USDJPY 20d change` 的语义迁移
  - `BAA spread` 不应变成新的 suppressor

这样下一轮再看候选时，不需要先翻设计文档猜“哪些已经在代码里，哪些还只是口头约束”。

## 15. 2026-06-04 补充：`081030` 已成为当前 family-hybrid 主线最好候选

在把第 13、14 节的联合审计结论真正下沉到训练约束和离线筛选后，下一版候选
`us_formal_family_hybrid_20260604T081030` 已完成离线、快速评审和正式评审三层复核。

结果可以直接归纳成三点：

1. 这版没有再走回 `064930 / 064040` 的坏分支；
2. `regional_banks` 的 `20d` 连续性没有再被继续侵蚀；
3. runtime `actionable_precision` 明显提高，但 `timely_warning_rate` 仍卡在 `10.0%`。

### 15.1 离线筛选结果

相对上一版主线候选 `034053`：

- `us_regional_banks_2023`
  - `20d hits: 27 -> 24`
  - `positive_window hit rate: 75% -> 75%`
  - `positive_window_avg_probability: 0.237 -> 0.239`
- `2023-02-01 ~ 2023-02-15`
  - `20d hits: 4 -> 1`
- `2023-07-01 ~ 2023-07-20`
  - `20d hits: 12 -> 6`

离线结论是 `worth_fast_review`，因为它已经证明：

- 常态窗口误报继续收敛；
- 但没有再通过压低 `positive_window` 原始概率来换结果。

### 15.2 Runtime review 结果

相对 active baseline `us_formal_interaction_tail_extmix10_20260602T061401`：

- fast review：
  - `timely_warning_rate 10.0% -> 10.0%`
  - `actionable_precision 54.8% -> 71.4%`
  - `longest_false_positive_episode_days 5 -> 5`
  - `guard_passed=true`
- `strict_rebuild` 正式 review：
  - `timely_warning_rate 10.0% -> 10.0%`
  - `actionable_precision 54.8% -> 71.4%`
  - `longest_false_positive_episode_days 5 -> 5`
  - `guard_passed=true`

因此当前 family-hybrid 主线的排序已经更新为：

1. `081030` 是当前最好候选；
2. `034053` 退居为“上一个稳定基线”；
3. `064930 / 064040` 继续保留为反例证据，不再作为主线方向。

### 15.3 这版仍然没有解决的问题

`081030` 的提升主要发生在“误报更少、动作命中更干净”，不是“更早知道危机会来”。

正式 review 里的 `Scenario-Level Backtests` 已经把问题说得很清楚：

- `2023 美国区域银行危机` 仍是唯一达到 `timely_to_timely` 的真实场景；
- `2000-2001 科网泡沫出清`、`1990-1993 美国银行与衰退压力` 等场景虽然存在
  `L2` 提前量，但始终没有进入 `L3 actionable`；
- runtime summary 也仍然显示：
  - `5d=usable_early_warning_separation`
  - `20d=usable_early_warning_separation`
  - `60d=separated_but_below_runtime_floor`

这说明下一轮的最高优先级已经不再是继续压 `20d` 误报，而是：

1. 直接解释为什么 `60d` 仍然“有分离、但过不了 runtime floor”；
2. 解释为什么 `2000-2001 / 1990-1993` 只能给出结构性 `L2`，却给不出动作级 `L3`；
3. 把训练目标、threshold policy 与 runtime posture 的优化重点迁到“恢复可执行提前量”。

### 15.4 2026-06-04 补充：当前主瓶颈已明确为 review/runtime 失配加 posture continuity

结合这轮新增的 `Focus Scenarios` 诊断，可以把 `081030` 当前卡住的位置说得更具体：

1. 不少真实危机场景并不是“模型没看见”，而是已经先命中了 runtime floor；
2. 但 `release review` 里的 strict `L3 actionable` 仍要求更硬的 `p20d / p60d` 门槛；
3. 即便概率已经够高，`1990-1993 / 2000-2001` 这类长窗结构性样本里，`posture/time_bucket`
   仍频繁停在 `normal`，导致始终凑不出 sustained `3/5` actionable hits。

这意味着 family-hybrid 主线的下一轮最高优先级应再收紧为：

1. 先补双口径 `release review` 诊断，显式区分 `strict-review-actionable` 和
   `runtime-actionable-potential`；
2. 再专项修 `1990-1993 / 2000-2001` 的 posture continuity，而不是继续把主要精力放在
   `20d` 短误报压缩；
3. 只有在这两条证据链补齐后，才继续决定是否需要新的 threshold / calibration / training 形态。

### 15.5 2026-06-06 补充：已新增可重复的 lead-time 审计入口

为了不再靠人工翻 `release-review` JSON，这条主线现在已经有固定离线入口：

- `scripts/formal-candidate-leadtime-audit.ps1`
- `just formal-candidate-leadtime-audit <baseline> <candidate>`

这条审计会直接把下列问题按固定格式摊开：

1. `timely_warning_rate / strict_actionable_point_count / runtime_floor_hit_count /
   actionable_precision` 是否同步改善；
2. `5d / 20d / 60d` 各 horizon 的 `runtime separation diagnosis`、
   `floor gap` 和 `threshold hit rate`；
3. 哪些历史场景已经有 `L2 lead time`，但仍没有 `L3 actionable`；
4. `Focus Scenarios` 里的 `runtime block mix` 是否仍以
   `review_gate_gap`、`posture_bucket_normal` 为主；
5. `Historical Audit Workstreams / Actions` 当前是否已经把问题收敛到
   `strict review vs runtime mapping` 或 `posture continuity`。

这样后续就不该再用“看起来 60d 已经有分离”这种口头判断推进，而应先跑这条脚本，确认：

- `60d` 到底还是 `separated_but_below_runtime_floor`；
- 还是已经穿过 runtime floor，但仍被 `strict gate` / `posture continuity`
  挡住；
- 以及 `1990-1993 / 2000-2001` 这类场景现在到底卡在 review gate，
  还是卡在 `prepare/months` 连续性。

相关设计与后续字段约束已单独沉淀在
[release-review-runtime-alignment-design.md](release-review-runtime-alignment-design.md)。

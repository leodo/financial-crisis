# 2023 区域银行危机 L3 修复设计

状态：`Draft`

最后更新：2026-06-04

## 1. 目标

这份文档只解决一个当前最关键的问题：

```text
为什么 family-hybrid 候选仍保留了 2023 美国区域银行危机的 L2=83d，
却丢掉了 baseline 的 L3=70d？
```

它不是新的总设计文档，而是一份**定向修复设计**。目标是把当前 release review 已定位出来的故障，从“结论”推进到“下一轮实验计划”。

## 2. 当前结论

基线对比：

- baseline：`us_formal_interaction_tail_extmix10_20260602T061401`
- candidate：`us_formal_family_hybrid_20260603T144814`

来自最新 fast review 的已确认事实：

1. 真正丢掉的 timely 样本只有一个：
   - `2023 美国区域银行危机`
2. baseline 与 candidate 都保留了：
   - `L2=83d`
3. 只有 baseline 保留了：
   - `L3=70d`
4. candidate 不是“从来没触发动作用条件”，而是：
   - pre-crisis actionable points 只有 `2`
   - baseline 同窗口有 `13`
5. backtest 的 `L3` 不是只看“有没有命中过一天”，而是要求：
   - `5` 天窗口里至少 `3` 次命中 sustained actionable signal

所以当前主故障已经收敛成：

- **不是** `60d` 首次越线完全消失；
- **不是** 单纯 runtime `prepare_p60d` 太高；
- **而是** candidate 的 `20d / posture context / actionable continuity` 无法持续，导致 `3/5 sustained hits` 失败。

## 3. 关键证据

从 `Focus Scenarios` 逐日复盘可直接读出：

### 3.1 2022-12-28 只出现短暂 prepare 命中

- baseline：`p_20d=49.4%`，`p_60d=72.1%`，`prepare/months`，actionable=`yes`
- candidate：`p_20d=19.6%`，`p_60d=70.3%`，`prepare/months`，actionable=`yes`

说明 candidate 不是完全没有动作级能力。

### 3.2 2022-12-31 起 candidate 的 20d 连续性开始塌缩

- baseline：`p_20d=48.6%`，仍保持 actionable
- candidate：`p_20d=17.6%`，已跌破 `18%`，同日 actionable=`no`

也就是说 candidate 在第一次 prepare 之后，没有把可执行窗口维持住。

### 3.3 2023-02-20 ~ 2023-02-27 baseline 已连续 hedge，candidate 大量退回 normal

代表性日期：

- `2023-02-20`
  - baseline：`hedge / weeks / hedge_p20d_context / actionable=yes`
  - candidate：`normal / normal / — / actionable=no`
- `2023-02-21`
  - baseline：`p_20d=77.4%`
  - candidate：`p_20d=42.2%`
- `2023-02-22`
  - baseline 与 candidate 都进入 `hedge`
- `2023-02-23 ~ 2023-02-27`
  - baseline 持续 `hedge`
  - candidate 再次大多退回 `normal`

这说明 candidate 不是“完全不会 hedge”，而是**只会点状命中，不会形成连续区间**。

## 4. 非目标

这轮修复明确不做以下事情：

1. 不继续做“放宽 runtime `prepare_p60d`”类单点阈值微调；
2. 不继续做同类 `60d positive/cooldown` 权重盲调；
3. 不把所有问题都归咎于 overlay 开关；
4. 不在还没看清 `2023` 样本根因前继续大范围训练更多 family 变体。

这些路径已经被前序实验基本证伪，继续投入只会重复消耗算力和 review 成本。

## 5. 修复假设矩阵

下一轮实验只允许围绕以下四类假设展开：

### H1：20d 概率连续性被 family-hybrid 压扁

症状：

- candidate 在关键窗口里 `p_20d` 经常低于 baseline 很多；
- 明明 `p_60d` 仍不低，但 `p_20d` 维持不住；
- 导致 `hedge_p20d_context` 无法持续触发。

需要验证：

- family-hybrid 的 `20d` base head / overlay contribution 是否在 `2022-12-31`、`2023-02-20~02-27` 持续压低了最终 `p_20d`；
- 是 raw model 压低，还是 calibration 压低。

### H2：prepare/hedge posture clause 的连续性不足

症状：

- candidate 只在单日进入 `prepare` 或 `hedge`，随后快速回落为 `normal`；
- 说明不是单个概率点错，而是 posture context 不能维持。

需要验证：

- `prepare_structural_downgrade`
- `prepare_p60d_structural`
- `hedge_p20d_context`

这三类 clause 在 baseline 与 candidate 中各自的连续天数、命中段长度、段间断裂位置。

### H3：family proxy / overlay 在 2023 场景上的时间定位有偏移

症状：

- candidate 可能学到了“2023 属于 systemic credit / banking stress”，但没学到“哪几天最该进入动作窗口”；
- 结果是结构上有响应，时机上不连续。

需要验证：

- 2023 场景对应的 family gate、overlay contribution、family feature 值是否在关键窗口稳定激活；
- 是否存在 gate 激活了，但 final `p_20d` 仍被 base head 或 calibration 稀释。

### H4：当前训练目标没有直接奖励 sustained actionable continuity

症状：

- 模型能偶尔打到一个高点；
- 但没有学到“关键窗口里连续几天都要维持在动作区间”。

需要验证：

- 当前目标函数是否只奖励单点命中；
- 是否缺少“pre-crisis 连续段”级别的监督或 regularization。

## 6. 必须先补的诊断产物

下一轮编码/训练前，先产出以下诊断表，不允许跳过：

### 6.1 训练样本切片

对 `2023 美国区域银行危机` 导出 `2022-12-01 ~ 2023-03-15` 的训练/校准/评估样本切片，至少包含：

- `as_of_date`
- `split_name`
- `scenario_family`
- `days_to_primary_crisis_start`
- `regime_20d`
- `regime_60d`
- `prepare_episode_label`
- `hedge_episode_label`
- `primary_action_level`
- 关键 family proxy features

当前已经补上命令入口：

- `just formal-dataset-slice us_regional_banks_2023 2022-12-01 2023-03-15`
- 默认导出到 `artifacts/research/formal-dataset-slices/`

首轮实跑结果：

- 当前主数据集 `formal_v1_main_1990_daily:20260601T172759` 在该窗口导出了 `68` 行、`15` 个特征；
- 这批样本全部落在 `evaluation` split；
- 时间范围实际从 `2023-01-07` 开始，到 `2023-03-15` 结束；
- `regime_60d` 基本一直处于 `positive_window`，而 `regime_20d` 在 `normal / pre_warning_buffer / positive_window` 之间切换；
- `2023-02-20`、`2023-02-27` 已带 `hedge_episode_label=1`，说明训练标签层面并不是完全没给出动作上下文，后续重点仍应放在 runtime 概率连续性和 family 特征如何影响 `20d` 抬升。
- 按该切片内部的标准化均值差做快速扫描，`us_usdjpy_change_20d`、`us_usdjpy_level` 在 `hedge_episode_label=1` 窗口中都排在前列，说明“日元套息/美元兑日元”至少值得继续保留在后续 family feature 根因复盘的第一批重点变量里；但这还只是探索性信号，不等于已证明因果。

目的：

- 确认 2023 样本在训练数据里到底被怎么标；
- 确认关键窗口是否被错误归入 normal/cooldown。

### 6.2 Baseline vs candidate 概率拆解

对同一时间窗导出：

- raw `p_20d / p_60d`
- calibrated `p_20d / p_60d`
- base head contribution
- family overlay contribution
- final probability

目的：

- 判定是 base、overlay 还是 calibration 在拉低连续性。

当前已经补上离线命令入口：

- `just formal-probability-slice us_formal_interaction_tail_extmix10_20260602T061401 us_regional_banks_2023 2022-12-01 2023-03-15`
- `just formal-probability-slice us_formal_family_hybrid_20260603T144814 us_regional_banks_2023 2022-12-01 2023-03-15`
- `just formal-probability-compare us_formal_interaction_tail_extmix10_20260602T061401 us_formal_family_hybrid_20260603T144814 us_regional_banks_2023 2022-12-01 2023-03-15`
- 默认导出到 `artifacts/research/formal-dataset-slices/`
- compare 默认导出到 `artifacts/research/formal-probability-compares/`

首轮实跑结果：

- baseline：`us_formal_interaction_tail_extmix10_20260602T061401`
  - `20d` 峰值 `0.800457`，出现在 `2023-02-22`；
  - `20d` 有 `13` 个日期超过本版 decision threshold `0.522`，最早是 `2023-02-20`；
  - `20d` 在 `68` 条样本里有 `67` 条高于 `10%`，明显偏宽；
  - `60d` 全窗口都高于 `10%`，但没有一天超过本版 threshold `0.656`，说明 `60d` 更像高背景值而非真正可执行的动作触发；
  - overlay contribution 全为 `0`，因为这版 bundle 没启用 family overlay。
- candidate：`us_formal_family_hybrid_20260603T144814`
  - `20d` 峰值只有 `0.471417`，出现在 `2023-03-15`；
  - `20d` 只有 `1` 个日期超过本版 decision threshold `0.46`，而且已经晚到危机爆发后；
  - `20d` 只有 `28/68` 条样本高于 `10%`，说明主问题是 `20d` 主体概率整体被压平，而不是只差一点持续性；
  - `60d` 同样全窗口高于 `10%`，但没有一天超过 threshold `0.661`，说明 `60d` 的 cooldown bleed / 高背景问题依旧存在；
  - `20d` overlay delta 仅在 `-0.010498 ~ +0.003177` 之间，`60d` overlay 关闭后贡献为 `0`，说明 candidate 失败并不是 overlay 把概率“打坏了”，而是 base `20d` head + calibration/threshold 组合本身没有把 pre-warning 抬起来。
- 关键日期对比进一步确认了这一点：
  - `2023-02-20`：baseline `p20d=0.549744`，candidate `p20d=0.202496`；
  - `2023-02-27`：baseline `p20d=0.559458`，candidate `p20d=0.186702`；
  - 这两天训练标签都已有 `hedge_episode_label=1`，所以“提前量丢失”不是标签没给，而是 candidate 的 `20d` 概率头没有跟上。

结论：

- `family_hybrid` 的主故障已经可以收敛成一句话：`20d` 主体概率在 pre-warning / hedge 窗口被压得太低；
- `60d overlay` 不是主因，继续只调 overlay 权重不会解决 `2023 regional banks`；
- 下一轮应直接审计 `20d` base head 的特征、权重、calibration 与 threshold selection，同时单独处理 `60d` cooldown bleed。

继续下钻到 base-head feature contribution 后，又拿到一层更具体的证据：

- 在 `2023-02-20`，candidate `20d` raw probability 只有 `0.205`，而 baseline 是 `0.550`；candidate 最大负贡献来自：
  - `tail_neg__us_curve_10y2y_level__0 = -2.397`
  - `interaction__us_curve_10y2y_level__us_fed_funds_level = -0.803`
  - `us_usdjpy_change_20d = -0.425`
  - `us_usdjpy_level = -0.110`
- 在 `2023-02-27`，candidate `20d` raw probability 只有 `0.191`，而 baseline 是 `0.559`；candidate 最大负贡献进一步扩大为：
  - `tail_neg__us_curve_10y2y_level__0 = -2.649`
  - `interaction__us_curve_10y2y_level__us_fed_funds_level = -0.848`
  - `us_usdjpy_change_20d = -0.607`
  - `us_usdjpy_level = -0.125`
- 同样两天里 baseline 也承受 `curve inversion` 与 `USDJPY 20d change` 的负贡献，但差别在于：
  - baseline 的 `tail_neg__us_curve_10y2y_level__0` 权重更轻；
  - baseline 的 `us_usdjpy_level` 是小幅正贡献，而 candidate 把它学成了负贡献；
  - 这说明 candidate 不是“没看到日元和利差风险”，而是把这些变量学成了过度惩罚 `20d` 提前量的方向。

这组证据把下一轮改模边界进一步收紧为：

1. 重点审计 `curve inversion tail`、`USDJPY level`、`USDJPY 20d change` 在 `20d` head 里的符号与权重；
2. 如无额外证据支撑，不要再继续扩大 `tail_neg__us_curve_10y2y_level__0` 这类负 tail 项的惩罚强度；
3. 把 `jpy carry` 相关变量视为“高优先级人工复核特征”，必要时补单调约束、family proxy 重构或更细的 episode-native target。

新 compare 产物又补了两条很有用的“落地排序信息”：

- `20d` 差异最大的日期不是随机散点，而是集中在 `2023-02-21 ~ 2023-02-27` 这段真正的 pre-warning / hedge 窗口，其中：
  - `2023-02-24 / 2023-02-25 / 2023-02-26` 的 `candidate - baseline p20d` 都约为 `-0.40`
  - `2023-02-27` 为 `-0.373`
  - 这些天都带 `hedge_episode_label=1`
- 在这些“差得最厉害”的日期里，影响最大的特征差分仍然稳定集中在：
  - `tail_neg__us_curve_10y2y_level__0`
  - `interaction__external_dimension_score__us_usdjpy_level`
  - `tail_abs_pos__us_usdjpy_change_20d__4`
  - `family_context__rate_shock__external_dimension_score`
- 同时 compare + slice 也确认了另一件关键事实：
  - 在这段 `2023 regional banks` 窗口里，两版的 `20d / 60d` 都基本是 `raw = calibrated`
  - baseline `20d` / `60d` 没有额外 calibration 压缩；
  - candidate `20d` 只在 overlay 后从 `0.205 -> 0.202`、`0.191 -> 0.187` 这种极小幅度变化；
  - 这意味着当前故障几乎不在 calibration 层，而是在 `20d` base head 自身和 decision threshold policy。

这说明下一轮不该再平均用力，而应该优先做两类定点修复：

1. 先治理 `20d` 上与 `curve inversion`、`USDJPY` 相关的过强负 tail / 交互；
2. 再单独检查 `rate_shock` family context 是否在 `2023` 这类银行流动性样本上起到了错误的替代解释。

最新一轮 compare 聚合摘要又把问题收得更窄了一些：

- 整个窗口里，candidate 相对 baseline 的平均 `20d` 概率差是 `-0.215`；
- 只看 `regime_20d=positive_window` 的 `20` 条样本，平均 `20d` 差扩大到 `-0.306`；
- 只看 `hedge_episode_label=1` 的 `15` 条样本，平均 `20d` 差进一步扩大到 `-0.344`；
- `positive_window` 子窗口里，baseline `20d` 命中率是 `40%`，candidate 是 `0%`。

这说明 candidate 的问题不是“阈值边缘偶发漏报”，而是：

- 一旦进入真正应当抬升 `20d` 的窗口，candidate 会系统性少给大约 `20 ~ 35` 个百分点；
- 且这种系统性压低在 `hedge` 标签样本里更严重。

同一份聚合摘要还给出了整个窗口里反复出现的 top feature delta：

- overall top 3：
  1. `tail_neg__us_curve_10y2y_level__0`
  2. `family_context__rate_shock__external_dimension_score`
  3. `us_curve_10y2y_level`
- hedge 窗口 top 3：
  1. `tail_neg__us_curve_10y2y_level__0`
  2. `family_context__rate_shock__external_dimension_score`
  3. `interaction__external_dimension_score__us_usdjpy_level`

所以，下一轮真正该优先排的顺序已经比较明确：

1. 先压低 `20d` 上 `curve inversion tail` 的过度惩罚；
2. 再复核 `rate_shock family context` 是否把银行流动性风险错误解释成了别的宏观场景；
3. 同时保留 `USDJPY / jpy carry` 为高优先级特征复核对象，因为它在 hedge 窗口里持续排在前列。

### 6.3 2026-06-04 定向修复实验结果

基于上面这组证据，我先做了一轮很小、但方向明确的约束实验：

1. 先对 `20d/60d` 同时施加 derived tail 单调约束，得到候选 `us_formal_family_hybrid_20260603T191209`；
2. 然后把该约束收窄成 **只作用于 `20d`**，得到候选 `us_formal_family_hybrid_20260603T192249`。

两轮结果的差异非常清楚：

- `191209`：
  - `20d hits` 从 baseline 的 `13` 提到 `29`
  - 但 `60d hits` 从 `0` 直接膨胀到 `62`
  - 说明“把 derived tail 约束同时扩到 `60d`”会把长窗抬得过宽
- `192249`：
  - 相比 baseline，`20d hits` 仍保持 `13 -> 29`
  - `positive_window` 子窗口命中率从 `40% -> 80%`
  - `60d hits` 保持 `0 -> 0`
  - `candidate max p60d` 回到 `0.596`，不再重现 `191209` 的 `0.916`

这说明当前最重要的一条结论已经可以写死：

- **derived tail 单调约束对 `20d` 是有效修复；**
- **但这条约束不能直接平移到 `60d`。**

我随后又对 `192249` 补跑了两层运行态证据：

- `just release-review-fast us_formal_family_hybrid_20260603T192249`
- `just release-review us_formal_family_hybrid_20260603T192249`

两轮 review 结论一致：

- `timely_warning_rate`: `10.0% -> 10.0%`
- `actionable_precision`: `55.9% -> 57.9%`
- `longest_false_positive_episode_days`: `5 -> 7`
- `guard_passed=true`

也就是说，这版候选已经不再是“局部切片看起来不错”，而是：

- 在当前最关键的 `2023 regional banks` 场景上恢复了 `20d` 提前量；
- 没有再把 `60d` 弄炸；
- 对当前 active baseline 的正式 `strict_rebuild` review 也没有触发 bundle-level guard 回归。

但这还不等于“可以直接晋升”为新的 active release。当前仍有两件事没闭环：

1. `longest_false_positive_episode_days` 从 `5` 增到 `7`，需要继续下钻是哪几个场景、哪类 posture clause 在拉长误报段；
2. 这次修复证明了 `20d` 方向对了，但还没有证明 `mixed_systemic`、`jpy_carry`、`rate_shock context` 这些 family 结构已经到位。

所以这轮实验后的正确结论不是“family-hybrid 已完成”，而是：

- 当前主故障已经从“20d 连续性严重塌缩”收敛到“`20d` 已修复、`60d` 未恶化、但误报段和 family 语义仍需人工复核”；
- 下一步应继续围绕 `20d` 特征符号/权重、`rate_shock` 上下文、`USDJPY/jpy carry` 代理语义做定点修复，而不是回头重开一轮大范围 `60d` 权重盲调。

### 6.4 Sustained hit 证据表

把 backtest `L3` 口径显式拆成：

- 每日 actionable flag
- rolling 5d hit count
- 是否达到 `3/5`
- 第一次达到 `3/5` 的日期

目的：

- 把“为什么 baseline 有 L3、candidate 没有”直接变成机器可读证据；
- 避免后面继续用“感觉差一点”讨论。

## 7. 下一轮实验边界

### 7.1 允许做的改动

1. 针对 `20d` 动作连续性补目标函数或 regularizer；
2. 针对 `hedge_p20d_context` / `prepare_structural_downgrade` 补持续性约束；
3. 调整 2023 family proxy 的构造或 gate 逻辑；
4. 在 release review 中新增 sustained-hit 证据输出（已完成，后续作为固定诊断面板保留）。

### 7.2 暂时不允许做的改动

1. 继续只调 runtime floor；
2. 再训一批没有额外诊断支撑的新 family overlay 组合；
3. 同时大改 5d/20d/60d 所有目标定义；
4. 为了追求 `2023` 单样本命中而接受明显更差的 false positive 结果。
5. 在没有新增证据前，把 derived tail 单调约束再次一把扩回 `60d`。

## 8. Go / No-Go 条件

下一轮候选要继续推进，至少满足：

1. `2023 美国区域银行危机` 恢复 sustained `L3`，而不是只恢复单点命中；
2. `timely_warning_rate` 不低于当前 active baseline；
3. `actionable_precision` 不低于当前 active 的 `90%`；
4. 不再出现通过大量 runtime threshold 放宽换来的假性改善；
5. release review 能直接输出 sustained-hit 证据，说明恢复原因不是黑盒猜测。

若只出现以下情况，则一律判定 No-Go：

- 只是 `prepare` 次数变多；
- 只是 `months` bucket 变多；
- 只是某一天又重新命中一次 actionable；
- 但 `3/5 sustained hits` 仍没恢复。

## 9. 与后续主线的关系

这份设计不是要把系统永久绑定到 `2023` 单样本，而是承认当前现实：

- 真正的 timely 样本极少；
- `2023 regional banks` 是目前最关键的“可执行提前量”样本；
- 如果连这条样本都恢复不了，继续讨论更复杂的 family overlay 扩展没有意义。

因此，下一轮正确顺序应是：

1. 先用这份设计把 `2023` 根因拆明；
2. 再决定是修 `20d continuity`、family proxy 还是 actionability objective；
3. 再基于修复后的结构扩展到更多家族与场景。

## 10. 实施入口

本设计对应的当前活跃任务见：

- [危机概率评估设计 TODO](../roadmap/crisis-probability-design-todo.md)
- [family overlay bundle schema 设计](family-overlay-bundle-schema-design.md)

## 11. 2026-06-04 最新进展

基于这份诊断设计，family-hybrid 主线已经又向前走了一步，但还没有解决 `L3`：

1. `20d-only derived tail` 约束把 `regional_banks` 的 `20d` 命中从 `13 -> 29`，并确认 `60d` 不能套用同一规则；
2. 后续再对“明确应同向放大风险”的 interaction 加 sign constraint，得到当前最好候选 `us_formal_family_hybrid_20260604T034053`；
3. 这版正式 strict review 已达到：
   - `timely_warning_rate 10.0% -> 10.0%`
   - `actionable_precision 54.8% -> 67.3%`
   - `longest_false_positive_episode_days 5 -> 5`
4. 同时局部 compare 已确认：
   - `2023-02` 非正例窗口平均抬升已收敛到 `+0.085`
   - `2023-07` 非正例窗口额外 `20d hits` 从 `17` 降到 `13`
   - `regional_banks` 正窗口 `20d hits` 仍保持 `28`
5. 最新候选 `us_formal_family_hybrid_20260604T064930` 又把 `2023-02 / 2023-07` 的常态误报继续压下去了：
   - 相比 `034053`，`2023-02-01 ~ 2023-02-15` 的 `20d hits` 从 `4` 压到 `1`
   - `2023-07-01 ~ 2023-07-20` 的 `20d hits` 从 `12` 压到 `2`
   - 但 `regional_banks` 的 `20d` 连续性也回落到 `20d hits 27 -> 19`、`positive_window hit rate 75% -> 60%`
   - fast review 只有 `actionable_precision 54.8% -> 65.1%`，仍低于 `034053` 的 `67.3%`
6. 更激进的同方向候选 `us_formal_family_hybrid_20260604T064040` 已可直接离线 No-Go：
   - `2023-02-01 ~ 2023-02-15` 的 `20d hits` 从 `4` 直接压到 `0`
   - `2023-07-01 ~ 2023-07-20` 的 `20d hits` 也从 `12` 压到 `0`
   - 但 `regional_banks` 的 `20d` 连续性同步塌到 `20d hits 27 -> 7`、`positive_window hit rate 75% -> 25%`
   - 特征审计也确认这不是阈值单独导致：`20d threshold` 只从 `0.477` 变到 `0.471`，真正发生的是 `tail_neg__us_curve_10y2y_level__0` 从 `0.00` 压到 `-0.12`，连带把 `positive_window_avg_probability` 从 `0.237` 压到 `0.157`

这说明当前主问题已经继续收窄：

- 不再主要是 `curve/fed-funds` 方向学错；
- 也不再主要是 `rate_shock family context` 过宽；
- 现在更准确地说，是 `curve inversion / baa spread / USDJPY level` 与 `20d threshold`
  的耦合仍然过强；继续硬压它们虽然能清常态误报，但会直接吃掉 `regional_banks`
  的 `20d` 连续性。

所以这份文档的下一轮实施重点也应随之更新：

1. 优先复盘 `tail_neg__us_curve_10y2y_level__0`、`tail_pos__us_baa_10y_spread_level__2`、
   `us_usdjpy_level` 为什么会一起主导 `2023-02 / 2023-07` 的常态窗口；
2. 判断这几组特征当前是该保留“直接压到 0”的硬约束，还是改成更温和的单调正权；
3. 在不牺牲 `regional_banks` 连续性的前提下，再评估 `20d threshold` 是否能用更稳妥的软约束，
   而不是继续靠硬压系数来换干净窗口。

当前这条复盘线也已经固定了两个标准离线入口，后续候选应先跑它们，再决定是否值得进 runtime review：

- `just formal-candidate-window-audit <baseline> <candidate>`
  - 固定输出 `regional_banks`、`2023-02`、`2023-07` 三段窗口的 compare；
- `just formal-candidate-feature-audit <baseline> <candidate>`
  - 固定输出 `20d threshold`、regime 概率分布，以及 `curve / spread / USDJPY / family context`
    的关键权重差异。

## 12. 2026-06-04 补充：面向 `regional_banks` 的下一轮训练约束

这份文档现在可以再往前收一层，不再只是说“要继续复盘”，而是直接给出下一轮训练边界。

结合 `034053 / 064930 / 064040` 的联合审计，当前已经能确认：

1. `regional_banks` 丢失连续性的直接触发器，不是 `20d threshold` 本身；
2. 真正先坏掉的是 `20d` 原始概率在 `positive_window` 被压平；
3. 造成这次压平的主组合，是：
   - `tail_neg__us_curve_10y2y_level__0`
   - `us_usdjpy_level`
   - `interaction__external_dimension_score__us_usdjpy_level`
   - 以及更高的 `us_fed_funds_level` 压力共同叠加。

这组证据对应到 `regional_banks` 场景，可以直接转成下面的实施约束。

### 12.1 下一轮允许做的事

1. 继续审计 `curve / spread / USDJPY` 三组特征，但重点放在“语义重构”而不是“继续压系数”
   - `curve tail` 允许改成更温和、可解释的单调非负 / protected-context 语义；
   - `USDJPY` 允许继续迁往 `jpy carry proxy/context`，但必须带“高位 + 变化 + 外部确认”
     的确认逻辑；
   - `20d threshold` 允许保留软约束与 soft penalty，但只能作为次级策略层。

2. 继续保留 `regional_banks` 作为第一优先受保护场景
   - 后续候选先看 `regional_banks` 的 `20d hits`、`positive_window hit rate`
     和 `positive_window_avg_probability`；
   - 这些指标先过线，再看 `2023-02 / 2023-07` 的常态误报是否继续收口。

### 12.2 下一轮不允许再做的事

1. 不允许再把 `tail_neg__us_curve_10y2y_level__0` 往更负方向压
   - `034053=0.00`、`064930=-0.05`、`064040=-0.12` 的序列已经足够说明问题；
   - 这条线每往前走一步，`regional_banks` 的 `20d` 连续性就更差。

2. 不允许再把 `us_usdjpy_level` 的 base weight 下压到 `0.22` 左右，同时继续增强
   `interaction__external_dimension_score__us_usdjpy_level`
   - 这类组合看起来能清 `2023-02 / 2023-07`，但会一起打掉真正的预警窗口。

3. 不允许再尝试“raw probability 已经被压坏，再靠 lowering threshold 补回来”
   - `064930` 与 `064040` 已经证明，即使 threshold 仍在 `0.46 ~ 0.47`，
     `regional_banks` 也一样会丢连续性；
   - 所以后续修复顺序必须先改 feature semantics，再谈 threshold policy。

### 12.3 后续场景级 Go / No-Go 规则

1. 只要离线 compare 显示 `positive_window_avg_probability` 相比 `034053` 继续明显下滑，
   同时 `regional_banks` `20d hits` / `positive_window hit rate` 也同步恶化，
   该候选直接 `No-Go offline`。
2. 如果候选只是轻微牺牲 `regional_banks` 连续性，但确实显著压掉了常态误报，
   可以保留为 `worth_fast_review`，但不能直接晋升主线。
3. 只有当候选同时满足下面两点，才值得争取晋升：
   - `regional_banks` 连续性至少不弱于 `034053`；
   - runtime 指标实质超过 `034053` 当前的 `actionable_precision 67.3%`。

## 13. 2026-06-04 补充：`081030` 证明联合审计约束方向正确

在第 12 节约束真正落到训练与离线筛选后，新候选
`us_formal_family_hybrid_20260604T081030` 已给出一组更干净的证据：

1. 相对 `034053`，`regional_banks` 的 `20d` 连续性没有继续塌：
   - `20d hits: 27 -> 24`
   - `positive_window hit rate: 75% -> 75%`
   - `positive_window_avg_probability: 0.237 -> 0.239`
2. 同时 `2023-02 / 2023-07` 的常态误报继续收口：
   - `2023-02 hits: 4 -> 1`
   - `2023-07 hits: 12 -> 6`
3. 正式 `strict_rebuild` review 已进一步确认：
   - `timely_warning_rate 10.0% -> 10.0%`
   - `actionable_precision 54.8% -> 71.4%`
   - `longest_false_positive_episode_days 5 -> 5`
   - `guard_passed=true`

这组结果足以说明两件事：

- 第 12 节那套“不要再走 `curve tail negative + USDJPY blunt suppression`”的约束，
  方向是对的；
- 当前 `regional_banks` 修复主线已经从“守住连续性”进入下一阶段：在不反噬
  `regional_banks` 的前提下，去追回其他真实危机场景的动作级提前量。

因此，这份文档后续的角色也要更新：

1. `regional_banks` 仍然是第一优先受保护场景；
2. 但它不再是唯一目标函数；
3. 下一轮需要和 `2000-2001 / 1990-1993` 的 `L3 actionable` 缺口联合看，
   不能再只围绕 `20d` 连续性单点优化。

进一步说，这份文档现在应该承担的是“保护 `regional_banks` 不再被修坏”的边界约束，
而不是单独定义整个主线优先级。

新的主线判断已经比较清楚：

1. `regional_banks` 仍是必须守住的唯一真实 timely 样本；
2. 但 `timely_warning_rate=10%` 的主瓶颈，已经不再只是它的 `20d` 连续性；
3. 更大的问题是 `release review` strict gate 与 runtime floor 不一致，以及
   `1990-1993 / 2000-2001` 长窗结构性样本的 posture continuity 失效。

因此后续围绕这份文档继续做实验时，默认前提应是：

1. 不允许为修 `regional_banks` 再回到大范围 `20d threshold` 微调；
2. 要把 `regional_banks` 诊断结果和
   [release-review-runtime-alignment-design.md](release-review-runtime-alignment-design.md)
   的双口径审计一起看；
3. 只有在不牺牲 `regional_banks` 的前提下，才允许继续向 `1990-1993 / 2000-2001`
   追回动作级提前量。

# Release Review Runtime 对齐设计

状态：`Draft`

最后更新：2026-06-04

## 1. 问题

当前 `release review` 里的 `L3 actionable` 判定，和 API runtime 真正在用的动作阈值，不是同一套口径。

这会带来一个很实际的问题：

- 模型可能已经很早命中了 runtime floor；
- 但 `release review` 仍把它记成“没有动作级预警”；
- 于是后续训练和调参，容易围绕错误目标继续优化。

## 2. 已确认的证据

基于 `us_formal_interaction_tail_extmix10_20260602T061401`
vs
`us_formal_family_hybrid_20260604T081030`
的复核结果，当前证据已经足够明确。

### 2.1 `2000-2001 科网泡沫出清`

- baseline 首次 `runtime floor hit without L3`：`1999-12-11`
- candidate 首次 `runtime floor hit without L3`：`1999-12-20`

说明：

- 长窗/中窗概率很早就已经超过 runtime floor；
- 问题不是“模型完全没看到风险”；
- 而是 `release review` 的严格门槛仍要求：
  - `p20d >= 18%`
  - `p60d >= 45%`

同时，在后续不少高概率日期里，诊断又变成：

- `posture/bucket stayed normal`

也就是说，这个场景同时暴露出两类问题：

1. `runtime floor` 与 `strict review gate` 不一致；
2. posture 连续性不足。

### 2.2 `1990-1993 美国银行与衰退压力`

- baseline / candidate 首次 `runtime floor hit without L3` 都是：`1990-07-03`
- 两版都有 pre-crisis actionable points：`3`
- 但都没有形成 sustained `3/5` 命中，因此 `L3` 仍为 `—`

这个场景说明：

- 不只是 candidate 有问题；
- baseline 自己也存在“概率已经很高，但 posture 大部分日期仍停在 normal，只出现零星 prepare/months 脉冲”的问题；
- 所以 `timely_warning_rate=10%` 的主瓶颈，不是单纯 candidate 退化，而是整个 formal main/posture policy 在长窗结构性危机上都不够稳定。

## 3. 根因拆分

当前 bottleneck 已经可以明确拆成两类：

### A. Review / Runtime 口径失配

`release_review_is_actionable_warning_point(...)` 当前使用硬编码严格门槛：

- `p20d >= 18%`
- `p60d >= 45%`

而 formal main 的 runtime policy 是动态且更低的：

- `prepare_p60d` 由 bundle threshold 与 runtime floor 联合决定；
- `hedge_p20d`、`defend_p5d` 也是同样机制；
- formal main runtime floor 明显低于 review 里的硬编码 `18% / 45%`。

结果就是：

- runtime 视角看“已经值得开始准备”；
- strict review 视角仍把它记成“没有动作级预警”。

### B. Posture continuity failure

在 `1990-1993`、`2000-2001` 这类场景里，经常出现：

- `p20d / p60d` 已经很高；
- 但 `posture = normal`、`time_bucket = normal`；
- 或者只短暂出现单点 `prepare`，随后又掉回 `normal`；
- 最终无法形成 `3/5 sustained hits`。

这说明仅仅继续调概率阈值，不足以恢复真正可执行的 `L3` 提前量。

## 4. 设计决策

### 4.1 不直接替换现有 strict review 指标

当前 `timely_warning_rate`、`actionable_precision` 已经被大量历史候选和文档引用。

所以这轮不应静默修改它们的定义。

### 4.2 增加双口径输出

后续 `release review` 应显式区分：

1. `strict-review-actionable`
   - 保留当前严格门槛
   - 用于正式 go/no-go 护栏
2. `runtime-actionable-potential`
   - 只问“是否已命中 runtime floor，且是否被 posture / continuity 挡住”
   - 用于根因分析与训练方向判断

### 4.3 Focus Scenarios 必须覆盖“有 L2、无 L3”的真实场景

即使 baseline 和 candidate 的最终 outcome 同为 `missed_to_missed`，只要存在：

- `L2` 提前量；
- 但没有 `L3 actionable`

就必须进入 `Focus Scenarios`。

否则像 `1990-1993` 这种关键结构性失败样本会被静默漏掉。

## 5. 实施方案

### 5.1 已完成

- `Focus Scenarios` 已新增：
  - `first runtime-floor hit without L3`
  - point-level `actionable_diagnostic`
- `Focus Scenarios` 选择逻辑已扩展到：
  - `lead_time_days.is_some() && actionable_lead_time_days.is_none()`
- `release review` 输出已新增结构化双口径字段：
  - point-level:
    - `strict_review_actionable`
    - `runtime_floor_hit`
    - `runtime_actionable_block_reason`
  - summary-level:
    - `strict_actionable_point_count`
    - `runtime_floor_hit_count`
- markdown / 控制台 summary 已同步展示上述双口径计数与 block reason。

### 5.2 下一步文档与评审对齐

1. 在 backtest / guardrail 文档中明确：
   - 当前正式护栏仍以 strict review 为准；
   - 但训练主线优先修复 `runtime floor 已命中、却长期不能形成 L3` 的场景。
2. 让正式 `strict_rebuild release review` 产物持续携带这组双口径字段，避免后续新报告回退成只有自由文本。

### 5.3 posture 审计主线

在 `1990-1993 / 2000-2001` 上继续专项审计：

1. 为什么高概率日期仍保持 `normal`；
2. 为什么 `prepare/months` 只能单点脉冲出现；
3. 为什么 `3/5 sustained` 无法建立；
4. 哪些 clause / blocker / score confirmation 在长期结构性危机场景上过严。

## 6. 非目标

这份设计当前不做：

1. 不直接改写正式 `timely_warning_rate` 定义；
2. 不把 runtime floor 直接等价成 `L3 actionable`；
3. 不继续把“压 20d 短误报”当作最高优先级。

## 7. 当前结论

到 `2026-06-04` 为止，可以把问题说成人话：

- `081030` 已经是当前 family-hybrid 最干净的候选；
- 但它没有把系统变成“更早知道危机会来”；
- 真正卡住 `timely_warning_rate=10%` 的，不是一个简单阈值，而是：
  - `review gate` 比 runtime policy 严得多；
  - 长窗结构性危机上的 `posture` 连续性又不够。

因此，后续主线应先修：

1. `release review` 的双口径诊断；
2. `1990-1993 / 2000-2001` 的 posture continuity；
3. 之后再决定是否调整阈值、目标函数或训练形态。

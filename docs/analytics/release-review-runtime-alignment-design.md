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
- `Runtime block mix` 显示：
  - `review_gate_gap: baseline=14 | candidate=1`
  - `posture_bucket_normal: baseline=2 | candidate=0`
  - `review_l3_gate_not_satisfied: baseline=3 | candidate=0`

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

但如果按主导矛盾排序，`2000-2001` 当前更像是：

1. 先被 `review_gate_gap` 大量挡住；
2. 只有少量 runtime 命中点已经进入“概率够高，但 posture/bucket 仍正常”的阶段。

因此它更接近“严格评审门槛比 runtime policy 明显更严”，而不是典型的“已经持续进入非 normal posture，仍被别的 clause 卡住”。

### 2.2 `1990-1993 美国银行与衰退压力`

- baseline / candidate 首次 `runtime floor hit without L3` 都是：`1990-07-03`
- 两版都有 pre-crisis actionable points：`3`
- 但都没有形成 sustained `3/5` 命中，因此 `L3` 仍为 `—`
- `Runtime block mix` 显示：
  - `posture_bucket_normal: baseline=78 | candidate=85`
  - `review_gate_gap: baseline=9 | candidate=2`

这个场景说明：

- 不只是 candidate 有问题；
- baseline 自己也存在“概率已经很高，但 posture 大部分日期仍停在 normal，只出现零星 prepare/months 脉冲”的问题；
- 所以 `timely_warning_rate=10%` 的主瓶颈，不是单纯 candidate 退化，而是整个 formal main/posture policy 在长窗结构性危机上都不够稳定。

如果按主导矛盾排序，`1990-1993` 和 `2000-2001` 恰好不同：

1. `1990-1993` 不是先卡在 strict review gate；
2. 而是大多数 runtime 命中点从一开始就长期停在 `posture = normal / bucket = normal`；
3. 这说明真正失败的是 posture continuity，而不是简单阈值不足。

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
- `release review` 主报告现在还会新增一张 horizon 级
  `Runtime Separation Comparison` 表：
  - 逐个对比 baseline / candidate 的 `5d / 20d / 60d` diagnosis；
  - 直接展示各 horizon 的 runtime floor；
  - 直接展示 early-warning regime 的平均概率、相对 normal 的 gap /
    lift；
  - 直接展示 `early_warning_avg_probability - runtime_floor` 的 floor gap。

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

最新一轮代码已经把这条审计往前推进了一步：

- `Focus Scenarios` 现在不只输出自由文本 reason，还会输出
  `runtime_actionable_block_category`；
- 每个焦点场景还会汇总 `runtime block mix`，直接按类别统计 baseline /
  candidate 在危机前各被哪类条件挡住了多少次。
- 现在又进一步增加了 `runtime continuity facets`，会对所有
  “hit runtime floor but not strict L3” 的点继续拆成：
  - `posture:*`
  - `bucket:*`
  - `trigger:*`
  - `gate_gap:*`
  - `confirmation:*`
- `Focus Scenarios` 还会直接给出两组主导项 summary：
  - `Dominant runtime block`
  - `Dominant continuity facet`
- 现在还会再往前走一步，直接给出：
  - `Primary failure mode`
- 现在又继续上提成跨场景 summary：
  - `Failure Mode Summary`

当前这层会先把焦点场景归类成几种更容易决策的失败模式：

- `strict_gate_mismatch`
- `posture_continuity_failure`
- `score_confirmation_failure`
- `transitional_bridge_failure`
- `residual_review_l3_failure`

这样正式报告不需要先读完整个 block mix 长列表，先看主导项就能快速判断：

- 这一段主要是 `review_gate_gap` 在挡；
- 还是 `posture_bucket_normal` 在挡；
- 还是 `months_score_low / trigger:none` 这类 continuity 问题在挡。

并且现在还能直接在同一行看到，这个场景更应该优先归到：

- strict review gate 和 runtime floor 口径不一致；
- 还是 posture / months continuity 根本没有建立起来；
- 还是 confirmation / bridge 规则过严。

`Failure Mode Summary` 再往上做了一层聚合，会直接按失败模式汇总：

- baseline 有多少历史场景主要卡在 `strict_gate_mismatch`；
- candidate 有多少历史场景主要卡在 `posture_continuity_failure`；
- 每类失败模式分别对应哪些历史样本。

现在又继续往前推了一层 `Historical Audit Priorities`，会直接把这些历史样本落到下一步该修的工作流：

- `2000-2001` 这类 `strict_gate_mismatch` 场景，会直接归到
  `strict_review_vs_runtime_mapping`；
- `1990-1993` 这类 `posture_continuity_failure` 场景，会直接归到
  `posture_continuity`；
- 同时还会保留该场景的 `family / training_role / protected_window`，
  避免把 `candidate_optional / extension_only / protected stress` 样本和正式主正例混在一起讨论。

现在又把这层 priority 再往上提成 `Historical Audit Workstream Summary`：

- 先直接汇总当前有多少历史场景落在 `strict_review_vs_runtime_mapping`；
- 再汇总有多少场景落在 `posture_continuity`；
- 同时给出每条 workstream 当前覆盖的 `scenario list / family / training role / protected count`。

现在又再往上压了一层 `Historical Audit Takeaways`：

- 不再只给 workstream 表；
- 会直接用几条结论说明：
  - 先修 `strict review gate vs runtime floor`；
  - 还是先修 `posture continuity`；
  - 还是要回去复核 `score confirmation / transitional bridge`。

这样正式报告会先回答：

- 下一步先修哪条线；
- 这条线现在覆盖哪些历史样本；
- 这些样本是 `candidate_optional`、`extension_only`，还是正式主正例之外的
  `protected stress`。

而 `Historical Audit Takeaways` 会进一步把这些信息说成人话，避免读报告的人还要自己把
`workstream / scenario list / suggested review` 再拼回一句结论。

现在还需要再补一层 `Historical Audit Attribution`：

- 不只回答“下一步修哪条线”；
- 还要回答这条线更像：
  - `both_baseline_and_candidate`：baseline 和 candidate 都掉进同一类失败；
  - `baseline_shared_weakness`：这是 formal main 既有短板，不是 candidate 新退化；
  - `candidate_regression`：baseline 没有同类问题，是 candidate 这版自己退化出来的。

这层归因的目的，是避免把两种性质完全不同的问题混在一起讨论：

- 如果是 `baseline_shared_weakness`，后续重点应放在 formal main 的长期结构修复；
- 如果是 `candidate_regression`，后续重点应放在 candidate 这轮训练、阈值或 policy 改动；
- 如果是 `both_baseline_and_candidate`，说明 candidate 既没有修掉主线短板，也还不能拿来替换当前 active。

再往前一步，还需要把这层归因继续压成 `Historical Audit Actions`：

- 不再让读报告的人自己把 attribution 翻译成下一步动作；
- 直接给出这条 workstream 当前更应该进入哪一种处理路径：
  - `candidate_reject_or_retrain`
  - `shared_blocker_fix_before_promotion`
  - `baseline_research_fix`

这样 `release review` 最终 recommendation 就不再只是泛化地说“需要继续复核”，而是能更明确地区分：

- 当前 candidate 是否应该直接判退；
- 当前 candidate 是否只是被 baseline 主线共性短板挡住；
- 某条问题到底是 release go/no-go blocker，还是 formal main 的长期研究修复项。

这意味着后续不需要再只靠肉眼扫长表，已经可以直接回答：

- 是 `review gate` 挡住更多；
- 还是大部分日期都卡在 `posture/bucket stayed normal`；
- 还是 `months / prepare score confirmation` 过严。

并且还能继续回答更细的问题：

- 卡在 `review_gate_gap` 时，主要缺的是 `p20d`、`p60d` 还是两者都缺；
- 卡在 `posture_bucket_normal` 时，是否长期没有任何 `prepare / hedge / defend` trigger；
- `prepare / months` 场景里，阻塞主要来自 score confirmation 还是 transitional bridge。
- `60d` 到底是“没有 early-warning separation”，还是“已经分离但仍穿不过
  runtime floor”。

这条新增对照尤其重要，因为当前 `timely_warning_rate` 的一个主瓶颈，
已经不是“看不见 60d 风险”，而是：

- `60d` 有时能达到 `separated_but_below_runtime_floor`；
- 但如果不把 floor gap 直接打到 `release review` 主报告里，
  后续就很容易把“目标函数问题”和“阈值映射问题”重新混在一起讨论。

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

更具体地说：

- `2000-2001` 应优先复核 strict review gate 与 runtime floor 的映射是否过严；
- `1990-1993` 应优先复核为什么长期高 `p20d/p60d` 仍无法把 posture/bucket 从 `normal` 推到 `prepare/months`；
- 只有把这两类失败拆开，后续训练和 policy 修改才不会继续混成一个问题处理。

而 `Historical Audit Priorities` + `Historical Audit Workstream Summary` 的意义，就是把
“哪些历史样本需要继续复核”以及“下一步先修哪条工作流”都直接固化到正式
`release review` 产物里，避免后续每一轮 review 都重新手工解释一遍。

再加上 `Historical Audit Attribution` 之后，正式产物还会进一步回答：

- 这是当前 formal main 的共性问题，还是 candidate 本轮新增退化；
- 哪些 workstream 属于“主线先天不足”，哪些属于“候选版本自己弄坏了”；
- 后续优先级应先放在主线结构修复，还是直接淘汰当前 candidate。

而补上 `Historical Audit Actions` 之后，正式产物会继续把这些结论落成动作：

- `candidate_regression` 直接进入 candidate 判退 / 重训 / 回滚改动路径；
- `both_baseline_and_candidate` 直接进入晋升前置 blocker 路径；
- `baseline_shared_weakness` 直接进入 formal main 主线研究修复路径。

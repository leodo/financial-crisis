# Release Review Runtime 对齐设计

状态：`Draft`

最后更新：2026-06-07

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

### 2.3 `2026-06-07` posture continuity 口径错位补充

在把 formal main runtime 分类修正到 `feature_formal_v1_main_*` 全前缀之后，
又复跑了 candidate `us_formal_family_hybrid_20260606T112926` 的
`strict_rebuild` 全历史回放，确认还有一条更具体的 continuity 口径错位：

- `build_assessment_snapshot(...)` 此前把 `prepare_reference_p60d` 取自
  `60d horizon.final_probability`
- 但 formal bundle runtime / 历史回放 / UI 真正展示和审计的是
  `runtime_final_probability`
- 当 `60d` 因 monotonic lift 被 runtime 抬高时，这两者可能明显不同

首个直接证据来自 `1990-10-19`：

- 修复前：
  - `raw_p_60d = 0.498`
  - `calibrated_p_60d = 0.802`
  - `time_to_risk_bucket = months`
  - `posture = normal`
  - `posture_trigger_codes = []`
- `probability_diagnostics_json` 显示：
  - `60d final_probability = 0.498`
  - `60d runtime_final_probability = 0.802`
  - `monotonic_lift = 0.304`
- 修复后：
  - `posture = prepare`
  - `posture_trigger_codes = [\"prepare_probability_plateau\"]`

这说明此前并不是 runtime 根本没有看到 `prepare` 所需的长窗风险，而是
posture continuity 在读取一份比 runtime 更低的 `60d` 参考值。

同时，最新 `strict_rebuild` 也确认这不是全部问题都已解决：

- `1998-09-03` 仍是 `raw_p_60d = calibrated_p_60d = 0.718`，但
  `posture = normal`
- `2007-08-01` 在第二轮修复前也属于 `months + normal`，根因是
  `prepare_continuity_bridge` 只会把 `time_to_risk_bucket` 推到 `months`，
  但 `posture` 侧仍被 `conviction_score >= 0.54` 的额外门槛拦住

因此当前结论应更新为：

1. `prepare_reference_p60d` 使用 `runtime_final_probability` 是必须修复的口径错误；
2. 修复后已经消除了一类典型的 `months + normal` 假阴性；
3. 后续又确认 `prepare_continuity_bridge` 也要和 `months` bucket 对齐，不能继续被
   独立 conviction gate 压回 `normal`；
4. 后续又把 `prepare_probability_plateau` 的 `p20d` 从硬编码 `0.45` 调整为
   runtime 派生门槛；在 `2026-06-07 strict_rebuild` 下，
   `1998-09-03` 也已从 `prepare/months` 缺口恢复为
   `prepare + prepare_probability_plateau`。
5. 但正式 `release review` 仍显示 `1987 / 1990-1993 / 1998` 属于
   scenario-level `posture_continuity_failure`，说明当前修复主要解决了
   关键点位口径错位，还没有完全恢复整段 `3/5 sustained` 提前量连续性。

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

同时，当前实现还要额外覆盖另一类样本：

- backtest 摘要里还没有记成 `L2`；
- 但逐日 runtime 已经出现 `runtime floor hit without L3`。

因为这类样本同样说明“模型已经看见了风险，只是没有转成 strict/actionable”，
如果只靠 backtest 摘要字段筛选，仍会把 `2000-2001` 这类长窗结构性失败静默漏掉。

## 5. 实施方案

### 5.1 已完成

- `Focus Scenarios` 已新增：
  - `first runtime-floor hit without L3`
  - point-level `actionable_diagnostic`
- `Focus Scenarios` 选择逻辑已扩展到：
  - `lead_time_days.is_some() && actionable_lead_time_days.is_none()`
  - or any pre-crisis point already satisfies `runtime_floor_hit_without_l3`
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
  - `candidate_regression`：baseline 没有同类问题，是 candidate 这版自己退化出来的；
  - `candidate_revealed_next_blocker`：candidate 虽然在这条线上新暴露出阻塞，
    但历史动作结局没有比 baseline 更差，且 runtime floor 命中没有回退，
    更像 candidate 先修掉了上游问题，随后暴露出下一层 blocker。

这层归因的目的，是避免把两种性质完全不同的问题混在一起讨论：

- 如果是 `baseline_shared_weakness`，后续重点应放在 formal main 的长期结构修复；
- 如果是 `candidate_regression`，后续重点应放在 candidate 这轮训练、阈值或 policy 改动；
- 如果是 `candidate_revealed_next_blocker`，后续重点应放在这条新暴露出来的下游 blocker，
  不应把它和“纯退化”混成同一类判退理由；
- 如果是 `both_baseline_and_candidate`，说明 candidate 既没有修掉主线短板，也还不能拿来替换当前 active。

再往前一步，还需要把这层归因继续压成 `Historical Audit Actions`：

- 不再让读报告的人自己把 attribution 翻译成下一步动作；
- 直接给出这条 workstream 当前更应该进入哪一种处理路径：
  - `candidate_reject_or_retrain`
  - `next_blocker_fix_before_promotion`
  - `shared_blocker_fix_before_promotion`
  - `baseline_research_fix`

这样 `release review` 最终 recommendation 就不再只是泛化地说“需要继续复核”，而是能更明确地区分：

- 当前 candidate 是否应该直接判退；
- 当前 candidate 是否是在“不变差”的前提下暴露出新的下游 blocker；
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

## 8. Repeatable Audit Entry

The release-review alignment work now has a fixed offline entrypoint:

- `scripts/formal-candidate-leadtime-audit.ps1`
- `just formal-candidate-leadtime-audit <baseline> <candidate>`
- `scripts/formal-candidate-scenario-pack-audit.ps1`
- `just formal-candidate-scenario-pack-audit <baseline> <candidate>`

This audit is intentionally narrower than the full release-review report. It exists to answer, in one pass:

1. whether `timely_warning_rate`, `strict_actionable_point_count`, and
   `runtime_floor_hit_count` moved together or diverged;
2. whether `60d` is still `separated_but_below_runtime_floor`, or has already crossed into
   `usable_early_warning_separation` without converting into higher `timely_warning_rate`;
3. which real-history scenarios still have `L2 lead time` but no `L3 actionable`;
4. whether the dominant blocker is still `review_gate_gap`,
   `posture_bucket_normal`, or another runtime block family;
5. which `Historical Audit Workstreams / Actions` remain active for the candidate.

The new `scenario-pack-audit` is broader and complements the lead-time audit:

1. it runs a fixed US history pack spanning `1987 / 1990s / 2000 / 2008 / 2011 / 2020 / 2022 / 2023`;
2. it automatically chooses `formal_v1_main_1990_daily`, `formal_v1_ext_stress_1990_daily`,
   or `formal_v1_ext_acute_pre1990` per scenario instead of relying on manual dataset memory;
3. it merges `formal-probability-compare`, scenario coverage grade / free sources / blocking gaps,
   and release-review blocker labels into one JSON artifact;
4. it exists to answer, in one pass, whether a scenario is blocked by
   free-data coverage, `review_gate_gap`, `posture_continuity`, or residual `L3` conversion.

## 9. 2026-06-06 当前结论

本轮先修的是最明显的 `strict review gate vs runtime floor` 失配，而且只动
`p60d` 这一侧，不同时放松 `p20d`。

### 9.1 已落地

1. worker 侧 `release review` 的 strict `prepare p60d` 不再死写 `45%`：
   - 现在改成从 runtime `prepare_p60d` 推导；
   - 规则是 `max(runtime_prepare_p60d + 0.04, runtime_prepare_p60d * 1.10)`；
   - 并限制在 `[25%, 45%]`。
2. API / backtest / timeline / rolling audit 已同步到同一套 strict `p60d` 映射。
3. legacy / heuristic 路径没有被一起改掉：
   - 只有 formal-main runtime 才会启用这次新的 strict `p60d` 推导；
   - 旧模式仍保持原来的 strict `18% / 45%` 回测口径。

### 9.2 已验证的效果

重新跑 formal candidate lead-time audit 后，能看到一类明确变化：

- 一部分历史场景的 `review_gate_gap` 已从 `p20d_and_p60d` 收缩成 `p20d_only`；
- 说明这次修复确实消除了“明明 runtime 已经可准备，但 strict review 还被 60d=45% 卡死”的一部分失配。

但 top-line 结论没有因此根本改善：

- `timely_warning_rate` 仍没有抬起来；
- 当前主瓶颈更明确地收敛为：
  1. strict `p20d` gate；
  2. posture continuity；
  3. 少量 residual `review_l3_gate_not_satisfied`。

### 9.3 这意味着什么

到这里可以把后续优先级讲清楚：

1. 不应继续只围绕 `p60d` 做微调；
2. 下一步要么继续审 strict `p20d` 映射；
3. 要么直接进 `1990-1993 / 2000-2001 / 2007-2009 / 2023 regional banks`
   的 posture continuity 复盘。

更直接地说：

- `p60d` 这一刀已经证明“review/runtime 口径失配”是真问题；
- 但它不是当前 `timely_warning_rate` 卡住不动的唯一问题，更不是最后一个问题。

### 9.4 2026-06-07 补充：posture continuity 先落一刀 plateau 修复

基于 `1987` 与 `1990-1993` 的逐点复盘，当前已经确认一类更具体的 continuity 失效：

1. `p20d / p60d` 已长期处在高位；
2. 但 `posture` 仍停在 `normal`，`time_to_risk_bucket` 也停在 `normal`；
3. 同时 `actionability_prepare` 在早期历史样本上经常过低，无法单靠
   `prepare_continuity_bridge` 自己把 `prepare/months` 推起来。

因此本轮先补了一条新的 runtime policy：

- `prepare_probability_plateau`

它专门处理“长窗高概率平台期”：

- `p20d` 已明显抬高；
- `p60d` 保持高位；
- `overall / structural / trigger / external / breadth` 至少形成一组平台期上下文；
- 即使 actionability 头还偏弱，也允许先把 `posture` 推到 `prepare`，
  并把 `time_to_risk_bucket` 推到 `months`。

同步约束也已经补齐：

1. API `posture guidance` / `time bucket` 已识别 `prepare_probability_plateau`；
2. backtest `is_actionable_warning_point` 已把这条 code 视为 formal-main 的强
   `prepare` 证据之一；
3. worker `release review` strict L3 逻辑已使用同一条 code，避免 runtime 已升级、
   review 仍按旧 continuity 口径漏掉。

这不是“已经证明 posture continuity 全修好”。
当前已经完成的是：

- 规则与测试已落地；
- `1987` / `1990-1993` 的目标样本已经能被新的 unit / review tests 覆盖；
- 本地服务已重启到新代码版本。

仍待补的证据是：

1. 重新跑一轮真实 `strict_rebuild release review`；
2. 再用 lead-time audit 确认 `1987 / 1990-1993` 是否从
   `posture_continuity_failure` 中真实退出，还是只改善了其中一部分点位。

### 9.5 2026-06-07 补充：formal-main release 分类不能再死锁到 `20260531`

本轮又确认了一条更基础的运行时问题：

1. worker 默认训练/发布出来的 formal-main 新候选已经使用
   `feature_formal_v1_main_20260606_gatefix` 一类的新版本号；
2. 但 API runtime policy 与 history replay 里的 formal-main 识别，之前仍然只认
   精确字符串 `feature_formal_v1_main_20260531`；
3. 结果是新 formal candidate 虽然 `probability_mode=formal_bundle_v1`、
   `label_version=formal_label_v1_main`，却被错误走进 legacy/release 路径：
   - runtime threshold 回退到 legacy `prepare=35% / hedge=30% / defend=30%`
   - `use_transitional_actionable_bridge` 也会误判为仍可使用过渡 bridge
   - `history_runtime_policy_version` / replay cache key 也会掉到 `class=release`

这会直接污染后续所有 `strict gate / runtime floor / posture continuity` 复盘，
因为比较对象已经不是“当前 formal runtime”。

因此本轮先修这条基础设施 bug：

- formal-main feature set 识别改成前缀匹配 `feature_formal_v1_main*`
- 保留 `formal_label_v1_main` 作为第二层约束
- API runtime threshold、history cache / replay、transition bridge 统一用这套识别

修完并重启本地 API 后，live `/api/assessment/current` 已重新识别当前 active release：

- `feature_set_version=feature_formal_v1_main_20260606_gatefix`
- `probability_mode=formal_bundle_v1`
- posture upgrade condition 不再显示 legacy `0.35` 一类阈值，
  而是重新回到 bundle/formal runtime 口径

这意味着后续再审 `strict p20d gate` 时，终于是在正确的 formal-main runtime 前提下继续推进。

After this entrypoint exists, the expected workflow is:

1. run `just formal-candidate-leadtime-audit ...`;
2. decide whether the current bottleneck is mainly review/runtime mapping,
   posture continuity, or false-positive spillover;
3. only then choose whether to edit training constraints, threshold policy, or
   runtime posture logic.

That keeps future work anchored to evidence instead of ad hoc reading of large
`release-review` JSON files.

## 9. Strict Gate Gap Subtype Audit

The historical audit chain now carries one more layer for
`strict_review_vs_runtime_mapping` scenarios:

- `baseline_gate_gap_profile`
- `candidate_gate_gap_profile`

Current labels are intentionally narrow:

- `p20d_only`
- `p60d_only`
- `p20d_and_p60d`

These labels are derived from `Focus Scenarios` runtime continuity facets,
not from manual reading of the long JSON report. The goal is to stop treating
all `strict_gate_mismatch` cases as one undifferentiated blocker.

This now answers a more actionable question:

- is the candidate still mainly blocked by the `20d` strict gate;
- is it already through `20d` but still blocked by `60d`;
- or are both long-window gates still too strict.

The subtype is now surfaced in:

1. `Historical Audit Priorities`
2. `Historical Audit Workstream Summary`
3. `Historical Audit Takeaways`
4. console `release review` summary output
5. exported Markdown review artifacts

That means the next research loop can say, directly and repeatably:

- “first loosen/fix `p60d` strict gate mapping”,
- or “the real blocker is still both `p20d` and `p60d`”,

without re-reading the raw continuity facet tables by hand.

## 10. Attribution Correction

After the first plateau repair, a new audit-semantics issue showed up:

- `1990-1993` / `1998` moved from `posture_continuity_failure`
  to `strict_gate_mismatch`;
- `2000-2001` moved from `strict_gate_mismatch`
  to `residual_review_l3_failure`;
- but some of these scenarios kept `timely_to_timely` or `missed_to_missed`
  outcomes and did not lose runtime-floor coverage.

Treating all of those as `candidate_regression` is too coarse. It collapses
two different meanings:

1. the candidate really made the scenario worse;
2. the candidate repaired an upstream block enough to reveal the next blocker.

The audit logic now distinguishes those two cases explicitly:

- if the candidate makes the warning outcome worse, or runtime-floor coverage
  falls back, keep `candidate_regression`;
- if the baseline already had some other failure mode, the candidate outcome is
  not worse, and runtime-floor hits do not fall back, mark it as
  `candidate_revealed_next_blocker`.

That new attribution maps to a different action:

- `next_blocker_fix_before_promotion`

So release review recommendation text can now separate:

- “this candidate regressed and should be rejected/retrained”;
- from “this candidate still cannot be promoted, but it exposed the next
  blocker rather than simply getting worse”.

## 11. 2026-06-07 Strict Rebuild + Lead-Time Audit Closure

After the attribution correction landed, the next missing piece was not another
threshold guess. It was evidence:

1. rerun the real `strict_rebuild release review`;
2. rerun the fixed `formal-candidate-leadtime-audit`;
3. confirm which blocker now dominates by scenario.

That loop is now closed for:

- baseline `us_formal_family_hybrid_20260605T202246`
- candidate `us_formal_family_hybrid_20260606T112926`

Observed result:

1. `strict_rebuild release review` finished end to end and restored the original
   active release correctly;
2. `formal-candidate-leadtime-audit` initially failed for a trivial reason:
   the PowerShell script had a corrupted placeholder string and could not parse;
   that script bug has now been repaired;
3. the repaired lead-time audit confirms the current blocker order more clearly
   than the long Markdown report alone:
   - `1987` still sits in shared `posture_continuity_failure`;
   - `1990-1993` and `1998` are now primarily `strict_gate_mismatch`, with
     `p20d_only` dominating their gate-gap counts;
   - `2000-2001` and `2023` are now better described as
     `residual_review_l3_failure`;
   - `60d` top-line diagnosis on the candidate is now `cooldown_bleed`, so the
     remaining work is no longer “just recover more runtime hits”.

In plain terms, the candidate is no longer mainly failing because the runtime
never sees risk. It is failing because the review/actionable conversion stack
is still too brittle after runtime hits already exist.

That changes the implementation order:

1. fix the strict `p20d` review gate first, especially for `prepare/months`
   states that already crossed the runtime floor via `p60d`;
2. then fix `months_score_confirmation / posture continuity`, especially where
   `posture` falls back to `normal` after plateau states;
3. only after those two are cleaner, revisit residual `review_l3_gate_not_satisfied`
   points such as `2000-2001` and `2023`.

## 12. 2026-06-07 Continuity / Hysteresis Reruns

The `2026-06-07` review stream ended up needing two distinct narrow repairs:

1. a relaxed `prepare_probability_plateau` continuity path for dates where
   `p20d/p60d` were already extreme but the structural / external context sat
   just below the old plateau guard;
2. a history-only `prepare_history_hysteresis` carry path that rescues already
   anchored `prepare/months` states for a small subset of `1987` / `1990`
   continuity points.

These were intentionally not broad threshold drops. They only target:

- “high-probability plateau already visible, but continuity still falls back to
  normal”; and
- “history replay already promoted to `prepare/months`, but strict actionable
  conversion still misses the same dates”.

### 12.1 Strict rebuild evidence

After rerunning the real `strict_rebuild` review for:

- baseline `us_formal_family_hybrid_20260605T202246`
- candidate `us_formal_family_hybrid_20260606T112926`

the validated result was:

1. `timely_warning_rate` stayed `40.0% -> 40.0%`;
2. `strict_actionable_point_count` improved `161 -> 173`;
3. `runtime_floor_hit_count` improved `327 -> 351`;
4. `actionable_precision` improved `52.8% -> 67.7%`;
5. `longest_false_positive_episode_days` improved `15 -> 13`.

Point-level evidence in the strict rebuild artifact also changed in a useful
way:

- `1987-09-01 .. 1987-09-03` now show candidate `prepare / months` with
  trigger `prepare_history_hysteresis`, but the remaining failure reason is
  still `months_score_confirmation`;
- `1990-07-16 .. 1990-07-19` now show the same `prepare_history_hysteresis`
  promotion pattern;
- late-September `1998` still looks weaker at the probability level itself, so
  it is not primarily a hysteresis-floor problem.

That means the runtime/history repair is real, but it is still not enough to
restore scenario-level `3/5 sustained` continuity.

### 12.2 Review mirror sync evidence

The worker/API strict actionable mirror then needed one more correction:

- the new `history_hysteresis_months_signal` must only accept points carrying
  the explicit `prepare_history_hysteresis` trigger, not any generic
  “strong prepare” trigger;
- otherwise weak `prepare_p60d_structural` / relaxed plateau points are
  incorrectly reclassified as actionable.

After narrowing the mirror and rerunning the `default` review against the same
baseline/candidate pair, the observed result was:

1. `timely_warning_rate` still stayed `40.0% -> 40.0%`;
2. `strict_actionable_point_count` improved `173 -> 185`;
3. `runtime_floor_hit_count` stayed `327 -> 351`;
4. `actionable_precision` stayed `52.8% -> 67.7%`;
5. `longest_false_positive_episode_days` stayed `15 -> 13`.

So the mirror sync did recover additional point-level strict conversion, but it
still did **not** clear the real promotion blocker. The next highest-value work
remains:

1. sustained `prepare/months` continuity for `1987 / 1990-1993 / 1998`;
2. strict gate cleanup for `2000-2001 / 2022`;
3. only then another candidate retrain / rereview loop.

### 12.3 Activation governance alignment

One more operational conflict had to be closed after the reruns above:

- full `release review` already said candidate
  `us_formal_family_hybrid_20260606T112926` should not replace baseline
  `us_formal_family_hybrid_20260605T202246`;
- but `research release activate --reload-api` still compared only the live
  rolling-audit snapshot and tried to keep the candidate active because its
  short-form `actionable_precision` looked better at runtime.

That created a real governance contradiction:

1. formal review said “do not promote candidate”;
2. activation guard still behaved as if the candidate should remain default.

The activation path is now aligned to the latest relevant release-review
artifact:

- if the latest relevant formal review already marked the target release as a
  failed candidate against the current active baseline, activation is blocked;
- if the current active release is that failed candidate and the operator is
  restoring its reviewed baseline, activation is allowed and the runtime
  regression rollback loop is skipped.

Validated runtime result on `2026-06-07`:

1. restoring baseline `20260605T202246` from active candidate
   `20260606T112926` now succeeds;
2. trying to re-activate candidate `20260606T112926` is now rejected up front
   with the strict-review failure reason;
3. runtime `/api/assessment/method` now remains on baseline
   `us_formal_family_hybrid_20260605T202246`.

### 12.4 2026-06-08 narrow `prepare/weeks` score-confirmation rescue

The next targeted experiment stayed deliberately narrow:

- do not lower the generic `prepare` score-confirmation floor;
- only rescue `prepare / weeks` points that simultaneously carry
  `prepare_probability_plateau` and `prepare_history_hysteresis`;
- still require the relaxed plateau probability shape
  (`p20d >= relaxed plateau threshold`, `p60d >= 0.65`);
- and add a small score guard
  (`overall >= 51.5`, `external_shock_score >= 33.0`).

This is meant to catch the exact `regional_banks` early-window setup that had
already crossed runtime evidence but was still just below the previous strict
confirmation line.

Validated `default` review result after rerunning:

1. `timely_warning_rate` stayed `10.0% -> 10.0%`;
2. `strict_actionable_point_count` improved `80 -> 84`;
3. `runtime_floor_hit_count` stayed `90 -> 91`;
4. `actionable_precision` stayed `70.5%`;
5. `longest_false_positive_episode_days` stayed `13`.

Point-level evidence changed as intended:

- `2022-12-09 .. 2022-12-12` in `us_regional_banks_2023` now convert to
  strict actionable;
- `2022-12-08` and `2022-12-13` still remain in
  `prepare_weeks_score_confirmation`, so the clause did not blindly clear the
  whole window;
- `2023-05-04 .. 2023-05-07` still remain blocked because the external-shock
  side is weaker, which keeps the clause from broadening into a generic
  post-crisis permissive path.

The practical conclusion is that this repair is behaving like a point-targeted
strict L3 sync, not a broad threshold relaxation.

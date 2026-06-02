# Formal Main Protected Context 设计

状态：`Draft`

最后更新：2026-06-02

## 1. 背景

截至 `2026-06-02`，项目已经具备三条可复现的数据链：

1. `formal_v1_main_1990_daily:20260601T163337`
2. `formal_v1_ext_stress_1990_daily:20260601T162655`
3. `formal_v1_ext_acute_pre1990:20260601T163102`

其中扩展包已经证明两件事：

- 免费历史数据回填是可落地的；
- `1990-1993 / 1994 / 1998 / 2000-2001 / 2011 / 1987` 不再只是“配置里写着”，而是已经能形成可用 dataset summary。

但 formal main 仍然失败在同一个点：

- `usable_early_warning_horizons=0`
- `20d=weak_regime_separation`
- `60d=cooldown_bleed`
- release review `guard_passed=false`

因此，下一轮核心问题已经不是“有没有更多历史样本”，而是：

```text
formal main 要不要把 candidate_optional / protected stress 正式吸收到主训练上下文里？
```

## 2. 问题定义

当前 formal main 的主正例只有：

- `2008`
- `2020`
- `2023`

这会带来三个直接问题：

1. `train` 仍容易被单一场景主导；
2. `protected stress` 在主数据集里是 `0`，导致 release guard 虽然能检查 protected 行，但主数据集本身不给样本；
3. `20d / 60d` 很难学出“危机前高压但未正式爆发”的 regime separation。

扩展 stress 包已经展示了另一面：

- `1990-1993`
- `1994`
- `2000-2001`
- `2011`

这些样本虽然不一定都该当正式主正例，但它们显然能提供：

- `prepare / hedge`
- protected stress
- 高压但不必等同于主危机爆发的上下文

## 3. 设计目标

下一阶段要解决的不是“把更多历史样本硬塞成主正例”，而是：

1. 让 formal main 看见足够多的高压上下文；
2. 仍然保持 `forward_crisis` 主标签的纪律，不把 `1994 / 2011` 误当成 `2008` 同级危机；
3. 让 `20d / 60d` 学会把 `positive_window` 从 `normal` 拉开，同时压制 `cooldown_bleed`；
4. 保持 release review、historical analog、protected stress 三条链使用同一份场景目录。

## 4. 非目标

这一轮不做：

- 把全部 extension 样本直接升级成 formal main 主正例；
- 重新定义 `2008 / 2020 / 2023` 的危机起点；
- 用更复杂模型替代当前 Rust 可加载 bundle；
- 只靠调 serving 阈值解决 separation 问题。

## 5. 推荐方案

推荐把 formal main 的场景输入拆成两层：

### 5.1 主正例层（positive label scenarios）

继续只由 label set 决定：

- `formal_label_v1_main`
- 当前仍是 `2008 / 2020 / 2023`

用途：

- 生成 `label_5d / 20d / 60d`
- 决定 `forward_crisis` 主概率头的正例

### 5.2 保护上下文层（protected context scenarios）

由 window set 或显式配置决定，首版建议至少包含：

- `protected_stress_windows_v1`
- `candidate_optional`
- `extension_only + protected_window=true`

首批建议实际纳入：

- `1990-1993`
- `1994`
- `2000-2001`
- `2011`
- `2022`

用途：

- 生成 `prepare / hedge / protected_action_window`
- 生成主数据集中的 protected stress 行
- 在 regime-aware 训练与评估里充当“高压但不等同主危机正例”的上下文

## 6. 数据集语义

正式主数据集在下一轮应同时携带两种语义：

1. `forward_crisis` 主正例：
   - 只来自主 label set
2. `protected_context` / `actionability_context`：
   - 来自 protected window set 或 candidate_optional 场景

这意味着一行样本可能满足：

```text
forward_crisis label = 0
prepare / hedge / protected_action_window = 1
```

这是预期行为，不是标签冲突。

## 7. 训练策略建议

### 7.1 概率头

第一阶段不改二元输出头，只改样本治理：

- 主正例仍只用 `forward_crisis` 标签；
- 但对 `protected_context` 行增加 regime-aware 负样本权重；
- 在 candidate selection 与 release guard 中，显式检查：
  - `positive_window > normal`
  - `protected_context` 不应长期高于真正的 `positive_window`
  - `cooldown_bleed` 是否下降

### 7.2 动作头

动作头应允许从 protected context 里拿 `prepare / hedge`：

- 不要求 protected context 一定提供 `defend`
- 重点提升 `prepare / hedge` 的 precision、scenario_count 与 on-time rate
- 仍然禁止把扩展包单独当成正式上线判断依据

## 8. 推荐实现顺序

1. 新增“formal main context scenario loader”
   - label set 负责主正例
   - window set / role filter 负责 protected context
2. 改造 dataset builder
   - forward labels 只看主正例场景
   - action episode / protected flags 看合并后的上下文场景
3. 改 dataset summary
   - 主动区分 `forward positive` 与 `protected context`
4. 重建 `formal_v1_main`
5. 重训 candidate release
6. 重跑 release review / rolling audit / runtime regime audit

## 9. 代码落点

预计会动到：

- `apps/worker/src/main.rs`
  - `load_label_set_crisis_scenarios`
  - formal dataset builder
  - action episode 选择逻辑
  - dataset summary / recommendation
- `config/research_crisis_scenarios.us.json`
  - 如需新增 main-context window set 或 role filter
- `docs/analytics/formal-dataset-spec.md`
- `docs/analytics/regime-separation-training-objective-design.md`
- `docs/roadmap/crisis-probability-design-todo.md`

## 10. 验收标准

只有以下条件同时满足，才算这份设计真正落地：

1. formal main dataset summary 中 `protected` 行不再是 `0`
2. `20d / 60d` 至少出现 `1` 个 usable early-warning horizon
3. release review 不再因为 `cooldown_bleed` / `zero usable early-warning horizons` 被直接拦截
4. actionability 头至少在 `prepare` 或 `hedge` 上恢复基本可用，不再被直接禁用

## 11. 风险与边界

主要风险：

1. protected context 加得太重，会把主概率头重新拉向“长期高压常亮”
2. `candidate_optional` 与 `extension_only` 的边界不清，会让 formal main 又变成一锅杂烩
3. 如果只扩 actionability context，不扩 probability 的 regime-aware 评估口径，可能继续出现“动作头看起来热，概率头仍然冷”的错位

因此实现时必须坚持：

- 主正例语义不变；
- protected context 单独可解释；
- release guard 用同一套口径复核。

## 12. 2026-06-02 第一阶段实现结果

第一阶段代码已经落地：

1. formal main dataset builder 会额外加载 `protected_stress_windows_v1` 作为 context scenarios；
2. `forward label` 仍只来自 `formal_label_v1_main`；
3. `primary_scenario / action episode / protected_action_window / split` 已能看见 `2000 / 2011 / 2022` 这类 protected context；
4. formal main dataset summary 中 `protected` 行已经不再是 `0`。

对应实测数据集：

- `formal_v1_main_1990_daily:20260601T170716`
- `formal_v1_main_1990_daily:20260601T172759`

这说明：

- protected context 已经真正进入 formal main；
- 不再只是文档设计或扩展包侧的旁路能力。

## 13. 2026-06-02 第一阶段复核结论

虽然 protected context 已进入 formal main，但它还没有单独解决主线问题。

最新候选版：

- `us_formal_main_20260601T173113`

最新 release review：

- `reports/release-review/2026-06-01-us_formal_transitional_20260531T094603-vs-us_formal_main_20260601T173113-release-review.md`

结论仍然是：

- `guard_passed=false`
- `usable_early_warning_horizons=0`
- `20d=cold_across_all_regimes`
- `60d=cooldown_bleed`

因此当前的判断是：

1. `protected context` 接入 formal main 是必要步骤；
2. 但只做 context 合并、split 改造、负样本减权，还不足以让 `20d/60d` 形成可用 separation；
3. 下一阶段必须继续改训练目标本身，而不是再把希望寄托在数据接线或 serving 阈值上。

## 14. 下一步

基于当前结果，后续优先级应调整为：

1. 在 `ForwardCrisis` 概率头中加入更强的 protected-context / pre-warning pairwise separation；
2. 检查当前线性概率头是否已经触到容量上限；
3. 若仍然 `cold_across_all_regimes`，评估 horizon-specific feature selection 或更强的目标函数，而不是继续只调 sample weight。

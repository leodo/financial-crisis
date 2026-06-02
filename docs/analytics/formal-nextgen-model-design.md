# 正式危机概率模型下一代设计

- Status: Accepted
- Updated: 2026-06-02

## 1. 背景

当前 `formal main` 主线已经完成了几轮必要但偏工程化的收敛：

1. `raw observations -> feature snapshots -> persisted formal dataset -> bundle/release review` 链路已打通；
2. `scenario_family`、`scenario_training_role`、`default_horizon_roles` 已进入正式训练语义；
3. `regime separation`、`runtime sanity guard`、`strict rebuild release review` 已能较准确识别“bundle 看起来有分离，但 runtime 没法形成可执行提前量”的候选版。

到 `extmix2 / extmix3 / extmix4` 为止，新的事实已经很清楚：

- bundle evaluation 可以学出一定 separation；
- 但 runtime 侧仍长期停在 `timely_warning_rate = 10%`；
- `longest_false_positive_episode_days` 也没有再明显改善；
- 继续只调 `sample-weight / soft-label / pairwise margin` 的收益已经接近上限。

因此，下一步不能再把“再拧一轮权重”当主线，而要切到**更强的模型形态**。

## 2. 要解决的核心问题

系统的真实目标不是“训练集分数更好看”，而是：

1. 在危机真正爆发前数周给出可执行、可解释的离场/对冲准备；
2. 不把 `2022-11 ~ 2023-01` 这类高压但未爆发阶段全部误报成危机；
3. 让 `1990-1993 / 1994 / 1998 / 2000 / 2008 / 2011 / 2020 / 2022 / 2023` 这些不同形态都能被模型部分学进去，而不是只学会单一银行危机模板。

当前线性概率头的主要问题，是它更像“单层加权打分器”，表达不了下面这些形态：

- 风险分数与市场波动指标同时升高时的非线性放大；
- 利率倒挂、融资压力、信用利差、USDJPY 套息波动一起抬头时的组合效应；
- `normal -> pre_warning -> positive_window -> in_crisis -> cooldown` 不是一条近似线性的迁移。

## 3. 约束

下一代方案必须继续满足这些硬约束：

1. **免费数据优先**：不能依赖付费实时数据才能训练或上线；
2. **PIT 一致性**：训练、回放、在线评分必须使用同一套 point-in-time 可见性口径；
3. **可解释**：不能直接跳到难以审计的黑盒方案；
4. **可序列化上线**：训练产物要继续能落成 bundle，并在 API 里稳定复用；
5. **样本稀缺现实**：1990 以后真正主危机样本就不多，不能假设能像通用机器学习那样靠海量标签解决问题。

## 4. 方案比较

### 4.1 继续只做权重微调

不再作为下一轮主线。

原因很简单：`extmix2 / extmix3 / extmix4` 已经证明，这条路线只能在 bundle evaluation 上继续小幅优化，难以穿透到 runtime。

### 4.2 直接上树模型或大黑盒

暂不作为第一优先级。

原因：

- 当前样本量不大，先跳黑盒很容易得到“回放好看、上线解释困难”的结果；
- 当前 bundle / serving / release review 都围绕可解释线性模型搭起来了；
- 先做一个仍可解释、但表达力更强的中间层，工程成本和验证成本都更可控。

### 4.3 第一优先级：`interaction_tail_v1`

这是下一轮默认主线。

核心思想：

- 仍使用 logistic 概率头；
- 但不再只吃原始线性特征；
- 而是给模型补一批**交互特征**和**尾部/阈值特征**，让它具备最基本的非线性表达力；
- 保持当前 release review、runtime 审计、bundle 序列化方式基本不变。

### 4.4 第二优先级：`family_conditional_v1`

只有在 `interaction_tail_v1` 明确失败后，才进入这条线。

目标是让不同危机家族拥有部分独立表达能力，例如：

- `acute_market_liquidity_crash`
- `systemic_credit_banking_crisis`
- `mixed_systemic_stress`
- `rate_shock_or_policy_dislocation`

但这条线对样本量要求更高，也需要更谨慎的在线解释口径，因此不作为第一步。

## 5. 选定路线：`interaction_tail_v1`

## 5.1 模型形态

第一阶段不替换整个 bundle 结构，只新增一个训练形态：

- `linear_v1`
  - 现状
  - 原始 formal features 直接进入 logistic head
- `interaction_tail_v1`
  - 原始 formal features
  - 加上一组 hand-crafted interaction features
  - 再加上一组 tail / hinge / inversion-depth features

这样做的好处是：

1. 对现有代码侵入小；
2. 训练和 serving 能共用同一个 bundle 结构；
3. 失败时可以直接与 `linear_v1` 做 apples-to-apples 对比。

## 5.2 第一批交互特征

第一批只做少量高价值交互，不追求铺满：

1. `overall_score x us_vix_level`
2. `structural_score x trigger_score`
3. `trigger_score x us_vix_level`
4. `trigger_score x us_usdjpy_change_20d`
5. `external_dimension_score x us_usdjpy_level`
6. `us_curve_10y2y_level x us_fed_funds_level`
7. `us_nfci_level x us_stlfsi_level`
8. `us_baa_10y_spread_level x us_vix_level`

这些交互都是在表达“单个指标不一定危险，但组合同时抬升时风险会突然跳变”。

## 5.3 第一批尾部 / 阈值特征

第一批只放与 crisis escalation 关系最强的尾部特征：

1. `vix_excess_24`
2. `vix_excess_32`
3. `baa_10y_spread_excess_2`
4. `stlfsi_excess_1`
5. `usdjpy_excess_145`
6. `abs(usdjpy_change_20d)_excess_4`
7. `curve_10y2y_inversion_depth`
8. `overall_score_excess_55`
9. `structural_score_excess_52`
10. `trigger_score_excess_50`
11. `external_dimension_score_excess_50`

目的不是用人工规则替代模型，而是把“进入危险尾部以后斜率更陡”这件事显式交给模型。

## 5.4 训练目标

`interaction_tail_v1` 第一阶段仍沿用当前已经验证有效的外围机制：

1. `scenario_training_role + scenario_family + horizon_support` 权重；
2. `positive_window / pre_warning_buffer / cooldown` soft labels；
3. regime pairwise margins；
4. sign constraints；
5. 现有 calibration 与 release review 体系。

也就是说，第一阶段的重点不是再发明一整套目标函数，而是先验证：

> 当模型有了更强的非线性表达力后，现有 regime-aware 目标是否终于能穿透到 runtime。

## 6. 为什么先做这个，而不是直接做 family head

因为当前最先需要回答的问题不是“不同危机家族要不要单独头”，而是：

> 现有共享头是不是本身就表达力不够。

如果共享头加少量非线性特征后，`timely_warning_rate` 就能明显恢复，说明当前问题主要是模型形态；
如果仍然失败，才有理由进入第二阶段，把问题定位到“不同危机家族确实需要部分条件化表达”。

## 7. 实施顺序

### Phase 1: 元数据与可序列化骨架

1. bundle 增加 `model_family` / `feature_transform` 元数据；
2. `train-probability` / `bootstrap-formal-release` 增加 `--model-shape`；
3. release note / evaluation report 显式记录候选使用的模型形态。

### Phase 2: 训练与 serving 共用的 derived feature resolver

1. 在共享 domain 层实现 derived feature 解析；
2. worker 训练和 API serving 都走同一套 resolver；
3. 避免训练用的是 interaction/tail feature，线上却只会读取原始值。

### Phase 3: 第一批 `interaction_tail_v1` 候选训练

建议先跑：

- `formal main + ext_stress + ext_acute`
- 且使用新的 `interaction_tail_v1`

理由：

- 这已经是当前覆盖最全、语义最完整的一组训练输入；
- 可以直接判断“表达力增强”是否让扩展场景真的学进去。

### Phase 4: 严格复核

仍然必须通过：

1. bundle evaluation
2. release review
3. runtime regime audit
4. historical scenario missed-case review

不能因为模型形态更复杂就降低护栏。

## 8. 晋级门槛

下一代候选至少要满足下面这些方向性目标，才算继续有效：

1. `timely_warning_rate` 明显高于当前 `10%` 基线；
2. `actionable_precision` 不低于当前可接受区间；
3. `longest_false_positive_episode_days` 不能重新膨胀到不可用；
4. runtime 不再只会命中 `2023`；
5. `1990-1993 / 1998 / 2000 / 2008 / 2011 / 2020 / 2022` 至少部分场景开始出现可执行提前量。

当前建议的研究门槛：

- `timely_warning_rate >= 25%`
- `actionable_precision >= 55%`
- `longest_false_positive_episode_days <= 14`
- runtime 至少 `2` 个 horizon 具备可用 early-warning separation

这还不是最终 production gate，但已经足够区分“只是又一个 bundle 看起来不错的候选”，还是“真正往决策系统靠近了一步”。

## 9. 风险

### 9.1 过拟合

加入交互和 tail 特征后，最直接的风险就是过拟合。

控制方式：

1. 第一批 derived features 数量严格受限；
2. 仍保留 sign constraints 与 release review；
3. 严格以 runtime review 为准，不接受只在 bundle evaluation 上变好。

### 9.2 急性冲击与慢性积压被同一头混淆

这正是 `family_conditional_v1` 作为第二阶段存在的原因。

如果 `interaction_tail_v1` 仍明显只学会一种危机形态，就不要继续死调，而是进入 family-conditional 设计。

### 9.3 线上解释复杂度上升

解决方式不是回避模型升级，而是：

1. 在 bundle metadata 里记录模型形态；
2. 后续 UI / methodology 页把“交互与尾部特征”解释成“风险共振”和“尾部放大”；
3. 不把新特征包装成用户看不懂的技术黑盒。

## 10. 与后续工作的关系

这份文档解决的是“下一轮正式概率模型该怎么升级”的问题，不替代下面几条长期主线：

1. `analytics_prediction_snapshots` 退回审计与桥接视图角色；
2. raw PIT feature store 继续补齐；
3. 扩展历史覆盖矩阵继续完善；
4. 若 `interaction_tail_v1` 失败，再补 `family_conditional_v1` 细分设计。

## 11. 结论

从这一轮开始，项目对正式概率模型的主线判断应明确调整为：

- **停止把 sample-weight 微调当主突破口**
- **优先推进 `interaction_tail_v1` 这条可解释的非线性基线**
- **只有它失败后，再进入 `family_conditional_v1`**

这条路线最符合当前项目的真实约束：免费数据、样本稀缺、需要解释、需要上线、又必须提升“危机前数周可执行提前量”。

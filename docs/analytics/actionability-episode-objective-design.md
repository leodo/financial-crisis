# 动作 Episode 目标设计

状态：`Draft`

最后更新：2026-06-01

## 1. 目标

把 `prepare / hedge / defend` 从“`60d / 20d / 5d` 的 proxy horizon 标签”升级成真正独立的动作目标。

这份文档解决四件事：

1. 每个动作层级到底在什么时间窗口内算正例。
2. 一天样本是否允许同时属于多个动作层级。
3. protected stress window、危机中、危机后余震如何影响动作标签。
4. 训练、评估、release review 应该如何围绕动作 episode 而不是 horizon proxy 做判断。

## 2. 为什么必须单独设计

截至 `2026-06-01`，已经验证过三类旧方案都不够：

1. `forward_crisis` 单头概率；
2. `bounded action window` 版 `action_label_5d / 20d / 60d`；
3. `proxy dual-head actionability`。

共性问题是：

- 能得到可用的离线校准结果；
- 但动作级提前量仍不足；
- 或者为了提高提前量，把正例窗口铺得过宽，导致 `actionable_precision` 和纯误报段明显恶化。

因此，下一阶段不应继续问“20d 阈值调多少”，而应先回答：

```text
prepare 到底是什么动作时刻
hedge 到底是什么动作时刻
defend 到底是什么动作时刻
```

## 3. 设计原则

- 动作标签优先表达“执行节奏”，不是表达“危机是否已经发生”。
- 三个动作层级必须能映射到明确的持仓处置语义。
- 第一版先采用**互斥 primary phase**，避免一个样本同时承担多层级正例。
- episode 目标要允许按场景家族使用不同模板，不能要求 `1987` 和 `2008` 共用一套固定窗口。
- protected stress window 可以支持 `prepare / hedge` 的合理命中，但不应被默认当成 `defend` 主正例。

## 4. 核心定义

### 4.1 动作层级的人话定义

| 层级 | 人话定义 | 目标动作 |
|---|---|---|
| `prepare` | 系统认为脆弱性已经积累，应该先收紧高 beta、检查流动性、准备保护工具。 | 降高 beta、做流动性预案 |
| `hedge` | 系统认为未来几周风险已值得主动保护，不能再等事件完全落地。 | 加保护、增现金、收久期 |
| `defend` | 系统认为短端风险窗口已打开，资本保全和流动性优先。 | 降杠杆、收高风险敞口、保现金 |

### 4.2 动作 episode

每个历史场景都可以派生一个或多个动作 episode。

最小字段：

```text
episode_id
scenario_id
episode_template_id
action_level
primary_start
primary_end
validation_end
cooldown_end nullable
episode_grade
protected_window
evidence_basis
```

字段含义：

- `primary_start / primary_end`：该动作层级真正应该提早触发的窗口；
- `validation_end`：允许“过晚确认”的最晚边界，用于训练后评估，不等于主正例窗口；
- `cooldown_end`：危机后仍允许保守动作存在的余震期；
- `episode_grade`：`mandatory / extension / protected_only`；
- `protected_window`：命中后不应直接按纯误报处理。

## 5. 标签结构

第一版动作标签不再只保留 `action_label_5d / 20d / 60d`，而是新增：

```text
prepare_episode_label
hedge_episode_label
defend_episode_label
primary_action_level nullable
action_episode_id nullable
action_episode_phase
protected_action_window
```

### 5.1 `primary_action_level`

每个样本日最多只有一个 primary action level：

```text
defend > hedge > prepare > none
```

也就是说：

- 如果某天同时落在 `prepare` 和 `hedge` 的理论窗口内，primary 只记 `hedge`；
- `prepare / hedge / defend` 三个二分类标签仍然可以分别训练，但评估、场景计数和 release review 以 `primary_action_level` 为主。

### 5.2 `action_episode_phase`

第一版固定四类：

| phase | 含义 |
|---|---|
| `primary` | 模型应该提前命中的主正例窗口 |
| `late_validation` | 允许统计“过晚确认”，但不计作提前命中 |
| `cooldown` | 危机后高压余震，只用于审计，不计入主动作正例 |
| `outside` | 非动作窗口 |

## 6. 场景模板

### 6.1 `systemic_credit_banking_crisis`

适用场景：

- `us_gfc_2008`
- `us_regional_banks_2023`

默认模板：

| 层级 | primary window | late validation |
|---|---|---|
| `prepare` | `pre_warning_start` 到 `crisis_start - 21d` | `crisis_start - 20d` 到 `crisis_start - 11d` |
| `hedge` | `crisis_start - 20d` 到 `crisis_start - 6d` | `crisis_start - 5d` 到 `crisis_start + 3d` |
| `defend` | `crisis_start - 5d` 到 `acute_start + 3d` | `acute_start + 4d` 到 `acute_start + 10d` |

说明：

- 这类场景最重视 `prepare / hedge` 的提前量；
- `defend` 可以允许在危机前几天到急性触发后的短确认段内命中；
- `cooldown` 不进入 primary label。

### 6.2 `acute_market_liquidity_crash`

适用场景：

- `us_black_monday_1987`
- `us_ltcm_1998`
- `us_covid_liquidity_2020`

默认模板：

| 层级 | primary window | late validation |
|---|---|---|
| `prepare` | `acute_start - 20d` 到 `acute_start - 11d` | `acute_start - 10d` 到 `acute_start - 7d` |
| `hedge` | `acute_start - 10d` 到 `acute_start - 4d` | `acute_start - 3d` 到 `acute_start + 1d` |
| `defend` | `acute_start - 3d` 到 `acute_start + 2d` | `acute_start + 3d` 到 `acute_start + 7d` |

说明：

- 急性冲击不强求长达数月的 `prepare`；
- `prepare` 更像短期流动性预案，而不是长期结构减仓；
- `defend` 对时效最敏感。

### 6.3 `mixed_systemic_stress`

适用场景：

- `us_dotcom_unwind_2000`
- `us_funding_stress_2011`

默认模板：

| 层级 | primary window | late validation |
|---|---|---|
| `prepare` | `pre_warning_start` 到 `crisis_start - 16d` | `crisis_start - 15d` 到 `crisis_start - 8d` |
| `hedge` | `crisis_start - 15d` 到 `crisis_start - 5d` | `crisis_start - 4d` 到 `crisis_start + 3d` |
| `defend` | 默认关闭，需场景级 override | 仅在有明确 `acute_start` 且 evidence 足够强时开启 |

说明：

- 这类场景更适合检验 `prepare / hedge` 是否合理；
- 默认不把它们强行当成 `defend` 主正例；
- 若配置 `protected_window=true`，命中后默认进入 protected 审计。

### 6.4 `rate_shock_or_policy_dislocation`

适用场景：

- `us_bond_massacre_1994`
- `us_rate_shock_2022`

默认模板：

| 层级 | primary window | late validation |
|---|---|---|
| `prepare` | `pre_warning_start` 到 `crisis_start - 16d` | `crisis_start - 15d` 到 `crisis_start - 8d` |
| `hedge` | `crisis_start - 15d` 到 `crisis_start - 5d` | `crisis_start - 4d` 到 `crisis_start + 2d` |
| `defend` | 默认不进入主训练 | 只进入 protected / analog 审计 |

说明：

- 这类场景可以支持“为什么该提前防守”，但不直接当成主银行危机正例；
- 第一阶段主要服务 posture、protected stress 和样本扩展。

## 7. 与 horizon label 的关系

新的动作 episode 与旧 horizon label 的关系是：

| 对象 | 用途 |
|---|---|
| `label_5d / 20d / 60d` | 危机先验、历史类比、长期脆弱性排序 |
| `prepare / hedge / defend episode labels` | 动作级提前量、posture 升降、position guidance |

明确要求：

- 不再把 `action_label_60d = prepare` 直接当成正式答案；
- `prepare / hedge / defend` 可以共享底层特征，但不共享目标定义；
- runtime 的 posture 要优先看动作层和上下文，而不是把 horizon 阈值硬翻译成动作。

## 8. 评估口径

### 8.1 场景级

每个动作层级至少输出：

```text
scenario_count
advance_warning_scenario_count
late_confirmation_scenario_count
missed_scenario_count
advance_warning_rate
late_confirmation_rate
missed_rate
median_lead_time_days
```

### 8.2 点级

每个动作层级至少输出：

```text
precision_at_threshold
primary_recall_at_threshold
late_validation_recall_at_threshold
predicted_positive_count
false_positive_count
```

### 8.3 protected stress 口径

对于 `protected_window=true` 的场景：

- 不把命中直接记为纯误报；
- 但也不计入 `mandatory` 主正例成功数；
- 单独输出 `protected_precision` 与 `protected_episode_count`。

## 9. Go / No-Go 护栏

动作层 release review 最低线建议：

### `prepare`

- `scenario_count >= 3`
- `advance_warning_rate >= 35%`
- `missed_rate <= 65%`

### `hedge`

- `scenario_count >= 3`
- `advance_warning_rate >= 25%`
- `missed_rate <= 65%`

### `defend`

- `scenario_count >= 2`
- 不强求高 `advance_warning_rate`
- 但 `late_confirmation_rate <= 40%`
- `missed_rate <= 50%`

补充要求：

- 任一层级只落在单一 evaluation 场景上时，不允许晋升为正式候选；
- 若 `defend` 只能在危机已经发生后才触发，不允许把它包装成“提前防守能力”。

## 10. 数据与配置变更

### 10.1 场景目录新增字段

建议在场景配置中新增：

```text
episode_template_id
action_episode_overrides nullable
allow_defend_training
protected_action_levels
```

### 10.2 正式数据集新增列

建议在 `research_formal_dataset_rows` 新增：

```text
prepare_episode_label
hedge_episode_label
defend_episode_label
primary_action_level
action_episode_id
action_episode_phase
protected_action_window
```

## 11. 实现顺序

1. 先把场景模板和 override 落到配置；
2. 再生成 episode-native 动作标签；
3. 然后更新训练侧评估；
4. 再更新 release review guard；
5. 最后再改 runtime fusion 和前端解释。

## 12. 当前建议

下一轮不要继续优先做：

- 动作阈值小修小补；
- dual-head 展示字段再加几个版本号；
- 用同一套 `action_label_20d / 5d` 再训练更多候选。

下一轮必须优先做：

1. `episode-native actionability label builder`
2. `primary_action_level` 场景级评估
3. protected stress 独立统计
4. 按动作层级而不是按 horizon proxy 做 release guard

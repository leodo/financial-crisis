# 危机场景目录

状态：`Draft`

最后更新：2026-05-31

## 1. 目标

把“哪些历史事件算本系统的正式场景、分别服务什么标签和回测用途”固定下来。

这份文档解决五件事：

1. 场景不是靠记忆临时写进代码。
2. `1987`、`2008`、`2020` 不再混成同一种危机标签。
3. `p_5d / p_20d / p_60d` 各自用哪些场景做正例。
4. 哪些场景只用于历史类比，哪些可进入正式训练。
5. 哪些阶段应该算受保护压力窗口，而不是纯误报。

## 2. 场景家族

| 家族 | 含义 | 典型用途 |
|---|---|---|
| `acute_market_liquidity_crash` | 急性市场崩盘与流动性冲击 | `p_5d`、`p_20d`、historical analog |
| `systemic_credit_banking_crisis` | 银行、信用、融资系统性危机 | `p_20d`、`p_60d` 主标签 |
| `mixed_systemic_stress` | 融资、政策、风险偏好共振，但未必发展为银行危机 | protected stress windows、historical analog |
| `rate_shock_or_policy_dislocation` | 利率、政策与久期冲击主导 | posture、protected stress windows |

## 3. 标准字段

每个场景至少要有：

```text
scenario_id
family
label
pre_warning_start
crisis_start
acute_start nullable
crisis_peak nullable
crisis_end
default_horizon_roles
training_role
protected_window
evidence_basis
```

## 4. 场景使用规则

### 4.1 `p_5d`

优先使用：

- 有明确急性触发点的场景
- `acute_start` 清晰
- 市场和流动性压力快速共振

### 4.2 `p_20d`

同时使用：

- 急性冲击场景
- 银行/信用危机场景
- 触发前几周已明显脆弱的混合型场景

### 4.3 `p_60d`

优先使用：

- 有明确结构性脆弱积累的场景
- 不能只靠单日市场暴跌定义

因此：

- `1987` 适合 `p_5d / p_20d`
- `2008` 适合 `p_20d / p_60d`
- `2022` 更适合 posture 和 protected window，不应直接充当典型银行危机正例

## 5. 正式场景目录

| 场景 ID | 家族 | `pre_warning_start` | `crisis_start` | `acute_start` | `crisis_end` | 训练角色 | protected window | 主要用途 |
|---|---|---|---|---|---|---|---|---|
| `us_black_monday_1987` | `acute_market_liquidity_crash` | `1987-09-01` | `1987-10-14` | `1987-10-19` | `1987-11-20` | `extension_only` | 否 | 急性市场崩盘类比、`p_5d/p_20d` 扩展样本 |
| `us_ltcm_1998` | `acute_market_liquidity_crash` | `1998-07-01` | `1998-08-17` | `1998-08-31` | `1998-10-15` | `extension_only` | 否 | 杠杆去化和流动性冲击类比 |
| `us_dotcom_unwind_2000` | `mixed_systemic_stress` | `1999-12-01` | `2000-03-10` | `2000-04-14` | `2001-04-04` | `candidate_optional` | 是 | 资产泡沫出清、风险偏好塌缩 |
| `us_gfc_2008` | `systemic_credit_banking_crisis` | `2007-02-27` | `2007-08-01` | `2008-09-15` | `2009-06-30` | `mandatory` | 否 | `p_20d/p_60d` 主样本 |
| `us_funding_stress_2011` | `mixed_systemic_stress` | `2011-06-01` | `2011-07-29` | `2011-08-08` | `2011-10-31` | `extension_only` | 是 | 美欧融资压力与风险偏好冲击 |
| `us_covid_liquidity_2020` | `acute_market_liquidity_crash` | `2020-01-24` | `2020-02-24` | `2020-03-09` | `2020-04-30` | `mandatory` | 否 | `p_5d/p_20d` 主样本 |
| `us_rate_shock_2022` | `rate_shock_or_policy_dislocation` | `2021-11-01` | `2022-01-03` | `2022-06-10` | `2022-10-31` | `no_positive_main` | 是 | posture、protected stress、误报分离 |
| `us_regional_banks_2023` | `systemic_credit_banking_crisis` | `2023-02-01` | `2023-03-08` | `2023-03-10` | `2023-05-15` | `mandatory` | 否 | 银行和存款压力主样本 |

## 6. 各场景证据口径

### 6.1 `us_black_monday_1987`

证据重点：

- 美股极端单日暴跌；
- 波动和流动性压力急升；
- 适合定义为“急性崩盘”，不定义为银行信用危机。

### 6.2 `us_ltcm_1998`

证据重点：

- 俄罗斯违约外溢；
- 杠杆去化与流动性收缩；
- 市场和融资条件快速恶化。

### 6.3 `us_gfc_2008`

证据重点：

- 信贷、房产、银行、融资链条持续恶化；
- 既有前瞻脆弱性，又有明确急性触发；
- 是 `p_20d / p_60d` 最核心的系统性样本。

### 6.4 `us_covid_liquidity_2020`

证据重点：

- 市场与流动性在极短时间内同步失稳；
- 对 `p_5d` 训练极关键；
- 可检验模型是否会在触发前几周提早升温。

### 6.5 `us_rate_shock_2022`

证据重点：

- 联储加息与久期冲击导致资产广谱承压；
- 并非典型银行信用危机；
- 主要用于解释“为什么动作级防守可能合理，但不应全算纯误报”。

## 7. 训练角色定义

| 角色 | 含义 |
|---|---|
| `mandatory` | 必须进入正式标签流水线 |
| `candidate_optional` | 可进入研究版训练，但先不做主样本硬依赖 |
| `extension_only` | 扩展场景，只用于类比、补充或专项急性冲击模型 |
| `no_positive_main` | 不作为主正例，但可进入 protected stress 或 posture 审计 |

## 8. 第一版正式标签建议

### 8.1 `formal_label_v1_main`

仅纳入：

- `us_gfc_2008`
- `us_covid_liquidity_2020`
- `us_regional_banks_2023`

原因：

- 这三类和当前目标最贴近；
- 特征覆盖相对完整；
- 既有急性冲击，也有银行/信用系统压力。

### 8.2 `formal_label_v1_ext_acute`

扩展纳入：

- `us_black_monday_1987`
- `us_ltcm_1998`

用途：

- 急性冲击短窗研究；
- historical analog；
- 检查 `p_5d` 是否过度依赖 `2020` 单一事件。

## 9. protected stress windows 的建议

这些场景建议默认进入受保护压力窗口目录：

- `us_dotcom_unwind_2000`
- `us_funding_stress_2011`
- `us_rate_shock_2022`

理由：

- 它们可能产生较强的 `hedge / defend` 信号；
- 但不应简单归类为“纯误报”；
- 更适合拿来评估动作策略是否合理，而不是强行当作标准危机正例。

## 10. 当前代码与目标状态的差距

当前代码里的正式危机起点目录只有：

- `2007-08-01`
- `2020-02-24`
- `2023-03-08`

这足以跑通过渡版训练，但还不够支撑最终正式模型。

下一步至少要把：

- 场景目录落到配置或数据库；
- `1987 / 1998 / 2011 / 2022` 的角色写入元数据；
- `acute_start` 和 `protected_window` 从代码常量中剥离出来。

## 11. 对实现的直接要求

至少需要这些对象：

```text
research_crisis_scenarios
research_scenario_evidence
research_scenario_role_sets
```

并支持：

- `label_set = formal_label_v1_main`
- `label_set = formal_label_v1_ext_acute`
- `window_set = protected_stress_windows_v1`

## 12. 下一步

文档补齐后，后续编码顺序应是：

1. 场景目录配置化；
2. 标签流水线支持多 `label_set`；
3. 回测与审计支持按场景角色分层汇总。

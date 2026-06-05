# 美国历史场景数据覆盖矩阵

状态：`Draft`

最后更新：2026-06-01

## 1. 目标

把美国主线历史压力样本逐个拆开，明确：

1. 每个场景有哪些免费可回补数据。
2. 哪些特征可以进入 `1990+` 主面板，哪些只能做 extension / analog / protected stress。
3. 哪些场景当前已经可落地，哪些还需要补 feature、标签或 replay 能力。

这份文档不再泛泛讨论“历史数据够不够”，而是直接给后续实现提供执行矩阵。

## 2. 使用规则

场景最终只能进入四种角色之一：

| 角色 | 含义 |
|---|---|
| `main_training` | 可进入 `formal_v1_main_1990_daily` 或其后续主线训练 |
| `extension_training` | 单独扩展训练包，不与主面板硬拼 |
| `protected_stress` | 主要用于 posture / rolling audit / false-positive 分离 |
| `historical_analog_only` | 只做类比与解释，不承担正式训练 |

## 3. 特征覆盖等级

| 等级 | 含义 |
|---|---|
| `A` | `core_1990` 快慢变量齐全，可支撑主面板训练 |
| `B` | 核心变量够用，但事件层或部分日频变量较弱，更适合扩展训练或 protected stress |
| `C` | 只能靠代理特征包，适合 extension / analog |
| `D` | 只能做解释，不适合正式训练 |

## 4. 场景矩阵

| 场景 | 家族 | 推荐角色 | 覆盖等级 | 免费主源 | PIT 等级 | 当前状态 | 主要缺口 |
|---|---|---|---|---|---|---|---|
| `1987 黑色星期一` | `acute_market_liquidity_crash` | `extension_training + historical_analog_only` | `C` | `DEXJPUS`、`DFF/FEDFUNDS`、`BAA10Y`、`UNRATE`、`INDPRO`、`HOUST`、`GS10/GS2`、`NFCI` | `best_effort` | 可做急性冲击扩展包 | 缺 `VIX`、无日频 Treasury 曲线、无 EDGAR 事件层 |
| `1990-1993 银行与衰退压力` | `mixed_systemic_stress` | `protected_stress + extension_training` | `B` | `VIXCLS`、Treasury curve、`BAA10Y`、`DFF`、`NFCI`、`UNRATE`、`INDPRO`、`HOUST`、`DEXJPUS` | `best_effort` | 主面板可覆盖 | 事件层弱，`STLFSI` 尚未覆盖这段早期年份 |
| `1994 联储加息与债市暴跌` | `rate_shock_or_policy_dislocation` | `protected_stress + extension_training` | `B` | Treasury curve、`BAA10Y`、`DFF`、`NFCI`、`UNRATE`、`INDPRO`、`HOUST`、`DEXJPUS` | `best_effort` | 已可配置入目录 | 需要明确 Orange County / rate shock 的 episode 模板，不宜直接当主危机正例 |
| `1998 LTCM/俄违约` | `acute_market_liquidity_crash` | `extension_training` | `B` | `VIXCLS`、Treasury curve、`BAA10Y`、`DFF`、`NFCI`、`UNRATE`、`DEXJPUS` | `best_effort` | 可作为急性扩展样本 | 事件层仍弱，场景窗口需要与 5d/20d acute template 对齐 |
| `2000-2001 科网出清` | `mixed_systemic_stress` | `protected_stress + extension_training` | `A-` | `VIXCLS`、Treasury curve、`BAA10Y`、`DFF`、`NFCI`、`STLFSI4`、`UNRATE`、`INDPRO`、`HOUST`、`DEXJPUS` | `best_effort` | 数据覆盖已接近主面板标准 | 不应直接混成银行危机主正例，需要 role/episode 约束 |
| `2007-2009 GFC` | `systemic_credit_banking_crisis` | `main_training` | `A` | 主面板核心因子 + `SEC` 事件层（`1994+`） | `best_effort + 部分 strict` | 已是正式主样本 | 仍缺更完整 strict PIT 和更多信用代理 |
| `2011 美欧融资压力` | `mixed_systemic_stress` | `protected_stress + extension_training` | `A-` | 主面板核心因子 + `SEC` 事件层 | `best_effort + 部分 strict` | 已适合 protected stress / extension | 需要明确定义何时允许 `hedge`、何时禁止 `defend` 主训练 |
| `2020 疫情流动性冲击` | `acute_market_liquidity_crash` | `main_training` | `A` | 主面板核心因子 + `SEC` 事件层 | `best_effort + 部分 strict` | 已是正式主样本 | 仍需更严格 replay 审计验证 |
| `2022 加息与久期冲击` | `rate_shock_or_policy_dislocation` | `protected_stress` | `A` | 主面板核心因子 + `SEC` 事件层 | `best_effort + 部分 strict` | 适合 posture / protected stress | 默认不作为主危机正例 |
| `2023 区域银行危机` | `systemic_credit_banking_crisis` | `main_training` | `A` | 主面板核心因子 + `SEC` 事件层 | `best_effort + 部分 strict` | 已是正式主样本 | 还需和 2008 一起支撑更稳的 action episode 评估 |

## 5. 分场景说明

## 5.1 `1987`

当前最合理定位：

- `p_5d / p_20d` 急性扩展训练；
- historical analog 强制纳入；
- 不进入 `formal_v1_main_1990_daily` 主宽表。

原因：

- 快速冲击语义强；
- 但主面板关键快变量 `VIX` 与现代日频事件层缺失；
- 更适合用代理特征包做“急性崩盘能力校验”。

## 5.2 `1994`

当前最合理定位：

- `rate_shock_or_policy_dislocation`
- protected stress 与 extension 样本

原因：

- 主轴是利率、久期和政策冲击；
- 对你的系统很重要，因为它能帮助区分“应保护但未必是银行危机”的阶段；
- 但不应直接把它作为 `2008` 那种主危机正例。

## 5.3 `2000-2001`

当前最合理定位：

- protected stress
- extension training

原因：

- 这段时间对 `prepare / hedge` 很有价值；
- 但把它与银行信用危机主标签完全等权，会稀释主目标。

## 5.4 `2011`

当前最合理定位：

- protected stress
- extension training

原因：

- 市场与融资条件共振明显；
- 事件层已经可用；
- 但更适合检验系统是否过度误报，而不是当成主银行危机正例。

## 6. 与 PIT 等级的关系

### 6.1 当前可承诺

- `1990+` 主面板：`best_effort`
- `1994+` EDGAR 事件层：部分 `strict`
- `1987 / 1998` 扩展包：`best_effort`

### 6.2 当前不能承诺

- `1987` 全量 strict PIT
- 所有场景统一拥有同质量的事件层
- 所有历史区间都具备现代信用代理与公告层

## 7. 哪些场景可以先进入下一轮主线

下一轮实现应优先分两批推进。

### 批次 A：先进入正式扩展流水线

- `1994`
- `1998`
- `2000-2001`
- `2011`

要求：

- 写入场景目录 metadata
- 明确 family / training role / episode template
- 在 dataset builder 中能够按 role 纳入或排除

### 批次 B：继续保留为扩展急性包

- `1987`

要求：

- 进入 extension dataset
- 进入 historical analog
- 不强行塞进主面板统一宽表

## 8. 对数据回补的直接要求

### 8.1 必须补齐

1. `1990+` 主面板的 feature snapshot 长历史重建
2. `1994 / 1998 / 2000-2001 / 2011` 的 scenario-aware dataset summary
3. `DEXJPUS/BOJ` 长历史在扩展场景中的 PIT 可见性落地

### 8.2 不必强求

1. `1987` 拥有现代级别事件层
2. `1987` 使用与 `2023` 完全同维度的日频宽表
3. 用弱信用代理硬做全时期统一 strict 主训练

## 9. 对实现的直接要求

建议新增一份可机器读取的覆盖配置或表：

```text
research_scenario_data_coverage
```

至少包含：

```text
scenario_id
recommended_role
coverage_grade
point_in_time_mode
usable_for_main_training
usable_for_extension_training
usable_for_protected_stress
usable_for_historical_analog
blocking_gaps_json
```

## 10. 完成定义

只有以下条件同时满足，才算这份矩阵真正落地：

1. `1987 / 1994 / 1998 / 2000-2001 / 2011 / 2020 / 2023` 都有明确角色；
2. dataset builder 能按角色纳入 / 排除；
3. release review / historical analog / protected stress 使用同一份场景角色定义；
4. 不再靠口头说明判断“这个场景到底算主正例还是 protected stress”。

## 11. 下一步

### 11.1 当前落地状态（2026-06-02）

1. `formal_v1_ext_acute_pre1990:20260601T163102`
   - `1987 / 1998` 都已进入扩展 acute 包；
   - 两个场景都已跨 `2` 个 split；
   - 可用于 `5d/20d` 历史类比与急性尾段研究；
   - 不作为正式主模型上线判断依据。
2. `formal_v1_ext_stress_1990_daily:20260601T162655`
   - `1990-1993 / 1994 / 2000-2001 / 2011` 已进入扩展 stress 包；
   - `calibration / evaluation` 已拿到 forward 正例、episode-native 主正例与 `protected` 行；
   - 可用于 protected stress、历史对照与扩展训练研究；
   - 不作为正式主模型 go/no-go 的单独依据。
3. `formal_v1_main_1990_daily:20260606Tfullhistorygatefix`
   - 已使用 `feature_formal_v1_main_20260606_gatefix` 完成全历史重建；
   - 范围已从旧版 `1998-01-05 -> 2026-05-31` 恢复为 `1990-01-02 -> 2026-05-31`；
   - `jp_rates_call_rate` 不再被当作 formal main 硬依赖，`STLFSI` 仅从 `1993-12-31` 起进入核心/触发硬覆盖；
   - 当前 formal main 的主问题已不再是历史覆盖起点，而是后续训练是否能恢复 timely warning 与 actionability。

后续编码顺序建议是：

1. 重训 candidate release，并重跑 release review / rolling audit / runtime regime audit；
2. 继续补 raw PIT 与 `best_effort PIT` 的长期可回放能力；
3. 最后再决定哪些扩展样本有资格进入更正式的 regime-aware 主线训练。

# Family-Conditional Model Design

状态：`Experimental`

最后更新：2026-06-03

## 1. 为什么进入这条线

`interaction_tail_v1` 已经证明共享头可以学出一定 regime separation，但连续多轮没有把 runtime `timely_warning_rate` 从 `10.0%` 拉出来。

最新的 `prepare_p60d_episode_native_v1` 也失败了：

- `60d pre_warning_buffer` soft target 从 `26.0%` 提到 `45.2%`；
- objective weight 从 `0.630` 提到 `0.900`；
- 但 runtime `timely_warning_rate` 仍是 `10.0%`，`actionable_precision` 还轻微下降。

因此，问题不再是“60d buffer 没被看见”，而是共享 `60d` 头仍把不同风险形态混在一起。

## 2. 目标

第一版 `family_conditional_v1` 的目标是：

```text
让模型在同一个可序列化 logistic bundle 内，拥有按风险家族分化的表达能力。
```

需要覆盖的风险家族：

| Family proxy | 代表风险 |
| --- | --- |
| `systemic_credit` | 银行、信用、融资压力 |
| `mixed_systemic` | 科技泡沫、财政/主权信用、跨资产慢性压力 |
| `rate_shock` | 利率/政策冲击、曲线倒挂 |
| `acute_liquidity` | VIX/流动性急性冲击 |
| `jpy_carry` | 日元套息交易反转与外部流动性冲击 |

## 3. 关键约束

不能把训练标签里的 `scenario_family` 直接作为线上特征。

原因：

- 训练时知道历史场景属于哪个 family；
- 线上当天不知道“当前未来会属于哪个 family”；
- 直接把 `scenario_family` 做 one-hot 会让回测变好看，但 serving 无法真实计算。

所以第一版只允许使用**免费指标可实时计算**的 family proxy derived features。

## 4. 第一版模型形态

新增：

```text
model_shape = family_conditional_v1
feature_transform = family_conditional_v1
```

它包含三层特征：

1. formal base features；
2. `interaction_tail_v1` 已有交互/尾部特征；
3. 新增 family proxy / family context 特征。

这样仍然保持：

- bundle JSON 可序列化；
- API serving 和 worker training 共用同一个 derived feature resolver；
- release review 不需要换协议；
- 失败时可以与 `interaction_tail_v1` 做直接对比。

## 5. Derived Features

### 5.1 Family Proxy

第一批 proxy：

```text
family_proxy__systemic_credit
family_proxy__mixed_systemic
family_proxy__rate_shock
family_proxy__acute_liquidity
family_proxy__jpy_carry
```

这些不是标签，而是由 VIX、信用利差、金融压力、利率、曲线、USDJPY 等免费指标计算出的风险簇强度。

### 5.2 Family Context

第一批 context：

```text
family_context__systemic_credit__structural_score
family_context__mixed_systemic__trigger_score
family_context__rate_shock__external_dimension_score
family_context__acute_liquidity__trigger_score
family_context__jpy_carry__external_dimension_score
```

含义：

- proxy 表示“当前像不像某类风险环境”；
- context 表示“在这类环境下，哪个维度的风险分数更重要”。

## 6. 训练和评估

训练输入沿用当前覆盖最全组合：

- `formal_v1_main_1990_daily:20260601T172759`
- `formal_v1_ext_stress_1990_daily:20260601T162655`
- `formal_v1_ext_acute_pre1990:20260601T163102`

第一轮只做 PoC，不直接晋升 active。

必须通过：

1. `cargo test --workspace`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `just release-review-fast <candidate>`

## 7. Go / No-Go

Go 条件：

- `timely_warning_rate` 必须高于 active `10.0%`；
- `actionable_precision` 不低于 active 的 `90%`；
- `longest_false_positive_episode_days <= active + 2`；
- runtime 不能只靠 `2023` 或单一 family 命中。

No-Go 条件：

- `timely_warning_rate` 仍停在 `10.0%`；
- 或 `months` / `prepare` 误报段重新扩散；
- 或 family proxy 特征只让 bundle evaluation 变好，runtime review 不动。

若第一版 No-Go，下一步才考虑真正的多头结构：

```text
shared base head + family-specific calibration/head overlays
```

这会比 derived-feature PoC 更侵入，需要单独设计 bundle schema 与 UI 解释。

## 8. 第一版 PoC 结果

候选：

- `us_formal_family_conditional_20260603T084333`

训练输入：

- `formal_v1_main_1990_daily:20260601T172759`
- `formal_v1_ext_stress_1990_daily:20260601T162655`
- `formal_v1_ext_acute_pre1990:20260601T163102`

离线 bundle 指标看起来变好：

- `brier = 0.0114`
- `log_loss = 0.0548`
- `ece = 0.0292`
- `5d / 20d / 60d` bundle evaluation 都显示 usable separation

但 runtime fast review 失败：

| Metric | Active `extmix10` | Candidate `family_conditional` | 结论 |
| --- | ---: | ---: | --- |
| `timely_warning_rate` | `10.0%` | `0.0%` | 明显倒退 |
| `actionable_precision` | `55.9%` | `54.5%` | 轻微倒退 |
| `longest_false_positive_episode_days` | `5` | `5` | 未扩散 |
| `prepare_p60d` floor | `65.6%` | `70.8%` | 变得更难触发 |
| `p_60d>=prepare` history hits | `112` | `29` | 过度收窄 |
| Runtime 60d diagnosis | `usable_early_warning_separation` | `late_only_no_early_warning` | 早期分离被打坏 |

结论：

- 单纯增加 family proxy / context derived features 不够；
- 它让离线指标更好，但 runtime 60d early warning 变差；
- 下一步不应继续堆类似 proxy 特征；
- 需要进入真正的多头或分层校准 schema 设计。


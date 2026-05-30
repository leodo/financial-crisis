# 日元套息外部风险模块设计

状态：`Draft`

最后更新：2026-05-30

## 1. 目标

建立一个围绕日元套息交易的外部风险模块，用来识别“日本利率/汇率变化通过全球杠杆和平仓路径放大美国市场压力”的风险。

这个模块是美国主线的放大器，不是独立主引擎。

## 2. 为什么需要单独设计

用户当前最关心的外部风险不是泛全球覆盖，而是：

- 日元长期融资货币角色
- BOJ 政策变化
- USDJPY 快速反向波动
- 套息交易平仓带来的跨资产连锁冲击

这类风险可能在美国内部已经脆弱时，把“数周风险”压缩成“数日风险”。

## 3. 模块定位

在总体架构中，它属于：

```text
external_shock_score
```

作用方式：

- 平时低权重
- 当美国内部 trigger score 已升高时，提高 `p_5d` 和 `p_20d`
- 提供历史类比和解释文案

## 4. 第一批指标

### 4.1 汇率水平和变化

| 指标 ID | 含义 |
|---|---|
| `us_external_usdjpy_level` | USDJPY 当前水平 |
| `us_external_usdjpy_change_5d` | 5 日变化 |
| `us_external_usdjpy_change_20d` | 20 日变化 |
| `us_external_usdjpy_realized_vol_20d` | 20 日实现波动 |

### 4.2 利差

| 指标 ID | 含义 |
|---|---|
| `us_external_us_jp_2y_rate_diff` | 美国 2Y - 日本 2Y |
| `us_external_us_jp_short_rate_diff` | 美国短端 - 日本短端 |

### 4.3 BOJ 与日本资金环境

| 指标 ID | 含义 |
|---|---|
| `jp_rates_call_rate` | 日本无担保隔夜拆借利率 |
| `jp_policy_shift_proxy` | BOJ 政策变化代理 |

### 4.4 联动压力

| 指标 ID | 含义 |
|---|---|
| `us_external_usdjpy_vix_coupling` | USDJPY 与 VIX 联动压力 |
| `us_external_usdjpy_credit_coupling` | USDJPY 与信用利差联动压力 |
| `us_external_jpy_carry_stress` | 套息压力综合分 |

## 5. 数据源

优先级：

1. `BOJ`：官方 FX 和时序 API
2. `FRED`：H.10 FX 系列，例如 JPY/USD
3. `Treasury + BOJ`：美日利差代理
4. `Stooq`：只做原型补充，不做核心依赖

## 6. 信号逻辑

### 6.1 单独高 USDJPY 不等于风险

不能因为日元弱就直接判定风险高。

真正危险的是：

- 波动率突然上升
- 方向快速反转
- BOJ 利率抬升或政策预期变化
- 同时伴随 VIX 和信用利差恶化

### 6.2 共振增强

建议规则：

```text
if trigger_score high
and usdjpy volatility high
and rate_diff compressing
then raise external_shock_score
```

### 6.3 第一阶段不做

- 杠杆资金真实头寸估计
- 非公开融资链追踪
- 跨机构级联仿真

## 7. 模块输出

```text
external_shock_score
jpy_carry_state
jpy_carry_reason
jpy_carry_contributors
```

状态建议：

- `quiet`
- `building`
- `stress`
- `unwind`

## 8. UI 表达

面板上应显示：

- 日元套息风险当前状态
- 本周变化
- 与 VIX/信用的联动情况
- 历史上相近阶段的提示

不要只展示一个汇率数字。

## 9. 回测思路

第一阶段不单独做日元危机预测，而是评估：

- 加入该模块后，`p_5d` / `p_20d` 是否更早或更准确
- 是否减少“美国内部已高压，但外部放大器未被识别”的漏报

## 10. 后续开发顺序

1. 先落地 USDJPY、BOJ 利率和美日利差。
2. 再做联动特征。
3. 最后做综合分和 UI 卡片。

## 11. 参考入口

- [BOJ Foreign Exchange Rates (Daily)](https://www.boj.or.jp/en/statistics/market/forex/fxdaily/index.htm)
- [BOJ Time-Series Data Search API](https://www.boj.or.jp/en/statistics/outline/notice_2026/not260218a.htm)
- [FRED DEXJPUS](https://fred.stlouisfed.org/graph/?g=eZIE)

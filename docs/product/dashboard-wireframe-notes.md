# Web 面板草图说明

状态：`Draft`

最后更新：2026-05-30

## 1. 目标

用文字草图定义第一版 Web 面板的主要布局和组件。本文不是视觉设计稿，而是后续 UI 设计和前端实现的结构依据。

## 2. 总览页草图

```text
┌──────────────────────────────────────────────────────────────┐
│ Region: US  Scope: Financial System  As-of: 2026-05-30       │
│ Method: scoring_v1  Data Quality: A-                         │
├──────────────────────────────────────────────────────────────┤
│ Overall Risk: L2 Stress       Score: 63      +8 in 30 days    │
│ [Risk trend line: 1y]                                          │
├───────────────────────┬──────────────────────────────────────┤
│ Structural Risk: 58   │ Trigger Risk: 69                     │
│ Macro: 44             │ Market Stress: 76                    │
│ Credit: 71            │ Liquidity: 66                        │
│ Banking: 52           │ Events: 49                           │
├───────────────────────┴──────────────────────────────────────┤
│ Top Contributors                                              │
│ 1. High Yield OAS             score 82   +12                 │
│ 2. VIX                        score 78   +21                 │
│ 3. 10Y-2Y Curve               score 74   -3                  │
│ 4. NFCI                       score 68   +7                  │
├──────────────────────────────────────────────────────────────┤
│ Latest Alerts                         Data Source Issues      │
│ L2 Credit stress triggered            GDELT delayed 3h        │
│ L1 Bank filing watch                  yfinance prototype      │
└──────────────────────────────────────────────────────────────┘
```

首屏必须能看到：

- 当前等级。
- 当前分数。
- 最近变化。
- 主要贡献。
- 数据质量。

## 3. 分项风险页草图

```text
┌──────────────────────────────────────────────────────────────┐
│ Dimension: Credit & Leverage                                 │
├──────────────────────────────────────────────────────────────┤
│ Score: 71  Level: L3 Warning  Change 30d: +14                │
│ [Dimension score trend: 3y]                                  │
├──────────────────────────────────────────────────────────────┤
│ Indicator Contributions                                      │
│ High Yield OAS      82  weight 0.30  contribution 24.6       │
│ Baa Spread          76  weight 0.20  contribution 15.2       │
│ Credit/GDP Gap      65  weight 0.25  contribution 16.3       │
│ Bank Loan Growth    48  weight 0.15  contribution 7.2        │
├──────────────────────────────────────────────────────────────┤
│ Related Alerts | Data Quality | Method Notes                 │
└──────────────────────────────────────────────────────────────┘
```

## 4. 指标库页草图

```text
┌──────────────────────────────────────────────────────────────┐
│ Filters: Region [US] Dimension [All] Level [L2+] Quality [A-C]│
├──────────────────────────────────────────────────────────────┤
│ Indicator              Value   Score Level  30d   Quality Src │
│ High Yield OAS         4.92%   82    L3     +12   A       FRED│
│ VIX                    27.1    78    L3     +21   A       FRED│
│ 10Y-2Y Curve          -0.32%   74    L3     -3    A       FRED│
│ GDELT Negative Tone    0.41    55    L2     +8    B       GDELT│
└──────────────────────────────────────────────────────────────┘
```

要求：

- 表格支持排序、筛选、列显隐。
- 风险分和质量等级必须同时展示。
- 点击指标进入详情页。

## 5. 指标详情页草图

```text
┌──────────────────────────────────────────────────────────────┐
│ High Yield OAS                                               │
│ Source: FRED / BAMLH0A0HYM2  Frequency: Daily  Quality: A     │
├──────────────────────────────────────────────────────────────┤
│ Latest: 4.92%  Percentile: 82  Score: 82  Level: L3           │
│ [Time series with risk bands]                                │
├──────────────────────────────────────────────────────────────┤
│ Scoring                                                       │
│ Direction: higher_is_riskier                                  │
│ Transform: percentile over 5y window                          │
│ Contribution: 24.6 to Credit dimension                        │
├──────────────────────────────────────────────────────────────┤
│ Data Quality                                                  │
│ Last updated: 2026-05-30  Raw payload: available              │
│ Checks: freshness pass, range pass, consistency pass          │
└──────────────────────────────────────────────────────────────┘
```

## 6. 预警记录页草图

```text
┌──────────────────────────────────────────────────────────────┐
│ Filters: Status [Open] Level [L2+] Date [Last 90d]            │
├──────────────────────────────────────────────────────────────┤
│ Level Status  Triggered          Scope       Reason           │
│ L3    Open    2026-05-28 09:00   Credit      HY OAS spike     │
│ L2    Ack     2026-05-21 09:00   Liquidity   NFCI rising      │
└──────────────────────────────────────────────────────────────┘
```

事件详情：

- 触发快照。
- 贡献指标。
- 升级/降级历史。
- 人工确认记录。

## 7. 数据源状态页草图

```text
┌──────────────────────────────────────────────────────────────┐
│ Source       Status     Last Success      Delay   Quality     │
│ FRED         Healthy    2026-05-30 08:01  0h      A           │
│ SEC EDGAR    Healthy    2026-05-30 08:30  0h      A           │
│ GDELT        Delayed    2026-05-30 04:00  4h      B           │
│ yfinance     Prototype  2026-05-30 08:10  0h      C           │
└──────────────────────────────────────────────────────────────┘
```

要求：

- 原型源必须显式标记。
- 授权待审查的数据源必须提示。
- 点击数据源可进入运行记录列表。

## 8. 回测页草图

```text
┌──────────────────────────────────────────────────────────────┐
│ Scenario: 2008 Global Financial Crisis                        │
│ Method: scoring_v1                                            │
├──────────────────────────────────────────────────────────────┤
│ [Risk score chart with crisis window shaded]                  │
│ First L2: 2007-07-xx  First L3: 2007-08-xx                    │
│ Lead time to crisis marker: xx days                           │
├──────────────────────────────────────────────────────────────┤
│ Metrics: alerts, false positives, missed windows, stability   │
└──────────────────────────────────────────────────────────────┘
```

## 9. 视觉和交互原则

- 面板风格应偏操作台，不做营销式首页。
- 风险颜色应克制，避免整页大面积红色。
- 分数、等级、变化和质量应同时出现。
- 所有图表都需要支持时间范围切换。
- 表格中的异常项应可排序和过滤。
- 所有风险解释都能下钻到指标和数据源。

## 10. API 粒度提示

前端不应直接组合过多底层接口。建议后端提供：

- `/api/overview`
- `/api/dimensions`
- `/api/indicators`
- `/api/indicators/{id}`
- `/api/alerts`
- `/api/sources`
- `/api/backtests`


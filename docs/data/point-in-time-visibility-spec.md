# Point-in-Time 可见性规范

状态：`Draft`

最后更新：2026-05-31

## 1. 目标

定义“在某个 `as_of_date` 当天，系统到底允许看到哪些数据”。

这份文档是正式模型和严格回测的基础。如果这部分不定死，任何“危机前就预警到了”的回测都可能掺了未来函数。

## 2. 设计原则

- 先定义默认评估时点，再定义数据可见性。
- 优先使用官方时间戳；没有时间戳时使用保守规则。
- `strict` 和 `best_effort` 必须明确区分，不能混写成一个 mode。
- 不同来源、不同频率的数据，允许不同可见性等级。

## 3. 默认评估时点

第一版统一采用：

```text
assessment_cutoff = as_of_date 17:30 America/New_York
```

含义：

- 当天收盘后再做系统评估；
- 只有在这个时间点之前已经可见的数据，才允许进入该日特征；
- 第二天才发布的数据，不能回写到前一天。

## 4. 核心时间字段

所有原始观测或事件至少要支持：

```text
observed_at
published_at nullable
accepted_at nullable
ingested_at
visible_at
point_in_time_grade
visibility_rule_id
```

字段含义：

- `observed_at`：数据对应的经济或市场观察时间
- `published_at`：官方发布日期/时间
- `accepted_at`：对 EDGAR 这类事件源，官方接收时间
- `ingested_at`：本系统抓到数据的时间
- `visible_at`：回测时真正判断“当时是否可见”的时间

## 5. 可见性等级

| 等级 | 规则 |
|---|---|
| `strict` | `visible_at` 来自官方明确时间戳 |
| `best_effort` | 无明确时间戳，但有官方日期，可按固定 cutover 或保守 lag 近似 |
| `weak` | 只有观测日期，无可靠发布时间，不进正式严格训练 |
| `none` | 仅解释或原型，不参与正式训练 |

## 6. 统一判定规则

### 6.1 主规则

某条数据在 `as_of_date=t` 可见，当且仅当：

```text
visible_at <= t 17:30 America/New_York
```

### 6.2 回退规则

如果没有官方 `published_at` 或 `accepted_at`，按以下顺序回退：

1. 使用官方日期 + 源默认 cutover 时间
2. 如果连官方日期也没有，使用保守 lag 规则
3. 如果连保守 lag 也无法合理定义，则标记为 `weak` 或 `none`

## 7. 源别可见性规则

### 7.1 FRED Graph CSV

规则：

- 默认只有 `observation_date`，没有 vintage 和精确发布时间；
- 只能标为 `best_effort`；
- 不允许把它声称为 `strict point-in-time`。

默认 `visible_at`：

| 类型 | 规则 |
|---|---|
| 日频市场/利率类 | `observation_date 17:30 America/New_York` |
| 周频指标 | `observation_period_end + 3 天 17:30 America/New_York` |
| 月频指标 | `observation_period_end + 15 天 17:30 America/New_York` |
| 季频指标 | `observation_period_end + 45 天 17:30 America/New_York` |

说明：

- 这是保守近似，不是官方逐系列 release calendar。
- 要进入 `strict`，必须切到官方 API / ALFRED 或补正式发布日期元数据。

### 7.2 FRED API / ALFRED

规则：

- 若同时保留 `realtime_start / realtime_end` 与官方发布日期，可提升到接近 `strict`；
- 若只有 vintage date 没有具体时间，仍为 `best_effort`。

### 7.3 Treasury 日频收益率曲线

规则：

- 默认按官方当日发布处理；
- 因缺少稳定的统一精确时间戳，第一版记为 `best_effort`。

默认 `visible_at`：

```text
observation_date 18:00 America/New_York
```

### 7.4 World Bank 年频数据

规则：

- 主要用于慢变量背景，不做短窗触发；
- 若无明确发布时间，按保守年频 lag。

默认 `visible_at`：

```text
period_year_end + 270 天 17:30 America/New_York
```

### 7.5 SEC EDGAR

规则：

- 使用 `acceptance_datetime` 或同等级官方接收时间；
- 这是当前最接近 `strict` 的免费事件层。

默认 `visible_at`：

```text
accepted_at
```

若只有 `filing_date` 无 `acceptance_datetime`：

```text
filing_date 18:00 America/New_York
```

### 7.6 BOJ / 官方 FX 页面

规则：

- 若页面或 API 只有日期无统一时间，按日本时区日终发布近似；
- 第一版为 `best_effort`。

默认 `visible_at`：

```text
observation_date 17:00 Asia/Tokyo
```

回测时再换算到 `America/New_York`。

### 7.7 GDELT

规则：

- 新闻抓取和聚合噪声高；
- 第一版仅作为解释层；
- 不纳入正式严格训练。

等级：

```text
point_in_time_grade = none
```

## 8. 特征可见性规则

派生特征的 `visible_at` 取输入特征中最晚可见的那个时间：

```text
feature_visible_at = max(input_feature_visible_at)
```

因此：

- `10Y2Y` 要等 10Y 和 2Y 都可见；
- `banking_event_market_coupling` 要等市场特征和事件特征都可见。

## 9. 标签与可见性的关系

特征可见性只约束输入，不约束未来标签。

也就是说：

- `label_5d = 1` 表示未来 5 个交易日内进入危机窗口；
- 但模型输入只能用 `as_of_date` 当天 `17:30 ET` 前已可见的数据。

## 10. 数据集模式

### 10.1 `strict`

要求：

- 所有核心特征都具有 `strict` 或被批准的 `best_effort_strict_compatible` 规则；
- 事件层必须有官方时间；
- 允许牺牲样本数量换取可信度。

### 10.2 `best_effort`

要求：

- 允许市场、利率、宏观的保守 lag 近似；
- 可作为第一版正式训练基线；
- 必须在 release 元数据中显式标出。

### 10.3 `research_loose`

要求：

- 允许更宽松的近似和 explain-only 特征；
- 只能用于研究，不允许直接晋升到正式线上 release。

## 11. 当前正式建议

截至 `2026-05-31`：

- `formal_v1_main_1990_daily` 采用 `best_effort`
- `strict` 仍是后续目标，不是当前现状
- 使用 FRED CSV 的任何结果，都不能宣传为“严格 point-in-time”

## 12. 对存储和实现的直接要求

至少新增或固化这些字段：

```text
published_at
accepted_at
visible_at
point_in_time_grade
visibility_rule_id
visibility_mode
```

并且：

- 每次生成 feature snapshot 时，必须输出 `latest_visible_at`
- 每个训练数据集必须记录 `visibility_mode`
- 每个 release manifest 必须写 `point_in_time_mode`

## 13. 失败时的处理

若某日关键特征无法满足该 release 声明的可见性要求：

- 在线评分退回 `heuristic_mvp` 或降级结果；
- 历史回测该日标记为 `coverage_or_visibility_failed`；
- 该日不允许悄悄混入正式训练集。

## 14. 下一步

后续编码顺序：

1. 原始观测表补时间字段；
2. feature snapshot 记录 `latest_visible_at`；
3. dataset builder 按 `visibility_mode` 过滤样本；
4. release manifest 与 API method 响应显式暴露这一层信息。

当前实现进度（`2026-05-31`）：

- `feature snapshot` 已记录 `latest_visible_at`
- `formal dataset builder` 已按 `point_in_time_mode` 过滤样本
- `best_effort` 已按源规则落地：
  - `fred` / `treasury` / `world_bank` / `boj` 使用规则化可见时间
  - `sec_edgar` 优先使用事件发布时间
- `strict` 已能在代码层被强约束，但当前免费主数据源仍缺足够官方时间戳，因此还不能作为默认正式训练口径

还没做完的部分：

- 原始观测表还没有完整落 `accepted_at / visible_at / visibility_rule_id`
- 现有历史免费数据里，部分 `publication_time` 仍带有“回填抓取时间”痕迹，所以 `strict` 只能作为审计失败信号，不能当默认训练模式

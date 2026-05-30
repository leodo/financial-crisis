# Assessment API Contract

状态：`Draft`

最后更新：2026-05-31

## 1. 目标

定义 assessment API，对外提供危机概率、时距判断、决策 posture、仓位预算建议、历史对照、数据可信度，以及“当前看到的值是不是实时值”的运行时解释。

## 2. 设计原则

- 不再把旧 `/api/overview` 作为唯一核心接口
- 概率、强度、posture 明确分开
- 响应可直接驱动新的决策面板
- `position_guidance` 是系统级动作预算，不是自动交易指令
- 必须显式暴露 `data_mode`、关键指标日期和 stale warning，避免把 demo 或旧库值误当成实时市场值

## 3. 主要接口

### 3.1 `GET /api/assessment/current`

返回当前评估快照。

### 3.2 `GET /api/assessment/history`

返回一段时间的历史评估轨迹。

查询参数：

```text
from
to
entity_id
```

当前实现补充：

- 支持 `limit`
- 默认返回较长窗口，不再只截取最近十几个点

### 3.3 `GET /api/assessment/analogs`

返回当前最接近的历史阶段。

### 3.4 `GET /api/assessment/data-trust`

返回数据覆盖率和可信度。

### 3.5 `GET /api/assessment/posture`

返回 posture 及其原因和升级/降级条件。

### 3.6 `GET /api/assessment/method`

返回方法版本和说明。

### 3.7 `POST /api/system/reload`

立即重新加载当前数据源，适合刚完成 SQLite backfill 后刷新 API 内存态。

## 4. `current` 响应结构

```json
{
  "as_of_date": "2026-05-30",
  "entity_id": "us",
  "market_scope": "financial_system",
  "probabilities": {
    "p_5d": 0.364,
    "p_20d": 0.549,
    "p_60d": 0.599
  },
  "time_to_risk_bucket": "now",
  "posture": "defend",
  "conviction_score": 0.95,
  "scores": {
    "overall_score": 63.1,
    "structural_score": 61.8,
    "trigger_score": 64.6,
    "external_shock_score": 59.1
  },
  "summary": "短期风险窗口已经打开。5d / 20d / 60d 概率分别为 36% / 55% / 60%，当前 posture 为 defend。",
  "posture_reason": "系统认为短期风险窗口已经打开，优先资本保全和流动性管理。",
  "top_risk_drivers": [],
  "top_relief_drivers": [],
  "historical_analogs": [],
  "data_trust": {
    "coverage_score": 1.0,
    "core_feature_coverage": 1.0,
    "trigger_feature_coverage": 1.0,
    "external_feature_coverage": 1.0,
    "quality_grade": "a",
    "warnings": []
  },
  "jpy_carry": {
    "state": "quiet",
    "score": 33.9,
    "usdjpy_level": 148.0,
    "jp_call_rate": 0.48,
    "us_short_rate": 5.12,
    "us_jp_short_rate_diff": 4.64,
    "change_5d": 7.0,
    "change_20d": null,
    "realized_vol_20d": 0.088,
    "funding_pressure_score": 55.7,
    "vix_coupling_score": 49.8,
    "credit_coupling_score": 45.2,
    "reason": "USDJPY 波动与美股/信用压力暂未形成明显共振，美日短端利差约 4.64%。"
  },
  "position_guidance": {
    "target_equity_exposure_pct": 25.0,
    "target_cash_pct": 45.0,
    "hedge_ratio_pct": 40.0,
    "leverage_cap_pct": 20.0,
    "option_overlay_pct": 15.0,
    "action_summary": "进入资本保全区间，优先流动性、现金和保护覆盖。",
    "actions": [],
    "guardrails": []
  },
  "runtime": {
    "data_mode": "demo",
    "generated_at": "2026-05-31T00:00:00Z",
    "requested_as_of_date": "2026-05-30",
    "latest_observation_at": "2026-05-30",
    "latest_observation_lag_days": 0,
    "demo_mode": true,
    "stale_warning": "当前页面运行在 demo 模式，关键指标值是示例数据，不代表真实市场最新状态。"
  },
  "key_indicators": [],
  "event_assessment": {
    "state": "confirmed",
    "confirmation_score": 66.4,
    "recent_event_count": 2,
    "summary": "事件层已经提供了实质性确认，当前风险判断不再只是市场噪声。",
    "confirmed_signals": [],
    "pending_gaps": [],
    "recent_events": []
  },
  "backtest_summary": {
    "scenario_count": 3,
    "timely_warning_rate": 1.0,
    "missed_rate": 0.0,
    "avg_lead_time_days": 27.0,
    "median_lead_time_days": 21.0,
    "total_false_positive_count": 4,
    "summary": "当前内置场景回测覆盖 3 个危机样本，至少提前 7 天给出有效预警的比例约为 100%。"
  },
  "user_preferences": {
    "profile": "neutral",
    "cash_floor_pct": 15.0,
    "max_equity_cap_pct": 70.0,
    "max_leverage_pct": 100.0,
    "option_overlay_preference_pct": 5.0,
    "allow_aggressive_reentry": false,
    "note": "profile=neutral, cash_floor=15%, max_equity=70%, max_leverage=100%, option_overlay=5%"
  },
  "method": {
    "score_method_version": "scoring_v1_20260530",
    "prob_model_version": "prob_v1_20260530",
    "calibration_version": "calib_v1_20260530",
    "feature_set_version": "feature_v1_20260530",
    "label_version": "label_v1_20260530",
    "posture_policy_version": "posture_v1_20260530"
  }
}
```

## 5. 类型定义

### 5.1 ProbabilityBlock

```text
p_5d: number
p_20d: number
p_60d: number
```

范围：

- `0.0` 到 `1.0`

### 5.2 TimeToRiskBucket

```text
normal | months | weeks | now
```

### 5.3 Posture

```text
normal | prepare | hedge | defend
```

### 5.4 HistoricalAnalog

```text
scenario_id
name
similarity_score
reference_phase
note
peak_score
lead_time_days
```

### 5.5 DataTrust

```text
coverage_score
core_feature_coverage
trigger_feature_coverage
external_feature_coverage
quality_grade
data_quality_summary
warnings[]
```

### 5.6 JpyCarrySnapshot

```text
state
score
usdjpy_level
jp_call_rate
us_short_rate
us_jp_short_rate_diff
change_5d
change_20d
realized_vol_20d
funding_pressure_score
vix_coupling_score
credit_coupling_score
reason
```

说明：

- 当前重点是看日元融资环境是否会放大美国风险资产的同步回撤。
- `funding_pressure_score` 和 `us_jp_short_rate_diff` 已进入概率与 posture 映射。

### 5.7 PositionGuidance

```text
target_equity_exposure_pct
target_cash_pct
hedge_ratio_pct
leverage_cap_pct
option_overlay_pct
action_summary
actions[]
guardrails[]
```

说明：

- 这是系统级仓位预算和保护建议。
- 不能当成用户个性化投资建议或自动交易指令。

### 5.8 RuntimeMetadata

```text
data_mode
generated_at
requested_as_of_date
latest_observation_at
latest_observation_lag_days
demo_mode
stale_warning
```

说明：

- `data_mode` 必须区分 `demo / sqlite / postgres`
- `demo_mode=true` 时，页面必须明确提示当前不是实时市场值

### 5.9 KeyIndicatorStatus

```text
indicator_id
display_name
entity_id
source_id
dataset_id
unit
latest_value
latest_as_of_date
lag_days
stale_threshold_days
status
note
```

说明：

- 关键指标 freshness 至少覆盖 `USDJPY`、日本隔夜拆借利率、`EFFR`、`VIX`
- 用于解释“为什么页面现在显示的值可能与真实市场有偏差”

### 5.10 EventAssessment

```text
state
confirmation_score
recent_event_count
summary
confirmed_signals[]
pending_gaps[]
recent_events[]
```

### 5.11 BacktestPerformanceSummary

```text
scenario_count
timely_warning_rate
missed_rate
avg_lead_time_days
median_lead_time_days
total_false_positive_count
summary
```

### 5.12 UserRiskPreferences

```text
profile
cash_floor_pct
max_equity_cap_pct
max_leverage_pct
option_overlay_preference_pct
allow_aggressive_reentry
note
```

## 6. 兼容旧接口

第一阶段保留：

- `/api/overview`
- `/api/dimensions`
- `/api/indicators`
- `/api/alerts`
- `/api/backtests`

但新面板应以 `/api/assessment/*` 为主。

## 7. 错误响应

统一格式：

```json
{
  "error": {
    "code": "assessment_data_unavailable",
    "message": "No calibrated assessment snapshot is available.",
    "details": {}
  }
}
```

## 8. 第一阶段实现建议

1. 以 `/api/assessment/current` 作为主聚合接口
2. 旧 `/api/overview` 继续服务指标和维度下钻
3. Web 首页优先解释概率、时距和 posture，再解释强度分
4. 下一阶段继续补 `events` 和真实回测接口

## 9. 风险

- 当前概率仍是启发式 MVP，不是正式校准后的危机发生率
- `position_guidance` 尚未经过完整历史回测验证
- 新旧接口并存期间仍可能出现语义漂移

# Assessment API Contract

状态：`Draft`

最后更新：2026-06-01

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

返回方法版本、说明、默认历史轨迹的 provenance 摘要，以及当前滚动审计使用的受保护压力窗口目录。

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
  "actionability": {
    "prepare": 0.611,
    "hedge": 0.536,
    "defend": 0.382
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
    "usdjpy_level": 159.2,
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
    "action_playbook_version": "action_playbook_v1_20260531",
    "execution_urgency": "立即执行；当日到 2 个交易日内优先去杠杆、补现金并建立核心保护覆盖。",
    "confidence_gate": "当前数据可信度和事件确认度足以支持执行主要防守动作。",
    "target_equity_exposure_pct": 25.0,
    "target_cash_pct": 45.0,
    "hedge_ratio_pct": 40.0,
    "leverage_cap_pct": 20.0,
    "option_overlay_pct": 15.0,
    "action_summary": "进入资本保全区间，优先流动性、现金和保护覆盖。",
    "actions": [],
    "forbidden_actions": [],
    "reentry_conditions": [],
    "guardrails": [],
    "capital_preservation_overlay_enabled": true,
    "governance": {
      "system_budget_only": true,
      "auto_execution_allowed": false,
      "manual_confirmation_required": true,
      "policy_change_requires_release_review": true,
      "policy_change_requires_go_no_go": true,
      "required_operator_checks": [
        "先确认当前动作框架版本与 active release 一致，再解释仓位预算。",
        "先检查数据模式、关键指标日期和 stale warning，避免把演示值或陈旧值当成当前市场。"
      ]
    }
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
    "real_scenario_count": 3,
    "fallback_scenario_count": 0,
    "coverage_scope_note": "这里的“本地覆盖场景 / 模板参照场景”按场景回测历史窗口 2007-05-03 到 2026-05-31 统计；它回答的是危机场景目录里有多少样本能直接落在这段本地历史上，不等于上面默认历史轨迹是否已经进入 PIT 正式证据层。",
    "structural_warning_rate": 0.667,
    "timely_warning_rate": 0.667,
    "missed_rate": 0.333,
    "avg_structural_lead_time_days": 41.5,
    "avg_lead_time_days": 11.5,
    "median_lead_time_days": 15.0,
    "total_false_positive_count": 2,
    "history_start": "2007-05-03",
    "history_end": "2026-05-31",
    "rolling_audit": {
      "history_point_count": 4971,
      "actionable_signal_count": 464,
      "pre_crisis_signal_count": 9,
      "in_crisis_signal_count": 331,
      "stress_window_signal_count": 114,
      "false_positive_signal_count": 10,
      "false_positive_episode_count": 3,
      "longest_false_positive_episode_days": 9,
      "actionable_precision": 0.925,
      "classified_episodes": [
        {
          "start_date": "2016-01-04",
          "end_date": "2016-01-21",
          "duration_days": 18,
          "signal_count": 13,
          "classification": "stress_window",
          "note": "2015-2016 汇率与美元流动性压力：人民币贬值、能源信用与美元流动性收缩共同触发全球风险资产承压。"
        },
        {
          "start_date": "2022-11-10",
          "end_date": "2022-11-18",
          "duration_days": 9,
          "signal_count": 7,
          "classification": "false_positive",
          "note": "未落入危机前 20 日窗口，也不在受保护压力窗口内。"
        }
      ],
      "summary": "全历史滚动审计覆盖 2007-05-03 到 2026-05-31；动作级信号共 464 个评估点，其中危机前 9 个、危机中 331 个、受保护压力窗口 114 个、纯误报 10 个，形成 3 段纯误报区间，动作信号精度约为 92%。"
    },
    "summary": "当前回测覆盖 3 个真实危机样本；结构性抬升至少提前 7 天出现的比例约为 67%，可执行预警至少提前 7 天出现的比例约为 67%。"
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
    "actionability_model_version": "actionability_bundle_20260601T003145",
    "actionability_calibration_version": "actionability_platt_20260601T003145",
    "feature_set_version": "feature_v1_20260530",
    "label_version": "label_v1_20260530",
    "posture_policy_version": "posture_v1_20260530",
    "action_playbook_version": "action_playbook_v1_20260531",
    "fusion_policy_version": "fusion_policy_v1_actionability_diag_20260601",
    "actionability_enabled": true,
    "probability_mode": "formal_bundle_v1",
    "release_status": "approved",
    "release_id": "us_formal_pit_dualhead_20260601T003145",
    "point_in_time_mode": "best_effort"
  }
}
```

## 4.1 `method` 响应结构

```json
{
  "method": {
    "score_method_version": "scoring_v2_20260531",
    "prob_model_version": "prob_v1_20260531",
    "calibration_version": "calib_v1_20260531",
    "actionability_model_version": "actionability_bundle_20260601T003145",
    "actionability_calibration_version": "actionability_platt_20260601T003145",
    "feature_set_version": "feature_v2_20260531",
    "label_version": "label_v1_20260530",
    "posture_policy_version": "posture_v1_20260530",
    "action_playbook_version": "action_playbook_v1_20260531",
    "fusion_policy_version": "fusion_policy_v1_actionability_diag_20260601",
    "actionability_enabled": true,
    "probability_mode": "formal_bundle_v1",
    "release_status": "approved",
    "release_id": "us_formal_pit_dualhead_20260601T003145",
    "point_in_time_mode": "best_effort"
  },
  "note": "assessment 概率、风险强度和 posture 为不同层的输出；当前版本为启发式 MVP，不是校准后的正式危机概率模型。页面应优先检查 data_mode、关键指标日期和 stale warning，再解释当前数值。",
  "history_provenance": {
    "evidence_tier": "raw_observation_transitional",
    "dominant_source": "raw_observation_replay",
    "total_points": 180,
    "feature_backed_points": 132,
    "raw_observation_points": 48,
    "snapshot_bridge_points": 0,
    "runtime_only_points": 0,
    "latest_feature_backed_date": "2026-05-28",
    "latest_raw_observation_date": "2026-05-30",
    "latest_snapshot_bridge_date": null,
    "latest_replay_run_id": "replay:financial_system:20260609T101500Z",
    "note": "默认历史轨迹已经避开旧 snapshot bridge，但仍有 48/180 个点只是 raw observation 过渡口径，说明 replay 还没有完全绑定到 persisted PIT feature snapshot。",
    "sources": [
      {
        "source_id": "raw_pit_feature_replay",
        "count": 132,
        "latest_as_of_date": "2026-05-28",
        "note": "这类点已经绑定到已落库的 PIT feature snapshot，可作为 formal history 审计的正式证据层。"
      }
    ]
  },
  "protected_stress_window_catalog": {
    "catalog_id": "us_protected_stress_windows_v1",
    "market_scope": "us",
    "note": "这些区间用于把应允许保护性减仓或对冲的系统压力阶段，与真正的纯误报区分开来。它们服务于滚动审计解释，不直接改变当前时点的 posture 结论。",
    "source": "embedded:config/protected_stress_windows.us.json",
    "warning": null,
    "windows": [
      {
        "window_id": "us_rate_shock_2022",
        "label": "2022 联储加息与流动性抽紧",
        "start_date": "2022-03-01",
        "end_date": "2022-10-31",
        "note": "这是利率冲击主导的系统性压力窗口，应与纯误报分开统计。"
      }
    ]
  }
}
```

补充约定：

- `actionability_enabled=true` 表示当前 release 内置独立动作头，前端可以直接展示 `prepare / hedge / defend`；
- `actionability_enabled=false` 表示当前仍由旧逻辑根据危机先验概率和评分诊断映射动作概率，前端需要明确标注为过渡解释；
- `fusion_policy_version` 只说明 serving 端如何把动作头并入 `time_to_risk_bucket / posture`，不代表该模型已经通过正式准入护栏。

## 5. 类型定义

### 5.1 ActionabilityBlock

```text
prepare: number
hedge: number
defend: number
```

范围：

- `0.0` 到 `1.0`

含义：

- `prepare`：数月到数周级的预备动作概率
- `hedge`：未来几周应主动增加保护的动作概率
- `defend`：近端风险窗口已经打开、需要资本保全的动作概率

### 5.2 ProbabilityBlock

```text
p_5d: number
p_20d: number
p_60d: number
```

范围：

- `0.0` 到 `1.0`

### 5.3 TimeToRiskBucket

```text
normal | months | weeks | now
```

### 5.4 Posture

```text
normal | prepare | hedge | defend
```

### 5.5 HistoricalAnalog

```text
scenario_id
name
similarity_score
reference_phase
note
peak_score
lead_time_days
actionable_lead_time_days
```

### 5.6 DataTrust

```text
coverage_score
core_feature_coverage
trigger_feature_coverage
external_feature_coverage
quality_grade
data_quality_summary
warnings[]
```

### 5.7 JpyCarrySnapshot

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

### 5.8 PositionGuidance

```text
action_playbook_version
execution_urgency
confidence_gate
target_equity_exposure_pct
target_cash_pct
hedge_ratio_pct
leverage_cap_pct
option_overlay_pct
action_summary
actions[]
forbidden_actions[]
reentry_conditions[]
guardrails[]
capital_preservation_overlay_enabled
governance.system_budget_only
governance.auto_execution_allowed
governance.manual_confirmation_required
governance.policy_change_requires_release_review
governance.policy_change_requires_go_no_go
governance.required_operator_checks[]
```

说明：

- 这是系统级仓位预算和保护建议。
- 不能当成用户个性化投资建议或自动交易指令。
- `governance` 是动作层治理边界，明确这套输出只能作为系统预算建议，任何规则升级都要先经过 `release review` 与正式 `Go/No-Go`。

### 5.9 RuntimeMetadata

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

### 5.10 KeyIndicatorStatus

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

### 5.11 EventAssessment

```text
state
confirmation_score
recent_event_count
summary
confirmed_signals[]
pending_gaps[]
recent_events[]
```

### 5.12 BacktestPerformanceSummary

```text
scenario_count
real_scenario_count
fallback_scenario_count
coverage_scope_note
structural_warning_rate
timely_warning_rate
missed_rate
avg_structural_lead_time_days
avg_lead_time_days
median_lead_time_days
total_false_positive_count
history_start
history_end
rolling_audit
summary
```

说明：

- `lead_time_days`：结构性抬升提前量。
- `actionable_lead_time_days`：可执行预警提前量。
- `real_scenario_count` / `fallback_scenario_count`：统计的是“危机场景目录里有多少样本被当前场景回测历史窗口直接覆盖”，不是默认历史轨迹是否已经进入 PIT 正式证据层。
- `coverage_scope_note`：把“场景回测历史窗口”与“危机场景目录覆盖”这两个口径拆开说明，避免把最近 260 个 PIT 点和更早危机场景样本混为一谈。
- `timely_warning_rate`：按可执行预警口径统计，不再把仅有结构性脆弱的样本算作动作级命中。
- `total_false_positive_count` 仍表示场景内 `预警折返/动作信号回落次数`。
- 真正的全样本滚动审计在 `rolling_audit` 中展示，并区分受保护压力窗口与纯误报。

### 5.13 BacktestRollingAudit

```text
history_point_count
actionable_signal_count
pre_crisis_signal_count
in_crisis_signal_count
stress_window_signal_count
false_positive_signal_count
false_positive_episode_count
longest_false_positive_episode_days
actionable_precision
classified_episodes[]
summary
```

说明：

- `stress_window_signal_count` 表示动作信号落在受保护压力窗口中的点数，例如 `2009` 余震、`2015-2016` 美元流动性压力、`2022` 联储加息冲击。
- `false_positive_signal_count` 只统计真正的纯误报。
- `actionable_precision` 当前按 `(危机前命中 + 受保护压力窗口) / (危机前命中 + 受保护压力窗口 + 纯误报)` 计算。
- `classified_episodes` 只返回最长的若干段非危机动作区间，供前端解释最需要复盘的阶段。

### 5.14 BacktestRollingAuditEpisode

```text
start_date
end_date
duration_days
signal_count
classification
note
```

说明：

- `classification` 当前取值：
  - `stress_window`
  - `false_positive`

### 5.15 AssessmentMethodResponse

```text
method
note
history_provenance
protected_stress_window_catalog
runtime_thresholds
```

说明：

- `history_provenance` 用于解释当前默认历史轨迹到底有多少点已经绑定到 PIT feature snapshot、还有多少点仍是 raw observation 过渡口径，或者是否还残留旧 prediction snapshot bridge；
- `protected_stress_window_catalog` 用于解释滚动审计里哪些非危机动作区间被视为“可接受的系统压力防守”。

### 5.15.1 HistoryProvenanceSummary

```text
evidence_tier
dominant_source
total_points
feature_backed_points
raw_observation_points
snapshot_bridge_points
runtime_only_points
latest_feature_backed_date
latest_raw_observation_date
latest_snapshot_bridge_date
latest_replay_run_id
note
sources[]
```

说明：

- `evidence_tier` 当前取值包括：
  - `pit_feature_backed`
  - `raw_observation_transitional`
  - `snapshot_bridge_transitional`
  - `runtime_only`
- `sources[]` 会进一步列出每种 `history_source` 的点数、最近日期和解释文案，供方法页和审计页说明“这条概率轨迹能不能当正式历史证据”。

### 5.16 ProtectedStressWindowCatalog

```text
catalog_id
market_scope
note
source
warning
windows[]
```

说明：

- 默认目录文件位于 `config/protected_stress_windows.us.json`。
- 若设置 `FC_PROTECTED_STRESS_WINDOWS_PATH`，API 会优先读取外部文件；失败时回退到内置目录，并在 `warning` 中说明。

### 5.17 ProtectedStressWindow

```text
window_id
label
start_date
end_date
note
```

### 5.18 UserRiskPreferences

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

## 10. 2026-05-31 新增研究审计接口

为支撑 formal bundle 上线与日常复盘，当前实现新增：

```text
GET /api/research/audit
```

响应包含：

- `supported`
- `storage_mode`
- `market_scope`
- `active_release_id`
- `runtime_probability_mode`
- `runtime_release_status`
- `history_provenance`
- `latest_snapshot_date`
- `latest_replay_run_id`
- `latest_release_review`
- `note`
- `releases[]`
- `replay_runs[]`
- `snapshots[]`

用途：

- 核对当前 API 实际在跑 `heuristic_mvp` 还是 `formal_bundle_v1`
- 直接查看当前默认历史轨迹到底是 `PIT feature-backed` 正式证据、`raw observation` 过渡口径，还是仍残留旧 `snapshot bridge`
- 查看本地 release registry 中有哪些 candidate / approved / active 版本
- 查看最近一条 replay run 是否和当前 runtime / active release 对得上
- 查看 `prediction snapshots` 是否跟 active release 对齐

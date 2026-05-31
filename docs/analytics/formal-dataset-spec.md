# 正式训练数据集规格

状态：`Draft`

最后更新：2026-05-31

## 1. 目标

定义最终正式概率模型要吃什么数据，而不是继续依赖“从 `prediction snapshots` 倒推训练集”的过渡方式。

这份文档回答：

1. 正式训练集的行、列和版本应该长什么样。
2. `1990+` 主面板和 `1987` 扩展场景是否使用同一数据集。
3. 什么样的样本允许进入 `formal_v1`。

## 2. 设计原则

- 正式训练集必须来自原始观测 -> PIT 特征 -> 标签流水线。
- 训练集必须显式记录 `visibility_mode`、覆盖率和数据缺口。
- 第一版主训练集只做 `1990+` 统一日频主面板。
- `1987 / 1998` 扩展场景单独建包，不强行并入主宽表。
- 训练切分只能按时间，不允许随机打散。

## 3. 数据集族

第一阶段固定三类：

| 数据集 ID | 含义 | 用途 |
|---|---|---|
| `formal_v1_main_1990_daily` | `1990+` 美国主线统一日频主面板 | 正式 `p_5d / p_20d / p_60d` 主训练集 |
| `formal_v1_ext_acute_pre1990` | `1987 / 1998` 扩展急性冲击包 | 短窗研究、historical analog、专项校验 |
| `formal_v1_eval_all` | 主面板 + 扩展样本的评估视图 | 审计与对照，不直接作为统一训练输入 |

## 4. 行粒度

主数据集按：

```text
entity_id = us
as_of_date = 每个交易日一行
```

第一版保留宽表，一行同时带：

```text
label_5d
label_20d
label_60d
```

## 5. 元数据列

每行至少要有：

```text
dataset_id
dataset_version
entity_id
as_of_date
market_scope
visibility_mode
feature_set_version
label_version
scenario_set_version
coverage_score
core_feature_coverage
trigger_feature_coverage
external_feature_coverage
latest_visible_at
sample_quality_grade
```

## 6. 特征列

### 6.1 `formal_v1_main_1990_daily` 最小核心特征

```text
us_vix_level
us_vix_change_5d
us_treasury_10y_level
us_treasury_2y_level
us_curve_10y2y
us_baa_10y_spread
us_fed_funds_level
us_nfci_level
us_stlfsi_level
us_unemployment_level
us_industrial_production_level
us_housing_starts_level
us_usdjpy_level
us_usdjpy_change_20d
structural_score
trigger_score
external_shock_score
```

### 6.2 第二阶段可选增强

```text
us_sec_banking_event_count
us_sec_liquidity_keyword_score
banking_event_market_coupling
us_jpy_carry_rate_diff
historical_analog_distance_top1
```

要求：

- 第二阶段增强特征不能反过来变成第一阶段训练集的硬依赖。

## 7. 标签列

```text
label_5d
label_20d
label_60d
primary_scenario_id nullable
scenario_family nullable
```

规则：

- 标签来自 [危机场景目录](scenario-catalog.md)
- 默认 `label_set = formal_label_v1_main`
- 若构建扩展包，使用 `label_set = formal_label_v1_ext_acute`

## 8. 样本纳入规则

主训练集一行样本要进入 `formal_v1_main_1990_daily`，至少满足：

1. `as_of_date >= 1990-01-02`
2. `visibility_mode = best_effort` 或更严格
3. `coverage_score >= 0.85`
4. `core_feature_coverage >= 0.90`
5. `trigger_feature_coverage >= 0.75`
6. `external_feature_coverage >= 0.70`
7. 关键特征不得同时缺失：
   - `us_vix_level`
   - `us_curve_10y2y`
   - `us_baa_10y_spread`
   - `us_fed_funds_level`

若任一条件不满足：

- 主训练集剔除；
- 但可保留到评估或审计视图。

## 9. 扩展场景包规则

`formal_v1_ext_acute_pre1990` 只保留：

- `1987`
- `1998`

且仅使用可跨时代稳定取得的代理特征：

```text
proxy_gs10_level
proxy_gs2_level
proxy_curve_10y2y_monthly
proxy_baa_10y_spread
proxy_fed_funds_level
proxy_nfci_level
proxy_usdjpy_level
proxy_unemployment_level
```

用途：

- 检查短窗风险逻辑；
- 验证历史类比；
- 不与主面板直接拼成一套统一宽表模型。

## 10. 缺失值策略

第一阶段固定：

| 场景 | 规则 |
|---|---|
| 慢变量短期未更新 | 允许前值保持，并打 `feature_forward_filled` |
| 快变量缺失 1 天 | 可局部前值保持，但该样本降级 |
| 快变量连续缺失 >= 2 天 | 主训练集剔除 |
| 代理特征替代主特征 | 必须打 `feature_proxy_used` |

要求：

- 缺失值处理必须写入数据集 manifest；
- 不能在训练脚本里偷偷做无记录的填补。

## 11. 数据集切分

第一版建议的默认时间切分：

| 切分 | 日期范围 | 用途 |
|---|---|---|
| `train` | `1990-01-02` 到 `2014-12-31` | 模型拟合 |
| `calibration` | `2015-01-01` 到 `2019-12-31` | 概率校准 |
| `evaluation` | `2020-01-01` 到当前 | 最终评估 |

原因：

- 不把 `2020 / 2023` 混进训练段；
- 保留现代样本用于真实 out-of-sample 检查；
- 避免时间泄漏。

## 12. 数据集产物

每个数据集版本至少导出：

```text
dataset_manifest.json
dataset_summary.json
dataset_train.parquet
dataset_calibration.parquet
dataset_evaluation.parquet
feature_manifest.json
label_manifest.json
coverage_report.json
```

## 13. 当前过渡方案与最终方案的关系

当前代码里已经存在一条过渡路径：

```text
prediction snapshots -> dataset rows -> transitional formal bundle
```

这条链路仍可保留，但只能算：

- bootstrap
- 工程链路验证
- release/audit 验证

不能再把它当最终正式数据集。

正式方案必须变成：

```text
raw observations
-> point-in-time visibility filtering
-> feature snapshots
-> scenario labels
-> formal dataset
-> train / calibrate / evaluate
```

## 14. 数据集准入失败的处理

若数据集不满足最小门槛：

- 不允许发布为 `candidate release`
- 可以输出 `research_only` 报告
- API 不得把其标记为正式概率模型来源

## 15. 对实现的直接要求

至少要有：

```text
research_feature_snapshots
research_formal_datasets
research_formal_dataset_rows
research_formal_dataset_reports
```

Worker 至少支持：

```text
research dataset build-main
research dataset build-extension
research dataset report
```

## 16. 下一步

文档补齐后，后续编码顺序应是：

1. 原始观测补 `visible_at`
2. 生成正式 feature snapshot
3. 场景目录驱动标签
4. 构建 `formal_v1_main_1990_daily`
5. 再替换当前基于 `prediction snapshots` 的过渡训练

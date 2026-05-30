# 可观测性设计

状态：`Draft`

最后更新：2026-05-30

## 1. 目标

定义日志、指标、追踪和告警设计，确保系统能回答：

- 数据是否按时抓取？
- 哪个数据源失败？
- 哪个指标质量下降？
- 风险评分是否正常产出？
- API 和前端是否可用？
- 预警事件是否按规则生成？

## 2. 观测对象

```text
data ingestion
data quality
scoring engine
api service
frontend
database
alert engine
external data sources
```

## 3. 日志

日志采用结构化 JSON。

核心字段：

```text
timestamp
level
service
environment
trace_id
run_id
source_id
dataset_id
indicator_id
event_type
message
error_type
```

日志级别：

- `debug`：本地开发。
- `info`：任务开始、任务成功、评分完成。
- `warn`：质量警告、延迟、可重试失败。
- `error`：终止失败、发布失败。
- `fatal`：服务无法启动。

禁止记录：

- API key。
- 数据库密码。
- 未脱敏用户 token。

## 4. 指标

建议暴露 Prometheus 指标。

### 4.1 抓取指标

```text
ingestion_runs_total{source,status}
ingestion_run_duration_seconds{source,dataset}
ingestion_records_written_total{source,dataset}
ingestion_failures_total{source,error_type}
ingestion_last_success_timestamp{source,dataset}
ingestion_lag_seconds{source,dataset}
```

### 4.2 数据质量指标

```text
data_quality_score{source,indicator}
data_quality_failures_total{rule,severity}
stale_indicators_total{dimension}
quarantined_batches_total{source}
prototype_source_indicators_total
```

### 4.3 评分指标

```text
scoring_runs_total{status,method_version}
scoring_duration_seconds{scope}
risk_score_current{entity,dimension}
risk_level_current{entity,dimension}
insufficient_data_dimensions_total
```

### 4.4 API 指标

```text
http_requests_total{method,path,status}
http_request_duration_seconds{method,path}
api_errors_total{path,error_type}
```

## 5. 追踪

一次抓取到评分的链路应共享 trace context：

```text
scheduled job
  fetch raw
  parse
  quality check
  publish observations
  compute features
  score
  create alert
```

后续 Rust 实现建议使用 `tracing`，并预留 OpenTelemetry 接入。

## 6. 告警规则

### 6.1 数据源告警

- P0 数据源连续失败 3 次。
- P0 数据源超过 freshness SLO。
- 原始响应保存失败。
- schema 变化导致解析失败。

### 6.2 数据质量告警

- 核心指标质量低于 C。
- 同一维度超过 30% 指标 stale。
- quarantine 批次数量连续增长。

### 6.3 评分告警

- 每日评分任务未完成。
- 某维度数据不足。
- 风险分异常跳变且无指标解释。

### 6.4 系统告警

- API 5xx 超过阈值。
- 数据库连接失败。
- worker 无心跳。
- 磁盘空间不足。

## 7. 运维面板

Grafana 内部面板建议：

- 数据源健康总览。
- 抓取任务成功率。
- 数据延迟热力图。
- 质量分趋势。
- API 延迟和错误率。
- worker 心跳。
- 数据库容量。

注意：Grafana 是内部运维面板，不替代用户风险面板。

## 8. SLO 草案

MVP 阶段建议：

| 对象 | SLO |
|---|---|
| API 可用性 | 99% |
| P0 日频数据延迟 | 2 个交易日内 |
| P0 月频数据延迟 | 45 个自然日内 |
| 每日评分任务 | 每日成功一次 |
| 原始响应追溯 | 99.9% 标准化记录可追溯 |

## 9. 事件关联

金融风险事件页面应能关联：

- 评分运行 ID。
- 指标质量。
- 数据源抓取状态。
- 方法版本。

这样用户可以判断预警是否由真实风险驱动，还是由数据异常驱动。


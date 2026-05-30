# 数据库 Schema 设计

状态：`Draft`

最后更新：2026-05-30

## 1. 目标

设计第一版存储模型，覆盖数据源、指标元数据、抓取任务、原始数据、标准化时序、质量检查、风险评分、预警事件和审计信息。

本文档描述逻辑 schema，不要求第一版完全照搬表名，但后续实现应保持边界一致。

## 2. 存储分层

```text
PostgreSQL
  metadata    数据源、指标、实体、映射、日历、配置
  ingest      抓取任务、运行记录、水位线
  raw         原始响应索引，不直接存大对象
  staging     解析后的候选记录
  quality     数据质量规则和检查结果
  analytics   风险评分、特征、回测快照
  alerts      预警事件和状态
  audit       配置变更和人工操作

TimescaleDB
  ts.indicator_observations
  ts.risk_scores
  ts.feature_values

Object/Parquet storage
  raw responses
  backtest snapshots
  large model datasets
```

第一版可以先用 PostgreSQL 表承载全部数据，保留 TimescaleDB hypertable 迁移路径。

## 3. 元数据表

### 3.1 `metadata.sources`

数据源注册表。

核心字段：

```text
source_id              text primary key
display_name           text
source_type            text
official_url           text
documentation_url      text
access_method          text
auth_required          boolean
auth_secret_ref        text nullable
rate_limit_policy      jsonb
license_note           text
commercial_use_status  text
production_allowed     boolean
enabled                boolean
created_at             timestamptz
updated_at             timestamptz
```

### 3.2 `metadata.datasets`

数据源下的数据集。

```text
dataset_id             text primary key
source_id              text references metadata.sources
display_name           text
frequency_set          text[]
region_set             text[]
supports_backfill      boolean
supports_incremental   boolean
supports_vintage       boolean
expected_latency       interval
config_version         text
enabled                boolean
```

### 3.3 `metadata.indicators`

内部指标定义。

```text
indicator_id           text primary key
display_name           text
domain                 text
subdomain              text
description            text
unit                   text
currency               text nullable
frequency              text
risk_direction         text
default_transform      text
default_source_id      text
quality_tier           text
enabled                boolean
```

`risk_direction` 可选：

- `higher_is_riskier`
- `lower_is_riskier`
- `two_sided`
- `falling_fast_is_riskier`
- `rising_fast_is_riskier`
- `manual_rule`

### 3.4 `metadata.external_indicator_mappings`

外部指标代码映射。

```text
mapping_id             uuid primary key
indicator_id           text references metadata.indicators
source_id              text
dataset_id             text
external_code          text
external_params        jsonb
valid_from             date
valid_to               date nullable
priority               integer
```

### 3.5 `metadata.entities`

国家、市场、资产、机构等实体。

```text
entity_id              text primary key
entity_type            text
display_name           text
iso_country_code       text nullable
currency               text nullable
parent_entity_id       text nullable
metadata               jsonb
```

### 3.6 `metadata.calendars`

交易日和发布日历。

```text
calendar_id            text primary key
region                 text
calendar_type          text
date                   date
is_open                boolean
note                   text nullable
```

## 4. 抓取表

### 4.1 `ingest.jobs`

调度任务定义。

```text
job_id                 uuid primary key
source_id              text
dataset_id             text
target_id              text nullable
run_mode               text
schedule               text
priority               integer
enabled                boolean
next_run_at            timestamptz
config_version         text
```

### 4.2 `ingest.runs`

每次执行记录。

```text
run_id                 uuid primary key
job_id                 uuid nullable
source_id              text
dataset_id             text
target_id              text nullable
run_mode               text
status                 text
started_at             timestamptz
finished_at            timestamptz nullable
attempt                integer
watermark_before       jsonb
watermark_after        jsonb
records_read           integer
records_written        integer
error_type             text nullable
error_message          text nullable
```

### 4.3 `ingest.watermarks`

```text
source_id
dataset_id
target_id
last_successful_period
last_publication_time
last_revision_time
last_run_id
updated_at
```

主键：

```text
source_id + dataset_id + target_id
```

## 5. 原始数据表

### 5.1 `raw.objects`

只保存索引和 hash，大对象保存到文件系统、对象存储或压缩归档。

```text
raw_payload_id         uuid primary key
run_id                 uuid references ingest.runs
source_id              text
dataset_id             text
request_url            text
request_params_hash    text
response_hash          text
content_type           text
content_length         bigint
raw_object_uri         text
fetched_at             timestamptz
```

## 6. 时序表

### 6.1 `ts.indicator_observations`

标准化指标观测值。

```text
indicator_id           text
entity_id              text
as_of_date             date
period_start           date nullable
period_end             date nullable
frequency              text
value                  numeric
unit                   text
currency               text nullable
source_id              text
dataset_id             text
revision_time          timestamptz nullable
publication_time       timestamptz nullable
raw_payload_id         uuid
quality_score          numeric
quality_flags          text[]
created_at             timestamptz
```

建议唯一约束：

```text
indicator_id + entity_id + as_of_date + frequency + source_id + coalesce(revision_time)
```

TimescaleDB hypertable 时间列建议使用 `as_of_date`。

### 6.2 `ts.feature_values`

评分前的特征值。

```text
feature_id
indicator_id
entity_id
as_of_date
feature_name
feature_value
lookback_window
method_version
quality_score
```

### 6.3 `ts.risk_scores`

风险评分输出。

```text
score_id
score_scope
entity_id
as_of_date
dimension
score
level
method_version
top_contributors jsonb
explanation jsonb
quality_score
created_at
```

## 7. 质量表

### 7.1 `quality.rules`

```text
rule_id
rule_name
scope_type
scope_id
severity
config
enabled
```

### 7.2 `quality.check_results`

```text
check_id
run_id
indicator_id
entity_id
as_of_date
rule_id
status
severity
message
observed_value
expected_value
created_at
```

## 8. 预警表

### 8.1 `alerts.events`

```text
alert_id
entity_id
scope
dimension
level
status
triggered_at
triggered_as_of_date
resolved_at
score
trigger_reason
contributors jsonb
related_indicators text[]
method_version
```

### 8.2 `alerts.event_history`

记录升级、降级、确认、备注和解除。

```text
history_id
alert_id
event_type
from_status
to_status
actor
note
created_at
```

## 9. 审计表

配置、权重、阈值、数据源启停都要审计。

```text
audit.config_changes
  change_id
  object_type
  object_id
  before_value
  after_value
  actor
  reason
  created_at
```

## 10. 分区和索引建议

重点索引：

- `indicator_observations(indicator_id, entity_id, as_of_date desc)`
- `indicator_observations(source_id, dataset_id, as_of_date desc)`
- `risk_scores(entity_id, score_scope, as_of_date desc)`
- `ingest.runs(source_id, dataset_id, status, started_at desc)`
- `alerts.events(status, level, triggered_at desc)`

如果使用 TimescaleDB：

- `indicator_observations` 按 `as_of_date` 建 hypertable。
- 高频数据可以增加 `entity_id` 或 `indicator_id` 空间分区。
- 常用日频面板查询可建 continuous aggregate。

## 11. 保留策略

建议：

- 原始响应：永久保留 P0 数据源，P1/P2 可按体积设保留期。
- 标准化时序：永久保留。
- 抓取运行记录：至少保留 3 年。
- 质量检查结果：至少保留 3 年。
- API 请求日志：按安全策略保留 90 到 180 天。
- 回测快照：按模型版本保留。


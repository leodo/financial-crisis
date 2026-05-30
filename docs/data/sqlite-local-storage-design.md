# SQLite 本地存储方案

状态：`Draft`

最后更新：2026-05-30

## 1. 目标

定义本地 SQLite 存储的第一版表结构边界、运行参数和迁移路径，使系统可以在无 PostgreSQL 的情况下完成：

- 免费历史数据抓取。
- 本地持久化。
- 风险评分。
- Web 面板展示。
- 历史回测。

## 2. 数据库文件

默认位置：

```text
data/fc-local.sqlite
```

环境变量：

```text
FC_DATA_MODE=sqlite
FC_SQLITE_PATH=data/fc-local.sqlite
```

## 3. SQLite pragma

连接初始化时执行：

```sql
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;
PRAGMA synchronous = NORMAL;
```

说明：

- WAL 用于读写并发体验。
- foreign keys 默认开启，避免孤儿记录。
- busy timeout 避免短时间写锁导致前端或任务直接失败。
- 所有写入仍必须通过单 writer 控制。

## 4. 第一版核心表

### 4.1 元数据

```text
metadata_sources
metadata_datasets
metadata_indicators
metadata_external_indicator_mappings
metadata_entities
metadata_calendars
```

### 4.2 抓取

```text
ingest_jobs
ingest_runs
ingest_watermarks
```

### 4.3 原始响应

```text
raw_responses
```

核心字段：

```text
raw_response_id
source_id
dataset_id
request_url
request_params_hash
http_status
content_type
response_hash
raw_file_path
fetched_at
parser_version
```

### 4.4 标准化时序

SQLite 表名：

```text
ts_indicator_observations
```

核心字段：

```text
indicator_id
entity_id
as_of_date
period_start
period_end
frequency
value
unit
source_id
dataset_id
publication_time
revision_time
vintage_date
quality_score
quality_flags_json
raw_response_id
```

唯一键：

```text
indicator_id + entity_id + as_of_date + source_id + vintage_date
```

### 4.5 分析结果

```text
analytics_feature_values
analytics_risk_snapshots
analytics_indicator_risks
analytics_backtest_runs
analytics_backtest_points
```

### 4.6 预警和审计

```text
alerts_events
audit_log
```

## 5. 索引

第一版必须建索引：

```text
ts_indicator_observations(indicator_id, entity_id, as_of_date)
ts_indicator_observations(entity_id, as_of_date)
ingest_runs(source_id, dataset_id, started_at)
ingest_watermarks(source_id, dataset_id, target_id)
analytics_risk_snapshots(entity_id, market_scope, as_of_date)
alerts_events(triggered_at, level)
```

## 6. JSON 字段

SQLite 中 JSON 先以 text 保存：

- `rate_limit_policy_json`
- `external_params_json`
- `metadata_json`
- `quality_flags_json`
- `top_contributors_json`

规则：

- 关键查询字段必须拆成结构化列。
- JSON 只放扩展信息，不作为核心 join 条件。
- 写入前由 Rust 类型序列化，读取后由 Rust 类型反序列化和校验。

## 7. Raw 文件策略

原始响应不直接大批量写入 SQLite。

```text
data/raw/<source_id>/<yyyy>/<mm>/<request_hash>.<json|csv>.gz
```

数据库保存：

- 相对路径。
- hash。
- content type。
- parser version。
- response size。

这样可以：

- 让 SQLite 保持轻量。
- 支持重新解析。
- 支持后续迁移对象存储或 Parquet。

## 8. 锁和事务

写入规则：

- 抓取完成后，每个 source batch 一个事务。
- 事务中只做 DB 写入，不做网络请求。
- 大批历史回填按日期或分页分批提交。
- Web API 只读连接不持有长事务。

## 9. 备份

推荐命令层能力：

```text
just db-backup
just db-check
just db-vacuum
```

第一版可以先文档化，后续再实现。

备份内容：

- `data/fc-local.sqlite`
- `data/raw/`
- migration version。

## 10. 与 PostgreSQL 的差异

| 主题 | SQLite 本地版 | PostgreSQL 生产版 |
|---|---|---|
| schema namespace | 表名前缀 | schema |
| JSON | text + Rust 校验 | jsonb |
| 时间类型 | text/整数时间戳 | timestamptz |
| 并发写 | 单 writer | 多连接事务 |
| 大对象 | 文件系统 | 对象存储/表索引 |
| 时序优化 | 普通索引 | TimescaleDB hypertable 可选 |

## 11. 实现提示

- Rust 推荐通过 `sqlx` 增加 SQLite feature。
- migrations 分为 `migrations/sqlite` 和 `migrations/postgres`。
- Store trait 负责屏蔽 SQL 方言差异。
- API 的 `FC_DATA_MODE` 增加 `sqlite`。
- Worker 默认写 SQLite，除非用户显式配置 PostgreSQL。

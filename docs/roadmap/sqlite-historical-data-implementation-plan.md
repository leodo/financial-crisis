# SQLite 历史数据实现路线

状态：`Draft`

最后更新：2026-05-30

说明：本文档保留为 SQLite 专项实施背景与阶段拆分说明。若这里新增了“当前仍在推进”的任务，必须同步镜像回：

- [危机概率评估设计 TODO](crisis-probability-design-todo.md)，或
- [工程维护性 TODO](engineering-maintainability-todo.md)

## 1. 审查结论

现有设计文档已经能回答“免费历史数据和 SQLite 是否可落地”，但还不足以直接进入编码。缺少的实现级信息包括：

- 哪些 Rust crate 和 app 先改。
- SQLite 最小 migration 应该包含哪些表。
- `PostgresStore` 和未来 `SqliteStore` 如何共享接口。
- 首个真实数据源如何从抓取写入本地库。
- 开发命令和验收标准如何定义。

本文档补齐后，后续开发可以按阶段拆 PR。

## 2. 当前代码基线

当前实现状态：

- `crates/storage` 只有 `PostgresStore`，直接依赖 `sqlx::PgPool`。
- `apps/api` 支持 `FC_DATA_MODE=postgres`，否则使用 demo 数据。
- `crates/ingestion` 已有 `FredConnector`，可以 fetch/parse FRED observations，但还未落库。
- `apps/worker` 目前只跑 `MockConnector` demo。
- `migrations/0001_init.sql` 是 PostgreSQL/TimescaleDB 版本，没有 SQLite migration。

## 3. 开发目标

第一阶段目标不是接完所有数据源，而是打通一个真实数据闭环：

```text
SQLite init
  -> seed metadata
  -> FRED historical backfill
  -> load observations from SQLite
  -> scoring engine
  -> API / Web dashboard
```

完成后，demo 数据仍保留，但用户可以切换到真实本地历史库。

## 4. 阶段拆分

### Phase 0：配置和命令

新增环境变量：

```text
FC_DATA_MODE=demo|sqlite|postgres
FC_SQLITE_PATH=data/fc-local.sqlite
# Optional only for official FRED API mode. Default FRED graph CSV backfill needs no key.
FRED_API_KEY=<optional user local secret>
FC_RAW_DATA_DIR=data/raw
```

新增建议命令：

```text
just db-init
just db-seed
just backfill-fred
just score-local
just db-check
```

验收：

- 默认 `backfill fred` 不依赖 `FRED_API_KEY`；只有 `backfill fred --api` 未配置 key 时给出明确错误，不 panic。
- `just db-init` 可以重复运行。
- `data/`、`data/raw/` 默认进入 `.gitignore`。

### Phase 1：Storage trait

在 `crates/storage` 引入 trait，避免业务层绑定具体数据库：

```text
RiskStore
  load_indicators()
  load_observations(entity_id, as_of_date)
  upsert_indicator(indicator)
  insert_observations(observations)
  save_ingestion_run(run)
  upsert_raw_response(raw)
  load_watermark(source_id, dataset_id, target_id)
  save_watermark(...)
```

实现：

- `PostgresStore` 继续保留。
- 新增 `SqliteStore`。
- 解析/格式化枚举逻辑抽到共享函数，避免复制。

验收：

- 现有 PostgreSQL 路径行为不变。
- `SqliteStore` 可以通过单元测试插入并读取 indicators/observations。
- API 不直接依赖 `PostgresStore` 类型。

### Phase 2：SQLite migration

新增：

```text
migrations/sqlite/0001_init.sql
```

第一版最小表：

```text
metadata_sources
metadata_datasets
metadata_indicators
metadata_external_indicator_mappings
metadata_entities
ingest_runs
ingest_watermarks
raw_responses
ts_indicator_observations
analytics_risk_snapshots
```

暂缓表：

```text
alerts_events
audit_log
analytics_backtest_points
quality_results
```

原因：第一阶段先完成数据读取和评分闭环，预警审计和完整回测点可以在第二阶段补。

验收：

- migration 可重复应用到空 SQLite 文件。
- 建立必要唯一键和索引。
- `PRAGMA journal_mode=WAL`、`foreign_keys=ON` 由连接初始化执行。

### Phase 3：Metadata seed

新增 seed 能力：

```text
crates/storage/src/seed.rs
```

初始 seed：

- `fred` source。
- `fred_series_observations` dataset。
- `us` entity。
- 第一批 FRED indicators。
- `metadata_external_indicator_mappings`。

验收：

- `just db-seed` 可重复运行。
- seed 后 API 能读取指标元数据。
- 外部 series code 不直接作为内部 indicator id。

### Phase 4：FRED backfill

在 worker 中加入真实模式：

```text
cargo run -p fc-worker -- backfill fred --start 1990-01-01 --end today
```

第一版可以先不做完整 CLI 框架，但 worker 内部应按配置生成 FetchPlan。

流程：

1. 从 mapping 表读取 FRED series。
2. 按 series 创建 FetchPlan。
3. `FredConnector.fetch()` 获取 raw payload。
4. raw 写入 `data/raw/fred/...`。
5. raw index 写入 SQLite。
6. `FredConnector.parse()` 生成 observations。
7. observations upsert 到 `ts_indicator_observations`。
8. 更新 watermark。

验收：

- 至少 10 个 FRED 指标回填成功。
- 同一命令重复运行不产生重复记录。
- 指标 `VIXCLS`、`BAMLH0A0HYM2`、`T10Y2Y` 可在 API 里形成真实历史分位。

### Phase 5：API SQLite mode

`apps/api` 支持：

```text
FC_DATA_MODE=sqlite
FC_SQLITE_PATH=data/fc-local.sqlite
```

行为：

- 如果 SQLite 文件不存在或无数据，返回清晰错误。
- 如果数据存在，API 从 SQLite 读取 indicators/observations 并评分。
- backtests 第一版仍可使用场景摘要，但应标记为 summary，不伪装为完整回测。

验收：

- `GET /api/overview` 使用 SQLite 真实观测。
- `GET /api/indicators` 显示真实 latest observation。
- demo 模式不受影响。

## 5. 推荐实现顺序

1. `crates/storage` 增加 SQLite feature 和 `SqliteStore`。
2. 增加 `migrations/sqlite/0001_init.sql`。
3. 增加 `just db-init`、`just db-seed`。
4. seed FRED metadata 和首批指标。
5. worker 从 FRED 拉历史并写 SQLite。
6. API 增加 `FC_DATA_MODE=sqlite`。
7. Web 面板增加“真实数据 / demo 数据”状态提示。
8. 再接 World Bank、SEC、GDELT。

## 6. 第一阶段完成定义

- `just verify` 通过。
- `just db-init` 生成本地 SQLite。
- `just db-seed` 写入 metadata。
- `just backfill-fred` 写入至少 10 个指标、每个指标不少于 5 年历史。
- `FC_DATA_MODE=sqlite just api` 可以启动。
- Web 面板展示真实 SQLite 风险评分。
- README 写明默认无 key FRED CSV、Treasury 兜底源和可选 FRED API key 配置。

## 7. 暂不做事项

- 不接高频行情。
- 不做自动交易或自动清仓。
- 不做 IMF/BIS 全库镜像。
- 不做复杂机器学习模型。
- 不把 GDELT 新闻信号作为单独危机判定。

这些事项会显著增加复杂度，应等真实历史闭环稳定后再做。

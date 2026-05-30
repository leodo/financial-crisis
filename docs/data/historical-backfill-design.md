# 历史回填设计

状态：`Draft`

最后更新：2026-05-30

## 1. 目标

历史回填解决“免费数据源能否真正支撑历史危机回测”的问题。第一版要做到：

- 批量拉取 P0/P1 数据源历史数据。
- 保存原始响应和标准化时序。
- 支持失败重试和幂等 upsert。
- 支持评分引擎按任意历史日期读取数据。
- 为后续 point-in-time 回测保留 vintage/revision 字段。

## 2. 回填范围

| 优先级 | 数据源 | 第一批历史范围 | 输出粒度 | 用途 |
|---|---|---|---|---|
| P0 | FRED | 1990 至今，能更早则全量 | 日/周/月/季 | 美国核心风险指标 |
| P0 | World Bank | 1960 至今 | 年 | 全球慢变量 |
| P0 | SEC EDGAR | 2008 至今，先 CIK 白名单 | 事件/日聚合 | 公告风险事件 |
| P1 | GDELT | 2015 至今，先关键词聚合 | 日 | 新闻压力事件 |
| P0 | IMF | 2000 至今，按 dataset 能力 | 月/季/年 | 外储、国际收支、金融稳健 |
| P0 | BIS | 1990 至今，按 dataset 能力 | 月/季/年 | 信贷、银行、房地产 |

## 3. 回填模式

### 3.1 全量回填

用于首次建库或新增指标。

```text
pending -> running -> raw_saved -> parsed -> quality_checked -> published -> completed
```

要求：

- 所有请求参数可复现。
- 原始响应先保存，再解析。
- publish 前做数据质量检查。
- upsert 不能制造重复观测。

### 3.2 增量回填

用于日常更新。

```text
last_watermark -> request window -> raw_saved -> parsed -> upsert -> new_watermark
```

要求：

- 每个 `source_id + dataset_id + target_id` 有独立 watermark。
- 对可能修订的数据源设置 lookback window，例如每次回看 30-90 天。
- 如果发现历史数据变化，记录 revision event。

### 3.3 修复回填

用于 parser 修复、指标口径变化或数据源 schema 变化。

- 不直接覆盖已发布历史。
- 写入新 `parser_version` 或 `method_version`。
- 重新评分时保留旧评分快照，便于对比。

## 4. 数据源策略

### 4.1 FRED

接入方式：

- 使用 `series/observations`。
- 保存 `realtime_start`、`realtime_end`、`date`、`value`。
- FRED API key 放本地环境变量 `FRED_API_KEY`，不写入仓库。

回填策略：

- 每个 FRED series 一个 backfill job。
- 首次从 `observation_start` 拉全量。
- 增量使用 `observation_start = watermark - lookback`。
- 缺失值 `.` 不发布为有效观测，进入质量记录。

### 4.2 World Bank

接入方式：

- 按 country + indicator 分页抓取。
- 保存 country code、indicator code、date、value、decimal、unit。

回填策略：

- 年频慢变量，不参与日内触发。
- 回填 1960 至今。
- 空值年份保留缺失记录，不强行插值。

### 4.3 SEC EDGAR

接入方式：

- 使用 submissions JSON 和 company facts JSON。
- 必须设置合理 User-Agent。
- 先维护 CIK 白名单。

回填策略：

- 首批只抓系统重要银行、券商、保险、资产管理、交易所和关键上市公司。
- 保存 accession number、form、filing date、report date、primary document。
- 第一版输出日聚合指标：风险公告数量、8-K 数量、特定风险关键词数量。
- 事件抽取结果必须能回链到原始 filing。

### 4.4 GDELT

接入方式：

- 使用 DOC API 做关键词和实体过滤。
- 第一版抓时间线聚合，不保存所有新闻正文。

回填策略：

- 按日或按月窗口请求，避免超大响应。
- 输出 `news_count`、`negative_tone_avg`、`stress_keyword_count`。
- 新闻信号只作为触发性辅助，不单独触发 L4。

### 4.5 IMF/BIS

接入方式：

- 先做 dataflow/dataset discovery。
- 统一实现 SDMX connector。
- 维护外部 key 到内部指标的 mapping 表。

回填策略：

- 先接少量高价值 dataset，不做全库镜像。
- 所有维度组合必须入库，不能只保存展示名称。
- 多国家指标要记录 entity_id、country code、sector、frequency。

## 5. 幂等和唯一键

标准化观测的唯一键：

```text
indicator_id
entity_id
as_of_date
source_id
vintage_date
```

如果数据源不提供 vintage：

- `vintage_date = fetched_at::date`
- 同一日期同一 source 默认覆盖 staging，不静默覆盖 published。

## 6. 质量检查

每批回填至少检查：

- 时间序列是否倒序、重复或缺口异常。
- 数值是否为无穷、NaN、占位符或非数字。
- 单位是否匹配指标元数据。
- 频率是否符合指标定义。
- 最新数据是否超过 freshness SLO。
- 异常跳变是否超过历史阈值。

质量结果写入 `quality_results`，评分引擎读取质量等级决定是否降权。

## 7. 回测支持

评分引擎读取历史时应支持：

```text
score(as_of_date, entity_id, method_version, point_in_time_mode)
```

模式：

- `latest_available`：使用当前已知的完整历史，适合模型研究。
- `point_in_time`：只使用当时可见数据，适合真实回测。

MVP 可以先实现 `latest_available`，但表结构必须保留 `publication_time`、`revision_time`、`vintage_date`。

## 8. 实现顺序

1. 建立 SQLite 本地表。
2. 先实现 FRED 全量回填。
3. 加 World Bank 慢变量回填。
4. 加 SEC CIK 白名单和 filing 元数据。
5. 加 GDELT 日聚合。
6. 扩展 IMF/BIS SDMX connector。
7. 用真实历史数据替换 demo 回测。

## 9. 完成标准

- 本地 SQLite 至少包含 20 个核心指标。
- FRED P0 指标回填 1990 至今或可用最早日期。
- World Bank 至少覆盖美国、中国、日本、欧元区主要国家和全球慢变量。
- SEC 至少覆盖 30 个重点 CIK。
- GDELT 至少覆盖 2008、2020、2023 三个压力窗口。
- Web 面板能展示真实历史分位，不再依赖 demo 数组。

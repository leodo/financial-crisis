# 金融危机概率评估系统

当前项目处于文档优先的设计阶段，目标是逐步设计并实现一个以美国金融系统为主、以免费数据为基础的金融危机概率评估系统。

系统最终应覆盖：

- 数据抓取：接入免费或低成本宏观、市场、公告和新闻数据源。
- 数据治理：保留原始数据、标准化指标、质量检查和版本记录。
- 风险分析：生成结构脆弱性、触发压力、外部冲击、风险强度和解释。
- 概率评估：输出 `5d / 20d / 60d` 危机窗口概率、时距判断和决策 posture。
- 网页面板：展示当前整体评估、历史对照、主要驱动因素和指标细项。
- 回测验证：检验历史危机前的提前预警能力、概率校准和误报情况。

## 当前实现

当前已经落地一个可运行 MVP 骨架：

- Rust workspace：领域模型、数据抓取契约、FRED/Mock 连接器、规则评分引擎、SQLite/PostgreSQL 存储层。
- Axum API：同时提供旧风险总览接口和新的 assessment 接口，输出 `5d / 20d / 60d` 概率、time bucket、posture、历史类比和数据可信度。
- Axum API：本地 SQLite / PostgreSQL 模式支持后台定时刷新，也支持手动触发 reload，无需每次 backfill 后重启服务。
- React 面板：已切到新的决策面板，首页直接展示 posture、概率窗口、历史危机对照、仓位预算建议和日元套息风险放大器。
- SQLite：提供本地历史库 migration、FRED metadata seed、FRED/Treasury/World Bank/BOJ/SEC EDGAR/GDELT 历史回填入口，以及 `JPY carry` 所需的免费 USDJPY 回填入口。
- PostgreSQL/TimescaleDB：提供初始 migration 和生产迁移路径。
- Docker Compose：提供数据库、API 和 Web 面板组合部署草案。

## 文档入口

- [文档总览](docs/README.md)
- [全局设计](docs/architecture/global-design.md)
- [危机窗口与标签设计](docs/analytics/horizon-label-design.md)
- [危机概率引擎设计](docs/analytics/probability-engine-design.md)
- [决策支持策略设计](docs/analytics/decision-support-policy.md)
- [特征库设计](docs/analytics/feature-store-design.md)
- [概率校准设计](docs/analytics/probability-calibration-design.md)
- [真实回测执行设计](docs/analytics/real-backtest-execution-design.md)
- [历史相似阶段设计](docs/analytics/historical-analog-design.md)
- [Posture 阈值调优设计](docs/analytics/posture-threshold-tuning.md)
- [美国主线免费数据方案](docs/data/us-centric-free-data-plan.md)
- [日元套息外部风险模块设计](docs/data/jpy-carry-risk-module-design.md)
- [SEC EDGAR 连接器实现规格](docs/data/sec-edgar-connector-spec.md)
- [BOJ / USDJPY 连接器实现规格](docs/data/boj-connector-spec.md)
- [决策面板设计](docs/product/decision-dashboard-design.md)
- [Assessment API Contract](docs/product/assessment-api-contract.md)
- [方法页设计](docs/product/methodology-page-design.md)
- [危机概率评估设计 TODO](docs/roadmap/crisis-probability-design-todo.md)
- [免费数据源与抓取设计](docs/data/free-data-ingestion.md)
- [设计 TODO 总索引](docs/roadmap/design-todo.md)
- [免费数据源目录](docs/data/source-catalog.md)
- [指标体系设计](docs/analytics/indicator-taxonomy.md)
- [风险评分方法](docs/analytics/scoring-methodology.md)
- [Web 面板信息架构](docs/product/dashboard-information-architecture.md)
- [开源项目参考](docs/references/open-source-projects.md)
- [第二轮细分设计清单](docs/roadmap/second-round-design-backlog.md)
- [ADR-0001 初始架构方向](docs/decisions/0001-initial-architecture.md)

## 当前状态

- 当前已完成基础架构设计、风险强度层设计、危机概率评估主线设计和 MVP 工程骨架。
- `just dev` 会优先使用本地 SQLite 历史库；如果 `data/fc-local.sqlite` 不存在，则退回内置 demo 数据。也可以显式设置 `FC_DATA_MODE=sqlite` 和 `FC_SQLITE_PATH`。
- 设置 `FC_DATA_MODE=postgres` 和 `DATABASE_URL` 后，可从 PostgreSQL 读取指标和观测值并即时评分。
- 当前代码层已经具备免费历史数据的首批接入能力，并且已经提供新的决策面板与 assessment API。
- 当前 assessment 输出已经把 `风险强度`、`危机概率`、`time bucket`、`posture` 和 `position guidance` 分层展示，避免把一个总分误当成危机发生率。
- 当前 `/api/assessment/history` 和 `/api/backtests/timeline` 已支持 `from` / `to` / `limit` 查询参数，默认返回较长窗口，而不是旧版只看最近十几个点。
- 当前 `5d / 20d / 60d` 概率仍是启发式 MVP，不是经过正式回测校准的最终危机概率模型。

## 本地运行

### 1. 最快体验：一键启动面板

直接运行：

```powershell
just dev
```

启动后访问：

- Web 面板：<http://127.0.0.1:5173>
- API 健康检查：<http://127.0.0.1:18080/health>

查看服务状态：

```powershell
just status
```

停止后台服务：

```powershell
just stop
```

后台日志：

- `logs/api.log`
- `logs/web.log`

说明：

- 如果 `data/fc-local.sqlite` 已存在，`just dev` 默认让 API 跑在 `sqlite` 模式。
- 如果本地 SQLite 不存在，`just dev` 会退回 `demo` 模式，页面会明确显示 demo/stale warning。
- 真实评估前建议先执行 `just db-check`，确认 USDJPY、VIX、EFFR 等关键指标不是 stale 或 missing。
- 日常更新本地评估库时，优先运行 `just refresh-latest`。这个命令会刷新最近一段免费数据，并在本地 API 运行时自动触发 reload。
- 面板展示的是免费日频/周频风险数据，不是逐笔行情。解读单个数值前，先看 `data mode`、指标日期和 stale warning。
- API 默认每 `60` 秒自动重载一次本地库；也可以从前端右上角刷新按钮触发即时 reload。

### 2. 使用本地 SQLite 历史库

先初始化 SQLite 和元数据：

```powershell
just bootstrap-sqlite
```

这个命令等价于：

```powershell
just db-init   # 创建本地 SQLite 数据库
just db-seed   # 写入指标、实体、免费数据源映射
```

然后回填免费历史数据：

```powershell
just backfill-fred-range 2020-01-01 2020-12-31
just backfill-treasury-yield-range 2020-01-01 2020-12-31
just backfill-world-bank-range 1960-01-01 2024-12-31
just backfill-boj-fx-range 2020-01-01 2020-12-31
just backfill-jpy-carry-range 2020-01-01 2020-12-31
just backfill-sec-edgar-range 2026-01-01 2026-05-31
just backfill-gdelt-range 2026-03-01 2026-05-30
```

其中：

- `backfill-fred-*`：美国宏观和市场主序列，默认无 key。
- FRED 免费 CSV 已默认按窗口拆分回填；如果某个序列遇到源站 `502/504`，可以用 `--chunk-days` 和 `--external-code` 定向重试，例如 `cargo run -p fc-worker -- backfill fred --start 2026-01-02 --end 2026-05-30 --chunk-days 31 --external-code VIXCLS`。
- `backfill-treasury-yield-*`：美国国债收益率曲线官方兜底源。
- `backfill-world-bank-*`：年频慢变量。
- `backfill-boj-fx-*`：BOJ 官方 USDJPY 历史数据。
- `backfill-jpy-carry-*`：`JPY carry` 模块使用的 USDJPY 免费历史数据，优先 BOJ，失败时回退到 FRED。
- `backfill-sec-edgar-*`：SEC EDGAR 官方 filings metadata，聚合为银行公告事件特征和本地告警。
- `backfill-gdelt-*`：GDELT DOC API 的新闻压力聚合序列，仅作为 prototype 辅助信号，默认不纳入 `refresh-latest`。

回填完成后，用 SQLite 模式启动：

```powershell
just dev-sqlite
```

### 3. 历史轨迹接口

历史评估和时间线接口支持日期范围与点数限制：

```text
GET /api/assessment/history?from=2025-01-01&to=2026-05-30&limit=260
GET /api/backtests/timeline?from=2025-01-01&to=2026-05-30&limit=260
```

如果刚刚完成 backfill，不想等后台定时刷新，也可以手动触发：

```text
POST /api/system/reload
```

在打开面板前，先检查关键指标是不是最新：

```powershell
just db-check
```

这个命令会直接检查 `USDJPY`、日本隔夜拆借利率、`EFFR`、`VIX` 是否陈旧，并给出对应的免费回填命令。

如果只是日常更新最近数据，直接运行：

```powershell
just refresh-latest
```

默认行为：

- FRED / Treasury / BOJ / SEC EDGAR：默认刷新最近约 `45` 天的高频与事件数据。
- World Bank：刷新最近约 `15` 年的年频慢变量。
- 如果本地 API 正在运行，会自动调用 `POST /api/system/reload`。
- 最后自动执行一次 `db-check`，确认关键指标的新鲜度。

或者显式设置环境变量只启动 API：

```powershell
$env:FC_DATA_MODE = "sqlite"
$env:FC_SQLITE_PATH = "data/fc-local.sqlite"
just api
```

### 3. 单独启动服务

如果只想单独启动后端 API：

```powershell
just api
```

FRED 默认使用公开图表 CSV 回填，不需要 API key：

```powershell
just backfill-fred-range 2020-01-01 2020-12-31
```

如果 FRED CSV 不可用，或要用官方 API 的 vintage 字段，可选配置免费 key 后运行：

```powershell
$env:FRED_API_KEY = "your-fred-api-key"
just backfill-fred-api-range 2020-01-01 2020-12-31
```

美国国债收益率曲线也可直接从 Treasury 官方源回填，作为 FRED 利率数据兜底：

```powershell
just backfill-treasury-yield-range 2020-01-01 2020-12-31
```

World Bank 年频慢变量同样可以直接回填，不需要 key：

```powershell
just backfill-world-bank-range 1960-01-01 2024-12-31
```

BOJ 官方 USDJPY 和日本隔夜拆借利率也可以直接回填，不需要 key：

```powershell
just backfill-boj-fx-range 2020-01-01 2020-12-31
just backfill-boj-money-market-range 2020-01-01 2020-12-31
```

SEC EDGAR 银行/金融机构公告事件也可以直接回填，不需要 key：

```powershell
just backfill-sec-edgar-range 2026-01-01 2026-05-31
```

GDELT 新闻聚合也可以手动回填，不需要 key，但当前只建议作为辅助原型信号：

```powershell
just backfill-gdelt-range 2026-03-01 2026-05-30
```

使用 SQLite 数据启动 API：

```powershell
$env:FC_DATA_MODE = "sqlite"
$env:FC_SQLITE_PATH = "data/fc-local.sqlite"
just api
```

如果只想单独启动前端面板：

```powershell
just web-install
just web-dev
```

常用命令：

```powershell
just                  # 查看所有命令
just dev              # 一键启动 API + Web；有本地 SQLite 时优先使用真实库，否则 demo
just dev-sqlite       # 一键启动 API + Web，并强制 API 走本地 SQLite
just bootstrap-sqlite # 初始化 SQLite 并写入元数据
just refresh-latest   # 一键刷新最近免费数据，并在 API 运行时自动 reload
just db-check         # 检查本地 SQLite 关键指标是否足够新鲜
just status           # 查看后台服务状态，并直接显示 data mode / 最新观测 / USDJPY
just db-init          # 初始化本地 SQLite
just db-seed          # 写入 FRED/Treasury/World Bank/BOJ/SEC 元数据与映射
just backfill-fred            # 无 key FRED CSV 回填
just backfill-treasury-yield  # 无 key Treasury 收益率回填
just backfill-world-bank      # 无 key World Bank 年频回填
just backfill-sec-edgar       # 无 key SEC EDGAR 公告事件回填
just backfill-gdelt           # 无 key GDELT 新闻聚合回填（prototype）
just backfill-boj-fx          # 无 key BOJ 官方 USDJPY 回填
just backfill-boj-money-market # 无 key BOJ 官方日本隔夜拆借利率回填
just backfill-jpy-carry       # 无 key USDJPY 回填，优先 BOJ，失败时回退到 FRED
just stop             # 停止一键启动的服务
just fmt              # Rust 格式化
just test             # Rust 测试
just lint             # Rust clippy
just web-build        # 前端生产构建
just verify           # fmt + test + clippy + 前端构建
```

## 主要接口

当前前端主要依赖以下新接口：

- `/api/assessment/current`
- `/api/assessment/history`
- `/api/assessment/posture`
- `/api/assessment/data-trust`
- `/api/assessment/analogs`
- `/api/assessment/method`

其中：

- `/api/assessment/current` 同时返回 `jpy_carry` 和 `position_guidance`。
- `position_guidance` 是系统级仓位预算和保护建议，不是自动交易指令。
- `scores.overall_score` 是历史压力强度，不等于 `5d / 20d / 60d` 概率。

旧接口仍保留，主要用于兼容：

- `/api/overview`
- `/api/dimensions`
- `/api/indicators`
- `/api/sources`
- `/api/backtests`

Docker Compose 草案：

```powershell
docker compose -f deploy/docker-compose.yml up --build
```

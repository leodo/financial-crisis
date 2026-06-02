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
- [系统可行性分析](docs/architecture/system-feasibility-analysis.md)
- [代码结构与可维护性评审](docs/architecture/codebase-maintainability-review.md)
- [工程治理方案](docs/architecture/engineering-governance-plan.md)
- [危机窗口与标签设计](docs/analytics/horizon-label-design.md)
- [危机场景目录](docs/analytics/scenario-catalog.md)
- [危机概率引擎设计](docs/analytics/probability-engine-design.md)
- [决策支持策略设计](docs/analytics/decision-support-policy.md)
- [持仓动作手册设计](docs/analytics/portfolio-action-playbook.md)
- [特征库设计](docs/analytics/feature-store-design.md)
- [特征覆盖矩阵](docs/analytics/feature-coverage-matrix.md)
- [Point-in-Time 可见性规范](docs/data/point-in-time-visibility-spec.md)
- [正式训练数据集规格](docs/analytics/formal-dataset-spec.md)
- [概率校准设计](docs/analytics/probability-calibration-design.md)
- [真实回测执行设计](docs/analytics/real-backtest-execution-design.md)
- [模型发布与在线评分设计](docs/analytics/model-release-and-serving-design.md)
- [正式模型准入与 Go/No-Go](docs/analytics/model-go-no-go.md)
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
- [工程维护性 TODO](docs/roadmap/engineering-maintainability-todo.md)
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
- 当前 `/api/backtests` 已显式区分 `真实历史样本` 与 `模板参考样本`，避免把本地库尚未覆盖的历史危机误读为真实回测结果。
- 当前 `/api/backtests` 与前端回测页已把 `结构性抬升提前量` 和 `可执行预警提前量` 分开展示，避免把几个月前的脆弱性积累误读成已经给出可执行清仓信号。
- 当前 `backtest_summary.rolling_audit` 已接入全历史滚动审计，显式区分 `危机前命中`、`受保护压力窗口` 和 `纯误报`，同时展示最长的非危机动作区间，避免把 2022 这类系统压力阶段误写成纯噪声误报。
- 当前受保护压力窗口目录已经抽到 [config/protected_stress_windows.us.json](config/protected_stress_windows.us.json)，默认随构建一起发布，也可以用 `FC_PROTECTED_STRESS_WINDOWS_PATH` 覆盖。
- 当前代码已经支持 `heuristic_mvp` 和 `formal_bundle_v1` 两种 probability mode，并且支持从本地 SQLite `prediction snapshots` 训练、发布、激活和回滚正式概率 release。
- 当前 `/api/research/audit` 与前端“发布审计”页已经打通，用于核对 `release registry`、`runtime probability mode` 与每日 `prediction snapshot` 是否一致。

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

导出一份当前滚动审计报告：

```powershell
just audit-report
```

查看当前激活的 release 元数据与已落库的 prediction snapshot：

```powershell
just release-list
just snapshot-list
just snapshot-export
just snapshot-dataset
just formal-train
```

一键训练并激活 formal bundle：

```powershell
just formal-bootstrap
```

停止后台服务：

```powershell
just stop
```

后台日志：

- `logs/api.log`
- `logs/web.log`
- `reports/rolling-audit/*.md` / `*.json`

说明：

- 如果 `data/fc-local.sqlite` 已存在，`just dev` 默认让 API 跑在 `sqlite` 模式。
- 如果本地 SQLite 不存在，`just dev` 会退回 `demo` 模式，页面会明确显示 demo/stale warning。
- 真实评估前建议先执行 `just db-check`，确认 USDJPY、VIX、EFFR 等关键指标不是 stale 或 missing。
- 日常更新本地评估库时，优先运行 `just refresh-latest`。这个命令会刷新最近一段免费数据，并在本地 API 运行时自动触发 reload。
- 如果想把 GDELT 新闻聚合也一起拉进当前运行链路，可以运行 `just refresh-latest-full`。
- 面板展示的是免费日频/周频风险数据，不是逐笔行情。解读单个数值前，先看 `data mode`、指标日期和 stale warning。
- API 默认每 `60` 秒自动重载一次本地库；也可以从前端右上角刷新按钮触发即时 reload。
- `just audit-report` 会从当前运行中的 API 拉取 assessment/backtests/method，并输出一份 JSON + Markdown 审计报告；如果 API 仍是旧进程，命令会退回本地内置压力窗口目录并给出 warning。
- 在 `sqlite` 模式下，API 每次启动或 reload 时都会把当前 assessment 和历史轨迹同步写入 `analytics_prediction_snapshots`，用于后续 release 审计、方法复盘和历史对照。
- 从这一步开始，`/api/assessment/history` 在 `sqlite` 模式下会优先复用已落库的 `prediction snapshots`，只在缺口日期补算，因此 reload 的响应速度会明显快于“每次全历史重算”。
- 如果要核对线上到底是在跑 heuristic 还是 formal bundle，可以直接打开前端“发布审计”页，或者访问 `GET /api/research/audit`。

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
- `backfill-gdelt-*`：GDELT DOC API 的新闻压力聚合序列，仅作为 prototype 辅助信号；默认不纳入 `refresh-latest`，但可通过 `refresh-latest-full` 一并刷新。

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

如果要把当前 heuristic MVP 明确登记进 release registry，而不是继续依赖硬编码 method 元数据，可以运行：

```powershell
just release-bootstrap
```

随后再用：

```powershell
just release-list
just snapshot-list
just snapshot-export
just snapshot-dataset
```

确认本地 SQLite 已经有 `active release` 和 `prediction snapshot` 历史。

如果要把当前免费历史数据直接训练成一版 formal bundle，并且激活到运行中的 API：

```powershell
just formal-bootstrap
```

这个命令会：

1. 从当前 active release 的 `prediction snapshots` 构建 point-in-time 特征和危机前瞻标签；
2. 训练 `5d / 20d / 60d` 三个 horizon 的逻辑回归原始模型；
3. 用时间切分的校准集拟合 `Platt` calibration；
4. 生成 `bundle / evaluation / release manifest`；
5. 写入 SQLite release registry，并激活到当前 market scope；
6. 最后触发 API reload，使前端 method 和审计页切到新 release。

注意：第一次把一个全新的 release 激活到线上时，API 可能需要为该 `release_id` 回填全历史 `prediction snapshots`，因此 reload 可能持续数十秒。

补充：

- 如果没有显式传 `--release-id`，formal 训练默认优先使用最近一版 `heuristic_mvp` release 的快照，避免把正式模型自己的输出再次当作训练输入。
- formal release id 现在带到秒级时间戳，避免同一天重复训练时覆盖同名 artifact。
- 从本轮工程治理开始，`formal-train`、`formal-bootstrap`、`release-review`、`dataset summarize` 这类研究命令默认把生成物写到 `artifacts/research/**`，避免实验副产物长期污染 Git 工作区。
- 只有需要长期保留的证据，才显式输出到 `config/model-*/generated` 或 `reports/*` 目录。

如果只是日常更新最近数据，直接运行：

```powershell
just refresh-latest
```

如果想把 GDELT 聚合新闻压力也一并刷新：

```powershell
just refresh-latest-full
```

默认行为：

- FRED / Treasury / BOJ / SEC EDGAR：默认刷新最近约 `45` 天的高频与事件数据。
- World Bank：刷新最近约 `15` 年的年频慢变量。
- `refresh-latest-full` 会在以上基础上额外刷新最近 `90` 天内可请求的 GDELT 新闻聚合，并自动做 watermark overlap。
- 如果本地 API 正在运行，会自动调用 `POST /api/system/reload`。
- 最后自动执行一次 `db-check`，确认关键指标的新鲜度。

### 4. 为真实历史回测准备长区间历史库

如果你想让 `/api/backtests` 尽量使用本地真实历史，而不是模板参考，直接运行：

```powershell
just backfill-backtest-history
```

默认行为：

- 从 `2006-01-01` 开始逐项回填核心 `FRED / Treasury / BOJ / World Bank`
- 从 `2022-01-01` 开始回填 `SEC EDGAR`
- 最后自动执行 `just db-check`
- `backfill-backtest-history` 现在按核心指标逐条回填，避免一次性全量 FRED 扫描导致超时后看起来“好像跑过了”。
- 免费 FRED 图表 CSV 下，`us_credit_high_yield_oas` 当前实际历史只从 `2023-05-30` 开始可得；做 `2008/2020` 长历史回测时，应把它视为近年信用补充信号，而不是唯一信用主轴。

也可以指定区间：

```powershell
just backfill-backtest-history-range 2006-01-01 2026-05-31
```

完成后重载 API，再看 `/api/backtests` 中每个场景的 `样本来源` 和 `说明`，就能知道哪些已经进入真实历史覆盖，哪些仍是模板参考。

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
just refresh-latest-full # 日常刷新 + GDELT prototype 新闻聚合
just backfill-backtest-history # 构建长区间免费历史库，尽量让 backtests 使用真实历史
just db-check         # 检查本地 SQLite 关键指标是否足够新鲜
just status           # 查看后台服务状态，并直接显示 data mode / 最新观测 / USDJPY
just release-review <candidate_release_id> # 默认导出到忽略目录 artifacts/research/release-review
just release-review-tracked <candidate_release_id> # 显式导出到 reports/release-review，作为长期证据保留
just formal-train     # 默认导出到忽略目录 artifacts/research/model-*/generated
just formal-train-tracked # 显式导出到版本化 generated 目录
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

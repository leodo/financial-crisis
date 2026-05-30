# 金融危机预警系统

当前项目处于文档优先的设计阶段，目标是逐步设计并实现一个金融危机预警系统。

系统最终应覆盖：

- 数据抓取：接入免费或低成本宏观、市场、公告和新闻数据源。
- 数据治理：保留原始数据、标准化指标、质量检查和版本记录。
- 风险分析：生成整体风险评分、分项风险评分、预警等级和解释。
- 网页面板：展示当前整体评估、风险贡献、历史趋势和指标细项。
- 回测验证：检验历史危机前的提前预警能力和误报情况。

## 当前实现

当前已经落地一个可运行 MVP 骨架：

- Rust workspace：领域模型、数据抓取契约、FRED/Mock 连接器、规则评分引擎、PostgreSQL 存储层。
- Axum API：提供总览、分项风险、指标、预警、数据源和回测接口。
- React 面板：提供总览、指标库、预警记录、数据源状态和回测页面。
- PostgreSQL/TimescaleDB：提供初始 migration。
- Docker Compose：提供数据库、API 和 Web 面板组合部署草案。

## 文档入口

- [文档总览](docs/README.md)
- [全局设计](docs/architecture/global-design.md)
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

- 当前目录暂未初始化 Git 仓库。
- 当前已完成全局设计、细分设计文档和 MVP 工程骨架。
- 当前 API 默认使用内置 demo 数据；设置 `FC_DATA_MODE=postgres` 和 `DATABASE_URL` 后，可从 PostgreSQL 读取指标和观测值并即时评分。

## 本地运行

后端 API：

```powershell
cargo run -p fc-api
```

前端面板：

```powershell
cd apps/web
npm install
npm run dev
```

访问：

- API: <http://127.0.0.1:18080/health>
- Web: <http://127.0.0.1:5173>

常用命令：

```powershell
just fmt
just test
just lint
just web-build
```

Docker Compose 草案：

```powershell
docker compose -f deploy/docker-compose.yml up --build
```

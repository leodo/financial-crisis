# ADR-0001：初始架构方向

状态：`Draft`

日期：2026-05-30

## 背景

项目目标是建设金融危机预警系统，需要覆盖数据抓取、数据治理、风险分析、API 服务和网页面板。用户偏好 Rust，同时最担心免费数据源抓取的可行性。

当前项目为空目录，没有既有代码、构建方式或技术债。

## 决策

采用文档优先、Rust 后端、React 前端、PostgreSQL/TimescaleDB 存储的总体方向。

第一阶段建议：

- 后端服务：Rust + Tokio + Axum。
- 数据库访问：sqlx。
- 数据存储：PostgreSQL + TimescaleDB。
- 原始数据和回测归档：Parquet。
- 前端：React + TypeScript + Vite + ECharts。
- 抓取任务：Rust worker + PostgreSQL job table。
- 数据源策略：优先官方免费 API，谨慎使用非官方市场数据源。
- 开源项目策略：参考 Equibles、OpenBB、Canairy、psymonitor、SystemicRisk，但不直接绑定其中任何一个作为主干。

## 理由

- Rust 适合长期运行的数据抓取、任务调度、API 和风险评分服务。
- React 生态更适合复杂金融面板，图表、表格和交互方案更成熟。
- PostgreSQL 适合元数据、任务状态、预警事件和配置。
- TimescaleDB 适合时间序列指标查询。
- 免费数据源质量参差不齐，必须把数据源治理作为第一类模块，而不是散落在脚本中。

## 影响

正面影响：

- 架构清晰，适合逐步实现。
- 数据抓取、分析和展示职责分离。
- 后续可替换市场数据源或接入商业数据。
- 评分解释和数据追溯可以作为核心能力沉淀。

代价：

- 前后端双技术栈复杂度高于全 Rust。
- Rust 数据分析生态弱于 Python，需要保留研究环境扩展点。
- TimescaleDB 增加部署组件，但换来更好的时序查询能力。

## 暂不采纳的方案

### 全 Rust 前后端

例如 Axum + Leptos。

暂不优先采用，原因是金融面板需要大量成熟图表、表格和交互控件，React 生态更省成本。

### Python 单体应用

例如 FastAPI + Pandas + Streamlit。

适合快速原型，但不符合用户 Rust 偏好，也不利于长期稳定运行的抓取和服务化。

### 直接 fork 现有开源项目

现有项目没有完全匹配“金融危机预警系统”的主干。更合理的是参考其数据源、模型和页面形态，重新设计适合本项目的核心架构。

## 后续需要验证

- 免费数据源条款和稳定性。
- FRED、SEC、World Bank、IMF、BIS 首批指标可用性。
- TimescaleDB 是否需要从第一版就引入，还是先使用纯 PostgreSQL。
- 前端组件库选择。
- 是否需要从第一版加入 Grafana 作为内部数据源监控面板。


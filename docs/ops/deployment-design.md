# 部署设计

状态：`Draft`

最后更新：2026-05-30

## 1. 目标

定义金融危机预警系统从本地开发到生产部署的阶段性方案。第一版以 Docker Compose 为主，后续再考虑 Kubernetes。

## 2. 部署单元

建议服务：

```text
api-service          Rust Axum API
worker-service       Rust 抓取和分析 worker
frontend             React/Vite 构建后的静态站点
postgres             PostgreSQL + TimescaleDB
redis                可选，用于缓存和轻量队列
prometheus           指标采集
grafana              内部监控面板
object-storage       可选，本地 MinIO 或文件系统
```

第一版最小部署：

```text
api-service
worker-service
frontend
postgres
```

## 3. 环境分层

### 3.1 本地开发

目标：

- 快速启动。
- 使用小样本数据。
- 可重复初始化数据库。

建议：

- Docker Compose 启动数据库。
- Rust 服务本地运行。
- 前端本地 Vite dev server。

### 3.2 集成测试环境

目标：

- 验证抓取、解析、评分、API 和前端闭环。

建议：

- 使用固定测试数据源或录制响应。
- 避免频繁打真实 API。
- 每次构建重建测试数据库。

### 3.3 生产环境

目标：

- 稳定抓取。
- 数据可恢复。
- 配置和 secret 安全。
- 有监控和告警。

建议：

- Docker Compose 起步。
- 数据库持久卷。
- 原始数据归档目录独立挂载。
- 定期备份 PostgreSQL 和 raw store。

## 4. 配置管理

配置分为：

- 应用配置：端口、日志级别、数据库连接。
- 数据源配置：API base URL、限流、启停。
- Secret：API key、数据库密码。
- 评分配置：指标权重、阈值、方法版本。

要求：

- Secret 不进入 Git。
- 评分配置要版本化并可审计。
- 数据源启停要记录原因。

## 5. 数据持久化

必须持久化：

- PostgreSQL 数据目录。
- 原始响应存储。
- Parquet 回测快照。
- Grafana dashboard 和 Prometheus 数据，视环境决定。

备份优先级：

1. PostgreSQL。
2. 原始响应。
3. 评分配置和方法版本。
4. 回测快照。

## 6. 启动顺序

```text
postgres
  -> migration
  -> api-service
  -> worker-service
  -> frontend
  -> monitoring
```

worker 必须等待 migration 完成后才能启动。

## 7. 数据库迁移

后续实现建议：

- 使用 Rust 生态迁移工具或 sqlx migration。
- migration 文件进入 Git。
- 生产迁移前先备份。
- 破坏性迁移需要 ADR 或变更记录。

## 8. 部署安全基线

- API 不直接暴露数据库。
- 管理接口需要认证。
- 数据源 API key 使用 secret。
- 前端不持有数据源 API key。
- 生产环境关闭调试端点。
- 日志不得打印 secret。

## 9. 容量初始估算

MVP 阶段：

- 指标数量：100 到 300。
- 主要频率：日、周、月、季。
- 数据点规模：百万级以内。
- PostgreSQL 足够支撑。

扩展阶段：

- 高频行情增加后，数据点会快速增长。
- 需要 TimescaleDB hypertable、压缩、分区和归档。
- 行情流式处理可能引入 NATS、Kafka 或 Redpanda。

## 10. 发布流程

建议流程：

1. 文档和设计更新。
2. 实现变更。
3. 本地测试。
4. 集成环境回放数据。
5. 数据库迁移验证。
6. 生产部署。
7. 观察抓取状态和评分输出。

## 11. 后续扩展

当满足以下条件时再考虑 Kubernetes：

- 多个 worker 实例需要水平扩展。
- 数据源抓取任务明显增多。
- 需要更复杂的滚动发布。
- 需要多环境统一运维。


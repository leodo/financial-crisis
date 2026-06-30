# Core Crate 测试补充计划

状态：`Draft`

最后更新：2026-06-30

## 1. 当前覆盖概况

| Crate | 现有测试 | 主要覆盖 | 主要缺失 |
|-------|---------|---------|---------|
| fc-ingestion | 16 | URL 构建、CSV 解析 | 错误路径（网络超时、格式异常、空响应）、MockConnector 边界 |
| fc-storage | 14 | SQLite 写入/读取 round-trip | 连接异常、空数据集、并发写入、数据不存在回退、种子数据验证 |
| fc-domain | 42 | probability_bundle、scenario_catalog | 序列化/反序列化边界、alert/assessment/backtest 等类型验证 |

## 2. 测试原则

- **不 mock 类型**：domain 类型用 `Default` 或 builder 构造真实实例
- **mock 外部依赖**：ingestion 的 HTTP 调用用 mock server 或 trait 替身
- **SQLite 用内存模式**：storage 测试用 `SqliteStore::connect(":memory:")`，不需要真实数据库
- **一个测试只验证一个行为**：不把多个场景塞到一个 test fn 里

## 3. Ingestion 补测计划

现有测试覆盖了各 connector 的 URL 构建和正常解析路径。需要补：

### 3.1 MockConnector（mock.rs）
```rust
// 现有：正常返回数据
// 缺：
// - 空数据集返回
// - connector 返回错误时传播
// - 超大响应截断
```

### 3.2 FredConnector（fred.rs）
```rust
// 现有：URL 构建
// 缺：
// - API key 未设置时降级行为
// - 日期范围边界（start > end）
// - 超大日期范围不会超过 API 限制
```

### 3.3 FredGraphCsvConnector（fred_graph_csv.rs）
```rust
// 现有：URL 构建、CSV 解析、日期过滤
// 缺：
// - CSV 字段数不匹配
// - CSV 日期格式异常（非 ISO 日期或有 BOM）
// - CSV 数值列含非数字
// - 空 CSV（只有表头）
// - HTTP 返回非 200
```

### 3.4 Http 客户端（http_client.rs）
```rust
// 缺：
// - 超时错误转换
// - 非 2xx 状态码处理
// - 空响应体
```

### 3.5 Contract（contract.rs）
```rust
// 现有：接口定义
// 缺：
// - Connector trait 可派生 Send + Sync 验证
// - 空 FetchPlan 行为
```

## 4. Storage 补测计划

### 4.1 SqliteStore（sqlite.rs）
```rust
// 缺：
// - 连接无效路径返回错误
// - 重复创建表不会报错（幂等性）
// - 同时读写不崩溃
```

### 4.2 Observations（observations.rs）
```rust
// 现有：round-trip 写入读取
// 缺：
// - 写入空列表
// - 查询不存在的 indicator_id
// - 按时间范围查询边界
```

### 4.3 Seeds（seeds/）
```rust
// 缺：
// - 种子数据完整性校验（所有必需 catalog 条目都存在）
// - 重复 seed 不会出错
```

### 4.4 Operational（operational.rs）
```rust
// 缺：
// - 写入运行状态记录
// - 读取最新状态
// - 状态不存在时返回 None
```

### 4.5 Migrations（migrations.rs）
```rust
// 缺：
// - 迁移幂等性（重复执行不报错）
// - 迁移版本号递增验证
```

## 5. Domain 补测计划

### 5.1 序列化/反序列化
```rust
// 验证每个主要类型的 serde round-trip：
// - Alert → JSON → Alert
// - Assessment → JSON → Assessment
// - Backtest → JSON → Backtest
// - 含 Option 字段的类型在 None 时也能正确反序列化
```

### 5.2 类型约束
```rust
// - RiskLevel 的排序关系（normal < watch < stress < warning < crisis）
// - QualityGrade 的排序关系
// - DecisionPosture 的排序关系
// - 枚举反序列化未知值返回错误
```

## 6. 优先级

| 层级 | 事项 | 预估 | 理由 |
|------|------|------|------|
| P1 | ingestion 错误路径 | 1 天 | 数据抓取是入口，异常场景最重要 |
| P1 | storage 错误路径 | 1 天 | 数据持久化基础，空/异常场景 |
| P2 | domain 序列化验证 | 0.5 天 | 边界检查，但不太会出问题 |
| P3 | storage 种子验证 | 0.5 天 | 低频改动，出问题容易发现 |
| P3 | ingestion 超大响应/boundary | 0.5 天 | 边缘场景 |

## 7. 运行方式

```bash
# 单 crate
cargo test -p fc-ingestion
cargo test -p fc-storage
cargo test -p fc-domain

# 全部
cargo test --workspace
```

# 危机概率评估设计 TODO

状态：`Draft`

最后更新：2026-05-30

## 1. 目的

本清单用于跟踪“从风险强度看板升级为危机概率评估系统”这一轮设计工作。

## 2. 当前已完成的设计

- [x] `docs/architecture/global-design.md`
- [x] `docs/analytics/horizon-label-design.md`
- [x] `docs/analytics/probability-engine-design.md`
- [x] `docs/analytics/decision-support-policy.md`
- [x] `docs/analytics/feature-store-design.md`
- [x] `docs/analytics/probability-calibration-design.md`
- [x] `docs/analytics/real-backtest-execution-design.md`
- [x] `docs/analytics/historical-analog-design.md`
- [x] `docs/analytics/posture-threshold-tuning.md`
- [x] `docs/data/us-centric-free-data-plan.md`
- [x] `docs/data/jpy-carry-risk-module-design.md`
- [x] `docs/data/sec-edgar-connector-spec.md`
- [x] `docs/data/boj-connector-spec.md`
- [x] `docs/product/decision-dashboard-design.md`
- [x] `docs/product/assessment-api-contract.md`
- [x] `docs/product/methodology-page-design.md`
- [x] `docs/events/banking-event-taxonomy.md`

## 3. 下一批必须补齐的开发前设计

### P0

- [x] `docs/data/sec-edgar-connector-spec.md`
- [x] `docs/data/boj-connector-spec.md`
- [x] `docs/analytics/feature-store-design.md`
- [x] `docs/analytics/probability-calibration-design.md`
- [x] `docs/analytics/real-backtest-execution-design.md`
- [x] `docs/product/assessment-api-contract.md`

### P1

- [x] `docs/analytics/historical-analog-design.md`
- [x] `docs/analytics/posture-threshold-tuning.md`
- [x] `docs/events/banking-event-taxonomy.md`
- [x] `docs/product/methodology-page-design.md`

## 4. 本轮开发建议顺序

1. 先完成 `SEC` 和 `BOJ` 连接器规格。
2. 再做特征库和标签流水线。
3. 然后做真实回测执行设计。
4. 再定义 assessment API contract。
5. 最后改造前端和接口。

## 5. 完成定义

当以下条件满足时，说明这一轮设计足以支撑开发：

- 危机标签定义稳定。
- 三个 horizon 概率模型设计明确。
- 免费数据主线明确到连接器级别。
- 决策面板信息架构明确。
- API contract 和回测执行设计补齐。

## 6. 当前结论

当前这轮“危机概率评估系统”开发前设计已经完成第一批和第二批文档落地。

后续可以按以下顺序直接进入编码：

1. `SEC EDGAR` 连接器
2. `BOJ / USDJPY` 连接器
3. feature store
4. assessment API
5. 真实回测链路
6. 新决策面板

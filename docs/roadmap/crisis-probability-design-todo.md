# 危机概率评估设计 TODO

状态：`Draft`

最后更新：2026-06-01

## 1. 目的

本清单用于跟踪“从风险强度看板升级为危机概率评估系统”这一轮设计工作。

## 2. 当前已完成的设计

- [x] `docs/architecture/system-feasibility-analysis.md`
- [x] `docs/architecture/global-design.md`
- [x] `docs/analytics/horizon-label-design.md`
- [x] `docs/analytics/scenario-catalog.md`
- [x] `docs/analytics/probability-engine-design.md`
- [x] `docs/analytics/decision-support-policy.md`
- [x] `docs/analytics/portfolio-action-playbook.md`
- [x] `docs/analytics/feature-store-design.md`
- [x] `docs/analytics/feature-coverage-matrix.md`
- [x] `docs/analytics/formal-dataset-spec.md`
- [x] `docs/analytics/probability-calibration-design.md`
- [x] `docs/analytics/real-backtest-execution-design.md`
- [x] `docs/analytics/model-release-and-serving-design.md`
- [x] `docs/analytics/model-go-no-go.md`
- [x] `docs/analytics/historical-analog-design.md`
- [x] `docs/analytics/posture-threshold-tuning.md`
- [x] `docs/data/us-centric-free-data-plan.md`
- [x] `docs/data/point-in-time-visibility-spec.md`
- [x] `docs/data/jpy-carry-risk-module-design.md`
- [x] `docs/data/sec-edgar-connector-spec.md`
- [x] `docs/data/boj-connector-spec.md`
- [x] `docs/product/decision-dashboard-design.md`
- [x] `docs/product/assessment-api-contract.md`
- [x] `docs/product/methodology-page-design.md`
- [x] `docs/events/banking-event-taxonomy.md`

## 3. 下一批必须补齐的开发前设计

### P0

- [x] `docs/analytics/feature-coverage-matrix.md`
- [x] `docs/analytics/scenario-catalog.md`
- [x] `docs/data/point-in-time-visibility-spec.md`
- [x] `docs/analytics/formal-dataset-spec.md`
- [x] `docs/analytics/model-go-no-go.md`
- [x] `docs/data/sec-edgar-connector-spec.md`
- [x] `docs/data/boj-connector-spec.md`
- [x] `docs/analytics/feature-store-design.md`
- [x] `docs/analytics/probability-calibration-design.md`
- [x] `docs/analytics/real-backtest-execution-design.md`
- [x] `docs/analytics/portfolio-action-playbook.md`
- [x] `docs/analytics/model-release-and-serving-design.md`
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
- posture 到动作预算、保护和再入场规则明确。
- 免费数据主线明确到连接器级别。
- 模型发布、激活、回滚和在线评分链路明确。
- 决策面板信息架构明确。
- API contract 和回测执行设计补齐。

## 6. 当前结论

当前这轮“危机概率评估系统”主线设计已经足以支撑后续开发，但要把“设计可开工”和“正式模型可上线”分开看：

1. `P0 / P1` 的工程开发已经有足够文档支撑，可以继续推进。
2. 正式 PIT 候选版已经完成一轮真实复核，但还没有达到可替代当前 transitional baseline 的水平。

后续可以按以下顺序直接进入编码：

1. `SEC EDGAR` 连接器
2. `BOJ / USDJPY` 连接器
3. feature store
4. 正式概率模型发布与在线评分链路
5. assessment API
6. 真实回测链路
7. 持仓动作手册与新决策面板

补充判断：

- 如果目标是继续完善当前可运行系统，上述顺序仍成立。
- 如果目标是做“最终可信的 formal probability model”，现在应优先进入：
  - `raw feature store`
  - `scenario catalog` 配置化
  - `point-in-time visibility` 落库与过滤
  - `formal dataset builder`
  - `release` 准入门槛实现

### 6.1 2026-06-01 新增结论

本轮已经完成两类 formal PIT 候选版复核：

- `us_formal_pit_20260531T160129`
- `us_formal_pit_weighted_20260531T171025`

它们都没有通过当前运行时护栏：

- `timely_warning_rate` 明显低于当前 active transitional baseline
- `rolling_audit.actionable_precision` 明显下降
- `longest_false_positive_episode_days` 明显变长

同时也验证过“按 formal main release 下调运行时动作阈值”的方案，结论仍然是：

- 问题不只是阈值映射；
- 更大概率出在标签稀疏、动作级目标不足、以及 raw PIT 历史审计链还未完全替代 persisted snapshot bridge。

另外，当前代码已经补上一层更严格的 formal history 刷新逻辑：

- formal main release 在缓存版本失配时，不再直接相信旧 `prediction snapshots`
- 会改为基于原始观测全量重建该 release 的历史轨迹后再做 rolling audit / release review

在这个前提下重新复核两个 PIT 候选，护栏结论仍然不变。

随后又补做了一轮“场景感知加权”训练：

- 让正样本权重显式感知 `scenario family`
- 感知该场景是否适合作为对应 horizon 的主正例
- 感知离 `crisis_start` 还有多少天
- 对 `5d` 急性场景使用更贴近 `acute_start` 的标签锚点

新的候选版 `us_formal_pit_scenweight_20260531T184905` 仍未通过护栏。

因此可以把当前判断再收敛一步：

- “formal 主线失败” 不再主要是缓存问题；
- 也不再主要是简单类别失衡问题；
- 下一个必须做的方向，应是 `action-oriented labels / episode objective / actionability layer`。

因此当前工程状态应理解为：

- 设计文档已经够用；
- 系统代码已经具备继续开发条件；
- 但正式概率模型主线还处于研究候选阶段，当前默认线上版仍应保持 `us_formal_transitional_20260531T094603`。

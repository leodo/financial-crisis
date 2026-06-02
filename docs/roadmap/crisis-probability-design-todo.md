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
- [x] `docs/analytics/actionability-episode-objective-design.md`
- [x] `docs/analytics/regime-separation-training-objective-design.md`
- [x] `docs/analytics/raw-pit-history-replay-design.md`
- [x] `docs/data/us-centric-free-data-plan.md`
- [x] `docs/data/point-in-time-visibility-spec.md`
- [x] `docs/data/us-historical-scenario-coverage-matrix.md`
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

### P1.5

- [x] `docs/analytics/actionability-episode-objective-design.md`
- [x] `docs/analytics/regime-separation-training-objective-design.md`
- [x] `docs/analytics/raw-pit-history-replay-design.md`
- [x] `docs/data/us-historical-scenario-coverage-matrix.md`

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

另外，当前代码已经补上一层更严格的 bundle-backed history 刷新逻辑：

- 只要 active release 带有 bundle-backed 概率模型，且缓存版本失配，就不再直接相信旧 `prediction snapshots`
- 会改为基于原始观测全量重建该 release 的历史轨迹后再做 rolling audit / release review

在这个前提下重新复核两个 PIT 候选，护栏结论仍然不变。

随后又补做了一轮“场景感知加权”训练：

- 让正样本权重显式感知 `scenario family`
- 感知该场景是否适合作为对应 horizon 的主正例
- 感知离 `crisis_start` 还有多少天
- 对 `5d` 急性场景使用更贴近 `acute_start` 的标签锚点

新的候选版 `us_formal_pit_scenweight_20260531T184905` 仍未通过护栏。

接着又补做了两轮 `action window` 标签实验：

- `us_formal_pit_actionwin_20260531T192253`
- `us_formal_pit_actionwinv2_20260531T193705`

其中：

- 第一版把动作窗口持续到整个 `crisis_end`，导致 `actionable_precision` 掉到 `10.2%`、最长纯误报段拉到 `84` 天；
- 第二版收紧到 bounded action window 后，`actionable_precision` 恢复到 `20.6%`、最长纯误报段降到 `18` 天；
- 但两版的 `timely_warning_rate` 都仍然只有 `12.5%`，没有解决“危机前可执行提前量不足”的核心问题。

因此可以把当前判断再收敛一步：

- “formal 主线失败” 不再主要是缓存问题；
- 也不再主要是简单类别失衡问题；
- 也不再主要是“把整套标签直接改成 action window”就能解决的问题；
- 下一个必须做的方向，应是 `action-oriented labels / episode objective / actionability layer / dual-head fusion`。

因此当前工程状态应理解为：

- 设计文档已经够用；
- 系统代码已经具备继续开发条件；
- 但正式概率模型主线还处于研究候选阶段，当前默认线上版仍应保持 `us_formal_transitional_20260531T094603`。

### 6.2 2026-06-01 dual-head 工程已落地，但模型结论仍是 No-Go

本轮又把 `actionability layer / dual-head fusion` 的第一版工程实现补齐了：

- worker 可以训练并导出 `crisis-prior + actionability` 双头 bundle；
- domain / release manifest / API response 已支持 `actionability` 及其方法版本字段；
- web 面板已能显示 `prepare / hedge / defend` 三类动作概率，并区分“独立动作头”还是“旧逻辑回推”；
- release review 链路已经能自动对比 dual-head 候选和当前 active baseline。

对应候选版：

- `us_formal_pit_dualhead_20260601T003145`

但 review 结论仍然失败：

- `timely_warning_rate`: `37.5% -> 12.5%`
- `actionable_precision`: `29.6% -> 20.6%`
- `longest_false_positive_episode_days`: `9 -> 18`

这说明当前还不能把“已经有 dual-head 代码”误解成“正式模型已经接近可上线”。当前更准确的工程判断是：

1. 设计文档足够继续开发；
2. 系统代码也足够继续做 formal 研究；
3. 但模型层下一步必须改训练目标和样本治理，而不是继续围绕 serving 阈值做微调。

因此，接下来应优先推进以下剩余工作：

- [ ] 定义 `prepare / hedge / defend` 独立 episode 目标，而不是继续复用 `60d / 20d / 5d proxy`
- [x] 给 runtime audit / release review 增加 clause-level posture 触发拆解，明确是 probability floor、structural context 还是 transitional bridge 在驱动 `prepare / hedge / defend`
- [ ] 让 formal main 训练目标显式约束 `positive_window` 相对 `normal` 的概率抬升，而不是允许全 regime 输出近似常数
- [x] 补训练侧 `actionability` 专属评估口径，区分“提前命中”“过晚确认”“完全漏报”
- [x] 在 dataset summary / 训练输出里显式暴露动作层 `scenario_count`
- [x] 在 dataset summary / release review 里显式暴露 `regime mix` 和按 regime 的 runtime 概率分布
- [x] 把 `actionability` 专属评估继续接入 release review / go-no-go 护栏
- [x] 给 release review 增加 `runtime sanity guard`，避免 candidate 因为“和同样失效的 baseline 一样差”而被误判通过
- [x] 让 `actionability` guard 从“基础拦截”升级到“分层级、分角色”的正式阈值
- [ ] 把 formal history 审计链继续往 `raw point-in-time feature store` 收口，减少对 persisted snapshots 的过渡依赖
- [ ] 扩展美国历史压力样本，尽量覆盖 `1987 / 1994 / 2000 / 2001 / 2008 / 2011 / 2020 / 2023` 中免费可回补的区间
- [x] 把方法页和面板解释继续补强，让用户能看懂“危机先验”和“动作概率”不是同一个东西

补充观察：

- 当前 `formal_v1_main_1990_daily:20260601Tactionwindowv2` 的 `evaluation` split，动作标签虽然数量已经不算太少，但场景覆盖仍只有 `1`；
- 这意味着动作头的离线评估和阈值判断容易被单场景主导；
- 因此下一步除了改标签，还必须补“更多免费可回补历史样本”或重做时间切分，避免继续在单场景上过拟合。

### 6.3 2026-06-01 专项设计已补齐，下一阶段按顺序推进

本轮已补齐 4 份直接支撑下一阶段编码的专项设计：

- [x] `docs/analytics/actionability-episode-objective-design.md`
- [x] `docs/analytics/regime-separation-training-objective-design.md`
- [x] `docs/analytics/raw-pit-history-replay-design.md`
- [x] `docs/data/us-historical-scenario-coverage-matrix.md`
- [x] `docs/analytics/formal-main-protected-context-design.md`

在此基础上，后续工作按以下顺序推进：

1. Episode-native 动作目标
   - [x] 在 `scenario catalog` 中落 `episode_template_id / action_episode_overrides / protected_action_levels`
   - [x] 在 dataset builder 中生成 `prepare / hedge / defend` 的 episode-native 标签
   - [x] 新增 `primary_action_level / action_episode_phase / protected_action_window`
   - [x] 把 dataset summary / release review / actionability guard 从旧 `action_label_*` proxy 计数切到 `primary / late_validation / protected` 口径
   - [x] 把 API / runtime / UI 对动作头的解释补到 episode-native 口径，避免继续显示成 `60d / 20d / 5d` 代理语义
2. Regime-aware formal main 训练目标
   - [x] 扩充 `regime separation` summary，显式暴露 `positive_window / cooldown` 概率与 lift
   - [x] 把 runtime sanity guard 接上 `20d positive_window <= normal` 与 `cooldown_bleed`
   - [x] 固化 `positive_window > normal` 的 regime-aware 样本权重与候选选择分
   - [x] 把 `cooldown_bleed`、`cold_across_all_regimes` 正式接入 release guard
   - [x] 让 `20d / 60d` 至少出现可用 early-warning separation（候选版 `us_formal_main_20260601T184003` 已在 bundle evaluation 达成）
   - [x] 设计并实现 formal main 对 `candidate_optional / protected stress` 的第一阶段纳入策略
   - [x] 明确 protected stress 在 formal main 中属于 `actionability context + regime-aware protected sample`，不是正式主正例
   - [x] 在 `ForwardCrisis` 概率头中引入更强的 protected-context / pre-warning separation（方向约束、软标签、margin pairwise、regime-aware calibration scope 已落地）
   - [x] 把 runtime `prepare / hedge` 触发从“有分离就直接越线”收敛到“可用但不过宽”，当前已把 `longest_false_positive_episode_days` 从 `38` 压到 `5`
   - [x] 补入 `2022-11-03 ~ 2023-01-13` 的银行资产负债表压力 protected window，避免把区域银行危机前的系统性积压误记为纯噪声
   - [ ] 在 runtime floor 收紧后复核 `timely_warning_rate / actionable_precision` 是否仍保持正收益（当前 `actionable_precision=66.7%`，但 `timely_warning_rate` 从 `30.0%` 回落到 `10.0%`）
   - [ ] 逐个复盘 `1990-1993 / 2000 / 2008 / 2011 / 2020 / 2022` 的 missed 场景，判断是 `prepare` 过严、`hedge` 过晚，还是训练目标只学会了 `2023` 这类银行危机形态
   - [x] 为 formal 训练管线增加 `--aux-dataset-key`，支持把 `main + ext_stress + ext_acute` 作为同一轮候选训练输入
   - [x] 把 `scenario_training_role` 贯通到 formal dataset row / SQLite / dataset CSV / training row，并接入 `ForwardCrisis` 正例权重
   - [ ] `main + ext_stress + ext_acute` 组合候选 `us_formal_main_extmix_20260601T215225` 已验证：bundle evaluation 出现 `5d/20d/60d usable separation`，但 strict runtime review 仍只有 `timely_warning_rate=10.0%`，说明“扩展样本接入能力”已经打通，但“如何真正学进去”还没解决
   - [x] 基于新的 `scenario_training_role + scenario_family` 加权重新训练 `extmix2 / extmix3 / extmix4`，验证“继续加权/软标签/pairwise 微调”后，runtime 仍停留在 `timely_warning_rate=10.0%`
   - [x] 已连续复核 `extmix2 / extmix3 / extmix4`：角色加权、soft-label 上调、pairwise margin 加强、normal/cooldown 负样本惩罚，均未突破 `timely_warning_rate=10.0% / longest_false_positive_episode_days=30`
   - [ ] 下一轮不再继续做同类 sample-weight 微调；需要先设计更强的模型形态或目标函数（如 regime-tail objective、family-conditional head、非线性模型基线），否则只会重复得到“bundle 有分离、runtime 仍失败”的结果
   - [ ] “强 prepare 也算动作级预警”的审计口径已接入，但在 `extmix` 复核里没有提高 `timely_warning_rate`，反而把 `longest_false_positive_episode_days` 拉到 `30`；这说明仅靠回测口径放宽不能替代训练目标修正
   - [x] 补写 `docs/analytics/formal-nextgen-model-design.md`，把下一轮主线正式切到 `interaction_tail_v1 -> family_conditional_v1`
   - [x] 在 `crates/domain` 增加 derived feature resolver 与 bundle metadata（记录 `model_family / feature_transform`）
   - [x] 在 `apps/worker` 为 `train-probability` / `bootstrap-formal-release` 增加 `--model-shape linear_v1|interaction_tail_v1`
   - [x] 实现 `interaction_tail_v1` 第一批交互/尾部特征，并确保训练与 API serving 共用同一套特征解析
   - [x] 用 `main + ext_stress + ext_acute` 重训第一版 `interaction_tail_v1` 候选 `us_formal_interaction_tail_extmix1_20260602T015347`；结果是 bundle `5d/20d/60d` 全部可用，runtime 已恢复 `20d/60d usable separation`，但仍只达到 `timely_warning_rate=10.0% / actionable_precision=63.8% / longest_false_positive_episode_days=21`
   - [ ] 在 `interaction_tail_v1` 上继续压缩 `5d normal leakage`，避免 `5d normal_avg_probability >= positive_window_avg_probability`
   - [ ] 在 `interaction_tail_v1` 上继续压缩 `60d normal / cooldown` months 过宽问题；runtime guard tightening 已把 `months` 从 `1053` 历史点压到 `56`，但 `60d` 仍是 `separated_but_below_runtime_floor`
   - [ ] 复核 `interaction_tail_v1` 的 calibration / decision-threshold 选择是否把 `prepare_p60d` 拉得过高（当前 runtime floor 已到 `73.2%`）
   - [x] 产出 `interaction_tail_extmix2` 并重跑 strict rebuild review；结果是 `actionable_precision` 从 `63.8%` 提到 `65.8%`、`longest_false_positive_episode_days` 从 `21` 降到 `19`，但 `timely_warning_rate` 仍停在 `10.0%`
   - [x] 拆解 `prepare_carry_structural / prepare_p60d_structural / prepare_structural_downgrade` 的 overfire；已确认 `carry` 过宽是 runtime posture 问题，收紧 guard 后 `prepare` 从 `477` 点降到 `30`、`longest_false_positive_episode_days` 降到 `5`
   - [ ] 复盘 `interaction_tail_extmix2` 为什么离线 `5d usable separation` 没有穿透到 runtime，优先看 `5d` label / calibration / posture clause 是否口径错位
   - [ ] 在新的 runtime guard 下重训下一版 `interaction_tail` 候选，目标从“继续压误报”切到“把 `60d pre_warning_buffer` 真正推过 `prepare` floor，并恢复绝对提前量”
   - [ ] 只有当 `interaction_tail_v1` 连续两轮仍无法提升 `timely_warning_rate` 且无法压下误报段时，再进入 `family_conditional_v1` 细分设计与 PoC
3. Raw PIT history replay 闭环
   - [x] 新增 historical replay run / point 存储结构
   - [x] release review 默认走 `strict_rebuild`
   - [ ] 让 `analytics_prediction_snapshots` 退回到运行审计与桥接视图角色
4. 美国扩展历史样本落地
   - [x] 把 `1994 / 1998 / 2000-2001 / 2011` 逐个纳入 extension/protected stress 数据集与 summary
   - [x] 保持 `1987` 作为 acute extension + historical analog，不强行并入主宽表
   - [ ] 按覆盖矩阵补齐 `best_effort PIT` 可回放能力
5. 重建、重训、重审计
   - [ ] 重建 formal dataset（`ext_acute/ext_stress` 已完成，`formal main` 待按“恢复 timely warning”目标重建）
   - [ ] 重训 candidate release（下一轮重点不再是压误报，而是恢复可执行提前量）
   - [ ] 重跑 release review / rolling audit / runtime regime audit

### 6.5 2026-06-02 扩展数据集实测结果

- [x] `formal_v1_ext_acute_pre1990:20260601T163102`
  - 已纳入 `1987 / 1998`
  - `1987` 与 `1998` 都已跨 `2` 个 split
  - 适合 `5d/20d` 历史类比与短窗研究
  - 不作为正式主模型上线判断依据
- [x] `formal_v1_ext_stress_1990_daily:20260601T162655`
  - `1990-1993 / 1994 / 2000-2001 / 2011` 已进入扩展 stress 包
  - `calibration / evaluation` 已拥有 forward 正例、episode-native 主正例与 `protected` 行
  - 适合 protected stress、历史对照与扩展训练研究
  - 不作为正式主模型 go-no-go 的单独依据

当前剩余主线不再是“扩展历史样本有没有数据”，而是：

1. 用最新 episode-native / regime-aware 口径重建 `formal main`；
2. 重训 candidate release；
3. 在已经出现 `usable early-warning separation` 的前提下，把 runtime false-positive episode 收紧到可接受区间；
4. 重跑 release review / rolling audit / runtime regime audit；
5. 继续把 raw PIT history 与 persisted snapshot 彻底解耦。

### 6.4 2026-06-01 Episode-native 第一阶段代码已落地

本轮已经把第一阶段里最容易继续分叉的底层结构先收口：

- `scenario catalog` 已新增 `episode_template_id / protected_action_levels`，并预留 `action_episode_overrides`；
- formal dataset row 已新增 `prepare / hedge / defend episode labels`、`primary_action_level`、`action_episode_phase`、`action_episode_id`、`protected_action_window`；
- actionability 训练头已从旧 `bounded action window proxy` 切到新的 `episode-native` 标签；
- SQLite / migrations / dataset CSV / 相关测试已同步更新。

因此 episode-native 第一阶段已经收口完成，下一步不该再回头继续往旧 `action_label_5d/20d/60d` 代理逻辑里堆 patch，而应直接推进：

1. `formal main` 的 regime-aware separation；
2. raw PIT replay 与扩展历史样本；
3. 基于新口径重建 dataset / retrain / release review。

# 危机概率评估设计 TODO

状态：`Draft`

最后更新：2026-06-04

## 1. 目的

本清单用于跟踪“从风险强度看板升级为危机概率评估系统”这一轮设计工作。

自 2026-06-04 起，本文件是“模型/数据/回测主线”的唯一活跃 TODO 真相源。

- 工程结构、模块边界、代码质量与门禁治理，统一由
  [工程维护性 TODO](engineering-maintainability-todo.md) 管理；
- 第一阶段设计索引与旧 backlog 只保留历史导航角色，不再承载当前活跃任务；
- 如果某个专项实施文档产生了当前任务，必须镜像回本清单或工程治理清单之一。

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
     - [x] 已导出 `threshold_diagnostics` 到 bundle 元数据，并用 `extmix8/extmix9` 复核 threshold repair 路径
     - [x] 已确认 `60d` 不是“完全没有 early-warning hit”，而是 calibration split 上 `early_warning_hit_rate=6.0%` 低于 `normal_hit_rate=11.6%`
     - [x] 已确认 `60d repair_reason=early_warning_lift_below_guardrail`，说明当前 calibration split 的 `pre_warning_buffer` calibrated lift 连 `1.5x` 护栏都没跨过
     - [x] 已把 `in_crisis` 从 `20d/60d threshold selection` 剥离，`extmix10` 把 `60d threshold` 从 `0.732` 压到 `0.656`
     - [x] `extmix10` strict rebuild review 已完成：`timely_warning_rate=10.0%` 不变，`actionable_precision=55.9%`，`longest_false_positive_episode_days=5`
     - [x] 下一步改 `60d calibration evidence` 构造：优先审计 `probability_row_is_calibration_eligible`、split regime mix、`pre_warning_buffer` soft-label / weight
       - 已在 bundle `threshold_diagnostics.calibration_regime_evidence` 中按 regime 导出：full split 占比、calibration eligible 行数、calibration used 行数、threshold selected 行数、硬标签均值、训练 soft target 均值、目标权重均值、protected action window 占比。
       - 这一步只补证据链，不改变训练目标；下一轮重训/审计要用这组 evidence 判断 `60d pre_warning_buffer` 是否仍被 normal/cooldown 稀释。
   - [x] 产出 `interaction_tail_extmix2` 并重跑 strict rebuild review；结果是 `actionable_precision` 从 `63.8%` 提到 `65.8%`、`longest_false_positive_episode_days` 从 `21` 降到 `19`，但 `timely_warning_rate` 仍停在 `10.0%`
   - [x] 拆解 `prepare_carry_structural / prepare_p60d_structural / prepare_structural_downgrade` 的 overfire；已确认 `carry` 过宽是 runtime posture 问题，收紧 guard 后 `prepare` 从 `477` 点降到 `30`、`longest_false_positive_episode_days` 降到 `5`
   - [ ] 复盘 `interaction_tail_extmix2` 为什么离线 `5d usable separation` 没有穿透到 runtime，优先看 `5d` label / calibration / posture clause 是否口径错位
   - [ ] 在新的 runtime guard 下重训下一版 `interaction_tail` 候选，目标从“继续压误报”切到“把 `60d pre_warning_buffer` 真正推过 `prepare` floor，并恢复绝对提前量”
     - [x] `extmix8/extmix9` 已完成，但结论是 threshold repair 仍不足以改变 `60d=0.732`
     - [x] `extmix10` 已验证“剥离 in_crisis threshold 惩罚”可把 `60d` 压到 `0.656`
     - [x] `extmix11` 已验证“轻微上调 60d pre_warning_buffer soft-label / 降低 negative weight”对当前数据集几乎无效，不应继续做同类微调
     - [x] 下一版重训前先完成 `60d calibration evidence` 设计与实现，避免继续停在 `timely_warning_rate=10.0%`
     - [x] 基于新增 `calibration_regime_evidence` 重训下一版 `interaction_tail`，复核 `60d pre_warning_buffer` 的 hard/soft/weight/selected 证据是否支持下调 `prepare_p60d`
       - 候选 `us_formal_interaction_tail_extmix_20260603T062837` 已用 `main + ext_stress + ext_acute` 重训并发布为非 active 候选。
       - 证据显示 `60d pre_warning_buffer` 并没有被过滤：`150/150` 行 calibration eligible / used / threshold selected；但 hard label 均值 `0.0`、soft target `26.0%`、目标权重均值 `0.63`，且 calibration hit rate `8.7%` 低于 normal `13.7%`。
       - 快速 review（`history_mode=default, history_limit=5000`）显示相对 `extmix10` 没有改善：`timely_warning_rate=10.0%`、`actionable_precision=55.9%`、`longest_false_positive_episode_days=5` 均不变。
       - 结论：下一刀不应继续做同类重训，应改 `60d pre_warning_buffer` 的目标定义，或进入更强的 family/episode 条件头设计。
     - [x] 为 release review 增加 `--history-mode` / `--history-limit`，并新增 `just release-review-fast`，用于 strict rebuild 太慢时先做方向性 triage；正式 Go/No-Go 仍必须跑 `just release-review`。
     - [x] 补写 `docs/analytics/episode-native-prewarning-target-design.md`，把下一轮实验限定为 `prepare_p60d_episode_native_v1`：只抬升已进入 `prepare episode`、仍在危机前、且场景支持 60d 的 `pre_warning_buffer`，避免把所有 buffer 都当正例。
     - [x] 实现 `prepare_p60d_episode_native_v1` 训练目标并重训候选 `us_formal_interaction_tail_prepare_20260603T081710`；训练 evidence 把 `60d pre_warning_buffer` soft target 从 `26.0%` 提到 `45.2%`、weight 从 `0.630` 提到 `0.900`。
     - [x] `prepare_p60d_episode_native_v1` fast review 已 No-Go：`timely_warning_rate=10.0% -> 10.0%`，`actionable_precision=55.9% -> 54.8%`，`longest_false_positive_episode_days=5 -> 5`；active 已恢复 `us_formal_interaction_tail_extmix10_20260602T061401`。
   - [ ] 进入 `family_conditional_v1` 细分设计与 PoC；停止继续调同类 `60d pre_warning_buffer` soft target / objective weight
     - [x] 补写 `docs/analytics/family-conditional-model-design.md`，第一版限定为线上可计算的 family proxy / family context derived features，不把历史 `scenario_family` 标签直接塞进 serving。
     - [x] 在 `crates/domain` 增加 `family_conditional_v1` feature transform 与 resolver。
     - [x] 在 `apps/worker` 增加 `--model-shape family_conditional_v1`。
     - [x] 用 `main + ext_stress + ext_acute` 训练第一版 family-conditional 候选 `us_formal_family_conditional_20260603T084333` 并跑 fast review。
     - [x] fast review 已 No-Go：`timely_warning_rate=10.0% -> 0.0%`，`actionable_precision=55.9% -> 54.5%`，`longest_false_positive_episode_days=5 -> 5`，runtime `60d` 从 `usable_early_warning_separation` 退化为 `late_only_no_early_warning`。
     - [ ] 升级到真正多头 / 分层校准 bundle schema 设计，不再继续堆同类 family proxy derived features。
       - [x] 补写 `docs/analytics/family-overlay-bundle-schema-design.md`。
       - [x] 在 domain bundle schema 增加 `family_overlays` 兼容字段和 scoring helper。
       - [x] worker 训练侧输出 overlay metadata / row count 审计。
       - [x] 实现第一版 overlay training。
       - [x] API runtime 输出 overlay contribution diagnostics。
       - [x] 基于真实 formal dataset 重训 overlay 候选并跑 release review / runtime regime audit。
         - 已用 `formal main + ext_stress + ext_acute` 训练并发布 `us_formal_family_conditional_20260603T114855`。
         - runtime 已能输出真实 `20d acute_liquidity` overlay contribution，不再只是空 schema。
         - `release review` 结论仍是 No-Go：`timely_warning_rate=10.0% -> 0.0%`，`60d` runtime 退化为 `weak_regime_separation`。
         - `just formal-train-family-overlay` 已固化最新 main/ext 数据集自动解析入口；脚本验证产物 `us_formal_family_conditional_20260603T121703` 复现了“仅 20d acute_liquidity 真正成头”的结构。
         - [x] 在 worker 增加 `balanced` family overlay fallback split，并用 `us_formal_family_conditional_20260603T135847` 验证“样本死区”已明显缓解：`5d=3 overlays`、`20d=3 overlays`、`60d=2 overlays`。
         - [x] 增加 `family_hybrid_v1`：`5d/20d` 保留 family conditional + overlay，`60d` base head 退回 `interaction_tail_v1`，并新增 `just formal-train-family-hybrid` / `-tracked`。
       - [x] 前端进一步把 overlay 贡献和 family audit 接入研究审计 / 发布审计视图，而不只放在方法说明页。
         - 发布审计页已新增 `Overlay 运行审计`，可直接查看 active release 的 runtime contribution、family split 行数和 gate-active 审计。
       - [ ] 下一轮 family overlay 不再以“先训出任意一个 overlay”为目标，而要恢复 `60d` 提前量并通过 release review。
         - [x] 优先推进 `systemic_credit / mixed_systemic / rate_shock` 的 family-level split，而不是继续沿用当前宽表 split。
         - [ ] 审计为什么 `us_formal_family_conditional_20260603T135847` 已经训出多个 overlay，但 runtime replay 里 `60d positive_window raw P (2.3%) < 60d normal raw P (3.7%)`，导致 `timely_warning_rate` 仍是 `0.0%`。
         - [x] 设计并实现 `60d` hybrid / fallback 方案，避免 family 条件头把 baseline `interaction_tail_v1` 的长窗提前量打坏。
         - [x] hybrid 候选 `us_formal_family_hybrid_20260603T142649` 已完成正式 review：`60d` 不再是正例反向，而是 `cooldown_bleed`；但 `timely_warning_rate` 仍是 `0.0%`，`actionable_precision=55.9% -> 44.4%`，所以还不能上线。
         - [x] 已验证“关闭 60d overlay”并不能消除 `cooldown_bleed`：候选 `us_formal_family_hybrid_20260603T144814` 的正式 review 仍是 `timely_warning_rate=0.0% / actionable_precision=44.4%`。
         - [x] 已验证“继续加大 60d positive/cooldown 权重与 pairwise 惩罚”会更差：候选 `us_formal_family_hybrid_20260603T151447` 的正式 review 退化到 `timely_warning_rate=0.0% / actionable_precision=37.5% / longest_false_positive_episode_days=7`，该组更激进权重未保留在当前实现中。
         - [x] 已验证“只在 API runtime 软下调 `prepare_p60d`”同样无效：candidate `us_formal_family_hybrid_20260603T144814` 把 `prepare_p60d` 从 `66.1%` 压到 `61.0%` 后，`prepare` 次数增多，但 `timely_warning_rate` 仍是 `0.0%`，`actionable_precision` 进一步降到 `39.1%`，所以这条阈值软收敛策略已回退。
         - [x] `release review` 已补齐 `Scenario-Level Backtests` 场景表，能够直接列出每个危机样本的 `L2/L3/false_positive/outcome` 对比，不再只看聚合指标。
         - [x] 最新 fast review 已定位当前唯一真正丢失的 timely 样本是 `2023 美国区域银行危机`：baseline 仍有 `L2=83d / L3=70d`，candidate 只剩 `L2=83d`，动作级 `L3` 消失。
         - [x] `release review` 已新增 `Focus Scenarios` 逐日复盘表，并确认 `2023 美国区域银行危机` 的 `L3` 丢失不是“完全没打到动作级条件”，而是 candidate 在危机前只有 `2` 个零星 actionable points，没达到 backtest `5 天窗口至少 3 次命中` 的 sustained 口径；baseline 同窗口有 `13` 个 pre-crisis actionable points。
         - [x] `release review` 已继续补齐 `3/5 sustained-hit` 证据列：逐日表现在直接输出 `forward 5d actionable hits` 与 `sustained yes/no`，并已用 `us_formal_family_hybrid_20260603T144814` 的 fast review 验证 `2023-02-20` 可明确显示 baseline=`5 hits / yes`、candidate=`1 hit / no`。
         - [x] 已进一步确认当前主故障更像是 `20d / hedge posture context` 的持续性塌缩：`2022-12-28` candidate 还能短暂满足 `prepare` 动作级条件，但从 `2022-12-31` 起 `p_20d` 快速回落，`2023-02-20 ~ 2023-02-27` baseline 已连续 `hedge`，candidate 只有 `2023-02-22` 单日进入 `hedge`，其余日期大多退回 `normal`。
        - [ ] 既然 `60d overlay` 不是主因，而盲目加大 60d 权重/惩罚又会恶化结果，下一步直接审计 `60d interaction_tail + episode-native target + runtime threshold policy` 本身，重点看为什么 `cooldown` 仍高于 `positive_window`、以及为什么 `prepare_p60d` 会重新抬高。
          - [x] 已新增 `research release formal-probability-slice` / `just formal-probability-slice`，可脱离 API strict rebuild，直接对 persisted formal dataset 场景窗口做 bundle 离线打分。
          - [x] 已新增 `research release formal-probability-compare` / `just formal-probability-compare`，可直接输出 baseline vs candidate 的逐日概率差、阈值命中差与 top feature contribution delta。
          - [x] 已对 baseline `us_formal_interaction_tail_extmix10_20260602T061401` 与 candidate `us_formal_family_hybrid_20260603T144814` 跑出 `2022-12-01 ~ 2023-03-15` 的 `us_regional_banks_2023` 概率拆解切片。
          - [x] 已确认 `60d overlay` 不是主因：candidate `60d` overlay 已关闭，baseline 也没有 overlay，但两版 `60d` 在该窗口都没有越过各自 decision threshold，只表现为持续偏高背景值。
          - [ ] 下一步把 `60d` 的高背景值继续下钻到 base feature / calibration / threshold selection，确认 cooldown bleed 的真正来源。
         - [x] 已补写 [regional-banks-2023-l3-repair-design.md](../analytics/regional-banks-2023-l3-repair-design.md)，把 `2023 regional banks` 的根因修复从“复盘结论”升级成可执行实验设计，明确诊断产物、允许改动边界、`3/5 sustained hits` 证据表与下一轮 Go/No-Go 条件。
         - [x] 以 `2023 美国区域银行危机` 为第一优先场景，逐日复盘 baseline vs family-hybrid 的 `p_60d / p_20d / posture trigger clause / actionable bridge`，并确认 `L3=70d` 丢失的直接原因是 candidate 没有形成 `actionable 3/5 sustained hits`，而不是单独缺少第一次 `60d` prepare 命中。
        - [ ] 把这次 `2023 regional banks` 复盘进一步下钻到训练样本与 family feature 层，确认究竟是哪组 family-hybrid 特征/权重把 `20d` 连续性压回 `normal`。
          - [x] 已新增 `research dataset slice-main` / `just formal-dataset-slice`，可直接导出单场景 formal dataset 样本切片（JSON + CSV）。
          - [x] 已对 `us_regional_banks_2023` 跑出首份样本切片：`2022-12-01 ~ 2023-03-15` 实际落到 `2023-01-07 ~ 2023-03-15` 的 `68` 条 `evaluation` 样本、`15` 个特征，`regime_60d` 基本恒为 `positive_window`，而 `regime_20d` 在 `normal / pre_warning_buffer / positive_window` 间切换。
          - [x] 已新增 `formal-probability-slice` 对照切片，并确认 candidate 的主故障不是 overlay 过宽，而是 `20d` base head 在 `2023-02-20 / 2023-02-27` 这些 `hedge_episode_label=1` 日期仍只给出 `0.202 / 0.187` 级别概率。
          - [x] 已继续把 `20d` base-head contribution 导出到离线切片，确认 candidate 的主要压分项集中在 `tail_neg__us_curve_10y2y_level__0`、`interaction__us_curve_10y2y_level__us_fed_funds_level`、`us_usdjpy_change_20d`，且 `us_usdjpy_level` 在 candidate 中被学成负贡献。
          - [x] compare 已进一步确认 `20d` 损失最严重的日期集中在 `2023-02-21 ~ 2023-02-27`，其中 `2023-02-24 ~ 2023-02-26` 的 `candidate - baseline p20d` 都接近 `-0.40`。
          - [x] 已确认这条故障基本不在 calibration 层：`2023 regional banks` 关键窗口里两版 `20d/60d` 基本都是 `raw = calibrated`，candidate `20d` 只有极小 overlay 改动。
          - [x] compare 聚合摘要已确认 candidate 的 `20d` 缺陷是窗口级系统性压低，而不是少数边缘样本：overall 平均差 `-0.215`，`positive_window` 平均差 `-0.306`，`hedge` 标签平均差 `-0.344`，且 `positive_window` 里 baseline 命中率 `40%`、candidate `0%`。
          - [x] 已验证“derived tail 单调约束只保留在 `20d`”这条修复方向是有效的：候选 `us_formal_family_hybrid_20260603T192249` 相比 baseline `us_formal_interaction_tail_extmix10_20260602T061401`，在 `us_regional_banks_2023` 上把 `20d hits` 从 `13 -> 29`、`positive_window hit rate` 从 `40% -> 80%`，同时 `60d hits` 仍保持 `0 -> 0`。
          - [x] 已确认“derived tail 单调约束不能粗暴扩到 `60d`”：中间候选 `us_formal_family_hybrid_20260603T191209` 虽然同样把 `20d hits` 提到 `29`，但 `60d hits` 失真膨胀到 `62`；把该约束收窄回 `20d only` 后，`us_formal_family_hybrid_20260603T192249` 在保持同等 `20d` 改善的同时把 `60d hits` 恢复到 `0`。
          - [x] 已完成新一轮正式复核：在重启本地 API 并重新执行 `just release-review`（`strict_rebuild`）后，候选 `us_formal_family_hybrid_20260603T192249` 的正式指标变为 `timely_warning_rate 10.0% -> 10.0%`、`actionable_precision 54.8% -> 64.0%`、`longest_false_positive_episode_days 5 -> 5`，`guard_passed=true`；当前结论是“可进入下一轮人工复核，暂不自动晋升 active release，但误报时长已不再恶化”。
          - [x] 已确认此前看到的 `false_positive_episode_days 5 -> 7` 主要来自两类混淆：一是本地 API 未重启时，review 仍在打旧二进制；二是 `release-review-fast` 与正式 `release-review` 过去会把同名产物互相覆盖。重启后再跑 `strict_rebuild`，`2023-08-21 ~ 2023-08-24` 这段由 runtime monotonic gap 抬起的 `60d` 已不再被算成 candidate 的 actionability false positive。
          - [x] 已把 `release-review` / `probability-slice` 的导出文件名加上 `history_mode` 后缀，避免 `default` 与 `strict_rebuild` 证据互相覆盖，后续复盘时可以直接区分“快速 triage”与“正式 go/no-go”。
          - [x] 已继续把 candidate 剩余短误报拆开做 formal compare：
            - `2023-02-01 ~ 2023-02-15`：candidate 在非正例窗口额外打出 `4` 个 `20d` hits，`avg delta p20d = +0.128`；主导差分集中在 `tail_neg__us_curve_10y2y_level__0`、`tail_pos__us_baa_10y_spread_level__2`、`us_curve_10y2y_level`，并伴随 `family_context__rate_shock__external_dimension_score` 与 `family_proxy__rate_shock` 的正向抬升。
            - `2023-07-01 ~ 2023-07-20`：candidate 在非正例窗口额外打出 `17` 个 `20d` hits，`avg delta p20d = +0.288`；最主要的放大项是 `tail_neg__us_curve_10y2y_level__0`、`family_context__rate_shock__external_dimension_score`、`us_curve_10y2y_level` 与 `family_proxy__rate_shock`，而 `60d` 反而整体较 baseline 更低。
          - [x] 已验证“只压 `curve/fed-funds` cap”还不够：候选 `us_formal_family_hybrid_20260604T031738` 与 `us_formal_family_hybrid_20260604T022954` 一样，formal fast review 仍停在 `actionable_precision=60.9% / longest_false_positive_episode_days=5`，没有超过 `192249`。
          - [x] 已继续验证“对明确同向放大风险的 interaction 加 sign constraint”是有效方向：
            - 新候选 `us_formal_family_hybrid_20260604T034053` 在 `2023-02-01 ~ 2023-02-15` 把 `avg delta p20d` 从 `+0.111` 压到 `+0.085`；
            - 在 `2023-07-01 ~ 2023-07-20` 把额外 `20d hits` 从 `17` 压到 `13`；
            - `us_regional_banks_2023` 仍保持 `20d hits 13 -> 28`、`positive_window hit rate 40% -> 80%`；
            - 正式 `strict_rebuild` review 已确认该版当前是 family-hybrid 主线的最好结果：`timely_warning_rate 10.0% -> 10.0%`、`actionable_precision 54.8% -> 67.3%`、`longest_false_positive_episode_days 5 -> 5`、`guard_passed=true`。
          - [x] 已继续验证“把 `USDJPY / jpy_carry / rate_shock` 进一步压成辅助上下文”这条线的收益边界：
            - 候选 `us_formal_family_hybrid_20260604T043437`（新增 `us_usdjpy_level` / `jpy_carry` caps）把局部窗口继续收窄到 `2023-02 avg delta p20d=+0.055`、`2023-07 avg delta p20d=+0.222`，但 fast review 只达到 `actionable_precision=66.7%`；
            - 候选 `us_formal_family_hybrid_20260604T045257`（进一步收紧 `rate_shock` cap 到 `0.12 / 0.06`）把 `2023-07` 额外 `20d hits` 从 `13` 压到 `12`、`avg delta p20d` 压到 `+0.209`，但 fast review 进一步回落到 `actionable_precision=66.0%`；
            - 结论：单纯继续堆 `USDJPY / jpy_carry / rate_shock` 系数 cap，已经没有超过 `034053` 的把握，应停止在这条线上继续微调。
          - [x] 已验证“soft 20d threshold + confirmation-driven jpy_carry proxy”这一更贴近主问题的改法：
            - 候选 `us_formal_family_hybrid_20260604T055652` 把 `20d threshold` 收在 `0.451`，同时保持 `us_regional_banks_2023` 的 `20d hits 13 -> 28 / positive_window hit rate 40% -> 80%`；
            - 但 `2023-02-01 ~ 2023-02-15` 仍有 `4` 个额外 `20d hits`，`2023-07-01 ~ 2023-07-20` 仍有 `12` 个额外 `20d hits`；
            - fast review 最终只有 `actionable_precision 54.8% -> 65.5%`，仍没有超过 `034053` 的 `67.3%`；
            - 结论：`soft threshold` 可保留，但“单独重构 `jpy_carry proxy`”还不足以成为下一版正式主线。
          - [x] 已验证“直接把 `USDJPY raw interaction` 迁成 tail interaction”这条实现不可取：
            - 候选 `us_formal_family_hybrid_20260604T061852` 把 `20d threshold` 重新拉到 `0.294`；
            - `predicted_positive_count` 膨胀到 `1196`，`normal hit rate` 升到 `14.2%`；
            - 这说明当前不能用“简单替换 `interaction__external_dimension_score__us_usdjpy_level`”的方式来完成 `USDJPY level -> tail/context` 迁移。
          - [x] 已继续验证“再往下压 `curve/USDJPY` 常态误报”这条线的收益上限：
            - 候选 `us_formal_family_hybrid_20260604T064930` 相比 `034053`，在 `2023-02-01 ~ 2023-02-15` 把 `20d hits` 从 `4` 压到 `1`、`avg delta p20d` 再降 `-0.100`；
            - 在 `2023-07-01 ~ 2023-07-20` 把 `20d hits` 从 `12` 压到 `2`、`avg delta p20d` 再降 `-0.157`；
            - 但 `regional_banks` 的 `20d` 连续性也同步回落：相对 `034053`，`20d hits 27 -> 19`、`positive_window hit rate 75% -> 60%`；
            - runtime fast review 最终是 `timely_warning_rate 10.0% -> 10.0%`、`actionable_precision 54.8% -> 65.1%`、`longest_false_positive_episode_days 5 -> 5`，虽然 `guard_passed=true`，但仍低于 `034053` 的 `67.3%`；
            - 结论：继续沿 `tail_neg__us_curve_10y2y_level__0 + USDJPY level` 这条线硬压误报，已经开始直接侵蚀 `regional_banks` 的 `20d` 连续性，当前不应把它作为新的正式主线。
          - [x] 已继续确认这条线的更激进版本同样应直接 No-Go：
            - 候选 `us_formal_family_hybrid_20260604T064040` 在 `2023-02-01 ~ 2023-02-15` 把 `20d hits` 从 `4` 进一步压到 `0`，在 `2023-07-01 ~ 2023-07-20` 把 `20d hits` 从 `12` 压到 `0`；
            - 但 `regional_banks` 的 `20d` 连续性也同步塌到 `20d hits 27 -> 7`、`positive_window hit rate 75% -> 25%`；
            - 离线 compare 已足够说明问题：`tail_neg__us_curve_10y2y_level__0` 从 `034053` 的 `0.00` 继续压到 `-0.12` 后，危机窗口前半段与尾段都被系统性拉到阈值下方，不值得再跑 runtime review；
            - 结论：这类“继续加深 `tail_neg__curve` 负权、再压 `USDJPY level` 基础权重”的候选，后续可以直接离线 No-Go。
          - [x] 已把这条 family-hybrid 主线的候选筛选流程收口成标准命令：
            - `just formal-candidate-window-audit <baseline> <candidate>`：固定输出 `regional_banks`、`2023-02`、`2023-07` 三段窗口 compare；
            - `just formal-candidate-feature-audit <baseline> <candidate>`：固定输出 `20d threshold`、regime 概率分布和关键特征权重差异；
            - `just formal-candidate-screen <baseline> <candidate>`：先跑窗口审计，再汇总为离线筛选结论；
            - 当前已验证该筛选门会把 `064930` 归为 `worth_fast_review`，把 `064040` 归为 `no_go_offline`，与人工结论一致。
          - [x] 已完成 `034053 / 064930 / 064040` 的 20d 联合审计，并把结论固化到设计文档：
            1. `20d threshold` 不是 `064930 / 064040` 丢失 `regional_banks` 连续性的主因，真正先坏掉的是 `positive_window` 原始概率被压低；
            2. `tail_neg__us_curve_10y2y_level__0` 不应继续往负方向加深；`034053=0.00 -> 064930=-0.05 -> 064040=-0.12` 已足够说明这条线会直接侵蚀 `regional_banks`；
            3. `us_usdjpy_level` 不应继续走“下压 base weight + 加强 interaction”的 blunt suppression 组合，下一轮应迁往更保守的 `jpy carry proxy/context` 语义；
            4. `tail_pos__us_baa_10y_spread_level__2` 当前没有证据支持把它变成新的负向 suppressor；
            5. 对应约束已写入 `family-overlay-bundle-schema-design.md` 与 `regional-banks-2023-l3-repair-design.md`。
          - [x] 已把最关键的联合审计结论下沉到训练配置 / 离线筛选，而不再只是人工口头约束：
            1. `tail_neg__us_curve_10y2y_level__0` 在 `20d` 训练约束里已收紧为 `>= 0`；
            2. `USDJPY` 已不再允许“base level 下压 + interaction 放大”自由组合：`us_usdjpy_level` family cap 已改到 `0.30 ~ 0.40`，`interaction__external_dimension_score__us_usdjpy_level` 新增上界 `0.58`；
            3. `just formal-candidate-screen` 已新增 `positive_window_avg_probability` 与 `curve tail + USDJPY mix` 的离线 `No-Go` 规则。
          - [ ] 下一步继续把剩余结论下沉到训练 / 评审策略：
            1. `20d threshold` 只保留 soft penalty / policy guard，不再当作主修复手段；
            2. `USDJPY` 的最终目标仍是迁往“高位 + 变化率/波动率 + 外部确认”的 proxy/context 结构，而不是长期停在 base level cap；
            3. 需要验证新约束下重训出的下一版候选，是否真的不再自然走回 `064930 / 064040` 分支。
          - [x] 已验证新约束下的下一版候选 `us_formal_family_hybrid_20260604T081030` 没有再走回 `064930 / 064040` 分支：
            1. 相对 `034053`，`regional_banks` 的 `20d hits 27 -> 24`、`positive_window hit rate 75% -> 75%`、`positive_window_avg_probability 0.237 -> 0.239`；
            2. 同时 `2023-02` 的 `20d hits 4 -> 1`、`2023-07` 的 `20d hits 12 -> 6`；
            3. `release-review-fast` 与 `strict_rebuild release-review` 都确认 runtime `actionable_precision 54.8% -> 71.4%`、`timely_warning_rate 10.0% -> 10.0%`、`longest_false_positive_episode_days 5 -> 5`；
            4. 结论：`081030` 已成为当前 family-hybrid 主线最好候选，但核心瓶颈已转为“如何恢复 timely warning”。
          - [ ] 下一步优先做一轮“`curve/bond-spread pair + USDJPY semantics + 20d threshold role`”落地审计：
            1. 把 `curve inversion / fed funds / USDJPY level / USDJPY 20d change / jpy carry proxy / rate_shock family context`
               逐列对齐到训练样本、feature engineering、threshold selection 与单调约束配置；
            2. 输出哪些约束已经在训练层落实，哪些还停留在文档结论；
            3. 对仍未落实的项给出最小代码改动入口。
          - [ ] 下一步最高优先级改成“恢复 timely warning / actionable lead time”：
            1. 以 `081030` 为新的 family-hybrid 主线基线，不再继续围绕 `034053` 做主线决策；
            2. 直接审计为什么 `60d` 仍是 `separated_but_below_runtime_floor`；
            3. 直接审计为什么 `2000-2001 / 1990-1993` 只有 `L2` 提前量，却始终无法进入 `L3 actionable`；
            4. 训练、threshold policy、runtime posture 后续都以“提前一周以上可执行预警”作为首要目标，而不是继续优先压短误报。
          - [x] 已确认 `081030` 之后的主瓶颈不是“继续压 20d 误报”，而是
            `strict review gate` 与 runtime floor 的口径失配，加上
            `1990-1993 / 2000-2001` 的 posture continuity failure；
            对应设计已沉淀到
            [release-review-runtime-alignment-design.md](../analytics/release-review-runtime-alignment-design.md)。
          - [x] 已补 `release review` 双口径输出，不再把 runtime 信号和 strict L3 混成一个指标：
            1. 在 point 级输出显式区分 `strict_review_actionable`、`runtime_floor_hit`、
               `runtime_actionable_block_reason`；
            2. 在聚合 summary 中增加 `strict_actionable_point_count` 与
               `runtime_floor_hit_count`；
            3. 让 `Focus Scenarios` 和正式 guard 说明都能同时表达“runtime 已有信号，但 strict L3 仍未成立”。
          - [x] 已把 `Runtime Separation Comparison` 接入 `release review` 主报告与控制台 summary：
            1. 按 `5d / 20d / 60d` 直接对比 baseline / candidate 的 diagnosis、
               runtime floor、early-warning 平均概率、normal 均值、EW gap、floor gap、hit rate；
            2. 主报告新增 `Runtime Interpretation`，明确区分
               “完全没有 early-warning separation”、
               “已经 separated 但仍低于 runtime floor”、
               “cooldown bleed” 三类问题；
            3. 这样后续审计 `60d` 时，可以直接判断更像训练目标问题、阈值映射问题，还是 cooldown 背景值问题。
          - [~] 已开始专项审计 `1990-1993 / 2000-2001` 的 posture continuity：
            1. 逐日对齐 `p20d / p60d / posture / time_bucket / actionable bridge`；
            2. 已补 `runtime_actionable_block_category` 与场景级 `runtime block mix`，
               可以结构化统计为什么高概率日期仍长期停在 `normal` 或被其他条件挡住；
            3. 已确认两类 missed 场景的主导阻塞并不相同：
               - `2000-2001` 主要是 `review_gate_gap`，说明 strict review gate 比 runtime floor 更严；
               - `1990-1993` 主要是 `posture_bucket_normal`，说明真正失败的是 posture continuity；
            4. 已补 `runtime continuity facets`，把 runtime 命中但未形成 strict L3 的点继续拆成：
               `posture:* / bucket:* / trigger:* / gate_gap:* / confirmation:*`；
            5. 下一步不再把两类 missed 场景混成一个问题：
               - 对 `2000-2001` 先专项复核 strict gate 与 runtime floor 的映射；
               - 对 `1990-1993` 先专项复核为什么高 `p20d/p60d` 长期无法推动 `prepare/months` 连续成立。
          - [ ] 下一步以 `034053` 为保护基线继续收口剩余短误报，但约束顺序必须固定：
            1. 先守住 `regional_banks` 的 `20d hits / positive_window hit rate / positive_window_avg_probability`；
            2. 再处理 `2023-02-03 ~ 2023-02-05`、`2023-02-14`、`2023-07-13` 等剩余 `20d` 误报点；
            3. 只有在不牺牲上述连续性的前提下，才允许小幅回调 `20d threshold`。
         - [ ] 对 `mixed_systemic` 先重做 proxy 定义；当前 `gate_active_total=0`，继续训练 overlay 没有有效样本基础。
         - [ ] 把 `jpy_carry` 继续维持为 proxy-only family，先补 protected / proxy rows，再决定是否进入正式 overlay 训练。
         - [x] 复核当前 active release 是否仍停在 review fail 的 family candidate；review 结束后已恢复 `us_formal_interaction_tail_extmix10_20260602T061401`。
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

### 6.4 2026-06-02 扩展数据集实测结果

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
2. 先补 `release review` 双口径审计，避免继续把 runtime 与 strict review 混成一个指标；
3. 再专项修 `1990-1993 / 2000-2001` 的 posture continuity；
4. 在此基础上重训 candidate release；
5. 再处理剩余 runtime false-positive episode 收口；
6. 重跑 release review / rolling audit / runtime regime audit；
7. 继续把 raw PIT history 与 persisted snapshot 彻底解耦。

### 6.5 2026-06-01 Episode-native 第一阶段代码已落地

本轮已经把第一阶段里最容易继续分叉的底层结构先收口：

- `scenario catalog` 已新增 `episode_template_id / protected_action_levels`，并预留 `action_episode_overrides`；
- formal dataset row 已新增 `prepare / hedge / defend episode labels`、`primary_action_level`、`action_episode_phase`、`action_episode_id`、`protected_action_window`；
- actionability 训练头已从旧 `bounded action window proxy` 切到新的 `episode-native` 标签；
- SQLite / migrations / dataset CSV / 相关测试已同步更新。

因此 episode-native 第一阶段已经收口完成，下一步不该再回头继续往旧 `action_label_5d/20d/60d` 代理逻辑里堆 patch，而应直接推进：

1. `formal main` 的 regime-aware separation；
2. raw PIT replay 与扩展历史样本；
3. 基于新口径重建 dataset / retrain / release review。

# 危机概率评估设计 TODO

状态：`Draft`

最后更新：2026-06-10

## 1. 目的

本清单用于跟踪“从风险强度看板升级为危机概率评估系统”这一轮设计工作。

自 2026-06-04 起，本文件是“模型/数据/回测主线”的唯一活跃 TODO 真相源。

- 工程结构、模块边界、代码质量与门禁治理，统一由
  [工程维护性 TODO](engineering-maintainability-todo.md) 管理；
- 第一阶段设计索引与旧 backlog 只保留历史导航角色，不再承载当前活跃任务；
- 如果某个专项实施文档产生了当前任务，必须镜像回本清单或工程治理清单之一。

### 1.1 2026-06-10 MVP 迭代重排

当前开发顺序重新收敛到“先做可用、可信、不会误导持仓决策的 MVP”，而不是继续优先扩大 formal 模型研究范围。

MVP 当前判定：

- 最终成品进度约 `60%`：数据、SQLite、评分、历史类比、动作建议和网页骨架已具备，但正式概率模型、时距解释和决策可信度仍未达到可直接辅助仓位决策的标准。
- 最小可用版进度约 `75%`：规则风险层、关键数据新鲜度、历史类比和动作边界已有基础，但页面上仍存在会误导用户的异常数字。
- 任何会影响首页结论的数字问题，优先级高于继续扩大模型形态、训练目标或研究审计页面。

#### P0：数字可信 MVP

- [x] 修复或降级 `20d` formal 概率异常偏冷问题；在通过 Go/No-Go 前，`5d / 20d / 60d` 只能作为审计读数，不能参与 MVP 主结论。
  - 2026-06-10：已完成 MVP/UI 降级，formal 概率 `audit_only` 时三期限都不再输出风险时距或触线倍数；真正的 `20d` 模型本体修复仍保留在 P2。
- [x] 重做“离风险还有多远”的页面口径：不再显示容易误读的 `0` 或极小值，而是展示离 `观察 / 准备 / 对冲 / 防守` 下一阶段还差哪些证据。
  - 2026-06-10：audit-only 状态下改为展示“下一阶段还差：证据共振”、阻断项和下一步，而不是机械完成度、差值或放大倍数。
- [x] 拆分“结论把握度”：至少区分数据新鲜度、模型可信度、当前结论可信度，避免长期固定数值让用户误以为系统很确定。
  - 2026-06-10：首页已拆成“结论可信度 / 模型可信度 / 数据新鲜度 / 动作升级证据”。
- [x] 给概率轨迹增加鼠标悬停细节，显示日期、`5d / 20d / 60d`、posture、时距桶和数据来源；当 formal 概率为 `audit_only` 时必须明确标注。
  - 2026-06-10：tooltip 明细已补日期、时距桶、posture、raw/formal 对照、较前点变化和历史来源；概率轨迹保留 audit-only 复核说明。
- [x] 增加 MVP 回归检查，防止异常 formal 概率、误导性时距数字或缺失 audit-only 文案再次进入首页。
  - 2026-06-10：`just mvp-regression` 已增加 20d 偏冷时必须降级为 `audit_only`、必须暴露语义异常 blocker、必须防止“风险已经远离”误读的检查；本轮另做浏览器 DOM 复核确认首页不再显示机械完成度和触线倍数。
- [x] 给首页增加“当前数字可信度清单”，把主结论、正式概率审计态、USDJPY、数据新鲜度和持仓预算分开标注。
  - 2026-06-10：页面首屏已增加数字审计清单；`just mvp-regression` 同步禁止“机械完成度 / 触线仍需 / 还差多少倍”等容易误读的 UI 文案回归。

#### P1：决策面板 MVP

- [ ] 首页首屏只回答四个问题：当前是否危险、风险更像数月/数周/当下、为什么、当前应观察/准备/对冲/防守。
- [ ] 组合动作建议继续保持系统预算口径，不输出自动交易指令；重点展示风险资产上限、现金目标、对冲覆盖和保护性期权比例。
- [x] 历史类比固定覆盖核心美国场景包：`1987 / 2000 / 2008 / 2011 / 2020 / 2022 / 2023`，并展示结构提前量、动作提前量和证据差异。

#### P1：免费数据可靠性

- [ ] 固化日频刷新任务、失败重试和抓取日志。
- [x] 在面板展示每个关键源的最新日期、免费可用性、官方/替代来源和是否影响当前结论。
  - 2026-06-10：决策页“关键免费数据源是否可信”已覆盖 MVP 关键指标：USDJPY、日元隔夜拆借利率、EFFR、VIX；每行展示免费/官方属性、source/dataset、最新日期、替代路径、lineage 和对结论的影响。
- [x] 关键指标缺失或陈旧时，只允许降级结论可信度，不允许静默复用旧值形成高置信结论。
  - 2026-06-10：后端已在 `data_trust` 计算链路加入关键指标 freshness guard；缺失/陈旧关键指标会把数据可信度封顶到不可高置信档，延迟关键指标最多按 B 档解释，并由单元测试覆盖。

#### P2：正式概率模型

- [ ] 在 MVP 页面数字可信之前，不把 formal 模型作为首页主结论。
- [ ] 后续再修 `20d` 当前态过冷、`60d` 背景值、样本稀疏、标签和校准冲突。
- [ ] 只有 release review 与 Go/No-Go 通过后，formal 概率才恢复为主决策输入。

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

- [x] 定义 `prepare / hedge / defend` 独立 episode 目标，而不是继续复用 `60d / 20d / 5d proxy`
- [x] 给 runtime audit / release review 增加 clause-level posture 触发拆解，明确是 probability floor、structural context 还是 transitional bridge 在驱动 `prepare / hedge / defend`
- [x] 让 formal main 训练目标显式约束 `positive_window` 相对 `normal` 的概率抬升，而不是允许全 regime 输出近似常数
- [x] 补训练侧 `actionability` 专属评估口径，区分“提前命中”“过晚确认”“完全漏报”
- [x] 在 dataset summary / 训练输出里显式暴露动作层 `scenario_count`
- [x] 在 dataset summary / release review 里显式暴露 `regime mix` 和按 regime 的 runtime 概率分布
- [x] 把 `actionability` 专属评估继续接入 release review / go-no-go 护栏
- [x] 给 release review 增加 `runtime sanity guard`，避免 candidate 因为“和同样失效的 baseline 一样差”而被误判通过
- [x] 让 `actionability` guard 从“基础拦截”升级到“分层级、分角色”的正式阈值
- [ ] 把 formal history 审计链继续往 `raw point-in-time feature store` 收口，减少对 persisted snapshots 的过渡依赖
  - 2026-06-09：场景回测与滚动审计已优先复用本地 SQLite 中更长的 persisted replay 历史，并在 API/UI 中显式拆开“场景回测历史窗口”“滚动审计历史窗口”和“默认运行历史轨迹”的口径；这解决了用户面板层的历史窗口混淆，但仍属于 persisted replay 过渡模型，不是最终的 raw PIT formal history。
  - 2026-06-09：默认历史 provenance 现已显式区分“当天精确 PIT feature replay”与“沿用旧 PIT snapshot”的 carry-forward 点，避免把 prior-snapshot reuse 误记成完全 formal 的 raw PIT 覆盖；剩余缺口转为继续压缩这类 reuse 点，而不只是笼统统计 `feature_snapshot_id` 是否存在。
  - 2026-06-09：API runtime 已补上“缺失当天 snapshot 时按当天 PIT 规则重建 exact feature snapshot”的路径；在本地 SQLite production reload 下，默认历史与 research audit 已实测收口到 `2000/2000 raw_pit_feature_replay`，`reused_feature_snapshot_points=0`，最后一个 `2026-06-08` 的 prior-snapshot reuse 点已消失。
  - 2026-06-09：`build_formal_feature_snapshot_record` 已下沉到 `crates/domain`，worker `feature build` 与 API `exact PIT rebuild` 现已共用同一套分数归一化、`external_dimension_score`、coverage / visibility status、`feature_count` 组装逻辑；formal feature snapshot 的“记录长什么样”不再由两侧各自维护。
  - 2026-06-09：`raw_pit_feature_replay` 证据等级改成保守解析：只有形如 `market_scope:entity:YYYY-MM-DD:feature_set:pit_mode` 且日期等于评估日的 feature snapshot id 才会标记为当天精确 PIT；非标准或不可解析 id 会降级为 `raw_pit_feature_reuse`，避免把桥接/测试 id 误报成正式 PIT 证据。
  - 2026-06-09：SQLite observation loader 已补同日多来源确定性去重，按 `default_source_id` 与 mapping priority 选择主源，并让运行值与 lineage 使用同一选择口径；这修复了 `USDJPY` 同日 BOJ/FRED 并存时可能因调用顺序混源的问题，也把规则写入 `feature-store-design.md`，后续新增免费源必须先配置 priority。
  - 现阶段剩余问题不再是“默认运行历史里还有最后几个 reused PIT 点”，而是更广义的 feature-store 治理：继续把训练/运行两侧的 PIT 可见性与 formal feature snapshot 逻辑收敛到共享层，并把更早历史区间的可回放覆盖补齐。
- [x] 扩展美国历史压力样本，尽量覆盖 `1987 / 1994 / 2000 / 2001 / 2008 / 2011 / 2020 / 2023` 中免费可回补的区间
  - 2026-06-09：已把场景覆盖矩阵落成机器可读配置 `config/research_scenario_data_coverage.us.json`，并接入 `/api/assessment/method` 与方法页；至少现在“哪些场景可做主训练 / 扩展训练 / protected stress / historical analog、当前缺什么免费数据”已经不再停留在文档口头说明。
  - 2026-06-09：worker 的 `research dataset summarize-main` 现已直接输出 coverage catalog、dataset intent、场景对齐计数，以及逐场景的 `coverage_grade / recommended_role / PIT 口径 / free_sources / blocking_gaps`；formal main 也已按 `main_training + protected_context` 正确识别 `2000 / 2011 / 2022` 这类 protected context，而不会被误判成“混入了不该进入主数据集的扩展样本”。
  - 2026-06-09：`research release review` 现已复用同一份 `scenario_data_coverage_v1`，把场景覆盖上下文接到 `Historical Audit` 和 `Focus Scenarios`：导出报告会直接显示 `Coverage role / Grade / PIT / Free sources / Blocking gaps`，避免训练 summary、历史审计和逐场景复盘继续用三套不同口径解释同一批历史样本。
  - 2026-06-09：`/api/research/audit` 与前端“发布审计”页也已接入同一份 release review 场景覆盖结果；现在可以直接在网页上看到 `回测覆盖 / 重点覆盖 / 主训练可用 / protected stress 可用` 汇总，以及逐场景的 `Family / 原始角色 / 目录结论 / Grade / PIT / 免费主源 / 主要缺口`，避免这部分解释只停留在 CLI/Markdown 工件里。
  - 2026-06-09：新增 `scripts/formal-candidate-scenario-pack-audit.ps1` 与 `just formal-candidate-scenario-pack-audit <baseline> <candidate>`，会按固定美国历史场景包自动选择 `main / ext_stress / ext_acute` dataset，串起 `formal-probability-compare + scenario coverage + release-review blocker`，把“免费数据是否覆盖、该用哪个 dataset、主要卡在 review gate 还是 posture continuity”一次性落成结构化 JSON，避免继续靠手工逐场景拼命令。
  - 2026-06-09：首轮实跑 `112926 -> 173701` 已落出 `artifacts/research/spa/20260606T112926-vs-20260608T173701-default-scenario-pack-audit.json`；当前已不再粗暴落成一堆 `no_review_focus_signal`，而是能直接区分：
    `2022 / 2023 -> posture_continuity`，
    `2008 / 2020 -> shared_missed_signal`，
    `1994 / 2011 -> shared_no_signal`，
    `1987 / 2000 -> stable_pass`，
    `1990-1993 -> stable_pass_with_margin_erosion`。
    这说明下一轮优先级可以更明确地落回 continuity、共享漏报和共享无信号，而不是继续手工解释每个场景。
  - 2026-06-09：`/api/research/audit` 与前端“发布审计”页现已继续接入 `latest_scenario_pack_audit`；现在不只 CLI/JSON 工件可读，网页里也能直接看到固定美国历史场景包的 blocker 分布、逐场景 lead time、dataset 选择、免费主源与结论摘要。
  - 2026-06-09：`/api/research/audit` 与前端“发布审计”页现已继续接入 `latest_rate_shock_audit`；现在不只本地 JSON 可读，网页里也能直接看到 `2022 联储加息与久期冲击` 的 phase/action-level continuity 审计，包括 `primary / late_validation / prepare / hedge` 的命中数、最长连续段、threshold gap 与焦点窗口摘要。
  - 2026-06-09：`/api/research/audit` 与前端“发布审计”页现已继续接入 `latest_dataset_summaries`；网页里可以直接看到 `main / ext_stress / ext_acute` 三套 formal dataset 的真实 `dataset_key / row_count / split 行数 / 正标签 / scenario coverage / blocking gaps`，不再只靠目录口径判断“有没有历史样本”。
  - 2026-06-09：新增 `scripts/formal-dataset-summary-pack.ps1` 与 `just formal-dataset-summary-pack`；它会自动选出 `formal_v1_main_1990_daily`、`formal_v1_ext_stress_1990_daily`、`formal_v1_ext_acute_pre1990` 当前最新且更完整的一版 key，并把 summary 工件导出到 `artifacts/research/dataset-summary-check`，便于把 SQLite 里的真实样本证据固定下来。
  - 2026-06-09：当前 SQLite 已有三套 formal dataset 的真实样本证据：`main 8/8 aligned`、`ext_stress 4/4 aligned`、`ext_acute 2/2 aligned`；对应覆盖已把 `1987 / 1994 / 2000-2001 / 2008 / 2011 / 2020 / 2023` 这些免费可回补的美国历史压力区间落到正式主训练、扩展训练、protected stress 或历史类比角色中。后续剩余重点不再是“这些历史样本有没有”，而是 raw PIT feature store 治理、动作层标签与 release gate 的进一步收口。
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
          - [x] 已定位并修复 API `formal_bundle_v1` runtime 的跨 horizon monotonic uplift：`20d/60d` 不再被 runtime 强行抬到短窗之上；同时把 replay cache key 升到 `runtime_history_v8_20260608`，避免 `default release review` 继续复用旧回放结果。最新 `us_formal_family_hybrid_20260606T112926` 默认 review 中，`60d` 的平均 runtime lift 已从 `+6.3pp` 收敛到 `-0.17pp`，`post_crisis_cooldown avg_probability 54.9% -> 43.9%`，说明 runtime distortion 已基本清掉，剩余 `cooldown_bleed` 更像 bundle 本体的高背景值问题。
          - [x] 2026-06-10 运行时 UI 复核发现：活跃版 `us_formal_family_hybrid_20260606T112926` 在当前样本上的 `20d` head 输出过冷（2026-06-09 为 `0.0067%`，明显低于 `5d=0.14%` 与 `60d=0.076%`）。这不是前端画图错误，也不应通过重新打开 runtime monotonic gap 来硬抬；已把前端说明、release review 最新点诊断和 `20d` horizon coherence guard 落地，避免候选版带同类问题上线。
          - [ ] 回到训练 / calibration / threshold selection，修复 `20d` 当前态过冷与历史 positive-window 连续性之间的冲突；重点解释为什么当前样本 `20d raw=0.0024%`，而 5d/60d 仍明显更高。
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
          - [x] 已把 worker 与 API/backtest 的 strict `prepare p60d` 审计口径先对齐到 runtime floor 派生映射，并明确限制在 formal-main 路径；legacy / heuristic 仍保留旧 `18% / 45%` 口径。
          - [x] 已复核 strict `prepare p20d` 不应继续死写 `18%`：formal-main 路径现在通过 `fc_domain::strict_prepare_p20d_threshold` 从 runtime `external_prepare_p20d` 派生，并钳制在 `12% ~ 18%`；worker release review 已新增回归测试，确认 focus diagnostic 不再把 runtime-derived `p20d=13%` 误报成旧 `18%` gap。
          - [ ] 继续专项复盘 `1990-1993 / 2000-2001 / 2007-2009 / 2023 regional banks` 的 posture continuity，确认 `timely_warning_rate` 卡住的主因到底是 `p20d` gate、posture continuity，还是 residual review clause。
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
            4. 2026-06-09：`just formal-candidate-screen` 已继续接入 default `release review` 的全局治理证据；脚本现在会读取或生成 review artifact，并把 `actionable_precision < 70% / 下滑超过 5pp`、`longest_false_positive_episode_days` 显著变长、`runtime_floor_hit_count` 明显下降，以及 `20d cooldown_bleed / cooldown avg >= positive_window avg` 直接纳入 `no_go_offline`。这一步把 `us_formal_family_hybrid_20260608T191024` 这类“局部连续性看似改善、但 20d cooldown 与误报治理恶化”的候选挡在 fast review 前。
            5. 2026-06-10：`runtime_floor_hit_count` 明显下降已经进一步从 `formal-candidate-screen` 脚本下沉到 `release review` 正式 guardrail；候选相对 baseline 少 5 个及以上 runtime floor 命中点时，会直接进入 `overall_guard_regressions`，不再只作为报告里的 comparison 指标。
            6. 2026-06-10：`20d positive_window` 概率保留率也已从脚本筛查下沉到 release review runtime sanity guard；候选 runtime `positive_window_avg_probability` 相对 baseline 保留不到 75%，或绝对下滑 6pp 及以上时，会直接进入 release review regression。
            7. 2026-06-10：同一条保留率与 cooldown dominance 门禁已扩展到 `60d`，`formal-candidate-screen` 和 release review guard 现在都会拒绝 `60d positive_window` 明显塌陷、或 `60d cooldown >= positive_window` 的候选，避免只靠局部 20d 改善继续推进。
          - [ ] 下一步继续把剩余结论下沉到训练 / 评审策略：
            1. `20d threshold` 只保留 soft penalty / policy guard，不再当作主修复手段；
            2. `USDJPY` 的最终目标仍是迁往“高位 + 变化率/波动率 + 外部确认”的 proxy/context 结构，而不是长期停在 base level cap；
            3. 需要验证新约束下重训出的下一版候选，是否真的不再自然走回 `064930 / 064040` 分支。
          - [x] 已验证新约束下的下一版候选 `us_formal_family_hybrid_20260604T081030` 没有再走回 `064930 / 064040` 分支：
            1. 相对 `034053`，`regional_banks` 的 `20d hits 27 -> 24`、`positive_window hit rate 75% -> 75%`、`positive_window_avg_probability 0.237 -> 0.239`；
            2. 同时 `2023-02` 的 `20d hits 4 -> 1`、`2023-07` 的 `20d hits 12 -> 6`；
            3. `release-review-fast` 与 `strict_rebuild release-review` 都确认 runtime `actionable_precision 54.8% -> 71.4%`、`timely_warning_rate 10.0% -> 10.0%`、`longest_false_positive_episode_days 5 -> 5`；
            4. 结论：`081030` 已成为当前 family-hybrid 主线最好候选，但核心瓶颈已转为“如何恢复 timely warning”。
          - [x] 已补一轮“`curve/bond-spread pair + USDJPY semantics + 20d threshold role`”落地审计：
            1. 已新增 `scripts/formal-candidate-semantics-audit.ps1` / `just formal-candidate-semantics-audit`，
               会把 `curve inversion / fed funds / USDJPY level / USDJPY 20d change /
               jpy carry proxy / rate_shock family context` 逐列对齐到
               bundle 权重、family overlay audit 与 `20d threshold diagnostics`；
            2. 它会直接区分哪些约束已经是 `training_guardrail`
               （当前入口主要在 `apps/worker/src/model/constraints.rs` 与
               `apps/worker/src/probability/threshold/decision/{selection,regime}.rs`），
               哪些仍是 `doc_only` 结论；
            3. 已用 `034053 -> 064930` 旧坏分支完成验证：脚本能直接指出
               `curve tail negative suppression`、`USDJPY base-level 下压`
               与 `USDJPY external interaction` 放大这几类问题；
            4. 当前仍未自动化落实的主项已经被缩到两类：
               `USDJPY 20d change` 的语义迁移，以及
               `BAA spread` 不应变成新 suppressor；
               其最小代码入口已在脚本输出中明确标为
               `crates/domain/src/probability_bundle/features.rs` /
               `apps/worker/src/model/constraints.rs`。
          - [~] 下一步最高优先级改成“恢复 timely warning / actionable lead time”：
            1. 以 `081030` 为新的 family-hybrid 主线基线，不再继续围绕 `034053` 做主线决策；
            2. [x] 已新增固定离线入口 `scripts/formal-candidate-leadtime-audit.ps1` /
               `just formal-candidate-leadtime-audit <baseline> <candidate>`，
               可以直接复盘 `60d` runtime separation、`L2` 但无 `L3` 的历史样本、
               `Focus Scenarios` 的 runtime block mix，以及 `Historical Audit Actions`；
            3. [x] 继续基于这条审计链直接解释为什么 `60d` 仍是
               `separated_but_below_runtime_floor` 或为什么虽然已过 runtime floor，
               仍没有转成更高的 timely warning；
               - 2026-06-10 复跑最新 `default` 审计 `112926 -> 162641` 后，当前候选的 `60d` 已不是“有分离但低于 floor”，而是退化成 `cold_across_all_regimes`，`runtime_floor_hit_count 91 -> 69`、`timely_warning_rate 50% -> 10%`；这类候选应按 No-Go / retrain 处理，不能通过继续放松 floor 解释为可上线。
            4. [x] 继续基于这条审计链解释为什么 `2000-2001 / 1990-1993`
               只有 `L2` 提前量，却始终无法进入 `L3 actionable`；
               - 最新 lead-time artifact 已直接列出 `L2-not-L3` 场景：`1987=6d`、`2000-2001=29d`、`2011=11d`、`2007-2009=47d`、`1998=7d`；当前 `162641` 的 focus blocker 主要退化到 `2023 regional banks` 的 `strict_gate_mismatch / p60d_only`，而 `1990-1993` 在这轮候选里没有形成足够可审计的 candidate runtime block，说明后续优先级应回到训练样本 / feature separation，而不是继续 runtime patch。
            5. 训练、threshold policy、runtime posture 后续都以“提前一周以上可执行预警”作为首要目标，而不是继续优先压短误报。
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
          - [x] 已补 `release review` 报告层回归测试，锁定 `strict_rebuild` 产物必须持续携带
            `History mode`、`Runtime Separation Comparison`、
            `strict_actionable_point_count`、`runtime_floor_hit_count`，
            避免后续 Markdown / 导出路径回退成只有自由文本或丢失双口径字段。
          - [~] 已开始专项审计 `1990-1993 / 2000-2001` 的 posture continuity：
            1. 逐日对齐 `p20d / p60d / posture / time_bucket / actionable bridge`；
            2. 已补 `runtime_actionable_block_category` 与场景级 `runtime block mix`，
               可以结构化统计为什么高概率日期仍长期停在 `normal` 或被其他条件挡住；
            3. 已确认两类 missed 场景的主导阻塞并不相同：
               - `2000-2001` 主要是 `review_gate_gap`，说明 strict review gate 比 runtime floor 更严；
               - `1990-1993` 主要是 `posture_bucket_normal`，说明真正失败的是 posture continuity；
            4. 已补 `runtime continuity facets`，把 runtime 命中但未形成 strict L3 的点继续拆成：
               `posture:* / bucket:* / trigger:* / gate_gap:* / confirmation:*`；
            4.1 [x] `Focus Scenarios` 已直接输出 `Dominant runtime block` /
                `Dominant continuity facet`，不用再先读完整个 block mix 长表；
            4.2 [x] `Focus Scenarios` 已进一步输出 `Primary failure mode`，
                先把场景归类成 `strict_gate_mismatch` /
                `posture_continuity_failure` / `score_confirmation_failure` 等可决策结论；
            4.3 [x] `release review` 已新增 `Failure Mode Summary`，
                可直接汇总 baseline / candidate 各有多少历史场景主要卡在
                `strict_gate_mismatch`、`posture_continuity_failure` 等失败模式；
            4.4 [x] `release review` 已新增 `Historical Audit Priorities`，
                会把 `2000-2001 / 1990-1993` 这类历史样本直接归到
                `strict_review_vs_runtime_mapping` /
                `posture_continuity` 等下一步工作流，
                同时保留 `family / training_role / protected_window` 约束；
            4.5 [x] `release review` 已新增 `Historical Audit Workstream Summary`，
                会把上述 priority 再按 `workstream` 聚合，
                直接显示下一步优先修哪条线、覆盖哪些历史样本、
                以及这些样本属于哪些 `family / training_role / protected stress` 约束；
            4.6 [x] `release review` 已新增 `Historical Audit Takeaways`，
                会把 `workstream summary` 再压成几条直接可执行的结论，
                明确说明是先修 `strict gate vs runtime floor`、
                还是先修 `posture continuity / score confirmation / bridge`；
            4.7 [x] `release review` 已新增 `Historical Audit Attribution`，
                会继续区分每条 workstream 到底是：
                `both_baseline_and_candidate`、
                `baseline_shared_weakness`，
                `candidate_regression`，
                以及 `candidate_revealed_next_blocker`，
                避免把 formal main 既有短板、candidate 本轮真实退化，
                和“candidate 只是暴露出下一层 blocker”混成同一个问题；
            4.8 [x] `release review` 已新增 `Historical Audit Actions`，
                会把 attribution 进一步落成
                `candidate_reject_or_retrain` /
                `next_blocker_fix_before_promotion` /
                `shared_blocker_fix_before_promotion` /
                `baseline_research_fix` 三类动作，
                让最终 recommendation 能直接回答
                “该判退 candidate、先修共享 blocker，
                先修新暴露的下游 blocker，
                还是纳入 formal main 主线修复”；
            4.9 [x] `Focus Scenarios` 的筛选逻辑已继续从“只看 backtest
                `L2/L3` 摘要差异”扩成“也覆盖 `runtime floor hit without L3`
                的真实样本”：
                1. 现在不会再因为 backtest 摘要还没记成 `L2`，就把
                   `2000-2001` 这类 runtime 已经命中 floor 的长窗场景静默漏掉；
                2. 重新执行 `strict_rebuild release review` 后，当前报告已能稳定把
                   `2000-2001`、`2022` 归到
                   `strict_review_vs_runtime_mapping`，
                   同时把 `1987 / 1990-1993 / 1998` 归到
                   `posture_continuity`；
                3. 这样后续 lead-time / posture 主线不再只靠人工翻大 JSON 才能发现
                   哪些历史样本该优先修。
            4.10 [x] `strict_review_vs_runtime_mapping` 已进一步细分出
                `baseline_gate_gap_profile / candidate_gate_gap_profile`：
                1. 当前会直接区分 `p20d_only / p60d_only / p20d_and_p60d`；
                2. `Historical Audit Priorities / Workstream Summary / Takeaways`
                   和控制台摘要都已直接带出这层 subtype；
                3. 这样下一轮训练和 policy 调整可以直接回答
                   “先修 `p60d` strict gate，还是 `p20d/p60d` 都还卡住”，
                   不再需要手工翻 continuity facet 长表。
            4.11 [x] API/runtime 已补 `prepare_probability_plateau`：
                1. 面向 `1987 / 1990-1993` 这类长窗高 `p20d/p60d` 平台期，
                   不再只依赖 `prepare_continuity_bridge` 的 actionability 头；
                2. 新规则会在 `overall / structural / trigger / external / breadth`
                   达到平台期上下文时，把 `posture` 推到 `prepare`、
                   `time_to_risk_bucket` 推到 `months`；
                3. backtest `is_actionable_warning_point` 与 worker strict review
                   已同步识别这条 trigger code，避免 runtime 升级后 review 仍按旧 continuity 口径漏掉。
            4.12 [x] 真实历史证据闭环已补第一轮：
                1. 已重新跑完 candidate 的 `strict_rebuild release review`；
                2. 已修复 `just formal-candidate-leadtime-audit` 的脚本编码损坏并恢复可执行；
                3. lead-time audit 已确认：
                   - `1987` 仍是共享 `posture_continuity_failure`；
                   - `1990-1993 / 1998` 当前主阻塞已收敛到 `strict_gate_mismatch`，而且以 `p20d_only` 为主；
                   - `2000-2001 / 2023` 则更像 `residual_review_l3_failure`。
            4.13 [x] formal-main runtime 分类已从“死锁 `20260531` 精确版本”改成前缀识别：
                1. 已确认 active candidate `feature_formal_v1_main_20260606_gatefix`
                   之前会被 API runtime 误判成 legacy/release 路径；
                2. 误判后会把 runtime threshold、transition bridge、
                   history runtime policy cache key 一起带回 legacy 口径；
                3. 现已统一改成 `feature_formal_v1_main* + formal_label_v1_main`
                   识别，live API 已恢复走 formal bundle runtime。
            4.14 [x] lead-time 转化链审计已从控制台输出升级为结构化证据：
                1. `scripts/formal-candidate-leadtime-audit.ps1` 现会把 summary metrics、
                   runtime separation、L2-not-L3 gap、focus failure mode、block mix、
                   continuity facet、workstream/action 和 takeaways 写到
                   `artifacts/research/leadtime-audit/*-leadtime-audit.json`；
                2. `/api/research/audit` 已按最近一次 release review 的
                   `baseline / candidate / history_mode / market_scope` 匹配最新
                   lead-time artifact；
                3. 前端“发布审计”页已新增“可执行提前量转化审计”，可以直接看到
                   timely warning、strict actionable、runtime floor hits、最长误报、
                   60d 诊断、L2 未转 L3 场景，以及候选主阻塞；
                4. 当前 `112926 -> 173701 default` 审计显示：`60d` 已是
                   `usable_early_warning_separation`，但 timely warning 没有提升，
                   下一步仍应优先修 `strict/actionable` 转化链、`p20d` gate 和
                   posture continuity，而不是继续单独放松概率 floor。
            5. 下一步不再把所有 missed / degraded 场景混成一个问题，而是按 blocker 排序：
               - 先专项复核 `strict p20d gate`，重点覆盖 `1990-1993 / 1998 / 2007-2009 / 2023`；
               - 再专项复核 `months_score_confirmation / posture continuity`，重点覆盖 `1987 / 1990-1993 / 1998`；
               - 最后复核 `residual_review_l3_failure`，重点覆盖 `2000-2001 / 2023`。
          - [ ] 下一步以 `034053` 为保护基线继续收口剩余短误报，但约束顺序必须固定：
            1. 先守住 `regional_banks` 的 `20d hits / positive_window hit rate / positive_window_avg_probability`；
            2. 再处理 `2023-02-03 ~ 2023-02-05`、`2023-02-14`、`2023-07-13` 等剩余 `20d` 误报点；
            3. 只有在不牺牲上述连续性的前提下，才允许小幅回调 `20d threshold`。
         - [ ] 完成 `mixed_systemic` formal overlay 收口；早期 `gate_active_total=0` 已修复，但 2011 仍未形成正式 runtime floor 命中，下一步重点是训练拓扑、contribution 权重治理和 runtime continuity，而不是继续盲调 proxy。
           - [x] 已把 proxy 改成“信用利差 / 曲线 / NFCI”作为慢性压力锚点，`trigger / VIX / external` 只做确认，并把 overlay gate 从 `0.50` 收到 `0.38`。
           - [x] 已用真实 formal dataset 切片和 `formal-probability-compare` 复核 `2000 / 2011`：
             1. 真正的问题不只是 “gate_active_total=0”，还包括 `mixed_systemic proxy` 对
                `overall / trigger / external` 这组 score 特征误用了 `0-100` 门槛，
                但 formal feature store 里实际喂入的是 `0-1` 归一化值；
             2. 修正量纲后，候选 `us_formal_family_hybrid_20260606T112926` 的
                overlay 审计已明显恢复：`mixed_systemic` gate-active rows 从
                `1452/1208/5881`（主要落在 2011 后段与 cooldown）提升为
                `1452/1208/5881` 之外，`2000 / 2011` 的有效窗口也开始能点亮 gate；
             3. 场景级 compare 显示：
                - `2000-2001` 从 `20d hits 0 -> 7`，说明 dotcom unwind 不再完全冷掉；
                - `2011` 仍未形成正式 `20d hit`，但 candidate `max p20d` 已从
                  `0.007 -> 0.234`，不再是“完全无感”；
             4. 正式 `strict_rebuild release review` 结果也同步改善：
                `strict_actionable_point_count 65 -> 67`、
                `runtime_floor_hit_count 360 -> 439`、
                `actionable_precision 69.7% -> 72.5%`。
           - [ ] 但这还不等于 `mixed_systemic` 已经完成：
             1. `2000-2001` 仍落在 `strict_review_vs_runtime_mapping`，candidate 现在是
                `p20d_and_p60d` 双 gate gap；
             2. `2011` 依然只到“有明显抬升但没形成正式命中”的状态；
             3. 下一步不再继续猜 proxy 权重，而是转向 strict gate / runtime floor /
                continuity 的逐场景复核。
             - `2026-06-09` 二轮 funding-stress 绝对贡献审计已确认：`mixed_systemic` proxy 不是缺失，而是已经在候选 scored slice 中活跃；`family_proxy__mixed_systemic` 的 2011 窗口均值约 `0.3206`，归一化均值约 `0.5610`，overlay gate 均值约 `0.3930`，blend 均值约 `0.0982`，overlay 平均贡献约 `+0.0219`。
             - 同一审计显示，2011 绝对概率仍低，主要因为 base head 里一组特征在 20d 窗口把候选概率压低：`us_fed_funds_level`、`us_curve_10y2y_level`、`us_usdjpy_level`、`external_dimension_score × us_usdjpy_level`、`curve × fed_funds` 是主要负贡献；因此下一步不应直接降 runtime floor，而应验证这些负贡献是否来自 evaluation-only trainability、候选权重约束或 family/context 迁移不足。
             - 2026-06-10 复跑最新候选 `112926 -> 162641` 的 funding-stress audit：2011 已从“无正式 runtime floor 命中”推进到 `partial_runtime_signal`，`20d hits=3`、`max p20d=0.839` 已高于 `0.806` floor；但 `60d max=0.0206` 仍低于 `0.028` floor。`formal-train-family-hybrid-dry-run` 同时确认 topology repair 已生效：`mixed_sys_primary_repair train=205`，所以后续不应再把主问题归成“2011 没进训练”，而要转向 `60d` 目标/权重、base head 负贡献和 candidate threshold policy。
         - [x] 已把 `jpy_carry` 继续维持为 proxy-only family，并补齐足以进入正式 overlay 训练的 protected / proxy rows 支持。
           - [x] `proxy-only audit` 现在会把 `protected_action_window` 和 gate-active carry rows 一并视为候选支持，不再和训练数据集构建口径脱节。
           - [x] overlay dataset builder 现在会在 formal main / ext_stress / ext_acute 叠加时合并重复 identity 行，保留更强的 `label/regime/protected_action_window`，不再让主数据集的弱标签覆盖扩展数据。
           - [x] 已按真实 free-history formal dataset 分布把 `jpy_carry` gate 从 `0.50` 收到 `0.38`；当前受保护/预警窗口里的 carry proxy 最高约 `0.389`，旧 gate 在数据上不可能点亮。
           - [x] 候选 `us_formal_family_hybrid_20260606T104037` 已证明 `jpy_carry` 真实进入 `5d/20d` overlay 训练：`configured=5`，并且 fast review `guard_passed=true`，`actionable_precision 75.2% -> 75.8%`，没有带来新的 bundle-level probability guard regression。
           - [x] 下一步继续做 scenario-level audit，确认 `1987 / 1990 / 2024` 这些高 FX 波动窗口里，`jpy_carry` overlay 的收益是否主要来自真正的 protected/pre-warning carry 压力，而不是被少量普通汇率尖峰误带。
             - 2026-06-09：新增 `just formal-candidate-jpy-carry-audit`，按正式 resolver 公式重算三段窗口的 `family_proxy__jpy_carry` 并输出结构化 JSON；首轮审计结果为 `needs_proxy_tightening`：`1987` gate-active `25/25` 有风险上下文，`1990` gate-active `45/45` 有风险上下文，但 `2024` JPY unwind watch window 出现 `29/29` ordinary gate-active，最高 proxy `0.562225 @ 2024-08-01`。
           - [x] 已把 `jpy_carry` proxy 收紧为“快速 USDJPY 变化 + 严格资金/信用/流动性确认，或结构确认后的 VIX/trigger 压力”，不再让 external dimension 单独自我确认。
             - 2026-06-09：复跑 `just formal-candidate-jpy-carry-audit` 后，`1987` gate-active `25/25` 仍全是 supported，`1990` gate-active `49/49` 仍全是 supported，`2024` ordinary gate-active 从 `29` 降到 `0`，最高 proxy 从 `0.562225` 降到 `0.267600 @ 2024-08-05`，总体结论转为 `supported_with_ordinary_spikes_suppressed`。
         - [x] 复核当前 active release 是否仍停在 review fail 的 family candidate；review 结束后已恢复 `us_formal_interaction_tail_extmix10_20260602T061401`。
3. Raw PIT history replay 闭环
   - [x] 新增 historical replay run / point 存储结构
   - [x] release review 默认走 `strict_rebuild`
   - [ ] 让 `analytics_prediction_snapshots` 退回到运行审计与桥接视图角色
     - 已完成一部分：API 默认历史路径对 `bundle-backed release` 已切到 `replay-first`，若无可复用 replay cache 会直接 raw rebuild 并落 replay run，不再静默复用旧 `prediction snapshots`
     - 已完成一部分：`bundle-backed formal release` 的 default history 路径也不再先读取 `prediction snapshots` 做缺口/新鲜度判断；现在 formal 历史只认 replay cache，缺 cache 就直接 raw rebuild
     - 已完成一部分：`bundle-backed formal release` 的历史 raw rebuild 现在只写 `historical replay run / point`，不再把整段历史 assessment 反向回填到 `analytics_prediction_snapshots`
     - 已完成一部分：`assessment history` 点位现在会显式带 `history_source / replay_run_id / feature_snapshot_id`
       - `transitional_snapshot_bridge`：仍在走旧 `prediction snapshots`
       - `raw_observation_rebuild`：fresh rebuild 过程中的中间态，还没写回 replay run
       - `raw_observation_replay`：已经避开旧 snapshot，也已经落成 replay run，但这段点位还没有对上已落库的 PIT feature snapshot
       - `raw_pit_feature_replay`：这段点位既来自 replay，又已经绑定到真实 `feature snapshot id`
     - 已完成一部分：`bundle-backed formal release` 的 raw rebuild 会优先去匹配已落库的 `feature snapshots`，命中后把真实 `feature_snapshot_id` 写回 replay point，而不是继续只拼一个“看起来像 snapshot id”的字符串
     - 已完成一部分：已把 replay cache 版本升到 `history_cache_v5_20260608`，并修掉同日尾点刷新时丢失 replay metadata 的问题；当前默认 `/api/assessment/history` 已回到 `260/260 raw_pit_feature_replay`，`2026-06-08` 也能复用最近一笔 `2026-05-31` 的 persisted `feature_snapshot_id`
     - 已完成一部分：fresh rebuild 返回给前端的同一批历史点现在也会立即带上真实 `replay_run_id`，不需要再等下一次 cache 命中；`history_source / replay_run_id / feature_snapshot_id` 三个字段的运行时语义已对齐
     - 已完成一部分：`bootstrap-formal-release` 已拒绝 `--dataset-source snapshot`，正式发布命令层不再允许把过渡 snapshot 训练直接包装成 formal release
     - 已完成一部分：`research snapshot dataset` 与 `train-probability --dataset-source snapshot` 现在也会拒绝 `formal bundle release`；这条路径只保留给 heuristic/transitional research snapshots，legacy 无 manifest 的老快照也要求 `probability_mode=heuristic_mvp`
     - 已完成一部分：snapshot 过渡训练生成的 manifest 现在会标成 `candidate/shadow`；`release publish` 默认只接受 `approved/healthy` 正式 manifest，候选版必须显式 `--review-only` 才能入库，且 `release activate/publish --activate` 会拒绝直接激活 `candidate/*` 或 `*/shadow` release
     - 已完成一部分：API `/api/system/reload` 已新增 `runtime_purpose=production|review` 分流；默认 production reload 会把 `candidate/*` 或 `*/shadow` release 降级回 heuristic runtime，只有 release review / probability slice 显式带 `runtime_purpose=review` 时才允许临时装载 review-only bundle
     - 已完成一部分：`/api/research/audit` 已新增 `prediction_snapshot_audit`，前端“发布审计”页也已把旧“历史预测快照”改成“运行快照 / 旧桥接视图”；用户现在能直接看到 active release 快照数、其他 release 快照数、formal 截面数和 heuristic / 降级截面数，并且页面明确说明这张表不是 formal history 主证据链
     - 当前剩余缺口：heuristic / 兼容路径仍允许复用 `prediction snapshots`，formal dataset / 长历史审计链也还没有完全摆脱 bridge 视图
4. 美国扩展历史样本落地
   - [x] 把 `1994 / 1998 / 2000-2001 / 2011` 逐个纳入 extension/protected stress 数据集与 summary
   - [x] 保持 `1987` 作为 acute extension + historical analog，不强行并入主宽表
   - [ ] 按覆盖矩阵补齐 `best_effort PIT` 可回放能力
5. 重建、重训、重审计
   - [x] 重建 formal dataset（`ext_acute/ext_stress` 已完成，`formal main` 已按“恢复 timely warning”目标重建）
     - 已修正 formal main 的覆盖门槛实现偏差：`jp_rates_call_rate` 不再被当作主数据集硬依赖，`STLFSI` 仅从 `1993-12-31` 起计入核心/触发硬覆盖
     - 已完成全历史重建：`formal_v1_main_1990_daily:20260606Tfullhistorygatefix`
     - 实测范围已从旧版 `1998-01-05 -> 2026-05-31` 恢复为 `1990-01-02 -> 2026-05-31`，行数 `13296`
   - [ ] 重训 candidate release（下一轮重点不再是压误报，而是恢复可执行提前量）
     - [x] 先修 `strict p20d gate` 与 runtime floor 的映射，避免 `p20d_only` 长期压住 `L3` 转化
       - 已补 worker focus diagnostic 回归测试，锁定 formal-main strict `p20d` 使用 runtime-derived `12% ~ 18%` floor；这只解决 review gate 口径错位，后续 `months_score_confirmation / posture continuity` 和重训仍继续推进。
     - [ ] 再修 `months_score_confirmation / posture continuity`，避免高 `p20d/p60d` 长期停在 `normal`
       - [x] 已修正 `prepare_reference_p60d` 误读 `60d final_probability` 的口径错位；`2026-06-07 strict_rebuild` 下 `1990-10-19` 已从 `normal` 恢复为 `prepare + prepare_probability_plateau`
       - [x] 已去掉 `prepare_continuity_bridge` 对独立 conviction gate 的额外依赖；`2026-06-07 strict_rebuild` 下 `2007-08-01` 已从 `months + normal` 恢复为 `prepare + prepare_continuity_bridge`
       - [x] 已把 `prepare_probability_plateau` 的 `p20d` 从硬编码 `0.45` 改成 runtime 派生门槛；`2026-06-07 strict_rebuild` 下 `1998-09-03` 已从 `months + normal` 恢复为 `prepare + prepare_probability_plateau`
       - [x] 已补一条更窄的 `relaxed probability plateau` continuity 路径，并同步对齐 backtest / worker strict review 的 plateau 识别：
         1. 只放宽“`p20d/p60d` 已极高、但结构/外部上下文略弱”的 plateau 日期；
         2. `2026-06-07` 重新跑正式 `strict_rebuild release review` 后，`strict_actionable_point_count` 已从 `161` 抬到 `173`；
         3. `1987-09-01..03` 与 `1990-07-16..19` 已恢复为 `prepare/months + prepare_history_hysteresis`，说明点位 continuity 修复已经生效；
         4. 但 `timely_warning_rate` 仍停在 `40.0%`，说明这条修复只补到部分点位，主阻塞仍是 continuity，而不是单个 plateau 门槛。
       - [x] 已把 worker / API strict actionable mirror 收窄到只接受 `prepare_history_hysteresis` 的 months 点，避免把弱 `prepare_p60d_structural` / plateau 点误记成 actionable
         1. `2026-06-07` default `release review` 复核后，`strict_actionable_point_count` 进一步从 `173` 抬到 `185`；
         2. `runtime_floor_hit_count` 维持 `327 -> 351`，`actionable_precision` 维持 `52.8% -> 67.7%`，`longest_false_positive_episode_days` 维持 `15 -> 13`；
         3. 说明 mirror 已对齐，但主阻塞仍不是 point-level strict conversion，而是 scenario-level continuity。
       - [x] 已继续把 `history_prepare_hysteresis` 修到 `structural carry + single-day carry grace`
         1. 先把 `structural carry` 的 `structural_score floor` 下调到 `58.0`，补回 `1990-07-25` 这类 `p20d` 已回落但 `p60d / structural / overall` 仍高的 continuity 点；
         2. 再补一条只保留 `1` 天状态、不主动升级姿态的 `carry grace`，允许 `1990-07-26` 这种 `p20d` 短暂塌陷但 `p60d=0.77 / overall=42.3 / structural=55.0` 仍高的冷却日不立刻清空 continuity state；
         3. `2026-06-07` 重新跑 `strict_rebuild` 后，`1990-07-25` 与 `1990-07-27..29` 已恢复为 `prepare/months + prepare_history_hysteresis`，而 `1990-07-26` 仍保持 `normal/normal`，说明这条修复补的是 scenario continuity，不是把本应冷却的一天硬拉成高风险。
       - [x] 已把 `prepare_continuity_bridge` 的 `p20d/p60d` 从 legacy `18% / 45%` 硬门槛改成 runtime 派生 continuity floor
         1. `p20d` 现在跟随 `hedge_p20d` 派生，`p60d` 跟随 `elevated_weeks_p60d` 派生，再限制在 legacy 上限之内；
         2. formal-main 低阈值场景下，`0.13 / 0.23` 这类长窗压力点现在可以稳定进入 `prepare_continuity_bridge` 与 `months`，不再被旧硬门槛压回 `normal`；
         3. 对 legacy / 高阈值路径仍保持原有上限，不把这次修复扩成无界放宽。
       - [x] 已在 `2026-06-08` 正式 `strict_rebuild release review` 下确认：旧的 scenario-level continuity blocker 不再是当前 candidate 的主阻塞
         1. 最新 strict artifact `artifacts/research/release-review/2026-06-08-us_formal_family_hybrid_20260605T202246-vs-us_formal_family_hybrid_20260606T112926-strict_rebuild-release-review.md` 已给出 `Verdict: PASS`；
         2. `1987 / 1998 / 2000-2001 / 2011 / 2022` 在最新 strict/default review 中都已被重新压回 `baseline_shared_weakness` 的 `prewarning_signal_gap / weak_signal_continuity`，而不是继续作为当前 candidate 的 continuity / gate blocker；
         3. 说明这轮 `history_hysteresis / strict gate / weeks-L3` 修复已经足够，后续不应继续围绕旧 continuity bucket 盲加 runtime patch。
       - [x] 已清 `residual_review_l3_failure` 作为当前 candidate 的晋升阻塞
        1. `2023 美国区域银行危机` 在 `2026-06-08` default review 中已不再给 candidate 挂 `candidate_primary_failure_mode`；
        2. 最新 strict artifact 下 `2023` 的 `candidate L3 = 83d`，已优于 baseline 的 `71d`；
        3. 当前剩余问题已经转成 baseline 主线研究项，而不是这版 candidate 的 Go/No-Go blocker。
   - [x] 重跑 release review / rolling audit / runtime regime audit
     - 以下 `2026-06-07` 条目保留为中间检查点；当前结论以 `2026-06-08` 的 strict/default review 和同日 rolling audit 为准。
     - [x] 已重跑 baseline `us_formal_family_hybrid_20260605T202246` vs candidate `us_formal_family_hybrid_20260606T112926` 的正式 `strict_rebuild release review`
       - `timely_warning_rate 40.0% -> 40.0%`
       - `strict_actionable_point_count 161 -> 173`
       - `runtime_floor_hit_count 327 -> 351`
       - `actionable_precision 52.8% -> 67.7%`
       - `longest_false_positive_episode_days 15 -> 13`
       - `guard_passed=false`，当前主阻塞仍是 `1987 / 1990-1993 / 1998` 的 scenario-level `posture_continuity_failure`，以及 `2000-2001 / 2022` 的 `strict_gate_mismatch`
     - [x] 已重跑 baseline `us_formal_family_hybrid_20260605T202246` vs candidate `us_formal_family_hybrid_20260606T112926` 的 `default release review`
       - `timely_warning_rate 40.0% -> 40.0%`
       - `strict_actionable_point_count 173 -> 185`
       - `runtime_floor_hit_count 327 -> 351`
       - `actionable_precision 52.8% -> 67.7%`
       - `longest_false_positive_episode_days 15 -> 13`
       - `guard_passed=false`，当前默认 review 也确认 point-level strict conversion 已改善，但主阻塞仍是 `1987 / 1990-1993 / 1998` 的 continuity 与 `2000-2001 / 2022` 的 gate mismatch
     - [x] 已在 continuity 修复后重跑 baseline `us_formal_family_hybrid_20260605T202246` vs candidate `us_formal_family_hybrid_20260606T112926` 的最新 `default release review`
       - `timely_warning_rate 10.0% -> 10.0%`
       - `strict_actionable_point_count 63 -> 80`
       - `runtime_floor_hit_count 90 -> 91`
       - `actionable_precision 46.4% -> 70.5%`
       - `longest_false_positive_episode_days 16 -> 13`
       - `guard_passed=true`，说明在修掉 runtime monotonic uplift 并强制 replay cache 失效后，candidate 已通过当前默认 runtime/probability guard；默认 review 的剩余问题主要收敛到 `2023 美国区域银行危机 / residual_review_l3_failure`，以及 5 个 `baseline_shared_weakness` 的 residual release-review audit 样本。
       - 下一步不再先修 `runtime monotonic gap` 或 `default review cache`；这些已闭环。后续优先级转成：
         1. 继续下钻 `2023 美国区域银行危机` 的 `residual_review_l3_failure`；
         2. 对 `1987 / 1998 / 2000-2001 / 2011 / 2022` 这 5 个 `baseline_shared_weakness` 样本做 residual review clause / continuity facet 逐点审计；
         3. 只在确认剩余 `cooldown_bleed` 仍来自 bundle 本体后，再继续改 `60d` 训练目标与特征结构。
       - [x] 已清理 `scenario_focus` 的 candidate failure mode 语义噪声：对于 `timely_to_timely` 且 `candidate_first_l3_date <= baseline_first_l3_date`、`candidate_actionable_point_count >= baseline_actionable_point_count` 的场景，不再继续打 `candidate_primary_failure_mode`。
         - `2026-06-08` 实测：`us_regional_banks_2023` 现在保留 `baseline_primary_failure_mode = posture_continuity_failure`，但 `candidate_primary_failure_mode = null`，更符合当前 review 结论。
         - 这一步只收紧了 review 解释层，不改变 runtime block counts / continuity facets / actionable counts，本质上是避免把 baseline 既有短板继续误挂到 candidate 头上。
       - [x] 已把 `residual release-review audit` 进一步拆成更像“pre-warning signal gap”还是“弱连续性信号”的解释层。
         - `2026-06-08` 实测：`1987 / 1998 / 2000-2001 / 2011` 这 4 个 `baseline_shared_weakness` 样本现在会直接提示“窗口里几乎没有 non-normal、runtime floor 或 actionable evidence”，优先回到训练样本 / feature coverage / label window；`2022` 则会提示“已经出现 non-normal 或零星 runtime floor，但没有形成可执行 pre-warning”，优先复核 feature separation、months/prepare continuity 与阈值前置量。
         - 这一步仍然不改变 release review 的 go/no-go 结果，只是把 residual bucket 从“人工猜”收紧成可执行的下一轮排障方向。
       - [x] 已把上面的 residual 解释层正式升级成独立 `historical audit workstream`
         - `prewarning_signal_gap`：当前 `1987 / 1998 / 2000-2001 / 2011` 会单独汇总到这条线，不再和 `2022` 混在一起。
         - `weak_signal_continuity`：当前 `2022` 会单独汇总到这条线，明确它已经有弱 pre-warning 痕迹，但没有形成可执行延续。
         - `2026-06-08` 实测：`historical audit workstream / attribution / actions / priorities / recommendation` 五条输出链都已经切到这两个新 key，最终 recommendation 也会直接点名 `weak signal continuity, pre-warning signal gap`，不再只给笼统的 baseline shortfall 结论。
       - [x] 已新增 `just formal-candidate-workstream-audit`，把 residual workstream 直接落到 dataset evidence
         - 当前入口会按场景自动选 `formal_v1_main_1990_daily / formal_v1_ext_stress_1990_daily / formal_v1_ext_acute_pre1990`，不再因为 `1987 / 1998` 不在 main dataset 就直接失败。
         - `2026-06-08` 实测：baseline `us_formal_family_hybrid_20260605T202246` vs candidate `us_formal_family_hybrid_20260606T112926` 的默认 review 已成功导出 `artifacts/research/workstream-audit/us_formal_family_hybrid_20260605T202246-vs-us_formal_family_hybrid_20260606T112926-default-workstream-audit.json`。
         - 首轮证据显示：
           1. `weak_signal_continuity / 2022` 在 main dataset 里有 `66` 个 evaluation rows、`prepare=48 / hedge=11`，但 `20d/60d` 正标签都是 `0`，说明主问题更像正标签/continuity 约束缺失，不是 coverage 缺口。
           2. `prewarning_signal_gap` 的 `1987 / 1998 / 2000-2001 / 2011` 已分别落到 acute/stress 扩展数据集，合计 `343` 行，`20d` 正标签 `80`、`60d` 正标签 `240`，平均 coverage `0.9665`，说明这些 residual 样本已经具备可训练证据，下一步应直接回到 feature separation / gate / release slice 对比，而不是继续停留在“可能没数据”的猜测层。
         - `2026-06-09` 已把这条 evidence 正式接入 `/api/research/audit` 与前端“发布审计”页：当前会新增 `latest_workstream_audit`，并在页面展示 `Residual Workstream 审计` 区块，直接回答“哪些 residual 主线短板仍存在、各自落在哪个 dataset、覆盖了多少历史场景、正标签和 protected rows 到底有多少”。
         - 当前 UI 的选择策略不再机械绑定“最新 review pair 的最新 JSON”，而是优先展示“与当前 active/review 上下文相关、且 residual 覆盖更完整”的 artifact。原因是最新 pair `112926 -> 173701` 只覆盖 `weak_signal_continuity`，而更完整的 residual 证据仍在 `202246 -> 112926` 这对工件里；如果仍只盯最新 pair，用户会误以为 `prewarning_signal_gap` 已经消失。
       - [x] 已把 `focus scenario` 点位证据补充到 `overall_score / external_shock_score`，并把 `prepare/weeks + plateau/history_hysteresis` 的 L3 漏接住情况单独归成 `score_confirmation_failure`
         - `2026-06-08` 默认 review 产物现在会在 `scenario_focus.interesting_points` 和 Markdown 表格里直接带出 `Base/Cand overall`、`Base/Cand external`，不再只能看概率和 posture。
         - 基于这组新字段，`2023 美国区域银行危机` 的早期窗口已可直接看出：candidate 在 `2022-12-08 ~ 2022-12-13` 已经进入 `prepare/weeks`，`p20d/p60d` 也明显抬升，并带有 `prepare_probability_plateau / prepare_history_hysteresis`，但 `overall_score` 只有 `51.8 ~ 52.6`，因此之前才会落进笼统的 `review_l3_gate_not_satisfied / residual_review_l3_failure`。
         - 现在这类点位会被明确记成 `prepare_weeks_score_confirmation`，对应诊断文案是“prepare/weeks trigger setup stayed below strict score confirmation”，后续若要放宽 strict L3 准入，可以直接围绕这条 clause 做 targeted 实验，而不是继续在 residual 桶里盲改。
       - [x] 已让 `release review` 的 `Focus Scenarios` markdown 同步镜像 `historical audit priority`
         - `2026-06-08` 之后的报告会在每个 focus scenario 下直接展示 `historical audit refinement`，把 `pre-warning signal gap / weak signal continuity / strict gate vs runtime floor` 与 `suggested review` 明确贴回场景本身。
         - 这样读报告时不需要再在 `scenario_focus`、`historical audit priorities`、`historical audit actions` 三段之间来回对照，尤其能避免把 `1987 / 1998` 这类 `baseline_shared_weakness` 误读成“只剩 posture continuity 一条问题”。
       - [x] 已落一条更窄的 `prepare/weeks + plateau + history_hysteresis` strict L3 修复实验
         - 这次没有去降通用 `prepare` score floor，只新增一条更窄的 strict actionable clause：要求 `prepare/weeks` 同时带 `prepare_probability_plateau + prepare_history_hysteresis`、`p20d/p60d` 达到 relaxed plateau 档位、且 `overall >= 51.5 / external >= 33.0`。
         - `2026-06-08` 实测：重新跑 baseline `us_formal_family_hybrid_20260605T202246` vs candidate `us_formal_family_hybrid_20260606T112926` 的 `default release review` 后，`strict_actionable_point_count 80 -> 84`，`timely_warning_rate 10.0% -> 10.0%`、`actionable_precision 70.5% -> 70.5%`、`longest_false_positive_episode_days 13 -> 13`，说明补到的是窄点位而不是泛化放宽。
         - `regional_banks` 的 `2022-12-09 ~ 2022-12-12` 已从 `prepare_weeks_score_confirmation` 转成 `actionable`；`2022-12-08 / 2022-12-13` 仍被保留为 score confirmation gap，`2023-05-04 ~ 2023-05-07` 仍未被放行，因为它们外部冲击上下文仍不足。
         - [x] 已把同一条 `prepare/weeks + plateau + history_hysteresis` strict actionable clause 镜像回 API/backtest `is_actionable_warning_point`
           - 避免 worker `release review` 已把这组点位记成 `actionable`，但 API `rolling audit / timeline / scenario backtest` 仍按旧逻辑漏掉。
           - 已新增 API demo 回归测试，固定 `2022-12-09` 这类真实点位在无 thresholds 与 formal-main thresholds 下都能通过 strict actionable 判定。
     - [x] 已在 `2026-06-08` 再次重跑 baseline `us_formal_family_hybrid_20260605T202246` vs candidate `us_formal_family_hybrid_20260606T112926` 的正式 `strict_rebuild release review`
       - `Verdict: PASS`
       - `timely_warning_rate 40.0% -> 40.0%`
       - `strict_actionable_point_count 165 -> 198`
       - `runtime_floor_hit_count 327 -> 351`
       - `actionable_precision 54.9% -> 67.5%`
       - `longest_false_positive_episode_days 17 -> 17`
       - 当前 strict 结论已经不再把 candidate 记成新的 continuity / strict-gate regression；剩余 workstream 只剩 `weak_signal_continuity (2022)` 与 `prewarning_signal_gap (1987 / 1998 / 2000-2001 / 2011)` 这两条 baseline 主线研究项。
     - [x] 已在 `2026-06-09` 导出最新 rolling audit report
       - `just audit-report` 已生成 `reports/rolling-audit/2026-06-08-rolling-audit.md` 与对应 JSON；
       - 当前 active release=`us_formal_family_hybrid_20260606T112926`，`actionable_precision=67.5%`、`actionable_signal_count=603`、`pure_false_positive_count=96`、`longest_false_positive_episode=17d`；
       - 这份报告已经把“当前线上运行口径”的历史误报/受保护压力窗口重新拉平，后续 priority 应以这份 rolling audit 为准，而不是继续引用 5 月底旧报告。
     - [x] 已对齐 `release activate` 的 `operational guard` 与 `release review` 的 `go/no-go`
       - 现在 `release activate --reload-api` 会读取最新相关 `release review` 产物：
         1. 若目标 release 已在最新正式 review 中被判为失败 candidate，则直接阻止激活；
         2. 若当前 active release 正是该失败 candidate，而目标是它的 reviewed baseline，则允许恢复 baseline，并跳过 runtime regression rollback loop。
       - `2026-06-07` 实测：
         1. baseline `us_formal_family_hybrid_20260605T202246` 已能从 active candidate `us_formal_family_hybrid_20260606T112926` 成功恢复；
         2. candidate `us_formal_family_hybrid_20260606T112926` 现在会被激活链路直接拒绝；
         3. 回退链路已验证成功。
       - `2026-06-08` strict review 再次通过后，当前 active runtime 已重新切回 `us_formal_family_hybrid_20260606T112926`。

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

当前剩余主线不再是“扩展历史样本有没有数据”，也不再是继续围绕旧 continuity/gate bucket 打 patch，而是：

1. 先围绕 `2022 联储加息与久期冲击` 的 `weak_signal_continuity` 做 feature separation、`months/prepare` continuity 与阈值前置量专项审计；
   - `2026-06-09` 已先补一层 scenario-definition 证据：给 `us_rate_shock_2022` 增加 `action_episode_overrides` 后，重建 `formal_v1_main_1990_daily:20260609Trateshockoverride` 的切片已从 `66` 行恢复到 `365` 行，区间覆盖 `2021-11-01 -> 2022-10-31`；
   - 当前 expanded slice 的 `phase` 已变成 `primary=228 / late_validation=137`，且 `protected_action_window=365`，说明之前的弱连续性不只是模型冷，而是 `rate_shock` 默认 episode 模板把 2022 年大部分 protected stress 直接排除在 action episode 之外；
   - 但同一 slice 的 `label_20d / label_60d` 仍然都是 `0`，说明接下来要解决的重点已经从“有没有上下文行”转成“这些 protected rows 在训练目标和 review 口径里如何发挥作用”。
   - `2026-06-09` 已继续把 `ForwardCrisis` episode-native 监督往前推进一小步：`no_positive_main + protected_action_window` 的 `prepare/60d` 与 `hedge/20d` 行，不再只吃 generic `pre_warning_buffer` 软标签，而是改成更保守的 episode-native soft target（`60d=0.48/0.95`，`20d=0.34/0.90`）；
   - 这一步的目标不是把 `2022` 伪装成正式主正例，而是避免它继续被训练成普通冷负样本；对应单测已经补齐，但还没有完成基于新数据集/新目标的 retrain 与 release review。
   - `2026-06-09` 已基于 `formal_v1_main_1990_daily:20260609Trateshockoverride` 重训一版 `family_hybrid` 候选：`us_formal_family_hybrid_20260608T173701`；默认 fast review 相对 baseline `us_formal_family_hybrid_20260606T112926` 的结果是 `guard_passed=true`、`actionable_precision 70.5% -> 73.1%`、`longest_false_positive_episode_days 13 -> 11`，说明这轮改动没有把 runtime 泛化放宽，反而让整体候选更收敛；
   - 但同一轮 `us_rate_shock_2022` 的 formal probability compare 仍显示 `20d hits 48 -> 48`、`60d hits 22 -> 22`、`avg delta 20d overall = -0.009`，说明新增监督还不足以真正拉起 `2022` 的弱连续性；后续主线要从“有没有软目标”进一步收敛到 `feature separation / months-prepare continuity / threshold lead` 本身；
   - `2026-06-09` 已新增固定入口 `just formal-candidate-rate-shock-audit <baseline> <candidate>`，把 `us_rate_shock_2022` 的 formal dataset slice 与 probability compare 合并成 phase/action-level continuity 审计，直接输出 `primary / late_validation / prepare / hedge` 各自的 hit rate、最长连续命中、threshold gap 与 top feature separation；
   - 这份专项审计已经确认当前瓶颈更具体地落在 `primary` 窗口：baseline `primary avg p20d = 1.77%`、candidate `1.14%`，`prepare` 窗口 `20d` 连一次 threshold hit 都没有，`hedge` 窗口也只有 `1` 段、最长 `1` 天；相反 `late_validation` 阶段两边 `20d hit rate` 都还有 `34.3%`。说明当前更像“真正该提前升温的 primary window 太冷”，不是“阈值设置不同”。
   - 同一轮 feature audit 还补出了一条更硬的约束：`2022` 这批 row 在当前 `formal_v1_main_1990_daily:20260609Trateshockoverride` 里全部落在 `evaluation` split，所以单纯给它们补软目标并不会改变 `20d` raw coefficient；这也解释了为什么 baseline/candidate 的 `20d` tracked feature weights 完全一致。下一步若要真正修 `2022`，必须动 split/topology、family overlay 训练入口，或引入更早可训练的同类 rate-shock context，而不是继续只改这批 evaluation rows 的 target。
   - `2026-06-09` 已先把这条约束收敛成一层明确的训练拓扑修复：在 `apps/worker/src/commands/pipeline/dataset/formal.rs` 里，`no_positive_main + protected_action_window` 且已经进入 action episode 的 row，不再继续作为纯 evaluation-only 冷样本参与 formal 训练；当前版本先只把 `primary` phase 重路由到 `train_topology_repair`，不再直接改 calibration，因为前一轮实验已经证实把 `late_validation` 强行推进 calibration 会把 `20d` threshold 顶得过高。
   - 这层 repair 故意只发生在训练加载阶段，不改 persisted formal dataset 的原始 `split_name`，这样 `formal dataset slice / summary` 仍会诚实暴露“2022 在主数据集里原本是 evaluation-only”这条证据；但真正训练时，这批 row 已经能进入 raw coefficient / calibration 目标，不会继续出现“target 改了但系数完全不动”的伪修复。
   - 对应单测已经补到 `commands::pipeline::dataset::formal`，验证 `primary -> train_topology_repair`、其余 row 继续保持 evaluation；下一步要用真实 retrain + rate-shock audit 验证它是否真的把 `2022 primary` 的 `20d/60d` continuity 拉起来，同时不再引入新的 threshold 副作用。
   - `2026-06-09` 已基于这版 `primary-only topology repair` 重训候选 `us_formal_family_hybrid_20260608T191024`；专项 rate-shock audit 相对 `us_formal_family_hybrid_20260608T173701` 已出现实质抬升：`20d hits 48 -> 74`、`60d hits 22 -> 71`、`primary avg p20d 1.14% -> 18.61%`、`primary 20d hit rate 0.4% -> 3.9%`，说明“2022 一直太冷”的核心约束确实来自 trainability，而不是单纯 feature 不动。
   - 但同一版 `release-review-fast` 仍未过护栏：相对当前 baseline `us_formal_family_hybrid_20260606T112926`，`actionable_precision 70.5% -> 61.1%`、`longest_false_positive_episode_days 13 -> 28`、`runtime_floor_hit_count 91 -> 82`，并继续暴露 `20d cooldown bleed`。这说明 topology repair 本身已经证明方向有效，但它还不是最终解；下一步主线必须收敛到 `20d cooldown / false-positive governance`，而不是再怀疑 2022 这批 row 到底有没有进训练。
   - `2026-06-09` 已新增固定入口 `just formal-candidate-cooldown-audit <baseline> <candidate>`，直接读取或生成 default release review，并把 `actionable_precision`、最长纯误报 episode、`runtime_floor_hit_count`、`20d/60d` regime cooldown、候选新增/拉长误报 episode 与场景误报 delta 导出成 `artifacts/research/cooldown-audit/*-cooldown-audit.json`。
     - 对 `us_formal_family_hybrid_20260606T112926` vs `us_formal_family_hybrid_20260608T191024` 的实测结论为 `no_go_cooldown_false_positive`；no-go 原因包括 `actionable_precision_regression`、`longest_false_positive_episode_regression`、`runtime_floor_hit_count_regression`、`candidate_20d_cooldown_bleed`、`candidate_20d_cooldown_not_below_positive`。
     - 同一审计还把 `2023-07-31 -> 2023-08-27` 识别为拉长后的候选误报 episode，把 `2023-07-01 -> 2023-07-24` 识别为 candidate-only 误报 episode；后续候选筛选应优先看这份结构化 JSON，而不是只靠控制台输出。
   - `2026-06-09` 已基于 `2011 mixed-systemic primary topology repair` 再重训候选 `us_formal_family_hybrid_20260609T151925`，并补跑 `funding-stress / cooldown / rate-shock` 三组审计：
     - dry-run 先确认 `topology_repair train=433`、`mixed_sys_primary_repair train=205`，完整训练使用同一套样本路由；
     - `funding-stress` 审计显示 2011 的 `candidate max p20d` 已从 baseline `0.202` 抬到 `0.839`，证明拓扑修复确实能把 mixed-systemic 信号拉起来；但该候选自己的 `20d floor` 同时被抬到 `0.900`，所以 2011 仍是 `0` 次 runtime-floor hit，`60d max` 也只有 `0.0206`；
     - `default release review / cooldown audit` 直接判为 no-go：`timely_warning_rate 50.0% -> 10.0%`、`strict_actionable_point_count 80 -> 21`、`runtime_floor_hit_count 91 -> 59`，虽然 `actionable_precision 69.8% -> 82.1%` 且最长纯误报 `17d -> 4d`，但这是以牺牲提前预警为代价；
     - `rate-shock` 审计也确认同一问题：候选把 2022 的 `primary avg p20d 1.38% -> 21.56%` 抬起来，但 `20d threshold 0.282 -> 0.900` 后 hit 数反而 `48 -> 33`，`60d hits 22 -> 5`；
     - 结论：训练拓扑修复方向有效，但不能单独 promote；下一步必须把 `threshold candidate selection / cooldown penalty / 60d cold_across_all_regimes` 纳入训练或筛选目标，而不是继续只增加 protected rows。
   - `2026-06-10` 已完成第一版 `over-tight 20d threshold repair` 并重训候选 `us_formal_family_hybrid_20260609T162641`：
     - 训练诊断显示 `20d base=0.900 final=0.806 repair=true`，calibration pre-warning hits 从 `3/116` 升到 `7/116`；
     - `funding-stress` 审计显示 2011 已从 `0` 个 20d runtime hit 改善到 `3` 个，`candidate max p20d=0.839` 高于 `0.806` floor，问题从 `no_runtime_floor_signal` 推进到 `partial_runtime_signal`；
     - `rate-shock` 审计显示 2022 的 20d hits 从 `48 -> 81`，late-validation hit rate 从 `34.3% -> 51.1%`；
     - 但 `formal-candidate-screen` 仍判为 `no_go_offline`：regional banks positive-window hit rate `80.0% -> 5.0%`、runtime floor hit count `91 -> 69`、candidate 20d 仍是 `cooldown_bleed` 且 cooldown avg `0.488605` 高于 positive-window avg `0.441499`；
     - 结论：本项已证明“阈值过紧”可以被局部修复，但不能继续单独放宽阈值；下一步必须把 `positive-window retention` 和 `cooldown < positive-window` 作为训练/筛选硬目标，同时继续处理 `60d cold_across_all_regimes`。
   - 为了避免下一轮继续靠猜，threshold diagnostics 现在已经能单独暴露 `episode_native_objective_row_count` 与 `protected_no_positive_main_*` 指标，后续训练输出可直接看出这类行在 calibration evidence 里到底占了多少。
   - `2026-06-09` 同一批专项工件现在也已经被 `/api/research/audit` 和前端“发布审计”页直接消费；做 release review 时，不需要再单独翻 `artifacts/research/rate-shock-audit/*.json` 才能判断 `2022 weak_signal_continuity` 是不是已经改善。
2. 再围绕 `1987 / 1998 / 2000-2001 / 2011` 的 `prewarning_signal_gap` 做训练样本、特征覆盖与标签窗口专项复盘，确认为什么连稳定的 non-normal / runtime floor 都没有形成；
   - `2026-06-09` 已新增固定入口 `just formal-candidate-prewarning-gap-audit <baseline> <candidate>`，针对 `1987 / 1998 / 2000-2001 / 2011` 自动串起 `formal-probability-compare` 与 `formal dataset slice`，把免费数据覆盖、split、forward label、episode-native action label、protected row、20d/60d threshold hit、near-threshold 行数和下一步诊断合并到 `artifacts/research/prewarning-gap-audit/*-prewarning-gap-audit.json`。
   - 首轮实跑 `us_formal_family_hybrid_20260606T112926 -> us_formal_family_hybrid_20260608T173701` 后，四个场景不应再被粗暴归成同一类：
     - `1987 黑色星期一`：`candidate_margin_erosion`，仍有稳定 20d/60d hit，但候选均值边际弱化；
     - `1998 LTCM`：`candidate_margin_erosion`，20d hit 从 baseline 的 `29` 缩到 candidate 的 `24`，但 60d 仍稳定；
     - `2000-2001 科网泡沫出清`：`protected_context_signal_present`，不是“没有数据/没有标签”，而是 protected context 与动作窗口怎样进入正式主线的问题；
     - `2011 美欧融资压力`：`no_runtime_floor_signal`，`20d candidate max p20d=18.4%` 仍低于当前 `28.2%` 对冲线，是真正需要优先审计 feature separation / family context 的场景。
   - 因此下一步如果继续修 `prewarning_signal_gap`，不应再把 1987/1998 当作“没预警”样本，也不应把 2000 当作“没数据”样本；真正的第一刀应聚焦 `2011 funding stress` 的 mixed-systemic feature separation，以及 1998 的候选边际弱化是否来自候选本身而非数据缺口。
   - `2026-06-09` 已新增固定入口 `just formal-candidate-funding-stress-audit <baseline> <candidate>`，专门下钻 `2011 美欧融资压力 / no_runtime_floor_signal`：
     - 自动跑该场景的 `formal-probability-compare` 与 formal dataset slice，并输出到 `artifacts/research/funding-stress-audit/*-funding-stress-audit.json`；
     - 把 split、protected/action episode、20d/60d runtime floor 距离、near-threshold 行数、mixed-systemic family proxy 是否存在、关键 funding/liquidity/curve/VIX/USDJPY feature separation 合到同一份 JSON；
     - 这一步只补证据链，不直接放宽阈值；若审计继续证明 2011 全部是 evaluation split 或缺 mixed-systemic proxy，下一步应先修训练拓扑 / family context，而不是先降 runtime floor。
   - `2026-06-09` funding-stress 审计已升级为“scored slice + 绝对贡献”口径：
     - 脚本现在会额外生成候选 scored slice，并在 JSON 中登记 `candidate_scored_slice_path`、raw/resolved feature 数、候选 resolved relevant features、20d/60d 分组的 base contribution 与 overlay contribution；
     - 当前 `us_formal_family_hybrid_20260606T112926 -> us_formal_family_hybrid_20260608T173701` 结论已经从“可能缺 mixed-systemic proxy”更新为“proxy 活跃，但绝对概率被 base head 负贡献和训练拓扑压低”；
     - 下一轮修复顺序必须先看 `train_topology / analogous mixed-systemic train rows / candidate weight governance`，再决定是否需要 runtime floor 微调；不得把这份审计结果解读为“直接降低对冲线即可”。
   - `2026-06-09` 已先落地训练拓扑第一刀：formal training loader 的 protected topology repair 现在会把
     `extension_only + mixed_systemic_stress + protected_action_window + primary + action_episode_id`
     的 evaluation 行提升为 `train_topology_repair`。
     - 当前 funding-stress slice 中符合条件的真实 2011 行为 `205/213`，全部来自 `extension_only`；
     - `late_validation` 8 行仍保留 evaluation，非 `mixed_systemic_stress` 的 `extension_only` 行也不会被提升；
     - 这一步只解决“2011 primary 行完全进不了训练”的问题，尚未证明新候选通过 release review；下一步仍必须重训候选并跑 funding-stress / cooldown / release-review 审计。
   - `2026-06-09` 已给 `research pipeline train-probability` 增加 `--dry-run` 训练拓扑审计，并新增 `just formal-train-family-overlay-dry-run` / `just formal-train-family-hybrid-dry-run`：
     - dry-run 会加载与正式训练同一套 train / calibration / evaluation row，但不拟合模型、不写 bundle；
     - 输出 `topology_repair`、protected rows、`mixed_sys_primary_ext` 与 `mixed_sys_primary_repair` 在三类 split 中的计数；
     - 后续每次调整 formal loader / split repair / extension dataset 入口时，必须先跑 dry-run 验证真实样本路由，再决定是否启动完整重训。
   - `2026-06-09` 已用 dry-run + 完整重训闭环验证 `2011 mixed-systemic topology repair`：
     - dry-run 输出 `topology_repair train=433`、`mixed_sys_primary_repair train=205`；
     - 新候选 `us_formal_family_hybrid_20260609T151925` 把 2011 `max p20d` 拉到 `0.839`，明显高于 baseline `0.202`；
     - 但候选 `20d floor=0.900`，导致 2011 仍无 runtime-floor hit；同时 `default release review` 退化为 `timely_warning_rate 50.0% -> 10.0%`、`runtime_floor_hit_count 91 -> 59`；
     - 结论：问题已经从“2011 样本完全不可训练”推进到“训练后阈值 / cooldown / 60d 冷信号治理”。
   - `2026-06-10` 已继续闭环 `over-tight threshold repair`：
     - 新候选 `us_formal_family_hybrid_20260609T162641` 将 20d floor 修为 `0.806` 后，2011 20d runtime hits 从 `0 -> 3`；
     - 同时暴露新的硬瓶颈：2023 regional banks positive-window retention 只有 `6.3%`，20d cooldown avg 仍高于 positive-window avg；
     - 后续 TODO 顺序调整为：先做 positive-window retention / cooldown dominance 的候选筛选与训练目标约束，再继续 60d 冷信号。
   - `2026-06-09` 已把这条专项审计接入 `/api/research/audit` 与前端“发布审计”页：
     - API 新增 `latest_prewarning_gap_audit`，从 `artifacts/research/prewarning-gap-audit/*-prewarning-gap-audit.json` 读取与当前 release review baseline/candidate 匹配的工件；
     - UI 新增“提前预警缺口审计”区块，直接展示 `candidate_margin_erosion / no_runtime_floor_signal / protected_context_signal_present` 分类、dataset 行数、20d/60d 命中与下一步建议；
     - 当前页面已经能直接看出：真正的第一优先缺口是 `2011 美欧融资压力 / no_runtime_floor_signal`，而 `1987 / 1998` 更像候选边际弱化，`2000-2001` 已有 protected context 证据。
   - `2026-06-10` 已在前端概率卡里补出 `Base 头贡献` 与 crosshair hover 明细，API/历史回放/正式 slice 也同步透出 top base feature contributions：
     - 这解决的是“为什么 20d 看起来异常偏冷、哪些底层特征在压它”的解释缺口，不是把 20d runtime 值硬抬高；
     - 当前 20d 过冷仍应按模型诊断处理，后续优先看训练拓扑、特征分离和阈值/冷却治理，不能把它误判成纯 UI 问题。
   - `2026-06-10` 已继续修正决策面板的概率可读性：
     - `概率轨迹` 现在保留绝对概率图，同时新增每条期限按自身近期区间归一的相对变化图；hover 明细会显示日期、精确百分比、bp、接口小数和较前点变化，用来确认 `20d` 低位线是否真的没有变化；
     - `离风险还有多远` 现在把当前正式概率、动作进入线、占进入线比例、触线倍数和百分点差值拆开显示，并明确动作进入线不是“危机已经发生线”；
     - 本次只修 UI 解释和排查能力，不改变 active release 概率输出；若 `20d` 长期过冷，仍必须通过候选训练、feature semantics、threshold/cooldown guardrail 处理。
   - `2026-06-10` 用户复核后继续暴露两个产品风险：`20d` 折线在绝对概率图上长期贴底，以及“离风险还有多远”的小数容易被误读成时间距离或零风险证明。
     - 前端已把 hover 改为吸附到最近折线点，并在 tooltip 里明确当前吸附期限、精确概率、bp、接口小数和较前点变化；
     - 决策页已新增风险时距摘要，把 `time_to_risk_bucket`、最接近动作线的触线完成度、`20d head` 偏冷状态前置展示；
     - 后续模型主线仍需继续审计当前 active release 的 `20d current=0.0067%` 与 `hedge floor=28.2%` 的巨大落差，不能通过 UI 文案替代训练 / 阈值 / cooldown 治理。
   - `2026-06-10` 对当前 active release 的 20d runtime contribution 复核确认：最大压分项是 `tail_pos__us_usdjpy_level__145`，在 USDJPY `160`、tail raw `15.0` 时贡献约 `-9.02`，导致高 USDJPY 反而强力压低 20d 危机概率。
     - 这与“日元套息交易可能成为风险放大器”的产品假设冲突，应归为 USDJPY / JPY carry feature semantics 缺陷，而不是前端显示问题；
     - 已给 20d family-context 模型新增训练硬约束：`tail_pos__us_usdjpy_level__145` 必须 `>= 0` 且 `<= 0.18`，并接入 semantics audit；
     - 重训候选 `us_formal_family_hybrid_20260609T200148` 后，semantics audit 已确认该尾部负权重消失，`tail_pos__us_usdjpy_level__145=0.0`。
   - `2026-06-10` 对 `us_formal_family_hybrid_20260609T200148` 执行 `review-only publish + formal-candidate-screen` 后结论为 `no_go_offline`，不能激活：
     - 2023 regional banks 正窗口命中率从 `80.0% -> 0.0%`，20d hits 从 `46 -> 7`，说明虽然平均概率抬高，但没有形成可用的连续触线；
     - 20d final threshold 从 `28.2%` 被推到 `90.0%`，positive-window avg p20d `45.97%` 仍低于进入线，导致“离风险还有多远”继续表现为很小的触线完成度；
     - 60d 正窗口保留严重塌陷，release review 标记为 `cold_across_all_regimes`，runtime floor hit count `91 -> 62`；
     - 结论：这轮只固化 “高 USDJPY 不能作为 20d 强压分项” 的训练护栏；下一轮主线必须同时解决 `20d threshold policy`、`positive-window continuity` 与 `60d cold signal`，不能把该候选发布到生产。
   - `2026-06-10` 已继续验证 threshold repair 是否能单独解决“离风险还有多远数值过小”：
     - 代码层把 20d/60d threshold 的“可用支持”补严为：base threshold 不能只靠 pre-warning 零星命中，还必须有 positive-window 触线，且 positive-window 不得弱于 cooldown；
     - 单测新增 `regime_support_adjustment_rejects_prewarning_only_20d_threshold`，防止以后再接受“pre-warning 有高点但 positive-window 全断线”的 20d floor；
     - 重训候选 `us_formal_family_hybrid_20260609T204721` 仍显示 `20d base=0.900 final=0.900`，`formal-candidate-screen` 仍是 `no_go_offline`，regional banks positive-window hit rate 仍为 `80.0% -> 0.0%`；
     - 结论：阈值规则补丁只能防回归，不能安全修复当前候选；真正瓶颈是 20d/60d 训练分布与特征分离，下一步应优先做 `positive-window vs cooldown/normal` 的训练目标与特征审计，而不是继续调 runtime 进入线。
   - `2026-06-10` 已按用户复核继续修正决策页的“可解释性而不掩盖模型问题”：
     - `概率轨迹` hover tooltip 现在把吸附到的最近折线作为焦点显示，直接给出日期、期限、精确百分比、bp、接口小数和较前点变化；
     - `离风险还有多远` 的顶部摘要与三张概率卡会识别 active release 中 `tail_pos__us_usdjpy_level__145` 在 USDJPY 高位时强力负贡献的语义异常，并显示“读数待审计 / 模型待审计”；
     - 触线完成度改用更精确的百分比和比例小数展示，避免 `0.0238%` 这类极小值被误看成 `0` 或“剩余天数”；
     - 本轮仍不硬抬概率、不激活 No-Go 候选；页面只是诚实暴露当前 active release 输出偏冷，真正修复仍必须通过训练约束、feature transfer 与 release review。
   - `2026-06-10` `formal-candidate-feature-audit` 已补充 bundle evaluation 与 runtime replay 的 regime separation 对比：
     - 20d 候选在 bundle evaluation 里 `positive_minus_cooldown_gap=+8.28pp`，但 runtime replay 变成 `-6.70pp`，说明训练评估到运行回放出现分布漂移；
     - 60d 候选在 runtime replay 中 `positive / cooldown / normal` 均约 `2%`，诊断为 `cold_across_all_regimes`；
     - 后续优先级应放在 train/runtime feature transfer、scenario-conditioned feature separation 和 positive-window continuity，而不是继续只调阈值。
   - `2026-06-10` 已新增 `scripts/formal-candidate-regime-contribution-audit.ps1` 与 `just formal-candidate-regime-contribution-audit <baseline> <candidate>`：
     - 它会复用或生成 `formal-probability-compare` 工件，并按 `20d / 60d`、`regime_20d / regime_60d` 聚合 baseline/candidate 平均概率、命中率、decision threshold 与 top feature contribution；
     - 对 `us_formal_family_hybrid_20260606T112926 -> us_formal_family_hybrid_20260609T204721` 的 `us_regional_banks_2023` 窗口，确认 20d 候选正窗口均值虽升到 `76.48%`，但 `candidate threshold=90.00%`，因此 positive-window hit rate 仍是 `0%`；这解释了“概率看着不低但离触线很远”的一类异常；
     - 同一审计确认 60d 不是阈值小问题，而是正窗口均值从 baseline `95.66%` 塌到 candidate `1.39%`，属于 `cold signal / feature transfer` 问题；
   - 对 `2023-07` 常态窗口，20d candidate normal avg 已达 `56.63%`，占 `90%` threshold 的 `62.9%`；如果只下调阈值，会很容易把常态误报重新放出来；
   - 重要边界：当前 formal compare 的 `regional_banks` 样本没有 normal/cooldown 行，无法单独解释 runtime `cooldown_bleed`；该脚本只能做 top feature delta 级 triage，下一步仍必须补 runtime replay contribution 证据。
   - `2026-06-10` 已新增 `scripts/formal-candidate-runtime-contribution-audit.ps1` 与 `just formal-candidate-runtime-contribution-audit <baseline> <candidate>`：
     - 该脚本复用或生成 runtime `release probability-slice`，直接比较 baseline/candidate 当前回放里的 `base_contributions`、runtime threshold、touchline ratio 与 USDJPY 语义异常；
     - 对 `us_formal_family_hybrid_20260606T112926 -> us_formal_family_hybrid_20260609T204721` 的 `2026-06-09` 当前点，baseline 机械触线完成度为 `5d=0.02842 / 20d=0.000238 / 60d=0.001333`，这解释了面板“离风险还有多远”为什么出现极小数；
     - candidate 20d runtime avg 已升到 `56.7713%`，但 candidate threshold 仍是 `90.0000%`，touchline ratio 只有 `0.630792`，因此不能激活；5d 仍只有 `0.0312%`，60d 只有 `2.8079%`；
     - runtime contribution 明确显示 active baseline 三个 horizon 都命中 `tail_pos__us_usdjpy_level__145` 高 USDJPY 负贡献，candidate 5d/60d 仍未清掉该语义异常，20d 也仍有 `us_usdjpy_change_20d` 正变化负贡献；
     - 脚本已继续扩展为多日期 runtime group attribution：输出 `date_rows`，并按 candidate 的 `time_to_risk_bucket / posture / threshold_state` 聚合 `runtime_group_summaries`；
     - 对 `2026-06-01 -> 2026-06-09` 的实跑结果显示，20d candidate 在 normal/posture normal 下分裂为 `building=5 天` 与 `cold=4 天`，candidate avg `39.0093%` 仍低于 `90.0000%` floor；这说明问题已经具体到“运行时连续性与阈值策略”，不再只是单日显示异常；
     - 结论：当前 No-Go 不是前端显示问题，也不能靠运行时硬抬概率解决；下一步必须做多日期 runtime regime attribution，并把 `USDJPY/Jpy carry semantics`、`20d threshold policy`、`positive-window continuity`、`60d cold signal` 一起纳入训练和 release review。
   - `2026-06-10` 用户再次复核后确认：决策面板十字星 hover 能给出日期、三条线精确概率、bp、接口小数和较前点变化；20d 主图贴底不是绘图 bug，而是 active release 当前 20d head 只有 `0.0067%`，最近窗口 `0.0031% -> 0.023%`，在共用纵轴上被压缩。
     - `离风险还有多远` 的极小数来自 active release 的机械触线完成度：`5d=2.842% / 20d=0.0238% / 60d=0.1333%`，且三期限都命中 USDJPY 高位 tail 负贡献；这些数只能作为审计证据，不能解释成“风险离得很远”。
     - UI 约束已进一步收紧：图表 hover 直接前置“当前十字星”吸附折线读数；风险距离卡在模型语义异常时禁用触线倍数和时距结论，只展示机械审计比例，避免把 `20d` 的极小比例或几千倍机械反推误写成可执行距离。
     - `2026-06-10` 已继续把同一条约束前移到“当前结论怎么来的 / 危机先验”摘要：只要 `5d / 20d / 60d` 任一期限命中 USDJPY 高位 tail 压低读数的语义异常，摘要主值显示“正式概率待审计”，精确概率只作为模型审计证据保留，不能再被摘要层解释成低风险或可用时距。
     - 已把 `tail_pos__us_usdjpy_level__145` 训练护栏从 20d family-context 扩展为 base / family-context 通用的 `5d / 20d / 60d` 约束：5d 上限 `0.12`，20d/60d 上限 `0.18`，三期限下限均为 `0.0`；这只影响下一轮候选训练，不改变当前 active release。
     - 候选 `us_formal_family_hybrid_20260609T224315` 验证了 5d/20d tail 负贡献会消失，但 60d 仍保留 `tail_pos__us_usdjpy_level__145=-7.10`，说明只把护栏挂在 family-context 形态上还不够。
     - 候选 `us_formal_family_hybrid_20260609T230426` 继续验证 base / family-context 通用护栏：runtime contribution audit 中 5d/20d/60d 的 `usdjpy_high_tail_negative` 均已从 candidate anomalies 消失；当前点候选概率为 5d `2.1657%`、20d `56.7713%`、60d `1.6579%`，触线完成度分别为 `56.99% / 63.08% / 59.21%`。
     - `230426` 仍是 `no_go_offline`，不能激活：regional banks 20d positive-window hit rate `80.0% -> 0.0%`，runtime floor hit count `91 -> 62`，20d `cooldown_bleed`，60d `cold_across_all_regimes`；下一步不能继续单点修 USDJPY tail，必须转向 `us_usdjpy_change_20d` 语义迁移、20d threshold/continuity 与 60d feature transfer。
     - 下一步必须重训候选并跑 runtime contribution audit、semantics audit、candidate screen 和 release review；只有候选同时解决 USDJPY 语义、20d threshold/continuity 与 60d cold signal，才能进入 Go/No-Go。
   - `2026-06-10` 已开始 `us_usdjpy_change_20d` 语义迁移的第一步训练护栏：
     - signed `us_usdjpy_change_20d` 与 signed `interaction__trigger_score__us_usdjpy_change_20d` 方向有歧义：上涨可能代表 carry build-up，下跌也可能代表 unwind，因此不能让它们成为强负向 suppressor 或强方向性 driver；
     - 新 guardrail 把这两个 signed 权重在 `5d / 20d / 60d` 全部固定为 `0.0`，并把 `tail_abs_pos__us_usdjpy_change_20d__4` 约束为非负、上限 `0.22`；
     - `formal-candidate-semantics-audit` 已把该项从 `doc_only` 升级为 `training_guardrail`，后续候选会自动检查 base / trigger interaction / abs tail 是否越界；
     - 该补丁仍只影响下一轮候选训练，不改变当前 active release；是否可发布仍必须看 runtime contribution audit、candidate screen 和 release review。
   - `2026-06-10` 已重训候选 `us_formal_family_hybrid_20260609T234038` 验证该 guardrail：
     - semantics audit 显示 `us_usdjpy_change_20d=-0.600932 -> 0`、`interaction__trigger_score__us_usdjpy_change_20d=0.440880 -> 0`、`tail_abs_pos__us_usdjpy_change_20d__4=-0.049667 -> 0`，三项 guardrail 均为 `ok`；
     - runtime contribution audit 当前点显示 candidate 的 `5d / 20d / 60d` 均无 `usdjpy_high_tail_negative` 与 `usdjpy_change_negative` anomaly，说明 USDJPY/Jpy carry 语义异常已被本轮候选清掉；
     - 但 candidate 仍不能激活：20d runtime `54.7972% < 90.0000%`，touchline `0.608858`；60d runtime `1.6087% < 2.8000%`，touchline `0.574536`；
     - candidate screen 仍为 `no_go_offline`：regional banks 20d positive-window hit rate `80.0% -> 0.0%`，20d hits `46 -> 0`，runtime floor hit count `91 -> 41`，actionable precision `69.8% -> 33.3%`，并继续命中 `20d cooldown_bleed`、`60d cold_across_all_regimes`；
     - 结论：USDJPY 语义修复是必要但不充分条件；下一步优先级应转向 `20d positive-window continuity`、`threshold 90% 过高的根因`、`60d cold signal / feature transfer`，而不是继续加单点 USDJPY guardrail。
   - `2026-06-10` 已新增 `scripts/formal-candidate-separation-audit.ps1` 与 `just formal-candidate-separation-audit <baseline> <candidate>`：
     - 它把 `us_regional_banks_2023` 的 `20d positive_window` 与 `2023-02`、`2023-07` 两段误报压力窗口放到同一份 JSON 里，横向比较 candidate feature contribution / delta contribution；
     - 这一步专门服务 `20d threshold=90%` 根因分析：如果同一批特征同时抬高正例和误报窗口，就不能靠继续降低 threshold 解决；
     - 输出会标记 `false_positive_coupled_lift`、`false_positive_only_lift`、`regional_preferential_lift`、`regional_suppression`，作为下一轮训练约束或 family gating 改动的证据入口。
     - 对 `us_formal_family_hybrid_20260609T234038` 实跑显示：regional positive-window candidate avg p20d `73.74%`，但 February false-positive max p20d `87.20%`，二者都低于 candidate threshold `90.00%`；这证明“直接降 20d threshold”会重新放出 2 月误报。
     - 同一审计把当前首批耦合抬升特征锁定为 `interaction__us_curve_10y2y_level__us_fed_funds_level`、`trigger_score`、`external_dimension_score`：这些特征在误报窗口的 delta lift 接近或超过 regional positive-window，应优先做 gating/context 约束，而不是继续调 runtime floor。
   - `2026-06-10` 已把 `20d interaction__us_curve_10y2y_level__us_fed_funds_level` 的 family-context 训练护栏收紧为 `0.18..0.46`：
     - 目的不是运行时硬抬概率，而是防止该交互项塌到 `0` 后丢掉高利率 + 曲线窗口里的 stabilizer 语义；
     - `formal-candidate-semantics-audit` 已把该项升级为 `curve/fed-funds interaction stabilizer band`，后续候选会自动检查。
   - 已重训 review-only 候选 `us_formal_family_hybrid_20260610T004609` 验证该护栏：
     - semantics audit 全部通过，USDJPY 高位 tail 负贡献与 signed 20d change 负贡献在 candidate 中均已清掉；
     - runtime contribution 当前点显示 candidate 概率为 `5d=3.7533% / 20d=54.3004% / 60d=1.6087%`，对应触线完成度 `1.2511 / 0.603338 / 0.574536`；这验证了 active release 的极小数确实来自旧模型语义异常，不是前端 bug；
     - 但 candidate 仍是 `no_go_offline`：regional banks `20d` positive-window hit rate `80.0% -> 0.0%`，actionable precision `69.8% -> 33.3%`，runtime floor hit count `91 -> 41`，并继续命中 `20d cooldown_bleed` 与 `60d cold_across_all_regimes`；
     - separation audit 显示 regional positive-window avg p20d `72.48%` 低于 `90.00%` threshold，而 February false-positive avg/max 已达 `80.10% / 87.20%`；`curve/fed-funds interaction`、`trigger_score`、`external_dimension_score` 仍是正例与误报耦合抬升的主因；
     - 结论：本轮护栏可以保留为“禁止错误语义退化”的训练约束，但不能发布候选；下一步必须做 `20d threshold policy`、family/context gating 与正例/误报分离，而不是继续单点增加 USDJPY 或 curve/fed-funds cap。
   - `formal-candidate-screen` 已把 `formal-candidate-separation-audit` 接入标准 7 步筛选流程：
     - 三段窗口 compare 之后会自动生成 20d cross-window separation JSON；
     - 后续候选第一轮筛选会同时输出正例、2023-02 与 2023-07 误报窗口的耦合抬升特征，不再依赖人工额外记得跑 separation audit。
     - `2026-06-10` 已继续把 separation audit 下沉成 screen 内置的 `20d threshold policy blockers`：当 regional positive-window 均值仍低于候选 threshold，且 February/July false-positive max 接近或超过 regional 均值时，会输出 `threshold_lowering_unsafe` hard blocker，并写入 ignored 的 `artifacts/research/candidate-screen/*-candidate-screen.json`。
     - 对 `112926 -> 004609` 的实测 screen 现在会把 `february_false_positive max p20d 87.20% is 120.3% of regional positive-window avg 72.48%` 直接加入 No-Go reason，避免后续再把“直接降低 20d threshold”当成安全修复路径。
   - `2026-06-10` 已把 separation audit 里识别出的 broad-score 耦合抬升下沉为训练护栏：
     - `trigger_score` 与 `external_dimension_score` 会同时抬高 regional positive-window 和 2023-02 / 2023-07 误报窗口，不能让它们在 family-hybrid `20d` head 中继续成为泛化主驱动；
     - 新 guardrail 只在 family-context 特征集的 `20d forward-crisis` head 生效：`trigger_score <= 0.65`、`external_dimension_score <= 0.42`；
     - `formal-candidate-semantics-audit` 现在会输出 `Broad score weights`，并把这两项列为 `training_guardrail`；`formal-candidate-screen` 也把它们加入 tracked features，后续候选筛选会直接看到是否重新膨胀。
   - `2026-06-10` 继续复核 `021404` 后发现 broad-score cap 仍有 tail 绕行路径：
     - 该候选虽然满足 `trigger_score=0.65`，但 `tail_pos__trigger_score__50` 从 baseline `0.27` 膨胀到 `1.20`，等价于把高触发压力通过 tail 派生项重新变成泛化 `20d` driver；
     - 已把 family-context `20d forward-crisis` 的 broad-score tail 加入训练护栏：`tail_pos__trigger_score__50 <= 0.35`、`tail_pos__external_dimension_score__50 <= 0.25`；
     - `formal-candidate-semantics-audit` 的 `Broad score weights` 与 `formal-candidate-screen` 的 tracked features 现在都会输出这两项，后续候选不能只看 base `trigger_score / external_dimension_score` 是否合规。
   - 已重训 review-only 候选 `us_formal_family_hybrid_20260610T030736` 验证 broad-score tail 护栏：
     - semantics audit 显示 `tail_pos__trigger_score__50=0.35`、`tail_pos__external_dimension_score__50=0.221149`，新护栏全部 `ok`；旧候选 `021404` 则会被标为 `trigger high-tail broad-lift cap violated`；
     - screen 仍为 `no_go_offline`：regional banks positive-window hit rate `80.0% -> 0.0%`、runtime floor hit count `91 -> 41`、actionable precision `69.8% -> 0.0%`，并继续命中 `20d cooldown_bleed`、`60d cold_across_all_regimes`；
     - 本轮改善了误报强度但没有解决核心矛盾：February false-positive max p20d 从旧候选约 `87.2%` 降到 `82.07%`，但 regional positive-window avg p20d 仍只有 `74.27%`，低于 candidate threshold `90.00%`；
     - separation audit 仍显示 `interaction__us_curve_10y2y_level__us_fed_funds_level`、`tail_pos__trigger_score__50`、`trigger_score` 是正例/误报耦合抬升主因。下一步不能继续只加 broad cap，而应做 context gating 或训练目标层的 positive-window vs false-positive separation。
   - `2026-06-10` 已开始把 `20d` 训练目标从“pre-warning buffer 先抬升”调整为“positive-window 必须优先高于 normal / cooldown”：
     - `20d` pairwise target 现在把 `PositiveWindow > PostCrisisCooldown` 设为最强约束，其次是 `PositiveWindow > Normal`，并降低 `PreWarningBuffer > Normal/Cooldown` 的优先级；
     - 新增单测固定这个训练目标顺序，防止后续重新把前置缓冲区抬得比真正危机正窗口更强；
     - 这一步只改变下一轮候选训练的目标函数，不改变当前 active release，也不允许绕过 `formal-candidate-screen`、semantics audit、runtime contribution audit 和 release review；
     - 后续必须重训 review-only 候选并验证：`20d` positive-window hit rate 是否恢复、cooldown 是否低于 positive-window、February/July false-positive 是否不会因 threshold 下调重新放出、`60d cold_across_all_regimes` 是否没有恶化。
   - 已重训 review-only 候选 `us_formal_family_hybrid_20260610T035217` 验证 positive-window 优先 pairwise 目标：
     - 当前点 runtime contribution audit 显示 USDJPY 高位 tail 与 signed 20d change 负贡献已从 candidate 中清掉，`20d` 当前概率从 baseline `0.0067%` 升到 candidate `53.3457%`，说明页面极小值确实来自旧 active release 的语义异常；
     - 但候选仍是 `no_go_offline`：regional banks positive-window hit rate `80.0% -> 0.0%`，20d threshold 仍为 `90.00%`，candidate 20d touchline 只有 `0.59273`；
     - false-positive separation 仍不安全：February false-positive max p20d `79.79%` 是 regional positive-window avg `74.90%` 的 `106.5%`，不能用简单下调 20d threshold 解决；
     - 60d 继续退化为 `cold_across_all_regimes`，当前点 candidate 60d 只有 `1.6087% < 12.00%`，touchline `0.134058`；下一步必须做 20d family/context gating 与 60d feature transfer，而不是发布该候选。
   - `2026-06-10` 已把下一轮候选训练的约束推进到两条更直接的修复线：
     - `60d` tail sign repair：`tail_pos__*` / `tail_neg__*` 现在会跨 forward-crisis horizons 继承底层风险语义，避免 `tail_pos__us_baa_10y_spread_level__2`、`tail_pos__overall_score__55` 这类高位风险 tail 在 `60d` 里变成强负 suppressor；唯一保留的例外是 `20d tail_neg__us_curve_10y2y_level__0`，因为此前审计显示强行非负会重新打开 normal-window 噪声。
     - `20d` broad-score-to-family-context transfer：family-context heads 里 `trigger_score / external_dimension_score / tail_pos__trigger_score__50 / tail_pos__external_dimension_score__50` 的上限进一步收紧为 `0.45 / 0.30 / 0.18 / 0.12`，并新增 `systemic_credit`、`mixed_systemic` proxy/context 下限，迫使正例信号更多通过系统性信用与混合系统性上下文，而不是继续用泛化 broad score 同时抬高正例和 February/July 误报窗口。
     - `formal-candidate-semantics-audit` 已同步这些新护栏，并把 bond-spread high-tail suppressor 从 `doc_only` 升级为 `training_guardrail`；下一步只能重训 review-only candidate，再用 `formal-candidate-screen`、runtime contribution audit 和 release review 判断是否仍为 No-Go。
     - 当前 active release 不变；决策面板上 `5d / 20d / 60d` 极小概率仍应按“模型待审计”解释，不应当成“离风险很远”。
   - 已重训 review-only 候选 `us_formal_family_hybrid_20260610T043016` 验证上述约束：
     - 当前点 runtime contribution audit 显示 USDJPY 高位 tail / signed 20d change 负贡献已清掉；`20d` 当前概率从 baseline `0.0067%` 升到 candidate `60.0664%`，进一步确认页面旧读数是 active release 语义缺陷，不是 UI 画图错误。
     - 候选仍为 `no_go_offline`，不能激活：regional banks `20d` positive-window hit rate `80.0% -> 75.0%`，但 `20d` hits `46 -> 30`、runtime floor hit count `91 -> 84`，且 `60d` positive-window avg probability 只保留 `4.3%`。
     - Threshold 仍不安全：regional positive-window avg p20d `85.42%` 低于 candidate threshold `88.00%`，February false-positive max p20d `89.81%` 是 regional avg 的 `105.1%`，July false-positive max p20d `83.02%` 是 regional avg 的 `97.2%`；不能靠继续下调 `20d` threshold 解决。
     - 下一步优先级应转向 `curve/fed-funds interaction` 的 context gating、`60d` feature transfer/threshold 语义修复，以及 episode-native 动作头质量，而不是激活 `043016` 或继续在运行时硬抬概率。
   - `2026-06-10` 已继续重训 review-only 候选 `us_formal_family_hybrid_20260610T053555`，验证新增 `systemic_credit` trigger/external context 下限：
     - 新增特征和训练护栏生效，semantics audit 显示 USDJPY 高位 tail、signed 20d change、trigger-change interaction 的负向/错误语义已从 candidate 中清掉；
     - 当前点 runtime contribution audit 显示 `20d` 从 baseline `0.0067%` 升到 candidate `59.9069%`，进一步确认用户在页面看到的 `20d` 贴底直线是 active release 语义缺陷，不是前端折线渲染 bug；
     - 候选仍为 `no_go_offline`，不能激活：regional banks `20d` positive-window hit rate `80.0% -> 75.0%`，`20d` hits `46 -> 30`，runtime floor hit count `91 -> 85`，且 `60d` positive-window avg probability 只保留 `4.3%`；
     - Threshold 仍是硬阻塞：candidate `20d` threshold 为 `87.80%`，regional positive-window avg p20d `85.02%` 仍低于 threshold；February false-positive max p20d `89.59%` 是 regional avg 的 `105.4%`，July false-positive max p20d `82.90%` 是 regional avg 的 `97.5%`；
     - 结论：`systemic_credit` context transfer 是必要护栏，但仍不能解决正例/误报耦合和 `60d cold_across_all_regimes`。下一步必须做 `curve/fed-funds interaction` context gating、`60d` feature transfer/threshold 语义修复和 episode-native actionability，而不是发布该候选或简单下调 `20d` threshold。
   - `2026-06-10` 补充候选审计运行治理：
     - `formal-candidate-screen` 与 `formal-candidate-runtime-contribution-audit` 都会临时切换 API active release；这些脚本现在通过 `scripts/review-active-release-lock.ps1` 串行化，避免并发审计时互相覆盖恢复状态，把 review-only No-Go candidate 留在线上；
     - 本轮已把 active release 恢复为 `us_formal_family_hybrid_20260606T112926` 并 reload API；页面继续按“模型待审计”展示 `5d / 20d / 60d` 极小正式概率，不把这些小数解释成风险很远。
3. 只有在上面两条 evidence 清楚后，才决定是否需要新的 candidate retrain；当前 `us_formal_family_hybrid_20260606T112926` 已通过最新 strict/default review，不应继续把 release-review clause 微调当成主线；
4. 继续把 formal history / rolling audit 链从 `persisted snapshots` 的过渡依赖收口到 `raw point-in-time feature store`，避免研究结论长期混用两套历史口径。

### 6.4.1 2026-06-09 免费数据抓取审计链路已补齐

- [x] 已把 backfill 运行审计从“只有 raw payload 和 watermark”补成可追溯的 `ingest_runs -> raw_responses -> ts_indicator_observations` 链路。
  - `crates/storage` 新增 `IngestionRunRecord` 与 `insert_ingestion_run`，`raw_responses.run_id` 现在会写入并受外键约束保护；
  - market 型免费数据回填（FRED / Treasury / World Bank / BOJ / JPY carry 路径）会先写 `running`，结束后更新为 `success` 或 `failed`；
  - 事件型回填（GDELT / SEC EDGAR）也同步使用同一套 `running -> success/failed/skipped` 终态；
  - 失败不再只写日志，`error_type / error_message` 会落到 `ingest_runs`。
- [x] 已用正式本地 SQLite 做单日 FRED/VIX 实测：
  - 修复前新增 `raw_responses.run_id` 会触发外键失败，因为 run 尚未先写入；
  - 修复后 `2026-06-05 VIXCLS` 单日回填成功，`ingest_runs.status=success`、`records_written=1`，新增 raw payload 可通过 `run_id` 反查到该 run；
  - 对应观测 `us_market_vix_close / 2026-06-05 / 21.51` 已绑定新的 `raw_payload_id`。
- 后续约束：
  - 不允许新增抓取路径只写 `raw_responses` 或 watermark 而不写 `ingest_runs`；
  - 页面数据可信度若要回答“这条数据从哪来”，优先从 run/raw/observation 三层链路取证，而不是只看最新观测值。
- [x] 已把关键指标 lineage 接入 `/api/assessment/current` 与决策面板：
  - `KeyIndicatorStatus.lineage` 会按 `run_raw_observation / raw_observation / observation_only / missing` 显式区分证据层级；
  - 决策面板“关键指标是否最新”现在会直接显示 `run+raw / raw / 仅观测` 追溯标签、抓取时间和写入记录数；
  - 当前本地库实测：VIX 已有完整 `run + raw + observation` 链路，USDJPY / BOJ 利率 / EFFR 等历史 raw 记录会诚实显示为 `raw_observation`，不再伪装成完整 run 证据。

### 6.4.2 2026-06-10 MVP 止血版优先级

用户复核后，当前优先级从“继续扩大 formal candidate 研究”临时收敛为“先交付不误导用户的最小可用决策面板”：

- [x] API 增加 `mvp_risk_state`，在 formal 概率命中 USDJPY 高位 tail 语义异常时，把概率状态降级为 `audit_only`，主结论改由保守规则层输出 `observe / prepare / hedge / defend`。
- [x] API `summary` 在 `audit_only` 时不再输出“当前仍偏常态区间”作为主结论，而是明确写成 “MVP 风险状态 + 正式概率审计读数”。
- [x] 决策面板首屏、启动占位、信号层和“离风险还有多远”模块在概率异常时优先展示 `mvp_risk_state.label / summary`，正式 5d/20d/60d 只保留为模型审计证据。
- [x] “离风险还有多远”在异常时禁用机械触线距离，避免把 active release 偏冷误读成风险很远。
- [x] 已完成浏览器验收：首屏显示 `观察为主（概率待审计）`，概率卡显示 `模型待审计 / 审计读数`，风险时距模块显示 `MVP 决策口径`。
- [x] 正式模型研究恢复前，页面主结论、API summary、warmup 面板和方法说明页都不能把 `audit_only` 概率当成可用危机概率；本轮已完成这些入口的口径修正。

### 6.4.3 MVP 迭代式收敛计划

当前开发节奏必须先服务用户最初目标：用免费数据评估当前美国金融体系离危机有多近，并为仓位处置提供可解释的预警，而不是先追求复杂完整的正式概率模型。

阶段性判断：

- 当前未偏离目标，但前一阶段过早进入正式概率候选研究，导致页面上出现“数字很精细、但主结论不可用”的风险；后续迭代顺序必须改为先保证页面数字可信、解释不误导，再继续模型升级。
- 当前最小可用版本完成度约 `60%`：免费数据、SQLite、本地面板、历史类比、数据 lineage 和 MVP 审计态已经可用；正式概率模型、动作模型、自动刷新与告警仍未达到可直接辅助大仓位决策的标准。
- 在 `audit_only` 状态解除前，所有 5d/20d/60d 概率只能作为模型审计读数，不能作为“还有几天/几周离场”的主结论。

后续按以下迭代推进：

1. P0 数字可信与误导止血：
   - [x] USDJPY 等关键指标必须显示真实本地库值、来源、日期和新鲜度；
   - [x] 正式概率异常时，主结论切到 `mvp_risk_state`；
   - [x] “离风险还有多远”在模型异常时不再输出机械触线距离；
   - [~] 对首屏所有核心数字建立 snapshot/UI 回归测试，防止以后再次出现 0、旧价、极小比例被解释成低风险。
     - 已新增 `just mvp-regression`，对运行中的本地 API 校验 SQLite 模式、USDJPY 与 JPY carry 一致性、近端关键指标存在性，以及 USDJPY tail 语义异常时必须进入 `audit_only`；
     - 2026-06-10：`just mvp-regression` 已继续补强仓位预算和用户偏好约束，要求动作建议必须标记为系统预算、禁止自动执行、要求人工确认，并在 `audit_only` 摘要里明确“审计读数不能解释成风险已经远离”；
     - 仍需补浏览器/DOM 级回归，覆盖“危机先验（审计）”“MVP 决策口径”和首屏文本是否实际渲染。
2. P1 最小可用决策面板：
   - [x] 把主结论固定为四档：观察 / 准备 / 对冲 / 防守，并在每档给出仓位动作边界；
     - 2026-06-10：组合动作建议面板新增四档动作边界表，覆盖风险资产上限、现金目标、对冲覆盖、期权保护、杠杆上限和执行窗口；`audit_only` 时当前档按 MVP 规则层高亮，正式概率只保留为审计读数；
   - [x] 把当前状态与 1987、2000、2008、2011、2020、2022、2023 的历史前窗口放到同一张“相似度 + 领先天数 + 证据差异”视图；
     - 2026-06-10：后端历史类比不再只截断 Top 3，而是固定纳入 `1987 / 2000 / 2008 / 2011 / 2020 / 2022 / 2023` 七个美国核心场景并按相似度排序；前端历史类比面板已改成“历史场景 / 相似度 / 结构提前 / 动作提前 / 证据差异”表格；
   - [~] 对“结论把握度”重新定义为数据覆盖、模型状态、事件确认和历史相似度的组合，不再长期固定在一个难解释的数值。
     - 页面已改为“结论可靠性”，按数据覆盖 35%、模型状态 25%、事件确认 20%、历史相似度 10%、关键数据新鲜度 10% 汇总；
     - `audit_only` 时可靠性分数会封顶并明确提示“不能解释成模型结论已经很有把握”；
     - 后续仍可把该公式下沉到 API contract，避免前端长期独占解释口径。
3. P1 免费数据可靠性：
   - [ ] 固化日频刷新任务、失败重试和抓取日志；
   - [ ] 对 FRED/BOJ/Treasury/SEC/GDELT/公开市场数据分别显示最新日期、免费可得性和替代源；
   - [ ] 对缺失或滞后的关键指标降权，而不是静默沿用旧值。
4. P2 正式概率模型修复：
   - [ ] 先完成 raw point-in-time feature store 与训练样本可行性审计，再决定是否继续重训；
   - [ ] 对 active release 的 USDJPY 高位 tail 负贡献、20d/60d 过冷、阈值过高分别做训练侧根因修复；
   - [ ] 任何候选 release 上线前必须通过 release review / Go-No-Go，不能靠运行时硬调概率。
5. P2 产品化告警与操作闭环：
   - [ ] 增加“今日变化原因”和“本周风险变化”；
   - [ ] 增加可配置提醒阈值，但默认只提醒、不自动交易；
   - [ ] 输出组合动作清单时必须附带“不适用场景”和人工确认项。

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

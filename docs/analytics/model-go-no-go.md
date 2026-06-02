# 正式模型准入与 Go/No-Go

状态：`Draft`

最后更新：2026-06-01

## 1. 目标

定义什么样的模型可以：

1. 继续停留在研究态；
2. 成为候选 release；
3. 激活为正式线上默认概率层；
4. 被拒绝或回滚。

一句话说清楚：

不是“能训练出来”就能上线，更不是“前端能看到概率”就算正式模型。

## 2. 发布等级

| 等级 | 含义 |
|---|---|
| `research_only` | 只能用于离线研究和报表，不可线上默认展示 |
| `candidate` | 工程产物完整，可进入人工评审 |
| `active_experimental` | 可以在线运行，但前端和方法页必须明确标记为实验/过渡 |
| `active_default` | 可以作为默认正式概率层对用户展示 |

## 3. `active_default` 的基本前提

全部满足才允许：

1. 正式数据集来自原始观测和 PIT 特征，而不是 `prediction snapshots` 反推。
2. `point_in_time_mode` 至少达到文档声明标准。
3. 标签目录不止 `2008 / 2020 / 2023` 三个主场景。
4. 历史滚动审计和场景回测结果不比当前基线明显更差。
5. 模型能改善 posture 决策，而不只是重命名旧风险分。

## 4. Go / No-Go 维度

### 4.1 数据基础

必须满足：

- `1990+` 主面板连续可回放；
- `core_feature_coverage >= 0.90`
- `coverage_score >= 0.85`
- 关键特征可见性达标；
- 样本切分、缺失值、代理使用全部可追溯。

任一不满足：

```text
No-Go
```

### 4.2 标签基础

必须满足：

- `formal_label_v1_main` 稳定；
- `mandatory` 场景都已进入标签流水线；
- `acute_start`、`crisis_start`、`protected_window` 可追溯；
- 扩展场景角色不再写死在代码里。

若仍只有 3 个起点常量：

```text
最多 active_experimental
```

### 4.3 模型质量

至少满足：

- `ECE <= 0.05`
- `Brier score` 不差于当前 active baseline
- `p_5d / p_20d / p_60d` 不出现明显系统性失真
- 单调性保护后无明显概率反常

### 4.4 场景提前量

至少满足：

- `2008 / 2020 / 2023` 三个主场景中，动作级预警不得明显晚于当前基线
- `p_5d` 在急性样本上不能只在事件发生当天才抬升
- `p_60d` 不能长期无差别高位漂浮

### 4.5 滚动审计

至少满足：

- 最长纯误报区间不得显著恶化
- 纯误报 episode 数量不得显著恶化
- 受保护压力窗口和纯误报必须能区分
- 动作信号精度不得严重下滑

第一阶段建议最低线：

- `actionable_precision >= 0.70`
- `longest_false_positive_episode_days <= 30`
- `false_positive_episode_count` 不高于当前基线太多

### 4.6 决策价值

必须满足：

- posture 的升级/降级更稳定，不是更抖；
- 能明确区分：
  - `观察`
  - `准备`
  - `对冲`
  - `防守`
- 对组合动作有可解释价值，而不是只是数值变化更复杂。

如果做不到：

```text
No-Go for active_default
```

## 5. 明确的 No-Go 条件

出现以下任一情况，直接不允许晋升：

1. 训练集仍来自 `prediction snapshots` 过渡产物。
2. release 声称 `strict PIT`，但底层其实主要靠 FRED CSV。
3. 关键特征覆盖率低于门槛。
4. 只有 `2008 / 2020 / 2023` 三个危机起点，且无扩展场景治理。
5. 模型只在危机发生后才抬升。
6. 误报显著拉长，动作层比启发式更差。
7. 前端无法解释概率来自哪些证据。

## 6. `active_experimental` 的允许条件

若满足以下条件，可以保留在线实验态：

- release bundle 可训练、可加载、可回滚；
- 数据和标签链路至少部分版本化；
- 概率结果比纯启发式更有研究价值；
- 前端明确提示这是实验/过渡版本。

这正是当前系统更接近的位置。

## 7. 对当前系统的判断

截至 `2026-05-31`，当前仓库的 formal bundle：

- 已具备工程链路价值；
- 已具备审计和回滚价值；
- 但仍属于“过渡版 formal serving”。

按本文件标准判断：

```text
current_state = active_experimental
not active_default
```

原因：

- 训练仍依赖 `prediction snapshots`
- 还没有最终 raw PIT feature store
- 虽然 `scenario catalog` 已配置化，但扩展样本还未完整沉淀为最终正式数据集
- `rolling audit` 仍需要兼容 persisted snapshots 的 bridge 规则，尚未切换到完整 raw PIT 历史

这句人话解释是：

- 当前系统已经不再把危机起点散落在代码常量里；
- 但历史审计仍要迁就旧版落库快照，因为那批快照里的概率很多时候被校准下限压平了；
- 所以现在的动作级回测属于“过渡期可解释版本”，还不是最终 `raw observations -> PIT features -> formal dataset -> release` 的正式闭环。

### 7.1 2026-05-31 实际候选复核结果

本轮先后复核了两个 formal PIT 候选：

- `us_formal_pit_20260531T160129`
- `us_formal_pit_weighted_20260531T171025`

它们相对旧过渡版的共同特点是：

- 数据链路上已经比旧过渡版更正式；
- 概率校准指标也更好；
- 但对本项目真正关心的“提前离场能力”更差。

对 `us_formal_pit_20260531T160129`，实测对比当前 transitional baseline：

- `timely_warning_rate`: `37.5% -> 12.5%`
- `actionable_precision`: `29.6% -> 20.6%`
- `longest_false_positive_episode_days`: `9 -> 18`

对加入正样本加权后的 `us_formal_pit_weighted_20260531T171025`，结果没有实质改善：

- `timely_warning_rate`: `37.5% -> 12.5%`
- `actionable_precision`: `29.6% -> 20.6%`
- `longest_false_positive_episode_days`: `9 -> 18`

我们还额外验证过一轮“运行时动作阈值下调”，即：

- 对 `feature_formal_v1_main_20260531 + formal_label_v1_main` 这条正式主线，
- 将 `prepare / hedge / defend` 阈值从旧过渡版的高阈值切到更低的 PIT 口径阈值，
- 然后重新跑 release review。

结果依然没有改变护栏结论。

随后又补做了一轮更严格的复核：

- API 不再继续盲信旧的 `prediction snapshots`；
- 对带有 bundle-backed 概率模型的 active release，如果历史缓存版本失配，会基于原始观测全量重建该 release 的历史评估轨迹；
- 再重新跑 `release review`。

结果仍然没有改变：

- `us_formal_pit_20260531T160129` 仍然是 `37.5% -> 12.5% / 29.6% -> 20.6% / 9 -> 18`
- `us_formal_pit_weighted_20260531T171025` 仍然是 `37.5% -> 12.5% / 29.6% -> 20.6% / 9 -> 18`

这意味着：

- “旧缓存污染了 formal 候选版复核结果” 现在已经不是主要解释；
- formal PIT 主线当前确实没有学出足够稳定的动作级提前量。

### 7.2 2026-06-01 候选 `us_formal_pit_scensplit4regime4_20260601T055211` 复核补充

今天又复核了一版更晚的 formal main 候选：

- `us_formal_pit_scensplit4regime4_20260601T055211`

这轮先暴露出一个工程问题：

- API 历史缓存只看 release manifest 版本，没把运行时 posture/action 阈值策略带进 `method_version`；
- 导致 formal main 已经改过运行时阈值，但 SQLite 里还在复用旧 posture 历史；
- 旧缓存下会出现明显不合理的历史点，例如 `p_20d≈2.5% / p_60d≈7%` 仍被记成 `prepare/hedge`。

因此本轮先修了缓存口径：

- `analytics_prediction_snapshots.method_version` 现在额外包含 `runtime_policy=...`
- 只要 bundle-backed release 的 runtime 阈值或缓存版本发生变化，就会强制重建该 release 的历史评估轨迹

修复缓存后再重跑 review，结果和“旧缓存口径”完全不同：

后续又补了一层 posture clause 持久化复核：

- `posture_trigger_codes` / `posture_blocker_codes` 已写入 `prediction snapshots` 并能随 SQLite round-trip 保留；
- 对当前 active baseline `us_formal_transitional_20260531T094603` 和候选 `us_formal_pit_scensplit4regime4_20260601T055211` 再跑 runtime review 后，`posture_trigger_distribution=[]` 已确认是“全历史均为 normal”的模型输出结果，而不是 clause 丢失。

- 旧缓存口径下，候选曾表现为：
  - `timely_warning_rate`: `22.2% -> 55.6%`
  - `actionable_precision`: `80.0% -> 43.6%`
  - `longest_false_positive_episode_days`: `2 -> 66`
- 新缓存口径下，候选实际表现为：
  - `timely_warning_rate`: `22.2% -> 0.0%`
  - `actionable_precision`: `80.0% -> 0.0%`
  - `longest_false_positive_episode_days`: `2 -> 0`

这句人话解释是：

- 之前看到的“提前量大幅提升但误报过长”，很大一部分是旧历史 posture 缓存污染出来的假象；
- 真正按当前正式 runtime 阈值回放后，这版 formal main 候选反而几乎完全打不出动作级预警；
- 所以当前主问题已经从“误报太长”收敛成“模型本体太冷、动作层完全打不出来”。

结论不变，而且更明确：

```text
No-Go for active_default
```

下一步不该继续围着前端或缓存打转，而应回到：

1. 正式标签窗口是否过窄；
2. formal main 概率头是否整体偏冷；
3. runtime posture 阈值和训练目标是否脱节；
4. 是否需要独立 actionability head，而不是继续只靠概率阈值推 posture。

### 7.2 2026-06-01 场景感知加权复核结果

本轮又补做了一次更贴近业务目标的训练尝试：

- 保留现有 formal dataset 主线；
- 不再只做简单的正负样本类别加权；
- 额外引入 `scenario family + horizon role + days_to_crisis_start` 的正样本权重；
- 同时把 `5d` 急性场景的标签锚点切到更接近 `acute_start` 的位置。

生成的候选版：

- `us_formal_pit_scenweight_20260531T184905`

离线指标仍然不差：

- `brier=0.0134`
- `log_loss=0.0695`
- `ece=0.0040`

但 runtime guard 结果仍然没有改善：

- `timely_warning_rate`: `37.5% -> 12.5%`
- `actionable_precision`: `29.6% -> 20.6%`
- `longest_false_positive_episode_days`: `9 -> 18`

这进一步说明：

- formal 主线现在的问题不只是“训练时没给正样本足够权重”；
- 更像是当前二元 horizon 标签本身就没有把“提前一周可执行离场”表达清楚；
- 下一步应优先进入 `action-oriented label / episode objective / separate actionability model`，而不是继续微调同一套逻辑回归权重。

这说明当前主要矛盾已经不是“线上动作阈值太高”这么简单，而更可能是：

- 正式训练样本的标签过稀；
- 特征和动作级目标之间还没有学到足够强的提前量；
- 目前的 formal bundle 仍更擅长做校准后的概率排序，不擅长做“危机前一周可执行离场”的动作决策。

结论：

```text
No-Go for active_default
No-Go for active_experimental default serving
```

因此这两个 release 目前都只保留为：

- 已训练完成的研究候选；
- 不作为当前默认线上版本；
- 后续需要先解决标签与目标函数对“提前量”的约束，再重新评估。

当前建议继续保留 `us_formal_transitional_20260531T094603` 为 active release，直到：

- raw PIT history 审计链完整打通；
- 新数据集在动作级护栏上至少不劣于当前 transitional baseline；
- 前端也能清楚解释“概率低但系统仍可能偏脆弱”的语义。

### 7.3 2026-06-01 动作窗口标签复核结果

本轮又补做了两版 `action-oriented label` 实验：

1. `us_formal_pit_actionwin_20260531T192253`
2. `us_formal_pit_actionwinv2_20260531T193705`

对应数据集：

1. `formal_v1_main_1990_daily:20260601Tactionwindow`
2. `formal_v1_main_1990_daily:20260601Tactionwindowv2`

两者差别：

- 第一版把动作标签延续到整个 `crisis_end`，结果误报拖得过长；
- 第二版改为 bounded action window，只保留“前置缓冲 + 危机起点后有限验证窗口”，并保留跨角色 fallback 样本但训练降权。

结果：

- `actionwin`：
  - `timely_warning_rate`: `37.5% -> 12.5%`
  - `actionable_precision`: `29.6% -> 10.2%`
  - `longest_false_positive_episode_days`: `9 -> 84`
- `actionwinv2`：
  - `timely_warning_rate`: `37.5% -> 12.5%`
  - `actionable_precision`: `29.6% -> 20.6%`
  - `longest_false_positive_episode_days`: `9 -> 18`

解释：

- bounded action window 明显比“全危机持续为正例”更合理；
- 它把 `actionable_precision` 和误报持续天数从失控状态拉回来了；
- 但 `timely_warning_rate` 仍停在 `12.5%`，说明单头概率模型还是没学到“危机前可执行离场”这件事。

因此本轮结论进一步收敛为：

- 不能再把“改标签”理解成只要换掉 `label_*` 就能解决；
- formal 主线已经证明：`forward label only` 不够，`full action window` 也不够，`bounded action window` 仍不够；
- 下一步应该进入 `dual-head / separate actionability layer / episode objective`，而不是继续在同一单头逻辑回归里拧权重和阈值。

### 7.4 2026-06-01 dual-head actionability 首轮复核结果

本轮已经把 `separate actionability layer` 的第一版工程链路打通：

- `ProbabilityBundle` 新增独立 `actionability` bundle；
- API `assessment/current` 新增 `actionability.prepare / hedge / defend`；
- `method` metadata 新增 `actionability_model_version / actionability_calibration_version / fusion_policy_version / actionability_enabled`；
- runtime 允许在新 release 上启用动作头与 posture / time bucket 的诊断融合。

对应候选版：

- `us_formal_pit_dualhead_20260601T003145`

它的离线校准指标并不差：

- `brier=0.0134`
- `log_loss=0.0695`
- `ece=0.0040`

但 release review 结果仍然没有通过：

- `timely_warning_rate`: `37.5% -> 12.5%`
- `actionable_precision`: `29.6% -> 20.6%`
- `longest_false_positive_episode_days`: `9 -> 18`

这次结果有三个重要含义：

1. 现在已经可以确认，问题不在“API 没暴露动作头”或“线上没有融合逻辑”；
2. 当前这版 dual-head 本质上仍是 `bounded action window` 的 `60d / 20d / 5d proxy`，还不是独立定义的 `prepare / hedge / defend` 训练目标；
3. 简单把动作头接到 serving 侧，不会自动制造出模型原本没有学到的“提前一周可执行离场能力”。

因此这轮结论应进一步收敛为：

- `dual-head plumbing` 值得保留，因为它把后续实验入口和前端解释层打通了；
- 但 `proxy actionability head + serving fusion` 还不能替代当前 active release；
- 下一步不该继续围绕阈值和融合细节小修小补，而应进入：
  - `episode-native actionability labels`
  - `prepare / hedge / defend` 独立目标设计
  - `raw PIT history` 审计链补全
  - 场景样本治理与更早历史覆盖扩充

结论：

```text
No-Go for active_default
No-Go for replacing transitional baseline
```

当前默认线上版仍应保持：

- `us_formal_transitional_20260531T094603`

### 7.5 2026-06-01 actionability guard 已接入 release review

本轮不是只补了训练侧指标，还把动作层护栏正式接进了 `research release review`：

- review 报告现在会单独展示 `prepare / hedge / defend` 的：
  - `scenario_count`
  - `advance_warning_rate`
  - `late_confirmation_rate`
  - `missed_rate`
  - `pre_start_recall_at_threshold`
  - `post_start_recall_at_threshold`
- guard 现在不再只看 runtime 的 `timely_warning_rate / actionable_precision / false_positive_episode_days`；
- 对内置动作头的候选版，还会额外拦截两类情况：
  1. `scenario_count < 2`
  2. evaluation 正样本存在，但动作头一个命中都没有

对应复核候选版：

- `us_formal_pit_dualheadguard_20260601T012122`

新 review 结果比之前更明确：

- runtime guard 仍然失败：
  - `timely_warning_rate`: `37.5% -> 12.5%`
  - `actionable_precision`: `29.6% -> 20.6%`
  - `longest_false_positive_episode_days`: `9 -> 18`
- actionability guard 也失败：
  - `prepare / hedge / defend` 的 `scenario_count` 都只有 `1`
  - 三个层级在 evaluation 正样本上都没有任何命中

这一步的意义是：

- 以后就算某个候选版 runtime 护栏勉强没退化，只要动作头评估还是“单场景 + 零命中”，也不允许把它说成正式可晋升候选；
- `actionability` 现在已经从“页面上多显示几个数”升级成了真正参与 Go/No-Go 的对象。

### 7.6 2026-06-01 formal main `scensplit4` 继续复核：问题已收敛到标签 / regime 口径

在 `formal_v1_main_1990_daily:20260601Tscensplit4` 这条线上，本轮继续做了三类修正：

1. `20d / 60d` 主概率头允许在“raw ranking 明显反向”时保留负 `alpha` 校准，而不是一律丢弃；
2. 动态阈值允许落到 `1%` 以下，但把 `5d` 重新收回保守底线，避免短端阈值被放得过低；
3. API runtime 对 `actionability` 增加了 `probability + structural/trigger context` 融合门控，不再只看动作头自己的 margin。

对应候选版：

- `us_formal_pit_scensplit4negcal2_20260601T041308`
- `us_formal_pit_scensplit4negcal3_20260601T043828`

复核结论仍然是否决，而且结论已经足够清晰：

- `negcal2` 激活护栏结果：
  - `timely_warning_rate`: `37.5% -> 25.0%`
  - `actionable_precision`: `29.6% -> 9.1%`
  - `longest_false_positive_episode_days`: `9 -> 128`
- `negcal3`（仅把 `5d` 阈值收回）激活护栏结果：
  - `actionable_precision`: `29.6% -> 10.1%`
  - `longest_false_positive_episode_days`: `9 -> 288`
  - 依然触发自动回滚

这里最关键的新发现不是“阈值还没调好”，而是：

- `20d / 60d` raw 主模型在 calibration / evaluation 上确实存在**反向排序**；
- 这不是纯 serving bug，而是 `forward_crisis` 标签把大量“危机已发生 / 危机余震期”的高压样本记成了负样本；
- 因此模型会学到“高压 != 即将发生危机”，随后只能靠负 `alpha` 校准去补救；
- serving 层再怎么加阈值和融合门控，也只能局部止血，不能从根上修复。

所以这条主线下一步不应继续只做 runtime 微调，而应直接进入：

1. 给 formal dataset / training row 增加明确的 `regime` 标记（至少区分 `pre-crisis / in-crisis / post-crisis cooldown / normal`）；
2. 对 `forward_crisis` 主概率头重新定义负样本口径，至少不要再把危机已发生后的高压区间按普通负样本等权训练；
3. 重跑 `scensplit4` 或新的 scenario-aware split，再复核 `20d / 60d` 是否还需要负 `alpha` 才能工作；
4. 只有在这一步成立后，后续的 threshold / actionability fusion 微调才值得继续投入。

## 8. 准入清单

发布前至少勾选：

- [x] 数据集来自 `raw observations -> PIT features -> labels`
- [ ] `feature coverage matrix` 已落实到代码和 manifest
- [x] `scenario catalog` 已配置化
- [x] `best_effort point-in-time visibility` 已实现到 dataset builder
- [ ] `strict point-in-time visibility` 已具备足够官方时间戳覆盖
- [x] `formal dataset spec` 已生成真实数据集
- [ ] `candidate release` 的离线指标达标
- [ ] `rolling audit` 未明显恶化
- [ ] `rolling audit` 已不再依赖 persisted snapshot bridge
- [ ] 前端方法页能解释当前 release 的数据模式、PIT 模式和局限

## 9. 人工审批要求

晋升到 `active_default` 前，至少要有一次人工复核，确认：

1. 不是单次幸运命中；
2. 不是未来函数；
3. 不是因为数据缺失导致误报被隐藏；
4. 不是把启发式旧逻辑重新包装成“模型升级”。

## 10. 回滚条件

触发以下任一情况，允许或要求回滚：

- 新 release 上线后明显增加误报；
- 关键数据源断流，达不到该 release 声明的 PIT / coverage 标准；
- 线上解释与离线评估不一致；
- 数据或标签治理发现错误。

当前代码已经加入一层运行时护栏：

- `research release activate --reload-api`
- `research release publish --activate --reload-api`
- `research pipeline bootstrap-formal-release`
- `research release review --candidate-release-id ...`

在这三条链路下，worker 会在 reload 后自动对比：

- `timely_warning_rate`
- `rolling_audit.actionable_precision`
- `rolling_audit.longest_false_positive_episode_days`

如果新 release 明显劣化，会自动回滚到上一个 active release。

推荐评审顺序：

1. 先跑 `research release review --candidate-release-id ...`
2. 看 JSON / Markdown 报告是否通过护栏
3. 再决定是否允许进入人工上线复核
4. 最后才是 `activate` / `publish --activate`

## 11. 对开发顺序的影响

这份文档补完后，开发顺序应变成：

1. 先实现 `feature coverage matrix`
2. 已完成 `scenario catalog` 配置化，后续改动都应继续走配置
3. 再实现 `point-in-time visibility`
4. 再生成正式 dataset
5. 再把 `rolling audit` 从 persisted snapshot bridge 切到 raw PIT history
6. 最后才是“是否允许正式模型替代启发式”

### 7.7 2026-06-01 `regime-aware + actionability quality gate` 复核：动作头不是主因

在 `scensplit4` 主线上又做了一轮更激进的治理：

1. 给 training row 增加 `ProbabilityTrainingRegime`，并对 `forward_crisis` 主概率头按 `pre-warning / in-crisis / cooldown / normal` 重加权；
2. 给 `actionability` 头加入精度 / 预测量质量门，质量不过线就直接不打进 bundle；
3. API runtime 对 formal main 增加更保守的 posture floor，不再把训练出的极低分类阈值直接当线上动作阈值。

对应候选版：

- `us_formal_pit_scensplit4regime1_20260601T052013`
- `us_formal_pit_scensplit4regime4_20260601T055211`

这轮的关键结论是：

- `regime1` 虽然把 `timely_warning_rate` 拉到 `75.0%`，但 `actionable_precision` 只有 `10.1%`，`longest_false_positive_episode_days` 直接拉到 `288` 天；
- 给 `actionability` 头加独立质量门后，worker 已经会自动输出：
  - `precision below required floor`
  - `predicted positives exceed ceiling`
  - 并把动作头从 bundle 里剔除；
- 即便动作头被剔除，`regime4` 复核依然不通过：
  - 在旧 runtime 下：`timely_warning_rate 37.5% -> 75.0%`，但 `actionable_precision 29.6% -> 10.1%`
  - 在更保守 runtime floor 下：`timely_warning_rate 25.0% -> 50.0%`，但 `actionable_precision 21.1% -> 9.8%`
  - `longest_false_positive_episode_days` 仍然维持 `288`

因此当前可以明确排除一个误区：

- **不是独立动作头把 release 搞坏了；**
- 即便完全回退到 `probability-context fusion`，formal main 主概率头仍会在若干年份拉出超长 `prepare / hedge` 误报段；
- runtime floor 只能把 `timely_warning_rate` 压下来，但没有把长误报段真正消掉。

现阶段最重要的工程结论：

1. `actionability` 头可以保留为实验项，但默认必须走质量门，不得再无条件写入正式 bundle；
2. formal main 下一步不应继续主要投入在动作头阈值，而应回到主概率头本身：
   - 重新审视 `forward_crisis` 标签边界；
   - 增补更能区分“结构脆弱但未临近危机”和“真正进入离场窗口”的特征；
   - 把 `rolling audit` 的纯误报长段直接纳入训练 / 选阈 / 选模目标；
3. 在主概率头没有把 `288` 天这类长误报段打掉之前，formal main 仍然只能算研究候选，不具备替代当前 active release 的资格。

### 7.8 2026-06-01 runtime regime 审计补充：现在能证明 candidate 是“全 regime 都偏冷”

本轮继续把诊断链补到了两个层面：

1. formal dataset summary 新增 `regime mix`，直接统计 `train / calibration / evaluation` 在 `5d / 20d / 60d` 三个 horizon 下的 `normal / pre_warning_buffer / positive_window / in_crisis / post_crisis_cooldown` 分布；
2. release review 新增 `runtime regime probability` 表，直接看历史 assessment 在不同 regime 下的平均概率、最大概率和 runtime floor 命中次数。

先看正式训练集 `formal_v1_main_1990_daily:20260601Tscensplit4` 的 evaluation split：

- `5d`：`normal 2117`、`pre_warning_buffer 3`、`positive_window 5`、`in_crisis 116`、`post_crisis_cooldown 28`
- `20d`：`normal 2058`、`pre_warning_buffer 15`、`positive_window 20`、`in_crisis 116`、`post_crisis_cooldown 60`
- `60d`：`normal 2003`、`positive_window 60`、`in_crisis 116`、`post_crisis_cooldown 90`

这说明当前正式数据集不是“完全没有 regime 样本”；至少对 `20d / 60d` 来说，正例窗口、危机中、危机后余震都确实存在，只是占比很低。

再看候选版 `us_formal_pit_scensplit4regime4_20260601T055211` 的 release review：

- `5d`：
  - `normal avg=0.496%`
  - `positive_window avg=0.500%`
  - `in_crisis avg=0.501%`
  - `max=0.700%`
- `20d`：
  - `normal avg=2.496%`
  - `positive_window avg=2.495%`
  - `in_crisis avg=2.501%`
  - `max=2.700%`
- `60d`：
  - `normal avg=5.996%`
  - `positive_window avg=5.700%`
  - `in_crisis avg=5.591%`
  - `max=7.500%`
- 三个 horizon 的 runtime floor 命中次数都是 `0`

这个结果已经不是“阈值有点偏高”这么简单，而是：

- candidate 在 `normal / pre-warning / positive_window / in-crisis / cooldown` 各个 regime 下，概率几乎没有拉开；
- `5d / 20d` 基本完全压成常数；
- `60d` 虽然有轻微波动，但方向和幅度都不足以触发任何 runtime floor；
- 因此 candidate 不是“会提前预警但被 serving 压掉”，而是**主概率头本身就没有学出可用的 regime separation**。

同时，baseline `us_formal_transitional_20260531T094603` 的 fallback 审计也说明了另一个问题：

- baseline 的 `5d / 20d / 60d` 概率同样几乎是常数；
- 但它依然能在历史上给出大量 `prepare` posture；
- 这意味着当前 baseline 的动作效果主要来自 transitional serving 语义，而不是概率头本身具备了好的可分层能力。

因此下一步的工程重点要再收窄一层：

1. 不要继续主要投入在 runtime floor 微调；
2. runtime / release review 现在已经能输出 clause-level posture 审计，并能确认 baseline / candidate 当前并没有留下可用的非 `normal` 历史；
3. formal main 训练目标必须显式要求：
   - `positive_window` 相对 `normal` 有正向拉升；
   - `in_crisis / cooldown` 不要和真正的“危机前数周窗口”混成一个概率水平；
4. 在没有看到 `pre_warning / positive_window` 与 `normal` 拉开之前，任何新候选版都不应进入默认线上版本讨论。

### 7.9 2026-06-01 runtime sanity guard 补充：relative pass 不再等于可上线

在上一轮诊断里，`us_formal_pit_scensplit4regime4_20260601T055211` 虽然已经被证明：

- `13331` 个历史点全部是 `normal`
- `prepare / hedge / defend` 三条 runtime probability floor 命中数全部为 `0`
- `5d=calibration_crushed_early_warning`
- `20d=cold_across_all_regimes`
- `60d=mixed_or_unclear`

但旧版 `release review` 仍然会给出 `guard_passed=true`，原因只是：

- baseline `us_formal_transitional_20260531T094603` 也同样偏冷；
- 旧护栏只看“candidate 是否比 baseline 更差”，没有拦截“baseline 和 candidate 同样都不具备可执行预警能力”的情况。

因此本轮又补了一条 `runtime sanity guard`：

- 如果 candidate 全历史始终 `normal`
- 且三个 runtime floor 命中数全部为 `0`
- 且 regime separation 里没有任何可用的 early-warning 诊断

那么 candidate 不能再因为“和 baseline 一样差”而被误判通过。

按这个新护栏重跑 `us_formal_transitional_20260531T094603 vs us_formal_pit_scensplit4regime4_20260601T055211` 后：

- `overall_guard_passed=false`
- `runtime_sanity_regressions` 会明确写出：
  - candidate 全历史 all-normal / zero-floor-hit / no usable early-warning separation
  - baseline 也同样 all-normal / zero-floor-hit，因此 relative guardrails 不是充分晋升依据

这一步的意义不是“把候选版打回去”这么简单，而是：

- 当前 active baseline 也不能再被当成一个可靠的正式概率基线；
- 后续研究必须以“绝对可执行性”而不是“相对不更差”作为 release 晋升门槛；
- 在主概率头没有重新学出可区分的 pre-warning / positive-window 信号之前，不应再讨论默认上线。

### 7.10 2026-06-02 formal main 重建候选 `us_formal_main_20260601T163415`

本轮已经基于最新主数据集重新跑了一次 formal main：

- dataset: `formal_v1_main_1990_daily:20260601T163337`
- release: `us_formal_main_20260601T163415`
- baseline: `us_formal_transitional_20260531T094603`
- review: `reports/release-review/2026-06-01-us_formal_transitional_20260531T094603-vs-us_formal_main_20260601T163415-release-review.md`

先看 dataset summary：

- `evaluation` 已经不再只有单场景，`2020 / 2023` 都进入了评估段；
- 但 `train` 仍然主要由 `2008` 主导；
- `protected` 行仍然是 `0`，说明 formal main 还没有把 protected stress 当成正式上下文样本。

再看训练输出：

- actionability 头直接被禁用；
- `usable_early_warning_horizons=0`
- `5d=cold_across_all_regimes`
- `20d=weak_regime_separation`
- `60d=cooldown_bleed`

release review 的结论也一致：

- `guard_passed=false`
- `probability head has zero usable early-warning horizons in bundle evaluation`
- `60d regime diagnosis is cooldown_bleed in bundle evaluation`
- `candidate us_formal_main_20260601T163415 has zero usable early-warning horizons in runtime regime audit`

因此这次候选版说明的不是“formal main 完全没法跑”，而是：

1. 现有主线代码链路已经可以完整重建、训练、发布候选、跑 strict-rebuild review；
2. 但 formal main 的样本治理仍不足以支撑 `20d/60d` 形成稳定的 pre-warning separation；
3. 下一轮最该做的是 formal main 与 `candidate_optional / protected stress` 的关系设计，而不是继续换一版阈值或校准。

### 7.11 2026-06-02 formal main protected context 第一阶段已接入，但仍未过关

本轮继续推进了 formal main 主线，不再停留在设计文档：

- formal main dataset 已接入 `protected_stress_windows_v1`
- 主数据集 `formal_v1_main_1990_daily:20260601T170716` / `20260601T172759` 的 `protected` 行已不再为 `0`
- `2000 / 2011 / 2022` 已进入 formal main 的 context / split / actionability 语义

但最新候选版：

- `us_formal_main_20260601T173113`

在 strict-rebuild release review 下仍然失败：

- `guard_passed=false`
- `probability head has zero usable early-warning horizons in bundle evaluation`
- `20d regime diagnosis is cold_across_all_regimes in bundle evaluation`
- `60d regime diagnosis is cooldown_bleed in bundle evaluation`

这次复核的意义是把一个关键不确定性打掉：

- 之前可以怀疑“是不是因为 formal main 完全没吃到 protected context”
- 现在这个疑点已经排除

也就是说，主线当前剩余问题已经更收敛：

1. 不是扩展历史样本没接进来；
2. 也不是 release review 没走 strict rebuild；
3. 而是当前 `ForwardCrisis` 目标函数 + 线性头，仍不足以把 `20d/60d` 从偏冷状态拉出来。

因此下一轮优先级必须继续下沉到训练目标本身：

1. 更强的 protected-context / pre-warning separation
2. 如仍失败，再评估 horizon-specific feature 选择或更强目标函数
3. 不再继续主要投入在 serving 阈值或单纯 sample-weight 微调

### 7.12 2026-06-02 runtime 误报压缩后，主瓶颈切到“绝对提前量”

在后续一轮 runtime 收紧里，formal main 候选版 `us_formal_main_20260601T184003` 的状态已经明显变化：

- `20d / 60d` bundle evaluation 仍保持 `usable_early_warning_separation`
- runtime `longest_false_positive_episode_days`: `38 -> 5`
- runtime `actionable_precision`: `37.6% -> 66.7%`

说明两个结论已经成立：

1. 当前 candidate 不再是“长误报段失控”的状态；
2. runtime posture 收敛已经能把 `2022-11 ~ 2023-01` 这类高压未爆发阶段，与真正纯噪声更好地区分开。

但新的瓶颈也更明确了：

- `timely_warning_rate` 同时从 `30.0%` 回落到 `10.0%`
- 当前场景级 backtests 里，真正形成动作级提前量的仍只有 `2023 美国区域银行危机`；`1990-1993 / 2000 / 2008 / 2011 / 2020 / 2022` 这些场景仍是 missed

也就是说，这一轮把“误报过宽”压住了，但也把“可执行提前离场”压得偏保守。

因此当前最准确的项目判断是：

- 这版 candidate 已经更接近 `active_experimental` 的研究基线；
- 但仍不满足 `active_default`，因为绝对提前量还不够稳，不能支持“危机前数周就能从容处置仓位”的目标。

### 7.13 2026-06-02 `main + ext_stress + ext_acute` 组合训练能力已打通，但仍未解决 runtime 提前量

本轮又推进了一步工程能力，而不是只停留在分析：

- `research pipeline train-probability` / `bootstrap-formal-release`
- 已支持 `--aux-dataset-key`
- 可以把主数据集与扩展压力/急性样本一起送进同一轮 formal 训练

首个组合候选版：

- `us_formal_main_extmix_20260601T215225`

其 bundle evaluation 有两个好消息：

- `5d / 20d / 60d` 都出现了 `usable_early_warning_separation`
- 说明 `1987 / 1998 / 2011 / 2022` 这类扩展样本至少已经进入了训练目标视野，不再只是“有文档但喂不到模型里”

但 strict rebuild runtime review 的结论仍然偏保守：

- `timely_warning_rate = 10.0%`
- `actionable_precision = 70.0%`（随后在放宽 strong prepare 审计口径后回落到 `63.2%`）
- `longest_false_positive_episode_days = 5`（随后在放宽 strong prepare 审计口径后被拉到 `30`）

这一步说明的不是“组合训练没意义”，而是：

1. 数据集组合输入能力已经不再是瓶颈；
2. 但把扩展样本接进训练，并不等于模型真的学会了这些场景；
3. 下一轮必须直接处理“扩展场景在目标函数与样本权重里为什么仍然学不进去”，而不是继续停留在数据 plumbing 或审计口径层面。

### 7.14 2026-06-02 `scenario_training_role` 已正式下沉到训练行与权重，但尚未完成复核

本轮又补上了一块之前缺失的“工程语义闭环”：

- formal dataset row / SQLite / dataset CSV 已新增 `scenario_training_role`
- `load_formal_training_dataset(...)` 对旧 row 也会从 `scenario catalog` 回填 role，避免历史 dataset 因缺字段而白跑
- `ForwardCrisis` 概率头的正例权重，已开始同时看：
  - `scenario_training_role`
  - `scenario_family`
  - `default_horizon_roles`

这一步的意义很直接：

1. 之前 `ext_stress / ext_acute` 能接进同一轮训练，但权重逻辑并不知道哪些是 `mandatory`、哪些只是 `candidate_optional / extension_only`
2. 现在模型至少终于拿到了“这些样本在训练里应该多重看待”的显式信号
3. 但这仍只是训练前提，不等于 runtime 已经恢复可执行提前量

因此当前状态应理解为：

- “扩展场景角色元数据缺失” 这个瓶颈已经解除；
- 下一步必须立刻重训并重跑 strict review；
- 如果 `extmix2` 仍然只在 bundle evaluation 有分离、但 runtime 依旧只会报 `2023`，那就说明剩余瓶颈已经进一步收敛到目标函数本身，而不再是样本角色治理。

### 7.15 2026-06-02 `extmix2 / extmix3 / extmix4` 连续复核后，可以确认“权重微调”已接近收益上限

本轮在同一组 `main + ext_stress + ext_acute` 数据上，连续做了三次更深入的训练侧修正：

1. `extmix2`
   - 引入 `scenario_training_role`
   - 让 `ForwardCrisis` 正例权重同时看 `training_role + family + default_horizon_roles`
2. `extmix3`
   - 提高 `20d/60d pre_warning_buffer / positive_window` soft label
   - 降低 `20d/60d cooldown` soft label
   - 强化 `positive_window > normal / cooldown` 的 pairwise margin
3. `extmix4`
   - 继续提高 `20d/60d normal / cooldown` 负样本惩罚
   - 尤其针对 runtime 里“normal 太高、cooldown 压不下去”的形态

结果很一致：

| Candidate | timely_warning_rate | actionable_precision | longest_false_positive_episode_days | runtime diagnosis |
| --- | --- | --- | --- | --- |
| `extmix2` | `10.0%` | `60.8%` | `30` | `5d weak / 20d cold / 60d weak` |
| `extmix3` | `10.0%` | `62.5%` | `30` | `5d weak / 20d weak / 60d weak` |
| `extmix4` | `10.0%` | `62.5%` | `30` | 与 `extmix3` 基本相同 |

这说明：

1. 当前代码里“让样本角色更明确、把 pairwise 和 soft-label 再拧一圈”已经不是主要阻塞；
2. 模型可以在 bundle evaluation 学出 separation，但这层 separation 无法稳定穿透到 runtime；
3. 下一轮如果还只是继续调 sample-weight / soft-label / pairwise margin，大概率只会重复得到同类结果。

因此从现在开始，下一步应升级为：

- 重新设计目标函数或模型形态；
- 而不是继续把当前线性头当作主要突破口。

更直白地说：

- 当前剩余瓶颈已经不是“参数还没调够”；
- 而是“这套模型形态对你要的那类提前离场能力，表达力可能不够”。

### 7.16 2026-06-02 下一轮主线正式切到 `interaction_tail_v1`

基于 `extmix2 / extmix3 / extmix4` 的连续复核结果，当前已经可以做一个更强的工程决策：

1. 下一轮主线不再继续主要投入在 `sample-weight / soft-label / pairwise margin` 微调；
2. 第一优先级切到可解释的非线性基线：`interaction_tail_v1`；
3. 只有当 `interaction_tail_v1` 明确失败后，才进入 `family_conditional_v1`。

这样切换的原因不是“想换个更复杂的模型试试”，而是：

- 现有线性头已经能在 bundle evaluation 上学出一些 separation；
- 但这种 separation 无法稳定穿透到 runtime；
- 说明主瓶颈更像是模型表达力，而不是再多拧一圈权重。

下一轮方案已单独整理到：

- `docs/analytics/formal-nextgen-model-design.md`

这份设计文档的目标很明确：

1. 先在不破坏现有 bundle / release review / serving 结构的前提下，引入 `interaction + tail` 特征；
2. 用同一套 strict rebuild runtime review 判断表达力增强后，是否终于恢复“危机前数周的可执行提前量”；
3. 如果仍失败，再进入 family-conditional 的第二阶段设计，而不是继续在同一条低收益路线里循环。

### 7.17 2026-06-02 首版 `interaction_tail_v1` 已证明“模型形态升级有效”，但还没到可上线

按新设计跑出的第一版候选：

- `us_formal_interaction_tail_extmix1_20260602T015347`

训练输入：

- `formal_v1_main_1990_daily:20260601T172759`
- `formal_v1_ext_stress_1990_daily:20260601T162655`
- `formal_v1_ext_acute_pre1990:20260601T163102`

先看 bundle evaluation：

- `5d / 20d / 60d` 三个 horizon 全部进入 `usable_early_warning_separation`
- 这说明 `interaction + tail` 这条线不是空转，模型表达力确实增强了

再看 strict rebuild runtime review：

- `timely_warning_rate: 0.0% -> 10.0%`
- `actionable_precision: 0.0% -> 63.8%`
- `longest_false_positive_episode_days: 0 -> 21`

runtime regime 诊断也已经和前一轮明显不同：

- `5d = weak_regime_separation`
- `20d = usable_early_warning_separation`
- `60d = usable_early_warning_separation`

这意味着一个非常关键的新结论：

1. `interaction_tail_v1` 已经把问题从“全 regime 都偏冷”推进到了“中长 horizon 已有可用 separation”；
2. 当前剩余瓶颈不再是“模型完全学不会”，而是：
   - `5d` 仍然存在明显的 normal leakage
   - `60d` 正常期与 cooldown 仍偏宽，导致 runtime `prepare` 太常触发
3. 因此这次 FAIL 不是在否定 `interaction_tail_v1`，而是在告诉我们：
   - 下一轮应继续沿着 `interaction_tail_v1` 压缩 `5d normal` 与 `60d cooldown/normal`；
   - 还不到直接跳去 `family_conditional_v1` 的时候

从 review 细项看，当前 candidate 的主要问题是：

- `prepare_p60d` runtime floor 被 bundle threshold 拉到 `68.9%`
- 历史里 `p_60d>=prepare` 仍命中 `2406` 次
- `5d normal` 平均概率 `3.9%`，高于 `5d positive_window` 的 `3.8%`
- 因此虽然 `20d/60d` 已经开始具备“数周级”的可用分离，但短窗与 months bucket 还没有收口到可执行水平

所以这轮最重要的项目判断是：

- **`interaction_tail_v1` 是正确方向，应继续推进**
- **但它当前仍不能替代默认线上版本**
- **下一轮应优先修正 `5d normal leakage` 与 `60d/20d runtime overfire`，而不是回头继续做纯权重微调**

### 7.18 2026-06-02 `interaction_tail_extmix2`：离线继续变好，但 runtime 仍卡在 `timely_warning_rate=10%`

在首版 `interaction_tail_extmix1` 的基础上，又补了一轮更强的 regime pairwise 约束，产出：

- `us_formal_interaction_tail_extmix2_20260602T022315`

新的 bundle evaluation 继续改善：

- `5d / 20d / 60d` 仍全部是 `usable_early_warning_separation`
- 且 `5d` 的离线 `positive_window lift` 明显抬高

但 strict rebuild runtime review 的结果只出现了“精度改善、误报缩短”，没有恢复绝对提前量：

- `timely_warning_rate: 10.0% -> 10.0%`
- `actionable_precision: 63.8% -> 65.8%`
- `longest_false_positive_episode_days: 21 -> 19`

runtime 诊断说明两件事：

1. `5d` 依然是 `weak_regime_separation`
2. `20d / 60d` 虽然仍是 `usable_early_warning_separation`，但 `prepare` 触发仍偏宽

而且这次 review 还暴露了一个更具体的新瓶颈：

- `prepare_carry_structural` 占 `prepare` posture 的 `80.3%`
- `prepare_p60d_structural` 占 `57.2%`
- `prepare_structural_downgrade` 占 `46.5%`

这意味着当前剩余问题已经不再只是概率头本身，还包括：

1. runtime posture 融合里，`carry + structural + downgrade` 这组 prepare clause 偏宽；
2. `5d` 的 label / calibration / runtime 口径仍没把 `normal` 压到足够低；
3. 因此继续单纯强化离线 pairwise，并不能自动换来更高的 `timely_warning_rate`。

所以从 `extmix2` 开始，下一轮任务需要明确拆成两条并行主线：

- 继续优化 `interaction_tail_v1` 本身的 `5d` 与 `60d cooldown/normal` 形态；
- 同时审计 runtime `prepare` clause，避免结构性高压阶段被过宽地推入 months posture。

### 7.19 2026-06-02 收紧 runtime `prepare` guard 后，`extmix2` 已接近新的 `active_experimental` 基线

针对上一轮 `extmix2` review 暴露出来的 runtime 问题，本轮先没有继续训练新模型，而是先收紧了 serving 侧的 `prepare` / `months` 判定：

- `prepare` 不再允许“单个 structural / external / carry 信号”直接升级；
- `prepare_external_structural` 现在要求 `p_20d` 概率伴随确认；
- `prepare_carry_structural` 现在要求 `stressed_carry + non-carry confirmation`；
- `prepare_actionability` 现在要求 `p_60d` 同行确认；
- `time_to_risk_bucket=months` 也同步切到“概率 + 多重上下文确认”。

在重启 API 并按新 runtime policy 重新跑 `strict rebuild review` 后，`us_formal_interaction_tail_extmix2_20260602T022315` 的结果发生了明显变化：

- `timely_warning_rate` 仍是 `10.0%`
- `actionable_precision` 从 `65.8%` 回落到 `51.9%`
- `longest_false_positive_episode_days` 从 `19` 进一步压到 `5`

更关键的是 posture / bucket 分布出现了真正的收口：

- `prepare` posture 从 `477` 个历史点降到 `30`
- `months` bucket 从 `1053` 个历史点降到 `56`
- `prepare_carry_structural` 已经不再成为主要触发项
- 剩余 `prepare` 主要来自：
  - `prepare_p60d_structural`（`23`）
  - `prepare_structural_downgrade`（`9`）

对应的 runtime 诊断也变了：

- `5d=usable_early_warning_separation`
- `20d=usable_early_warning_separation`
- `60d=separated_but_below_runtime_floor`

这说明本轮修正已经把问题进一步收敛清楚：

1. 之前 `extmix2` 的大块 months/prepare 误报，确实有一部分是 runtime clause 过宽；
2. 收紧 guard 后，`carry` 主导的伪 `prepare` 基本被切掉了；
3. 但绝对提前量仍停在 `10.0%`，说明剩余瓶颈已经更集中地落在：
   - `5d` 仍不够稳定；
   - `60d pre_warning_buffer` 虽有 separation，但还穿不过 `prepare_p60d=73.2%` 的 runtime floor。

因此当前更准确的工程判断是：

- 这版 `extmix2` **已经可以视为新的 `active_experimental` 研究基线**；
- 但它**仍不能直接晋升为默认正式版**；
- 下一轮重点不该回头放宽 runtime guard，而应转向：
  - 压缩 `5d normal leakage`
  - 提高 `60d pre_warning_buffer` 对 runtime floor 的穿透能力
  - 复核 `prepare_p60d` 阈值选择是否过高，或训练目标是否没有显式优化“越过 floor”的能力

### 7.20 2026-06-02 训练侧 threshold repair 已落地，但 `extmix7` 说明当前瓶颈不在“简单阈值打分规则”

本轮又连续做了两步训练侧修正：

1. 把 `select_probability_decision_threshold` 改成 horizon-aware：
   - `5d` 继续偏 precision-first
   - `20d/60d` 提高 recall / F-beta 权重
2. 又补了一层 `regime-aware threshold repair`：
   - 如果 `20d/60d` 已经学出 early-warning separation
   - 但 base threshold 连 calibration 里的 early-warning regime 都完全打不到
   - 就尝试向下修正 threshold，并增加 `early-warning cap` 保护

对应训练候选：

- `us_formal_interaction_tail_extmix6_20260602T052712`
- `us_formal_interaction_tail_extmix7_20260602T053257`

但结果出现了一个很关键的工程事实：

- 两版候选的 bundle threshold 仍然是：
  - `5d = 0.03`
  - `20d = 0.522`
  - `60d = 0.732`
- 离线 `regime_eval` 也没有变化，仍然是 `5d/20d/60d usable_early_warning_separation`

这说明：

1. 当前问题已经不是“threshold 选择函数纯粹按 precision 排序”这么简单；
2. 即便补了 horizon-aware / regime-aware 修正，当前 calibration split 里仍没有提供足够强的 `60d pre_warning_buffer` 穿越证据；
3. 所以下一步不该继续盲目堆阈值小修，而应优先确认：
   - calibration split 是否真的包含足够可用的 `pre_warning_buffer`
   - `60d` 的 positive / protected / buffer 角色是否在 split 上被稀释
   - 是否需要把 threshold selection 显式接到 `calibration_rows` 的 regime diagnostics 导出，而不是只在训练内部静默处理

这句人话解释是：

- 训练侧已经尽力“想把 threshold 往下拉”；
- 但当前这批 calibration 样本本身没有给出足够强的下调理由；
- 所以瓶颈更像是 `split / label / calibration evidence`，而不是下一轮再改一版阈值打分公式。

## 12. 结论

从这一步开始，项目里出现“formal bundle”不再自动等于“正式模型”。

只有满足本文件门槛，才配叫：

```text
active_default formal probability model
```

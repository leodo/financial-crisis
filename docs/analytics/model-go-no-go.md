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
- 对 formal main release，如果历史缓存版本失配，会基于原始观测全量重建该 release 的历史评估轨迹；
- 再重新跑 `release review`。

结果仍然没有改变：

- `us_formal_pit_20260531T160129` 仍然是 `37.5% -> 12.5% / 29.6% -> 20.6% / 9 -> 18`
- `us_formal_pit_weighted_20260531T171025` 仍然是 `37.5% -> 12.5% / 29.6% -> 20.6% / 9 -> 18`

这意味着：

- “旧缓存污染了 formal 候选版复核结果” 现在已经不是主要解释；
- formal PIT 主线当前确实没有学出足够稳定的动作级提前量。

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

## 12. 结论

从这一步开始，项目里出现“formal bundle”不再自动等于“正式模型”。

只有满足本文件门槛，才配叫：

```text
active_default formal probability model
```

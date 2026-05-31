# 正式模型准入与 Go/No-Go

状态：`Draft`

最后更新：2026-05-31

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

## 8. 准入清单

发布前至少勾选：

- [ ] 数据集来自 `raw observations -> PIT features -> labels`
- [ ] `feature coverage matrix` 已落实到代码和 manifest
- [x] `scenario catalog` 已配置化
- [x] `best_effort point-in-time visibility` 已实现到 dataset builder
- [ ] `strict point-in-time visibility` 已具备足够官方时间戳覆盖
- [ ] `formal dataset spec` 已生成真实数据集
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

## 12. 结论

从这一步开始，项目里出现“formal bundle”不再自动等于“正式模型”。

只有满足本文件门槛，才配叫：

```text
active_default formal probability model
```

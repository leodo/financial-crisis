# 持仓动作手册设计

状态：`Draft`

最后更新：2026-05-31

## 1. 目标

把系统输出的 `probability + time_to_risk + posture + data_trust` 翻译成可执行的持仓动作手册，重点回答以下问题：

1. 当前应不应该明显减仓。
2. 是降到“小仓位”，还是进入强防守。
3. 是否需要保护性认沽、指数对冲或去杠杆。
4. 风险缓和后如何分批再入场。

本设计服务于“美国主线风险资产持有人”的系统级风险处置，不是个性化投资建议。

## 2. 非目标

第一阶段不覆盖：

- 个股选择或行业轮动 alpha。
- 税务、账户结构和交易成本最优化。
- 高频自动交易。
- 依赖付费 Greeks 或波动率曲面数据的复杂期权策略。

## 3. 设计原则

- 不允许把单个分数直接翻译成“清仓”。
- 不允许把未校准概率直接当成最终仓位指令。
- 动作必须分阶段执行，避免一次性大幅误杀。
- 先保护流动性和杠杆，再讨论抄底和再加仓。
- 同一 posture 下，`time_to_risk_bucket` 决定动作速度。
- 系统只能给“风险预算”和“执行顺序”，不能冒充自动交易系统。

## 4. 适用范围

第一阶段默认覆盖：

- 美国股票 / ETF / 指数基金
- 美股指数期货或 ETF 对冲
- 保护性认沽
- 现金 / T-Bill / 短债替代

默认不直接给出：

- 单名信用债
- 私募和不动产
- 高换手多空书

这些资产先通过“流动性折扣”和“减仓优先级”间接处理。

## 5. 输入

```text
posture
time_to_risk_bucket
p_5d
p_20d
p_60d
structural_score
trigger_score
external_shock_score
conviction_score
data_trust
historical_analogs
event_confirmation_state
user_risk_profile
user_preference_overrides
```

## 6. 输出对象

```text
action_playbook_version
action_level
execution_urgency
target_risk_asset_exposure_pct
target_cash_pct
hedge_ratio_pct
option_overlay_pct
leverage_cap_pct
illiquid_asset_cap_pct
primary_actions[]
secondary_actions[]
forbidden_actions[]
upgrade_conditions[]
downgrade_conditions[]
reentry_conditions[]
confidence_gate
```

字段语义：

- `target_risk_asset_exposure_pct`：股票、信用和商品等风险资产总敞口上限。
- `target_cash_pct`：现金、货币基金和短票据等高流动性仓位目标。
- `hedge_ratio_pct`：需要被指数对冲或保护性工具覆盖的风险资产比例。
- `option_overlay_pct`：使用保护性期权覆盖的风险资产比例，不是期权权利金占总资产比例。

## 7. 时间桶到执行速度

### 7.1 `normal`

- 不触发系统性去风险动作。
- 允许按常规节奏维护组合。

### 7.2 `months`

- 执行窗口：`3-10` 个交易日。
- 先做低成本调整：降高 beta、降杠杆、补流动性、准备对冲工具。

### 7.3 `weeks`

- 执行窗口：`1-5` 个交易日。
- 开始明确提高现金和保护比例，不能只停留在“观察”。

### 7.4 `now`

- 执行窗口：当日到 `2` 个交易日。
- 优先保护流动性和去杠杆，不等待更多主观确认。

## 8. 默认动作矩阵

| posture | 风险资产上限 | 现金目标 | 对冲覆盖 | 期权覆盖 | 杠杆上限 | 非流动性资产上限 | 含义 |
|---|---:|---:|---:|---:|---:|---:|---|
| `normal` | 65% - 85% | 5% - 15% | 0% - 5% | 0% - 3% | 100% | 25% | 风险处于常态，不主动大幅防守 |
| `prepare` | 50% - 70% | 15% - 25% | 5% - 15% | 0% - 5% | 75% | 20% | 脆弱性升高，开始做准备性收缩 |
| `hedge` | 30% - 50% | 25% - 40% | 20% - 35% | 5% - 12% | 50% | 15% | 风险进入几周尺度，不能只口头防守 |
| `defend` | 10% - 25% | 45% - 65% | 35% - 60% | 10% - 20% | 0% | 10% | 短期风险窗口已打开，资本保全优先 |

附加规则：

- `defend` 不是默认“清仓”。系统只把组合压到小仓位和高流动性，不直接给出“一键全清”。
- 仅当同时满足 `defend + now + 高可信度 + 事件确认` 时，才允许进入 `capital_preservation` 叠加模式。
- `capital_preservation` 叠加模式下，风险资产可压到 `0% - 15%`，但仍要求保留最小可管理仓位或最小对冲腿，避免完全失去再进入场的纪律。

## 9. 分阶段执行顺序

### 9.1 `prepare`

优先动作：

1. 停止新增杠杆和高 beta 曝露。
2. 先减流动性差、相关性高、波动率高的仓位。
3. 把组合现金补到最低安全线。
4. 确认对冲工具可用性：指数 ETF、股指期货、保护性认沽。

禁止动作：

- 在数据可信度不足时主动扩大风险资产仓位。
- 因为单日反弹取消全部预案。

### 9.2 `hedge`

优先动作：

1. 把高 beta、周期和杠杆资产降到目标区间。
2. 建立第一层系统性保护：指数空头、保护性认沽或波动率对冲。
3. 提高现金和短久期资产占比。
4. 降低集中持仓和尾部相关性。

禁止动作：

- 在没有 re-entry 规则前逆势放大仓位。
- 把所有保护都压在单一标的或单一期权到期日上。

### 9.3 `defend`

优先动作：

1. 先去杠杆，再降高风险仓位。
2. 先保留高流动性和核心防守仓位，后处理低流动性仓位。
3. 将保护性对冲提升到组合级，而不是单仓位级。
4. 若事件确认持续恶化，优先保留现金和可随时卖出的工具。

禁止动作：

- 仅因单日大反弹就撤掉全部防守。
- 在流动性受压阶段新增复杂、滑点高的保护结构。

## 10. 风险偏好叠加

沿用现有 `conservative / neutral / aggressive` 三档，但只作为上层覆盖，不改变主 posture。

### 10.1 `conservative`

- 风险资产上限额外下调 `5% - 10%`
- 现金底线额外提高 `5% - 10%`
- 对冲比例额外提高 `5%`

### 10.2 `neutral`

- 使用默认动作矩阵

### 10.3 `aggressive`

- 仅在 `normal / prepare` 阶段允许额外提高 `5%` 风险资产上限
- `hedge / defend` 阶段不允许因为激进偏好而放松防守纪律

## 11. 升级与降级规则

### 11.1 升级到 `prepare`

满足以下任一即可：

- `p_60d` 明显抬升且 structural score 恶化
- JPY carry、信用、金融条件同时出现结构压力
- 当前与历史压力前期相似度明显升高

### 11.2 升级到 `hedge`

需要满足：

- `p_20d` 抬升，或
- `prepare` 基础上 trigger score、外部冲击和 breadth 同步恶化

### 11.3 升级到 `defend`

需要同时满足：

- `p_5d` 或 `time_to_risk_bucket=now` 给出急性风险信号
- 数据可信度不低于 `B`
- 至少存在 trigger、external、event 中的两类确认

### 11.4 从 `defend` 降回 `hedge`

至少满足：

- `p_5d < 0.20` 连续 `3` 个交易日
- trigger score 回落
- 没有新的高等级事件确认

### 11.5 从 `hedge` 降回 `prepare`

至少满足：

- `p_20d < 0.25` 连续 `5` 个交易日
- 外部冲击与信用压力同步缓和

### 11.6 从 `prepare` 降回 `normal`

至少满足：

- `p_60d < 0.25`
- structural score 不再继续抬升
- 关键指标新鲜度和覆盖率恢复正常

## 12. 再入场规则

再入场必须分批，不允许一次性恢复到 `normal` 仓位。

默认节奏：

1. 第一次恢复：先恢复目标风险资产缺口的 `1/3`
2. 第二次恢复：再观察 `3-5` 个交易日
3. 第三次恢复：只有在 trigger 和 external 同时改善时才补齐

再入场前必须检查：

- 是否只是短期超跌反弹
- 信用、流动性和 USDJPY / JPY carry 是否仍紧张
- 当前点位与历史类比是否仍处于危机前窗口

## 13. 受保护压力窗口的处理

若系统命中已定义的 `protected_stress_window`：

- 允许 `prepare / hedge` 较早出现
- 允许较长时间维持中度防守
- 但不应自动升级为“纯危机已到来”

这用于区分：

- 可以接受的系统性压力防守
- 真正需要强资本保全的危机窗口

## 14. API 与 UI 补充需求

`/api/assessment` 在现有 `position_guidance` 之外，后续应增加：

```text
action_playbook_version
execution_urgency
forbidden_actions
reentry_conditions
confidence_gate
capital_preservation_overlay_enabled
```

前端应明确展示：

- 当前 posture 对应的动作区间，而不是单个绝对数字
- 系统为何没有建议“清仓”
- 从当前 posture 升级或降级需要什么条件
- 再入场不是一次完成，而是分批恢复

## 15. 需要落库的对象

```text
analytics_action_playbooks
analytics_action_playbook_versions
analytics_action_decisions
user_action_preferences
```

## 16. 实现顺序

1. 固化字段语义，尤其是 `hedge_ratio_pct` 和 `option_overlay_pct`
2. 先实现默认动作矩阵
3. 再实现 upgrade / downgrade / re-entry 条件
4. 将动作手册接入回测，统计提前量、误报和反复横跳
5. 最后再做用户模板和资产类别差异化

## 17. 风险

- 概率未校准时，动作手册仍只能作为过渡层
- 用户真实持仓结构差异很大，默认预算只能给系统级参考
- 免费数据在信用和事件层仍可能延迟，影响 `defend` 触发速度
- 若没有再入场纪律，系统容易在压力缓和后长期过度保守

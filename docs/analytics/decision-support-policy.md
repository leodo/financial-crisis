# 决策支持策略设计

状态：`Draft`

最后更新：2026-05-30

## 1. 目标

把系统输出的概率、风险强度和数据可信度，转换成用户可理解的风险处理 posture，服务于仓位管理和保护性对冲判断。

这不是个性化投资建议文档，而是系统层面的动作映射规范。

## 2. 设计原则

- 不允许只看单个数字就触发全仓动作。
- 不允许把低可信度高概率直接当成强交易信号。
- 优先给 posture，再给具体触发原因。
- 用户必须可以自定义阈值和动作映射。

## 3. 输入

```text
p_5d
p_20d
p_60d
structural_score
trigger_score
external_shock_score
conviction_score
data_quality_grade
historical_analogs
```

字段边界：

- `conviction_score` 是历史兼容字段，当前等价于 `action_evidence.score`，含义是“动作升级证据是否足够”，不是“系统对当前结论有多大把握”。
- 结论可靠性应由 `data_trust.coverage_score`、`data_trust.quality_grade`、`method.probability_mode`、`method.release_status`、关键指标最新日期和 stale warning 共同解释。
- 低风险、数据覆盖良好、但风险广度和结构/触发共振没有打开时，`conviction_score` 可能长期在 0.50 左右；这表示“数据可用但不足以升级仓位动作”，不表示系统只有 50% 把握。

## 4. 输出 posture

### 4.1 `normal`

含义：

- 风险处于常态
- 不支持主动大幅防守

### 4.2 `prepare`

含义：

- 中期脆弱性已明显上升
- 近期可能尚未触发

典型动作表达：

- 检查流动性
- 降低高 beta 暴露
- 提前规划对冲工具

### 4.3 `hedge`

含义：

- 几周尺度风险已抬升
- 需要考虑保护性对冲而不是等事件落地

典型动作表达：

- 提高现金或短久期比例
- 考虑保护性认沽或波动率对冲

### 4.4 `defend`

含义：

- 短期风险窗口已明显打开
- 系统倾向资本保全和流动性优先

典型动作表达：

- 收缩高风险敞口
- 优先流动性
- 降低杠杆和尾部暴露

## 5. 默认映射逻辑

### 5.1 `normal`

```text
p_5d < 0.10
and p_20d < 0.20
and p_60d < 0.35
```

### 5.2 `prepare`

```text
p_60d >= 0.35
or structural_score elevated
```

### 5.3 `hedge`

```text
p_20d >= 0.35
or (p_60d high and trigger_score rising)
```

### 5.4 `defend`

```text
p_5d >= 0.30
or (p_20d very high and trigger_score high and conviction_score high)
```

以上仅为第一阶段默认规则，最终阈值必须经回测校准。

注意：这里的 `conviction_score high` 是动作升级门槛，用来避免在风险证据不宽、结构/触发未共振时过早进入 `defend`。它不是最终 posture 的置信概率。

## 6. 限制条件

以下情况禁止升级到 `defend`：

- 数据质量低于 `C`
- 只有单一噪声指标异常
- 外部冲击高但美国内部压力低
- 模型和规则结论明显冲突且没有事件确认

## 7. UI 表达

系统不应显示“建议清仓”这种绝对化文案，而应显示：

- 当前 posture
- posture 原因
- 升级到更高 posture 的条件
- 降级回较低 posture 的条件

## 8. 用户自定义

后续需要支持：

- 阈值模板
- 激进/中性/保守风格
- 资产类别差异化映射
- 是否启用期权保护建议

## 9. 需要落库的对象

```text
analytics_decision_postures
analytics_posture_policy_versions
user_posture_preferences
```

## 10. 开发提示

- 第一阶段先只做系统默认 posture。
- 第二阶段再做用户个性化。
- posture 计算要有完整解释链，避免黑盒建议。

# 危机概率引擎设计

状态：`Draft`

最后更新：2026-05-30

## 1. 目标

在保留可解释评分层的前提下，建立一个输出 `5d/20d/60d` 危机概率的模型体系。

## 2. 设计原则

- 解释优先，概率其次，但概率必须真实可校准。
- 规则分层不能直接冒充概率。
- 每个 horizon 独立建模，但共享特征和解释框架。
- 模型必须支持历史回放和版本冻结。

## 3. 引擎分层

```text
Layer 0  原始数据和元数据
Layer 1  可解释指标与风险强度
Layer 2  特征工程
Layer 3  Horizon probability models
Layer 4  Probability calibration
Layer 5  Decision posture mapping
```

## 4. Layer 1：解释性风险强度

保留现有规则评分卡，但角色改变：

- 不再把 `0-100` 当成最终产品输出。
- 改为概率模型的重要输入特征。
- 继续提供单指标、维度、结构/触发/外部三层解释。

这层输出：

```text
indicator_scores
dimension_scores
structural_score
trigger_score
external_shock_score
top_contributors
```

## 5. Layer 2：特征工程

第一阶段特征分五类。

### 5.1 水平类

- VIX
- OAS
- 期限利差
- 失业率

### 5.2 变化类

- 5 日、20 日、60 日变化
- 波动率加速
- 融资条件恶化速度

### 5.3 结构类

- 结构脆弱性分
- 银行体系分
- 房地产分

### 5.4 共振类

- 多维度同时升高
- 市场与信用共振
- 市场与事件共振
- 美国内部压力与 JPY carry 压力共振

### 5.5 历史位置类

- 历史分位
- 历史极值距离
- 与已知危机场景的相似度

## 6. Layer 3：Horizon Models

第一阶段建议采用“简单可控”的组合。

### 6.1 `p_5d`

建议模型：

- 逻辑回归 / Probit
- 重点使用快变量、波动、信用、流动性和事件特征

### 6.2 `p_20d`

建议模型：

- 逻辑回归 + 规则增强
- 同时考虑快变量与结构脆弱性

### 6.3 `p_60d`

建议模型：

- 逻辑回归 / Hazard 风格模型
- 更依赖慢变量和结构脆弱性

### 6.4 第一阶段不采用

- 深度学习
- 复杂图神经网络
- 难以解释的端到端黑盒

## 7. 规则增强

概率模型上方保留一层 override 逻辑。

例子：

- 当 `trigger_score` 极高且 `events` 确认时，允许对 `p_5d` 做上调。
- 当数据严重缺失时，对高概率结论降权。
- 当结构很低、只有单一外部噪声时，不允许 `p_5d` 无约束飙升。

## 8. 校准

未经校准的概率不能直接给用户。

第一阶段至少支持：

- Platt scaling
- Isotonic calibration

每个 horizon 独立校准。

输出：

```text
raw_probability
calibrated_probability
calibration_version
```

## 9. Conviction Score

概率之外再给一个可信度分。

输入因素：

- 数据覆盖率
- 关键特征是否缺失
- 当前是否为多维度共振
- 历史上类似区间的样本数量
- 模型在相似场景下的稳定性

## 10. 历史相似度

系统要支持“当前像不像 `2008` / `2020` / `2023`”。

第一阶段不做复杂度量学习，采用：

- 关键特征向量距离
- 分维度相似度
- 场景窗口局部相似度

输出：

```text
historical_analogs
  scenario_id
  similarity_score
  note
```

## 11. 决策映射

概率引擎不直接输出交易动作，但必须提供 posture input：

```text
if p_5d high and conviction high -> defend
if p_20d elevated and p_5d moderate -> hedge
if p_60d rising and trigger low -> prepare
else -> normal
```

具体阈值见 [decision-support-policy.md](decision-support-policy.md)。

## 12. 模型版本

```text
prob_model_version = prob_v1_YYYYMMDD
```

版本变化包括：

- 标签变化
- 特征增删
- 模型类型变化
- 校准变化
- override 规则变化

## 13. 服务接口

后端建议暴露：

```text
GET /api/assessment/current
GET /api/assessment/history
GET /api/assessment/analogs
GET /api/assessment/method
```

## 14. 最小可行实现

第一阶段 MVP：

1. 用 SQLite 中的真实指标数据生成特征。
2. 先跑规则评分层。
3. 用历史场景生成 `5d/20d/60d` 标签。
4. 对三个 horizon 训练基础逻辑回归。
5. 加上校准层。
6. 输出概率、时距和解释。

## 15. 风险

- 标签少，模型容易不稳。
- 样本类别极不平衡。
- 数据修订会污染概率。
- 如果没有事件层，`p_5d` 的精度可能不够。

## 16. 后续扩展

- Survival analysis
- Markov regime switching
- Tree-based ensemble
- 分市场子模型
- 资产类别专属概率层

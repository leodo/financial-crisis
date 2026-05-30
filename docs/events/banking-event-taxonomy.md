# 银行业风险事件分类设计

状态：`Draft`

最后更新：2026-05-30

## 1. 目标

定义银行与金融机构事件的分类体系，服务 SEC、新闻和后续事件层。

## 2. 一级分类

- liquidity_funding
- deposit_outflow
- capital_adequacy
- credit_loss
- restructuring_resolution
- regulatory_supervisory
- governance_control
- market_confidence

## 3. 事件示例

### 3.1 liquidity_funding

- 融资困难
- 流动性支持
- 紧急融资安排

### 3.2 deposit_outflow

- 存款外流
- 客户集中提款

### 3.3 capital_adequacy

- 资本补充
- 资本约束

### 3.4 regulatory_supervisory

- 监管措施
- 接管 / 救助 / 特别安排

## 4. 输出字段

```text
event_type
event_subtype
severity
entity_id
source_id
source_document_id
evidence_spans
```

## 5. 第一阶段用途

- SEC filing 规则聚合
- 银行事件严重度
- `p_5d / p_20d` 特征输入

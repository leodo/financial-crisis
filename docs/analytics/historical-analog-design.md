# 历史相似阶段设计

状态：`Draft`

最后更新：2026-05-30

## 1. 目标

让系统不仅输出概率，还能回答“当前更像哪段历史压力期”。

## 2. 输出用途

- 总览页历史对照
- 方法页解释
- 防止用户只盯一个概率数字

## 3. 相似度对象

默认场景：

- `2008` 信用与银行危机
- `2020` 流动性冲击
- `2022` 利率重估
- `2023` 区域银行压力

## 4. 特征空间

第一阶段使用：

- structural / trigger / external 分组分
- VIX / OAS / 10Y2Y / NFCI / unemployment
- USDJPY / JPY carry 状态
- 银行业事件强度

## 5. 计算方法

第一阶段不做复杂嵌入学习，使用：

- 标准化距离
- 分组加权距离
- 局部窗口最小距离

## 6. 输出

```text
scenario_id
scenario_name
similarity_score
reference_phase
key_common_drivers
key_differences
```

## 7. UI 表达

- 当前最像哪段历史
- 相似的是“预警前”“冲击中”还是“恢复期”
- 哪些点相似，哪些点不同

## 8. 风险

- 相似不等于重复
- 样本少时容易过拟合叙事

# 概率校准设计

状态：`Draft`

最后更新：2026-05-30

## 1. 目标

把原始模型输出转换成可解释、可回测、可展示的真实概率。

## 2. 必要性

未经校准的模型分通常不是好概率。

如果不校准：

- `0.7` 不一定代表 `70%`
- 高误报会误导仓位和对冲动作

## 3. 校准对象

每个 horizon 单独校准：

- `p_5d_raw`
- `p_20d_raw`
- `p_60d_raw`

输出：

- `p_5d_calibrated`
- `p_20d_calibrated`
- `p_60d_calibrated`

## 4. 方法

第一阶段支持：

- Platt scaling
- Isotonic regression

默认流程：

1. 训练主模型
2. 在独立验证段拟合校准器
3. 固化 `calibration_version`

## 5. 数据切分

禁止随机切分。

建议：

- 训练段
- 校准段
- 最终评估段

全部按时间顺序切分。

## 6. 评估指标

- Brier score
- Log loss
- Reliability curve
- Expected calibration error

## 7. 结果表

```text
analytics_probability_predictions
  prediction_id
  entity_id
  as_of_date
  horizon
  raw_probability
  calibrated_probability
  model_version
  calibration_version
  label_version
  feature_set_version
  created_at
```

## 8. 校准工件

```text
analytics_probability_calibrators
  calibration_version
  horizon
  method
  trained_on_period
  validated_on_period
  parameters_json
  metrics_json
  created_at
```

## 9. 重训条件

以下任一变化都需要重训或重校准：

- 标签变化
- 特征集变化
- 主模型变化
- 样本区间显著扩展

## 10. 前端表达

前端默认只展示：

- 校准后概率
- 概率区间解释
- 当前可信度

方法页再展示：

- 原始概率 vs 校准后概率
- reliability 曲线

## 11. 第一阶段规则

- 样本不足时显示低可信度
- 未校准概率不得作为正式值展示

## 12. 实现顺序

1. 落预测结果表
2. 落校准器工件表
3. 接回测评估

## 13. 风险

- 小样本下 isotonic 容易过拟合
- `5d` 危机样本更少，校准可能不稳

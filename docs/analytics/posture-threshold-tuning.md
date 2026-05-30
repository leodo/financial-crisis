# Posture 阈值调优设计

状态：`Draft`

最后更新：2026-05-30

## 1. 目标

定义 `normal / prepare / hedge / defend` 四类 posture 的默认阈值如何通过历史回测调优。

## 2. 输入

- `p_5d`
- `p_20d`
- `p_60d`
- conviction
- trigger / structural / external 分
- 误报和提前量统计

## 3. 调优原则

- 先控制极端误报
- 再提升提前量
- `defend` 必须保守
- `prepare` 可以更早

## 4. 调优流程

1. 设定初始阈值
2. 在回测场景中评估
3. 比较误报/提前量/稳定性
4. 固化 posture policy version

## 5. 输出

```text
posture_policy_version
thresholds_json
backtest_metrics_json
notes
```

## 6. 风险

- 过度追求提前量会增加误报
- 过度压误报会失去防守价值

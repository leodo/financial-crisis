# 研究和模型工作台设计

状态：`Draft`

最后更新：2026-05-30

## 1. 目标

定义后续模型研究环境，支持在不影响生产抓取和评分服务的前提下，实验统计模型、机器学习模型、图模型和事件分析模型。

生产系统第一阶段使用规则评分卡。研究工作台用于验证改进方法，不直接改变线上评分。

## 2. 边界

研究工作台负责：

- 读取指标快照和回测数据。
- 生成特征。
- 训练和评估模型。
- 输出候选模型报告。
- 对比规则评分卡。

研究工作台不负责：

- 直接写入线上风险等级。
- 直接修改生产权重。
- 绕过数据质量检查。
- 使用未经授权的数据源。

## 3. 推荐技术组合

```text
生产核心：Rust
研究实验：Python + notebooks/scripts
批量数据：Parquet
查询分析：DuckDB / Polars
模型：scikit-learn / statsmodels / XGBoost
```

Rust 仍负责：

- 数据抓取。
- 标准化。
- 质量检查。
- 生产评分。
- API。

Python 负责：

- 特征探索。
- 模型训练。
- 回测实验。
- 图表和研究报告。

## 4. 数据输入

研究工作台只能读取经过发布或快照的数据：

```text
indicator_observations
feature_values
risk_scores
alerts
backtest_snapshots
quality_results
```

导出格式：

- Parquet。
- CSV 仅用于小样本检查。
- JSON 用于模型配置和报告元数据。

## 5. 实验结构

建议目录：

```text
research/
  datasets/
  experiments/
  notebooks/
  reports/
  model_configs/
```

当前阶段只设计，不创建代码目录。等进入实现阶段再决定是否引入。

## 6. 候选模型

### 6.1 统计模型

- Logit/Probit 危机概率模型。
- Survival analysis。
- Markov regime switching。
- Dynamic factor model。

适合：

- 样本较少。
- 需要解释。
- 需要估计未来 3/6/12 个月风险。

### 6.2 机器学习模型

- Random Forest。
- XGBoost/LightGBM。
- Logistic Regression with regularization。

适合：

- 非线性特征组合。
- 指标较多。
- 作为规则评分卡的对照。

注意：

- 危机样本少，不能只看准确率。
- 必须时间序列切分。
- 特征必须 point-in-time。

### 6.3 图和传染模型

- DebtRank。
- 金融机构网络传播。
- GNN 研究模型。

适合：

- 银行间、债务网络、持仓网络数据可得后。

第一版不建议实现。

## 7. 实验元数据

每个实验必须记录：

```text
experiment_id
created_at
data_snapshot_id
feature_set_version
model_type
model_config
train_period
validation_period
test_period
target_definition
metrics
artifact_uri
report_uri
```

## 8. 目标变量设计

危机标签不能随意定义。每个 target 必须声明：

```text
target_id
event_type
region
crisis_window
prediction_horizon
labeling_rule
source_note
```

示例：

- 未来 6 个月是否进入 L3/L4 历史危机窗口。
- 未来 3 个月是否出现市场压力事件。
- 未来 12 个月是否发生银行危机。

## 9. 模型进入生产的门槛

模型不能因为一次回测好就上线。

进入生产候选需要：

- 跨场景表现优于规则评分卡。
- 误报率可接受。
- 能输出解释。
- 对数据质量敏感性可控。
- 训练和推理可复现。
- 有版本和回滚方案。

## 10. 研究报告模板

每个实验报告包含：

- 研究问题。
- 数据快照。
- 特征清单。
- 标签定义。
- 模型配置。
- 训练/验证/测试切分。
- 回测结果。
- 误报和漏报分析。
- 与规则评分卡对比。
- 是否建议进入候选生产。

## 11. 与生产系统的接口

研究模型如果进入生产，应通过统一评分接口输出：

```text
model_score
confidence
prediction_horizon
top_features
method_version
data_quality_summary
```

生产总分是否采用模型分，需要单独 ADR 决策。


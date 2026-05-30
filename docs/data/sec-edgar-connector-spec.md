# SEC EDGAR 连接器实现规格

状态：`Draft`

最后更新：2026-05-30

## 1. 目标

实现一个以银行与金融机构风险事件为主的 `SEC EDGAR` 连接器，为危机概率系统提供：

- filing metadata
- company facts 索引
- 日频事件聚合特征
- 银行业风险公告统计

第一阶段目标不是全文智能解析，而是先打通“官方 API -> 原始响应 -> 结构化元数据 -> 日频事件特征”的稳定链路。

## 2. 范围

### 2.1 第一阶段纳入

- `submissions` JSON
- `companyfacts` JSON
- 8-K / 10-Q / 10-K metadata
- CIK 白名单
- filing document URL 追溯
- 风险关键词和表单规则聚合

### 2.2 第一阶段不纳入

- XBRL 全量财报建模
- 全文 LLM 逐篇解析
- 全市场所有公司全量抓取
- 秒级实时推送

## 3. 官方入口

第一阶段主入口：

```text
https://data.sec.gov/submissions/CIK##########.json
https://data.sec.gov/api/xbrl/companyfacts/CIK##########.json
```

说明：

- `CIK` 必须左侧补零到 `10` 位
- 需要合理 `User-Agent`
- 需要遵守 fair access

## 4. 连接器注册

```text
source_id: sec_edgar
dataset_id:
  sec_company_submissions
  sec_company_facts
  sec_filing_events
```

能力：

- `backfill`
- `incremental`
- `parse_raw`
- `normalize`
- `validate`

## 5. 抓取对象

### 5.1 CIK 白名单

第一阶段只抓固定白名单：

- G-SIB
- 主要美国区域银行
- 重要保险、经纪商、交易所和资产管理机构

建议数量：

- `20` 到 `50`

白名单元数据：

```text
entity_id
cik
ticker
display_name
entity_type
entity_importance
enabled
```

## 6. 抓取模式

### 6.1 历史回填

流程：

1. 对每个 CIK 拉取 `submissions`
2. 保存原始响应
3. 解析 filing metadata
4. 如需更长历史，追加 archived submissions
5. 生成 staging records
6. 聚合为日频事件指标

### 6.2 增量更新

流程：

1. 读取该 CIK 最近 filing watermark
2. 拉最新 `submissions`
3. 找新增 accession
4. 写入 staging
5. 回刷最近 `30` 天事件聚合

## 7. 原始响应保存

每次请求至少保存：

```text
raw_payload_id
source_id=sec_edgar
dataset_id
entity_id
request_url
http_status
response_hash
raw_object_uri
fetched_at
```

## 8. 解析输出

### 8.1 Filing Metadata

最小字段：

```text
entity_id
cik
accession_number
form_type
filing_date
report_date
acceptance_datetime
primary_document
primary_doc_description
is_xbrl
is_inline_xbrl
act
file_number
film_number
raw_payload_id
```

### 8.2 Company Facts

第一阶段只做索引式落地：

```text
entity_id
cik
taxonomy
fact_name
unit
period_end
filed
value
accession_number
raw_payload_id
```

## 9. 事件规则

### 9.1 重点表单

- `8-K`
- `10-Q`
- `10-K`

### 9.2 初始风险关键词

- liquidity
- funding
- deposit
- capital
- downgrade
- restructuring
- bankruptcy
- supervisory
- material weakness
- going concern

### 9.3 第一批指标

| 指标 ID | 含义 |
|---|---|
| `us_event_bank_8k_count` | 白名单银行 8-K 数量 |
| `us_event_risk_keyword_count` | 风险关键词命中数 |
| `us_banking_filing_stress_count` | 银行业 filing 风险聚合计数 |
| `us_event_official_filing_severity` | 官方公告严重度聚合分 |

## 10. 严重度打分

建议：

```text
severity =
  form_type_base
  + entity_importance_boost
  + keyword_boost
  + multi_filing_same_day_boost
```

示例基础分：

- `8-K`: 40
- `10-Q`: 15
- `10-K`: 10

增强项：

- 涉及流动性/资本/融资：`+10` 到 `+25`
- 核心银行：`+10`
- 短期多次申报：`+5` 到 `+15`

## 11. 数据质量规则

必须检查：

- accession number 不为空
- filing_date 可解析
- form_type 在允许范围内
- accession 去重

质量 flags：

- `sec_missing_report_date`
- `sec_duplicate_accession`
- `sec_unknown_form_type`
- `sec_partial_history`
- `sec_keyword_rule_only`

## 12. 限流与公平访问

建议：

- 单实例并发 `1-2`
- 请求最小间隔 `200ms-500ms`
- 默认只跑白名单

## 13. 存储落点

建议新增对象：

```text
staging_sec_filings
staging_sec_company_facts
analytics_event_daily_features
metadata_entity_cik_mappings
```

聚合后的日频事件指标再写入：

```text
ts_indicator_observations
```

## 14. Worker 命令建议

```text
backfill sec-edgar --entity-group us_banks
backfill sec-edgar --cik 0000320193
refresh sec-edgar --entity-group us_banks
```

## 15. 测试要求

- submissions 正常样本
- companyfacts 正常样本
- 重复 accession 样本
- 空 filings 样本
- `429 / 403 / 5xx` 错误样本

## 16. 实现顺序

1. 建 CIK 白名单
2. 完成 submissions 抓取与解析
3. 完成 filing metadata staging
4. 完成事件聚合
5. 再补 companyfacts

## 17. 风险

- recent filings 历史深度有限
- 规则法精度有限
- 白名单过窄会漏掉传染路径

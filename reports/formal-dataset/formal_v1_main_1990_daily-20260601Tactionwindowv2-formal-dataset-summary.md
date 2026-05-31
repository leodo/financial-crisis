# Formal Dataset Summary

- Generated at: 2026-05-31T19:36:56.969370400+00:00
- Dataset key: formal_v1_main_1990_daily:20260601Tactionwindowv2
- Market scope: financial_system
- Feature set: feature_formal_v1_main_20260531
- Label version: formal_label_v1_main
- Scenario set: scenario_v1_main
- PIT mode: best_effort
- Rows: 10374
- Range: 1998-01-05 -> 2026-05-31

## Split Summary

| Split | Rows | Forward 5d+ | Forward 20d+ | Forward 60d+ | Action 5d+ | Action 20d+ | Action 60d+ | Avg Coverage | Core | Trigger | External | Scenarios |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| train | 6224 | 5 (0.1%) | 20 (0.3%) | 60 (1.0%) | 18 (0.3%) | 56 (0.9%) | 121 (1.9%) | 100.0% | 100.0% | 100.0% | 100.0% | 1 |
| calibration | 2075 | 5 (0.2%) | 20 (1.0%) | 60 (2.9%) | 18 (0.9%) | 52 (2.5%) | 62 (3.0%) | 100.0% | 100.0% | 100.0% | 100.0% | 1 |
| evaluation | 2075 | 5 (0.2%) | 20 (1.0%) | 60 (2.9%) | 18 (0.9%) | 56 (2.7%) | 66 (3.2%) | 100.0% | 100.0% | 100.0% | 100.0% | 1 |

## Scenario Coverage

| Scenario | Family | Rows | Splits | Range |
| --- | --- | --- | --- | --- |
| us_covid_liquidity_2020 | acute_market_liquidity_crash | 82 | 1 | 2019-12-26 -> 2020-03-16 |
| us_gfc_2008 | systemic_credit_banking_crisis | 121 | 1 | 2007-05-03 -> 2007-08-31 |
| us_regional_banks_2023 | systemic_credit_banking_crisis | 91 | 1 | 2023-01-07 -> 2023-04-07 |

## Quality Mix

- grade a: 10374 rows

## Recommendation

样本量、split 和覆盖率已具备基础研究条件，可以进入正式训练与 release review。

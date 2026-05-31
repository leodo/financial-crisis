# Formal Dataset Summary

- Generated at: 2026-05-31T19:22:43.630378300+00:00
- Dataset key: formal_v1_main_1990_daily:20260601Tactionwindow
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
| train | 6224 | 5 (0.1%) | 20 (0.3%) | 60 (1.0%) | 299 (4.8%) | 735 (11.8%) | 790 (12.7%) | 100.0% | 100.0% | 100.0% | 100.0% | 1 |
| calibration | 2075 | 5 (0.2%) | 20 (1.0%) | 60 (2.9%) | 63 (3.0%) | 98 (4.7%) | 98 (4.7%) | 100.0% | 100.0% | 100.0% | 100.0% | 1 |
| evaluation | 2075 | 5 (0.2%) | 20 (1.0%) | 60 (2.9%) | 77 (3.7%) | 104 (5.0%) | 104 (5.0%) | 100.0% | 100.0% | 100.0% | 100.0% | 1 |

## Scenario Coverage

| Scenario | Family | Rows | Splits | Range |
| --- | --- | --- | --- | --- |
| us_covid_liquidity_2020 | acute_market_liquidity_crash | 127 | 1 | 2019-12-26 -> 2020-04-30 |
| us_gfc_2008 | systemic_credit_banking_crisis | 790 | 1 | 2007-05-03 -> 2009-06-30 |
| us_regional_banks_2023 | systemic_credit_banking_crisis | 129 | 1 | 2023-01-07 -> 2023-05-15 |

## Quality Mix

- grade a: 10374 rows

## Recommendation

样本量、split 和覆盖率已具备基础研究条件，可以进入正式训练与 release review。

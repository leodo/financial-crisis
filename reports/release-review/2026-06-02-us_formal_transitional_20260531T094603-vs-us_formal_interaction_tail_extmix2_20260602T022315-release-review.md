# Release Review

- Reviewed at: 2026-06-02T02:42:03.656343500+00:00
- Market scope: financial_system
- Verdict: FAIL
- Original active release: us_formal_transitional_20260531T094603
- Restored release after review: us_formal_transitional_20260531T094603

## Releases

| Role | Release ID | Prob Mode | PIT | Feature | Label | Status |
| --- | --- | --- | --- | --- | --- | --- |
| baseline | us_formal_transitional_20260531T094603 | formal_bundle_v1 | best_effort | feature_prob_meta_v1 | label_forward_crisis_v1 | active |
| candidate | us_formal_interaction_tail_extmix2_20260602T022315 | formal_bundle_v1 | best_effort | feature_formal_v1_main_20260531 | formal_label_v1_main | approved |

## Current Runtime Snapshot

| Metric | Baseline | Candidate | Delta |
| --- | --- | --- | --- |
| p_5d | 0.6% | 2.5% | +1.9pp |
| p_20d | 2.6% | 4.5% | +1.9pp |
| p_60d | 5.6% | 18.9% | +13.3pp |
| Posture | normal | normal | — |
| Time bucket | normal | normal | — |

## Runtime Diagnostics

### baseline Runtime

- Release: us_formal_transitional_20260531T094603
- History points: 15152
- Note: 基于运行中 API 返回的 runtime_thresholds 统计历史概率越线次数。 当前 release label_version=label_forward_crisis_v1 不在 scenario catalog 中，Regime 概率分布回退到 scenario_v1_main/formal_label_v1_main 重算（原始错误：label set label_forward_crisis_v1 was not found in scenario catalog）。 Runtime separation summary: 5d=mixed_or_unclear, 20d=mixed_or_unclear, 60d=calibration_crushed_early_warning.
- Thresholds: prepare_p60d=35.0%, hedge_p20d=30.0%, defend_p5d=30.0%
- Runtime policy version: runtime_history_v1_20260601|class=release|prepare=0.350|hedge=0.300|defend=0.300
- Probability floor hits: p_60d>=prepare 0 / p_20d>=hedge 0 / p_5d>=defend 0

| Posture | Count |
| --- | --- |
| normal | 15152 |

| Time bucket | Count |
| --- | --- |
| normal | 15152 |

| Horizon | Early regime | Normal P | Positive-window P | Cooldown P | Early raw lift | Early calibrated lift | Positive-window lift | Cooldown lift | Gap retention | Diagnosis |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 5d | positive_window | 0.6% | 0.6% | 0.6% | 1.22x | 1.00x | 1.00x | 1.00x | -0.00 | mixed_or_unclear |
| 20d | pre_warning_buffer | 2.6% | 2.6% | 2.6% | 1.34x | 1.00x | 1.00x | 1.00x | -0.00 | mixed_or_unclear |
| 60d | pre_warning_buffer | 5.6% | 5.6% | 5.6% | 1.51x | 1.00x | 1.00x | 1.00x | -0.00 | calibration_crushed_early_warning |

| Horizon | Regime | Rows | Share | Avg raw P | Max raw P | Avg calibrated P | Max calibrated P | Raw lift vs normal | Calibrated lift vs normal | Gap retention | Floor hits |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 5d | in_crisis | 826 | 5.5% | 0.6% | 3.2% | 0.6% | 0.6% | 1.44x | 1.00x | -0.00 | 0 |
| 5d | normal | 14261 | 94.1% | 0.4% | 5.7% | 0.6% | 0.6% | 1.00x | 1.00x | — | 0 |
| 5d | positive_window | 15 | 0.1% | 0.5% | 0.8% | 0.6% | 0.6% | 1.22x | 1.00x | -0.00 | 0 |
| 5d | post_crisis_cooldown | 42 | 0.3% | 0.5% | 1.3% | 0.6% | 0.6% | 1.15x | 1.00x | -0.00 | 0 |
| 5d | pre_warning_buffer | 8 | 0.1% | 0.8% | 1.5% | 0.6% | 0.6% | 1.98x | 1.00x | -0.00 | 0 |
| 20d | in_crisis | 836 | 5.5% | 0.9% | 6.2% | 2.6% | 2.6% | 1.56x | 1.00x | -0.00 | 0 |
| 20d | normal | 14125 | 93.2% | 0.6% | 9.7% | 2.6% | 2.6% | 1.00x | 1.00x | — | 0 |
| 20d | positive_window | 60 | 0.4% | 1.2% | 4.3% | 2.6% | 2.6% | 2.04x | 1.00x | -0.00 | 0 |
| 20d | post_crisis_cooldown | 90 | 0.6% | 0.7% | 4.5% | 2.6% | 2.6% | 1.24x | 1.00x | -0.00 | 0 |
| 20d | pre_warning_buffer | 41 | 0.3% | 0.8% | 2.0% | 2.6% | 2.6% | 1.34x | 1.00x | -0.00 | 0 |
| 60d | in_crisis | 836 | 5.5% | 1.6% | 6.6% | 5.6% | 5.6% | 0.71x | 1.00x | 0.00 | 0 |
| 60d | normal | 13971 | 92.2% | 2.2% | 18.7% | 5.6% | 5.6% | 1.00x | 1.00x | — | 0 |
| 60d | positive_window | 180 | 1.2% | 2.7% | 5.1% | 5.6% | 5.6% | 1.19x | 1.00x | -0.00 | 0 |
| 60d | post_crisis_cooldown | 135 | 0.9% | 1.6% | 5.0% | 5.6% | 5.6% | 0.73x | 1.00x | 0.00 | 0 |
| 60d | pre_warning_buffer | 30 | 0.2% | 3.4% | 4.5% | 5.6% | 5.6% | 1.51x | 1.00x | -0.00 | 0 |

### candidate Runtime

- Release: us_formal_interaction_tail_extmix2_20260602T022315
- History points: 15152
- Note: 基于运行中 API 返回的 runtime_thresholds 统计历史概率越线次数。 Regime 概率分布基于 scenario_v1_main/formal_label_v1_main 重算。 Runtime separation summary: 5d=weak_regime_separation, 20d=usable_early_warning_separation, 60d=usable_early_warning_separation.
- Thresholds: prepare_p60d=73.2%, hedge_p20d=52.2%, defend_p5d=5.0%
- Runtime policy version: runtime_history_v1_20260601|class=formal_main|prepare=0.732|hedge=0.522|defend=0.050
- Probability floor hits: p_60d>=prepare 2400 / p_20d>=hedge 999 / p_5d>=defend 2480

| Posture | Count |
| --- | --- |
| normal | 14605 |
| prepare | 477 |
| hedge | 70 |

| Time bucket | Count |
| --- | --- |
| normal | 13900 |
| months | 1053 |
| weeks | 199 |

| Posture | Trigger clause | Count | Share of posture |
| --- | --- | --- | --- |
| prepare | prepare_carry_structural | 383 | 80.3% |
| prepare | prepare_p60d_structural | 273 | 57.2% |
| prepare | prepare_structural_downgrade | 222 | 46.5% |
| hedge | hedge_p20d_context | 70 | 100.0% |

| Horizon | Early regime | Normal P | Positive-window P | Cooldown P | Early raw lift | Early calibrated lift | Positive-window lift | Cooldown lift | Gap retention | Diagnosis |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 5d | positive_window | 4.1% | 3.5% | 1.3% | 0.85x | 0.85x | 0.85x | 0.31x | 1.00 | weak_regime_separation |
| 20d | pre_warning_buffer | 11.3% | 18.0% | 8.6% | 1.54x | 1.33x | 1.59x | 0.76x | 0.72 | usable_early_warning_separation |
| 60d | pre_warning_buffer | 25.8% | 38.8% | 34.7% | 0.92x | 0.82x | 1.51x | 1.35x | 2.42 | usable_early_warning_separation |

| Horizon | Regime | Rows | Share | Avg raw P | Max raw P | Avg calibrated P | Max calibrated P | Raw lift vs normal | Calibrated lift vs normal | Gap retention | Floor hits |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 5d | in_crisis | 826 | 5.5% | 1.3% | 83.8% | 1.3% | 83.8% | 0.32x | 0.32x | 1.00 | 50 |
| 5d | normal | 14261 | 94.1% | 4.1% | 97.0% | 4.1% | 97.0% | 1.00x | 1.00x | — | 2421 |
| 5d | positive_window | 15 | 0.1% | 3.5% | 10.0% | 3.5% | 10.0% | 0.85x | 0.85x | 1.00 | 5 |
| 5d | post_crisis_cooldown | 42 | 0.3% | 1.3% | 5.3% | 1.3% | 5.3% | 0.31x | 0.31x | 1.00 | 1 |
| 5d | pre_warning_buffer | 8 | 0.1% | 16.5% | 42.4% | 16.5% | 42.4% | 3.97x | 3.97x | 1.00 | 3 |
| 20d | in_crisis | 836 | 5.5% | 25.3% | 100.0% | 25.3% | 93.0% | 2.66x | 2.23x | 0.88 | 117 |
| 20d | normal | 14125 | 93.2% | 9.5% | 99.7% | 11.3% | 93.0% | 1.00x | 1.00x | — | 877 |
| 20d | positive_window | 60 | 0.4% | 17.6% | 57.6% | 18.0% | 57.6% | 1.84x | 1.59x | 0.83 | 5 |
| 20d | post_crisis_cooldown | 90 | 0.6% | 8.2% | 33.9% | 8.6% | 33.9% | 0.86x | 0.76x | 2.10 | 0 |
| 20d | pre_warning_buffer | 41 | 0.3% | 14.7% | 44.8% | 15.1% | 44.8% | 1.54x | 1.33x | 0.72 | 0 |
| 60d | in_crisis | 836 | 5.5% | 14.0% | 96.1% | 33.2% | 93.0% | 0.61x | 1.29x | -0.82 | 126 |
| 60d | normal | 13971 | 92.2% | 23.0% | 100.0% | 25.8% | 93.0% | 1.00x | 1.00x | — | 2173 |
| 60d | positive_window | 180 | 1.2% | 37.3% | 96.5% | 38.8% | 93.0% | 1.62x | 1.51x | 0.92 | 56 |
| 60d | post_crisis_cooldown | 135 | 0.9% | 30.7% | 94.8% | 34.7% | 93.0% | 1.33x | 1.35x | 1.16 | 45 |
| 60d | pre_warning_buffer | 30 | 0.2% | 21.1% | 28.7% | 21.1% | 28.7% | 0.92x | 0.82x | 2.42 | 0 |

## Backtest Guardrails

| Metric | Baseline | Candidate | Delta |
| --- | --- | --- | --- |
| timely_warning_rate | 0.0% | 10.0% | +10.0pp |
| actionable_precision | 0.0% | 65.8% | +65.8pp |
| longest_false_positive_episode_days | 0 | 19 | +19 |

## Actionability Diagnostics

### baseline Actionability

- Enabled: false
- Note: This release has no independent actionability head; release review only applies runtime guardrails.

### candidate Actionability

- Enabled: false
- Note: This release has no independent actionability head; release review only applies runtime guardrails.

## Guardrail Result

### Runtime Guard

- longest_false_positive_episode_days increased from 0 to 19

### Probability Guard

- No probability-head guard regressions detected.

### Actionability Guard

- No actionability guard regressions detected.

### Runtime Sanity Guard

- baseline us_formal_transitional_20260531T094603 is also all-normal / zero-floor-hit, so relative guardrails alone are not a sufficient promotion test

### Overall

- longest_false_positive_episode_days increased from 0 to 19
- baseline us_formal_transitional_20260531T094603 is also all-normal / zero-floor-hit, so relative guardrails alone are not a sufficient promotion test

## Recommendation

候选版未通过当前概率头 / 运行时护栏，不应替代当前默认线上版本。应先修正训练目标、标签口径或样本治理，再重新训练复核。

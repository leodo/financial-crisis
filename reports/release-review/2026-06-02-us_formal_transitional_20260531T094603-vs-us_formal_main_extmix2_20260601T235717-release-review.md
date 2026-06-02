# Release Review

- Reviewed at: 2026-06-02T00:40:44.210951400+00:00
- Market scope: financial_system
- Verdict: FAIL
- Original active release: us_formal_transitional_20260531T094603
- Restored release after review: us_formal_transitional_20260531T094603

## Releases

| Role | Release ID | Prob Mode | PIT | Feature | Label | Status |
| --- | --- | --- | --- | --- | --- | --- |
| baseline | us_formal_transitional_20260531T094603 | formal_bundle_v1 | best_effort | feature_prob_meta_v1 | label_forward_crisis_v1 | active |
| candidate | us_formal_main_extmix2_20260601T235717 | formal_bundle_v1 | best_effort | feature_formal_v1_main_20260531 | formal_label_v1_main | retired |

## Current Runtime Snapshot

| Metric | Baseline | Candidate | Delta |
| --- | --- | --- | --- |
| p_5d | 0.6% | 2.8% | +2.2pp |
| p_20d | 2.6% | 4.8% | +2.2pp |
| p_60d | 5.6% | 36.5% | +30.9pp |
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

- Release: us_formal_main_extmix2_20260601T235717
- History points: 15152
- Note: 基于运行中 API 返回的 runtime_thresholds 统计历史概率越线次数。 Regime 概率分布基于 scenario_v1_main/formal_label_v1_main 重算。 Runtime separation summary: 5d=weak_regime_separation, 20d=cold_across_all_regimes, 60d=weak_regime_separation.
- Thresholds: prepare_p60d=58.3%, hedge_p20d=18.3%, defend_p5d=5.0%
- Runtime policy version: runtime_history_v1_20260601|class=formal_main|prepare=0.583|hedge=0.183|defend=0.050
- Probability floor hits: p_60d>=prepare 2865 / p_20d>=hedge 3769 / p_5d>=defend 2898

| Posture | Count |
| --- | --- |
| normal | 14636 |
| prepare | 287 |
| hedge | 229 |

| Time bucket | Count |
| --- | --- |
| normal | 13708 |
| months | 938 |
| weeks | 506 |

| Posture | Trigger clause | Count | Share of posture |
| --- | --- | --- | --- |
| hedge | hedge_p20d_context | 229 | 100.0% |
| prepare | prepare_carry_structural | 228 | 79.4% |
| prepare | prepare_p60d_structural | 159 | 55.4% |
| prepare | prepare_structural_downgrade | 91 | 31.7% |

| Horizon | Early regime | Normal P | Positive-window P | Cooldown P | Early raw lift | Early calibrated lift | Positive-window lift | Cooldown lift | Gap retention | Diagnosis |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 5d | positive_window | 4.5% | 3.9% | 1.4% | 0.85x | 0.85x | 0.85x | 0.32x | 1.00 | weak_regime_separation |
| 20d | pre_warning_buffer | 17.7% | 19.4% | 10.4% | 0.76x | 0.74x | 1.10x | 0.59x | 1.15 | cold_across_all_regimes |
| 60d | pre_warning_buffer | 27.0% | 36.1% | 27.7% | 1.26x | 1.14x | 1.34x | 1.03x | 0.58 | weak_regime_separation |

| Horizon | Regime | Rows | Share | Avg raw P | Max raw P | Avg calibrated P | Max calibrated P | Raw lift vs normal | Calibrated lift vs normal | Gap retention | Floor hits |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 5d | in_crisis | 826 | 5.5% | 1.8% | 59.6% | 1.8% | 59.6% | 0.39x | 0.39x | 1.00 | 61 |
| 5d | normal | 14261 | 94.1% | 4.5% | 90.9% | 4.5% | 90.9% | 1.00x | 1.00x | — | 2829 |
| 5d | positive_window | 15 | 0.1% | 3.9% | 10.6% | 3.9% | 10.6% | 0.85x | 0.85x | 1.00 | 5 |
| 5d | post_crisis_cooldown | 42 | 0.3% | 1.4% | 3.0% | 1.4% | 3.0% | 0.32x | 0.32x | 1.00 | 0 |
| 5d | pre_warning_buffer | 8 | 0.1% | 13.8% | 36.1% | 13.8% | 36.1% | 3.05x | 3.05x | 1.00 | 3 |
| 20d | in_crisis | 836 | 5.5% | 13.2% | 96.4% | 13.5% | 93.0% | 0.80x | 0.76x | 1.27 | 224 |
| 20d | normal | 14125 | 93.2% | 16.5% | 99.9% | 17.7% | 93.0% | 1.00x | 1.00x | — | 3473 |
| 20d | positive_window | 60 | 0.4% | 18.6% | 55.0% | 19.4% | 55.0% | 1.13x | 1.10x | 0.79 | 31 |
| 20d | post_crisis_cooldown | 90 | 0.6% | 9.0% | 45.3% | 10.4% | 45.3% | 0.54x | 0.59x | 0.97 | 26 |
| 20d | pre_warning_buffer | 41 | 0.3% | 12.6% | 35.8% | 13.1% | 35.8% | 0.76x | 0.74x | 1.15 | 15 |
| 60d | in_crisis | 836 | 5.5% | 13.7% | 84.2% | 20.3% | 93.0% | 0.56x | 0.75x | 0.64 | 78 |
| 60d | normal | 13971 | 92.2% | 24.3% | 100.0% | 27.0% | 93.0% | 1.00x | 1.00x | — | 2681 |
| 60d | positive_window | 180 | 1.2% | 34.7% | 79.5% | 36.1% | 79.5% | 1.43x | 1.34x | 0.87 | 61 |
| 60d | post_crisis_cooldown | 135 | 0.9% | 24.8% | 81.1% | 27.7% | 81.1% | 1.02x | 1.03x | 1.37 | 45 |
| 60d | pre_warning_buffer | 30 | 0.2% | 30.7% | 35.2% | 30.7% | 35.2% | 1.26x | 1.14x | 0.58 | 0 |

## Backtest Guardrails

| Metric | Baseline | Candidate | Delta |
| --- | --- | --- | --- |
| timely_warning_rate | 0.0% | 10.0% | +10.0pp |
| actionable_precision | 0.0% | 60.8% | +60.8pp |
| longest_false_positive_episode_days | 0 | 30 | +30 |

## Actionability Diagnostics

### baseline Actionability

- Enabled: false
- Note: This release has no independent actionability head; release review only applies runtime guardrails.

### candidate Actionability

- Enabled: false
- Note: This release has no independent actionability head; release review only applies runtime guardrails.

## Guardrail Result

### Runtime Guard

- longest_false_positive_episode_days increased from 0 to 30

### Probability Guard

- No probability-head guard regressions detected.

### Actionability Guard

- No actionability guard regressions detected.

### Runtime Sanity Guard

- candidate us_formal_main_extmix2_20260601T235717 has zero usable early-warning horizons in runtime regime audit
- baseline us_formal_transitional_20260531T094603 is also all-normal / zero-floor-hit, so relative guardrails alone are not a sufficient promotion test

### Overall

- longest_false_positive_episode_days increased from 0 to 30
- candidate us_formal_main_extmix2_20260601T235717 has zero usable early-warning horizons in runtime regime audit
- baseline us_formal_transitional_20260531T094603 is also all-normal / zero-floor-hit, so relative guardrails alone are not a sufficient promotion test

## Recommendation

候选版未通过当前概率头 / 运行时护栏，不应替代当前默认线上版本。应先修正训练目标、标签口径或样本治理，再重新训练复核。

# Release Review

- Reviewed at: 2026-06-02T04:22:18.748312500+00:00
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
| p_5d | 0.6% | 0.0% | -0.6pp |
| p_20d | 2.6% | 2.0% | -0.6pp |
| p_60d | 5.6% | 5.0% | -0.6pp |
| Posture | normal | normal | — |
| Time bucket | normal | normal | — |

## Runtime Diagnostics

### baseline Runtime

- Release: us_formal_transitional_20260531T094603
- History points: 15152
- Note: 基于运行中 API 返回的 runtime_thresholds 统计历史概率越线次数。 当前 release label_version=label_forward_crisis_v1 不在 scenario catalog 中，Regime 概率分布回退到 scenario_v1_main/formal_label_v1_main 重算（原始错误：label set label_forward_crisis_v1 was not found in scenario catalog）。 Runtime separation summary: 5d=mixed_or_unclear, 20d=mixed_or_unclear, 60d=mixed_or_unclear.
- Thresholds: prepare_p60d=35.0%, hedge_p20d=30.0%, defend_p5d=30.0%
- Runtime policy version: runtime_history_v2_20260602|class=release|prepare=0.350|hedge=0.300|defend=0.300
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
| 20d | pre_warning_buffer | 2.6% | 2.6% | 2.6% | 1.28x | 1.00x | 1.00x | 1.00x | -0.00 | mixed_or_unclear |
| 60d | pre_warning_buffer | 5.6% | 5.6% | 5.6% | 1.20x | 1.00x | 1.00x | 1.00x | -0.00 | mixed_or_unclear |

| Horizon | Regime | Rows | Share | Avg raw P | Max raw P | Avg calibrated P | Max calibrated P | Raw lift vs normal | Calibrated lift vs normal | Gap retention | Floor hits |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 5d | in_crisis | 826 | 5.5% | 0.6% | 3.2% | 0.6% | 0.6% | 1.44x | 1.00x | -0.00 | 0 |
| 5d | normal | 14261 | 94.1% | 0.4% | 5.7% | 0.6% | 0.6% | 1.00x | 1.00x | — | 0 |
| 5d | positive_window | 15 | 0.1% | 0.5% | 0.8% | 0.6% | 0.6% | 1.22x | 1.00x | -0.00 | 0 |
| 5d | post_crisis_cooldown | 42 | 0.3% | 0.5% | 1.3% | 0.6% | 0.6% | 1.15x | 1.00x | -0.00 | 0 |
| 5d | pre_warning_buffer | 8 | 0.1% | 0.8% | 1.5% | 0.6% | 0.6% | 1.98x | 1.00x | -0.00 | 0 |
| 20d | in_crisis | 836 | 5.5% | 1.1% | 6.2% | 2.6% | 2.6% | 1.73x | 1.00x | -0.00 | 0 |
| 20d | normal | 14125 | 93.2% | 0.6% | 9.7% | 2.6% | 2.6% | 1.00x | 1.00x | — | 0 |
| 20d | positive_window | 60 | 0.4% | 1.2% | 4.3% | 2.6% | 2.6% | 1.95x | 1.00x | -0.00 | 0 |
| 20d | post_crisis_cooldown | 90 | 0.6% | 0.8% | 4.5% | 2.6% | 2.6% | 1.29x | 1.00x | -0.00 | 0 |
| 20d | pre_warning_buffer | 41 | 0.3% | 0.8% | 2.0% | 2.6% | 2.6% | 1.28x | 1.00x | -0.00 | 0 |
| 60d | in_crisis | 836 | 5.5% | 3.2% | 11.1% | 5.6% | 5.6% | 1.27x | 1.00x | -0.00 | 0 |
| 60d | normal | 13971 | 92.2% | 2.5% | 18.7% | 5.6% | 5.6% | 1.00x | 1.00x | — | 0 |
| 60d | positive_window | 180 | 1.2% | 2.7% | 5.1% | 5.6% | 5.6% | 1.06x | 1.00x | -0.00 | 0 |
| 60d | post_crisis_cooldown | 135 | 0.9% | 2.4% | 5.8% | 5.6% | 5.6% | 0.94x | 1.00x | 0.00 | 0 |
| 60d | pre_warning_buffer | 30 | 0.2% | 3.0% | 4.3% | 5.6% | 5.6% | 1.20x | 1.00x | -0.00 | 0 |

### candidate Runtime

- Release: us_formal_interaction_tail_extmix2_20260602T022315
- History points: 15152
- Note: 基于运行中 API 返回的 runtime_thresholds 统计历史概率越线次数。 Regime 概率分布基于 scenario_v1_main/formal_label_v1_main 重算。 Runtime separation summary: 5d=usable_early_warning_separation, 20d=usable_early_warning_separation, 60d=separated_but_below_runtime_floor.
- Thresholds: prepare_p60d=73.2%, hedge_p20d=52.2%, defend_p5d=5.0%
- Runtime policy version: runtime_history_v2_20260602|class=formal_main|prepare=0.732|hedge=0.522|defend=0.050
- Probability floor hits: p_60d>=prepare 1362 / p_20d>=hedge 1313 / p_5d>=defend 1686

| Posture | Count |
| --- | --- |
| normal | 15074 |
| hedge | 48 |
| prepare | 30 |

| Time bucket | Count |
| --- | --- |
| normal | 14920 |
| weeks | 176 |
| months | 56 |

| Posture | Trigger clause | Count | Share of posture |
| --- | --- | --- | --- |
| hedge | hedge_p20d_context | 48 | 100.0% |
| prepare | prepare_p60d_structural | 23 | 76.7% |
| prepare | prepare_structural_downgrade | 9 | 30.0% |

| Horizon | Early regime | Normal P | Positive-window P | Cooldown P | Early raw lift | Early calibrated lift | Positive-window lift | Cooldown lift | Gap retention | Diagnosis |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 5d | positive_window | 2.8% | 6.5% | 0.2% | 2.34x | 2.34x | 2.34x | 0.07x | 1.00 | usable_early_warning_separation |
| 20d | pre_warning_buffer | 13.0% | 28.3% | 8.9% | 1.25x | 1.15x | 2.17x | 0.68x | 0.66 | usable_early_warning_separation |
| 60d | pre_warning_buffer | 20.7% | 29.4% | 14.5% | 2.26x | 1.88x | 1.42x | 0.70x | 0.84 | separated_but_below_runtime_floor |

| Horizon | Regime | Rows | Share | Avg raw P | Max raw P | Avg calibrated P | Max calibrated P | Raw lift vs normal | Calibrated lift vs normal | Gap retention | Floor hits |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 5d | in_crisis | 826 | 5.5% | 1.9% | 94.4% | 1.9% | 94.4% | 0.66x | 0.66x | 1.00 | 68 |
| 5d | normal | 14261 | 94.1% | 2.8% | 98.7% | 2.8% | 98.7% | 1.00x | 1.00x | — | 1610 |
| 5d | positive_window | 15 | 0.1% | 6.5% | 21.3% | 6.5% | 21.3% | 2.34x | 2.34x | 1.00 | 5 |
| 5d | post_crisis_cooldown | 42 | 0.3% | 0.2% | 0.8% | 0.2% | 0.8% | 0.07x | 0.07x | 1.00 | 0 |
| 5d | pre_warning_buffer | 8 | 0.1% | 24.2% | 64.2% | 24.2% | 64.2% | 8.64x | 8.64x | 1.00 | 3 |
| 20d | in_crisis | 836 | 5.5% | 13.9% | 96.2% | 14.5% | 93.0% | 1.16x | 1.11x | 0.78 | 63 |
| 20d | normal | 14125 | 93.2% | 12.0% | 99.1% | 13.0% | 93.0% | 1.00x | 1.00x | — | 1238 |
| 20d | positive_window | 60 | 0.4% | 28.0% | 80.1% | 28.3% | 80.1% | 2.33x | 2.17x | 0.95 | 11 |
| 20d | post_crisis_cooldown | 90 | 0.6% | 7.6% | 53.5% | 8.9% | 53.5% | 0.64x | 0.68x | 0.96 | 1 |
| 20d | pre_warning_buffer | 41 | 0.3% | 15.0% | 44.6% | 15.0% | 44.6% | 1.25x | 1.15x | 0.66 | 0 |
| 60d | in_crisis | 836 | 5.5% | 11.8% | 91.7% | 20.2% | 93.0% | 0.68x | 0.97x | 0.10 | 23 |
| 60d | normal | 13971 | 92.2% | 17.2% | 99.9% | 20.7% | 93.0% | 1.00x | 1.00x | — | 1337 |
| 60d | positive_window | 180 | 1.2% | 24.7% | 56.0% | 29.4% | 83.1% | 1.44x | 1.42x | 1.15 | 2 |
| 60d | post_crisis_cooldown | 135 | 0.9% | 10.6% | 46.6% | 14.5% | 56.5% | 0.62x | 0.70x | 0.95 | 0 |
| 60d | pre_warning_buffer | 30 | 0.2% | 38.9% | 51.3% | 38.9% | 51.3% | 2.26x | 1.88x | 0.84 | 0 |

## Backtest Guardrails

| Metric | Baseline | Candidate | Delta |
| --- | --- | --- | --- |
| timely_warning_rate | 0.0% | 10.0% | +10.0pp |
| actionable_precision | 0.0% | 51.9% | +51.9pp |
| longest_false_positive_episode_days | 0 | 5 | +5 |

## Actionability Diagnostics

### baseline Actionability

- Enabled: false
- Note: This release has no independent actionability head; release review only applies runtime guardrails.

### candidate Actionability

- Enabled: false
- Note: This release has no independent actionability head; release review only applies runtime guardrails.

## Guardrail Result

### Runtime Guard

- No runtime guard regressions detected.

### Probability Guard

- No probability-head guard regressions detected.

### Actionability Guard

- No actionability guard regressions detected.

### Runtime Sanity Guard

- baseline us_formal_transitional_20260531T094603 is also all-normal / zero-floor-hit, so relative guardrails alone are not a sufficient promotion test

### Overall

- baseline us_formal_transitional_20260531T094603 is also all-normal / zero-floor-hit, so relative guardrails alone are not a sufficient promotion test

## Recommendation

候选版已经通过当前概率头、相对运行时护栏与动作精度约束，当前唯一阻塞是 baseline 仍属于全程 normal 的冷模型，因此这次 review 还不能直接支持“替代默认正式版”。更合适的结论是：该候选版可以视为新的 active_experimental 研究基线，但要晋升为默认正式版，仍需补足绝对提前量门槛与样本/标签治理证据。

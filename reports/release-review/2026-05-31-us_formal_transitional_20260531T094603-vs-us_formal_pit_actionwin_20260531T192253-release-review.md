# Release Review

- Reviewed at: 2026-05-31T19:33:18.782779400+00:00
- Market scope: financial_system
- Verdict: FAIL
- Original active release: us_formal_transitional_20260531T094603
- Restored release after review: us_formal_transitional_20260531T094603

## Releases

| Role | Release ID | Prob Mode | PIT | Feature | Label | Status |
| --- | --- | --- | --- | --- | --- | --- |
| baseline | us_formal_transitional_20260531T094603 | formal_bundle_v1 | best_effort | feature_prob_meta_v1 | label_forward_crisis_v1 | active |
| candidate | us_formal_pit_actionwin_20260531T192253 | formal_bundle_v1 | best_effort | feature_formal_v1_main_20260531 | formal_label_v1_main | approved |

## Current Runtime Snapshot

| Metric | Baseline | Candidate | Delta |
| --- | --- | --- | --- |
| p_5d | 0.6% | 2.9% | +2.3pp |
| p_20d | 2.6% | 7.6% | +5.0pp |
| p_60d | 5.6% | 10.6% | +5.0pp |
| Posture | normal | normal | — |
| Time bucket | normal | normal | — |

## Backtest Guardrails

| Metric | Baseline | Candidate | Delta |
| --- | --- | --- | --- |
| timely_warning_rate | 37.5% | 12.5% | -25.0pp |
| actionable_precision | 29.6% | 10.2% | -19.4pp |
| longest_false_positive_episode_days | 9 | 84 | +75 |

## Guardrail Result

- timely_warning_rate dropped from 37.5% to 12.5%
- actionable_precision dropped from 29.6% to 10.2%
- longest_false_positive_episode_days increased from 9 to 84

## Recommendation

候选版未通过当前运行时护栏，不应替代当前默认线上版本。应先修正训练目标、标签口径或样本治理，再重新训练复核。

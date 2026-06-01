# Release Review

- Reviewed at: 2026-06-01T01:30:58.841417900+00:00
- Market scope: financial_system
- Verdict: FAIL
- Original active release: us_formal_transitional_20260531T094603
- Restored release after review: us_formal_transitional_20260531T094603

## Releases

| Role | Release ID | Prob Mode | PIT | Feature | Label | Status |
| --- | --- | --- | --- | --- | --- | --- |
| baseline | us_formal_transitional_20260531T094603 | formal_bundle_v1 | best_effort | feature_prob_meta_v1 | label_forward_crisis_v1 | active |
| candidate | us_formal_pit_dualheadguard_20260601T012122 | formal_bundle_v1 | best_effort | feature_formal_v1_main_20260531 | formal_label_v1_main | approved |

## Current Runtime Snapshot

| Metric | Baseline | Candidate | Delta |
| --- | --- | --- | --- |
| p_5d | 0.6% | 0.5% | -0.1pp |
| p_20d | 2.6% | 2.5% | -0.1pp |
| p_60d | 5.6% | 5.5% | -0.1pp |
| Posture | normal | normal | — |
| Time bucket | normal | normal | — |

## Backtest Guardrails

| Metric | Baseline | Candidate | Delta |
| --- | --- | --- | --- |
| timely_warning_rate | 37.5% | 12.5% | -25.0pp |
| actionable_precision | 29.6% | 20.6% | -9.0pp |
| longest_false_positive_episode_days | 9 | 18 | +9 |

## Actionability Diagnostics

### baseline Actionability

- Enabled: false
- Note: This release has no independent actionability head; release review only applies runtime guardrails.

### candidate Actionability

- Enabled: true
- Note: Separate actionability head trained from bounded action-window labels to complement the crisis-prior horizons without replacing them outright.
- Versions: model=actionability_bundle_20260601T012122 calib=actionability_platt_20260601T012122 fusion=fusion_policy_v1_actionability_diag_20260601

| Level | Scenarios | Advance Warn | Late Confirm | Missed | Pre-start Recall | Post-start Recall | Precision | Pred+ | Actual+ | FP |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| prepare | 1 | 0.0% | 0.0% | 100.0% | 0.0% | 0.0% | — | 0 | 66 | 0 |
| hedge | 1 | 0.0% | 0.0% | 100.0% | 0.0% | 0.0% | — | 0 | 56 | 0 |
| defend | 1 | 0.0% | 0.0% | 100.0% | 0.0% | 0.0% | — | 0 | 18 | 0 |

## Guardrail Result

### Runtime Guard

- timely_warning_rate dropped from 37.5% to 12.5%
- actionable_precision dropped from 29.6% to 20.6%
- longest_false_positive_episode_days increased from 9 to 18

### Actionability Guard

- actionability prepare scenario_count is 1 (<2), so the evaluation slice is too narrow for go/no-go
- actionability prepare produced no hits in 66 labeled evaluation positives
- actionability hedge scenario_count is 1 (<2), so the evaluation slice is too narrow for go/no-go
- actionability hedge produced no hits in 56 labeled evaluation positives
- actionability defend scenario_count is 1 (<2), so the evaluation slice is too narrow for go/no-go
- actionability defend produced no hits in 18 labeled evaluation positives

### Overall

- timely_warning_rate dropped from 37.5% to 12.5%
- actionable_precision dropped from 29.6% to 20.6%
- longest_false_positive_episode_days increased from 9 to 18
- actionability prepare scenario_count is 1 (<2), so the evaluation slice is too narrow for go/no-go
- actionability prepare produced no hits in 66 labeled evaluation positives
- actionability hedge scenario_count is 1 (<2), so the evaluation slice is too narrow for go/no-go
- actionability hedge produced no hits in 56 labeled evaluation positives
- actionability defend scenario_count is 1 (<2), so the evaluation slice is too narrow for go/no-go
- actionability defend produced no hits in 18 labeled evaluation positives

## Recommendation

候选版未通过当前运行时 / 动作层护栏，不应替代当前默认线上版本。应先修正训练目标、标签口径、样本切分或样本治理，再重新训练复核。

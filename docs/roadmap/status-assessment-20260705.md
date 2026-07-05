# Crisis-probability model status — 2026-07-05

This is a dated audit snapshot, not the live TODO. Live planning stays in
`crisis-probability-design-todo.md`; this file exists to record a grounded
"where are we, how confident, what blocks us" statement at a single point in
time, after the lead-time audit + adjudication pipeline landed.

All numbers below are pulled from artifacts under `artifacts/research/` and from
the release-review set dated 2026-07-04. Active release baseline:
`us_formal_family_hybrid_20260606T112926` (unchanged ~4 weeks).

## 1. What the model can demonstrably do today

Lead-time on cataloged US crises (baseline L2, from 2026-07-04 release reviews
and `leadtime-audit/.../leadtime-curve-buckets.csv`):

| Crisis | Baseline L2 lead | First-score reach vs crisis_start |
|---|---|---|
| 2007-2009 GFC | 47d before | max -90..-60d |
| 2000 dotcom | 29d before | max -74d |
| 2023 regional banks | 90d before | max +12d (peak AFTER start) |
| 2020 COVID liquidity | 13d before | max +8d (peak AFTER start) |
| 2011 funding stress | 11d before | max +95d (peak AFTER start) |
| 1998 LTCM | 7d before | - |
| 1987 Black Monday | 6d before | - |
| 2022 rate shock | none | max +119d (peak AFTER start) |
| 1994 bond massacre | none | - |
| 1990-93 banking | none | - |

Reading: 5/10 cataloged crises get a multi-day warning before the catalog
`crisis_start` (47/29/11/13/7/6 days). 4/10 clear the "> 1 week actionable"
bar (2011/2020/2000/2008). 3/10 are missed entirely (1987 is at 6d, 1994 and
1990 have no L2 hit). On 4 crises the score *peaks* after `crisis_start`
(2023 +12d, 2020 +8d, 2011 +95d, 2022 +119d) — the high-score mass is
post-event, not pre-event.

Best candidate metrics ever observed (`us_formal_family_hybrid_20260604T034053`
and successor `0604T081030`): `timely_warning_rate 10.0%`,
`actionable_precision ~67-71%`, `longest_false_positive_episode 5d`,
`guard_passed = true`.

**Honest summary of capability**: the model produces a reproducible,
multi-day early-warning signal on roughly half the cataloged US crises, with a
useful precision (~70%) and a short maximum FP episode (~5 trading days). It is
not a 1-week-ahead warning across the board; it is a 1-week-ahead warning on a
subset (GFC, dotcom, 2011, COVID), and a near-coincident signal on the rest.

## 2. Credibility of those numbers

Three independent forms of degradation apply to the headline metrics above.

**(a) The metrics are not strictly held-out.** The release-review
precision/recall/longest-FP numbers are computed across the full loaded runtime
history (`history_limit` 200-2000 points mixing train+calibration+evaluation),
not on the `evaluation` split alone. A held-out `evaluation` split does exist
(`formal_v1_main_1990_daily`, 2,309 main rows, 3 scenarios), but the review
pipeline does not isolate it. So the ~70% precision figure is in-sample-leaning.

**(b) Every 2026-07-04 release-review verdict is FAIL.** Ten reviews, all
`Verdict: FAIL`, all "restored release = baseline". The latest candidate
candidate trained off `feature_formal_v1_main_20260704_macrofragility` regressed
`timely_warning_rate 10.0% → 0.0%` and `actionable_precision 80.5% → 0.0%`
because its 60d decision threshold was repaired to 0.99 — above every realized
score. The active baseline has not been displaced in roughly 4 weeks.

**(c) ~38 evaluation-split no-scenario rows are at risk of becoming false
positives pending human adjudication.** The lead-time audit surfaced 99
no-scenario high-score rows (score ≥ 0.90) in 21 clusters; 8 of those clusters
(45 days across all splits, 38 in the evaluation split) are not explained by
any catalog window and require a human to decide `near_miss_prepare_positive`
(good model) vs `true_false_positive` (bad model) vs the other four classes. The
manual-review template is at
`artifacts/research/leadtime-audit/adjudication/...-manual-review.csv`; policy
at `adjudication/ADJUDICATION-POLICY.md`; consumer at
`scripts/formal-prepare-eval-adjudicated.mjs`. Until those 38 evaluation rows
are labeled, the "true" prepare-FP rate on the evaluation split is undecided in
either direction up to ±38 days.

**Net credibility**: reproducible, internally consistent, and the lead-time
ordering is real (GFC > dotcom > 2011/COVID > LTCM/Black Monday matches the
intuitive severity ordering). But the headline precision is in-sample-leaning,
no candidate has passed guardrails in ~4 weeks, and a non-trivial slice of the
evaluation FP rate is contingent on labels a human has not yet filled in.

## 3. Current bottlenecks, ranked by blocker strength

1. **Threshold-selection policy is broken at 60d.** The 60d decision threshold
   was repaired to 0.990, sitting above the positive-window average of 58.6%
   — i.e. the "alarm threshold" is set higher than any score the model ever
   produces. The leadtime-threshold sweep confirms: at threshold 0.99 there are
   *zero* hits in every bucket. This single bug zeros out the candidate's
   `timely_warning_rate` and `actionable_precision` regardless of how good the
   underlying scores are. (`docs/roadmap/crisis-probability-design-todo.md`
   L273-280; commit `7bc81fd` is a first pass at the fix but has not produced a
   passing review yet.)

2. **60d positive_window separation collapsed in `interaction_tail_v2`.** The
   branch's regime pairwise targets successfully fixed the cooldown bleed
   (cooldown now below normal baseline — good), but the trade squeezed the
   positive_window below where it needs to be for runtime threshold policy to
   admit positives. This is the principal *non-threshold* blocker for advancing
   a candidate on this branch.

3. **Actionability head has insufficient episodes.**
   `insufficient_independent_eligible_episodes: 2 < 4` and
   `insufficient_eligible_scenario_diversity: 2 < 3` for prepare/hedge/defend at
   all three splits. The independent actionability head was omitted entirely in
   at least one recent review pipeline run. The model cannot train an
   independent prepare/hedge/defend head with the current episode inventory.

4. **Catalog is thin.** 10 scenarios, 4 families, 36 years (1990-2026). Of
   those, only **3** are `mandatory` main positives (2020 COVID, 2008 GFC, 2023
   regional banks); 5 are `extension_only`, 1 `candidate_optional`, 1
   `no_positive_main`. The model is essentially fit on 3 distinct genuine
   crisis episodes for the main line.

5. **Some crises fire late.** On 4 of the 10 crises (2022 rate shock, 2011
   funding, 2020 COVID caveat, 2023 regional banks) the *peak* score lands after
   `crisis_start`. The features that drive the model's score on these episodes
   are post-event features, not leading indicators. For these, "the score
   eventually gets high" is not the same as "early warning".

6. **No isolated evaluation report.** As in §2(a), the review pipeline reports
   metrics over mixed splits; there is no "evaluation-only" report card. This
   does not block progress directly, but it makes the credibility question
   harder to answer cleanly.

## 4. Root causes for each bottleneck

1. **Threshold** — software/policy bug, not capacity. The functions
   `select_probability_decision_threshold` and
   `adjust_probability_decision_threshold_for_regime_support` are too
   aggressive and clamp the threshold to the unreachable end. Fixable in code.

2. **positive_window collapse** — interaction-tail pairwise constraints were
   added to kill cooldown bleed; the squeeze on positive_window was a
   side-effect that was not parameterized against. Fixable by retuning the
   pairwise constraint strengths (the branch commit `e31b5b3` already exposes
   these as named constants).

3. **Insufficient episodes** — quantitative data limitation. The catalog has
   too few genuine crisis episodes for the actionability head's episode-count
   gate. Addressable only by catalog expansion (analogs, family-level
   positives) or by relaxing the gate, each with its own risk.

4. **Catalog thinness** — partly inherent (US history has a fixed number of
   crises) and partly a design choice (5/10 are extension/protected only).
   Expandable with international/sectoral analogs at reduced relevance, but
   not without research effort.

5. **Late-firing crises** — feature/timing design. The signature that drives a
   high score on, e.g., the 2022 rate shock is a post-event signature. Needs
   leading-indicator feature work or pre-window label refinement.

6. **No eval-only report** — tooling gap. The release-review pipeline mixes
   splits; a small change to filter to `split_name == "evaluation"` before
   metric computation would close it. Pure engineering.

## 5. Hope for improvement

- **Strong, near-term**: bottlenecks 1 and 6 are pure code/tooling. #1 has a
  partial fix in flight (`7bc81fd`); #6 is a small filter. Neither requires
  retraining or new data.
- **Strong, medium-term**: bottleneck 2 is a tuning exercise on already-named
  constants. The interaction_tail machinery is correct; the trade-off just
  needs re-balancing. Expect a few candidate trainings.
- **Medium, medium-term**: bottleneck 5 (late-firing) can be partially
  addressed with leading-indicator feature work and is the kind of problem the
  interaction_tail line is meant to chip at. Will not be fully solved on the
  current branch.
- **Hard, longer-term**: bottlenecks 3 and 4 (catalog and episode count) require
  actual catalog-design research — analog selection, family positives, gate
  policy. There is genuine hope (US history is thin but international/sectoral
  analogs exist), but the work is research-shaped, not engineering-shaped, and
  the payoff is bounded by analog relevance.

So: **yes, there is real hope, and a concrete ordering.** The next two steps
(threshold-policy audit, positive_window pairwise re-tune) can be done on this
branch without new data; the catalog/episode work is the long pole and should
be its own track.

## 6. Concrete next steps in priority order

1. Audit and fix `select_probability_decision_threshold` /
   `adjust_probability_decision_threshold_for_regime_support` so the 60d
   decision threshold stays below the realized positive-window average. Resume
   candidate training against the repaired threshold policy.
2. Re-tune `interaction_tail_v2` pairwise constraint strengths (commit
   `e31b5b3` constants) to restore positive_window separation *without*
   re-introducing cooldown bleed. The 20d separation of 3.13× (positive_window
   vs normal) is the target to recover on 60d.
3. Human-adjudicate the 8 manual-review clusters in
   `...-manual-review.csv` (38 evaluation rows + 7 calibration). This is the
   user's call; without it the evaluation FP rate is undecidable.
4. Add an evaluation-only report path to the release-review pipeline (filter
   rows by `split_name == "evaluation"` before metric computation), so future
   candidates have a credible held-out number.
5. Catalog-design track (long-pole): pick the next 2-3 international analogs or
   family-level positives to lift the mandatory-main count above 3 and clear
   the actionability episode gate.
6. Feature-design track (long-pole): identify leading indicators for the 4
   late-firing crises (2022 rate shock especially) so the peak-score-on-this-
   episode problem is addressed at the feature level.

## 7. State of the adjudication pipeline landed this branch

- Manual-review template:
  `artifacts/research/leadtime-audit/adjudication/...-manual-review.csv` —
  8 clusters, 17 columns (cluster_id, split_names, regime_60d_values, base +
  overlay contributions, provisional_class, blank adjudicated_class).
- Policy: `adjudication/ADJUDICATION-POLICY.md` (binding). Cooldown = 90d
  rebuttable presumption, TFP penalizes FP, cluster_id stable across re-runs.
- Consumer: `scripts/formal-prepare-eval-adjudicated.mjs`. Post-hoc Node, not a
  training-pipeline hook. Verified: 99 high-score rows / 8 manual + 13 auto
  clusters / 0 orphaned; per-split breakdown (train 0 manual / calibration 7 /
  evaluation 38). Reclassification TP/FP-stays paths verified via a throwaway
  fixture (deleted).
- The 8 rows of `adjudicated_class` are intentionally left blank by the
  assistant; the model never closes that loop (anti-contamination rule).
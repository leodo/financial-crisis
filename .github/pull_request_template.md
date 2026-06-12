## Summary

- What changed:
- Why this change is needed:
- Main affected areas:

## Checks

- [ ] `just verify` passed locally
- [ ] If Rust/API/worker/domain logic changed, I checked whether training/runtime shared logic should be unified instead of patching one side only
- [ ] If I touched a current hotspot file, I either split it first or documented why a direct edit was still justified
- [ ] If product behavior or explanation changed, I updated the relevant docs or TODO
- [ ] If versioned artifacts are included, I documented why they are formal release artifacts, baseline evidence, or curated long-term evidence

## Design / TODO

- Active task source:
  - [ ] `docs/roadmap/crisis-probability-design-todo.md`
  - [ ] `docs/roadmap/engineering-maintainability-todo.md`
- Related design docs:
  - 

## Review Evidence

- [ ] Not applicable
- [ ] `just release-review-fast <candidate>` run and summarized below
- [ ] `just release-review <candidate>` run and summarized below

Evidence / key numbers:

```text
fill in timely_warning_rate / actionable_precision / longest_false_positive_episode_days
```

## Risk Notes

- Backward compatibility / contract impact:
- Remaining risks or follow-ups:

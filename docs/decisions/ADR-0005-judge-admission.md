# ADR-0005: Judge Admission

- **Status.** Accepted
- **Date.** 2026-09-28
- **Provenance.** [Product contract](../PRODUCT-CONTRACT.md) clause 5, [ADR-0002](ADR-0002-sonnet-sole-judge.md), and the [research digest](../research/2026-09-28-design-loops/README.md): JudgeBench's verified pairs, order effects above ten percent, and Design Arena's comparison floor of fifteen.

## Context

The Judge ranks; it never gates. A ranking from a Judge that cannot tell a verified-worse interface from a better one, or that changes its mind when the order is swapped, is noise with a confidence interval. Admission is the check that the Judge is worth listening to, and it must itself be a deterministic computation over recorded Judgments.

## Decision

1. **The admission set** lives under `verification/admission/`. Each pair is a base Swatch and a variant made from it by one deterministic defect injection. A pair is admitted into the set only if a probe Check passes on the base and fails on the variant at the pair's viewport, so "verified worse" is decided by a Check, never by opinion. A test asserts this for every pair.
2. **Each pair is judged twice**, base first and variant first. A pair is **correct** when both Judgments prefer the base; **flipped** when the two Judgments name the same presentation slot (the Judge followed the order, not the content); a tie counts as neither correct nor flipped, and as wrong.
3. **Admission thresholds**, over one complete run of the set, computed by a pure kernel in `jacquard-core`:
   - at least 20 pairs in the set;
   - known-answer accuracy, correct pairs over all pairs, at least 0.90;
   - position-flip rate, flipped pairs over all pairs, at most 0.10;
   - every Judgment answered by the pinned model.
   Any threshold missed rejects the configuration. An incomplete run is `Inconclusive`, never admitted.
4. **What admission establishes**: that this Judge configuration can see mechanical defects of the kinds in the set, consistently across order. **What it does not**: aesthetic judgment, agreement with the owner, or self-preference. Agreement with the owner's Votes is reported beside admission once 15 Votes exist and becomes a threshold by a later ADR. Pairs whose worse side was woven by Sonnet join the set with the first Loom (milestone 05), per ADR-0002.
5. **Re-admission** is required when the rubric, the model identifier, the transport, or the set changes. The admission report records the rubric hash, the model, the set hash, and every Judgment.

## Consequences

The set tests the easy half of judging on purpose: a Judge that fails it is disqualified cheaply, and one that passes has earned a hearing, not trust. A full run is 40 Judgments.

## Validation

Falsified if a configuration is used for ranking without an admission report that meets every threshold, or if a pair is in the set without a Check that separates its two sides.

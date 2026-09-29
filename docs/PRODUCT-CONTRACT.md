# Product Contract

What Jacquard promises. Each clause names the mechanism that keeps it and the milestone that lands the mechanism. A clause without a landed mechanism is a promise, and `CURRENT.md` says so.

## The Factory

1. **Only Checks gate.** A Swatch is accepted when every registered Check reports `Passed` with a Receipt naming the command, the revision, the toolchain and the output hash. No Judgment, Vote, ranking, model approval, consensus, quota or availability decides acceptance. *(Check registry and receipts: milestone 02.)*
2. **Nothing unrendered is judged.** A Swatch that fails to build, render or hydrate at every audit viewport is rejected before any Check beyond rendering runs and before any Judge sees it. *(Renderer port: milestone 03.)*
3. **Judgments are pairwise, order-swapped, and blind.** The Judge sees two Renders and a decomposed rubric, never the generator's log, plan or claims. Every comparison is presented in both orders; a tie scores one half each. No absolute score is claimed. *(Judge port: milestone 03.)*
4. **Rankings carry intervals.** A Ranking is a Bradley–Terry estimate with a confidence interval per Swatch; a Swatch with fewer comparisons than the floor is unranked. *(Semantic kernels: milestone 04.)*
5. **The Judge is admitted, not assumed.** A Judge configuration is used only after it passes a known-answer set (verified-better against verified-worse pairs), reports its position-flip rate, and reports its agreement with the owner's Votes; any change to its prompt or model re-runs admission. *(Milestone 03.)*
6. **Swatch identity is content.** The same source is the same Swatch. Nothing that feeds a Check depends on wall-clock time, randomness or iteration order; for a pinned toolchain and fixtures a Check's Outcome is the same on every run.
7. **Counterexamples are kept.** A Swatch that a Check or Judge wrongly accepted or rejected, once found by any means, is minimized where practical and committed as a fixture with its provenance. Every Check ships with a defect it must reject.
8. **Records, not claims.** "Passed", "accepted" and "ranked" are written only when the corresponding record exists. `NotRun`, `Interrupted`, `Inconclusive`, `Failed` and `EnvironmentUnavailable` are reported as themselves.

## The Products

9. **Modern and performant.** Every Product declares byte budgets, Largest Contentful Paint and Cumulative Layout Shift limits, and WCAG 2.2 AA as Checks, at the audit viewports 2560×1440, 1440×900, 1024×768 and 390×844. The phone viewport is a first-class target, never a degraded one.
10. **Every element earns its place.** A Product's own contract says what each visible element represents; an element whose purpose cannot be explained is removed, not restyled.
11. **Private material never ships.** No Product and no commit names a client, a figure under non-disclosure, compensation or a private assessment. The history of this repository starts clean for that reason (ADR-0003).

## Validation

The contract is falsified if a Swatch is accepted whose only support is a model's verdict, if a Check's expected outcome changes inside an ordinary implementation commit, or if a Product ships with a Check reporting anything but `Passed`.

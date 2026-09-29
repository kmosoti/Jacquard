# ADR-0007: Semantic Mutants

- **Status.** Accepted
- **Date.** 2026-09-28
- **Provenance.** Nomos ADR 0015 §9 and FabricO11y ADR-0018's negative controls; [ADR-0001](ADR-0001-accept-work-on-executable-evidence.md).

## Context

A test suite that passes proves nothing about what it would catch. Generic mutation testing answers that with a score, and a score is what a generator optimizes. What the factory needs to know is narrower: for each invariant it claims, does a named test fail when the code commits the specific wrong behavior the invariant forbids?

## Decision

1. `tests/semantic-mutants/corpus.toml` names, per invariant, one wrong behavior as a source patch (a file, a text that occurs exactly once, its replacement) and the one test that must fail against it.
2. `cargo xtask mutants semantic` copies the tracked tree, applies each patch in turn, and runs the named test alone. It reports, with stable codes: `mutant-survived` (the test passed), `mutant-not-applicable` (the text does not occur exactly once), `mutant-test-missing` (the named test did not run), and `mutant-baseline-failed` (the test fails without the mutant, so it proves nothing). The check runs on the required path.
3. The first corpus names these wrong behaviors: a tie counted as a correct admission answer; either admission threshold made exclusive; the contrast minimum loosened; inline links no longer exempt, or every target exempt; the judge accepting a context with tools attached; the judge hiding the model that answered; the renderer accepting a page that received another viewport; a comparison asked in one order only; Swatch identity without length prefixes; ranking that ignores ties, ranks below the floor, or depends on record order.
4. A surviving mutant is a test to strengthen, never a mutant to weaken or remove. `cargo mutants`, the generic tool, remains a scheduled calibration with no score.

## Consequences

Each invariant has a named defender, and the defender is proven able to fail.

## Validation

Falsified if a corpus entry is removed or its patch changed to make it caught, or if the check passes with a surviving mutant.

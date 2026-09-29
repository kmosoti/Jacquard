# ADR-0001: Accept Work on Executable Evidence, Not on Model Review

- **Status.** Accepted
- **Date.** 2026-09-28
- **Provenance.** FabricO11y ADR-0018 and Nomos ADR 0015, and the [design-loops research digest](../research/2026-09-28-design-loops/README.md).

## Context

In the Installation's build, waves were accepted on a verifier agent's report and a visual critique by a second model, and end-to-end tests were rewritten in the same commits as the code they graded. The research digest shows why that is fragile: a trace claiming success made judges accept 78–90% of visibly failed outputs; agents reached 100% pass rates hiding 68% of true capability; judges' rankings shift up to 14 places across benchmarks; model-generated tests are biased toward what the code does rather than what it should.

## Decision

Candidate generation may be stochastic; acceptance is reproducible verification.

- **Only Checks gate.** Acceptance is decided by registered Checks with Receipts, negative controls, and the product contract. No model approval, consensus, quota or availability is a gate, and no unnamed reviewer replaces one.
- **Judgments rank; they never gate.** A Judge is admitted by a known-answer set, reports its position-flip rate and its agreement with the owner's Votes, and is re-admitted on any change.
- **Oracles are protected.** The product contract, `docs/decisions/`, `docs/architecture/layers.json`, `crates/core/PURITY.toml`, `tests/`, `verification/`, `crates/bin/jacquard-xtask/`, `.github/workflows/`, `.cargo/`, the root `Cargo.toml` and `rust-toolchain.toml`. A change to them is its own commit with a `Trust-Boundary:` line and touches no implementation crate. Enforcement lands in milestone 02.
- **Generated tests are regression tests** unless their oracle comes from outside the code: a contract clause, an exhaustive table, an independently written reference, a metamorphic relation.
- **Negative controls.** Every Check ships with a defect it must reject.
- **Counterexamples become fixtures** with their provenance.
- **Records, not claims.** `Passed` is written only by a run.

## Alternatives Considered

Keep cross-family model review as a gate: blocks on vendor availability and certifies prose, not behavior. A single mandatory reviewer: the same failure with one fewer check. No review: loses the defects reviews find; hence negative controls and counterexample fixtures are required instead.

## Consequences

Easier: completion never waits on a model's quota. Harder: every Check needs a negative control; review by people or models enters the repository only as a counterexample or a Check.

## Validation

Falsified if a Swatch is accepted whose only support is a Judgment, or if a Check's expected outcome changes inside an implementation commit.

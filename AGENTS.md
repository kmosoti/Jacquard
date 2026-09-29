# AGENTS.md

Operational contract for coding agents in Jacquard. Humans follow the same rules through [CONTRIBUTING.md](CONTRIBUTING.md); the two agree.

## Project

Jacquard is a UI factory. A Draft is compiled design intent. A Loom weaves Swatches from it. A cascade of Checks, a Judge and the owner's Votes decide which Swatch is accepted, and only Checks gate. The repository is at the **skeleton stage**: crates carry module docs and the types that pin the evidence rule. Do not add implementation unless the task names a milestone from [docs/CURRENT.md](docs/CURRENT.md) and the crates it covers.

## Source of Truth

| Need | Read |
| --- | --- |
| What is promised | `docs/PRODUCT-CONTRACT.md` |
| Vocabulary | `docs/glossary.md` |
| Crate layout and the dependency rule | `docs/architecture/hexagon.md`, `docs/architecture/layers.json`, `docs/decisions/ADR-0000-foundations.md` |
| The cascade | `docs/architecture/loop.md` |
| Past decisions | `docs/decisions/` |
| Present state, active milestone, risks | `docs/CURRENT.md` |
| Research findings and their sources | `docs/research/` |

Order of authority: product contract, accepted ADRs, architecture views, current state, research. Code is evidence of what exists. If a task conflicts with these, stop and report the conflict.

## Models

Every agent in the design workflow (weaving, critique, judging, visual review, mockups) runs **Claude Sonnet 5.5** and nothing else. Sonnet is the sole Judge ([ADR-0002](docs/decisions/ADR-0002-sonnet-sole-judge.md)). Small local models may answer fixed structured questions as a filter; they never rank. Repository and tooling work is not the design workflow and follows the owner's usual model choice.

## Commands

The toolchain is pinned in `rust-toolchain.toml`.

```sh
cargo fmt    --all --check
cargo check  --workspace --locked --all-targets
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test   --workspace --locked
```

A change is complete only when all four pass. Gates (`cargo xtask check-layers`, `check-core-purity`, `check-trust-boundary`, `checks --profile fast`, `receipts validate`) arrive with milestone 02 and join this list then.

## Layout

| Path | Layer | May depend on |
| --- | --- | --- |
| `crates/core/` | Domain | nothing; `no_std`; external crates only through `crates/core/PURITY.toml` |
| `crates/ports/` | Driven ports | `core/` |
| `crates/app/` | Use cases | `core/`, `ports/` |
| `crates/adapters/` | Driven adapters | `core/` and the one port each implements |
| `crates/bin/` | CLI, composition root, `jacquard-xtask` | anything |

## Rules

1. **Never break the dependency rule.** An adapter dependency in `app/`, `core/` or `ports/` is always wrong.
2. **New crates, new ports and changes to the rule need an ADR**, numbered next.
3. **Use the glossary's vocabulary exactly:** Draft, Loom, Swatch, Render, Check, Outcome, Receipt, Judge, Judgment, Vote, Ranking, Counterexample, Product, Cascade. A model's opinion is a Judgment, never a Check.
4. **Only Checks gate.** No Judgment, Vote, ranking, model approval, consensus or quota decides acceptance.
5. **The Judge sees rendered evidence only.** Never the generator's log, plan or claims.
6. **Comparisons are pairwise and order-swapped.** No absolute scores are claimed.
7. **Determinism.** Swatch identity is a content hash. Nothing that feeds a Check depends on wall-clock time, randomness or iteration order.
8. **No `unsafe`.** Workspace lints forbid it.
9. **Tooling is Rust**, in `crates/bin/jacquard-xtask`, run as `cargo xtask`.
10. **Never weaken a specification to pass a check.** The product contract, `docs/decisions/`, `docs/architecture/layers.json`, `crates/core/PURITY.toml`, `tests/`, `verification/`, the xtask crate, `.github/workflows/`, `.cargo/`, the root `Cargo.toml` and `rust-toolchain.toml` are protected. A commit that touches them carries `Trust-Boundary: specification` or `Trust-Boundary: verifier` and touches no implementation crate; an ignored test, skipped mutant or allowed lint carries `Trust-Boundary: escape-hatch`. Mechanical enforcement lands in milestone 02; the rule holds now.
11. **Private material never enters a Product or the history.** Nothing that names a client, a figure under NDA, compensation or a private assessment.
12. **Record what ran.** Never report a command you did not run as passing; a check that did not run is `NotRun`, a timeout `Inconclusive`. A test you wrote for code you wrote is a regression test unless its oracle comes from outside the code.
13. **Counterexamples become fixtures**, with what found them, the seed or trace, the contract violated and the fixing commit.

## Process

- Branches are `milestone/<nn>-<name>`; never `claude/*`, `agent/*` or `pr-*`.
- Merge with merge commits; a squash loses the declarations.
- Plain Git. Never rewrite published history or force-push.
- One focused change per commit, imperative subject, the *why* in the body.
- Update `docs/CURRENT.md`, the glossary and the affected ADR in the same change.
- Diagrams are Mermaid, unlabeled arrows, rendering on GitHub.

## Completion Report

End a task with: branch, base and final commit; documents and ADRs changed; commands run with their results as printed; test counts; negative controls exercised; receipts written; unchecked behavior; open decisions for the owner. No narrative of reasoning.

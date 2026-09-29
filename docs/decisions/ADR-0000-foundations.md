# ADR-0000: Foundations

- **Status.** Accepted
- **Date.** 2026-09-28
- **Provenance.** The layouts of Nomos (ADR 0000, 0003, 0016) and FabricO11y (ADR-0015, ADR-0016), which the owner named as the model for this repository.

## Context

Jacquard is a factory whose product is interfaces and whose workers are language models. The cost of a wrong structure compounds: a judge that can reach the generator's log, a check that can be edited by the change it grades, a core that reads the clock. The sibling repositories settled these questions once; Jacquard adopts their answers rather than rediscovering them.

## Decision

1. **Hexagonal layout.** Five layers under `crates/`: `core/`, `ports/`, `app/`, `adapters/`, `bin/`. Dependencies point inward: `core` depends on nothing, `ports` on `core`, `app` on `core` and `ports`, an adapter on `core` and the one port it implements, `bin` on anything. Dev, build and target-specific dependencies get no exemption. `docs/architecture/layers.json` assigns every crate; `cargo xtask check-layers` enforces it from milestone 02.
2. **Pure core.** `jacquard-core` is `no_std`, forbids unsafe code, has no build script, and takes external dependencies only from `crates/core/PURITY.toml`. It never reads the clock, environment, filesystem, network, process identity, randomness, threads or global mutable state. Effects are values the application layer performs.
3. **Pinned toolchain.** `rust-toolchain.toml` pins the channel; `--locked` is not optional; workspace lints forbid `unsafe_code`.
4. **Tooling is Rust.** Checks, generators and reproducers that produce a record live in `crates/bin/jacquard-xtask` and run as `cargo xtask <command>`, never in shell or Python scripts.
5. **Vocabulary is fixed** in `docs/glossary.md`. One concept, one name.
6. **Process.** Milestone branches `milestone/<nn>-<name>`; merge commits; no rewriting of published history; one focused change per commit with the *why* in the body.

## Consequences

Easier: an adapter can be replaced without touching a use case; a Judge cannot reach a generator's state by construction; the core tests on the host with no I/O. Harder: every new technology is a new crate; every structural change is an ADR.

## Validation

Falsified if `check-layers` or `check-core-purity` ever needs an exemption that no ADR records.

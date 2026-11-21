# Contributing to Jacquard

Jacquard is licensed under Apache-2.0 and built on Rust `1.100.0`. The most valuable contributions are careful ones: a sharper contract, a cleaner port, a pure-Rust UI primitive, or a check with a negative control.

## Before You Start

1. [README](README.md): what Jacquard is for.
2. [Current state](docs/CURRENT.md): what exists, what is active, what is open.
3. [Product contract](docs/PRODUCT-CONTRACT.md): what is promised.
4. [Architecture](docs/architecture/README.md) and [ADR-0000](docs/decisions/ADR-0000-foundations.md).
5. [Glossary](docs/glossary.md): one concept, one name.

For anything bigger than a small fix, open an issue first.

## Toolchain and Checks

Rust `1.100.0` (with `wasm32-unknown-unknown`) is pinned in `rust-toolchain.toml`. Before opening a pull request:

```sh
cargo fmt    --all --check
cargo check  --workspace --locked --all-targets
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test   --workspace --locked
```

Then the gates, which CI runs on every push and pull request through the same command:

```sh
cargo xtask checks --profile fast --base origin/main
```

That runs the four commands above plus `check-layers`, `check-core-purity`, `check-trust-boundary`, `mutants semantic`, and `receipts validate`, and writes a receipt per check under `target/verification/receipts/`. Each gate prints a stable code per failure and is tested against fixtures under `tests/fixtures/`, one case or more per code ([verification/README.md](verification/README.md)).

## Where Code Goes

```mermaid
flowchart LR
    bin["crates/bin/<br/>jacquard CLI, xtask"] --> app["crates/app/<br/>use cases"]
    bin --> adapters["crates/adapters/<br/>jacquard-&lt;port&gt;-&lt;technology&gt;"]
    app --> core["crates/core/<br/>domain"]
    app --> ports["crates/ports/<br/>contracts"]
    adapters -.-> ports
    ports --> core
```

Dependencies point inward. `core/` depends on nothing. `app/` never depends on `adapters/`. Only `bin/` wires adapters to ports. A new technology is a new adapter crate; it never adds a dependency to the core. Reference Products live under `products/<name>/` in their own isolated Cargo workspaces.

## How Work Is Judged

Most code here is reviewed against executable Checks. A model's verdict is never a gate ([ADR-0001](docs/decisions/ADR-0001-accept-work-on-executable-evidence.md)). What gates is a registered Check with a Receipt. The specification and the verifier are protected; a change to them is declared in its own commit with a `Trust-Boundary:` line ([AGENTS.md](AGENTS.md) rule 10). A test written alongside the code it tests is a regression test; a test whose oracle comes from outside the code is evidence for a property.

## Documentation

Direct, compact, skeptical prose built on mechanisms. American spelling, Title Case headings, Oxford comma. Mermaid diagrams only, with no labels on arrows. Decisions go in `docs/decisions/` with Status, Date, Context, Decision, Consequences and Validation. Research snapshots under `docs/research/` are dated, cite their sources, and are never edited; a revision is a new snapshot.

## Commits, Branches, Pull Requests

- Single-branch `main` workflow with stacked Jujutsu (`jj`) pull requests (`stack/<version>/<nn>-<name>`). Collaborators (including `@kennmosoti-maker`) open PRs targeting `main` with `@kmosoti` CODEOWNERS review.
- One change per commit. Oracle changes (`Trust-Boundary: specification` or `Trust-Boundary: verifier`) stay in their own commits, separated from implementation commits.
- Imperative subject, the *why* in the body.
- CI is green before review. Preserve commit boundaries (`--no-ff` merge) so `check-trust-boundary` passes on every commit.

## License

Licensed under the [Apache License, Version 2.0](LICENSE) (see [NOTICE](NOTICE)).

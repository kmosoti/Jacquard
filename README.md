# Jacquard

**Jacquard** is an Apache-2.0 Rust UI factory. It takes design intent, weaves candidate interfaces, and accepts them on executable evidence: rendering, WASM byte budgets, OKLCH WCAG contrast floors, settled-idle runtime checks, layout probes, and blind pairwise comparison. A language model proposes; the repository verifies. No model's verdict is a gate.

The name is the loom. Punched cards told it which threads to lift, and one machine could weave any pattern. Here the card is a Draft, the cloth is a Swatch, and the inspection is a cascade of checks that gets more expensive only for candidates that survive the cheap ones.

## Reference Product: `products/cockpit`

The shipped reference Product in this repository is [`products/cockpit`](products/cockpit/README.md), a pure-Rust **Hallmark x Leptos `0.8.23` (`tachys`)** telemetry cockpit and documentation platform built from four workspace crates:

- [`iso-diagram`](products/cockpit/crates/iso-diagram/src/lib.rs): zero-dependency 2.5D isometric hardware schematic eDSL, 2-pass Sugiyama layered layout with virtual nodes, Hanan-grid orthogonal A* conduit router, topological 2.5D painter's sort, lightweight `SceneGeometry`, 360-degree orbit projection, and `iso-render` SVG CLI.
- [`telemetry-scene`](products/cockpit/crates/telemetry-scene/src/lib.rs): generic telemetry JSON-to-2.5D scene projector (`ClusterTopology`, `TelemetryBundle`, `example-cluster` fixtures).
- [`cockpit-wiki`](products/cockpit/crates/cockpit-wiki/src/lib.rs): pure-Rust Markdown with native `iso-diagram` Mermaid flowchart-to-SVG rendering, server-side syntax highlighting, BM25 inverted search index, strict URL scheme allowlist, table-of-contents, internal link verifier, and RSS 2.0 / Atom 1.0 feed generator.
- [`cockpit-leptos`](products/cockpit/crates/cockpit-leptos/src/lib.rs): Hallmark `@layer` / `@scope` OKLCH design tokens, fine-grained `--features hydrate` `#[island]`s (`StageOrbitIsland`, `InspectorStreamIsland`, `CommandPaletteIsland`), `verify_tokens_wcag_aa`, and `0 Hz` settled idle SSE bridge (`<= 250 KiB` WASM budget verified at `239,025 B`).

## Repository Architecture

Jacquard follows the same engineering model as [Nomos](https://github.com/kmosoti/Nomos) and [FabricO11y](https://github.com/kmosoti/FabricO11y): a product contract, numbered decisions, a pure core behind hexagonal ports, semantic mutants, signed-off check receipts, and counterexamples kept as fixtures.

- [`crates/core/jacquard-core`](crates/core/jacquard-core): pure domain types, viewports, WCAG/layout probe rules, Bradley-Terry ranking, and admission kernels (`PURITY.toml`).
- [`crates/ports/jacquard-ports`](crates/ports/jacquard-ports): Loom, Renderer, Checker, Judge, and Ledger port traits.
- [`crates/adapters`](crates/adapters): Chrome & Playwright renderers, filesystem ledger, and Claude Loom/Judge adapters.
- [`crates/app/jacquard-app`](crates/app/jacquard-app) & [`crates/bin/jacquard`](crates/bin/jacquard): orchestration and `jacquard` CLI (`render`, `compare`, `rank`, `admission verify|run`, `product check|probe|budget`).
- [`crates/bin/jacquard-xtask`](crates/bin/jacquard-xtask): `cargo xtask` verification gates (`check-layers`, `check-core-purity`, `check-trust-boundary`, `mutants semantic`, `receipts validate|record`, `checks --profile fast`).
- [`verification/`](verification/checks.toml): trust-boundary manifest, check registry, admission fixtures, semantic mutants, and NDJSON check receipts.

## Quickstart & Verification

Toolchain is pinned to Rust `1.100.0` (including `wasm32-unknown-unknown`) via [`rust-toolchain.toml`](rust-toolchain.toml). Layout rendering checks expect Chromium at `JACQUARD_CHROME`; WASM budget packaging requires `wasm-bindgen` (`0.2.130`) and `wasm-opt` (`binaryen`).

```sh
# Full repository fast verification suite (root + products/cockpit + receipts)
JACQUARD_CHROME=/usr/bin/google-chrome cargo xtask checks --profile fast --base origin/main

# Standalone products/cockpit workspace + WASM checks
(cd products/cockpit && \
  cargo fmt --all --check && \
  cargo clippy --workspace --all-targets --locked -- -D warnings && \
  cargo test --workspace --locked && \
  cargo check -p cockpit-leptos --no-default-features --features hydrate --target wasm32-unknown-unknown --locked)

# Validate committed check receipts
cargo xtask receipts validate
```

## CI & Reproducible Releases

- [`.github/workflows/ci.yml`](.github/workflows/ci.yml): SHA-pinned CI running `cargo xtask checks --profile fast` and `cockpit-leptos` WASM hydrate checks on Rust `1.100.0`.
- [`.github/workflows/release.yml`](.github/workflows/release.yml): SHA-pinned tag release workflow producing attested `jacquard` CLI (`tar.zst`), reproducible `cockpit-dist` WASM/SSR/SVG bundle (`tar.zst`), `SHA256SUMS`, `RELEASE-EVIDENCE.json`, and `actions/attest-build-provenance`.

## Documentation & Governance

- [Project State & Milestones](docs/CURRENT.md)
- [Product Contract](docs/PRODUCT-CONTRACT.md) & [Cockpit Contract](products/cockpit/CONTRACT.md)
- [Architecture Overview](docs/architecture/README.md) & [Cockpit Guide](products/cockpit/README.md)
- [Contributing](CONTRIBUTING.md) · [Agent Rules](AGENTS.md) · [Security Policy](SECURITY.md) · [Apache-2.0 License](LICENSE)

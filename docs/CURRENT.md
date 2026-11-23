# Current Project State

## Active Work

**Security Hardening & Release Prep (`v0.1.1-alpha.1` on `fix/v0.1.1-alpha.1-daybreak`)** on Rust `1.100.0`:

- Remediated all 11 Daybreak findings across `jacquard-ports`, `jacquard` CLI (`swatch_loader`), `jacquard-renderer-chrome`, `jacquard-renderer-playwright`, `jacquard-loom-claude`, `cockpit-wiki`, `jacquard-xtask`, and `.github/workflows/release.yml` (see [`docs/releases/v0.1.1-alpha.1.md`](releases/v0.1.1-alpha.1.md) and [`SECURITY.md`](../SECURITY.md)).

**Public Apache-2.0 Release & `products/cockpit` (`stack/v0.1.0-alpha.1`)** on Rust `1.100.0`:

- Adopted Apache-2.0 (`LICENSE`, `NOTICE`), `SECURITY.md`, `.github/CODEOWNERS` (`* @kmosoti`), and single-branch `main` stacked `jj` PR workflow.
- Superseded ADR-0003 for public release with [ADR-0009](decisions/ADR-0009-public-cockpit-product.md), keeping personal portfolio sources outside Jacquard.
- Landed `products/cockpit` (`iso-diagram` with 2-pass Sugiyama + virtual nodes + Hanan-grid A* router + topological 2.5D painter's sort + `SceneGeometry`, `telemetry-scene`, `cockpit-wiki` with BM25 search + syntax highlighting + native SVG Mermaid rendering + strict scheme allowlist, and Hallmark x Leptos `0.8.23` fine-grained `--features hydrate` Islands), gated by executable `cockpit-*` fast checks emitting [`verification/receipts/2026-11-22-cockpit.ndjson`](../verification/receipts/2026-11-22-cockpit.ndjson) (`17/17` fast checks passing; WASM size `239,025 B <= 256,000 B`).
- Upgraded `probe.js` (CSS Color Level 4 canvas sRGB readback + `O(N)` spatial grid overlap bucketing) and `jacquard-xtask` (merge-commit `--cc` combined-diff trust-boundary inspection + command-bound receipt SHA-256).
- Added SHA-pinned [`.github/workflows/release.yml`](../.github/workflows/release.yml) publishing attested `jacquard` CLI, reproducible `cockpit-dist`, `SHA256SUMS`, and `RELEASE-EVIDENCE.json`.

## Milestones

| Nn | Name | Lands | Status |
| --- | --- | --- | --- |
| 01 | architecture-foundation | workspace, layers, contract, glossary, ADR-0000..0003, research digest | landed |
| 02 | verification-foundation | `jacquard-xtask`: `check-layers`, `check-core-purity`, `check-trust-boundary`, check registry with receipts, fixture negative controls, CI | landed |
| 03 | factory-ports | Loom, Renderer, Checker, Judge and Ledger ports and their first adapters; Judge admission set | landed |
| 04 | semantic-kernels | Bradley–Terry ranking with reference implementation and metamorphic relations; semantic-mutant corpus | landed |
| 05 | public-cockpit-product | Rust `1.100.0`, Apache-2.0, Product adapters (`jacquard-renderer-playwright`, `jacquard-loom-claude`), `products/cockpit` (`iso-diagram`, `telemetry-scene`, `cockpit-wiki`, `cockpit-leptos` hydrate Islands), executable Hallmark/WCAG/idle/WASM Checks (`2026-11-22-cockpit.ndjson`), SHA-pinned release workflow, docs | staged locally (`stack/v0.1.0-alpha.1`) |
| 06 | loom-sight | a Loom that sees its own renders and test output within a Weave; needs its own ADR | proposed |
| 07 | local-filter | local VLM answering fixed structured questions, measured against fixtures | planned |

## Implemented

- Workspace across five layers, members globbed by layer ([hexagon](architecture/hexagon.md), [layers.json](architecture/layers.json)).
- `jacquard-core`: vocabulary types, viewports, the five probe rules (overflow, clipped text, overlapping text, contrast, target size), and the admission kernel.
- Ports: Loom, Renderer, Checker, Judge, Ledger. Adapters: `jacquard-ledger-fs`, `jacquard-renderer-chrome`, `jacquard-judge-claude`, `jacquard-renderer-playwright`, `jacquard-loom-claude`.
- `jacquard render | compare [--ledger] | rank | admission verify | admission run | product ...`.
- Ranking (ADR-0006): Bradley–Terry with a phantom prior, MM estimation agreeing with an independent Newton reference to 1e-8, 95% bootstrap intervals, the floor of 15.
- The semantic-mutant corpus (ADR-0007): 16 wrong behaviors, all caught.
- The admission set: 25 known-answer pairs, all verified. **Sonnet 5.5 admitted** on 2026-09-28: rubric `5e4b17a93faf026b`, 25 of 25 correct, 0 flipped, all 50 Judgments answered by `claude-sonnet-5-5` ([report](../verification/admission/reports/5e4b17a93faf026b-claude-sonnet-5-5.ndjson)).
- `jacquard-xtask`: five gates (layers, purity, trust boundary, receipts, semantic mutants), the check registry, receipts, SHA-pinned CI.
- Product contract, glossary, ADR-0000 to ADR-0009, the design-loops research digest.

## Known Gaps

- The admission set tests mechanical defects only, and Sonnet 5.5 scored at the ceiling. A harder set (subtle hierarchy, spacing and density differences) and the owner's Votes are what would measure taste.
- The alias `sonnet` answers as `claude-sonnet-5`; design-workflow agents must pin `claude-sonnet-5-5`.
- `check-trust-boundary` inspects every commit including merge-commit combined diffs (`--cc`); semantic weakening inside a declared oracle commit remains a human review responsibility.
- Receipts are not signed; a receipt proves structure and that the recorder ran the command.
- The `std` scan in `check-core-purity` is syntactic; effects reached through an allowlisted crate are a review matter (the allowlist is empty).

## Current Assumptions

- Sonnet 5.5 is both Loom and sole Judge ([ADR-0002](decisions/ADR-0002-sonnet-sole-judge.md)); self-preference is mitigated, not removed.
- Local models filter; they never rank.
- Designers agree with each other at Krippendorff α≈0.25 on UI preference, so the owner's Votes, not a population, are the calibration target.

## Unresolved Questions

- Whether `jacquard-judge-local` is worth its adapter before the harder taste admission set lands.

# ADR-0003: The Installation Is the First Product, Imported From a Clean Snapshot

- **Status.** Superseded for public release by [ADR-0009](ADR-0009-public-cockpit-product.md)
- **Date.** 2026-09-28
- **Provenance.** The Installation repository (the personal portfolio rebuild, main at 4a7f0a1 on 2026-09-28) and the owner's decision to reframe the work as a UI factory.

## Context

The Installation is a Rust and Three.js site built with the loop Jacquard now formalizes. Its working tree is fit to keep; its history is not. Early commits carry a document that names a client and figures under non-disclosure, which the product contract forbids in a Product or its history. The owner's rules forbid rewriting published history; this history is unpublished, so it can simply not be published.

## Decision

1. The Installation becomes Jacquard's first Product in milestone 05, imported as a snapshot of its working tree at a stated commit, under a product directory, with a manifest recording the source commit. Its history stays in the owner's local checkout.
2. The Product brings its own contract, which adds clauses Jacquard's does not have: every element on its board represents a plausible system behavior with an origin in its simulation; claim discipline with evidence classes; the positioning rules of its profile.
3. Its existing checks (end-to-end tests, budgets, the claim validator, the `__sceneDiag` probe) are registered as Checks with Receipts, and its known open findings (the Archive bay's framing, truncated status text, label overlap) become the first Counterexamples.
4. Until milestone 05, the Installation is served from its own checkout and is not part of this workspace.

## Consequences

Easier: Jacquard's gates are exercised by a real Product from milestone 05. Harder: the Product's build (WASM, Vite, headless Chromium) has to run under `cargo xtask` on one machine with the resource limits already learned.

## Validation

Falsified if any commit in this repository contains the excluded document or the client's name.

# ADR-0009: Public Apache-2.0 Release and the Cockpit Reference Product

- **Status.** Accepted
- **Date.** 2026-11-21
- **Supersedes (for public release).** [ADR-0003](ADR-0003-installation-first-product.md)
- **Provenance.** [ADR-0000](ADR-0000-foundations.md), [ADR-0001](ADR-0001-accept-work-on-executable-evidence.md), [ADR-0008](ADR-0008-products.md).

## Context

[ADR-0003](ADR-0003-installation-first-product.md) originally planned to import a personal portfolio site as Jacquard's first Product. Publishing Jacquard under Apache-2.0 requires keeping personal portfolio content outside Jacquard's open-source repository and shipping a clean, reusable, pure-Rust reference Product with no personal fixtures, private metrics, or Node/Three.js runtime dependency.

## Decision

1. **Keep personal portfolio sites outside Jacquard.** Supersede ADR-0003 for public release; Jacquard's public history carries zero personal portfolio sources or private client material ([Product Contract](../PRODUCT-CONTRACT.md) Clause 11).
2. **Adopt Apache-2.0 and Rust `1.100.0`.** Publish Jacquard under Apache-2.0 (`LICENSE`, `NOTICE`) with single-branch `main` and stacked `jj` pull requests.
3. **Land `products/cockpit` as the reference Product.** `products/cockpit` provides four pure-Rust crates under its own workspace:
   - `iso-diagram`: zero-dependency 2.5D isometric eDSL, 12 procedural hardware sprites, orthogonal weighted conduit router, orbit camera, shadow projection, and SVG renderer.
   - `telemetry-scene`: generic telemetry JSON-to-2.5D scene projector without vendor runtime CLI dependencies.
   - `cockpit-wiki`: compile-time and runtime Markdown, Mermaid, and RSS/Atom engine with path-traversal and HTML sanitization.
   - `cockpit-leptos`: Hallmark x Leptos `0.8.23` Islands/SSR+CSR cockpit gated by Hallmark CSS checks, WCAG 2.2 AA contrast floor, `0 Hz` idle verification, and a `<= 900 KiB` WASM byte budget emitting Jacquard Receipts.

## Consequences

Easier: downstream sites and operator consoles consume pure-Rust crates without Node/Three.js or personal fixtures, and every UI quality rule produces an executable Jacquard Receipt. Harder: `products/cockpit` owns its own isolated Cargo workspace and `wasm-bindgen`/`wasm-opt` packaging budget check.

## Validation

Falsified if any private client or personal configuration enters `products/cockpit` or reachable git history, or if any registered `cockpit-*` fast Check fails.

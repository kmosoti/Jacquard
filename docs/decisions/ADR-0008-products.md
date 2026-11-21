# ADR-0008: Products, the Page Renderer, and the Product Loom

- **Status.** Accepted
- **Date.** 2026-09-29
- **Provenance.** [ADR-0003](ADR-0003-installation-first-product.md), [ADR-0004](ADR-0004-factory-ports.md), and the Installation's build as of its commit 4a7f0a1: a Leptos SSR server, a hydrating WASM bundle and a Three.js scene, tested with Playwright under SwiftShader.

## Context

A Product is not a static page. The Installation needs its own server, hydration and a WebGL scene before there is anything to see, so the file-based Chrome renderer of ADR-0004 cannot render it. Its code is Rust, TypeScript and CSS across dozens of files, so a Loom that returns one HTML file cannot improve it. And its build is heavy enough that two concurrent builds have taken this machine down.

## Decision

### Layout

- `products/<name>/` holds a snapshot of the Product's tree. It is its own Cargo workspace and Node package, built with its own target directory, never a member of Jacquard's workspace.
- `products/<name>/CONTRACT.md` is the Product's contract and is **specification**.
- `verification/products/<name>/` holds the snapshot manifest, the pages, the recorded baselines and the Product's measurement records, and is **verifier**.
- Everything else under `products/` is **implementation**. Its own tests are regression tests in the sense of ADR-0001.

### Page Rendering

A new port, `PageRenderer`, renders a URL at viewports into the same `Render` the file renderer produces. Its first adapter, `jacquard-renderer-playwright`, drives the Product's own Playwright and Chromium through a Node script, waits for the page to settle (hydrated, the scene ready or the schematic shown, the strip ticking), injects the same layout probe the Chrome renderer injects, and screenshots the viewport. The probe's rules stay in `jacquard-core`.

### The Product Loom

`jacquard-loom-claude` weaves by running Claude Sonnet 5.5, pinned as `claude-sonnet-5-5`, as a coding agent over a copy of the Product's tree. It gets the Product's contract and one Draft, and only the tools Read, Glob, Grep, Edit and Write, in that copy. It has no shell: builds, tests and renders are run by the factory, serialized under the machine locks. It never sees a Judgment. The Weave is the edited copy and the list of files it changed.

### The Product Gate Is a Ratchet

The Product's existing layout findings are recorded once as a baseline in `verification/products/<name>/`; each is a Counterexample to fix. The probe gate passes when the current findings are a subset of the baseline: a change may remove findings, never add them. Shrinking the baseline is a verifier change, committed with the evidence that the finding is gone.

### Acceptance of a Woven Change

A Weave is accepted only if the Product builds and every registered Product check passes. Among accepted Weaves, the Sonnet Judge compares each against the current Product in both orders at the Draft's pages and viewports; a Weave the Judge does not prefer in both orders is not merged. The owner can veto any merge.

## Consequences

Easier: the factory can improve a real application, and every improvement arrives with its checks and Judgments. Harder: a candidate costs a build (minutes), so Drafts are narrow and candidates few; the Product's checks run locally, not in GitHub CI, until the Product's toolchain is set up there.

## Validation

Falsified if a merged Weave was not built and checked, if the probe baseline grows, or if a Loom run had a shell.

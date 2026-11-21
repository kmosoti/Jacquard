# Product Contract: Jacquard Cockpit

`products/cockpit` is the open-source pure-Rust reference Product of Jacquard (see [ADR-0008](../../docs/decisions/ADR-0008-products.md) and [ADR-0009](../../docs/decisions/ADR-0009-public-cockpit-product.md)). It proves that complex 2.5D interactive telemetry cockpits, documentation wikis, and blogs can be built, SSR-rendered, hydrated via Leptos `0.8.23` (`tachys`) fine-grained `--features hydrate` Islands, and accepted on executable evidence with zero Node.js/Three.js bundlers.

Every numbered claim in this contract binds a testable gate.

## 1. Pure-Rust 2.5D Architecture (No Three.js / Bundler)

All 2.5D diagrams, shadows, orthogonal conduit routing, and telemetry scenes compile through pure Rust (`iso-diagram` and `telemetry-scene`) and render into semantic SVG/HTML via Leptos `0.8.23` (`tachys`). Legacy JS/Three.js `wasm-head` bundles are retired.

## 2. Hallmark CSS Cascade & Semantic Landmarks

1. CSS declares `@layer reset, tokens, base, layout, components, utilities` up front with zero global `*` reset, zero `!important`, zero `transition: all`, zero decorative gradients, and zero raw color literals outside `tokens.css`.
2. Operational surfaces scope under `@scope (.app-surface)` without card-in-card borders, and use Container Queries (`@container`), `subgrid` alignment, View Transitions, `<dialog closedby="any">`, and viewport-aware `popover="auto"` anchor positioning.
3. Rendered SSR/CSR HTML contains one `<main>` landmark, `<header>`, `<nav>`, `<aside>`, `<h1>`, skip link, accessible focus outlines (`:focus-visible`), paired geometric SVG + text status badges (WCAG 1.4.1), and zero emoji status markers.

## 3. WCAG 2.2 AA Contrast Floor

Resolved OKLCH pairs in both light (`Atelier-Draft`) and dark (`Oxide-Monolith`) palettes satisfy WCAG 2.2 Level AA:

- Primary, muted, accent-button, and semantic status text pairs over their paired canvas/surface/status backgrounds achieve contrast ratio `>= 4.5:1`.
- UI borders, strong focus rings, and graphical status indicators achieve `>= 3.0:1`.

## 4. `0 Hz` Settled Idle Rule

At rest (`animations_enabled == false`, `demo_running == false`) and when the browser tab is hidden (`visibilityState == "hidden"`), the cockpit consumes `0 Hz` continuous rendering: zero idle `requestAnimationFrame` loops, zero active `EventSource` or timers while hidden, and `prefers-reduced-motion: reduce` disables transitions.

## 5. WASM Bundle Size Budget (`<= 256_000 B`)

The stripped/optimized `wasm32-unknown-unknown` `--features hydrate` Islands binary of `cockpit-leptos` MUST NOT exceed `256_000` bytes (`250 KiB`), measured directly by `jacquard product budget --product cockpit` and tracked in `budgets.toml`. Layout (`iso-diagram`), telemetry (`telemetry-scene`), and wiki (`cockpit-wiki`) crates remain strictly SSR-only.

## 6. Clean Open-Source Fixtures & Zero Private Data

Built-in fixtures represent generic `example-cluster` environments (`fd00:db8::/64`, `example-node-01`, `wasmtime://`) and embedded Markdown documentation/wiki articles (`cockpit-wiki`). Zero personal hostnames, private cloud project names, or client metrics may appear in fixtures or views.

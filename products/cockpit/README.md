# Jacquard Cockpit (`products/cockpit`)

Pure-Rust **Hallmark x Leptos `0.8.23` (`tachys`)** fine-grained `--features hydrate` Islands/SSR telemetry cockpit, 2.5D isometric schematic engine, and Markdown wiki/blog engine. Licensed under Apache-2.0.

See [CONTRACT.md](CONTRACT.md) and [budgets.toml](budgets.toml) for the product acceptance budgets enforced by Jacquard Checks.

## Architecture & Crates

| Crate | Purpose | Key Exports |
| --- | --- | --- |
| [`crates/iso-diagram`](crates/iso-diagram/src/lib.rs) | Zero-dep declarative 2.5D isometric infrastructure diagramming library + `iso-render` SVG CLI | `iso_diagram!`, `Diagram::builder`, 2-pass Sugiyama + virtual nodes + explicit `(col, row)` placement hints, Hanan-grid orthogonal A* conduit router, topological 2.5D painter's sort, lightweight `SceneGeometry`, 12 procedural 2.5D sprites, `svg::render_svg` |
| [`crates/telemetry-scene`](crates/telemetry-scene/src/lib.rs) | Generic telemetry JSON-to-2.5D scene projector (zero runtime CLI or async IO dependencies) | `ClusterTopology`, `TelemetryBundle`, `cluster_to_diagram`, `telemetry_to_diagram`, `example_cluster_topology`, `example_cluster_bundle` |
| [`crates/cockpit-wiki`](crates/cockpit-wiki/src/lib.rs) | Pure-Rust Markdown + native `iso-diagram` Mermaid flowchart-to-SVG + BM25 search + syntax highlighting + strict scheme allowlist Wiki & Blog engine | `WikiCatalog`, `SearchIndex`, `highlight_code`, `render_mermaid_block`, `parse_document`, `render_markdown`, `render_rss`, `render_atom`, `validate_safe_rel_path` |
| [`crates/cockpit-leptos`](crates/cockpit-leptos/src/lib.rs) | Hallmark CSS (`tokens.css`, `layout.css`, `components.css`) x Leptos `0.8.23` fine-grained `--features hydrate` Islands workspace | `StageOrbitIsland`, `InspectorStreamIsland`, `CommandPaletteIsland`, `App`, `render_ssr_shell_html` (`<HydrationScripts options islands=true/>`), `verify_tokens_wcag_aa`, `bundled_stylesheet` |

## Hallmark Engineering Guarantees

- **CSS Cascade & Modern Primitives:** `@layer` cascade ordering (`tokens.css`, `layout.css`, `components.css`), `@scope (.app-surface)` and `@scope (.wiki-prose)`, `light-dark()` OKLCH semantic tokens across `Atelier-Draft` (light) and `Oxide-Monolith` (dark), `forced-colors` fallbacks, CSS `subgrid`, `@container` queries, Popover API (`popover="auto"` + `position-try-fallbacks`), and `<dialog closedby="any">`.
- **WCAG 2.2 AA Contrast Floor:** [`verify_tokens_wcag_aa`](crates/cockpit-leptos/src/wcag.rs) converts every critical OKLCH foreground/background token pair to relative luminance and asserts `>= 4.5:1` for text and `>= 3.0:1` for UI focus/component borders (`18/18` pairs + negative control).
- **`0 Hz` Settled Idle Rule:** The visibility-gated SSE bridge ([`sse_bridge.rs`](crates/cockpit-leptos/src/sse_bridge.rs)) halts active polling/heartbeats when settled (`settled_idle_hz = 0`) and when the tab is hidden (`hidden_tab_poll_hz = 0`).
- **WASM Byte Budget (`<= 250 KiB`):** `cockpit-leptos` compiled with `--no-default-features --features hydrate --target wasm32-unknown-unknown` and optimized with `wasm-bindgen` + `wasm-opt -Oz` MUST stay `<= 256,000 B` (`250 KiB`; observed `cockpit_leptos_bg.wasm`: `239,025 B` — a `71%` reduction from monolithic CSR). `iso-diagram`, `telemetry-scene`, and `cockpit-wiki` are strictly SSR-only.
- **Zero Unsafe / Clean Public Fixtures:** `#![forbid(unsafe_code)]` across `iso-diagram`, `telemetry-scene`, `cockpit-wiki`, and non-WASM `cockpit-leptos` targets (`#![cfg_attr(not(target_arch = "wasm32"), forbid(unsafe_code))]` for WASM bindings). Generic `example-cluster` (`fd00:db8::/64`) fixtures only.

## Verification & Usage

Run from `products/cockpit` (requires Rust `1.100.0` with `wasm32-unknown-unknown` via `rust-toolchain.toml`):

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo check -p cockpit-leptos --no-default-features --features hydrate --target wasm32-unknown-unknown --locked
```

Run the Jacquard WASM byte budget check from the repository root (`wasm-bindgen` `0.2.130` + `wasm-opt`):

```sh
cargo run --quiet --package jacquard --locked -- product budget --product cockpit
```

Render any standalone 2.5D schematic SVG (`campus-ha`, `payment-gateway`, or `asset-gallery` in `paper` or `obsidian` theme):

```sh
cargo run --manifest-path products/cockpit/Cargo.toml --locked -p iso-diagram --bin iso-render -- --scene campus-ha --theme paper campus-ha.svg
```

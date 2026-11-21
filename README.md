# Jacquard

Jacquard is a UI factory. It takes design intent, weaves candidate interfaces, and accepts them on executable evidence: rendering, budgets, accessibility, layout probes, and blind pairwise comparison. A language model proposes; the repository verifies. No model's verdict is a gate.

The name is the loom. Punched cards told it which threads to lift, and one machine could weave any pattern. Here the card is a Draft, the cloth is a Swatch, and the inspection is a cascade of checks that gets more expensive only for candidates that survive the cheap ones.

The reference Product in this repository is `products/cockpit`, a pure-Rust Hallmark × Leptos `0.8.23` telemetry cockpit with `iso-diagram` 2.5D schematics and a Markdown wiki/blog engine. Jacquard follows the same engineering model as [Nomos](https://github.com/kmosoti/Nomos) and [FabricO11y](https://github.com/kmosoti/FabricO11y): a product contract, numbered decisions, a pure core behind ports, gates with negative controls, receipts for what ran, and counterexamples kept as fixtures.

Start with [docs/CURRENT.md](docs/CURRENT.md), then the [product contract](docs/PRODUCT-CONTRACT.md) and the [architecture](docs/architecture/README.md). Contributors read [CONTRIBUTING.md](CONTRIBUTING.md); agents read [AGENTS.md](AGENTS.md); the two agree.

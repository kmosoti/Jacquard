//! Built-in reference Markdown & Mermaid wiki/blog articles compiled via `cockpit-wiki`.

use cockpit_wiki::{SiteFeedConfig, WikiCatalog, render_atom, render_rss};

const ARCH_DOC: &str = "\
+++
title = \"Hallmark x Leptos 0.8 Islands Architecture\"
slug = \"cockpit-architecture\"
kind = \"wiki\"
published_at = \"2026-11-21\"
summary = \"Pure-Rust 2.5D isometric projection paired with Leptos 0.8 Islands and Hallmark CSS tokens.\"
tags = [\"rust\", \"leptos\", \"iso-diagram\"]
evidence_class = \"Architecture\"
+++

# Hallmark x Leptos 0.8 Islands Architecture

Jacquard Cockpit separates static prose and documentation markup from interactive island hydration.

```mermaid
flowchart LR
  Wiki[cockpit-wiki Markdown] --> SSR[Leptos 0.8 SSR HTML]
  Telemetry[telemetry-scene JSON] --> Iso[iso-diagram 2.5D Layout]
  Iso --> Island[CockpitWorkspace Island]
```

## Verification Gates

- Contrast floor verified against resolved OKLCH tokens (`>= 4.5:1` text, `>= 3.0:1` UI).
- Settled idle rule maintains `0 Hz` timers and `EventSource` teardown on hidden tabs.
- Stripped `wasm32-unknown-unknown` binary stays under `900 KiB`.

See also the [Alpha Release Log](/blog/alpha-release-log).
";

const RELEASE_POST: &str = "\
+++
title = \"Stage A Public Cockpit Alpha Release Log\"
slug = \"alpha-release-log\"
kind = \"blog\"
published_at = \"2026-11-21\"
summary = \"Replacing JavaScript bundlers with pure-Rust iso-diagram SVG, telemetry-scene, and cockpit-wiki.\"
tags = [\"release\", \"jacquard\", \"cockpit\"]
evidence_class = \"Released Alpha\"
+++

# Stage A Public Cockpit Alpha Release Log

All interactive 2.5D topologies render directly in Rust without Node.js or external WebGL engines.

| Crate | Purpose | Target Support |
| --- | --- | --- |
| `iso-diagram` | Pure 2.5D sprites & ortho routing | Native + WASM |
| `telemetry-scene` | JSON telemetry-to-diagram adapter | Native + WASM |
| `cockpit-wiki` | Markdown, Mermaid & RSS/Atom engine | Native + WASM |
| `cockpit-leptos` | Hallmark x Leptos 0.8 Islands UI | SSR + Hydrate + CSR |

Read the [Architecture Note](/wiki/cockpit-architecture) for details.
";

/// Compiles the built-in Cockpit documentation catalog cleanly via `cockpit-wiki`.
pub fn builtin_wiki_catalog() -> WikiCatalog {
    WikiCatalog::from_embedded(&[
        ("wiki/cockpit-architecture.md", ARCH_DOC),
        ("blog/alpha-release-log.md", RELEASE_POST),
    ])
    .expect("embedded cockpit documentation catalog must always parse")
}

/// Renders an RSS 2.0 feed for the built-in documentation blog entries.
pub fn builtin_rss_feed() -> String {
    let catalog = builtin_wiki_catalog();
    let cfg = SiteFeedConfig {
        site_title: "Jacquard Cockpit Docs".into(),
        site_url: "https://example.org".into(),
        description: "Pure-Rust UI factory cockpit architecture and release logs".into(),
        author_name: "Jacquard Contributors".into(),
        language: "en-us".into(),
    };
    let entries = catalog.list(None);
    render_rss(&entries, &cfg, None)
}

/// Renders an Atom 1.0 feed for the built-in documentation entries.
pub fn builtin_atom_feed() -> String {
    let catalog = builtin_wiki_catalog();
    let cfg = SiteFeedConfig {
        site_title: "Jacquard Cockpit Docs".into(),
        site_url: "https://example.org".into(),
        description: "Pure-Rust UI factory cockpit architecture and release logs".into(),
        author_name: "Jacquard Contributors".into(),
        language: "en-us".into(),
    };
    let entries = catalog.list(None);
    render_atom(&entries, &cfg, None)
}

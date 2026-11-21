#![cfg_attr(not(target_arch = "wasm32"), forbid(unsafe_code))]
//! Hallmark x Leptos `0.8.23` (`tachys`) Islands/SSR telemetry cockpit.
//!
//! Renders semantic HTML/SVG with zero JS/Three.js bundlers: SSR compiles
//! layouts through `iso-diagram`, `telemetry-scene`, and `cockpit-wiki`, while
//! `wasm32` (`--features hydrate`) hydrates only three lean `#[island]`s
//! (`StageOrbitIsland`, `InspectorStreamIsland`, `CommandPaletteIsland`).

#[cfg(feature = "ssr")]
pub mod api;
#[cfg(feature = "ssr")]
pub mod domain;
#[cfg(feature = "ssr")]
pub mod dto;
#[cfg(feature = "ssr")]
pub mod sse_bridge;
#[cfg(feature = "ssr")]
pub mod state;
pub mod views;
#[cfg(feature = "ssr")]
pub mod wcag;
#[cfg(feature = "wiki")]
pub mod wiki_fixtures;

#[cfg(feature = "ssr")]
use crate::state::CockpitStore;
#[cfg(feature = "ssr")]
use leptos::config::LeptosOptions;
#[cfg(feature = "ssr")]
use leptos::hydration::HydrationScripts;
#[cfg(feature = "ssr")]
use leptos::prelude::*;

/// Hallmark Tier-1/Tier-2 design token & cascade layer CSS (`tokens.css`).
pub const TOKENS_CSS: &str = include_str!("../style/tokens.css");

/// Hallmark 3-column spatial layout CSS with subgrid and container queries (`layout.css`).
pub const LAYOUT_CSS: &str = include_str!("../style/layout.css");

/// Hallmark scoped component, popover, `<dialog>`, and wiki prose CSS (`components.css`).
pub const COMPONENTS_CSS: &str = include_str!("../style/components.css");

/// Concatenates the three Hallmark stylesheets in exact cascade order.
pub fn bundled_stylesheet() -> String {
    format!("{TOKENS_CSS}\n{LAYOUT_CSS}\n{COMPONENTS_CSS}")
}

/// Top-level SSR application entrypoint composing the static shell and three `#[island]`s.
#[cfg(feature = "ssr")]
#[component]
pub fn App() -> impl IntoView {
    let store = CockpitStore::new_with_defaults();
    provide_context(store);
    view! { <views::CockpitShell /> }
}

/// Renders the complete SSR HTML document with `<HydrationScripts options islands=true/>`
/// for SSR serving and automated `hallmark_sloplint` checks.
#[cfg(feature = "ssr")]
pub fn render_ssr_shell_html() -> String {
    let options = LeptosOptions::builder()
        .output_name("cockpit_leptos")
        .site_pkg_dir("pkg")
        .build();
    let owner = Owner::new_root(Some(std::sync::Arc::new(
        hydration_context::SsrSharedContext::new(),
    )));
    owner.with(|| {
        view! {
            <!DOCTYPE html>
            <html lang="en">
                <head>
                    <meta charset="utf-8" />
                    <meta name="viewport" content="width=device-width, initial-scale=1" />
                    <meta name="color-scheme" content="light dark" />
                    <title>"Jacquard Cockpit · Telemetry & Documentation"</title>
                    <link rel="stylesheet" href="/app.css" />
                    <HydrationScripts options=options islands=true />
                </head>
                <body>
                    <div id="cockpit-root">
                        <App />
                    </div>
                </body>
            </html>
        }
        .to_html()
    })
}

/// Browser WASM entry point: hydrates fine-grained Leptos `0.8.23` Islands in place.
#[cfg(all(target_arch = "wasm32", feature = "hydrate"))]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn hydrate_islands_entry() {
    leptos::mount::hydrate_islands();
}

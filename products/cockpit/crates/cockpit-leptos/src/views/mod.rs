//! Hallmark `CockpitShell` SSR-only layout composing static chrome (`<header>`,
//! `<nav>`, `<h1>`, wiki/service panels) with three fine-grained Leptos `0.8.23`
//! `#[island]` components (`StageOrbitIsland`, `InspectorStreamIsland`,
//! `CommandPaletteIsland`).

#[cfg(feature = "ssr")]
pub mod app_views;
#[cfg(feature = "ssr")]
pub mod app_views_ops;
pub mod helpers;
pub mod island_bus;
pub mod island_inspector;
pub mod island_palette;
pub mod island_stage;
pub mod iso_client_math;
pub mod iso_client_scene;
#[cfg(feature = "ssr")]
pub mod rail_left;
#[cfg(feature = "ssr")]
pub mod ssr_stage_svg;
mod stage_dom;
#[cfg(feature = "ssr")]
pub mod stage_iso_model;
#[cfg(feature = "ssr")]
pub mod wiki_view;

#[cfg(feature = "ssr")]
use crate::state::CockpitStore;
#[cfg(feature = "ssr")]
use crate::views::app_views::ServiceAppView;
#[cfg(feature = "ssr")]
use crate::views::helpers::format_cents;
#[cfg(feature = "ssr")]
use crate::views::island_inspector::InspectorStreamIsland;
#[cfg(feature = "ssr")]
use crate::views::island_palette::CommandPaletteIsland;
#[cfg(feature = "ssr")]
use crate::views::island_stage::StageOrbitIsland;
#[cfg(feature = "ssr")]
use crate::views::rail_left::LeftRail;
#[cfg(feature = "ssr")]
use crate::views::ssr_stage_svg::{SsrAssetsDrawer, SsrStageSvg};
#[cfg(feature = "ssr")]
use crate::views::stage_iso_model::{compile_stage_scene, encode_palette_seeds};
#[cfg(feature = "ssr")]
use leptos::prelude::*;

/// Root Hallmark 3-column SSR-only Cockpit shell composing three fine-grained `#[island]`s.
#[cfg(feature = "ssr")]
#[component]
pub fn CockpitShell() -> impl IntoView {
    let store = expect_context::<CockpitStore>();
    let env_label = store.active_env.get_untracked().label().to_owned();
    let env_badge = store.active_env.get_untracked().badge();
    let svc_label = store.active_service.get_untracked().label();
    let targets = store.filtered_targets();
    let healthy = store.healthy_count();
    let total = targets.len();
    let (monthly, budget) = store
        .snapshot
        .with_untracked(|s| (s.total_monthly_cents, s.budget_limit_cents));
    let poll_ms = store.poll_interval_ms();
    let density = store.density.get_untracked();
    let active_tab = store.active_tab.get_untracked();

    let (stage_scene, stage_geom) =
        compile_stage_scene(&targets, &format!("{env_label} · {svc_label}"));
    let palette_data = encode_palette_seeds(&targets);
    let svg_targets = targets.clone();

    view! {
        <a href="#stage-heading" class="skip-link">"Skip to main stage"</a>
        <div class="cockpit-shell" data-density=density>
            <header class="cockpit-topbar">
                <div class="topbar-brand">
                    <span class="brand-mark">
                        <svg role="img" viewBox="0 0 16 16" width="14" height="14">
                            <title>"Jacquard Cockpit topology emblem"</title>
                            <polygon points="8,1 14.5,4.7 14.5,11.3 8,15 1.5,11.3 1.5,4.7" fill="none" stroke="currentColor" stroke-width="1.5" />
                            <polygon points="8,4.2 11.5,6.2 8,8.2 4.5,6.2" fill="currentColor" />
                        </svg>
                        <span>"Jacquard Cockpit"</span>
                    </span>
                    <span class="kbd-hint">{env_badge}</span>
                    <span class="brand-sub">{env_label.clone()}</span>
                </div>

                <div class="topbar-telemetry">
                    <span class="metric-value muted">{format!("{healthy}/{total} healthy")}</span>
                    <span class="spend muted">
                        {format!("{} / {}", format_cents(monthly), format_cents(budget))}
                    </span>
                </div>

                <div id="palette-seed-data" class="topbar-actions" data-items=palette_data>
                    <CommandPaletteIsland />
                </div>
            </header>

            <LeftRail />

            <main class="stage-pane">
                <div class="stage-header">
                    <h1 id="stage-heading" tabindex="-1" class="view-heading">
                        {format!("{env_label} · {svc_label}")}
                    </h1>
                    <div class="stage-toolbar">
                        <span class="muted num">{format!("poll {poll_ms}ms")}</span>
                    </div>
                </div>

                <div class="stage-body">
                    <div class="stage-panel">
                        <StageOrbitIsland>
                            <SsrStageSvg scene=stage_scene geom=stage_geom targets=svg_targets />
                        </StageOrbitIsland>
                    </div>
                    {active_tab.map(|svc| view! { <ServiceAppView svc=svc /> })}
                </div>
            </main>

            <aside class="inspector-rail" aria-label="Target inspector and assets">
                <InspectorStreamIsland>
                    <SsrAssetsDrawer targets=targets />
                </InspectorStreamIsland>
            </aside>
        </div>
    }
}

//! Lightweight cross-island reactive signal bus (`selected_urn`, `stream_connected`).
//!
//! All three hydrated Leptos islands (`StageOrbitIsland`, `InspectorStreamIsland`,
//! `CommandPaletteIsland`) share a single `wasm32` module instance, so a tiny
//! thread-local signal pair coordinates selection without pulling the SSR store
//! into the client bundle.

use std::cell::RefCell;

use leptos::prelude::*;

/// Shared selection and stream status signals across hydrated islands.
#[derive(Clone, Copy)]
pub struct IslandBus {
    /// Currently selected cluster target URN.
    pub selected_urn: RwSignal<Option<String>>,
    /// Live SSE connection indicator (`false` at `0 Hz` settled idle).
    pub stream_connected: RwSignal<bool>,
}

thread_local! {
    static BUS: RefCell<Option<IslandBus>> = const { RefCell::new(None) };
}

/// Returns the shared [`IslandBus`], initializing it lazily inside the active owner.
#[must_use]
pub fn use_island_bus(default_urn: Option<String>) -> IslandBus {
    #[cfg(target_arch = "wasm32")]
    {
        BUS.with(|cell| {
            let mut slot = cell.borrow_mut();
            if let Some(existing) = *slot {
                if existing.selected_urn.get_untracked().is_none() && default_urn.is_some() {
                    existing.selected_urn.set(default_urn);
                }
                existing
            } else {
                let bus = IslandBus {
                    selected_urn: RwSignal::new(default_urn),
                    stream_connected: RwSignal::new(false),
                };
                *slot = Some(bus);
                bus
            }
        })
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        IslandBus {
            selected_urn: RwSignal::new(default_urn),
            stream_connected: RwSignal::new(false),
        }
    }
}

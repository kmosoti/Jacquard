//! Fine-grained Leptos `0.8.23` `#[island]` for the `Ctrl+K` native `<dialog closedby="any">`
//! fuzzy command palette (`role="combobox"` + `role="listbox"`, `0 Hz` closed-state gating,
//! zero `serde_json` in `wasm32`).

use leptos::prelude::*;

use crate::views::helpers::{close_dialog_by_id, handle_dialog_backdrop_click, open_dialog_by_id};
use crate::views::island_bus::use_island_bus;
#[cfg(target_arch = "wasm32")]
use crate::views::stage_dom::read_dom_attr;

/// DOM element ID for the command palette `<dialog>`.
pub const PALETTE_DIALOG_ID: &str = "command-palette";

/// Compact palette seed `(title, category, target_urn_or_empty)`.
pub type PaletteSeed = (String, String, String);

#[cfg(target_arch = "wasm32")]
fn decode_seeds(raw: &str) -> Vec<PaletteSeed> {
    raw.split('\n')
        .filter(|l| !l.is_empty())
        .filter_map(|line| {
            let mut it = line.split('\t');
            Some((
                it.next()?.to_owned(),
                it.next()?.to_owned(),
                it.next().unwrap_or("").to_owned(),
            ))
        })
        .collect()
}

fn fuzzy_score(haystack: &str, needle: &str) -> Option<u32> {
    if needle.is_empty() {
        return Some(1);
    }
    let h = haystack.to_ascii_lowercase();
    if let Some(pos) = h.find(needle) {
        return Some(200u32.saturating_sub(pos as u32));
    }
    let mut n_chars = needle.chars().peekable();
    let (mut score, mut streak) = (0u32, 0u32);
    for ch in h.chars() {
        if n_chars.peek() == Some(&ch) {
            n_chars.next();
            streak += 1;
            score += 4 + streak * 2;
        } else {
            streak = 0;
        }
    }
    n_chars.peek().is_none().then_some(score)
}

/// Interactive `Ctrl+K` fuzzy command palette island (`#[island]`).
#[island]
pub(crate) fn CommandPaletteIsland() -> impl IntoView {
    let bus = use_island_bus(None);
    let open = RwSignal::new(false);
    let query = RwSignal::new(String::new());
    let active_idx = RwSignal::new(0usize);
    #[cfg(target_arch = "wasm32")]
    let initial_items = decode_seeds(&read_dom_attr("palette-seed-data", "data-items"));
    #[cfg(not(target_arch = "wasm32"))]
    let initial_items: Vec<PaletteSeed> = Vec::new();
    let seed = StoredValue::new(initial_items);
    attach_ctrl_k(open);

    let filtered = move || {
        if !open.get() {
            return Vec::new();
        }
        let q = query.get().trim().to_ascii_lowercase();
        seed.with_value(|all| {
            let mut scored: Vec<(u32, PaletteSeed)> = all
                .iter()
                .filter_map(|item| {
                    let hay = format!("{} {}", item.1, item.0);
                    fuzzy_score(&hay, &q).map(|sc| (sc, item.clone()))
                })
                .collect();
            if !q.is_empty() {
                for i in 1..scored.len() {
                    let mut j = i;
                    while j > 0 && scored[j].0 > scored[j - 1].0 {
                        scored.swap(j, j - 1);
                        j -= 1;
                    }
                }
            }
            scored.into_iter().take(18).map(|(_, e)| e).collect()
        })
    };

    let activate = move |target_urn: String| {
        if !target_urn.is_empty() {
            bus.selected_urn.set(Some(target_urn));
        }
        close_dialog_by_id(PALETTE_DIALOG_ID);
        open.set(false);
    };

    let on_input_keydown = move |ev: leptos::ev::KeyboardEvent| {
        let list = filtered();
        if list.is_empty() {
            return;
        }
        match ev.key().as_str() {
            "ArrowDown" => {
                ev.prevent_default();
                active_idx.update(|i| *i = (*i + 1) % list.len());
            }
            "ArrowUp" => {
                ev.prevent_default();
                active_idx.update(|i| *i = (*i + list.len() - 1) % list.len());
            }
            "Enter" => {
                ev.prevent_default();
                let idx = active_idx.get_untracked().min(list.len() - 1);
                activate(list[idx].2.clone());
            }
            _ => {}
        }
    };

    view! {
        <button
            type="button"
            class="btn palette-launcher"
            aria-label="Open Command Palette"
            on:click=move |_| {
                open.set(true);
                open_dialog_by_id(PALETTE_DIALOG_ID);
            }
        >
            <span>"Commands"</span>
            <kbd class="kbd-hint">"Ctrl+K"</kbd>
        </button>

        <dialog
            id=PALETTE_DIALOG_ID
            class="cockpit-dialog"
            closedby="any"
            aria-labelledby="palette-title"
            on:close=move |_| {
                open.set(false);
                query.set(String::new());
                active_idx.set(0);
            }
            on:click=move |ev| handle_dialog_backdrop_click(ev, PALETTE_DIALOG_ID)
        >
            <div class="dialog-header">
                <h2 id="palette-title">"Command Palette"</h2>
                <button
                    type="button"
                    class="btn quiet"
                    on:click=move |_| close_dialog_by_id(PALETTE_DIALOG_ID)
                >
                    "Close"
                </button>
            </div>
            <div class="dialog-body">
                <search>
                    <label for="palette-search" class="visually-hidden">
                        "Filter environments, services, commands, and cluster assets"
                    </label>
                    <input
                        id="palette-search"
                        type="search"
                        class="palette-input"
                        autofocus
                        role="combobox"
                        aria-expanded=move || if open.get() { "true" } else { "false" }
                        aria-controls="palette-listbox"
                        aria-autocomplete="list"
                        placeholder="Fuzzy jump to asset, wiki page, or command..."
                        prop:value=move || query.get()
                        on:input=move |ev| {
                            query.set(event_target_value(&ev));
                            active_idx.set(0);
                        }
                        on:keydown=on_input_keydown
                    />
                </search>
                <ul id="palette-listbox" class="palette-list" role="listbox" aria-label="Command results">
                    {move || {
                        filtered()
                            .into_iter()
                            .enumerate()
                            .map(|(idx, (title, cat, urn))| {
                                view! {
                                    <li role="none">
                                        <button
                                            type="button"
                                            role="option"
                                            aria-selected=move || if active_idx.get() == idx { "true" } else { "false" }
                                            class="btn palette-item"
                                            on:click=move |_| activate(urn.clone())
                                        >
                                            <span>{title}</span>
                                            <span class="kbd-hint">{cat}</span>
                                        </button>
                                    </li>
                                }
                            })
                            .collect_view()
                    }}
                </ul>
            </div>
        </dialog>
    }
}

fn attach_ctrl_k(open: RwSignal<bool>) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::{JsCast, closure::Closure};
        if let Some(win) = web_sys::window() {
            let cb = Closure::<dyn Fn(web_sys::KeyboardEvent)>::new(
                move |ev: web_sys::KeyboardEvent| {
                    if !ev.repeat()
                        && (ev.ctrl_key() || ev.meta_key())
                        && ev.key().eq_ignore_ascii_case("k")
                    {
                        ev.prevent_default();
                        open.set(true);
                        open_dialog_by_id(PALETTE_DIALOG_ID);
                    }
                },
            );
            let _ = win.add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
            cb.forget();
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = open;
    }
}

//! Fine-grained Leptos `0.8.23` `#[island]` for the right Target Inspector and
//! `0 Hz` visibility-gated SSE status bridge (zero `serde_json` in `wasm32`).

use leptos::prelude::*;

use crate::views::helpers::{
    close_dialog_by_id, format_cents, handle_dialog_backdrop_click, open_dialog_by_id, status_tone,
};
use crate::views::island_bus::use_island_bus;
use crate::views::stage_dom::clicked_urn;
#[cfg(target_arch = "wasm32")]
use crate::views::stage_dom::read_dom_attr;

const RECONCILE_DIALOG_ID: &str = "reconcile-confirm-dialog";

/// Compact target tuple `(urn, title, group, kind, status, monthly_cents)`.
pub type InspectorRow = (String, String, String, String, String, u32);

#[cfg(target_arch = "wasm32")]
fn decode_rows(raw: &str) -> Vec<InspectorRow> {
    raw.split('\n')
        .filter(|l| !l.is_empty())
        .filter_map(|line| {
            let mut it = line.split('\t');
            Some((
                it.next()?.to_owned(),
                it.next()?.to_owned(),
                it.next()?.to_owned(),
                it.next()?.to_owned(),
                it.next()?.to_owned(),
                it.next()?.parse().ok()?,
            ))
        })
        .collect()
}

/// Interactive target inspector & SSE stream island (`#[island]`).
#[island]
pub(crate) fn InspectorStreamIsland(
    /// Server-rendered `<details class="assets-drawer">` inventory with `data-rows` metadata.
    children: Children,
) -> impl IntoView {
    #[cfg(target_arch = "wasm32")]
    let initial_rows = decode_rows(&read_dom_attr("inspector-assets-wrap", "data-rows"));
    #[cfg(not(target_arch = "wasm32"))]
    let initial_rows: Vec<InspectorRow> = Vec::new();

    let default_urn = initial_rows.first().map(|i| i.0.clone());
    let bus = use_island_bus(default_urn);
    attach_visibility_sse(bus.stream_connected);

    let items_sig = RwSignal::new(initial_rows);
    let action_note = RwSignal::new(None::<String>);

    let selected = move || {
        let sel = bus.selected_urn.get();
        items_sig.with(|list| {
            sel.as_deref()
                .and_then(|u| list.iter().find(|i| i.0 == u))
                .or_else(|| list.first())
                .cloned()
        })
    };

    view! {
        <section class="inspector-block">
            <div class="rail-item">
                <h2 class="rail-heading">"TARGET INSPECTOR"</h2>
                <span
                    class="status-pill"
                    data-tone=move || if bus.stream_connected.get() { "ok" } else { "info" }
                >
                    {move || if bus.stream_connected.get() { "SSE Live" } else { "SSE Idle" }}
                </span>
            </div>
            {move || action_note.get().map(|msg| view! {
                <p class="status-banner" role="status" aria-live="polite">{msg}</p>
            })}
            <div class="inspector-block">
                <div class="rail-item">
                    <h3>{move || selected().map_or_else(|| "ctrl-01".to_owned(), |t| t.1)}</h3>
                    <span
                        class="status-pill"
                        data-tone=move || selected().map_or("ok", |t| status_tone(&t.4))
                    >
                        {move || selected().map_or_else(|| "Converged".to_owned(), |t| t.4)}
                    </span>
                </div>
                <dl class="inspector-meta">
                    <dt>"URN"</dt>
                    <dd class="mono">{move || selected().map_or_else(|| "cluster.node/ctrl-01".to_owned(), |t| t.0)}</dd>
                    <dt>"Group"</dt>
                    <dd>{move || selected().map_or_else(|| "Compute".to_owned(), |t| t.2)}</dd>
                    <dt>"Kind"</dt>
                    <dd>{move || selected().map_or_else(|| "ComputeInstance".to_owned(), |t| t.3)}</dd>
                    <dt>"Monthly"</dt>
                    <dd class="spend">{move || selected().map_or_else(|| "$0.00".to_owned(), |t| format_cents(t.5))}</dd>
                </dl>
                <div class="inspector-actions">
                    <button
                        type="button"
                        class="btn"
                        on:click=move |_| {
                            let urn = selected().map_or_else(|| "cluster".to_owned(), |t| t.0);
                            action_note.set(Some(format!("Diff clean for {urn}")));
                        }
                    >
                        "Diff"
                    </button>
                    <button
                        type="button"
                        class="btn primary"
                        on:click=move |_| open_dialog_by_id(RECONCILE_DIALOG_ID)
                    >
                        "Reconcile"
                    </button>
                </div>
            </div>
        </section>

        <div on:click=move |ev: leptos::ev::MouseEvent| {
            if let Some(urn) = clicked_urn(&ev) {
                bus.selected_urn.set(Some(urn));
            }
        }>
            {children()}
        </div>

        <dialog
            id=RECONCILE_DIALOG_ID
            class="cockpit-dialog"
            closedby="any"
            aria-labelledby="reconcile-dialog-title"
            on:click=move |ev| handle_dialog_backdrop_click(ev, RECONCILE_DIALOG_ID)
        >
            <div class="dialog-header">
                <h2 id="reconcile-dialog-title">"Confirm Reconcile"</h2>
                <button type="button" class="btn quiet" on:click=move |_| close_dialog_by_id(RECONCILE_DIALOG_ID)>"Close"</button>
            </div>
            <div class="dialog-body">
                <p>
                    {move || {
                        let urn = selected().map_or_else(|| "cluster".to_owned(), |t| t.0);
                        format!("Apply deterministic Merkle-DAG reconciliation to `{urn}`?")
                    }}
                </p>
            </div>
            <div class="dialog-footer">
                <button type="button" class="btn" autofocus on:click=move |_| close_dialog_by_id(RECONCILE_DIALOG_ID)>"Cancel"</button>
                <button
                    type="button"
                    class="btn primary"
                    on:click=move |_| {
                        if let Some((urn, ..)) = selected() {
                            items_sig.update(|list| {
                                if let Some(entry) = list.iter_mut().find(|i| i.0 == urn) {
                                    entry.4 = "Converged".to_owned();
                                }
                            });
                            action_note.set(Some(format!("Reconciled {urn}")));
                        }
                        close_dialog_by_id(RECONCILE_DIALOG_ID);
                    }
                >
                    "Confirm Reconcile"
                </button>
            </div>
        </dialog>
    }
}

fn attach_visibility_sse(stream_connected: RwSignal<bool>) {
    #[cfg(target_arch = "wasm32")]
    {
        use std::cell::RefCell;
        use wasm_bindgen::{JsCast, closure::Closure};
        use web_sys::{Event, EventSource, VisibilityState};

        thread_local! {
            static SSE: RefCell<Option<EventSource>> = const { RefCell::new(None) };
        }
        let open_es = move || {
            let hidden = web_sys::window()
                .and_then(|w| w.document())
                .is_some_and(|d| d.visibility_state() == VisibilityState::Hidden);
            if hidden {
                return;
            }
            if let Ok(es) = EventSource::new("/v1/events") {
                let on_open = Closure::once_into_js(move || stream_connected.set(true));
                let on_err = Closure::once_into_js(move || stream_connected.set(false));
                es.set_onopen(Some(on_open.unchecked_ref()));
                es.set_onerror(Some(on_err.unchecked_ref()));
                SSE.with(|c| *c.borrow_mut() = Some(es));
            }
        };
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            let on_vis = Closure::<dyn FnMut(Event)>::new(move |_: Event| {
                let hidden = web_sys::window()
                    .and_then(|w| w.document())
                    .is_some_and(|d| d.visibility_state() == VisibilityState::Hidden);
                if hidden {
                    SSE.with(|c| {
                        if let Some(es) = c.borrow_mut().take() {
                            es.close();
                        }
                    });
                    stream_connected.set(false);
                } else {
                    open_es();
                }
            });
            let _ = doc.add_event_listener_with_callback(
                "visibilitychange",
                on_vis.as_ref().unchecked_ref(),
            );
            on_vis.forget();
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = stream_connected;
    }
}

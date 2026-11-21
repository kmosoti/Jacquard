//! Shared Hallmark UI helpers: tabular currency formatting, paired geometric status glyphs,
//! native `<dialog>` helpers (with backdrop-dismiss fallback), focus routing,
//! and reduced-motion-safe View Transitions.

use leptos::prelude::*;

/// Formats integer cents into a crisp tabular USD string (e.g. `$14.20`).
pub fn format_cents(cents: u32) -> String {
    format!("${}.{:02}", cents / 100, cents % 100)
}

/// Formats bytes into GiB with one decimal place.
#[cfg(feature = "ssr")]
pub fn format_gib(bytes: u64) -> String {
    let gib = (bytes as f64) / (1024.0 * 1024.0 * 1024.0);
    format!("{gib:.1} GiB")
}

/// Maps target/service status strings into Hallmark semantic tone (`ok`, `info`, `warn`, `err`).
pub fn status_tone(status: &str) -> &'static str {
    match status {
        "Converged" | "Healthy" | "OK" | "Nominal" | "Live" => "ok",
        "InFlight" | "Active" | "INFO" | "Planned" | "Elevated" => "info",
        "Drifted" | "Degraded" | "WARN" | "Constrained" | "Idle" => "warn",
        "Poisoned" | "Failed" | "ERR" | "Critical" => "err",
        _ => "info",
    }
}

/// Maps target status string into normalized `data-state` token for O(1) SVG attribute styling.
pub fn status_data_state(status: &str) -> &'static str {
    match status {
        "Converged" | "Healthy" => "healthy",
        "InFlight" | "Active" => "active",
        "Drifted" | "Degraded" => "degraded",
        "Poisoned" | "Failed" => "failed",
        _ => "pending",
    }
}

/// Paired geometric SVG glyph + text status badge (WCAG 1.4.1: never color-alone, zero emojis).
#[component]
pub fn StatusBadge(
    /// Badge text label.
    #[prop(into)]
    label: Signal<String>,
    /// Semantic tone (`ok`, `info`, `warn`, `err`).
    #[prop(into)]
    tone: Signal<&'static str>,
) -> impl IntoView {
    use leptos::either::EitherOf4;
    view! {
        <span class="status-pill" data-tone=move || tone.get()>
            <svg
                class="status-glyph"
                viewBox="0 0 10 10"
                width="10"
                height="10"
                aria-hidden="true"
            >
                {move || match tone.get() {
                    "ok" => EitherOf4::A(view! {
                        <circle cx="5" cy="5" r="3.6" fill="currentColor" />
                    }),
                    "warn" => EitherOf4::B(view! {
                        <polygon points="5,1 9,8.6 1,8.6" fill="currentColor" />
                    }),
                    "err" => EitherOf4::C(view! {
                        <rect x="1.5" y="1.5" width="7" height="7" fill="currentColor" />
                    }),
                    _ => EitherOf4::D(view! {
                        <polygon points="5,1 9,5 5,9 1,5" fill="currentColor" />
                    }),
                }}
            </svg>
            <span>{move || label.get()}</span>
        </span>
    }
}

/// Static paired geometric SVG glyph + text status badge (zero reactive closures for list rows).
#[component]
pub fn StaticStatusBadge(
    /// Badge text label.
    label: String,
    /// Semantic tone (`ok`, `info`, `warn`, `err`).
    tone: &'static str,
) -> impl IntoView {
    use leptos::either::EitherOf4;
    let glyph = match tone {
        "ok" => EitherOf4::A(view! { <circle cx="5" cy="5" r="3.6" fill="currentColor" /> }),
        "warn" => EitherOf4::B(view! { <polygon points="5,1 9,8.6 1,8.6" fill="currentColor" /> }),
        "err" => EitherOf4::C(
            view! { <rect x="1.5" y="1.5" width="7" height="7" fill="currentColor" /> },
        ),
        _ => EitherOf4::D(view! { <polygon points="5,1 9,5 5,9 1,5" fill="currentColor" /> }),
    };
    view! {
        <span class="status-pill" data-tone=tone>
            <svg class="status-glyph" viewBox="0 0 10 10" width="10" height="10" aria-hidden="true">
                {glyph}
            </svg>
            <span>{label}</span>
        </span>
    }
}

/// Opens a native `<dialog>` element by DOM id via `.showModal()` on `wasm32`.
pub fn open_dialog_by_id(id: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        if let Some(el) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id(id))
            .and_then(|el| el.dyn_into::<web_sys::HtmlDialogElement>().ok())
            && !el.open()
        {
            let _ = el.show_modal();
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = id;
    }
}

/// Closes a native `<dialog>` element by DOM id via `.close()` on `wasm32`.
pub fn close_dialog_by_id(id: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        if let Some(el) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id(id))
            .and_then(|el| el.dyn_into::<web_sys::HtmlDialogElement>().ok())
            && el.open()
        {
            el.close();
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = id;
    }
}

/// Fallback light-dismiss for browsers lacking `closedby="any"`:
/// closes the dialog if the click landed directly on the `<dialog>` backdrop outside its box.
pub fn handle_dialog_backdrop_click(ev: leptos::ev::MouseEvent, dialog_id: &'static str) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        let Some(target) = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        else {
            return;
        };
        if target.id() != dialog_id {
            return;
        }
        let rect = target.get_bounding_client_rect();
        let x = f64::from(ev.client_x());
        let y = f64::from(ev.client_y());
        let outside = x < rect.left() || x > rect.right() || y < rect.top() || y > rect.bottom();
        if outside {
            close_dialog_by_id(dialog_id);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (ev, dialog_id);
    }
}

/// Programmatically moves keyboard focus to an element by DOM id (`tabindex="-1"`).
pub fn focus_by_id(id: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        if let Some(el) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id(id))
            .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok())
        {
            let _ = el.focus();
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = id;
    }
}

/// Executes a view switch inside `document.startViewTransition()` when supported and motion
/// is not reduced, flushing a microtask Promise before snapshot capture and routing focus
/// to `heading_id` (`tabindex="-1"`) after completion.
pub fn transition_view_and_focus(heading_id: &'static str, update: impl FnOnce() + 'static) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::{JsCast, JsValue, closure::Closure};

        let Some(win) = web_sys::window() else {
            update();
            return;
        };
        let Some(doc) = win.document() else {
            update();
            return;
        };
        if prefers_reduced_motion(&win) {
            update();
            focus_by_id(heading_id);
            return;
        }
        let Ok(func) = js_sys::Reflect::get(&doc, &JsValue::from_str("startViewTransition"))
            .and_then(|v| v.dyn_into::<js_sys::Function>())
        else {
            update();
            focus_by_id(heading_id);
            return;
        };
        let cb = Closure::once_into_js(move || -> JsValue {
            update();
            js_sys::Promise::resolve(&JsValue::UNDEFINED).into()
        });
        if let Ok(vt) = func.call1(&doc, &cb)
            && let Ok(finished) = js_sys::Reflect::get(&vt, &JsValue::from_str("finished"))
            && let Ok(finally_fn) = js_sys::Reflect::get(&finished, &JsValue::from_str("finally"))
                .and_then(|v| v.dyn_into::<js_sys::Function>())
        {
            let focus_cb = Closure::once_into_js(move || {
                focus_by_id(heading_id);
            });
            let _ = finally_fn.call1(&finished, &focus_cb);
        } else {
            focus_by_id(heading_id);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = heading_id;
        update();
    }
}

#[cfg(target_arch = "wasm32")]
fn prefers_reduced_motion(win: &web_sys::Window) -> bool {
    use wasm_bindgen::{JsCast, JsValue};
    let Ok(mm_fn) = js_sys::Reflect::get(win, &JsValue::from_str("matchMedia"))
        .and_then(|v| v.dyn_into::<js_sys::Function>())
    else {
        return false;
    };
    let Ok(mql) = mm_fn.call1(win, &JsValue::from_str("(prefers-reduced-motion: reduce)")) else {
        return false;
    };
    js_sys::Reflect::get(&mql, &JsValue::from_str("matches"))
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

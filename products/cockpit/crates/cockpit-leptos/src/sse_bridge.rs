//! Visibility-gated `/v1/events` SSE bridge + governor-paced fallback polling (`0 Hz` when hidden).

use crate::state::CockpitStore;

/// Attaches the browser SSE subscription and `visibilitychange` gate on `wasm32`.
/// No-op on non-`wasm32` targets (SSR / unit tests).
pub fn attach_sse_bridge(store: CockpitStore) {
    #[cfg(target_arch = "wasm32")]
    wasm_bridge::start(store);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = store;
}

#[cfg(target_arch = "wasm32")]
mod wasm_bridge {
    use crate::api::refresh_all;
    use crate::state::{CockpitStore, EnvKind, SseEventDto};
    use leptos::prelude::*;
    use std::cell::RefCell;
    use wasm_bindgen::JsCast;
    use wasm_bindgen::closure::Closure;
    use web_sys::{Document, Event, EventSource, MessageEvent, VisibilityState};

    struct BridgeRuntime {
        store: CockpitStore,
        source: Option<EventSource>,
        on_open: Option<Closure<dyn FnMut(Event)>>,
        on_message: Option<Closure<dyn FnMut(MessageEvent)>>,
        on_error: Option<Closure<dyn FnMut(Event)>>,
        on_visibility: Option<Closure<dyn FnMut(Event)>>,
        poll_timer_id: Option<i32>,
        on_poll: Option<Closure<dyn FnMut()>>,
    }

    impl BridgeRuntime {
        fn close_source(&mut self) {
            if let Some(es) = self.source.take() {
                es.set_onopen(None);
                es.set_onmessage(None);
                es.set_onerror(None);
                es.close();
            }
            self.on_open = None;
            self.on_message = None;
            self.on_error = None;
            self.store.stream_connected.set(false);
        }

        fn clear_poll_timer(&mut self) {
            if let Some(id) = self.poll_timer_id.take()
                && let Some(win) = web_sys::window()
            {
                win.clear_timeout_with_handle(id);
            }
            self.on_poll = None;
        }

        fn open_source(&mut self) {
            if self.source.is_some() || is_document_hidden() {
                return;
            }
            let Ok(es) = EventSource::new("/v1/events") else {
                self.store.stream_connected.set(false);
                return;
            };

            let store_open = self.store;
            let on_open = Closure::<dyn FnMut(Event)>::new(move |_: Event| {
                store_open.stream_connected.set(true);
            });
            es.set_onopen(Some(on_open.as_ref().unchecked_ref()));

            let store_msg = self.store;
            let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |evt: MessageEvent| {
                if store_msg.active_env.get_untracked() == EnvKind::Simulation {
                    return;
                }
                let Some(raw) = evt.data().as_string() else {
                    return;
                };
                if let Ok(parsed) = serde_json::from_str::<SseEventDto>(&raw) {
                    if let Some(tele) = parsed.telemetry {
                        store_msg.apply_telemetry(tele);
                    }
                    if let Some(gov) = parsed.governor {
                        store_msg.apply_governor(gov);
                    }
                }
            });
            es.set_onmessage(Some(on_message.as_ref().unchecked_ref()));

            let store_err = self.store;
            let on_error = Closure::<dyn FnMut(Event)>::new(move |_: Event| {
                store_err.stream_connected.set(false);
            });
            es.set_onerror(Some(on_error.as_ref().unchecked_ref()));

            self.source = Some(es);
            self.on_open = Some(on_open);
            self.on_message = Some(on_message);
            self.on_error = Some(on_error);
        }

        fn schedule_next_poll(&mut self) {
            self.clear_poll_timer();
            if is_document_hidden() {
                return;
            }
            let Some(win) = web_sys::window() else {
                return;
            };
            let delay_ms = self.store.poll_interval_ms().min(i32::MAX as u32) as i32;
            let store = self.store;
            let cb = Closure::<dyn FnMut()>::new(move || {
                if !is_document_hidden() && store.active_env.get_untracked() != EnvKind::Simulation
                {
                    leptos::task::spawn_local(async move {
                        let _ = refresh_all(store).await;
                    });
                }
                BRIDGE.with(|cell| {
                    if let Some(rt) = cell.borrow_mut().as_mut() {
                        rt.schedule_next_poll();
                    }
                });
            });
            if let Ok(id) = win.set_timeout_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                delay_ms,
            ) {
                self.poll_timer_id = Some(id);
                self.on_poll = Some(cb);
            }
        }

        fn teardown(&mut self) {
            self.close_source();
            self.clear_poll_timer();
            if let Some(cb) = self.on_visibility.take()
                && let Some(doc) = current_document()
            {
                let _ = doc.remove_event_listener_with_callback(
                    "visibilitychange",
                    cb.as_ref().unchecked_ref(),
                );
            }
        }
    }

    impl Drop for BridgeRuntime {
        fn drop(&mut self) {
            self.teardown();
        }
    }

    thread_local! {
        static BRIDGE: RefCell<Option<BridgeRuntime>> = const { RefCell::new(None) };
    }

    pub fn start(store: CockpitStore) {
        let mut rt = BridgeRuntime {
            store,
            source: None,
            on_open: None,
            on_message: None,
            on_error: None,
            on_visibility: None,
            poll_timer_id: None,
            on_poll: None,
        };

        if let Some(doc) = current_document() {
            let on_vis = Closure::<dyn FnMut(Event)>::new(move |_: Event| {
                if is_document_hidden() {
                    BRIDGE.with(|cell| {
                        if let Some(rt) = cell.borrow_mut().as_mut() {
                            rt.close_source();
                            rt.clear_poll_timer();
                        }
                    });
                } else {
                    BRIDGE.with(|cell| {
                        if let Some(rt) = cell.borrow_mut().as_mut() {
                            rt.open_source();
                            rt.schedule_next_poll();
                            let st = rt.store;
                            leptos::task::spawn_local(async move {
                                let _ = refresh_all(st).await;
                            });
                        }
                    });
                }
            });
            let _ = doc.add_event_listener_with_callback(
                "visibilitychange",
                on_vis.as_ref().unchecked_ref(),
            );
            rt.on_visibility = Some(on_vis);
        }

        if !is_document_hidden() {
            rt.open_source();
            rt.schedule_next_poll();
            leptos::task::spawn_local(async move {
                let _ = refresh_all(store).await;
            });
        }

        BRIDGE.with(|cell| {
            *cell.borrow_mut() = Some(rt);
        });

        on_cleanup(|| {
            BRIDGE.with(|cell| {
                let _ = cell.borrow_mut().take();
            });
        });
    }

    fn current_document() -> Option<Document> {
        web_sys::window().and_then(|w| w.document())
    }

    fn is_document_hidden() -> bool {
        current_document()
            .map(|d| d.visibility_state() == VisibilityState::Hidden)
            .unwrap_or(false)
    }
}

//! Semantic `<nav>` left rail: ENVIRONMENTS, SERVICES filter, and vertical APP TABS
//! with CSS `anchor-name: --active-rail-item` indicator and View Transition focus routing.

use crate::state::{CockpitStore, EnvKind, ServiceId};
use crate::views::helpers::{focus_by_id, transition_view_and_focus};
use leptos::prelude::*;

/// Left workspace navigation rail switching environments, app/wiki tabs, and stage filters.
#[component]
pub fn LeftRail() -> impl IntoView {
    let store = expect_context::<CockpitStore>();

    let service_counts = Memo::new(move |_| {
        store.snapshot.with(|snap| {
            ServiceId::STAGE_FILTERS.map(|svc| {
                snap.targets
                    .iter()
                    .filter(|t| svc.matches_target(t))
                    .count()
            })
        })
    });

    view! {
        <nav class="rail-left" aria-label="Workspace navigation">
            <section class="rail-section">
                <h2 class="rail-heading">"ENVIRONMENTS"</h2>
                <ul class="rail-list">
                    {EnvKind::ALL
                        .into_iter()
                        .map(|env| {
                            let is_active = move || store.active_env.get() == env;
                            view! {
                                <li class="rail-item" class:is-active=is_active>
                                    <button
                                        type="button"
                                        class="rail-btn"
                                        aria-pressed=move || if is_active() { "true" } else { "false" }
                                        on:click=move |_| store.switch_env(env)
                                    >
                                        <span>{env.label()}</span>
                                        <span class="kbd-hint">{env.badge()}</span>
                                    </button>
                                </li>
                            }
                        })
                        .collect_view()}
                </ul>
            </section>

            <section class="rail-section">
                <h2 class="rail-heading">"APP TABS"</h2>
                <ul class="rail-list rail-tabs-list">
                    <li
                        class="rail-item"
                        class:is-active=move || store.active_tab.get().is_none()
                    >
                        <button
                            type="button"
                            class="rail-btn"
                            aria-current=move || store.active_tab.get().is_none().then_some("page")
                            on:click=move |_| {
                                transition_view_and_focus("stage-heading", move || {
                                    store.active_tab.set(None);
                                });
                            }
                        >
                            <span>"Cluster Stage"</span>
                            <span class="rail-item-meta num">
                                {move || service_counts.get()[0]}
                            </span>
                        </button>
                    </li>

                    <For
                        each=move || store.open_tabs.get()
                        key=|svc| svc.slug()
                        children=move |svc| {
                            let is_active = move || store.active_tab.get() == Some(svc);
                            let close_label = format!("Close {} tab", svc.label());
                            view! {
                                <li class="rail-item" class:is-active=is_active>
                                    <button
                                        type="button"
                                        class="rail-btn"
                                        aria-current=move || is_active().then_some("page")
                                        on:click=move |_| {
                                            transition_view_and_focus("stage-heading", move || {
                                                store.open_service_tab(svc);
                                            });
                                        }
                                    >
                                        <span>{svc.label()}</span>
                                    </button>
                                    <button
                                        type="button"
                                        class="rail-close-btn"
                                        aria-label=close_label
                                        on:click=move |_| {
                                            store.close_service_tab(svc);
                                            focus_by_id("stage-heading");
                                        }
                                    >
                                        <svg
                                            viewBox="0 0 12 12"
                                            width="10"
                                            height="10"
                                            aria-hidden="true"
                                        >
                                            <path
                                                d="M2.5 2.5L9.5 9.5M9.5 2.5L2.5 9.5"
                                                stroke="currentColor"
                                                stroke-width="1.5"
                                                stroke-linecap="round"
                                            />
                                        </svg>
                                    </button>
                                </li>
                            }
                        }
                    />
                </ul>
            </section>

            <section class="rail-section">
                <h2 class="rail-heading">"SERVICES"</h2>
                <ul class="rail-list">
                    {ServiceId::STAGE_FILTERS
                        .into_iter()
                        .enumerate()
                        .map(|(idx, svc)| {
                            let is_active = move || {
                                store.active_tab.get().is_none() && store.active_service.get() == svc
                            };
                            view! {
                                <li class="rail-item" class:is-active=is_active>
                                    <button
                                        type="button"
                                        class="rail-btn"
                                        aria-pressed=move || if is_active() { "true" } else { "false" }
                                        on:click=move |_| {
                                            store.active_service.set(svc);
                                            store.active_tab.set(None);
                                        }
                                    >
                                        <span>{svc.label()}</span>
                                        <span class="rail-item-meta num">
                                            {move || service_counts.get()[idx]}
                                        </span>
                                    </button>
                                    {(svc != ServiceId::All).then(|| {
                                        let open_label = format!("Open {} app view", svc.label());
                                        view! {
                                            <button
                                                type="button"
                                                class="rail-close-btn"
                                                aria-label=open_label
                                                on:click=move |_| {
                                                    transition_view_and_focus("stage-heading", move || {
                                                        store.open_service_tab(svc);
                                                    });
                                                }
                                            >
                                                <svg
                                                    viewBox="0 0 12 12"
                                                    width="10"
                                                    height="10"
                                                    aria-hidden="true"
                                                >
                                                    <path
                                                        d="M3 9L9 3M4.5 3H9V7.5"
                                                        fill="none"
                                                        stroke="currentColor"
                                                        stroke-width="1.4"
                                                        stroke-linecap="round"
                                                        stroke-linejoin="round"
                                                    />
                                                </svg>
                                            </button>
                                        }
                                    })}
                                </li>
                            }
                        })
                        .collect_view()}
                </ul>
            </section>
        </nav>
    }
}

//! Built-in Wiki & Blog documentation view rendered through `cockpit-wiki` on SSR/wiki builds
//! with a zero-parser summary fallback in ultra-lean CSR WASM bundles.

#[cfg(feature = "wiki")]
use crate::state::CockpitStore;
#[cfg(feature = "wiki")]
use crate::views::helpers::StatusBadge;
#[cfg(feature = "wiki")]
use crate::wiki_fixtures::builtin_wiki_catalog;
use leptos::prelude::*;

/// Interactive Wiki & Release Blog surface backed by `cockpit-wiki`.
#[component]
pub fn WikiPortalSurface() -> impl IntoView {
    #[cfg(feature = "wiki")]
    {
        let store = expect_context::<CockpitStore>();
        let catalog = StoredValue::new(builtin_wiki_catalog());

        let articles = catalog.with_value(|cat| {
            cat.list(None)
                .into_iter()
                .map(|a| (a.route.clone(), a.meta.title.clone()))
                .collect::<Vec<_>>()
        });

        let active_article = Memo::new(move |_| {
            let route = store.active_wiki_route.get();
            catalog.with_value(|cat| {
                cat.find_by_route(&route)
                    .cloned()
                    .or_else(|| cat.list(None).into_iter().next().cloned())
            })
        });

        view! {
            <section class="surface-section">
                <h2>"Cockpit Wiki & Architecture Notes"</h2>
                <p class="surface-lead">
                    "Rendered from embedded Markdown with sanitized HTML output, anchor ToC, and Mermaid fenced diagram blocks."
                </p>
                <div class="stage-toolbar" role="navigation" aria-label="Documentation articles">
                    {articles
                        .into_iter()
                        .map(|(route, title)| {
                            let r_cmp = route.clone();
                            let r_click = route;
                            let is_cur = move || store.active_wiki_route.get() == r_cmp;
                            view! {
                                <button
                                    type="button"
                                    class="btn"
                                    aria-pressed=move || if is_cur() { "true" } else { "false" }
                                    on:click=move |_| store.active_wiki_route.set(r_click.clone())
                                >
                                    {title}
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
            </section>

            {move || active_article.get().map(|article| {
                let evidence = article
                    .meta
                    .evidence_class
                    .clone()
                    .unwrap_or_else(|| "Reference".into());
                let pub_date = article.meta.published_at.clone();
                let toc_items = article.toc.clone();
                let html_body = article.html.clone();
                view! {
                    <article class="surface-section">
                        <div class="rail-item">
                            <StatusBadge
                                label=Signal::derive(move || evidence.clone())
                                tone=Signal::derive(|| "info")
                            />
                            <span class="muted num">{format!("{pub_date} · {} words", article.word_count)}</span>
                        </div>
                        {(!toc_items.is_empty()).then(|| view! {
                            <nav aria-label="Table of contents">
                                <ul class="stage-toolbar">
                                    {toc_items
                                        .into_iter()
                                        .map(|h| view! {
                                            <li>
                                                <a class="chip-btn" href=format!("#{}", h.id)>{h.text}</a>
                                            </li>
                                        })
                                        .collect_view()}
                                </ul>
                            </nav>
                        })}
                        <div class="wiki-prose" inner_html=html_body />
                    </article>
                }
            })}
        }
        .into_any()
    }

    #[cfg(not(feature = "wiki"))]
    {
        view! {
            <section class="surface-section">
                <h2>"Cockpit Wiki & Architecture Notes"</h2>
                <p class="surface-lead">
                    "Wiki and blog Markdown documents are SSR-rendered on the server via `cockpit-wiki` so browser islands remain strictly under budget."
                </p>
                <div class="wiki-prose">
                    <pre class="mermaid" data-lang="mermaid">
                        {"flowchart LR\n  Wiki[cockpit-wiki Markdown] --> SSR[Leptos 0.8 SSR HTML]\n  Telemetry[telemetry-scene JSON] --> Iso[iso-diagram 2.5D Layout]"}
                    </pre>
                </div>
            </section>
        }
        .into_any()
    }
}

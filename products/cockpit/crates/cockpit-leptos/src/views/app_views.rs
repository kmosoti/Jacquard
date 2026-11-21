//! Scoped operational app views (`@scope (.app-surface)`) for Control Plane & Governor,
//! Telemetry Custody/Blame/Lineage Studio, Overlay Mesh Matrix, and dispatch to ops/wiki tabs.

use crate::api;
use crate::state::{CockpitStore, ServiceId};
use crate::views::app_views_ops::OpsServiceView;
use crate::views::helpers::{StatusBadge, status_tone, transition_view_and_focus};
use crate::views::wiki_view::WikiPortalSurface;
use leptos::prelude::*;

/// Top-level app/wiki tab body rendered inside `.stage-body` when `active_tab` is set.
#[component]
pub fn ServiceAppView(
    /// Active tab service identifier.
    svc: ServiceId,
) -> impl IntoView {
    let store = expect_context::<CockpitStore>();
    let route_info = Memo::new(move |_| {
        store
            .ui_routes
            .with(|routes| routes.iter().find(|r| r.service == svc.slug()).cloned())
    });

    view! {
        <section class="app-surface" aria-label=format!("{} operational view", svc.label())>
            <div class="surface-section">
                <div class="stage-toolbar">
                    <button
                        type="button"
                        class="btn quiet"
                        on:click=move |_| {
                            transition_view_and_focus("stage-heading", move || {
                                store.active_tab.set(None);
                            });
                        }
                    >
                        "Back to Cluster Stage"
                    </button>
                    {move || route_info.get().map(|r| view! {
                        <span class="kbd-hint">{r.mode.to_ascii_uppercase()}</span>
                        <code class="mono muted">{r.api_uri}</code>
                        <span class="kbd-hint num">{format!("TTL {}ms", r.cache_ttl_ms)}</span>
                    })}
                    <button
                        type="button"
                        class="btn"
                        on:click=move |_| api::trigger_refresh_all(store)
                    >
                        "Refresh"
                    </button>
                </div>
            </div>

            {match svc {
                ServiceId::ControlPlane => view! { <ControlPlaneSurface /> }.into_any(),
                ServiceId::Telemetry => view! { <TelemetryStudioSurface /> }.into_any(),
                ServiceId::Mesh => view! { <OverlayMeshSurface /> }.into_any(),
                ServiceId::DocsWiki => view! { <WikiPortalSurface /> }.into_any(),
                other => view! { <OpsServiceView svc=other /> }.into_any(),
            }}
        </section>
    }
}

#[component]
fn ControlPlaneSurface() -> impl IntoView {
    let store = expect_context::<CockpitStore>();
    view! {
        <section class="surface-section">
            <h2>"Predictive Fleet Governor"</h2>
            <p class="surface-lead">
                "Closed-loop cgroup v2 & PSI governor directives adjusting poll cadence, worker concurrency, and WAL flush policy across service classes."
            </p>
            <div class="governor-strip">
                <For
                    each=move || store.governor_cards.get()
                    key=|c| c.class_label.clone()
                    children=move |c| {
                        let tier_label = format!("{:?}", c.tier);
                        let tone = status_tone(&tier_label);
                        view! {
                            <article class="governor-cell">
                                <div class="rail-item">
                                    <h3>{c.class_label}</h3>
                                    <StatusBadge
                                        label=Signal::derive(move || tier_label.clone())
                                        tone=Signal::derive(move || tone)
                                    />
                                </div>
                                <div class="kpi-value">
                                    {format!("{}% pressure · {:.1}% mem", c.composite_pressure, f32::from(c.mem_ratio_permille) / 10.0)}
                                </div>
                                <div class="muted num">
                                    {format!("poll {}ms · workers <= {} · {}", c.poll_interval_ms, c.max_concurrent_agents, c.reason)}
                                </div>
                            </article>
                        }
                    }
                />
            </div>
        </section>

        <section class="surface-section">
            <h2>"QUIC Dataplane & UI Forwarder Routes"</h2>
            <div class="kpi-strip">
                <article class="kpi-cell">
                    <h3>"Streams Served"</h3>
                    <div class="kpi-value">{move || store.telemetry.with(|t| t.quic.streams_served)}</div>
                    <div class="muted num">{move || format!("{} active conns", store.telemetry.with(|t| t.quic.active_connections))}</div>
                </article>
                <article class="kpi-cell">
                    <h3>"QUIC RTT"</h3>
                    <div class="kpi-value">{move || format!("{} us", store.telemetry.with(|t| t.quic.rtt_us))}</div>
                    <div class="muted num">{move || format!("cwnd {} B", store.telemetry.with(|t| t.quic.cwnd_bytes))}</div>
                </article>
                <article class="kpi-cell">
                    <h3>"ECN CE / Loss"</h3>
                    <div class="kpi-value">
                        {move || store.telemetry.with(|t| format!("{} / {}", t.quic.ecn_ce_events, t.quic.lost_packets))}
                    </div>
                    <div class="muted">"Sans-I/O CoreEngine"</div>
                </article>
                <article class="kpi-cell">
                    <h3>"vTPM 2.0 Secrets"</h3>
                    <div class="kpi-value">
                        {move || store.telemetry.with(|t| if t.secrets.tpm2_available { "vTPM2 Ready" } else { "Unavailable" })}
                    </div>
                    <div class="muted num">
                        {move || store.telemetry.with(|t| format!(
                            "sealed {} · runtime {}",
                            if t.secrets.sealed_creds.is_empty() { "none".into() } else { t.secrets.sealed_creds.join(",") },
                            if t.secrets.runtime_creds.is_empty() { "none".into() } else { t.secrets.runtime_creds.join(",") },
                        ))}
                    </div>
                </article>
            </div>

            <div class="data-table-wrap">
                <table class="data-table">
                    <caption>"Modular UI Forwarder Route Registry"</caption>
                    <thead>
                        <tr>
                            <th scope="col">"Service"</th>
                            <th scope="col">"Mode"</th>
                            <th scope="col">"Endpoint"</th>
                            <th scope="col">"Priority"</th>
                            <th scope="col">"Cache TTL"</th>
                        </tr>
                    </thead>
                    <tbody>
                        <For
                            each=move || store.ui_routes.get()
                            key=|r| r.service.clone()
                            children=move |r| view! {
                                <tr>
                                    <th scope="row">{r.tab_title}</th>
                                    <td>{r.mode}</td>
                                    <td class="mono">{r.api_uri}</td>
                                    <td>{r.priority}</td>
                                    <td data-numeric="true">{format!("{} ms", r.cache_ttl_ms)}</td>
                                </tr>
                            }
                        />
                    </tbody>
                </table>
            </div>
        </section>

        <section class="surface-section">
            <h2>"Recent Reconciler Activity"</h2>
            <ul>
                <For
                    each=move || store.snapshot.with(|s| s.logs.clone())
                    key=|l| format!("{}:{}:{}", l.timestamp, l.urn, l.message)
                    children=move |l| {
                        let lvl = l.level.clone();
                        let tone = status_tone(&l.level);
                        view! {
                            <li class="log-row">
                                <span class="num muted">{l.timestamp}</span>
                                <StatusBadge
                                    label=Signal::derive(move || lvl.clone())
                                    tone=Signal::derive(move || tone)
                                />
                                <code class="mono">{l.urn}</code>
                                <span>{l.message}</span>
                            </li>
                        }
                    }
                />
            </ul>
        </section>
    }
}

#[component]
fn TelemetryStudioSurface() -> impl IntoView {
    let store = expect_context::<CockpitStore>();
    view! {
        <section class="surface-section">
            <h2>"Telemetry Custody, Blame & Lineage Studio"</h2>
            <p class="surface-lead">
                "Canonical telemetry sink on obs-01 ([fd00:db8::4]:17440) tracking Merkle custody receipts, watermark lag, and cgroup memory blame."
            </p>
            <div class="kpi-strip">
                <article class="kpi-cell">
                    <h3>"Custody Records"</h3>
                    <div class="kpi-value">{move || store.telemetry.with(|t| t.custody_records)}</div>
                    <div class="muted">{move || store.telemetry.with(|t| t.source_desc.clone())}</div>
                </article>
                <article class="kpi-cell">
                    <h3>"Watermark Lag"</h3>
                    <div class="kpi-value">{move || format!("{} seq", store.telemetry.with(|t| t.watermark_lag))}</div>
                    <div class="muted">"WAL origin -> sink"</div>
                </article>
                <article class="kpi-cell">
                    <h3>"Top Blame"</h3>
                    <div class="kpi-value">{move || store.telemetry.with(|t| t.top_blame.clone())}</div>
                    <div class="muted">"Cgroup memory attribution"</div>
                </article>
            </div>
        </section>

        <section class="surface-section">
            <h2>"Observability Pipeline Nodes"</h2>
            <div class="data-table-wrap">
                <table class="data-table">
                    <caption>"Origin WAL & canonical receiver lineage targets"</caption>
                    <thead>
                        <tr>
                            <th scope="col">"URN"</th>
                            <th scope="col">"Status"</th>
                            <th scope="col">"Endpoint"</th>
                            <th scope="col">"Spec Hash"</th>
                        </tr>
                    </thead>
                    <tbody>
                        <For
                            each=move || {
                                store.snapshot.with(|s| {
                                    s.targets
                                        .iter()
                                        .filter(|t| ServiceId::Telemetry.matches_target(t))
                                        .cloned()
                                        .collect::<Vec<_>>()
                                })
                            }
                            key=|t| t.urn.clone()
                            children=move |t| {
                                let status = t.status.clone();
                                let tone = status_tone(&t.status);
                                view! {
                                    <tr>
                                        <th scope="row" class="mono">{t.urn}</th>
                                        <td>
                                            <StatusBadge
                                                label=Signal::derive(move || status.clone())
                                                tone=Signal::derive(move || tone)
                                            />
                                        </td>
                                        <td class="mono">{t.endpoint.unwrap_or_else(|| "[fd00:db8::4]:17440".into())}</td>
                                        <td class="mono">{t.spec_hash_short}</td>
                                    </tr>
                                }
                            }
                        />
                    </tbody>
                </table>
            </div>
        </section>
    }
}

#[component]
fn OverlayMeshSurface() -> impl IntoView {
    let store = expect_context::<CockpitStore>();
    view! {
        <section class="surface-section">
            <h2>"WireGuard Overlay IPv6 Mesh Matrix (MTU 1360)"</h2>
            <p class="surface-lead">
                "Control-plane RPC, replication packs, and telemetry streams traverse encrypted ChaCha20-Poly1305 IPv6 ULA peers under fd00:db8::/64."
            </p>
            <div class="data-table-wrap">
                <table class="data-table">
                    <caption>"Mesh peers and firewall allowances"</caption>
                    <thead>
                        <tr>
                            <th scope="col">"Peer / Rule"</th>
                            <th scope="col">"Status"</th>
                            <th scope="col">"Mesh Endpoint"</th>
                            <th scope="col">"Latency"</th>
                            <th scope="col">"Overhead Budget"</th>
                        </tr>
                    </thead>
                    <tbody>
                        <For
                            each=move || {
                                store.snapshot.with(|s| {
                                    s.targets
                                        .iter()
                                        .filter(|t| ServiceId::Mesh.matches_target(t))
                                        .cloned()
                                        .collect::<Vec<_>>()
                                })
                            }
                            key=|t| t.urn.clone()
                            children=move |t| {
                                let status = t.status.clone();
                                let tone = status_tone(&t.status);
                                let lat = t.latency_ms.map_or_else(|| "—".into(), |m| format!("{m} ms"));
                                view! {
                                    <tr>
                                        <th scope="row" class="mono">{t.urn}</th>
                                        <td>
                                            <StatusBadge
                                                label=Signal::derive(move || status.clone())
                                                tone=Signal::derive(move || tone)
                                            />
                                        </td>
                                        <td class="mono">{t.endpoint.unwrap_or_else(|| "fd00:db8::/64".into())}</td>
                                        <td data-numeric="true">{lat}</td>
                                        <td data-numeric="true">"80 B (16 WG + 48 IPv6/UDP + 16 Tag)"</td>
                                    </tr>
                                }
                            }
                        />
                    </tbody>
                </table>
            </div>
        </section>
    }
}

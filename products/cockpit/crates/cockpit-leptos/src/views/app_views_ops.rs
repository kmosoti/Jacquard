//! Operational service surfaces for Vault Storage Explorer, Spot CI Queue,
//! Host PSI/ZRAM Storage, and Wasmtime Sandbox.

use crate::state::{CockpitStore, ServiceId};
use crate::views::helpers::{StatusBadge, format_cents, format_gib, status_tone};
use leptos::prelude::*;

/// Dispatches non-stage operational views to dedicated domain tables.
#[component]
pub fn OpsServiceView(
    /// Selected service plane identifier.
    svc: ServiceId,
) -> impl IntoView {
    match svc {
        ServiceId::StorageVault => view! { <VaultStorageSurface /> }.into_any(),
        ServiceId::BatchCi => view! { <BatchCiSurface /> }.into_any(),
        ServiceId::HostMem => view! { <HostPsiStorageSurface /> }.into_any(),
        _ => view! { <WasmSandboxSurface /> }.into_any(),
    }
}

#[component]
fn VaultStorageSurface() -> impl IntoView {
    let store = expect_context::<CockpitStore>();
    view! {
        <section class="surface-section">
            <h2>"Framed Bundle & Archive Storage Explorer"</h2>
            <p class="surface-lead">
                "Content-addressed seekable Zstd frames (<= 256 KiB per frame) backed by WAL snapshot durability and scoped object-store buckets."
            </p>
            <div class="kpi-strip">
                <article class="kpi-cell">
                    <h3>"Bundle Format"</h3>
                    <div class="kpi-value">"ZSTD-FRAME"</div>
                    <div class="muted">"Streaming seekable frames"</div>
                </article>
                <article class="kpi-cell">
                    <h3>"Archive Policy"</h3>
                    <div class="kpi-value">"Immutable"</div>
                    <div class="muted">"Scoped storage credential"</div>
                </article>
                <article class="kpi-cell">
                    <h3>"Storage Targets"</h3>
                    <div class="kpi-value">
                        {move || store.snapshot.with(|s| {
                            s.targets.iter().filter(|t| ServiceId::StorageVault.matches_target(t)).count()
                        })}
                    </div>
                    <div class="muted">"vault-01 + archive bucket"</div>
                </article>
            </div>
            <TargetGroupTable svc=ServiceId::StorageVault caption="Vault nodes and archive buckets" />
        </section>
    }
}

#[component]
fn BatchCiSurface() -> impl IntoView {
    let store = expect_context::<CockpitStore>();
    view! {
        <section class="surface-section">
            <h2>"Spot CI Runner Queue"</h2>
            <p class="surface-lead">
                "Ephemeral verification job templates throttled by FleetGovernor concurrency caps to preserve control-plane headroom."
            </p>
            <div class="kpi-strip">
                <article class="kpi-cell">
                    <h3>"Max Concurrent Runners"</h3>
                    <div class="kpi-value">
                        {move || store.governor_cards.with(|c| {
                            c.first().map_or(2, |g| g.max_concurrent_agents)
                        })}
                    </div>
                    <div class="muted">"Governor enforced cap"</div>
                </article>
                <article class="kpi-cell">
                    <h3>"Runner Templates"</h3>
                    <div class="kpi-value">
                        {move || store.snapshot.with(|s| {
                            s.targets.iter().filter(|t| ServiceId::BatchCi.matches_target(t)).count()
                        })}
                    </div>
                    <div class="muted">"Preemptible worker class"</div>
                </article>
            </div>
            <TargetGroupTable svc=ServiceId::BatchCi caption="Registered Spot CI runner templates" />
        </section>
    }
}

#[component]
fn HostPsiStorageSurface() -> impl IntoView {
    let store = expect_context::<CockpitStore>();
    let mem_used_pct = Memo::new(move |_| {
        store.telemetry.with(|t| {
            if t.mem_total_bytes == 0 {
                0.0
            } else {
                let used = t.mem_total_bytes.saturating_sub(t.mem_available_bytes);
                (used as f64 / t.mem_total_bytes as f64) * 100.0
            }
        })
    });

    view! {
        <section class="surface-section">
            <h2>"Host PSI / ZRAM & Persistent Storage"</h2>
            <p class="surface-lead">
                "Linux Pressure Stall Information (avg10) and memory headroom sampled directly from procfs without external daemon dependencies."
            </p>
            <div class="kpi-strip">
                <article class="kpi-cell">
                    <h3>"Memory Available"</h3>
                    <div class="kpi-value">{move || store.telemetry.with(|t| format_gib(t.mem_available_bytes))}</div>
                    <div class="muted num">
                        {move || format!("{:.1}% used of {}", mem_used_pct.get(), store.telemetry.with(|t| format_gib(t.mem_total_bytes)))}
                    </div>
                </article>
                <article class="kpi-cell">
                    <h3>"PSI CPU / Mem / IO"</h3>
                    <div class="kpi-value">
                        {move || store.telemetry.with(|t| {
                            format!("{:.1}% / {:.1}% / {:.1}%", t.psi_cpu_avg10, t.psi_mem_avg10, t.psi_io_avg10)
                        })}
                    </div>
                    <div class="meter-track" aria-hidden="true">
                        <div
                            class="meter-fill"
                            data-tone=move || {
                                let p = mem_used_pct.get();
                                if p > 85.0 { "err" } else if p > 70.0 { "warn" } else { "ok" }
                            }
                            style:inline-size=move || format!("{:.0}%", mem_used_pct.get().clamp(2.0, 100.0))
                        />
                    </div>
                </article>
            </div>
            <TargetGroupTable svc=ServiceId::HostMem caption="Persistent block volumes and storage targets" />
        </section>
    }
}

#[component]
fn WasmSandboxSurface() -> impl IntoView {
    view! {
        <section class="surface-section">
            <h2>"WASM Sandbox & Credential Isolation"</h2>
            <p class="surface-lead">
                "Zero-trust Wasmtime execution boundary (`wasmtime://`): candidate runners never receive raw OAuth tokens, refresh tokens, or service keys."
            </p>
            <TargetGroupTable svc=ServiceId::Sandbox caption="Sandboxed execution nodes and credential boundaries" />
        </section>
    }
}

#[component]
fn TargetGroupTable(svc: ServiceId, caption: &'static str) -> impl IntoView {
    let store = expect_context::<CockpitStore>();
    view! {
        <div class="data-table-wrap">
            <table class="data-table">
                <caption>{caption}</caption>
                <thead>
                    <tr>
                        <th scope="col">"URN"</th>
                        <th scope="col">"Kind"</th>
                        <th scope="col">"Status"</th>
                        <th scope="col">"Monthly"</th>
                        <th scope="col">"Untrusted Secrets"</th>
                    </tr>
                </thead>
                <tbody>
                    <For
                        each=move || {
                            store.snapshot.with(|s| {
                                s.targets
                                    .iter()
                                    .filter(|t| svc.matches_target(t))
                                    .cloned()
                                    .collect::<Vec<_>>()
                            })
                        }
                        key=|t| t.urn.clone()
                        children=move |t| {
                            let status = t.status.clone();
                            let tone = status_tone(&t.status);
                            let spend = format_cents(t.monthly_cents);
                            let secret_policy = if t.allow_llm_secrets { "Allowed" } else { "Blocked" };
                            view! {
                                <tr>
                                    <th scope="row" class="mono">{t.urn}</th>
                                    <td>{t.kind_label}</td>
                                    <td>
                                        <StatusBadge
                                            label=Signal::derive(move || status.clone())
                                            tone=Signal::derive(move || tone)
                                        />
                                    </td>
                                    <td data-numeric="true">{spend}</td>
                                    <td>{secret_policy}</td>
                                </tr>
                            }
                        }
                    />
                </tbody>
            </table>
        </div>
    }
}

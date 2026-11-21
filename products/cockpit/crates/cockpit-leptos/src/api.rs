//! Same-origin REST actions (`/v1/snapshot`, `/v1/telemetry`, `/v1/governor`, `/v1/ui/routes`,
//! `/v1/diff`, `/v1/reconcile`) with offline/simulation fallback for `Simulation` and native tests.

use crate::state::{
    ClusterSnapshotDto, CockpitStore, EnvKind, GovernorResponseDto, LogDto, TelemetryDto,
    UiRoutesResponseDto,
};
use leptos::prelude::*;

/// Refreshes snapshot, telemetry, governor directives, and UI routes into `store`.
pub async fn refresh_all(store: CockpitStore) -> Result<(), String> {
    if store.active_env.get_untracked() == EnvKind::Simulation {
        store
            .status_banner
            .set(Some("Testing · sim state refreshed locally".into()));
        return Ok(());
    }
    #[cfg(target_arch = "wasm32")]
    {
        let mut errs = Vec::new();
        match fetch_json::<ClusterSnapshotDto>("/v1/snapshot").await {
            Ok(snap) => store.apply_snapshot(snap),
            Err(e) => errs.push(e),
        }
        match fetch_json::<TelemetryDto>("/v1/telemetry").await {
            Ok(tele) => store.apply_telemetry(tele),
            Err(e) => errs.push(e),
        }
        match fetch_json::<GovernorResponseDto>("/v1/governor").await {
            Ok(gov) => store.apply_governor(gov),
            Err(e) => errs.push(e),
        }
        match fetch_json::<UiRoutesResponseDto>("/v1/ui/routes").await {
            Ok(routes) => store.apply_ui_routes(routes),
            Err(e) => errs.push(e),
        }
        if errs.is_empty() {
            store.status_banner.set(None);
            Ok(())
        } else {
            let msg = errs.join("; ");
            store
                .status_banner
                .set(Some(format!("Sync warning: {msg}")));
            Err(msg)
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (
            core::any::type_name::<ClusterSnapshotDto>(),
            core::any::type_name::<TelemetryDto>(),
            core::any::type_name::<GovernorResponseDto>(),
            core::any::type_name::<UiRoutesResponseDto>(),
        );
        store.status_banner.set(None);
        Ok(())
    }
}

/// Runs a non-destructive drift diff for `urn` against `POST /v1/diff`.
pub async fn run_diff(store: CockpitStore, urn: String) -> Result<(), String> {
    if store.active_env.get_untracked() == EnvKind::Simulation || cfg!(not(target_arch = "wasm32"))
    {
        apply_local_diff(store, &urn);
        return Ok(());
    }
    #[cfg(target_arch = "wasm32")]
    {
        let body = serde_json::json!({ "urn": urn }).to_string();
        match post_json::<ClusterSnapshotDto>("/v1/diff", &body).await {
            Ok(snap) => {
                store.apply_snapshot(snap);
                store
                    .status_banner
                    .set(Some(format!("Diff clean for {urn}")));
                Ok(())
            }
            Err(err) => {
                store.status_banner.set(Some(format!("Diff failed: {err}")));
                Err(err)
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        unreachable!()
    }
}

/// Triggers reconciliation for `urn` against `POST /v1/reconcile`.
pub async fn run_reconcile(store: CockpitStore, urn: String) -> Result<(), String> {
    if store.active_env.get_untracked() == EnvKind::Simulation || cfg!(not(target_arch = "wasm32"))
    {
        apply_local_reconcile(store, &urn);
        return Ok(());
    }
    #[cfg(target_arch = "wasm32")]
    {
        let body = serde_json::json!({ "urn": urn }).to_string();
        match post_json::<ClusterSnapshotDto>("/v1/reconcile", &body).await {
            Ok(snap) => {
                store.apply_snapshot(snap);
                store.status_banner.set(Some(format!("Reconciled {urn}")));
                Ok(())
            }
            Err(err) => {
                store
                    .status_banner
                    .set(Some(format!("Reconcile failed: {err}")));
                Err(err)
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        unreachable!()
    }
}

/// Fire-and-forget helper for UI button click handlers (`Refresh`).
pub fn trigger_refresh_all(store: CockpitStore) {
    #[cfg(target_arch = "wasm32")]
    leptos::task::spawn_local(async move {
        let _ = refresh_all(store).await;
    });
    #[cfg(not(target_arch = "wasm32"))]
    {
        store.status_banner.set(None);
    }
}

/// Fire-and-forget helper for UI button click handlers (`Diff`).
pub fn trigger_diff(store: CockpitStore, urn: String) {
    #[cfg(target_arch = "wasm32")]
    leptos::task::spawn_local(async move {
        let _ = run_diff(store, urn).await;
    });
    #[cfg(not(target_arch = "wasm32"))]
    {
        apply_local_diff(store, &urn);
    }
}

/// Fire-and-forget helper for UI button click handlers (`Reconcile`).
pub fn trigger_reconcile(store: CockpitStore, urn: String) {
    #[cfg(target_arch = "wasm32")]
    leptos::task::spawn_local(async move {
        let _ = run_reconcile(store, urn).await;
    });
    #[cfg(not(target_arch = "wasm32"))]
    {
        apply_local_reconcile(store, &urn);
    }
}

fn apply_local_diff(store: CockpitStore, urn: &str) {
    store.snapshot.update(|snap| {
        snap.logs.push(LogDto {
            timestamp: "local".into(),
            urn: urn.to_owned(),
            level: "INFO".into(),
            message: format!("diff clean for {urn}"),
        });
    });
    store
        .status_banner
        .set(Some(format!("Diff clean for {urn}")));
}

fn apply_local_reconcile(store: CockpitStore, urn: &str) {
    store.snapshot.update(|snap| {
        for target in &mut snap.targets {
            if target.urn == urn || target.status == "Drifted" {
                target.status = "Converged".into();
                target.drift_count = Some(0);
                target.generation = target.generation.saturating_add(1);
            }
        }
        snap.logs.push(LogDto {
            timestamp: "local".into(),
            urn: urn.to_owned(),
            level: "OK".into(),
            message: format!("reconciled {urn}"),
        });
    });
    store.status_banner.set(Some(format!("Reconciled {urn}")));
}

#[cfg(target_arch = "wasm32")]
async fn fetch_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, String> {
    let init = web_sys::RequestInit::new();
    init.set_method("GET");
    init.set_mode(web_sys::RequestMode::SameOrigin);
    send_request(path, &init).await
}

#[cfg(target_arch = "wasm32")]
async fn post_json<T: serde::de::DeserializeOwned>(path: &str, body: &str) -> Result<T, String> {
    let init = web_sys::RequestInit::new();
    init.set_method("POST");
    init.set_mode(web_sys::RequestMode::SameOrigin);
    let headers = web_sys::Headers::new().map_err(|_| "failed to allocate headers".to_owned())?;
    headers
        .set("content-type", "application/json")
        .map_err(|_| "failed to set content-type".to_owned())?;
    init.set_headers(&headers);
    init.set_body(&wasm_bindgen::JsValue::from_str(body));
    send_request(path, &init).await
}

#[cfg(target_arch = "wasm32")]
async fn send_request<T: serde::de::DeserializeOwned>(
    path: &str,
    init: &web_sys::RequestInit,
) -> Result<T, String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let win = web_sys::window().ok_or_else(|| "window unavailable".to_owned())?;
    let resp_val = JsFuture::from(win.fetch_with_str_and_init(path, init))
        .await
        .map_err(|e| format!("fetch {path}: {e:?}"))?;
    let resp: web_sys::Response = resp_val
        .dyn_into()
        .map_err(|_| format!("invalid response for {path}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {} on {path}", resp.status()));
    }
    let text_promise = resp
        .text()
        .map_err(|_| format!("text() failed for {path}"))?;
    let text_val = JsFuture::from(text_promise)
        .await
        .map_err(|e| format!("read {path}: {e:?}"))?;
    let raw = text_val
        .as_string()
        .ok_or_else(|| format!("non-string body on {path}"))?;
    serde_json::from_str(&raw).map_err(|e| format!("decode {path}: {e}"))
}

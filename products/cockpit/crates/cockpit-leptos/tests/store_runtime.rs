//! Integration tests for `CockpitStore`, `0 Hz` settled idle invariants, wiki portal, and DTO parsing.

use cockpit_leptos::api::{trigger_diff, trigger_reconcile};
use cockpit_leptos::state::{
    CockpitStore, DEFAULT_PITCH_DEG, DEFAULT_YAW_DEG, DEFAULT_ZOOM, EnvKind, GovernorResponseDto,
    ProjectionKind, ServiceId, SseEventDto, TelemetryDto, UiRoutesResponseDto,
};
use cockpit_leptos::wiki_fixtures::{builtin_atom_feed, builtin_rss_feed, builtin_wiki_catalog};
use leptos::prelude::*;

#[test]
fn defaults_populate_example_cluster_and_idle_zero_hz_flags() {
    let owner = Owner::new();
    owner.with(|| {
        let store = CockpitStore::new_with_defaults();
        assert_eq!(store.active_env.get_untracked(), EnvKind::ExampleCluster);
        assert_eq!(store.active_service.get_untracked(), ServiceId::All);
        assert_eq!(store.active_tab.get_untracked(), None);
        assert_eq!(
            store.projection.get_untracked(),
            ProjectionKind::Isometric25D
        );
        assert!(!store.animations_enabled.get_untracked());
        assert!(!store.demo_running.get_untracked());
        assert!(store.poll_interval_ms() >= 5_000);
        assert_eq!(store.density.get_untracked(), "compact");
        assert_eq!(store.snapshot.with_untracked(|s| s.targets.len()), 15);
        assert_eq!(store.governor_cards.with_untracked(|g| g.len()), 5);
        assert_eq!(store.ui_routes.with_untracked(|r| r.len()), 7);
        assert_eq!(
            store.selected_target().map(|t| t.urn),
            Some("cluster.node/ctrl-01".to_owned())
        );
    });
}

#[test]
fn env_switching_tabs_demo_and_orbit_reset_preserve_zero_hz_idle() {
    let owner = Owner::new();
    owner.with(|| {
        let store = CockpitStore::new_with_defaults();
        store.switch_env(EnvKind::Simulation);
        assert_eq!(store.active_env.get_untracked(), EnvKind::Simulation);
        assert_eq!(store.healthy_count(), 14);

        trigger_diff(store, "cluster.node/vault-01".into());
        assert!(
            store
                .status_banner
                .get_untracked()
                .unwrap_or_default()
                .contains("Diff clean")
        );

        trigger_reconcile(store, "cluster.node/vault-01".into());
        assert_eq!(store.healthy_count(), 15);

        store.open_service_tab(ServiceId::DocsWiki);
        assert_eq!(store.active_tab.get_untracked(), Some(ServiceId::DocsWiki));
        assert!(
            store
                .open_tabs
                .with_untracked(|t| t.contains(&ServiceId::DocsWiki))
        );

        store.close_service_tab(ServiceId::DocsWiki);
        assert_eq!(store.active_tab.get_untracked(), None);

        store.yaw_deg.set(120.0);
        store.pitch_deg.set(22.0);
        store.zoom.set(1.75);
        store.reset_orbit();
        assert_eq!(store.yaw_deg.get_untracked(), DEFAULT_YAW_DEG);
        assert_eq!(store.pitch_deg.get_untracked(), DEFAULT_PITCH_DEG);
        assert_eq!(store.zoom.get_untracked(), DEFAULT_ZOOM);

        store.trigger_demo();
        assert!(store.demo_running.get_untracked());
        assert!(store.animations_enabled.get_untracked());
        store.trigger_demo();
        assert!(!store.demo_running.get_untracked());
        assert!(!store.animations_enabled.get_untracked());
    });
}

#[test]
fn builtin_wiki_catalog_and_feeds_have_valid_links_and_mermaid() {
    let cat = builtin_wiki_catalog();
    assert!(cat.verify_internal_links(&[]).is_empty());
    let arch = cat
        .find_by_route("/wiki/cockpit-architecture")
        .expect("arch article");
    assert!(arch.has_mermaid);
    assert!(builtin_rss_feed().contains("Stage A Public Cockpit Alpha Release Log"));
    assert!(builtin_atom_feed().contains("Hallmark x Leptos 0.8 Islands Architecture"));
}

#[test]
fn sse_and_rest_dtos_deserialize_example_cluster_payloads() {
    let gov_json = serde_json::json!({
        "classes": [
            {
                "class_label": "wasm-sandbox",
                "tier": "Critical",
                "reason": "memory_high_watermark",
                "composite_pressure": 91,
                "mem_ratio_permille": 930,
                "mem_slope_kib_per_sec": 120,
                "eta_mem_high_secs": 30,
                "poll_interval_ms": 5000,
                "max_concurrent_agents": 0,
                "flush_wal_and_bundles": true,
                "shed_optional_probes": true
            }
        ],
        "quic": {
            "active_connections": 2,
            "streams_served": 11,
            "rx_datagrams": 4,
            "tx_datagrams": 4,
            "rtt_us": 740,
            "cwnd_bytes": 16384,
            "ecn_ce_events": 0,
            "lost_packets": 0,
            "cert_fingerprint_hex": "deadbeef"
        }
    });
    let tele_json = serde_json::json!({
        "online": true,
        "source_desc": "telemetry@fd00:db8::4",
        "ts_ms": 5000,
        "mem_total_bytes": 17179869184u64,
        "mem_available_bytes": 10737418240u64,
        "psi_cpu_avg10": 1.2,
        "psi_mem_avg10": 0.4,
        "psi_io_avg10": 0.1,
        "custody_records": 412,
        "watermark_lag": 1,
        "top_blame": "ctrl-01 (48 MiB)",
        "quic": gov_json["quic"],
        "canonical_endpoint": "[fd00:db8::4]:17440",
        "canonical_authority": "example-cluster-canonical",
        "canonical_online": true,
        "canonical_frontiers": { "ctrl-01": 99, "vault-01": 42 },
        "canonical_error": null
    });
    let sse_raw = serde_json::json!({ "governor": gov_json, "telemetry": tele_json }).to_string();
    let sse: SseEventDto = serde_json::from_str(&sse_raw).expect("parse SSE");
    let gov: GovernorResponseDto = sse.governor.expect("governor present");
    let tele: TelemetryDto = sse.telemetry.expect("telemetry present");

    let routes_raw = r#"{"tier":"Nominal","routes":[{"service":"control-plane","tab_title":"Ctrl · WASM","mode":"web","api_uri":"http://[::1]:8080/v1/governor","web_url":"http://[::1]:8080/","priority":"critical","cache_ttl_ms":2500}]}"#;
    let routes: UiRoutesResponseDto = serde_json::from_str(routes_raw).expect("parse routes");

    let owner = Owner::new();
    owner.with(|| {
        let store = CockpitStore::new_with_defaults();
        store.apply_telemetry(tele);
        store.apply_governor(gov);
        store.apply_ui_routes(routes);

        assert!(
            store
                .telemetry
                .with_untracked(|t| t.online && t.custody_records == 412)
        );
        assert!(store.telemetry.with_untracked(|t| {
            t.canonical_online && t.canonical_frontiers.get("ctrl-01") == Some(&99)
        }));
        assert_eq!(
            store.telemetry.with_untracked(|t| t.quic.streams_served),
            11
        );
        assert_eq!(store.ui_routes.with_untracked(|r| r.len()), 1);
    });
}

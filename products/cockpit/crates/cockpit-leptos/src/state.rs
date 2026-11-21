//! Fine-grained reactive store (`CockpitStore`) and re-exported DTO/domain types.

pub use crate::domain::{
    DEFAULT_PITCH_DEG, DEFAULT_YAW_DEG, DEFAULT_ZOOM, EnvKind, PressureTier, ProjectionKind,
    ServiceId,
};
pub use crate::dto::{
    ClusterSnapshotDto, GovernorCardDto, GovernorResponseDto, LogDto, QuicStatsDto,
    SecretsPostureDto, SseEventDto, TargetDto, TelemetryDto, UiRouteDto, UiRoutesResponseDto,
};
use crate::dto::{default_governor_cards, default_snapshot, default_telemetry, default_ui_routes};
use leptos::prelude::*;

/// Fine-grained reactive workspace store driving the Cockpit UI.
#[derive(Clone, Copy)]
pub struct CockpitStore {
    /// Active environment profile (`ExampleCluster`, `LocalHost`, `Simulation`).
    pub active_env: RwSignal<EnvKind>,
    /// Active stage resource filter.
    pub active_service: RwSignal<ServiceId>,
    /// Currently open service or wiki app tab (`None` = 2.5D Cluster Stage).
    pub active_tab: RwSignal<Option<ServiceId>>,
    /// Ordered open tabs pinned in the left rail.
    pub open_tabs: RwSignal<Vec<ServiceId>>,
    /// Selected cluster target URN for inspector & highlight ring.
    pub selected_urn: RwSignal<Option<String>>,
    /// Currently selected `cockpit-wiki` article route when viewing `DocsWiki`.
    pub active_wiki_route: RwSignal<String>,
    /// Active 2.5D vs 2D schematic projection mode.
    pub projection: RwSignal<ProjectionKind>,
    /// Stage horizontal azimuth orbit angle in degrees.
    pub yaw_deg: RwSignal<f32>,
    /// Stage elevation pitch angle in degrees.
    pub pitch_deg: RwSignal<f32>,
    /// Stage SVG viewport zoom scale (`0.55..=2.2`).
    pub zoom: RwSignal<f32>,
    /// Animation flag (`false` at settled `0 Hz` idle).
    pub animations_enabled: RwSignal<bool>,
    /// Active demo mode toggle (`false` at settled `0 Hz` idle).
    pub demo_running: RwSignal<bool>,
    /// UI density token mode (`"compact"` or `"spacious"`).
    pub density: RwSignal<&'static str>,
    /// Command palette modal open state (`false` when closed -> 0 work on ticks).
    pub palette_open: RwSignal<bool>,
    /// Live SSE connection state.
    pub stream_connected: RwSignal<bool>,
    /// Current environment DAG snapshot.
    pub snapshot: RwSignal<ClusterSnapshotDto>,
    /// Latest host & WAL custody telemetry.
    pub telemetry: RwSignal<TelemetryDto>,
    /// Latest governor class policy cards.
    pub governor_cards: RwSignal<Vec<GovernorCardDto>>,
    /// Registered UI/API forwarder routes.
    pub ui_routes: RwSignal<Vec<UiRouteDto>>,
    /// Dismissible live status banner text.
    pub status_banner: RwSignal<Option<String>>,
}

impl CockpitStore {
    /// Initializes a deterministic `0 Hz` settled idle store backed by `example-cluster` fixtures.
    pub fn new_with_defaults() -> Self {
        let initial_snap = default_snapshot("example-cluster", "ctrl-01 (SSR)", true);
        let initial_urn = initial_snap
            .targets
            .iter()
            .find(|t| t.urn == "cluster.node/ctrl-01")
            .or_else(|| initial_snap.targets.first())
            .map(|t| t.urn.clone());
        Self {
            active_env: RwSignal::new(EnvKind::ExampleCluster),
            active_service: RwSignal::new(ServiceId::All),
            active_tab: RwSignal::new(None),
            open_tabs: RwSignal::new(vec![
                ServiceId::ControlPlane,
                ServiceId::Telemetry,
                ServiceId::DocsWiki,
            ]),
            selected_urn: RwSignal::new(initial_urn),
            active_wiki_route: RwSignal::new("/wiki/cockpit-architecture".to_owned()),
            projection: RwSignal::new(ProjectionKind::Isometric25D),
            yaw_deg: RwSignal::new(DEFAULT_YAW_DEG),
            pitch_deg: RwSignal::new(DEFAULT_PITCH_DEG),
            zoom: RwSignal::new(DEFAULT_ZOOM),
            animations_enabled: RwSignal::new(false),
            demo_running: RwSignal::new(false),
            density: RwSignal::new("compact"),
            palette_open: RwSignal::new(false),
            stream_connected: RwSignal::new(false),
            snapshot: RwSignal::new(initial_snap),
            telemetry: RwSignal::new(default_telemetry()),
            governor_cards: RwSignal::new(default_governor_cards()),
            ui_routes: RwSignal::new(default_ui_routes()),
            status_banner: RwSignal::new(None),
        }
    }

    /// Switches active cluster environment and updates snapshot/telemetry fixtures deterministically.
    pub fn switch_env(&self, env: EnvKind) {
        self.active_env.set(env);
        let mut snap = match env {
            EnvKind::ExampleCluster => default_snapshot("example-cluster", "ctrl-01", true),
            EnvKind::LocalHost => default_snapshot("host-local", "host-procfs", true),
            EnvKind::Simulation => {
                let mut s = default_snapshot("sim-chaos", "sim-sandbox", false);
                if let Some(vault) = s
                    .targets
                    .iter_mut()
                    .find(|t| t.urn == "cluster.node/vault-01")
                {
                    vault.status = "Drifted".into();
                    vault.drift_count = Some(2);
                    vault.runbook_hint =
                        Some("Simulated WAL checkpoint lag; run Reconcile.".into());
                }
                s.logs.push(LogDto {
                    timestamp: "120ms".into(),
                    urn: "cluster.node/vault-01".into(),
                    level: "WARN".into(),
                    message: "simulated spec drift detected (2 attributes)".into(),
                });
                s
            }
        };
        if snap
            .targets
            .iter()
            .all(|t| Some(&t.urn) != self.selected_urn.get_untracked().as_ref())
        {
            let fallback = snap
                .targets
                .iter()
                .find(|t| t.urn == "cluster.node/ctrl-01")
                .or_else(|| snap.targets.first())
                .map(|t| t.urn.clone());
            self.selected_urn.set(fallback);
        }
        if env == EnvKind::Simulation {
            let mut tele = default_telemetry();
            tele.online = true;
            tele.source_desc = "sim-chaos · deterministic".into();
            tele.custody_records = 1_024;
            tele.watermark_lag = 3;
            tele.top_blame = "sim-worker (412 MiB)".into();
            tele.quic.streams_served = 14;
            tele.quic.rtt_us = 680;
            tele.quic.cwnd_bytes = 28_672;
            self.telemetry.set(tele);
            snap.source_label = "sim-chaos".into();
        }
        self.snapshot.set(snap);
        self.status_banner
            .set(Some(format!("Switched environment to {}", env.label())));
    }

    /// Opens or focuses a service/wiki tab in the workspace.
    pub fn open_service_tab(&self, svc: ServiceId) {
        if svc == ServiceId::All {
            self.active_service.set(ServiceId::All);
            self.active_tab.set(None);
            return;
        }
        self.open_tabs.update(|tabs| {
            if !tabs.contains(&svc) {
                tabs.push(svc);
            }
        });
        self.active_service.set(svc);
        self.active_tab.set(Some(svc));
    }

    /// Closes an open service tab and returns to stage if it was active.
    pub fn close_service_tab(&self, svc: ServiceId) {
        self.open_tabs.update(|tabs| tabs.retain(|&t| t != svc));
        if self.active_tab.get_untracked() == Some(svc) {
            self.active_tab.set(None);
        }
    }

    /// Toggles demo animation mode; turns off cleanly back to `0 Hz` settled idle.
    pub fn trigger_demo(&self) {
        let next = !self.demo_running.get_untracked();
        self.demo_running.set(next);
        self.animations_enabled.set(next);
        if next {
            self.snapshot.update(|s| {
                s.logs.push(LogDto {
                    timestamp: "demo".into(),
                    urn: "cluster.node/ctrl-01".into(),
                    level: "INFO".into(),
                    message: "Feature demo active: inspecting DAG propagation & governor telemetry"
                        .into(),
                });
            });
            self.status_banner.set(Some(
                "Feature demo running (toggle again to return to 0 Hz idle)".into(),
            ));
        } else {
            self.status_banner
                .set(Some("Feature demo stopped (0 Hz idle restored)".into()));
        }
    }

    /// Restores default 2:1 isometric orbit angles and zoom level.
    pub fn reset_orbit(&self) {
        self.yaw_deg.set(DEFAULT_YAW_DEG);
        self.pitch_deg.set(DEFAULT_PITCH_DEG);
        self.zoom.set(DEFAULT_ZOOM);
    }

    /// Returns the currently selected cluster target (falling back to primary node).
    pub fn selected_target(&self) -> Option<TargetDto> {
        let urn = self.selected_urn.get();
        self.snapshot.with(|snap| {
            if let Some(u) = urn.as_deref()
                && let Some(found) = snap.targets.iter().find(|t| t.urn == u)
            {
                return Some(found.clone());
            }
            snap.targets
                .iter()
                .find(|t| t.urn == "cluster.node/ctrl-01")
                .or_else(|| snap.targets.first())
                .cloned()
        })
    }

    /// Returns cluster targets matching `active_service`.
    pub fn filtered_targets(&self) -> Vec<TargetDto> {
        let svc = self.active_service.get();
        self.snapshot.with(|snap| {
            snap.targets
                .iter()
                .filter(|t| svc.matches_target(t))
                .cloned()
                .collect()
        })
    }

    /// Counts non-drifted/non-poisoned targets in the active snapshot.
    pub fn healthy_count(&self) -> usize {
        self.snapshot.with(|snap| {
            snap.targets
                .iter()
                .filter(|t| t.status != "Drifted" && t.status != "Poisoned")
                .count()
        })
    }

    /// Returns governor-enforced fallback polling period (`>= 5000ms`).
    pub fn poll_interval_ms(&self) -> u32 {
        self.governor_cards.with(|cards| {
            cards
                .iter()
                .find(|c| c.class_label == "cockpit-ui")
                .or_else(|| cards.first())
                .map_or(15_000, |c| c.poll_interval_ms.max(5_000))
        })
    }

    /// Applies an incoming snapshot update while preserving selection if valid.
    pub fn apply_snapshot(&self, snap: ClusterSnapshotDto) {
        if let Some(sel) = self.selected_urn.get_untracked()
            && !snap.targets.iter().any(|t| t.urn == sel)
        {
            self.selected_urn
                .set(snap.targets.first().map(|t| t.urn.clone()));
        }
        self.snapshot.set(snap);
    }

    /// Replaces host telemetry state with the latest sample.
    pub fn apply_telemetry(&self, tele: TelemetryDto) {
        self.telemetry.set(tele);
    }

    /// Replaces governor cards and updates QUIC summary when present.
    pub fn apply_governor(&self, gov: GovernorResponseDto) {
        if !gov.classes.is_empty() {
            self.governor_cards.set(gov.classes);
        }
        if !gov.quic.cert_fingerprint_hex.is_empty() || gov.quic.streams_served > 0 {
            self.telemetry.update(|t| t.quic = gov.quic);
        }
    }

    /// Replaces UI forwarder routes when non-empty.
    pub fn apply_ui_routes(&self, resp: UiRoutesResponseDto) {
        if !resp.routes.is_empty() {
            self.ui_routes.set(resp.routes);
        }
    }
}

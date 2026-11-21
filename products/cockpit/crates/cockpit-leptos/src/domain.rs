//! Domain enums (`EnvKind`, `ServiceId`, `ProjectionKind`, `PressureTier`) for `cockpit-leptos`.

use crate::dto::TargetDto;
use serde::{Deserialize, Serialize};

/// Default camera orbit azimuth angle in degrees.
pub const DEFAULT_YAW_DEG: f32 = 45.0;
/// Default isometric elevation pitch angle in degrees (`atan(0.5)`).
pub const DEFAULT_PITCH_DEG: f32 = 35.264;
/// Default neutral stage viewport zoom multiplier.
pub const DEFAULT_ZOOM: f32 = 1.0;

/// Selectable cluster environment profile in the left workspace rail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EnvKind {
    /// Deterministic reference `example-cluster` environment.
    ExampleCluster,
    /// Local host `/proc` & sysfs telemetry mode.
    LocalHost,
    /// Chaos / drift simulation sandbox environment.
    Simulation,
}

impl EnvKind {
    /// Complete ordered list of environment choices.
    pub const ALL: [Self; 3] = [Self::ExampleCluster, Self::LocalHost, Self::Simulation];

    /// Display label rendered in topbar and rail buttons.
    pub const fn label(self) -> &'static str {
        match self {
            Self::ExampleCluster => "Live · example-cluster",
            Self::LocalHost => "Local · host",
            Self::Simulation => "Testing · sim",
        }
    }

    /// Short project/profile identifier string.
    pub const fn project_id(self) -> &'static str {
        match self {
            Self::ExampleCluster => "example-cluster",
            Self::LocalHost => "host-local",
            Self::Simulation => "sim-chaos",
        }
    }

    /// Short badge label rendered beside the environment name.
    pub const fn badge(self) -> &'static str {
        match self {
            Self::ExampleCluster => "LIVE",
            Self::LocalHost => "LOCAL",
            Self::Simulation => "SIM",
        }
    }
}

/// Service planes and operational application views in the left rail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServiceId {
    /// Unfiltered cluster stage showing every zone and target.
    All,
    /// Control-plane governor and QUIC dataplane metrics.
    ControlPlane,
    /// Host telemetry, WAL custody, and blame attribution studio.
    Telemetry,
    /// Framed artifact / snapshot vault storage explorer.
    StorageVault,
    /// Encrypted IPv6 overlay mesh matrix.
    Mesh,
    /// Preemptible CI runner queue.
    BatchCi,
    /// Linux PSI / ZRAM memory and block volume surface.
    HostMem,
    /// Isolated WebAssembly / microVM execution sandbox posture.
    Sandbox,
    /// Embedded Markdown + Mermaid documentation wiki & release notes portal.
    DocsWiki,
}

impl ServiceId {
    /// All filterable stage scopes (`All` + infrastructure planes).
    pub const STAGE_FILTERS: [Self; 8] = [
        Self::All,
        Self::ControlPlane,
        Self::Telemetry,
        Self::StorageVault,
        Self::Mesh,
        Self::BatchCi,
        Self::HostMem,
        Self::Sandbox,
    ];

    /// All openable operational and documentation app tabs.
    pub const APP_SERVICES: [Self; 8] = [
        Self::ControlPlane,
        Self::Telemetry,
        Self::StorageVault,
        Self::Mesh,
        Self::BatchCi,
        Self::HostMem,
        Self::Sandbox,
        Self::DocsWiki,
    ];

    /// Human-readable service plane label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::All => "All Resources",
            Self::ControlPlane => "Control Plane",
            Self::Telemetry => "Telemetry Plane",
            Self::StorageVault => "Vault Storage",
            Self::Mesh => "Overlay Mesh",
            Self::BatchCi => "Spot CI",
            Self::HostMem => "Host Storage",
            Self::Sandbox => "WASM Sandbox",
            Self::DocsWiki => "Wiki & Docs",
        }
    }

    /// Stable route/tab slug identifier.
    pub const fn slug(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::ControlPlane => "control-plane",
            Self::Telemetry => "telemetry",
            Self::StorageVault => "vault-storage",
            Self::Mesh => "overlay-mesh",
            Self::BatchCi => "spot-ci",
            Self::HostMem => "storage-mem",
            Self::Sandbox => "sandbox",
            Self::DocsWiki => "wiki",
        }
    }

    /// Returns true if `target` belongs to this service filter scope.
    pub fn matches_target(self, target: &TargetDto) -> bool {
        let u = target.urn.as_str();
        match self {
            Self::All | Self::DocsWiki => true,
            Self::ControlPlane => {
                u.contains("ctrl-01")
                    || target.group == "Iam"
                    || target.group == "Apis"
                    || target.group == "Network"
            }
            Self::Telemetry => u.contains("obs-01"),
            Self::StorageVault => u.contains("vault") || u.contains("bucket"),
            Self::Mesh => target.group == "Mesh" || u.contains("fw-mesh"),
            Self::BatchCi => u.contains("ci-runner") || u.contains("batch"),
            Self::HostMem => target.group == "Storage" || u.contains("disk-"),
            Self::Sandbox => u.contains("sandbox") || u.contains("ctrl-01"),
        }
    }
}

/// Interactive topology projection preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProjectionKind {
    /// True 2:1 360-degree extruded 2.5D isometric campus view.
    Isometric25D,
    /// Flattened top-down 2D architectural schematic view.
    Schematic2D,
}

impl ProjectionKind {
    /// Human-readable projection mode label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Isometric25D => "2.5D Iso",
            Self::Schematic2D => "2D Schematic",
        }
    }

    /// Cycles between 2.5D isometric and 2D schematic modes.
    pub const fn toggle(self) -> Self {
        match self {
            Self::Isometric25D => Self::Schematic2D,
            Self::Schematic2D => Self::Isometric25D,
        }
    }
}

/// Resource pressure classification tier returned by fleet governor telemetry.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum PressureTier {
    /// Normal headroom with default cadence.
    #[default]
    Nominal,
    /// Slightly elevated usage requiring proactive flushing.
    Elevated,
    /// Constrained headroom throttling optional probes.
    Constrained,
    /// Critical pressure prioritizing core stability.
    Critical,
}

//! Generic Serde DTOs and deterministic `example-cluster` defaults for `cockpit-leptos`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use telemetry_scene::{example_cluster_bundle, mem_used_pct};

use crate::domain::PressureTier;

/// Monthly cost ceiling in cents shown in topbar budget telemetry.
pub const MONTHLY_BUDGET_LIMIT_CENTS: u32 = 25_000;

/// Single infrastructure or service target in the Cockpit DAG.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TargetDto {
    /// Canonical resource URN (e.g. `cluster.node/ctrl-01`).
    pub urn: String,
    /// Short display title.
    pub display_name: String,
    /// Asset kind string mapped onto procedural 2.5D sprites.
    pub kind_label: String,
    /// Logical group/zone name (`Compute`, `Storage`, `Mesh`, `Network`, `Iam`, `Apis`).
    pub group: String,
    /// Convergence / operational state (`Converged`, `InFlight`, `Drifted`, `Poisoned`, `Planned`).
    pub status: String,
    /// Generation counter incremented on reconcile.
    pub generation: u64,
    /// Short deterministic specification hash prefix.
    pub spec_hash_short: String,
    /// Monthly resource spend in cents.
    pub monthly_cents: u32,
    /// True if assigned external IPv6 connectivity.
    pub external_ipv6: bool,
    /// True if untrusted workloads are permitted credential access (always false in zero-trust tiers).
    pub allow_llm_secrets: bool,
    /// Scoped identity roles attached to this target.
    pub iam_roles: Vec<String>,
    /// Optional mesh or RPC socket endpoint.
    pub endpoint: Option<String>,
    /// Human-readable component role description.
    pub description: String,
    /// Optional duration of last apply in milliseconds.
    pub last_apply_ms: Option<u64>,
    /// Number of drifted attributes if any.
    pub drift_count: Option<usize>,
    /// Observed mesh RTT latency in milliseconds.
    pub latency_ms: Option<u64>,
    /// Upstream dependency URNs.
    pub upstream: Vec<String>,
    /// Downstream dependent URNs.
    pub downstream: Vec<String>,
    /// Serialized specification summary JSON.
    pub manifest_json: String,
    /// Optional remediation hint shown when drifted.
    pub runbook_hint: Option<String>,
}

/// Structured event log row.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LogDto {
    /// Timestamp or relative offset tag.
    pub timestamp: String,
    /// Subject URN.
    pub urn: String,
    /// Log severity (`OK`, `INFO`, `WARN`, `ERR`).
    pub level: String,
    /// Human-readable log detail.
    pub message: String,
}

/// Full snapshot of an infrastructure environment graph and recent event log.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ClusterSnapshotDto {
    /// Environment project identifier.
    pub project_id: String,
    /// Source descriptor shown in the UI.
    pub source_label: String,
    /// Aggregate monthly spend in cents across all targets.
    pub total_monthly_cents: u32,
    /// Target budget ceiling in cents.
    pub budget_limit_cents: u32,
    /// Ordered cluster targets.
    pub targets: Vec<TargetDto>,
    /// Recent control-plane logs.
    pub logs: Vec<LogDto>,
}

/// QUIC dataplane transport counters.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct QuicStatsDto {
    /// Active peer connections.
    pub active_connections: usize,
    /// Total served streams.
    pub streams_served: u64,
    /// Received datagram count.
    pub rx_datagrams: u64,
    /// Transmitted datagram count.
    pub tx_datagrams: u64,
    /// Smoothed RTT in microseconds.
    pub rtt_us: u64,
    /// Current congestion window in bytes.
    pub cwnd_bytes: u64,
    /// ECN Congestion Experienced packet count.
    pub ecn_ce_events: u64,
    /// Lost packet count.
    pub lost_packets: u64,
    /// Active peer certificate fingerprint hex.
    pub cert_fingerprint_hex: String,
}

/// Hardware attestation and credential isolation status.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SecretsPostureDto {
    /// True if hardware vTPM2 sealing is available.
    pub tpm2_available: bool,
    /// Sealed credential labels.
    pub sealed_creds: Vec<String>,
    /// Ephemeral runtime credentials held in memory only.
    pub runtime_creds: Vec<String>,
}

/// Live host & custody telemetry payload pushed over REST/SSE.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TelemetryDto {
    /// True when the upstream telemetry collector is reachable.
    pub online: bool,
    /// Telemetry endpoint source label.
    pub source_desc: String,
    /// Sample Unix timestamp in milliseconds.
    pub ts_ms: u64,
    /// Total installed physical RAM bytes.
    pub mem_total_bytes: u64,
    /// Available physical RAM bytes.
    pub mem_available_bytes: u64,
    /// Linux PSI CPU some avg10.
    pub psi_cpu_avg10: f64,
    /// Linux PSI memory some avg10.
    pub psi_mem_avg10: f64,
    /// Linux PSI I/O some avg10.
    pub psi_io_avg10: f64,
    /// Total durable WAL custody records.
    pub custody_records: u64,
    /// Unprojected watermark lag sequence count.
    pub watermark_lag: u64,
    /// Dominant resource attribution label.
    pub top_blame: String,
    /// QUIC dataplane telemetry.
    pub quic: QuicStatsDto,
    /// Canonical receiver endpoint string.
    pub canonical_endpoint: Option<String>,
    /// Canonical cluster authority UUID.
    pub canonical_authority: Option<String>,
    /// True if the canonical receiver is healthy.
    pub canonical_online: bool,
    /// Per-origin frontier offsets.
    pub canonical_frontiers: BTreeMap<String, u64>,
    /// Optional canonical error summary.
    pub canonical_error: Option<String>,
    /// Hardware secret isolation posture.
    pub secrets: SecretsPostureDto,
}

/// Governor policy summary card rendered per service class.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GovernorCardDto {
    /// Service class slug (`control-plane`, `cockpit-ui`, etc.).
    pub class_label: String,
    /// Current resource pressure tier.
    pub tier: PressureTier,
    /// Machine-readable reason code.
    pub reason: String,
    /// Composite pressure score (`0..=100`).
    pub composite_pressure: u16,
    /// Memory ratio in permille (`0..=1000`).
    pub mem_ratio_permille: u16,
    /// Memory slope in KiB/sec.
    pub mem_slope_kib_per_sec: i64,
    /// Estimated time before `memory.high` throttle.
    pub eta_mem_high_secs: Option<u32>,
    /// Enforced UI/service poll interval in milliseconds.
    pub poll_interval_ms: u32,
    /// Enforced concurrent worker cap.
    pub max_concurrent_agents: u8,
    /// Whether WAL/bundle flushes are expedited.
    pub flush_wal_and_bundles: bool,
    /// Whether low-priority probes are shed.
    pub shed_optional_probes: bool,
}

/// REST `/v1/governor` payload envelope.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GovernorResponseDto {
    /// Active governor cards across service classes.
    pub classes: Vec<GovernorCardDto>,
    /// Associated QUIC transport summary.
    pub quic: QuicStatsDto,
}

/// Forwarded UI/API route descriptor.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiRouteDto {
    /// Service plane slug.
    pub service: String,
    /// Human-readable tab label.
    pub tab_title: String,
    /// Routing mode (`web` or `api`).
    pub mode: String,
    /// Target API or sandbox URI.
    pub api_uri: String,
    /// Optional browser endpoint URL.
    pub web_url: Option<String>,
    /// Scheduling priority (`critical`, `standard`, `background`).
    pub priority: String,
    /// Recommended client cache TTL in milliseconds.
    pub cache_ttl_ms: u64,
}

/// REST `/v1/ui/routes` payload envelope.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct UiRoutesResponseDto {
    /// Active cluster pressure tier.
    pub tier: PressureTier,
    /// Registered UI/API routes.
    pub routes: Vec<UiRouteDto>,
}

/// SSE event message wrapping incremental governor and telemetry deltas.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct SseEventDto {
    /// Optional updated governor response.
    pub governor: Option<GovernorResponseDto>,
    /// Optional updated host telemetry sample.
    pub telemetry: Option<TelemetryDto>,
}

struct TargetSeed<'a> {
    urn: &'a str,
    name: &'a str,
    kind: &'a str,
    group: &'a str,
    cents: u32,
    hash: &'a str,
    endpoint: Option<&'a str>,
    latency_ms: Option<u64>,
    downstream: &'a [&'a str],
    desc: &'a str,
}

/// Builds a deterministic 12-node `example-cluster` snapshot for SSR & offline tests.
pub fn default_snapshot(
    project_id: &str,
    source_label: &str,
    converged: bool,
) -> ClusterSnapshotDto {
    let seeds = [
        TargetSeed {
            urn: "cluster.api/control",
            name: "control-api",
            kind: "EnabledApi",
            group: "Apis",
            cents: 0,
            hash: "a101b204",
            endpoint: Some("https://control.example.internal"),
            latency_ms: Some(2),
            downstream: &["cluster.net/mesh-vpc"],
            desc: "Declarative cluster reconciler control API",
        },
        TargetSeed {
            urn: "cluster.iam/controller-sa",
            name: "controller-sa",
            kind: "ServiceAccount",
            group: "Iam",
            cents: 0,
            hash: "b208c311",
            endpoint: None,
            latency_ms: None,
            downstream: &["cluster.node/ctrl-01", "cluster.node/obs-01"],
            desc: "Least-privilege control-plane service identity",
        },
        TargetSeed {
            urn: "cluster.net/mesh-vpc",
            name: "mesh-vpc",
            kind: "VpcNetwork",
            group: "Network",
            cents: 0,
            hash: "c309d422",
            endpoint: Some("fd00:db8::/48"),
            latency_ms: None,
            downstream: &["cluster.net/fw-mesh", "cluster.net/subnet-v6"],
            desc: "Dual-stack isolated overlay network",
        },
        TargetSeed {
            urn: "cluster.net/subnet-v6",
            name: "subnet-v6",
            kind: "Ipv6Subnet",
            group: "Network",
            cents: 0,
            hash: "d410e533",
            endpoint: Some("fd00:db8:1::/64"),
            latency_ms: None,
            downstream: &["cluster.node/ctrl-01", "cluster.node/vault-01"],
            desc: "IPv6 ULA subnet allocation",
        },
        TargetSeed {
            urn: "cluster.net/fw-mesh",
            name: "fw-mesh",
            kind: "FirewallRule",
            group: "Mesh",
            cents: 0,
            hash: "e512f644",
            endpoint: Some("udp/51820"),
            latency_ms: Some(1),
            downstream: &["wg.peer/ctrl-01", "wg.peer/obs-01"],
            desc: "Ingress policy permitting encrypted WireGuard UDP/51820",
        },
        TargetSeed {
            urn: "cluster.node/ctrl-01",
            name: "ctrl-01",
            kind: "ComputeInstance",
            group: "Compute",
            cents: 4_800,
            hash: "f6130755",
            endpoint: Some("[fd00:db8::1]:51820"),
            latency_ms: Some(4),
            downstream: &[
                "cluster.disk/disk-ctrl",
                "cluster.node/vault-01",
                "cluster.node/obs-01",
            ],
            desc: "Primary control-plane & SSR cockpit host",
        },
        TargetSeed {
            urn: "cluster.node/vault-01",
            name: "vault-01",
            kind: "ComputeInstance",
            group: "Compute",
            cents: 4_200,
            hash: "07141866",
            endpoint: Some("[fd00:db8::2]:51820"),
            latency_ms: Some(8),
            downstream: &["cluster.disk/disk-vault", "cluster.store/bucket-archive"],
            desc: "Framed content-addressed bundle storage node",
        },
        TargetSeed {
            urn: "cluster.node/obs-01",
            name: "obs-01",
            kind: "ComputeInstance",
            group: "Compute",
            cents: 4_400,
            hash: "18152977",
            endpoint: Some("[fd00:db8::4]:17440"),
            latency_ms: Some(6),
            downstream: &["cluster.disk/disk-obs"],
            desc: "Canonical telemetry receiver and custody sink",
        },
        TargetSeed {
            urn: "cluster.batch/ci-runner",
            name: "ci-runner",
            kind: "BatchJobTemplate",
            group: "Compute",
            cents: 1_200,
            hash: "29163a88",
            endpoint: Some("wasmtime://fd00:db8::3/sandbox"),
            latency_ms: Some(11),
            downstream: &[],
            desc: "Ephemeral Wasmtime sandboxed verification runner",
        },
        TargetSeed {
            urn: "cluster.disk/disk-ctrl",
            name: "disk-ctrl",
            kind: "PersistentDisk",
            group: "Storage",
            cents: 600,
            hash: "3a174b99",
            endpoint: None,
            latency_ms: None,
            downstream: &[],
            desc: "Control-plane WAL & snapshot disk",
        },
        TargetSeed {
            urn: "cluster.disk/disk-vault",
            name: "disk-vault",
            kind: "PersistentDisk",
            group: "Storage",
            cents: 1_400,
            hash: "4b185caa",
            endpoint: None,
            latency_ms: None,
            downstream: &[],
            desc: "Replication pack storage block volume",
        },
        TargetSeed {
            urn: "cluster.disk/disk-obs",
            name: "disk-obs",
            kind: "PersistentDisk",
            group: "Storage",
            cents: 1_200,
            hash: "5c196dbb",
            endpoint: None,
            latency_ms: None,
            downstream: &[],
            desc: "Time-series and custody index disk",
        },
        TargetSeed {
            urn: "cluster.store/bucket-archive",
            name: "bucket-archive",
            kind: "DriveBucketFolder",
            group: "Storage",
            cents: 400,
            hash: "6d207ecc",
            endpoint: Some("s3://example-cluster-archive"),
            latency_ms: None,
            downstream: &[],
            desc: "Cold immutable bundle archive bucket",
        },
        TargetSeed {
            urn: "wg.peer/ctrl-01",
            name: "peer-ctrl-01",
            kind: "WireGuardPeer",
            group: "Mesh",
            cents: 0,
            hash: "7e218fdd",
            endpoint: Some("[fd00:db8::1]:51820"),
            latency_ms: Some(3),
            downstream: &["wg.peer/obs-01"],
            desc: "Encrypted ChaCha20-Poly1305 mesh peer",
        },
        TargetSeed {
            urn: "wg.peer/obs-01",
            name: "peer-obs-01",
            kind: "WireGuardPeer",
            group: "Mesh",
            cents: 0,
            hash: "8f2290ee",
            endpoint: Some("[fd00:db8::4]:51820"),
            latency_ms: Some(5),
            downstream: &[],
            desc: "Telemetry sink WireGuard mesh peer",
        },
    ];

    let mut targets: Vec<TargetDto> = seeds
        .iter()
        .map(|s| TargetDto {
            urn: s.urn.to_owned(),
            display_name: s.name.to_owned(),
            kind_label: s.kind.to_owned(),
            group: s.group.to_owned(),
            status: if converged {
                "Converged".into()
            } else {
                "Planned".into()
            },
            generation: u64::from(converged),
            spec_hash_short: s.hash.to_owned(),
            monthly_cents: s.cents,
            external_ipv6: s.group == "Network",
            allow_llm_secrets: false,
            iam_roles: vec!["viewer".into()],
            endpoint: s.endpoint.map(str::to_owned),
            description: s.desc.to_owned(),
            last_apply_ms: if converged { Some(320) } else { None },
            drift_count: if converged { Some(0) } else { None },
            latency_ms: s.latency_ms,
            upstream: Vec::new(),
            downstream: s.downstream.iter().map(|d| (*d).to_owned()).collect(),
            manifest_json: format!(
                "{{\"urn\":\"{}\",\"kind\":\"{}\",\"spec_hash\":\"{}\"}}",
                s.urn, s.kind, s.hash
            ),
            runbook_hint: None,
        })
        .collect();

    for i in 0..targets.len() {
        let down = targets[i].downstream.clone();
        let src_urn = targets[i].urn.clone();
        for d in down {
            if let Some(child) = targets.iter_mut().find(|t| t.urn == d) {
                child.upstream.push(src_urn.clone());
            }
        }
    }

    let total_monthly_cents: u32 = targets.iter().map(|t| t.monthly_cents).sum();
    ClusterSnapshotDto {
        project_id: project_id.to_owned(),
        source_label: source_label.to_owned(),
        total_monthly_cents,
        budget_limit_cents: MONTHLY_BUDGET_LIMIT_CENTS,
        targets,
        logs: vec![LogDto {
            timestamp: "0ms".into(),
            urn: "cluster.node/ctrl-01".into(),
            level: "INFO".into(),
            message: format!("initialized {source_label} topology ({project_id})"),
        }],
    }
}

/// Builds default host telemetry derived cleanly from `telemetry_scene::example_cluster_bundle`.
pub fn default_telemetry() -> TelemetryDto {
    let bundle = example_cluster_bundle();
    let snap = bundle.snapshot.as_ref();
    let custody = bundle.custody.as_ref();
    let mem_pct = snap.map_or(0.0, |s| mem_used_pct(&s.memory));
    let _ = mem_pct;
    TelemetryDto {
        online: false,
        source_desc: bundle.source_desc,
        ts_ms: snap.map_or(0, |s| s.timestamp_unix_ms),
        mem_total_bytes: snap.map_or(16 * 1024 * 1024 * 1024, |s| s.memory.mem_total_bytes),
        mem_available_bytes: snap.map_or(11 * 1024 * 1024 * 1024, |s| s.memory.mem_available_bytes),
        psi_cpu_avg10: snap.map_or(0.4, |s| s.psi.cpu_some.avg10),
        psi_mem_avg10: snap.map_or(0.2, |s| s.psi.memory_some.avg10),
        psi_io_avg10: snap.map_or(0.1, |s| s.psi.io_some.avg10),
        custody_records: custody.map_or(0, |c| c.records),
        watermark_lag: custody.map_or(0, telemetry_scene::custody_lag),
        top_blame: bundle
            .blame_memory
            .and_then(|b| b.entries.into_iter().next())
            .map_or_else(|| "none".into(), |e| e.label),
        quic: QuicStatsDto::default(),
        canonical_endpoint: custody.and_then(|c| c.server_endpoint.clone()),
        canonical_authority: custody.and_then(|c| c.server_authority.clone()),
        canonical_online: custody.is_some_and(|c| c.server_online),
        canonical_frontiers: custody.map_or_else(BTreeMap::new, |c| c.server_frontiers.clone()),
        canonical_error: None,
        secrets: SecretsPostureDto {
            tpm2_available: true,
            sealed_creds: vec!["mesh-key".into()],
            runtime_creds: vec!["ephemeral-session".into()],
        },
    }
}

/// Builds deterministic governor policy cards for all five service classes.
pub fn default_governor_cards() -> Vec<GovernorCardDto> {
    [
        ("control-plane", 5_000, 2),
        ("vault-storage", 5_000, 2),
        ("telemetry-plane", 5_000, 1),
        ("wasm-sandbox", 5_000, 2),
        ("cockpit-ui", 15_000, 2),
    ]
    .into_iter()
    .map(
        |(class_label, poll_interval_ms, max_concurrent_agents)| GovernorCardDto {
            class_label: class_label.into(),
            tier: PressureTier::Nominal,
            reason: "nominal".into(),
            composite_pressure: 18,
            mem_ratio_permille: 240,
            mem_slope_kib_per_sec: 0,
            eta_mem_high_secs: None,
            poll_interval_ms,
            max_concurrent_agents,
            flush_wal_and_bundles: false,
            shed_optional_probes: false,
        },
    )
    .collect()
}

/// Builds default UI/API route table using documentation IPv6 ULA (`fd00:db8::/64`) and `wasmtime://`.
pub fn default_ui_routes() -> Vec<UiRouteDto> {
    vec![
        route(
            "control-plane",
            "Ctrl · WASM",
            "web",
            "http://[::1]:8080/v1/governor",
            Some("http://[::1]:8080/"),
            "critical",
            2_500,
        ),
        route(
            "telemetry",
            "Telemetry Studio",
            "api",
            "telemetry://[fd00:db8::4]:17440/custody",
            None,
            "critical",
            4_000,
        ),
        route(
            "vault-storage",
            "Vault Explorer",
            "api",
            "http://[::1]:8081/v1/stats",
            None,
            "standard",
            5_000,
        ),
        route(
            "overlay-mesh",
            "Overlay Mesh",
            "api",
            "wg://fd00:db8::/64:51820",
            None,
            "standard",
            6_000,
        ),
        route(
            "spot-ci",
            "Spot CI Queue",
            "api",
            "http://[::1]:8080/health",
            None,
            "background",
            5_000,
        ),
        route(
            "storage-mem",
            "Host Storage",
            "api",
            "procfs://localhost/meminfo",
            None,
            "standard",
            3_000,
        ),
        route(
            "sandbox",
            "WASM Sandbox",
            "api",
            "wasmtime://fd00:db8::3/sandbox",
            None,
            "background",
            8_000,
        ),
    ]
}

fn route(
    service: &str,
    tab_title: &str,
    mode: &str,
    api_uri: &str,
    web_url: Option<&str>,
    priority: &str,
    cache_ttl_ms: u64,
) -> UiRouteDto {
    UiRouteDto {
        service: service.into(),
        tab_title: tab_title.into(),
        mode: mode.into(),
        api_uri: api_uri.into(),
        web_url: web_url.map(str::to_owned),
        priority: priority.into(),
        cache_ttl_ms,
    }
}

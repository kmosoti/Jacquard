//! Generic JSON telemetry and cluster topology schemas.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub use crate::schema_ext::{GpuMetric, LineageEdge, LineageGraph, LineageNode};

/// Complete host telemetry bundle rendered by the Cockpit.
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct TelemetryBundle {
    /// Whether the telemetry source is currently reachable.
    pub online: bool,
    /// Human-readable description of the telemetry source.
    pub source_desc: String,
    /// Optional collection or transport error summary.
    pub error: Option<String>,
    /// Non-fatal operational warnings.
    pub warnings: Vec<String>,
    /// Point-in-time host kernel and cgroup snapshot.
    pub snapshot: Option<HostSnapshot>,
    /// Log custody and canonical frontier metrics.
    pub custody: Option<CustodyMetrics>,
    /// Attribution blame summary for dominant resource pressure.
    pub blame_memory: Option<BlameReport>,
    /// Configuration or runtime policy drift findings.
    pub drift: Option<DriftReport>,
    /// Self-diagnostics and resource ceiling checks.
    pub doctor: Option<DoctorReport>,
    /// Historical time series samples `(unix_ms, value)`.
    pub metric_series: Vec<(u64, f64)>,
}

/// Point-in-time host telemetry snapshot.
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct HostSnapshot {
    /// Capture timestamp in Unix milliseconds.
    pub timestamp_unix_ms: u64,
    /// Host identifier shown in scene titles and cards.
    pub hostname: String,
    /// Linux PSI (Pressure Stall Information) metrics.
    pub psi: PsiMetrics,
    /// Memory and swap/zram counters.
    pub memory: MemoryMetrics,
    /// Filesystem mount utilization list.
    pub mounts: Vec<MountUsage>,
    /// Cgroup v2 slice accounting entries.
    pub slices: Vec<SliceUsage>,
    /// Tracked workload processes / agents.
    pub agents: Vec<AgentProcess>,
    /// Optional GPU telemetry list.
    pub gpus: Vec<GpuMetric>,
    /// Telemetry collector self-health snapshot.
    pub self_health: SelfHealth,
}

/// CPU, memory, and I/O PSI windows.
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct PsiMetrics {
    /// Some tasks stalled on CPU.
    pub cpu_some: PsiStat,
    /// All runnable tasks stalled on CPU.
    pub cpu_full: PsiStat,
    /// Some tasks stalled on memory.
    pub memory_some: PsiStat,
    /// All runnable tasks stalled on memory.
    pub memory_full: PsiStat,
    /// Some tasks stalled on I/O.
    pub io_some: PsiStat,
    /// All runnable tasks stalled on I/O.
    pub io_full: PsiStat,
}

/// Rolling PSI averages (`avg10`, `avg60`, `avg300`) and cumulative stall microseconds.
#[derive(Debug, Clone, Copy, PartialEq, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct PsiStat {
    /// 10-second stall percentage.
    pub avg10: f64,
    /// 60-second stall percentage.
    pub avg60: f64,
    /// 300-second stall percentage.
    pub avg300: f64,
    /// Cumulative stall duration in microseconds.
    pub total_usec: u64,
}

/// Host memory and optional zram usage counters.
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct MemoryMetrics {
    /// Total installed physical memory in bytes.
    pub mem_total_bytes: u64,
    /// Available physical memory in bytes.
    pub mem_available_bytes: u64,
    /// Total configured swap space in bytes.
    pub swap_total_bytes: u64,
    /// Active swap utilization in bytes.
    pub swap_used_bytes: u64,
    /// Optional zram swap device telemetry.
    pub zram: Option<ZramMetrics>,
}

/// Zram compressed memory swap device counters.
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ZramMetrics {
    /// Block device name (for example `zram0`).
    pub device: String,
    /// Uncompressed data size stored in zram in bytes.
    pub orig_data_size_bytes: u64,
    /// Compressed data size stored in RAM in bytes.
    pub compr_data_size_bytes: u64,
    /// Total memory consumed including allocator metadata.
    pub mem_used_total_bytes: u64,
    /// Optional hard RAM ceiling for zram.
    pub mem_limit_bytes: u64,
    /// Ratio of `orig_data_size` to `compr_data_size`.
    pub compression_ratio: f64,
    /// Zero/same-filled page deduplication count.
    pub same_pages: u64,
    /// Total compacted pages count.
    pub pages_compacted: u64,
}

/// Filesystem mount capacity and utilization snapshot.
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct MountUsage {
    /// Mount path (for example `/`).
    pub mount_point: String,
    /// Filesystem type (for example `btrfs`, `ext4`).
    pub fs_type: String,
    /// Total filesystem capacity in bytes.
    pub total_bytes: u64,
    /// Remaining available bytes.
    pub available_bytes: u64,
    /// Currently allocated bytes.
    pub used_bytes: u64,
    /// Utilization percentage (`0.0..=100.0`).
    pub used_pct: f64,
}

/// Cgroup v2 slice accounting summary.
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct SliceUsage {
    /// Slice unit name (for example `workload.slice`).
    pub slice_name: String,
    /// Full cgroup path.
    pub cgroup_path: String,
    /// Current resident + page cache memory footprint in bytes.
    pub memory_current_bytes: u64,
    /// Optional soft throttle watermark (`memory.high`).
    pub memory_high_bytes: Option<u64>,
    /// Optional hard limit (`memory.max`).
    pub memory_max_bytes: Option<u64>,
    /// Historical peak memory usage in bytes.
    pub memory_peak_bytes: Option<u64>,
    /// Cumulative CPU time consumed in microseconds.
    pub cpu_usage_usec: u64,
    /// Cumulative bytes read from storage.
    pub io_read_bytes: u64,
    /// Cumulative bytes written to storage.
    pub io_write_bytes: u64,
    /// Current PID count within this slice.
    pub pids_current: u64,
    /// Count of `memory.high` throttle events.
    pub memory_high_events: u64,
    /// Count of OOM kill events.
    pub oom_events: u64,
}

/// Tracked process / workload agent attributed to a cgroup slice.
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct AgentProcess {
    /// Logical session or service instance identifier.
    pub session_id: String,
    /// Short worker/agent name.
    pub agent_name: String,
    /// Containing cgroup slice name.
    pub slice: String,
    /// Systemd unit or scope name.
    pub unit: String,
    /// Active PID list.
    pub pids: Vec<u64>,
    /// Aggregate RSS bytes.
    pub rss_bytes: u64,
    /// Cumulative write bytes.
    pub write_bytes: u64,
    /// Filesystem prefixes accessed recently.
    pub touched_paths: Vec<String>,
}

/// Collector self-confinement and health telemetry.
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct SelfHealth {
    /// Collector process ID.
    pub pid: u64,
    /// Resident set size in bytes.
    pub rss_bytes: u64,
    /// Open file descriptor count.
    pub open_fds: u64,
    /// Temporary files created since startup.
    pub tmp_files_created: u64,
    /// Collector cgroup path.
    pub cgroup_path: String,
    /// True if the collector is confined inside its dedicated slice.
    pub slice_confined: bool,
    /// Total dropped telemetry events.
    pub dropped_events_total: u64,
    /// True when memory pressure shed mode is active.
    pub self_shedding_active: bool,
}

/// Append-only WAL custody and replication frontier metrics.
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct CustodyMetrics {
    /// Local custody UNIX socket or IPC endpoint.
    pub spindle_socket: String,
    /// Whether the local custody engine is online.
    pub spindle_online: bool,
    /// Origin identifier for the local custody node.
    pub spindle_origin: Option<String>,
    /// Optional custody error message.
    pub spindle_error: Option<String>,
    /// Total persisted WAL records.
    pub records: u64,
    /// Highest committed record ID locally.
    pub highest_record_id: u64,
    /// Highest record ID confirmed on canonical sink.
    pub canonical_through: u64,
    /// Highest compacted/reclaimed record ID.
    pub reclaimed_through: u64,
    /// Optional forwarding transport error.
    pub forwarding_error: Option<String>,
    /// Upstream canonical receiver endpoint.
    pub server_endpoint: Option<String>,
    /// Whether the canonical receiver is reachable.
    pub server_online: bool,
    /// Canonical cluster authority UUID.
    pub server_authority: Option<String>,
    /// Optional canonical receiver error.
    pub server_error: Option<String>,
    /// Per-origin durable frontiers known to the canonical receiver.
    pub server_frontiers: BTreeMap<String, u64>,
    /// Collector watermark checkpoint record ID.
    pub collector_spindle_watermark_id: Option<u64>,
    /// Optional catalog query error.
    pub catalog_error: Option<String>,
}

/// Blame attribution report for a host resource dimension.
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct BlameReport {
    /// Attributed resource name (`memory`, `io`, `cpu`).
    pub resource: String,
    /// Aggregate observed quantity.
    pub total_observed: u64,
    /// Unit label (`bytes`, `usec`, etc.).
    pub unit_label: String,
    /// Concise summary of current headroom and pressure.
    pub pressure_summary: String,
    /// Top ranked contributors.
    pub entries: Vec<BlameEntry>,
}

/// Single contributor row inside a [`BlameReport`].
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct BlameEntry {
    /// One-based rank order.
    pub rank: usize,
    /// Unique contributor identifier.
    pub node_id: String,
    /// Display label.
    pub label: String,
    /// Absolute resource contribution.
    pub contribution_value: u64,
    /// Measurement unit label.
    pub unit_label: String,
    /// Share of observed resource (`0.0..=100.0`).
    pub share_pct: f64,
    /// Optional OS process ID.
    pub pid: Option<u64>,
    /// Optional process command name.
    pub comm: Option<String>,
    /// Optional unit name.
    pub unit: Option<String>,
    /// Optional cgroup slice.
    pub slice: Option<String>,
    /// Optional workload/agent tag.
    pub agent: Option<String>,
    /// Optional dominant path involved.
    pub target_path: Option<String>,
    /// Causal lineage path strings.
    pub lineage_chain: Vec<String>,
}

/// Drift detection summary.
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct DriftReport {
    /// Total findings across all severities.
    pub total_findings: usize,
    /// Critical finding count.
    pub critical_count: usize,
    /// Warning finding count.
    pub warn_count: usize,
    /// Informational finding count.
    pub info_count: usize,
    /// Detailed findings.
    pub findings: Vec<DriftFinding>,
}

/// Individual policy/runtime drift item.
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct DriftFinding {
    /// Subsystem pillar (`lineage`, `storage`, `network`, etc.).
    pub pillar: String,
    /// Severity string (`critical`, `warn`, `info`).
    pub severity: String,
    /// Subject identifier.
    pub target: String,
    /// Human-readable description.
    pub summary: String,
    /// Observed value.
    pub metric_value: f64,
    /// Expected threshold value.
    pub threshold_value: f64,
}

/// Self-check diagnostic report for the telemetry agent.
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct DoctorReport {
    /// Aggregate health pass/fail.
    pub healthy: bool,
    /// Whether the collector daemon is running.
    pub daemon_running: bool,
    /// Collector PID if active.
    pub pid: Option<u64>,
    /// Current collector RSS bytes.
    pub rss_bytes: u64,
    /// Configured RSS budget ceiling in bytes.
    pub rss_budget_bytes: u64,
    /// Currently open file descriptors.
    pub open_fds: u64,
    /// Maximum allowed file descriptors.
    pub max_open_fds_budget: u64,
    /// Temporary files created.
    pub tmp_files_created: u64,
    /// Whether the daemon is cgroup-confined.
    pub slice_confined: bool,
    /// Data directory path.
    pub data_dir: String,
    /// Scratch/temp directory path.
    pub tmp_dir: String,
    /// Active cgroup slice path.
    pub cgroup_path: String,
    /// Total dropped telemetry events.
    pub dropped_events_total: u64,
    /// Shedding memory watermark in bytes.
    pub watchdog_shedding_threshold_bytes: u64,
    /// Whether shed mode is currently engaged.
    pub self_shedding_active: bool,
    /// Individual check outcomes.
    pub checks: Vec<DoctorCheck>,
}

/// Single diagnostic check line in a [`DoctorReport`].
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct DoctorCheck {
    /// Check identifier.
    pub name: String,
    /// True if check passed.
    pub passed: bool,
    /// Observed value formatted for display.
    pub actual: String,
    /// Target budget/threshold formatted for display.
    pub threshold: String,
    /// Diagnostic note.
    pub detail: String,
}

/// Asset primitive hints for generic [`ClusterTopology`] JSON projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TelemetryAssetKind {
    /// Multi-blade server rack.
    #[default]
    Rack,
    /// Storage cylinder / disk drum.
    Drum,
    /// Backup / cartridge bin crate.
    Crate,
    /// Masonry security firewall.
    Firewall,
    /// Network switch.
    Switch,
    /// Gateway router.
    Router,
    /// Workstation / operator client.
    Laptop,
    /// Cryptographic / TLS lock.
    Lock,
    /// Enclave / hardware chip.
    Chip,
    /// External WAN cloud.
    Cloud,
    /// Compact compute pod.
    Pod,
    /// Load balancer.
    Balancer,
}

/// Declarative multi-node cluster topology JSON input.
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ClusterTopology {
    /// Scene title.
    pub title: String,
    /// Logical clusters / bays containing nodes.
    pub nodes: Vec<ClusterNode>,
    /// Directed connections between nodes.
    pub links: Vec<ClusterLink>,
}

/// Single node inside a [`ClusterTopology`].
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ClusterNode {
    /// Unique slug identifier.
    pub slug: String,
    /// Display title.
    pub title: String,
    /// Optional concise metric subtitle.
    pub subtitle: Option<String>,
    /// Logical floor zone name (`Control Plane`, `Storage`, etc.).
    pub zone: String,
    /// Sprite kind.
    pub kind: TelemetryAssetKind,
    /// Operational health (`healthy`, `active`, `degraded`, `failed`, `pending`).
    pub status: String,
}

/// Directed link inside a [`ClusterTopology`].
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ClusterLink {
    /// Source node slug.
    pub from: String,
    /// Destination node slug.
    pub to: String,
    /// Optional edge label.
    pub label: Option<String>,
    /// Link style (`solid`, `dashed`, `dotted`).
    pub style: String,
}

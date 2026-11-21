//! Extended telemetry schema types (GPU, causal lineage) and formatting helpers.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::schema::{CustodyMetrics, MemoryMetrics};

/// Optional GPU utilization sample.
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct GpuMetric {
    /// DRM card identifier (for example `card0`).
    pub card: String,
    /// Device vendor string.
    pub vendor: String,
    /// Optional model name.
    pub name: Option<String>,
    /// Allocated VRAM in bytes.
    pub vram_used_bytes: Option<u64>,
    /// Total VRAM capacity in bytes.
    pub vram_total_bytes: Option<u64>,
    /// Engine busy percentage.
    pub busy_pct: Option<f64>,
    /// Temperature in degrees Celsius.
    pub temperature_c: Option<f64>,
}

/// Causal lineage graph extracted for a traced target.
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct LineageGraph {
    /// Query target subject.
    pub target: String,
    /// Matched root entities.
    pub matched_roots: Vec<String>,
    /// Traversal direction (`upstream` or `downstream`).
    pub direction: String,
    /// Traversal depth limit.
    pub max_depth: usize,
    /// Graph vertices.
    pub nodes: Vec<LineageNode>,
    /// Directed causal edges.
    pub edges: Vec<LineageEdge>,
}

/// Single vertex inside a [`LineageGraph`].
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct LineageNode {
    /// Node identifier.
    pub id: String,
    /// Semantic entity kind (`process`, `slice`, `mount`, `socket`).
    pub kind: String,
    /// Display label.
    pub label: String,
    /// Extensible metadata properties.
    pub metadata: BTreeMap<String, serde_json::Value>,
}

/// Directed edge inside a [`LineageGraph`].
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct LineageEdge {
    /// Source node ID.
    pub src: String,
    /// Destination node ID.
    pub dst: String,
    /// Relationship kind.
    pub kind: String,
    /// Relative attribution weight.
    pub weight: f64,
    /// Human-readable edge note.
    pub note: String,
}

/// Formats byte counts compactly (`GiB`, `MiB`, `KiB`).
pub fn format_bytes_iec(bytes: u64) -> String {
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    const MIB: f64 = 1024.0 * 1024.0;
    const KIB: f64 = 1024.0;
    let b = bytes as f64;
    if b >= GIB {
        format!("{:.1} GiB", b / GIB)
    } else if b >= MIB {
        format!("{:.0} MiB", b / MIB)
    } else if b >= KIB {
        format!("{:.0} KiB", b / KIB)
    } else {
        format!("{bytes} B")
    }
}

/// Computes used RAM percentage in `[0.0, 100.0]`.
pub fn mem_used_pct(mem: &MemoryMetrics) -> f64 {
    if mem.mem_total_bytes == 0 {
        return 0.0;
    }
    let used = mem.mem_total_bytes.saturating_sub(mem.mem_available_bytes);
    (used as f64 / mem.mem_total_bytes as f64) * 100.0
}

/// Returns unprojected record lag between highest local record ID and canonical receiver.
pub fn custody_lag(custody: &CustodyMetrics) -> u64 {
    custody
        .highest_record_id
        .saturating_sub(custody.canonical_through)
}

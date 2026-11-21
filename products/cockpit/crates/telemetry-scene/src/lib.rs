#![forbid(unsafe_code)]
//! Generic pure-Rust telemetry JSON-to-2.5D isometric scene projector.
//!
//! Converts host telemetry JSON bundles and cluster topology descriptors into
//! [`iso_diagram::model::Diagram`] scenes with zero runtime CLI or async IO
//! dependencies, enabling use in both native SSR servers and WASM browsers.

pub mod mock;
pub mod project;
pub mod schema;
pub mod schema_ext;

pub use mock::{example_cluster_bundle, example_cluster_topology};
pub use project::{
    cluster_to_diagram, parse_cluster_json, parse_telemetry_json, telemetry_to_diagram,
    telemetry_to_diagram_themed,
};
pub use schema::{
    AgentProcess, BlameEntry, BlameReport, ClusterLink, ClusterNode, ClusterTopology,
    CustodyMetrics, DoctorCheck, DoctorReport, DriftFinding, DriftReport, HostSnapshot,
    MemoryMetrics, MountUsage, PsiMetrics, PsiStat, SelfHealth, SliceUsage, TelemetryAssetKind,
    TelemetryBundle, ZramMetrics,
};
pub use schema_ext::{
    GpuMetric, LineageEdge, LineageGraph, LineageNode, custody_lag, format_bytes_iec, mem_used_pct,
};

//! Deterministic fixtures representing a generic `example-cluster` environment.

use std::collections::BTreeMap;

use crate::schema::{
    AgentProcess, BlameEntry, BlameReport, ClusterLink, ClusterNode, ClusterTopology,
    CustodyMetrics, DoctorCheck, DoctorReport, DriftFinding, DriftReport, HostSnapshot,
    MemoryMetrics, MountUsage, PsiMetrics, PsiStat, SelfHealth, SliceUsage, TelemetryAssetKind,
    TelemetryBundle, ZramMetrics,
};

/// Builds a deterministic host telemetry snapshot for tests and offline previews.
pub fn example_cluster_bundle() -> TelemetryBundle {
    let mut frontiers = BTreeMap::new();
    frontiers.insert("origin-example-01".to_string(), 130);

    let mut metric_series = Vec::with_capacity(16);
    for i in 0..16u64 {
        let avail_gib = 24.3 + ((i % 4) as f64) * 0.12 - (i as f64) * 0.02;
        metric_series.push((
            1_795_112_200_000 + i * 30_000,
            avail_gib * 1024.0 * 1024.0 * 1024.0,
        ));
    }

    TelemetryBundle {
        online: true,
        source_desc: "telemetry@example-node-01 (fixture)".into(),
        error: None,
        warnings: Vec::new(),
        snapshot: Some(HostSnapshot {
            timestamp_unix_ms: 1_795_113_123_300,
            hostname: "example-node-01".into(),
            psi: PsiMetrics {
                cpu_some: PsiStat {
                    avg10: 9.08,
                    avg60: 12.2,
                    avg300: 12.97,
                    total_usec: 13_193_003_794,
                },
                io_some: PsiStat {
                    avg10: 0.78,
                    avg60: 0.91,
                    avg300: 0.62,
                    total_usec: 938_720_150,
                },
                ..PsiMetrics::default()
            },
            memory: MemoryMetrics {
                mem_total_bytes: 33_489_129_472,
                mem_available_bytes: 26_103_709_696,
                swap_total_bytes: 8_589_930_496,
                swap_used_bytes: 3_795_132_416,
                zram: Some(ZramMetrics {
                    device: "zram0".into(),
                    orig_data_size_bytes: 3_515_707_392,
                    compr_data_size_bytes: 1_135_854_598,
                    mem_used_total_bytes: 1_178_206_208,
                    mem_limit_bytes: 0,
                    compression_ratio: 3.10,
                    same_pages: 25_366,
                    pages_compacted: 143_257,
                }),
            },
            mounts: vec![
                MountUsage {
                    mount_point: "/".into(),
                    fs_type: "btrfs".into(),
                    total_bytes: 509_315_383_296,
                    available_bytes: 286_151_041_024,
                    used_bytes: 214_501_269_504,
                    used_pct: 42.1,
                },
                MountUsage {
                    mount_point: "/var/lib/telemetry".into(),
                    fs_type: "ext4".into(),
                    total_bytes: 490_984_882_176,
                    available_bytes: 234_425_491_456,
                    used_bytes: 251_542_786_048,
                    used_pct: 51.2,
                },
            ],
            slices: vec![
                SliceUsage {
                    slice_name: "workload.slice".into(),
                    memory_current_bytes: 6_618_468_352,
                    pids_current: 213,
                    memory_high_events: 1024,
                    ..SliceUsage::default()
                },
                SliceUsage {
                    slice_name: "system.slice".into(),
                    memory_current_bytes: 5_762_764_800,
                    pids_current: 1336,
                    ..SliceUsage::default()
                },
                SliceUsage {
                    slice_name: "telemetry.slice".into(),
                    memory_current_bytes: 138_477_568,
                    pids_current: 10,
                    ..SliceUsage::default()
                },
            ],
            agents: vec![
                AgentProcess {
                    session_id: "svc-gateway".into(),
                    agent_name: "ingress-proxy".into(),
                    slice: "system.slice".into(),
                    pids: vec![1201, 1202, 1203],
                    rss_bytes: 1_033_232_384,
                    ..AgentProcess::default()
                },
                AgentProcess {
                    session_id: "worker-pool-01".into(),
                    agent_name: "batch-worker".into(),
                    slice: "workload.slice".into(),
                    pids: vec![4410, 4418],
                    rss_bytes: 366_051_328,
                    write_bytes: 12_353_226_547,
                    ..AgentProcess::default()
                },
            ],
            self_health: SelfHealth {
                pid: 8840,
                rss_bytes: 56_565_760,
                open_fds: 17,
                slice_confined: true,
                ..SelfHealth::default()
            },
            gpus: Vec::new(),
        }),
        custody: Some(CustodyMetrics {
            spindle_socket: "/run/telemetry/custody.sock".into(),
            spindle_online: true,
            spindle_origin: Some("origin-example-01".into()),
            records: 130,
            highest_record_id: 130,
            canonical_through: 130,
            server_endpoint: Some("[fd00:db8::10]:17440".into()),
            server_online: true,
            server_authority: Some("00000000-0000-4000-8000-000000000001".into()),
            server_frontiers: frontiers,
            collector_spindle_watermark_id: Some(130),
            ..CustodyMetrics::default()
        }),
        blame_memory: Some(BlameReport {
            resource: "memory".into(),
            total_observed: 8_670_560_256,
            unit_label: "bytes".into(),
            pressure_summary: "available=24894 MiB mem.some.avg10=0.00".into(),
            entries: vec![
                BlameEntry {
                    rank: 1,
                    node_id: "pid:1203".into(),
                    label: "ingress-proxy (pid 1203)".into(),
                    contribution_value: 601_333_760,
                    unit_label: "bytes".into(),
                    share_pct: 6.94,
                    slice: Some("system.slice".into()),
                    agent: Some("ingress-proxy".into()),
                    ..BlameEntry::default()
                },
                BlameEntry {
                    rank: 2,
                    node_id: "pid:4418".into(),
                    label: "batch-worker (pid 4418)".into(),
                    contribution_value: 366_051_328,
                    unit_label: "bytes".into(),
                    share_pct: 4.22,
                    slice: Some("workload.slice".into()),
                    agent: Some("batch-worker".into()),
                    ..BlameEntry::default()
                },
            ],
        }),
        drift: Some(DriftReport {
            total_findings: 2,
            critical_count: 0,
            warn_count: 2,
            info_count: 0,
            findings: vec![DriftFinding {
                pillar: "lineage".into(),
                severity: "warn".into(),
                target: "unit:worker-pool-01.scope".into(),
                summary: "transient scope missing CollectMode=inactive-or-failed".into(),
                metric_value: 1.0,
                threshold_value: 0.0,
            }],
        }),
        doctor: Some(DoctorReport {
            healthy: true,
            daemon_running: true,
            pid: Some(8840),
            rss_bytes: 56_565_760,
            rss_budget_bytes: 104_857_600,
            open_fds: 17,
            max_open_fds_budget: 64,
            tmp_files_created: 0,
            slice_confined: true,
            data_dir: "/var/lib/telemetry".into(),
            tmp_dir: "/tmp/telemetry".into(),
            cgroup_path: "/system.slice/telemetry.slice".into(),
            dropped_events_total: 0,
            watchdog_shedding_threshold_bytes: 125_829_120,
            self_shedding_active: false,
            checks: vec![DoctorCheck {
                name: "rss_budget".into(),
                passed: true,
                actual: "53 MiB".into(),
                threshold: "< 100 MiB".into(),
                detail: "resident set size within budget".into(),
            }],
        }),
        metric_series,
    }
}

/// Builds a generic three-zone `example-cluster` topology fixture.
pub fn example_cluster_topology() -> ClusterTopology {
    ClusterTopology {
        title: "example-cluster Control & Storage Plane".into(),
        nodes: vec![
            ClusterNode {
                slug: "ctrl_01".into(),
                title: "ctrl-01".into(),
                subtitle: Some("fd00:db8::1 · active".into()),
                zone: "Control Plane".into(),
                kind: TelemetryAssetKind::Switch,
                status: "healthy".into(),
            },
            ClusterNode {
                slug: "enclave_01".into(),
                title: "enclave-01".into(),
                subtitle: Some("mTLS · attested".into()),
                zone: "Control Plane".into(),
                kind: TelemetryAssetKind::Lock,
                status: "healthy".into(),
            },
            ClusterNode {
                slug: "worker_01".into(),
                title: "worker-01".into(),
                subtitle: Some("fd00:db8::11 · 42% RAM".into()),
                zone: "Compute Pool".into(),
                kind: TelemetryAssetKind::Rack,
                status: "healthy".into(),
            },
            ClusterNode {
                slug: "worker_02".into(),
                title: "worker-02".into(),
                subtitle: Some("fd00:db8::12 · backpressure".into()),
                zone: "Compute Pool".into(),
                kind: TelemetryAssetKind::Pod,
                status: "degraded".into(),
            },
            ClusterNode {
                slug: "wal_sink".into(),
                title: "wal-sink-01".into(),
                subtitle: Some("NVMe quorum · 0 lag".into()),
                zone: "Canonical Storage".into(),
                kind: TelemetryAssetKind::Drum,
                status: "healthy".into(),
            },
        ],
        links: vec![
            ClusterLink {
                from: "ctrl_01".into(),
                to: "enclave_01".into(),
                label: Some("attest".into()),
                style: "solid".into(),
            },
            ClusterLink {
                from: "ctrl_01".into(),
                to: "worker_01".into(),
                label: Some("wg0".into()),
                style: "solid".into(),
            },
            ClusterLink {
                from: "ctrl_01".into(),
                to: "worker_02".into(),
                label: Some("wg0".into()),
                style: "dashed".into(),
            },
            ClusterLink {
                from: "worker_01".into(),
                to: "wal_sink".into(),
                label: Some("replication".into()),
                style: "solid".into(),
            },
            ClusterLink {
                from: "worker_02".into(),
                to: "wal_sink".into(),
                label: Some("async".into()),
                style: "dotted".into(),
            },
        ],
    }
}

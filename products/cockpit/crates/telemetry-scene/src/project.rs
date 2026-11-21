//! Pure adapters translating generic JSON telemetry into 2.5D [`iso_diagram::model::Diagram`]s.

use std::collections::{BTreeMap, HashMap};

use iso_diagram::builder::DiagramBuilder;
use iso_diagram::model::{
    Asset, Diagram, EdgeStyle, NodeId, NodeStatus, ThemePreset, Tint, ZoneId, ZoneStyle,
    sanitize_slug,
};

use crate::schema::{ClusterTopology, TelemetryAssetKind, TelemetryBundle};
use crate::schema_ext::{custody_lag, format_bytes_iec};

/// Parses a JSON string into a [`TelemetryBundle`].
pub fn parse_telemetry_json(json_str: &str) -> Result<TelemetryBundle, serde_json::Error> {
    serde_json::from_str(json_str)
}

/// Parses a JSON string into a [`ClusterTopology`].
pub fn parse_cluster_json(json_str: &str) -> Result<ClusterTopology, serde_json::Error> {
    serde_json::from_str(json_str)
}

/// Converts a [`TelemetryBundle`] into an interactive 2.5D isometric topology ([`ThemePreset::WarmPaper`]).
pub fn telemetry_to_diagram(bundle: &TelemetryBundle) -> Diagram {
    telemetry_to_diagram_themed(bundle, ThemePreset::WarmPaper)
}

/// Converts a [`TelemetryBundle`] into a themed 2.5D isometric topology.
pub fn telemetry_to_diagram_themed(bundle: &TelemetryBundle, preset: ThemePreset) -> Diagram {
    let host_label = bundle
        .snapshot
        .as_ref()
        .map(|s| s.hostname.as_str())
        .filter(|h| !h.is_empty())
        .unwrap_or("localhost");

    let mut b = DiagramBuilder::new(format!("Telemetry · {host_label}"));
    b.theme(preset);

    let custody_zone = b.add_zone(
        "custody_plane",
        "Custody & Control Plane",
        ZoneStyle::Solid,
        Tint::Pine,
        None,
    );

    let (daemon_sub, daemon_status) = daemon_node_meta(bundle);
    let daemon_id = b
        .insert_node(
            "telemetry_agent",
            Asset::Switch { ports: 8 },
            "telemetry-agent",
            Some(custody_zone),
        )
        .subtitle(daemon_sub)
        .tint(Tint::Pine)
        .status(daemon_status)
        .id();

    let (spindle_sub, spindle_status) = spindle_node_meta(bundle);
    let spindle_id = b
        .insert_node(
            "wal_custody",
            Asset::Lock,
            "WAL Custody",
            Some(custody_zone),
        )
        .subtitle(spindle_sub)
        .tint(Tint::Pine)
        .status(spindle_status)
        .id();

    let (srv_sub, srv_status) = server_node_meta(bundle);
    let srv_id = b
        .insert_node(
            "canonical_sink",
            Asset::Balancer,
            "Canonical Sink",
            Some(custody_zone),
        )
        .subtitle(srv_sub)
        .tint(Tint::Cobalt)
        .status(srv_status)
        .id();

    let _ = b
        .edge(daemon_id, spindle_id)
        .style(EdgeStyle::Solid)
        .tint(Tint::Pine)
        .id();
    let _ = b
        .edge(spindle_id, srv_id)
        .style(EdgeStyle::Dashed)
        .tint(Tint::Cobalt)
        .label("canonical")
        .id();

    let Some(snap) = &bundle.snapshot else {
        return b.build();
    };

    let slices_zone = b.add_zone(
        "slices_workloads",
        "Cgroup v2 Slices & Workloads",
        ZoneStyle::Solid,
        Tint::Amber,
        None,
    );

    let mut sorted_slices = snap.slices.clone();
    sorted_slices.sort_by_key(|a| std::cmp::Reverse(a.memory_current_bytes));
    let mut slice_nodes: HashMap<String, NodeId> = HashMap::new();

    for slice in sorted_slices.into_iter().take(4) {
        let slug = format!("slice_{}", sanitize_slug(&slice.slice_name));
        let status = if slice.oom_events > 0 {
            NodeStatus::Failed
        } else if slice.memory_high_events > 0 {
            NodeStatus::Degraded
        } else {
            NodeStatus::Healthy
        };
        let pedestal =
            slice.slice_name == "workload.slice" || slice.slice_name == "telemetry.slice";
        let nid = b
            .insert_node(
                slug,
                Asset::Rack {
                    blades: 3,
                    pedestal,
                },
                &slice.slice_name,
                Some(slices_zone),
            )
            .subtitle(format!(
                "{} · {}p",
                format_bytes_iec(slice.memory_current_bytes),
                slice.pids_current
            ))
            .tint(Tint::Amber)
            .status(status)
            .id();
        let _ = b
            .edge(nid, daemon_id)
            .style(EdgeStyle::Solid)
            .tint(Tint::Amber)
            .id();
        slice_nodes.insert(slice.slice_name, nid);
    }

    let mut grouped_agents: BTreeMap<(String, String), (u64, usize)> = BTreeMap::new();
    for ag in &snap.agents {
        let key = (ag.agent_name.clone(), ag.slice.clone());
        let entry = grouped_agents.entry(key).or_insert((0, 0));
        entry.0 = entry.0.saturating_add(ag.rss_bytes);
        entry.1 += ag.pids.len();
    }
    let mut top_agents: Vec<_> = grouped_agents.into_iter().collect();
    top_agents.sort_by_key(|a| std::cmp::Reverse(a.1.0));

    for (idx, ((agent_name, slice_name), (rss, pids))) in top_agents.into_iter().take(4).enumerate()
    {
        let slug = format!("worker_{idx}_{}", sanitize_slug(&agent_name));
        let nid = b
            .insert_node(slug, Asset::Pod, &agent_name, Some(slices_zone))
            .subtitle(format!("{} · {pids}p", format_bytes_iec(rss)))
            .tint(Tint::Plum)
            .status(NodeStatus::Healthy)
            .id();
        if let Some(&slice_id) = slice_nodes.get(&slice_name) {
            let _ = b
                .edge(nid, slice_id)
                .style(EdgeStyle::Dotted)
                .tint(Tint::Plum)
                .id();
        } else {
            let _ = b
                .edge(nid, daemon_id)
                .style(EdgeStyle::Dotted)
                .tint(Tint::Plum)
                .id();
        }
    }

    let store_zone = b.add_zone(
        "storage_mem",
        "Storage & Memory",
        ZoneStyle::Dashed,
        Tint::Cobalt,
        None,
    );

    for (idx, mount) in snap.mounts.iter().take(4).enumerate() {
        let mslug = if mount.mount_point == "/" {
            "mnt_root".to_string()
        } else {
            format!("mnt_{idx}_{}", sanitize_slug(&mount.mount_point))
        };
        let status = if mount.used_pct >= 90.0 {
            NodeStatus::Failed
        } else if mount.used_pct >= 80.0 {
            NodeStatus::Degraded
        } else {
            NodeStatus::Healthy
        };
        let nid = b
            .insert_node(
                mslug,
                Asset::Drum { rings: 3 },
                &mount.mount_point,
                Some(store_zone),
            )
            .subtitle(format!("{:.0}% · {}", mount.used_pct, mount.fs_type))
            .tint(Tint::Cobalt)
            .status(status)
            .id();
        let _ = b
            .edge(nid, daemon_id)
            .style(EdgeStyle::Solid)
            .tint(Tint::Cobalt)
            .id();
    }

    if let Some(zram) = &snap.memory.zram {
        let zid = b
            .insert_node("zram_swap", Asset::Crate, &zram.device, Some(store_zone))
            .subtitle(format!(
                "{:.2}x · {}",
                zram.compression_ratio,
                format_bytes_iec(zram.compr_data_size_bytes)
            ))
            .tint(Tint::Sage)
            .status(NodeStatus::Healthy)
            .id();
        let _ = b
            .edge(zid, daemon_id)
            .style(EdgeStyle::Dashed)
            .tint(Tint::Sage)
            .label("zram")
            .id();
    }

    b.build()
}

/// Projects a declarative multi-node [`ClusterTopology`] into a themed 2.5D [`Diagram`].
pub fn cluster_to_diagram(cluster: &ClusterTopology, preset: ThemePreset) -> Diagram {
    let mut b = DiagramBuilder::new(if cluster.title.is_empty() {
        "Cluster Topology".to_string()
    } else {
        cluster.title.clone()
    });
    b.theme(preset);

    let zone_tints = [Tint::Pine, Tint::Amber, Tint::Cobalt, Tint::Slate];
    let mut zones: BTreeMap<String, ZoneId> = BTreeMap::new();
    for node in &cluster.nodes {
        let zname = if node.zone.is_empty() {
            "Default Bay".to_string()
        } else {
            node.zone.clone()
        };
        if !zones.contains_key(&zname) {
            let tint = zone_tints[zones.len() % zone_tints.len()];
            let zid = b.add_zone(sanitize_slug(&zname), &zname, ZoneStyle::Solid, tint, None);
            zones.insert(zname, zid);
        }
    }

    let mut nodes: HashMap<String, NodeId> = HashMap::new();
    for node in &cluster.nodes {
        let zname = if node.zone.is_empty() {
            "Default Bay"
        } else {
            node.zone.as_str()
        };
        let zid = zones.get(zname).copied();
        let status = parse_status(&node.status);
        let asset = map_asset(node.kind);
        let mut nr = b.insert_node(sanitize_slug(&node.slug), asset, &node.title, zid);
        if let Some(sub) = &node.subtitle {
            nr = nr.subtitle(sub);
        }
        nr = nr.status(status);
        nodes.insert(node.slug.clone(), nr.id());
    }

    for link in &cluster.links {
        if let (Some(&from), Some(&to)) = (nodes.get(&link.from), nodes.get(&link.to)) {
            let style = match link.style.as_str() {
                "dashed" => EdgeStyle::Dashed,
                "dotted" => EdgeStyle::Dotted,
                _ => EdgeStyle::Solid,
            };
            let mut er = b.edge(from, to).style(style);
            if let Some(label) = &link.label {
                er = er.label(label);
            }
            let _ = er.id();
        }
    }

    b.build()
}

fn map_asset(kind: TelemetryAssetKind) -> Asset {
    match kind {
        TelemetryAssetKind::Rack => Asset::Rack {
            blades: 3,
            pedestal: true,
        },
        TelemetryAssetKind::Drum => Asset::Drum { rings: 3 },
        TelemetryAssetKind::Crate => Asset::Crate,
        TelemetryAssetKind::Firewall => Asset::Firewall,
        TelemetryAssetKind::Switch => Asset::Switch { ports: 8 },
        TelemetryAssetKind::Router => Asset::Router,
        TelemetryAssetKind::Laptop => Asset::Laptop,
        TelemetryAssetKind::Lock => Asset::Lock,
        TelemetryAssetKind::Chip => Asset::Chip,
        TelemetryAssetKind::Cloud => Asset::Cloud,
        TelemetryAssetKind::Pod => Asset::Pod,
        TelemetryAssetKind::Balancer => Asset::Balancer,
    }
}

fn parse_status(raw: &str) -> NodeStatus {
    match raw.to_ascii_lowercase().as_str() {
        "active" => NodeStatus::Active,
        "degraded" => NodeStatus::Degraded,
        "failed" => NodeStatus::Failed,
        "pending" => NodeStatus::Pending,
        _ => NodeStatus::Healthy,
    }
}

fn daemon_node_meta(bundle: &TelemetryBundle) -> (String, NodeStatus) {
    let snap_shedding = bundle
        .snapshot
        .as_ref()
        .is_some_and(|s| s.self_health.self_shedding_active);
    if let Some(doc) = &bundle.doctor {
        let shedding = doc.self_shedding_active || snap_shedding;
        let status = if !doc.daemon_running {
            NodeStatus::Failed
        } else if !doc.healthy || shedding {
            NodeStatus::Degraded
        } else {
            NodeStatus::Healthy
        };
        let suffix = if shedding { " · shedding" } else { "" };
        return (
            format!(
                "{} · {} FDs{suffix}",
                format_bytes_iec(doc.rss_bytes),
                doc.open_fds
            ),
            status,
        );
    }
    if let Some(snap) = &bundle.snapshot {
        let status = if snap.self_health.slice_confined && !snap_shedding {
            NodeStatus::Healthy
        } else {
            NodeStatus::Degraded
        };
        let suffix = if snap_shedding { " · shedding" } else { "" };
        return (
            format!(
                "{} · {} FDs{suffix}",
                format_bytes_iec(snap.self_health.rss_bytes),
                snap.self_health.open_fds
            ),
            status,
        );
    }
    ("offline".to_owned(), NodeStatus::Pending)
}

fn spindle_node_meta(bundle: &TelemetryBundle) -> (String, NodeStatus) {
    if let Some(c) = &bundle.custody {
        let lag = custody_lag(c);
        let status = if !c.spindle_online || c.spindle_error.is_some() {
            NodeStatus::Failed
        } else if lag > 0 || c.forwarding_error.is_some() {
            NodeStatus::Degraded
        } else {
            NodeStatus::Healthy
        };
        let sub = if let Some(err) = &c.spindle_error {
            format!(
                "{}/{} rec · err: {}",
                c.records,
                c.canonical_through,
                clip_short(err, 22)
            )
        } else if lag > 0 {
            format!("{}/{} rec · lag {lag}", c.records, c.canonical_through)
        } else if let Some(err) = &c.forwarding_error {
            format!(
                "{}/{} rec · fwd: {}",
                c.records,
                c.canonical_through,
                clip_short(err, 20)
            )
        } else {
            format!("{}/{} rec", c.records, c.canonical_through)
        };
        return (sub, status);
    }
    ("awaiting custody".to_owned(), NodeStatus::Pending)
}

fn server_node_meta(bundle: &TelemetryBundle) -> (String, NodeStatus) {
    if let Some(c) = &bundle.custody {
        let ep = c.server_endpoint.as_deref().unwrap_or("loopback");
        let status = if !c.server_online {
            if c.server_error.is_some() {
                NodeStatus::Failed
            } else {
                NodeStatus::Degraded
            }
        } else if c.server_error.is_some() {
            NodeStatus::Degraded
        } else {
            NodeStatus::Healthy
        };
        let sub = match &c.server_error {
            Some(err) => format!("{ep} · err: {}", clip_short(err, 24)),
            None => ep.to_owned(),
        };
        return (sub, status);
    }
    ("canonical receiver".to_owned(), NodeStatus::Pending)
}

fn clip_short(s: &str, max_chars: usize) -> String {
    let trimmed = s.trim();
    if trimmed.chars().count() <= max_chars {
        trimmed.to_owned()
    } else {
        let mut out: String = trimmed.chars().take(max_chars.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

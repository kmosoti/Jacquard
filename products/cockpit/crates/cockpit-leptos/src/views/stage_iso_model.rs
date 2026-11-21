//! SSR-only translation from `TargetDto` cluster snapshots through `iso-diagram`
//! (`compile_layout` + `compile_scene_geometry`) into compact [`ClientScene`] data.

use std::collections::HashMap;
use std::fmt::Write as _;

use iso_diagram::builder::DiagramBuilder;
use iso_diagram::geometry::{SceneGeometry, compile_scene_geometry};
use iso_diagram::layout::compile_layout;
use iso_diagram::model::{Asset, Diagram, EdgeStyle, NodeId, NodeStatus, Tint, ZoneStyle};

use crate::state::TargetDto;
use crate::views::iso_client_math::ClientScene;

fn to_centi(v: f32) -> i32 {
    (v * 100.0).round() as i32
}

/// Compiles `targets` through `iso-diagram` into `(ClientScene, SceneGeometry)` on SSR.
#[must_use]
pub fn compile_stage_scene(targets: &[TargetDto], title: &str) -> (ClientScene, SceneGeometry) {
    let diag = targets_to_diagram(targets, title);
    let layout = compile_layout(&diag);
    let geom = compile_scene_geometry(&layout);
    let zones = geom
        .zones
        .iter()
        .map(|z| {
            (
                z.id,
                [
                    to_centi(z.u_min),
                    to_centi(z.v_min),
                    to_centi(z.u_max),
                    to_centi(z.v_max),
                ],
            )
        })
        .collect();
    let boxes = geom
        .boxes
        .iter()
        .map(|b| {
            (
                b.slug.clone(),
                b.title.len(),
                [
                    to_centi(b.aabb.u_min),
                    to_centi(b.aabb.u_max),
                    to_centi(b.aabb.v_min),
                    to_centi(b.aabb.v_max),
                    to_centi(b.aabb.z_max),
                ],
            )
        })
        .collect();
    let routes = geom
        .routes
        .iter()
        .map(|r| {
            (
                r.id,
                r.points
                    .iter()
                    .map(|&(u, v)| (to_centi(u), to_centi(v)))
                    .collect(),
            )
        })
        .collect();
    let client = ClientScene {
        extent: [
            to_centi(geom.grid_u_min),
            to_centi(geom.grid_v_min),
            to_centi(geom.grid_u_max),
            to_centi(geom.grid_v_max),
        ],
        zones,
        boxes,
        routes,
    };
    (client, geom)
}

/// Encodes SSR palette seeds as tab-delimited lines for `data-items` hydration.
#[must_use]
pub fn encode_palette_seeds(targets: &[TargetDto]) -> String {
    let mut out = String::from("Reset 360-degree camera orbit & zoom\tSTAGE\t\n");
    for t in targets {
        let _ = writeln!(out, "{} ({})\tASSET\t{}", t.display_name, t.urn, t.urn);
    }
    out
}

/// Translates cluster DTOs into a declarative [`Diagram`].
pub fn targets_to_diagram(targets: &[TargetDto], title: &str) -> Diagram {
    let mut b = DiagramBuilder::new(title);
    let group_defs = [
        (
            "Apis",
            "control_apis",
            "Control APIs",
            ZoneStyle::Wireframe,
            Tint::Cobalt,
        ),
        (
            "Iam",
            "iam_identity",
            "IAM & Principals",
            ZoneStyle::Dashed,
            Tint::Plum,
        ),
        (
            "Network",
            "overlay_v6",
            "IPv6 Overlay",
            ZoneStyle::Solid,
            Tint::Cobalt,
        ),
        (
            "Storage",
            "storage",
            "Block & Archive Vaults",
            ZoneStyle::Solid,
            Tint::Sage,
        ),
        (
            "Compute",
            "compute",
            "Compute Pool",
            ZoneStyle::Solid,
            Tint::Amber,
        ),
        (
            "Mesh",
            "mesh_plane",
            "WireGuard Mesh",
            ZoneStyle::Dashed,
            Tint::Pine,
        ),
    ];
    let mut zone_ids = HashMap::new();
    for &(grp, slug, label, style, tint) in &group_defs {
        if targets.iter().any(|t| t.group == grp) {
            zone_ids.insert(grp, b.add_zone(slug, label, style, tint, None));
        }
    }
    let obs_zone = targets.iter().any(|t| t.urn.contains("obs-01")).then(|| {
        b.add_zone(
            "telemetry_plane",
            "Telemetry Plane",
            ZoneStyle::Solid,
            Tint::Pine,
            None,
        )
    });
    let mut node_by_urn: HashMap<&str, NodeId> = HashMap::with_capacity(targets.len());
    for t in targets {
        let zid = if matches!(
            t.urn.as_str(),
            "cluster.node/obs-01" | "cluster.disk/disk-obs"
        ) {
            obs_zone.or_else(|| zone_ids.get(t.group.as_str()).copied())
        } else {
            zone_ids.get(t.group.as_str()).copied()
        };
        let status = match t.status.as_str() {
            "Converged" => NodeStatus::Healthy,
            "InFlight" => NodeStatus::Active,
            "Drifted" => NodeStatus::Degraded,
            "Poisoned" => NodeStatus::Failed,
            _ => NodeStatus::Pending,
        };
        let mut nr = b.insert_node(&t.urn, asset_for_target(t), &t.display_name, zid);
        if let Some(ep) = &t.endpoint {
            nr = nr.subtitle(ep.as_str());
        }
        let nid = nr.tint(tint_for_target(t)).status(status).id();
        node_by_urn.insert(t.urn.as_str(), nid);
    }
    for t in targets {
        let Some(&from_id) = node_by_urn.get(t.urn.as_str()) else {
            continue;
        };
        for down in &t.downstream {
            let Some(&to_id) = node_by_urn.get(down.as_str()) else {
                continue;
            };
            let is_wg = t.group == "Mesh" || down.starts_with("wg.peer/");
            let is_iam = t.group == "Iam" || t.group == "Apis";
            let style = if is_wg {
                EdgeStyle::Dashed
            } else if is_iam {
                EdgeStyle::Dotted
            } else {
                EdgeStyle::Solid
            };
            let tint = if is_wg {
                Tint::Pine
            } else if is_iam {
                Tint::Plum
            } else {
                Tint::Cobalt
            };
            let mut er = b.edge(from_id, to_id).style(style).tint(tint);
            if is_wg {
                er = er.label("wg0");
            }
            let _ = er.id();
        }
    }
    b.build()
}

fn asset_for_target(t: &TargetDto) -> Asset {
    match t.kind_label.as_str() {
        "ComputeInstance" => Asset::Rack {
            blades: 3,
            pedestal: t.urn.contains("ctrl-01")
                || t.urn.contains("vault-01")
                || t.urn.contains("obs-01"),
        },
        "PersistentDisk" => Asset::Drum { rings: 3 },
        "DriveBucketFolder" => Asset::Crate,
        "FirewallRule" => Asset::Firewall,
        "VpcNetwork" => Asset::Switch { ports: 8 },
        "Ipv6Subnet" | "StaticIpv6Address" => Asset::Router,
        "WireGuardPeer" => Asset::Lock,
        "ServiceAccount" => Asset::Chip,
        "EnabledApi" => Asset::Cloud,
        _ => Asset::Pod,
    }
}

fn tint_for_target(t: &TargetDto) -> Tint {
    if t.urn.contains("obs-01") {
        return Tint::Pine;
    }
    match t.group.as_str() {
        "Compute" => Tint::Amber,
        "Storage" => Tint::Sage,
        "Mesh" => Tint::Pine,
        "Iam" => Tint::Plum,
        "Network" | "Apis" => Tint::Cobalt,
        _ => Tint::Slate,
    }
}

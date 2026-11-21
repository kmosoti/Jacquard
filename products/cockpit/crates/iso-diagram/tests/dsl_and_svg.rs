//! Integration tests for `iso-diagram` eDSL, 2.5D sprites, orbit camera, and SVG export.

use iso_diagram::assets::emit_asset;
use iso_diagram::catalog;
use iso_diagram::layout::{asset_footprint, compile_layout};
use iso_diagram::model::Asset;
use iso_diagram::prelude::*;
use iso_diagram::projection::{IsoProjector, Rect2, hit_test, resolve_label_rects_with_obstacles};
use iso_diagram::svg::render_svg;

#[test]
fn typed_macro_and_builder_match_and_route_orthogonally() {
    let diag = iso_diagram! {
        title: "Unit Topology",
        theme: WarmPaper,
        zone core ("Core Zone", Solid, Pine) {
            node fw = Firewall => ("Edge Policy", "Ingress");
            node sw = switch(8) => ("Core Switch");
        }
        node srv = rack_pedestal(3) => ("Server 1", "App") [Cobalt];
        edges {
            fw -> sw [Pine] : "10G";
            sw ~> srv [Cobalt] : "wg0";
        }
    };

    assert_eq!(diag.zones.len(), 1);
    assert_eq!(diag.nodes.len(), 3);
    assert_eq!(diag.edges.len(), 2);

    let layout = compile_layout(&diag);
    assert_eq!(layout.nodes.len(), 3);
    for edge in &layout.edges {
        assert!(edge.points.len() >= 2);
        for seg in edge.points.windows(2) {
            let du = (seg[1].0 - seg[0].0).abs();
            let dv = (seg[1].1 - seg[0].1).abs();
            assert!(
                (du < 1e-4) ^ (dv < 1e-4),
                "non-orthogonal segment in edge {:?}: {:?} -> {:?}",
                edge.id,
                seg[0],
                seg[1]
            );
        }
    }
}

#[test]
fn all_12_procedural_assets_emit_geometries_and_footprints() {
    let diag = catalog::asset_gallery();
    let layout = compile_layout(&diag);
    let proj = IsoProjector::fit(&layout, 1280.0, 800.0, 48.0);
    for &asset in Asset::ALL {
        let (hu, hv, hz) = asset_footprint(asset);
        assert!(hu > 0.1 && hv > 0.1 && hz > 0.1);
        let prims = emit_asset(asset, 4.0, 4.0, &proj);
        assert!(!prims.is_empty(), "empty geometry for {:?}", asset);
    }
}

#[test]
fn svg_renders_clean_for_all_showcase_scenes_and_dsl_roundtrips() {
    for name in ["campus-ha", "payment-gateway", "asset-gallery"] {
        let diag = catalog::by_name(name).expect("scene exists");
        let svg = render_svg(&diag, 1440.0, 900.0);
        assert!(svg.starts_with("<svg "));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains(&diag.nodes[0].title));

        let dsl = emit_rust_dsl(&diag);
        assert!(dsl.contains("iso_diagram!"));
        assert!(dsl.contains("edges {"));
    }
}

#[test]
fn label_deoverlap_and_hit_testing_work_deterministically() {
    let diag = catalog::campus_ha();
    let layout = compile_layout(&diag);
    let proj = IsoProjector::fit(&layout, 1280.0, 800.0, 48.0);

    for placed in &layout.nodes {
        let mid = proj.project(placed.u, placed.v, placed.height_z * 0.45);
        let hit = hit_test(&layout, &proj, mid);
        assert!(
            hit.is_some(),
            "missed projected center for {}",
            placed.spec.title
        );
    }

    let anchor = proj.project(3.0, 3.0, 1.0);
    let r = Rect2::new(anchor.x - 40.0, anchor.y - 25.0, 80.0, 26.0);
    let resolved = resolve_label_rects_with_obstacles(
        &[r],
        &[(r, anchor), (r, anchor)],
        Rect2::new(0.0, 0.0, 1280.0, 800.0),
    );
    assert!(!resolved[0].intersects(resolved[1]));
    assert!(!resolved[0].intersects(r));
}

#[test]
fn orbit_projection_unproject_roundtrip_and_shadows_across_360_degrees() {
    use iso_diagram::assets::Tone;

    let diag = catalog::campus_ha();
    let layout = compile_layout(&diag);
    let ext = layout.grid_extent;
    let pivot_u = (ext.u_min + ext.u_max) * 0.5;
    let pivot_v = (ext.v_min + ext.v_max) * 0.5;
    let base = IsoProjector::fit(&layout, 1280.0, 800.0, 32.0);

    for yaw_deg in [0.0f32, 45.0, 135.0, 225.0, 315.0] {
        for pitch in [0.35f32, 0.50, 0.68] {
            let proj = base.with_orbit(yaw_deg.to_radians(), pitch, pivot_u, pivot_v);
            for placed in &layout.nodes {
                let floor_pt = proj.project(placed.u, placed.v, 0.0);
                let (ru, rv) = proj.unproject_floor(floor_pt.x, floor_pt.y);
                assert!(
                    (ru - placed.u).abs() < 1e-2 && (rv - placed.v).abs() < 1e-2,
                    "unproject mismatch at yaw={yaw_deg} pitch={pitch}: ({ru},{rv}) vs ({},{})",
                    placed.u,
                    placed.v
                );

                let mid = proj.project(placed.u, placed.v, placed.height_z * 0.45);
                let hit = hit_test(&layout, &proj, mid);
                assert!(
                    hit.is_some(),
                    "hit_test missed {} at yaw={yaw_deg} pitch={pitch}",
                    placed.spec.title
                );
            }
        }
    }

    let rotated = base.with_orbit(135.0f32.to_radians(), 0.52, pivot_u, pivot_v);
    for asset in [
        Asset::Rack {
            blades: 3,
            pedestal: true,
        },
        Asset::Drum { rings: 3 },
    ] {
        let prims = emit_asset(asset, pivot_u, pivot_v, &rotated);
        assert!(prims.iter().any(|p| matches!(
            p,
            iso_diagram::assets::DrawPrim::Polygon {
                fill: Some(Tone::ShadowCore),
                ..
            }
        )));
        assert!(prims.iter().any(|p| matches!(
            p,
            iso_diagram::assets::DrawPrim::Polygon {
                fill: Some(Tone::Shadow),
                ..
            }
        )));
    }
}

#[test]
fn weighted_router_places_valid_chevrons_and_maintains_orthogonality() {
    use iso_diagram::camera::eval_polyline_uv;

    let diag = catalog::campus_ha();
    let layout = compile_layout(&diag);
    assert!(!layout.edges.is_empty());

    let mut centers = Vec::with_capacity(layout.edges.len());
    for edge in &layout.edges {
        assert!(
            edge.chevron_t >= 0.20 && edge.chevron_t <= 0.80,
            "chevron_t out of bounds: {}",
            edge.chevron_t
        );
        for seg in edge.points.windows(2) {
            let du = (seg[1].0 - seg[0].0).abs();
            let dv = (seg[1].1 - seg[0].1).abs();
            assert!(
                (du < 1e-3) ^ (dv < 1e-3),
                "weighted route not orthogonal: {:?}",
                seg
            );
        }
        let pos = eval_polyline_uv(&edge.points, edge.chevron_t).0;
        centers.push(pos);
    }

    let mut close_pairs = 0usize;
    for i in 0..centers.len() {
        for j in (i + 1)..centers.len() {
            if (centers[i].0 - centers[j].0).hypot(centers[i].1 - centers[j].1) < 0.12 {
                close_pairs += 1;
            }
        }
    }
    assert_eq!(
        close_pairs, 0,
        "distinct edge chevrons overlapped within 0.12 UV"
    );
}

#[test]
fn sugiyama_virtual_nodes_and_explicit_grid_hints_order_cleanly() {
    use iso_diagram::layout::{LayoutSpec, compile_layout_with_spec};
    use std::collections::BTreeMap;

    let mut b = Diagram::builder("Sugiyama Hints");
    let a = b.node(Asset::Router, "A").at_grid(0, 0).id();
    let d = b.node(Asset::Pod, "D").at_grid(0, 1).id();
    let mid = b.node(switch(8), "Mid").id();
    let x = b.node(rack(2), "X").id();
    let y = b.node(rack(2), "Y").id();
    b.edge(a, mid);
    b.edge(mid, x);
    b.edge(a, y); // multi-tier edge spanning tier 0 -> tier 2 via virtual node
    b.edge(d, y);
    let diag = b.build();

    let spec = LayoutSpec {
        hints: BTreeMap::from([(x, (2, 0)), (y, (2, 1))]),
    };
    let layout = compile_layout_with_spec(&diag, &spec);
    let pos = |id| {
        let n = layout.nodes.iter().find(|n| n.id == id).expect("node");
        (n.u, n.v)
    };
    assert!(pos(a).0 < pos(mid).0 && pos(mid).0 < pos(x).0);
    assert!(pos(a).1 < pos(d).1);
    assert!(pos(x).1 < pos(y).1);
}

#[test]
fn hanan_astar_avoids_obstacles_across_seven_parallel_corridors() {
    let mut b = Diagram::builder("Dense Corridors");
    let left = b.node(Asset::Router, "Ingress").at_grid(0, 1).id();
    let blocker = b.node(rack_pedestal(4), "Blocker").at_grid(1, 1).id();
    let right = b.node(drum(3), "Storage").at_grid(2, 1).id();
    for i in 0..7 {
        b.connect(left, right, format!("c{i}"));
    }
    b.edge(left, blocker);
    let diag = b.build();
    let layout = compile_layout(&diag);
    let blk = layout.nodes.iter().find(|n| n.id == blocker).expect("blk");

    for edge in layout.edges.iter().filter(|e| e.to == right) {
        for seg in edge.points.windows(2) {
            let du = (seg[1].0 - seg[0].0).abs();
            let dv = (seg[1].1 - seg[0].1).abs();
            assert!((du < 1e-3) ^ (dv < 1e-3), "non-orthogonal: {seg:?}");
            let (min_u, max_u) = (seg[0].0.min(seg[1].0), seg[0].0.max(seg[1].0));
            let (min_v, max_v) = (seg[0].1.min(seg[1].1), seg[0].1.max(seg[1].1));
            let hits_blocker = blk.u + blk.half_u > min_u
                && blk.u - blk.half_u < max_u
                && blk.v + blk.half_v > min_v
                && blk.v - blk.half_v < max_v;
            assert!(!hits_blocker, "edge {:?} crossed blocker obstacle", edge.id);
        }
    }
}

#[test]
fn topological_painter_sort_handles_elongated_prisms_and_exports_scene_geometry() {
    use iso_diagram::geometry::{Aabb3, compile_scene_geometry, topological_painter_order};

    let diag = catalog::campus_ha();
    let layout = compile_layout(&diag);
    let geom = compile_scene_geometry(&layout);
    assert_eq!(geom.boxes.len(), layout.nodes.len());
    assert_eq!(geom.routes.len(), layout.edges.len());
    assert_eq!(geom.labels.len(), layout.nodes.len());

    // Elongated wall behind a compact foreground box whose centroid depth is smaller
    // than the wall's centroid depth — centroid sort gets this wrong, DAG sort gets it right.
    let proj = IsoProjector::fit(&layout, 1280.0, 800.0, 32.0);
    let wall = Aabb3 {
        u_min: 0.0,
        u_max: 0.6,
        v_min: 0.0,
        v_max: 6.0,
        z_min: 0.0,
        z_max: 2.0,
    };
    let front_box = Aabb3 {
        u_min: 1.0,
        u_max: 1.8,
        v_min: 0.5,
        v_max: 1.3,
        z_min: 0.0,
        z_max: 1.5,
    };
    let order = topological_painter_order(&[front_box, wall], &proj);
    assert_eq!(
        order,
        vec![1, 0],
        "wall (idx 1) must be painted before front_box (idx 0)"
    );
}

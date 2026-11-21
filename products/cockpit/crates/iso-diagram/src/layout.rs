//! Deterministic campus-grid placement and nested zone wrapping.

use std::collections::BTreeMap;

use crate::layout_tiers::{
    build_node_unit_map, compute_unit_tiers, materialize_placements, sugiyama_order_layers,
};
use crate::model::{Asset, Diagram, EdgeId, EdgeSpec, NodeId, NodeSpec, ZoneId, ZoneSpec};
use crate::routing::route_edges;

const CELL_STEP: f32 = 2.60;
const ZONE_PAD: f32 = 0.78;
const SUBZONE_PAD: f32 = 0.60;
const BLOCK_GAP: f32 = 1.65;
const ROW_GAP: f32 = 2.45;

/// Rectangular grid bounding box in isometric `(u, v)` floor units.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GridBounds {
    /// Minimum floor `u` coordinate.
    pub u_min: f32,
    /// Minimum floor `v` coordinate.
    pub v_min: f32,
    /// Maximum floor `u` coordinate.
    pub u_max: f32,
    /// Maximum floor `v` coordinate.
    pub v_max: f32,
}

impl GridBounds {
    /// Span along the `u` axis.
    pub fn width(self) -> f32 {
        (self.u_max - self.u_min).max(0.0)
    }

    /// Span along the `v` axis.
    pub fn height(self) -> f32 {
        (self.v_max - self.v_min).max(0.0)
    }

    /// Shifts all corners by `(du, dv)`.
    pub fn translate(self, du: f32, dv: f32) -> Self {
        Self {
            u_min: self.u_min + du,
            v_min: self.v_min + dv,
            u_max: self.u_max + du,
            v_max: self.v_max + dv,
        }
    }
}

/// Floor zone placed into absolute `(u, v)` campus coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedZone {
    /// Source zone handle.
    pub id: ZoneId,
    /// Original zone specification.
    pub spec: ZoneSpec,
    /// Nesting depth (`0` for top-level zones).
    pub depth: usize,
    /// Placed floor bounds.
    pub bounds: GridBounds,
}

/// Hardware node placed at an absolute `(u, v)` coordinate on the floor.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedNode {
    /// Source node handle.
    pub id: NodeId,
    /// Original node specification.
    pub spec: NodeSpec,
    /// Center `u` coordinate.
    pub u: f32,
    /// Center `v` coordinate.
    pub v: f32,
    /// Painter's depth key (`u + v`).
    pub depth: f32,
    /// Footprint half-extent along `u`.
    pub half_u: f32,
    /// Footprint half-extent along `v`.
    pub half_v: f32,
    /// Sprite extrusion height along `z`.
    pub height_z: f32,
}

/// Routed orthogonal floor conduit between two [`PlacedNode`]s.
#[derive(Debug, Clone, PartialEq)]
pub struct RoutedEdge {
    /// Source edge handle.
    pub id: EdgeId,
    /// Origin node identifier.
    pub from: NodeId,
    /// Target node identifier.
    pub to: NodeId,
    /// Original edge specification.
    pub spec: EdgeSpec,
    /// Orthogonal polyline waypoints in `(u, v)` space.
    pub points: Vec<(f32, f32)>,
    /// Normalized parameter `[0.0, 1.0]` for collision-free directional chevron placement.
    pub chevron_t: f32,
}

/// Deterministic layout compilation output ready for SVG or DOM/WebGL rendering.
#[derive(Debug, Clone, PartialEq)]
pub struct CompiledLayout {
    /// Source diagram title.
    pub diagram_title: String,
    /// Depth-ordered floor zones.
    pub zones: Vec<PlacedZone>,
    /// Depth-sorted placed nodes.
    pub nodes: Vec<PlacedNode>,
    /// Orthogonal routed edges.
    pub edges: Vec<RoutedEdge>,
    /// Enclosing floor grid bounding box.
    pub grid_extent: GridBounds,
}

/// Returns `(half_u, half_v, height_z)` footprint metrics for each hardware asset.
pub fn asset_footprint(asset: Asset) -> (f32, f32, f32) {
    match asset {
        Asset::Rack { pedestal, .. } => (0.74, 0.74, if pedestal { 1.95 } else { 1.62 }),
        Asset::Drum { .. } => (0.68, 0.68, 1.52),
        Asset::Crate => (0.74, 0.66, 1.00),
        Asset::Firewall => (0.78, 0.50, 1.50),
        Asset::Switch { .. } => (0.86, 0.56, 0.56),
        Asset::Router => (0.74, 0.56, 1.05),
        Asset::Laptop => (0.74, 0.68, 1.04),
        Asset::Lock => (0.60, 0.52, 1.34),
        Asset::Chip => (0.76, 0.62, 0.38),
        Asset::Cloud => (0.88, 0.70, 1.06),
        Asset::Pod => (0.58, 0.58, 0.96),
        Asset::Balancer => (0.78, 0.58, 1.06),
    }
}

pub(crate) struct LocalBlock {
    pub w: f32,
    pub h: f32,
    pub zones: Vec<PlacedZone>,
    pub nodes: Vec<(NodeId, f32, f32)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum CampusUnit {
    Zone(ZoneId),
    Node(NodeId),
}

/// Optional layout overrides such as explicit `(col, row)` placement hints.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LayoutSpec {
    /// Explicit `(col, row)` tier/row overrides keyed by [`NodeId`].
    pub hints: BTreeMap<NodeId, (usize, usize)>,
}

/// Deterministically lays out all zones, subzones, nodes, and orthogonal pipes.
pub fn compile_layout(diagram: &Diagram) -> CompiledLayout {
    compile_layout_with_spec(diagram, &LayoutSpec::default())
}

/// Deterministically lays out `diagram` honoring explicit `(col, row)` hints in `spec`.
pub fn compile_layout_with_spec(diagram: &Diagram, spec: &LayoutSpec) -> CompiledLayout {
    if diagram.nodes.is_empty() && diagram.zones.is_empty() {
        return CompiledLayout {
            diagram_title: diagram.title.clone(),
            zones: Vec::new(),
            nodes: Vec::new(),
            edges: Vec::new(),
            grid_extent: GridBounds::default(),
        };
    }

    let mut units = Vec::new();
    let mut blocks = Vec::new();
    for z in diagram.zones.iter().filter(|z| z.parent.is_none()) {
        units.push(CampusUnit::Zone(z.id));
        blocks.push(layout_zone_recursive(diagram, z.id, 0));
    }
    for n in diagram.nodes.iter().filter(|n| n.zone.is_none()) {
        units.push(CampusUnit::Node(n.id));
        blocks.push(LocalBlock {
            w: 2.1,
            h: 2.1,
            zones: Vec::new(),
            nodes: vec![(n.id, 1.05, 1.05)],
        });
    }

    let unit_of_node = build_node_unit_map(diagram, &units);
    let mut hints = vec![None; units.len()];
    for node in &diagram.nodes {
        let h = spec.hints.get(&node.id).copied().or(node.grid_hint);
        if let (Some(h), Some(&ui)) = (h, unit_of_node.get(&node.id))
            && hints[ui].is_none()
        {
            hints[ui] = Some(h);
        }
    }
    let tiers = compute_unit_tiers(diagram, &units, &unit_of_node, &hints);
    let by_tier = sugiyama_order_layers(diagram, units.len(), &unit_of_node, &tiers, &hints);

    let mut placements = vec![(0.0f32, 0.0f32); units.len()];
    let mut u_cursor = 0.0f32;
    for ordered in by_tier.values() {
        let tier_w = ordered
            .iter()
            .map(|&(i, _)| blocks[i].w)
            .fold(0.0f32, f32::max);
        let total_v: f32 = ordered.iter().map(|&(i, _)| blocks[i].h).sum::<f32>()
            + ROW_GAP * ordered.len().saturating_sub(1) as f32;
        let mut v_cursor = -u_cursor * 0.36 - total_v * 0.5;
        for &(ui, _) in ordered {
            let align_u = (tier_w - blocks[ui].w) * 0.5;
            placements[ui] = (u_cursor + align_u, v_cursor);
            v_cursor += blocks[ui].h + ROW_GAP;
        }
        u_cursor += tier_w + BLOCK_GAP;
    }

    let (placed_zones, mut placed_nodes, grid_extent) =
        materialize_placements(diagram, &blocks, &placements);
    let edges = route_edges(diagram, &placed_nodes);
    placed_nodes.sort_by(|a, b| a.depth.total_cmp(&b.depth));

    CompiledLayout {
        diagram_title: diagram.title.clone(),
        zones: placed_zones,
        nodes: placed_nodes,
        edges,
        grid_extent,
    }
}

fn layout_zone_recursive(diagram: &Diagram, zid: ZoneId, depth: usize) -> LocalBlock {
    let mut sub_blocks: Vec<LocalBlock> = diagram
        .zones
        .iter()
        .filter(|z| z.parent == Some(zid))
        .map(|z| layout_zone_recursive(diagram, z.id, depth + 1))
        .collect();
    for n in diagram.nodes.iter().filter(|n| n.zone == Some(zid)) {
        sub_blocks.push(LocalBlock {
            w: CELL_STEP * 0.96,
            h: CELL_STEP * 0.96,
            zones: Vec::new(),
            nodes: vec![(n.id, CELL_STEP * 0.48, CELL_STEP * 0.48)],
        });
    }

    let pad = if depth == 0 { ZONE_PAD } else { SUBZONE_PAD };
    if sub_blocks.is_empty() {
        let min_b = GridBounds {
            u_min: 0.0,
            v_min: 0.0,
            u_max: pad * 2.5,
            v_max: pad * 2.5,
        };
        return LocalBlock {
            w: min_b.width(),
            h: min_b.height(),
            zones: vec![PlacedZone {
                id: zid,
                spec: diagram.zone(zid).clone(),
                depth,
                bounds: min_b,
            }],
            nodes: Vec::new(),
        };
    }

    let cols = (sub_blocks.len() as f32).sqrt().ceil().max(1.0) as usize;
    let gap = 0.52f32;
    let mut row_heights = Vec::new();
    let mut col_widths = vec![0.0f32; cols];
    for (idx, sb) in sub_blocks.iter().enumerate() {
        let (r, c) = (idx / cols, idx % cols);
        if row_heights.len() <= r {
            row_heights.push(0.0f32);
        }
        row_heights[r] = row_heights[r].max(sb.h);
        col_widths[c] = col_widths[c].max(sb.w);
    }

    let mut col_u = vec![pad; cols];
    for c in 1..cols {
        col_u[c] = col_u[c - 1] + col_widths[c - 1] + gap;
    }
    let mut row_v = vec![pad; row_heights.len()];
    for r in 1..row_heights.len() {
        row_v[r] = row_v[r - 1] + row_heights[r - 1] + gap;
    }

    let mut out_zones = Vec::new();
    let mut out_nodes = Vec::new();
    for (idx, sb) in sub_blocks.into_iter().enumerate() {
        let (du, dv) = (col_u[idx % cols], row_v[idx / cols]);
        for mut z in sb.zones {
            z.bounds = z.bounds.translate(du, dv);
            out_zones.push(z);
        }
        for (nid, nu, nv) in sb.nodes {
            out_nodes.push((nid, nu + du, nv + dv));
        }
    }

    let total_w = col_widths.iter().sum::<f32>() + gap * cols.saturating_sub(1) as f32 + pad * 2.0;
    let total_h = row_heights.iter().sum::<f32>()
        + gap * row_heights.len().saturating_sub(1) as f32
        + pad * 2.0;
    let mut all_zones = vec![PlacedZone {
        id: zid,
        spec: diagram.zone(zid).clone(),
        depth,
        bounds: GridBounds {
            u_min: 0.0,
            v_min: 0.0,
            u_max: total_w,
            v_max: total_h,
        },
    }];
    all_zones.extend(out_zones);

    LocalBlock {
        w: total_w,
        h: total_h,
        zones: all_zones,
        nodes: out_nodes,
    }
}

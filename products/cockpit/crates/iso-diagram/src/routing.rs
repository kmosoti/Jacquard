//! Hanan-grid orthogonal A* conduit router (`(du == 0) ^ (dv == 0)`) with
//! obstacle avoidance, turn penalties, shared-segment congestion spreading, and
//! collision-free directional chevron placement.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use crate::camera::pick_chevron_t;
use crate::layout::{PlacedNode, RoutedEdge};
use crate::model::{Asset, Diagram, NodeId};

const OBS_MARGIN: f32 = 0.22;
const TRACK_PAD: f32 = 0.34;
const BEND_PENALTY: u64 = 160;
const CONGESTION_PENALTY: u64 = 420;
const OBSTACLE_UNIT_COST: u64 = 50_000;

/// Compact orthogonal Hanan grid built from node centers, obstacle edges, and corridors.
struct HananGrid {
    us: Vec<f32>,
    vs: Vec<f32>,
}

impl HananGrid {
    fn build(nodes: &[PlacedNode]) -> Self {
        let mut raw_u = Vec::with_capacity(nodes.len() * 4 + 8);
        let mut raw_v = Vec::with_capacity(nodes.len() * 4 + 8);
        let (mut min_u, mut max_u) = (f32::MAX, f32::MIN);
        let (mut min_v, mut max_v) = (f32::MAX, f32::MIN);

        for n in nodes {
            let (u0, u1) = (n.u - n.half_u - TRACK_PAD, n.u + n.half_u + TRACK_PAD);
            let (v0, v1) = (n.v - n.half_v - TRACK_PAD, n.v + n.half_v + TRACK_PAD);
            raw_u.extend_from_slice(&[u0, n.u, u1]);
            raw_v.extend_from_slice(&[v0, n.v, v1]);
            min_u = min_u.min(u0);
            max_u = max_u.max(u1);
            min_v = min_v.min(v0);
            max_v = max_v.max(v1);
        }
        if min_u < f32::MAX {
            for ring in [0.55f32, 1.05, 1.55] {
                raw_u.push(min_u - ring);
                raw_u.push(max_u + ring);
                raw_v.push(min_v - ring);
                raw_v.push(max_v + ring);
            }
        }
        let us = compress_with_midpoints(raw_u, 0.14);
        let vs = compress_with_midpoints(raw_v, 0.14);
        Self { us, vs }
    }

    fn nearest(&self, u: f32, v: f32) -> (usize, usize) {
        (nearest_axis(&self.us, u), nearest_axis(&self.vs, v))
    }
}

fn compress_with_midpoints(mut vals: Vec<f32>, tol: f32) -> Vec<f32> {
    vals.sort_by(f32::total_cmp);
    let mut base: Vec<f32> = Vec::with_capacity(vals.len());
    for v in vals {
        if base.last().is_none_or(|&prev| (v - prev).abs() >= tol) {
            base.push(v);
        }
    }
    let mut with_mids = Vec::with_capacity(base.len() * 2);
    for w in base.windows(2) {
        with_mids.push(w[0]);
        if (w[1] - w[0]) >= 0.65 {
            with_mids.push((w[0] + w[1]) * 0.5);
        }
    }
    if let Some(&last) = base.last() {
        with_mids.push(last);
    }
    with_mids
}

fn nearest_axis(axis: &[f32], val: f32) -> usize {
    axis.iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| (*a - val).abs().total_cmp(&(*b - val).abs()))
        .map_or(0, |(i, _)| i)
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct GridEdge {
    a: (usize, usize),
    b: (usize, usize),
}

impl GridEdge {
    fn new(p0: (usize, usize), p1: (usize, usize)) -> Self {
        if p0 <= p1 {
            Self { a: p0, b: p1 }
        } else {
            Self { a: p1, b: p0 }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct State {
    f_cost: u64,
    g_cost: u64,
    ui: usize,
    vi: usize,
    dir: u8, // 0 = start, 1 = horiz (u), 2 = vert (v)
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .f_cost
            .cmp(&self.f_cost)
            .then_with(|| other.g_cost.cmp(&self.g_cost))
            .then_with(|| (self.ui, self.vi, self.dir).cmp(&(other.ui, other.vi, other.dir)))
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub(crate) fn route_edges(diagram: &Diagram, nodes: &[PlacedNode]) -> Vec<RoutedEdge> {
    let by_id: HashMap<NodeId, &PlacedNode> = nodes.iter().map(|n| (n.id, n)).collect();
    let grid = HananGrid::build(nodes);
    let mut edge_usage: HashMap<GridEdge, u32> = HashMap::new();
    let mut placed_chevrons: Vec<(f32, f32)> = Vec::with_capacity(diagram.edges.len());
    let mut routed = Vec::with_capacity(diagram.edges.len());

    for edge in &diagram.edges {
        let (Some(&a), Some(&b)) = (by_id.get(&edge.from), by_id.get(&edge.to)) else {
            continue;
        };
        let raw = route_single_astar(a, b, nodes, &grid, &mut edge_usage);
        let points = simplify_ortho(raw);
        let chevron_t = pick_chevron_t(&points, nodes, &mut placed_chevrons);
        routed.push(RoutedEdge {
            id: edge.id,
            from: edge.from,
            to: edge.to,
            spec: edge.clone(),
            points,
            chevron_t,
        });
    }
    routed
}

fn route_single_astar(
    a: &PlacedNode,
    b: &PlacedNode,
    nodes: &[PlacedNode],
    grid: &HananGrid,
    edge_usage: &mut HashMap<GridEdge, u32>,
) -> Vec<(f32, f32)> {
    if a.id == b.id {
        let (u, v) = (a.u, a.v);
        return vec![
            (u, v),
            (u + 1.0, v),
            (u + 1.0, v + 1.0),
            (u, v + 1.0),
            (u, v),
        ];
    }
    if grid.us.is_empty() || grid.vs.is_empty() {
        return vec![(a.u, a.v), (b.u, a.v), (b.u, b.v)];
    }

    let ((su, sv), (tu, tv)) = (grid.nearest(a.u, a.v), grid.nearest(b.u, b.v));
    let (nu, nv) = (grid.us.len(), grid.vs.len());
    let idx = |u: usize, v: usize, d: u8| -> usize { ((u * nv + v) * 3) + usize::from(d) };
    let mut dist = vec![u64::MAX; nu * nv * 3];
    let mut parent = vec![usize::MAX; nu * nv * 3];
    let mut heap = BinaryHeap::new();
    let start_s = idx(su, sv, 0);
    dist[start_s] = 0;
    heap.push(State {
        f_cost: manhattan_cost(grid.us[su], grid.vs[sv], grid.us[tu], grid.vs[tv]),
        g_cost: 0,
        ui: su,
        vi: sv,
        dir: 0,
    });

    let mut goal_s = None;
    while let Some(cur) = heap.pop() {
        let cs = idx(cur.ui, cur.vi, cur.dir);
        if cur.g_cost > dist[cs] {
            continue;
        }
        if cur.ui == tu && cur.vi == tv {
            goal_s = Some(cs);
            break;
        }
        let p0 = (grid.us[cur.ui], grid.vs[cur.vi]);
        for (opt_u, opt_v, ndir) in [
            (cur.ui.checked_sub(1), Some(cur.vi), 1u8),
            ((cur.ui + 1 < nu).then_some(cur.ui + 1), Some(cur.vi), 1u8),
            (Some(cur.ui), cur.vi.checked_sub(1), 2u8),
            (Some(cur.ui), (cur.vi + 1 < nv).then_some(cur.vi + 1), 2u8),
        ] {
            let (Some(nui), Some(nvi)) = (opt_u, opt_v) else {
                continue;
            };
            let p1 = (grid.us[nui], grid.vs[nvi]);
            let bend = if cur.dir != 0 && cur.dir != ndir {
                BEND_PENALTY
            } else {
                0
            };
            let ge = GridEdge::new((cur.ui, cur.vi), (nui, nvi));
            let cong = u64::from(*edge_usage.get(&ge).unwrap_or(&0)) * CONGESTION_PENALTY;
            let ng = cur
                .g_cost
                .saturating_add(manhattan_cost(p0.0, p0.1, p1.0, p1.1).max(1))
                .saturating_add(bend)
                .saturating_add(obstacle_penalty(p0, p1, a.id, b.id, nodes))
                .saturating_add(cong);
            let ns = idx(nui, nvi, ndir);
            if ng < dist[ns] {
                dist[ns] = ng;
                parent[ns] = cs;
                let h = manhattan_cost(p1.0, p1.1, grid.us[tu], grid.vs[tv]);
                heap.push(State {
                    f_cost: ng.saturating_add(h),
                    g_cost: ng,
                    ui: nui,
                    vi: nvi,
                    dir: ndir,
                });
            }
        }
    }

    let Some(mut cur_s) = goal_s else {
        return vec![(a.u, a.v), (b.u, a.v), (b.u, b.v)];
    };
    let mut cell_path = Vec::new();
    while cur_s != usize::MAX {
        let cell = cur_s / 3;
        cell_path.push((cell / nv, cell % nv));
        cur_s = parent[cur_s];
    }
    cell_path.reverse();
    for w in cell_path.windows(2) {
        *edge_usage.entry(GridEdge::new(w[0], w[1])).or_insert(0) += 1;
    }

    let mut pts = Vec::with_capacity(cell_path.len() + 4);
    pts.push((a.u, a.v));
    if let Some(&(u0, v0)) = cell_path.first() {
        let (gu, gv) = (grid.us[u0], grid.vs[v0]);
        if (gu - a.u).abs() > 1e-4 && (gv - a.v).abs() > 1e-4 {
            pts.push((gu, a.v));
        }
    }
    for (ui, vi) in cell_path {
        pts.push((grid.us[ui], grid.vs[vi]));
    }
    if let Some(&(lu, lv)) = pts.last()
        && (lu - b.u).abs() > 1e-4
        && (lv - b.v).abs() > 1e-4
    {
        pts.push((b.u, lv));
    }
    pts.push((b.u, b.v));
    pts
}

fn manhattan_cost(u0: f32, v0: f32, u1: f32, v1: f32) -> u64 {
    (((u1 - u0).abs() + (v1 - v0).abs()) * 100.0).round() as u64
}

fn node_importance_weight(asset: Asset) -> u64 {
    match asset {
        Asset::Rack { .. } | Asset::Firewall | Asset::Switch { .. } | Asset::Drum { .. } => 3,
        Asset::Router | Asset::Balancer | Asset::Cloud | Asset::Lock | Asset::Crate => 2,
        Asset::Chip | Asset::Pod | Asset::Laptop => 1,
    }
}

fn obstacle_penalty(
    p0: (f32, f32),
    p1: (f32, f32),
    src: NodeId,
    dst: NodeId,
    nodes: &[PlacedNode],
) -> u64 {
    let (min_u, max_u) = (p0.0.min(p1.0), p0.0.max(p1.0));
    let (min_v, max_v) = (p0.1.min(p1.1), p0.1.max(p1.1));
    nodes
        .iter()
        .filter(|n| n.id != src && n.id != dst)
        .filter(|n| {
            let (pu, pv) = (n.half_u + OBS_MARGIN, n.half_v + OBS_MARGIN);
            n.u + pu > min_u && n.u - pu < max_u && n.v + pv > min_v && n.v - pv < max_v
        })
        .map(|n| node_importance_weight(n.spec.asset) * OBSTACLE_UNIT_COST)
        .sum()
}

fn simplify_ortho(raw: Vec<(f32, f32)>) -> Vec<(f32, f32)> {
    let mut dedup: Vec<(f32, f32)> = Vec::with_capacity(raw.len());
    for pt in raw {
        if dedup
            .last()
            .is_none_or(|&(lu, lv)| (lu - pt.0).abs() > 1e-4 || (lv - pt.1).abs() > 1e-4)
        {
            dedup.push(pt);
        }
    }
    if dedup.len() <= 2 {
        return dedup;
    }
    let mut out = vec![dedup[0]];
    for i in 1..(dedup.len() - 1) {
        let a = *out.last().expect("non-empty");
        let b = dedup[i];
        let c = dedup[i + 1];
        let same_u = (a.0 - b.0).abs() < 1e-4 && (b.0 - c.0).abs() < 1e-4;
        let same_v = (a.1 - b.1).abs() < 1e-4 && (b.1 - c.1).abs() < 1e-4;
        if !(same_u || same_v) {
            out.push(b);
        }
    }
    out.push(*dedup.last().expect("non-empty"));
    out
}

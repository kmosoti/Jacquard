//! Sugiyama 2-pass crossing minimization with virtual nodes, adjacent-pair
//! transpose local search, explicit `(col, row)` hints, and grid materialization.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::layout::{CampusUnit, GridBounds, LocalBlock, PlacedNode, PlacedZone, asset_footprint};
use crate::model::{Diagram, NodeId};

pub(crate) fn build_node_unit_map(
    diagram: &Diagram,
    units: &[CampusUnit],
) -> HashMap<NodeId, usize> {
    let mut root_zone_to_unit = HashMap::new();
    let mut node_to_unit = HashMap::new();
    for (i, u) in units.iter().enumerate() {
        match *u {
            CampusUnit::Zone(zid) => {
                root_zone_to_unit.insert(zid, i);
            }
            CampusUnit::Node(nid) => {
                node_to_unit.insert(nid, i);
            }
        }
    }
    for n in &diagram.nodes {
        let Some(mut z) = n.zone else { continue };
        while let Some(parent) = diagram.zone(z).parent {
            z = parent;
        }
        if let Some(&ui) = root_zone_to_unit.get(&z) {
            node_to_unit.insert(n.id, ui);
        }
    }
    node_to_unit
}

pub(crate) fn compute_unit_tiers(
    diagram: &Diagram,
    units: &[CampusUnit],
    node_to_unit: &HashMap<NodeId, usize>,
    hints: &[Option<(usize, usize)>],
) -> Vec<usize> {
    let n = units.len();
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for e in &diagram.edges {
        if let (Some(&u), Some(&v)) = (node_to_unit.get(&e.from), node_to_unit.get(&e.to))
            && u != v
            && !adj[u].contains(&v)
        {
            adj[u].push(v);
        }
    }
    let mut tiers = vec![0usize; n];
    for (i, h) in hints.iter().enumerate() {
        if let Some((col, _)) = h {
            tiers[i] = *col;
        }
    }
    for _ in 0..n.min(12) {
        let mut changed = false;
        for u in 0..n {
            for &v in &adj[u] {
                if hints[v].is_none() && tiers[v] <= tiers[u] && !has_back_path(v, u, &adj) {
                    tiers[v] = tiers[u] + 1;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    tiers
}

fn has_back_path(src: usize, target: usize, adj: &[Vec<usize>]) -> bool {
    let (mut stack, mut seen) = (vec![src], HashSet::new());
    while let Some(cur) = stack.pop() {
        if cur == target {
            return true;
        }
        if seen.insert(cur) {
            stack.extend(&adj[cur]);
        }
    }
    false
}

/// Runs Sugiyama crossing minimization with virtual nodes across multi-tier edges,
/// iterative forward + backward barycenter sweeps, and adjacent-pair transpose
/// local search (`<= 6` passes), returning ordered `(real_unit_idx, aligned_slot)` per tier.
pub(crate) fn sugiyama_order_layers(
    diagram: &Diagram,
    units_len: usize,
    node_to_unit: &HashMap<NodeId, usize>,
    tiers: &[usize],
    hints: &[Option<(usize, usize)>],
) -> BTreeMap<usize, Vec<(usize, usize)>> {
    if units_len == 0 {
        return BTreeMap::new();
    }
    let max_tier = tiers.iter().copied().max().unwrap_or(0);
    let mut layers: Vec<Vec<usize>> = vec![Vec::new(); max_tier + 1];
    for (ui, &t) in tiers.iter().enumerate() {
        layers[t].push(ui);
    }
    for layer in &mut layers {
        layer.sort_by_key(|&u| (hints[u].map_or(usize::MAX, |(_, r)| r), u));
    }

    // Build normalized unit-span DAG by inserting virtual vertices (`>= units_len`).
    let mut next_vid = units_len;
    let mut layer_edges: Vec<Vec<(usize, usize)>> = vec![Vec::new(); max_tier];
    for e in &diagram.edges {
        let (Some(&u), Some(&v)) = (node_to_unit.get(&e.from), node_to_unit.get(&e.to)) else {
            continue;
        };
        if u == v || tiers[u] == tiers[v] {
            continue;
        }
        let (src, dst) = if tiers[u] < tiers[v] { (u, v) } else { (v, u) };
        let (t0, t1) = (tiers[src], tiers[dst]);
        let mut prev = src;
        let mut prev_t = t0;
        for (t, layer) in layers.iter_mut().enumerate().take(t1).skip(t0 + 1) {
            let vid = next_vid;
            next_vid += 1;
            layer.push(vid);
            layer_edges[prev_t].push((prev, vid));
            prev = vid;
            prev_t = t;
        }
        layer_edges[prev_t].push((prev, dst));
    }

    let mut best_layers = layers.clone();
    let mut best_crossings = total_crossings(&layers, &layer_edges);

    for _pass in 0..6 {
        // Forward sweep (tier t -> t + 1).
        for t in 1..layers.len() {
            let prev_pos = pos_map(&layers[t - 1]);
            barycenter_sort(&mut layers[t], &layer_edges[t - 1], &prev_pos, true, hints);
        }
        // Backward sweep (tier t + 1 -> t).
        for t in (0..layers.len().saturating_sub(1)).rev() {
            let next_pos = pos_map(&layers[t + 1]);
            barycenter_sort(&mut layers[t], &layer_edges[t], &next_pos, false, hints);
        }
        // Adjacent-pair transpose local search.
        transpose_pass(&mut layers, &layer_edges, hints);

        let cur = total_crossings(&layers, &layer_edges);
        if cur < best_crossings {
            best_crossings = cur;
            best_layers = layers.clone();
            if best_crossings == 0 {
                break;
            }
        }
    }

    let mut out: BTreeMap<usize, Vec<(usize, usize)>> = BTreeMap::new();
    for (t, layer) in best_layers.into_iter().enumerate() {
        let mut real = Vec::new();
        for (slot, vid) in layer.into_iter().enumerate() {
            if vid < units_len {
                real.push((vid, slot));
            }
        }
        if !real.is_empty() {
            out.insert(t, real);
        }
    }
    out
}

fn pos_map(layer: &[usize]) -> HashMap<usize, usize> {
    layer.iter().enumerate().map(|(i, &v)| (v, i)).collect()
}

fn is_row_pinned(vid: usize, hints: &[Option<(usize, usize)>]) -> Option<usize> {
    hints.get(vid).and_then(|h| h.map(|(_, r)| r))
}

fn barycenter_sort(
    layer: &mut [usize],
    edges: &[(usize, usize)],
    neighbor_pos: &HashMap<usize, usize>,
    forward: bool,
    hints: &[Option<(usize, usize)>],
) {
    let cur_pos: HashMap<usize, usize> = layer.iter().enumerate().map(|(i, &v)| (v, i)).collect();
    let mut keys: Vec<(usize, f32)> = layer
        .iter()
        .map(|&v| {
            let (mut sum, mut cnt) = (0.0f32, 0usize);
            for &(a, b) in edges {
                let (mine, other) = if forward { (b, a) } else { (a, b) };
                if mine == v
                    && let Some(&p) = neighbor_pos.get(&other)
                {
                    sum += p as f32;
                    cnt += 1;
                }
            }
            let bc = is_row_pinned(v, hints)
                .map(|r| r as f32)
                .unwrap_or_else(|| {
                    if cnt > 0 {
                        sum / cnt as f32
                    } else {
                        *cur_pos.get(&v).unwrap_or(&0) as f32
                    }
                });
            (v, bc)
        })
        .collect();
    keys.sort_by(|&(va, ba), &(vb, bb)| ba.total_cmp(&bb).then_with(|| va.cmp(&vb)));
    for (slot, (v, _)) in keys.into_iter().enumerate() {
        layer[slot] = v;
    }
}

fn transpose_pass(
    layers: &mut [Vec<usize>],
    layer_edges: &[Vec<(usize, usize)>],
    hints: &[Option<(usize, usize)>],
) {
    for _ in 0..4 {
        let mut improved = false;
        for t in 0..layers.len() {
            let len = layers[t].len();
            for i in 0..len.saturating_sub(1) {
                let (a, b) = (layers[t][i], layers[t][i + 1]);
                if is_row_pinned(a, hints).is_some() || is_row_pinned(b, hints).is_some() {
                    continue;
                }
                let before = incident_crossings(t, layers, layer_edges);
                layers[t].swap(i, i + 1);
                let after = incident_crossings(t, layers, layer_edges);
                if after < before {
                    improved = true;
                } else {
                    layers[t].swap(i, i + 1);
                }
            }
        }
        if !improved {
            break;
        }
    }
}

fn incident_crossings(
    t: usize,
    layers: &[Vec<usize>],
    layer_edges: &[Vec<(usize, usize)>],
) -> usize {
    let mut sum = 0;
    if t > 0 {
        sum += count_bilayer_crossings(&layers[t - 1], &layers[t], &layer_edges[t - 1]);
    }
    if t + 1 < layers.len() {
        sum += count_bilayer_crossings(&layers[t], &layers[t + 1], &layer_edges[t]);
    }
    sum
}

fn total_crossings(layers: &[Vec<usize>], layer_edges: &[Vec<(usize, usize)>]) -> usize {
    let mut sum = 0;
    for t in 0..layer_edges.len() {
        sum += count_bilayer_crossings(&layers[t], &layers[t + 1], &layer_edges[t]);
    }
    sum
}

fn count_bilayer_crossings(left: &[usize], right: &[usize], edges: &[(usize, usize)]) -> usize {
    let (lpos, rpos) = (pos_map(left), pos_map(right));
    let mut pairs: Vec<(usize, usize)> = edges
        .iter()
        .filter_map(|&(u, v)| Some((*lpos.get(&u)?, *rpos.get(&v)?)))
        .collect();
    pairs.sort_unstable();
    let mut crossings = 0usize;
    for i in 0..pairs.len() {
        crossings += pairs[(i + 1)..]
            .iter()
            .filter(|&&(u, v)| pairs[i].0 < u && pairs[i].1 > v)
            .count();
    }
    crossings
}

pub(crate) fn materialize_placements(
    diagram: &Diagram,
    blocks: &[LocalBlock],
    placements: &[(f32, f32)],
) -> (Vec<PlacedZone>, Vec<PlacedNode>, GridBounds) {
    let (mut zones, mut nodes_raw) = (Vec::new(), Vec::new());
    for (block, &(du, dv)) in blocks.iter().zip(placements.iter()) {
        zones.extend(block.zones.iter().map(|z| {
            let mut moved = z.clone();
            moved.bounds = moved.bounds.translate(du, dv);
            moved
        }));
        for &(nid, nu, nv) in &block.nodes {
            let spec = diagram.node(nid);
            let (half_u, half_v, height_z) = asset_footprint(spec.asset);
            let (u, v) = (nu + du, nv + dv);
            nodes_raw.push(PlacedNode {
                id: nid,
                spec: spec.clone(),
                u,
                v,
                depth: u + v,
                half_u,
                half_v,
                height_z,
            });
        }
    }
    let (mut min_u, mut min_v, mut max_u, mut max_v) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for z in &zones {
        (min_u, min_v) = (min_u.min(z.bounds.u_min), min_v.min(z.bounds.v_min));
        (max_u, max_v) = (max_u.max(z.bounds.u_max), max_v.max(z.bounds.v_max));
    }
    for n in &nodes_raw {
        (min_u, min_v) = (min_u.min(n.u - n.half_u), min_v.min(n.v - n.half_v));
        (max_u, max_v) = (max_u.max(n.u + n.half_u), max_v.max(n.v + n.half_v));
    }
    if min_u == f32::MAX {
        return (zones, nodes_raw, GridBounds::default());
    }
    let (su, sv) = (-min_u + 1.0, -min_v + 1.0);
    zones
        .iter_mut()
        .for_each(|z| z.bounds = z.bounds.translate(su, sv));
    for n in &mut nodes_raw {
        (n.u, n.v, n.depth) = (n.u + su, n.v + sv, n.u + su + n.v + sv);
    }
    zones.sort_by_key(|z| z.depth);
    let extent = GridBounds {
        u_min: min_u + su,
        v_min: min_v + sv,
        u_max: max_u + su,
        v_max: max_v + sv,
    };
    (zones, nodes_raw, extent)
}

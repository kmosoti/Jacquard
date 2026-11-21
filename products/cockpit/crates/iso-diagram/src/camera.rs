//! 360-degree isometric orbit camera math, directional sun lighting, and 2D hull/callout geometry.

use crate::projection::{IsoProjector, Pt2, Rect2};

/// Default horizontal orbit yaw (`45 deg`, classic 2:1 South-East isometric view).
pub const DEFAULT_YAW: f32 = std::f32::consts::FRAC_PI_4;
/// Default vertical elevation compression (`0.50` = strict 2:1 isometric ratio).
pub const DEFAULT_PITCH: f32 = 0.50;
/// Shallowest elevation compression when orbiting down toward the horizon.
pub const MIN_PITCH: f32 = 0.30;
/// Steepest elevation compression when tilting toward a high overview.
pub const MAX_PITCH: f32 = 0.72;

/// World-space directional sunlight shadow cast per unit of height along `u`.
pub const SUN_CAST_U: f32 = 0.36;
/// World-space directional sunlight shadow cast per unit of height along `v`.
pub const SUN_CAST_V: f32 = -0.14;

/// Returns the world-space shadow cast vector `(du, dv)` per unit height `z`.
pub const fn sun_cast_uv() -> (f32, f32) {
    (SUN_CAST_U, SUN_CAST_V)
}

/// True when a vertical box face with outward normal `(nu, nv)` faces the directional sun.
pub fn face_sun_lit(nu: f32, nv: f32) -> bool {
    nu * (-SUN_CAST_U) + nv * (-SUN_CAST_V) >= 0.0
}

/// Clamps elevation pitch to comfortable orthographic bounds.
pub fn clamp_pitch(pitch: f32) -> f32 {
    pitch.clamp(MIN_PITCH, MAX_PITCH)
}

/// Wraps horizontal orbit yaw smoothly to `[0, 2*pi)`.
pub fn wrap_yaw(yaw: f32) -> f32 {
    yaw.rem_euclid(std::f32::consts::TAU)
}

/// Computes the 2D convex hull of `pts` using Andrew's monotone chain.
pub fn convex_hull_pt2(pts: &[Pt2]) -> Vec<Pt2> {
    if pts.len() <= 2 {
        return pts.to_vec();
    }
    let mut sorted = pts.to_vec();
    sorted.sort_by(|a, b| a.x.total_cmp(&b.x).then_with(|| a.y.total_cmp(&b.y)));
    sorted.dedup_by(|a, b| (a.x - b.x).abs() < 1e-4 && (a.y - b.y).abs() < 1e-4);
    if sorted.len() <= 2 {
        return sorted;
    }

    let cross = |o: Pt2, a: Pt2, b: Pt2| (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x);
    let mut hull: Vec<Pt2> = Vec::with_capacity(sorted.len() * 2);
    for &p in &sorted {
        while hull.len() >= 2 && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0.0 {
            hull.pop();
        }
        hull.push(p);
    }
    let lower_len = hull.len() + 1;
    for &p in sorted.iter().rev().skip(1) {
        while hull.len() >= lower_len && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0.0
        {
            hull.pop();
        }
        hull.push(p);
    }
    if hull.len() > 1 {
        hull.pop();
    }
    hull
}

/// Ray-casting polygon containment test.
pub fn point_in_poly(p: Pt2, poly: &[Pt2]) -> bool {
    if poly.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (pi, pj) = (poly[i], poly[j]);
        if (pi.y > p.y) != (pj.y > p.y) {
            let x_int = (pj.x - pi.x) * (p.y - pi.y) / (pj.y - pi.y + 1e-6) + pi.x;
            if p.x < x_int {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

/// Resolves non-overlapping callout card rectangles against static `obstacles` and earlier cards.
pub fn resolve_label_rects_with_obstacles(
    obstacles: &[Rect2],
    desired: &[(Rect2, Pt2)],
    bounds: Rect2,
) -> Vec<Rect2> {
    const OFFSETS: &[(f32, f32)] = &[
        (0.0, 0.0),
        (0.0, -24.0),
        (0.0, 30.0),
        (-72.0, -18.0),
        (72.0, -18.0),
        (-96.0, 0.0),
        (96.0, 0.0),
        (0.0, -56.0),
        (-78.0, -48.0),
        (78.0, -48.0),
        (-74.0, 30.0),
        (74.0, 30.0),
        (0.0, -86.0),
        (-128.0, -24.0),
        (128.0, -24.0),
        (-128.0, 26.0),
        (128.0, 26.0),
        (-148.0, -56.0),
        (148.0, -56.0),
        (0.0, 62.0),
        (-102.0, 56.0),
        (102.0, 56.0),
        (0.0, -116.0),
    ];
    let mut placed: Vec<Rect2> = Vec::with_capacity(desired.len());
    for &(base_rect, anchor) in desired {
        let mut best = clamp_rect(base_rect, bounds);
        let mut best_score = f32::MAX;
        for &(dx, dy) in OFFSETS {
            let cand = clamp_rect(
                Rect2::new(base_rect.x + dx, base_rect.y + dy, base_rect.w, base_rect.h),
                bounds,
            );
            let obs_hits = obstacles
                .iter()
                .filter(|o| cand.padded(8.0).intersects(**o))
                .count();
            let card_hits = placed
                .iter()
                .filter(|p| cand.padded(6.0).intersects(**p))
                .count();
            let dist_pen = cand.center().dist(anchor) * 0.05;
            let score = (obs_hits as f32) * 400.0 + (card_hits as f32) * 1000.0 + dist_pen;
            if obs_hits == 0 && card_hits == 0 {
                best = cand;
                break;
            }
            if score < best_score {
                best_score = score;
                best = cand;
            }
        }
        placed.push(best);
    }
    placed
}

/// Evaluates normalized `t in [0.0, 1.0]` along a campus `(u, v)` polyline, returning `(pos_uv, unit_dir_uv)`.
pub fn eval_polyline_uv(points: &[(f32, f32)], t: f32) -> ((f32, f32), (f32, f32)) {
    if points.is_empty() {
        return ((0.0, 0.0), (1.0, 0.0));
    }
    if points.len() == 1 {
        return (points[0], (1.0, 0.0));
    }
    let mut total = 0.0f32;
    for w in points.windows(2) {
        total += (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1);
    }
    if total < 1e-4 {
        return (points[0], (1.0, 0.0));
    }
    let target = t.clamp(0.0, 1.0) * total;
    let mut acc = 0.0f32;
    for (idx, w) in points.windows(2).enumerate() {
        let (du, dv) = (w[1].0 - w[0].0, w[1].1 - w[0].1);
        let seg = du.hypot(dv);
        if acc + seg >= target || idx + 2 == points.len() {
            let local_t = if seg > 1e-5 {
                ((target - acc) / seg).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let n = seg.max(1e-5);
            return (
                (w[0].0 + du * local_t, w[0].1 + dv * local_t),
                (du / n, dv / n),
            );
        }
        acc += seg;
    }
    (points[0], (1.0, 0.0))
}

fn clamp_rect(r: Rect2, bounds: Rect2) -> Rect2 {
    let max_x = (bounds.right() - r.w).max(bounds.x);
    let max_y = (bounds.bottom() - r.h).max(bounds.y);
    Rect2::new(
        r.x.clamp(bounds.x, max_x),
        r.y.clamp(bounds.y, max_y),
        r.w,
        r.h,
    )
}

/// Evaluates normalized `t in [0.0, 1.0]` along a projected `(u, v)` floor polyline, returning `(pos, unit_dir)`.
pub fn eval_polyline_pt2(points: &[(f32, f32)], t: f32, proj: &IsoProjector) -> (Pt2, Pt2) {
    if points.is_empty() {
        return (Pt2::default(), Pt2::new(1.0, 0.0));
    }
    if points.len() == 1 {
        return (
            proj.project(points[0].0, points[0].1, 0.0),
            Pt2::new(1.0, 0.0),
        );
    }
    let screen: Vec<Pt2> = points
        .iter()
        .map(|&(u, v)| proj.project(u, v, 0.0))
        .collect();
    let lengths: Vec<f32> = screen.windows(2).map(|w| w[0].dist(w[1])).collect();
    let total: f32 = lengths.iter().sum();
    if total < 1e-3 {
        return (screen[0], Pt2::new(1.0, 0.0));
    }
    let target = t.clamp(0.0, 1.0) * total;
    let mut acc = 0.0f32;
    for (idx, &len) in lengths.iter().enumerate() {
        if acc + len >= target || idx + 1 == lengths.len() {
            let local_t = if len > 1e-4 {
                ((target - acc) / len).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let pos = screen[idx].lerp(screen[idx + 1], local_t);
            let delta = screen[idx + 1] - screen[idx];
            let norm = (delta.x * delta.x + delta.y * delta.y).sqrt().max(1e-4);
            return (pos, Pt2::new(delta.x / norm, delta.y / norm));
        }
        acc += len;
    }
    (screen[0], Pt2::new(1.0, 0.0))
}

pub(crate) fn pick_chevron_t(
    points: &[(f32, f32)],
    nodes: &[crate::layout::PlacedNode],
    placed_chevrons: &mut Vec<(f32, f32)>,
) -> f32 {
    const SAMPLES: [f32; 8] = [0.24, 0.32, 0.40, 0.48, 0.54, 0.62, 0.70, 0.78];
    let mut best_t = 0.52f32;
    let mut best_pos = eval_polyline_uv(points, best_t).0;
    let mut best_score = usize::MAX;
    for t in SAMPLES {
        let pos = eval_polyline_uv(points, t).0;
        let node_hits = nodes
            .iter()
            .filter(|n| {
                (pos.0 - n.u).abs() <= n.half_u + 0.26 && (pos.1 - n.v).abs() <= n.half_v + 0.26
            })
            .count();
        let corner_hits = points
            .iter()
            .skip(1)
            .take(points.len().saturating_sub(2))
            .filter(|&&(cu, cv)| (pos.0 - cu).hypot(pos.1 - cv) < 0.30)
            .count();
        let chev_hits = placed_chevrons
            .iter()
            .filter(|&&(cu, cv)| (pos.0 - cu).hypot(pos.1 - cv) < 0.48)
            .count();
        let score = node_hits * 2500
            + corner_hits * 700
            + chev_hits * 500
            + ((t - 0.50).abs() * 40.0) as usize;
        if score < best_score {
            best_score = score;
            best_t = t;
            best_pos = pos;
        }
    }
    placed_chevrons.push(best_pos);
    best_t
}

//! Compact fixed-point `ClientScene` ASCII codec (zero `serde_json` in `wasm32`)
//! and 3D AABB topological painter's sort (Kahn DAG sort).

#[cfg(feature = "ssr")]
use std::fmt::Write as _;

use crate::views::iso_client_math::{ClientProjector, fast_sin_cos, from_centi};

/// Minimal pre-compiled fixed-point scene geometry passed via SSR `data-scene`.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct ClientScene {
    /// Fixed-point centi-unit grid extent `[u_min, v_min, u_max, v_max]`.
    pub extent: [i32; 4],
    /// Floor zones as `(id, [u_min, v_min, u_max, v_max])`.
    pub zones: Vec<(usize, [i32; 4])>,
    /// Placed 3D node prisms as `(slug, title_char_len, [u_min, u_max, v_min, v_max, z_max])`.
    pub boxes: Vec<(String, usize, [i32; 5])>,
    /// Pre-routed orthogonal conduits as `(id, points)`.
    pub routes: Vec<(usize, Vec<(i32, i32)>)>,
}

impl ClientScene {
    /// Encodes `self` into a compact ASCII string (`ext|zones|boxes|routes`) on SSR.
    #[cfg(feature = "ssr")]
    #[must_use]
    pub fn encode(&self) -> String {
        let mut s = String::with_capacity(1024);
        let [a, b, c, d] = self.extent;
        let _ = write!(s, "{a},{b},{c},{d}|");
        for (i, &(id, [u0, v0, u1, v1])) in self.zones.iter().enumerate() {
            if i > 0 {
                s.push(';');
            }
            let _ = write!(s, "{id},{u0},{v0},{u1},{v1}");
        }
        s.push('|');
        for (i, (slug, tlen, [u0, u1, v0, v1, z1])) in self.boxes.iter().enumerate() {
            if i > 0 {
                s.push(';');
            }
            let _ = write!(s, "{slug},{tlen},{u0},{u1},{v0},{v1},{z1}");
        }
        s.push('|');
        for (i, (id, pts)) in self.routes.iter().enumerate() {
            if i > 0 {
                s.push(';');
            }
            let _ = write!(s, "{id}");
            for &(u, v) in pts {
                let _ = write!(s, ",{u},{v}");
            }
        }
        s
    }

    /// Decodes a compact ASCII string produced by [`Self::encode`] without `serde_json`.
    #[must_use]
    pub fn decode(raw: &str) -> Self {
        let mut parts = raw.split('|');
        let ext_s = parts.next().unwrap_or("");
        let mut ext_it = ext_s.split(',').filter_map(|x| x.parse::<i32>().ok());
        let extent = [
            ext_it.next().unwrap_or(0),
            ext_it.next().unwrap_or(0),
            ext_it.next().unwrap_or(800),
            ext_it.next().unwrap_or(800),
        ];
        let zones = parts
            .next()
            .unwrap_or("")
            .split(';')
            .filter(|r| !r.is_empty())
            .filter_map(|row| {
                let mut it = row.split(',');
                Some((
                    it.next()?.parse().ok()?,
                    [
                        it.next()?.parse().ok()?,
                        it.next()?.parse().ok()?,
                        it.next()?.parse().ok()?,
                        it.next()?.parse().ok()?,
                    ],
                ))
            })
            .collect();
        let boxes = parts
            .next()
            .unwrap_or("")
            .split(';')
            .filter(|r| !r.is_empty())
            .filter_map(|row| {
                let mut it = row.split(',');
                Some((
                    it.next()?.to_owned(),
                    it.next()?.parse().ok()?,
                    [
                        it.next()?.parse().ok()?,
                        it.next()?.parse().ok()?,
                        it.next()?.parse().ok()?,
                        it.next()?.parse().ok()?,
                        it.next()?.parse().ok()?,
                    ],
                ))
            })
            .collect();
        let routes = parts
            .next()
            .unwrap_or("")
            .split(';')
            .filter(|r| !r.is_empty())
            .filter_map(|row| {
                let mut it = row.split(',');
                let id = it.next()?.parse().ok()?;
                let mut pts = Vec::new();
                while let (Some(us), Some(vs)) = (it.next(), it.next()) {
                    if let (Ok(u), Ok(v)) = (us.parse(), vs.parse()) {
                        pts.push((u, v));
                    }
                }
                Some((id, pts))
            })
            .collect();
        Self {
            extent,
            zones,
            boxes,
            routes,
        }
    }
}

/// Topological 2.5D painter's sort over boxes (Kahn DAG sort on 3D AABB occlusion).
#[must_use]
pub fn topological_box_order(
    boxes: &[(String, usize, [i32; 5])],
    proj: ClientProjector,
) -> Vec<usize> {
    let n = boxes.len();
    if n <= 1 {
        return (0..n).collect();
    }
    let (sy, cy) = fast_sin_cos(proj.yaw);
    let rects: Vec<[f32; 4]> = boxes.iter().map(|b| box_screen_rect(b.2, proj)).collect();
    let depths: Vec<f32> = boxes
        .iter()
        .map(|b| {
            proj.depth(
                from_centi(b.2[0] + b.2[1]) * 0.5,
                from_centi(b.2[2] + b.2[3]) * 0.5,
            )
        })
        .collect();
    let mut adj = vec![Vec::new(); n];
    let mut indeg = vec![0usize; n];
    for i in 0..n {
        for j in (i + 1)..n {
            if !(rects[i][0] < rects[j][2]
                && rects[i][2] > rects[j][0]
                && rects[i][1] < rects[j][3]
                && rects[i][3] > rects[j][1])
            {
                continue;
            }
            let ij = aabb_behind(boxes[i].2, boxes[j].2, sy, cy);
            let ji = aabb_behind(boxes[j].2, boxes[i].2, sy, cy);
            if ij && !ji {
                adj[i].push(j);
                indeg[j] += 1;
            } else if ji && !ij {
                adj[j].push(i);
                indeg[i] += 1;
            }
        }
    }
    let mut done = vec![false; n];
    let mut out = Vec::with_capacity(n);
    while out.len() < n {
        let next = (0..n)
            .filter(|&i| !done[i] && indeg[i] == 0)
            .min_by(|&a, &b| depths[a].total_cmp(&depths[b]).then_with(|| a.cmp(&b)))
            .or_else(|| {
                (0..n)
                    .filter(|&i| !done[i])
                    .min_by(|&a, &b| depths[a].total_cmp(&depths[b]).then_with(|| a.cmp(&b)))
            });
        let Some(u) = next else { break };
        done[u] = true;
        out.push(u);
        for &v in &adj[u] {
            indeg[v] = indeg[v].saturating_sub(1);
        }
    }
    out
}

fn box_screen_rect(bounds: [i32; 5], proj: ClientProjector) -> [f32; 4] {
    let [u0, u1, v0, v1, z1] = bounds.map(from_centi);
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for u in [u0, u1] {
        for v in [v0, v1] {
            for z in [0.0, z1] {
                let (x, y) = proj.project(u, v, z);
                (min_x, min_y, max_x, max_y) =
                    (min_x.min(x), min_y.min(y), max_x.max(x), max_y.max(y));
            }
        }
    }
    [min_x, min_y, max_x, max_y]
}

fn aabb_behind(a: [i32; 5], b: [i32; 5], sy: f32, cy: f32) -> bool {
    let u_ok = if sy >= 0.0 {
        a[1] <= b[0]
    } else {
        a[0] >= b[1]
    };
    let v_ok = if cy >= 0.0 {
        a[3] <= b[2]
    } else {
        a[2] >= b[3]
    };
    u_ok || v_ok
}

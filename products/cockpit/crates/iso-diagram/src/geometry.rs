//! Lightweight serializable scene geometry and 2.5D topological painter's ordering.
//!
//! Exports pure geometry records (`BoxGeom`, `RouteGeom`, `LabelGeom`, `SceneGeometry`)
//! so WASM hydration islands can project pre-compiled diagrams with pure orthographic
//! math without pulling layout, routing, or DSL code into `wasm32`.

use crate::layout::CompiledLayout;
use crate::model::Tint;
use crate::projection::{IsoProjector, Rect2};

/// Axis-aligned 3D bounding box in campus `(u, v, z)` coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Aabb3 {
    /// Minimum `u` bound.
    pub u_min: f32,
    /// Maximum `u` bound.
    pub u_max: f32,
    /// Minimum `v` bound.
    pub v_min: f32,
    /// Maximum `v` bound.
    pub v_max: f32,
    /// Minimum vertical `z` bound.
    pub z_min: f32,
    /// Maximum vertical `z` bound.
    pub z_max: f32,
}

impl Aabb3 {
    /// Constructs a 3D bounding box centered on `(u, v)` spanning `z_min..=z_max`.
    #[must_use]
    pub const fn from_footprint(u: f32, v: f32, hu: f32, hv: f32, z0: f32, z1: f32) -> Self {
        Self {
            u_min: u - hu,
            u_max: u + hu,
            v_min: v - hv,
            v_max: v + hv,
            z_min: z0,
            z_max: z1,
        }
    }

    /// Computes the screen-space 2D axis-aligned bounding rectangle of all 8 corners.
    #[must_use]
    pub fn screen_bounds(self, proj: &IsoProjector) -> Rect2 {
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        for u in [self.u_min, self.u_max] {
            for v in [self.v_min, self.v_max] {
                for z in [self.z_min, self.z_max] {
                    let pt = proj.project(u, v, z);
                    min_x = min_x.min(pt.x);
                    min_y = min_y.min(pt.y);
                    max_x = max_x.max(pt.x);
                    max_y = max_y.max(pt.y);
                }
            }
        }
        Rect2::new(
            min_x,
            min_y,
            (max_x - min_x).max(0.0),
            (max_y - min_y).max(0.0),
        )
    }

    /// Geometric center `(u, v)` of the box footprint.
    #[must_use]
    pub fn center_uv(self) -> (f32, f32) {
        (
            (self.u_min + self.u_max) * 0.5,
            (self.v_min + self.v_max) * 0.5,
        )
    }
}

/// Pure floor zone geometry record for client projection.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ZoneGeom {
    /// Stable numeric zone index.
    pub id: usize,
    /// Human-readable zone title.
    pub title: String,
    /// Tint tag string.
    pub tint: String,
    /// Zone style tag (`"solid"`, `"dashed"`, `"wireframe"`).
    pub style: String,
    /// Nesting depth (`0` for root zones).
    pub depth: usize,
    /// Floor minimum `u`.
    pub u_min: f32,
    /// Floor minimum `v`.
    pub v_min: f32,
    /// Floor maximum `u`.
    pub u_max: f32,
    /// Floor maximum `v`.
    pub v_max: f32,
}

/// Pure 3D prism node geometry record for client projection.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BoxGeom {
    /// Stable numeric node index.
    pub id: usize,
    /// Identifier slug.
    pub slug: String,
    /// Display title.
    pub title: String,
    /// Optional compact telemetry subtitle.
    pub subtitle: Option<String>,
    /// Hardware asset kind name.
    pub asset_kind: String,
    /// Resolved semantic tint tag.
    pub tint: String,
    /// Operational health state tag.
    pub status: String,
    /// 3D axis-aligned bounding box in campus space.
    pub aabb: Aabb3,
}

/// Pure routed orthogonal conduit geometry record.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RouteGeom {
    /// Stable numeric edge index.
    pub id: usize,
    /// Source node index.
    pub from: usize,
    /// Target node index.
    pub to: usize,
    /// Stroke style tag (`"solid"`, `"dashed"`, `"dotted"`).
    pub style: String,
    /// Semantic tint tag.
    pub tint: String,
    /// Optional badge label.
    pub label: Option<String>,
    /// Normalized parameter for directional chevron placement.
    pub chevron_t: f32,
    /// Orthogonal polyline waypoints in `(u, v)` floor units.
    pub points: Vec<(f32, f32)>,
}

/// Pure callout label anchor record.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LabelGeom {
    /// Owning node index.
    pub owner_id: usize,
    /// Label text.
    pub text: String,
    /// Anchor `u` coordinate.
    pub u: f32,
    /// Anchor `v` coordinate.
    pub v: f32,
    /// Anchor `z` elevation.
    pub z: f32,
}

/// Complete lightweight pre-compiled 2.5D scene geometry ready for WASM projection.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SceneGeometry {
    /// Scene title.
    pub title: String,
    /// Floor grid minimum `u`.
    pub grid_u_min: f32,
    /// Floor grid minimum `v`.
    pub grid_v_min: f32,
    /// Floor grid maximum `u`.
    pub grid_u_max: f32,
    /// Floor grid maximum `v`.
    pub grid_v_max: f32,
    /// Depth-ordered floor zones.
    pub zones: Vec<ZoneGeom>,
    /// Placed 3D node prisms.
    pub boxes: Vec<BoxGeom>,
    /// Routed orthogonal floor conduits.
    pub routes: Vec<RouteGeom>,
    /// Node callout anchors.
    pub labels: Vec<LabelGeom>,
}

/// Converts a [`CompiledLayout`] into a compact [`SceneGeometry`] snapshot.
#[must_use]
pub fn compile_scene_geometry(layout: &CompiledLayout) -> SceneGeometry {
    let zones = layout
        .zones
        .iter()
        .map(|z| ZoneGeom {
            id: z.id.0,
            title: z.spec.title.clone(),
            tint: format!("{:?}", z.spec.tint).to_ascii_lowercase(),
            style: format!("{:?}", z.spec.style).to_ascii_lowercase(),
            depth: z.depth,
            u_min: z.bounds.u_min,
            v_min: z.bounds.v_min,
            u_max: z.bounds.u_max,
            v_max: z.bounds.v_max,
        })
        .collect();
    let mut boxes = Vec::with_capacity(layout.nodes.len());
    let mut labels = Vec::with_capacity(layout.nodes.len());
    for n in &layout.nodes {
        let tint = n
            .spec
            .tint
            .or_else(|| {
                n.spec.zone.and_then(|zid| {
                    layout
                        .zones
                        .iter()
                        .find(|pz| pz.id == zid)
                        .map(|pz| pz.spec.tint)
                })
            })
            .unwrap_or(Tint::Cobalt);
        boxes.push(BoxGeom {
            id: n.id.0,
            slug: n.spec.slug.clone(),
            title: n.spec.title.clone(),
            subtitle: n.spec.subtitle.clone(),
            asset_kind: n.spec.asset.kind_name().to_owned(),
            tint: format!("{tint:?}").to_ascii_lowercase(),
            status: format!("{:?}", n.spec.status).to_ascii_lowercase(),
            aabb: Aabb3::from_footprint(n.u, n.v, n.half_u, n.half_v, 0.0, n.height_z),
        });
        labels.push(LabelGeom {
            owner_id: n.id.0,
            text: n.spec.title.clone(),
            u: n.u + n.half_u * 0.15,
            v: n.v + n.half_v * 0.85,
            z: 0.0,
        });
    }
    let routes = layout
        .edges
        .iter()
        .map(|e| RouteGeom {
            id: e.id.0,
            from: e.from.0,
            to: e.to.0,
            style: format!("{:?}", e.spec.style).to_ascii_lowercase(),
            tint: format!("{:?}", e.spec.tint).to_ascii_lowercase(),
            label: e.spec.label.clone(),
            chevron_t: e.chevron_t,
            points: e.points.clone(),
        })
        .collect();
    SceneGeometry {
        title: layout.diagram_title.clone(),
        grid_u_min: layout.grid_extent.u_min,
        grid_v_min: layout.grid_extent.v_min,
        grid_u_max: layout.grid_extent.u_max,
        grid_v_max: layout.grid_extent.v_max,
        zones,
        boxes,
        routes,
        labels,
    }
}

/// Orders 3D AABBs back-to-front using an occlusion DAG and Kahn topological sort.
///
/// Whenever two boxes have overlapping 2D projected screen bounds, adds an ordering
/// edge `a -> b` if `a` strictly precedes `b` along the camera-facing `u`, `v`, or `z`
/// axes. Deterministic tie-breaking uses `(centroid_depth, index)`.
#[must_use]
pub fn topological_painter_order(boxes: &[Aabb3], proj: &IsoProjector) -> Vec<usize> {
    let n = boxes.len();
    if n <= 1 {
        return (0..n).collect();
    }
    let (sy, cy) = proj.yaw.sin_cos();
    let rects: Vec<Rect2> = boxes.iter().map(|b| b.screen_bounds(proj)).collect();
    let depths: Vec<f32> = boxes
        .iter()
        .map(|b| {
            let (cu, cv) = b.center_uv();
            proj.depth(cu, cv) + (b.z_min + b.z_max) * 0.05
        })
        .collect();

    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut indeg = vec![0usize; n];
    for i in 0..n {
        for j in (i + 1)..n {
            if !rects[i].intersects(rects[j]) {
                continue;
            }
            let i_behind_j = is_behind(boxes[i], boxes[j], sy, cy);
            let j_behind_i = is_behind(boxes[j], boxes[i], sy, cy);
            if i_behind_j && !j_behind_i {
                adj[i].push(j);
                indeg[j] += 1;
            } else if j_behind_i && !i_behind_j {
                adj[j].push(i);
                indeg[i] += 1;
            }
        }
    }

    let mut emitted = vec![false; n];
    let mut order = Vec::with_capacity(n);
    while order.len() < n {
        let next = (0..n)
            .filter(|&i| !emitted[i] && indeg[i] == 0)
            .min_by(|&a, &b| depths[a].total_cmp(&depths[b]).then_with(|| a.cmp(&b)))
            .or_else(|| {
                // Cycle fallback: break deterministically by furthest remaining centroid depth.
                (0..n)
                    .filter(|&i| !emitted[i])
                    .min_by(|&a, &b| depths[a].total_cmp(&depths[b]).then_with(|| a.cmp(&b)))
            });
        let Some(u) = next else {
            break;
        };
        emitted[u] = true;
        order.push(u);
        for &v in &adj[u] {
            indeg[v] = indeg[v].saturating_sub(1);
        }
    }
    order
}

fn is_behind(a: Aabb3, b: Aabb3, sy: f32, cy: f32) -> bool {
    const EPS: f32 = 1e-4;
    let u_behind = if sy >= 0.0 {
        a.u_max <= b.u_min + EPS
    } else {
        a.u_min >= b.u_max - EPS
    };
    let v_behind = if cy >= 0.0 {
        a.v_max <= b.v_min + EPS
    } else {
        a.v_min >= b.v_max - EPS
    };
    let z_behind = a.z_max <= b.z_min + EPS;
    u_behind || v_behind || z_behind
}

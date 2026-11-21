//! Isometric 2:1 projection math with 360-degree orbit camera, polyline evaluation, hit testing, and callout de-overlap.

use crate::camera::{DEFAULT_PITCH, DEFAULT_YAW, clamp_pitch, convex_hull_pt2, wrap_yaw};
pub use crate::camera::{eval_polyline_pt2, point_in_poly, resolve_label_rects_with_obstacles};
pub use crate::geometry::{Aabb3, topological_painter_order};
use crate::layout::CompiledLayout;
use crate::model::NodeId;

/// 2D screen-space point.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Pt2 {
    /// Horizontal screen coordinate in CSS pixels.
    pub x: f32,
    /// Vertical screen coordinate in CSS pixels.
    pub y: f32,
}

impl std::ops::Add for Pt2 {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        Self::new(self.x + o.x, self.y + o.y)
    }
}

impl std::ops::Sub for Pt2 {
    type Output = Self;
    fn sub(self, o: Self) -> Self {
        Self::new(self.x - o.x, self.y - o.y)
    }
}

impl Pt2 {
    /// Constructs a 2D point from `(x, y)` coordinates.
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Multiplies both coordinates by `s`.
    pub fn scale(self, s: f32) -> Self {
        Self::new(self.x * s, self.y * s)
    }

    /// Euclidean distance between `self` and `o`.
    pub fn dist(self, o: Self) -> f32 {
        (self.x - o.x).hypot(self.y - o.y)
    }

    /// Linearly interpolates toward `o` by `t in [0, 1]`.
    pub fn lerp(self, o: Self, t: f32) -> Self {
        Self::new(self.x + (o.x - self.x) * t, self.y + (o.y - self.y) * t)
    }
}

/// Axis-aligned 2D rectangle in screen coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect2 {
    /// Left X coordinate.
    pub x: f32,
    /// Top Y coordinate.
    pub y: f32,
    /// Width in pixels.
    pub w: f32,
    /// Height in pixels.
    pub h: f32,
}

impl Rect2 {
    /// Constructs an axis-aligned rectangle.
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    /// Right edge X coordinate.
    pub fn right(self) -> f32 {
        self.x + self.w
    }

    /// Bottom edge Y coordinate.
    pub fn bottom(self) -> f32 {
        self.y + self.h
    }

    /// Geometric center point.
    pub fn center(self) -> Pt2 {
        Pt2::new(self.x + self.w * 0.5, self.y + self.h * 0.5)
    }

    /// Returns a rectangle expanded symmetrically by `pad`.
    pub fn padded(self, pad: f32) -> Self {
        Self::new(
            self.x - pad,
            self.y - pad,
            self.w + pad * 2.0,
            self.h + pad * 2.0,
        )
    }

    /// True if `self` strictly overlaps `o`.
    pub fn intersects(self, o: Self) -> bool {
        self.x < o.right() && self.right() > o.x && self.y < o.bottom() && self.bottom() > o.y
    }
}

/// 2:1 isometric projection transforming `(u, v, z)` campus space to 2D screen pixels,
/// supporting 360-degree orbit yaw, elevation pitch, and viewport-anchored pivoting.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IsoProjector {
    /// Screen X origin offset.
    pub origin_x: f32,
    /// Screen Y origin offset.
    pub origin_y: f32,
    /// Horizontal tile width in pixels.
    pub tile_w: f32,
    /// Vertical tile height in pixels.
    pub tile_h: f32,
    /// Vertical extrusion scale in pixels per world unit.
    pub scale_z: f32,
    /// Orbit horizontal azimuth angle in radians.
    pub yaw: f32,
    /// Elevation foreshortening ratio.
    pub pitch: f32,
    /// Campus pivot `u` coordinate.
    pub pivot_u: f32,
    /// Campus pivot `v` coordinate.
    pub pivot_v: f32,
}

impl IsoProjector {
    /// Fits `layout` cleanly inside a `(width, height)` viewport using exact projected bounds.
    pub fn fit(layout: &CompiledLayout, width: f32, height: f32, margin: f32) -> Self {
        let mut min_ix = f32::MAX;
        let mut max_ix = f32::MIN;
        let mut min_iy = f32::MAX;
        let mut max_iy = f32::MIN;

        for z in &layout.zones {
            let b = z.bounds;
            for (u, v) in [
                (b.u_min, b.v_min),
                (b.u_max, b.v_min),
                (b.u_max, b.v_max),
                (b.u_min, b.v_max),
            ] {
                let ix = u - v;
                let iy = (u + v) * 0.5;
                min_ix = min_ix.min(ix);
                max_ix = max_ix.max(ix);
                min_iy = min_iy.min(iy);
                max_iy = max_iy.max(iy);
            }
        }
        for n in &layout.nodes {
            for (du, dv, z) in [
                (-n.half_u - 0.35, -n.half_v - 0.35, n.height_z + 0.72),
                (n.half_u + 0.35, -n.half_v - 0.35, 0.0),
                (n.half_u + 0.35, n.half_v + 0.35, -0.15),
                (-n.half_u - 0.35, n.half_v + 0.35, 0.0),
            ] {
                let (u, v) = (n.u + du, n.v + dv);
                let ix = u - v;
                let iy = (u + v) * 0.5 - z * 0.92;
                min_ix = min_ix.min(ix);
                max_ix = max_ix.max(ix);
                min_iy = min_iy.min(iy);
                max_iy = max_iy.max(iy);
            }
        }
        if min_ix == f32::MAX {
            let ext = layout.grid_extent;
            min_ix = ext.u_min - ext.v_max;
            max_ix = ext.u_max - ext.v_min;
            min_iy = (ext.u_min + ext.v_min) * 0.5;
            max_iy = (ext.u_max + ext.v_max) * 0.5;
        }

        let span_x = (max_ix - min_ix).max(3.5);
        let span_y = (max_iy - min_iy).max(2.5);

        let avail_w = (width - margin * 2.0).max(120.0);
        let avail_h = (height - margin * 2.0 - 16.0).max(120.0);

        let tile_w = (avail_w / span_x).min(avail_h / span_y).clamp(8.0, 74.0);
        let tile_h = tile_w * 0.5;
        let scale_z = tile_w * 0.92;

        let mid_x = (min_ix + max_ix) * 0.5 * tile_w;
        let mid_y = (min_iy + max_iy) * 0.5 * tile_w;
        let origin_x = width * 0.5 - mid_x;
        let origin_y = height * 0.53 - mid_y;

        Self {
            origin_x,
            origin_y,
            tile_w,
            tile_h,
            scale_z,
            yaw: DEFAULT_YAW,
            pitch: DEFAULT_PITCH,
            pivot_u: 0.0,
            pivot_v: 0.0,
        }
    }

    /// Applies horizontal orbit `yaw`, vertical `pitch`, and campus pivot `(pivot_u, pivot_v)`.
    pub fn with_orbit(mut self, yaw: f32, pitch: f32, pivot_u: f32, pivot_v: f32) -> Self {
        self.yaw = wrap_yaw(yaw);
        self.pitch = clamp_pitch(pitch);
        self.pivot_u = pivot_u;
        self.pivot_v = pivot_v;
        self
    }

    /// Projects campus-space `(u, v, z)` into screen coordinates under the active orbit camera.
    pub fn project(&self, u: f32, v: f32, z: f32) -> Pt2 {
        let du = u - self.pivot_u;
        let dv = v - self.pivot_v;
        let (sy, cy) = self.yaw.sin_cos();
        let rx = (du * cy - dv * sy) * std::f32::consts::SQRT_2;
        let ry = (du * sy + dv * cy) * std::f32::consts::SQRT_2;
        let base_x = (self.pivot_u - self.pivot_v) * self.tile_w;
        let base_y = (self.pivot_u + self.pivot_v) * self.tile_h;
        let z_factor = ((1.0 - self.pitch * self.pitch).max(0.18) / 0.75).sqrt();
        Pt2::new(
            self.origin_x + base_x + rx * self.tile_w,
            self.origin_y + base_y + ry * (self.tile_w * self.pitch)
                - z * (self.scale_z * z_factor),
        )
    }

    /// Exact inverse of `project(u, v, 0.0)` mapping a screen point back to floor `(u, v)`.
    pub fn unproject_floor(&self, sx: f32, sy_px: f32) -> (f32, f32) {
        let base_x = (self.pivot_u - self.pivot_v) * self.tile_w;
        let base_y = (self.pivot_u + self.pivot_v) * self.tile_h;
        let rx = (sx - (self.origin_x + base_x)) / self.tile_w.max(1.0);
        let ry = (sy_px - (self.origin_y + base_y)) / (self.tile_w * self.pitch).max(1.0);
        let (sy, cy) = self.yaw.sin_cos();
        let du = (rx * cy + ry * sy) * std::f32::consts::FRAC_1_SQRT_2;
        let dv = (ry * cy - rx * sy) * std::f32::consts::FRAC_1_SQRT_2;
        (self.pivot_u + du, self.pivot_v + dv)
    }

    /// Builds a zoomed sprite projector pinned at floor tile `(u, v)` preserving the active orbit angle.
    pub fn local_sprite_projector(&self, u: f32, v: f32, scale: f32) -> Self {
        let anchor = self.project(u, v, 0.0);
        let tile_w = self.tile_w * scale;
        let tile_h = self.tile_h * scale;
        Self {
            origin_x: anchor.x - (u - v) * tile_w,
            origin_y: anchor.y - (u + v) * tile_h,
            tile_w,
            tile_h,
            scale_z: self.scale_z * scale,
            yaw: self.yaw,
            pitch: self.pitch,
            pivot_u: u,
            pivot_v: v,
        }
    }

    /// Painter's-order depth along the camera view axis (smaller = further back, larger = closer).
    pub fn depth(&self, u: f32, v: f32) -> f32 {
        let (sy, cy) = self.yaw.sin_cos();
        (u - self.pivot_u) * sy + (v - self.pivot_v) * cy
    }

    /// True if a vertical wall with outward normal `(nu, nv)` faces toward the camera.
    pub fn face_visible(&self, nu: f32, nv: f32) -> bool {
        let (sy, cy) = self.yaw.sin_cos();
        nu * sy + nv * cy > 0.04
    }

    /// Computes the screen-space convex hull silhouette of an axis-aligned 3D prism.
    pub fn prism_hull(&self, u: f32, v: f32, hu: f32, hv: f32, z0: f32, z1: f32) -> Vec<Pt2> {
        let c = self.box_prism(u, v, hu, hv, z0, z1);
        convex_hull_pt2(&c)
    }

    /// Projects an axis-aligned 3D box `[top_n, top_e, top_s, top_w, bot_n, bot_e, bot_s, bot_w]`.
    pub fn box_prism(&self, u: f32, v: f32, hu: f32, hv: f32, z0: f32, z1: f32) -> [Pt2; 8] {
        [
            self.project(u - hu, v - hv, z1),
            self.project(u + hu, v - hv, z1),
            self.project(u + hu, v + hv, z1),
            self.project(u - hu, v + hv, z1),
            self.project(u - hu, v - hv, z0),
            self.project(u + hu, v - hv, z0),
            self.project(u + hu, v + hv, z0),
            self.project(u - hu, v + hv, z0),
        ]
    }
}

/// Hit-tests `cursor` against placed nodes in front-to-back topological painter's order.
pub fn hit_test(layout: &CompiledLayout, proj: &IsoProjector, cursor: Pt2) -> Option<NodeId> {
    let boxes: Vec<Aabb3> = layout
        .nodes
        .iter()
        .map(|n| Aabb3::from_footprint(n.u, n.v, n.half_u, n.half_v, 0.0, n.height_z))
        .collect();
    let order = topological_painter_order(&boxes, proj);
    for idx in order.into_iter().rev() {
        let node = &layout.nodes[idx];
        let hull = proj.prism_hull(
            node.u,
            node.v,
            node.half_u + 0.14,
            node.half_v + 0.14,
            0.0,
            node.height_z + 0.08,
        );
        if point_in_poly(cursor, &hull) {
            return Some(node.id);
        }
    }
    None
}

//! Tiny pure-math orthographic 2.5D projector and integer-tenths SVG formatters
//! (zero `serde_json`, zero `core::fmt::float`, zero `libm` tables in `wasm32`).

use std::fmt::Write as _;

pub use crate::views::iso_client_scene::{ClientScene, topological_box_order};

/// Logical SVG viewport width for the 2.5D stage.
pub const STAGE_W: f32 = 1080.0;
/// Logical SVG viewport height for the 2.5D stage.
pub const STAGE_H: f32 = 660.0;

#[inline]
pub(crate) fn from_centi(v: i32) -> f32 {
    (v as f32) * 0.01
}

/// Panic-free `f32` clamp that never pulls `core::fmt::float` (`dragon`/`grisu`) into `wasm32`.
#[inline]
#[must_use]
pub fn clamp_f32(x: f32, lo: f32, hi: f32) -> f32 {
    if x < lo {
        lo
    } else if x > hi {
        hi
    } else {
        x
    }
}

/// Fast branchless `(sin, cos)` avoiding `libm::rem_pio2_large` tables in `wasm32`.
#[inline]
#[must_use]
pub fn fast_sin_cos(rad: f32) -> (f32, f32) {
    fn sin_approx(r: f32) -> f32 {
        const PI: f32 = std::f32::consts::PI;
        const TAU: f32 = std::f32::consts::TAU;
        let x = r - (r / TAU).round() * TAU;
        let b = 4.0 / PI;
        let c = -4.0 / (PI * PI);
        let y = b * x + c * x * x.abs();
        0.225 * (y * y.abs() - y) + y
    }
    (
        sin_approx(rad),
        sin_approx(rad + std::f32::consts::FRAC_PI_2),
    )
}

/// Pure-math 2:1 orthographic orbit projector (`wasm32`-safe, zero dependencies).
#[derive(Clone, Copy, PartialEq)]
pub struct ClientProjector {
    /// Screen X origin.
    pub origin_x: f32,
    /// Screen Y origin.
    pub origin_y: f32,
    /// Horizontal tile width.
    pub tile_w: f32,
    /// Vertical extrusion scale.
    pub scale_z: f32,
    /// Orbit yaw angle in radians.
    pub yaw: f32,
    /// Elevation pitch ratio (`0.30..=0.72`).
    pub pitch: f32,
    /// Orbit pivot `u`.
    pub pivot_u: f32,
    /// Orbit pivot `v`.
    pub pivot_v: f32,
}

impl ClientProjector {
    /// Fits `scene` inside `STAGE_W x STAGE_H` and applies `(yaw_deg, pitch_deg, is_2d)`.
    #[must_use]
    pub fn fit_orbit(scene: &ClientScene, yaw_deg: f32, pitch_deg: f32, is_2d: bool) -> Self {
        let [u0, v0, u1, v1] = scene.extent.map(from_centi);
        let span_u = (u1 - u0).max(4.0);
        let span_v = (v1 - v0).max(4.0);
        let span_x = (span_u + span_v).max(6.0);
        let span_y = ((span_u + span_v) * 0.5 + 3.2).max(4.5);
        let tile_w = clamp_f32(
            ((STAGE_W - 88.0) / span_x).min((STAGE_H - 96.0) / span_y),
            12.0,
            66.0,
        );
        let (yaw, pitch, scale_z) = if is_2d {
            (0.0, 0.72, tile_w * 0.14)
        } else {
            (
                yaw_deg.to_radians().rem_euclid(std::f32::consts::TAU),
                clamp_f32(pitch_deg / 70.528, 0.30, 0.72),
                tile_w * 0.92,
            )
        };
        Self {
            origin_x: STAGE_W * 0.5,
            origin_y: STAGE_H * 0.52,
            tile_w,
            scale_z,
            yaw,
            pitch,
            pivot_u: (u0 + u1) * 0.5,
            pivot_v: (v0 + v1) * 0.5,
        }
    }

    /// Projects campus `(u, v, z)` into `(x, y)` SVG coordinates.
    #[must_use]
    pub fn project(self, u: f32, v: f32, z: f32) -> (f32, f32) {
        let (du, dv) = (u - self.pivot_u, v - self.pivot_v);
        let (sy, cy) = fast_sin_cos(self.yaw);
        let rx = (du * cy - dv * sy) * std::f32::consts::SQRT_2;
        let ry = (du * sy + dv * cy) * std::f32::consts::SQRT_2;
        let z_fac = ((1.0 - self.pitch * self.pitch).max(0.18) / 0.75).sqrt();
        (
            self.origin_x + rx * self.tile_w,
            self.origin_y + ry * (self.tile_w * self.pitch) - z * (self.scale_z * z_fac),
        )
    }

    /// Projects fixed-point centi-unit `(u_c, v_c)` on `z` into `(x, y)` SVG coordinates.
    #[must_use]
    pub fn project_c(self, u_c: i32, v_c: i32, z: f32) -> (f32, f32) {
        self.project(from_centi(u_c), from_centi(v_c), z)
    }

    /// Camera depth along look axis (smaller = further back, larger = closer).
    #[must_use]
    pub fn depth(self, u: f32, v: f32) -> f32 {
        let (sy, cy) = fast_sin_cos(self.yaw);
        (u - self.pivot_u) * sy + (v - self.pivot_v) * cy
    }
}

/// Projected fixed-slot SVG coordinates for one node box (zero DOM rebuilds on orbit).
#[derive(Clone, PartialEq)]
pub struct ProjectedBox {
    /// Selection ring floor polygon `points`.
    pub ring: String,
    /// Ground shadow polygon `points`.
    pub shadow: String,
    /// First camera-facing vertical wall `points`.
    pub side_a: String,
    /// Second camera-facing vertical wall `points`.
    pub side_b: String,
    /// Roof top face `points`.
    pub top: String,
    /// Status LED center `(cx, cy)`.
    pub led: (f32, f32),
    /// Callout box top-left `(x, y, width)`.
    pub callout: (f32, f32, f32),
}

/// Projects a fixed-point box `(title_len, bounds)` into its 6 SVG primitives.
#[must_use]
pub fn project_box_bounds(
    title_len: usize,
    bounds: [i32; 5],
    proj: ClientProjector,
) -> ProjectedBox {
    let [u0, u1, v0, v1, z1] = bounds.map(from_centi);
    let ring = fmt_pts(&[
        proj.project(u0 - 0.14, v0 - 0.14, 0.01),
        proj.project(u1 + 0.14, v0 - 0.14, 0.01),
        proj.project(u1 + 0.14, v1 + 0.14, 0.01),
        proj.project(u0 - 0.14, v1 + 0.14, 0.01),
    ]);
    let (su, sv) = (0.36 * z1, -0.14 * z1);
    let shadow = fmt_pts(&[
        proj.project(u0 + su, v0 + sv, 0.0),
        proj.project(u1 + su, v0 + sv, 0.0),
        proj.project(u1 + su, v1 + sv, 0.0),
        proj.project(u0 + su, v1 + sv, 0.0),
    ]);
    let (sy, cy) = fast_sin_cos(proj.yaw);
    let u_front = if sy >= 0.0 { u1 } else { u0 };
    let v_front = if cy >= 0.0 { v1 } else { v0 };
    let side_a = fmt_pts(&[
        proj.project(u_front, v0, 0.0),
        proj.project(u_front, v1, 0.0),
        proj.project(u_front, v1, z1),
        proj.project(u_front, v0, z1),
    ]);
    let side_b = fmt_pts(&[
        proj.project(u0, v_front, 0.0),
        proj.project(u1, v_front, 0.0),
        proj.project(u1, v_front, z1),
        proj.project(u0, v_front, z1),
    ]);
    let top = fmt_pts(&[
        proj.project(u0, v0, z1),
        proj.project(u1, v0, z1),
        proj.project(u1, v1, z1),
        proj.project(u0, v1, z1),
    ]);
    let led = proj.project(u_front, (v0 + v1) * 0.5, z1 * 0.82);
    let anchor = proj.project((u0 + u1) * 0.5, v1 + 0.25, 0.0);
    let cw = clamp_f32((title_len as f32) * 6.8 + 22.0, 84.0, 168.0);
    ProjectedBox {
        ring,
        shadow,
        side_a,
        side_b,
        top,
        led,
        callout: (
            clamp_f32(anchor.0 - cw * 0.5, 8.0, STAGE_W - cw - 8.0),
            clamp_f32(anchor.1 + 6.0, 8.0, STAGE_H - 40.0),
            cw,
        ),
    }
}

/// Appends `v` formatted to 1 decimal place using integer arithmetic.
pub fn push_t1(out: &mut String, v: f32) {
    let t = (v * 10.0).round() as i32;
    if t < 0 {
        out.push('-');
    }
    let abs = t.unsigned_abs();
    let _ = write!(out, "{}.{}", abs / 10, abs % 10);
}

/// Formats `v` to 1 decimal place without linking `core::fmt::float`.
#[must_use]
pub fn fmt_t1(v: f32) -> String {
    let mut s = String::with_capacity(8);
    push_t1(&mut s, v);
    s
}

/// Formats SVG polyline/polygon coordinate pairs using integer tenths.
#[must_use]
pub fn fmt_pts(pts: &[(f32, f32)]) -> String {
    let mut out = String::with_capacity(pts.len() * 14);
    for (i, &(x, y)) in pts.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        push_t1(&mut out, x);
        out.push(',');
        push_t1(&mut out, y);
    }
    out
}

/// Computes an O(1) centered SVG `viewBox` string for `zoom`.
#[must_use]
pub fn zoom_viewbox(zoom: f32) -> String {
    let z = clamp_f32(zoom, 0.55, 2.2);
    let (vw, vh) = (STAGE_W / z, STAGE_H / z);
    let mut out = String::with_capacity(32);
    push_t1(&mut out, (STAGE_W - vw) * 0.5);
    out.push(' ');
    push_t1(&mut out, (STAGE_H - vh) * 0.5);
    out.push(' ');
    push_t1(&mut out, vw);
    out.push(' ');
    push_t1(&mut out, vh);
    out
}

/// Maps tint tag string to Hallmark CSS custom property.
#[cfg(feature = "ssr")]
#[must_use]
pub fn tint_css_var(tint: &str) -> &'static str {
    match tint {
        "pine" => "var(--iso-tint-pine)",
        "amber" => "var(--iso-tint-amber)",
        "crimson" => "var(--iso-tint-crimson)",
        "plum" => "var(--iso-tint-plum)",
        "sage" => "var(--iso-tint-sage)",
        "slate" => "var(--iso-tint-slate)",
        _ => "var(--iso-tint-cobalt)",
    }
}

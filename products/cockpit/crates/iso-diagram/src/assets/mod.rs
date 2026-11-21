//! Backend-agnostic 2.5D procedural hardware sprite primitives (`DrawPrim`).

mod compute_storage;
mod network_client;
mod shadow;

use crate::camera::face_sun_lit;
use crate::model::Asset;
use crate::projection::{IsoProjector, Pt2};
pub(crate) use shadow::{push_box_shadow, push_cyl_shadow};

/// Semantic face/detail shading token resolved against the active theme + node tint + status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tone {
    /// Soft outer ambient penumbra shadow.
    ShadowSoft,
    /// Primary cast umbra shadow.
    Shadow,
    /// Contact ambient-occlusion floor crease.
    ShadowCore,
    /// Sun-facing horizontal top plane.
    Top,
    /// Lit vertical chassis face.
    Left,
    /// Shaded vertical chassis face.
    Right,
    /// Highlighted top plane using the node accent color.
    AccentTop,
    /// Shaded side plane using the node accent color.
    AccentSide,
    /// Brushed light alloy panel.
    MetalLight,
    /// Brushed dark alloy bezel.
    MetalDark,
    /// Recessed chassis vent / dark cavity.
    Dark,
    /// Crisp technical ink outline.
    Outline,
    /// Status indicator LED dot.
    Led,
}

/// Pure vector primitive emitted by all procedural 2.5D hardware assets.
#[derive(Debug, Clone, PartialEq)]
pub enum DrawPrim {
    /// Closed polygon with optional fill and stroke.
    Polygon {
        /// Ordered perimeter vertices.
        pts: Vec<Pt2>,
        /// Optional semantic fill tone.
        fill: Option<Tone>,
        /// Optional semantic outline tone.
        stroke: Option<Tone>,
        /// Outline width in CSS pixels.
        width: f32,
    },
    /// Open polyline accent stroke.
    Polyline {
        /// Ordered path vertices.
        pts: Vec<Pt2>,
        /// Semantic stroke tone.
        stroke: Tone,
        /// Line width in CSS pixels.
        width: f32,
    },
    /// Circular badge or LED marker.
    Circle {
        /// Screen-space center point.
        center: Pt2,
        /// Circle radius in CSS pixels.
        radius: f32,
        /// Optional fill tone.
        fill: Option<Tone>,
        /// Optional stroke tone.
        stroke: Option<Tone>,
        /// Stroke width in CSS pixels.
        width: f32,
    },
}

/// Specification for drawing an axis-aligned 3D isometric box.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PrismSpec {
    pub u: f32,
    pub v: f32,
    pub hu: f32,
    pub hv: f32,
    pub z0: f32,
    pub z1: f32,
    pub top: Tone,
    pub left: Tone,
    pub right: Tone,
}

/// Specification for drawing an upright isometric cylinder.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CylinderSpec {
    pub u: f32,
    pub v: f32,
    pub radius: f32,
    pub z0: f32,
    pub z1: f32,
    pub top_tone: Tone,
    pub side_tone: Tone,
}

/// Emits procedural 2.5D primitives for `asset` anchored on floor tile `(u, v)`.
pub fn emit_asset(asset: Asset, u: f32, v: f32, proj: &IsoProjector) -> Vec<DrawPrim> {
    const SPRITE_SCALE: f32 = 1.48;
    let sprite_proj = proj.local_sprite_projector(u, v, SPRITE_SCALE);
    let mut out = Vec::with_capacity(28);
    let sp = &sprite_proj;
    match asset {
        Asset::Rack { blades, pedestal } => {
            compute_storage::emit_rack(&mut out, sp, u, v, blades, pedestal);
        }
        Asset::Drum { rings } => compute_storage::emit_drum(&mut out, sp, u, v, rings),
        Asset::Crate => compute_storage::emit_crate(&mut out, sp, u, v),
        Asset::Pod => compute_storage::emit_pod(&mut out, sp, u, v),
        Asset::Chip => compute_storage::emit_chip(&mut out, sp, u, v),
        Asset::Firewall => network_client::emit_firewall(&mut out, sp, u, v),
        Asset::Switch { ports } => network_client::emit_switch(&mut out, sp, u, v, ports),
        Asset::Router => network_client::emit_router(&mut out, sp, u, v),
        Asset::Laptop => network_client::emit_laptop(&mut out, sp, u, v),
        Asset::Cloud => network_client::emit_cloud(&mut out, sp, u, v),
        Asset::Lock => network_client::emit_lock(&mut out, sp, u, v),
        Asset::Balancer => network_client::emit_balancer(&mut out, sp, u, v),
    }
    out
}

pub(crate) fn push_prism(out: &mut Vec<DrawPrim>, proj: &IsoProjector, s: PrismSpec) {
    let c = proj.box_prism(s.u, s.v, s.hu, s.hv, s.z0, s.z1);
    let sw = (proj.tile_w * 0.028).clamp(1.0, 1.65);
    let faces = [
        (0.0f32, 1.0f32, [c[3], c[2], c[6], c[7]]),
        (1.0f32, 0.0f32, [c[2], c[1], c[5], c[6]]),
        (0.0f32, -1.0f32, [c[0], c[1], c[5], c[4]]),
        (-1.0f32, 0.0f32, [c[0], c[3], c[7], c[4]]),
    ];
    for (nu, nv, pts) in faces {
        if proj.face_visible(nu, nv) {
            let tone = if face_sun_lit(nu, nv) {
                s.left
            } else {
                s.right
            };
            out.push(DrawPrim::Polygon {
                pts: pts.to_vec(),
                fill: Some(tone),
                stroke: Some(Tone::Outline),
                width: sw,
            });
        }
    }
    out.push(DrawPrim::Polygon {
        pts: vec![c[0], c[1], c[2], c[3]],
        fill: Some(s.top),
        stroke: Some(Tone::Outline),
        width: sw,
    });
}

pub(crate) fn push_cylinder(out: &mut Vec<DrawPrim>, proj: &IsoProjector, s: CylinderSpec) {
    use std::f32::consts::{FRAC_PI_2, PI};
    let steps = 18usize;
    let sw = (proj.tile_w * 0.028).clamp(1.0, 1.60);
    let cam_a = proj.yaw.cos().atan2(proj.yaw.sin());
    let start_a = cam_a - FRAC_PI_2;
    let end_a = cam_a + FRAC_PI_2;
    let mut front_body = Vec::with_capacity(steps * 2 + 2);
    for i in 0..=steps {
        let a = start_a + (end_a - start_a) * (i as f32 / steps as f32);
        front_body.push(proj.project(s.u + s.radius * a.cos(), s.v + s.radius * a.sin(), s.z1));
    }
    for i in (0..=steps).rev() {
        let a = start_a + (end_a - start_a) * (i as f32 / steps as f32);
        front_body.push(proj.project(s.u + s.radius * a.cos(), s.v + s.radius * a.sin(), s.z0));
    }
    out.push(DrawPrim::Polygon {
        pts: front_body,
        fill: Some(s.side_tone),
        stroke: Some(Tone::Outline),
        width: sw,
    });

    let mut top_ring = Vec::with_capacity(steps * 2);
    for i in 0..(steps * 2) {
        let a = 2.0 * PI * (i as f32 / (steps * 2) as f32);
        top_ring.push(proj.project(s.u + s.radius * a.cos(), s.v + s.radius * a.sin(), s.z1));
    }
    out.push(DrawPrim::Polygon {
        pts: top_ring,
        fill: Some(s.top_tone),
        stroke: Some(Tone::Outline),
        width: sw,
    });
}

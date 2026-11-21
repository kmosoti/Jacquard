//! Compute & storage procedural 2.5D sprites: Rack, Drum, Crate, Pod, Chip.

use crate::assets::{
    CylinderSpec, DrawPrim, PrismSpec, Tone, push_box_shadow, push_cyl_shadow, push_cylinder,
    push_prism,
};
use crate::projection::IsoProjector;

pub(crate) fn emit_rack(
    out: &mut Vec<DrawPrim>,
    proj: &IsoProjector,
    u: f32,
    v: f32,
    blades: u8,
    pedestal: bool,
) {
    let count = blades.clamp(1, 5);
    let blade_h = 0.24f32;
    let gap = 0.05f32;
    let total_h =
        (if pedestal { 0.14 } else { 0.0 }) + (count as f32) * blade_h + ((count - 1) as f32) * gap;
    if pedestal {
        push_cyl_shadow(out, proj, u, v, 0.62, total_h);
    } else {
        push_box_shadow(out, proj, u, v, 0.46, 0.46, total_h);
    }
    let mut base_z = 0.0f32;
    if pedestal {
        push_cylinder(
            out,
            proj,
            CylinderSpec {
                u,
                v,
                radius: 0.62,
                z0: 0.0,
                z1: 0.14,
                top_tone: Tone::AccentTop,
                side_tone: Tone::AccentSide,
            },
        );
        base_z = 0.14;
    }

    let sw = (proj.tile_w * 0.022).clamp(0.7, 1.15);
    let front_south = proj.face_visible(0.0, 1.0);
    let front_east = proj.face_visible(1.0, 0.0);

    for b in 0..count {
        let z0 = base_z + (b as f32) * (blade_h + gap);
        let z1 = z0 + blade_h;
        let top_tone = if b + 1 == count {
            Tone::Top
        } else {
            Tone::MetalLight
        };
        push_prism(
            out,
            proj,
            PrismSpec {
                u,
                v,
                hu: 0.46,
                hv: 0.46,
                z0,
                z1,
                top: top_tone,
                left: Tone::Left,
                right: Tone::Right,
            },
        );
        if front_south {
            let slot = vec![
                proj.project(u - 0.34, v + 0.465, z1 - 0.06),
                proj.project(u + 0.22, v + 0.465, z1 - 0.06),
                proj.project(u + 0.22, v + 0.465, z0 + 0.06),
                proj.project(u - 0.34, v + 0.465, z0 + 0.06),
            ];
            out.push(DrawPrim::Polygon {
                pts: slot,
                fill: Some(Tone::Dark),
                stroke: None,
                width: 0.0,
            });
            out.push(DrawPrim::Circle {
                center: proj.project(u + 0.34, v + 0.47, (z0 + z1) * 0.5),
                radius: (proj.tile_w * 0.055).clamp(1.4, 2.6),
                fill: Some(Tone::Led),
                stroke: None,
                width: 0.0,
            });
        }
        if front_east {
            out.push(DrawPrim::Polyline {
                pts: vec![
                    proj.project(u + 0.465, v - 0.26, (z0 + z1) * 0.5),
                    proj.project(u + 0.465, v + 0.26, (z0 + z1) * 0.5),
                ],
                stroke: Tone::Dark,
                width: sw,
            });
        }
    }
}

pub(crate) fn emit_drum(out: &mut Vec<DrawPrim>, proj: &IsoProjector, u: f32, v: f32, rings: u8) {
    use std::f32::consts::FRAC_PI_2;
    let total_h = 0.96f32;
    push_cyl_shadow(out, proj, u, v, 0.44, total_h);
    let bands = rings.clamp(2, 4);
    let band_h = total_h / (bands as f32);
    let cam_a = proj.yaw.cos().atan2(proj.yaw.sin());
    for r in 0..bands {
        let z0 = (r as f32) * band_h;
        let z1 = z0 + band_h;
        let side = if r % 2 == 0 {
            Tone::Left
        } else {
            Tone::MetalLight
        };
        push_cylinder(
            out,
            proj,
            CylinderSpec {
                u,
                v,
                radius: 0.44,
                z0,
                z1,
                top_tone: Tone::Top,
                side_tone: side,
            },
        );
        let mut seam = Vec::with_capacity(12);
        for i in 0..=10 {
            let a = (cam_a - FRAC_PI_2) + std::f32::consts::PI * (i as f32 / 10.0);
            seam.push(proj.project(u + 0.445 * a.cos(), v + 0.445 * a.sin(), z0 + 0.02));
        }
        out.push(DrawPrim::Polyline {
            pts: seam,
            stroke: Tone::Dark,
            width: (proj.tile_w * 0.032).clamp(0.9, 1.6),
        });
    }
    out.push(DrawPrim::Circle {
        center: proj.project(u + 0.20, v + 0.24, total_h + 0.02),
        radius: (proj.tile_w * 0.055).clamp(1.5, 2.8),
        fill: Some(Tone::Led),
        stroke: Some(Tone::Outline),
        width: 0.8,
    });
}

pub(crate) fn emit_crate(out: &mut Vec<DrawPrim>, proj: &IsoProjector, u: f32, v: f32) {
    push_box_shadow(out, proj, u, v, 0.48, 0.44, 0.62);
    push_prism(
        out,
        proj,
        PrismSpec {
            u,
            v,
            hu: 0.48,
            hv: 0.44,
            z0: 0.0,
            z1: 0.58,
            top: Tone::AccentTop,
            left: Tone::AccentSide,
            right: Tone::Right,
        },
    );
    let well = vec![
        proj.project(u - 0.34, v - 0.30, 0.585),
        proj.project(u + 0.34, v - 0.30, 0.585),
        proj.project(u + 0.34, v + 0.30, 0.585),
        proj.project(u - 0.34, v + 0.30, 0.585),
    ];
    out.push(DrawPrim::Polygon {
        pts: well,
        fill: Some(Tone::Dark),
        stroke: Some(Tone::Outline),
        width: 0.9,
    });
    for idx in [-0.14f32, 0.06] {
        push_prism(
            out,
            proj,
            PrismSpec {
                u: u + idx,
                v: v - 0.02,
                hu: 0.07,
                hv: 0.22,
                z0: 0.44,
                z1: 0.70,
                top: Tone::Top,
                left: Tone::MetalLight,
                right: Tone::Left,
            },
        );
    }
    if proj.face_visible(0.0, 1.0) {
        let sw = (proj.tile_w * 0.024).clamp(0.8, 1.3);
        for sx in [-0.26f32, -0.08, 0.10, 0.28] {
            out.push(DrawPrim::Polyline {
                pts: vec![
                    proj.project(u + sx, v + 0.445, 0.08),
                    proj.project(u + sx, v + 0.445, 0.50),
                ],
                stroke: Tone::Outline,
                width: sw,
            });
        }
    }
}

pub(crate) fn emit_pod(out: &mut Vec<DrawPrim>, proj: &IsoProjector, u: f32, v: f32) {
    push_box_shadow(out, proj, u, v, 0.34, 0.34, 0.60);
    push_prism(
        out,
        proj,
        PrismSpec {
            u,
            v,
            hu: 0.34,
            hv: 0.34,
            z0: 0.0,
            z1: 0.54,
            top: Tone::Top,
            left: Tone::Left,
            right: Tone::Right,
        },
    );
    push_prism(
        out,
        proj,
        PrismSpec {
            u,
            v,
            hu: 0.24,
            hv: 0.24,
            z0: 0.54,
            z1: 0.60,
            top: Tone::AccentTop,
            left: Tone::MetalLight,
            right: Tone::MetalDark,
        },
    );
    if proj.face_visible(0.0, 1.0) {
        out.push(DrawPrim::Circle {
            center: proj.project(u + 0.14, v + 0.35, 0.36),
            radius: (proj.tile_w * 0.055).clamp(1.4, 2.6),
            fill: Some(Tone::Led),
            stroke: None,
            width: 0.0,
        });
    }
}

pub(crate) fn emit_chip(out: &mut Vec<DrawPrim>, proj: &IsoProjector, u: f32, v: f32) {
    push_box_shadow(out, proj, u, v, 0.50, 0.36, 0.21);
    push_prism(
        out,
        proj,
        PrismSpec {
            u,
            v,
            hu: 0.50,
            hv: 0.36,
            z0: 0.0,
            z1: 0.16,
            top: Tone::AccentTop,
            left: Tone::AccentSide,
            right: Tone::Right,
        },
    );
    push_prism(
        out,
        proj,
        PrismSpec {
            u: u - 0.14,
            v: v - 0.04,
            hu: 0.16,
            hv: 0.14,
            z0: 0.16,
            z1: 0.21,
            top: Tone::MetalLight,
            left: Tone::MetalDark,
            right: Tone::MetalDark,
        },
    );
    let bar = vec![
        proj.project(u + 0.14, v - 0.22, 0.165),
        proj.project(u + 0.38, v - 0.22, 0.165),
        proj.project(u + 0.38, v + 0.18, 0.165),
        proj.project(u + 0.14, v + 0.18, 0.165),
    ];
    out.push(DrawPrim::Polygon {
        pts: bar,
        fill: Some(Tone::Top),
        stroke: Some(Tone::Outline),
        width: 0.85,
    });
}

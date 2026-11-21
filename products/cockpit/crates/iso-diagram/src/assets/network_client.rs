//! Network, security, client & cloud procedural 2.5D sprites: Firewall, Switch, Router, Laptop, Cloud, Lock, Balancer.

use crate::assets::{
    CylinderSpec, DrawPrim, PrismSpec, Tone, push_box_shadow, push_cyl_shadow, push_cylinder,
    push_prism,
};
use crate::projection::IsoProjector;

pub(crate) fn emit_firewall(out: &mut Vec<DrawPrim>, proj: &IsoProjector, u: f32, v: f32) {
    push_box_shadow(out, proj, u, v, 0.48, 0.24, 0.92);
    push_prism(
        out,
        proj,
        PrismSpec {
            u,
            v,
            hu: 0.48,
            hv: 0.24,
            z0: 0.0,
            z1: 0.92,
            top: Tone::AccentTop,
            left: Tone::AccentSide,
            right: Tone::Right,
        },
    );
    let sw = (proj.tile_w * 0.022).clamp(0.8, 1.2);
    if proj.face_visible(0.0, 1.0) {
        for course in [0.23f32, 0.46, 0.69] {
            out.push(DrawPrim::Polyline {
                pts: vec![
                    proj.project(u - 0.48, v + 0.245, course),
                    proj.project(u + 0.48, v + 0.245, course),
                ],
                stroke: Tone::Outline,
                width: sw,
            });
        }
        for (row, z0) in [0.0f32, 0.23, 0.46, 0.69].into_iter().enumerate() {
            let xs: &[f32] = if row % 2 == 0 {
                &[-0.18, 0.18]
            } else {
                &[-0.32, 0.0, 0.32]
            };
            for &xu in xs {
                out.push(DrawPrim::Polyline {
                    pts: vec![
                        proj.project(u + xu, v + 0.245, z0),
                        proj.project(u + xu, v + 0.245, z0 + 0.23),
                    ],
                    stroke: Tone::Outline,
                    width: sw,
                });
            }
        }
        push_prism(
            out,
            proj,
            PrismSpec {
                u,
                v: v + 0.26,
                hu: 0.14,
                hv: 0.05,
                z0: 0.30,
                z1: 0.62,
                top: Tone::MetalLight,
                left: Tone::MetalLight,
                right: Tone::MetalDark,
            },
        );
    }
}

pub(crate) fn emit_switch(out: &mut Vec<DrawPrim>, proj: &IsoProjector, u: f32, v: f32, ports: u8) {
    push_box_shadow(out, proj, u, v, 0.56, 0.34, 0.30);
    push_prism(
        out,
        proj,
        PrismSpec {
            u,
            v,
            hu: 0.56,
            hv: 0.34,
            z0: 0.0,
            z1: 0.30,
            top: Tone::Top,
            left: Tone::MetalLight,
            right: Tone::MetalDark,
        },
    );
    if proj.face_visible(0.0, 1.0) {
        let shown = (ports.clamp(4, 12) as usize).min(6);
        for i in 0..shown {
            let frac = (i as f32 + 0.5) / (shown as f32);
            let pu = u - 0.44 + frac * 0.88;
            let port = vec![
                proj.project(pu - 0.04, v + 0.345, 0.08),
                proj.project(pu + 0.04, v + 0.345, 0.08),
                proj.project(pu + 0.04, v + 0.345, 0.20),
                proj.project(pu - 0.04, v + 0.345, 0.20),
            ];
            out.push(DrawPrim::Polygon {
                pts: port,
                fill: Some(Tone::Dark),
                stroke: None,
                width: 0.0,
            });
            out.push(DrawPrim::Circle {
                center: proj.project(pu, v + 0.348, 0.25),
                radius: (proj.tile_w * 0.036).clamp(1.0, 2.0),
                fill: Some(Tone::Led),
                stroke: None,
                width: 0.0,
            });
        }
    }
}

pub(crate) fn emit_router(out: &mut Vec<DrawPrim>, proj: &IsoProjector, u: f32, v: f32) {
    push_box_shadow(out, proj, u, v, 0.46, 0.32, 0.36);
    push_prism(
        out,
        proj,
        PrismSpec {
            u,
            v,
            hu: 0.46,
            hv: 0.32,
            z0: 0.0,
            z1: 0.26,
            top: Tone::Top,
            left: Tone::Left,
            right: Tone::Right,
        },
    );
    let sw = (proj.tile_w * 0.05).clamp(1.6, 3.0);
    for du in [-0.28f32, 0.28] {
        out.push(DrawPrim::Polyline {
            pts: vec![
                proj.project(u + du, v - 0.22, 0.25),
                proj.project(u + du, v - 0.22, 0.72),
            ],
            stroke: Tone::Dark,
            width: sw,
        });
        out.push(DrawPrim::Circle {
            center: proj.project(u + du, v - 0.22, 0.73),
            radius: sw * 0.6,
            fill: Some(Tone::AccentTop),
            stroke: None,
            width: 0.0,
        });
    }
    if proj.face_visible(0.0, 1.0) {
        for du in [-0.22f32, 0.0, 0.22] {
            out.push(DrawPrim::Circle {
                center: proj.project(u + du, v + 0.325, 0.14),
                radius: (proj.tile_w * 0.04).clamp(1.2, 2.2),
                fill: Some(Tone::Led),
                stroke: None,
                width: 0.0,
            });
        }
    }
}

pub(crate) fn emit_laptop(out: &mut Vec<DrawPrim>, proj: &IsoProjector, u: f32, v: f32) {
    push_box_shadow(out, proj, u, v + 0.04, 0.46, 0.38, 0.56);
    push_prism(
        out,
        proj,
        PrismSpec {
            u,
            v: v + 0.06,
            hu: 0.46,
            hv: 0.34,
            z0: 0.0,
            z1: 0.08,
            top: Tone::MetalLight,
            left: Tone::Left,
            right: Tone::Right,
        },
    );
    out.push(DrawPrim::Polygon {
        pts: vec![
            proj.project(u - 0.36, v - 0.16, 0.085),
            proj.project(u + 0.36, v - 0.16, 0.085),
            proj.project(u + 0.36, v + 0.12, 0.085),
            proj.project(u - 0.36, v + 0.12, 0.085),
        ],
        fill: Some(Tone::Dark),
        stroke: None,
        width: 0.0,
    });
    let lid = vec![
        proj.project(u - 0.46, v - 0.28, 0.08),
        proj.project(u + 0.46, v - 0.28, 0.08),
        proj.project(u + 0.46, v - 0.34, 0.68),
        proj.project(u - 0.46, v - 0.34, 0.68),
    ];
    let sw = (proj.tile_w * 0.025).clamp(0.85, 1.35);
    out.push(DrawPrim::Polygon {
        pts: lid,
        fill: Some(Tone::MetalDark),
        stroke: Some(Tone::Outline),
        width: sw,
    });
    if proj.face_visible(0.0, 1.0) {
        let screen = vec![
            proj.project(u - 0.39, v - 0.29, 0.14),
            proj.project(u + 0.39, v - 0.29, 0.14),
            proj.project(u + 0.39, v - 0.33, 0.62),
            proj.project(u - 0.39, v - 0.33, 0.62),
        ];
        out.push(DrawPrim::Polygon {
            pts: screen,
            fill: Some(Tone::AccentTop),
            stroke: Some(Tone::Outline),
            width: sw * 0.8,
        });
    }
}

pub(crate) fn emit_cloud(out: &mut Vec<DrawPrim>, proj: &IsoProjector, u: f32, v: f32) {
    push_cyl_shadow(out, proj, u, v, 0.48, 0.56);
    push_cylinder(
        out,
        proj,
        CylinderSpec {
            u: u - 0.24,
            v: v + 0.08,
            radius: 0.26,
            z0: 0.0,
            z1: 0.34,
            top_tone: Tone::Top,
            side_tone: Tone::Left,
        },
    );
    push_cylinder(
        out,
        proj,
        CylinderSpec {
            u: u + 0.22,
            v: v - 0.08,
            radius: 0.24,
            z0: 0.0,
            z1: 0.34,
            top_tone: Tone::Top,
            side_tone: Tone::Right,
        },
    );
    push_cylinder(
        out,
        proj,
        CylinderSpec {
            u,
            v,
            radius: 0.34,
            z0: 0.0,
            z1: 0.56,
            top_tone: Tone::Top,
            side_tone: Tone::Left,
        },
    );
}

pub(crate) fn emit_lock(out: &mut Vec<DrawPrim>, proj: &IsoProjector, u: f32, v: f32) {
    push_box_shadow(out, proj, u, v, 0.34, 0.24, 0.84);
    let sw = (proj.tile_w * 0.09).clamp(2.2, 4.4);
    out.push(DrawPrim::Polyline {
        pts: vec![
            proj.project(u - 0.18, v, 0.56),
            proj.project(u - 0.18, v, 0.86),
            proj.project(u, v, 0.96),
            proj.project(u + 0.18, v, 0.86),
            proj.project(u + 0.18, v, 0.56),
        ],
        stroke: Tone::MetalDark,
        width: sw,
    });
    push_prism(
        out,
        proj,
        PrismSpec {
            u,
            v,
            hu: 0.34,
            hv: 0.24,
            z0: 0.0,
            z1: 0.56,
            top: Tone::AccentTop,
            left: Tone::AccentSide,
            right: Tone::Right,
        },
    );
    if proj.face_visible(0.0, 1.0) {
        out.push(DrawPrim::Circle {
            center: proj.project(u, v + 0.25, 0.32),
            radius: (proj.tile_w * 0.075).clamp(2.0, 3.6),
            fill: Some(Tone::Dark),
            stroke: None,
            width: 0.0,
        });
    }
}

pub(crate) fn emit_balancer(out: &mut Vec<DrawPrim>, proj: &IsoProjector, u: f32, v: f32) {
    push_box_shadow(out, proj, u, v, 0.52, 0.34, 0.64);
    push_prism(
        out,
        proj,
        PrismSpec {
            u,
            v,
            hu: 0.22,
            hv: 0.22,
            z0: 0.0,
            z1: 0.42,
            top: Tone::MetalLight,
            left: Tone::MetalDark,
            right: Tone::Right,
        },
    );
    push_prism(
        out,
        proj,
        PrismSpec {
            u,
            v,
            hu: 0.52,
            hv: 0.34,
            z0: 0.42,
            z1: 0.64,
            top: Tone::AccentTop,
            left: Tone::Left,
            right: Tone::Right,
        },
    );
    for du in [-0.28f32, 0.0, 0.28] {
        out.push(DrawPrim::Circle {
            center: proj.project(u + du, v, 0.65),
            radius: (proj.tile_w * 0.048).clamp(1.4, 2.5),
            fill: Some(Tone::Led),
            stroke: Some(Tone::Outline),
            width: 0.8,
        });
    }
}

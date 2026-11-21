//! Physically grounded 3-layer directional cast shadows (penumbra, umbra, contact AO).

use crate::assets::{DrawPrim, Tone};
use crate::camera::{SUN_CAST_U, SUN_CAST_V, convex_hull_pt2};
use crate::projection::{IsoProjector, Pt2};

fn push_ground_poly(out: &mut Vec<DrawPrim>, pts: Vec<Pt2>, tone: Tone) {
    if pts.len() >= 3 {
        out.push(DrawPrim::Polygon {
            pts,
            fill: Some(tone),
            stroke: None,
            width: 0.0,
        });
    }
}

fn box_cast_hull(
    proj: &IsoProjector,
    u: f32,
    v: f32,
    half_uv: (f32, f32),
    sun_uv: (f32, f32),
    pad: f32,
) -> Vec<Pt2> {
    let mut pts = Vec::with_capacity(8);
    let bhu = half_uv.0 + pad;
    let bhv = half_uv.1 + pad;
    let (su, sv) = sun_uv;
    for (cu, cv) in [
        (u - bhu, v - bhv),
        (u + bhu, v - bhv),
        (u + bhu, v + bhv),
        (u - bhu, v + bhv),
        (u - bhu + su, v - bhv + sv),
        (u + bhu + su, v - bhv + sv),
        (u + bhu + su, v + bhv + sv),
        (u - bhu + su, v + bhv + sv),
    ] {
        pts.push(proj.project(cu, cv, 0.0));
    }
    convex_hull_pt2(&pts)
}

pub(crate) fn push_box_shadow(
    out: &mut Vec<DrawPrim>,
    proj: &IsoProjector,
    u: f32,
    v: f32,
    hu: f32,
    hv: f32,
    height_z: f32,
) {
    let su = SUN_CAST_U * height_z.max(0.12);
    let sv = SUN_CAST_V * height_z.max(0.12);

    let penumbra = box_cast_hull(
        proj,
        u,
        v,
        (hu, hv),
        (su * 1.14 + 0.03, sv * 1.14 - 0.01),
        0.06,
    );
    push_ground_poly(out, penumbra, Tone::ShadowSoft);

    let umbra = box_cast_hull(proj, u, v, (hu, hv), (su, sv), 0.0);
    push_ground_poly(out, umbra, Tone::Shadow);

    let chu = hu + 0.025;
    let chv = hv + 0.025;
    let contact = vec![
        proj.project(u - chu, v - chv, 0.0),
        proj.project(u + chu, v - chv, 0.0),
        proj.project(u + chu, v + chv, 0.0),
        proj.project(u - chu, v + chv, 0.0),
    ];
    push_ground_poly(out, contact, Tone::ShadowCore);
}

fn cyl_cast_hull(proj: &IsoProjector, u: f32, v: f32, r: f32, su: f32, sv: f32) -> Vec<Pt2> {
    use std::f32::consts::TAU;
    const STEPS: usize = 12;
    let mut pts = Vec::with_capacity(STEPS * 2);
    for i in 0..STEPS {
        let a = TAU * (i as f32 / STEPS as f32);
        let (sa, ca) = a.sin_cos();
        pts.push(proj.project(u + r * ca, v + r * sa, 0.0));
        pts.push(proj.project(u + su + r * ca, v + sv + r * sa, 0.0));
    }
    convex_hull_pt2(&pts)
}

pub(crate) fn push_cyl_shadow(
    out: &mut Vec<DrawPrim>,
    proj: &IsoProjector,
    u: f32,
    v: f32,
    radius: f32,
    height_z: f32,
) {
    use std::f32::consts::TAU;
    let su = SUN_CAST_U * height_z.max(0.12);
    let sv = SUN_CAST_V * height_z.max(0.12);

    let penumbra = cyl_cast_hull(
        proj,
        u,
        v,
        radius + 0.055,
        su * 1.14 + 0.03,
        sv * 1.14 - 0.01,
    );
    push_ground_poly(out, penumbra, Tone::ShadowSoft);

    let umbra = cyl_cast_hull(proj, u, v, radius, su, sv);
    push_ground_poly(out, umbra, Tone::Shadow);

    let mut contact = Vec::with_capacity(16);
    let cr = radius + 0.02;
    for i in 0..16 {
        let a = TAU * (i as f32 / 16.0);
        contact.push(proj.project(u + cr * a.cos(), v + cr * a.sin(), 0.0));
    }
    push_ground_poly(out, contact, Tone::ShadowCore);
}

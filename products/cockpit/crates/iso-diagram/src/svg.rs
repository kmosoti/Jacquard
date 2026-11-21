//! Zero-dependency standalone SVG renderer producing Warm Paper or Obsidian Slate output.

use std::fmt::Write;

use crate::assets::{DrawPrim, emit_asset};
use crate::camera::sun_cast_uv;
use crate::layout::{CompiledLayout, GridBounds, compile_layout};
use crate::model::{Diagram, EdgeStyle, ThemePreset, Tint, ZoneStyle};
use crate::projection::{
    Aabb3, IsoProjector, Pt2, Rect2, eval_polyline_pt2, topological_painter_order,
};
use crate::svg_palette::{
    format_pts, palette, tint_colors, tone_color, write_callouts, xml_escape,
};

/// Renders `diagram` into a standalone self-contained SVG string.
pub fn render_svg(diagram: &Diagram, width: f32, height: f32) -> String {
    let layout = compile_layout(diagram);
    render_layout_svg(&layout, diagram.theme, width, height)
}

/// Renders an already-compiled isometric layout as an SVG document.
pub fn render_layout_svg(
    layout: &CompiledLayout,
    preset: ThemePreset,
    width: f32,
    height: f32,
) -> String {
    let p = palette(preset);
    let proj = IsoProjector::fit(layout, width, height, 30.0);
    let mut svg = String::with_capacity(16_384);
    let _ = writeln!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width:.0} {height:.0}\" width=\"{width:.0}\" height=\"{height:.0}\" font-family=\"Inter, ui-sans-serif, system-ui, sans-serif\">"
    );
    let _ = writeln!(
        svg,
        "<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>",
        p.bg
    );

    write_floor_grid(&mut svg, &layout.grid_extent, &proj, p.grid);
    let zone_obstacles = write_zones(&mut svg, layout, preset, &proj, p.ink_primary);
    write_edges(&mut svg, layout, preset, &proj, p.card_bg, p.ink_primary);
    write_nodes(&mut svg, layout, preset, &proj);
    write_callouts(&mut svg, layout, &zone_obstacles, &proj, width, height, p);

    let _ = writeln!(svg, "</svg>");
    svg
}

fn write_floor_grid(svg: &mut String, ext: &GridBounds, proj: &IsoProjector, color: &str) {
    let u0 = (ext.u_min - 24.0).floor() as i32;
    let u1 = (ext.u_max + 24.0).ceil() as i32;
    let v0 = (ext.v_min - 24.0).floor() as i32;
    let v1 = (ext.v_max + 24.0).ceil() as i32;
    let _ = writeln!(
        svg,
        "<g stroke=\"{color}\" stroke-width=\"0.9\" opacity=\"0.75\">"
    );
    for u in u0..=u1 {
        let a = proj.project(u as f32, v0 as f32, 0.0);
        let b = proj.project(u as f32, v1 as f32, 0.0);
        let _ = writeln!(
            svg,
            "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\"/>",
            a.x, a.y, b.x, b.y
        );
    }
    for v in v0..=v1 {
        let a = proj.project(u0 as f32, v as f32, 0.0);
        let b = proj.project(u1 as f32, v as f32, 0.0);
        let _ = writeln!(
            svg,
            "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\"/>",
            a.x, a.y, b.x, b.y
        );
    }
    let _ = writeln!(svg, "</g>");
}

fn write_zones(
    svg: &mut String,
    layout: &CompiledLayout,
    preset: ThemePreset,
    proj: &IsoProjector,
    ink: &str,
) -> Vec<Rect2> {
    let mut obstacles = Vec::with_capacity(layout.zones.len());
    for z in &layout.zones {
        let b = z.bounds;
        let pts = [
            proj.project(b.u_min, b.v_min, 0.0),
            proj.project(b.u_max, b.v_min, 0.0),
            proj.project(b.u_max, b.v_max, 0.0),
            proj.project(b.u_min, b.v_max, 0.0),
        ];
        let (_, _, stroke_c) = tint_colors(preset, z.spec.tint);
        let (fill_op, dash) = match z.spec.style {
            ZoneStyle::Solid => ("0.15", ""),
            ZoneStyle::Dashed => ("0.08", " stroke-dasharray=\"7 5\""),
            ZoneStyle::Wireframe => ("0.0", " stroke-dasharray=\"3 4\""),
        };
        let pts_s = format_pts(&pts);
        let _ = writeln!(
            svg,
            "<polygon points=\"{pts_s}\" fill=\"{stroke_c}\" fill-opacity=\"{fill_op}\" stroke=\"{stroke_c}\" stroke-width=\"1.8\"{dash}/>"
        );
        let anchor = proj.project((b.u_min + b.u_max) * 0.5, b.v_max + 0.36, 0.0);
        let w = (z.spec.title.len() as f32) * 7.8 + 14.0;
        let _ = writeln!(
            svg,
            "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"12\" font-weight=\"700\" fill=\"{ink}\">{}</text>",
            anchor.x,
            anchor.y + 4.0,
            xml_escape(&z.spec.title)
        );
        obstacles.push(Rect2::new(anchor.x - w * 0.5, anchor.y - 9.0, w, 18.0));
    }
    obstacles
}

fn write_edges(
    svg: &mut String,
    layout: &CompiledLayout,
    preset: ThemePreset,
    proj: &IsoProjector,
    badge_bg: &str,
    ink: &str,
) {
    for e in &layout.edges {
        if e.points.len() < 2 {
            continue;
        }
        let (_, _, col) = tint_colors(preset, e.spec.tint);
        let dash = match e.spec.style {
            EdgeStyle::Solid => "",
            EdgeStyle::Dashed => " stroke-dasharray=\"10 6\"",
            EdgeStyle::Dotted => " stroke-dasharray=\"2.4 5.2\"",
        };
        let (su, sv) = sun_cast_uv();
        let shad_pts: Vec<Pt2> = e
            .points
            .iter()
            .map(|&(u, v)| proj.project(u + 0.060 * su, v + 0.060 * sv, 0.0))
            .collect();
        let base_pts: Vec<Pt2> = e
            .points
            .iter()
            .map(|&(u, v)| proj.project(u, v, 0.024))
            .collect();
        let top_pts: Vec<Pt2> = e
            .points
            .iter()
            .map(|&(u, v)| proj.project(u, v, 0.060))
            .collect();
        let crest_pts: Vec<Pt2> = e
            .points
            .iter()
            .map(|&(u, v)| proj.project(u, v, 0.082))
            .collect();
        for (pts, stroke, op, w) in [
            (&shad_pts, ink, "0.14", "6.4"),
            (&base_pts, ink, "0.46", "5.6"),
            (&top_pts, col, "1.0", "3.6"),
            (&crest_pts, badge_bg, "0.58", "1.05"),
        ] {
            let _ = writeln!(
                svg,
                "<polyline points=\"{}\" fill=\"none\" stroke=\"{stroke}\" stroke-opacity=\"{op}\" stroke-width=\"{w}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"{dash}/>",
                format_pts(pts)
            );
        }
        for i in 1..top_pts.len().saturating_sub(1) {
            let _ = writeln!(
                svg,
                "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"4.0\" fill=\"{ink}\" fill-opacity=\"0.46\"/><circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"2.8\" fill=\"{col}\"/><circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"1.0\" fill=\"{badge_bg}\" fill-opacity=\"0.78\"/>",
                base_pts[i].x,
                base_pts[i].y,
                top_pts[i].x,
                top_pts[i].y,
                crest_pts[i].x,
                crest_pts[i].y
            );
        }
        let (mid, dir) = eval_polyline_pt2(&e.points, e.chevron_t, proj);
        let (mu, mv) = proj.unproject_floor(mid.x, mid.y);
        let shad_c = proj.project(mu + 0.075 * su, mv + 0.075 * sv, 0.0);
        let lifted = Pt2::new(mid.x, mid.y - 0.060 * proj.scale_z);
        let perp = Pt2::new(-dir.y, dir.x);
        let (tip, back) = (lifted + dir.scale(6.8), lifted - dir.scale(6.2));
        let (w1, w2, inner) = (
            back + perp.scale(6.4),
            back - perp.scale(6.4),
            back + dir.scale(2.8),
        );
        let chev = [tip, w1, inner, w2];
        let (sdx, sdy) = (shad_c.x - mid.x, shad_c.y - mid.y);
        let shad: [Pt2; 4] = chev.map(|p| Pt2::new(p.x + sdx, p.y + (0.060 * proj.scale_z) + sdy));
        let wall: [Pt2; 4] = chev.map(|p| Pt2::new(p.x, p.y + 1.8));
        let _ = writeln!(
            svg,
            "<polygon points=\"{}\" fill=\"{ink}\" fill-opacity=\"0.14\"/>",
            format_pts(&shad)
        );
        let _ = writeln!(
            svg,
            "<polygon points=\"{}\" fill=\"{ink}\" fill-opacity=\"0.48\" stroke=\"{badge_bg}\" stroke-width=\"2.4\" stroke-linejoin=\"round\"/>",
            format_pts(&wall)
        );
        let _ = writeln!(
            svg,
            "<polygon points=\"{}\" fill=\"{col}\" stroke=\"{ink}\" stroke-opacity=\"0.48\" stroke-width=\"1.0\" stroke-linejoin=\"round\"/>",
            format_pts(&chev)
        );
        if let Some(label) = &e.spec.label {
            let label_t = if e.chevron_t > 0.50 {
                (e.chevron_t - 0.16).clamp(0.18, 0.82)
            } else {
                (e.chevron_t + 0.16).clamp(0.18, 0.82)
            };
            let (lpt, _) = eval_polyline_pt2(&e.points, label_t, proj);
            let ly = lpt.y - 0.060 * proj.scale_z;
            let bw = (label.len() as f32) * 6.0 + 10.0;
            let _ = writeln!(
                svg,
                "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{bw:.1}\" height=\"16\" rx=\"4\" fill=\"{badge_bg}\" stroke=\"{col}\" stroke-width=\"1\"/>",
                lpt.x - bw * 0.5,
                ly - 8.0
            );
            let _ = writeln!(
                svg,
                "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"9.5\" font-weight=\"600\" fill=\"{col}\">{}</text>",
                lpt.x,
                ly + 3.5,
                xml_escape(label)
            );
        }
    }
}

fn write_nodes(
    svg: &mut String,
    layout: &CompiledLayout,
    preset: ThemePreset,
    proj: &IsoProjector,
) {
    let boxes: Vec<Aabb3> = layout
        .nodes
        .iter()
        .map(|n| Aabb3::from_footprint(n.u, n.v, n.half_u, n.half_v, 0.0, n.height_z))
        .collect();
    for idx in topological_painter_order(&boxes, proj) {
        let node = &layout.nodes[idx];
        let tint = node
            .spec
            .tint
            .or_else(|| {
                node.spec.zone.map(|z| {
                    layout
                        .zones
                        .iter()
                        .find(|pz| pz.id == z)
                        .map_or(Tint::Cobalt, |pz| pz.spec.tint)
                })
            })
            .unwrap_or(Tint::Cobalt);
        for prim in emit_asset(node.spec.asset, node.u, node.v, proj) {
            match prim {
                DrawPrim::Polygon {
                    pts,
                    fill,
                    stroke,
                    width,
                } => {
                    let f = fill.map_or("none", |t| tone_color(preset, t, tint, node.spec.status));
                    let s =
                        stroke.map_or("none", |t| tone_color(preset, t, tint, node.spec.status));
                    let _ = writeln!(
                        svg,
                        "<polygon points=\"{}\" fill=\"{f}\" stroke=\"{s}\" stroke-width=\"{width:.2}\" stroke-linejoin=\"round\"/>",
                        format_pts(&pts)
                    );
                }
                DrawPrim::Polyline { pts, stroke, width } => {
                    let s = tone_color(preset, stroke, tint, node.spec.status);
                    let _ = writeln!(
                        svg,
                        "<polyline points=\"{}\" fill=\"none\" stroke=\"{s}\" stroke-width=\"{width:.2}\" stroke-linecap=\"round\"/>",
                        format_pts(&pts)
                    );
                }
                DrawPrim::Circle {
                    center,
                    radius,
                    fill,
                    stroke,
                    width,
                } => {
                    let f = fill.map_or("none", |t| tone_color(preset, t, tint, node.spec.status));
                    let s =
                        stroke.map_or("none", |t| tone_color(preset, t, tint, node.spec.status));
                    let _ = writeln!(
                        svg,
                        "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{radius:.2}\" fill=\"{f}\" stroke=\"{s}\" stroke-width=\"{width:.2}\"/>",
                        center.x, center.y
                    );
                }
            }
        }
    }
}

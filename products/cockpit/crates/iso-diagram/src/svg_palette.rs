//! Palette mapping for standalone Warm Paper / Obsidian Slate SVG rendering.

use crate::assets::Tone;
use crate::model::{NodeStatus, ThemePreset, Tint};

pub(crate) struct SvgPalette {
    pub bg: &'static str,
    pub grid: &'static str,
    pub card_bg: &'static str,
    pub ink_primary: &'static str,
    pub outline: &'static str,
    pub top: &'static str,
    pub left: &'static str,
    pub right: &'static str,
    pub metal_light: &'static str,
    pub metal_dark: &'static str,
    pub dark: &'static str,
    pub shadow_soft: &'static str,
    pub shadow: &'static str,
    pub shadow_core: &'static str,
}

pub(crate) fn palette(preset: ThemePreset) -> SvgPalette {
    match preset {
        ThemePreset::WarmPaper => SvgPalette {
            bg: "#F5EFE6",
            grid: "#DED5C5",
            card_bg: "#FEFBF6",
            ink_primary: "#1F2421",
            outline: "#2B302C",
            top: "#FCF9F2",
            left: "#E7DFD1",
            right: "#CFC4B2",
            metal_light: "#D7DCE2",
            metal_dark: "#8994A2",
            dark: "#262C33",
            shadow_soft: "rgba(31, 29, 26, 0.06)",
            shadow: "rgba(31, 29, 26, 0.13)",
            shadow_core: "rgba(31, 29, 26, 0.22)",
        },
        ThemePreset::ObsidianSlate => SvgPalette {
            bg: "#14181F",
            grid: "#242C38",
            card_bg: "#1C222C",
            ink_primary: "#EBF0F6",
            outline: "#0C0F14",
            top: "#333D4D",
            left: "#262E3B",
            right: "#1B212B",
            metal_light: "#627187",
            metal_dark: "#3A4454",
            dark: "#0E1218",
            shadow_soft: "rgba(0, 0, 0, 0.14)",
            shadow: "rgba(0, 0, 0, 0.28)",
            shadow_core: "rgba(0, 0, 0, 0.42)",
        },
    }
}

pub(crate) fn tint_colors(
    preset: ThemePreset,
    tint: Tint,
) -> (&'static str, &'static str, &'static str) {
    match (preset, tint) {
        (ThemePreset::WarmPaper, Tint::Pine) => ("#498C5E", "#2D6A42", "#2F6C45"),
        (ThemePreset::WarmPaper, Tint::Amber) => ("#E6A23C", "#BF7B1C", "#B86F14"),
        (ThemePreset::WarmPaper, Tint::Crimson) => ("#D45B4D", "#A83B2E", "#B23C2F"),
        (ThemePreset::WarmPaper, Tint::Cobalt) => ("#4678B8", "#2B5690", "#2A5B9A"),
        (ThemePreset::WarmPaper, Tint::Plum) => ("#8962A8", "#644282", "#6D498C"),
        (ThemePreset::WarmPaper, Tint::Sage) => ("#6A8E7F", "#47695B", "#496D5E"),
        (ThemePreset::WarmPaper, Tint::Slate) => ("#8E9AAF", "#626E84", "#556275"),
        (ThemePreset::ObsidianSlate, Tint::Pine) => ("#4EA86D", "#2E7447", "#5AC47D"),
        (ThemePreset::ObsidianSlate, Tint::Amber) => ("#EBAA4B", "#B37922", "#F2B65C"),
        (ThemePreset::ObsidianSlate, Tint::Crimson) => ("#E06C5E", "#A63F34", "#EA796C"),
        (ThemePreset::ObsidianSlate, Tint::Cobalt) => ("#5E95DB", "#3665A4", "#70A6EA"),
        (ThemePreset::ObsidianSlate, Tint::Plum) => ("#9F79C2", "#6E4B91", "#B18AD4"),
        (ThemePreset::ObsidianSlate, Tint::Sage) => ("#78A392", "#4F7566", "#88B4A3"),
        (ThemePreset::ObsidianSlate, Tint::Slate) => ("#96A5BC", "#637289", "#AAB8CE"),
    }
}

pub(crate) fn tone_color(
    preset: ThemePreset,
    tone: Tone,
    tint: Tint,
    status: NodeStatus,
) -> &'static str {
    let p = palette(preset);
    let (acc_top, acc_side, _) = tint_colors(preset, tint);
    match tone {
        Tone::ShadowSoft => p.shadow_soft,
        Tone::Shadow => p.shadow,
        Tone::ShadowCore => p.shadow_core,
        Tone::Top => p.top,
        Tone::Left => p.left,
        Tone::Right => p.right,
        Tone::AccentTop => acc_top,
        Tone::AccentSide => acc_side,
        Tone::MetalLight => p.metal_light,
        Tone::MetalDark => p.metal_dark,
        Tone::Dark => p.dark,
        Tone::Outline => p.outline,
        Tone::Led => match status {
            NodeStatus::Healthy => "#3AA76D",
            NodeStatus::Active => "#4C8CE0",
            NodeStatus::Degraded | NodeStatus::Pending => "#E59D34",
            NodeStatus::Failed => "#D94F45",
        },
    }
}

pub(crate) fn write_callouts(
    svg: &mut String,
    layout: &crate::layout::CompiledLayout,
    obstacles: &[crate::projection::Rect2],
    proj: &crate::projection::IsoProjector,
    width: f32,
    height: f32,
    p: SvgPalette,
) {
    use crate::projection::{Rect2, resolve_label_rects_with_obstacles};
    use std::fmt::Write as _;
    let mut all_obs = obstacles.to_vec();
    for n in &layout.nodes {
        let top = proj.project(n.u, n.v, n.height_z);
        let bot = proj.project(n.u + n.half_u, n.v + n.half_v, 0.0);
        let sw = (n.half_u + n.half_v) * proj.tile_w * 0.85;
        all_obs.push(Rect2::new(
            top.x - sw * 0.5,
            top.y,
            sw,
            (bot.y - top.y).max(16.0),
        ));
    }
    let desired: Vec<_> = layout
        .nodes
        .iter()
        .map(|node| {
            let base = proj.project(
                node.u + node.half_u * 0.15,
                node.v + node.half_v * 0.85,
                0.0,
            );
            let cw = ((node.spec.title.len() as f32) * 6.0).max(46.0) + 12.0;
            (Rect2::new(base.x - cw * 0.5, base.y + 4.0, cw, 16.0), base)
        })
        .collect();
    let resolved = resolve_label_rects_with_obstacles(
        &all_obs,
        &desired,
        Rect2::new(10.0, 10.0, width - 20.0, height - 20.0),
    );
    for (node, card) in layout.nodes.iter().zip(resolved.iter()) {
        let _ = writeln!(
            svg,
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"3\" fill=\"{}\" fill-opacity=\"0.86\"/>",
            card.x, card.y, card.w, card.h, p.card_bg
        );
        let _ = writeln!(
            svg,
            "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"9.5\" font-weight=\"600\" fill=\"{}\">{}</text>",
            card.center().x,
            card.y + 11.5,
            p.ink_primary,
            xml_escape(&node.spec.title)
        );
    }
}

pub(crate) fn format_pts(pts: &[crate::projection::Pt2]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(pts.len() * 12);
    for (i, p) in pts.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        let _ = write!(out, "{:.1},{:.1}", p.x, p.y);
    }
    out
}

pub(crate) fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

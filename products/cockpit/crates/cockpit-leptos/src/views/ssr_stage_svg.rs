//! SSR-only static SVG and `<details class="assets-drawer">` components passed
//! as zero-WASM `children` into `StageOrbitIsland` and `InspectorStreamIsland`.

use std::collections::HashMap;
use std::fmt::Write as _;

use iso_diagram::geometry::SceneGeometry;
use leptos::prelude::*;

use crate::state::TargetDto;
use crate::views::helpers::{StaticStatusBadge, format_cents, status_data_state, status_tone};
use crate::views::iso_client_math::{
    ClientProjector, ClientScene, fmt_pts, fmt_t1, project_box_bounds, tint_css_var,
    topological_box_order, zoom_viewbox,
};
use crate::views::stage_dom::node_dom_id;

/// Renders the initial SSR `<svg>` passed as zero-WASM `children` to `StageOrbitIsland`.
#[component]
pub fn SsrStageSvg(
    /// Compact client scene geometry.
    scene: ClientScene,
    /// Full SSR geometry with titles, styles, and tints.
    geom: SceneGeometry,
    /// Active cluster targets for status labels.
    targets: Vec<TargetDto>,
) -> impl IntoView {
    let proj = ClientProjector::fit_orbit(&scene, 45.0, 35.264, false);
    let status_map: HashMap<String, String> = targets
        .iter()
        .map(|t| (t.urn.clone(), t.status.clone()))
        .collect();
    let healthy = targets
        .iter()
        .filter(|t| t.status != "Drifted" && t.status != "Poisoned")
        .count();
    let order = topological_box_order(&scene.boxes, proj);
    let total = geom.boxes.len();
    let encoded_scene = scene.encode();

    view! {
        <svg
            id="iso-stage-svg"
            class="stage-svg"
            role="group"
            aria-roledescription="interactive isometric topology diagram"
            viewBox=zoom_viewbox(1.0)
            data-scene=encoded_scene
            aria-labelledby="iso-stage-title iso-stage-desc"
        >
            <title id="iso-stage-title">{format!("{} (2.5D Isometric)", geom.title)}</title>
            <desc id="iso-stage-desc">
                {format!("Interactive isometric infrastructure stage showing {total} resources ({healthy} healthy).")}
            </desc>

            <g id="iso-zones-layer" class="iso-zones">
                {geom.zones.into_iter().map(|z| {
                    let stroke = tint_css_var(&z.tint);
                    let (fill_op, dash) = match z.style.as_str() {
                        "dashed" => ("0.07", "7 5"),
                        "wireframe" => ("0.02", "3 4"),
                        _ => ("0.12", ""),
                    };
                    let pts = fmt_pts(&[
                        proj.project(z.u_min, z.v_min, 0.0),
                        proj.project(z.u_max, z.v_min, 0.0),
                        proj.project(z.u_max, z.v_max, 0.0),
                        proj.project(z.u_min, z.v_max, 0.0),
                    ]);
                    let lbl = proj.project((z.u_min + z.u_max) * 0.5, z.v_max + 0.32, 0.0);
                    view! {
                        <g>
                            <polygon id=format!("iso-zone-p-{}", z.id) points=pts fill=stroke fill-opacity=fill_op stroke=stroke stroke-width="1.6" stroke-dasharray=dash />
                            <text id=format!("iso-zone-t-{}", z.id) class="iso-zone-label" x=fmt_t1(lbl.0) y=fmt_t1(lbl.1 + 4.0) text-anchor="middle">{z.title}</text>
                        </g>
                    }
                }).collect_view()}
            </g>

            <g id="iso-edges-layer" class="iso-edges" aria-hidden="true">
                {geom.routes.into_iter().map(|r| {
                    let stroke = tint_css_var(&r.tint);
                    let dash = match r.style.as_str() {
                        "dashed" => "10 6",
                        "dotted" => "2.4 5.2",
                        _ => "",
                    };
                    let sc: Vec<(f32, f32)> = r.points.iter().map(|&(u, v)| proj.project(u, v, 0.05)).collect();
                    view! {
                        <polyline id=format!("iso-edge-{}", r.id) points=fmt_pts(&sc) fill="none" stroke=stroke stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round" stroke-dasharray=dash />
                    }
                }).collect_view()}
            </g>

            <g id="iso-nodes-layer" class="iso-nodes">
                {order.into_iter().enumerate().map(|(pos, idx)| {
                    let b = &geom.boxes[idx];
                    let (_, tlen, bounds) = &scene.boxes[idx];
                    let pb = project_box_bounds(*tlen, *bounds, proj);
                    let status = status_map.get(&b.slug).cloned().unwrap_or_else(|| "Planned".to_owned());
                    let state = status_data_state(&status);
                    let dom_id = node_dom_id(&b.slug);
                    let (cx, cy, cw) = pb.callout;
                    let aria_lbl = format!("{}, {status}", b.title);
                    view! {
                        <g
                            id=dom_id.clone()
                            class="iso-node"
                            role="button"
                            tabindex=if pos == 0 { "0" } else { "-1" }
                            aria-label=aria_lbl
                            data-urn=b.slug.clone()
                            data-tint=b.tint.clone()
                            data-state=state
                            data-selected=if pos == 0 { "true" } else { "false" }
                        >
                            <polygon id=format!("{dom_id}-r") class="iso-select-ring" points=pb.ring />
                            <polygon id=format!("{dom_id}-s") points=pb.shadow fill="var(--iso-shadow)" />
                            <polygon id=format!("{dom_id}-a") points=pb.side_a fill="var(--iso-left)" stroke="var(--iso-outline)" stroke-width="0.9" stroke-linejoin="round" />
                            <polygon id=format!("{dom_id}-b") points=pb.side_b fill="var(--iso-right)" stroke="var(--iso-outline)" stroke-width="0.9" stroke-linejoin="round" />
                            <polygon id=format!("{dom_id}-t") points=pb.top fill="var(--node-tint)" stroke="var(--iso-outline)" stroke-width="1.0" stroke-linejoin="round" />
                            <circle id=format!("{dom_id}-l") cx=fmt_t1(pb.led.0) cy=fmt_t1(pb.led.1) r="2.6" fill="var(--node-led)" />
                            <g class="iso-callout" aria-hidden="true">
                                <rect id=format!("{dom_id}-cb") class="iso-callout-box" x=fmt_t1(cx) y=fmt_t1(cy) width=fmt_t1(cw) height="34" rx="3" />
                                <text id=format!("{dom_id}-ct") class="iso-callout-title" x=fmt_t1(cx + cw * 0.5) y=fmt_t1(cy + 14.0) text-anchor="middle">{b.title.clone()}</text>
                                <circle id=format!("{dom_id}-cg") class="iso-callout-glyph" data-state=state cx=fmt_t1(cx + 12.0) cy=fmt_t1(cy + 26.0) r="3.4" />
                                <text id=format!("{dom_id}-cs") class="iso-callout-sub" x=fmt_t1(cx + 19.5) y=fmt_t1(cy + 29.5) text-anchor="start">{status}</text>
                            </g>
                        </g>
                    }
                }).collect_view()}
            </g>
        </svg>
    }
}

/// Renders the SSR `<details class="assets-drawer">` inventory passed as zero-WASM children.
#[component]
pub fn SsrAssetsDrawer(
    /// Active cluster targets.
    targets: Vec<TargetDto>,
) -> impl IntoView {
    let total_cents: u32 = targets.iter().map(|t| t.monthly_cents).sum();
    let count = targets.len();
    let mut rows_attr = String::with_capacity(targets.len() * 64);
    for t in &targets {
        let _ = writeln!(
            rows_attr,
            "{}\t{}\t{}\t{}\t{}\t{}",
            t.urn, t.display_name, t.group, t.kind_label, t.status, t.monthly_cents
        );
    }
    view! {
        <div id="inspector-assets-wrap" data-rows=rows_attr>
            <details class="assets-drawer" open>
                <summary class="assets-summary">
                    <span>{format!("ASSETS ({count})")}</span>
                    <span class="num muted">{format_cents(total_cents)}</span>
                </summary>
                <ul class="rail-list">
                    {targets.into_iter().map(|t| {
                        let tone = status_tone(&t.status);
                        let spend = format_cents(t.monthly_cents);
                        view! {
                            <li class="asset-row">
                                <button type="button" class="asset-row-btn" data-urn=t.urn>
                                    <span>{t.display_name}</span>
                                    <span class="rail-item">
                                        <StaticStatusBadge label=t.status tone=tone />
                                        <span class="spend muted">{spend}</span>
                                    </span>
                                </button>
                            </li>
                        }
                    }).collect_view()}
                </ul>
            </details>
        </div>
    }
}

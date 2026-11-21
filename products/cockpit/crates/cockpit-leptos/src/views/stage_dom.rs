//! Browser DOM in-place SVG attribute patcher, selection sync, and pointer helpers.

#[cfg(target_arch = "wasm32")]
use crate::views::iso_client_math::{
    ClientProjector, ClientScene, fmt_pts, fmt_t1, project_box_bounds, topological_box_order,
    zoom_viewbox,
};

/// Computes deterministic SVG DOM identifier for a cluster URN / slug.
#[must_use]
pub(crate) fn node_dom_id(slug: &str) -> String {
    let mut out = String::with_capacity(slug.len() + 9);
    out.push_str("iso-node-");
    for ch in slug.chars() {
        out.push(if ch.is_ascii_alphanumeric() { ch } else { '-' });
    }
    out
}

/// Updates `viewBox`, zone/edge/node SVG geometry attributes, and topological
/// painter's DOM order in place on `wasm32` without recreating any SVG nodes.
#[cfg(target_arch = "wasm32")]
pub(crate) fn patch_svg_in_place(scene: &ClientScene, proj: ClientProjector, zoom: f32) {
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    let set_attr = |id: &str, name: &str, val: &str| {
        if let Some(el) = doc.get_element_by_id(id) {
            let _ = el.set_attribute(name, val);
        }
    };
    set_attr("iso-stage-svg", "viewBox", &zoom_viewbox(zoom));
    for &(zid, [u0, v0, u1, v1]) in &scene.zones {
        let pts = fmt_pts(&[
            proj.project_c(u0, v0, 0.0),
            proj.project_c(u1, v0, 0.0),
            proj.project_c(u1, v1, 0.0),
            proj.project_c(u0, v1, 0.0),
        ]);
        set_attr(&format!("iso-zone-p-{zid}"), "points", &pts);
        let lbl = proj.project_c((u0 + u1) / 2, v1 + 32, 0.0);
        let tid = format!("iso-zone-t-{zid}");
        set_attr(&tid, "x", &fmt_t1(lbl.0));
        set_attr(&tid, "y", &fmt_t1(lbl.1 + 4.0));
    }
    for (rid, r_pts) in &scene.routes {
        let sc: Vec<(f32, f32)> = r_pts
            .iter()
            .map(|&(u, v)| proj.project_c(u, v, 0.05))
            .collect();
        set_attr(&format!("iso-edge-{rid}"), "points", &fmt_pts(&sc));
    }
    let layer = doc.get_element_by_id("iso-nodes-layer");
    for idx in topological_box_order(&scene.boxes, proj) {
        let (slug, title_len, bounds) = &scene.boxes[idx];
        let pb = project_box_bounds(*title_len, *bounds, proj);
        let base = node_dom_id(slug);
        set_attr(&format!("{base}-r"), "points", &pb.ring);
        set_attr(&format!("{base}-s"), "points", &pb.shadow);
        set_attr(&format!("{base}-a"), "points", &pb.side_a);
        set_attr(&format!("{base}-b"), "points", &pb.side_b);
        set_attr(&format!("{base}-t"), "points", &pb.top);
        let lid = format!("{base}-l");
        set_attr(&lid, "cx", &fmt_t1(pb.led.0));
        set_attr(&lid, "cy", &fmt_t1(pb.led.1));
        let (cx, cy, cw) = pb.callout;
        let cb = format!("{base}-cb");
        set_attr(&cb, "x", &fmt_t1(cx));
        set_attr(&cb, "y", &fmt_t1(cy));
        set_attr(&cb, "width", &fmt_t1(cw));
        let ct = format!("{base}-ct");
        set_attr(&ct, "x", &fmt_t1(cx + cw * 0.5));
        set_attr(&ct, "y", &fmt_t1(cy + 14.0));
        let cg = format!("{base}-cg");
        set_attr(&cg, "cx", &fmt_t1(cx + 12.0));
        set_attr(&cg, "cy", &fmt_t1(cy + 26.0));
        let cs = format!("{base}-cs");
        set_attr(&cs, "x", &fmt_t1(cx + 19.5));
        set_attr(&cs, "y", &fmt_t1(cy + 29.5));
        if let (Some(parent), Some(node_el)) = (&layer, doc.get_element_by_id(&base)) {
            let _ = parent.append_child(&node_el);
        }
    }
}

/// Syncs `data-selected` and `tabindex` across SSR node `<g>` elements on selection change.
#[cfg(target_arch = "wasm32")]
pub(crate) fn sync_node_selection(scene: &ClientScene, selected: Option<&str>) {
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    for (idx, (slug, ..)) in scene.boxes.iter().enumerate() {
        let is_sel = selected == Some(slug.as_str());
        if let Some(el) = doc.get_element_by_id(&node_dom_id(slug)) {
            let _ = el.set_attribute("data-selected", if is_sel { "true" } else { "false" });
            let tab = if is_sel || (idx == 0 && selected.is_none()) {
                "0"
            } else {
                "-1"
            };
            let _ = el.set_attribute("tabindex", tab);
        }
    }
}

/// Reads an attribute from a server-rendered DOM element by id on `wasm32`.
#[cfg(target_arch = "wasm32")]
pub(crate) fn read_dom_attr(id: &str, attr: &str) -> String {
    web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(id))
        .and_then(|el| el.get_attribute(attr))
        .unwrap_or_default()
}

/// Extracts `data-urn` from a clicked node or asset row ancestor.
pub(crate) fn clicked_urn(ev: &leptos::ev::MouseEvent) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        ev.target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .and_then(|el| el.closest("[data-urn]").ok().flatten())
            .and_then(|el| el.get_attribute("data-urn"))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = ev;
        None
    }
}

/// True if a pointer event originated on a node sprite or callout card.
pub(crate) fn hit_node_target(ev: &leptos::ev::PointerEvent) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        ev.target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .and_then(|el| el.closest(".iso-node, .iso-callout").ok().flatten())
            .is_some()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = ev;
        false
    }
}

/// Acquires or releases pointer capture on `wasm32`.
pub(crate) fn capture_ptr(ev: &leptos::ev::PointerEvent, capture: bool) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        if let Some(el) = ev
            .current_target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        {
            let _ = if capture {
                el.set_pointer_capture(ev.pointer_id())
            } else {
                el.release_pointer_capture(ev.pointer_id())
            };
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (ev, capture);
    }
}
